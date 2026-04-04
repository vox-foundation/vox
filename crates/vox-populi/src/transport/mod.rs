//! Minimal HTTP control plane for populi join / list / heartbeat / leave (loopback-first).
//!
//! When **any** mesh-class secret resolves via Clavis (`VOX_MESH_TOKEN` and/or worker/submitter/admin
//! role tokens, or optional **`VOX_MESH_JWT_HMAC_SECRET`** for HS256 JWTs), all routes except
//! **`GET /health`** and **`POST /v1/populi/bootstrap/exchange`**
//! require `Authorization: Bearer <token>` matching a configured role or valid JWT (`role` / `jti` / `exp`;
//! values are never logged).
//! Opaque bearer comparison uses [`subtle::ConstantTimeEq`] on UTF-8 bytes when lengths match.
//!
//! When **`VOX_MESH_SCOPE_ID`** is set on the server process, **`POST /v1/populi/join`** and
//! **`POST /v1/populi/heartbeat`** require the JSON [`crate::NodeRecord::scope_id`] to match.
//!
//! **Optional HTTP rate limiting:** **`VOX_MESH_HTTP_RATE_LIMIT`** = `1` / `true` / `on` / `yes`, plus
//! **`VOX_MESH_HTTP_RATE_LIMIT_PER_SEC`** / **`VOX_MESH_HTTP_RATE_LIMIT_BURST`**.

mod auth;
mod handlers;
mod mesh_replay;
mod result_attestation;
mod router;
mod store;

pub use auth::{PopuliAuthContext, PopuliBearerRole, PopuliMeshAuthRuntime};
pub use router::{PopuliHttpAuth, populi_http_app, populi_http_app_with_auth, router, serve};

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{PopuliRegistryError, PopuliRegistryFile};

/// Well-known A2A [`A2ADeliverRequest::message_type`] tokens for mesh job flows (Horde-style hooks).
/// Submit work to a receiver inbox (payload carries job definition).
pub const A2A_MESSAGE_JOB_SUBMIT: &str = "job_submit";
/// Worker-side claim notification (optional; inbox claim uses `claimer_node_id`).
pub const A2A_MESSAGE_JOB_CLAIM: &str = "job_claim";
/// Result payload from worker to submitter (convention; payload is JSON contract-defined).
pub const A2A_MESSAGE_JOB_RESULT: &str = "job_result";
/// Terminal failure from worker (convention).
pub const A2A_MESSAGE_JOB_FAIL: &str = "job_fail";

/// Body for [`leave_node`]: remove a node id from the in-memory registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveRequest {
    /// Node id to remove (same as [`NodeRecord::id`]).
    pub id: String,
}

/// Request to deliver an A2A message to a local agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ADeliverRequest {
    /// Sender agent id: non-empty **decimal digit** string after trim (orchestrator `AgentId` / `u64` wire form).
    pub sender_agent_id: String,
    /// Receiver agent id: same constraints as [`Self::sender_agent_id`].
    pub receiver_agent_id: String,
    /// The message type/schema name.
    pub message_type: String,
    /// The JSON or raw payload.
    pub payload: String,
    /// Idempotency key: duplicate delivers return the same `message_id` while pending. If omitted, each
    /// request gets a new `message_id` (server does **not** synthesize a default; see Populi / MCP docs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    /// Privacy / isolation class for claim-side policy (e.g. `public`, `private`, `trusted`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy_class: Option<String>,
    /// BLAKE3 digest of UTF-8 [`Self::payload`] as **64 hex** chars (`job_result` / `job_fail`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_blake3_hex: Option<String>,
    /// Ed25519 signature (Standard base64, 64 bytes) over raw 32-byte BLAKE3 digest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_ed25519_sig_b64: Option<String>,
    /// JWE (JSON Web Encryption) block containing forwarded Clavis secrets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jwe_payload: Option<String>,
}

