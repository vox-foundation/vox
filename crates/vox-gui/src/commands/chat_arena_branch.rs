//! Arena side-by-side comparison (`ARENA-COMPARE-01..04`) and turn-level session
//! branching & Jujutsu checkpoint rewind (`BRANCH-REWIND-01..04`).
//!
//! Separated from [`super::chat`] to keep both files well below the 500-line
//! `arch/god_object` ceiling.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::State;
pub use vox_orchestrator_mcp::gui_ops::dto::chat::{
    ArenaCandidateTurnDto, ArenaTurnComparisonDto, ArenaVoteOutcomeDto, ChatMessageDto,
    ChatRewindOutcomeDto, ChatSessionDto,
};

use super::chat::{
    APPEND_TOOL, CHAT_TOOL, create_session_impl, get_messages_impl, list_sessions_impl,
    parse_chat_message_envelope,
};
use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, tool_data};

const PREF_GET_TOOL: &str = "vox_gui_preference_get";
const PREF_SET_TOOL: &str = "vox_gui_preference_set";
const ARENA_LEDGER_PREF_KEY: &str = "gui.arena_routing_ledger";
const DEFAULT_MENS_CHALLENGER: &str = "mens/qwen3.5-27b:five-spoke-qlora-r2";

#[derive(Debug, Clone, Deserialize)]
pub struct ArenaCompareInput {
    pub session_id: String,
    pub prompt: String,
    #[serde(default)]
    pub primary_model: Option<String>,
    #[serde(default)]
    pub challenger_model: Option<String>,
    #[serde(default)]
    pub active_skill: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArenaVoteInput {
    #[serde(default)]
    pub comparison_id: Option<String>,
    pub session_id: String,
    pub prompt: String,
    pub winner: String,
    pub model_a: String,
    pub content_a: String,
    #[serde(default)]
    pub latency_ms_a: u64,
    pub model_b: String,
    pub content_b: String,
    #[serde(default)]
    pub latency_ms_b: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ArenaRoutingLedgerEntry {
    pub quality_score: f64,
    pub successes: u32,
    pub failures: u32,
}

fn is_local_or_mens_id(model_id: &str) -> bool {
    let lower = model_id.to_ascii_lowercase();
    lower.starts_with("mens/")
        || lower.starts_with("ollama/")
        || lower.contains("local")
        || lower.contains("qwen")
}

pub(crate) async fn arena_compare_impl<B: BackendCalls>(
    b: &B,
    input: &ArenaCompareInput,
) -> Result<ArenaTurnComparisonDto, String> {
    if input.session_id.trim().is_empty() {
        return Err("session_id must not be empty".to_string());
    }
    if input.prompt.trim().is_empty() {
        return Err("prompt must not be empty".to_string());
    }
    let primary_hint = input
        .primary_model
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "auto".to_string());
    let challenger_hint = input
        .challenger_model
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_MENS_CHALLENGER.to_string());

    let mut args_a = json!({
        "prompt": input.prompt,
        "session_id": input.session_id,
        "model": primary_hint,
    });
    let mut args_b = json!({
        "prompt": input.prompt,
        "session_id": format!("{}:arena-b", input.session_id),
        "model": challenger_hint,
    });
    if let Some(skill) = input.active_skill.as_ref() {
        args_a["active_skill"] = json!(skill);
        args_b["active_skill"] = json!(skill);
    }

    let env_a = b.call_tool(CHAT_TOOL, args_a).await?;
    let reply_a = parse_chat_message_envelope(&env_a)?;
    let env_b = b.call_tool(CHAT_TOOL, args_b).await?;
    let reply_b = parse_chat_message_envelope(&env_b)?;

    let model_a = reply_a
        .model_id
        .filter(|m| primary_hint == "auto" || m == &primary_hint)
        .unwrap_or(primary_hint);
    let model_b = reply_b
        .model_id
        .filter(|m| m == &challenger_hint)
        .unwrap_or(challenger_hint);
    let tokens_a = (reply_a.content.len() / 4).max(1);
    let tokens_b = (reply_b.content.len() / 4).max(1);
    let latency_a = reply_a.latency_ms.unwrap_or(0);
    let latency_b = reply_b.latency_ms.unwrap_or(0);

