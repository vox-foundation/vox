/// Shared automatic cloud model strategy used by runtime and MCP paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoModelStrategy {
    /// Delegate final model selection to provider virtual routing (e.g. `openrouter/auto`).
    ProviderAuto,
    /// Use preferred configured model id when available.
    PreferredModel,
}

impl AutoModelStrategy {
    #[must_use]
    pub fn from_env() -> Self {
        let raw = std::env::var("VOX_AUTO_MODEL_STRATEGY")
            .unwrap_or_else(|_| "provider_auto".to_string());
        match raw.trim().to_ascii_lowercase().as_str() {
            "preferred_model" | "preferred" => Self::PreferredModel,
            _ => Self::ProviderAuto,
        }
    }
}

/// Resolve OpenRouter model id for the selected strategy.
#[must_use]
pub fn resolve_openrouter_model(preferred: Option<String>) -> String {
    match AutoModelStrategy::from_env() {
        AutoModelStrategy::ProviderAuto => crate::OPENROUTER_AUTO.to_string(),
        AutoModelStrategy::PreferredModel => preferred
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                tracing::warn!(
                    "PreferredModel strategy active but no model specified. Falling back to {}.",
                    crate::OPENROUTER_AUTO
                );
                crate::OPENROUTER_AUTO.to_string()
            }),
    }
}

/// Weighted routing priorities for normalized auto selection.
///
/// Read from `VOX_AUTO_ROUTING_PRIORITY` using CSV key/value pairs, e.g.
/// `efficiency=35,precision=30,latency=15,availability=15,balance=5,mobile=0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoRoutingPriority {
    pub efficiency: u8,
    pub precision: u8,
    pub latency: u8,
    pub availability: u8,
    pub balance: u8,
    pub mobile: u8,
}

impl Default for AutoRoutingPriority {
    fn default() -> Self {
        Self {
            efficiency: 25,
            precision: 30,
            latency: 20,
            availability: 20,
            balance: 5,
            mobile: 0,
        }
    }
}

