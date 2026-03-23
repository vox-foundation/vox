//! Static provider support metadata for `vox doctor` (codex build).
//!
//! Mirrors DeI-style policy lookups without depending on the excluded `vox-dei` workspace crate.

/// How trustworthy quota numbers are for a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaTruthLevel {
    /// Provider documents quotas in public materials.
    Documented,
    /// Estimated from observed rate limits or community reports.
    Estimated,
    /// Variable or undocumented.
    Unknown,
}

/// Routing support tier for a registry id (`auth.json` key).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderSupportLevel {
    /// First-class routing and testing in Vox.
    SupportedFirstClass,
    /// Supported with lighter in-tree coverage.
    SupportedCommunity,
    /// Not routed until policy is refreshed.
    UnsupportedInitially,
}

/// Effective policy row returned for a configured registry.
#[derive(Debug, Clone)]
pub struct ProviderPolicy {
    /// Support tier used for doctor gates.
    pub support_level: ProviderSupportLevel,
    /// Quota reporting confidence.
    pub quota_truth_level: QuotaTruthLevel,
    /// Optional disclosure shown at login / status (reserved for richer doctor output).
    #[allow(dead_code)]
    pub disclosure_text: Option<String>,
}

/// Static registry-driven policy lookup (no network I/O).
#[derive(Debug, Default)]
pub struct ProviderPolicyEngine;

impl ProviderPolicyEngine {
    /// Create a policy engine (stateless).
    pub fn new() -> Self {
        Self
    }

    /// Lookup policy by registry id, ASCII case-insensitive.
    pub fn policy_for(&self, registry: &str) -> Option<ProviderPolicy> {
        let id = registry.to_ascii_lowercase();
        Some(match id.as_str() {
            "google" | "gemini" => ProviderPolicy {
                support_level: ProviderSupportLevel::SupportedFirstClass,
                quota_truth_level: QuotaTruthLevel::Documented,
                disclosure_text: None,
            },
            "openrouter" => ProviderPolicy {
                support_level: ProviderSupportLevel::SupportedFirstClass,
                quota_truth_level: QuotaTruthLevel::Estimated,
                disclosure_text: None,
            },
            "groq" => ProviderPolicy {
                support_level: ProviderSupportLevel::SupportedFirstClass,
                quota_truth_level: QuotaTruthLevel::Estimated,
                disclosure_text: None,
            },
            "anthropic" | "openai" => ProviderPolicy {
                support_level: ProviderSupportLevel::SupportedFirstClass,
                quota_truth_level: QuotaTruthLevel::Documented,
                disclosure_text: None,
            },
            "mistral" | "cerebras" | "deepseek" | "sambanova" => ProviderPolicy {
                support_level: ProviderSupportLevel::SupportedCommunity,
                quota_truth_level: QuotaTruthLevel::Estimated,
                disclosure_text: Some(
                    "Third-party free tiers change frequently — verify limits in the vendor console."
                        .to_string(),
                ),
            },
            // Example of an explicitly blocked placeholder registry used in fixtures/tests.
            "unsupported_stub" => ProviderPolicy {
                support_level: ProviderSupportLevel::UnsupportedInitially,
                quota_truth_level: QuotaTruthLevel::Unknown,
                disclosure_text: Some("This registry id is reserved — remove it from auth.json.".to_string()),
            },
            _ => return None,
        })
    }
}
