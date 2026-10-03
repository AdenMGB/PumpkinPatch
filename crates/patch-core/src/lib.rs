//! Pumpkin Patch core: networks, downloads, processes, and ping.

mod analytics;
mod db;
mod error;
mod hub_settings;
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
    write_patch_plan_config, NetworkAnalyticsOverview, ServerAnalyticsSnapshot,
};
pub use db::Database;
pub use error::{Error, Result};
pub use hub_settings::{read_hub_settings, write_hub_settings, HubSettings, HubSettingsPatch};
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
    PumpkinMarketClient, PumpkinMarketListResult, PumpkinMarketPluginDetail,
    PumpkinMarketPluginSummary,
};
pub use releases::{DownloadProgress, ReleaseService};
pub use state::{AppDirs, AppState};
