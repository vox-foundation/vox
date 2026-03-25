//! Wire types and routing hints for agent-to-agent messaging.

use serde::{Deserialize, Serialize};

/// Stable A2A wire type for remote task execution envelopes.
pub const REMOTE_TASK_ENVELOPE_TYPE: &str = "remote_task_envelope";
/// Stable A2A wire type for remote task execution acknowledgements.
pub const REMOTE_TASK_ACK_TYPE: &str = "remote_task_ack";
/// Stable A2A wire type for remote task execution results.
pub const REMOTE_TASK_RESULT_TYPE: &str = "remote_task_result";

/// Envelope sent across mesh A2A relay to request remote execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteTaskEnvelope {
    /// Idempotency key used by receivers to deduplicate requests.
    pub idempotency_key: String,
    /// Local task id from the originating orchestrator.
    pub task_id: u64,
    /// Originating repository id.
    pub repository_id: String,
    /// Requested capability hints encoded as JSON.
    pub capability_requirements_json: String,
    /// Opaque task payload contract for the receiver.
    pub payload: String,
}

/// Ack payload for a remote task envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteTaskAck {
    /// Idempotency key from the original envelope.
    pub idempotency_key: String,
    /// Whether receiver accepted the envelope.
    pub accepted: bool,
    /// Optional diagnostic detail.
    pub detail: Option<String>,
}

/// Result payload for a remote task envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteTaskResult {
    /// Idempotency key from the original envelope.
    pub idempotency_key: String,
    /// Whether remote execution succeeded.
    pub success: bool,
    /// Optional result payload.
    pub result: Option<String>,
    /// Optional error detail.
    pub error: Option<String>,
}

/// Database-persisted A2A message row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbA2AMessage {
    pub id: u64,
    pub message_uuid: String,
    pub sender_agent: String,
    pub receiver_agent: String,
    pub msg_type: String,
    pub payload: String,
    pub priority: i64,
    pub thread_id: Option<String>,
    pub acknowledged: bool,
    pub created_at: String,
    pub repository_id: String,
}

/// Routing hint for mens messaging.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum A2ARoute {
    /// Local in-memory delivery to an agent on the same node.
    Local,
    /// Send via direct HTTP relay to node.
    Relay(String),
    /// Persist in database for polling.
    Db,
}
