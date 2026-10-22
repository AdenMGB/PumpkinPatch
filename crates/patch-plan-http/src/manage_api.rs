//! Plan React `/manage` RBAC endpoints (upstream `manageService.js`).

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use patch_core::{
    delete_web_group, list_all_web_permissions, list_permissions_for_group, list_web_group_names,
    plan_log_access, plan_validate_cookie, save_permissions_for_group, username_for_session_cookie,
};

use crate::auth_layer::extract_plan_session;
use serde::Deserialize;

use crate::PlanHttpState;

#[derive(Debug, Deserialize)]
pub struct GroupQuery {
    pub group: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteGroupQuery {
    pub group: String,
    #[serde(rename = "moveTo")]
    pub move_to: String,
}

pub async fn web_groups(State(st): State<PlanHttpState>) -> impl IntoResponse {
    match list_web_group_names(&st.pool).await {
        Ok(groups) => Json(groups).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn permissions_catalog(State(st): State<PlanHttpState>) -> impl IntoResponse {
    match list_all_web_permissions(&st.pool).await {
        Ok(permissions) => Json(serde_json::json!({ "permissions": permissions })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn group_permissions(
    State(st): State<PlanHttpState>,
    Query(q): Query<GroupQuery>,
) -> impl IntoResponse {
    match list_permissions_for_group(&st.pool, &q.group).await {
        Ok(permissions) => Json(serde_json::json!({ "permissions": permissions })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn save_group_permissions(
    State(st): State<PlanHttpState>,
    headers: HeaderMap,
    Query(q): Query<GroupQuery>,
    Json(body): Json<Vec<String>>,
) -> impl IntoResponse {
    match save_permissions_for_group(&st.pool, &q.group, &body).await {
        Ok(()) => {
            let actor = session_username(&st, &headers).await;
            let _ = plan_log_access(
                &st.pool,
                actor.as_deref(),
                "POST",
                "/v1/saveGroupPermissions",
                200,
            )
            .await;
            Json(serde_json::json!({ "saved": q.group })).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn session_username(st: &PlanHttpState, headers: &HeaderMap) -> Option<String> {
    let session = extract_plan_session(headers)?;
    if !plan_validate_cookie(&st.pool, &session).await.unwrap_or(false) {
        return None;
    }
    username_for_session_cookie(&st.pool, &session)
        .await
        .ok()
        .flatten()
}

pub async fn delete_group(
    State(st): State<PlanHttpState>,
    Query(q): Query<DeleteGroupQuery>,
) -> impl IntoResponse {
    match delete_web_group(&st.pool, &q.group, &q.move_to).await {
        Ok(()) => Json(serde_json::json!({ "deleted": q.group })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}
