//! Socrates grounding snippets and telemetry for chat / inline / ghost tools.

use serde::Deserialize;
use serde_json::Value;
use vox_socrates_policy::{
    CLARIFICATION_INTERRUPT_COST_MS, ClarificationStopReason, ConfidencePolicy, QuestionCandidate,
    QuestionKind, QuestioningPolicy, RiskDecision,
};

use crate::server::ServerState;

/// JSON shape of the `socrates` field returned to MCP clients (must match [`socrates_tool_meta`]).
#[derive(Debug, Deserialize)]
pub(crate) struct SocratesJsonMeta {
    pub(crate) risk_decision: RiskDecision,
    pub(crate) confidence_estimate: f64,
    pub(crate) contradiction_ratio: f64,
    #[serde(default)]
    pub(crate) questioning: Option<QuestioningJsonMeta>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct QuestioningJsonMeta {
    pub(crate) question_needed: bool,
    #[serde(default)]
    pub(crate) question_kind: Option<QuestionKind>,
    #[serde(default)]
    pub(crate) prompt: Option<String>,
    #[serde(default)]
    pub(crate) expected_information_gain_bits: f64,
    #[serde(default)]
    pub(crate) expected_user_cost: f64,
    #[serde(default)]
    pub(crate) utility_bits_per_cost: f64,
    #[serde(default)]
    pub(crate) stop_reason: Option<ClarificationStopReason>,
}

#[must_use]
pub(crate) fn socrates_system_rider(policy: &ConfidencePolicy) -> String {
    let p = policy;
    format!(
        "\n## Socrates (grounding)\n\
         - Below {:.0}% calibrated confidence: do not speculate; state what evidence is missing.\n\
         - {:.0}–{:.0}%: answer with explicit uncertainty or ask one focused clarifying question.\n\
         - Before plan-changing actions: ask a bounded clarification when scope or constraints are ambiguous.\n\
         - Above {:.0}%: answer normally; tie claims to files or tools you used.\n",
        p.abstain_threshold * 100.0,
        p.abstain_threshold * 100.0,
        p.ask_for_help_threshold * 100.0,
        p.ask_for_help_threshold * 100.0,
    )
}

pub(crate) fn spawn_socrates_telemetry(
    state: &ServerState,
    surface: &'static str,
    socrates_value: Value,
    model_used: Option<String>,
) {
    spawn_socrates_telemetry_with_meta(state, surface, socrates_value, model_used, None);
}

pub(crate) fn spawn_socrates_telemetry_with_meta(
    state: &ServerState,
    surface: &'static str,
    socrates_value: Value,
    model_used: Option<String>,
    retrieval_meta: Option<Value>,
) {
    let Some(db) = state.db.clone() else {
        return;
    };
    let repository_id = state.repository.repository_id.clone();
    tokio::spawn(async move {
        let meta = match serde_json::from_value::<SocratesJsonMeta>(socrates_value.clone()) {
            Ok(m) => m,
            Err(e) => {
                let payload = serde_json::to_string(&socrates_value)
                    .unwrap_or_else(|_| "<non-serializable>".into());
                let snippet: String = payload.chars().take(400).collect();
                tracing::warn!(
                    surface,
                    error = %e,
                    payload_snippet = %snippet,
                    "socrates telemetry: JSON shape mismatch (must match socrates_tool_meta)"
                );
                return;
            }
        };
        match db
            .record_socrates_surface_event(
                &repository_id,
                surface,
                meta.risk_decision,
                meta.confidence_estimate,
                meta.contradiction_ratio,
                model_used.as_deref(),
                retrieval_meta,
            )
            .await
        {
            Ok(id) => {
                tracing::info!(
                    target: "vox_socrates_telemetry",
                    row_id = id,
                    surface,
                    repository_id = %repository_id,
                    decision = ?meta.risk_decision,
                    "persisted socrates_surface"
                );
            }
            Err(e) => tracing::warn!(
                error = %e,
                surface,
                "socrates telemetry insert failed"
            ),
        }
    });
}

pub(crate) fn spawn_questioning_trace_from_socrates(
    state: &ServerState,
    surface: &'static str,
    socrates_value: Value,
    session_id: Option<String>,
    prompt_hint: Option<String>,
) {
    let Some(db) = state.db.clone() else {
        return;
    };
    let spend_state = state.clone();
    let repository_id = state.repository.repository_id.clone();
    tokio::spawn(async move {
        let meta = match serde_json::from_value::<SocratesJsonMeta>(socrates_value.clone()) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(
                    surface,
                    error = %e,
                    "questioning trace skipped: invalid socrates payload"
                );
                return;
            }
        };
        let Some(q) = meta.questioning else {
            return;
        };

        let session_key = session_id.unwrap_or_else(|| format!("mcp:{repository_id}:{surface}"));
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let open = match db
            .find_open_question_session_for_repo(&session_key, &repository_id)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(surface, error = %e, "questioning trace: open_session lookup failed");
                None
            }
        };

        if q.question_needed {
            if db
                .has_pending_clarification_for_mcp_session(&session_key, &repository_id)
                .await
                .unwrap_or(false)
            {
                let _ = db
                    .record_questioning_metric(
                        &session_key,
                        Some(q.expected_information_gain_bits),
                        &serde_json::json!({
                            "surface": surface,
                            "question_needed": true,
                            "deferred": true,
                            "reason": "pending_unanswered_clarification",
                        })
                        .to_string(),
                    )
                    .await;
                return;
            }

            let question_session_id = if let Some(ref row) = open {
                row.id
            } else {
                match db
                    .create_question_session(vox_db::QuestionSessionCreateParams {
                        session_id: &session_key,
                        repository_id: &repository_id,
                        task_id: None,
                        policy_version: "v1",
                        started_at_ms: now_ms,
                    })
                    .await
                {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::warn!(surface, error = %e, "questioning trace: create_session failed");
                        return;
                    }
                }
            };

            let turn_index = match db.next_question_event_turn_index(question_session_id).await {
                Ok(t) => i64::from(t),
                Err(e) => {
                    tracing::warn!(surface, error = %e, "questioning trace: turn_index failed");
                    return;
                }
            };

            let kind = q.question_kind.unwrap_or(QuestionKind::OpenEnded);
            let kind_str = match kind {
                QuestionKind::MultipleChoice => "multiple_choice",
                QuestionKind::OpenEnded => "open_ended",
                QuestionKind::Entry => "entry",
            };
            let prompt = q
                .prompt
                .clone()
                .or(prompt_hint)
                .unwrap_or_else(|| "Clarify intent".to_string());
            let question_id = format!("{surface}:{question_session_id}:{now_ms}");
            let utility = if q.utility_bits_per_cost > 0.0 {
                q.utility_bits_per_cost
            } else {
                q.expected_information_gain_bits / q.expected_user_cost.max(1e-6)
            };
            let event_row_id = match db
                .insert_question_event(vox_db::QuestionEventParams {
                    question_session_id,
                    question_id: &question_id,
                    turn_index,
                    actor: "assistant",
                    question_kind: kind_str,
                    prompt: &prompt,
                    expected_information_gain_bits: q.expected_information_gain_bits,
                    expected_user_cost: q.expected_user_cost,
                    utility_bits_per_cost: utility,
                    answer_text: None,
                    answer_type: None,
                    answered_at_ms: None,
                    created_at_ms: now_ms,
                })
                .await
            {
                Ok(id) => id,
                Err(e) => {
                    tracing::warn!(surface, error = %e, "questioning trace: insert_question_event failed");
                    return;
                }
            };

            if kind == QuestionKind::MultipleChoice {
                let p = 1.0_f64 / 3.0_f64;
                let opts: [(&str, &str, bool); 3] = [
                    ("a", "First listed intent / primary interpretation", false),
                    ("b", "Alternative scope or constraint setup", false),
                    ("c", "Other — describe in chat", true),
                ];
                for (oid, label, is_other) in opts {
                    let _ = db
                        .upsert_question_option(vox_db::QuestionOptionParams {
                            question_event_id: event_row_id,
                            option_id: oid,
                            label,
                            prior_probability: Some(p),
                            posterior_probability: None,
                            is_other,
                        })
                        .await;
                }
            }

            let clarify_cost = (q.expected_user_cost.clamp(0.0, 1.0)
                * CLARIFICATION_INTERRUPT_COST_MS as f64)
                .ceil() as u64;
            spend_state.record_questioning_attention_spend(&session_key, clarify_cost.max(1));

            let msg_uuid = format!("clarification-{surface}-{question_session_id}-{now_ms}");
            let _ = db
                .send_a2a_clarification_message(vox_db::A2aClarificationMessageParams {
                    message_uuid: &msg_uuid,
                    sender_agent: "mcp_chat",
                    receiver_agent: "orchestrator_clarifier",
                    msg_type: "clarification_request",
                    repository_id: &repository_id,
                    thread_id: Some(&session_key),
                    priority: 5,
                    clarification_intent: "disambiguate_user_intent",
                    hypothesis_set_id: &question_id,
                    question_kind: Some(kind_str),
                    expected_information_gain_bits: Some(q.expected_information_gain_bits),
                    expected_user_cost: Some(q.expected_user_cost),
                    requested_evidence_dimensions_json: None,
                    urgency: Some("normal"),
                    stop_policy_json: Some(r#"{"max_turns":3,"min_gain_bits":0.08}"#),
                })
                .await;

            let _ = db
                .record_questioning_metric(
                    &session_key,
                    Some(q.expected_information_gain_bits),
                    &serde_json::json!({
                        "surface": surface,
                        "question_needed": true,
                        "question_kind": kind_str,
                        "question_id": question_id,
                        "turn_index": turn_index,
                        "expected_information_gain_bits": q.expected_information_gain_bits,
                        "expected_user_cost": q.expected_user_cost,
                        "utility_bits_per_cost": utility,
                    })
                    .to_string(),
                )
                .await;
            return;
        }

        if let Some(stop_reason) = q.stop_reason {
            let stop_reason_str = match stop_reason {
                ClarificationStopReason::ConfidenceSufficient => "confidence_sufficient",
                ClarificationStopReason::RiskGateBlocked => "risk_gate_blocked",
                ClarificationStopReason::MaxClarificationTurns => "max_clarification_turns",
                ClarificationStopReason::MarginalGainTooLow => "marginal_gain_too_low",
                ClarificationStopReason::UserCostTooHigh => "user_cost_too_high",
                ClarificationStopReason::AttentionBudgetExceeded => "attention_budget_exceeded",
            };

            let question_session_id = if let Some(row) = open {
                row.id
            } else {
                match db
                    .create_question_session(vox_db::QuestionSessionCreateParams {
                        session_id: &session_key,
                        repository_id: &repository_id,
                        task_id: None,
                        policy_version: "v1",
                        started_at_ms: now_ms,
                    })
                    .await
                {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::warn!(surface, error = %e, "questioning trace: create_session failed");
                        return;
                    }
                }
            };

            let turn_for_stop = db
                .next_question_event_turn_index(question_session_id)
                .await
                .map(i64::from)
                .unwrap_or(0_i64);

            let _ = db
                .insert_question_stop_event(vox_db::QuestionStopEventParams {
                    question_session_id,
                    stop_reason: stop_reason_str,
                    confidence_at_stop: Some(meta.confidence_estimate),
                    marginal_gain_bits: Some(q.expected_information_gain_bits),
                    expected_user_cost: Some(q.expected_user_cost),
                    turn_index: Some(turn_for_stop),
                    created_at_ms: now_ms,
                })
                .await;
            let _ = db
                .record_questioning_metric(
                    &session_key,
                    Some(q.expected_information_gain_bits),
                    &serde_json::json!({
                        "surface": surface,
                        "question_needed": false,
                        "stop_reason": stop_reason_str,
                        "confidence_estimate": meta.confidence_estimate,
                        "expected_information_gain_bits": q.expected_information_gain_bits,
                        "expected_user_cost": q.expected_user_cost,
                    })
                    .to_string(),
                )
                .await;
            let _ = db
                .close_question_session(question_session_id, "question_skipped", now_ms)
                .await;
        }
    });
}