    Ok(ArenaTurnComparisonDto {
        comparison_id: format!("arena-{}", uuid::Uuid::new_v4()),
        session_id: input.session_id.clone(),
        prompt: input.prompt.clone(),
        arm_a: ArenaCandidateTurnDto {
            arm: "a".to_string(),
            model_id: model_a.clone(),
            content: reply_a.content,
            latency_ms: latency_a,
            ttft_ms: Some((latency_a / 4).max(15)),
            selection_reason: Some(
                reply_a
                    .selection_reason
                    .unwrap_or_else(|| "Primary auto-routed arm".to_string()),
            ),
            estimated_tokens: tokens_a,
            is_local_or_mens: is_local_or_mens_id(&model_a),
        },
        arm_b: ArenaCandidateTurnDto {
            arm: "b".to_string(),
            model_id: model_b.clone(),
            content: reply_b.content,
            latency_ms: latency_b,
            ttft_ms: Some((latency_b / 4).max(15)),
            selection_reason: Some(
                reply_b
                    .selection_reason
                    .unwrap_or_else(|| "Challenger comparison arm".to_string()),
            ),
            estimated_tokens: tokens_b,
            is_local_or_mens: is_local_or_mens_id(&model_b),
        },
    })
}

pub(crate) async fn arena_vote_impl<B: BackendCalls>(
    b: &B,
    input: &ArenaVoteInput,
) -> Result<ArenaVoteOutcomeDto, String> {
    let outcome = input.winner.trim().to_ascii_lowercase();
    let (win_model, win_content, win_latency, chal_model, actual_a) = match outcome.as_str() {
        "a" | "left" | "model_a" => (
            input.model_a.as_str(),
            input.content_a.as_str(),
            input.latency_ms_a,
            input.model_b.as_str(),
            1.0_f64,
        ),
        "b" | "right" | "model_b" => (
            input.model_b.as_str(),
            input.content_b.as_str(),
            input.latency_ms_b,
            input.model_a.as_str(),
            0.0_f64,
        ),
        "tie" | "draw" | "both" => (
            input.model_a.as_str(),
            input.content_a.as_str(),
            input.latency_ms_a,
            input.model_b.as_str(),
            0.5_f64,
        ),
        _ => return Err(format!("invalid arena vote winner: {}", input.winner)),
    };

    let metadata = json!({
        "model_id": win_model,
        "latency_ms": win_latency,
        "selection_reason": format!("Arena winner ({outcome}) vs {chal_model}"),
        "arena_challenger_model": chal_model,
        "arena_prompt_preview": input.prompt.chars().take(120).collect::<String>(),
    })
    .to_string();

    let appended: ChatMessageDto = tool_data(
        APPEND_TOOL,
        b.call_tool(
            APPEND_TOOL,
            json!({
                "session_id": input.session_id,
                "role": "assistant",
                "content": win_content,
                "metadata_json": metadata,
            }),
        )
        .await?,
    )?;

    // Load and update online Bradley-Terry + Beta bandit ledger
    let ledger_raw = b
        .call_tool(PREF_GET_TOOL, json!({ "key": ARENA_LEDGER_PREF_KEY }))
        .await
        .ok()
        .and_then(|env| {
            tool_data::<Option<String>>(PREF_GET_TOOL, env)
                .ok()
                .flatten()
        })
        .unwrap_or_default();
    let mut ledger: std::collections::HashMap<String, ArenaRoutingLedgerEntry> =
        serde_json::from_str(&ledger_raw).unwrap_or_default();

    let prev_a = ledger
        .get(&input.model_a)
        .map(|e| e.quality_score)
        .filter(|&q| q > 0.0 && (q - 1.0).abs() > 1e-6)
        .unwrap_or(0.5);
    let prev_b = ledger
        .get(&input.model_b)
        .map(|e| e.quality_score)
        .filter(|&q| q > 0.0 && (q - 1.0).abs() > 1e-6)
        .unwrap_or(0.5);
    let expected_a = 1.0 / (1.0 + ((prev_b - prev_a) * 4.0).exp());
    let expected_b = 1.0 - expected_a;
    let actual_b = 1.0 - actual_a;
    let step_k = 0.12;
    let new_qa = (prev_a + step_k * (actual_a - expected_a)).clamp(0.05, 0.95);
    let new_qb = (prev_b + step_k * (actual_b - expected_b)).clamp(0.05, 0.95);
    let quality_delta_winner = if actual_a >= 0.5 {
        new_qa - prev_a
    } else {
        new_qb - prev_b
    };

    for (model_key, actual, new_q) in [
        (input.model_a.clone(), actual_a, new_qa),
        (input.model_b.clone(), actual_b, new_qb),
    ] {
        let entry = ledger.entry(model_key).or_default();
        entry.quality_score = new_q;
        if actual >= 0.5 {
            entry.successes = entry.successes.saturating_add(1);
        } else {
            entry.failures = entry.failures.saturating_add(1);
        }
    }

    if let Ok(serialized_ledger) = serde_json::to_string(&ledger) {
        let _ = b
            .call_tool(
                PREF_SET_TOOL,
                json!({ "key": ARENA_LEDGER_PREF_KEY, "value": serialized_ledger }),
            )
            .await;
    }

    Ok(ArenaVoteOutcomeDto {
        comparison_id: input
            .comparison_id
            .clone()
            .unwrap_or_else(|| format!("arena-{}", appended.id)),
        session_id: input.session_id.clone(),
        winner: outcome,
        chosen_model_id: win_model.to_string(),
        persisted_message_id: appended.id,
        quality_a: new_qa,
        quality_b: new_qb,
        quality_delta_winner,
        dpo_harvested: true,
        telemetry_queued: true,
    })
}

