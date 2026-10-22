use super::Result;
use crate::bundled_plugins::refresh_plan_configs_all_networks;
use crate::state::AppStateHandle;
use patch_core::{detect_java_candidates, resolve_java_executable, AppSettings};
use serde::Serialize;

#[tauri::command]
pub async fn settings_get(state: tauri::State<'_, AppStateHandle>) -> Result<AppSettings> {
    let inner = state.get().await?;
    Ok(inner.settings().await)
}

#[tauri::command]
pub async fn settings_set(
    state: tauri::State<'_, AppStateHandle>,
    settings: AppSettings,
) -> Result<()> {
    let inner = state.get().await?;
    inner.update_settings(settings).await?;
    refresh_plan_configs_all_networks(&state, &inner).await;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct JavaDetectResult {
    pub resolved_path: Option<String>,
    pub candidates: Vec<String>,
}

#[tauri::command]
pub async fn settings_detect_java(
    state: tauri::State<'_, AppStateHandle>,
) -> Result<JavaDetectResult> {
    let inner = state.get().await?;
    let settings = inner.settings().await;
    let resolved = resolve_java_executable(&settings.java_path)
        .ok()
        .map(|p| p.to_string_lossy().into_owned());
    let candidates = detect_java_candidates()
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    Ok(JavaDetectResult {
        resolved_path: resolved,
        candidates,
    })
}