/// Persisted A2A delivery envelope in the control plane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AStoredMessage {
    /// Monotonic message row id.
    pub id: u64,
    /// Sender agent ID.
    pub sender_agent_id: String,
    /// Receiver agent ID.
    pub receiver_agent_id: String,
    /// Message type / schema name.
    pub message_type: String,
    /// JSON or raw payload.
    pub payload: String,
    /// Control-plane wall time when stored (unix ms).
    pub created_unix_ms: u64,
    /// Whether the receiver has acked delivery.
    pub acknowledged: bool,
    /// Node id holding an inbox processing lease, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_holder_node_id: Option<String>,
    /// Wall time when [`Self::lease_holder_node_id`] expires (unix ms).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_expires_unix_ms: Option<u64>,
    /// Privacy / routing class copied from deliver (for worker claim policy).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy_class: Option<String>,
    /// Internal dedupe map key when [`A2ADeliverRequest::idempotency_key`] was set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_dedupe_key: Option<String>,
    /// Copied from deliver when attestation fields are set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_blake3_hex: Option<String>,
    /// Copied from deliver: Standard base64 Ed25519 signature over raw BLAKE3 digest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_ed25519_sig_b64: Option<String>,
    /// Copied from deliver: JWE block containing forwarded Clavis secrets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jwe_payload: Option<String>,
}

/// Reply from the control plane after an A2A deliver attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ADeliverResponse {
    /// Whether the message was accepted for storage/delivery.
    pub accepted: bool,
    /// Assigned [`A2AStoredMessage::id`] when accepted.
    pub message_id: u64,
}

/// Inbox poll: identify the receiving agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AInboxRequest {
    /// Receiver agent ID.
    pub receiver_agent_id: String,
    /// When set, only return messages unleased, leased to this node, or with expired lease; may refresh lease on first matching row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimer_node_id: Option<String>,
    /// Optional maximum row count for non-claimer inbox fetches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_messages: Option<usize>,
    /// Optional cursor for non-claimer inbox fetches (return rows with `id < before_message_id`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_message_id: Option<u64>,
}

/// Inbox poll: queued messages for the receiver.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AInboxResponse {
    /// Pending messages (order is transport-defined).
    pub messages: Vec<A2AStoredMessage>,
}

/// Ack a delivered inbox message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AAckRequest {
    /// Receiver agent ID.
    pub receiver_agent_id: String,
    /// [`A2AStoredMessage::id`] to acknowledge.
    pub message_id: u64,
}

/// Grant or refresh a **remote execution** lease for an opaque `scope_key` (correlation id for a task /
/// workflow slice / resource slot). Distinct from per-A2A-row inbox leases; stored in-memory on the
/// control plane only. TTL uses the same wall clock as inbox leases ([`a2a_lease_duration_ms`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteExecLeaseGrantRequest {
    /// Node id requesting the lease (must be registered via join).
    pub claimer_node_id: String,
    /// Opaque key identifying what is leased (e.g. `workflow_run:…` / `task:…`); trimmed; must be non-empty.
    pub scope_key: String,
}

/// Server response for [`RemoteExecLeaseGrantRequest`]: includes stable `lease_id` for renew/release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteExecLeaseGrantResponse {
    /// Monotonic decimal lease id (correlation handle).
    pub lease_id: String,
    /// Echo of normalized `scope_key`.
    pub scope_key: String,
    /// Holder after grant (same as request `claimer_node_id` when successful).
    pub holder_node_id: String,
    /// Wall time when the lease expires if not renewed (unix ms).
    pub expires_unix_ms: u64,
}

/// Extend an active remote execution lease held by `claimer_node_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteExecLeaseRenewRequest {
    /// [`RemoteExecLeaseGrantResponse::lease_id`].
    pub lease_id: String,
    /// Must match the current holder.
    pub claimer_node_id: String,
}

/// Release a remote execution lease held by `claimer_node_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteExecLeaseReleaseRequest {
    /// [`RemoteExecLeaseGrantResponse::lease_id`].
    pub lease_id: String,
    /// Must match the current holder.
    pub claimer_node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RemoteExecLeaseRow {
    pub(super) lease_id: String,
    pub(super) scope_key: String,
    pub(super) holder_node_id: String,
    pub(super) expires_unix_ms: u64,
}

