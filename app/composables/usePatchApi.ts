import { invoke } from '@tauri-apps/api/core'
import type {
  AddBackendRequest,
  AppSettings,
  CreateNetworkRequest,
  InstalledPlugin,
  JoinInfo,
  LogLine,
  MinecraftVersionInfo,
  ModrinthSearchResult,
  ModrinthVersion,
  PumpkinMarketListResult,
  PumpkinMarketPluginDetail,
  NetworkAnalyticsOverview,
  NetworkTopology,
  RouteWireSpec,
  PlanCommandUsageRow,
  PlanGraphPlayersOnline,
  PlanKillRow,
  PlanPlayerProfileV1,
  PlanPlayerTableRow,
  PlanPlayersTableV1,
  PlanPlayersV1,
  PlanSessionsV1,
  PlanPlayerVersusV1,
  PlanSessionsOverviewV1,
  PlanJoinAddressRow,
  PlanPingTableRow,
  PlanFiltersCatalog,
  PlanQueryRequest,
  PlanQueryResult,
  PlanOnlineOverviewV1,
  PlanPlayerbaseOverviewV1,
  PlanPerformanceOverviewV1,
  PlanRetentionV1,
  PlanServerOverviewV1,
  PlanSessionRow,
  PlanPluginInstallResult,
  NetworkSummary,
  PingResult,
  ServerRecord,
  ServerSettings,
  UpdateHubSettingsRequest,
  UpdateServerSettingsRequest,
  HubSettings,
  DownloadProgress,
  PluginManifestEntry,
  JavaDetectResult,
} from '~/types/patch'

