use super::Result;
use crow_swarm::{SwarmPlan, SwarmReport};
use patch_cas::CasStore;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CasPutResponse {
    pub digest: String,
    pub size: u64,
}

#[tauri::command]
pub async fn crow_swarm_run(plan: SwarmPlan) -> Result<SwarmReport> {
    Ok(crow_swarm::run_swarm(plan).await)
}

#[tauri::command]
pub async fn patch_cas_put_bytes(
    state: tauri::State<'_, crate::state::AppStateHandle>,
    data: Vec<u8>,
) -> Result<CasPutResponse> {
    let inner = state.get().await?;
    let root = inner.dirs.cache.join("cas");
    let mut store = CasStore::new(root);
    let object = store
        .put_bytes(&data)
        .map_err(|e| patch_core::Error::Other(e.to_string()))?;
    Ok(CasPutResponse {
        digest: object.digest,
        size: object.size,
    })
}
