//! Utilities to check whether provider API keys are present.

use super::ProviderType;
use vox_secrets::SecretId;

/// Every inference provider Vox can pay for *right now*, by checking each
/// provider's Clavis key via `provider_secret_is_available`. Local providers
/// (no key needed) are always included. This is the credential-aware SSOT the
/// selector and the `vox_credentials_status` surface consult — OpenRouter is
/// one of many.
pub fn available_inference_providers() -> Vec<ProviderType> {
    CANDIDATE_PROVIDERS
        .iter()
        .filter(|p| provider_secret_is_available(p))
        .cloned()
        .collect()
}

/// Every provider the selector will consider, in display order.
pub const CANDIDATE_PROVIDERS: &[ProviderType] = &[
    ProviderType::GoogleDirect,
    ProviderType::OpenRouter,
    ProviderType::Groq,
    ProviderType::Mistral,
    ProviderType::DeepSeek,
    ProviderType::SambaNova,
    ProviderType::Cerebras,
    ProviderType::Anthropic,
    ProviderType::HuggingFaceRouter,
    ProviderType::Ollama,
    ProviderType::PopuliMesh,
    ProviderType::VoxLocal,
];

/// Per-provider credential presence for the full candidate list — the
/// GUI availability panel's SSOT (B9).
pub fn inference_provider_statuses() -> Vec<(ProviderType, bool)> {
    CANDIDATE_PROVIDERS
        .iter()
        .map(|p| (p.clone(), provider_secret_is_available(p)))
        .collect()
}

/// Checks if the primary required secret for a given provider type is currently available.
#[must_use]
pub fn provider_secret_is_available(ptype: &ProviderType) -> bool {
    let secret_id = match ptype {
        ProviderType::GoogleDirect => SecretId::GeminiApiKey,
        ProviderType::OpenRouter => SecretId::OpenRouterApiKey,
        ProviderType::Groq => SecretId::GroqApiKey,
        ProviderType::Cerebras => SecretId::CerebrasApiKey,
        ProviderType::Mistral => SecretId::MistralApiKey,
        ProviderType::DeepSeek => SecretId::DeepSeekApiKey,
        ProviderType::SambaNova => SecretId::SambaNovaApiKey,
        ProviderType::Anthropic => SecretId::AnthropicApiKey,
        ProviderType::HuggingFaceRouter => SecretId::HuggingFaceToken,
        ProviderType::Custom(_) => SecretId::CustomOpenaiApiKey,
        ProviderType::Ollama | ProviderType::PopuliMesh | ProviderType::VoxLocal => {
            // Local endpoints don't strictly require a resolved secret in the same way,
            // or use environment variables instead.
            return true;
        }
    };

    vox_secrets::resolve_secret(secret_id).expose().is_some()
}

// Test-only provider-key availability, per thread (selection is synchronous and runs on
// the calling thread), so tests that set it cannot leak into tests running in parallel.
// `None` means "use the real Clavis check", exactly like
// `route_policy::set_test_privacy_override`.
thread_local! {
    static TEST_KEY_OVERRIDE: std::cell::RefCell<Option<Vec<ProviderType>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, feature = "test-support"))]
pub fn set_test_key_availability(providers: Option<Vec<ProviderType>>) {
    TEST_KEY_OVERRIDE.with(|o| *o.borrow_mut() = providers);
}

/// How a selected model's request actually leaves the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceWire {
    /// The provider's own endpoint with its own key (or a keyless local endpoint).
    Native,
    /// The vendor key is absent, but the model's catalog id is an OpenRouter slug and an
    /// OpenRouter key is present, so OpenRouter serves it.
    OpenRouterFallback,
}

