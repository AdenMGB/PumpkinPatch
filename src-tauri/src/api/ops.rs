use super::Result;
use crate::state::AppStateHandle;
use patch_core::{DownloadProgress, NetworkOrchestrator};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct PortMapEntry {
    pub label: String,
    pub port: u16,
}

#[tauri::command]
pub fn network_proposed_ports(hub_port: u16, backend_count: usize) -> Vec<PortMapEntry> {
    NetworkOrchestrator::proposed_port_map(hub_port, backend_count)
        .into_iter()
        .map(|(label, port)| PortMapEntry { label, port })
        .collect()
}

#[tauri::command]
pub async fn network_set_auto_restart(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    enabled: bool,
) -> Result<()> {
    let inner = state.get().await?;
    inner.db.set_network_auto_restart(network_id, enabled).await?;
    Ok(())
}

#[tauri::command]
pub async fn network_set_favorite(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    favorite: bool,
) -> Result<()> {
    let inner = state.get().await?;
    inner.db.set_network_favorite(network_id, favorite).await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDefinitionExport {
    pub network: patch_core::NetworkRecord,
    pub servers: Vec<patch_core::ServerRecord>,
}

#[tauri::command]
pub async fn network_export_definition(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    dest_path: String,
) -> Result<()> {
    let inner = state.get().await?;
    let network = inner.db.get_network(network_id).await?;
    let servers = inner.db.list_servers_for_network(network_id).await?;
    let payload = NetworkDefinitionExport { network, servers };
    let raw = serde_json::to_string_pretty(&payload)?;
    std::fs::write(dest_path, raw)?;
    Ok(())
}

#[tauri::command]
pub async fn java_validate(state: tauri::State<'_, AppStateHandle>) -> Result<String> {
    let inner = state.get().await?;
    let settings = inner.settings().await;
    let path = patch_core::resolve_java_executable(&settings.java_path)?;
    patch_core::ensure_java_meets_minimum(&path, settings.java_min_major)?;
    let major = patch_core::java_major_version(&path)?;
    Ok(format!("Java {major} at {}", path.display()))
}

#[tauri::command]
pub async fn network_update_binaries(
    app: AppHandle,
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
) -> Result<()> {
    let inner = state.get().await?;
    let servers = inner.db.list_servers_for_network(network_id).await?;
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel::<DownloadProgress>(32);
    let app_handle = app.clone();
    tokio::spawn(async move {
        while let Some(p) = progress_rx.recv().await {
            let _ = app_handle.emit("download-progress", &p);
        }
    });
    let _ = inner
        .networks
        .ensure_binaries_for_network(&servers, Some(progress_tx))
        .await?;
    Ok(())
}

#[tauri::command]
#[tauri::command]
pub async fn plugin_apply_profile(
    state: tauri::State<'_, AppStateHandle>,
    profile_id: String,
    instance_path: String,
    server_role: String,
) -> Result<Vec<String>> {
    let inner = state.get().await?;
    let profiles_root = state.plugins_root().join("profiles");
    let profile = patch_core::load_plugin_profile(&profiles_root, &profile_id)?;
    let deployed = patch_core::apply_plugin_profile(
        &state.plugins_root(),
        &inner.dirs.plugins_cache,
        &profile,
        PathBuf::from(&instance_path).as_path(),
        &server_role,
    )?;
    for path in &deployed {
        inner
            .db
            .record_deployed_plugin(&profile_id, &instance_path, &path.to_string_lossy())
            .await
            .ok();
    }
    Ok(deployed
        .into_iter()
        .map(|p| p.to_string_lossy().into())
        .collect())
}

#[tauri::command]
pub async fn cache_clear(state: tauri::State<'_, AppStateHandle>) -> Result<u64> {
    let inner = state.get().await?;
    let mut bytes = 0u64;
    for dir in [&inner.dirs.cache, &inner.dirs.plugins_cache] {
        if dir.exists() {
            bytes += dir_size(dir.as_path())?;
            std::fs::remove_dir_all(dir).ok();
            std::fs::create_dir_all(dir).ok();
        }
    }
    Ok(bytes)
}

fn dir_size(path: &std::path::Path) -> Result<u64> {
    let mut total = 0u64;
    if path.is_file() {
        return Ok(path.metadata()?.len());
    }
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        total += dir_size(&entry.path())?;
    }
    Ok(total)
}
