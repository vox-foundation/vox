//! MCP tools for Vox Oratio (Candle Whisper STT).

use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::llm_bridge::{McpInferRouting, mcp_infer_completion};
use crate::server::ServerState;

fn resolve_audio_path(state: &ServerState, path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        state.repository.root.join(p)
    }
}

fn parse_profile(args: &Value) -> vox_oratio::refine::OratioCorrectionProfile {
    match args
        .get("profile")
        .and_then(|v| v.as_str())
        .unwrap_or("balanced")
    {
        "conservative" => vox_oratio::refine::OratioCorrectionProfile::Conservative,
        "aggressive" => vox_oratio::refine::OratioCorrectionProfile::Aggressive,
        _ => vox_oratio::refine::OratioCorrectionProfile::Balanced,
    }
}

fn parse_route_mode(args: &Value) -> vox_oratio::RouteMode {
    match args
        .get("route_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("none")
    {
        "tool" => vox_oratio::RouteMode::Tool,
        "chat" => vox_oratio::RouteMode::Chat,
        "orchestrator" => vox_oratio::RouteMode::Orchestrator,
        _ => vox_oratio::RouteMode::None,
    }
}

/// `vox_oratio_transcribe`: thin STT + deterministic refine only (no session envelope, no routing).
pub fn transcribe(state: &ServerState, args: Value) -> anyhow::Result<String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing string field `path`"))?;
    let full = resolve_audio_path(state, path);
    let language_hint = args
        .get("language_hint")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);
    let debug_parser_payload = args
        .get("debug_parser_payload")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let profile = parse_profile(&args);
    let rtc = vox_oratio::OratioRuntimeConfig::resolve();
    let ctx =
        vox_oratio::refine::CorrectionContext::from_runtime(&rtc, profile, debug_parser_payload);
    let detail = vox_oratio::transcribe_path_detailed(&full, &ctx, language_hint.as_deref())?;
    let correlation_id = vox_oratio::trace::new_correlation_id();
    let mut out = json!({
        "path": full,
        "correlation_id": correlation_id,
        "raw_text": detail.raw_text,
        "refined_text": detail.refined_text,
        "text": detail.refined_text,
        "confidence": detail.confidence,
        "clarification_recommended": vox_oratio::clarification_recommended(
            detail.confidence,
            rtc.routing.tool_route_min_confidence,
        ),
    });
    if debug_parser_payload {
        out["correction_trace"] = json!(detail.correction_trace);
        out["runtime_config"] = vox_oratio::runtime_config_diagnostic_json(&rtc);
    }
    Ok(serde_json::to_string(&out)?)
}

fn validate_llm_surface(original: &str, corrected: &str) -> bool {
    if corrected.is_empty() {
        return false;
    }
    let max_len = original.len().saturating_mul(5).max(1024).min(32_768);
    if corrected.len() > max_len {
        return false;
    }
    for marker in ["::", "--"] {
        let oc = original.matches(marker).count();
        let cc = corrected.matches(marker).count();
        if cc + 2 < oc {
            return false;
        }
    }
    let om = original.matches('/').count() + original.matches('\\').count();
    let cm = corrected.matches('/').count() + corrected.matches('\\').count();
    if om > 0 && cm + 2 < om {
        return false;
    }
    true
}

/// Ensure flag-like, path-like, and module-path tokens from the deterministic transcript appear in the LLM output.
fn protected_tokens_preserved(original: &str, corrected: &str) -> bool {
    for raw in original.split_whitespace() {
        let flag_like = raw.starts_with("--");
        let path_like = raw.contains('/') || raw.contains('\\');
        let mod_like = raw.contains("::");
        if flag_like || path_like || mod_like {
            if !corrected.contains(raw) {
                let redacted = if path_like {
                    Path::new(raw.trim_matches(|c| c == '`' || c == '"'))
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("(path)")
                } else {
                    raw
                };
                tracing::debug!(
                    target: "vox_mcp_oratio",
                    stage = "llm_pass",
                    missing_token_redacted = redacted,
                    "protected token not preserved in LLM correction"
                );
                return false;
            }
        }
    }
    true
}

