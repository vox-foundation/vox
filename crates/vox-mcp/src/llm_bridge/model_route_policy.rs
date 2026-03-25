//! Pure model resolution for MCP chat: registry lookup, free-tier enforcement, context signals.

use vox_config::{
    AutoRoutingPriority, GeminiRoutePolicy, gemini_route_targets_from_env,
    inference_profile_allows_local_ollama_http, inference_profile_from_env,
};
use vox_orchestrator::Orchestrator;
use vox_orchestrator::config::CostPreference;
use vox_orchestrator::models::{ModelRegistry, ModelSpec, ProviderType};
use vox_orchestrator::types::TaskCategory;
use vox_orchestrator::usage::{RemainingBudget, UsageTracker};

use super::MCP_GLOBAL_LLM_AGENT;
use crate::server::ServerState;

/// Heuristics for [`resolve_mcp_chat_model_sync`].
#[derive(Debug, Clone)]
pub struct McpChatModelResolution {
    /// When resolution fails, fall back to the cheapest free / cheapest model.
    pub allow_cheapest_fallback: bool,
    /// Task complexity hint (1–10) for registry routing.
    pub complexity: u8,
    /// Prefer a free model with large context (ghost text / latency-sensitive paths).
    pub free_tier_latency_critical: bool,
    /// Hint that the workload is fill-in-the-middle (affects free-tier preference).
    pub free_tier_fill_in_middle: bool,
    /// When set, never return a paid model (sticky override included); errors if no free model.
    pub enforce_free_tier_only: bool,
    /// `tokens_used / effective_max` for the MCP LLM budget agent when known (raises routing complexity).
    pub context_fill_ratio: Option<f32>,
}

impl Default for McpChatModelResolution {
    fn default() -> Self {
        Self {
            allow_cheapest_fallback: false,
            complexity: 5,
            free_tier_latency_critical: false,
            free_tier_fill_in_middle: false,
            enforce_free_tier_only: false,
            context_fill_ratio: None,
        }
    }
}

fn enforce_free_tier_if_needed(
    registry: &ModelRegistry,
    res: &McpChatModelResolution,
    spec: ModelSpec,
) -> Result<ModelSpec, String> {
    if !res.enforce_free_tier_only || spec.is_free {
        return Ok(spec);
    }
    let task = TaskCategory::CodeGen;
    registry
        .best_free_for_with_filter(task, mcp_ollama_model_allowed)
        .or_else(|| registry.cheapest_free_with_filter(mcp_ollama_model_allowed))
        .ok_or_else(|| {
            "No free-tier model available (enforce_free_tier_only) after VOX_INFERENCE_PROFILE rules; clear sticky override, allow desktop_ollama/lan_gateway for Ollama, or add a non-Ollama free model in models.toml".to_string()
        })
}

#[must_use]
fn mcp_ollama_model_allowed(m: &ModelSpec) -> bool {
    !matches!(m.provider_type, ProviderType::Ollama) || inference_profile_allows_local_ollama_http()
}

#[must_use]
fn budget_match(limit_model: &str, model: &str) -> bool {
    limit_model == model
        || limit_model == "*"
        || (limit_model == ":free" && model.ends_with(":free"))
}

#[must_use]
fn model_budget_hint(model: &ModelSpec, hints: Option<&[RemainingBudget]>) -> (u32, bool) {
    let usage = model.llm_usage_key();
    let mut remaining_max = 0u32;
    let mut any_rate_limited = false;
    for b in hints.unwrap_or(&[]) {
        if b.provider == usage.provider && budget_match(&b.model, &usage.model) {
            remaining_max = remaining_max.max(b.remaining);
            any_rate_limited |= b.rate_limited;
        }
    }
    (remaining_max, any_rate_limited)
}

#[must_use]
fn quality_score(m: &ModelSpec) -> f64 {
    let token_component = (m.max_tokens as f64).log10().clamp(1.0, 7.0) / 7.0;
    let paid_component = if m.is_free { 0.35 } else { 0.95 };
    ((token_component * 0.6) + (paid_component * 0.4)).clamp(0.0, 1.0)
}

#[must_use]
fn efficiency_score(m: &ModelSpec) -> f64 {
    let blended = if m.cost_per_1k_input > 0.0 || m.cost_per_1k_output > 0.0 {
        (m.cost_per_1k_input + m.cost_per_1k_output) / 2.0
    } else {
        m.cost_per_1k
    };
    if blended <= 0.0 {
        return 1.0;
    }
    (1.0 / (1.0 + blended * 100.0)).clamp(0.0, 1.0)
}

