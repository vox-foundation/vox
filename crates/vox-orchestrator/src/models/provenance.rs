//! Where a resolved model id came from, for chat routing events and the routing DTO.

use crate::models::spec::PricingSource;
use crate::models::{ModelSpec, ProviderType};

/// Provenance of a resolved model id. The GUI shows a concrete version only for
/// [`ResolvedFrom::Catalog`]; a bootstrap pick shows its family marked offline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedFrom {
    /// Priced from a live source (OpenRouter, a direct provider API, LiteLLM, telemetry, user config).
    Catalog,
    /// Still carries the compiled bootstrap JSON's pricing: its version may be stale.
    Bootstrap,
    /// On-device or mesh inference (Ollama, VoxLocal, Populi mesh).
    Local,
}

impl ResolvedFrom {
    /// Wire name used by the `routing_decision` turn event and `RoutingSummaryDto`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Catalog => "catalog",
            Self::Bootstrap => "bootstrap",
            Self::Local => "local",
        }
    }
}

/// Classify `spec` by backend, then by `PricingSource`: the catalog refresh re-stamps
/// fetched specs from `Bootstrap` to `OpenRouter` (`registry.rs`), so a spec still marked
/// `Bootstrap` never came from a live source.
#[must_use]
pub fn resolved_from(spec: &ModelSpec) -> ResolvedFrom {
    match (&spec.provider_type, &spec.pricing_source) {
        (ProviderType::Ollama | ProviderType::VoxLocal | ProviderType::PopuliMesh, _) => {
            ResolvedFrom::Local
        }
        (_, PricingSource::Bootstrap) => ResolvedFrom::Bootstrap,
        _ => ResolvedFrom::Catalog,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ModelCapabilities, StrengthTag};

    fn spec(provider_type: ProviderType, pricing_source: PricingSource) -> ModelSpec {
        ModelSpec {
            id: "acme/widget-5.5".into(),
            canonical_slug: "acme/widget-5.5".into(),
            provider: "test".into(),
            provider_type,
            max_tokens: 32_000,
            cost_per_1k: 0.001,
            cost_per_1k_input: 0.001,
            cost_per_1k_output: 0.001,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![StrengthTag::Generalist],
            capabilities: ModelCapabilities::default(),
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source,
            supported_parameters: vec![],
        }
    }

    #[test]
    fn an_openrouter_priced_spec_comes_from_the_catalog() {
        assert_eq!(
            resolved_from(&spec(ProviderType::OpenRouter, PricingSource::OpenRouter)),
            ResolvedFrom::Catalog
        );
    }

    #[test]
    fn a_bootstrap_priced_cloud_spec_comes_from_the_bootstrap_file() {
        assert_eq!(
            resolved_from(&spec(ProviderType::OpenRouter, PricingSource::Bootstrap)),
            ResolvedFrom::Bootstrap
        );
    }

    #[test]
    fn direct_provider_specs_are_catalog_not_bootstrap() {
        assert_eq!(
            resolved_from(&spec(
                ProviderType::Anthropic,
                PricingSource::AnthropicDirect
            )),
            ResolvedFrom::Catalog
        );
        assert_eq!(
            resolved_from(&spec(ProviderType::GoogleDirect, PricingSource::LiteLLM)),
            ResolvedFrom::Catalog
        );
    }

    #[test]
    fn local_backends_are_local_whatever_their_pricing_source() {
        for provider in [
            ProviderType::Ollama,
            ProviderType::VoxLocal,
            ProviderType::PopuliMesh,
        ] {
            for pricing in [PricingSource::Bootstrap, PricingSource::UserConfig] {
                assert_eq!(
                    resolved_from(&spec(provider.clone(), pricing.clone())),
                    ResolvedFrom::Local,
                    "{provider:?} priced by {pricing:?}"
                );
            }
        }
    }

    #[test]
    fn wire_names_match_the_turn_event_contract() {
        assert_eq!(ResolvedFrom::Catalog.as_str(), "catalog");
        assert_eq!(ResolvedFrom::Bootstrap.as_str(), "bootstrap");
        assert_eq!(ResolvedFrom::Local.as_str(), "local");
    }
}
