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
    REG.get_or_init(|| {
        let mut map = HashMap::new();
        map.insert(
            "harvestmetrics".to_string(),
            harvest_metrics_status as ResolverHandler,
        );
        RwLock::new(map)
    })
}

fn harvest_metrics_status() -> Value {
    json!({
        "plugin": "HarvestMetrics",
        "status": "ok",
        "duckdb": true,
    })
}

pub fn register_resolver_route(plugin: &str, handler: ResolverHandler) {
    registry()
        .write()
        .expect("resolver registry lock")
        .insert(plugin.to_ascii_lowercase(), handler);
}

pub async fn resolver_plugin_get(
    State(_st): State<PlanHttpState>,
    Path(plugin): Path<String>,
) -> impl IntoResponse {
    let key = plugin.to_ascii_lowercase();
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
