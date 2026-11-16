//! Content-addressed disk spill store for oversized research payloads.
//!
//! Callers compute the 64-character lowercase SHA3-256 hex digest (via `vox-crypto` in
//! `vox-orchestrator-mcp`) and pass it to [`ChunkSpillStore::spill_with_digest`].
//! This crate keeps zero direct cryptographic crate imports so `crate-edges` stays clean.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

/// Payloads larger than 16 KiB are spilled out-of-line to disk.
pub const SPILL_THRESHOLD_BYTES: usize = 16 * 1024;
/// Default maximum disk budget for spilled chunks (128 MiB).
pub const DEFAULT_MAX_SPILL_BYTES: u64 = 128 * 1024 * 1024;
/// Maximum UTF-8 characters retained in [`SpillPointer::preview`].
pub const DEFAULT_PREVIEW_CHARS: usize = 256;
const CHUNK_EXT: &str = ".chunk";

/// Metadata pointer returned when a payload exceeds [`SPILL_THRESHOLD_BYTES`] and is spilled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpillPointer {
    pub content_hash: String,
    pub byte_len: usize,
    pub preview: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_url: Option<String>,
}

/// Bounded, content-addressed disk store for spilled research payload chunks.
#[derive(Debug, Clone)]
pub struct ChunkSpillStore {
    root: PathBuf,
    max_bytes: u64,
    threshold_bytes: usize,
}

impl Default for ChunkSpillStore {
    fn default() -> Self {
        Self::new(
            default_spill_root(),
            DEFAULT_MAX_SPILL_BYTES,
            SPILL_THRESHOLD_BYTES,
        )
    }
}

/// Default spill root under the OS temp directory (never hardcodes POSIX root paths).
#[must_use]
pub fn default_spill_root() -> PathBuf {
    std::env::temp_dir().join("vox-research-spill")
}

/// Strictly validate that `digest_hex` is 64 lowercase ASCII hex digits (path-traversal guard).
#[must_use]
pub fn is_valid_digest_hex(digest_hex: &str) -> bool {
    digest_hex.len() == 64
        && digest_hex
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn make_preview(bytes: &[u8], max_chars: usize) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut out: String = text.chars().take(max_chars).collect();
    if text.chars().nth(max_chars).is_some() {
        out.push_str("...");
    }
    out
}

fn chunk_filename(digest_hex: &str) -> String {
    format!("{digest_hex}{CHUNK_EXT}")
}

