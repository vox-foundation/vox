//! Model registry resolution and telemetry labels.

use crate::Orchestrator;
use crate::models::{ModelSpec, ProviderType};
use crate::route_policy::route_policy_allows_model;
use crate::types::TaskCategory;
use crate::usage::{RemainingBudget, UsageTracker};
use vox_runtime::model_resolution::{ChatRouteBackend, backend_telemetry_labels};

use super::super::MCP_GLOBAL_LLM_AGENT;
use super::policy::{apply_gemini_policy, enforce_free_tier_if_needed, mcp_local_model_allowed};
use super::types::McpChatModelResolution;
use crate::mcp_tools::server_state::ServerState;
use crate::models::scoring::auto_score_model;

fn provider_allowed_by_route_policy(model: &ModelSpec) -> bool {
    route_policy_allows_model(model)
}

/// Task categories where the Vox-trained local model is preferred when available.
const VOX_LOCAL_PREFERRED_TASKS: &[TaskCategory] = &[
    TaskCategory::CodeGen,
    TaskCategory::Testing,
    TaskCategory::Parsing,
    TaskCategory::TypeChecking,
];

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
    if crate::mcp_tools::llm_bridge::infer_test_stub::infer_stub_env_active() {
        return Ok((
            crate::mcp_tools::llm_bridge::infer_test_stub::stub_plan_model_spec(),
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
                if !provider_allowed_by_route_policy(&m) {
                    return Err("Sticky MCP model denied by VOX_ROUTE_* capability policy.".into());
                }
                let m = enforce_free_tier_if_needed(&registry, &res, m.clone())?;
                return Ok((m.clone(), m.is_free));
            }
        }
    }

    let task = res.task_category;

    if res.free_tier_latency_critical {
        if let Some(m) = registry.best_free_for_with_filter(task, |m| {
            mcp_local_model_allowed(m) && provider_allowed_by_route_policy(m)
        }) {
            let m = enforce_free_tier_if_needed(&registry, &res, m.clone())?;
            return Ok((m.clone(), m.is_free));
        }
        if res.allow_cheapest_fallback {
            if let Some(m) = registry.cheapest_free_with_filter(|m| {
                mcp_local_model_allowed(m) && provider_allowed_by_route_policy(m)
            }) {
                let m = enforce_free_tier_if_needed(&registry, &res, m.clone())?;
                return Ok((m.clone(), m.is_free));
            }
        }
    }

    // Prefer the Vox-trained local model for code-oriented tasks when available and permitted.
    if VOX_LOCAL_PREFERRED_TASKS.contains(&task) && !res.enforce_free_tier_only {
        if let Some(m) = registry
            .list_models()
            .into_iter()
            .filter(|m| matches!(m.provider_type, ProviderType::VoxLocal))
            .filter(|m| mcp_local_model_allowed(m))
            .filter(provider_allowed_by_route_policy)
            .max_by(|a, b| a.max_tokens.cmp(&b.max_tokens))
        {
            return Ok((m.clone(), m.is_free));
        }
    }

    if let Some(m) = registry
        .list_models()
        .into_iter()
        .filter(mcp_local_model_allowed)
        .filter(provider_allowed_by_route_policy)
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
        if let Some(m) = registry.cheapest_free_with_filter(|m| {
            mcp_local_model_allowed(m) && provider_allowed_by_route_policy(m)
        }) {
            let m = enforce_free_tier_if_needed(&registry, &res, m.clone())?;
            return Ok((m.clone(), m.is_free));
        }
        if let Some(m) = registry.cheapest_with_filter(|m| {
            mcp_local_model_allowed(m) && provider_allowed_by_route_policy(m)
        }) {
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
    backend_telemetry_labels(match *provider {
        ProviderType::GoogleDirect => ChatRouteBackend::GeminiDirect,
        ProviderType::OpenRouter => ChatRouteBackend::OpenRouter,
        ProviderType::Ollama => ChatRouteBackend::Ollama,
        ProviderType::PopuliMesh => ChatRouteBackend::PopuliMesh,
        ProviderType::VoxLocal => ChatRouteBackend::VoxLocal,
        ProviderType::Groq
        | ProviderType::Cerebras
        | ProviderType::Mistral
        | ProviderType::DeepSeek
        | ProviderType::SambaNova
        | ProviderType::Anthropic
        | ProviderType::HuggingFaceRouter
        | ProviderType::Custom(_) => ChatRouteBackend::CascadeFallback,
    })
}
