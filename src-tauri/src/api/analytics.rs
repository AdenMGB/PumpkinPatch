use super::Result;
use crate::bundled_plugins::{install_plan_plugin_on_backends, PlanPluginInstallResult};
use crate::state::AppStateHandle;
use patch_core::{
    graph_players_online, list_command_usages, list_join_addresses,
    list_kills, list_players_table, list_sessions, network_overview_v1, ping_table_v1, plan_graph,
    aggregate_network_analytics, execute_plan_query, extension_data_merged, load_cached_plan_query,
    online_overview_v1, with_plan_cache, PlanJsonCache,
    performance_overview_v1,
    plan_filters_catalog, player_profile_v1, player_versus_v1, playerbase_overview_v1, players_table_v1,
    players_v1,
    retention_v1, sessions_v1,
    server_overview_v1, sessions_overview_v1, sync_network_plan_exports, NetworkAnalyticsOverview,
    PlanCommandUsageRow, PlanFiltersCatalog, PlanOnlineOverviewV1, PlanPerformanceOverviewV1,
    PlanPlayerbaseOverviewV1, PlanQueryRequest, PlanQueryResult, PlanRetentionV1,
    PlanGraphPlayersOnline, PlanJoinAddressRow, PlanKillRow, PlanNetworkOverviewV1, PlanPingTableRow,
    PlanPlayerProfileV1, PlanPlayerTableRow, PlanPlayerVersusV1, PlanServerOverviewV1,
    PlanPlayersTableV1, PlanPlayersV1, PlanSessionRow, PlanSessionsOverviewV1, PlanSessionsV1,
    PlanSyncReport, PlanExtensionDataV1,
};
use serde_json::Value;
use uuid::Uuid;

async fn ensure_plan_exports_synced(
    inner: &patch_core::AppState,
    network_id: Uuid,
) -> Result<()> {
    let network = inner.db.get_network(network_id).await?;
    let token = inner.db.ensure_analytics_export_token(network_id).await?;
    let servers = inner.db.list_servers_for_network(network_id).await?;
    sync_network_plan_exports(&inner.db, &network, &servers, &token).await?;
    inner.plan_json_cache.clear().await;
    Ok(())
}

async fn live_players_online(
    inner: &patch_core::AppState,
    network_id: Uuid,
    server_id: Option<Uuid>,
) -> Result<u32> {
    let network = inner.db.get_network(network_id).await?;
    let token = inner.db.ensure_analytics_export_token(network_id).await?;
    let servers = inner.db.list_servers_for_network(network_id).await?;
    let overview = aggregate_network_analytics(&network, &servers, &token)?;
    if let Some(id) = server_id {
        let srv = inner.db.get_server(id).await?;
        let sid = srv.id.to_string();
        Ok(overview
            .servers
            .iter()
            .find(|s| s.server_id == sid)
            .map(|s| s.online_now)
            .unwrap_or(0))
    } else {
        Ok(overview.servers.iter().map(|s| s.online_now).sum())
    }
}

#[tauri::command]
pub async fn analytics_network_overview(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
) -> Result<NetworkAnalyticsOverview> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let token = inner.db.ensure_analytics_export_token(network_id).await?;
    let servers = inner.db.list_servers_for_network(network_id).await?;
    Ok(aggregate_network_analytics(&network, &servers, &token)?)
}

#[tauri::command]
pub async fn analytics_plan_sync(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
) -> Result<Vec<PlanSyncReport>> {
    let inner = state.get().await?;
    let network = inner.db.get_network(network_id).await?;
    let token = inner.db.ensure_analytics_export_token(network_id).await?;
    let servers = inner.db.list_servers_for_network(network_id).await?;
    let reports = sync_network_plan_exports(&inner.db, &network, &servers, &token).await?;
    inner.plan_json_cache.clear().await;
    Ok(reports)
}

