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
    online_overview_v1, performance_overview_v1, plan_filters_catalog, plan_graph, plan_theme_json,
    player_profile_v1, player_versus_v1, playerbase_overview_v1, players_table_v1, players_v1,
    retention_v1, server_overview_v1, sessions_overview_v1, sessions_v1, PlanQueryRequest,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
pub struct PlanHttpState {
    pub network_id: String,
    pub pool: sqlx::SqlitePool,
    pub online_by_server: Arc<HashMap<String, u32>>,
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
    Router::new()
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
        .with_state(state)
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

async fn filters(State(st): State<PlanHttpState>) -> impl IntoResponse {
    match plan_filters_catalog(&st.pool, &st.network_id).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn query(
    State(st): State<PlanHttpState>,
    Json(body): Json<PlanQueryRequest>,
) -> impl IntoResponse {
    match execute_plan_query(&st.pool, &st.network_id, &body).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn theme(Query(q): Query<ServerQuery>) -> impl IntoResponse {
    let name = q.name.as_deref().unwrap_or("default");
    Json(plan_theme_json(name))
}
