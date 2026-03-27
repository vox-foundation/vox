//! A2A (Agent-to-Agent) MCP tools — send, inbox, ack, broadcast, history.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ServerState, ToolResult};
use vox_orchestrator::types::{A2AMessageType, AgentId};

const REM_A2A_ACK: &str = "List inbox with `a2a_inbox` and use a pending `message_id` for this agent; ids are consumed after ack.";

// ---------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------

/// MCP tool arguments: point-to-point message on the orchestrator A2A bus.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct A2ASendParams {
    /// Numeric id of the sending agent.
    pub sender_id: u64,
    /// Numeric id of the receiving agent.
    pub receiver_id: u64,
    /// Logical type string (e.g. `plan_handoff`, `scope_request`); unknown values become `FreeForm`.
    pub msg_type: String,
    /// Opaque UTF-8 payload stored in the message record.
    pub payload: String,
    /// Optional end-to-end correlation id for tracing.
    pub correlation_id: Option<String>,
    /// Optional tenant/session id for tracing and dashboards.
    pub tenant_id: Option<String>,
}

/// MCP tool arguments: fetch unacknowledged inbox entries for one agent.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct A2AInboxParams {
    /// Agent whose inbox is read.
    pub agent_id: u64,
}

/// MCP tool arguments: mark one inbox message as read/processed.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct A2AAckParams {
    /// Agent holding the inbox entry.
    pub agent_id: u64,
    /// Message id returned by send/broadcast tools.
    pub message_id: u64,
}

/// MCP tool arguments: fan-out to every agent except the sender.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct A2ABroadcastParams {
    /// Originating agent (excluded from delivery list).
    pub sender_id: u64,
    /// Same semantics as [`A2ASendParams::msg_type`].
    pub msg_type: String,
    /// Broadcast payload bytes as UTF-8 text.
    pub payload: String,
}

/// MCP tool arguments: audit trail query (`since_ms` filters by wall-clock ms; `limit` caps rows).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct A2AHistoryParams {
    /// Only messages at or after this epoch milliseconds (inclusive).
    pub since_ms: Option<u64>,
    /// Maximum rows to return (default applied in handler when `None`).
    pub limit: Option<usize>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// JSON row returned in inbox/history tool responses (mirrors orchestrator message bus records).
