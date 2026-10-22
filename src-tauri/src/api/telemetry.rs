use super::Result;
use crate::state::AppStateHandle;
use tauri::ipc::Channel;
use uuid::Uuid;

#[tauri::command]
pub async fn subscribe_live_telemetry(
    state: tauri::State<'_, AppStateHandle>,
    server_id: Uuid,
    on_frame: Channel<Vec<u8>>,
) -> Result<()> {
    let inner = state.get().await?;
    inner
        .vineyard_bus
        .register_server(server_id, server_id.to_string())
        .await;
    let mut rx = inner.vineyard_bus.subscribe_server_frames(server_id).await;
    patch_core::telemetry_bus::spawn_host_telemetry_sampler(&inner, server_id, 0);
    tauri::async_runtime::spawn(async move {
        while let Ok(frame) = rx.recv().await {
            if on_frame.send(frame).is_err() {
                break;
            }
        }
    });
    Ok(())
}
