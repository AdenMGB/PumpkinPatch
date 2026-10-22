export type NetworkRuntimeStatus = 'stopped' | 'starting' | 'running' | 'stopping' | 'error'

export interface NetworkRecord {
  id: string
  name: string
  hub_type: string
  hub_port: number
  forwarding_secret: string
  lobby_server_id: string | null
  data_path: string
  created_at: string
}

export interface ServerRecord {
  id: string
  network_id: string
  role: string
  name: string
  velocity_name: string
  game_port: number
  data_path: string
  pumpkin_channel: string
  minecraft_version: string
  created_at: string
}

export interface NetworkSummary {
  network: NetworkRecord
  servers: ServerRecord[]
  status: NetworkRuntimeStatus
  join_address: string
}

export interface CreateNetworkRequest {
  name: string
  hub_port?: number
  hub_type?: 'velocity'
  backend_names: string[]
  pumpkin_channel?: string
  minecraft_version?: string
}

export interface AppSettings {
  java_path: string
  bind_host: string
  pumpkin_channel_default: string
  plan_http_enabled: boolean
  plan_http_port: number
  harvest_duckdb_enabled: boolean
  gourd_enabled: boolean
}

export interface TopologyNode {
  id: string
  kind: string
  label: string
  port?: number | null
}

export interface TopologyEdge {
  proxy_id: string
  backend_server_id: string
  server_alias: string
  forced_hosts: string[]
}

export interface NetworkTopology {
  network_id: string
  nodes: TopologyNode[]
  edges: TopologyEdge[]
}

export interface RouteWireSpec {
  proxy_id: string
  backend_server_id: string
  server_alias: string
  forced_hosts: string[]
  priority_tier: number
  permission_gate?: string | null
}

export interface PingResult {
  online: boolean
  latency_ms: number | null
  version: string | null
  motd: string | null
  players_online: number | null
  players_max: number | null
}

export interface JoinInfo {
  host: string
  port: number
  address: string
}

export interface LogLine {
  network_id: string
  source: string
  line: string
}

export interface DownloadProgress {
  downloaded: number
  total: number | null
  label: string
}

export interface PluginManifestEntry {
  id: string
  name: string
  version: string
  description: string
  artifact: string
  targets: string[]
  auto_deploy: boolean
}

export interface JavaDetectResult {
  resolved_path: string | null
  candidates: string[]
}

export interface ServerSettings {
  bind_address: string
  online_mode: boolean
  max_players: number
  view_distance: number
  simulation_distance: number
  motd: string
  commands_enabled: boolean
}

export interface UpdateServerSettingsRequest {
  bind_address?: string
  online_mode?: boolean
  max_players?: number
  view_distance?: number
  simulation_distance?: number
  motd?: string
  commands_enabled?: boolean
}

export interface MinecraftVersionInfo {
  version: string
  stable: boolean
}

export interface ModrinthSearchHit {
  project_id: string
  slug: string
  title: string
  description: string
  icon_url: string | null
  downloads: number
  author: string
}

export interface ModrinthSearchResult {
  hits: ModrinthSearchHit[]
  total_hits: number
}

export interface PumpkinMarketPluginSummary {
  id: number
  name: string
  version: string
  category: string
  dev_name: string
  downloads: number
  preview_path: string | null
  price_cents: number
  type: string
  is_early_access: boolean
  description: string
  screenshots: string[]
  status: string | null
}

export interface PumpkinMarketListResult {
  items: PumpkinMarketPluginSummary[]
  has_more: boolean
  next_cursor: string | null
}

export interface PumpkinMarketPluginVersion {
  id: number
  version: string
  track: string | null
  release_notes: string | null
  created_at: string | null
}

export interface PumpkinMarketPluginCommand {
  name: string
  description: string
  permission: string | null
}

export interface PumpkinMarketPluginDetail {
  id: number
  name: string
  version: string
  category: string
  dev_name: string
  downloads: number
  preview_path: string | null
  price_cents: number
  type: string
  wasm_path: string | null
  file_size: number | null
  description: string
  owned: boolean | null
  keywords: string | null
  source_link: string | null
  youtube_video_url: string | null
  status: string | null
  is_early_access: boolean
  screenshots: string[]
  versions: PumpkinMarketPluginVersion[]
  commands: PumpkinMarketPluginCommand[]
}

export interface ModrinthVersion {
  id: string
  version_number: string
  name: string
  game_versions: string[]
  date_published: string
  files: { filename: string; url: string; primary: boolean }[]
}

export interface InstalledPlugin {
  filename: string
  path: string
  size_bytes: number
}

export interface StatisticRow {
  id: string
  total: number
}

export interface ServerTotalsView {
  total_logins: number
  total_kicks: number
  total_deaths: number
  total_advancements: number
  total_mob_kills: number
  total_player_kills: number
  total_recipes_discovered: number
  total_blocks_harvested: number
  total_chat_messages: number
  total_commands: number
  top_statistics: StatisticRow[]
}

