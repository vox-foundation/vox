use super::*;

/// Narrow `ModelSpec -> LlmConfig` mapper (Task 1.3d, F24 wiring).
///
/// Deliberately covers only the two simplest, most common provider shapes:
///
/// - [`ProviderType::OpenRouter`]: Vox's de-facto default provider for the vast
///   majority of catalog models. Maps to [`LlmConfig::openrouter`], which already
///   resolves the API key from `vox_secrets::SecretId::OpenRouterApiKey` — no
///   fallback/vision/budget logic required.
/// - [`ProviderType::Ollama`]: local inference with a fixed, well-known URL shape
///   (`$OLLAMA_URL/v1/chat/completions`), mirroring the existing
///   `ModelRegistry::get_llm_config` conversion for the same provider type
///   (`crates/vox-orchestrator/src/models/registry.rs`).
/// - [`ProviderType::GoogleDirect`] and [`ProviderType::Anthropic`]: mapped to
///   [`LlmConfig::openrouter`], preserving agent loop native tool execution.
/// - [`ProviderType::VoxLocal`]: local `vox mens serve` (Task B2, MENS
///   end-to-end completion). Maps to `$VOX_LOCAL_ENDPOINT/v1/chat/completions`
///   (or the config SSOT's default probe candidate when unset) — the new
///   OpenAI-chat-shaped route added to `vox-ml-cli`'s serve binary alongside
///   its existing `/v1/completions` (text-prompt) route.
///
/// Returns `None` for every other [`ProviderType`] (`HuggingFaceRouter`,
/// `PopuliMesh`, `Mistral`, `DeepSeek`, `SambaNova`, `Groq`, `Cerebras`,
/// `Custom`) — those require the provider-specific fallback chains,
/// dedicated-endpoint resolution, or unreachable-provider handling that only
/// `crate::llm_bridge::infer::mcp_infer_completion` implements, and this mapper
/// deliberately does not attempt to replicate that pipeline. Callers must fall
/// back to `mcp_infer_completion` when this returns `None`.
#[must_use]
pub(crate) fn model_spec_to_llm_config(spec: &ModelSpec) -> Option<LlmConfig> {
    match spec.provider_type {
        ProviderType::OpenRouter => Some(LlmConfig::openrouter(spec.id.clone())),
        ProviderType::GoogleDirect | ProviderType::Anthropic => {
            let model = if spec.id.contains('/') {
                spec.id.clone()
            } else if !spec.canonical_slug.is_empty() && spec.canonical_slug.contains('/') {
                spec.canonical_slug.clone()
            } else {
                match spec.provider_type {
                    ProviderType::GoogleDirect => format!("google/{}", spec.id),
                    ProviderType::Anthropic => format!("anthropic/{}", spec.id),
                    _ => spec.id.clone(),
                }
            };
            Some(LlmConfig::openrouter(model))
        }
        ProviderType::Ollama => {
            // An empty/unset OLLAMA_URL must not disable tool-calling: the
            // config SSOT already supplies a working default, and returning
            // None here routes the turn to the non-tool
            // mcp_infer_completion path with no error.
            let base = vox_secrets::resolve_secret(vox_secrets::SecretId::OllamaUrl)
                .expose()
                .filter(|s: &&str| !s.trim().is_empty())
                .map(std::string::ToString::to_string)
                .unwrap_or_else(vox_config::inference::local_ollama_populi_base_url);
            let base_url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));
            Some(LlmConfig {
                provider: "ollama".to_string(),
                model: spec.id.clone(),
                cost_per_1k: None,
                base_url: Some(base_url),
                api_key: None,
                temperature: None,
                top_p: None,
                max_tokens: Some(spec.max_tokens),
                response_format: None,
                tools: None,
                tool_choice: None,
                timeout_ms: None,
                telemetry_session_id: None,
                telemetry_user_id: None,
                telemetry_task_category: None,
                telemetry_strength_tag: None,
                telemetry_trace_id: None,
                telemetry_attempt_number: None,
                telemetry_skip_interaction: true,
                seed: None,
            })
        }
        ProviderType::VoxLocal => {
            let base = vox_config::inference::vox_local_endpoint_probe_candidates()
                .into_iter()
                .next()
                .unwrap_or_else(|| vox_config::inference::VOX_LOCAL_ENDPOINT_DEFAULT.to_string());
            let base_url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));
            Some(LlmConfig {
                provider: "voxlocal".to_string(),
                model: spec.id.clone(),
                cost_per_1k: None,
                base_url: Some(base_url),
                api_key: None,
                temperature: None,
                top_p: None,
                max_tokens: Some(spec.max_tokens),
                response_format: None,
                tools: None,
                tool_choice: None,
                timeout_ms: None,
                telemetry_session_id: None,
                telemetry_user_id: None,
                telemetry_task_category: None,
                telemetry_strength_tag: None,
                telemetry_trace_id: None,
                telemetry_attempt_number: None,
                telemetry_skip_interaction: true,
                seed: None,
            })
        }
        ProviderType::HuggingFaceRouter
        | ProviderType::PopuliMesh
        | ProviderType::Mistral
        | ProviderType::DeepSeek
        | ProviderType::SambaNova
        | ProviderType::Groq
        | ProviderType::Cerebras => {
            let provider = match spec.provider_type {
                ProviderType::HuggingFaceRouter => "hf_router",
                ProviderType::PopuliMesh => "populi_local",
                ProviderType::Mistral => "mistral",
                ProviderType::DeepSeek => "deepseek",
                ProviderType::SambaNova => "sambanova",
                ProviderType::Groq => "groq",
                ProviderType::Cerebras => "cerebras",
                _ => "openrouter",
            };
            Some(LlmConfig {
                provider: provider.to_string(),
                model: spec.id.clone(),
                cost_per_1k: None,
                base_url: None,
                api_key: None,
                temperature: None,
                top_p: None,
                max_tokens: Some(spec.max_tokens),
                response_format: None,
                tools: None,
                tool_choice: None,
                timeout_ms: None,
                telemetry_session_id: None,
                telemetry_user_id: None,
                telemetry_task_category: None,
                telemetry_strength_tag: None,
                telemetry_trace_id: None,
                telemetry_attempt_number: None,
                telemetry_skip_interaction: true,
                seed: None,
            })
        }
        ProviderType::Custom(_) => None,
    }
}

