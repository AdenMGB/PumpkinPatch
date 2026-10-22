//! Pumpkin Patch core: networks, downloads, processes, and ping.

mod analytics;
mod db;
mod plan_graph;
mod plan_disk_cache;
mod plan_json_cache;
mod plan_overviews;
mod plan_perf;
mod plan_query;
mod plan_store;
mod plan_db;
mod plan_import;
mod plan_sync;
mod plan_auth;
mod plan_web_groups;
mod plan_extension;
mod plan_data_clear;
mod plan_extension_sql;
mod plan_export;
mod plan_geolite;
mod plan_theme;
mod orchestration;
mod harvest_sync;
pub mod telemetry_bus;
mod plan_v1;
mod error;
mod hub_lobby;
mod hub_settings;
mod log_store;
mod ports;
mod instance_config;
mod instance_plugins;
mod java;
mod minecraft_versions;
mod modrinth;
mod models;
mod network;
mod ping;
mod plugin_install;
mod plugins;
mod port_wait;
mod process;
mod pumpkin_market;
mod rcon;
mod releases;
mod server_settings;
mod state;

pub use analytics::{
    aggregate_network_analytics, generate_export_secret, read_server_analytics,
    plan_export_is_proxy, write_patch_plan_config, NetworkAnalyticsOverview,
    ServerAnalyticsSnapshot,
};
pub use plan_store::ensure_plan_schema;
pub fn gourd_pool_scale_recommend(
    current_ccu: u32,
    active_backends: u32,
    players_per_backend: u32,
    hidden_ready: u32,
) -> u32 {
    gourd_proxy::predictive_scale_recommendation(
        current_ccu,
        active_backends,
        players_per_backend,
        hidden_ready,
    )
}