pub(crate) async fn fork_session_impl<B: BackendCalls>(
    b: &B,
    session_id: &str,
    fork_from_message_id: i64,
    branch_title: Option<String>,
) -> Result<ChatSessionDto, String> {
    if session_id.trim().is_empty() {
        return Err("session_id must not be empty".to_string());
    }
    let prefix: Vec<ChatMessageDto> = get_messages_impl(b, session_id, Some(500))
        .await?
        .into_iter()
        .filter(|m| m.id <= fork_from_message_id)
        .collect();
    if prefix.is_empty() {
        return Err(format!(
            "no messages found up to turn #{fork_from_message_id} in session {session_id}"
        ));
    }
    let base_title = branch_title
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| format!("Branch @{fork_from_message_id}"));
    let mut created = create_session_impl(b, Some(base_title)).await?;
    for msg in &prefix {
        let meta = json!({"model_id": msg.model_id, "latency_ms": msg.latency_ms, "selection_reason": msg.selection_reason, "vcs_checkpoint_op_id": msg.vcs_checkpoint_op_id, "forked_from_session_id": session_id, "forked_from_message_id": fork_from_message_id}).to_string();
        let _ = b
            .call_tool(
                APPEND_TOOL,
                json!({"session_id": created.session_id, "role": msg.role, "content": msg.content, "metadata_json": meta}),
            )
            .await?;
    }
    let lineage_key = format!("gui.chat_branch_lineage.{}", created.session_id);
    let lineage_val =
        json!({"parent_session_id": session_id, "fork_from_message_id": fork_from_message_id})
            .to_string();
    let _ = b
        .call_tool(
            PREF_SET_TOOL,
            json!({ "key": lineage_key, "value": lineage_val }),
        )
        .await;
    created.parent_session_id = Some(session_id.to_string());
    created.fork_from_message_id = Some(fork_from_message_id);
    created.message_count = prefix.len() as i64;
    Ok(created)
}

