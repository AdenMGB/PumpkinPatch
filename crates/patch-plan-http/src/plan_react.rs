//! Upstream Plan React dashboard static hosting (indepth.md §6).

use axum::{
    http::{header::LOCATION, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};

pub fn plan_react_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static/plan-react")
}

pub fn plan_react_available() -> bool {
    plan_react_dir().join("index.html").exists()
}

/// Plan React vs legacy HTML pages (must stay inside one `Router::new()` chain for axum typing).
pub fn wrap_public_routes<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    if plan_react_available() {
        apply_react_shell_routes(router)
    } else {
        router
    }
}

fn apply_react_shell_routes<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router
        .route("/legacy-query", get(legacy_query_redirect))
        .route("/manifest.json", get(plan_react_manifest))
        .route("/favicon.ico", get(plan_react_favicon))
        .route("/robots.txt", get(plan_react_robots))
        .route("/pageExtensionApi.js", get(plan_react_page_extension_api))
        .route("/login", get(plan_react_spa_shell))
        .route("/register", get(plan_react_spa_shell))
        .route("/players", get(plan_react_spa_shell))
        .route("/network", get(plan_react_spa_shell))
        .route("/network/{*rest}", get(plan_react_spa_shell))
        .route("/query", get(plan_react_spa_shell))
        .route("/query/{*rest}", get(plan_react_spa_shell))
        .route("/manage", get(plan_react_spa_shell))
        .route("/manage/{*rest}", get(plan_react_spa_shell))
        .route("/theme-editor", get(plan_react_spa_shell))
        .route("/theme-editor/{*rest}", get(plan_react_spa_shell))
        .route("/docs", get(plan_react_spa_shell))
        .route("/docs/{*rest}", get(plan_react_spa_shell))
        .route("/errors", get(plan_react_spa_shell))
        .route("/player/{*rest}", get(plan_react_spa_shell))
        .route("/server/{*rest}", get(plan_react_spa_shell))
}

async fn legacy_query_redirect() -> impl IntoResponse {
    (
        StatusCode::TEMPORARY_REDIRECT,
        [(LOCATION, "/app?panel=query")],
    )
}

pub async fn plan_react_spa_shell() -> impl IntoResponse {
    serve_plan_react_asset("index.html").await
}

async fn plan_react_manifest() -> impl IntoResponse {
    serve_plan_react_asset("manifest.json").await
}

async fn plan_react_favicon() -> impl IntoResponse {
    serve_plan_react_asset("favicon.ico").await
}

async fn plan_react_robots() -> impl IntoResponse {
    serve_plan_react_asset("robots.txt").await
}

async fn plan_react_page_extension_api() -> impl IntoResponse {
    serve_plan_react_asset("pageExtensionApi.js").await
}

pub async fn serve_plan_react_asset(name: &str) -> axum::response::Response {
    let file = plan_react_dir().join(name);
    match tokio::fs::read(&file).await {
        Ok(bytes) => {
            let ctype = match file.extension().and_then(|e| e.to_str()) {
                Some("js") => "application/javascript; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                Some("json") => "application/json; charset=utf-8",
                Some("ico") => "image/x-icon",
                Some("txt") => "text/plain; charset=utf-8",
                Some("png") => "image/png",
                _ => "text/html; charset=utf-8",
            };
            (
                [(axum::http::header::CONTENT_TYPE, ctype)],
                bytes,
            )
                .into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
