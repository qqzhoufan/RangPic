mod assets;
mod auth;
mod config;
mod db;
mod error;
mod imaging;
mod netguard;
mod routes;
mod state;
mod storage;

use config::Config;
use state::AppState;
use storage::LocalStorage;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,rangpic=debug,sqlx=warn".into()),
        )
        .with_target(false)
        .init();

    let config = Config::from_env()?;
    tracing::info!(
        "站点: {} | 存储目录: {}",
        config.site_name,
        config.storage_dir.display()
    );

    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(16)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db).await?;
    bootstrap_admin(&db, &config).await;
    seed_random_pool(&db, &config).await;

    let storage = Box::new(LocalStorage::new(&config.storage_dir)?);
    let state = AppState::new(config, db, storage);

    // Apply the admin-configured site domain (overrides env SITE_URL). Empty = relative links.
    if let Ok(Some((v,))) =
        sqlx::query_as::<_, (String,)>("SELECT value FROM settings WHERE key = 'site_url'")
            .fetch_optional(state.db())
            .await
    {
        state.set_base_url(v);
    }

    let addr: std::net::SocketAddr = state.cfg().bind_addr.parse()?;
    let app = routes::app_router(state.clone());
    tracing::info!("服务器启动在 http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

fn validate_admin_credentials(username: &str, password: &str) -> Result<(), &'static str> {
    let name_len = username.chars().count();
    if !(3..=32).contains(&name_len) {
        return Err("用户名长度需为 3–32 个字符");
    }
    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("用户名只能包含字母、数字、下划线与短横线");
    }
    let pw_len = password.chars().count();
    if pw_len < 6 {
        return Err("密码至少 6 个字符");
    }
    if pw_len > 200 {
        return Err("密码过长");
    }
    Ok(())
}

/// Create the deployment admin, or update that account when the configured
/// username or password changes. A password changed in the web UI is left
/// alone until `ADMIN_PASSWORD` itself changes.
async fn bootstrap_admin(db: &sqlx::PgPool, config: &Config) {
    let username = config.bootstrap_admin_username.trim();
    let password = config.bootstrap_admin_password.as_str();
    if let Err(msg) = validate_admin_credentials(username, password) {
        tracing::error!("管理员配置无效，已跳过同步: {msg}");
        return;
    }
    if let Err(e) = sync_admin(db, username, password).await {
        tracing::error!("同步管理员账号失败: {e}");
    }
}

#[derive(sqlx::FromRow)]
struct AdminRow {
    id: Uuid,
    username: String,
    password_hash: String,
    role: String,
}

