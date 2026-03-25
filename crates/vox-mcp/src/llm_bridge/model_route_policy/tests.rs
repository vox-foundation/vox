use std::sync::Mutex;
use vox_orchestrator::Orchestrator;
use vox_orchestrator::config::{CostPreference, OrchestratorConfig};
use vox_orchestrator::models::{ModelRegistry, ModelSpec, ProviderType};

use super::{
    McpChatModelResolution, mcp_global_llm_context_fill_ratio, mcp_provider_telemetry_labels,
    resolve_mcp_chat_model_sync,
};

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
        mcp_provider_telemetry_labels(&ProviderType::OpenRouter)
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
        mcp_provider_telemetry_labels(&ProviderType::Ollama)
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
