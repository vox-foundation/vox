//! HTTP inference loop: budget gate, provider dispatch, Ollama fallback, usage recording.

use vox_config::inference_profile_allows_local_ollama_http;
use vox_orchestrator::models::{ModelSpec, ProviderType};
use vox_orchestrator::usage::UsageTracker;
use vox_orchestrator::{AgentEventKind, BudgetGate, Gate, GateResult};

use crate::server::ServerState;

use super::MCP_GLOBAL_LLM_AGENT;
use super::error::HttpInferError;
use super::limits::HTTP_MAX_OUTPUT_TOKENS_CAP;
use super::model_route_policy::{McpChatModelResolution, resolve_mcp_chat_model_sync};
use super::providers::{http_gemini, http_ollama, http_openai_compatible, probe_ollama_tags};

/// Routing context for [`mcp_infer_completion`] (sticky override, free-tier policy, Ollama fallback).
#[derive(Clone)]
pub struct McpInferRouting<'a> {
    /// User text used when re-resolving under `enforce_free_tier_only`.
    pub user_prompt: &'a str,
    /// Sticky MCP model id override (same as registry resolve).
    pub sticky_model_pref: Option<&'a str>,
    /// Template merged with `enforce_free_tier_only` on mismatch; `context_fill_ratio` should match resolve.
    pub resolution_template: McpChatModelResolution,
    /// Resolver marked this path as free-tier (`ModelSpec.is_free` should match; enforced at infer).
    pub free_only: bool,
    /// When cloud gate denies (daily cap, in-memory budget) or HTTP fails, try local Ollama.
    /// Effective only if **`VOX_INFERENCE_PROFILE`** allows local Ollama HTTP (`desktop_ollama` or `lan_gateway`).
    pub allow_cloud_ollama_fallback: bool,
}

fn should_emit_llm_cost_events(state: &ServerState) -> bool {
    match std::env::var("VOX_MCP_LLM_COST_EVENTS").ok() {
        Some(v) => {
            let v = v.trim();
            if v == "0" || v.eq_ignore_ascii_case("false") {
                return false;
            }
            if v == "1" || v.eq_ignore_ascii_case("true") {
                return true;
            }
            state.db.is_none()
        }
        None => state.db.is_none(),
    }
}

async fn http_infer_model(
    client: &reqwest::Client,
    model: &ModelSpec,
    system_prompt: &str,
    user_prompt: &str,
    max_t: u64,
    temperature: f32,
    json_mode: bool,
) -> Result<(String, u32, u32), HttpInferError> {
    match model.provider_type {
        ProviderType::GoogleDirect => {
            let key = std::env::var("GEMINI_API_KEY").map_err(|_| HttpInferError {
                status: 0,
                message: "GEMINI_API_KEY is not set (required for Google-direct models)".into(),
            })?;
            http_gemini(
                client,
                &model.id,
                &key,
                system_prompt,
                user_prompt,
                max_t,
                temperature,
                json_mode,
            )
            .await
        }
        ProviderType::Ollama => {
            probe_ollama_tags(client).await?;
            http_ollama(
                client,
                &model.id,
                system_prompt,
                user_prompt,
                max_t,
                temperature,
                json_mode,
            )
            .await
        }
        ProviderType::OpenRouter => {
            let key = vox_config::inference::openrouter_api_key().unwrap_or_default();
            if key.is_empty() {
                return Err(HttpInferError {
                    status: 0,
                    message: "OPENROUTER_API_KEY is not set (required for OpenRouter models)"
                        .into(),
                });
            }
            http_openai_compatible(
                client,
                vox_config::inference::OPENROUTER_CHAT_COMPLETIONS_URL,
                &key,
                &model.id,
                system_prompt,
                user_prompt,
                max_t,
                temperature,
                json_mode,
            )
            .await
        }
        ProviderType::Groq
        | ProviderType::Cerebras
        | ProviderType::Mistral
        | ProviderType::DeepSeek
        | ProviderType::SambaNova
        | ProviderType::Custom(_) => Err(HttpInferError {
            status: 0,
            message: format!("Provider {:?} not yet fully supported via MCP infer bridge", model.provider_type),
        }),
    }
}

/// Prefer larger context, then stable id (registry list order is arbitrary).
async fn best_ollama_model(state: &ServerState) -> Option<ModelSpec> {
    if !inference_profile_allows_local_ollama_http() {
        return None;
    }
    let orch = &state.orchestrator;
    let mut v: Vec<ModelSpec> = crate::sync_lock::rw_read(&*orch.models_handle())
        .list_models()
        .into_iter()
        .filter(|m| matches!(m.provider_type, ProviderType::Ollama))
        .collect();
    v.sort_by(|a, b| {
        b.max_tokens
            .cmp(&a.max_tokens)
            .then_with(|| a.id.cmp(&b.id))
    });
    v.into_iter().next()
}