/// Hard bound on the number of model round-trips within a single `run_agent_turn`
/// call. Each iteration is: one `llm_chat` call, plus (if it requested tools) one
/// `handle_tool_call_with_mode` dispatch per call before looping again. 8 is chosen
/// to comfortably cover realistic tool-use chains (a typical "look something up,
/// then answer" turn is 2-3 iterations) while guaranteeing the loop cannot spin
/// forever if a model pathologically keeps requesting tools — this is a safety
/// bound, not a tuning knob callers are expected to raise routinely.
pub(crate) const DEFAULT_MAX_ITERATIONS: usize = 8;

/// Result of running one agent turn to completion (or to the iteration bound).
///
/// Called from `vox_chat_message`'s live entrypoint (`message.rs`) for the subset
/// of provider/model choices `model_spec_to_llm_config` can map without the
/// full `mcp_infer_completion` fallback pipeline (Task 1.3d, F24).
#[derive(Debug, Clone)]
pub struct AgentTurnOutcome {
    /// The final assistant-facing text. When the iteration bound is hit before the
    /// model stops requesting tools, this is the last assistant text seen (which may
    /// be empty) with [`AgentTurnOutcome::hit_iteration_limit`] set to `true` so
    /// callers can surface that distinctly rather than silently truncating.
    pub final_text: String,
    /// Model id reported by the last successful `llm_chat` response, or empty string
    /// if every attempt failed before any response was received.
    pub model_used: String,
    /// Total number of individual tool calls dispatched across every iteration of
    /// this turn (for observability/tests — not the same as iteration count, since
    /// one iteration's response may request several calls at once).
    pub tool_calls_made: usize,
    /// `true` if the loop stopped only because `DEFAULT_MAX_ITERATIONS` (or the
    /// caller-supplied `max_iterations`) was reached while the model was still
    /// requesting tools, rather than because the model returned a final answer.
    pub hit_iteration_limit: bool,
    /// Sum of `prompt_tokens + completion_tokens` reported by the wire response
    /// across every `llm_chat` round-trip made during this turn (for transcript
    /// bookkeeping — `message.rs` records this alongside the persisted turn).
    pub total_tokens: u64,
    /// Chat-turn-visible events derived from tool RESULTS during this turn (see
    /// `turn_event_for_result`) — e.g. a skill activation chip. Empty unless a
    /// dispatched tool call both matches a known event-worthy tool AND actually
    /// succeeded. Dispatched calls also emit `tool_receipt` events for cryptographic ledger verification.
    pub events: Vec<serde_json::Value>,
    /// Wall-clock latency of the final iteration's `llm_chat`/`stream_final_answer` call, in
    /// ms. `None` only if every iteration failed before any response was received (in which
    /// case the turn as a whole already errored out before `AgentTurnOutcome` is built).
    pub latency_ms: Option<u64>,
    /// Task M3: time to first token, in ms, from the final iteration's response. Only real on
    /// the streaming path (`stream_tokens: true`); the non-streaming path reports
    /// `latency_ms` here (see [`LlmResponse::ttft_ms`]'s doc comment).
    pub ttft_ms: Option<u64>,
    /// Task M3: time per output token, in ms, from the final iteration's response. `None`
    /// when that iteration had zero completion tokens.
    pub tpot_ms: Option<f64>,
    /// Task B2 (MENS end-to-end completion, VoxLocal tool-calling salvage policy):
    /// total tool calls dispatched this turn, including calls recovered from
    /// unstructured text via the salvage policy (see `telemetry`'s
    /// `tool_call_salvaged` key). Always equal to `tool_calls_made` today — a
    /// separate field so callers keyed on "was anything dispatched" don't need to
    /// know this struct predates the salvage policy.
    pub tool_calls_dispatched: usize,
    /// Task B2: the turn's final assistant-facing text, or `None` only when the
    /// loop hit `max_iterations` mid-tool-call with no final answer produced
    /// (`hit_iteration_limit: true` and `final_text` empty).
    pub reply_text: Option<String>,
    /// Task B2: per-turn telemetry flags.
    pub telemetry: std::collections::HashMap<String, serde_json::Value>,
    /// Phase 148 (SOTA-WIRECACHE-02): Sum of provider-reported `cache_read_tokens` across iterations.
    pub cache_read_tokens: u64,
    /// Phase 148 (SOTA-WIRECACHE-02): Sum of provider-reported `cost_usd` across iterations.
    pub cost_usd: Option<f64>,
    /// Phase 153 (SOTA-TELEM-CLOSE-01): Sum of prompt tokens across iterations.
    pub prompt_tokens: u64,
    /// Phase 153 (SOTA-TELEM-CLOSE-01): Sum of completion tokens across iterations.
    pub completion_tokens: u64,
}

