//! ResolverService subset — plugin-registered JSON handlers (indepth.md §8).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use crate::PlanHttpState;

type ResolverHandler = fn() -> Value;

fn registry() -> &'static RwLock<HashMap<String, ResolverHandler>> {
    static REG: OnceLock<RwLock<HashMap<String, ResolverHandler>>> = OnceLock::new();
    REG.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn register_resolver_route(plugin: &str, handler: ResolverHandler) {
    registry()
        .write()
        .expect("resolver registry lock")
        .insert(plugin.to_ascii_lowercase(), handler);
}

async fn harvest_metrics_status(st: &PlanHttpState) -> Value {
    let users = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM plan_users WHERE network_id = ?",
    )
    .bind(&st.network_id)
    .fetch_one(&st.pool)
    .await
    .unwrap_or(0);
    let sessions = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM plan_sessions WHERE network_id = ?",
    )
    .bind(&st.network_id)
    .fetch_one(&st.pool)
    .await
    .unwrap_or(0);
    let online = st.online_by_server.values().sum::<u32>();
    json!({
        "plugin": "HarvestMetrics",
        "status": "ok",
        "duckdb": true,
        "network_id": st.network_id,
        "players_total": users,
        "sessions_total": sessions,
        "players_online": online,
    })
}

pub async fn resolver_plugin_get(
    State(st): State<PlanHttpState>,
    Path(plugin): Path<String>,
) -> impl IntoResponse {
    let key = plugin.to_ascii_lowercase();
    if key == "harvestmetrics" {
        return Json(harvest_metrics_status(&st).await).into_response();
    }
    let reg = registry().read().expect("resolver registry lock");
    match reg.get(&key) {
        Some(handler) => Json(handler()).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "unknown resolver plugin", "plugin": plugin })),
        )
            .into_response(),
    }
}
