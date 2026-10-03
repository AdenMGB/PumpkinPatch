use crate::state::AppStateHandle;
use patch_core::{
    deploy_auto_plugins_for_server, write_patch_plan_config, NetworkRecord, NetworkSummary,
    PluginManifest, ServerRecord,
};
use std::path::PathBuf;

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
    if let Err(err) =
        provision_server_inner(state, inner, network, server).await
    {
        eprintln!(
            "[patch] plugin provisioning failed for {}: {err}",
            server.name
        );
    }
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
        provision_server_with_token(state, inner, &summary.network, server, &token)?;
    }
    Ok(())
}

async fn provision_server_inner(
    state: &AppStateHandle,
    inner: &patch_core::AppState,
    network: &NetworkRecord,
    server: &ServerRecord,
) -> patch_core::Result<()> {
    let token = inner.db.ensure_analytics_export_token(network.id).await?;
    provision_server_with_token(state, inner, network, server, &token)
}

fn provision_server_with_token(
    state: &AppStateHandle,
    inner: &patch_core::AppState,
    network: &NetworkRecord,
    server: &ServerRecord,
    token: &str,
) -> patch_core::Result<()> {
    let root = state.plugins_root();
    let manifest = PluginManifest::load_from_repo(&root)?;
    let instance = PathBuf::from(&server.data_path);
    deploy_auto_plugins_for_server(
        &root,
        &manifest,
        server,
        instance.as_path(),
        &inner.dirs.plugins_cache,
    )?;
    write_patch_plan_config(instance.as_path(), server, network.id, token)?;
    Ok(())
}
