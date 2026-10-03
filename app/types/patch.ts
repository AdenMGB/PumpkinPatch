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
  hub_type?: 'velocity' | 'vine'
  backend_names: string[]
  pumpkin_channel?: string
  minecraft_version?: string
}

export interface AppSettings {
  java_path: string
  bind_host: string
  pumpkin_channel_default: string
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
