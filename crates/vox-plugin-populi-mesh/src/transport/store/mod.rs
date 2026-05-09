//! Mesh storage helpers — JSON-file persistence for A2A inbox, exec leases, and dispatch results.
//!
//! These functions are the in-process fallback when no durable (Turso/VoxDb) store is attached.

use super::{A2AStoredMessage, DispatchResponse, PopuliTransportState, RemoteExecLeaseRow};
use crate::PopuliRegistryError;

use std::path::PathBuf;

pub fn a2a_store_path_from_env() -> Option<PathBuf> {
    if let Some(v) =
        vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMeshA2aStorePath).expose()
    {
        let t = v.trim();
        if !t.is_empty() {
            return Some(PathBuf::from(t));
        }
    }
    let mut p = crate::local_registry_path();
    p.set_file_name("a2a-store.json");
    Some(p)
}

pub fn load_a2a_store(
    path: &std::path::Path,
) -> Result<Vec<A2AStoredMessage>, PopuliRegistryError> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let raw = vox_bounded_fs::read_utf8_path_capped(path)
        .map_err(|e| PopuliRegistryError::Io(std::io::Error::other(e.to_string())))?;
    serde_json::from_str(&raw).map_err(|e| PopuliRegistryError::Json(e.to_string()))
}

pub fn exec_lease_store_path_from_env(a2a: Option<&PathBuf>) -> Option<PathBuf> {
    if let Some(v) =
        vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMeshExecLeaseStorePath).expose()
    {
        let t = v.trim();
        if !t.is_empty() {
            return Some(PathBuf::from(t));
        }
    }
    if let Some(a) = a2a {
        let mut p = a.clone();
        p.set_file_name("exec-lease-store.json");
        return Some(p);
    }
    let mut p = crate::local_registry_path();
    p.set_file_name("exec-lease-store.json");
    Some(p)
}

pub fn load_exec_lease_store(
    path: &std::path::Path,
) -> Result<Vec<RemoteExecLeaseRow>, PopuliRegistryError> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let raw = vox_bounded_fs::read_utf8_path_capped(path)
        .map_err(|e| PopuliRegistryError::Io(std::io::Error::other(e.to_string())))?;
    serde_json::from_str(&raw).map_err(|e| PopuliRegistryError::Json(e.to_string()))
}

pub fn dispatch_results_store_path_from_env(a2a: Option<&PathBuf>) -> Option<PathBuf> {
    if let Some(v) =
        vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMeshDispatchStorePath).expose()
    {
        let t = v.trim();
        if !t.is_empty() {
            return Some(PathBuf::from(t));
        }
    }
    if let Some(a) = a2a {
        let mut p = a.clone();
        p.set_file_name("dispatch-store.json");
        return Some(p);
    }
    let mut p = crate::local_registry_path();
    p.set_file_name("dispatch-store.json");
    Some(p)
}

pub fn load_dispatch_results_store(
    path: &std::path::Path,
) -> Result<std::collections::HashMap<String, DispatchResponse>, PopuliRegistryError> {
    if !path.is_file() {
        return Ok(std::collections::HashMap::new());
    }
    let raw = vox_bounded_fs::read_utf8_path_capped(path)
        .map_err(|e| PopuliRegistryError::Io(std::io::Error::other(e.to_string())))?;
    serde_json::from_str(&raw).map_err(|e| PopuliRegistryError::Json(e.to_string()))
}

pub fn persist_a2a_store(
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

pub fn persist_exec_lease_store(
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

pub fn persist_dispatch_results_store(
    path: &std::path::Path,
    map: &dashmap::DashMap<String, DispatchResponse>,
) -> Result<(), PopuliRegistryError> {
    let start = std::time::Instant::now();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(PopuliRegistryError::Io)?;
    }
    let snapshot: std::collections::HashMap<_, _> = map
        .iter()
        .map(|e| (e.key().clone(), e.value().clone()))
        .collect();
    let entries = snapshot.len();
    let payload = serde_json::to_string_pretty(&snapshot)
        .map_err(|e| PopuliRegistryError::Json(e.to_string()))?;
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

pub fn scope_ok(state: &PopuliTransportState, node: &crate::NodeRecord) -> bool {
    match &state.required_scope {
        None => true,
        Some(req) => node.scope_id.as_deref().is_some_and(|s| s == req.as_ref()),
    }
}
