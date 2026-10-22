//! Session cookie gate for sensitive Plan endpoints (indepth.md §5.2).

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use patch_core::{plan_log_access, plan_validate_cookie, username_for_session_cookie};

use crate::PlanHttpState;

pub fn extract_plan_session(headers: &axum::http::HeaderMap) -> Option<String> {
    if let Some(value) = headers.get("x-plan-session") {
        if let Ok(s) = value.to_str() {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    if let Some(value) = headers.get(axum::http::header::COOKIE) {
        if let Ok(raw) = value.to_str() {
            for part in raw.split(';') {
                let part = part.trim();
                if let Some(rest) = part.strip_prefix("plan_session=") {
                    return Some(rest.to_string());
                }
            }
        }
    }
    None
}

pub async fn require_plan_session(
    State(st): State<PlanHttpState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let uri = req.uri().path().to_string();
    let method = req.method().to_string();
    let session = extract_plan_session(req.headers());
    let authorized = match session {
        Some(cookie) => plan_validate_cookie(&st.pool, &cookie).await.unwrap_or(false),
        None => false,
    };
    if !authorized {
        let _ = plan_log_access(&st.pool, None, &method, &uri, 401).await;
        return (StatusCode::UNAUTHORIZED, "Plan session required").into_response();
    }
    let response = next.run(req).await;
    let _ = plan_log_access(
        &st.pool,
        None,
        &method,
        &uri,
        response.status().as_u16() as i32,
    )
    .await;
    response
}

pub async fn require_admin_session(
    State(st): State<PlanHttpState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let uri = req.uri().path().to_string();
    let method = req.method().to_string();
    let session = extract_plan_session(req.headers());
    let Some(cookie) = session else {
        return (StatusCode::UNAUTHORIZED, "Plan session required").into_response();
    };
    let Some(username) = username_for_session_cookie(&st.pool, &cookie)
        .await
        .unwrap_or(None)
    else {
        return (StatusCode::UNAUTHORIZED, "invalid session").into_response();
    };
    let is_admin = patch_core::user_has_permission(&st.pool, &username, "manage.groups")
        .await
        .unwrap_or(false);
    if !is_admin {
        let _ = plan_log_access(&st.pool, None, &method, &uri, 403).await;
        return (StatusCode::FORBIDDEN, "manage.groups required").into_response();
    }
    next.run(req).await
}
