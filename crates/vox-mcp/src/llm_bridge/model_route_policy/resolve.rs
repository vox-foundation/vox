//! Model registry resolution and telemetry labels.

use vox_orchestrator::Orchestrator;
use vox_orchestrator::models::{ModelSpec, ProviderType};
use vox_orchestrator::usage::{RemainingBudget, UsageTracker};
use vox_runtime::model_resolution::{ChatRouteBackend, backend_telemetry_labels};

use super::super::MCP_GLOBAL_LLM_AGENT;
use super::policy::{apply_gemini_policy, enforce_free_tier_if_needed, mcp_local_model_allowed};
use super::types::McpChatModelResolution;
use crate::server::ServerState;
use vox_orchestrator::models::scoring::auto_score_model;

/// Token fill ratio for the global MCP LLM budget agent (`AgentId(0)`), if tracked.
#[must_use]
pub fn mcp_global_llm_context_fill_ratio(orch: &Orchestrator) -> Option<f32> {
    crate::sync_lock::rw_read(&*orch.budget_handle())
        .check_budget(MCP_GLOBAL_LLM_AGENT)
        .map(|b| b.tokens_used as f32 / b.effective_max_tokens().max(1) as f32)
}

/// Resolve a concrete [`ModelSpec`] synchronously from sticky override + orchestrator registry.
pub fn resolve_mcp_chat_model_sync(
    orch: &Orchestrator,
    _user_prompt: &str,
    pref: Option<&str>,
    res: McpChatModelResolution,
    availability_hint: Option<&[RemainingBudget]>,
) -> Result<(ModelSpec, bool), String> {
    if crate::llm_bridge::infer_test_stub::infer_stub_env_active() {
        return Ok((
            crate::llm_bridge::infer_test_stub::stub_plan_model_spec(),
            true,
        ));
    }

    let models_handle = orch.models_handle();
    let registry = crate::sync_lock::rw_read(&*models_handle);
    let preference = {
        let config_handle = orch.config_handle();
        crate::sync_lock::rw_read(&*config_handle).cost_preference
    };

    if let Some(raw) = pref {
        let id = raw.trim();
        if !id.is_empty() {
            if let Some(m) = registry.get(id) {
                if !mcp_local_model_allowed(&m) {
                    return Err(
                        "Sticky MCP model uses Ollama but VOX_INFERENCE_PROFILE does not allow local Ollama HTTP; use desktop_ollama or lan_gateway, pick a cloud model, or clear the override (see docs/src/architecture/mobile-edge-ai-ssot.md).".into(),
                    );
                }
                let m = enforce_free_tier_if_needed(&registry, &res, m.clone())?;
                return Ok((m.clone(), m.is_free));
            }
        }
    }

    let task = res.task_category;

    if res.free_tier_latency_critical {
        if let Some(m) = registry.best_free_for_with_filter(task, mcp_local_model_allowed) {
            let m = enforce_free_tier_if_needed(&registry, &res, m.clone())?;
            return Ok((m.clone(), m.is_free));
        }
        if res.allow_cheapest_fallback {
            if let Some(m) = registry.cheapest_free_with_filter(mcp_local_model_allowed) {
                let m = enforce_free_tier_if_needed(&registry, &res, m.clone())?;
                return Ok((m.clone(), m.is_free));
            }
        }
    }

    if let Some(m) = registry
        .list_models()
        .into_iter()
        .filter(mcp_local_model_allowed)
        .max_by(|a, b| {
            let score_a = auto_score_model(
                a,
                res.complexity,
                res.free_tier_latency_critical,
                res.context_fill_ratio,
                preference,
                availability_hint,
            );
            let score_b = auto_score_model(
                b,
                res.complexity,
                res.free_tier_latency_critical,
                res.context_fill_ratio,
                preference,
                availability_hint,
            );
            score_a.total_cmp(&score_b)
        })
    {
        let m = apply_gemini_policy(&registry, m, false);
        let m = enforce_free_tier_if_needed(&registry, &res, m)?;
        return Ok((m.clone(), m.is_free));
    }

    if res.allow_cheapest_fallback {
        if let Some(m) = registry.cheapest_free_with_filter(mcp_local_model_allowed) {
            let m = enforce_free_tier_if_needed(&registry, &res, m.clone())?;
            return Ok((m.clone(), m.is_free));
        }
        if let Some(m) = registry.cheapest_with_filter(mcp_local_model_allowed) {
            let m = enforce_free_tier_if_needed(&registry, &res, m.clone())?;
            return Ok((m.clone(), m.is_free));
        }
    }

    Err(
        "No LLM model available — set OPENROUTER_API_KEY or GEMINI_API_KEY, install Ollama when \
         VOX_INFERENCE_PROFILE allows local/LAN Ollama (desktop_ollama or lan_gateway), \
         or add models.toml under the Vox config directory."
            .into(),
    )
}

/// Async resolver that includes per-user provider availability when DB is attached.
pub async fn resolve_mcp_chat_model(
    state: &ServerState,
    user_prompt: &str,
    pref: Option<&str>,
    res: McpChatModelResolution,
    user_id: Option<&str>,
) -> Result<(ModelSpec, bool), String> {
    let availability = if let Some(db) = state.db.as_ref() {
        let tracker = if let Some(uid) = user_id {
            UsageTracker::with_user(db.as_ref(), uid)
        } else {
            UsageTracker::new_ref(db.as_ref())
        };
        tracker.remaining_all().await.ok()
    } else {
        None
    };
    resolve_mcp_chat_model_sync(
        &state.orchestrator,
        user_prompt,
        pref,
        res,
        availability.as_deref(),
    )
}

/// Telemetry `(provider_family, route_choice)` — delegates to [`vox_runtime::model_resolution::backend_telemetry_labels`]
/// so MCP and runtime chat lanes share one string SSOT.
#[must_use]
pub fn mcp_provider_telemetry_labels(provider: &ProviderType) -> (&'static str, &'static str) {
    backend_telemetry_labels(match provider {
        ProviderType::GoogleDirect => ChatRouteBackend::GeminiDirect,
        ProviderType::OpenRouter => ChatRouteBackend::OpenRouter,
        ProviderType::Ollama => ChatRouteBackend::Ollama,
        ProviderType::PopuliMesh => ChatRouteBackend::PopuliMesh,
        ProviderType::Groq
        | ProviderType::Cerebras
        | ProviderType::Mistral
        | ProviderType::DeepSeek
        | ProviderType::SambaNova
        | ProviderType::Anthropic
        | ProviderType::Custom(_) => ChatRouteBackend::CascadeFallback,
    })
}
