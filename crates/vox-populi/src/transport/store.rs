use std::path::PathBuf;

use crate::{NodeRecord, PopuliRegistryError};

use super::A2AStoredMessage;
use super::PopuliTransportState;
use super::RemoteExecLeaseRow;

pub(super) fn a2a_store_path_from_env() -> Option<PathBuf> {
    if let Ok(v) = std::env::var("VOX_MESH_A2A_STORE_PATH") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    let mut p = crate::local_registry_path();
    p.set_file_name("a2a-store.json");
    Some(p)
}

pub(super) fn load_a2a_store(
    path: &std::path::Path,
) -> Result<Vec<A2AStoredMessage>, PopuliRegistryError> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let raw = vox_bounded_fs::read_utf8_path_capped(path)
        .map_err(|e| PopuliRegistryError::Io(std::io::Error::other(e.to_string())))?;
    serde_json::from_str(&raw).map_err(|e| PopuliRegistryError::Json(e.to_string()))
}

pub(super) fn exec_lease_store_path_from_env(a2a_store_path: Option<&PathBuf>) -> Option<PathBuf> {
    if let Ok(v) = std::env::var("VOX_MESH_EXEC_LEASE_STORE_PATH") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    if let Some(a2a) = a2a_store_path {
        let mut p = a2a.clone();
        p.set_file_name("exec-lease-store.json");
        return Some(p);
    }
    let mut p = crate::local_registry_path();
    p.set_file_name("exec-lease-store.json");
    Some(p)
}

pub(super) fn load_exec_lease_store(
    path: &std::path::Path,
) -> Result<Vec<RemoteExecLeaseRow>, PopuliRegistryError> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let raw = vox_bounded_fs::read_utf8_path_capped(path)
        .map_err(|e| PopuliRegistryError::Io(std::io::Error::other(e.to_string())))?;
    serde_json::from_str(&raw).map_err(|e| PopuliRegistryError::Json(e.to_string()))
}

pub(super) fn dispatch_results_store_path_from_env(a2a_store_path: Option<&PathBuf>) -> Option<PathBuf> {
    if let Ok(v) = std::env::var("VOX_MESH_DISPATCH_STORE_PATH") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    if let Some(a2a) = a2a_store_path {
        let mut p = a2a.clone();
        p.set_file_name("dispatch-store.json");
        return Some(p);
    }
    let mut p = crate::local_registry_path();
    p.set_file_name("dispatch-store.json");
    Some(p)
}

pub(super) fn load_dispatch_results_store(
    path: &std::path::Path,
) -> Result<std::collections::HashMap<String, super::DispatchResponse>, PopuliRegistryError> {
    if !path.is_file() {
        return Ok(std::collections::HashMap::new());
    }
    let raw = vox_bounded_fs::read_utf8_path_capped(path)
        .map_err(|e| PopuliRegistryError::Io(std::io::Error::other(e.to_string())))?;
    serde_json::from_str(&raw).map_err(|e| PopuliRegistryError::Json(e.to_string()))
}

pub(super) fn persist_a2a_store(
    path: &std::path::Path,
    rows: &[A2AStoredMessage],
) -> Result<(), PopuliRegistryError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(PopuliRegistryError::Io)?;
    }
    let payload =
        serde_json::to_string_pretty(rows).map_err(|e| PopuliRegistryError::Json(e.to_string()))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, payload.as_bytes()).map_err(PopuliRegistryError::Io)?;
    std::fs::rename(&tmp, path).map_err(PopuliRegistryError::Io)?;
    Ok(())
}

pub(super) fn persist_exec_lease_store(
    path: &std::path::Path,
    rows: &[RemoteExecLeaseRow],
) -> Result<(), PopuliRegistryError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(PopuliRegistryError::Io)?;
    }
    let payload =
        serde_json::to_string_pretty(rows).map_err(|e| PopuliRegistryError::Json(e.to_string()))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, payload.as_bytes()).map_err(PopuliRegistryError::Io)?;
    std::fs::rename(&tmp, path).map_err(PopuliRegistryError::Io)?;
    Ok(())
}

pub(super) fn persist_dispatch_results_store(
    path: &std::path::Path,
    map: &dashmap::DashMap<String, super::DispatchResponse>,
) -> Result<(), PopuliRegistryError> {
    let start = std::time::Instant::now();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(PopuliRegistryError::Io)?;
    }
    let mut hashmap = std::collections::HashMap::new();
    for entry in map.iter() {
        hashmap.insert(entry.key().clone(), entry.value().clone());
    }
    let entries = hashmap.len();
    let payload =
        serde_json::to_string_pretty(&hashmap).map_err(|e| PopuliRegistryError::Json(e.to_string()))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, payload.as_bytes()).map_err(PopuliRegistryError::Io)?;
    std::fs::rename(&tmp, path).map_err(PopuliRegistryError::Io)?;
    tracing::trace!(
        path = %path.display(),
        entries = entries,
        latency_ms = start.elapsed().as_millis(),
        "persisted dispatch results store to disk"
    );
    Ok(())
}

impl Default for PopuliTransportState {
    fn default() -> Self {
        Self::new()
    }
}

pub(super) fn scope_ok(state: &PopuliTransportState, node: &NodeRecord) -> bool {
    match &state.required_scope {
        None => true,
        Some(req) => node.scope_id.as_deref().is_some_and(|s| s == req.as_ref()),
    }
}
