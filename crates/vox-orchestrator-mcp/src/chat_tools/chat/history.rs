use super::super::now_ts;
use super::super::params::{ChatHistoryParams, ChatMessageParams, ChatTranscriptEntry};
use super::hydrate::context_history_or_hydrate;
use crate::params::ToolResult;
use crate::server_state::ServerState;
use crate::session_identity::normalize_chat_session_id;
use vox_orchestrator::compaction::{
    CompactionDirection, CompactionEngine, SignaturePreservingTurn,
};

fn persist_history_to_context(
    state: &ServerState,
    history_key: &str,
    history: &[ChatTranscriptEntry],
) -> Result<(), String> {
    let history_json = serde_json::to_string(history).map_err(|e| e.to_string())?;
    let ctx_handle = state.orchestrator.context_handle();
    let ctx = crate::sync_poison::poison_rw_write(ctx_handle.write(), "orchestrator context")
        .map_err(|e| e.to_string())?;
    ctx.set(vox_orchestrator::AgentId(0), history_key, &history_json, 0);
    Ok(())
}

/// Resolve a transcript message index by exact `id`, stripped `:user`/`:asst` suffix, or 1-based GUI numeric row ID (`T3-GUI-PIPELINE-01`).
fn find_transcript_message_index(
    history: &[ChatTranscriptEntry],
    target_id: &str,
) -> Option<usize> {
    let trimmed = target_id.trim();
    if let Some(idx) = history.iter().position(|e| e.id == trimmed) {
        return Some(idx);
    }
    let base_id = trimmed.split(':').next().unwrap_or(trimmed).trim();
    if !base_id.is_empty() {
        if let Some(idx) = history.iter().position(|e| e.id == base_id) {
            return Some(idx);
        }
        if let Ok(one_based) = base_id.parse::<usize>()
            && let Some(zero_based) = one_based.checked_sub(1)
            && zero_based < history.len()
        {
            return Some(zero_based);
        }
    }
    None
}

/// Return the full chat history for a session.
///
/// Pass `params.session_id` to retrieve the isolated transcript for a specific session.
/// Empty / omitted id normalizes to `"default"`, matching `chat_message`.
pub async fn chat_history(state: &ServerState, params: ChatHistoryParams) -> String {
    let (session_id, _) = normalize_chat_session_id(Some(params.session_id.as_str()));
    if let Some(tid) = params.trace_id.as_deref().filter(|s| !s.trim().is_empty()) {
        tracing::debug!(target: "vox_mcp::chat_history", trace_id = %tid, session = %session_id, "chat_history request");
    }
    let history_key = format!("chat_history:{session_id}");
    let history: Vec<ChatTranscriptEntry> =
        context_history_or_hydrate(state, history_key.as_str(), session_id.as_str()).await;
    ToolResult::ok(history).to_json()
}