#[tauri::command]
pub async fn analytics_install_plan_on_backends(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
) -> Result<Vec<PlanPluginInstallResult>> {
    let inner = state.get().await?;
    install_plan_plugin_on_backends(&state, &inner, network_id)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_players_v1(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
    limit: Option<i64>,
) -> Result<PlanPlayersV1> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    players_v1(
        inner.db.pool(),
        &network.id.to_string(),
        server_uuid.as_deref(),
        limit.unwrap_or(500),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_players_table_v1(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
    limit: Option<i64>,
) -> Result<PlanPlayersTableV1> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    players_table_v1(
        inner.db.pool(),
        &network.id.to_string(),
        server_uuid.as_deref(),
        limit.unwrap_or(500),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_sessions_v1(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
    limit: Option<i64>,
) -> Result<PlanSessionsV1> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    sessions_v1(
        inner.db.pool(),
        &network.id.to_string(),
        server_uuid.as_deref(),
        limit.unwrap_or(80),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_sessions(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
    limit: Option<i64>,
) -> Result<Vec<PlanSessionRow>> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    let pool = inner.db.pool();
    list_sessions(
        pool,
        &network.id.to_string(),
        server_uuid.as_deref(),
        limit.unwrap_or(80),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_commands(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
    limit: Option<i64>,
) -> Result<Vec<PlanCommandUsageRow>> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    let pool = inner.db.pool();
    list_command_usages(
        pool,
        &network.id.to_string(),
        server_uuid.as_deref(),
        limit.unwrap_or(50),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_players_table(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
    limit: Option<i64>,
) -> Result<Vec<PlanPlayerTableRow>> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    let pool = inner.db.pool();
    list_players_table(
        pool,
        &network.id.to_string(),
        server_uuid.as_deref(),
        limit.unwrap_or(200),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_graph_players_online(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
    bypass_cache: Option<bool>,
) -> Result<PlanGraphPlayersOnline> {
    let bypass = bypass_cache.unwrap_or(false);
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    let net_id = network.id.to_string();
    let pool = inner.db.pool();
    let cache = inner.plan_json_cache.clone();
    let key = PlanJsonCache::cache_key("playersOnline", &net_id, server_uuid.as_deref(), "");
    with_plan_cache(&cache, key, bypass, || async {
        graph_players_online(pool, &net_id, server_uuid.as_deref()).await
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_server_overview(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Uuid,
) -> Result<PlanServerOverviewV1> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let token = inner.db.ensure_analytics_export_token(network_id).await?;
    let server = inner.db.get_server(server_id).await?;
    let servers = inner.db.list_servers_for_network(network_id).await?;
    let overview = aggregate_network_analytics(&network, &servers, &token)?;
    let live_online = overview
        .servers
        .iter()
        .find(|s| s.server_id == server.id.to_string())
        .map(|s| s.online_now)
        .unwrap_or(0);
    let pool = inner.db.pool();
    server_overview_v1(
        pool,
        &network.id.to_string(),
        &server.id.to_string(),
        live_online,
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_kills(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
    limit: Option<i64>,
) -> Result<Vec<PlanKillRow>> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    list_kills(
        inner.db.pool(),
        &network.id.to_string(),
        server_uuid.as_deref(),
        limit.unwrap_or(60),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_player_versus(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
) -> Result<PlanPlayerVersusV1> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    player_versus_v1(
        inner.db.pool(),
        &network.id.to_string(),
        server_uuid.as_deref(),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_network_overview(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
) -> Result<PlanNetworkOverviewV1> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let token = inner.db.ensure_analytics_export_token(network_id).await?;
    let servers = inner.db.list_servers_for_network(network_id).await?;
    let overview = aggregate_network_analytics(&network, &servers, &token)?;
    let triples: Vec<(String, String, u32)> = overview
        .servers
        .iter()
        .map(|s| (s.server_id.clone(), s.server_name.clone(), s.online_now))
        .collect();
    network_overview_v1(inner.db.pool(), &network.id.to_string(), &triples)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_player(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    player_uuid: String,
) -> Result<PlanPlayerProfileV1> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    player_profile_v1(inner.db.pool(), &network.id.to_string(), &player_uuid)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_graph(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    graph_type: String,
    server_id: Option<Uuid>,
    bypass_cache: Option<bool>,
) -> Result<Value> {
    let bypass = bypass_cache.unwrap_or(false);
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    let net_id = network.id.to_string();
    let pool = inner.db.pool();
    let cache = inner.plan_json_cache.clone();
    let gt = graph_type.clone();
    let key = PlanJsonCache::cache_key("graph", &net_id, server_uuid.as_deref(), &gt);
    with_plan_cache(&cache, key, bypass, || async {
        plan_graph(pool, &net_id, server_uuid.as_deref(), &gt).await
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_sessions_overview(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
) -> Result<PlanSessionsOverviewV1> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    sessions_overview_v1(
        inner.db.pool(),
        &network.id.to_string(),
        server_uuid.as_deref(),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_filters(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
) -> Result<PlanFiltersCatalog> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    plan_filters_catalog(
        inner.db.pool(),
        &network.id.to_string(),
        server_uuid.as_deref(),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_query(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
    request: PlanQueryRequest,
) -> Result<PlanQueryResult> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    execute_plan_query(
        inner.db.pool(),
        &network.id.to_string(),
        server_uuid.as_deref(),
        &request,
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_query_by_token(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    token: String,
) -> Result<PlanQueryResult> {
    let inner = state.get().await?;
    let _network = inner.db.get_network(network_id).await?;
    load_cached_plan_query(inner.db.pool(), &token)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_join_addresses(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
) -> Result<Vec<PlanJoinAddressRow>> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    list_join_addresses(
        inner.db.pool(),
        &network.id.to_string(),
        server_uuid.as_deref(),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_online_overview(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
    bypass_cache: Option<bool>,
) -> Result<PlanOnlineOverviewV1> {
    let bypass = bypass_cache.unwrap_or(false);
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    let net_id = network.id.to_string();
    let pool = inner.db.pool();
    let cache = inner.plan_json_cache.clone();
    let key = PlanJsonCache::cache_key("onlineOverview", &net_id, server_uuid.as_deref(), "");
    with_plan_cache(&cache, key, bypass, || async {
        let online = live_players_online(&inner, network_id, server_id)
            .await
            .map_err(|e| patch_core::Error::Other(e.to_string()))?;
        online_overview_v1(pool, &net_id, server_uuid.as_deref(), online).await
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_playerbase_overview(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
    bypass_cache: Option<bool>,
) -> Result<PlanPlayerbaseOverviewV1> {
    let bypass = bypass_cache.unwrap_or(false);
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    let net_id = network.id.to_string();
    let pool = inner.db.pool();
    let cache = inner.plan_json_cache.clone();
    let key = PlanJsonCache::cache_key("playerbaseOverview", &net_id, server_uuid.as_deref(), "");
    with_plan_cache(&cache, key, bypass, || async {
        playerbase_overview_v1(pool, &net_id, server_uuid.as_deref()).await
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_performance_overview(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
    bypass_cache: Option<bool>,
) -> Result<PlanPerformanceOverviewV1> {
    let bypass = bypass_cache.unwrap_or(false);
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    let net_id = network.id.to_string();
    let pool = inner.db.pool();
    let cache = inner.plan_json_cache.clone();
    let key = PlanJsonCache::cache_key("performanceOverview", &net_id, server_uuid.as_deref(), "");
    with_plan_cache(&cache, key, bypass, || async {
        performance_overview_v1(pool, &net_id, server_uuid.as_deref()).await
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_ping_table(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
) -> Result<Vec<PlanPingTableRow>> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    ping_table_v1(
        inner.db.pool(),
        &network.id.to_string(),
        server_uuid.as_deref(),
    )
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_plan_retention(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Option<Uuid>,
    limit: Option<i64>,
) -> Result<PlanRetentionV1> {
    let inner = state.get().await?;
    ensure_plan_exports_synced(&inner, network_id).await?;
    let network = inner.db.get_network(network_id).await?;
    let server_uuid = if let Some(id) = server_id {
        Some(inner.db.get_server(id).await?.id.to_string())
    } else {
        None
    };
    retention_v1(
        inner.db.pool(),
        &network.id.to_string(),
        server_uuid.as_deref(),
        limit.unwrap_or(120),
    )
    .await
    .map_err(Into::into)
}

/// Plan `GET /v1/network/playerbaseOverview` (all servers in network).
#[tauri::command]
pub async fn analytics_plan_network_playerbase_overview(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    bypass_cache: Option<bool>,
) -> Result<PlanPlayerbaseOverviewV1> {
    analytics_plan_playerbase_overview(state, network_id, None, bypass_cache).await
}

/// Plan `GET /v1/network/sessionsOverview`.
#[tauri::command]
pub async fn analytics_plan_network_sessions_overview(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
) -> Result<PlanSessionsOverviewV1> {
    analytics_plan_sessions_overview(state, network_id, None).await
}

/// Plan `GET /v1/network/pingTable`.
#[tauri::command]
pub async fn analytics_plan_network_ping_table(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
) -> Result<Vec<PlanPingTableRow>> {
    analytics_plan_ping_table(state, network_id, None).await
}

#[tauri::command]
pub async fn analytics_plan_extension_data(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    _server_id: Option<Uuid>,
) -> Result<PlanExtensionDataV1> {
    let inner = state.get().await?;
    extension_data_merged(inner.db.pool(), &network_id.to_string()).await.map_err(Into::into)
}
