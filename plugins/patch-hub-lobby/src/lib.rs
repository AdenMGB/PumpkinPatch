//! Scaffold for the hub lobby plugin.
//!
//! Build a WASM artifact with Pumpkin's plugin toolchain and place it at
//! `plugins/artifacts/patch-hub-lobby.wasm` for the desktop app to deploy.
//! See https://docs.pumpkinmc.org/plugin-dev/introduction

use patch_network_protocol::{ServerListEntry, ServerListPayload};
use serde_json::json;

/// Example payload the lobby plugin would broadcast to clients.
pub fn default_server_list() -> ServerListPayload {
    ServerListPayload {
        servers: vec![
            ServerListEntry {
                velocity_name: "lobby".into(),
                display_name: "Lobby".into(),
                description: "Main hub".into(),
            },
        ],
    }
}

/// JSON used in integration tests and manifest validation.
pub fn default_server_list_json() -> String {
    json!(default_server_list()).to_string()
}
