use super::Result;
use crate::state::AppStateHandle;
use patch_core::{ensure_plan_schema, start_web_registration, RegistrationStart};
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