/// Max chars of a model-authored string (e.g. a raw skill id) echoed into a
/// turn event. Events render in trusted, system-styled chrome, so anything
/// derived from model/tool-call content must be bounded before it reaches
/// the UI, independent of the id-shape validation below.
pub(super) const TURN_EVENT_STRING_CAP: usize = 200;

/// A skill id is only ever a short slug (see `SkillManifest::id` producers —
/// installer-assigned, never user/model free text at creation time). Reject
/// anything else (path separators, `..`, whitespace, control chars) rather
/// than echoing it into system-styled UI: a model can put an arbitrary string
/// in a tool call's `id` argument (including content it read from untrusted
/// tool output earlier in the transcript), so this is a format allowlist, not
/// a registry lookup — [`run_agent_turn`]'s call site additionally confirms
/// the id resolves in the real skill registry before trusting it further.
pub(super) fn is_plausible_skill_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= TURN_EVENT_STRING_CAP
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Derive a chat-turn-visible event from a dispatched tool call's RESULT —
/// never from the call's arguments alone. `success` must reflect whether the
/// dispatch actually succeeded (e.g. `!tool_json_envelope_is_error(&content)`
/// at the call site), not merely whether the model requested the call.
///
/// Security rationale (see module docs and
/// `docs/superpowers/plans/2026-08-28-chat-harness-unification.md` Phase E
/// Task E1): a model can put anything in `args`, including text an earlier,
/// untrusted tool result injected into the conversation. Gating purely on
/// `success` — which is derived from what the tool dispatcher actually did,
/// not from what the model claimed — is what keeps a denied/errored/unknown
/// call from rendering a trusted-looking "skill activated" chip.
///
/// `result_content` is the raw dispatch response body (a [`crate::params::ToolResult`]
/// JSON envelope, e.g. `{"success":true,"data":{"agent_id":7,...}}`) — needed for
/// the delegation event (Phase D Task D3) because the spawned `agent_id` is
/// server-generated and only exists in the RESULT, never in `args`.
///
/// Returns `None` when `success` is `false`, or when `tool_name` is not a
/// tool this function knows how to turn into an event.
pub(crate) fn turn_event_for_result(
    tool_name: &str,
    args: &serde_json::Value,
    result_content: &str,
    success: bool,
) -> Option<serde_json::Value> {
    if !success {
        return None;
    }
    match tool_name {
        "vox_skill_use" => {
            let raw_id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let skill_id = if is_plausible_skill_id(raw_id) {
                raw_id.to_string()
            } else {
                "unknown".to_string()
            };
            Some(serde_json::json!({
                "kind": "skill_activated",
                "skill_id": skill_id,
            }))
        }
        "vox_spawn_agent" | "vox_submit_task" => {
            // Phase D Task D3: a "delegation" transcript row correlating this
            // turn to the agent/task it spawned. `agent_id` is always present
            // on both tools' success payloads (`SpawnAgentParams`'s handler and
            // `SubmitTaskResponse`); `task_id` only on `vox_submit_task`.
            let envelope: serde_json::Value = serde_json::from_str(result_content).ok()?;
            let data = envelope.get("data")?;
            let agent_id = data.get("agent_id").and_then(serde_json::Value::as_u64)?;
            Some(serde_json::json!({
                "kind": "delegation_spawned",
                "tool": tool_name,
                "agent_id": agent_id,
                "task_id": data.get("task_id").and_then(serde_json::Value::as_u64),
            }))
        }
        "vox_deep_research" | "vox_research" | "vox_research_run" => {
            let envelope: serde_json::Value = serde_json::from_str(result_content).ok()?;
            let data = envelope.get("data").unwrap_or(&envelope);
            Some(serde_json::json!({
                "kind": "research_milestone",
                "tool": tool_name,
                "query": args.get("query").and_then(|v| v.as_str()).unwrap_or(""),
                "waves_executed": data.get("waves_executed").and_then(serde_json::Value::as_u64).unwrap_or(1),
                "claims_verified": data.get("claims_verified").and_then(serde_json::Value::as_u64).unwrap_or(0),
                "contradictions_resolved": data.get("contradictions_resolved").and_then(serde_json::Value::as_u64).unwrap_or(0),
            }))
        }
        "vox_verify_task_claims" => {
            // Receipt IDs are model-supplied (fabricated ones by definition), so the
            // chip shows counts only, never the strings.
            let envelope: serde_json::Value = serde_json::from_str(result_content).ok()?;
            let data = envelope.get("data")?;
            let valid = data
                .get("valid")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let fabricated = data
                .get("fabricated")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let unverified = data
                .get("unverified")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            Some(serde_json::json!({
                "kind": "receipt_claims",
                "valid": valid,
                "fabricated": fabricated,
                "unverified": unverified,
            }))
        }
        _ => None,
    }
}

