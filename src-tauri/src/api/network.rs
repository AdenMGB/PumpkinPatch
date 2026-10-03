use super::Result;
use crate::bundled_plugins::provision_network_plugins;
use crate::state::AppStateHandle;
use tauri::Manager;
use patch_core::{
    deploy_plugin, ensure_java_meets_minimum, ping_server, CreateNetworkRequest, JoinInfo,
    LogLine, NetworkOrchestrator, NetworkSummary, PingResult, PluginManifest, PluginManifestEntry,
    ProcessExitEvent,
};
use std::path::PathBuf;
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
    deploy_lobby_plugin_if_configured(&state, &inner, &summary);

    let _ = app.emit("network-created", &summary.network.id);
    Ok(summary)
}

#[tauri::command]
pub async fn network_delete(state: tauri::State<'_, AppStateHandle>, id: Uuid) -> Result<()> {
    let inner = state.get().await?;
    {
        let mut processes = inner.processes.write().await;
        if processes.is_running(id) {
            let network = inner.db.get_network(id).await?;
            let servers = inner.db.list_servers_for_network(id).await?;
            processes.stop_network(id, &network, &servers).await?;
        }
    }
    inner.db.delete_network(id).await?;
    let path = inner.dirs.networks.join(id.to_string());
    if path.exists() {
        std::fs::remove_dir_all(path).ok();
    }
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
    ensure_java_meets_minimum(
        std::path::Path::new(java_path.as_str()),
        settings.java_min_major,
    )?;

    NetworkOrchestrator::preflight_start_ports(
        &settings.bind_host,
        network.hub_port,
        &servers,
    )?;

    let (log_tx, mut log_rx) = tokio::sync::mpsc::unbounded_channel::<LogLine>();
    let (exit_tx, mut exit_rx) = tokio::sync::mpsc::unbounded_channel::<ProcessExitEvent>();
    let app_handle = app.clone();
    tokio::spawn(async move {
        while let Some(line) = log_rx.recv().await {
            let _ = app_handle.emit("network-log", &line);
        }
    });

    let app_exit = app.clone();
    tokio::spawn(async move {
        while let Some(ev) = exit_rx.recv().await {
            let _ = app_exit.emit("network-process-exited", &ev);
            let state_handle = app_exit.state::<AppStateHandle>();
            if let Ok(inner) = state_handle.get().await {
                let _ = inner
                    .db
                    .record_network_crash(ev.network_id, &ev.source)
                    .await;
                let network = inner.db.get_network(ev.network_id).await.ok();
                let should_restart = network.as_ref().map(|n| n.auto_restart).unwrap_or(false);
                let attempts = network.as_ref().map(|n| n.restart_attempts).unwrap_or(0);
                {
                    let mut processes = inner.processes.write().await;
                    processes.mark_error(ev.network_id);
                }
                if should_restart && attempts < 5 {
                    if let (Ok(network), Ok(servers)) = (
                        inner.db.get_network(ev.network_id).await,
                        inner.db.list_servers_for_network(ev.network_id).await,
                    ) {
                        let settings = inner.settings().await;
                        let java_path = inner.resolve_java_for_runtime().await.ok();
                        let (log_tx, mut log_rx) =
                            tokio::sync::mpsc::unbounded_channel::<LogLine>();
                        let app_log = app_exit.clone();
                        tokio::spawn(async move {
                            while let Some(line) = log_rx.recv().await {
                                let _ = app_log.emit("network-log", &line);
                            }
                        });
                        if let Ok(java_path) = java_path {
                            if let Ok((pumpkin, velocity)) = inner
                                .networks
                                .ensure_binaries_for_network(&servers, None)
                                .await
                            {
                                let mut processes = inner.processes.write().await;
                                let _ = processes
                                    .start_network(
                                        &network,
                                        &servers,
                                        pumpkin,
                                        velocity,
                                        &java_path,
                                        &settings.bind_host,
                                        &inner.networks,
                                        Some(log_tx),
                                        None,
                                    )
                                    .await;
                            }
                        }
                    }
                }
            }
        }
    });

    let (pumpkin, velocity) = inner
        .networks
        .ensure_binaries_for_network(&servers, None)
        .await?;

    inner.db.reset_restart_attempts(id).await?;
    inner.db.touch_network_played(id).await?;

    let mut processes = inner.processes.write().await;
    processes
        .start_network(
            &network,
            &servers,
            pumpkin,
            velocity,
            &java_path,
            &settings.bind_host,
            &inner.networks,
            Some(log_tx),
            Some(exit_tx),
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
    let root = state.plugins_root();
    let manifest = PluginManifest::load_from_repo(&root)?;
    let entry = manifest
        .get(&plugin_id)
        .ok_or_else(|| patch_core::Error::NotFound(plugin_id.clone()))?;
    let dest = deploy_plugin(
        &root,
        entry,
        PathBuf::from(&instance_path).as_path(),
        &inner.dirs.plugins_cache,
        server_role.as_deref(),
    )?;
    inner
        .db
        .record_deployed_plugin(
            &entry.id,
            &instance_path,
            &dest.to_string_lossy(),
        )
        .await
        .ok();
    Ok(dest.to_string_lossy().into())
}

fn deploy_lobby_plugin_if_configured(
    state: &AppStateHandle,
    inner: &patch_core::AppState,
    summary: &NetworkSummary,
) {
    let Some(lobby) = summary
        .servers
        .iter()
        .find(|s| s.role == "lobby")
        .cloned()
    else {
        return;
    };

    let root = state.plugins_root();
    let Ok(manifest) = PluginManifest::load_from_repo(&root) else {
        eprintln!("[patch] failed to load bundled plugin manifest at {}", root.display());
        return;
    };
    let Some(entry) = manifest.get("patch-hub-lobby") else {
        return;
    };

    if let Err(err) = deploy_plugin(
        &root,
        entry,
        PathBuf::from(&lobby.data_path).as_path(),
        &inner.dirs.plugins_cache,
        Some("lobby"),
    ) {
        eprintln!(
            "[patch] lobby plugin deploy failed for {}: {err}",
            lobby.name
        );
    }
}
