use std::env;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub storage_dir: PathBuf,
    pub data_dir: PathBuf,
    pub site_name: String,
    pub site_url: Option<String>,
    pub max_upload_bytes: usize,
    pub quota_default_bytes: i64,
    pub registration_open: bool,
    /// Bootstrap admin credentials created on first run if no users exist.
    pub bootstrap_admin_username: String,
    pub bootstrap_admin_password: String,
    pub session_ttl_hours: i64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        // .env is optional; ignore failure when the file is absent.
        let _ = dotenvy::dotenv();

        let database_url = env_or_fatal("DATABASE_URL");
        let storage_dir = PathBuf::from(env::var("STORAGE_DIR").unwrap_or_else(|_| "storage".to_string()));
        let data_dir = PathBuf::from(env::var("DATA_DIR").unwrap_or_else(|_| ".".to_string()));

        Ok(Self {
            database_url,
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:17777".to_string()),
            storage_dir,
            data_dir,
            site_name: env::var("SITE_NAME").unwrap_or_else(|_| "咸鱼图床".to_string()),
            site_url: env::var("SITE_URL").ok().filter(|s| !s.is_empty()),
            max_upload_bytes: env::var("MAX_UPLOAD_BYTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(25 * 1024 * 1024),
            quota_default_bytes: env::var("QUOTA_DEFAULT_BYTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100 * 1024 * 1024), // 100 MiB default per new user
            registration_open: env_bool("REGISTRATION_OPEN", true),
            bootstrap_admin_username: env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string()),
            bootstrap_admin_password: env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "adminpass".to_string()),
            session_ttl_hours: env::var("SESSION_TTL_HOURS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(24 * 14),
        })
    }
}

fn env_or_fatal(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| {
        panic!("环境变量 {key} 未设置 — 请参考 README 配置 (env var {key} is required)")
    })
}

fn env_bool(key: &str, default: bool) -> bool {
    match env::var(key) {
        Ok(v) => matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => default,
    }
}
