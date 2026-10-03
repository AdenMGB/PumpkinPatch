//! Shared constants and payloads for Pumpkin Patch hub plugins.

pub const CHANNEL_SERVER_LIST: &str = "pumpkinpatch:server_list";
pub const CHANNEL_CONNECT: &str = "pumpkinpatch:connect";

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
