//! Durable mesh storage — [`MeshStore`] trait + VoxDb and in-memory backends.
//!
//! Three conceptual stores (A2A inbox, exec leases, dispatch results) are unified behind the
//! [`MeshStore`] trait. The default implementation uses the caller-supplied [`vox_db::VoxDb`]
//! handle so mesh data lives in the same Turso database as the rest of the Vox ecosystem.
//!
//! For unit tests [`InMemoryMeshStore`] provides an identical surface with no persistence.
//!
//! # Span attributes (§2.8)
//! - `vox.mesh.store.op` — operation name
//! - `vox.mesh.store.duration_ms`
//! - `vox.mesh.store.row_count` — list ops
//! - `vox.mesh.store.error` — only on failure

use std::collections::HashMap;

use async_trait::async_trait;

use super::{A2AStoredMessage, DispatchResponse, RemoteExecLeaseRow};
use crate::PopuliRegistryError;

pub mod in_memory;
pub mod voxdb;

#[cfg(test)]
mod tests;

pub use in_memory::InMemoryMeshStore;
pub use voxdb::VoxDbMeshStore;

// ──────────────────────────────────────────────────────────── errors ─

/// Errors returned by every [`MeshStore`] operation.
#[derive(Debug, thiserror::Error)]
pub enum MeshStoreError {
    #[error("mesh store I/O: {0}")]
    Io(String),
    #[error("mesh store schema mismatch: store is v{stored}, code expects v{expected}")]
    SchemaMismatch { stored: u32, expected: u32 },
    #[error("mesh store locked by another process")]
    LockContention,
    #[error("mesh store corrupt: {0}")]
    Corrupt(String),
    #[error("mesh store serialization: {0}")]
    Serialization(String),
    #[error("mesh store: {0}")]
    Other(String),
}

impl From<MeshStoreError> for PopuliRegistryError {
    fn from(e: MeshStoreError) -> Self {
        PopuliRegistryError::Json(e.to_string())
    }
}

// ──────────────────────────────────────────── pagination / ack types ─

/// Cursor for paginated A2A inbox reads.
#[derive(Debug, Clone, Default)]
pub struct A2APage {
    /// Only return messages for this receiver; `None` = all receivers.
    pub receiver_agent_id: Option<String>,
    /// Only return rows with `id > since_id`.
    pub since_id: Option<u64>,
    /// Maximum rows to return; `None` = no limit.
    pub limit: Option<usize>,
    /// When `false` (default) skip rows where `acknowledged = true`.
    pub include_acked: bool,
}

/// Ack payload for [`MeshStore::ack_a2a`].
#[derive(Debug, Clone)]
pub struct A2AAck {
    /// Mark as acknowledged (`true`) or un-acknowledged (`false`).
    pub acknowledged: bool,
    /// Wall-clock time of the ack (unix ms).
    pub acked_unix_ms: u64,
}

// ────────────────────────────────────────── integrity report types ───

/// A single integrity finding from [`MeshStore::integrity_check`].
#[derive(Debug, Clone)]
pub struct IntegrityFinding {
    /// Short machine-readable code (e.g. `"dedupe_violation"`).
    pub code: String,
    /// Human-readable detail.
    pub detail: String,
}

/// Report returned by [`MeshStore::integrity_check`].
#[derive(Debug, Clone)]
pub struct IntegrityReport {
    /// `true` when no findings detected.
    pub ok: bool,
    /// Ordered list of findings; empty when `ok`.
    pub findings: Vec<IntegrityFinding>,
    /// Schema version read from the store.
    pub schema_version: u32,
}

// ──────────────────────────────────────────────────── trait ──────────

/// Unified durable storage for A2A inbox / exec leases / dispatch results.
#[async_trait]
pub trait MeshStore: Send + Sync {
    // ── A2A inbox ──────────────────────────────────────────────────
    /// Persist (insert-or-update) an A2A message row.
    async fn put_a2a(&self, msg: &A2AStoredMessage) -> Result<(), MeshStoreError>;

    /// Paginated inbox read; rows returned in ascending `id` order.
    async fn list_a2a(&self, page: A2APage) -> Result<Vec<A2AStoredMessage>, MeshStoreError>;

    /// Acknowledge or un-acknowledge a stored message.
    async fn ack_a2a(&self, message_id: u64, ack: A2AAck) -> Result<(), MeshStoreError>;

    /// Load all A2A rows (used once at startup to populate in-memory cache).
    async fn load_all_a2a(&self) -> Result<Vec<A2AStoredMessage>, MeshStoreError>;

    // ── exec leases ────────────────────────────────────────────────
    /// Insert or replace an exec lease row.
    async fn put_exec_lease(&self, row: &RemoteExecLeaseRow) -> Result<(), MeshStoreError>;

    /// Return active (non-revoked) exec lease rows.
    async fn list_exec_leases(&self) -> Result<Vec<RemoteExecLeaseRow>, MeshStoreError>;

    /// Mark a lease as revoked (tombstone; does not hard-delete).
    async fn revoke_exec_lease(&self, lease_id: &str) -> Result<(), MeshStoreError>;

    /// Hard-delete an exec lease row (admin endpoint).
    async fn delete_exec_lease(&self, lease_id: &str) -> Result<(), MeshStoreError>;

    // ── dispatch results ───────────────────────────────────────────
    /// Upsert a dispatch result by key.
    async fn put_dispatch_result(
        &self,
        key: &str,
        value: &DispatchResponse,
    ) -> Result<(), MeshStoreError>;

    /// Retrieve a dispatch result by key; `None` when absent.
    async fn get_dispatch_result(
        &self,
        key: &str,
    ) -> Result<Option<DispatchResponse>, MeshStoreError>;

    /// Load all dispatch results (used once at startup).
    async fn load_all_dispatch_results(
        &self,
    ) -> Result<HashMap<String, DispatchResponse>, MeshStoreError>;

    // ── meta ───────────────────────────────────────────────────────
    /// Schema version; always `1` for both backends.
    fn schema_version(&self) -> u32;

    /// Run a structural integrity check and return a report.
    async fn integrity_check(&self) -> Result<IntegrityReport, MeshStoreError>;
}

// ──────────────────────────────────── legacy path helpers (kept) ─────
// These functions remain so `handlers.rs` keeps compiling unchanged during the
// transition. When `mesh_store` is `Some`, handlers delegate through it; these
// functions are the JSON-file fallback when `mesh_store` is `None`.

pub use legacy::{
    a2a_store_path_from_env, dispatch_results_store_path_from_env, exec_lease_store_path_from_env,
    load_a2a_store, load_dispatch_results_store, load_exec_lease_store, persist_a2a_store,
    persist_dispatch_results_store, persist_exec_lease_store, scope_ok,
};

mod legacy {
    use std::path::PathBuf;

    use super::super::{
        A2AStoredMessage, DispatchResponse, PopuliTransportState, RemoteExecLeaseRow,
    };
    use crate::PopuliRegistryError;

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
        let payload = serde_json::to_string_pretty(rows)
            .map_err(|e| PopuliRegistryError::Json(e.to_string()))?;
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
        let payload = serde_json::to_string_pretty(rows)
            .map_err(|e| PopuliRegistryError::Json(e.to_string()))?;
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
}
