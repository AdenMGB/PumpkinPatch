//! Embedded Plan-shaped HTTP API for external dashboards (indepth.md §5.3).

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use patch_core::{
    execute_plan_query, extension_data_v1, list_join_addresses, list_kills, network_overview_v1,
    plan_log_access, plan_web_login, start_web_registration,
    online_overview_v1, performance_overview_v1, plan_filters_catalog, plan_graph, plan_theme_json,
    player_profile_v1, player_versus_v1, playerbase_overview_v1, players_table_v1, players_v1,
    retention_v1, server_overview_v1, sessions_overview_v1, sessions_v1, PlanQueryRequest,
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
        .route("/v1/player", get(player_profile))
        .route("/v1/graph", get(graph))
        .route("/v1/filters", get(filters))
        .route("/v1/query", post(query))
        .route("/v1/theme", get(theme))
        .route("/v1/auth/login", post(auth_login))
        .route("/v1/auth/register", post(auth_register))
        .route("/v1/errors", get(errors_diagnostics))
        .route("/v1/docs", get(openapi_docs))
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

async fn auth_login(
    State(st): State<PlanHttpState>,
    Json(body): Json<LoginBody>,
) -> impl IntoResponse {
    match plan_web_login(&st.pool, &body.username, &body.password).await {
        Ok(Some(cookie)) => {
            let _ = plan_log_access(&st.pool, None, "POST", "/v1/auth/login", 200).await;
            Json(serde_json::json!({ "cookie": cookie })).into_response()
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

async fn server_overview(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let server = q
        .server
        .as_deref()
        .ok_or_else(|| "server query parameter required".to_string());
    match server {
        Ok(uuid) => match server_overview_v1(
            &st.pool,
            &st.network_id,
            uuid,
            live_online(&st, Some(uuid)),
        )
        .await
        {
            Ok(v) => Json(v).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
    }
}

async fn online_overview(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let server = q.server.as_deref();
    match online_overview_v1(
        &st.pool,
        &st.network_id,
        server,
        live_online(&st, server),
    )
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
    match sessions_overview_v1(&st.pool, &st.network_id, server).await {
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
    match playerbase_overview_v1(&st.pool, &st.network_id, server).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn performance_overview(
    State(st): State<PlanHttpState>,
    Query(q): Query<ServerQuery>,
) -> impl IntoResponse {
    let server = q.server.as_deref();
    match performance_overview_v1(&st.pool, &st.network_id, server).await {
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
    match plan_graph(&st.pool, &st.network_id, q.server.as_deref(), graph_type).await {
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
    Query(q): Query<ServerQuery>,
    Json(body): Json<PlanQueryRequest>,
) -> impl IntoResponse {
    match execute_plan_query(&st.pool, &st.network_id, q.server.as_deref(), &body).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn errors_diagnostics(State(st): State<PlanHttpState>) -> impl IntoResponse {
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

async fn openapi_docs() -> impl IntoResponse {
    Json(serde_json::json!({
        "openapi": "3.0.3",
        "info": { "title": "Pumpkin Patch Plan API", "version": "1.0.0" },
        "paths": {
            "/v1/serverOverview": { "get": { "summary": "Server overview KPIs" } },
            "/v1/query": { "post": { "summary": "Cohort query builder" } },
            "/v1/auth/register": { "post": { "summary": "Start web account registration" } },
            "/v1/auth/login": { "post": { "summary": "Web login" } }
        }
    }))
    .into_response()
}

async fn theme(Query(q): Query<ServerQuery>) -> impl IntoResponse {
    let name = q.name.as_deref().unwrap_or("default");
    Json(plan_theme_json(name))
}
