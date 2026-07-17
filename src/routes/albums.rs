use axum::extract::{Path, State};
use axum::routing::get;
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::QueryBuilder;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::db::{Album, ALBUM_COLUMNS};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/albums", get(list).post(create))
        .route("/api/albums/{id}", get(detail).patch(update).delete(remove))
}

type Router<S> = axum::Router<S>;

#[derive(FromRow, Serialize)]
struct AlbumListItem {
    id: Uuid,
    owner_id: Uuid,
    name: String,
    slug: String,
    description: String,
    privacy: String,
    created_at: chrono::DateTime<chrono::Utc>,
    image_count: i64,
    cover_id: Option<Uuid>,
}

async fn list(user: AuthUser, State(state): State<AppState>) -> AppResult<Json<Vec<AlbumListItem>>> {
    let rows: Vec<AlbumListItem> = sqlx::query_as(
        "SELECT a.id, a.owner_id, a.name, a.slug, a.description, a.privacy::text AS privacy, \
                a.created_at, \
                COUNT(i.id)::bigint AS image_count, \
                (SELECT i2.id FROM images i2 WHERE i2.album_id = a.id AND i2.deleted_at IS NULL \
                 ORDER BY i2.created_at DESC LIMIT 1) AS cover_id \
         FROM albums a \
         LEFT JOIN images i ON i.album_id = a.id AND i.deleted_at IS NULL \
         WHERE a.owner_id = $1 \
         GROUP BY a.id \
         ORDER BY a.created_at DESC",
    )
    .bind(user.0.id)
    .fetch_all(state.db())
    .await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
struct CreateAlbum {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    privacy: String,
}

fn slugify(s: &str) -> String {
    let s: String = s
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "album".to_string()
    } else {
        s.chars().take(60).collect()
    }
}

fn privacy_or_default(s: &str) -> Result<String, AppError> {
    Ok(match s {
        "" | "public" => "public".to_string(),
        "unlisted" => "unlisted".to_string(),
        "private" => "private".to_string(),
        _ => return Err(AppError::BadRequest("无效的隐私设置".into())),
    })
}

async fn create(
    user: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateAlbum>,
) -> AppResult<Json<Album>> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("相册名不能为空".into()));
    }
    let privacy = privacy_or_default(&body.privacy)?;
    let mut slug = slugify(name);

    // retry once on slug collision
    for attempt in 0..2u8 {
        let res = sqlx::query(
            "INSERT INTO albums (owner_id, name, slug, description, privacy) \
             VALUES ($1, $2, $3, $4, $5::image_privacy)",
        )
        .bind(user.0.id)
        .bind(name)
        .bind(&slug)
        .bind(&body.description)
        .bind(&privacy)
        .execute(state.db())
        .await;
        match res {
            Ok(_) => break,
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() && attempt == 0 => {
                slug = format!("{slug}-{}", &Uuid::new_v4().simple().to_string()[..6]);
            }
            Err(e) => return Err(e.into()),
        }
    }

    let album: Album = sqlx::query_as::<_, Album>(&format!(
        "SELECT {ALBUM_COLUMNS} FROM albums WHERE owner_id = $1 AND slug = $2"
    ))
    .bind(user.0.id)
    .bind(&slug)
    .fetch_one(state.db())
    .await?;
    Ok(Json(album))
}

async fn detail(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Album>> {
    let album: Album = sqlx::query_as::<_, Album>(&format!(
        "SELECT {ALBUM_COLUMNS} FROM albums WHERE id = $1 AND owner_id = $2"
    ))
    .bind(id)
    .bind(user.0.id)
    .fetch_optional(state.db())
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(album))
}

#[derive(Deserialize)]
struct UpdateAlbum {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    privacy: Option<String>,
}

async fn update(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateAlbum>,
) -> AppResult<Json<Album>> {
    let mut qb = QueryBuilder::new("UPDATE albums SET ");
    let mut started = false;
    if let Some(name) = body.name {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::BadRequest("相册名不能为空".into()));
        }
        qb.push("name = ").push_bind(name.to_string());
        qb.push(", slug = ").push_bind(slugify(&name));
        started = true;
    }
    if let Some(desc) = body.description {
        if started {
            qb.push(", ");
        }
        qb.push("description = ").push_bind(desc);
        started = true;
    }
    if let Some(privacy) = body.privacy {
        let privacy = privacy_or_default(&privacy)?;
        if started {
            qb.push(", ");
        }
        qb.push("privacy = ").push_bind(privacy);
        qb.push("::image_privacy");
        started = true;
    }
    if !started {
        qb.push("name = name");
    }
    qb.push(" WHERE id = ").push_bind(id);
    qb.push(" AND owner_id = ").push_bind(user.0.id);
    qb.push(format!(" RETURNING {ALBUM_COLUMNS}"));
    let album: Album = qb.build_query_as::<Album>().fetch_optional(state.db()).await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(album))
}

async fn remove(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let res = sqlx::query("DELETE FROM albums WHERE id = $1 AND owner_id = $2")
        .bind(id)
        .bind(user.0.id)
        .execute(state.db())
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}
