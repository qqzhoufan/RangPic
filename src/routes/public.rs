use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::Response;
use axum::routing::get;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::MaybeAuthUser;
use crate::db::{Image, User};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

use super::{
    base_url, bump_views, ensure_viewable, if_none_match, image_json, load_image, not_modified,
    render_image, serve_thumb,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        // flagship random-image API
        .route("/api/random-image", get(random_image_api))
        .route("/random-image", get(random_image_proxy))
        .route("/api/tags", get(tags_api))
        // public catalog
        .route("/api/explore", get(explore))
        .route("/api/images/{id}", get(get_image_public))
        .route("/api/site", get(site_api))
        .route("/api/me", get(me_api))
        // image bytes
        .route("/i/{id}", get(serve_original))
        .route("/t/{id}", get(serve_thumb_route))
}

type Router<S> = axum::Router<S>;

#[derive(Deserialize, Default)]
pub struct RandomQuery {
    pub tags: Option<String>,
    pub tag: Option<String>,
    /// Public folder scope: a album uuid, or `owner/slug`. Album must be public.
    pub album: Option<String>,
    /// User scope: a username — random from that user's public images.
    pub user: Option<String>,
}

impl RandomQuery {
    fn tag(&self) -> String {
        self.tags
            .as_deref()
            .or(self.tag.as_deref())
            .unwrap_or("")
            .trim()
            .to_string()
    }
}

#[derive(Default)]
struct RandomScope {
    user_id: Option<Uuid>,
    album_id: Option<Uuid>,
}

/// Resolve `?album=` and `?user=` into concrete ids. Albums must be public.
async fn resolve_scope(db: &sqlx::PgPool, q: &RandomQuery) -> AppResult<RandomScope> {
    let album_id = match q.album.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(a) => Some(resolve_album(db, a).await?),
        None => None,
    };
    let user_id = match q.user.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(u) => Some(resolve_user(db, u).await.ok_or(AppError::NotFound)?),
        None => None,
    };
    Ok(RandomScope { user_id, album_id })
}

async fn resolve_album(db: &sqlx::PgPool, param: &str) -> AppResult<Uuid> {
    if let Ok(id) = Uuid::parse_str(param) {
        let public: Option<bool> =
            sqlx::query_scalar("SELECT (privacy = 'public') FROM albums WHERE id = $1")
                .bind(id)
                .fetch_optional(db)
                .await?;
        return match public {
            Some(true) => Ok(id),
            _ => Err(AppError::NotFound),
        };
    }
    if let Some((owner, slug)) = param.split_once('/') {
        let id: Option<Uuid> = sqlx::query_scalar(
            "SELECT a.id FROM albums a JOIN users u ON u.id = a.owner_id \
             WHERE LOWER(u.username) = LOWER($1) AND a.slug = $2 AND a.privacy = 'public'",
        )
        .bind(owner.trim())
        .bind(slug.trim())
        .fetch_optional(db)
        .await?;
        return id.ok_or(AppError::NotFound);
    }
    Err(AppError::BadRequest(
        "album 参数需为相册 ID 或 用户名/Slug".into(),
    ))
}

async fn resolve_user(db: &sqlx::PgPool, username: &str) -> Option<Uuid> {
    sqlx::query_scalar("SELECT id FROM users WHERE LOWER(username) = LOWER($1) AND NOT banned")
        .bind(username)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
}

/// Pick uniformly via `rand_key` (indexed) instead of sorting the whole pool.
/// A miss above the threshold wraps to the smallest key.
async fn choose_random(
    db: &sqlx::PgPool,
    tag: String,
    scope: &RandomScope,
) -> AppResult<Option<Image>> {
    let threshold: f64 = rand::random();
    if let Some(img) = pick_random(db, &tag, scope, Some(threshold)).await? {
        return Ok(Some(img));
    }
    pick_random(db, &tag, scope, None).await
}

