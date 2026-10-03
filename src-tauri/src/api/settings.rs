use super::Result;
use crate::state::AppStateHandle;
use patch_core::AppSettings;

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
    Ok(())
}
