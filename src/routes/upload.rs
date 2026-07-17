use axum::extract::{Multipart, State};
use axum::routing::post;
use axum::Json;
use bytes::Bytes;
use chrono::{DateTime, Duration, Utc};
use futures::StreamExt;
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use serde_json::{json, Value};
use url::Url;

use crate::auth::AuthUser;
use crate::db::Image;
use crate::error::{AppError, AppResult};
use crate::imaging;
use crate::state::AppState;

use super::image_json;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/upload", post(upload_files))
        .route("/api/upload/url", post(upload_url))
}

type Router<S> = axum::Router<S>;

struct IngestOptions {
    privacy: String,
    tags: Vec<String>,
    album_id: Option<uuid::Uuid>,
    expires_at: Option<DateTime<Utc>>,
}

impl Default for IngestOptions {
    fn default() -> Self {
        Self {
            privacy: "public".into(),
            tags: vec![],
            album_id: None,
            expires_at: None,
        }
    }
}

fn parse_privacy(s: &str) -> AppResult<String> {
    match s {
        "" | "public" => Ok("public".into()),
        "unlisted" => Ok("unlisted".into()),
        "private" => Ok("private".into()),
        _ => Err(AppError::BadRequest("无效的隐私设置".into())),
    }
}

fn parse_tags(s: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for t in s.split(',') {
        let t = t.trim();
        if !t.is_empty() && !out.iter().any(|x| x == t) {
            out.push(t.to_string());
        }
    }
    out
}

fn parse_expires_hours(s: &str) -> AppResult<Option<DateTime<Utc>>> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    let h: i64 = s
        .parse()
        .map_err(|_| AppError::BadRequest("expires_hours 需为数字".into()))?;
    if h <= 0 {
        return Ok(None);
    }
    let h = h.min(87600); // cap at ~10 years
    Ok(Some(Utc::now() + Duration::hours(h)))
}

fn clean_filename(name: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        return String::new();
    }
    name.chars().take(180).collect::<String>()
}

/// Core ingestion: validate, hash, dedup, generate thumbnail, persist, update quota.
async fn ingest(
    state: &AppState,
    owner: &crate::db::User,
    original_name: &str,
    bytes: Bytes,
    opts: &IngestOptions,
) -> AppResult<Image> {
    let limit = state.cfg().max_upload_bytes;
    if bytes.len() > limit {
        return Err(AppError::PayloadTooLarge { limit });
    }
    let format = imaging::detect(&bytes).ok_or(AppError::UnsupportedFormat)?;
    let sha = imaging::sha256_hex(&bytes);

    // per-owner dedup
    let dup_sql = format!(
        "SELECT {} FROM images WHERE owner_id = $1 AND sha256 = $2 AND deleted_at IS NULL LIMIT 1",
        Image::COLUMNS
    );
    if let Some(existing) = sqlx::query_as::<_, Image>(&dup_sql)
        .bind(owner.id)
        .bind(&sha)
        .fetch_optional(state.db())
        .await?
    {
        return Ok(existing);
    }

    // quota (quota_bytes == 0 means unlimited)
    if owner.quota_bytes > 0 && owner.used_bytes + bytes.len() as i64 > owner.quota_bytes {
        return Err(AppError::QuotaExceeded);
    }

    let id = uuid::Uuid::new_v4();
    let id_hex = id.simple().to_string();
    let shard = &id_hex[..2];
    let ext = format.extension();
    let storage_key = format!("orig/{shard}/{id_hex}.{ext}");
    state
        .storage()
        .put(&storage_key, format.content_type(), bytes.clone())
        .await?;

    let dims = imaging::dimensions(&bytes);
    let thumb_key = format!("thumb/{shard}/{id_hex}.webp");
    let thumb_opt: Option<String> = match imaging::make_thumbnail(&bytes, 480) {
        Ok(t) => {
            state
                .storage()
                .put(&thumb_key, imaging::THUMBNAIL_CONTENT_TYPE, Bytes::from(t))
                .await?;
            Some(thumb_key)
        }
        Err(e) => {
            tracing::warn!(error = %e, "缩略图生成失败，仅存储原图");
            None
        }
    };

    let display_name = clean_filename(original_name);
    let img: Image = sqlx::query_as::<_, Image>(&format!(
        "INSERT INTO images \
         (id, owner_id, album_id, source_kind, storage_key, thumb_key, filename, original_name, \
          content_type, ext, size_bytes, width, height, sha256, privacy, tags, expires_at) \
         VALUES ($1,$2,$3,'upload',$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14::image_privacy,$15,$16) \
         RETURNING {}",
        Image::COLUMNS
    ))
    .bind(id)
    .bind(owner.id)
    .bind(opts.album_id)
    .bind(&storage_key)
    .bind(&thumb_opt)
    .bind(&display_name)
    .bind(original_name)
    .bind(format.content_type())
    .bind(ext)
    .bind(bytes.len() as i64)
    .bind(dims.map(|(w, _)| w as i32))
    .bind(dims.map(|(_, h)| h as i32))
    .bind(&sha)
    .bind(&opts.privacy)
    .bind(&opts.tags)
    .bind(opts.expires_at)
    .fetch_one(state.db())
    .await?;

    sqlx::query("UPDATE users SET used_bytes = used_bytes + $1 WHERE id = $2")
        .bind(bytes.len() as i64)
        .bind(owner.id)
        .execute(state.db())
        .await?;

    Ok(img)
}