async fn pick_random(
    db: &sqlx::PgPool,
    tag: &str,
    scope: &RandomScope,
    min_key: Option<f64>,
) -> AppResult<Option<Image>> {
    let sql = format!(
        "SELECT {} FROM images \
         WHERE deleted_at IS NULL AND privacy = 'public' \
           AND (expires_at IS NULL OR expires_at > now()) \
           AND (pool_excluded_until IS NULL OR pool_excluded_until <= now()) \
           AND ($1 = '' OR EXISTS (SELECT 1 FROM unnest(tags) AS t WHERE LOWER(t) LIKE LOWER('%' || $1 || '%'))) \
           AND ($2::uuid IS NULL OR owner_id = $2) \
           AND ($3::uuid IS NULL OR album_id = $3) \
           AND ($4::float8 IS NULL OR rand_key >= $4) \
         ORDER BY rand_key LIMIT 1",
        Image::COLUMNS
    );
    let img = sqlx::query_as::<_, Image>(&sql)
        .bind(tag)
        .bind(scope.user_id)
        .bind(scope.album_id)
        .bind(min_key)
        .fetch_optional(db)
        .await?;
    Ok(img)
}

async fn exclude_from_pool(db: &sqlx::PgPool, id: Uuid) {
    if let Err(e) = sqlx::query(
        "UPDATE images SET pool_excluded_until = now() + interval '30 minutes' WHERE id = $1",
    )
    .bind(id)
    .execute(db)
    .await
    {
        tracing::debug!(error = %e, %id, "暂时移出随机池失败");
    }
}

/// `GET /api/random-image?tags=&album=&user=` — JSON metadata for a random image.
async fn random_image_api(
    State(state): State<AppState>,
    Query(q): Query<RandomQuery>,
) -> AppResult<Json<Value>> {
    let scope = resolve_scope(state.db(), &q).await?;
    let tag = q.tag();
    let img = choose_random(state.db(), tag.clone(), &scope)
        .await?
        .ok_or(AppError::NotFound)?;
    let mut json = image_json(&img, None, &state);
    json["query"] = json!({ "tag": tag, "album": q.album, "user": q.user });
    Ok(Json(json))
}

/// `GET /random-image?tags=&album=&user=` — raw image bytes for use as an `<img src>`.
/// Retries up to a few times so a dead remote entry in the pool doesn't yield a
/// broken image; uploaded/local entries always succeed on the first try.
async fn random_image_proxy(
    State(state): State<AppState>,
    Query(q): Query<RandomQuery>,
) -> AppResult<Response> {
    let scope = resolve_scope(state.db(), &q).await?;
    let tag = q.tag();
    let mut last_err: Option<AppError> = None;
    for _ in 0..6 {
        let img = match choose_random(state.db(), tag.clone(), &scope).await? {
            Some(i) => i,
            None => break,
        };
        let tried = if img.is_local() {
            render_image(&state, &img).await
        } else {
            super::fetch_remote_image(&state, &img.url).await
        };
        match tried {
            Ok(mut resp) => {
                bump_views(&state, img.id);
                resp.headers_mut().insert(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("no-cache, no-store, must-revalidate"),
                );
                resp.headers_mut().remove(header::ETAG);
                return Ok(resp);
            }
            Err(e) => {
                tracing::debug!(image_id = %img.id, error = %e, "random pick failed, retrying");
                exclude_from_pool(state.db(), img.id).await;
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap_or(AppError::NotFound))
}

async fn tags_api(State(state): State<AppState>) -> AppResult<Json<Value>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT tag FROM ( \
           SELECT unnest(tags) AS tag FROM images \
           WHERE deleted_at IS NULL AND privacy = 'public' \
             AND (expires_at IS NULL OR expires_at > now()) \
         ) t WHERE tag <> '' ORDER BY tag",
    )
    .fetch_all(state.db())
    .await?;
    let tags: Vec<String> = rows.into_iter().map(|(t,)| t).collect();
    Ok(Json(json!(tags)))
}

#[derive(Deserialize)]
pub struct PageQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

impl PageQuery {
    fn clamp(self) -> (i64, i64) {
        let limit = self.limit.unwrap_or(24).clamp(1, 60);
        let page = self.page.unwrap_or(1).max(1);
        (limit, (page - 1) * limit)
    }
}

