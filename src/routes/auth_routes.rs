use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::Json;
use axum_extra::extract::CookieJar;
use cookie::time::Duration as CookieDuration;
use cookie::{Cookie, SameSite};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::{new_api_token, new_session_token, AuthUser};
use crate::db::{ApiToken, User};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

use super::public::UserSummary;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/account", get(account))
        .route("/api/account/password", post(change_password))
        .route("/api/account/tokens", get(list_tokens).post(create_token))
        .route("/api/account/tokens/{id}", delete(delete_token))
}

type Router<S> = axum::Router<S>;

const SESSION_COOKIE: &str = "session";

#[derive(Deserialize)]
struct Credentials {
    username: String,
    password: String,
}

fn validate_username(name: &str) -> AppResult<()> {
    let len = name.chars().count();
    if !(3..=32).contains(&len) {
        return Err(AppError::BadRequest("用户名长度需为 3–32 个字符".into()));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(AppError::BadRequest(
            "用户名只能包含字母、数字、下划线与短横线".into(),
        ));
    }
    Ok(())
}

fn validate_password(pw: &str) -> AppResult<()> {
    if pw.chars().count() < 6 {
        return Err(AppError::BadRequest("密码至少 6 个字符".into()));
    }
    if pw.chars().count() > 200 {
        return Err(AppError::BadRequest("密码过长".into()));
    }
    Ok(())
}

async fn registration_open(state: &AppState) -> bool {
    let Ok(row): Result<(String,), _> =
        sqlx::query_as("SELECT value FROM settings WHERE key = 'registration_open'")
            .fetch_one(state.db())
            .await
    else {
        return state.cfg().registration_open;
    };
    row.0 == "true"
}

/// Default per-user storage quota (bytes) for newly registered users.
/// Reads the `default_quota_bytes` admin setting; 0 means unlimited.
async fn default_quota_bytes(state: &AppState) -> i64 {
    if let Ok(Some((v,))) = sqlx::query_as::<_, (String,)>(
        "SELECT value FROM settings WHERE key = 'default_quota_bytes'",
    )
    .fetch_optional(state.db())
    .await
    {
        if let Ok(n) = v.trim().parse::<i64>() {
            return n.max(0);
        }
    }
    state.cfg().quota_default_bytes
}

fn make_session_cookie(token: &str, hours: i64) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE.to_string(), token.to_string()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(CookieDuration::hours(hours))
        .build()
}

async fn issue_session(
    state: &AppState,
    user: &User,
    jar: CookieJar,
    headers: &http::HeaderMap,
) -> AppResult<(CookieJar, UserSummary)> {
    let token = new_session_token();
    let ua = headers
        .get(http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.chars().take(255).collect::<String>());
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(state.cfg().session_ttl_hours);
    sqlx::query(
        "INSERT INTO sessions (token, user_id, expires_at, user_agent) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(&token)
    .bind(user.id)
    .bind(expires_at)
    .bind(ua)
    .execute(state.db())
    .await?;
    let jar = jar.add(make_session_cookie(&token, state.cfg().session_ttl_hours));
    Ok((jar, user.clone().into()))
}

async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: http::HeaderMap,
    Json(cred): Json<Credentials>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let username = cred.username.trim().to_string();
    validate_username(&username)?;
    validate_password(&cred.password)?;
    if !registration_open(&state).await {
        return Err(AppError::Forbidden);
    }
    // uniqueness check (the unique index also enforces this)
    let existing: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM users WHERE username = $1")
        .bind(&username)
        .fetch_optional(state.db())
        .await?;
    if existing.is_some() {
        return Err(AppError::Conflict("用户名已被占用".into()));
    }
    let hash = crate::auth::hash_password(&cred.password)?;
    let quota = default_quota_bytes(&state).await;
    let user: User = sqlx::query_as::<_, User>(&format!(
        "INSERT INTO users (username, password_hash, quota_bytes) VALUES ($1, $2, $3) \
         RETURNING {}",
        crate::db::USER_COLUMNS
    ))
    .bind(&username)
    .bind(hash)
    .bind(quota)
    .fetch_one(state.db())
    .await?;
    let (jar, summary) = issue_session(&state, &user, jar, &headers).await?;
    Ok((jar, Json(json!({ "you": summary }))))
}

async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: http::HeaderMap,
    Json(cred): Json<Credentials>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let username = cred.username.trim().to_string();
    let user: Option<User> = sqlx::query_as::<_, User>(&format!(
        "SELECT {} FROM users WHERE username = $1",
        crate::db::USER_COLUMNS
    ))
    .bind(&username)
    .fetch_optional(state.db())
    .await?;
    let user = match user {
        Some(u) if !u.banned && crate::auth::verify_password(&cred.password, &u.password_hash) => u,
        _ => return Err(AppError::Unauthorized),
    };
    let (jar, summary) = issue_session(&state, &user, jar, &headers).await?;
    Ok((jar, Json(json!({ "you": summary }))))
}

async fn logout(State(state): State<AppState>, jar: CookieJar) -> (CookieJar, Json<Value>) {
    if let Some(c) = jar.get(SESSION_COOKIE) {
        let token = c.value().to_string();
        let _ = sqlx::query("DELETE FROM sessions WHERE token = $1")
            .bind(token)
            .execute(state.db())
            .await;
    }
    let jar = jar.remove(make_removal_cookie());
    (jar, Json(json!({ "ok": true })))
}

fn make_removal_cookie() -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE.to_string(), String::new()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(CookieDuration::seconds(0))
        .build()
}

async fn account(user: AuthUser) -> Json<Value> {
    Json(json!({ "you": UserSummary::from(user.0) }))
}

#[derive(Deserialize)]
struct ChangePassword {
    current: String,
    new: String,
}

async fn change_password(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<ChangePassword>,
) -> AppResult<Json<Value>> {
    if !crate::auth::verify_password(&body.current, &user.0.password_hash) {
        return Err(AppError::Unauthorized);
    }
    validate_password(&body.new)?;
    if body.new == body.current {
        return Err(AppError::BadRequest("新密码不能与旧密码相同".into()));
    }
    let hash = crate::auth::hash_password(&body.new)?;
    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(hash)
        .bind(user.0.id)
        .execute(state.db())
        .await?;
    // Invalidate all sessions (force re-login).
    let _ = sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user.0.id)
        .execute(state.db())
        .await;
    Ok(Json(json!({ "ok": true })))
}

async fn list_tokens(
    user: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<ApiToken>>> {
    let rows: Vec<ApiToken> = sqlx::query_as::<_, ApiToken>(
        "SELECT id, user_id, name, prefix, last_used_at, created_at \
         FROM api_tokens WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user.0.id)
    .fetch_all(state.db())
    .await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
struct CreateToken {
    name: Option<String>,
}

async fn create_token(
    user: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateToken>,
) -> AppResult<Json<Value>> {
    let name = body.name.unwrap_or_default();
    let name = name.trim();
    let name = if name.is_empty() { "default" } else { name };
    let (raw, hash, prefix) = new_api_token();
    let row: ApiToken = sqlx::query_as::<_, ApiToken>(
        "INSERT INTO api_tokens (user_id, name, token_hash, prefix) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id, user_id, name, prefix, last_used_at, created_at",
    )
    .bind(user.0.id)
    .bind(name)
    .bind(&hash)
    .bind(&prefix)
    .fetch_one(state.db())
    .await?;
    Ok(Json(json!({ "token": raw, "info": row })))
}

async fn delete_token(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let res = sqlx::query("DELETE FROM api_tokens WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.0.id)
        .execute(state.db())
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(json!({ "ok": true })))
}
