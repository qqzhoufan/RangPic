use axum::extract::{Path, Query, State};
use axum::routing::{get, patch};
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::QueryBuilder;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::db::Image;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

use super::image_json;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/my/images", get(list_my))
        .route("/api/my/images/{id}", patch(update_image))
        .route(
            "/api/my/images/{id}/delete",
            axum::routing::post(delete_image),
        )
        .route("/api/my/stats", get(stats_my))
}

type Router<S> = axum::Router<S>;

fn can_modify(img: &Image, user: &crate::db::User) -> bool {
    user.is_admin() || img.owner_id == Some(user.id)
}

struct Filters {
    uid: Uuid,
    privacy: String,
    tag: String,
    q: String,
    album: Option<Uuid>,
}

fn apply_filters(qb: &mut QueryBuilder<'_, sqlx::Postgres>, f: &Filters) {
    qb.push(" WHERE owner_id = ").push_bind(f.uid);
    qb.push(" AND deleted_at IS NULL");
    if !f.privacy.is_empty() {
        qb.push(" AND privacy = ").push_bind(f.privacy.clone());
    }
    if !f.tag.is_empty() {
        qb.push(
            " AND EXISTS (SELECT 1 FROM unnest(tags) AS t \
             WHERE LOWER(t) LIKE '%' || LOWER(",
        )
        .push_bind(f.tag.clone())
        .push(") || '%')");
    }
    if !f.q.is_empty() {
        qb.push(" AND (LOWER(original_name) LIKE '%' || LOWER(")
            .push_bind(f.q.clone())
            .push(") || '%' OR LOWER(filename) LIKE '%' || LOWER(")
            .push_bind(f.q.clone())
            .push(") || '%')");
    }
    if let Some(album) = f.album {
        qb.push(" AND album_id = ").push_bind(album);
    }
}

#[derive(Deserialize)]
struct ListQuery {
    page: Option<i64>,
    limit: Option<i64>,
    privacy: Option<String>,
    tag: Option<String>,
    q: Option<String>,
    album: Option<Uuid>,
}

async fn list_my(
    State(state): State<AppState>,
    user: AuthUser,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Value>> {
    let limit = q.limit.unwrap_or(24).clamp(1, 100);
    let offset = (q.page.unwrap_or(1).max(1) - 1) * limit;
    let uid = user.0.id;
    let f = Filters {
        uid,
        privacy: q.privacy.unwrap_or_default(),
        tag: q.tag.unwrap_or_default(),
        q: q.q.unwrap_or_default(),
        album: q.album,
    };

    let mut qbc = QueryBuilder::new("SELECT COUNT(*) FROM images");
    apply_filters(&mut qbc, &f);
    let total: i64 = qbc
        .build_query_scalar::<i64>()
        .fetch_one(state.db())
        .await?;

    let mut qbl = QueryBuilder::new(format!("SELECT {} FROM images", Image::COLUMNS).as_str());
    apply_filters(&mut qbl, &f);
    qbl.push(" ORDER BY created_at DESC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);
    let images: Vec<Image> = qbl.build_query_as::<Image>().fetch_all(state.db()).await?;

    let items: Vec<Value> = images
        .iter()
        .map(|i| image_json(i, Some(user.0.username.clone()), &state))
        .collect();
    Ok(Json(
        json!({ "items": items, "total": total, "limit": limit, "offset": offset }),
    ))
}

async fn stats_my(user: AuthUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let uid = user.0.id;
    let (count, used): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(size_bytes), 0)::bigint FROM images \
         WHERE owner_id = $1 AND deleted_at IS NULL",
    )
    .bind(uid)
    .fetch_one(state.db())
    .await?;
    let by_privacy: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT privacy::text, COUNT(*), COALESCE(SUM(size_bytes),0)::bigint FROM images \
         WHERE owner_id = $1 AND deleted_at IS NULL GROUP BY privacy",
    )
    .bind(uid)
    .fetch_all(state.db())
    .await?;
    Ok(Json(json!({
        "count": count,
        "used_bytes": used,
        "quota_bytes": user.0.quota_bytes,
        "by_privacy": by_privacy
            .into_iter()
            .map(|(p, c, s)| json!({ "privacy": p, "count": c, "bytes": s }))
            .collect::<Vec<_>>()
    })))
}