export interface PlayerDetailRow {
  uuid: string
  name: string
  join_count: number
  login_count: number
  playtime_secs: number
  deaths: number
  mob_kills: number
  player_kills: number
  advancements: number
  recipes_discovered: number
  blocks_harvested: number
  chat_messages: number
  commands_used: number
  current_gamemode: string
  last_world: string
  xp_level: number
  top_statistics: StatisticRow[]
  recent_advancements: string[]
  last_seen_unix: number
  afk_time_secs: number
  join_address: string
  activity_index: number
  activity_group: string
}

export interface DeathRow {
  t: number
  player_uuid: string
  player_name: string
  message: string
  server_name: string
}

export interface EventRow {
  t: number
  kind: string
  player_uuid: string
  player_name: string
  detail: string
  server_name: string
}

export interface ActivityPoint {
  t: number
  online: number
}

export interface ServerAnalyticsSnapshot {
  server_id: string
  server_name: string
  generated_at_unix: number
  online_now: number
  peak_online: number
  total_joins: number
  unique_players: number
  total_playtime_secs: number
  server_totals: ServerTotalsView
  top_players: PlayerDetailRow[]
  recent_deaths: DeathRow[]
  recent_events: EventRow[]
  activity_samples: ActivityPoint[]
  stale: boolean
  export_status: 'ok' | 'stale' | 'missing' | 'invalid' | string
  export_detail?: string | null
}

export interface PlanSessionGmSeries {
  survival: number
  creative: number
  adventure: number
  spectator: number
}

export interface PlanSessionWorldSlice {
  name: string
  y: number
}

export interface PlanSessionRow {
  player_name: string
  player_uuid: string
  server_name: string
  server_uuid: string
  start: number
  end: number
  length: number
  afk_time: number
  mob_kills: number
  deaths: number
  join_address: string
  first_session?: boolean
  world_series?: PlanSessionWorldSlice[]
  gm_series?: PlanSessionGmSeries
  player_kills?: PlanKillRow[]
}

export interface PlanCommandUsageRow {
  command: string
  times_used: number
  server_name: string
}

export interface PlanPlayerV1Entry {
  name: string
  uuid: string
  activity_index: number
  activity_group: string
  active_playtime: number
  sessions: number
  registered: number
  last_seen: number
  join_address: string
  geolocation: string
  ping_avg: number
  ping_min: number
  ping_max: number
  nick: string
  extension_values: Record<string, unknown>
}

export interface PlanPlayersV1 {
  data: PlanPlayerV1Entry[]
}

export interface PlanSessionsV1 {
  sessions: PlanSessionRow[]
  most_recent_sessions: PlanSessionRow[]
}

export interface PlanPlayerTableRow {
  name: string
  uuid: string
  activity_index: number
  activity_group: string
  active_playtime_ms: number
  sessions: number
  registered: number
  last_seen: number
  join_address: string
  geolocation: string
  ping_avg: number
  ping_min: number
  ping_max: number
  nick: string
  kick_count: number
}

export interface PlanPingTableRow {
  country: string
  avg_ping: number
  min_ping: number
  max_ping: number
}

export interface PlanGraphPlayersOnline {
  series: [number, number][]
}

export interface PlanKillRow {
  killer_name: string
  killer_uuid: string
  victim_name: string
  victim_uuid: string
  server_uuid: string
  weapon: string
  date: number
}

export interface PlanPlayerVersusV1 {
  numbers: {
    player_kills_total: number
    player_kills_30d: number
    player_kills_7d: number
    deaths_total: number
    mob_kills_total: number
    kdr_total: number
  }
  insights: {
    deadliest_pvp_weapon: string
    second_deadliest_pvp_weapon: string
    third_deadliest_pvp_weapon: string
  }
}

export interface PlanPlayerProfileStats {
  total_playtime_ms: number
  afk_time_ms: number
  mob_kills: number
  deaths: number
  session_count: number
  player_kill_count: number
}

export interface PlanPlayerNicknameRow {
  nickname: string
  server_name: string
  server_uuid: string
  last_used: number
}

export interface PlanPlayerConnectionRow {
  geolocation: string
  date: number
}

export interface PlanPlayerProfileV1 {
  info: PlanPlayerTableRow
  stats: PlanPlayerProfileStats
  nicknames: PlanPlayerNicknameRow[]
  connections: PlanPlayerConnectionRow[]
  punchcard_series: { x: number; y: number; z: number }[]
  world_pie_series: { name: string; y: number }[]
  server_pie_series: { name: string; y: number }[]
  sessions: PlanSessionRow[]
  kills_as_killer: PlanKillRow[]
  kills_as_victim: PlanKillRow[]
}

export interface PlanSessionsOverviewV1 {
  insights: {
    total_playtime_ms: number
    afk_time_ms: number
    afk_time_perc: number
    server_occupied_perc: number
    most_active_gamemode: string
  }
}

export interface PlanJoinAddressRow {
  join_address: string
  count: number
}

