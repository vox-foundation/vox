//! Routing capability policy alignment with `VOX_ROUTE_*` and `contracts/orchestration/model-routing.v1.yaml`.
//!
//! Shared by MCP model resolution and CLI/model explain surfaces.

use crate::models::{ModelSpec, ProviderType};
#[cfg(feature = "runtime")]
use vox_actor_runtime::route_capability_policy::{
    RouteCapabilityPolicySnapshot, exclusion_reason_for_llm_lane,
};

/// True for providers that run without a paid cloud credential (local inference over HTTP).
/// Canonical home for this predicate — reuse it rather than re-deriving the variant list
/// elsewhere (e.g. `vox-cli`'s `model list` command), so a future local provider only needs
/// updating in one place.
#[inline]
#[must_use]
pub fn is_local_http_provider(provider_type: &ProviderType) -> bool {
    matches!(
        provider_type,
        ProviderType::Ollama | ProviderType::PopuliMesh | ProviderType::VoxLocal
    )
}

#[cfg(any(test, not(feature = "runtime")))]
pub(crate) fn exclusion_reason_without_runtime_with<F: Fn(&str) -> Option<String>>(
    is_local_http_provider: bool,
    get_env: F,
) -> Option<&'static str> {
    let profile = get_env("VOX_ROUTE_POLICY_PROFILE")
        .unwrap_or_else(|| "balanced".to_string())
        .to_ascii_lowercase();
    let restricted = profile == "restricted";
    let parse_flag = |key: &str, default: bool| -> bool {
        get_env(key)
            .map(|v| {
                matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(default)
    };
    if !parse_flag("VOX_ROUTE_ALLOW_NET", !restricted) {
        return Some("route_policy:network_disabled");
    }
    if is_local_http_provider {
        if !parse_flag("VOX_ROUTE_ALLOW_LOCAL_MODEL_HTTP", !restricted) {
            return Some("route_policy:local_model_http_disabled");
        }
    } else if !parse_flag("VOX_ROUTE_ALLOW_PROVIDER_NETWORK", !restricted) {
        return Some("route_policy:provider_network_disabled");
    }
    None
}

#[cfg(not(feature = "runtime"))]
fn exclusion_reason_without_runtime(is_local_http_provider: bool) -> Option<&'static str> {
    exclusion_reason_without_runtime_with(is_local_http_provider, |k| std::env::var(k).ok())
}

/// Returns a static denial code when [`ModelSpec`] cannot be used under the current route policy.
#[must_use]
pub fn route_policy_exclusion_reason(model: &ModelSpec) -> Option<&'static str> {
    #[cfg(feature = "runtime")]
    {
        let snap = RouteCapabilityPolicySnapshot::from_env();
        exclusion_reason_for_llm_lane(is_local_http_provider(&model.provider_type), &snap)
    }
    #[cfg(not(feature = "runtime"))]
    {
        exclusion_reason_without_runtime(is_local_http_provider(&model.provider_type))
    }
}

#[must_use]
pub fn route_policy_allows_model(model: &ModelSpec) -> bool {
    route_policy_exclusion_reason(model).is_none()
}

/// Pure privacy decision core, relocated from `vox-orchestrator-mcp`'s
/// `llm_bridge::local_health` so it's reachable from this crate's own
/// model-selection filter chain (`models::registry::best_for_internal`),
/// closing a real coverage gap: the mcp-crate-only original was reachable
/// only from the synchronous chat path, never from `AiTaskProcessor` (the
/// pipeline handling the bulk of Vox's real agentic work), since
/// `vox-orchestrator` cannot depend on `vox-orchestrator-mcp`.
///
/// A hard filter, not a ranking hint: when `local_only`, non-local-provider
/// models are excluded from candidates entirely (see [`is_local_http_provider`]
/// for the local/cloud split reused here), never merely deprioritized.
#[must_use]
pub fn privacy_allows_model_for_mode(m: &ModelSpec, local_only: bool) -> bool {
    if local_only {
        return is_local_http_provider(&m.provider_type);
    }
    true
}

