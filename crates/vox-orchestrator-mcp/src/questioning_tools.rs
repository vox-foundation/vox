//! Record user answers to pending Socrates clarification rows in `question_events` / related questioning tables.
//!
//! After a successful answer, this module also posts an [`AttentionEvent`](vox_orchestrator::AttentionEvent) via
//! [`ServerState::record_attention_event`](crate::server_state::ServerState::record_attention_event) so **pilot attention budgeting**
//! stays consistent. That ledger path is distinct from questioning row storage (see `docs/src/architecture/telemetry-trust-ssot.md`).

use std::path::Path;

use serde::Deserialize;

use crate::params::ToolResult;
use crate::server_state::ServerState;

const REM_NO_DB: &str =
    "Attach Codex (VoxDb) to the MCP server so clarification sessions can be stored.";

#[derive(Debug, Deserialize)]
pub struct QuestioningSubmitAnswerParams {
    pub session_id: String,
    pub answer_text: String,
    #[serde(default)]
    pub answer_type: Option<String>,
    #[serde(default)]
    pub question_id: Option<String>,
    #[serde(default)]
    pub selected_option_id: Option<String>,
    #[serde(default)]
    pub information_contribution_bits: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct QuestioningPendingParams {
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
pub struct QuestioningSyncSsotParams {
    /// Workspace-relative path to the markdown SSOT (default: repo SSOT).
    #[serde(default)]
    pub relative_path: Option<String>,
}

/// Persist `information-theoretic-questioning.md` (or override path) through publication + search dual-write.
pub async fn questioning_sync_ssot(
    state: &ServerState,
    params: QuestioningSyncSsotParams,
) -> String {
    let Some(db) = state.db.as_ref() else {
        return ToolResult::<serde_json::Value>::err_with_remediation(
            "Database not attached.".to_string(),
            REM_NO_DB,
        )
        .to_json();
    };
    let root = state
        .workspace_root
        .as_deref()
        .unwrap_or_else(|| Path::new("."));
    let rel = params
        .relative_path
        .as_deref()
        .unwrap_or("docs/src/reference/information-theoretic-questioning.md");
    let path = root.join(rel);
    let body = match vox_bounded_fs::read_utf8_path_capped(&path) {
        Ok(s) => s,
        Err(e) => {
            return ToolResult::<serde_json::Value>::err(format!(
                "Could not read {path}: {e}",
                path = path.display()
            ))
            .to_json();
        }
    };
    let artifact = vox_db::QuestioningResearchArtifact {
        publication_id: "questioning-ssot-md",
        source_ref: Some(rel),
        title: "Information-theoretic questioning (SSOT)",
        author: "vox",
        abstract_text: Some(
            "Mirrored questioning policy markdown for publication manifest + search.",
        ),
        body_markdown: body.as_str(),
        citations_json: None,
        metadata_json: None,
        state: "published",
    };
    match db
        .persist_questioning_research_artifact_dual_write(artifact)
        .await
    {
        Ok((digest, doc_id)) => ToolResult::ok(serde_json::json!({
            "relative_path": rel,
            "content_sha3_256": digest,
            "search_document_id": doc_id,
        }))
        .to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    }
}

pub async fn questioning_pending(state: &ServerState, params: QuestioningPendingParams) -> String {
    let Some(db) = state.db.as_ref() else {
        return ToolResult::<serde_json::Value>::err_with_remediation(
            "Database not attached.".to_string(),
            REM_NO_DB,
        )
        .to_json();
    };
    let repo = state.repository.repository_id.as_str();
    match db
        .pending_clarifications_json_for_repo(&params.session_id, repo)
        .await
    {
        Ok(v) => ToolResult::ok(v).to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    }
}

pub async fn questioning_submit_answer(
    state: &ServerState,
    params: QuestioningSubmitAnswerParams,
) -> String {
    let Some(db) = state.db.as_ref() else {
        return ToolResult::<serde_json::Value>::err_with_remediation(
            "Database not attached.".to_string(),
            REM_NO_DB,
        )
        .to_json();
    };
    let repo = state.repository.repository_id.as_str();
    let open = match db
        .find_open_question_session_for_repo(&params.session_id, repo)
        .await
    {
        Ok(s) => s,
        Err(e) => return ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    };
    let Some(sess) = open else {
        return ToolResult::<serde_json::Value>::err(
            "No open clarification session for this session_id.".to_string(),
        )
        .to_json();
    };
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let answer_type = params.answer_type.as_deref().unwrap_or("free_text");
    let bits = params.information_contribution_bits.unwrap_or(0.08_f64);
    let resolved_qid = match db
        .record_questioning_user_answer(
            sess.id,
            params.question_id.as_deref(),
            &params.answer_text,
            answer_type,
            params.selected_option_id.as_deref(),
            bits,
            now_ms,
        )
        .await
    {
        Ok(id) => id,
        Err(e) => return ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    };
    let _ = db
        .merge_question_session_belief_answer(
            sess.id,
            &resolved_qid,
            &params.answer_text,
            now_ms,
            params.selected_option_id.as_deref(),
        )
        .await;

    {
        use vox_orchestrator::{
            AgentId, ApprovalOutcome, ApprovalTier, AttentionEvent, AttentionEventType,
        };
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let bm = state.orchestrator.budget_manager_handle();
        let trust = vox_orchestrator::sync_lock::rw_read(&*bm)
            .trust_snapshot()
            .get(&AgentId(0))
            .map(|t| t.trust_score)
            .unwrap_or(0.3);
        let evt = AttentionEvent {
            agent_id: AgentId(0),
            task_id: None,
            event_type: AttentionEventType::ClarificationAnswered,
            tier: ApprovalTier::Confirm,
            cost_ms: 0,
            outcome: ApprovalOutcome::Approved,
            trust_score_at_time: trust,
            effective_complexity: 0.0,
            decision_entropy_bits: bits,
            timestamp_ms: ts,
            channel: Some("vox_questioning_submit_answer".to_string()),
            policy_reason: Some(format!("answered_question {resolved_qid}")),
        };
        state.record_attention_event(evt);
    }

    let pending = db
        .has_pending_clarification_for_mcp_session(&params.session_id, repo)
        .await
        .unwrap_or(false);
    let body = serde_json::json!({
        "question_session_id": sess.id,
        "answered_at_ms": now_ms,
        "still_pending_clarification": pending,
    });
    ToolResult::ok(body).to_json()
}
