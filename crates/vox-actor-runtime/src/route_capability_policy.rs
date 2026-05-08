//! Environment-driven routing capability policy (aligned with `contracts/orchestration/model-routing.v1.yaml`).
//!
//! Used by chat route resolution and by orchestrator MCP/registry paths so denial reasons stay consistent.

/// Snapshot of `VOX_ROUTE_*` policy knobs after env resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteCapabilityPolicySnapshot {
    /// Normalized profile label from `VOX_ROUTE_POLICY_PROFILE` (lowercase trim).
    pub profile: String,
    pub allow_net: bool,
    pub allow_provider_network: bool,
    pub allow_local_model_http: bool,
}

impl RouteCapabilityPolicySnapshot {
    /// Read policy from environment (same semantics as chat route resolution).
    #[must_use]
    pub fn from_env() -> Self {
        let profile = std::env::var("VOX_ROUTE_POLICY_PROFILE")
            .unwrap_or_else(|_| "balanced".to_string())
            .to_ascii_lowercase();
        let restricted = profile == "restricted";
        let mut allow_net = !restricted;
        let mut allow_provider_network = !restricted;
        let mut allow_local_model_http = !restricted;
        if let Ok(v) = std::env::var("VOX_ROUTE_ALLOW_NET") {
            allow_net = matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on");
        }
        if let Ok(v) = std::env::var("VOX_ROUTE_ALLOW_PROVIDER_NETWORK") {
            allow_provider_network =
                matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on");
        }
        if let Ok(v) = std::env::var("VOX_ROUTE_ALLOW_LOCAL_MODEL_HTTP") {
            allow_local_model_http =
                matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on");
        }
        Self {
            profile,
            allow_net,
            allow_provider_network,
            allow_local_model_http,
        }
    }
}

/// When `true`, the provider uses loopback/LAN HTTP to a local runtime (Ollama / Populi / Vox local).
///
/// When `false`, the lane is treated as a remote cloud provider API (OpenRouter, HF router, etc.).
#[must_use]
pub fn exclusion_reason_for_llm_lane(
    is_local_http_provider: bool,
    policy: &RouteCapabilityPolicySnapshot,
) -> Option<&'static str> {
    if !policy.allow_net {
        return Some("route_policy:network_disabled");
    }
    if is_local_http_provider {
        if !policy.allow_local_model_http {
            return Some("route_policy:local_model_http_disabled");
        }
    } else if !policy.allow_provider_network {
        return Some("route_policy:provider_network_disabled");
    }
    None
}