#[derive(Deserialize)]
struct UpdateImage {
    #[serde(default)]
    privacy: Option<String>,
    #[serde(default)]
    tags: Option<String>,
    #[serde(default)]
    album_id: Option<Option<Uuid>>,
    #[serde(default)]
    expires_hours: Option<String>,
}

async fn update_image(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateImage>,
) -> AppResult<Json<Value>> {
    let img = super::load_image(state.db(), id).await?;
    if !can_modify(&img, &user.0) {
        return Err(AppError::Forbidden);
    }
    if let Some(Some(aid)) = body.album_id {
        let owner_id = img
            .owner_id
            .ok_or_else(|| AppError::BadRequest("无法为无主图片设置相册".into()))?;
        super::ensure_album_owner(state.db(), aid, owner_id).await?;
    }

    let mut qb = QueryBuilder::new("UPDATE images SET ");
    let mut started = false;

    if let Some(p) = body.privacy {
        let p = if p.is_empty() {
            "public".to_string()
        } else {
            p
        };
        match p.as_str() {
            "public" | "unlisted" | "private" => {}
            _ => return Err(AppError::BadRequest("无效的隐私设置".into())),
        }
        qb.push("privacy = ").push_bind(p.clone());
        qb.push("::image_privacy");
        started = true;
    }
    if let Some(tags) = body.tags {
        let parsed: Vec<String> = tags
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        if started {
            qb.push(", ");
        }
        qb.push("tags = ").push_bind(parsed);
        started = true;
    }
    if let Some(album) = body.album_id {
        if started {
            qb.push(", ");
        }
        qb.push("album_id = ").push_bind(album);
        started = true;
    }
    if let Some(eh) = body.expires_hours {
        if started {
            qb.push(", ");
        }
        qb.push("expires_at = ").push_bind(parse_expiry(&eh)?);
        started = true;
    }

    if !started {
        let fresh = super::load_image(state.db(), id).await?;
        return Ok(Json(image_json(
            &fresh,
            Some(user.0.username.clone()),
            &state,
        )));
    }

    qb.push(" WHERE id = ").push_bind(id);
    qb.push(format!(" RETURNING {}", Image::COLUMNS));
    let updated: Image = qb.build_query_as::<Image>().fetch_one(state.db()).await?;
    Ok(Json(image_json(
        &updated,
        Some(user.0.username.clone()),
        &state,
    )))
}

async fn delete_image(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let img = super::load_image(state.db(), id).await?;
    if !can_modify(&img, &user.0) {
        return Err(AppError::Forbidden);
    }
    // Free storage + quota.
    if let Some(key) = &img.storage_key {
        let _ = state.storage().delete(key).await;
    }
    if let Some(key) = &img.thumb_key {
        let _ = state.storage().delete(key).await;
    }
    if let Some(owner) = img.owner_id {
        let _ =
            sqlx::query("UPDATE users SET used_bytes = GREATEST(used_bytes - $1, 0) WHERE id = $2")
                .bind(img.size_bytes)
                .bind(owner)
                .execute(state.db())
                .await;
    }
    let res = sqlx::query("DELETE FROM images WHERE id = $1")
        .bind(id)
        .execute(state.db())
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(json!({ "ok": true })))
}

fn parse_expiry(s: &str) -> AppResult<Option<DateTime<Utc>>> {
    let s = s.trim();
    if s.is_empty() || s == "0" {
        return Ok(None);
    }
    let h: i64 = s
        .parse()
        .map_err(|_| AppError::BadRequest("expires_hours 需为数字".into()))?;
    if h <= 0 {
        return Ok(None);
    }
    Ok(Some(Utc::now() + Duration::hours(h.min(87600))))
}
