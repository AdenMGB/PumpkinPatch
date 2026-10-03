//! Phase 7 stubs — Vine hub, remote agent, and cloud sync entry points.

use super::Result;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FutureFeatureStatus {
    pub vine_hub: bool,
    pub remote_agent: bool,
    pub cloud_sync_definitions: bool,
    pub local_status_page: bool,
}

#[tauri::command]
pub fn future_features_status() -> FutureFeatureStatus {
    FutureFeatureStatus {
        vine_hub: false,
        remote_agent: false,
        cloud_sync_definitions: false,
        local_status_page: true,
    }
}

#[tauri::command]
pub async fn future_vine_create_network() -> Result<()> {
    Err(patch_core::Error::InvalidState(
        "Vine hub is not enabled yet. Track Phase 7 in the roadmap.".into(),
    )
    .into())
}
