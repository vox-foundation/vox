//! Resolves a primary, multi-provider-aware LLM candidate for a research
//! stage via the full key-gated `vox_orchestrator::models` selector,
//! bridging the `vox-actor-runtime` <-> `vox-orchestrator` dependency gap
//! (the cascade builders in `vox_actor_runtime::llm::cascade` cannot
//! depend on `vox-orchestrator` directly; this crate already depends on
//! both, so the bridging happens here).

use std::sync::OnceLock;

use vox_actor_runtime::llm::LlmConfig;
use vox_orchestrator::models::{
    ModelRegistry, ModelSelectionRequest, SelectionIntent, decide, llm_config_for_spec,
    task_category_strength,
};

static SHARED_REGISTRY: OnceLock<ModelRegistry> = OnceLock::new();

/// Returns a process-wide shared `ModelRegistry`, loaded from disk once on
/// first use. Prefer this over `ModelRegistry::from_cache()` in hot paths
/// (e.g. the claim-verifier's per-sample resampling loop) so the registry
/// isn't re-read/re-parsed from disk on every call within one research run.
fn shared_registry() -> &'static ModelRegistry {
    SHARED_REGISTRY.get_or_init(ModelRegistry::from_cache)
}

/// Resolves the winning `ModelSpec` for `intent` through the key-gated
/// `decide()` path and converts it to a dispatchable `LlmConfig`. Returns
/// `None` if no candidate clears selection (e.g. no keys configured for
/// any eligible provider) — callers should fall back to
/// `cascade_for_research_stage`'s local+OpenRouter lanes in that case,
/// never treat `None` as a hard error.
pub fn primary_candidate_for_intent(intent: SelectionIntent) -> Option<LlmConfig> {
    let task_type = intent.task;
    if let Some(forced) = vox_config::inference::forced_model() {
        // Tag exactly as `llm_config_for_spec` would for the unforced path,
        // so `chat_stage`'s dedup (which keeps this, the first/primary
        // entry) doesn't silently drop telemetry attribution relative to
        // the cascade's copy of the same pinned model.
        let mut cfg = LlmConfig::openrouter(forced);
        cfg.telemetry_task_category = Some(task_type.to_string());
        cfg.telemetry_strength_tag = Some(task_category_strength(task_type).to_string());
        return Some(cfg);
    }
    let registry = shared_registry();
    let request = ModelSelectionRequest::from_intent(intent);
    let decision = decide(&request, registry)?;
    Some(llm_config_for_spec(&decision.outcome.model_spec, task_type))
}

#[cfg(test)]
#[allow(unsafe_code)] // serialized env mutation under ENV_LOCK, mirrors vox-config's test idiom
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// Serializes tests in this module that mutate `VOX_MODEL_FORCE`.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn returns_none_or_some_without_panicking_for_research_intent() {
        // Smoke test, not a behavioral assertion: the real registry's
        // contents and configured keys vary by environment (CI has none
        // configured), so both None (nothing selectable) and Some (a
        // local/keyless candidate wins) are valid outcomes. What matters
        // is that this never panics.
        let _ = primary_candidate_for_intent(SelectionIntent::research());
    }

    #[test]
    fn forced_primary_candidate_carries_telemetry_tags_like_the_unforced_path() {
        // Regression test: `chat_stage`'s dedup keeps this (the first/primary)
        // entry over the cascade's tagged copy of the same pinned model, so a
        // pin must carry the same telemetry attribution the unforced
        // `llm_config_for_spec` path would set, or that attribution is
        // silently dropped for every pinned research call.
        let _guard = ENV_LOCK.lock().expect("env lock");
        let prior = std::env::var("VOX_MODEL_FORCE").ok();
        unsafe {
            std::env::set_var("VOX_MODEL_FORCE", "google/gemini-3.8-flash");
        }

        let cfg = primary_candidate_for_intent(SelectionIntent::research())
            .expect("a strict pin must always resolve to a candidate");

        assert_eq!(cfg.model, "google/gemini-3.8-flash");
        assert_eq!(cfg.provider, LlmConfig::openrouter("x").provider);
        assert!(
            cfg.telemetry_task_category.is_some(),
            "pinned primary candidate must carry telemetry_task_category"
        );
        assert!(
            cfg.telemetry_strength_tag.is_some(),
            "pinned primary candidate must carry telemetry_strength_tag"
        );

        unsafe {
            match prior {
                Some(v) => std::env::set_var("VOX_MODEL_FORCE", v),
                None => std::env::remove_var("VOX_MODEL_FORCE"),
            }
        }
    }
}
