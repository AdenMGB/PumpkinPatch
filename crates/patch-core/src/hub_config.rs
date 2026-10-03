use crate::error::Result;
use crate::models::ServerRecord;
use patch_network_protocol::{ServerListEntry, ServerListPayload};
use std::path::Path;

pub const HUB_LOBBY_PLUGIN_DIR: &str = "patch-hub-lobby";
pub const SERVER_LIST_FILENAME: &str = "server-list.json";

pub fn build_server_list(servers: &[ServerRecord]) -> ServerListPayload {
    let mut entries: Vec<ServerListEntry> = servers
        .iter()
        .filter(|s| s.role == "backend")
        .map(|s| ServerListEntry {
            velocity_name: s.velocity_name.clone(),
            display_name: s.name.clone(),
            description: format!("Join {} via /server {}", s.name, s.velocity_name),
        })
        .collect();
    entries.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    ServerListPayload { servers: entries }
}

pub fn write_lobby_server_list(lobby_data_path: &Path, servers: &[ServerRecord]) -> Result<()> {
    let payload = build_server_list(servers);
    let plugin_data = lobby_data_path
        .join("plugins")
        .join("data")
        .join(HUB_LOBBY_PLUGIN_DIR);
    std::fs::create_dir_all(&plugin_data)?;
    let path = plugin_data.join(SERVER_LIST_FILENAME);
    let raw = serde_json::to_string_pretty(&payload)?;
    std::fs::write(path, raw)?;
    Ok(())
}