#[derive(Debug, Serialize)]
pub struct A2AMessageInfo {
    /// Stable message id in the bus.
    pub id: u64,
    /// Sender agent id.
    pub sender: u64,
    /// `None` for broadcast-style entries without a single receiver.
    pub receiver: Option<u64>,
    /// Resolved type name for the payload.
    pub msg_type: String,
    /// Raw message body.
    pub payload: String,
    /// Creation time in epoch milliseconds.
    pub timestamp_ms: u64,
    /// Whether the receiver has acked this message.
    pub acknowledged: bool,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_msg_type(s: &str) -> A2AMessageType {
    match s.to_lowercase().as_str() {
        "plan_handoff" | "planhandoff" => A2AMessageType::PlanHandoff,
        "scope_request" | "scoperequest" => A2AMessageType::ScopeRequest,
        "scope_grant" | "scopegrant" => A2AMessageType::ScopeGrant,
        "progress_update" | "progressupdate" => A2AMessageType::ProgressUpdate,
        "help_request" | "helprequest" => A2AMessageType::HelpRequest,
        "completion_notice" | "completionnotice" => A2AMessageType::CompletionNotice,
        "error_report" | "errorreport" => A2AMessageType::ErrorReport,
        "conflict_detected" | "conflictdetected" => A2AMessageType::ConflictDetected,
        "conflict_resolved" | "conflictresolved" => A2AMessageType::ConflictResolved,
        "vcs_event" | "vcsevent" => A2AMessageType::VcsEvent,
        "cancel_request" | "cancelrequest" => A2AMessageType::CancelRequest,
        "snapshot_share" | "snapshotshare" => A2AMessageType::SnapshotShare,
        "broadcast_news" | "broadcastnews" => A2AMessageType::BroadcastNews,
        "remote_task_envelope" | "remotetaskenvelope" => A2AMessageType::PlanHandoff,
        "remote_task_ack" | "remotetaskack" => A2AMessageType::ProgressUpdate,
        "remote_task_result" | "remotetaskresult" => A2AMessageType::CompletionNotice,
        _ => A2AMessageType::FreeForm,
    }
}

// Wire format matches [`A2AMessageType::into_str`] (snake_case) — same as DB `a2a_messages.msg_type`.
fn msg_type_wire(mt: &A2AMessageType) -> String {
    mt.into_str().to_string()
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Send a targeted A2A message from one agent to another (async).
pub async fn a2a_send(state: &ServerState, params: A2ASendParams) -> String {
    let orch = &state.orchestrator;

    let sender = AgentId(params.sender_id);
    let receiver = AgentId(params.receiver_id);
    let msg_type = parse_msg_type(&params.msg_type);

    let msg_id = orch.send_a2a(sender, receiver, msg_type.clone(), params.payload.clone());
    let mut relay_attempted = false;
    let mut relay_ok = false;
    if let Some(base) = vox_populi::http_lifecycle::populi_http_control_base_from_env() {
        if !base.trim().is_empty() {
            relay_attempted = true;
            let client = vox_populi::http_client::PopuliHttpClient::new(base).with_env_token();
            relay_ok = client
                .relay_a2a(&vox_populi::transport::A2ADeliverRequest {
                    sender_agent_id: params.sender_id.to_string(),
                    receiver_agent_id: params.receiver_id.to_string(),
                    message_type: msg_type_wire(&msg_type),
                    payload: params.payload.clone(),
                    idempotency_key: None,
                    privacy_class: None,
                    payload_blake3_hex: None,
                    worker_ed25519_sig_b64: None,
                })
                .await
                .is_ok();
        }
    }

    ToolResult::ok(serde_json::json!({
        "message_id": msg_id.0,
        "sender": params.sender_id,
        "receiver": params.receiver_id,
        "correlation_id": params.correlation_id,
        "tenant_id": params.tenant_id,
        "relay_attempted": relay_attempted,
        "relay_ok": relay_ok,
        "dropped_messages": orch.message_bus().dropped_messages(),
    }))
    .to_json()
}

/// Read unacknowledged messages in an agent's inbox (async).
pub async fn a2a_inbox(state: &ServerState, params: A2AInboxParams) -> String {
    let orch = &state.orchestrator;

    let agent_id = AgentId(params.agent_id);
    let mut messages: Vec<A2AMessageInfo> = orch
        .message_bus()
        .inbox(agent_id)
        .into_iter()
        .map(|m| A2AMessageInfo {
            id: m.id.0,
            sender: m.sender.0,
            receiver: m.receiver.map(|r| r.0),
            msg_type: msg_type_wire(&m.msg_type),
            payload: format!("{} [Received {}s ago]", m.payload, m.elapsed_ms() / 1000),
            timestamp_ms: m.timestamp_ms,
            acknowledged: m.acknowledged,
        })
        .collect();
    let mut remote_attempted = false;
    let mut remote_ok = false;
    if let Some(base) = vox_populi::http_lifecycle::populi_http_control_base_from_env() {
        if !base.trim().is_empty() {
            remote_attempted = true;
            let client = vox_populi::http_client::PopuliHttpClient::new(base).with_env_token();
            if let Ok(remote) = client.relay_a2a_inbox(&params.agent_id.to_string()).await {
                remote_ok = true;
                let mut seen = std::collections::HashSet::new();
                for m in &messages {
                    seen.insert(m.id);
                }
                for m in remote.messages {
                    if seen.insert(m.id) {
                        messages.push(A2AMessageInfo {
                            id: m.id,
                            sender: m.sender_agent_id.parse::<u64>().unwrap_or(0),
                            receiver: m.receiver_agent_id.parse::<u64>().ok(),
                            msg_type: m.message_type,
                            payload: m.payload,
                            timestamp_ms: m.created_unix_ms,
                            acknowledged: m.acknowledged,
                        });
                    }
                }
            }
        }
    }
    messages.sort_by_key(|m| m.timestamp_ms);

    ToolResult::ok(serde_json::json!({
        "agent_id": params.agent_id,
        "unread_count": messages.len(),
        "remote_attempted": remote_attempted,
        "remote_ok": remote_ok,
        "dropped_messages": orch.message_bus().dropped_messages(),
        "messages": messages,
    }))
    .to_json()
}

/// Acknowledge a message in an agent's inbox (async).
pub async fn a2a_ack(state: &ServerState, params: A2AAckParams) -> String {
    let orch = &state.orchestrator;

    let agent_id = AgentId(params.agent_id);
    let message_id = vox_orchestrator::types::MessageId(params.message_id);

    // Need mutable access to message_bus for ack
    let local_success = orch.message_bus_mut().acknowledge(agent_id, message_id);
    let mut remote_attempted = false;
    let mut remote_success = false;
    if let Some(base) = vox_populi::http_lifecycle::populi_http_control_base_from_env() {
        if !base.trim().is_empty() {
            remote_attempted = true;
            let client = vox_populi::http_client::PopuliHttpClient::new(base).with_env_token();
            remote_success = client
                .relay_a2a_ack(&params.agent_id.to_string(), params.message_id)
                .await
                .unwrap_or(false);
        }
    }
    let success = local_success || remote_success;

    if success {
        ToolResult::ok(serde_json::json!({
            "acknowledged": true,
            "message_id": params.message_id,
            "local_acknowledged": local_success,
            "remote_attempted": remote_attempted,
            "remote_acknowledged": remote_success,
        }))
        .to_json()
    } else {
        ToolResult::<String>::err_with_remediation(
            format!(
                "Message {} not found in agent {}'s inbox",
                params.message_id, params.agent_id
            ),
            REM_A2A_ACK,
        )
        .to_json()
    }
}

/// Broadcast an A2A message to all agents except sender (async).
pub async fn a2a_broadcast(state: &ServerState, params: A2ABroadcastParams) -> String {
    let orch = &state.orchestrator;

    let sender = AgentId(params.sender_id);
    let msg_type = parse_msg_type(&params.msg_type);

    let msg_id = orch.broadcast_a2a(sender, msg_type, params.payload);

    let agent_count = orch.agent_ids().len().saturating_sub(1);

    ToolResult::ok(serde_json::json!({
        "message_id": msg_id.0,
        "sender": params.sender_id,
        "delivered_to": agent_count,
        "dropped_messages": orch.message_bus().dropped_messages(),
    }))
    .to_json()
}

/// Query the A2A audit trail (async).
pub async fn a2a_history(state: &ServerState, params: A2AHistoryParams) -> String {
    let orch = &state.orchestrator;

    let limit = params.limit.unwrap_or(50);

    let messages: Vec<A2AMessageInfo> = if let Some(since) = params.since_ms {
        orch.message_bus()
            .audit_since(since)
            .into_iter()
            .take(limit)
            .map(|m| A2AMessageInfo {
                id: m.id.0,
                sender: m.sender.0,
                receiver: m.receiver.map(|r| r.0),
                msg_type: msg_type_wire(&m.msg_type),
                payload: format!("{} [Received {}s ago]", m.payload, m.elapsed_ms() / 1000),
                timestamp_ms: m.timestamp_ms,
                acknowledged: m.acknowledged,
            })
            .collect()
    } else {
        let bus_guard = orch.message_bus();
        bus_guard
            .audit_trail()
            .iter()
            .rev()
            .take(limit)
            .map(|m| A2AMessageInfo {
                id: m.id.0,
                sender: m.sender.0,
                receiver: m.receiver.map(|r| r.0),
                msg_type: msg_type_wire(&m.msg_type),
                payload: format!("{} [Received {}s ago]", m.payload, m.elapsed_ms() / 1000),
                timestamp_ms: m.timestamp_ms,
                acknowledged: m.acknowledged,
            })
            .collect()
    };

    ToolResult::ok(serde_json::json!({
        "total_messages": orch.message_bus().total_messages(),
        "dropped_messages": orch.message_bus().dropped_messages(),
        "returned": messages.len(),
        "messages": messages,
    }))
    .to_json()
}
