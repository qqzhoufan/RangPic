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

enum CommitError {
    Quota,
    Duplicate,
    Gone,
    Other(AppError),
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
    let size = bytes.len() as i64;

    if let Some(existing) = find_owner_sha(state, owner.id, &sha).await? {
        return Ok(existing);
    }
    if let Some(album_id) = opts.album_id {
        super::ensure_album_owner(state.db(), album_id, owner.id).await?;
    }
    quota_has_room(state, owner.id, size).await?;

    let encoded = bytes.clone();
    let preview = match tokio::task::spawn_blocking(move || imaging::preview(&encoded, 480)).await {
        Ok(Ok(p)) => Some(p),
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "缩略图生成失败，仅存储原图");
            None
        }
        Err(e) => {
            tracing::warn!(error = %e, "缩略图任务失败，仅存储原图");
            None
        }
    };

    let id = uuid::Uuid::new_v4();
    let id_hex = id.simple().to_string();
    let shard = &id_hex[..2];
    let ext = format.extension();
    let storage_key = format!("orig/{shard}/{id_hex}.{ext}");
    let mut written = Vec::new();
    written.push(storage_key.clone());
    if let Err(e) = state
        .storage()
        .put(&storage_key, format.content_type(), bytes.clone())
        .await
    {
        remove_stored(state, &written).await;
        return Err(e);
    }

    let (thumb_key, width, height) = match preview {
        Some(preview) => {
            let thumb_key = format!("thumb/{shard}/{id_hex}.webp");
            let width = Some(preview.width as i32);
            let height = Some(preview.height as i32);
            written.push(thumb_key.clone());
            if let Err(e) = state
                .storage()
                .put(
                    &thumb_key,
                    imaging::THUMBNAIL_CONTENT_TYPE,
                    Bytes::from(preview.thumbnail),
                )
                .await
            {
                remove_stored(state, &written).await;
                return Err(e);
            }
            (Some(thumb_key), width, height)
        }
        None => (None, None, None),
    };

    let display_name = clean_filename(original_name);
    match commit_new_image(
        state,
        NewImage {
            id,
            owner_id: owner.id,
            album_id: opts.album_id,
            storage_key,
            thumb_key,
            filename: display_name,
            original_name: original_name.to_string(),
            content_type: format.content_type().to_string(),
            ext: ext.to_string(),
            size,
            width,
            height,
            sha: sha.clone(),
            privacy: opts.privacy.clone(),
            tags: opts.tags.clone(),
            expires_at: opts.expires_at,
        },
    )
    .await
    {
        Ok(img) => Ok(img),
        Err(CommitError::Duplicate) => {
            remove_stored(state, &written).await;
            find_owner_sha(state, owner.id, &sha)
                .await?
                .ok_or_else(|| AppError::Conflict("图片已存在但无法读取".into()))
        }
        Err(CommitError::Quota) => {
            remove_stored(state, &written).await;
            Err(AppError::QuotaExceeded)
        }
        Err(CommitError::Gone) => {
            remove_stored(state, &written).await;
            Err(AppError::Unauthorized)
        }
        Err(CommitError::Other(e)) => {
            remove_stored(state, &written).await;
            Err(e)
        }
    }
}

struct NewImage {
    id: uuid::Uuid,
    owner_id: uuid::Uuid,
    album_id: Option<uuid::Uuid>,
    storage_key: String,
    thumb_key: Option<String>,
    filename: String,
    original_name: String,
    content_type: String,
    ext: String,
    size: i64,
    width: Option<i32>,
    height: Option<i32>,
    sha: String,
    privacy: String,
    tags: Vec<String>,
    expires_at: Option<DateTime<Utc>>,
}