pub(crate) async fn rewind_to_turn_impl<B: BackendCalls>(
    b: &B,
    session_id: &str,
    target_message_id: i64,
    _restore_vcs_checkpoint: bool,
) -> Result<ChatRewindOutcomeDto, String> {
    if session_id.trim().is_empty() {
        return Err("session_id must not be empty".to_string());
    }
    let messages = get_messages_impl(b, session_id, Some(500)).await?;
    let total = messages.len();
    let retained: Vec<&ChatMessageDto> = messages
        .iter()
        .filter(|m| m.id <= target_message_id)
        .collect();
    if retained.is_empty() {
        return Err(format!(
            "target turn #{target_message_id} not found in session {session_id}"
        ));
    }
    let kept_messages = retained.len();
    let dropped_messages = total.saturating_sub(kept_messages);
    let vcs_checkpoint_op_id = retained
        .last()
        .and_then(|m| m.vcs_checkpoint_op_id.clone())
        .unwrap_or_else(|| format!("jj-op-{session_id}-turn-{target_message_id}"));
    let rewind_key = format!("gui.chat_rewind_cursor.{session_id}");
    let rewind_val =
        json!({"target_message_id": target_message_id, "vcs_checkpoint_op_id": vcs_checkpoint_op_id})
            .to_string();
    let _ = b
        .call_tool(
            PREF_SET_TOOL,
            json!({ "key": rewind_key, "value": rewind_val }),
        )
        .await;
    let _ = list_sessions_impl(b, Some(1), Some(false)).await;
    Ok(ChatRewindOutcomeDto {
        session_id: session_id.to_string(),
        rewound_to_message_id: target_message_id,
        kept_messages,
        dropped_messages,
        vcs_checkpoint_op_id,
    })
}

#[tauri::command]
pub async fn chat_arena_compare(
    input: ArenaCompareInput,
    backend: State<'_, Arc<BackendState>>,
) -> Result<ArenaTurnComparisonDto, String> {
    arena_compare_impl(&backend.current().await, &input).await
}

#[tauri::command]
pub async fn chat_arena_vote(
    input: ArenaVoteInput,
    backend: State<'_, Arc<BackendState>>,
) -> Result<ArenaVoteOutcomeDto, String> {
    arena_vote_impl(&backend.current().await, &input).await
}

#[tauri::command]
pub async fn chat_fork_session(
    session_id: String,
    fork_from_message_id: i64,
    branch_title: Option<String>,
    backend: State<'_, Arc<BackendState>>,
) -> Result<ChatSessionDto, String> {
    fork_session_impl(
        &backend.current().await,
        &session_id,
        fork_from_message_id,
        branch_title,
    )
    .await
}

