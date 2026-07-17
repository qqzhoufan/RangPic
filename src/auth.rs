use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::db::User;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

// ---- password hashing (argon2id) ----

pub fn hash_password(pw: &str) -> AppResult<String> {
    use argon2::password_hash::rand_core::OsRng;
    use argon2::password_hash::SaltString;
    use argon2::{Argon2, PasswordHasher};
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Other(anyhow::anyhow!("argon2 hash failed: {e}")))
}

pub fn verify_password(pw: &str, phc: &str) -> bool {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    let Ok(parsed) = PasswordHash::new(phc) else {
        return false;
    };
    Argon2::default()
        .verify_password(pw.as_bytes(), &parsed)
        .is_ok()
}

// ---- random tokens ----

fn random_hex(n_bytes: usize) -> String {
    let mut buf = vec![0u8; n_bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    hex::encode(buf)
}

/// A browser session token (stored verbatim in the `sessions` table).
pub fn new_session_token() -> String {
    random_hex(32)
}

/// A new API token. Returns (raw_token_to_show_once, sha256_hash_for_db, prefix_for_display).
pub fn new_api_token() -> (String, String, String) {
    let raw = format!("rpic_{}", random_hex(24));
    let hash = token_hash(&raw);
    let prefix = raw.chars().take(12).collect();
    (raw, hash, prefix)
}

pub fn token_hash(raw: &str) -> String {
    let mut h = Sha256::new();
    h.update(raw.as_bytes());
    hex::encode(h.finalize())
}

fn cookie_value<'a>(headers: &'a http::HeaderMap, name: &str) -> Option<&'a str> {
    let raw = headers.get(http::header::COOKIE)?.to_str().ok()?;
    for pair in raw.split(';') {
        let pair = pair.trim();
        if let Some((k, v)) = pair.split_once('=') {
            if k.trim() == name {
                return Some(v.trim());
            }
        }
    }
    None
}

/// Resolve the user (and whether the call used a bearer API token) from a request.
pub async fn resolve(parts: &Parts, state: &AppState) -> Option<(User, bool)> {
    // 1) bearer API token
    if let Some(auth) = parts.headers.get(http::header::AUTHORIZATION) {
        if let Ok(s) = auth.to_str() {
            if let Some(tok) = s.strip_prefix("Bearer ").map(str::trim) {
                if let Some(user) = user_by_api_token(state, tok).await {
                    return Some((user, true));
                }
            }
        }
    }
    // 2) session cookie
    if let Some(tok) = cookie_value(&parts.headers, "session") {
        if let Some(user) = user_by_session(state, tok).await {
            return Some((user, false));
        }
    }
    None
}

async fn user_by_session(state: &AppState, token: &str) -> Option<User> {
    let row = sqlx::query_as::<_, User>(&format!(
        "SELECT {} FROM users WHERE id = (SELECT user_id FROM sessions WHERE token = $1 AND expires_at > now())",
        crate::db::USER_COLUMNS
    ))
    .bind(token)
    .fetch_optional(state.db())
    .await
    .ok()??;
    (!row.banned).then_some(row)
}

async fn user_by_api_token(state: &AppState, raw: &str) -> Option<User> {
    let hash = token_hash(raw);
    let row = sqlx::query_as::<_, User>(&format!(
        "SELECT {} FROM users WHERE id = (SELECT user_id FROM api_tokens WHERE token_hash = $1)",
        crate::db::USER_COLUMNS
    ))
    .bind(&hash)
    .fetch_optional(state.db())
    .await
    .ok()??;
    if row.banned {
        return None;
    }
    // best-effort last-used update
    let _ = sqlx::query("UPDATE api_tokens SET last_used_at = now() WHERE token_hash = $1")
        .bind(&hash)
        .execute(state.db())
        .await;
    Some(row)
}

// ---- extractors ----

/// Requires an authenticated user; rejects with 401 otherwise.
#[derive(Clone, Debug)]
pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        match resolve(parts, state).await {
            Some((user, _)) => Ok(AuthUser(user)),
            None => Err(AppError::Unauthorized),
        }
    }
}

/// Optional auth — never rejects; `user` is None when anonymous.
#[derive(Clone, Debug)]
pub struct MaybeAuthUser(pub Option<User>);

impl FromRequestParts<AppState> for MaybeAuthUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        Ok(MaybeAuthUser(resolve(parts, state).await.map(|(u, _)| u)))
    }
}

/// Requires an admin.
#[derive(Clone, Debug)]
pub struct AdminUser(pub User);

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        match resolve(parts, state).await {
            Some((user, _)) if user.is_admin() => Ok(AdminUser(user)),
            Some(_) => Err(AppError::Forbidden),
            None => Err(AppError::Unauthorized),
        }
    }
}