#[must_use]
fn latency_score(m: &ModelSpec) -> f64 {
    match m.provider_type {
        ProviderType::Ollama => 0.95,
        ProviderType::GoogleDirect => 0.8,
        ProviderType::OpenRouter => 0.7,
        _ => 0.65,
    }
}

#[must_use]
fn mobile_score(m: &ModelSpec) -> f64 {
    match inference_profile_from_env() {
        vox_config::InferenceProfile::MobileLitert | vox_config::InferenceProfile::MobileCoreml => {
            if matches!(m.provider_type, ProviderType::Ollama) {
                0.0
            } else {
                1.0
            }
        }
        _ => 0.7,
    }
}

#[must_use]
fn auto_score_model(
    m: &ModelSpec,
    res: &McpChatModelResolution,
    preference: CostPreference,
    hints: Option<&[RemainingBudget]>,
) -> f64 {
    let mut w = AutoRoutingPriority::from_env();
    if res.complexity >= 8 {
        w.precision = w.precision.saturating_add(10);
    } else if res.complexity <= 3 {
        w.efficiency = w.efficiency.saturating_add(10);
        w.latency = w.latency.saturating_add(5);
    }
    match preference {
        CostPreference::Economy => w.efficiency = w.efficiency.saturating_add(15),
        CostPreference::Performance => w.precision = w.precision.saturating_add(12),
    }

    let (remaining, rate_limited) = model_budget_hint(m, hints);
    if rate_limited {
        return -10_000.0;
    }

    let balance_bias = 1.0_f64 - f64::from(res.context_fill_ratio.unwrap_or(0.0).clamp(0.0, 1.0));
    let availability_score = if remaining == 0 {
        0.35
    } else {
        (f64::from(remaining).log10() / 3.0).clamp(0.4, 1.0)
    };
    let total_w = f64::from(
        u16::from(w.efficiency)
            + u16::from(w.precision)
            + u16::from(w.latency)
            + u16::from(w.availability)
            + u16::from(w.balance)
            + u16::from(w.mobile),
    )
    .max(1.0);
    let score = f64::from(w.efficiency) * efficiency_score(m)
        + f64::from(w.precision) * quality_score(m)
        + f64::from(w.latency) * latency_score(m)
        + f64::from(w.availability) * availability_score
        + f64::from(w.balance) * balance_bias
        + f64::from(w.mobile) * mobile_score(m);
    score / total_w
}

