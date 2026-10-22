use crate::state::AppStateHandle;
use patch_core::{
    deploy_auto_plugins_for_server, deploy_plugin, ensure_hub_lobby_config_for_lobby,
    write_patch_plan_config, NetworkRecord, NetworkSummary, PluginManifest, ServerRecord,
};
use serde::Serialize;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct PlanPluginInstallResult {
    pub server_id: Uuid,
    pub server_name: String,
    pub success: bool,
    pub detail: String,
}

pub async fn refresh_plan_configs_all_networks(
    state: &AppStateHandle,
    inner: &patch_core::AppState,
) {
    let settings = inner.settings().await;
    if let Ok(networks) = inner.db.list_networks().await {
        for network in networks {
            let join_address = format!("{}:{}", settings.bind_host, network.hub_port);
            let servers = inner
                .db
                .list_servers_for_network(network.id)
                .await
                .unwrap_or_default();
            let summary = NetworkSummary {
                network,
                servers,
                status: patch_core::NetworkRuntimeStatus::Stopped,
                join_address,
            };
            provision_network_plugins(state, inner, &summary).await;
        }
    }
}

pub async fn provision_network_plugins(
    state: &AppStateHandle,
    inner: &patch_core::AppState,
    summary: &NetworkSummary,
) {
    if let Err(err) = provision_network_plugins_inner(state, inner, summary).await {
        eprintln!(
            "[patch] network plugin provisioning failed for {}: {err}",
            summary.network.name
        );
    }
}

pub async fn provision_server_plugin(
    state: &AppStateHandle,
    inner: &patch_core::AppState,
    network: &NetworkRecord,
    server: &ServerRecord,
) {
    if let Err(err) = provision_server_inner(state, inner, network, server).await {
        eprintln!(
            "[patch] plugin provisioning failed for {}: {err}",
            server.name
        );
    }
}

/// Deploys Patch Plan (and other auto-deploy manifest plugins for that role) on every backend.
pub async fn install_plan_plugin_on_backends(
    state: &AppStateHandle,
    inner: &patch_core::AppState,
    network_id: Uuid,
) -> patch_core::Result<Vec<PlanPluginInstallResult>> {
    let network = inner.db.get_network(network_id).await?;
    let servers = inner.db.list_servers_for_network(network_id).await?;
    let mut results = Vec::new();
    for server in servers.iter().filter(|s| s.role == "backend") {
        let (success, detail) = match provision_server_inner(state, inner, &network, server).await {
            Ok(()) => (
                true,
                "Patch Plan WASM copied and export config refreshed".to_string(),
            ),
            Err(err) => (false, err.to_string()),
        };
        results.push(PlanPluginInstallResult {
            server_id: server.id,
            server_name: server.name.clone(),
            success,
            detail,
        });
    }
    Ok(results)
}

async fn provision_network_plugins_inner(
    state: &AppStateHandle,
    inner: &patch_core::AppState,
    summary: &NetworkSummary,
) -> patch_core::Result<()> {
    let token = inner
        .db
        .ensure_analytics_export_token(summary.network.id)
        .await?;
    for server in &summary.servers {
        provision_server_with_token(
            state,
            inner,
            &summary.network,
            server,
            &summary.servers,
            &token,
            &summary.join_address,
        )
        .await?;
    }
    sync_lobby_hub_list(&summary.servers)?;
    Ok(())
}

async fn provision_server_inner(
    state: &AppStateHandle,
    inner: &patch_core::AppState,
    network: &NetworkRecord,
    server: &ServerRecord,
) -> patch_core::Result<()> {
    let token = inner.db.ensure_analytics_export_token(network.id).await?;
    let settings = inner.settings().await;
    let join_address = format!("{}:{}", settings.bind_host, network.hub_port);
    let all = inner.db.list_servers_for_network(network.id).await?;
    provision_server_with_token(state, inner, network, server, &all, &token, &join_address).await?;
    sync_lobby_hub_list(&all)?;
    Ok(())
}

fn sync_lobby_hub_list(servers: &[ServerRecord]) -> patch_core::Result<()> {
    if let Some(lobby) = servers.iter().find(|s| s.role == "lobby") {
        ensure_hub_lobby_config_for_lobby(
            std::path::Path::new(&lobby.data_path),
            servers,
        )?;
    }
    Ok(())
}

async fn provision_server_with_token(
    state: &AppStateHandle,
    inner: &patch_core::AppState,
    network: &NetworkRecord,
    server: &ServerRecord,
    _all_servers: &[ServerRecord],
    token: &str,
    join_address: &str,
) -> patch_core::Result<()> {
    let root = state.plugins_root();
    let manifest = PluginManifest::load_from_repo(&root)?;
    let instance = PathBuf::from(&server.data_path);
    let deployed = deploy_auto_plugins_for_server(
        &root,
        &manifest,
        server,
        instance.as_path(),
        &inner.dirs.plugins_cache,
    )?;
    for path in deployed {
        inner
            .db
            .record_plugin_deployment("auto", &server.data_path, &path.to_string_lossy())
            .await?;
    }
    write_patch_plan_config(instance.as_path(), server, network.id, token, join_address)?;
    Ok(())
}

pub async fn deploy_manifest_plugin(
    state: &AppStateHandle,
    inner: &patch_core::AppState,
    plugin_id: &str,
    instance_path: &str,
    server_role: Option<&str>,
) -> patch_core::Result<String> {
    let root = state.plugins_root();
    let manifest = PluginManifest::load_from_repo(&root)?;
    let entry = manifest
        .get(plugin_id)
        .ok_or_else(|| patch_core::Error::NotFound(plugin_id.to_string()))?;
    let dest = deploy_plugin(
        &root,
        entry,
        PathBuf::from(instance_path).as_path(),
        &inner.dirs.plugins_cache,
        server_role,
    )?;
    inner
        .db
        .record_plugin_deployment(plugin_id, instance_path, &dest.to_string_lossy())
        .await?;
    Ok(dest.to_string_lossy().into())
}