/// Partial directional session compaction (`T3-HIST-BATTERY-02` — `up_to` / `from` with `thoughtSignature` preservation).
pub async fn chat_compact_up_to(state: &ServerState, args: serde_json::Value) -> String {
    let raw_session_id = args
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let (session_id, _) = normalize_chat_session_id(Some(raw_session_id));
    let history_key = format!("chat_history:{session_id}");
    let history: Vec<ChatTranscriptEntry> =
        context_history_or_hydrate(state, history_key.as_str(), session_id.as_str()).await;
    if history.is_empty() {
        return ToolResult::ok(serde_json::json!({
            "session_id": session_id,
            "history": history,
            "compact_message_index": 0,
            "preserved_thought_signatures": Vec::<String>::new(),
        }))
        .to_json();
    }

    let target_idx = if let Some(msg_id) = args
        .get("message_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
    {
        match find_transcript_message_index(&history, msg_id) {
            Some(idx) => idx,
            None => {
                return ToolResult::<serde_json::Value>::err(format!(
                    "message_id '{msg_id}' not found in session '{session_id}'"
                ))
                .to_json();
            }
        }
    } else if let Some(idx) = args.get("index").and_then(|v| v.as_u64()) {
        (idx as usize).min(history.len().saturating_sub(1))
    } else {
        history.len().saturating_sub(1)
    };

    let dir_str = args
        .get("direction")
        .and_then(|v| v.as_str())
        .unwrap_or("up_to");
    let direction = if dir_str.eq_ignore_ascii_case("from") {
        CompactionDirection::FromIndex(target_idx)
    } else {
        CompactionDirection::UpToIndex(target_idx)
    };

    let summary_replacement = args
        .get("summary")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            let slice = match direction {
                CompactionDirection::UpToIndex(bound) => &history[..=bound.min(history.len() - 1)],
                CompactionDirection::FromIndex(bound) => &history[bound.min(history.len() - 1)..],
            };
            let snippet: Vec<String> = slice
                .iter()
                .take(6)
                .map(|m| {
                    let preview: String = m.content.chars().take(120).collect();
                    format!("{}: {}", m.role, preview)
                })
                .collect();
            format!(
                "[Compacted {} turns ({dir_str} index {target_idx})]: {}",
                slice.len(),
                snippet.join(" | ")
            )
        });

    let sig_turns: Vec<SignaturePreservingTurn> = history
        .iter()
        .map(|e| SignaturePreservingTurn {
            message_id: e.id.clone(),
            role: e.role.clone(),
            content: e.content.clone(),
            token_estimate: CompactionEngine::estimate_tokens(&e.content),
            thought_signature: e.thought_signature.clone(),
        })
        .collect();

    let engine = CompactionEngine::new(state.orchestrator_config.compaction.clone());
    let outcome =
        engine.compact_partial_with_signatures(&sig_turns, direction, summary_replacement);
    let now = now_ts();
    let compacted_history: Vec<ChatTranscriptEntry> = outcome
        .retained_turns
        .iter()
        .map(|t| {
            if let Some(orig) = history.iter().find(|h| h.id == t.message_id) {
                let mut cloned = orig.clone();
                cloned.thought_signature = t.thought_signature.clone();
                cloned
            } else {
                ChatTranscriptEntry {
                    id: t.message_id.clone(),
                    role: t.role.clone(),
                    content: t.content.clone(),
                    timestamp: now,
                    context_files: vec![],
                    model_used: None,
                    tokens: Some(t.token_estimate as u64),
                    thought_signature: t.thought_signature.clone(),
                    edited_at: None,
                }
            }
        })
        .collect();

    if let Err(e) = persist_history_to_context(state, &history_key, &compacted_history) {
        return ToolResult::<serde_json::Value>::err(format!(
            "Failed to persist compacted history: {e}"
        ))
        .to_json();
    }

    ToolResult::ok(serde_json::json!({
        "session_id": session_id,
        "history": compacted_history,
        "compact_message_index": outcome.compact_message_index,
        "preserved_thought_signatures": outcome.preserved_thought_signatures,
    }))
    .to_json()
}

