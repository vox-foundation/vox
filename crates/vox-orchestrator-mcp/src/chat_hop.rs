//! ChatHop JSONL dogfood trail (Tier D).
//!
//! Emitted **only** from orchestrator MCP (`dispatch` + chat message / agent
//! loop). The GUI contributes `trace_id` / `turn_id` as tool args — it must
//! never write this file. Path:
//! `ServerState::dogfood_trace_path_for("chat_hops.jsonl")`.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static HOP_SEQ: AtomicU64 = AtomicU64::new(0);

const SCHEMA_VERSION: &str = "chat_hop.v1";
const FILE_NAME: &str = "chat_hops.jsonl";

/// Kinds recorded on the hop trail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatHopKind {
    McpTool,
    ChatLlm,
    TurnBoundary,
    Retrieval,
}

/// Coarse turn / hop outcome for acceptance and triage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnOutcome {
    Ok,
    LlmError,
    IterationLimit,
    BudgetDenied,
    EmptyFrame,
    ToolEnvelopeError,
    DispatchError,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatHopRecord {
    pub schema_version: &'static str,
    pub hop_kind: ChatHopKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub hop_seq: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_outcome: Option<TurnOutcome>,
    pub duration_ms: u64,
    pub timestamp_ms: u64,
}

impl ChatHopRecord {
    pub fn new(
        hop_kind: ChatHopKind,
        success: bool,
        duration_ms: u64,
        turn_outcome: Option<TurnOutcome>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            hop_kind,
            trace_id: None,
            turn_id: None,
            session_id: None,
            hop_seq: HOP_SEQ.fetch_add(1, Ordering::Relaxed),
            tool_name: None,
            success,
            failure_class: None,
            turn_outcome,
            duration_ms,
            timestamp_ms: now_ms(),
        }
    }

    pub fn with_ids(
        mut self,
        trace_id: Option<String>,
        turn_id: Option<String>,
        session_id: Option<String>,
    ) -> Self {
        self.trace_id = redact_id(trace_id);
        self.turn_id = redact_id(turn_id);
        self.session_id = redact_id(session_id);
        self
    }

    pub fn with_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.tool_name = Some(tool_name.into());
        self
    }

    pub fn with_failure_class(mut self, class: impl Into<String>) -> Self {
        self.failure_class = Some(class.into());
        self
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Drop empty / whitespace-only ids; never log secret-looking blobs as ids.
fn redact_id(id: Option<String>) -> Option<String> {
    id.map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .filter(|s| !looks_like_secret(s))
}

fn looks_like_secret(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    lower.starts_with("sk-")
        || lower.starts_with("sk_")
        || lower.contains("api_key")
        || lower.contains("bearer ")
}

/// Resolve the dogfood ChatHop path, if dogfood tracing is configured.
pub fn chat_hop_path(state: &crate::server_state::ServerState) -> Option<PathBuf> {
    state.dogfood_trace_path_for(FILE_NAME)
}

/// Best-effort append. Errors are logged at debug and swallowed — hops must
/// never affect tool results.
pub fn append_chat_hop(path: &Path, record: &ChatHopRecord) {
    if let Err(e) = append_chat_hop_inner(path, record) {
        tracing::debug!(error = %e, path = %path.display(), "chat_hop append failed (ignored)");
    }
}

fn append_chat_hop_inner(path: &Path, record: &ChatHopRecord) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut line = serde_json::to_string(record)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    line.push('\n');
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    f.write_all(line.as_bytes())?;
    Ok(())
}

/// Fire-and-forget hop for an MCP tool dispatch result.
pub fn record_mcp_tool_hop(
    state: &crate::server_state::ServerState,
    tool_name: &str,
    trace_id: Option<String>,
    turn_id: Option<String>,
    session_id: Option<String>,
    success: bool,
    duration_ms: u64,
) {
    let Some(path) = chat_hop_path(state) else {
        return;
    };
    let outcome = if success {
        TurnOutcome::Ok
    } else {
        TurnOutcome::ToolEnvelopeError
    };
    let mut rec = ChatHopRecord::new(ChatHopKind::McpTool, success, duration_ms, Some(outcome))
        .with_ids(trace_id, turn_id, session_id)
        .with_tool(tool_name);
    if !success {
        rec = rec.with_failure_class("tool_envelope_or_dispatch");
    }
    append_chat_hop(&path, &rec);
}

/// Turn-boundary hop at the end of `vox_chat_message`.
pub fn record_turn_boundary(
    state: &crate::server_state::ServerState,
    trace_id: Option<String>,
    turn_id: Option<String>,
    session_id: Option<String>,
    success: bool,
    duration_ms: u64,
    turn_outcome: TurnOutcome,
) {
    let Some(path) = chat_hop_path(state) else {
        return;
    };
    let mut rec = ChatHopRecord::new(
        ChatHopKind::TurnBoundary,
        success,
        duration_ms,
        Some(turn_outcome),
    )
    .with_ids(trace_id, turn_id, session_id)
    .with_tool("vox_chat_message");
    if !success {
        let class = match turn_outcome {
            TurnOutcome::Ok => "ok",
            TurnOutcome::LlmError => "llm_error",
            TurnOutcome::IterationLimit => "iteration_limit",
            TurnOutcome::BudgetDenied => "budget_denied",
            TurnOutcome::EmptyFrame => "empty_frame",
            TurnOutcome::ToolEnvelopeError => "tool_envelope_error",
            TurnOutcome::DispatchError => "dispatch_error",
            TurnOutcome::Unknown => "unknown",
        };
        rec = rec.with_failure_class(class);
    }
    append_chat_hop(&path, &rec);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn append_writes_jsonl_under_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(FILE_NAME);
        let rec = ChatHopRecord::new(ChatHopKind::McpTool, true, 12, Some(TurnOutcome::Ok))
            .with_ids(
                Some("trace-1".into()),
                Some("turn-1".into()),
                Some("sess-1".into()),
            )
            .with_tool("vox_chat_message");
        append_chat_hop(&path, &rec);
        let body = fs::read_to_string(&path).expect("read hops");
        assert!(body.contains("\"schema_version\":\"chat_hop.v1\""));
        assert!(body.contains("\"trace_id\":\"trace-1\""));
        assert!(body.contains("\"turn_id\":\"turn-1\""));
        assert!(body.contains("mcp_tool"));
        assert!(!body.contains("sk-or-"));
    }

    #[test]
    fn redact_drops_secret_looking_ids() {
        let rec = ChatHopRecord::new(ChatHopKind::McpTool, false, 1, None).with_ids(
            Some("sk-or-v1-secret".into()),
            Some("turn-ok".into()),
            None,
        );
        assert!(rec.trace_id.is_none());
        assert_eq!(rec.turn_id.as_deref(), Some("turn-ok"));
    }

    #[test]
    fn looks_like_secret_detects_sk_prefix() {
        assert!(looks_like_secret("sk-or-v1-abc"));
        assert!(!looks_like_secret("trace-uuid-here"));
    }
}
