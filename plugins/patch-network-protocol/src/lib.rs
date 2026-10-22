//! Shared constants and payloads for Pumpkin Patch hub plugins.

pub const CHANNEL_SERVER_LIST: &str = "pumpkinpatch:server_list";
pub const CHANNEL_CONNECT: &str = "pumpkinpatch:connect";

/// Plugin metadata `name` and data folder under `plugins/data/`.
pub const HUB_LOBBY_PLUGIN_NAME: &str = "patch-hub-lobby";

/// Server list config written by Pumpkin Patch desktop (`plugins/data/patch-hub-lobby/`).
pub const HUB_SERVERS_CONFIG: &str = "hub_servers.json";

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerListEntry {
    pub velocity_name: String,
    pub display_name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerListPayload {
    pub servers: Vec<ServerListEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectRequest {
    pub velocity_name: String,
}
