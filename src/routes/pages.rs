use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Json;
use serde_json::json;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/static/{*path}", get(static_asset))
        .route("/login", get(|| async { page("login") }))
        .route("/register", get(|| async { page("register") }))
        .route("/dashboard", get(|| async { page("dashboard") }))
        .route("/upload", get(|| async { page("upload") }))
        .route("/gallery", get(|| async { page("gallery") }))
        .route("/explore", get(|| async { page("explore") }))
        .route("/albums", get(|| async { page("albums") }))
        .route("/settings", get(|| async { page("settings") }))
        .route("/admin", get(|| async { page("admin") }))
        .route("/s/{id}", get(|| async { page("share") }))
        .route("/i/{id}/view", get(|| async { page("share") }))
        .fallback(fallback)
}

type Router<S> = axum::Router<S>;

fn page(name: &'static str) -> Response {
    crate::assets::html_page(name)
}

async fn index() -> Response {
    page("index")
}

async fn static_asset(Path(path): Path<String>) -> Response {
    crate::assets::static_file(&path)
}

async fn fallback() -> Response {
    (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response()
}