#[must_use]
fn apply_gemini_policy(
    registry: &ModelRegistry,
    chosen: ModelSpec,
    sticky_override: bool,
) -> ModelSpec {
    if sticky_override {
        return chosen;
    }
    let targets = gemini_route_targets_from_env();
    let is_gemini = chosen.id.to_ascii_lowercase().contains("gemini");
    if !is_gemini {
        return chosen;
    }
    match GeminiRoutePolicy::from_env() {
        GeminiRoutePolicy::RegistryDefault => chosen,
        GeminiRoutePolicy::OpenRouterFirst => {
            if !matches!(chosen.provider_type, ProviderType::OpenRouter)
                && vox_config::openrouter_api_key().is_some()
            {
                registry.get(&targets.openrouter_model).unwrap_or(chosen)
            } else {
                chosen
            }
        }
        GeminiRoutePolicy::GoogleDirectOnly => {
            if !matches!(chosen.provider_type, ProviderType::GoogleDirect) {
                registry.get(&targets.google_direct_model).unwrap_or(chosen)
            } else {
                chosen
            }
        }
    }
}

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
                if !mcp_ollama_model_allowed(&m) {
                    return Err(
                        "Sticky MCP model uses Ollama but VOX_INFERENCE_PROFILE does not allow local Ollama HTTP; use desktop_ollama or lan_gateway, pick a cloud model, or clear the override (see docs/src/architecture/mobile-edge-ai-ssot.md).".into(),
                    );
                }
                let m = enforce_free_tier_if_needed(&registry, &res, m.clone())?;
                return Ok((m.clone(), m.is_free));
            }
        }
    }

    let task = TaskCategory::CodeGen;

    if res.free_tier_latency_critical {
        if let Some(m) = registry.best_free_for_with_filter(task, mcp_ollama_model_allowed) {
            let m = enforce_free_tier_if_needed(&registry, &res, m.clone())?;
            return Ok((m.clone(), m.is_free));
        }
        if res.allow_cheapest_fallback {
            if let Some(m) = registry.cheapest_free_with_filter(mcp_ollama_model_allowed) {
                let m = enforce_free_tier_if_needed(&registry, &res, m.clone())?;
                return Ok((m.clone(), m.is_free));
            }
        }
    }

    if let Some(m) =
        registry
            .list_models()
            .into_iter()
            .filter(mcp_ollama_model_allowed)
            .max_by(|a, b| {
                auto_score_model(a, &res, preference, availability_hint)
                    .total_cmp(&auto_score_model(b, &res, preference, availability_hint))
            })
    {
        let m = apply_gemini_policy(&registry, m, false);
        let m = enforce_free_tier_if_needed(&registry, &res, m)?;
        return Ok((m.clone(), m.is_free));
    }

    if res.allow_cheapest_fallback {
        if let Some(m) = registry.cheapest_free_with_filter(mcp_ollama_model_allowed) {
            let m = enforce_free_tier_if_needed(&registry, &res, m.clone())?;
            return Ok((m.clone(), m.is_free));
        }
        if let Some(m) = registry.cheapest_with_filter(mcp_ollama_model_allowed) {
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

/// Telemetry `(provider_family, route_choice)` aligned with [`vox_runtime::model_resolution::route_telemetry_labels`]
/// wherever MCP [`ProviderType`] maps to the same HTTP lane (local Ollama/Mens vs OpenRouter).
#[must_use]
pub fn mcp_provider_telemetry_labels(provider: &ProviderType) -> (&'static str, &'static str) {
    match provider {
        ProviderType::GoogleDirect => ("google", "direct"),
        ProviderType::OpenRouter => ("openrouter", "openrouter"),
        ProviderType::Ollama => ("mens", "populi_local"),
        ProviderType::Groq => ("groq", "groq"),
        ProviderType::Cerebras => ("cerebras", "cerebras"),
        ProviderType::Mistral => ("mistral", "mistral"),
        ProviderType::DeepSeek => ("deepseek", "deepseek"),
        ProviderType::SambaNova => ("sambanova", "sambanova"),
        ProviderType::Custom(_) => ("custom", "custom"),
    }
}

#[cfg(test)]
#[allow(unsafe_code)] // `set_var` / `remove_var` are `unsafe` in Rust 2024; serialized via `INFERENCE_PROFILE_TEST_LOCK`.
mod tests {
    use super::*;
    use std::sync::Mutex;
    use vox_orchestrator::Orchestrator;
    use vox_orchestrator::config::{CostPreference, OrchestratorConfig};
    use vox_orchestrator::models::{ModelRegistry, ModelSpec, ProviderType};

    static INFERENCE_PROFILE_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn tiny_registry_with_free_and_paid() -> ModelRegistry {
        let mut r = ModelRegistry::default();
        r.register(ModelSpec {
            id: "free-model".into(),
            canonical_slug: "test/free-model".into(),
            provider: "test".into(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 1000,
            cost_per_1k: 0.0,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            is_free: true,
            strengths: vec!["codegen".into()],
            capabilities: Default::default(),
            supported_parameters: vec![],
        });
        r.register(ModelSpec {
            id: "paid-model".into(),
            canonical_slug: "test/paid-model".into(),
            provider: "test".into(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 1000,
            cost_per_1k: 0.01,
            cost_per_1k_input: 0.01,
            cost_per_1k_output: 0.01,
            is_free: false,
            strengths: vec!["codegen".into()],
            capabilities: Default::default(),
            supported_parameters: vec![],
        });
        r
    }

    #[test]
    fn mcp_global_llm_context_fill_ratio_none_without_budget() {
        let mut config = OrchestratorConfig::for_testing();
        config.cost_preference = CostPreference::Performance;
        let orch = Orchestrator::new(config);
        assert!(mcp_global_llm_context_fill_ratio(&orch).is_none());
    }

    #[test]
    fn enforce_free_tier_only_swaps_paid_best_for() {
        let mut config = OrchestratorConfig::for_testing();
        config.cost_preference = CostPreference::Performance;
        let mut orch = Orchestrator::new(config);
        *crate::sync_lock::rw_write(&*orch.models_handle()) = tiny_registry_with_free_and_paid();

        let resolved = resolve_mcp_chat_model_sync(
            &orch,
            "",
            None,
            McpChatModelResolution {
                complexity: 8,
                allow_cheapest_fallback: true,
                enforce_free_tier_only: true,
                ..Default::default()
            },
            None,
        )
        .expect("resolve");
        assert!(resolved.0.is_free);
        assert_eq!(resolved.0.id, "free-model");
    }

    fn registry_ollama_only() -> ModelRegistry {
        let mut r = ModelRegistry::default();
        r.register(ModelSpec {
            id: "llama-local".into(),
            canonical_slug: "local/llama".into(),
            provider: "ollama".into(),
            provider_type: ProviderType::Ollama,
            max_tokens: 8192,
            cost_per_1k: 0.0,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            is_free: true,
            strengths: vec!["codegen".into()],
            capabilities: Default::default(),
            supported_parameters: vec![],
        });
        r
    }

    fn registry_paid_plus_ollama_free() -> ModelRegistry {
        let mut r = registry_ollama_only();
        r.register(ModelSpec {
            id: "paid-model".into(),
            canonical_slug: "test/paid-model".into(),
            provider: "test".into(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 1000,
            cost_per_1k: 0.01,
            cost_per_1k_input: 0.01,
            cost_per_1k_output: 0.01,
            is_free: false,
            strengths: vec!["codegen".into()],
            capabilities: Default::default(),
            supported_parameters: vec![],
        });
        r
    }

    #[test]
    fn sticky_ollama_rejected_when_inference_profile_disallows() {
        let _g = INFERENCE_PROFILE_TEST_LOCK.lock().expect("lock");
        // SAFETY: serialized with `INFERENCE_PROFILE_TEST_LOCK`; no concurrent env access in tests.
        unsafe {
            std::env::set_var("VOX_INFERENCE_PROFILE", "cloud_openai_compatible");
        }
        let mut config = OrchestratorConfig::for_testing();
        config.cost_preference = CostPreference::Performance;
        let mut orch = Orchestrator::new(config);
        *crate::sync_lock::rw_write(&*orch.models_handle()) = registry_ollama_only();

        let err = resolve_mcp_chat_model_sync(
            &orch,
            "",
            Some("llama-local"),
            McpChatModelResolution {
                complexity: 5,
                allow_cheapest_fallback: true,
                ..Default::default()
            },
            None,
        )
        .expect_err("sticky ollama must fail");
        assert!(
            err.contains("VOX_INFERENCE_PROFILE"),
            "expected profile hint: {err}"
        );
        unsafe {
            std::env::remove_var("VOX_INFERENCE_PROFILE");
        }
    }

    #[test]
    fn mcp_openrouter_label_matches_runtime_route_telemetry() {
        use vox_runtime::model_resolution::{ChatProviderRouteKind, route_telemetry_labels};
        let route = ChatProviderRouteKind::OpenRouter {
            model: "openai/gpt-4o".into(),
        };
        assert_eq!(
            route_telemetry_labels(&route),
            super::mcp_provider_telemetry_labels(&ProviderType::OpenRouter)
        );
    }

    #[test]
    fn mcp_ollama_label_matches_runtime_populi_local_telemetry() {
        use vox_runtime::model_resolution::{ChatProviderRouteKind, route_telemetry_labels};
        let route = ChatProviderRouteKind::PopuliLocal {
            base_url: "http://127.0.0.1:11434".into(),
            model: "llama3.2".into(),
        };
        assert_eq!(
            route_telemetry_labels(&route),
            super::mcp_provider_telemetry_labels(&ProviderType::Ollama)
        );
    }

    #[test]
    fn enforce_free_tier_only_fails_when_only_ollama_free_under_cloud_profile() {
        let _g = INFERENCE_PROFILE_TEST_LOCK.lock().expect("lock");
        unsafe {
            std::env::set_var("VOX_INFERENCE_PROFILE", "cloud_openai_compatible");
        }
        let mut config = OrchestratorConfig::for_testing();
        config.cost_preference = CostPreference::Performance;
        let mut orch = Orchestrator::new(config);
        *crate::sync_lock::rw_write(&*orch.models_handle()) = registry_paid_plus_ollama_free();

        let err = resolve_mcp_chat_model_sync(
            &orch,
            "",
            Some("paid-model"),
            McpChatModelResolution {
                complexity: 8,
                allow_cheapest_fallback: true,
                enforce_free_tier_only: true,
                ..Default::default()
            },
            None,
        )
        .expect_err("no allowed free model");
        assert!(
            err.contains("VOX_INFERENCE_PROFILE") || err.contains("enforce_free_tier_only"),
            "expected profile or enforce hint: {err}"
        );
        unsafe {
            std::env::remove_var("VOX_INFERENCE_PROFILE");
        }
    }
}
