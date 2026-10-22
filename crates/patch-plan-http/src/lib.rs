//! Embedded Plan-shaped HTTP API for external dashboards (indepth.md §5.3).

mod auth_layer;
mod client_ip;
mod manage_api;
mod plan_react;
mod plan_resolver;
mod plan_tls;
mod react_compat;

use axum::{
    extract::{Query, State},
    http::{header::SET_COOKIE, HeaderMap, HeaderValue, StatusCode},
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use patch_core::{
    execute_plan_query, execute_plan_sql_write, execute_readonly_sql, extension_data_merged,
    game_allowlist_bounces_v1,
    ingest_signed_export,
    plan_data_clear_events_since, sync_all_servers_extensions,
    list_command_usages,
    list_join_addresses, list_kills, players_online_at,
    list_web_users, load_cached_plan_query, logout_web_user, network_overview_v1,
    player_gdpr_export_json, plan_log_access, plan_web_login, read_disk_cache, start_web_registration,
    write_disk_cache, PlanJsonCache,
    online_overview_v1, performance_overview_v1, plan_filters_catalog, plan_graph, plan_theme_json,
    plan_theme_json_db, save_plan_theme_json,
    player_profile_v1, player_versus_v1, playerbase_overview_v1,
    players_table_v1, players_v1, statistics_v1,
    retention_v1, server_overview_v1, sessions_overview_v1, sessions_v1, with_plan_cache,
    PlanQueryRequest,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::services::ServeDir;

#[derive(Clone)]
pub struct PlanHttpState {
    pub network_id: String,
    pub pool: sqlx::SqlitePool,
    pub online_by_server: Arc<HashMap<String, u32>>,
    pub static_export_dir: Option<PathBuf>,
    pub json_cache: Arc<PlanJsonCache>,
    pub export_secret: String,
    /// When true, `X-Forwarded-For` / `X-Real-IP` populate `plan_access_log.from_ip` (indepth §5.2).
    pub trust_forwarded: bool,
    /// HTTPS material (PEM or PKCS12) per indepth §5.2.
    pub tls: Option<patch_core::PlanTlsMaterial>,
}

#[derive(Debug, Deserialize)]
struct ServerQuery {
    server: Option<String>,
    player: Option<String>,
    #[serde(rename = "type")]
    graph_type: Option<String>,
    name: Option<String>,
    token: Option<String>,
    /// Plan ETag-style cache bypass (`?timestamp=` epoch ms).
    timestamp: Option<i64>,
    /// `/v1/playersOnline?date=` epoch ms.
    date: Option<i64>,
    aggregate: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct QueryTokenQuery {
    token: String,
}

pub fn router(state: PlanHttpState) -> Router {
    let export_dir = state.static_export_dir.clone();
    let session_protected = Router::new()
        .route("/v1/query", get(query_by_token).post(query))
        .route("/v1/query/sql", post(query_sql_readonly))
        .route("/v1/query/dataClearEvents", get(query_data_clear_events))
        .route("/v1/import", post(import_signed))
        .route("/v1/errors", get(errors_diagnostics))
        .route("/v1/filters", get(filters))
        .route("/v1/docs", get(openapi_docs))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_layer::require_plan_session,
        ))
        .with_state(state.clone());

    let admin_protected = Router::new()
        .route("/v1/auth/users", get(auth_users))
        .route("/v1/auth/logout", post(auth_logout))
        .route("/v1/webGroups", get(manage_api::web_groups))
        .route("/v1/permissions", get(manage_api::permissions_catalog))
        .route("/v1/groupPermissions", get(manage_api::group_permissions))
        .route("/v1/saveGroupPermissions", post(manage_api::save_group_permissions))
        .route("/v1/deleteGroup", axum::routing::delete(manage_api::delete_group))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_layer::require_admin_session,
        ))
        .with_state(state.clone());

    let mut router = {
        let r = Router::new()
        .route("/v1/serverOverview", get(server_overview))
        .route("/v1/onlineOverview", get(online_overview))
        .route("/v1/sessionsOverview", get(sessions_overview))
        .route("/v1/playerVersus", get(player_versus))
        .route("/v1/playerbaseOverview", get(playerbase_overview))
        .route("/v1/performanceOverview", get(performance_overview))
        .route("/v1/sessions", get(sessions))
        .route("/v1/kills", get(kills))
        .route("/v1/pingTable", get(ping_table))
        .route("/v1/players", get(players))
        .route("/v1/playersTable", get(players_table))
        .route("/v1/joinAddresses", get(join_addresses))
        .route("/v1/retention", get(retention))
        .route("/v1/extensionData", get(extension_data))
        .route("/v1/extensions/sync", post(extensions_sync))
        .route("/v1/resolver/{plugin}", get(plan_resolver::resolver_plugin_get))
        .route("/v1/network/overview", get(network_overview))
        .route("/v1/network/servers", get(network_servers))
        .route("/v1/network/playerbaseOverview", get(network_playerbase_overview))
        .route("/v1/network/sessionsOverview", get(network_sessions_overview))
        .route("/v1/network/pingTable", get(network_ping_table))
        .route("/v1/network/retention", get(network_retention))
        .route("/v1/player", get(player_profile))
        .route("/v1/player/json", get(player_gdpr_json))
        .route("/v1/graph", get(graph))
        .route("/v1/capabilities", get(capabilities))
        .route("/v1/whoami", get(react_compat::whoami))
        .route("/auth/login", post(react_compat::auth_login_form))
        .route(
            "/auth/register",
            get(react_compat::auth_register_poll).post(react_compat::auth_register_form),
        )
        .route("/auth/logout", get(react_compat::auth_logout_get))
        .route("/v1/serverIdentity", get(server_identity))
        .route("/v1/onlineInsights", get(online_insights))
        .route("/v1/statistics", get(statistics))
        .route("/v1/gameAllowlistBounces", get(game_allowlist_bounces))
        .route("/v1/pluginHistory", get(plugin_history))
        .route("/v1/playersOnline", get(players_online))
        .route("/", get(plan_dashboard_redirect))
        .route("/app", get(plan_embedded_app))
        .route("/app/", get(plan_embedded_app));
        let r = plan_react::wrap_public_routes(r);
        let r = if plan_react::plan_react_available() {
            r
        } else {
            r.route("/query", get(plan_query_redirect))
                .route("/register", get(plan_register_page))
                .route("/theme-editor", get(plan_theme_editor_page))
                .route("/manage", get(plan_manage_page))
        };
        r.route("/v1/theme", get(theme).post(theme_save))
            .route("/v1/auth/login", post(auth_login))
            .route("/v1/auth/register", post(auth_register))
            .merge(session_protected)
            .merge(admin_protected)
            .with_state(state)
    };
    if let Some(dir) = export_dir {
        if dir.exists() {
            router = router.nest_service("/export", ServeDir::new(dir.clone()));
            let react_data = dir.join("data");
            if react_data.is_dir() {
                router = router.nest_service("/data", ServeDir::new(react_data));
            }
        }
    }
    let react_static = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static/plan-react/static");
    if react_static.exists() {
        router = router.nest_service("/static", ServeDir::new(react_static));
    }
    router
}

