use axum::extract::{Path, Query, State};
use axum::routing::{delete, get, patch};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

use crate::auth::AdminUser;
use crate::db::{Image, User, USER_COLUMNS};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

use super::image_json;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/stats", get(stats))
        .route("/api/admin/users", get(list_users))
        .route("/api/admin/users/{id}", patch(update_user).delete(delete_user))
        .route("/api/admin/images", get(list_images))
        .route("/api/admin/images/{id}", delete(delete_image))
        .route("/api/admin/settings", get(get_settings).put(set_settings))
}

type Router<S> = axum::Router<S>;

async fn stats(AdminUser(_): AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(state.db())
        .await?;
    let (images, storage, views): (i64, i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(size_bytes),0)::bigint, COALESCE(SUM(views),0)::bigint \
         FROM images WHERE deleted_at IS NULL",
    )
    .fetch_one(state.db())
    .await?;
    let public: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM images WHERE deleted_at IS NULL AND privacy='public' \
         AND (expires_at IS NULL OR expires_at > now())",
    )
    .fetch_one(state.db())
    .await?;
    let by_source: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT source_kind::text, COUNT(*), COALESCE(SUM(size_bytes),0)::bigint \
         FROM images WHERE deleted_at IS NULL GROUP BY source_kind",
    )
    .fetch_all(state.db())
    .await?;
    let last_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM images WHERE created_at > now() - interval '24 hours' AND deleted_at IS NULL",
    )
    .fetch_one(state.db())
    .await?;
    Ok(Json(json!({
        "users": users,
        "images": images,
        "storage_bytes": storage,
        "views": views,
        "public_pool": public,
        "uploads_24h": last_24h,
        "by_source": by_source.into_iter().map(|(s,c,b)| json!({"source":s,"count":c,"bytes":b})).collect::<Vec<_>>()
    })))
}

async fn list_users(
    AdminUser(_): AdminUser,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<User>>> {
    let users: Vec<User> = sqlx::query_as::<_, User>(&format!(
        "SELECT {USER_COLUMNS} FROM users ORDER BY created_at DESC"
    ))
    .fetch_all(state.db())
    .await?;
    Ok(Json(users))
}

#[derive(Deserialize)]
struct UpdateUser {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    quota_bytes: Option<i64>,
    #[serde(default)]
    banned: Option<bool>,
}

async fn update_user(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateUser>,
) -> AppResult<Json<Value>> {
    if id == admin.0.id {
        // don't let an admin demote or ban themselves
        if matches!(body.role.as_deref(), Some("user")) || body.banned == Some(true) {
            return Err(AppError::BadRequest("不能对自己执行降级或封禁".into()));
        }
    }
    let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new("UPDATE users SET ");
    let mut started = false;
    if let Some(role) = body.role {
        match role.as_str() {
            "admin" | "user" => {}
            _ => return Err(AppError::BadRequest("无效的角色".into())),
        }
        qb.push("role = ").push_bind(role);
        qb.push("::user_role");
        started = true;
    }
    if let Some(q) = body.quota_bytes {
        if started {
            qb.push(", ");
        }
        qb.push("quota_bytes = ").push_bind(q.max(0));
        started = true;
    }
    if let Some(banned) = body.banned {
        if started {
            qb.push(", ");
        }
        qb.push("banned = ").push_bind(banned);
        started = true;
    }
    if !started {
        return Err(AppError::BadRequest("没有需要更新的字段".into()));
    }
    qb.push(" WHERE id = ").push_bind(id);
    qb.push(format!(" RETURNING {USER_COLUMNS}"));
    let user: User = qb
        .build_query_as::<User>()
        .fetch_optional(state.db())
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(json!({ "user": user })))
}

async fn delete_user(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    if id == admin.0.id {
        return Err(AppError::BadRequest("不能删除自己".into()));
    }
    let res = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(state.db())
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct ImageListQuery {
    page: Option<i64>,
    limit: Option<i64>,
}

async fn list_images(
    AdminUser(_): AdminUser,
    State(state): State<AppState>,
    Query(q): Query<ImageListQuery>,
) -> AppResult<Json<Value>> {
    let limit = q.limit.unwrap_or(48).clamp(1, 200);
    let offset = (q.page.unwrap_or(1).max(1) - 1) * limit;
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM images WHERE deleted_at IS NULL")
        .fetch_one(state.db())
        .await?;
    let images: Vec<Image> = sqlx::query_as::<_, Image>(&format!(
        "SELECT {} FROM images WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        Image::COLUMNS
    ))
    .bind(limit)
    .bind(offset)
    .fetch_all(state.db())
    .await?;
    let items: Vec<Value> = images.iter().map(|i| image_json(i, None, &state)).collect();
    Ok(Json(json!({ "items": items, "total": total, "limit": limit, "offset": offset })))
}

async fn delete_image(
    AdminUser(_): AdminUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let img = super::load_image(state.db(), id).await?;
    if let Some(key) = &img.storage_key {
        let _ = state.storage().delete(key).await;
    }
    if let Some(key) = &img.thumb_key {
        let _ = state.storage().delete(key).await;
    }
    if let Some(owner) = img.owner_id {
        let _ = sqlx::query("UPDATE users SET used_bytes = GREATEST(used_bytes - $1, 0) WHERE id = $2")
            .bind(img.size_bytes)
            .bind(owner)
            .execute(state.db())
            .await;
    }
    sqlx::query("DELETE FROM images WHERE id = $1")
        .bind(id)
        .execute(state.db())
        .await?;
    Ok(Json(json!({ "ok": true })))
}

const ALLOWED_SETTINGS: &[&str] = &[
    "site_name",
    "site_description",
    "registration_open",
    "index_blurb",
    "site_url",
    "default_quota_bytes",
];

async fn get_settings(AdminUser(_): AdminUser, State(state): State<AppState>) -> AppResult<Json<HashMap<String, String>>> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
        .fetch_all(state.db())
        .await?;
    Ok(Json(rows.into_iter().collect()))
}

async fn set_settings(
    AdminUser(_): AdminUser,
    State(state): State<AppState>,
    Json(body): Json<HashMap<String, String>>,
) -> AppResult<Json<Value>> {
    for (k, v) in &body {
        if !ALLOWED_SETTINGS.contains(&k.as_str()) {
            continue;
        }
        sqlx::query("INSERT INTO settings (key, value) VALUES ($1, $2) ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value")
            .bind(k)
            .bind(v)
            .execute(state.db())
            .await?;
        // keep the cached base URL in sync so absolute share links update live
        if k == "site_url" {
            state.set_base_url(v.clone());
        }
    }
    Ok(Json(json!({ "ok": true })))
}
