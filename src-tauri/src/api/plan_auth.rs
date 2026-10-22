use super::Result;
use crate::state::AppStateHandle;
use patch_core::{
    delete_web_user, ensure_plan_schema, list_web_users, logout_web_user, set_web_user_group,
    start_web_registration, PlanWebUser, RegistrationStart,
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct PlanRegisterStartResponse {
    pub code: String,
    pub expires_ms: i64,
}

#[tauri::command]
pub async fn plan_auth_register_start(
    state: tauri::State<'_, AppStateHandle>,
    _network_id: Uuid,
    username: String,
    password: String,
) -> Result<PlanRegisterStartResponse> {
    let inner = state.get().await?;
    let pool = inner.db.pool();
    ensure_plan_schema(pool).await?;
    let start: RegistrationStart = start_web_registration(pool, &username, &password).await?;
    Ok(PlanRegisterStartResponse {
        code: start.code,
        expires_ms: start.expires_ms,
    })
}

#[tauri::command]
pub async fn plan_web_users_list(
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<PlanWebUser>> {
    let inner = state.get().await?;
    ensure_plan_schema(inner.db.pool()).await?;
    list_web_users(inner.db.pool()).await.map_err(Into::into)
}

#[tauri::command]
pub async fn plan_web_user_set_group(
    state: tauri::State<'_, AppStateHandle>,
    username: String,
    group_name: String,
) -> Result<()> {
    let inner = state.get().await?;
    set_web_user_group(inner.db.pool(), &username, &group_name).await?;
    Ok(())
}

#[tauri::command]
pub async fn plan_web_user_logout(
    state: tauri::State<'_, AppStateHandle>,
    username: String,
) -> Result<u64> {
    let inner = state.get().await?;
    Ok(logout_web_user(inner.db.pool(), &username).await?)
}

#[tauri::command]
pub async fn plan_web_user_delete(
    state: tauri::State<'_, AppStateHandle>,
    username: String,
) -> Result<()> {
    let inner = state.get().await?;
    delete_web_user(inner.db.pool(), &username).await?;
    Ok(())
}
