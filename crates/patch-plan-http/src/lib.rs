//! Embedded Plan-shaped HTTP API for external dashboards (indepth.md §5.3).

mod auth_layer;

use axum::{
    extract::{Query, State},
    http::{header::SET_COOKIE, HeaderMap, HeaderValue, StatusCode},
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use patch_core::{
    execute_plan_query, extension_data_v1, ingest_signed_export, list_join_addresses, list_kills,
    list_web_users, logout_web_user, network_overview_v1, player_gdpr_export_json,
    plan_log_access, plan_web_login, start_web_registration, PlanJsonCache,
    online_overview_v1, performance_overview_v1, plan_filters_catalog, plan_graph, plan_theme_json,
    player_profile_v1, player_versus_v1, playerbase_overview_v1, players_table_v1, players_v1,
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
}

#[derive(Debug, Deserialize)]
struct ServerQuery {
    server: Option<String>,
    player: Option<String>,
    #[serde(rename = "type")]
    graph_type: Option<String>,
    name: Option<String>,
}

pub fn router(state: PlanHttpState) -> Router {
    let export_dir = state.static_export_dir.clone();
    let session_protected = Router::new()
        .route("/v1/query", post(query))
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
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_layer::require_admin_session,
        ))
        .with_state(state.clone());

    let mut router = Router::new()
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
        .route("/v1/network/overview", get(network_overview))
        .route("/v1/network/servers", get(network_servers))
        .route("/v1/network/playerbaseOverview", get(network_playerbase_overview))
        .route("/v1/network/sessionsOverview", get(network_sessions_overview))
        .route("/v1/network/pingTable", get(network_ping_table))
        .route("/v1/player", get(player_profile))
        .route("/v1/player/json", get(player_gdpr_json))
        .route("/v1/graph", get(graph))
        .route("/", get(plan_dashboard_redirect))
        .route("/app", get(plan_embedded_app))
        .route("/app/", get(plan_embedded_app))
        .route("/v1/theme", get(theme))
        .route("/v1/auth/login", post(auth_login))
        .route("/v1/auth/register", post(auth_register))
        .merge(session_protected)
        .merge(admin_protected)
        .with_state(state);
    if let Some(dir) = export_dir {
        if dir.exists() {
            router = router.nest_service("/export", ServeDir::new(dir));
        }
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
    let app = router(state).layer(tower_http::cors::CorsLayer::permissive());
    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn live_online(st: &PlanHttpState, server: Option<&str>) -> u32 {
    server
        .and_then(|uuid| st.online_by_server.get(uuid).copied())
        .unwrap_or(0)
}

async fn plan_dashboard_redirect() -> impl IntoResponse {
    (
        StatusCode::TEMPORARY_REDIRECT,
        [(axum::http::header::LOCATION, "/app")],
    )
}

async fn plan_embedded_app() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("../static/plan-dashboard.html"),
    )
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
            match with_plan_cache(&st.json_cache, key, false, || async {
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
                Ok(v) => Json(v).into_response(),
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

async fn extension_data(State(_st): State<PlanHttpState>) -> impl IntoResponse {
    Json(extension_data_v1())
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

async fn player_profile(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let player = q.player.as_deref().unwrap_or("");
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

async fn theme(Query(q): Query<ServerQuery>) -> impl IntoResponse {
    let name = q.name.as_deref().unwrap_or("default");
    Json(plan_theme_json(name))
}