/// One non-expired remote execution lease for observability ([`GET /v1/populi/exec/leases`]).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteExecLeaseListItem {
    /// Lease id returned by grant.
    pub lease_id: String,
    /// Correlation key (e.g. `task:<id>`).
    pub scope_key: String,
    /// Node id of the current holder.
    pub holder_node_id: String,
    /// Wall-clock expiry (Unix ms).
    pub expires_unix_ms: u64,
}

/// Response for [`GET /v1/populi/exec/leases`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteExecLeaseListResponse {
    /// Active leases after server-side expiry sweep.
    pub leases: Vec<RemoteExecLeaseListItem>,
}

/// Extend an active inbox lease held by `claimer_node_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ALeaseRenewRequest {
    /// Receiver agent ID (must match row).
    pub receiver_agent_id: String,
    /// [`A2AStoredMessage::id`].
    pub message_id: u64,
    /// Node id that already holds the lease.
    pub claimer_node_id: String,
}

/// Operator quarantine toggle (blocks A2A claims for a node).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminQuarantineRequest {
    /// [`NodeRecord::id`].
    pub node_id: String,
    /// When true, claimers with this node id cannot receive new leases.
    pub quarantined: bool,
}

/// Operator maintenance toggle (blocks new A2A claims for a node while enabled).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminMaintenanceRequest {
    /// [`NodeRecord::id`].
    pub node_id: String,
    /// When true, claimers with this node id cannot receive new leases.
    pub maintenance: bool,
    /// When `maintenance` is true, clear drain automatically at this Unix ms (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintenance_until_unix_ms: Option<u64>,
    /// When `maintenance` is true and `maintenance_until_unix_ms` is unset, server sets deadline to `now + min(for_ms, MAX_MAINTENANCE_FOR_MS)]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintenance_for_ms: Option<u64>,
}

/// Operator removal of a remote execution lease row (does not require holder cooperation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminExecLeaseRevokeRequest {
    /// [`RemoteExecLeaseGrantResponse::lease_id`].
    pub lease_id: String,
}

/// Request a one-time bootstrap exchange for mesh join.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapExchangeRequest {
    /// Ephemeral bootstrap token provisioned by `vox populi up`.
    pub bootstrap_token: String,
}

/// Response payload for bootstrap exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapExchangeResponse {
    /// Long-lived mesh bearer token (same as `VOX_MESH_TOKEN`).
    pub mesh_token: String,
    /// Optional scope id to join.
    pub scope_id: Option<String>,
}

/// Shared registry state for the HTTP server (in-memory; optionally persisted by callers).
#[derive(Clone)]
pub struct PopuliTransportState {
    inner: Arc<RwLock<PopuliRegistryFile>>,
    a2a_messages: Arc<RwLock<Vec<A2AStoredMessage>>>,
    a2a_id_gen: Arc<AtomicU64>,
    exec_leases: Arc<RwLock<Vec<RemoteExecLeaseRow>>>,
    exec_lease_id_gen: Arc<AtomicU64>,
    /// JWT `jti` replay + A2A idempotency keys; optionally persisted (`mesh-replay-state.json`).
    pub(crate) mesh_replay: Arc<mesh_replay::MeshReplayState>,
    a2a_store_path: Option<PathBuf>,
    exec_lease_store_path: Option<PathBuf>,
    bootstrap_token: Option<Arc<str>>,
    bootstrap_expires_unix_ms: Option<u64>,
    bootstrap_used: Arc<AtomicBool>,
    /// When set, join/heartbeat must send the same [`NodeRecord::scope_id`].
    pub required_scope: Option<Arc<str>>,
    /// Optional Ed25519 verify key from **`VOX_MESH_WORKER_RESULT_VERIFY_KEY`** for signed job results.
    pub(super) worker_result_verify_key: Option<[u8; 32]>,
}

impl PopuliTransportState {
    /// New empty in-memory registry; does **not** read `VOX_MESH_SCOPE_ID` (for tests).
    #[must_use]
    pub fn new() -> Self {
        Self::with_required_scope(None)
    }