impl AutoRoutingPriority {
    #[must_use]
    pub fn from_env() -> Self {
        let raw = match std::env::var("VOX_AUTO_ROUTING_PRIORITY") {
            Ok(v) => v,
            Err(_) => return Self::default(),
        };
        let mut out = Self::default();
        for part in raw.split(',') {
            let mut it = part.splitn(2, '=');
            let key = it.next().map(str::trim).unwrap_or("").to_ascii_lowercase();
            let val = it.next().map(str::trim).unwrap_or("");
            let Ok(parsed) = val.parse::<u8>() else {
                continue;
            };
            match key.as_str() {
                "efficiency" | "cost" => out.efficiency = parsed,
                "precision" | "quality" => out.precision = parsed,
                "latency" | "speed" => out.latency = parsed,
                "availability" => out.availability = parsed,
                "balance" => out.balance = parsed,
                "mobile" => out.mobile = parsed,
                _ => {}
            }
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeminiRoutePolicy {
    RegistryDefault,
    OpenRouterFirst,
    GoogleDirectOnly,
}

impl GeminiRoutePolicy {
    #[must_use]
    pub fn from_env() -> Self {
        let raw = std::env::var("VOX_GEMINI_ROUTE_POLICY")
            .unwrap_or_else(|_| "openrouter_first".to_string());
        match raw.trim().to_ascii_lowercase().as_str() {
            "registry_default" | "default" => Self::RegistryDefault,
            "google_direct_only" | "google_only" | "direct_only" => Self::GoogleDirectOnly,
            _ => Self::OpenRouterFirst,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeminiRouteTargets {
    pub openrouter_model: String,
    pub google_direct_model: String,
}

#[must_use]
pub fn gemini_route_targets_from_env() -> GeminiRouteTargets {
    GeminiRouteTargets {
        openrouter_model: std::env::var("OPENROUTER_GEMINI_MODEL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "google/gemini-2.5-flash".to_string()),
        google_direct_model: std::env::var("GEMINI_DIRECT_MODEL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "gemini-2.5-flash".to_string()),
    }
}

/// Cost preference hint for callers in crates that cannot depend on `vox-orchestrator` directly.
///
/// Mirror of `vox_orchestrator::config::CostPreference` — keep the variants aligned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteCostPreference {
    /// Minimize spend; pick cheapest viable model.
    Economy,
    /// Maximize quality; pick highest-capability model.
    Performance,
}

/// OpenRouter provider-level routing strategy hint.
///
/// When the requested model is `openrouter/auto`, this hint is injected into the request body as
/// the `route` field and into `X-OpenRouter-Provider-Preferences` to guide OpenRouter's internal
/// broker without requiring us to manage provider allow-lists statically.
///
/// See: <https://openrouter.ai/docs/provider-routing>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenRouterRouteHint {
    /// Pick the cheapest provider that can service this request.
    Price,
    /// Pick the highest-quality provider (longest context, best capabilities).
    Quality,
    /// Prefer throughput; fall back to other providers if the primary is unavailable.
    Fallback,
}

impl OpenRouterRouteHint {
    /// Route hint label sent in the `route` JSON field.
    #[must_use]
    pub fn as_route_str(self) -> &'static str {
        match self {
            Self::Price => "price",
            Self::Quality => "quality",
            Self::Fallback => "fallback",
        }
    }
}

/// Derives the appropriate [`OpenRouterRouteHint`] for a given cost preference.
///
/// - `Performance` tasks want the best model → `Quality`.
/// - `Economy` tasks want cheapest → `Price`.
#[must_use]
pub fn derive_openrouter_route_hint(preference: RouteCostPreference) -> OpenRouterRouteHint {
    match preference {
        RouteCostPreference::Performance => OpenRouterRouteHint::Quality,
        RouteCostPreference::Economy => OpenRouterRouteHint::Price,
    }
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn auto_routing_priority_parses_env() {
        let _g = ENV_LOCK.lock().expect("env lock");
        let prev = std::env::var("VOX_AUTO_ROUTING_PRIORITY").ok();
        unsafe {
            std::env::set_var(
                "VOX_AUTO_ROUTING_PRIORITY",
                "efficiency=40,precision=30,latency=10,availability=10,balance=5,mobile=5",
            );
        }
        let p = AutoRoutingPriority::from_env();
        assert_eq!(p.efficiency, 40);
        assert_eq!(p.precision, 30);
        assert_eq!(p.mobile, 5);
        unsafe {
            if let Some(v) = prev {
                std::env::set_var("VOX_AUTO_ROUTING_PRIORITY", v);
            } else {
                std::env::remove_var("VOX_AUTO_ROUTING_PRIORITY");
            }
        }
    }

    #[test]
    fn gemini_policy_and_targets_read_env() {
        let _g = ENV_LOCK.lock().expect("env lock");
        let prev_policy = std::env::var("VOX_GEMINI_ROUTE_POLICY").ok();
        let prev_or = std::env::var("OPENROUTER_GEMINI_MODEL").ok();
        let prev_direct = std::env::var("GEMINI_DIRECT_MODEL").ok();
        unsafe {
            std::env::set_var("VOX_GEMINI_ROUTE_POLICY", "google_direct_only");
            std::env::set_var("OPENROUTER_GEMINI_MODEL", "google/gemini-2.5-pro");
            std::env::set_var("GEMINI_DIRECT_MODEL", "gemini-2.5-pro");
        }
        assert_eq!(
            GeminiRoutePolicy::from_env(),
            GeminiRoutePolicy::GoogleDirectOnly
        );
        let t = gemini_route_targets_from_env();
        assert_eq!(t.openrouter_model, "google/gemini-2.5-pro");
        assert_eq!(t.google_direct_model, "gemini-2.5-pro");
        unsafe {
            if let Some(v) = prev_policy {
                std::env::set_var("VOX_GEMINI_ROUTE_POLICY", v);
            } else {
                std::env::remove_var("VOX_GEMINI_ROUTE_POLICY");
            }
            if let Some(v) = prev_or {
                std::env::set_var("OPENROUTER_GEMINI_MODEL", v);
            } else {
                std::env::remove_var("OPENROUTER_GEMINI_MODEL");
            }
            if let Some(v) = prev_direct {
                std::env::set_var("GEMINI_DIRECT_MODEL", v);
            } else {
                std::env::remove_var("GEMINI_DIRECT_MODEL");
            }
        }
    }
}