/// Builds a `tool_receipt` event for the chat transcript.
///
/// Security rationale: every field in this event is server-derived
/// (registry-validated tool name, server-minted UUIDv7, ledger verification)
/// and nothing comes from `call.arguments`, matching [`turn_event_for_result`]'s
/// rule against echoing unvetted model payloads into trusted UI chrome.
pub(crate) fn receipt_turn_event(
    r: &crate::dispatch::ToolReceiptOutcome,
    verified: bool,
) -> serde_json::Value {
    serde_json::json!({
        "kind": "tool_receipt",
        "tool": r.tool_name,
        "receipt_id": r.receipt_id,
        "fulfilled": r.fulfilled,
        "verified": verified,
    })
}

#[cfg(test)]
mod turn_event_tests {
    use super::turn_event_for_result;
    use serde_json::json;

    #[test]
    fn skill_event_comes_from_the_result_not_the_call() {
        // Emitting from args would render "skill activated · X" for a call that
        // was denied, unknown, or errored — letting injected content assert in
        // system-styled UI that a trusted skill ran.
        assert!(
            turn_event_for_result("vox_skill_use", &json!({"id":"ponytail"}), "", true).is_some()
        );
        assert!(
            turn_event_for_result("vox_skill_use", &json!({"id":"ponytail"}), "", false).is_none()
        );
    }