export function usePatchApi() {
  return {
    initialize: () => invoke<void>('initialize_state'),
    networkList: () => invoke<NetworkSummary[]>('network_list'),
    networkGet: (id: string) => invoke<NetworkSummary>('network_get', { id }),
    networkCreate: (request: CreateNetworkRequest) =>
      invoke<NetworkSummary>('network_create', { request }),
    networkDelete: (id: string) => invoke<void>('network_delete', { id }),
    networkStart: (id: string) => invoke<void>('network_start', { id }),
    networkStop: (id: string) => invoke<void>('network_stop', { id }),
    networkJoinInfo: (id: string) => invoke<JoinInfo>('network_get_join_info', { id }),
    networkPingHub: (id: string) => invoke<PingResult>('network_ping_hub', { id }),
    networkConsoleSend: (networkId: string, request: { target: string; command: string }) =>
      invoke<{ output: string }>('network_console_send', { networkId, request }),
    networkLogsGet: (networkId: string, limit = 500) =>
      invoke<LogLine[]>('network_logs_get', { networkId, limit }),
    pluginListAvailable: () => invoke<PluginManifestEntry[]>('plugin_list_available'),
    pluginDeploy: (pluginId: string, instancePath: string, serverRole?: string) =>
      invoke<string>('plugin_deploy', {
        pluginId,
        instancePath,
        serverRole: serverRole ?? null,
      }),
    settingsGet: () => invoke<AppSettings>('settings_get'),
    settingsSet: (settings: AppSettings) => invoke<void>('settings_set', { settings }),
    settingsDetectJava: () => invoke<JavaDetectResult>('settings_detect_java'),
    serverPing: (host: string, port: number) => invoke<PingResult>('server_ping', { host, port }),
    hubGetSettings: (networkId: string) =>
      invoke<HubSettings>('hub_get_settings', { networkId }),
    hubUpdateSettings: (networkId: string, patch: UpdateHubSettingsRequest) =>
      invoke<HubSettings>('hub_update_settings', { networkId, patch }),
    serverGet: (id: string) => invoke<ServerRecord>('server_get', { id }),
    serverGetSettings: (id: string) => invoke<ServerSettings>('server_get_settings', { id }),
    serverUpdateSettings: (id: string, patch: UpdateServerSettingsRequest) =>
      invoke<ServerSettings>('server_update_settings', { id, patch }),
    serverSetMinecraftVersion: (id: string, minecraftVersion: string) =>
      invoke<ServerRecord>('server_set_minecraft_version', { id, minecraftVersion }),
    serverSetPumpkinChannel: (id: string, pumpkinChannel: string) =>
      invoke<ServerRecord>('server_set_pumpkin_channel', { id, pumpkinChannel }),
    serverAddBackend: (networkId: string, request: AddBackendRequest) =>
      invoke<ServerRecord>('server_add_backend', { networkId, request }),
    serverRemove: (networkId: string, serverId: string) =>
      invoke<void>('server_remove', { networkId, serverId }),
    serverListPlugins: (serverId: string) =>
      invoke<InstalledPlugin[]>('server_list_plugins', { serverId }),
    serverRemovePlugin: (serverId: string, filename: string) =>
      invoke<void>('server_remove_plugin', { serverId, filename }),
    minecraftListVersions: () => invoke<MinecraftVersionInfo[]>('minecraft_list_versions'),
    modrinthSearch: (query: string, minecraftVersion?: string, limit = 20, offset = 0) =>
      invoke<ModrinthSearchResult>('modrinth_search_plugins', {
        query,
        minecraftVersion: minecraftVersion ?? null,
        limit,
        offset,
      }),
    modrinthProjectVersions: (projectId: string, minecraftVersion?: string) =>
      invoke<ModrinthVersion[]>('modrinth_project_versions', {
        projectId,
        minecraftVersion: minecraftVersion ?? null,
      }),
    modrinthInstall: (projectId: string, versionId: string, instancePath: string) =>
      invoke<string>('modrinth_install_plugin', { projectId, versionId, instancePath }),
    pumpkinMarketList: (search?: string, category?: string, limit = 12, cursor?: string) =>
      invoke<PumpkinMarketListResult>('pumpkin_market_list_plugins', {
        search: search?.trim() ? search.trim() : null,
        category: category && category !== 'all' ? category : null,
        limit,
        cursor: cursor ?? null,
      }),
    pumpkinMarketGet: (pluginId: number) =>
      invoke<PumpkinMarketPluginDetail>('pumpkin_market_get_plugin', { pluginId }),
    pumpkinMarketInstall: (pluginId: number, instancePath: string) =>
      invoke<string>('pumpkin_market_install_plugin', { pluginId, instancePath }),
    analyticsNetworkOverview: (networkId: string) =>
      invoke<NetworkAnalyticsOverview>('analytics_network_overview', { networkId }),
    analyticsInstallPlanOnBackends: (networkId: string) =>
      invoke<PlanPluginInstallResult[]>('analytics_install_plan_on_backends', { networkId }),
    analyticsPlanSessions: (networkId: string, serverId?: string, limit = 80) =>
      invoke<PlanSessionRow[]>('analytics_plan_sessions', {
        networkId,
        serverId: serverId ?? null,
        limit,
      }),
    analyticsPlanCommands: (networkId: string, serverId?: string, limit = 50) =>
      invoke<PlanCommandUsageRow[]>('analytics_plan_commands', {
        networkId,
        serverId: serverId ?? null,
        limit,
      }),
    analyticsPlanPlayersTable: (networkId: string, serverId?: string, limit = 200) =>
      invoke<PlanPlayerTableRow[]>('analytics_plan_players_table', {
        networkId,
        serverId: serverId ?? null,
        limit,
      }),
    analyticsPlanPlayersV1: (networkId: string, serverId?: string, limit = 500) =>
      invoke<PlanPlayersV1>('analytics_plan_players_v1', {
        networkId,
        serverId: serverId ?? null,
        limit,
      }),
    analyticsPlanPlayersTableV1: (networkId: string, serverId?: string, limit = 500) =>
      invoke<PlanPlayersTableV1>('analytics_plan_players_table_v1', {
        networkId,
        serverId: serverId ?? null,
        limit,
      }),
    analyticsPlanSessionsV1: (networkId: string, serverId?: string, limit = 80) =>
      invoke<PlanSessionsV1>('analytics_plan_sessions_v1', {
        networkId,
        serverId: serverId ?? null,
        limit,
      }),
    analyticsPlanGraphPlayersOnline: (networkId: string, serverId?: string) =>
      invoke<PlanGraphPlayersOnline>('analytics_plan_graph_players_online', {
        networkId,
        serverId: serverId ?? null,
      }),
    analyticsPlanServerOverview: (networkId: string, serverId: string) =>
      invoke<PlanServerOverviewV1>('analytics_plan_server_overview', { networkId, serverId }),
    analyticsPlanKills: (networkId: string, serverId?: string, limit = 60) =>
      invoke<PlanKillRow[]>('analytics_plan_kills', {
        networkId,
        serverId: serverId ?? null,
        limit,
      }),
    analyticsPlanPlayerVersus: (networkId: string, serverId?: string) =>
      invoke<PlanPlayerVersusV1>('analytics_plan_player_versus', {
        networkId,
        serverId: serverId ?? null,
      }),
    analyticsPlanPlayer: (networkId: string, playerUuid: string) =>
      invoke<PlanPlayerProfileV1>('analytics_plan_player', { networkId, playerUuid }),
    analyticsPlanGraph: (networkId: string, graphType: string, serverId?: string) =>
      invoke<Record<string, unknown>>('analytics_plan_graph', {
        networkId,
        graphType,
        serverId: serverId ?? null,
      }),
    analyticsPlanSessionsOverview: (networkId: string, serverId?: string) =>
      invoke<PlanSessionsOverviewV1>('analytics_plan_sessions_overview', {
        networkId,
        serverId: serverId ?? null,
      }),
    analyticsPlanJoinAddresses: (networkId: string, serverId?: string) =>
      invoke<PlanJoinAddressRow[]>('analytics_plan_join_addresses', {
        networkId,
        serverId: serverId ?? null,
      }),
    analyticsPlanFilters: (networkId: string, serverId?: string) =>
      invoke<PlanFiltersCatalog>('analytics_plan_filters', {
        networkId,
        serverId: serverId ?? null,
      }),
    analyticsPlanQuery: (networkId: string, request: PlanQueryRequest, serverId?: string) =>
      invoke<PlanQueryResult>('analytics_plan_query', {
        networkId,
        serverId: serverId ?? null,
        request,
      }),
    analyticsPlanQueryByToken: (networkId: string, token: string) =>
      invoke<PlanQueryResult>('analytics_plan_query_by_token', { networkId, token }),
    analyticsPlanOnlineOverview: (networkId: string, serverId?: string) =>
      invoke<PlanOnlineOverviewV1>('analytics_plan_online_overview', {
        networkId,
        serverId: serverId ?? null,
      }),
    analyticsPlanPlayerbaseOverview: (networkId: string, serverId?: string) =>
      invoke<PlanPlayerbaseOverviewV1>('analytics_plan_playerbase_overview', {
        networkId,
        serverId: serverId ?? null,
      }),
    analyticsPlanPerformanceOverview: (networkId: string, serverId?: string) =>
      invoke<PlanPerformanceOverviewV1>('analytics_plan_performance_overview', {
        networkId,
        serverId: serverId ?? null,
      }),
    analyticsPlanRetention: (networkId: string, serverId?: string, limit = 120) =>
      invoke<PlanRetentionV1>('analytics_plan_retention', {
        networkId,
        serverId: serverId ?? null,
        limit,
      }),
    analyticsPlanPingTable: (networkId: string, serverId?: string) =>
      invoke<PlanPingTableRow[]>('analytics_plan_ping_table', {
        networkId,
        serverId: serverId ?? null,
      }),
    analyticsPlanNetworkPlayerbaseOverview: (networkId: string, bypassCache?: boolean) =>
      invoke<PlanPlayerbaseOverviewV1>('analytics_plan_network_playerbase_overview', {
        networkId,
        bypassCache: bypassCache ?? null,
      }),
    analyticsPlanNetworkSessionsOverview: (networkId: string) =>
      invoke<PlanSessionsOverviewV1>('analytics_plan_network_sessions_overview', { networkId }),
    analyticsPlanNetworkPingTable: (networkId: string) =>
      invoke<PlanPingTableRow[]>('analytics_plan_network_ping_table', { networkId }),
    networkTopologyGet: (networkId: string) =>
      invoke<NetworkTopology>('network_topology_get', { networkId }),
    networkHotWire: (networkId: string, spec: RouteWireSpec) =>
      invoke<string>('network_hot_wire_server_to_proxy', { networkId, spec }),
    gourdPoolScaleRecommend: (req: import('~/types/patch').GourdPoolScaleRecommendRequest) =>
      invoke<number>('gourd_pool_scale_recommend', { req }),
    gourdPoolPromote: (networkId: string, req: import('~/types/patch').GourdPoolPromoteRequest) =>
      invoke<string[]>('gourd_pool_promote', { networkId, req }),
    gourdScarecrowStats: (networkId: string) =>
      invoke<import('~/types/patch').GourdScarecrowStats | null>('gourd_scarecrow_stats', {
        networkId,
      }),
    planHttpStart: (networkId: string) =>
      invoke<string>('plan_http_start', { networkId }),
    planAuthRegisterStart: (networkId: string, username: string, password: string) =>
      invoke<import('~/types/patch').PlanRegisterStartResponse>('plan_auth_register_start', {
        networkId,
        username,
        password,
      }),
    planWebUsersList: () => invoke<import('~/types/patch').PlanWebUser[]>('plan_web_users_list'),
    planWebUserSetGroup: (username: string, groupName: string) =>
      invoke<void>('plan_web_user_set_group', { username, groupName }),
    planWebUserLogout: (username: string) =>
      invoke<number>('plan_web_user_logout', { username }),
    planWebUserDelete: (username: string) => invoke<void>('plan_web_user_delete', { username }),
    analyticsPlanExportStatic: (networkId: string, serverId: string) =>
      invoke<string>('analytics_plan_export_static', { networkId, serverId }),
    analyticsHarvestSync: (networkId: string) =>
      invoke<number>('analytics_harvest_sync', { networkId }),
  }
}