pub use plan_store::{
    graph_players_online, list_command_usages, list_kills, list_players_table, list_sessions,
    players_online_at, player_statistics_map, statistics_v1, PlanPlayerOnlineName,
    PlanStatisticsServerRow,
    list_join_addresses, network_overview_v1, ping_table_v1, player_gdpr_export_json,
    player_profile_v1, player_versus_v1,
    server_overview_v1, sessions_overview_v1, PlanJoinAddressRow, PlanSessionsOverviewV1,
    PlanCommandUsageRow, PlanGraphPlayersOnline, PlanKillRow, PlanNetworkOverviewV1, PlanPingTableRow,
    PlanPlayerConnectionRow, PlanPlayerNicknameRow, PlanPlayerProfileStats, PlanPlayerProfileV1,
    PlanPlayerPunchcardPoint, PlanPlayerTableRow, PlanPlayerVersusV1, PlanPlayerWorldSlice,
    PlanServerOverviewV1,
    PlanSessionRow, PlanSyncReport, PlanNetworkServerRow, plan_network_servers_v1,
};
pub use plan_graph::plan_graph;
pub use plan_disk_cache::{read_disk_cache, write_disk_cache, DISK_CACHE_TTL_SECS};
pub use plan_json_cache::{PlanJsonCache, MEMORY_CACHE_MS, REFRESH_BARRIER_MS, with_plan_cache};
pub use plan_query::{
    execute_plan_query, execute_plan_sql_write, execute_readonly_sql, load_cached_plan_query,
    plan_filters_catalog,
    ActivityIndexOnFilter, PlanFilterTypeInfo, PlanFiltersCatalog, PlanQueryCohortView,
    PlanQueryRequest, PlanQueryResult,
};
pub use plan_data_clear::{
    plan_data_clear_events_since, record_plan_player_data_cleared, PlanDataClearEvent,
};
pub use plan_extension_sql::{
    extension_columns_for_table, extension_data_v1_from_sql, extension_values_by_uuid,
    sync_all_servers_extensions, sync_registered_extensions_to_sql,
};
pub use plan_extension::{
    ensure_builtin_extensions, extension_data_merged, extension_data_v1, for_each_extension,
    register_plan_extension,
    PlanExtensionDataV1,
    PlanExtensionPlugin, PlanExtensionTab, ExtensionQueryProvider, PluginBooleanGroupFilter,
    PluginNumberFilter,
};
pub use plan_auth::{
    complete_web_registration, delete_web_user, list_web_users, log_access as plan_log_access,
    login as plan_web_login, logout_web_user, process_registration_completions,
    process_web_admin_actions,
    set_web_user_group, start_web_registration, user_has_permission,
    list_user_permissions, registration_code_pending, username_for_session_cookie,
    validate_cookie as plan_validate_cookie, PlanWebUser,
    RegistrationStart,
};
pub use plan_web_groups::{
    delete_web_group, list_all_web_permissions, list_permissions_for_group, list_web_group_names,
    save_permissions_for_group,
};
pub use plan_export::{default_export_dir, export_plan_static_bundle, plan_tls_pem_paths};
pub use plan_geolite::{configure_network_geolocation, geolite_cache_path};
pub use plan_theme::{plan_theme_json, plan_theme_json_db, save_plan_theme_json};
pub use orchestration::{
    build_topology, hot_wire_velocity_toml, NetworkTopology, TopologyEdge, TopologyNode,
};
pub use harvest_sync::sync_harvest_from_plan_sqlite;
pub use plan_v1::{
    players_table_v1, players_v1, sessions_v1, PlanExtensionColumn, PlanPlayerV1Entry,
    PlanPlayersTableV1, PlanPlayersV1, PlanSessionsV1,
};
pub use plan_overviews::{
    online_overview_v1, performance_overview_v1, playerbase_overview_v1, retention_v1,
    PlanOnlineOverviewV1, PlanPerformanceOverviewV1, PlanPlayerbaseOverviewV1, PlanRetentionPlayerRow,
    PlanRetentionV1,
};
pub use plan_db::{
    backup_plan_sqlite_file, clear_plan_network_data, mark_plan_server_uninstalled,
    optimize_plan_sqlite, process_desktop_plan_requests, remove_plan_player,
    list_plan_backups, restore_plan_sqlite_from_backup, PlanBackupEntry, PlanDbBackupReport,
};
pub use plan_import::{import_signed_plan_export, ingest_signed_export};
pub use plan_sync::sync_network_plan_exports;
pub use db::Database;
pub use error::{Error, Result};
pub use hub_lobby::{ensure_hub_lobby_config_for_lobby, parse_bind_host, write_hub_servers_config};
pub use hub_settings::{read_hub_settings, write_hub_settings, HubSettings, HubSettingsPatch};
pub use log_store::{append_log_line, read_log_tail, clear_logs};
pub use ports::{ensure_ports_available, is_tcp_port_available};
pub use instance_config::{
    rcon_password_for_server, rcon_port_for_game_port, sync_server_instance_config,
};
pub use rcon::send_rcon_command;
pub use instance_plugins::{list_installed_plugins, remove_installed_plugin, InstalledPlugin};
pub use java::{detect_java_candidates, resolve_java_executable, FILL_USER_AGENT};
pub use minecraft_versions::{fetch_minecraft_versions, MinecraftVersionInfo};
pub use modrinth::{
    ModrinthClient, ModrinthSearchHit, ModrinthSearchResult, ModrinthVersion,
};
pub use server_settings::{
    bind_address_for_port, read_server_settings, write_server_settings, ServerSettings,
};
pub use models::*;
pub use network::NetworkOrchestrator;
pub use ping::ping_server;
pub use plugin_install::discover_plugins_root;
pub use plugins::{
    deploy_auto_plugins_for_server, deploy_plugin, PluginManifest, PluginManifestEntry,
};
pub use process::{LogLine, ProcessSupervisor};
pub use pumpkin_market::{
    PumpkinMarketClient, PumpkinMarketListResult, PumpkinMarketPluginCommand,
    PumpkinMarketPluginDetail, PumpkinMarketPluginSummary, PumpkinMarketPluginVersion,
};
pub use releases::{DownloadProgress, ReleaseService};
pub use state::{AppDirs, AppState};