impl ChunkSpillStore {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>, max_bytes: u64, threshold_bytes: usize) -> Self {
        Self {
            root: root.into(),
            max_bytes,
            threshold_bytes,
        }
    }

    #[must_use]
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self::new(root, DEFAULT_MAX_SPILL_BYTES, SPILL_THRESHOLD_BYTES)
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn chunk_path(&self, digest_hex: &str) -> Result<PathBuf> {
        if !is_valid_digest_hex(digest_hex) {
            bail!("invalid chunk digest: expected 64 lowercase hex characters");
        }
        Ok(self.root.join(chunk_filename(digest_hex)))
    }

    /// Spill `bytes` only if `bytes.len() > self.threshold_bytes`.
    pub fn spill_with_digest(
        &self,
        digest_hex: &str,
        bytes: &[u8],
    ) -> Result<Option<SpillPointer>> {
        if !is_valid_digest_hex(digest_hex) {
            bail!("invalid chunk digest: expected 64 lowercase hex characters");
        }
        if bytes.len() <= self.threshold_bytes {
            return Ok(None);
        }
        Ok(Some(self.force_spill_with_digest(digest_hex, bytes)?))
    }

    /// Spill `bytes` unconditionally using a pre-computed 64-hex `digest_hex`.
    pub fn force_spill_with_digest(&self, digest_hex: &str, bytes: &[u8]) -> Result<SpillPointer> {
        let target = self.chunk_path(digest_hex)?;
        if self.max_bytes > 0 && (bytes.len() as u64) > self.max_bytes {
            bail!(
                "chunk size {} exceeds store max_bytes {}",
                bytes.len(),
                self.max_bytes
            );
        }
        fs::create_dir_all(&self.root)
            .with_context(|| format!("create spill dir {}", self.root.display()))?;

        let present = fs::metadata(&target)
            .map(|m| m.is_file() && m.len() == bytes.len() as u64)
            .unwrap_or(false);
        if present {
            if let Ok(file) = fs::OpenOptions::new().write(true).open(&target) {
                let _ = file.set_modified(SystemTime::now());
            }
        } else {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let tmp = self
                .root
                .join(format!(".tmp-{digest_hex}-{}-{nonce}", std::process::id()));
            fs::write(&tmp, bytes)
                .with_context(|| format!("write temp spill {}", tmp.display()))?;
            if let Err(err) = fs::rename(&tmp, &target) {
                let _ = fs::remove_file(&tmp);
                return Err(err).with_context(|| format!("rename spill {}", target.display()));
            }
        }
        self.evict_if_needed(digest_hex)?;
        Ok(SpillPointer {
            content_hash: digest_hex.to_string(),
            byte_len: bytes.len(),
            preview: make_preview(bytes, DEFAULT_PREVIEW_CHARS),
            chunk_url: None,
        })
    }

    /// Spill `bytes` when above threshold using caller-supplied digest and hex functions.
    pub fn spill_with_hasher(
        &self,
        bytes: &[u8],
        hasher: fn(&[u8]) -> [u8; 32],
        hex: fn(&[u8]) -> String,
    ) -> Result<Option<SpillPointer>> {
        if bytes.len() <= self.threshold_bytes {
            return Ok(None);
        }
        let digest_hex = hex(&hasher(bytes));
        self.spill_with_digest(&digest_hex, bytes)
    }

    /// Read raw chunk bytes for `digest_hex`, returning `Ok(None)` when absent.
    pub fn read_chunk(&self, digest_hex: &str) -> Result<Option<Vec<u8>>> {
        let path = self.chunk_path(digest_hex)?;
        let meta = match fs::metadata(&path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e).with_context(|| format!("stat chunk {}", path.display())),
        };
        let cap = vox_bounded_fs::max_file_bytes_hint().max(self.max_bytes);
        if meta.len() > cap {
            bail!(
                "spilled chunk {} is {} bytes; exceeds cap {}",
                path.display(),
                meta.len(),
                cap
            );
        }
        Ok(Some(fs::read(&path).with_context(|| {
            format!("read chunk {}", path.display())
        })?))
    }

    /// Read a UTF-8 chunk via `vox_bounded_fs::read_utf8_path_capped`, returning `Ok(None)` when absent.
    pub fn read_chunk_utf8(&self, digest_hex: &str) -> Result<Option<String>> {
        let path = self.chunk_path(digest_hex)?;
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(vox_bounded_fs::read_utf8_path_capped(&path)?))
    }

    pub fn total_bytes(&self) -> Result<u64> {
        Ok(self.list_chunks()?.iter().map(|c| c.size).sum())
    }

    pub fn chunk_count(&self) -> Result<usize> {
        Ok(self.list_chunks()?.len())
    }

    fn list_chunks(&self) -> Result<Vec<ChunkEntry>> {
        let entries = match fs::read_dir(&self.root) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e).with_context(|| format!("read_dir {}", self.root.display())),
        };
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(stem) = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(CHUNK_EXT))
            else {
                continue;
            };
            if !is_valid_digest_hex(stem) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_file() {
                out.push(ChunkEntry {
                    digest_hex: stem.to_string(),
                    path,
                    size: meta.len(),
                    modified: meta.modified().unwrap_or(UNIX_EPOCH),
                });
            }
        }
        Ok(out)
    }

    fn evict_if_needed(&self, protected_digest: &str) -> Result<()> {
        if self.max_bytes == 0 {
            return Ok(());
        }
        let mut chunks = self.list_chunks()?;
        let mut total: u64 = chunks.iter().map(|c| c.size).sum();
        if total <= self.max_bytes {
            return Ok(());
        }
        chunks.sort_by(|a, b| {
            a.modified
                .cmp(&b.modified)
                .then_with(|| a.digest_hex.cmp(&b.digest_hex))
        });
        for chunk in chunks {
            if total <= self.max_bytes {
                break;
            }
            if chunk.digest_hex != protected_digest && fs::remove_file(&chunk.path).is_ok() {
                total = total.saturating_sub(chunk.size);
            }
        }
        Ok(())
    }
}

