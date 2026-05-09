//! A2A (agent-to-agent) message types shared across vox-populi and vox-plugin-populi-mesh.

use serde::{Deserialize, Serialize};

/// Request to deliver an A2A message to a local agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ADeliverRequest {
    /// Sender agent id: non-empty **decimal digit** string after trim.
    pub sender_agent_id: String,
    /// Receiver agent id: same constraints as [`Self::sender_agent_id`].
    pub receiver_agent_id: String,
    /// The message type/schema name.
    pub message_type: String,
    /// The JSON or raw payload.
    pub payload: String,
    /// Idempotency key: duplicate delivers return the same `message_id` while pending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    /// Privacy / isolation class for claim-side policy (e.g. `public`, `private`, `trusted`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy_class: Option<String>,
    /// BLAKE3 digest of UTF-8 [`Self::payload`] as **64 hex** chars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_blake3_hex: Option<String>,
    /// Ed25519 signature (Standard base64, 64 bytes) over raw 32-byte BLAKE3 digest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_ed25519_sig_b64: Option<String>,
    /// JWE (JSON Web Encryption) block containing forwarded secrets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jwe_payload: Option<String>,
    /// Task priority (0=lowest, 255=highest).
    #[serde(default = "default_priority")]
    pub priority: u8,
    /// Task kind for donation policy filtering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_kind: Option<String>,
    /// Optional target model id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    /// W3C `traceparent` (e.g. `"00-{32hex}-{16hex}-01"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traceparent: Option<String>,
}

fn default_priority() -> u8 {
    128
}
