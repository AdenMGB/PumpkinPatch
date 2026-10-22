use super::Result;
use crate::state::AppStateHandle;
use patch_core::{
    backup_plan_sqlite_file, clear_plan_network_data, optimize_plan_sqlite, remove_plan_player,
    PlanDbBackupReport,
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
