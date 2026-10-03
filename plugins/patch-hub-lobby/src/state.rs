use patch_network_protocol::ServerListPayload;
use std::sync::Mutex;
use std::sync::OnceLock;

pub const SERVER_LIST_FILENAME: &str = "server-list.json";

pub struct RuntimeState {
    pub data_folder: String,
    pub list: ServerListPayload,
}

pub static STATE: OnceLock<Mutex<RuntimeState>> = OnceLock::new();

pub fn load_server_list(path: &str) -> Result<ServerListPayload, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

pub fn with_runtime<R>(f: impl FnOnce(&mut RuntimeState) -> R) -> Option<R> {
    STATE
        .get()
        .and_then(|m| m.lock().ok())
        .map(|mut g| f(&mut g))
}

pub fn reload_list_from_disk(rt: &mut RuntimeState) {
    let path = format!("{}/{SERVER_LIST_FILENAME}", rt.data_folder);
    if let Ok(list) = load_server_list(&path) {
        rt.list = list;
    }
}
