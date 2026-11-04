use axum::body::Body;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::Response;
use axum::Router;
use serde_json::{json, Value};
use tower_http::compression::CompressionLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::db::Image;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

mod admin;
mod albums;
mod auth_routes;
mod images;
mod pages;
mod public;
mod upload;

pub fn app_router(state: AppState) -> Router {
    let security_headers = |name: http::HeaderName, val: &'static str| {
        SetResponseHeaderLayer::overriding(name, HeaderValue::from_static(val))
    };

    Router::new()
        .merge(public::routes())
        .merge(auth_routes::routes())
        .merge(upload::routes())
        .merge(images::routes())
        .merge(albums::routes())
        .merge(admin::routes())
        .merge(pages::routes())
        .with_state(state)
        .layer(security_headers(
            http::header::X_CONTENT_TYPE_OPTIONS,
            "nosniff",
        ))
        .layer(security_headers(
            http::header::REFERRER_POLICY,
            "strict-origin-when-cross-origin",
        ))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}

// ---------- shared helpers ----------

pub(crate) fn base_url(state: &AppState) -> String {
    state.base_url()
}

pub(crate) async fn load_image(db: &sqlx::PgPool, id: Uuid) -> AppResult<Image> {
    let img = sqlx::query_as::<_, Image>(&format!(
        "SELECT {} FROM images WHERE id = $1 AND deleted_at IS NULL",
        Image::COLUMNS
    ))
    .bind(id)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(img)
}

/// Expiry check — applies to everyone (expired images are gone).
pub(crate) fn is_expired(img: &Image) -> bool {
    img.expires_at.map_or(false, |e| e <= chrono::Utc::now())
}

pub(crate) fn can_view(img: &Image, viewer: Option<&crate::db::User>) -> bool {
    match img.privacy.as_str() {
        "public" | "unlisted" => true,
        "private" => viewer.map_or(false, |u| u.is_admin() || img.owner_id == Some(u.id)),
        _ => false,
    }
}

pub(crate) fn ensure_viewable(img: &Image, viewer: Option<&crate::db::User>) -> AppResult<()> {
    if is_expired(img) || !can_view(img, viewer) {
        Err(AppError::NotFound)
    } else {
        Ok(())
    }
}

/// The album must belong to `owner_id`. Used when attaching an upload or moving an image.
pub(crate) async fn ensure_album_owner(
    db: &sqlx::PgPool,
    album_id: Uuid,
    owner_id: Uuid,
) -> AppResult<()> {
    let owns: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM albums WHERE id = $1 AND owner_id = $2)")
            .bind(album_id)
            .bind(owner_id)
            .fetch_one(db)
            .await?;
    if owns {
        Ok(())
    } else {
        Err(AppError::BadRequest("相册不存在或不属于该用户".into()))
    }
}

fn entity_tag(img: &Image) -> Option<HeaderValue> {
    let sha = img.sha256.as_deref()?;
    if sha.is_empty() || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    HeaderValue::from_str(&format!("\"{sha}\"")).ok()
}

pub(crate) fn if_none_match(headers: &axum::http::HeaderMap, img: &Image) -> bool {
    let Some(tag) = entity_tag(img) else {
        return false;
    };
    let Ok(tag) = tag.to_str() else {
        return false;
    };
    let Some(inm) = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    inm.split(',').any(|part| part.trim() == tag)
}

pub(crate) fn apply_media_cache(headers: &mut axum::http::HeaderMap, img: &Image) {
    let value = match img.privacy.as_str() {
        "public" => "public, max-age=3600",
        _ => "private, max-age=300",
    };
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(value));
    if let Some(tag) = entity_tag(img) {
        headers.insert(header::ETAG, tag);
    }
}

pub(crate) fn not_modified(img: &Image) -> Response {
    let mut resp = Response::new(Body::empty());
    *resp.status_mut() = StatusCode::NOT_MODIFIED;
    apply_media_cache(resp.headers_mut(), img);
    resp
}

