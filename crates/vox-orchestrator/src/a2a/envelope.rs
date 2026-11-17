//! Wire types and routing hints for agent-to-agent messaging.

use serde::{Deserialize, Serialize};

/// Stable A2A wire type for remote task execution envelopes.
pub const REMOTE_TASK_ENVELOPE_TYPE: &str = "remote_task_envelope";
/// Stable A2A wire type for remote task execution acknowledgements.
pub const REMOTE_TASK_ACK_TYPE: &str = "remote_task_ack";
/// Stable A2A wire type for remote task execution results.
pub const REMOTE_TASK_RESULT_TYPE: &str = "remote_task_result";
/// Stable A2A wire type for best-effort remote task cancellation hints.
pub const REMOTE_TASK_CANCEL_TYPE: &str = "remote_task_cancel";
// `OP_FRAGMENT_SYNC_TYPE` lives in `dispatch::op_fragment_sync` (the canonical
// definition consumed by the gossip path); do not re-declare here.

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
    /// Optional mesh privacy hint for downstream claim policy (`public`, `private`, `trusted`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy_class: Option<String>,
    /// Populi tenancy scope when mirroring control-plane policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub populi_scope_id: Option<String>,
    /// Originator wall clock when the envelope was emitted (unix ms).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submitted_unix_ms: Option<u64>,
    /// Populi execution lease id when lease-gated remote execution is active (ADR 017).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exec_lease_id: Option<String>,
    /// Optional reconstruction / campaign grouping id for workers and telemetry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_id: Option<String>,
    /// Optional JSON array or object of durable artifact references (URIs, ids) for retrieval-first handoff.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_refs_json: Option<String>,
    /// Optional logical session id for downstream context lookup / anti-bleed routing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Optional logical thread id for branch continuity within a session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    /// Optional serialized canonical context envelope carried directly on the wire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_envelope_json: Option<String>,
    /// Optional portable harness contract carried with the relay for stage/role/gate transparency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness_spec_json: Option<String>,
    /// Phase C: parent task id propagated from the caller's TRACE_CTX. None for root tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_task_id: Option<u64>,
    /// Phase C: agent identifier that issued this dispatch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caller_agent_id: Option<String>,
    /// Phase C: trace identifier shared across the entire call tree (UUID v4 string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Phase C: number of agent-to-agent hops from the root; root = 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span_depth: Option<u16>,
    /// P2-T4: content-hash of the workflow bundle this envelope dispatches.
    /// Receivers consult their bundle store; on miss they emit a
    /// `bundle_request` A2A message back to the sender.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_ref: Option<vox_package::bundle::BundleRef>,
    /// P2-T4: inline base64-encoded bundle bytes when the bundle is below
    /// the size threshold (default 1 MiB). When set, receivers skip the
    /// `bundle_request` round-trip and use these bytes directly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_inline_b64: Option<String>,
    /// Track B: BLAKE3 hex of the decoded `.vox` source. REQUIRED when
    /// `exec_source_b64` is set; a missing or mismatched hash refuses execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exec_source_b64: Option<String>,
    /// Track B: BLAKE3 hex of the decoded `.vox` source. REQUIRED when
    /// `exec_source_b64` is set; a missing or mismatched hash refuses execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exec_source_blake3_hex: Option<String>,
    /// Canonical causal swarm metadata (`CONC-01`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub causal_stamp: Option<vox_orchestrator_types::CausalSwarmStamp>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_envelope_without_trace_fields_deserializes() {
        let json = r#"{
            "idempotency_key": "k1",
            "task_id": 7,
            "repository_id": "repo",
            "capability_requirements_json": "{}",
            "payload": "test"
        }"#;
        let envelope: RemoteTaskEnvelope = serde_json::from_str(json).expect("deserialize");
        assert_eq!(envelope.task_id, 7);
        assert!(envelope.parent_task_id.is_none());
        assert!(envelope.trace_id.is_none());
        assert!(envelope.span_depth.is_none());
        assert!(envelope.causal_stamp.is_none());
    }

    #[test]
    fn envelope_with_trace_fields_round_trips() {
        let envelope = RemoteTaskEnvelope {
            idempotency_key: "k1".into(),
            task_id: 7,
            repository_id: "repo".into(),
            capability_requirements_json: "{}".into(),
            payload: "test".into(),
            privacy_class: None,
            populi_scope_id: None,
            submitted_unix_ms: None,
            exec_lease_id: None,
            campaign_id: None,
            artifact_refs_json: None,
            session_id: None,
            thread_id: None,
            context_envelope_json: None,
            harness_spec_json: None,
            parent_task_id: Some(5),
            caller_agent_id: Some("agent-3".into()),
            trace_id: Some("trace-123".into()),
            span_depth: Some(2),
            bundle_ref: None,
            bundle_inline_b64: None,
            exec_source_b64: None,
            exec_source_blake3_hex: None,
            causal_stamp: None,
        };
        let json = serde_json::to_string(&envelope).unwrap();
        let back: RemoteTaskEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(back.parent_task_id, Some(5));
        assert_eq!(back.span_depth, Some(2));
        assert_eq!(back.trace_id.as_deref(), Some("trace-123"));
    }

    #[test]
    fn envelope_with_exec_source_round_trips_and_legacy_omits_it() {
        // Legacy envelope (no exec fields) → both default to None (back-compat).
        let legacy: RemoteTaskEnvelope = serde_json::from_str(
            r#"{"idempotency_key":"k","task_id":1,"repository_id":"r","capability_requirements_json":"{}","payload":"p"}"#,
        )
        .expect("legacy deserialize");
        assert!(legacy.exec_source_b64.is_none());
        assert!(legacy.exec_source_blake3_hex.is_none());

        // With exec fields set, they round-trip and are emitted on the wire.
        let mut env = legacy;
        env.exec_source_b64 = Some("c291cmNl".into());
        env.exec_source_blake3_hex = Some("deadbeef".into());
        let json = serde_json::to_string(&env).unwrap();
        assert!(json.contains("exec_source_b64"));
        let back: RemoteTaskEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(back.exec_source_b64.as_deref(), Some("c291cmNl"));
        assert_eq!(back.exec_source_blake3_hex.as_deref(), Some("deadbeef"));
    }

    #[test]
    fn causal_stamp_round_trips_and_omits_when_none_across_envelopes() {
        let stamp = vox_orchestrator_types::CausalSwarmStamp {
            swarm_seq: 42,
            channel: "#verify".to_string(),
            traceparent: "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".to_string(),
            span_id: "00f067aa0ba902b7".to_string(),
            compute_tier: vox_orchestrator_types::ComputeTier::PopuliQuicPeer,
            file_lock_prefix: Some("crates/vox-orchestrator/".to_string()),
            workspace_mode: vox_orchestrator_types::SwarmWorkspaceMode::Share,
            parked_since_seq: Some(40),
        };
        let mut env: RemoteTaskEnvelope = serde_json::from_str(
            r#"{"idempotency_key":"k","task_id":1,"repository_id":"r","capability_requirements_json":"{}","payload":"p"}"#,
        )
        .expect("legacy envelope");
        assert!(env.causal_stamp.is_none());
        env.causal_stamp = Some(stamp.clone());
        let env_json = serde_json::to_string(&env).expect("serialize envelope");
        assert!(env_json.contains("causal_stamp"));
        let env_back: RemoteTaskEnvelope =
            serde_json::from_str(&env_json).expect("deserialize envelope");
        assert_eq!(env_back.causal_stamp, Some(stamp.clone()));

        let legacy_db: DbA2AMessage = serde_json::from_str(
            r#"{"id":1,"message_uuid":"u1","sender_agent":"a1","receiver_agent":"a2","msg_type":"t","payload":"{}","priority":1,"thread_id":null,"acknowledged":false,"created_at":"2026-11-16","repository_id":"repo"}"#,
        )
        .expect("legacy db message");
        assert!(legacy_db.causal_stamp.is_none());
        let mut stamped_db = legacy_db;
        stamped_db.causal_stamp = Some(stamp.clone());
        let db_json = serde_json::to_string(&stamped_db).expect("serialize db message");
        let db_back: DbA2AMessage = serde_json::from_str(&db_json).expect("deserialize db message");
        assert_eq!(db_back.causal_stamp, Some(stamp));
    }
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