fn llm_changes_well_formed(v: &Value) -> bool {
    match v.get("changes") {
        None => true,
        Some(Value::Array(arr)) => arr.iter().all(|item| {
            item.get("before").and_then(|x| x.as_str()).is_some()
                && item.get("after").and_then(|x| x.as_str()).is_some()
        }),
        Some(_) => false,
    }
}

fn llm_confidence_field_ok(v: &Value) -> bool {
    match v.get("confidence") {
        None => true,
        Some(c) => c.as_f64().is_some_and(|x| (0.0..=1.0).contains(&x)),
    }
}

fn strip_json_fence(s: &str) -> String {
    let block = s.trim();
    if let Some(rest) = block.strip_prefix("```json") {
        let mut inner = rest.trim_start_matches(['\n', '\r']).trim();
        if let Some(pos) = inner.rfind("```") {
            inner = inner[..pos].trim();
        }
        return inner.to_string();
    }
    if block.starts_with("```") {
        let rest = block.strip_prefix("```").unwrap_or(block).trim();
        let inner = rest.strip_suffix("```").unwrap_or(rest).trim();
        return inner.to_string();
    }
    block.to_string()
}

async fn maybe_llm_polish(
    state: &ServerState,
    session: &vox_oratio::OratioSessionResult,
    route_mode: vox_oratio::RouteMode,
    llm_refinement: bool,
    llm_min_det_confidence: f32,
    llm_max_output_tokens: u64,
    _runtime: &vox_oratio::OratioRuntimeConfig,
) -> Value {
    if !llm_refinement {
        return json!({
            "applied": false,
            "skipped": true,
            "reason": "llm_refinement_disabled",
        });
    }
    let route_requires_clarity = matches!(
        route_mode,
        vox_oratio::RouteMode::Tool | vox_oratio::RouteMode::Orchestrator
    );
    let entropy_high = session.correction_trace.len() > 8;
    let should_llm =
        session.confidence < llm_min_det_confidence || entropy_high || route_requires_clarity;
    if !should_llm {
        return json!({
            "skipped": true,
            "reason": "deterministic_confidence_sufficient",
            "deterministic_confidence": session.confidence,
        });
    }

    let user_prompt = format!(
        "Normalize this speech-to-text line for a Rust CLI. Preserve paths, --flags, and :: tokens.\n\n\
Original text:\n{}\n\n\
Raw ASR:\n{}\n\n\
Deterministic confidence: {}\n\n\
Reply with ONLY compact JSON (no markdown) matching this shape:\n\
{{\"corrected_text\":string,\"confidence\":number between 0 and 1,\"changes\":[{{\"before\":string,\"after\":string}}],\"keep_original\":boolean}}\n",
        session.text, session.raw_text, session.confidence
    );
    let system_prompt = "You correct ASR transcripts. Output JSON only; booleans lowercase; confidence between 0 and 1 inclusive.";

    let resolution_template = crate::llm_bridge::McpChatModelResolution {
        complexity: 1,
        ..Default::default()
    };
    let pref = match crate::sync_poison::poison_rw_read(
        state.mcp_chat_model_override.read(),
        "mcp_chat_model_override",
    ) {
        Ok(g) => g.clone(),
        Err(e) => {
            tracing::warn!(target: "vox_mcp_oratio", stage = "llm_pass", "mcp_chat_model_override: {e}");
            return json!({
                "applied": false,
                "reason": "model_pref_lock_failed",
                "error": format!("{e}"),
            });
        }
    };
    let (model, free_only) = match crate::tools::chat_model_resolve::resolve_chat_llm_model(
        state,
        &user_prompt,
        resolution_template.clone(),
        Some(session.session_id.as_str()),
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(target: "vox_mcp_oratio", stage = "llm_pass", "llm model resolve: {e}");
            return json!({
                "applied": false,
                "reason": "model_resolve_failed",
                "error": format!("{e}"),
            });
        }
    };

    let routing = McpInferRouting {
        user_prompt: &user_prompt,
        sticky_model_pref: pref.as_deref(),
        resolution_template,
        free_only,
        allow_cloud_ollama_fallback: true,
        user_id: Some(session.session_id.as_str()),
    };

    let raw = match mcp_infer_completion(
        state,
        model,
        "vox_oratio_listen",
        system_prompt,
        &routing,
        llm_max_output_tokens.max(64),
        0.1,
        true,
    )
    .await
    {
        Ok((t, _, _)) => t,
        Err(e) => {
            tracing::warn!(target: "vox_mcp_oratio", stage = "llm_pass", "llm infer: {e}");
            return json!({
                "applied": false,
                "reason": "infer_failed",
                "error": format!("{e}"),
            });
        }
    };

    let trimmed = strip_json_fence(&raw);
    let parsed: Value = match serde_json::from_str(&trimmed) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(target: "vox_mcp_oratio", stage = "llm_pass", "llm json parse: {e}");
            let preview: String = trimmed.chars().take(512).collect();
            return json!({
                "applied": false,
                "reason": "parse_failed",
                "error": format!("{e}"),
                "raw_preview": preview,
            });
        }
    };

    if !llm_changes_well_formed(&parsed) {
        return json!({
            "applied": false,
            "reason": "changes_schema_invalid",
        });
    }
    if !llm_confidence_field_ok(&parsed) {
        return json!({
            "applied": false,
            "reason": "confidence_out_of_range",
        });
    }

    if parsed
        .get("keep_original")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return json!({"applied": false, "keep_original": true, "llm": parsed});
    }

    let corrected = match parsed.get("corrected_text").and_then(|v| v.as_str()) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => {
            return json!({"applied": false, "reason": "missing_corrected_text", "llm": parsed});
        }
    };

    if !validate_llm_surface(&session.text, &corrected) {
        return json!({
            "applied": false,
            "reason": "surface_validation_failed",
        });
    }
    if !protected_tokens_preserved(&session.text, &corrected) {
        return json!({
            "applied": false,
            "reason": "protected_token_not_preserved",
        });
    }

    let conf = parsed
        .get("confidence")
        .and_then(|v| v.as_f64())
        .map(|x| x as f32)
        .unwrap_or(session.confidence);

    json!({
        "applied": true,
        "llm": parsed,
        "effective_confidence": conf,
        "corrected_text": corrected,
    })
}

