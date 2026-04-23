use super::super::now_ts;
use super::super::params::ChatTranscriptEntry;
use crate::mcp_tools::server_state::ServerState;

pub(crate) fn workspace_turn_to_chat_entry(
    row: vox_db::WorkspaceTranscriptTurnRow,
) -> ChatTranscriptEntry {
    let context_files: Vec<String> =
        serde_json::from_str(&row.context_files_json).unwrap_or_default();
    let timestamp = if row.created_unix > 0 {
        row.created_unix
    } else {
        now_ts()
    };
    let id = if !row.external_turn_id.is_empty() {
        row.external_turn_id
    } else {
        format!("row-{timestamp}")
    };
    ChatTranscriptEntry {
        id,
        role: row.role,
        content: row.content_text,
        timestamp,
        context_files,
        model_used: row.model_used,
        tokens: row.token_count.map(|t| t as u64),
    }
}

/// Read session transcript from orchestrator RAM, or hydrate from structured conversation rows when empty.
pub(crate) async fn context_history_or_hydrate(
    state: &ServerState,
    history_key: &str,
    session_id: &str,
) -> Vec<ChatTranscriptEntry> {
    let ctx_handle = state.orchestrator.context_handle();
    let mut history: Vec<ChatTranscriptEntry> = match crate::mcp_tools::sync_poison::poison_rw_read(
        ctx_handle.read(),
        "orchestrator context",
    ) {
        Ok(guard) => guard
            .get(history_key)
            .and_then(|s: String| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
        Err(e) => {
            tracing::warn!(error = %e, "chat transcript: context read poisoned");
            Vec::new()
        }
    };

    if history.is_empty() {
        if let Some(db) = &state.db {
            match db
                .chat_load_workspace_transcript_turns(
                    state.repository.repository_id.as_str(),
                    session_id,
                    100,
                )
                .await
            {
                Ok(rows) if !rows.is_empty() => {
                    history = rows.into_iter().map(workspace_turn_to_chat_entry).collect();
                    tracing::info!(
                        target: "vox_mcp::transcript_hydrate",
                        session = %session_id,
                        count = history.len(),
                        "hydrated chat history from structured conversation store"
                    );
                }
                Ok(_) => {}
                Err(e) => tracing::debug!(
                    target: "vox_mcp::transcript_hydrate",
                    error = %e,
                    "structured transcript hydrate skipped"
                ),
            }
        }
    }

    history
}