/// Vendors whose catalog ids are OpenRouter slugs (`google/...`, `anthropic/...`), so
/// OpenRouter can serve the same model. Groq/Cerebras/SambaNova ids are vendor-native and
/// are not valid OpenRouter ids.
#[must_use]
pub fn openrouter_can_serve(ptype: &ProviderType) -> bool {
    matches!(
        ptype,
        ProviderType::GoogleDirect
            | ProviderType::Anthropic
            | ProviderType::Mistral
            | ProviderType::DeepSeek
    )
}

fn native_key_available(ptype: &ProviderType) -> bool {
    if matches!(
        ptype,
        ProviderType::Ollama | ProviderType::PopuliMesh | ProviderType::VoxLocal
    ) {
        return true;
    }
    TEST_KEY_OVERRIDE
        .with(|o| o.borrow().as_ref().map(|list| list.contains(ptype)))
        .unwrap_or_else(|| provider_secret_is_available(ptype))
}

/// The wire a model of this provider would use right now, or `None` when no credential
/// can reach it. The selector's key gate and `llm_config_for_spec` both read this, so a
/// model is only selectable when the request it produces can actually authenticate.
#[must_use]
pub fn inference_wire_for(ptype: &ProviderType) -> Option<InferenceWire> {
    if native_key_available(ptype) {
        Some(InferenceWire::Native)
    } else if openrouter_can_serve(ptype) && native_key_available(&ProviderType::OpenRouter) {
        Some(InferenceWire::OpenRouterFallback)
    } else {
        None
    }
}

/// Provider-key availability as seen by dispatch-path model selection.
#[must_use]
pub(crate) fn selection_key_available(ptype: &ProviderType) -> bool {
    inference_wire_for(ptype).is_some()
}

#[cfg(test)]
mod avail_tests {
    use super::*;

    #[test]
    fn local_providers_always_available_and_listed() {
        let avail = available_inference_providers();
        // Local inference needs no Clavis key, so it must always be present.
        assert!(avail.contains(&ProviderType::Ollama));
        assert!(avail.contains(&ProviderType::PopuliMesh));
        assert!(avail.contains(&ProviderType::VoxLocal));
        // The function must be total (returns the providers it checked, not empty).
        assert!(avail.len() >= 3);
    }

    #[test]
    fn statuses_cover_every_candidate_and_mark_locals_present() {
        let statuses = inference_provider_statuses();
        assert_eq!(statuses.len(), CANDIDATE_PROVIDERS.len());
        for (p, present) in &statuses {
            if matches!(
                p,
                ProviderType::Ollama | ProviderType::PopuliMesh | ProviderType::VoxLocal
            ) {
                assert!(*present, "local provider {p:?} must always report present");
            }
        }
    }

    #[test]
    fn vendor_key_wins_then_openrouter_fallback_then_unavailable() {
        set_test_key_availability(Some(vec![ProviderType::GoogleDirect]));
        assert_eq!(
            inference_wire_for(&ProviderType::GoogleDirect),
            Some(InferenceWire::Native)
        );
        assert_eq!(inference_wire_for(&ProviderType::Anthropic), None);

        set_test_key_availability(Some(vec![ProviderType::OpenRouter]));
        assert_eq!(
            inference_wire_for(&ProviderType::Anthropic),
            Some(InferenceWire::OpenRouterFallback)
        );
        assert_eq!(
            inference_wire_for(&ProviderType::Groq),
            None,
            "Groq ids are not OpenRouter slugs; no fallback"
        );
        assert_eq!(
            inference_wire_for(&ProviderType::VoxLocal),
            Some(InferenceWire::Native)
        );
        set_test_key_availability(None);
    }

    #[test]
    fn selection_key_override_restricts_and_resets() {
        set_test_key_availability(Some(vec![ProviderType::DeepSeek]));
        assert!(selection_key_available(&ProviderType::DeepSeek));
        assert!(!selection_key_available(&ProviderType::OpenRouter));
        assert!(
            selection_key_available(&ProviderType::Ollama),
            "local providers never need a key"
        );
        set_test_key_availability(None);
    }
}
