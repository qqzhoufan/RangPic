use std::path::{Component, Path, PathBuf};

use async_trait::async_trait;
use bytes::Bytes;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::error::{AppError, AppResult};

/// Abstract blob storage. Keys are slash-relative paths managed by the service
/// layer (e.g. `orig/ab/cd/<id>.png`, `thumb/ab/cd/<id>.webp`). Backends must
/// reject any key that could escape the storage root.
#[async_trait]
#[allow(dead_code)]
pub trait Storage: Send + Sync + 'static {
    async fn put(&self, key: &str, content_type: &str, bytes: Bytes) -> AppResult<()>;
    /// Load the full blob into memory. Use only for modestly-sized blobs (thumbnails, proxied bytes).
    async fn get(&self, key: &str) -> AppResult<Bytes>;
    /// Open a handle for streaming (serving originals without buffering the whole file).
    async fn open(&self, key: &str) -> AppResult<fs::File>;
    async fn delete(&self, key: &str) -> AppResult<()>;
    async fn size(&self, key: &str) -> AppResult<u64>;
    fn root_display(&self) -> String;
}

pub struct LocalStorage {
    root: PathBuf,
}

impl LocalStorage {
    pub fn new(root: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    fn resolve(&self, key: &str) -> AppResult<PathBuf> {
        let safe = sanitize_key(key)?;
        let mut p = self.root.clone();
        p.push(safe);
        Ok(p)
    }
}

fn sanitize_key(key: &str) -> AppResult<PathBuf> {
    let mut out = PathBuf::new();
    for comp in Path::new(key).components() {
        match comp {
            Component::Normal(c) => out.push(c),
            Component::CurDir => {}
            _ => return Err(AppError::BadRequest("invalid storage key".into())),
        }
    }
    if out.as_os_str().is_empty() {
        return Err(AppError::BadRequest("empty storage key".into()));
    }
    Ok(out)
}

#[async_trait]
impl Storage for LocalStorage {
    async fn put(&self, key: &str, _content_type: &str, bytes: Bytes) -> AppResult<()> {
        let path = self.resolve(key)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut f = fs::File::create(&path).await?;
        f.write_all(&bytes).await?;
        f.flush().await?;
        Ok(())
    }

    async fn get(&self, key: &str) -> AppResult<Bytes> {
        let path = self.resolve(key)?;
        Ok(fs::read(&path).await?.into())
    }

    async fn open(&self, key: &str) -> AppResult<fs::File> {
        let path = self.resolve(key)?;
        Ok(fs::File::open(&path).await?)
    }

    async fn delete(&self, key: &str) -> AppResult<()> {
        let path = self.resolve(key)?;
        match fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    async fn size(&self, key: &str) -> AppResult<u64> {
        let path = self.resolve(key)?;
        Ok(fs::metadata(&path).await?.len())
    }

    fn root_display(&self) -> String {
        self.root.display().to_string()
    }
}
