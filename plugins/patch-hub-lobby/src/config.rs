use patch_network_protocol::{HUB_SERVERS_CONFIG, ServerListEntry, ServerListPayload};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct ServerListState {
    pub entries: Vec<ServerListEntry>,
}

impl ServerListState {
    pub fn replace(&mut self, entries: Vec<ServerListEntry>) {
        self.entries = entries;
    }
}

pub fn load_server_list(data_folder: &str) -> Result<Vec<ServerListEntry>, String> {
    let path = Path::new(data_folder).join(HUB_SERVERS_CONFIG);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let payload: ServerListPayload = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    Ok(payload.servers)
}