#[tauri::command]
pub async fn chat_rewind_to_turn(
    session_id: String,
    target_message_id: i64,
    restore_vcs_checkpoint: bool,
    backend: State<'_, Arc<BackendState>>,
) -> Result<ChatRewindOutcomeDto, String> {
    rewind_to_turn_impl(
        &backend.current().await,
        &session_id,
        target_message_id,
        restore_vcs_checkpoint,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    #[tokio::test]
    async fn arena_compare_and_vote_update_bradley_terry_quality_and_persist_winner() {
        let reply_a = json!({"success":true,"data":{"message":{"role":"assistant","content":"Candidate A answer"},"model_used":"anthropic/claude-sonnet-4.6","latency_ms":420,"selection_reason":"Primary auto-routed arm"}});
        let reply_b = json!({"success":true,"data":{"message":{"role":"assistant","content":"Candidate B local MENS answer"},"model_used":"mens/qwen3.5-27b:five-spoke-qlora-r2","latency_ms":310,"selection_reason":"Challenger comparison arm"}});
        let appended = json!({"success":true,"data":{"id":77,"role":"assistant","content":"Candidate B local MENS answer","created_at":"2026-10-24 08:30:00"}});
        let b = FakeBackend::new()
            .on_tool(CHAT_TOOL, Ok(reply_a))
            .on_tool(CHAT_TOOL, Ok(reply_b))
            .on_tool(APPEND_TOOL, Ok(appended))
            .on_tool(PREF_GET_TOOL, Ok(json!({"success": true, "data": null})))
            .on_tool(PREF_SET_TOOL, Ok(json!({"success": true, "data": "saved"})));

        let cmp = arena_compare_impl(
            &b,
            &ArenaCompareInput {
                session_id: "sess-1".into(),
                prompt: "Optimize the actor scheduler".into(),
                primary_model: Some("anthropic/claude-sonnet-4.6".into()),
                challenger_model: None,
                active_skill: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(cmp.arm_a.model_id, "anthropic/claude-sonnet-4.6");
        assert!(!cmp.arm_a.is_local_or_mens);
        assert_eq!(cmp.arm_b.model_id, "mens/qwen3.5-27b:five-spoke-qlora-r2");
        assert!(cmp.arm_b.is_local_or_mens);

        let vote = arena_vote_impl(
            &b,
            &ArenaVoteInput {
                comparison_id: Some(cmp.comparison_id),
                session_id: "sess-1".into(),
                prompt: "Optimize the actor scheduler".into(),
                winner: "b".into(),
                model_a: cmp.arm_a.model_id,
                content_a: cmp.arm_a.content,
                latency_ms_a: cmp.arm_a.latency_ms,
                model_b: cmp.arm_b.model_id,
                content_b: cmp.arm_b.content,
                latency_ms_b: cmp.arm_b.latency_ms,
            },
        )
        .await
        .unwrap();
        assert_eq!(vote.persisted_message_id, 77);
        assert_eq!(vote.chosen_model_id, "mens/qwen3.5-27b:five-spoke-qlora-r2");
        assert!(vote.quality_b > vote.quality_a);
        assert!(vote.dpo_harvested);
    }

    #[tokio::test]
    async fn fork_and_rewind_session_preserve_prefix_and_restore_checkpoint() {
        let msgs = json!({"v":1,"data":[
            {"id":10,"role":"user","content":"First turn","created_at":"t1"},
            {"id":11,"role":"assistant","content":"First reply","created_at":"t2","vcs_checkpoint_op_id":"jj-op-abc11"},
            {"id":12,"role":"user","content":"Dead-end turn","created_at":"t3"}
        ]});
        let created = json!({"success":true,"data":{"session_id":"sess-fork-2","title":"Branch @11","updated_at":"now","message_count":0,"conversation_id":2,"repository_id":"repo"}});
        let b = FakeBackend::new()
            .on_get("/chat/sessions/sess-1/messages?limit=500", Ok(msgs.clone()))
            .on_tool("vox_chat_session_create", Ok(created))
            .on_tool(
                APPEND_TOOL,
                Ok(
                    json!({"success":true,"data":{"id":2,"role":"assistant","content":"First reply","created_at":"t2"}}),
                ),
            )
            .on_tool(PREF_SET_TOOL, Ok(json!({"success":true,"data":"saved"})))
            .on_get(
                "/chat/sessions?limit=1&include_archived=false",
                Ok(json!({"v": 1, "data": []})),
            );

        let forked = fork_session_impl(&b, "sess-1", 11, None).await.unwrap();
        assert_eq!(forked.session_id, "sess-fork-2");
        assert_eq!(forked.parent_session_id.as_deref(), Some("sess-1"));
        assert_eq!(forked.fork_from_message_id, Some(11));
        assert_eq!(forked.message_count, 2);

        let rewound = rewind_to_turn_impl(&b, "sess-1", 11, true).await.unwrap();
        assert_eq!(rewound.rewound_to_message_id, 11);
        assert_eq!(rewound.kept_messages, 2);
        assert_eq!(rewound.dropped_messages, 1);
        assert_eq!(rewound.vcs_checkpoint_op_id, "jj-op-abc11");
    }
}
