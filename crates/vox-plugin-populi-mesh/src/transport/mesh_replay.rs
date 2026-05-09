//! Persisted JWT `jti` replay table + A2A deliver idempotency key map for multi-instance restarts.
//!
//! When a persist path is configured, maps are loaded at startup and written after updates (same
//! atomic rename pattern as the A2A JSON store). **Multi-writer concurrency** is last-writer-wins;
//! single control-plane writer per file is expected.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::PopuliRegistryError;

/// In-memory maps shared by auth middleware and A2A deliver handlers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct MeshReplayMaps {
    #[serde(default)]
    pub jwt_jti: HashMap<String, u64>,
    #[serde(default)]
    pub idempotency: HashMap<String, u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MeshReplayFile {
    schema_version: u32,
    #[serde(default)]
    jwt_jti: Vec<(String, u64)>,
    #[serde(default)]
    idempotency: Vec<(String, u64)>,
}

#[derive(Clone)]
pub(crate) struct MeshReplayState {
    maps: Arc<RwLock<MeshReplayMaps>>,
    persist_path: Option<PathBuf>,
}

fn persist_maps(path: &Path, maps: &MeshReplayMaps) -> Result<(), PopuliRegistryError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(PopuliRegistryError::Io)?;
    }
    let file = MeshReplayFile {
        schema_version: 1,
        jwt_jti: maps.jwt_jti.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        idempotency: maps
            .idempotency
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect(),
    };
    let payload = serde_json::to_string_pretty(&file)
        .map_err(|e| PopuliRegistryError::Json(e.to_string()))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, payload.as_bytes()).map_err(PopuliRegistryError::Io)?;
    std::fs::rename(&tmp, path).map_err(PopuliRegistryError::Io)?;
    Ok(())
}

impl MeshReplayState {
    #[must_use]
    pub(crate) fn in_memory() -> Arc<Self> {
        Arc::new(Self {
            maps: Arc::new(RwLock::new(MeshReplayMaps::default())),
            persist_path: None,
        })
    }

    pub(crate) fn maps(&self) -> &Arc<RwLock<MeshReplayMaps>> {
        &self.maps
    }

    pub(crate) async fn persist_if_configured(&self) {
        let Some(path) = self.persist_path.as_ref() else {
            return;
        };
        let maps = self.maps.read().await;
        if let Err(e) = persist_maps(path, &maps) {
            tracing::warn!(error = %e, "mesh replay persist failed");
        }
    }
}