/// Test-only seam for `VOX_INFERENCE_PRIVACY`, mirroring the pattern used by
/// `vox-orchestrator-mcp`'s `TEST_HEALTH_OVERRIDE` — avoids mutating the real
/// process env (which is racy under parallel `cargo test`) while still
/// exercising the real decision logic in [`inference_privacy_local_only_from_env`].
///
/// Gated on `cfg(test)` OR the `test-support` feature (never on by default):
/// `vox-orchestrator-mcp`'s test suite (`llm_bridge::local_health`) needs to
/// drive this override too, and that crate compiles `vox-orchestrator` as an
/// ordinary (non-`cfg(test)`) dev-dependency with `test-support` enabled — a
/// bare `#[cfg(test)]` gate here would compile out for that caller entirely,
/// silently breaking the seam, while a plain `pub fn` would leak a
/// privacy-bypass knob into every release build's public API. The static
/// itself stays always-compiled (its own `Mutex` is a negligible cost) so the
/// getter (`inference_privacy_local_only_from_env`) doesn't need its own gate.
static TEST_PRIVACY_OVERRIDE: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

#[cfg(any(test, feature = "test-support"))]
pub fn set_test_privacy_override(v: Option<&str>) {
    *TEST_PRIVACY_OVERRIDE
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = v.map(str::to_string);
}

/// Reads `VOX_INFERENCE_PRIVACY` (`any` (default) | `local_only`), or the
/// test-only override set via `set_test_privacy_override`, and returns
/// whether `local_only` is in effect.
#[must_use]
pub fn inference_privacy_local_only_from_env() -> bool {
    if let Some(v) = TEST_PRIVACY_OVERRIDE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
    {
        return v.trim().eq_ignore_ascii_case("local_only");
    }
    std::env::var("VOX_INFERENCE_PRIVACY")
        .map(|v| v.trim().eq_ignore_ascii_case("local_only"))
        .unwrap_or(false)
}

#[cfg(test)]
mod semcov_wave1b_tests {
    #![allow(unused_imports)]
    use super::*;

    #[test]
    fn local_http_providers_match_only_local_lanes() {
        // True arm: each of the three local HTTP provider variants.
        assert!(is_local_http_provider(&ProviderType::Ollama));
        assert!(is_local_http_provider(&ProviderType::PopuliMesh));
        assert!(is_local_http_provider(&ProviderType::VoxLocal));

        // False arm: a remote/cloud provider variant is not a local HTTP lane.
        assert!(!is_local_http_provider(&ProviderType::Anthropic));
    }

    #[test]
    fn local_http_providers_classified_true_others_false() {
        assert!(is_local_http_provider(&ProviderType::Ollama));
        assert!(is_local_http_provider(&ProviderType::PopuliMesh));
        assert!(is_local_http_provider(&ProviderType::VoxLocal));
        assert!(!is_local_http_provider(&ProviderType::OpenRouter));
    }

