use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Sensitivity level of a task or file context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PrivacyLevel {
    /// Public or non-sensitive internal data.
    Public,
    /// Sensitive internal data, no PII.
    Internal,
    /// Contains PII (Personally Identifiable Information).
    Private,
    /// High-stakes or regulated data (HIPAA, GDPR, etc).
    Regulated,
}

/// Routing policy for different privacy levels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyRoutingPolicy {
    /// Minimum model tier required for each level.
    pub min_tier: PrivacyLevel,
    /// Explicitly forbidden providers for sensitive data.
    pub forbidden_providers: HashSet<String>,
    /// Whether local inference is mandatory for Private/Regulated levels.
    pub force_local_for_private: bool,
}

impl Default for PrivacyRoutingPolicy {
    fn default() -> Self {
        let mut forbidden = HashSet::new();
        forbidden.insert("untrusted-provider".to_string());
        Self {
            min_tier: PrivacyLevel::Public,
            forbidden_providers: forbidden,
            force_local_for_private: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivacyRoutingDecision {
    /// Safe to use public/remote models.
    Allowed,
    /// Must redact PII before sending remote.
    Redact,
    /// Must use local inference only.
    LocalOnly,
    /// Privacy policy blocks this operation.
    Blocked,
}

impl std::fmt::Display for PrivacyLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Public => write!(f, "public"),
            Self::Internal => write!(f, "internal"),
            Self::Private => write!(f, "private"),
            Self::Regulated => write!(f, "regulated"),
        }
    }
}

impl std::fmt::Display for PrivacyRoutingDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Allowed => write!(f, "allowed"),
            Self::Redact => write!(f, "redact"),
            Self::LocalOnly => write!(f, "local-only"),
            Self::Blocked => write!(f, "blocked"),
        }
    }
}

/// A router that filters model candidates based on privacy constraints.
pub struct PrivacyRouter {
    pub policy: PrivacyRoutingPolicy,
}

impl PrivacyRouter {
    pub fn new(policy: PrivacyRoutingPolicy) -> Self {
        Self { policy }
    }

    /// Determines the routing decision for a given PII detection.
    pub fn route(&self, pii_detected: bool) -> PrivacyRoutingDecision {
        if pii_detected {
            if self.policy.force_local_for_private {
                PrivacyRoutingDecision::LocalOnly
            } else {
                PrivacyRoutingDecision::Redact
            }
        } else {
            PrivacyRoutingDecision::Allowed
        }
    }

    /// Filter available models based on task privacy level.
    pub fn filter_models(
        &self,
        level: PrivacyLevel,
        candidates: Vec<crate::models::ModelSpec>,
    ) -> Vec<crate::models::ModelSpec> {
        candidates
            .into_iter()
            .filter(|m| {
                // 1. Check forbidden providers
                if self.policy.forbidden_providers.contains(&m.provider) {
                    return false;
                }

                // 2. Enforce local-only for Private+ if configured
                if self.policy.force_local_for_private && level >= PrivacyLevel::Private {
                    if m.provider != "ollama" && m.provider != "local" {
                        return false;
                    }
                }

                true
            })
            .collect()
    }
}
