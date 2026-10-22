use super::Result;
use crate::state::AppStateHandle;
use patch_core::{aggregate_network_analytics, sync_network_plan_exports};
use patch_plan_http::PlanHttpState;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use uuid::Uuid;

#[tauri::command]
pub async fn plan_http_start(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
) -> Result<String> {
    let inner = state.get().await?;
    let settings = inner.settings().await;
    if !settings.plan_http_enabled {
        return Err(patch_core::Error::InvalidState("Plan HTTP API disabled in settings".into()).into());
    }
    let network = inner.db.get_network(network_id).await?;
    let token = inner.db.ensure_analytics_export_token(network_id).await?;
    let servers = inner.db.list_servers_for_network(network_id).await?;
    sync_network_plan_exports(&inner.db, &network, &servers, &token).await?;
    let overview = aggregate_network_analytics(&network, &servers, &token)?;
    let online_by_server: HashMap<String, u32> = overview
        .servers
        .iter()
        .map(|s| (s.server_id.clone(), s.online_now))
        .collect();

    let bind: SocketAddr = format!("{}:{}", settings.bind_host, settings.plan_http_port)
        .parse()
        .map_err(|e| patch_core::Error::InvalidState(format!("invalid plan http bind: {e}")))?;

    let http_state = PlanHttpState {
        network_id: network.id.to_string(),
        pool: inner.db.pool().clone(),
        online_by_server: Arc::new(online_by_server),
    };

    tokio::spawn(async move {
        if let Err(e) = patch_plan_http::serve(bind, http_state).await {
            eprintln!("Plan HTTP server stopped: {e}");
        }
    });

    Ok(format!("Plan /v1 API listening on http://{bind}"))
}