export interface PlanFilterTypeInfo {
  id: string
  label: string
  implemented: boolean
}

export interface PlanFiltersCatalog {
  activity_groups: string[]
  join_addresses: string[]
  geolocations: string[]
  filter_types: PlanFilterTypeInfo[]
}

export interface PlanQueryRequest {
  activity_groups: string[]
  join_addresses: string[]
  min_activity_index?: number | null
  max_activity_index?: number | null
  min_sessions?: number | null
  registered_after_ms?: number | null
  registered_before_ms?: number | null
  geolocations?: string[]
  bypass_cache?: boolean
  view_after_ms?: number | null
  view_before_ms?: number | null
  last_seen_after_ms?: number | null
  last_seen_before_ms?: number | null
  played_after_ms?: number | null
  played_before_ms?: number | null
  played_server_uuids?: string[]
}

export interface PlanQueryCohortView {
  view_after_ms: number
  view_before_ms: number
  sessions_in_window: number
  playtime_ms: number
  afk_ms: number
  players_active_in_window: number
}

export interface PlanQueryResult {
  matched_count: number
  players: PlanPlayerTableRow[]
  avg_activity_index: number
  avg_active_playtime_ms: number
  cache_token: string
  from_cache: boolean
  cohort_view?: PlanQueryCohortView | null
}

export interface PlanPlayersTableV1 {
  data: PlanPlayerV1Entry[]
  extension_columns: { key: string; label: string; plugin_name: string }[]
}

export interface PlanOnlineOverviewV1 {
  numbers: {
    unique_players_30d: number
    unique_players_7d: number
    unique_players_24h: number
    unique_players_30d_avg: number
    new_players_30d: number
    new_players_7d: number
    new_players_24h: number
    new_players_retention_30d: number
  }
  insights: {
    first_session_avg_length_ms: number
    first_session_median_length_ms: number
    lone_joins: number
    lone_new_joins: number
    players_online_on_first_join_avg: number
  }
}

export interface PlanPlayerbaseOverviewV1 {
  trends: {
    total_players_now: number
    total_players_then: number
    total_players_trend: number
    regular_players_now: number
    regular_players_then: number
    regular_players_trend: number
    playtime_avg_now_ms: number
    playtime_avg_then_ms: number
    afk_now_ms: number
    afk_then_ms: number
    regular_playtime_avg_now_ms: number
    regular_session_avg_now: number
  }
  insights: {
    avg_activity_index: number
    avg_global_playtime_ms: number
  }
}

export interface PlanPerformanceOverviewV1 {
  numbers: {
    low_tps_spikes_30d: number
    low_tps_spikes_7d: number
    low_tps_spikes_24h: number
    server_downtime_30d_ms: number
    server_downtime_7d_ms: number
    server_downtime_24h_ms: number
    avg_server_uptime_30d: number
    players_30d: number
    players_7d: number
    players_24h: number
    tps_30d: number
    tps_7d: number
    tps_24h: number
    cpu_30d: number
    cpu_7d: number
    cpu_24h: number
    ram_30d: number
    ram_7d: number
    ram_24h: number
    mspt_average_30d: number
    mspt_95th_30d: number
  }
  insights: {
    low_tps_players: number
    low_tps_chunks: number
    low_tps_entities: number
    low_tps_tps: number
  }
}

export interface PlanRetentionPlayerRow {
  uuid: string
  name: string
  registered: number
  last_seen: number
  playtime_ms: number
  active_playtime_ms: number
  session_count: number
  activity_index: number
  join_address: string
}

export interface PlanRetentionV1 {
  new_players: PlanRetentionPlayerRow[]
}

export interface PlanServerOverviewV1 {
  server_uuid: string
  server_name: string
  numbers: {
    total_players: number
    online_players: number
    total_sessions: number
    total_playtime_ms: number
    total_deaths: number
  }
  last_7_days: {
    unique_players: number
    session_count: number
    avg_session_length_ms: number
  }
}

export interface PlanPluginInstallResult {
  server_id: string
  server_name: string
  success: boolean
  detail: string
}

export interface NetworkAnalyticsOverview {
  network_id: string
  network_name: string
  combined_online: number
  combined_unique_players: number
  combined_total_joins: number
  combined_playtime_secs: number
  combined_deaths: number
  combined_advancements: number
  combined_mob_kills: number
  combined_player_kills: number
  combined_top_statistics: StatisticRow[]
  recent_deaths: DeathRow[]
  recent_events: EventRow[]
  servers: ServerAnalyticsSnapshot[]
}

export interface AddBackendRequest {
  name: string
  minecraft_version?: string
}

export interface HubSettings {
  bind: string
  motd: string
  online_mode: boolean
  show_max_players: number
  player_info_forwarding: string
}

export interface UpdateHubSettingsRequest {
  bind?: string
  motd?: string
  online_mode?: boolean
  show_max_players?: number
  player_info_forwarding?: string
}