struct ChunkEntry {
    digest_hex: String,
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn sha3_hex(bytes: &[u8]) -> String {
        vox_crypto::hex_encode(&vox_crypto::compliance_hash(bytes))
    }

    fn age_file(dir: &Path, digest: &str, secs_ago: u64) {
        let p = dir.join(chunk_filename(digest));
        let t = SystemTime::now() - Duration::from_secs(secs_ago);
        fs::OpenOptions::new()
            .write(true)
            .open(p)
            .unwrap()
            .set_modified(t)
            .unwrap();
    }

    #[test]
    fn below_or_equal_threshold_bypasses_disk_spill() {
        let dir = tempfile::tempdir().unwrap();
        let store = ChunkSpillStore::with_root(dir.path());
        let exact = vec![b'a'; SPILL_THRESHOLD_BYTES];
        assert!(
            store
                .spill_with_digest(&sha3_hex(&exact), &exact)
                .unwrap()
                .is_none()
        );
        assert_eq!(store.chunk_count().unwrap(), 0);
    }

    #[test]
    fn above_threshold_spills_deduplicates_and_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let store = ChunkSpillStore::with_root(dir.path());
        let payload = vec![b'x'; SPILL_THRESHOLD_BYTES + 128];
        let digest = sha3_hex(&payload);
        let first = store.spill_with_digest(&digest, &payload).unwrap().unwrap();
        let second = store.spill_with_digest(&digest, &payload).unwrap().unwrap();
        assert_eq!(first.content_hash, digest);
        assert_eq!(first.content_hash, second.content_hash);
        assert_eq!(store.chunk_count().unwrap(), 1);
        assert_eq!(store.total_bytes().unwrap(), payload.len() as u64);
        assert_eq!(store.read_chunk(&digest).unwrap().unwrap(), payload);
        assert_eq!(
            store.read_chunk_utf8(&digest).unwrap().unwrap().len(),
            payload.len()
        );
    }

    #[test]
    fn evicts_oldest_chunks_when_exceeding_max_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let store = ChunkSpillStore::new(dir.path(), 40 * 1024, SPILL_THRESHOLD_BYTES);
        let (p1, p2, p3) = (
            vec![b'1'; 20 * 1024],
            vec![b'2'; 20 * 1024],
            vec![b'3'; 20 * 1024],
        );
        let (d1, d2, d3) = (sha3_hex(&p1), sha3_hex(&p2), sha3_hex(&p3));
        store.spill_with_digest(&d1, &p1).unwrap().unwrap();
        age_file(dir.path(), &d1, 60);
        store.spill_with_digest(&d2, &p2).unwrap().unwrap();
        age_file(dir.path(), &d2, 10);
        store.spill_with_digest(&d3, &p3).unwrap().unwrap();
        assert!(store.total_bytes().unwrap() <= 40 * 1024);
        assert!(store.read_chunk(&d1).unwrap().is_none());
        assert!(
            store.read_chunk(&d2).unwrap().is_some() && store.read_chunk(&d3).unwrap().is_some()
        );
    }

    #[test]
    fn rejects_invalid_and_traversal_digests() {
        let dir = tempfile::tempdir().unwrap();
        let store = ChunkSpillStore::with_root(dir.path());
        let data = vec![b'z'; SPILL_THRESHOLD_BYTES + 1];
        for bad in [
            "",
            "abc",
            "../../etc/passwd",
            "../0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg",
        ] {
            assert!(store.spill_with_digest(bad, &data).is_err(), "{bad}");
            assert!(store.read_chunk(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn spill_with_hasher_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let store = ChunkSpillStore::with_root(dir.path());
        let data = vec![b'h'; SPILL_THRESHOLD_BYTES + 50];
        let ptr = store
            .spill_with_hasher(&data, vox_crypto::compliance_hash, vox_crypto::hex_encode)
            .unwrap()
            .unwrap();
        assert_eq!(ptr.content_hash, sha3_hex(&data));
        assert_eq!(store.read_chunk(&ptr.content_hash).unwrap().unwrap(), data);
    }
}
