use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

// NOTE on enums: `role`, `source_kind`, `privacy` are Postgres enums. We cast
// them to ::text in every SELECT and read them as String, so we don't need a
// sqlx Type derivation and the row mappings stay simple.

/// Columns for an `images` row, with enums already cast to text. Reuse via
/// `format!("SELECT {} FROM images", Image::COLUMNS)` in queries.
pub const IMAGE_COLUMNS: &str = "\
id, owner_id, album_id, source_kind::text AS source_kind, url, storage_key, thumb_key, \
filename, original_name, content_type, ext, size_bytes, width, height, sha256, \
privacy::text AS privacy, tags, views, created_at, expires_at, deleted_at";

#[derive(Clone, Debug, FromRow, Serialize)]
pub struct Image {
    pub id: Uuid,
    pub owner_id: Option<Uuid>,
    pub album_id: Option<Uuid>,
    pub source_kind: String,
    pub url: String,
    pub storage_key: Option<String>,
    pub thumb_key: Option<String>,
    pub filename: String,
    pub original_name: String,
    pub content_type: String,
    pub ext: String,
    pub size_bytes: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub sha256: Option<String>,
    pub privacy: String,
    pub tags: Vec<String>,
    pub views: i64,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip)]
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Image {
    pub const COLUMNS: &'static str = IMAGE_COLUMNS;

    /// `true` when this image is part of the public random-image pool.
    pub fn in_random_pool(&self) -> bool {
        self.deleted_at.is_none()
            && self.privacy == "public"
            && self.expires_at.map_or(true, |e| e > Utc::now())
    }

    pub fn is_local(&self) -> bool {
        self.source_kind == "upload" || self.source_kind == "local"
    }
}

pub const USER_COLUMNS: &str = "id, username, password_hash, role::text AS role, \
quota_bytes, used_bytes, banned, created_at";

#[derive(Clone, Debug, FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub quota_bytes: i64,
    pub used_bytes: i64,
    pub banned: bool,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

pub const ALBUM_COLUMNS: &str = "id, owner_id, name, slug, description, privacy::text AS privacy, created_at";

#[derive(Clone, Debug, FromRow, Serialize)]
pub struct Album {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub privacy: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, FromRow, Serialize)]
pub struct ApiToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub prefix: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