/// Cancel / revoke hint for a previously delivered remote task envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteTaskCancel {
    /// Idempotency key from the original envelope.
    pub idempotency_key: String,
    /// Originating orchestrator task id.
    pub task_id: u64,
    /// Optional operator or system reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Result payload for a remote task envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteTaskResult {
    /// Idempotency key from the original envelope.
    pub idempotency_key: String,
    /// Originating orchestrator task id when the worker includes it (avoids parsing [`Self::idempotency_key`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<u64>,
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
    /// Canonical causal swarm metadata (`CONC-01`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub causal_stamp: Option<vox_orchestrator_types::CausalSwarmStamp>,
}

/// Canonical delivery plane names shared by orchestrator docs and MCP A2A responses.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum A2ADeliveryPlane {
    /// In-process mailbox delivery via the orchestrator message bus.
    LocalEphemeral,
    /// Durable local storage via the orchestrator DB inbox.
    LocalDurable,
    /// Remote Populi mesh relay over HTTP.
    RemoteMesh,
}

impl A2ADeliveryPlane {
    /// Stable string used in JSON responses and protocol docs.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalEphemeral => "local_ephemeral",
            Self::LocalDurable => "local_durable",
            Self::RemoteMesh => "remote_mesh",
        }
    }

    /// Legacy MCP route token preserved for compatibility.
    #[must_use]
    pub fn legacy_route_token(self) -> &'static str {
        match self {
            Self::LocalEphemeral => "local",
            Self::LocalDurable => "db",
            Self::RemoteMesh => "mesh",
        }
    }
}

/// Canonical inbox source names exposed by MCP A2A reads.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum A2AInboxPlane {
    /// Read only the local in-process inbox.
    Local,
    /// Read only the Populi mesh inbox.
    Mesh,
    /// Merge local and mesh inbox sources.
    Merged,
}

impl A2AInboxPlane {
    /// Stable string used in JSON responses and protocol docs.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Mesh => "mesh",
            Self::Merged => "merged",
        }
    }
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