/// Edit an assistant or user message in-place without rewinding subsequent turns (`T3-REGEN-WORKTREE-01`).
pub async fn chat_edit_message_in_place(state: &ServerState, args: serde_json::Value) -> String {
    let raw_session_id = args
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let (session_id, _) = normalize_chat_session_id(Some(raw_session_id));
    let Some(message_id) = args
        .get("message_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
    else {
        return ToolResult::<serde_json::Value>::err("Missing required 'message_id'").to_json();
    };
    let Some(new_content) = args.get("new_content").and_then(|v| v.as_str()) else {
        return ToolResult::<serde_json::Value>::err("Missing required 'new_content'").to_json();
    };

    let history_key = format!("chat_history:{session_id}");
    let mut history: Vec<ChatTranscriptEntry> =
        context_history_or_hydrate(state, history_key.as_str(), session_id.as_str()).await;
    let Some(target_idx) = find_transcript_message_index(&history, message_id) else {
        return ToolResult::<serde_json::Value>::err(format!(
            "message_id '{message_id}' not found in session '{session_id}'"
        ))
        .to_json();
    };
    let entry = &mut history[target_idx];

    let edit_ts = now_ts();
    entry.content = new_content.to_string();
    entry.edited_at = Some(edit_ts);
    let updated_entry = entry.clone();

    if let Err(e) = persist_history_to_context(state, &history_key, &history) {
        return ToolResult::<serde_json::Value>::err(format!(
            "Failed to persist edited transcript: {e}"
        ))
        .to_json();
    }

    ToolResult::ok(serde_json::json!({
        "session_id": session_id,
        "edited_message": updated_entry,
        "history": history,
    }))
    .to_json()
}

/// Truncate session transcript from a target message (`from_message_id` or last turn) and optionally re-run (`T3-REGEN-WORKTREE-01`).
pub async fn chat_regenerate(state: &ServerState, args: serde_json::Value) -> String {
    let raw_session_id = args
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let (session_id, _) = normalize_chat_session_id(Some(raw_session_id));
    let history_key = format!("chat_history:{session_id}");
    let mut history: Vec<ChatTranscriptEntry> =
        context_history_or_hydrate(state, history_key.as_str(), session_id.as_str()).await;
    if history.is_empty() {
        return ToolResult::<serde_json::Value>::err(format!(
            "Cannot regenerate empty session '{session_id}'"
        ))
        .to_json();
    }

    let target_idx = if let Some(msg_id) = args
        .get("from_message_id")
        .or_else(|| args.get("message_id"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
    {
        match find_transcript_message_index(&history, msg_id) {
            Some(idx) => idx,
            None => {
                return ToolResult::<serde_json::Value>::err(format!(
                    "from_message_id '{msg_id}' not found in session '{session_id}'"
                ))
                .to_json();
            }
        }
    } else {
        // Default: rewind to the last user message so its assistant reply is regenerated.
        history
            .iter()
            .rposition(|e| e.role == "user")
            .unwrap_or(history.len().saturating_sub(1))
    };

    // Determine the user prompt to re-run and truncate from the user turn onwards if re-running,
    // or truncate after the user turn if `execute == false`.
    let execute = args
        .get("execute")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let user_turn_idx = if history[target_idx].role == "assistant" && target_idx > 0 {
        history[..target_idx]
            .iter()
            .rposition(|e| e.role == "user")
            .unwrap_or(target_idx)
    } else {
        target_idx
    };
    let replay_prompt = args
        .get("prompt_override")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| history[user_turn_idx].content.clone());
    let removed_count = history.len().saturating_sub(user_turn_idx);
    history.truncate(user_turn_idx);

    let restored_checkpoint_turn = if let Some(ref root) = state.workspace_root {
        let target_turn_number = (user_turn_idx / 2).max(1) as u32;
        if let Ok(ckpts) = vox_orchestrator::agentos::checkpoint_engine::GitPlumbingCheckpointStore::list_checkpoints(root, &session_id) {
            let max_known = ckpts.iter().map(|c| c.turn_count).max().unwrap_or(0);
            let chosen = ckpts
                .iter()
                .filter(|c| c.turn_count <= target_turn_number)
                .max_by_key(|c| c.turn_count)
                .or_else(|| ckpts.first());
            if let Some(ckpt) = chosen {
                if vox_orchestrator::agentos::checkpoint_engine::GitPlumbingCheckpointStore::restore_checkpoint(
                    root,
                    &session_id,
                    ckpt.turn_count,
                    max_known,
                )
                .is_ok()
                {
                    Some(ckpt.turn_count)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    if let Err(e) = persist_history_to_context(state, &history_key, &history) {
        return ToolResult::<serde_json::Value>::err(format!(
            "Failed to persist rewound transcript: {e}"
        ))
        .to_json();
    }

    if execute {
        let params = ChatMessageParams {
            prompt: replay_prompt,
            context_files: vec![],
            open_files: vec![],
            active_file: None,
            active_line: None,
            selected_text: None,
            diagnostics: vec![],
            session_id: Some(session_id),
            thread_id: None,
            journey_id: None,
            cognitive_profile: None,
            json_mode: false,
            trace_id: None,
            turn_id: None,
            correlation_id: None,
            attachment_manifest: None,
            temperature: None,
            top_p: None,
            skill: None,
            model_override: args
                .get("model_override")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            tier: None,
            clutch: None,
            risk: None,
            skill_exclusions: vec![],
            mode: None,
            priority: None,
            dry_run: None,
            force_research: None,
            research_scope: None,
            domain_mode: None,
            site_scope: None,
            oversight_level: None,
            kb_scope: None,
            grounding_check_enabled: None,
        };
        return super::message::chat_message(state, params).await;
    }

    ToolResult::ok(serde_json::json!({
        "session_id": session_id,
        "replay_prompt": replay_prompt,
        "removed_turns": removed_count,
        "restored_checkpoint_turn": restored_checkpoint_turn,
        "history": history,
    }))
    .to_json()
}

/// Resumable turn stream catch-up (`T3-LOOP-CHECKPOINT-03` & `T3-STREAM-QUEUE-01`): returns accumulated stream text
/// starting from `char_offset` (or `0`), sequenced chunks strictly after `after_seq`, `last_seq`, and `completed: bool`.
pub async fn chat_stream_resume(state: &ServerState, args: serde_json::Value) -> String {
    let raw_session_id = args
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let (session_id, _) = normalize_chat_session_id(Some(raw_session_id));
    let char_offset = args
        .get("char_offset")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let after_seq = args.get("after_seq").and_then(|v| v.as_u64());

    let reg = state.resumable_stream_registry.lock();
    let (reg_text, reg_reasoning, last_seq) = reg.accumulated_snapshot(&session_id);
    let reg_generating = reg.is_generating(&session_id);
    let reg_cancelled = reg.is_cancelled(&session_id);
    let replayed_chunks = reg.replay_after(&session_id, after_seq.unwrap_or(0));
    drop(reg);

    let (legacy_text, legacy_completed) = state
        .resumable_streams
        .lock()
        .get(&session_id)
        .cloned()
        .unwrap_or_else(|| (String::new(), true));

    let full_text = if !reg_text.is_empty() || last_seq > 0 {
        if !reg_text.is_empty() {
            reg_text
        } else {
            legacy_text
        }
    } else {
        legacy_text
    };
    let completed = if last_seq > 0 {
        !reg_generating && legacy_completed
    } else {
        legacy_completed
    };

    let chars: Vec<char> = full_text.chars().collect();
    let total_chars = chars.len();
    let clamped = char_offset.min(total_chars);
    let catch_up_delta: String = chars[clamped..].iter().collect();

    ToolResult::ok(serde_json::json!({
        "session_id": session_id,
        "catch_up_delta": catch_up_delta,
        "delta_text": catch_up_delta,
        "reasoning": reg_reasoning,
        "char_offset": clamped,
        "next_char_offset": total_chars,
        "next_offset": total_chars,
        "last_seq": last_seq,
        "chunks": replayed_chunks,
        "cancelled": reg_cancelled,
        "completed": completed,
    }))
    .to_json()
}

/// Cancel an in-flight resumable turn stream for `session_id` (`T3-STREAM-QUEUE-01`).
pub async fn chat_stream_cancel(state: &ServerState, args: serde_json::Value) -> String {
    let raw_session_id = args
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let (session_id, _) = normalize_chat_session_id(Some(raw_session_id));
    let cancelled = state
        .resumable_stream_registry
        .lock()
        .cancel_turn(&session_id);
    if let Some((_, completed)) = state.resumable_streams.lock().get_mut(&session_id) {
        *completed = true;
    }
    ToolResult::ok(serde_json::json!({
        "session_id": session_id,
        "cancelled": cancelled,
    }))
    .to_json()
}

/// Thread run queue & mid-turn steering (`T3-LOOP-CHECKPOINT-04`):
/// Enqueues a prompt in `state.thread_run_queue` and, when `promote_to_steer` is `true` (default),
/// immediately promotes it into `state.steered_prompts` so the active `agent_loop` turn injects
/// `[MID-TURN STEER INTERVENTION]: ...` on its next iteration.
pub async fn chat_queue_steer(state: &ServerState, args: serde_json::Value) -> String {
    let raw_session_id = args
        .get("session_id")
        .or_else(|| args.get("thread_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let (session_id, _) = normalize_chat_session_id(Some(raw_session_id));
    let prompt = args
        .get("prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let promote = args
        .get("promote_to_steer")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let entry_id = args
        .get("entry_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            use vox_orchestrator::id_allocator::VoxIdAllocatorV2;
            state
                .id_allocator
                .derive("q", &[session_id.as_str(), &now_ts().to_string()])
        });

    let mut queue = state.thread_run_queue.lock();
    if !prompt.is_empty() {
        queue.enqueue_prompt(session_id.clone(), entry_id.clone(), prompt.clone());
    }
    let steered_intervention = if promote {
        queue.promote_to_steer(&session_id, &entry_id)
    } else {
        None
    };
    drop(queue);

    if let Some(ref intervention) = steered_intervention {
        state
            .steered_prompts
            .lock()
            .entry(session_id.clone())
            .or_default()
            .push(intervention.clone());
    }

    ToolResult::ok(serde_json::json!({
        "session_id": session_id,
        "entry_id": entry_id,
        "promoted_to_steer": steered_intervention.is_some(),
        "steered_intervention": steered_intervention,
    }))
    .to_json()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn chat_compact_up_to_and_edit_and_regenerate_work_end_to_end() {
        let state = ServerState::new_test().await;
        let session_id = "hist-ops-session";
        let history_key = format!("chat_history:{session_id}");
        let initial = vec![
            ChatTranscriptEntry {
                id: "usr-1".into(),
                role: "user".into(),
                content: "First question".into(),
                timestamp: 10,
                context_files: vec![],
                model_used: None,
                tokens: None,
                thought_signature: None,
                edited_at: None,
            },
            ChatTranscriptEntry {
                id: "asst-1".into(),
                role: "assistant".into(),
                content: "First answer".into(),
                timestamp: 11,
                context_files: vec![],
                model_used: Some("gemini-3-flash".into()),
                tokens: Some(12),
                thought_signature: Some("sig-gemini-001".into()),
                edited_at: None,
            },
            ChatTranscriptEntry {
                id: "usr-2".into(),
                role: "user".into(),
                content: "Second question".into(),
                timestamp: 12,
                context_files: vec![],
                model_used: None,
                tokens: None,
                thought_signature: None,
                edited_at: None,
            },
            ChatTranscriptEntry {
                id: "asst-2".into(),
                role: "assistant".into(),
                content: "Second answer".into(),
                timestamp: 13,
                context_files: vec![],
                model_used: Some("gemini-3-flash".into()),
                tokens: Some(15),
                thought_signature: None,
                edited_at: None,
            },
        ];
        persist_history_to_context(&state, &history_key, &initial).expect("persist initial");

        // 1. Test in-place edit (`T3-REGEN-WORKTREE-01`)
        let edit_raw = chat_edit_message_in_place(
            &state,
            serde_json::json!({
                "session_id": session_id,
                "message_id": "asst-2",
                "new_content": "Second answer (refined in-place)"
            }),
        )
        .await;
        let edit_v: serde_json::Value = serde_json::from_str(&edit_raw).expect("json");
        assert_eq!(edit_v["success"], true);
        assert_eq!(
            edit_v["data"]["edited_message"]["content"],
            "Second answer (refined in-place)"
        );
        assert!(edit_v["data"]["edited_message"]["edited_at"].is_u64());

        // 2. Test partial compaction `up_to` message `asst-1` (`T3-HIST-BATTERY-02`)
        let compact_raw = chat_compact_up_to(
            &state,
            serde_json::json!({
                "session_id": session_id,
                "message_id": "asst-1",
                "direction": "up_to",
                "summary": "Summary of turn 1"
            }),
        )
        .await;
        let compact_v: serde_json::Value = serde_json::from_str(&compact_raw).expect("json");
        assert_eq!(compact_v["success"], true);
        assert_eq!(
            compact_v["data"]["preserved_thought_signatures"],
            serde_json::json!(["sig-gemini-001"])
        );
        let retained = compact_v["data"]["history"]
            .as_array()
            .expect("history array");
        assert_eq!(retained.len(), 3); // summary + usr-2 + asst-2
        assert_eq!(retained[0]["content"], "Summary of turn 1");

        // 3. Test regenerate truncation from `asst-2` (`T3-REGEN-WORKTREE-01`)
        let regen_raw = chat_regenerate(
            &state,
            serde_json::json!({
                "session_id": session_id,
                "from_message_id": "asst-2",
                "execute": false
            }),
        )
        .await;
        let regen_v: serde_json::Value = serde_json::from_str(&regen_raw).expect("json");
        assert_eq!(regen_v["success"], true);
        assert_eq!(regen_v["data"]["replay_prompt"], "Second question");
        assert_eq!(regen_v["data"]["removed_turns"], 2);
        assert_eq!(regen_v["data"]["history"].as_array().expect("arr").len(), 1);

        // 4. Test 1-based GUI numeric row IDs ("1" and "1:user") (`T3-GUI-PIPELINE-01`)
        let edit_num_raw = chat_edit_message_in_place(
            &state,
            serde_json::json!({
                "session_id": session_id,
                "message_id": "1",
                "new_content": "Summary of turn 1 (numeric id edit)"
            }),
        )
        .await;
        let edit_num_v: serde_json::Value = serde_json::from_str(&edit_num_raw).expect("json");
        assert_eq!(edit_num_v["success"], true);
        assert_eq!(
            edit_num_v["data"]["edited_message"]["content"],
            "Summary of turn 1 (numeric id edit)"
        );
    }
}