/// Build the public JSON shape for an image. `url`/`thumb`/`share` are relative
/// paths (safe for in-page `<img>` display from any origin); `links.*` are
/// absolute, built from the configured site domain, for copy/paste elsewhere.
pub(crate) fn image_json(img: &Image, owner_name: Option<String>, state: &AppState) -> Value {
    let base = base_url(state);
    let rel_direct = format!("/i/{}", img.id);
    let rel_thumb = format!("/t/{}", img.id);
    let rel_share = format!("/s/{}", img.id);
    let direct = if base.is_empty() {
        rel_direct.clone()
    } else {
        format!("{base}{rel_direct}")
    };
    let thumb_abs = if base.is_empty() {
        rel_thumb.clone()
    } else {
        format!("{base}{rel_thumb}")
    };
    let share_abs = if base.is_empty() {
        rel_share.clone()
    } else {
        format!("{base}{rel_share}")
    };
    let alt = if !img.original_name.is_empty() {
        img.original_name.clone()
    } else {
        format!("image {}", img.id)
    };
    json!({
        "id": img.id,
        "owner_id": img.owner_id,
        "album_id": img.album_id,
        "owner_name": owner_name,
        "source_kind": img.source_kind,
        "filename": img.filename,
        "original_name": img.original_name,
        "content_type": img.content_type,
        "ext": img.ext,
        "size_bytes": img.size_bytes,
        "width": img.width,
        "height": img.height,
        "sha256": img.sha256,
        "privacy": img.privacy,
        "tags": img.tags,
        "views": img.views,
        "created_at": img.created_at,
        "expires_at": img.expires_at,
        "url": rel_direct,
        "thumb": rel_thumb,
        "share": rel_share,
        "links": {
            "direct": direct,
            "thumbnail": thumb_abs,
            "share": share_abs,
            "markdown": format!("![{alt}]({direct})"),
            "html": format!("<img src=\"{direct}\" alt=\"{alt}\" />"),
            "bbcode": format!("[img]{direct}[/img]"),
        }
    })
}

/// Serve an image's bytes — from local storage for uploads, or proxied for remote URLs.
pub(crate) async fn render_image(state: &AppState, img: &Image) -> AppResult<Response> {
    if img.is_local() {
        let key = img.storage_key.as_deref().ok_or(AppError::NotFound)?;
        let file = state.storage().open(key).await?;
        let stream = tokio_util::io::ReaderStream::new(file);
        let body = Body::from_stream(stream);
        let mut resp = Response::new(body);
        *resp.status_mut() = StatusCode::OK;
        if let Ok(ct) = HeaderValue::from_str(&img.content_type) {
            resp.headers_mut().insert(header::CONTENT_TYPE, ct);
        }
        if img.size_bytes > 0 {
            if let Ok(cl) = HeaderValue::from_str(&img.size_bytes.to_string()) {
                resp.headers_mut().insert(header::CONTENT_LENGTH, cl);
            }
        }
        apply_media_cache(resp.headers_mut(), img);
        Ok(resp)
    } else {
        let mut resp = proxy_remote(state, &img.url).await?;
        apply_media_cache(resp.headers_mut(), img);
        Ok(resp)
    }
}

pub(crate) async fn proxy_remote(state: &AppState, url: &str) -> AppResult<Response> {
    let resp = crate::netguard::guarded_get(state.http(), url).await?;
    let status = resp.status();
    let ct = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let cl = resp
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let body = Body::from_stream(resp.bytes_stream());
    let mut out = Response::new(body);
    *out.status_mut() = status;
    if let Some(ct) = ct {
        if let Ok(v) = HeaderValue::from_str(&ct) {
            out.headers_mut().insert(header::CONTENT_TYPE, v);
        }
    }
    if let Some(cl) = cl {
        if let Ok(v) = HeaderValue::from_str(&cl) {
            out.headers_mut().insert(header::CONTENT_LENGTH, v);
        }
    }
    Ok(out)
}

/// Like `proxy_remote`, but returns `Err` when the upstream is not OK or not an
/// image — used by the random-image proxy so it can skip dead/remote entries
/// and try another pick.
pub(crate) async fn fetch_remote_image(state: &AppState, url: &str) -> AppResult<Response> {
    let resp = crate::netguard::guarded_get(state.http(), url).await?;
    if !resp.status().is_success() {
        return Err(AppError::NotFound);
    }
    let ct = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if !ct.starts_with("image/") {
        return Err(AppError::UnsupportedFormat);
    }
    let cl = resp
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let body = Body::from_stream(resp.bytes_stream());
    let mut out = Response::new(body);
    *out.status_mut() = StatusCode::OK;
    if let Ok(v) = HeaderValue::from_str(&ct) {
        out.headers_mut().insert(header::CONTENT_TYPE, v);
    }
    if let Some(cl) = cl {
        if let Ok(v) = HeaderValue::from_str(&cl) {
            out.headers_mut().insert(header::CONTENT_LENGTH, v);
        }
    }
    Ok(out)
}

pub(crate) async fn serve_thumb(state: &AppState, img: &Image) -> AppResult<Response> {
    let key = img.thumb_key.as_deref().ok_or_else(|| {
        // Fall back to the original when no thumbnail exists (e.g. remote entries).
        AppError::NotFound
    })?;
    let bytes = state.storage().get(key).await?;
    let mut resp = Response::new(Body::from(bytes));
    *resp.status_mut() = StatusCode::OK;
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(crate::imaging::THUMBNAIL_CONTENT_TYPE),
    );
    apply_media_cache(resp.headers_mut(), img);
    Ok(resp)
}

/// Best-effort view counter increment.
pub(crate) fn bump_views(state: &AppState, id: Uuid) {
    let db = state.db().clone();
    tokio::spawn(async move {
        let _ = sqlx::query("UPDATE images SET views = views + 1 WHERE id = $1")
            .bind(id)
            .execute(&db)
            .await;
    });
}
