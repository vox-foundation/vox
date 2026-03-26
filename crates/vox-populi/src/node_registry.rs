//! [`NodeRecord`] and on-disk registry file layout for Populi.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use vox_repository::TaskCapabilityHints;

use crate::now_ms;

/// One participant in the populi view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeRecord {
    /// Stable node id (operator- or env-assigned).
    pub id: String,
    /// Host capabilities (CPU + optional GPU hints).
    pub capabilities: TaskCapabilityHints,
    /// Optional listen address for control or data plane (phase 3+).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listen_addr: Option<String>,
    /// `CARGO_PKG_VERSION` of `vox-populi` / embedding crate at registration time.
    pub version: String,
    /// Wall-clock last update (epoch ms).
    pub last_seen_unix_ms: u64,
    /// Populi tenancy / cluster id; must match server [`crate::transport::PopuliTransportState::required_scope`] when the server enforces scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_id: Option<String>,
    /// Worker visibility for scheduling policy (`private` or `public` when set).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    /// Logical pool id (`pool=…` mesh label normalization).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool_id: Option<String>,
    /// Trust tier for public mesh policy (`new`, `probation`, `trusted`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust_tier: Option<String>,
    /// Declared workload classes (`infer`, `train`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workload_classes: Option<Vec<String>>,
    /// Privacy class advertised by this node (`public_ok`, `trusted_only`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy_class: Option<String>,
    /// When true, scheduler should not place new work here (drain-only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintenance: Option<bool>,
    /// Optional cloud / bridge provider tag (`runpod`, `vast`, …) for hybrid workers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// When true, server rejects new A2A claims for this node (set via admin quarantine API only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quarantined: Option<bool>,
}

/// Serializable registry file (`.vox/cache/populi/local-registry.json`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PopuliRegistryFile {
    /// Schema version for forward compatibility.
    pub schema_version: u32,
    /// Known nodes (typically one for local-only mode).
    pub nodes: Vec<NodeRecord>,
}

/// Drop nodes whose `last_seen_unix_ms` is older than `now - max_stale_ms` when `max_stale_ms` is Some and > 0.
#[must_use]
pub fn filter_registry_by_max_stale_ms(
    mut file: PopuliRegistryFile,
    max_stale_ms: Option<u64>,
) -> PopuliRegistryFile {
    let Some(threshold) = max_stale_ms.filter(|n| *n > 0) else {
        return file;
    };
    let now = now_ms();
    file.nodes
        .retain(|n| now.saturating_sub(n.last_seen_unix_ms) <= threshold);
    file
}

/// Local file-backed registry (single-writer; suitable for shared Docker volume in dev).
#[derive(Debug)]
pub struct LocalRegistry {
    path: PathBuf,
}

impl LocalRegistry {
    /// Default path under the user home: `~/.vox/cache/populi/local-registry.json`.
    #[must_use]
    pub fn default_path() -> PathBuf {
        let base = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        base.join(".vox")
            .join("cache")
            .join("populi")
            .join("local-registry.json")
    }

    /// Open registry at `path` (file may not exist yet).
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Prefer `VOX_MESH_REGISTRY_PATH`, else [`LocalRegistry::default_path`].
    #[must_use]
    pub fn resolved_default_path() -> PathBuf {
        std::env::var_os("VOX_MESH_REGISTRY_PATH")
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(Self::default_path)
    }

    /// Path on disk.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load or return empty registry.
    pub fn load(&self) -> Result<PopuliRegistryFile, PopuliRegistryError> {
        if !self.path.is_file() {
            return Ok(PopuliRegistryFile {
                schema_version: 1,
                nodes: Vec::new(),
            });
        }
        let raw = std::fs::read_to_string(&self.path).map_err(PopuliRegistryError::Io)?;
        let parsed: PopuliRegistryFile =
            serde_json::from_str(&raw).map_err(|e| PopuliRegistryError::Json(e.to_string()))?;
        Ok(parsed)
    }

    /// Replace registry contents atomically (write temp + rename).
    pub fn save(&self, reg: &PopuliRegistryFile) -> Result<(), PopuliRegistryError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(PopuliRegistryError::Io)?;
        }
        let json = serde_json::to_string_pretty(reg)
            .map_err(|e| PopuliRegistryError::Json(e.to_string()))?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json.as_bytes()).map_err(PopuliRegistryError::Io)?;
        std::fs::rename(&tmp, &self.path).map_err(PopuliRegistryError::Io)?;
        Ok(())
    }

    /// Upsert a node by `id` and persist.
    pub fn upsert_node(&self, mut record: NodeRecord) -> Result<(), PopuliRegistryError> {
        record.last_seen_unix_ms = now_ms();
        let mut reg = self.load()?;
        reg.schema_version = 1;
        if let Some(i) = reg.nodes.iter().position(|n| n.id == record.id) {
            reg.nodes[i] = record;
        } else {
            reg.nodes.push(record);
        }
        self.save(&reg)
    }
}

/// Registry I/O errors.
#[derive(Debug, thiserror::Error)]
pub enum PopuliRegistryError {
    /// Filesystem error.
    #[error("populi registry I/O: {0}")]
    Io(#[from] std::io::Error),
    /// JSON parse/serialize error.
    #[error("populi registry JSON: {0}")]
    Json(String),
    /// HTTP control plane error.
    #[error("populi HTTP: {0}")]
    Http(String),
}