#[derive(Deserialize)]
struct LoginBody {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct RegisterBody {
    username: String,
    password: String,
}

async fn auth_register(
    State(st): State<PlanHttpState>,
    Json(body): Json<RegisterBody>,
) -> impl IntoResponse {
    match start_web_registration(&st.pool, &body.username, &body.password).await {
        Ok(start) => {
            let _ = plan_log_access(&st.pool, None, "POST", "/v1/auth/register", 200).await;
            Json(serde_json::json!({
                "code": start.code,
                "expires_ms": start.expires_ms,
                "message": "Run /plan register <code> in-game to link your Minecraft account."
            }))
            .into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct LogoutBody {
    username: String,
}

async fn auth_users(State(st): State<PlanHttpState>) -> impl IntoResponse {
    match list_web_users(&st.pool).await {
        Ok(users) => Json(serde_json::json!({ "users": users })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn auth_logout(
    State(st): State<PlanHttpState>,
    Json(body): Json<LogoutBody>,
) -> impl IntoResponse {
    match logout_web_user(&st.pool, &body.username).await {
        Ok(count) => Json(serde_json::json!({ "sessions_cleared": count })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn auth_login(
    State(st): State<PlanHttpState>,
    Json(body): Json<LoginBody>,
) -> impl IntoResponse {
    match plan_web_login(&st.pool, &body.username, &body.password).await {
        Ok(Some(cookie)) => {
            let _ = plan_log_access(&st.pool, None, "POST", "/v1/auth/login", 200).await;
            let mut response = Json(serde_json::json!({ "cookie": cookie })).into_response();
            if let Ok(value) = HeaderValue::from_str(&format!(
                "plan_session={}; Path=/; HttpOnly; SameSite=Lax",
                cookie
            )) {
                response.headers_mut().append(SET_COOKIE, value);
            }
            response
        }
        Ok(None) => (StatusCode::UNAUTHORIZED, "invalid credentials").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn serve(bind: SocketAddr, state: PlanHttpState) -> Result<(), std::io::Error> {
    let tls_material = state.tls.clone();
    let app = router(state).layer(tower_http::cors::CorsLayer::permissive());
    if let Some(material) = tls_material {
        let tls = plan_tls::rustls_config_from(&material).await?;
        axum_server::bind_rustls(bind, tls)
            .serve(app.into_make_service())
            .await?;
    } else {
        let listener = tokio::net::TcpListener::bind(bind).await?;
        axum::serve(listener, app).await?;
    }
    Ok(())
}

fn live_online(st: &PlanHttpState, server: Option<&str>) -> u32 {
    server
        .and_then(|uuid| st.online_by_server.get(uuid).copied())
        .unwrap_or(0)
}

async fn capabilities() -> impl IntoResponse {
    Json(patch_plan_core::plan_capabilities_v1())
}

async fn plan_dashboard_redirect() -> axum::response::Response {
    if plan_react::plan_react_available() {
        return plan_react::serve_plan_react_asset("index.html").await;
    }
    (
        StatusCode::TEMPORARY_REDIRECT,
        [(axum::http::header::LOCATION, "/app")],
    )
        .into_response()
}

async fn plan_query_redirect() -> impl IntoResponse {
    (
        StatusCode::TEMPORARY_REDIRECT,
        [(axum::http::header::LOCATION, "/app?panel=query")],
    )
}

async fn plan_embedded_app() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("../static/plan-dashboard.html"),
    )
}

async fn plan_register_page() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("../static/register.html"),
    )
}

async fn plan_theme_editor_page(
    State(st): State<PlanHttpState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if auth_layer::authorize_session_permission(&st, &headers, "access.theme.editor")
        .await
        .is_err()
    {
        return (StatusCode::UNAUTHORIZED, "access.theme.editor required").into_response();
    }
    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("../static/theme-editor.html"),
    )
        .into_response()
}

async fn persist_disk_cache(st: &PlanHttpState, key: &str, json: &str) {
    if let Some(dir) = &st.static_export_dir {
        let _ = write_disk_cache(dir, key, json);
    }
}

async fn try_disk_cache(st: &PlanHttpState, key: &str) -> Option<String> {
    st.static_export_dir
        .as_ref()
        .and_then(|dir| read_disk_cache(dir, key).ok().flatten())
}

async fn import_signed(
    State(st): State<PlanHttpState>,
    headers: HeaderMap,
    Json(body): Json<patch_analytics_protocol::SignedExport>,
) -> impl IntoResponse {
    if let Err(resp) =
        auth_layer::authorize_session_permission(&st, &headers, "access.data.import").await
    {
        return (resp.0, resp.1).into_response();
    }
    match ingest_signed_export(&st.pool, &st.network_id, &body, &st.export_secret).await {
        Ok(report) => Json(serde_json::json!({
            "users_upserted": report.users_upserted,
            "sessions_inserted": report.sessions_inserted,
            "servers_synced": report.servers_synced,
        }))
        .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn player_gdpr_json(
    State(st): State<PlanHttpState>,
    headers: HeaderMap,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let player = q.player.as_deref().unwrap_or("");
    if player.is_empty() {
        return (StatusCode::BAD_REQUEST, "player query parameter required").into_response();
    }
    if let Some(username) = auth_layer::authorize_session_permission(
        &st,
        &headers,
        "page.player.other",
    )
    .await
    .ok()
    {
        let _ = username;
    } else if let Ok(username) =
        auth_layer::authorize_session_permission(&st, &headers, "page.player.self").await
    {
        let linked: Option<String> = sqlx::query_scalar(
            "SELECT linked_to_uuid FROM plan_security WHERE username = ? LIMIT 1",
        )
        .bind(&username)
        .fetch_optional(&st.pool)
        .await
        .unwrap_or(None);
        let allowed = linked
            .map(|u| u.eq_ignore_ascii_case(player))
            .unwrap_or(false);
        if !allowed {
            return (StatusCode::FORBIDDEN, "page.player.self — linked account only").into_response();
        }
    } else {
        return (StatusCode::UNAUTHORIZED, "Plan session required").into_response();
    }
    match player_gdpr_export_json(&st.pool, &st.network_id, player).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn server_overview(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let server = q
        .server
        .as_deref()
        .ok_or_else(|| "server query parameter required".to_string());
    match server {
        Ok(uuid) => {
            let key = PlanJsonCache::cache_key(
                "serverOverview",
                &st.network_id,
                Some(uuid),
                "",
            );
            let bypass = q.timestamp.is_some();
            if !bypass {
                if let Some(hit) = try_disk_cache(&st, &key).await {
                    if let Ok(v) = serde_json::from_str::<patch_core::PlanServerOverviewV1>(&hit) {
                        return Json(v).into_response();
                    }
                }
            }
            match with_plan_cache(&st.json_cache, key.clone(), bypass, || async {
                server_overview_v1(
                    &st.pool,
                    &st.network_id,
                    uuid,
                    live_online(&st, Some(uuid)),
                )
                .await
            })
            .await
            {
                Ok(v) => {
                    if let Ok(json) = serde_json::to_string(&v) {
                        persist_disk_cache(&st, &key, &json).await;
                    }
                    Json(v).into_response()
                }
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
            }
        }
        Err(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
    }
}

async fn online_overview(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let server = q.server.as_deref();
    let key = PlanJsonCache::cache_key("onlineOverview", &st.network_id, server, "");
    match with_plan_cache(&st.json_cache, key, false, || async {
        online_overview_v1(
            &st.pool,
            &st.network_id,
            server,
            live_online(&st, server),
        )
        .await
    })
    .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn sessions_overview(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let server = q.server.as_deref();
    let key = PlanJsonCache::cache_key("sessionsOverview", &st.network_id, server, "");
    match with_plan_cache(&st.json_cache, key, false, || async {
        sessions_overview_v1(&st.pool, &st.network_id, server).await
    })
    .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn player_versus(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let server = q.server.as_deref();
    match player_versus_v1(&st.pool, &st.network_id, server).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn playerbase_overview(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let server = q.server.as_deref();
    let key = PlanJsonCache::cache_key("playerbaseOverview", &st.network_id, server, "");
    match with_plan_cache(&st.json_cache, key, false, || async {
        playerbase_overview_v1(&st.pool, &st.network_id, server).await
    })
    .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn performance_overview(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let server = q.server.as_deref();
    let key = PlanJsonCache::cache_key("performanceOverview", &st.network_id, server, "");
    match with_plan_cache(&st.json_cache, key, false, || async {
        performance_overview_v1(&st.pool, &st.network_id, server).await
    })
    .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn sessions(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let server = q.server.as_deref();
    match sessions_v1(&st.pool, &st.network_id, server, 200).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn kills(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let server = q.server.as_deref();
    match list_kills(&st.pool, &st.network_id, server, 200).await {
        Ok(v) => Json(serde_json::json!({ "player_kills": v })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn ping_table(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let server = q.server.as_deref();
    match patch_core::ping_table_v1(&st.pool, &st.network_id, server).await {
        Ok(rows) => Json(serde_json::json!({ "ping_table": rows })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn players(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let server = q.server.as_deref();
    match players_v1(&st.pool, &st.network_id, server, 500).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn players_table(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let server = q.server.as_deref();
    match players_table_v1(&st.pool, &st.network_id, server, 500).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn join_addresses(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let server = q.server.as_deref();
    match list_join_addresses(&st.pool, &st.network_id, server).await {
        Ok(rows) => Json(serde_json::json!({ "join_addresses": rows })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn retention(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let server = q.server.as_deref();
    match retention_v1(&st.pool, &st.network_id, server, 500).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn extension_data(State(st): State<PlanHttpState>) -> impl IntoResponse {
    match extension_data_merged(&st.pool, &st.network_id).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn extensions_sync(
    State(st): State<PlanHttpState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(resp) =
        auth_layer::authorize_session_permission(&st, &headers, "access.query").await
    {
        return (resp.0, resp.1).into_response();
    }
    match sync_all_servers_extensions(&st.pool, &st.network_id).await {
        Ok(rows) => Json(serde_json::json!({ "rows_synced": rows })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn network_overview(State(st): State<PlanHttpState>) -> impl IntoResponse {
    let triples: Vec<(String, String, u32)> = st
        .online_by_server
        .iter()
        .map(|(uuid, online)| (uuid.clone(), uuid.clone(), *online))
        .collect();
    match network_overview_v1(&st.pool, &st.network_id, &triples).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn network_servers(State(st): State<PlanHttpState>) -> impl IntoResponse {
    match patch_core::plan_network_servers_v1(&st.pool, &st.network_id).await {
        Ok(rows) => Json(serde_json::json!({ "servers": rows })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn network_playerbase_overview(State(st): State<PlanHttpState>) -> impl IntoResponse {
    let key = PlanJsonCache::cache_key("playerbaseOverview", &st.network_id, None, "network");
    match with_plan_cache(&st.json_cache, key, false, || async {
        playerbase_overview_v1(&st.pool, &st.network_id, None).await
    })
    .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn network_sessions_overview(State(st): State<PlanHttpState>) -> impl IntoResponse {
    let key = PlanJsonCache::cache_key("sessionsOverview", &st.network_id, None, "network");
    match with_plan_cache(&st.json_cache, key, false, || async {
        sessions_overview_v1(&st.pool, &st.network_id, None).await
    })
    .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn network_ping_table(State(st): State<PlanHttpState>) -> impl IntoResponse {
    match patch_core::ping_table_v1(&st.pool, &st.network_id, None).await {
        Ok(rows) => Json(serde_json::json!({ "ping_table": rows })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn network_retention(State(st): State<PlanHttpState>) -> impl IntoResponse {
    match retention_v1(&st.pool, &st.network_id, None, 500).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn player_profile(
    State(st): State<PlanHttpState>,
    headers: HeaderMap,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let player = q.player.as_deref().unwrap_or("");
    if player.is_empty() {
        return (StatusCode::BAD_REQUEST, "player query parameter required").into_response();
    }
    if auth_layer::authorize_session_permission(&st, &headers, "page.player.other")
        .await
        .is_err()
    {
        if let Ok(username) =
            auth_layer::authorize_session_permission(&st, &headers, "page.player.self").await
        {
            let linked: Option<String> = sqlx::query_scalar(
                "SELECT linked_to_uuid FROM plan_security WHERE username = ? LIMIT 1",
            )
            .bind(&username)
            .fetch_optional(&st.pool)
            .await
            .unwrap_or(None);
            let allowed = linked
                .map(|u| u.eq_ignore_ascii_case(player))
                .unwrap_or(false);
            if !allowed {
                return (StatusCode::FORBIDDEN, "page.player.self — linked account only")
                    .into_response();
            }
        } else {
            return (StatusCode::UNAUTHORIZED, "Plan session required").into_response();
        }
    }
    match player_profile_v1(&st.pool, &st.network_id, player).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn graph(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let graph_type = q.graph_type.as_deref().unwrap_or("playersOnline");
    let key = PlanJsonCache::cache_key(
        "graph",
        &st.network_id,
        q.server.as_deref(),
        graph_type,
    );
    let cache_key = format!("graph|{}", key);
    match with_plan_cache(&st.json_cache, cache_key, false, || async {
        plan_graph(&st.pool, &st.network_id, q.server.as_deref(), graph_type).await
    })
    .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn filters(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    match plan_filters_catalog(&st.pool, &st.network_id, q.server.as_deref()).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn query_by_token(
    State(st): State<PlanHttpState>,
    headers: HeaderMap,
    Query(q): Query<QueryTokenQuery>,
) -> impl IntoResponse {
    if let Err(resp) =
        auth_layer::authorize_session_permission(&st, &headers, "access.query").await
    {
        return (resp.0, resp.1).into_response();
    }
    match load_cached_plan_query(&st.pool, &q.token).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct QuerySqlBody {
    sql: String,
    #[serde(default)]
    write: bool,
}

#[derive(Deserialize)]
struct DataClearSinceQuery {
    #[serde(default)]
    since: i64,
}

async fn query_data_clear_events(
    headers: HeaderMap,
    State(st): State<PlanHttpState>,
    Query(q): Query<DataClearSinceQuery>,
) -> impl IntoResponse {
    if let Err(resp) =
        auth_layer::authorize_session_permission(&st, &headers, "access.query").await
    {
        return (resp.0, resp.1).into_response();
    }
    let events = plan_data_clear_events_since(q.since);
    Json(serde_json::json!({ "events": events })).into_response()
}

async fn query_sql_readonly(
    State(st): State<PlanHttpState>,
    headers: HeaderMap,
    Json(body): Json<QuerySqlBody>,
) -> impl IntoResponse {
    if let Err(resp) =
        auth_layer::authorize_session_permission(&st, &headers, "access.query").await
    {
        return (resp.0, resp.1).into_response();
    }
    if body.write {
        match execute_plan_sql_write(&st.pool, &body.sql).await {
            Ok(rows_affected) => {
                Json(serde_json::json!({ "rows_affected": rows_affected })).into_response()
            }
            Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        }
    } else {
        match execute_readonly_sql(&st.pool, &body.sql).await {
            Ok(rows) => Json(serde_json::json!({ "rows": rows })).into_response(),
            Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        }
    }
}

async fn query(
    State(st): State<PlanHttpState>,
    headers: axum::http::HeaderMap,
    Query(q): Query<ServerQuery>,
    Json(body): Json<PlanQueryRequest>,
) -> impl IntoResponse {
    if let Err(resp) =
        auth_layer::authorize_session_permission(&st, &headers, "access.query").await
    {
        return (resp.0, resp.1).into_response();
    }
    match execute_plan_query(&st.pool, &st.network_id, q.server.as_deref(), &body).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn errors_diagnostics(
    State(st): State<PlanHttpState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    if let Err(resp) =
        auth_layer::authorize_session_permission(&st, &headers, "access.errors").await
    {
        return (resp.0, resp.1).into_response();
    }
    let rows = sqlx::query_as::<_, (i64, String, String, i32)>(
        "SELECT time, request_method, request_uri, response_code FROM plan_access_log ORDER BY time DESC LIMIT 50",
    )
    .fetch_all(&st.pool)
    .await;
    match rows {
        Ok(entries) => {
            let list = entries
                .into_iter()
                .map(|(time, method, uri, code)| {
                    serde_json::json!({
                        "time_ms": time,
                        "method": method,
                        "uri": uri,
                        "response_code": code,
                    })
                })
                .collect::<Vec<_>>();
            Json(serde_json::json!({ "access_log": list })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn openapi_docs(
    State(st): State<PlanHttpState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    if let Err(resp) =
        auth_layer::authorize_session_permission(&st, &headers, "access.docs").await
    {
        return (resp.0, resp.1).into_response();
    }
    Json(serde_json::json!({
        "openapi": "3.0.3",
        "info": { "title": "Pumpkin Patch Plan API", "version": "1.0.0" },
        "paths": {
            "/v1/serverOverview": { "get": { "summary": "Server overview KPIs" } },
            "/v1/query": { "post": { "summary": "Cohort query builder" } },
            "/v1/auth/register": { "post": { "summary": "Start web account registration" } },
            "/v1/auth/login": { "post": { "summary": "Web login" } },
            "/v1/import": { "post": { "summary": "Import signed export.json payload" } }
        }
    }))
    .into_response()
}

async fn server_identity(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let id = q.server.as_deref().unwrap_or("");
    if id.is_empty() {
        return (StatusCode::BAD_REQUEST, "server query parameter required").into_response();
    }
    let row = sqlx::query(
        "SELECT uuid, name FROM plan_servers WHERE network_id = ? AND (uuid = ? OR name = ?) LIMIT 1",
    )
    .bind(&st.network_id)
    .bind(id)
    .bind(id)
    .fetch_optional(&st.pool)
    .await;
    match row {
        Ok(Some(r)) => {
            use sqlx::Row;
            Json(serde_json::json!({
                "serverName": r.get::<String, _>("name"),
                "serverUuid": r.get::<String, _>("uuid"),
                "proxyMode": false,
            }))
            .into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "server not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn online_insights(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    match online_overview_v1(
        &st.pool,
        &st.network_id,
        q.server.as_deref(),
        live_online(&st, q.server.as_deref()),
    )
    .await
    {
        Ok(v) => Json(serde_json::json!({ "insights": v.insights })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn statistics(State(st): State<PlanHttpState>, Query(q): Query<ServerQuery>) -> impl IntoResponse {
    let aggregate = q.aggregate.unwrap_or(false);
    match statistics_v1(
        &st.pool,
        &st.network_id,
        q.server.as_deref(),
        aggregate,
    )
    .await
    {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn game_allowlist_bounces(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    match game_allowlist_bounces_v1(&st.pool, &st.network_id, q.server.as_deref()).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn plugin_history(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let rows = match list_command_usages(
        &st.pool,
        &st.network_id,
        q.server.as_deref(),
        200,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let history = rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "pluginName": row.command,
                "timesUsed": row.times_used,
                "serverName": row.server_name,
            })
        })
        .collect::<Vec<_>>();
    Json(serde_json::json!({ "history": history })).into_response()
}

async fn players_online(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let at_ms = q.date.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    });
    match players_online_at(
        &st.pool,
        &st.network_id,
        q.server.as_deref(),
        at_ms,
    )
    .await
    {
        Ok(names) => Json(names).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn plan_manage_page(
    State(st): State<PlanHttpState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if auth_layer::authorize_session_permission(&st, &headers, "manage.groups")
        .await
        .is_err()
    {
        return (StatusCode::UNAUTHORIZED, "manage.groups required").into_response();
    }
    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("../static/manage.html"),
    )
        .into_response()
}

#[derive(Deserialize)]
struct ThemeSaveBody {
    name: String,
    theme: serde_json::Value,
}

async fn theme_save(
    State(st): State<PlanHttpState>,
    headers: HeaderMap,
    Json(body): Json<ThemeSaveBody>,
) -> impl IntoResponse {
    if let Err(resp) =
        auth_layer::authorize_session_permission(&st, &headers, "manage.themes").await
    {
        return (resp.0, resp.1).into_response();
    }
    match save_plan_theme_json(&st.pool, &body.name, &body.theme).await {
        Ok(()) => Json(serde_json::json!({ "saved": body.name })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn theme(State(st): State<PlanHttpState>, Query(q): Query<ServerQuery>) -> impl IntoResponse {
    let name = q.name.as_deref().unwrap_or("default");
    match plan_theme_json_db(&st.pool, name).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use patch_core::ensure_plan_schema;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn test_state() -> PlanHttpState {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("pool");
        ensure_plan_schema(&pool).await.expect("schema");
        PlanHttpState {
            network_id: "test-net".into(),
            pool,
            online_by_server: Arc::new(HashMap::new()),
            static_export_dir: None,
            json_cache: PlanJsonCache::new(),
            export_secret: "test-secret".into(),
            trust_forwarded: true,
            tls: None,
        }
    }

    #[tokio::test]
    async fn spa_login_returns_html_when_react_bundle_present() {
        if !plan_react::plan_react_available() {
            return;
        }
        let app = router(test_state().await);
        let response = app
            .oneshot(Request::get("/login").body(Body::empty()).unwrap())
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let html = String::from_utf8_lossy(&body);
        assert!(html.contains("<!DOCTYPE html") || html.contains("<html"));
    }

    #[tokio::test]
    async fn resolver_harvest_metrics_route() {
        let app = router(test_state().await);
        let response = app
            .oneshot(
                Request::get("/v1/resolver/HarvestMetrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn capabilities_endpoint_returns_json() {
        let app = router(test_state().await);
        let response = app
            .oneshot(Request::get("/v1/capabilities").body(Body::empty()).unwrap())
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }
}
