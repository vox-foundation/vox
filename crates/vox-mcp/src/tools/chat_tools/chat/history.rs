use super::super::params::{ChatHistoryParams, ChatTranscriptEntry};
use crate::params::ToolResult;
use crate::server::ServerState;

/// Return the full chat history for a session.
///
/// Pass `params.session_id` to retrieve the isolated transcript for a specific session.
/// When `session_id` is `None`, falls back to `"default"` which matches the baseline
/// session used by `chat_message` when no session id is provided.
pub async fn chat_history(state: &ServerState, params: ChatHistoryParams) -> String {
    let session_id = &params.session_id;
    let history_key = format!("chat_history:{session_id}");
    let orch = &state.orchestrator;
    let ctx_handle = orch.context_handle();
    let history: Vec<ChatTranscriptEntry> =
        match crate::sync_poison::poison_rw_read(ctx_handle.read(), "orchestrator context") {
            Ok(g) => g
                .get(&history_key)
                .and_then(|s: String| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            Err(e) => {
                tracing::warn!(error = %e, "chat_history: context poisoned");
                Vec::new()
            }
        };
    ToolResult::ok(history).to_json()
}
