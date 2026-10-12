use serde::{Deserialize, Serialize};

/// Preference for balancing model quality vs operational cost.
///
/// Default is [`Economy`](CostPreference::Economy) — free-by-default product directive.
/// Callers that genuinely need the best model available should pass `Performance` explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CostPreference {
    /// Prioritize model performance/quality over cost.
    Performance,
    /// Prioritize lower-cost models; zero-cost and free-tier models are first-class choices.
    #[default]
    Economy,
}

/// Cost/tier-oriented routing profile. [`Default`] is [`Free`](Self::Free) —
/// the free-by-default product directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingProfile {
    /// Free-tier models only — no API keys required.
    #[default]
    Free,
    /// Mix free + paid; prefer free, fall back to paid.
    Mixed,
    /// Prioritize quality; paid models freely chosen (current paid path).
    Performance,
    /// Local-only (Mens, Ollama); no external calls.
    Local,
}

impl RoutingProfile {
    /// Canonical string key for telemetry / persistence.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Mixed => "mixed",
            Self::Performance => "performance",
            Self::Local => "local",
        }
    }

    /// True when this profile should restrict selection to free-tier models
    /// (no paid fallback). [`Mixed`](Self::Mixed) prefers free but permits a
    /// paid fallback, so it is *not* free-only.
    #[must_use]
    pub fn is_free_only(self) -> bool {
        matches!(self, Self::Free)
    }

    /// True when this profile prefers free models but allows a paid fallback.
    #[must_use]
    pub fn prefers_free(self) -> bool {
        matches!(self, Self::Free | Self::Mixed)
    }

    /// True when this profile restricts selection to local providers.
    #[must_use]
    pub fn is_local_only(self) -> bool {
        matches!(self, Self::Local)
    }
}

impl std::fmt::Display for RoutingProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for RoutingProfile {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "free" => Ok(Self::Free),
            "mixed" => Ok(Self::Mixed),
            "performance" | "perf" => Ok(Self::Performance),
            "local" => Ok(Self::Local),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_preference_wire_names_and_default_are_unchanged() {
        assert_eq!(
            serde_json::to_string(&CostPreference::Economy).unwrap(),
            "\"economy\""
        );
        assert_eq!(
            serde_json::to_string(&CostPreference::Performance).unwrap(),
            "\"performance\""
        );
        assert_eq!(CostPreference::default(), CostPreference::Economy);
    }

    #[test]
    fn routing_profile_wire_names_keys_and_default_are_unchanged() {
        for (p, key) in [
            (RoutingProfile::Free, "free"),
            (RoutingProfile::Mixed, "mixed"),
            (RoutingProfile::Performance, "performance"),
            (RoutingProfile::Local, "local"),
        ] {
            assert_eq!(serde_json::to_string(&p).unwrap(), format!("\"{key}\""));
            assert_eq!(p.as_str(), key);
        }
        assert_eq!(RoutingProfile::default(), RoutingProfile::Free);
    }
}
