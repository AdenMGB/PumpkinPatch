use super::Result;
use crow_swarm::{SwarmPlan, SwarmReport};
use patch_cas::CasStore;
use patchwork::{index_pack_assets, merge_manifests, read_pack_manifest, PackManifest};
use patch_audit::AuditLedger;
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
pub async fn audit_ledger_append(
    state: tauri::State<'_, crate::state::AppStateHandle>,
    actor: String,
    action: String,
    detail: String,
) -> Result<String> {
    let inner = state.get().await?;
    let path = inner.dirs.data.join("audit-chain.jsonl");
    let mut ledger = AuditLedger::open(&path).map_err(|e| patch_core::Error::Other(e.to_string()))?;
    let entry = ledger
        .append(&actor, &action, &detail)
        .map_err(|e| patch_core::Error::Other(e.to_string()))?;
    Ok(entry.entry_hash)
}

#[tauri::command]
pub async fn patchwork_merge_pack_dirs(
    base_dir: String,
    overlay_dir: String,
) -> Result<PackManifest> {
    let base_path = std::path::Path::new(&base_dir);
    let overlay_path = std::path::Path::new(&overlay_dir);
    let base = read_pack_manifest(base_path)?;
    let overlay = read_pack_manifest(overlay_path)?;
    let mut merged = merge_manifests(base, overlay);
    let mut files = index_pack_assets(base_path)?;
    files.extend(index_pack_assets(overlay_path)?);
    merged.files = files;
    Ok(merged)
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
