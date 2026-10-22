use super::Result;
use crate::bundled_plugins::{deploy_manifest_plugin, provision_network_plugins};
use crate::state::AppStateHandle;
use patch_core::{
    append_log_line, ping_server, CreateNetworkRequest, DownloadProgress, JoinInfo, LogLine,
    NetworkOrchestrator, NetworkSummary, PingResult, PluginManifest, PluginManifestEntry,
};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

#[tauri::command]
pub async fn initialize_state(state: tauri::State<'_, AppStateHandle>) -> Result<()> {
    state.init().await?;
    Ok(())
}

#[tauri::command]
pub async fn network_list(state: tauri::State<'_, AppStateHandle>) -> Result<Vec<NetworkSummary>> {
    let inner = state.get().await?;
    let settings = inner.settings().await;
    let networks = inner.db.list_networks().await?;
    let processes = inner.processes.read().await;
    let mut out = Vec::new();
    for network in networks {
        let servers = inner.db.list_servers_for_network(network.id).await?;
        let status = processes.status(network.id);
        let join_address = format!("{}:{}", settings.bind_host, network.hub_port);
        out.push(NetworkSummary {
            network,
            servers,
            status,
            join_address,
        });
    }
    Ok(out)
}

#[tauri::command]
pub async fn network_get(
    state: tauri::State<'_, AppStateHandle>,
    id: Uuid,
) -> Result<NetworkSummary> {
    let inner = state.get().await?;
    let settings = inner.settings().await;
    let network = inner.db.get_network(id).await?;
    let servers = inner.db.list_servers_for_network(id).await?;
    let status = inner.processes.read().await.status(id);
    let join_address = format!("{}:{}", settings.bind_host, network.hub_port);
    Ok(NetworkSummary {
        network,
        servers,
        status,
        join_address,
    })
}

#[tauri::command]
pub async fn network_create(
    app: AppHandle,
    state: tauri::State<'_, AppStateHandle>,
    request: CreateNetworkRequest,
) -> Result<NetworkSummary> {
    let inner = state.get().await?;
    let settings = inner.settings().await;
    let summary = inner
        .networks
        .create_network(&inner.db, &settings, request)
        .await?;

    provision_network_plugins(&state, &inner, &summary).await;

    let _ = app.emit("network-created", &summary.network.id);
    Ok(summary)
}

#[tauri::command]
pub async fn network_delete(state: tauri::State<'_, AppStateHandle>, id: Uuid) -> Result<()> {
    let inner = state.get().await?;
    let network = inner.db.get_network(id).await?;
    {
        let mut processes = inner.processes.write().await;
        if processes.is_running(id) {
            let servers = inner.db.list_servers_for_network(id).await?;
            processes.stop_network(id, &network, &servers).await?;
        }
    }
    inner.db.delete_network(id).await?;
    let path = inner.dirs.networks.join(id.to_string());
    if path.exists() {
        std::fs::remove_dir_all(path).ok();
    }
    patch_core::clear_logs(&inner.dirs.logs, id).ok();
    Ok(())
}

#[tauri::command]
pub async fn network_start(
    app: AppHandle,
    state: tauri::State<'_, AppStateHandle>,
    id: Uuid,
) -> Result<()> {
    let inner = state.get().await?;
    let network = inner.db.get_network(id).await?;
    let servers = inner.db.list_servers_for_network(id).await?;
    let java_path = inner.resolve_java_for_runtime().await?;
    let settings = inner.settings().await;

    let (log_tx, mut log_rx) = tokio::sync::mpsc::unbounded_channel::<LogLine>();
    let logs_dir = inner.dirs.logs.clone();
    let app_handle = app.clone();
    tokio::spawn(async move {
        while let Some(line) = log_rx.recv().await {
            append_log_line(&logs_dir, &line).ok();
            let _ = app_handle.emit("network-log", &line);
        }
    });

    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel::<DownloadProgress>(64);
    let progress_app = app.clone();
    tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            let _ = progress_app.emit("network-download-progress", &progress);
        }
    });

    let plugins_root = state.plugins_root();
    let (pumpkins, velocity) = inner
        .networks
        .ensure_binaries_for_network(&servers, Some(progress_tx), Some(plugins_root.as_path()))
        .await?;

    let status = inner.processes.read().await.status(id);
    let join_address = format!("{}:{}", settings.bind_host, network.hub_port);
    provision_network_plugins(
        &state,
        &inner,
        &NetworkSummary {
            network: network.clone(),
            servers: servers.clone(),
            status,
            join_address,
        },
    )
    .await;

    let bridge_bind = format!("{}:25589", settings.bind_host);
    if let Ok(addr) = bridge_bind.parse() {
        let bridge = velocity_bridge::VelocityBridge::new();
        tokio::spawn(async move {
            let _ = bridge.serve(addr).await;
        });
    }

    let mut processes = inner.processes.write().await;
    processes
        .start_network(
            &network,
            &servers,
            pumpkins,
            velocity,
            &java_path,
            &settings.bind_host,
            &inner.networks,
            Some(log_tx),
        )
        .await?;
    let _ = app.emit("network-started", id);
    Ok(())
}

#[tauri::command]
pub async fn network_stop(
    app: AppHandle,
    state: tauri::State<'_, AppStateHandle>,
    id: Uuid,
) -> Result<()> {
    let inner = state.get().await?;
    let network = inner.db.get_network(id).await?;
    let servers = inner.db.list_servers_for_network(id).await?;
    let mut processes = inner.processes.write().await;
    processes.stop_network(id, &network, &servers).await?;
    let _ = app.emit("network-stopped", id);
    Ok(())
}

#[tauri::command]
pub async fn network_get_join_info(
    state: tauri::State<'_, AppStateHandle>,
    id: Uuid,
) -> Result<JoinInfo> {
    let inner = state.get().await?;
    let network = inner.db.get_network(id).await?;
    let settings = inner.settings().await;
    Ok(NetworkOrchestrator::join_info(&network, &settings.bind_host))
}

#[tauri::command]
pub async fn network_ping_hub(
    state: tauri::State<'_, AppStateHandle>,
    id: Uuid,
) -> Result<PingResult> {
    let inner = state.get().await?;
    let network = inner.db.get_network(id).await?;
    let settings = inner.settings().await;
    Ok(ping_server(&settings.bind_host, network.hub_port).await?)
}

#[tauri::command]
pub async fn network_logs_get(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    limit: Option<u32>,
) -> Result<Vec<LogLine>> {
    let inner = state.get().await?;
    let cap = limit.unwrap_or(500).min(2000) as usize;
    Ok(patch_core::read_log_tail(&inner.dirs.logs, network_id, cap)?)
}

#[tauri::command]
pub async fn plugin_list_available(
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<PluginManifestEntry>> {
    let root = state.plugins_root();
    let manifest = PluginManifest::load_from_repo(&root)?;
    Ok(manifest.plugins)
}

#[tauri::command]
pub async fn plugin_deploy(
    state: tauri::State<'_, AppStateHandle>,
    plugin_id: String,
    instance_path: String,
    server_role: Option<String>,
) -> Result<String> {
    let inner = state.get().await?;
    deploy_manifest_plugin(
        &state,
        &inner,
        &plugin_id,
        &instance_path,
        server_role.as_deref(),
    )
    .await
    .map_err(Into::into)
}