async fn sync_admin(db: &sqlx::PgPool, username: &str, password: &str) -> anyhow::Result<()> {
    let mut tx = db.begin().await?;
    let remembered = remembered_admin(&mut tx).await?;
    let existing = find_managed_admin(&mut tx, username, remembered.id).await?;

    let user_id = if let Some(user) = existing {
        apply_admin_credentials(&mut tx, &user, username, password, Some(&remembered)).await?;
        user.id
    } else {
        let id = create_admin(&mut tx, username, password).await?;
        tracing::warn!("已创建管理员账号 '{username}'");
        id
    };

    sqlx::query(
        "INSERT INTO settings (key, value) VALUES ('managed_admin_id', $1) \
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind(user_id.to_string())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

struct RememberedAdmin {
    id: Option<Uuid>,
    username: Option<String>,
    password_hash: Option<String>,
}

async fn remembered_admin(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<RememberedAdmin, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM settings WHERE key IN \
         ('managed_admin_id', 'managed_admin_username', 'managed_admin_pass_hash')",
    )
    .fetch_all(&mut **tx)
    .await?;
    let mut remembered = RememberedAdmin {
        id: None,
        username: None,
        password_hash: None,
    };
    for (key, value) in rows {
        match key.as_str() {
            "managed_admin_id" => remembered.id = Uuid::parse_str(&value).ok(),
            "managed_admin_username" => remembered.username = Some(value),
            "managed_admin_pass_hash" => remembered.password_hash = Some(value),
            _ => {}
        }
    }
    Ok(remembered)
}

async fn find_managed_admin(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    username: &str,
    marked_id: Option<Uuid>,
) -> Result<Option<AdminRow>, sqlx::Error> {
    if let Some(id) = marked_id {
        return sqlx::query_as::<_, AdminRow>(
            "SELECT id, username, password_hash, role::text AS role FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&mut **tx)
        .await;
    }
    if let Some(row) = sqlx::query_as::<_, AdminRow>(
        "SELECT id, username, password_hash, role::text AS role FROM users WHERE username = $1",
    )
    .bind(username)
    .fetch_optional(&mut **tx)
    .await?
    {
        return Ok(Some(row));
    }
    sqlx::query_as::<_, AdminRow>(
        "SELECT id, username, password_hash, role::text AS role FROM users \
         WHERE role = 'admin' ORDER BY created_at ASC LIMIT 1",
    )
    .fetch_optional(&mut **tx)
    .await
}

async fn apply_admin_credentials(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user: &AdminRow,
    username: &str,
    password: &str,
    remembered: Option<&RememberedAdmin>,
) -> anyhow::Result<()> {
    let username_changed = match remembered.and_then(|r| r.username.as_deref()) {
        Some(prev) => prev != username,
        None => user.username != username,
    };
    let password_changed = match remembered.and_then(|r| r.password_hash.as_deref()) {
        Some(prev) => !auth::verify_password(password, prev),
        None => !auth::verify_password(password, &user.password_hash),
    };
    let promote = user.role != "admin";

    let mut next_username = user.username.clone();
    if username_changed {
        let taken: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM users WHERE username = $1 AND id <> $2)",
        )
        .bind(username)
        .bind(user.id)
        .fetch_one(&mut **tx)
        .await?;
        if taken {
            tracing::error!("管理员用户名 {username} 已被其他账号占用，未修改用户名");
        } else {
            next_username = username.to_string();
            tracing::warn!("管理员用户名已从 {} 改为 {username}", user.username);
        }
    }

    let next_hash = if password_changed {
        tracing::warn!("管理员密码已按配置更新");
        Some(auth::hash_password(password)?)
    } else {
        None
    };

    if next_username != user.username || next_hash.is_some() || promote {
        sqlx::query(
            "UPDATE users SET username = $1, password_hash = COALESCE($2, password_hash), role = 'admin' \
             WHERE id = $3",
        )
        .bind(&next_username)
        .bind(next_hash.as_deref())
        .bind(user.id)
        .execute(&mut **tx)
        .await?;
        if password_changed {
            sqlx::query("DELETE FROM sessions WHERE user_id = $1")
                .bind(user.id)
                .execute(&mut **tx)
                .await?;
        }
        if promote {
            tracing::warn!("已将部署管理员恢复为管理员角色");
        }
    }

    let stored_hash = next_hash.unwrap_or_else(|| {
        remembered
            .and_then(|r| r.password_hash.clone())
            .unwrap_or_else(|| user.password_hash.clone())
    });
    save_admin_marker(tx, &next_username, &stored_hash).await?;
    Ok(())
}

async fn create_admin(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    username: &str,
    password: &str,
) -> anyhow::Result<Uuid> {
    let hash = auth::hash_password(password)?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (username, password_hash, role, quota_bytes) \
         VALUES ($1, $2, 'admin', 0) RETURNING id",
    )
    .bind(username)
    .bind(&hash)
    .fetch_one(&mut **tx)
    .await?;
    save_admin_marker(tx, username, &hash).await?;
    Ok(id)
}

async fn save_admin_marker(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    username: &str,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES ('managed_admin_username', $1) \
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind(username)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES ('managed_admin_pass_hash', $1) \
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind(password_hash)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// On first run, import legacy `data/image_urls.txt` (one row per line: `url,tag1,tag2`)
/// into the random-image pool, mirroring the original Go service's seeding.
async fn seed_random_pool(db: &sqlx::PgPool, config: &Config) {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM images")
        .fetch_one(db)
        .await
        .unwrap_or(0);
    if count > 0 {
        return;
    }
    let path = config.data_dir.join("data").join("image_urls.txt");
    let Ok(content) = std::fs::read_to_string(&path) else {
        tracing::debug!("未找到 {}，跳过随机图池导入", path.display());
        return;
    };
    tracing::info!("正在从 {} 导入随机图池…", path.display());
    let mut inserted = 0u32;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split(',');
        let Some(url) = parts.next().map(str::trim) else {
            continue;
        };
        if url.is_empty() {
            continue;
        }
        let tags: Vec<String> = parts
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        let res = sqlx::query(
            "INSERT INTO images (source_kind, url, tags, privacy) VALUES ('remote', $1, $2, 'public')",
        )
        .bind(url)
        .bind(&tags)
        .execute(db)
        .await;
        if res.is_ok() {
            inserted += 1;
        }
    }
    tracing::info!("随机图池导入完成：{} 条", inserted);
}

#[cfg(test)]
mod tests {
    use super::validate_admin_credentials;

    #[test]
    fn accepts_normal_admin_credentials() {
        assert!(validate_admin_credentials("admin", "changeme").is_ok());
    }

    #[test]
    fn rejects_short_or_odd_credentials() {
        assert!(validate_admin_credentials("ab", "changeme").is_err());
        assert!(validate_admin_credentials("admin", "short").is_err());
        assert!(validate_admin_credentials("bad name", "changeme").is_err());
    }
}
