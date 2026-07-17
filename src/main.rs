mod assets;
mod auth;
mod config;
mod db;
mod error;
mod imaging;
mod routes;
mod state;
mod storage;

use config::Config;
use state::AppState;
use storage::LocalStorage;

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
    tracing::info!("站点: {} | 存储目录: {}", config.site_name, config.storage_dir.display());

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
    if let Ok(Some((v,))) = sqlx::query_as::<_, (String,)>("SELECT value FROM settings WHERE key = 'site_url'")
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

async fn bootstrap_admin(db: &sqlx::PgPool, config: &Config) {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(db)
        .await
        .unwrap_or(0);
    if count > 0 {
        return;
    }
    match auth::hash_password(&config.bootstrap_admin_password) {
        Ok(hash) => {
            let res = sqlx::query(
                "INSERT INTO users (username, password_hash, role, quota_bytes) VALUES ($1, $2, 'admin', $3)",
            )
            .bind(&config.bootstrap_admin_username)
            .bind(hash)
            .bind(0i64) // admins are unlimited (0 = no quota)
            .execute(db)
            .await;
            if let Err(e) = res {
                tracing::error!("创建初始管理员失败: {e}");
            } else {
                tracing::warn!(
                    "已创建初始管理员账号 '{}'（请尽快在后台修改密码）",
                    config.bootstrap_admin_username
                );
            }
        }
        Err(e) => tracing::error!("密码哈希失败: {e}"),
    }
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
