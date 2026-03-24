//! Persistent storage primitives for Vox: Blob (S3/FS) and Key-Value (SQL/KV).
//!
//! Replaces the standalone `vox-storage` crate with local modules.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Async trait for blob storage (local FS, S3, or GCS).
#[async_trait]
pub trait BlobStore: Send + Sync {
    async fn put(&self, path: &str, data: Vec<u8>) -> anyhow::Result<()>;
    async fn get(&self, path: &str) -> anyhow::Result<Vec<u8>>;
    async fn delete(&self, path: &str) -> anyhow::Result<()>;
    async fn list(&self, prefix: &str) -> anyhow::Result<Vec<String>>;
}

/// Metadata for a stored blob or KV entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMeta {
    pub size_bytes: u64,
    pub created_at_ms: i64,
    pub etag: Option<String>,
}
