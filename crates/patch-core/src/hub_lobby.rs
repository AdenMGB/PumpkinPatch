use crate::error::{Error, Result};
use crate::models::ServerRecord;
use patch_network_protocol::{HUB_LOBBY_PLUGIN_NAME, HUB_SERVERS_CONFIG, ServerListEntry, ServerListPayload};
use std::path::Path;

pub fn hub_lobby_data_dir(instance_path: &Path) -> std::path::PathBuf {
    instance_path
        .join("plugins")
        .join("data")
        .join(HUB_LOBBY_PLUGIN_NAME)
}

/// Writes the server list consumed by the `patch-hub-lobby` WASM plugin (lobby instances only).
pub fn write_hub_servers_config(instance_path: &Path, servers: &[ServerRecord]) -> Result<()> {
    let mut entries = Vec::new();
    for server in servers {
        if server.role != "backend" {
            continue;
        }
        entries.push(ServerListEntry {
            velocity_name: server.velocity_name.clone(),
            display_name: server.name.clone(),
            description: format!("Join {} via the hub", server.name),
        });
    }
    let dir = hub_lobby_data_dir(instance_path);
    std::fs::create_dir_all(&dir)?;
    let payload = ServerListPayload { servers: entries };
    let json = serde_json::to_string_pretty(&payload)?;
    let path = dir.join(HUB_SERVERS_CONFIG);
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, json)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

pub fn ensure_hub_lobby_config_for_lobby(
    instance_path: &Path,
    all_servers: &[ServerRecord],
) -> Result<()> {
    write_hub_servers_config(instance_path, all_servers)
}

pub fn parse_bind_host(bind: &str) -> Result<String> {
    let bind = bind.trim();
    if let Some((host, _port)) = bind.rsplit_once(':') {
        if host.is_empty() {
            return Err(Error::InvalidState("invalid bind address".into()));
        }
        return Ok(host.to_string());
    }
    Err(Error::InvalidState(
        "bind must be host:port (e.g. 127.0.0.1:25565)".into(),
    ))
}
