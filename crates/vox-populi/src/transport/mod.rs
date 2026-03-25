//! Minimal HTTP control plane for populi join / list / heartbeat / leave (loopback-first).
//!
//! When **`VOX_MESH_TOKEN`** is set and non-empty, all routes except **`GET /health`** require
//! `Authorization: Bearer <token>` (value is never logged). Bearer comparison uses
//! [`subtle::ConstantTimeEq`] on UTF-8 bytes when lengths match.
//!
//! When **`VOX_MESH_SCOPE_ID`** is set on the server process, **`POST /v1/populi/join`** and
//! **`POST /v1/populi/heartbeat`** require the JSON [`crate::NodeRecord::scope_id`] to match.

mod auth;
mod handlers;
mod router;
mod store;

pub use router::{PopuliHttpAuth, populi_http_app, populi_http_app_with_auth, router, serve};

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{PopuliRegistryError, PopuliRegistryFile};

/// Body for [`leave_node`]: remove a node id from the in-memory registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveRequest {
    /// Node id to remove (same as [`NodeRecord::id`]).
    pub id: String,
}

/// Request to deliver an A2A message to a local agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ADeliverRequest {
    /// Sender agent ID.
    pub sender_agent_id: String,
    /// Receiver agent ID.
    pub receiver_agent_id: String,
    /// The message type/schema name.
    pub message_type: String,
    /// The JSON or raw payload.
    pub payload: String,
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
    a2a_store_path: Option<PathBuf>,
    bootstrap_token: Option<Arc<str>>,
    bootstrap_expires_unix_ms: Option<u64>,
    bootstrap_used: Arc<AtomicBool>,
    /// When set, join/heartbeat must send the same [`NodeRecord::scope_id`].
    pub required_scope: Option<Arc<str>>,
}

impl PopuliTransportState {
    /// New empty in-memory registry; does **not** read `VOX_MESH_SCOPE_ID` (for tests).
    #[must_use]
    pub fn new() -> Self {
        Self::with_required_scope(None)
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
            a2a_store_path: None,
            bootstrap_token: None,
            bootstrap_expires_unix_ms: None,
            bootstrap_used: Arc::new(AtomicBool::new(false)),
            required_scope,
        }
    }

    /// Same as [`Self::new`] but sets [`Self::required_scope`] from **`VOX_MESH_SCOPE_ID`** when set.
    #[must_use]
    pub fn new_for_serve() -> Self {
        let mut s = Self::with_required_scope(crate::populi_scope_id_from_env());
        let store_path = store::a2a_store_path_from_env();
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
        s.a2a_store_path = store_path;
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
            let raw = crate::bounded_fs::read_utf8_path_capped(path).map_err(|e| {
                PopuliRegistryError::Io(std::io::Error::other(e.to_string()))
            })?;
            serde_json::from_str(&raw).map_err(|e| PopuliRegistryError::Json(e.to_string()))?
        } else {
            PopuliRegistryFile {
                schema_version: 1,
                nodes: Vec::new(),
            }
        };
        let store_path = store::a2a_store_path_from_env();
        let rows = if let Some(sp) = &store_path {
            store::load_a2a_store(sp).unwrap_or_default()
        } else {
            Vec::new()
        };
        let next_id = rows
            .iter()
            .map(|m| m.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        Ok(Self {
            inner: Arc::new(RwLock::new(reg)),
            a2a_messages: Arc::new(RwLock::new(rows)),
            a2a_id_gen: Arc::new(AtomicU64::new(next_id)),
            a2a_store_path: store_path,
            bootstrap_token: None,
            bootstrap_expires_unix_ms: None,
            bootstrap_used: Arc::new(AtomicBool::new(false)),
            required_scope: crate::populi_scope_id_from_env()
                .map(|s| Arc::from(s.into_boxed_str())),
        })
    }
}
