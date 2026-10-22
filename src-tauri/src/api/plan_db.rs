use super::Result;
use crate::state::AppStateHandle;
use patch_core::{
    backup_plan_sqlite_file, clear_plan_network_data, import_signed_plan_export,
    mark_plan_server_uninstalled, optimize_plan_sqlite, remove_plan_player, PlanDbBackupReport,
    PlanSyncReport,
};
use serde::Serialize;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct PlanDbBackupResponse {
    pub path: String,
    pub bytes: u64,
}

#[tauri::command]
pub async fn plan_db_optimize(state: tauri::State<'_, AppStateHandle>) -> Result<()> {
    let inner = state.get().await?;
    optimize_plan_sqlite(inner.db.pool()).await?;
    Ok(())
}

#[tauri::command]
pub async fn plan_db_backup(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    label: Option<String>,
) -> Result<PlanDbBackupResponse> {
    let inner = state.get().await?;
    let network = inner.db.get_network(network_id).await?;
    let backups_dir = PathBuf::from(&network.data_path).join("plan-backups");
    let sub = label.unwrap_or_else(|| "sqlite".into());
    let report: PlanDbBackupReport =
        backup_plan_sqlite_file(inner.db.file_path(), &backups_dir.join(&sub)).await?;
    Ok(PlanDbBackupResponse {
        path: report.path.display().to_string(),
        bytes: report.bytes,
    })
}

#[tauri::command]
pub async fn plan_db_remove_player(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    target: String,
) -> Result<u64> {
    let inner = state.get().await?;
    Ok(
        remove_plan_player(inner.db.pool(), &network_id.to_string(), &target)
            .await?,
    )
}

#[tauri::command]
pub async fn plan_db_clear_network(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
) -> Result<u64> {
    let inner = state.get().await?;
    Ok(clear_plan_network_data(inner.db.pool(), &network_id.to_string()).await?)
}

#[tauri::command]
pub async fn plan_db_mark_server_uninstalled(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server: String,
) -> Result<bool> {
    let inner = state.get().await?;
    Ok(
        mark_plan_server_uninstalled(inner.db.pool(), &network_id.to_string(), &server)
            .await?,
    )
}

#[tauri::command]
pub async fn plan_import_signed_export(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    export_path: String,
) -> Result<PlanSyncReport> {
    let inner = state.get().await?;
    let token = inner.db.ensure_analytics_export_token(network_id).await?;
    let report = import_signed_plan_export(
        inner.db.pool(),
        &network_id.to_string(),
        std::path::Path::new(&export_path),
        &token,
    )
    .await?;
    inner.plan_json_cache.clear().await;
    Ok(report)
}
