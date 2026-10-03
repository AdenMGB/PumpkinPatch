export type NetworkRuntimeStatus = 'stopped' | 'starting' | 'running' | 'stopping' | 'error'

export type HealthLevel = 'healthy' | 'degraded' | 'down' | 'unknown'

export interface ComponentHealth {
  id: string
  label: string
  level: HealthLevel
  detail: string
}

export interface NetworkHealthReport {
  network_id: string
  overall: HealthLevel
  hub: ComponentHealth
  servers: ComponentHealth[]
  plugins: ComponentHealth[]
  checked_at: string
}

export interface NetworkRecord {
  id: string
  name: string
  hub_type: string
  hub_port: number
  forwarding_secret: string
  lobby_server_id: string | null
  data_path: string
  analytics_export_token?: string
  auto_restart?: boolean
  last_crash_source?: string | null
  last_crash_at?: string | null
  restart_attempts?: number
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
  hub_type?: 'velocity' | 'vine'
  backend_names: string[]
  pumpkin_channel?: string
  minecraft_version?: string
}

export interface AppSettings {
  java_path: string
  bind_host: string
  pumpkin_channel_default: string
  java_min_major?: number
}

export interface PortMapEntry {
  label: string
  port: number
}

export interface NetworkTemplate {
  id: string
  name: string
  description: string
  backend_names: string[]
  pumpkin_channel?: string | null
  minecraft_version?: string | null
}

export interface ProcessExitEvent {
  network_id: string
  source: string
  server_id: string | null
  exit_code: number | null
}

export interface DownloadProgress {
  downloaded: number
  total: number | null
  label: string
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
}

export interface PumpkinMarketListResult {
  items: PumpkinMarketPluginSummary[]
  has_more: boolean
  next_cursor: string | null
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