async fn upload_files(
    State(state): State<AppState>,
    owner: AuthUser,
    mut multipart: Multipart,
) -> AppResult<Json<Value>> {
    let mut opts = IngestOptions::default();
    let mut files: Vec<(String, Bytes)> = vec![];
    let mut total_incoming = 0usize;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("上传解析失败: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        if matches!(
            name.as_str(),
            "privacy" | "tags" | "album_id" | "expires_hours"
        ) {
            let text = field.text().await.unwrap_or_default();
            match name.as_str() {
                "privacy" => opts.privacy = parse_privacy(&text)?,
                "tags" => opts.tags = parse_tags(&text),
                "album_id" => {
                    opts.album_id = if text.trim().is_empty() {
                        None
                    } else {
                        Some(
                            uuid::Uuid::parse_str(text.trim())
                                .map_err(|_| AppError::BadRequest("album_id 无效".into()))?,
                        )
                    }
                }
                "expires_hours" => opts.expires_at = parse_expires_hours(&text)?,
                _ => {}
            }
            continue;
        }
        // treat any remaining field as a file
        let filename = field.file_name().unwrap_or("file").to_string();
        let bytes = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("读取上传数据失败: {e}")))?;
        if bytes.is_empty() {
            continue;
        }
        total_incoming = total_incoming
            .checked_add(bytes.len())
            .ok_or(AppError::PayloadTooLarge {
                limit: state.cfg().max_upload_bytes,
            })?;
        if total_incoming > state.cfg().max_upload_bytes * 4 {
            return Err(AppError::PayloadTooLarge {
                limit: state.cfg().max_upload_bytes * 4,
            });
        }
        if files.len() >= 30 {
            return Err(AppError::BadRequest("单次最多上传 30 个文件".into()));
        }
        files.push((filename, bytes));
    }

    if files.is_empty() {
        return Err(AppError::BadRequest("未接收到文件".into()));
    }

    let owner = owner.0;
    let mut results = vec![];
    for (filename, bytes) in files {
        match ingest(&state, &owner, &filename, bytes, &opts).await {
            Ok(img) => results.push(json!({
                "ok": true,
                "image": image_json(&img, Some(owner.username.clone()), &state)
            })),
            Err(e) => results.push(json!({
                "ok": false,
                "filename": filename,
                "error": e.to_string()
            })),
        }
    }
    Ok(Json(json!({ "results": results })))
}

#[derive(Deserialize)]
struct UrlUpload {
    url: String,
    #[serde(default)]
    privacy: String,
    #[serde(default)]
    tags: String,
    #[serde(default)]
    album_id: Option<uuid::Uuid>,
    #[serde(default)]
    expires_hours: String,
}

fn filename_from_url(raw: &str) -> Option<String> {
    let parsed = Url::parse(raw).ok()?;
    let last = parsed.path_segments()?.last()?;
    let decoded = percent_decode_str(last).decode_utf8_lossy();
    let decoded = decoded.trim();
    if decoded.is_empty() {
        None
    } else {
        Some(decoded.to_string())
    }
}

async fn download_bounded(state: &AppState, raw: &str) -> AppResult<Bytes> {
    let max = state.cfg().max_upload_bytes;
    let resp = state
        .http()
        .get(raw)
        .send()
        .await
        .map_err(|e| AppError::BadRequest(format!("下载远程图片失败: {e}")))?;
    if !resp.status().is_success() {
        return Err(AppError::BadRequest(format!(
            "源站返回错误状态码: {}",
            resp.status()
        )));
    }
    if let Some(cl) = resp.content_length() {
        if cl as usize > max {
            return Err(AppError::PayloadTooLarge { limit: max });
        }
    }
    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::with_capacity(64 * 1024);
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::BadRequest(format!("下载中断: {e}")))?;
        if buf.len() + chunk.len() > max {
            return Err(AppError::PayloadTooLarge { limit: max });
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(buf.into())
}

async fn upload_url(
    State(state): State<AppState>,
    owner: AuthUser,
    Json(body): Json<UrlUpload>,
) -> AppResult<Json<Value>> {
    let raw = body.url.trim();
    if raw.is_empty() {
        return Err(AppError::BadRequest("URL 不能为空".into()));
    }
    let bytes = download_bounded(&state, raw).await?;
    let opts = IngestOptions {
        privacy: parse_privacy(&body.privacy)?,
        tags: parse_tags(&body.tags),
        album_id: body.album_id,
        expires_at: parse_expires_hours(&body.expires_hours)?,
    };
    let original_name = filename_from_url(raw).unwrap_or_else(|| "remote".to_string());
    let owner = owner.0;
    let img = ingest(&state, &owner, &original_name, bytes, &opts).await?;
    Ok(Json(json!({
        "ok": true,
        "image": image_json(&img, Some(owner.username.clone()), &state)
    })))
}