/// `vox_oratio_listen`: session envelope, routing, optional LLM polish (gated + validated).
pub async fn listen(state: &ServerState, args: Value) -> anyhow::Result<String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing string field `path`"))?;
    let full = resolve_audio_path(state, path);
    let rtc = vox_oratio::OratioRuntimeConfig::resolve();

    let timeout_ms = args
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(rtc.session_timing.capture_timeout_ms);
    let max_duration_ms = args
        .get("max_duration_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(rtc.session_timing.max_duration_ms);
    let inference_deadline_ms = args.get("inference_deadline_ms").and_then(|v| v.as_u64());
    let heartbeat_ms = args
        .get("heartbeat_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(rtc.session_timing.heartbeat_ms);

    let language_hint = args
        .get("language_hint")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);
    let debug_parser_payload = args
        .get("debug_parser_payload")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let profile = parse_profile(&args);
    let route_mode = parse_route_mode(&args);
    let llm_refinement = args
        .get("llm_refinement")
        .and_then(|v| v.as_bool())
        .unwrap_or(rtc.llm.llm_refinement_default);
    let llm_min_det_confidence = args
        .get("llm_min_det_confidence")
        .and_then(|v| v.as_f64())
        .map(|x| x as f32)
        .unwrap_or(rtc.llm.llm_min_det_confidence);
    let llm_max_output_tokens = args
        .get("llm_max_output_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(rtc.llm.llm_max_output_tokens);

    let cfg = vox_oratio::OratioSessionConfig {
        timeout_ms,
        max_duration_ms,
        inference_deadline_ms,
        language_hint: language_hint.clone(),
        correction_profile: profile,
        debug_parser_payload,
        heartbeat_ms,
        session_id: args
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
    };

    let full_for_block = full.clone();
    let rtc_block = rtc.clone();
    let mut session = tokio::task::spawn_blocking(move || {
        vox_oratio::transcribe_path_session_with_runtime(&full_for_block, &cfg, &rtc_block)
    })
    .await
    .map_err(|e| anyhow::anyhow!("join: {e}"))??;

    let llm_block = maybe_llm_polish(
        state,
        &session,
        route_mode,
        llm_refinement,
        llm_min_det_confidence,
        llm_max_output_tokens,
        &rtc,
    )
    .await;

    if let Some(applied) = llm_block.get("applied").and_then(|v| v.as_bool()) {
        if applied {
            if let Some(t) = llm_block.get("corrected_text").and_then(|v| v.as_str()) {
                session.refined_text = t.to_string();
                session.text = t.to_string();
            }
            if let Some(c) = llm_block
                .get("effective_confidence")
                .and_then(|v| v.as_f64())
                .map(|x| x as f32)
            {
                session.confidence = c;
            }
        }
    }

    let route = vox_oratio::route_transcript_with_options(
        route_mode,
        &session.session_id,
        &session.text,
        session.confidence,
        &rtc,
    );
    if let Some(asr_path) = args.get("emit_asr_refine_path").and_then(|v| v.as_str()) {
        let out = resolve_audio_path(state, asr_path);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&out)?;
        writeln!(
            f,
            "{}",
            serde_json::to_string(&json!({
                "noisy_text": session.raw_text,
                "corrected_text": session.refined_text,
            }))?
        )?;
    }

    let correlation_id = vox_oratio::trace::new_correlation_id();
    let intent_envelope =
        if matches!(route.mode, vox_oratio::RouteMode::Tool) && route.action != "none" {
            let intent_confidence = route
                .payload
                .get("intent_confidence")
                .and_then(|v| v.as_f64())
                .map(|x| x as f32)
                .unwrap_or(0.0);
            let env = vox_oratio::build_intent_envelope(
                &route.action,
                &session.text,
                intent_confidence,
                session.confidence,
            );
            let gaps = vox_oratio::missing_slot_ids(&env);
            let slot_hint = vox_oratio::clarification_prompt_for_slots(&env).map(str::to_string);
            Some((env, slot_hint, gaps))
        } else {
            None
        };

    let mut response = json!({
        "correlation_id": correlation_id,
        "session": session,
        "route": route,
        "clarification_recommended": vox_oratio::clarification_recommended(
            session.confidence,
            rtc.routing.tool_route_min_confidence,
        ),
    });
    if let Some((env, slot_hint, gaps)) = intent_envelope {
        response["intent_envelope"] = serde_json::to_value(&env)?;
        response["speech_escalation_recommended"] =
            json!(vox_oratio::speech_escalation_recommended(
                env.intent_confidence,
                env.transcript_confidence
            ));
        if !gaps.is_empty() {
            response["intent_slot_gaps"] = json!(gaps);
        }
        if let Some(h) = slot_hint {
            response["slot_clarification"] = json!(h);
        }
    }
    response["llm_refinement"] = llm_block;
    if debug_parser_payload {
        response["runtime_config"] = vox_oratio::runtime_config_diagnostic_json(&rtc);
    }
    Ok(serde_json::to_string(&response)?)
}

/// `vox_oratio_status`: static line + Candle backend JSON (model env defaults).
pub fn status() -> String {
    serde_json::to_string(&json!({
        "summary": vox_oratio::transcript_status(),
        "candle": vox_oratio::candle_backend_status_json(),
        "runtime": vox_oratio::runtime_config_diagnostic_json(&vox_oratio::OratioRuntimeConfig::resolve()),
    }))
    .unwrap_or_else(|_| "{\"error\":\"serialize\"}".to_string())
}