/// Dispatch a chat completion for MCP tools (inline edit, ghost text, etc.).
pub async fn mcp_infer_completion(
    state: &ServerState,
    mut model: ModelSpec,
    _tool: &str,
    system_prompt: &str,
    routing: &McpInferRouting<'_>,
    max_tokens: u64,
    temperature: f32,
    json_mode: bool,
) -> Result<(String, String, u64), String> {
    let max_t = super::clamp_http_max_output_tokens(max_tokens);
    let client = &state.http_client;
    let allow_ollama_fallback =
        routing.allow_cloud_ollama_fallback && inference_profile_allows_local_ollama_http();

    let mut first_pass = true;

    loop {
        if first_pass {
            first_pass = false;
            if routing.free_only && !model.is_free {
                let orch = &state.orchestrator;
                let mut res = routing.resolution_template.clone();
                res.enforce_free_tier_only = true;
                match resolve_mcp_chat_model_sync(
                    &*orch,
                    routing.user_prompt,
                    routing.sticky_model_pref,
                    res,
                ) {
                    Ok((m, _)) => model = m,
                    Err(e) => return Err(e),
                }
            } else if routing.free_only != model.is_free {
                tracing::debug!(
                    target: "vox.mcp.llm",
                    free_only = routing.free_only,
                    model_is_free = model.is_free,
                    model_id = %model.id,
                    "mcp_infer_completion: free_only flag disagrees with ModelSpec.is_free"
                );
            }
        }

        let usage = model.llm_usage_key();

        if let Some(db) = state.db.as_ref() {
            let tracker = UsageTracker::new_ref(db.as_ref());
            let gate = BudgetGate::new(state.budget_manager.as_ref(), &tracker);
            match gate.allow(MCP_GLOBAL_LLM_AGENT, &usage, 0).await {
                GateResult::Allowed => {}
                GateResult::BudgetExceeded { message } => {
                    if allow_ollama_fallback && !matches!(model.provider_type, ProviderType::Ollama)
                    {
                        if let Some(fb) = best_ollama_model(state).await {
                            model = fb;
                            continue;
                        }
                    }
                    return Err(message);
                }
                GateResult::RateLimited { .. } => {
                    if allow_ollama_fallback && !matches!(model.provider_type, ProviderType::Ollama)
                    {
                        if let Some(fb) = best_ollama_model(state).await {
                            model = fb;
                            continue;
                        }
                    }
                    return Err(
                        if allow_ollama_fallback {
                            "LLM daily quota or rate limit active for this provider; try a local Ollama model or wait."
                        } else {
                            "LLM daily quota or rate limit active for this provider; configure cloud keys, set VOX_INFERENCE_PROFILE=desktop_ollama or lan_gateway to allow Ollama fallback, or wait."
                        }
                        .into(),
                    );
                }
                GateResult::AttentionExhausted { message, .. } => {
                    return Err(message);
                }
            }
        }

        let chatml_collapsed: Option<String> = if state.orchestrator_config.chatml_strict {
            Some(format!(
                "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
                vox_config::sanitize_chatml(system_prompt),
                vox_config::sanitize_chatml(routing.user_prompt)
            ))
        } else {
            None
        };
        let (final_system, final_user): (&str, &str) = if let Some(ref collapsed) = chatml_collapsed {
            ("", collapsed.as_str())
        } else {
            (system_prompt, routing.user_prompt)
        };

        let infer_result = http_infer_model(
            client,
            &model,
            final_system,
            final_user,
            max_t,
            temperature,
            json_mode,
        )
        .await;

        match infer_result {
            Ok((text, pt, ct)) => {
                let total_tok = (pt + ct) as u64;
                let cost_usd = (total_tok as f64 / 1000.0) * model.cost_per_1k;
                if let Some(db) = state.db.as_ref() {
                    let tracker = UsageTracker::new_ref(db.as_ref());
                    let gate = BudgetGate::new(state.budget_manager.as_ref(), &tracker);
                    gate.record_usage(MCP_GLOBAL_LLM_AGENT, &usage, pt as u64, ct as u64, cost_usd)
                        .await;
                }

                if should_emit_llm_cost_events(state) {
                    let orch = &state.orchestrator;
                    orch.event_bus().emit(AgentEventKind::CostIncurred {
                        agent_id: MCP_GLOBAL_LLM_AGENT,
                        provider: usage.provider.clone(),
                        model: model.id.clone(),
                        input_tokens: pt,
                        output_tokens: ct,
                        cost_usd,
                        temporal_context: None,
                    });
                }

                return Ok((text, model.id, total_tok));
            }
            Err(e) => {
                if e.status == 429 {
                    if let Some(db) = state.db.as_ref() {
                        let tracker = UsageTracker::new_ref(db.as_ref());
                        let _ = tracker
                            .mark_rate_limited(&usage.provider, &usage.model)
                            .await;
                    }
                }
                if allow_ollama_fallback && !matches!(model.provider_type, ProviderType::Ollama) {
                    if let Some(fb) = best_ollama_model(state).await {
                        model = fb;
                        continue;
                    }
                }
                return Err(e.to_string());
            }
        }
    }
}

/// High-level chat used by `vox_chat_message`.
pub async fn call_llm(
    state: &ServerState,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<(String, String, u64), String> {
    let pref = state.mcp_chat_model_override.read().unwrap().clone();
    let (model, free_only, resolution_template) = {
        let orch = &state.orchestrator;
        let context_fill_ratio =
            super::model_route_policy::mcp_global_llm_context_fill_ratio(&*orch);
        let resolution_template = McpChatModelResolution {
            allow_cheapest_fallback: true,
            context_fill_ratio,
            ..Default::default()
        };
        let (model, free_only) = resolve_mcp_chat_model_sync(
            &*orch,
            user_prompt,
            pref.as_deref(),
            resolution_template.clone(),
        )?;
        (model, free_only, resolution_template)
    };

    let max_tokens = model.max_tokens.min(HTTP_MAX_OUTPUT_TOKENS_CAP).max(1);
    let routing = McpInferRouting {
        user_prompt,
        sticky_model_pref: pref.as_deref(),
        resolution_template,
        free_only,
        allow_cloud_ollama_fallback: true,
    };
    mcp_infer_completion(
        state,
        model,
        "mcp_chat",
        system_prompt,
        &routing,
        max_tokens,
        0.7,
        false,
    )
    .await
}
