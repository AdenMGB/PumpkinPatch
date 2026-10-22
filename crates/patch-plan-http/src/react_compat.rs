//! Plan upstream React dashboard compatibility (`/auth/*`, `/v1/whoami`, extra `/v1/*` shims).

use axum::{
    extract::{Query, State},
    http::{header::SET_COOKIE, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Form, Json,
};
use patch_core::{
    list_user_permissions, plan_validate_cookie, plan_web_login, registration_code_pending,
    start_web_registration, username_for_session_cookie,
};
use serde::Deserialize;
use serde_json::json;

use crate::auth_layer::extract_plan_session;
use crate::PlanHttpState;

#[derive(Deserialize)]
pub struct PlanAuthForm {
    pub user: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RegisterCheckQuery {
    pub code: String,
}

pub async fn whoami(State(st): State<PlanHttpState>, headers: HeaderMap) -> impl IntoResponse {
    let auth_required = true;
    let session = extract_plan_session(&headers);
    let Some(cookie) = session else {
        return Json(json!({
            "authRequired": auth_required,
            "loggedIn": false,
        }));
    };
    if !plan_validate_cookie(&st.pool, &cookie).await.unwrap_or(false) {
        return Json(json!({
            "authRequired": auth_required,
            "loggedIn": false,
        }));
    }
    let Some(username) = username_for_session_cookie(&st.pool, &cookie)
        .await
        .unwrap_or(None)
    else {
        return Json(json!({
            "authRequired": auth_required,
            "loggedIn": false,
        }));
    };
    let permissions = list_user_permissions(&st.pool, &username)
        .await
        .unwrap_or_default();
    let linked_uuid: Option<String> = sqlx::query_scalar(
        "SELECT linked_to_uuid FROM plan_security WHERE username = ? COLLATE NOCASE",
    )
    .bind(&username)
    .fetch_optional(&st.pool)
    .await
    .unwrap_or(None);
    Json(json!({
        "authRequired": auth_required,
        "loggedIn": true,
        "user": {
            "username": username,
            "playerUUID": linked_uuid.filter(|u| !u.is_empty()),
            "permissions": permissions,
        }
    }))
}

pub async fn auth_login_form(
    State(st): State<PlanHttpState>,
    Form(form): Form<PlanAuthForm>,
) -> impl IntoResponse {
    match plan_web_login(&st.pool, &form.user, &form.password).await {
        Ok(Some(cookie)) => {
            let mut response = Json(json!({ "success": true })).into_response();
            if let Ok(value) = HeaderValue::from_str(&format!(
                "plan_session={}; Path=/; HttpOnly; SameSite=Lax",
                cookie
            )) {
                response.headers_mut().append(SET_COOKIE, value);
            }
            response
        }
        Ok(None) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "success": false, "error": "Invalid credentials" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn auth_register_form(
    State(st): State<PlanHttpState>,
    Form(form): Form<PlanAuthForm>,
) -> impl IntoResponse {
    match start_web_registration(&st.pool, &form.user, &form.password).await {
        Ok(start) => Json(json!({ "code": start.code })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn auth_register_poll(
    State(st): State<PlanHttpState>,
    Query(q): Query<RegisterCheckQuery>,
) -> impl IntoResponse {
    let pending = registration_code_pending(&st.pool, &q.code)
        .await
        .unwrap_or(true);
    Json(json!({ "success": !pending }))
}

pub async fn auth_logout_get(headers: HeaderMap) -> impl IntoResponse {
    let mut response = (
        StatusCode::FOUND,
        [(axum::http::header::LOCATION, "/login")],
    )
        .into_response();
    if let Ok(value) = HeaderValue::from_str("plan_session=; Path=/; Max-Age=0; HttpOnly") {
        response.headers_mut().append(SET_COOKIE, value);
    }
    let _ = headers;
    response
}
