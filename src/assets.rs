use axum::body::Body;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use mime_guess::from_path;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "static/"]
struct Assets;

/// Serve a file from the embedded `static/` tree (e.g. `css/app.css`).
pub fn static_file(path: &str) -> Response {
    match Assets::get(path) {
        Some(file) => {
            let mime = from_path(path).first_or_octet_stream();
            let mut resp = Response::new(Body::from(file.data));
            *resp.status_mut() = StatusCode::OK;
            resp.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime.essence_str())
                    .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
            );
            // hashed asset paths are immutable; HTML pages share this route but
            // re-render is cheap, so a short cache is fine.
            resp.headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=300"));
            resp
        }
        None => (StatusCode::NOT_FOUND, "asset not found").into_response(),
    }
}

/// Serve an embedded HTML page by logical name (looks up `pages/<name>.html`).
pub fn html_page(name: &str) -> Response {
    static_file(&format!("pages/{name}.html"))
}