    #[test]
    fn unknown_skill_ids_are_labelled_unknown() {
        let ev =
            turn_event_for_result("vox_skill_use", &json!({"id":"../../etc/passwd"}), "", true)
                .expect("event");
        assert_eq!(ev["skill_id"], "unknown");
    }

    /// Phase D Task D3: a successful `vox_spawn_agent` call must produce a
    /// transcript-visible delegation event carrying the spawned `agent_id` —
    /// read from the dispatch RESULT (never from `args`, which a model
    /// controls and which never contains the server-assigned id anyway).
    #[test]
    fn spawn_agent_success_emits_delegation_event_with_agent_id() {
        let result = r#"{"success":true,"data":{"agent_id":42,"name":"child","dynamic":true}}"#;
        let ev = turn_event_for_result("vox_spawn_agent", &json!({}), result, true)
            .expect("delegation event");
        assert_eq!(ev["kind"], "delegation_spawned");
        assert_eq!(ev["tool"], "vox_spawn_agent");
        assert_eq!(ev["agent_id"], 42);
        assert!(ev["task_id"].is_null());
    }

    #[test]
    fn submit_task_success_emits_delegation_event_with_task_and_agent_id() {
        let result = r#"{"success":true,"data":{"task_id":9,"agent_id":3}}"#;
        let ev = turn_event_for_result("vox_submit_task", &json!({}), result, true)
            .expect("delegation event");
        assert_eq!(ev["agent_id"], 3);
        assert_eq!(ev["task_id"], 9);
    }

    #[test]
    fn failed_spawn_emits_no_delegation_event() {
        // Same rationale as the skill test above: a denied/errored call must
        // not render a trusted-looking "delegated" chip.
        let result = r#"{"success":false,"error":"boom"}"#;
        assert!(turn_event_for_result("vox_spawn_agent", &json!({}), result, false).is_none());
    }

    #[test]
    fn malformed_result_body_yields_no_event_rather_than_panicking() {
        assert!(turn_event_for_result("vox_spawn_agent", &json!({}), "not json", true).is_none());
    }

    #[test]
    fn test_turn_event_for_research_result() {
        let args = json!({ "query": "SQLite JSONB performance" });
        let result_content = json!({
            "success": true,
            "data": {
                "waves_executed": 3,
                "claims_verified": 12,
                "contradictions_resolved": 2
            }
        })
        .to_string();
        let event =
            turn_event_for_result("vox_deep_research", &args, &result_content, true).unwrap();
        assert_eq!(event["kind"], "research_milestone");
        assert_eq!(event["waves_executed"], 3);
        assert_eq!(event["claims_verified"], 12);
        assert_eq!(event["contradictions_resolved"], 2);

        let event_run =
            turn_event_for_result("vox_research_run", &args, &result_content, true).unwrap();
        assert_eq!(event_run["kind"], "research_milestone");
        assert_eq!(event_run["tool"], "vox_research_run");
    }

    #[test]
    fn claims_verdict_event_reports_counts_only() {
        let args = json!({ "receipt_ids": ["evil <b>id</b>"] });
        let result_content = json!({
            "success": true,
            "data": {
                "valid": ["a", "b"],
                "fabricated": ["evil <b>id</b>"],
                "unverified": []
            }
        })
        .to_string();
        let ev = turn_event_for_result("vox_verify_task_claims", &args, &result_content, true)
            .expect("claims verdict event");
        assert_eq!(ev["kind"], "receipt_claims");
        assert_eq!(ev["valid"], 2);
        assert_eq!(ev["fabricated"], 1);
        assert_eq!(ev["unverified"], 0);
        let serialized = serde_json::to_string(&ev).unwrap();
        assert!(!serialized.contains("evil"));
    }

    #[test]
    fn claims_verdict_event_requires_success() {
        let result = json!({
            "success": true,
            "data": {
                "valid": ["a"],
                "fabricated": [],
                "unverified": []
            }
        })
        .to_string();
        assert!(
            turn_event_for_result("vox_verify_task_claims", &json!({}), &result, false).is_none()
        );
        assert!(
            turn_event_for_result("vox_verify_task_claims", &json!({}), "not json", true).is_none()
        );
    }
}