    /// Override worker result attestation key (primarily tests; [`Self::new_for_serve`] reads Clavis otherwise).
    #[must_use]
    pub fn with_worker_result_verify_key(mut self, key: Option<[u8; 32]>) -> Self {
        self.worker_result_verify_key = key;
        self
    }

    /// New empty registry and optional required scope (trimmed; empty string becomes `None`).
    #[must_use]
    pub fn with_required_scope(scope: Option<String>) -> Self {
        let required_scope = scope
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(|s| Arc::from(s.into_boxed_str()));
        Self {
            inner: Arc::new(RwLock::new(PopuliRegistryFile {
                schema_version: 1,
                nodes: Vec::new(),
            })),
            a2a_messages: Arc::new(RwLock::new(Vec::new())),
            a2a_id_gen: Arc::new(AtomicU64::new(1)),
            exec_leases: Arc::new(RwLock::new(Vec::new())),
            exec_lease_id_gen: Arc::new(AtomicU64::new(1)),
            mesh_replay: mesh_replay::MeshReplayState::in_memory(),
            a2a_store_path: None,
            exec_lease_store_path: None,
            bootstrap_token: None,
            bootstrap_expires_unix_ms: None,
            bootstrap_used: Arc::new(AtomicBool::new(false)),
            required_scope,
            worker_result_verify_key: None,
        }
    }

    /// Same as [`Self::new`] but sets [`Self::required_scope`] from **`VOX_MESH_SCOPE_ID`** when set.
    #[must_use]
    pub fn new_for_serve() -> Self {
        let mut s = Self::with_required_scope(crate::populi_scope_id_from_env());
        let store_path = store::a2a_store_path_from_env();
        let exec_lease_store_path = store::exec_lease_store_path_from_env(store_path.as_ref());
        if let Some(path) = &store_path
            && let Ok(existing) = store::load_a2a_store(path)
        {
            let next_id = existing
                .iter()
                .map(|m| m.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1);
            s.a2a_messages = Arc::new(RwLock::new(existing));
            s.a2a_id_gen = Arc::new(AtomicU64::new(next_id));
        }
        if let Some(path) = &exec_lease_store_path
            && let Ok(existing) = store::load_exec_lease_store(path)
        {
            let next_lease_id = existing
                .iter()
                .filter_map(|r| r.lease_id.parse::<u64>().ok())
                .max()
                .unwrap_or(0)
                .saturating_add(1);
            s.exec_leases = Arc::new(RwLock::new(existing));
            s.exec_lease_id_gen = Arc::new(AtomicU64::new(next_lease_id));
        }
        s.a2a_store_path = store_path;
        s.exec_lease_store_path = exec_lease_store_path;
        s.bootstrap_token = std::env::var("VOX_MESH_BOOTSTRAP_TOKEN")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .map(Arc::from);
        s.bootstrap_expires_unix_ms = std::env::var("VOX_MESH_BOOTSTRAP_EXPIRES_UNIX_MS")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|ms| *ms > crate::now_ms());
        s
    }

    /// Load initial snapshot from disk (best-effort) and apply scope from **`VOX_MESH_SCOPE_ID`**.
    pub async fn load_from_path(path: &std::path::Path) -> Result<Self, PopuliRegistryError> {
        let reg = if path.is_file() {
            let raw = vox_bounded_fs::read_utf8_path_capped(path)
                .map_err(|e| PopuliRegistryError::Io(std::io::Error::other(e.to_string())))?;
            serde_json::from_str(&raw).map_err(|e| PopuliRegistryError::Json(e.to_string()))?
        } else {
            PopuliRegistryFile {
                schema_version: 1,
                nodes: Vec::new(),
            }
        };
        let store_path = store::a2a_store_path_from_env();
        let exec_lease_store_path = store::exec_lease_store_path_from_env(store_path.as_ref());
        let rows = if let Some(sp) = &store_path {
            store::load_a2a_store(sp).unwrap_or_default()
        } else {
            Vec::new()
        };
        let exec_lease_rows = if let Some(sp) = &exec_lease_store_path {
            store::load_exec_lease_store(sp).unwrap_or_default()
        } else {
            Vec::new()
        };
        let next_id = rows
            .iter()
            .map(|m| m.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let replay_path = mesh_replay::mesh_replay_persist_path(store_path.as_ref());
        Ok(Self {
            inner: Arc::new(RwLock::new(reg)),
            a2a_messages: Arc::new(RwLock::new(rows)),
            a2a_id_gen: Arc::new(AtomicU64::new(next_id)),
            exec_leases: Arc::new(RwLock::new(exec_lease_rows.clone())),
            exec_lease_id_gen: Arc::new(AtomicU64::new(
                exec_lease_rows
                    .iter()
                    .filter_map(|r| r.lease_id.parse::<u64>().ok())
                    .max()
                    .unwrap_or(0)
                    .saturating_add(1),
            )),
            mesh_replay: mesh_replay::MeshReplayState::load(replay_path),
            a2a_store_path: store_path,
            exec_lease_store_path,
            bootstrap_token: None,
            bootstrap_expires_unix_ms: None,
            bootstrap_used: Arc::new(AtomicBool::new(false)),
            required_scope: crate::populi_scope_id_from_env()
                .map(|s| Arc::from(s.into_boxed_str())),
            worker_result_verify_key: worker_result_verify_key_resolved(),
        })
    }
}

