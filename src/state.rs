use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::config::Config;
use crate::storage::Storage;

#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<Inner>,
}

pub struct Inner {
    pub config: Config,
    pub db: sqlx::PgPool,
    pub storage: Box<dyn Storage>,
    pub http: reqwest::Client,
    /// Cached public origin for absolute share/embed links (no trailing slash;
    /// empty = relative). Source of truth is the `site_url` DB setting; this
    /// cache is refreshed at startup and whenever admin edits it.
    pub base_url: RwLock<String>,
}

impl AppState {
    pub fn new(config: Config, db: sqlx::PgPool, storage: Box<dyn Storage>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            // Redirects are followed in `netguard`, which checks every hop.
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("RangPic/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("reqwest client");
        let base_url = RwLock::new(
            config
                .site_url
                .clone()
                .unwrap_or_default()
                .trim_end_matches('/')
                .to_string(),
        );
        Self {
            inner: Arc::new(Inner {
                config,
                db,
                storage,
                http,
                base_url,
            }),
        }
    }

    pub fn cfg(&self) -> &Config {
        &self.inner.config
    }
    pub fn db(&self) -> &sqlx::PgPool {
        &self.inner.db
    }
    pub fn storage(&self) -> &dyn Storage {
        self.inner.storage.as_ref()
    }
    pub fn http(&self) -> &reqwest::Client {
        &self.inner.http
    }

    /// Absolute-link base (no trailing slash). Empty means "emit relative links".
    pub fn base_url(&self) -> String {
        self.inner.base_url.read().unwrap().clone()
    }

    pub fn set_base_url(&self, mut v: String) {
        while v.ends_with('/') {
            v.pop();
        }
        *self.inner.base_url.write().unwrap() = v;
    }
}