    fn priv_spec(id: &str, provider_type: ProviderType) -> ModelSpec {
        use crate::models::{ModelCapabilities, spec::PricingSource};
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type,
            max_tokens: 8_000,
            cost_per_1k: 0.0,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            is_free: true,
            observed_cost_per_1k: None,
            strengths: vec![],
            capabilities: ModelCapabilities::default(),
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::Bootstrap,
            supported_parameters: vec![],
        }
    }

    #[test]
    fn privacy_allows_model_for_mode_blocks_cloud_under_local_only_and_allows_local() {
        let cloud = priv_spec("cloud-model", ProviderType::OpenRouter);
        let local = priv_spec("local-model", ProviderType::Ollama);
        assert!(!privacy_allows_model_for_mode(&cloud, true));
        assert!(privacy_allows_model_for_mode(&local, true));
        assert!(privacy_allows_model_for_mode(&cloud, false));
    }

    #[test]
    fn exclusion_reason_without_runtime_enforces_all_four_vox_route_env_overrides() {
        let lookup = |pairs: &[(&str, &str)]| {
            let map: std::collections::HashMap<String, String> = pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect();
            move |key: &str| map.get(key).cloned()
        };

        // 1. Default ("balanced") profile allows both local and cloud lanes.
        assert_eq!(
            exclusion_reason_without_runtime_with(true, lookup(&[])),
            None
        );
        assert_eq!(
            exclusion_reason_without_runtime_with(false, lookup(&[])),
            None
        );
        assert_eq!(
            exclusion_reason_without_runtime_with(
                false,
                lookup(&[("VOX_ROUTE_POLICY_PROFILE", "balanced")])
            ),
            None
        );

        // 2. "restricted" profile fails closed on network when ALLOW_* are unset.
        let restricted = lookup(&[("VOX_ROUTE_POLICY_PROFILE", "restricted")]);
        assert_eq!(
            exclusion_reason_without_runtime_with(true, &restricted),
            Some("route_policy:network_disabled")
        );
        assert_eq!(
            exclusion_reason_without_runtime_with(false, &restricted),
            Some("route_policy:network_disabled")
        );

        // 3. VOX_ROUTE_ALLOW_NET="false" blocks all network lanes regardless of profile.
        let no_net = lookup(&[("VOX_ROUTE_ALLOW_NET", "false")]);
        assert_eq!(
            exclusion_reason_without_runtime_with(true, &no_net),
            Some("route_policy:network_disabled")
        );
        assert_eq!(
            exclusion_reason_without_runtime_with(false, &no_net),
            Some("route_policy:network_disabled")
        );

        // 4. VOX_ROUTE_ALLOW_NET="true" + VOX_ROUTE_ALLOW_LOCAL_MODEL_HTTP="0" blocks local HTTP
        //    providers (Ollama / PopuliMesh / VoxLocal) while allowing cloud providers.
        let block_local = lookup(&[
            ("VOX_ROUTE_ALLOW_NET", "true"),
            ("VOX_ROUTE_ALLOW_LOCAL_MODEL_HTTP", "0"),
        ]);
        for local_pt in [
            ProviderType::Ollama,
            ProviderType::PopuliMesh,
            ProviderType::VoxLocal,
        ] {
            assert_eq!(
                exclusion_reason_without_runtime_with(
                    is_local_http_provider(&local_pt),
                    &block_local
                ),
                Some("route_policy:local_model_http_disabled")
            );
        }
        assert_eq!(
            exclusion_reason_without_runtime_with(
                is_local_http_provider(&ProviderType::OpenRouter),
                &block_local
            ),
            None
        );

        // 5. VOX_ROUTE_ALLOW_NET="true" + VOX_ROUTE_ALLOW_PROVIDER_NETWORK="0" blocks cloud
        //    providers while allowing local HTTP providers.
        let block_cloud = lookup(&[
            ("VOX_ROUTE_ALLOW_NET", "true"),
            ("VOX_ROUTE_ALLOW_PROVIDER_NETWORK", "0"),
        ]);
        assert_eq!(
            exclusion_reason_without_runtime_with(
                is_local_http_provider(&ProviderType::Anthropic),
                &block_cloud
            ),
            Some("route_policy:provider_network_disabled")
        );
        assert_eq!(
            exclusion_reason_without_runtime_with(
                is_local_http_provider(&ProviderType::Ollama),
                &block_cloud
            ),
            None
        );

        // 6. Under "restricted", selectively enabling net + local keeps cloud fail-closed.
        let restricted_local_only = lookup(&[
            ("VOX_ROUTE_POLICY_PROFILE", "restricted"),
            ("VOX_ROUTE_ALLOW_NET", "1"),
            ("VOX_ROUTE_ALLOW_LOCAL_MODEL_HTTP", "yes"),
        ]);
        assert_eq!(
            exclusion_reason_without_runtime_with(true, &restricted_local_only),
            None
        );
        assert_eq!(
            exclusion_reason_without_runtime_with(false, &restricted_local_only),
            Some("route_policy:provider_network_disabled")
        );

        // Smoke-check public entrypoint compiles and links under active feature set.
        let cloud = priv_spec("cloud-model", ProviderType::OpenRouter);
        let _ = route_policy_exclusion_reason(&cloud);
    }
}
