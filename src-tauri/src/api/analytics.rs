use super::Result;
use crate::state::AppStateHandle;
use patch_core::{aggregate_network_analytics, NetworkAnalyticsOverview};
use uuid::Uuid;

#[tauri::command]
pub async fn analytics_network_overview(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
) -> Result<NetworkAnalyticsOverview> {
    let inner = state.get().await?;
    let network = inner.db.get_network(network_id).await?;
    let token = inner.db.ensure_analytics_export_token(network_id).await?;
    let servers = inner.db.list_servers_for_network(network_id).await?;
    Ok(aggregate_network_analytics(&network, &servers, &token)?)
}
