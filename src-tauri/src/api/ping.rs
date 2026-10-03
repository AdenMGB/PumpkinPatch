use super::Result;
use patch_core::{ping_server, PingResult};

#[tauri::command]
pub async fn server_ping(host: String, port: u16) -> Result<PingResult> {
    Ok(ping_server(&host, port).await?)
}
