//! Object storage: local filesystem (**default**) and optional **S3-compatible** (Cloudflare R2)
//! via feature `r2` (`object_store` + `R2_*` env vars).

use async_trait::async_trait;
use std::path::PathBuf;
use thiserror::Error;

/// Errors from [`ObjectStorage`] implementations (local disk or R2).
#[derive(Debug, Error)]
pub enum StorageError {
    /// Local filesystem I/O failure.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Logical error (missing env, object_store failure message, etc.).
    #[error("{0}")]
    Msg(String),
}

/// Async object key/blob store (local disk or S3-compatible when `r2` is enabled).
#[async_trait]
pub trait ObjectStorage: Send + Sync {
    /// Write `bytes` at `key` (overwriting if present).
    async fn put(&self, key: &str, bytes: &[u8]) -> Result<(), StorageError>;
    /// Read object bytes, or `None` if missing.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError>;
    /// Remove object at `key` (no-op if missing).
    async fn delete(&self, key: &str) -> Result<(), StorageError>;
    /// List keys starting with `prefix` (relative to `root`, `/`-separated).
    async fn list_prefix(&self, prefix: &str) -> Result<Vec<String>, StorageError>;
}

/// Store blobs under `{root}/{key}` with unsafe path characters replaced.
pub struct LocalObjectStorage {
    root: PathBuf,
}

impl LocalObjectStorage {
    /// Filesystem root directory; keys map to paths under this root (sanitized).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn key_path(&self, key: &str) -> PathBuf {
        let safe = key
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '/' || c == '-' || c == '_' || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();
        self.root.join(safe.trim_start_matches('/'))
    }
}

#[async_trait]
impl ObjectStorage for LocalObjectStorage {
    async fn put(&self, key: &str, bytes: &[u8]) -> Result<(), StorageError> {
        let p = self.key_path(key);
        if let Some(parent) = p.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&p, bytes).await?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let p = self.key_path(key);
        match tokio::fs::read(&p).await {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let p = self.key_path(key);
        match tokio::fs::remove_file(p).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    async fn list_prefix(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        let pref = prefix.trim_start_matches('/').trim_end_matches('/');
        let walk_root = if pref.is_empty() {
            self.root.clone()
        } else {
            self.key_path(pref)
        };
        let mut out = Vec::new();
        if !walk_root.exists() {
            return Ok(out);
        }
        let mut stack = vec![walk_root];
        while let Some(dir) = stack.pop() {
            let mut rd = tokio::fs::read_dir(&dir).await?;
            while let Some(ent) = rd.next_entry().await? {
                let p = ent.path();
                let meta = ent.metadata().await?;
                if meta.is_dir() {
                    stack.push(p);
                } else if let Ok(rel) = p.strip_prefix(&self.root) {
                    let s = rel.to_string_lossy().replace('\\', "/");
                    if pref.is_empty() || s.starts_with(pref) {
                        out.push(s);
                    }
                }
            }
        }
        out.sort();
        Ok(out)
    }
}

/// Cloudflare R2 (or any S3-compatible endpoint) using `object_store` (**feature `r2`**).
///
/// Environment:
/// - `R2_ENDPOINT` — e.g. `https://<account>.r2.cloudflarestorage.com`
/// - `R2_BUCKET`
/// - `R2_ACCESS_KEY_ID` / `R2_SECRET_ACCESS_KEY`
#[cfg(feature = "r2")]
pub struct R2ObjectStorage {
    inner: std::sync::Arc<dyn object_store::ObjectStore>,
}

#[cfg(feature = "r2")]
impl R2ObjectStorage {
    /// Build client from `R2_*` environment variables (see struct docs).
    pub fn from_env() -> Result<Self, StorageError> {
        use object_store::aws::AmazonS3Builder;

        let endpoint = std::env::var("R2_ENDPOINT")
            .map_err(|_| StorageError::Msg("R2_ENDPOINT not set".into()))?;
        let bucket = std::env::var("R2_BUCKET")
            .map_err(|_| StorageError::Msg("R2_BUCKET not set".into()))?;
        let access = std::env::var("R2_ACCESS_KEY_ID")
            .map_err(|_| StorageError::Msg("R2_ACCESS_KEY_ID not set".into()))?;
        let secret = std::env::var("R2_SECRET_ACCESS_KEY")
            .map_err(|_| StorageError::Msg("R2_SECRET_ACCESS_KEY not set".into()))?;

        let store = AmazonS3Builder::new()
            .with_endpoint(&endpoint)
            .with_bucket_name(&bucket)
            .with_access_key_id(access)
            .with_secret_access_key(secret)
            .with_region("auto")
            .build()
            .map_err(|e| StorageError::Msg(e.to_string()))?;

        Ok(Self {
            inner: std::sync::Arc::new(store),
        })
    }
}

#[cfg(feature = "r2")]
#[async_trait]
impl ObjectStorage for R2ObjectStorage {
    async fn put(&self, key: &str, bytes: &[u8]) -> Result<(), StorageError> {
        use object_store::path::Path as ObjPath;
        let p = ObjPath::from(key.trim_start_matches('/'));
        self.inner
            .put(&p, bytes.to_vec().into())
            .await
            .map_err(|e| StorageError::Msg(e.to_string()))?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        use object_store::path::Path as ObjPath;
        let p = ObjPath::from(key.trim_start_matches('/'));
        match self.inner.get(&p).await {
            Ok(r) => {
                let b = r
                    .bytes()
                    .await
                    .map_err(|e| StorageError::Msg(e.to_string()))?;
                Ok(Some(b.to_vec()))
            }
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(e) => Err(StorageError::Msg(e.to_string())),
        }
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        use object_store::path::Path as ObjPath;
        let p = ObjPath::from(key.trim_start_matches('/'));
        self.inner
            .delete(&p)
            .await
            .map_err(|e| StorageError::Msg(e.to_string()))
    }

    async fn list_prefix(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        use futures_util::TryStreamExt;
        use object_store::path::Path as ObjPath;
        let pref = prefix.trim_start_matches('/');
        let owned = if pref.is_empty() {
            None
        } else {
            Some(ObjPath::from(pref))
        };
        let mut stream = self.inner.list(owned.as_ref());
        let mut out = Vec::new();
        while let Some(meta) = stream
            .try_next()
            .await
            .map_err(|e| StorageError::Msg(e.to_string()))?
        {
            out.push(meta.location.to_string());
        }
        out.sort();
        Ok(out)
    }
}
