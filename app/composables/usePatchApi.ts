import { invoke } from '@tauri-apps/api/core'
import type {
  AddBackendRequest,
  AppSettings,
  CreateNetworkRequest,
  InstalledPlugin,
  JoinInfo,
  MinecraftVersionInfo,
  ModrinthSearchResult,
  ModrinthVersion,
  PumpkinMarketListResult,
  NetworkSummary,
  PingResult,
  ServerRecord,
  ServerSettings,
  UpdateHubSettingsRequest,
  UpdateServerSettingsRequest,
  HubSettings,
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
    settingsGet: () => invoke<AppSettings>('settings_get'),
    settingsSet: (settings: AppSettings) => invoke<void>('settings_set', { settings }),
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
    pumpkinMarketList: (search?: string, limit = 12, cursor?: string) =>
      invoke<PumpkinMarketListResult>('pumpkin_market_list_plugins', {
        search: search?.trim() ? search.trim() : null,
        limit,
        cursor: cursor ?? null,
      }),
    pumpkinMarketInstall: (pluginId: number, instancePath: string) =>
      invoke<string>('pumpkin_market_install_plugin', { pluginId, instancePath }),
  }
}
