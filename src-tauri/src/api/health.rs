use super::Result;
use crate::state::AppStateHandle;
use patch_core::NetworkHealthReport;
use uuid::Uuid;

#[tauri::command]
pub async fn network_health_get(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
) -> Result<NetworkHealthReport> {
    let inner = state.get().await?;
    let network = inner.db.get_network(network_id).await?;
    let servers = inner.db.list_servers_for_network(network_id).await?;
    let settings = inner.settings().await;
    let token = inner.db.ensure_analytics_export_token(network_id).await?;
    let report =
        patch_core::check_network_health(&network, &servers, &settings.bind_host, &token).await;
    let _ = inner
        .db
        .record_health_snapshot(network_id, report.overall.as_str())
        .await;
    Ok(report)
}

#[tauri::command]
pub async fn network_status_page(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
) -> Result<patch_core::NetworkStatusPage> {
    let inner = state.get().await?;
    let network = inner.db.get_network(network_id).await?;
    let settings = inner.settings().await;
    let ping = patch_core::ping_server(&settings.bind_host, network.hub_port).await?;
    Ok(patch_core::build_status_page(&network, &settings.bind_host, &ping))
}
