use super::Result;
use crate::state::AppStateHandle;
use patch_core::{
    build_topology, default_export_dir, export_plan_static_bundle, hot_wire_velocity_toml,
    sync_harvest_from_plan_sqlite, NetworkTopology,
};
use serde::Deserialize;
use uuid::Uuid;
use vineyard_protocol::RouteWireSpec;

#[derive(Deserialize)]
pub struct GourdPoolScaleRecommendRequest {
    pub current_ccu: u32,
    pub active_backends: u32,
    pub players_per_backend: u32,
    pub hidden_ready: u32,
}

#[tauri::command]
pub fn gourd_pool_scale_recommend(req: GourdPoolScaleRecommendRequest) -> u32 {
    patch_core::gourd_pool_scale_recommend(
        req.current_ccu,
        req.active_backends,
        req.players_per_backend,
        req.hidden_ready,
    )
}

#[tauri::command]
pub async fn network_topology_get(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
) -> Result<NetworkTopology> {
    let inner = state.get().await?;
    let network = inner.db.get_network(network_id).await?;
    let servers = inner.db.list_servers_for_network(network_id).await?;
    Ok(build_topology(&network, &servers))
}

#[tauri::command]
pub async fn network_hot_wire_server_to_proxy(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    spec: RouteWireSpec,
) -> Result<String> {
    let inner = state.get().await?;
    let settings = inner.settings().await;
    let servers = inner.db.list_servers_for_network(network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let msg = hot_wire_velocity_toml(
        &inner.networks,
        &network,
        &servers,
        &settings.bind_host,
        &spec,
    )?;
    inner.velocity_bridge.apply(spec.clone()).await;
    {
        let processes = inner.processes.read().await;
        match processes
            .apply_gourd_route(network_id, &spec, &servers)
            .await
        {
            Ok(()) => {}
            Err(patch_core::Error::InvalidState(msg))
                if msg == "network hub is not Gourd" => {}
            Err(e) => return Err(e.into()),
        }
    }
    inner
        .vineyard_bus
        .publish_route_mutation(spec.clone())
        .await
        .map_err(|e| patch_core::Error::InvalidState(e.to_string()))?;
    Ok(msg)
}

#[tauri::command]
pub async fn analytics_plan_export_static(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Uuid,
) -> Result<String> {
    let inner = state.get().await?;
    let network = inner.db.get_network(network_id).await?;
    let server = inner.db.get_server(server_id).await?;
    let token = inner.db.ensure_analytics_export_token(network_id).await?;
    let servers = inner.db.list_servers_for_network(network_id).await?;
    let overview = patch_core::aggregate_network_analytics(&network, &servers, &token)?;
    let live_online = overview
        .servers
        .iter()
        .find(|s| s.server_id == server.id.to_string())
        .map(|s| s.online_now)
        .unwrap_or(0);
    let triples: Vec<(String, String, u32)> = overview
        .servers
        .iter()
        .map(|s| (s.server_id.clone(), s.server_name.clone(), s.online_now))
        .collect();
    let dir = default_export_dir(std::path::Path::new(&network.data_path));
    export_plan_static_bundle(
        inner.db.pool(),
        &network.id.to_string(),
        &server.id.to_string(),
        &dir,
        live_online,
        &triples,
    )
    .await?;
    Ok(dir.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn analytics_harvest_sync(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
) -> Result<i64> {
    let inner = state.get().await?;
    let settings = inner.settings().await;
    if !settings.harvest_duckdb_enabled {
        return Err(patch_core::Error::InvalidState("HarvestMetrics DuckDB disabled".into()).into());
    }
    let network = inner.db.get_network(network_id).await?;
    let duck_path = inner
        .dirs
        .cache
        .join("harvest")
        .join(format!("{}.harvest.json", network.id));
    sync_harvest_from_plan_sqlite(inner.db.pool(), &network.id.to_string(), &duck_path).await
}