#[must_use]
pub(crate) fn mcp_questioning_session_key(
    state: &ServerState,
    surface: &'static str,
    explicit_session_id: Option<&str>,
) -> String {
    explicit_session_id
        .map(std::string::ToString::to_string)
        .unwrap_or_else(|| format!("mcp:{}:{surface}", state.repository.repository_id))
}

pub(crate) async fn clarification_turn_for_session(state: &ServerState, session_key: &str) -> u32 {
    let Some(db) = state.db.as_ref() else {
        return 0;
    };
    let repo = state.repository.repository_id.as_str();
    match db
        .count_assistant_questions_in_open_session(session_key, repo)
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::debug!(error = %e, "clarification_turn_for_session failed");
            0
        }
    }
}

#[must_use]
pub(crate) fn socrates_tool_meta(
    policy: &ConfidencePolicy,
    grounding_score: f64,
    contradiction_hint: bool,
    clarification_turn_index: u32,
    spent_clarification_attention_ms: u64,
    max_clarification_attention_ms: u64,
) -> Value {
    let p = policy;
    let cr = if contradiction_hint {
        p.abstain_threshold
    } else {
        0.0_f64
    };
    let decision = p.evaluate_risk_decision(grounding_score, cr);
    let questioning_policy = QuestioningPolicy::default();
    let candidates = vec![
        QuestionCandidate {
            prompt: "Which option best matches your intent?".to_string(),
            question_kind: QuestionKind::MultipleChoice,
            expected_information_gain_bits: 0.20,
            expected_user_cost: 0.25,
        },
        QuestionCandidate {
            prompt: "Please share the key constraint in one sentence.".to_string(),
            question_kind: QuestionKind::OpenEnded,
            expected_information_gain_bits: 0.16,
            expected_user_cost: 0.45,
        },
        QuestionCandidate {
            prompt: "Provide the exact target value or path.".to_string(),
            question_kind: QuestionKind::Entry,
            expected_information_gain_bits: 0.14,
            expected_user_cost: 0.30,
        },
    ];
    let selection = p.select_clarification_question(
        grounding_score,
        cr,
        clarification_turn_index,
        &candidates,
        questioning_policy,
        spent_clarification_attention_ms,
        max_clarification_attention_ms,
    );
    serde_json::json!({
        "risk_decision": decision,
        "confidence_estimate": grounding_score,
        "contradiction_ratio": cr,
        "questioning": {
            "question_needed": selection.question_needed,
            "question_kind": selection.question_kind,
            "prompt": selection.prompt,
            "expected_information_gain_bits": selection.expected_information_gain_bits,
            "expected_user_cost": selection.expected_user_cost,
            "utility_bits_per_cost": selection.utility_bits_per_cost,
            "stop_reason": selection.stop_reason,
        }
    })
}
