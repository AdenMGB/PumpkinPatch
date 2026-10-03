use super::Result;
use crate::state::AppStateHandle;
use patch_core::{export_network_backup, import_network_backup};
use std::path::PathBuf;
use uuid::Uuid;

#[tauri::command]
pub async fn network_export_backup_default(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
) -> Result<String> {
    let inner = state.get().await?;
    let dir = inner.dirs.root.join("backups");
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(format!("{network_id}.zip"));
    export_network_backup(&inner.db, network_id, &dest).await?;
    inner
        .db
        .append_audit(
            "backup_export",
            "network",
            Some(&network_id.to_string()),
            &dest.to_string_lossy(),
        )
        .await?;
    Ok(dest.to_string_lossy().into())
}

#[tauri::command]
pub async fn network_export_backup(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    dest_path: String,
) -> Result<()> {
    let inner = state.get().await?;
    export_network_backup(&inner.db, network_id, PathBuf::from(dest_path).as_path()).await?;
    inner
        .db
        .append_audit(
            "backup_export",
            "network",
            Some(&network_id.to_string()),
            "network backup exported",
        )
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn network_import_backup(
    state: tauri::State<'_, AppStateHandle>,
    zip_path: String,
) -> Result<Uuid> {
    let inner = state.get().await?;
    let id = import_network_backup(
        &inner.db,
        PathBuf::from(zip_path).as_path(),
        &inner.dirs.networks,
        None,
    )
    .await?;
    inner
        .db
        .append_audit(
            "backup_import",
            "network",
            Some(&id.to_string()),
            "network restored from backup",
        )
        .await?;
    Ok(id)
}
