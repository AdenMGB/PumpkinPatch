use super::Result;
use crate::state::AppStateHandle;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ConsoleSendRequest {
    pub target: String,
    pub command: String,
}

#[derive(Debug, Serialize)]
pub struct ConsoleSendResponse {
    pub output: String,
}

#[tauri::command]
pub async fn network_console_send(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    request: ConsoleSendRequest,
) -> Result<ConsoleSendResponse> {
    let inner = state.get().await?;
    let network = inner.db.get_network(network_id).await?;
    let servers = inner.db.list_servers_for_network(network_id).await?;
    let mut processes = inner.processes.write().await;
    let output = processes
        .send_console_command(
            network_id,
            &network,
            &servers,
            &request.target,
            &request.command,
        )
        .await?;
    Ok(ConsoleSendResponse { output })
}