fn worker_result_verify_key_resolved() -> Option<[u8; 32]> {
    let resolved = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxMeshWorkerResultVerifyKey);
    let raw = resolved.expose()?;
    let t = raw.trim();
    if t.is_empty() {
        return None;
    }
    match result_attestation::parse_ed25519_public_key_bytes(t) {
        Ok(k) => Some(k),
        Err(e) => {
            tracing::warn!(
                error = %e,
                "VOX_MESH_WORKER_RESULT_VERIFY_KEY is invalid; job_result attestation disabled"
            );
            None
        }
    }
}

/// Optional server-side staleness window: hide nodes whose `last_seen_unix_ms` is older than this
/// many milliseconds from [`crate::now_ms`]. Unset or `0` = no pruning (default).
#[must_use]
pub(super) fn server_stale_prune_ms() -> Option<u64> {
    std::env::var("VOX_MESH_SERVER_STALE_PRUNE_MS")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .filter(|&n| n > 0)
}

/// Max in-memory A2A rows before oldest messages are dropped (persisted store is rewritten).
#[must_use]
pub(super) fn a2a_in_memory_cap() -> usize {
    const DEFAULT: usize = 50_000;
    /// Allow small caps for tests and single-node dev; operators should still use ≥100 in prod.
    const MIN: usize = 1;
    const MAX: usize = 500_000;
    std::env::var("VOX_MESH_A2A_MAX_MESSAGES")
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .map(|n| n.clamp(MIN, MAX))
        .unwrap_or(DEFAULT)
}

/// Drop expired inbox leases in-place (requeue semantics).
pub(super) fn a2a_sweep_expired_leases(messages: &mut [A2AStoredMessage], now_ms: u64) {
    for m in messages.iter_mut() {
        if m.acknowledged {
            continue;
        }
        if m.lease_expires_unix_ms.is_some_and(|exp| exp <= now_ms) {
            m.lease_holder_node_id = None;
            m.lease_expires_unix_ms = None;
        }
    }
}

/// Drop expired remote execution leases (same wall clock as inbox leases).
pub(super) fn exec_lease_sweep(rows: &mut Vec<RemoteExecLeaseRow>, now_ms: u64) {
    rows.retain(|r| r.expires_unix_ms > now_ms);
}

/// Inbox lease duration in milliseconds (claimer flows). Override with **`VOX_MESH_A2A_LEASE_MS`**.
#[must_use]
pub(super) fn a2a_lease_duration_ms() -> u64 {
    const DEFAULT: u64 = 120_000;
    const MIN: u64 = 1_000;
    const MAX: u64 = 3_600_000;
    std::env::var("VOX_MESH_A2A_LEASE_MS")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|n| n.clamp(MIN, MAX))
        .unwrap_or(DEFAULT)
}