/// `GET /api/explore` — newest public images for the public gallery grid.
async fn explore(
    State(state): State<AppState>,
    Query(q): Query<PageQuery>,
) -> AppResult<Json<Value>> {
    let (limit, offset) = q.clamp();
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM images WHERE deleted_at IS NULL AND privacy = 'public' \
         AND (expires_at IS NULL OR expires_at > now())",
    )
    .fetch_one(state.db())
    .await?;
    let images: Vec<Image> = sqlx::query_as::<_, Image>(&format!(
        "SELECT {} FROM images WHERE deleted_at IS NULL AND privacy = 'public' \
         AND (expires_at IS NULL OR expires_at > now()) \
         ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        Image::COLUMNS
    ))
    .bind(limit)
    .bind(offset)
    .fetch_all(state.db())
    .await?;
    let names = owner_names(state.db(), &images).await?;
    let items: Vec<Value> = images
        .iter()
        .map(|i| {
            let owner_name = i.owner_id.and_then(|id| names.get(&id).cloned());
            image_json(i, owner_name, &state)
        })
        .collect();
    Ok(Json(
        json!({ "items": items, "total": total, "limit": limit, "offset": offset }),
    ))
}

/// `GET /api/images/{id}` — public metadata (gated by privacy).
async fn get_image_public(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    user: MaybeAuthUser,
) -> AppResult<Json<Value>> {
    let img = load_image(state.db(), id).await?;
    ensure_viewable(&img, user.0.as_ref())?;
    let owner_name = owner_name(&state, &img).await;
    Ok(Json(image_json(&img, owner_name, &state)))
}

async fn owner_names(db: &sqlx::PgPool, images: &[Image]) -> AppResult<HashMap<Uuid, String>> {
    let mut ids: Vec<Uuid> = images.iter().filter_map(|i| i.owner_id).collect();
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT id, username FROM users WHERE id = ANY($1)")
            .bind(&ids)
            .fetch_all(db)
            .await?;
    Ok(rows.into_iter().collect())
}

async fn owner_name(state: &AppState, img: &Image) -> Option<String> {
    let owner_id = img.owner_id?;
    let row: Option<(String,)> = sqlx::query_as("SELECT username FROM users WHERE id = $1")
        .bind(owner_id)
        .fetch_optional(state.db())
        .await
        .ok()?;
    row.map(|(n,)| n)
}

/// `GET /i/{id}` — original image bytes (privacy-gated; unlisted = anyone with the link).
async fn serve_original(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    user: MaybeAuthUser,
) -> AppResult<Response> {
    let img = load_image(state.db(), id).await?;
    ensure_viewable(&img, user.0.as_ref())?;
    if if_none_match(&headers, &img) {
        return Ok(not_modified(&img));
    }
    bump_views(&state, img.id);
    render_image(&state, &img).await
}

/// `GET /t/{id}` — thumbnail (same gating as original).
async fn serve_thumb_route(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    user: MaybeAuthUser,
) -> AppResult<Response> {
    let img = load_image(state.db(), id).await?;
    ensure_viewable(&img, user.0.as_ref())?;
    if if_none_match(&headers, &img) {
        return Ok(not_modified(&img));
    }
    if let Ok(resp) = serve_thumb(&state, &img).await {
        return Ok(resp);
    }
    // No thumbnail (e.g. remote entry): fall back to the original / proxy.
    render_image(&state, &img).await
}

#[derive(Serialize)]
pub struct UserSummary {
    pub id: Uuid,
    pub username: String,
    pub role: String,
    pub used_bytes: i64,
    pub quota_bytes: i64,
}
impl From<User> for UserSummary {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            username: u.username,
            role: u.role,
            used_bytes: u.used_bytes,
            quota_bytes: u.quota_bytes,
        }
    }
}

async fn settings_map(db: &sqlx::PgPool) -> AppResult<HashMap<String, String>> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
        .fetch_all(db)
        .await?;
    Ok(rows.into_iter().collect())
}

async fn site_api(State(state): State<AppState>, user: MaybeAuthUser) -> AppResult<Json<Value>> {
    let s = settings_map(state.db()).await?;
    let you = user.0.map(UserSummary::from);
    Ok(Json(json!({
        "site_name": s.get("site_name").cloned().unwrap_or_else(|| state.cfg().site_name.clone()),
        "description": s.get("site_description").cloned().unwrap_or_default(),
        "registration_open": s.get("registration_open").map(|v| v == "true").unwrap_or(state.cfg().registration_open),
        "index_blurb": s.get("index_blurb").cloned().unwrap_or_default(),
        "version": env!("CARGO_PKG_VERSION"),
        "base_url": base_url(&state),
        "you": you,
    })))
}

async fn me_api(user: MaybeAuthUser) -> AppResult<Json<Value>> {
    Ok(Json(json!({ "you": user.0.map(UserSummary::from) })))
}
