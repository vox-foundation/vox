use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) async fn persist_and_build_chat_envelope(
    state: &ServerState,
    params: &ChatMessageParams,
    session_id: String,
    journey_id: String,
    thread_id_for_envelope: Option<String>,
    user_prompt: String,
    response_text: String,
    model_used: String,
    tokens: u64,
    selection_reason: Option<String>,
    mut events: Vec<Value>,
    chat_pin: Option<String>,
    turn_prompt_tokens: u64,
    turn_completion_tokens: u64,
    turn_cache_read_tokens: u64,
    turn_cost_usd: Option<f64>,
    turn_ttft_ms: Option<u64>,
    turn_tpot_ms: Option<f64>,
    turn_classified_spoke: Option<String>,
    turn_tool_ledger: Vec<String>,
    all_context_files: Vec<String>,
    retrieval_evidence: Option<crate::memory::RetrievalEvidenceEnvelope>,
    preamble_skipped_reason: Option<&'static str>,
    compaction_audit: super::super::conversation::ConversationCompactionAudit,
    mention_count: usize,
    est_input_tokens: u32,
    llm_started: std::time::Instant,
    intent: &super::super::research_intent::ResearchIntent,
    mut research_trace: super::super::research_turn::ResearchTrace,
) -> Value {
    // 4. Persist to session-scoped history.
    //
    // The history key is derived from `params.session_id` (defaulting to `"default"`).
    // Each distinct value yields an independent key, preventing context bleeding
    // across concurrent VS Code windows, agent threads, or other logical sessions.
    let history_key = format!("chat_history:{session_id}");
    let context_key = session_context_envelope_key(session_id.as_str());

    let turn_ts = now_ts();
    let (user_msg_id, asst_msg_id) = {
        use vox_orchestrator::id_allocator::VoxIdAllocatorV2;
        let ts_seg = turn_ts.to_string();
        (
            state
                .id_allocator
                .derive("usr", &[session_id.as_str(), journey_id.as_str(), &ts_seg]),
            state
                .id_allocator
                .derive("asst", &[session_id.as_str(), journey_id.as_str(), &ts_seg]),
        )
    };
    let user_msg = ChatTranscriptEntry {
        id: user_msg_id,
        role: "user".to_string(),
        content: params.prompt.clone(),
        timestamp: turn_ts,
        context_files: all_context_files,
        model_used: None,
        tokens: None,
        thought_signature: None,
        edited_at: None,
    };
    let parsed_special =
        vox_orchestrator::context_sanitizer::parse_special_entity_tags(&response_text);
    let extracted_thought_sig = (!parsed_special.extracted_reasoning.is_empty())
        .then(|| parsed_special.extracted_reasoning.join("\n\n"));
    let asst_msg = ChatTranscriptEntry {
        id: asst_msg_id,
        role: "assistant".to_string(),
        content: response_text.clone(),
        timestamp: turn_ts + 1,
        context_files: vec![],
        model_used: Some(model_used.clone()),
        tokens: Some(tokens),
        thought_signature: extracted_thought_sig,
        edited_at: None,
    };
    // Phase 150 (`SOTA-INTRATURN-03`): Persist a compact `[TURN TOOL LEDGER: ...]`
    // trailer in session history when tools ran during this turn so subsequent turns
    // know what tools/files were already inspected without re-sending raw tool bodies,
    // while keeping the returned `result["message"]` clean for the UI.
    let mut persisted_asst_msg = asst_msg.clone();
    if !turn_tool_ledger.is_empty() {
        persisted_asst_msg.content = format!(
            "{}\n\n[TURN TOOL LEDGER: {}]",
            persisted_asst_msg.content,
            turn_tool_ledger.join(", ")
        );
    }

    let mut history =
        context_history_or_hydrate(state, history_key.as_str(), session_id.as_str()).await;
    history.push(user_msg.clone());
    history.push(persisted_asst_msg);
    // Bound the *persisted/display* transcript independently of the token-aware
    // budget applied to what gets sent to the model (see `conversation.rs` /
    // Task 1.1): this cap exists only to bound storage/GUI-transcript size.
    trim_persisted_history(&mut history);

    match serde_json::to_string(&history) {
        Ok(history_json) => {
            let ctx_handle = state.orchestrator.context_handle();
            match crate::sync_poison::poison_rw_write(ctx_handle.write(), "orchestrator context") {
                Ok(ctx) => {
                    ctx.set(vox_orchestrator::AgentId(0), &history_key, &history_json, 0);
                    if let Some(ev) = &retrieval_evidence {
                        let context_envelope = ev.to_context_envelope(
                            state.repository.repository_id.as_str(),
                            Some(session_id.as_str()),
                        );
                        if let Ok(context_json) = serde_json::to_string(&context_envelope) {
                            ctx.set(
                                vox_orchestrator::AgentId(0),
                                &context_key,
                                &context_json,
                                3600,
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "chat_message: context poisoned persisting history");
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                session_id,
                "chat_message: failed to serialize chat history — \
                 history will not persist for this turn"
            );
        }
    }

    let privacy_posture = if session_id.starts_with("ephemeral")
        || session_id.starts_with("temp-")
        || session_id.contains(":ephemeral")
        || params.mode.as_deref() == Some("ephemeral")
        || params.mode.as_deref() == Some("incognito")
    {
        vox_orchestrator::models::spec::SessionPrivacyPosture::Ephemeral
    } else if params.tier.as_deref() == Some("local")
        || params.mode.as_deref() == Some("local_private")
    {
        vox_orchestrator::models::spec::SessionPrivacyPosture::LocalPrivate
    } else {
        vox_orchestrator::models::spec::SessionPrivacyPosture::Standard
    };
    let persisted_to_disk = privacy_posture.should_persist_to_db() && state.db.is_some();

    if privacy_posture.should_persist_to_db()
        && let Some(db) = &state.db
    {
        let repo_id = &state.repository.repository_id;
        let q_session = session_id.clone();
        let q_repo = repo_id.to_string();

        let route_reason = serde_json::json!({
            "cognitive_profile": params.cognitive_profile,
        });
        let route_reason_s = route_reason.to_string();
        let _ = db
            .record_routing_decision(
                Some(journey_id.as_str()),
                q_repo.as_str(),
                Some(q_session.as_str()),
                "vox_chat_message",
                Some(model_used.as_str()),
                Some(route_reason_s.as_str()),
            )
            .await;

        // Insert user turn
        let user_ctx_files = serde_json::to_string(&user_msg.context_files).unwrap_or_default();
        let _ = db
            .insert_chat_transcript_turn(
                user_msg.id.as_str(),
                q_session.as_str(),
                user_msg.role.as_str(),
                user_msg.content.as_str(),
                user_msg.model_used.as_deref(),
                user_msg.tokens.map(|t| t as i64),
                user_ctx_files.as_str(),
                q_repo.as_str(),
            )
            .await;

        // Insert assistant turn into chat_transcripts (V17 legacy / VS Code history API)
        let asst_ctx_files = serde_json::to_string(&asst_msg.context_files).unwrap_or_default();
        let _ = db
            .insert_chat_transcript_turn(
                asst_msg.id.as_str(),
                q_session.as_str(),
                asst_msg.role.as_str(),
                asst_msg.content.as_str(),
                asst_msg.model_used.as_deref(),
                asst_msg.tokens.map(|t| t as i64),
                asst_ctx_files.as_str(),
                q_repo.as_str(),
            )
            .await;

        let journey_payload = journey_envelope::build_journey_envelope_v1(
            journey_id.as_str(),
            q_session.as_str(),
            thread_id_for_envelope.as_deref(),
            params.trace_id.as_deref(),
            params.correlation_id.as_deref(),
            q_repo.as_str(),
            "mcp",
            params.cognitive_profile.as_deref(),
        );
        let journey_json = journey_payload.to_string();
        if let Ok(conv_id) = db
            .chat_ensure_workspace_conversation(
                q_repo.as_str(),
                q_session.as_str(),
                thread_id_for_envelope.as_deref(),
                "mcp",
            )
            .await
        {
            let _ = db
                .chat_append_workspace_message(
                    conv_id,
                    user_msg.id.as_str(),
                    user_msg.role.as_str(),
                    user_msg.content.as_str(),
                    user_msg.model_used.as_deref(),
                    user_msg.tokens.map(|t| t as i64),
                    Some(user_ctx_files.as_str()),
                    Some(journey_json.as_str()),
                )
                .await;
            let _ = db
                .chat_append_workspace_message(
                    conv_id,
                    asst_msg.id.as_str(),
                    asst_msg.role.as_str(),
                    asst_msg.content.as_str(),
                    asst_msg.model_used.as_deref(),
                    asst_msg.tokens.map(|t| t as i64),
                    Some(asst_ctx_files.as_str()),
                    Some(journey_json.as_str()),
                )
                .await;
        }

        let now_s = now_ts();
        let date_str = ts_to_date_str(now_s);
        let server_idle_secs = now_s.saturating_sub(state.orchestrator.last_activity_ms() / 1000);
        let ctx_handle = state.orchestrator.context_handle();
        let session_age_secs = match crate::sync_poison::poison_rw_read(
            ctx_handle.read(),
            "orchestrator context",
        ) {
            Ok(g) => g
                .age_secs(&format!("chat_history:{session_id}"))
                .unwrap_or(0),
            Err(e) => {
                tracing::warn!(error = %e, "chat_message: context poisoned for session_age_secs");
                0
            }
        };

        // Record high-quality LLM turn in agent_events for Mens replay/SFT
        let mut payload = serde_json::json!({
            "type": "llm_turn",
            "agent_id": 0u64,
            "prompt": user_prompt,
            "response": response_text,
            "model": model_used,
            "tokens": tokens,
            "session_id": q_session,
            "repository_id": state.repository.repository_id,
            "temporal_context": {
                "date": date_str,
                "server_idle_secs": server_idle_secs,
                "session_age_secs": session_age_secs,
            }
        });
        if let Some(ev) = &retrieval_evidence {
            payload["retrieval"] = serde_json::to_value(ev).unwrap_or(Value::Null);
        }
        if vox_gamify::config_gate::is_enabled() {
            let _ = vox_gamify::event_router::route_event_auto_user(db, &payload).await;
        } else {
            let _ = vox_db::ops_agent_telemetry::insert_event(
                db,
                "0",
                "llm_turn",
                Some(&payload.to_string()),
            )
            .await;
        }
    }

    // 5. Return updated history + the new assistant message

    let retrieval_contradiction = retrieval_evidence
        .as_ref()
        .map(|e| e.contradiction_count > 0)
        .unwrap_or(false);
    let retrieval_boost = retrieval_evidence
        .as_ref()
        .map(|e| match e.retrieval_tier.as_str() {
            "hybrid" => 0.08_f64,
            "bm25" => 0.04_f64,
            _ => 0.0_f64,
        })
        .unwrap_or(0.0_f64);
    let grounding = (chat_grounding_score(params, mention_count) + retrieval_boost).clamp(0.0, 1.0);
    let pol = state.orchestrator_config.effective_socrates_policy();
    let session_key =
        mcp_questioning_session_key(state, "vox_chat_message", Some(session_id.as_str()));
    let turn = clarification_turn_for_session(state, &session_key).await;
    let (spent_att, max_att) = state.questioning_attention_bounds(&session_key);
    let soc = socrates_tool_meta(
        &pol,
        grounding,
        retrieval_contradiction,
        turn,
        spent_att,
        max_att,
        retrieval_evidence.as_ref(),
    );
    let mut retrieval_meta = retrieval_evidence
        .as_ref()
        .and_then(|ev| serde_json::to_value(ev).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(meta_obj) = retrieval_meta.as_object_mut() {
        meta_obj.insert(
            "task_class".to_string(),
            serde_json::Value::String("chat_turn".to_string()),
        );
        meta_obj.insert(
            "domain_tags".to_string(),
            serde_json::json!(["interactive", "general_coding"]),
        );
    } else {
        retrieval_meta = socrates_surface_tags("chat_turn", &["interactive", "general_coding"]);
    }
    // T3 Wave 4 (`T3-HIST-BATTERY-03..04`): Compute real deterministic Flesch-Kincaid response
    // quality metrics and record dual-unit Battery Unit + USD cost transaction.
    let quality_metrics = vox_orchestrator::models::quality_analyzer::analyze_response_quality(
        &params.prompt,
        &response_text,
    );
    let normalized_quality_score = (quality_metrics.quality_score / 100.0).clamp(0.0, 1.0);
    let recorded_prompt_u32 = if turn_prompt_tokens > 0 {
        turn_prompt_tokens as u32
    } else {
        est_input_tokens
    };
    let recorded_completion_u32 = if turn_completion_tokens > 0 {
        turn_completion_tokens as u32
    } else {
        tokens as u32
    };
    let (catalog_cost_per_1k_input, catalog_cost_per_1k_output) = {
        let models_arc = state.orchestrator.models_handle();
        let registry = vox_orchestrator::sync_lock::rw_read(&models_arc);
        if let Some(spec) = registry.get(&model_used) {
            let in_rate = if spec.cost_per_1k_input > 0.0 {
                spec.cost_per_1k_input
            } else {
                0.001
            };
            let out_rate = if spec.cost_per_1k_output > 0.0 {
                spec.cost_per_1k_output
            } else {
                0.002
            };
            (in_rate, out_rate)
        } else {
            (0.001, 0.002)
        }
    };
    let battery_tx = {
        let mut ledger = state.battery_ledger.lock();
        let tx = ledger.record_turn_usage(
            journey_id.as_str(),
            ts_to_date_str(turn_ts),
            model_used.as_str(),
            recorded_prompt_u32,
            recorded_completion_u32,
            turn_cost_usd,
            catalog_cost_per_1k_input,
            catalog_cost_per_1k_output,
        );
        if turn_cache_read_tokens > 0 {
            ledger.add_cache_read_tokens(turn_cache_read_tokens as u32);
        }
        tx
    };

    let llm_turn = if privacy_posture.should_persist_to_db() {
        state.db.as_ref().map(|_| {
            let provider_slug = model_used
                .split_once('/')
                .map(|(p, _)| p)
                .unwrap_or("openrouter")
                .to_string();
            let strength_tag = params
                .cognitive_profile
                .clone()
                .or_else(|| turn_classified_spoke.clone())
                .unwrap_or_else(|| "generalist".to_string());
            let task_category = task_category_for_spoke_slug(turn_classified_spoke.as_deref());
            let recorded_input_tokens = if turn_prompt_tokens > 0 {
                Some(turn_prompt_tokens as i64)
            } else {
                Some(i64::from(est_input_tokens))
            };
            let recorded_output_tokens = if turn_completion_tokens > 0 {
                Some(turn_completion_tokens as i64)
            } else {
                Some(tokens as i64)
            };
            LlmSurfaceTelemetry {
                session_id: session_id.clone(),
                user_id: None,
                tenant_id: None,
                prompt: user_prompt.clone(),
                response: response_text.clone(),
                model_id: model_used.clone(),
                provider: provider_slug,
                task_category,
                strength_tag,
                latency_ms: llm_started.elapsed().as_millis() as i64,
                input_tokens: recorded_input_tokens,
                output_tokens: recorded_output_tokens,
                cache_read_tokens: Some(turn_cache_read_tokens as i64),
                trace_id: Some(journey_id.clone()),
                success: true,
                cost_usd: turn_cost_usd,
                quality_score: Some(normalized_quality_score),
                ttft_ms: turn_ttft_ms.map(|v| v as i64),
                tpot_ms: turn_tpot_ms,
            }
        })
    } else {
        None
    };
    if privacy_posture.should_persist_to_db() {
        spawn_socrates_telemetry_with_llm(
            state,
            "vox_chat_message",
            soc.clone(),
            Some(model_used.clone()),
            Some(retrieval_meta),
            llm_turn,
        );
        spawn_questioning_trace_from_socrates(
            state,
            "vox_chat_message",
            soc.clone(),
            Some(session_key.clone()),
            Some(user_prompt.clone()),
        );
    }
    // Honest selection classification (Phase B Task B1): derive `source` from the
    // RESOLVED model (`model_used`), never the requested one — see
    // `SelectionSource::classify`'s doc comment for why.
    let requested_model_for_selection = params
        .model_override
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let global_model_for_selection = match crate::sync_poison::poison_rw_read(
        state.mcp_chat_model_override.read(),
        "mcp_chat_model_override",
    ) {
        Ok(g) => g.clone(),
        Err(_) => None,
    };
    let selection = SelectionDto::new(
        requested_model_for_selection,
        &model_used,
        global_model_for_selection.as_deref(),
        selection_reason.clone(),
    );

    // Task 13: the chat role's model — the pin it was requested as (if the
    // strict-pin branch applied one) and the id that answered. A deep turn's
    // answer comes from the research roles, not chat.
    if intent.mode != super::super::research_intent::ResearchMode::Deep && !model_used.is_empty() {
        research_trace.set_chat_model(chat_pin.as_deref(), &model_used);
    }
    if intent.mode == super::super::research_intent::ResearchMode::Quick {
        let check = super::super::research_turn::check_citations(
            &response_text,
            research_trace.sources.len(),
        );
        research_trace.push(super::super::research_turn::citation_stage(
            &check,
            research_trace.sources.len(),
        ));
    }
    events.insert(0, research_trace.to_event());

    let kb_scope_mask = KbScopeMask::from_kb_scope(params.kb_scope.as_deref());
    let posture_audit = serde_json::json!({
        "tier": params.tier.as_deref().unwrap_or("auto"),
        "clutch": params.clutch.as_deref().unwrap_or("balanced"),
        "risk": params.risk.as_deref().unwrap_or("moderate"),
        "mode": params.mode.as_deref().unwrap_or("act"),
        "oversight_level": params.oversight_level.as_deref().unwrap_or("balanced"),
        "kb_scope": params.kb_scope.as_deref().unwrap_or("auto"),
        "kb_scope_mask": kb_scope_mask,
        "preamble_skipped_reason": preamble_skipped_reason,
        "grounding_check_enabled": params.grounding_check_enabled.unwrap_or(false),
        "context_files_count": params.context_files.len(),
        "cache_read_tokens": turn_cache_read_tokens,
        "cost_usd": turn_cost_usd,
        "ttft_ms": turn_ttft_ms,
        "tpot_ms": turn_tpot_ms,
        "classified_spoke": turn_classified_spoke,
        "turn_tool_ledger_count": turn_tool_ledger.len(),
        "quality_score": quality_metrics.quality_score,
        "quality_bucket": quality_metrics.bucket,
        "flesch_reading_ease": quality_metrics.readability_score,
        "battery_debited_bu": battery_tx.debited_bu,
        "battery_remaining_bu": battery_tx.balance_after_bu,
        "cost_provenance_tier": battery_tx.cost_receipt.tier,
        "privacy_posture": privacy_posture,
        "persisted_to_disk": persisted_to_disk,
    });
    let next_queued_follow_up = {
        let mut q = state.thread_run_queue.lock();
        if q.can_settle_turn(&session_id) {
            q.pop_next_follow_up(&session_id)
        } else {
            None
        }
    };
    let result = serde_json::json!({
        "message": asst_msg,
        "history": history,
        "model_used": model_used,
        "tokens": tokens,
        "cache_read_tokens": turn_cache_read_tokens,
        "cost_usd": turn_cost_usd,
        "latency_ms": llm_started.elapsed().as_millis() as u64,
        "selection_reason": selection_reason,
        "selection": selection,
        "session_id": session_id,
        "socrates": soc,
        "retrieval": retrieval_evidence,
        "compaction_audit": compaction_audit,
        "posture_audit": posture_audit,
        "next_queued_follow_up": next_queued_follow_up,
        "events": events,
    });

    result
}