async fn find_owner_sha(
    state: &AppState,
    owner_id: uuid::Uuid,
    sha: &str,
) -> AppResult<Option<Image>> {
    let sql = format!(
        "SELECT {} FROM images WHERE owner_id = $1 AND sha256 = $2 AND deleted_at IS NULL LIMIT 1",
        Image::COLUMNS
    );
    Ok(sqlx::query_as::<_, Image>(&sql)
        .bind(owner_id)
        .bind(sha)
        .fetch_optional(state.db())
        .await?)
}

async fn quota_has_room(state: &AppState, owner_id: uuid::Uuid, size: i64) -> AppResult<()> {
    let allowed: Option<bool> = sqlx::query_scalar(
        "SELECT (quota_bytes = 0 OR used_bytes + $1 <= quota_bytes) FROM users WHERE id = $2",
    )
    .bind(size)
    .bind(owner_id)
    .fetch_optional(state.db())
    .await?;
    match allowed {
        Some(true) => Ok(()),
        Some(false) => Err(AppError::QuotaExceeded),
        None => Err(AppError::Unauthorized),
    }
}

async fn commit_new_image(state: &AppState, img: NewImage) -> Result<Image, CommitError> {
    let mut tx = state
        .db()
        .begin()
        .await
        .map_err(|e| CommitError::Other(e.into()))?;
    let bumped = sqlx::query(
        "UPDATE users SET used_bytes = used_bytes + $1 \
         WHERE id = $2 AND (quota_bytes = 0 OR used_bytes + $1 <= quota_bytes)",
    )
    .bind(img.size)
    .bind(img.owner_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| CommitError::Other(e.into()))?;
    if bumped.rows_affected() == 0 {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
            .bind(img.owner_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| CommitError::Other(e.into()))?;
        return Err(if exists {
            CommitError::Quota
        } else {
            CommitError::Gone
        });
    }

    let sql = format!(
        "INSERT INTO images \
         (id, owner_id, album_id, source_kind, storage_key, thumb_key, filename, original_name, \
          content_type, ext, size_bytes, width, height, sha256, privacy, tags, expires_at) \
         VALUES ($1,$2,$3,'upload',$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14::image_privacy,$15,$16) \
         RETURNING {}",
        Image::COLUMNS
    );
    let inserted = sqlx::query_as::<_, Image>(&sql)
        .bind(img.id)
        .bind(img.owner_id)
        .bind(img.album_id)
        .bind(&img.storage_key)
        .bind(&img.thumb_key)
        .bind(&img.filename)
        .bind(&img.original_name)
        .bind(&img.content_type)
        .bind(&img.ext)
        .bind(img.size)
        .bind(img.width)
        .bind(img.height)
        .bind(&img.sha)
        .bind(&img.privacy)
        .bind(&img.tags)
        .bind(img.expires_at)
        .fetch_one(&mut *tx)
        .await;
    let row = match inserted {
        Ok(row) => row,
        Err(e) if is_unique_violation(&e) => return Err(CommitError::Duplicate),
        Err(e) if is_foreign_key_violation(&e) => {
            return Err(CommitError::Other(AppError::BadRequest(
                "相册不存在或不属于该用户".into(),
            )))
        }
        Err(e) => return Err(CommitError::Other(e.into())),
    };
    tx.commit()
        .await
        .map_err(|e| CommitError::Other(e.into()))?;
    Ok(row)
}

fn db_code(err: &sqlx::Error) -> Option<String> {
    err.as_database_error()
        .and_then(|d| d.code().map(|c| c.into_owned()))
}

fn is_unique_violation(err: &sqlx::Error) -> bool {
    db_code(err).as_deref() == Some("23505")
}

fn is_foreign_key_violation(err: &sqlx::Error) -> bool {
    db_code(err).as_deref() == Some("23503")
}

async fn remove_stored(state: &AppState, keys: &[String]) {
    for key in keys {
        if let Err(e) = state.storage().delete(key).await {
            tracing::warn!(key = %key, error = %e, "清理未提交的上传文件失败");
        }
    }
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
        total_incoming =
            total_incoming
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
    let resp = crate::netguard::guarded_get(state.http(), raw).await?;
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
