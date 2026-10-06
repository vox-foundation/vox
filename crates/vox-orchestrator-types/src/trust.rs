//! Single trust-tier vocabulary for stored and injected context (TRUST-01).
//!
//! Four-way mapping:
//! - `SystemVerified`: curated plus mechanical verification evidence.
//! - `Trusted`: curated (merged to git, indexed by the Library indexer).
//! - `Advisory`: agent-derived from curated inputs only.
//! - `Untrusted`: inbox, live web, legacy web rows, or anything derived from
//!   Untrusted input (D-06 taint rule: an LLM synthesis of Untrusted input is
//!   Untrusted, never Advisory).
//!
//! There is deliberately no `Default`: Turso cannot enforce a CHECK, so writers
//! must name the tier at the type boundary. Parsing is fail-closed.
//!
//! A missing tier must not compile into a value:
//!
//! ```compile_fail
//! let _ = vox_orchestrator_types::ContextTrustTier::default();
//! ```
//!
//! Stored strings parse back exactly:
//!
//! ```
//! use vox_orchestrator_types::ContextTrustTier;
//! assert_eq!(ContextTrustTier::from_db_str("trusted"), ContextTrustTier::Trusted);
//! ```

use serde::{Deserialize, Serialize};

/// Trust tier used by context conflict and injection policy.
/// Declaration order is ascending trust, so `Ord`/`min` pick the least trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextTrustTier {
    Untrusted,
    Advisory,
    Trusted,
    SystemVerified,
}

impl ContextTrustTier {
    /// The stored wire string (matches the serde form).
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Untrusted => "untrusted",
            Self::Advisory => "advisory",
            Self::Trusted => "trusted",
            Self::SystemVerified => "system_verified",
        }
    }

    /// Exact, case-sensitive parse; anything unrecognised is `Untrusted`.
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "advisory" => Self::Advisory,
            "trusted" => Self::Trusted,
            "system_verified" => Self::SystemVerified,
            _ => Self::Untrusted,
        }
    }
}

/// Tiers a finding may carry. A finding can never be `Trusted` or `SystemVerified`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingTier {
    Untrusted,
    Advisory,
}

impl FindingTier {
    pub fn as_db_str(self) -> &'static str {
        ContextTrustTier::from(self).as_db_str()
    }
}

impl From<FindingTier> for ContextTrustTier {
    fn from(t: FindingTier) -> Self {
        match t {
            FindingTier::Untrusted => Self::Untrusted,
            FindingTier::Advisory => Self::Advisory,
        }
    }
}

/// Tier for curated content: `SystemVerified` with mechanical evidence, else `Trusted`.
pub fn tier_for_curated(has_mechanical_evidence: bool) -> ContextTrustTier {
    if has_mechanical_evidence {
        ContextTrustTier::SystemVerified
    } else {
        ContextTrustTier::Trusted
    }
}

/// Tier for a knowledge-graph node type. Allowlist of two Advisory types;
/// every other or missing type is `Untrusted` (D-06 taint rule).
pub fn tier_for_knowledge_node_type(node_type: Option<&str>) -> ContextTrustTier {
    match node_type {
        Some("fact" | "concept") => ContextTrustTier::Advisory,
        _ => ContextTrustTier::Untrusted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ContextTrustTier::*;

    #[test]
    fn tier_order_is_ascending_trust() {
        assert!(Untrusted < Advisory && Advisory < Trusted && Trusted < SystemVerified);
    }

    #[test]
    fn min_of_equal_tiers_is_that_tier() {
        assert_eq!(Trusted.min(Trusted), Trusted);
    }

    #[test]
    fn min_trusted_untrusted_is_untrusted() {
        assert_eq!(Trusted.min(Untrusted), Untrusted);
    }

    #[test]
    fn as_db_str_matches_phase2_wire_strings() {
        assert_eq!(Untrusted.as_db_str(), "untrusted");
        assert_eq!(Advisory.as_db_str(), "advisory");
        assert_eq!(Trusted.as_db_str(), "trusted");
        assert_eq!(SystemVerified.as_db_str(), "system_verified");
    }

    #[test]
    fn from_db_str_round_trips_all_four() {
        for t in [Untrusted, Advisory, Trusted, SystemVerified] {
            assert_eq!(ContextTrustTier::from_db_str(t.as_db_str()), t);
        }
    }

    #[test]
    fn from_db_str_fails_closed() {
        for s in ["", "TRUSTED", "bogus", " trusted"] {
            assert_eq!(ContextTrustTier::from_db_str(s), Untrusted, "{s:?}");
        }
    }

    #[test]
    fn serde_wire_strings_unchanged() {
        assert_eq!(
            serde_json::from_str::<ContextTrustTier>("\"trusted\"").unwrap(),
            Trusted
        );
        assert_eq!(
            serde_json::to_string(&SystemVerified).unwrap(),
            "\"system_verified\""
        );
    }

    #[test]
    fn finding_tier_converts_and_encodes() {
        assert_eq!(ContextTrustTier::from(FindingTier::Untrusted), Untrusted);
        assert_eq!(ContextTrustTier::from(FindingTier::Advisory), Advisory);
        assert_eq!(FindingTier::Advisory.as_db_str(), "advisory");
        assert_eq!(FindingTier::Untrusted.as_db_str(), "untrusted");
    }

    #[test]
    fn tier_for_curated_splits_on_evidence() {
        assert_eq!(tier_for_curated(false), Trusted);
        assert_eq!(tier_for_curated(true), SystemVerified);
    }

    #[test]
    fn node_type_allowlist_is_fact_and_concept_only() {
        assert_eq!(tier_for_knowledge_node_type(Some("fact")), Advisory);
        assert_eq!(tier_for_knowledge_node_type(Some("concept")), Advisory);
        for t in [
            "external_research",
            "research_synthesis",
            "research_finding",
            "web_scrape",
            "web_research_source",
            "research_cache",
            "document",
            "anything_else",
        ] {
            assert_eq!(tier_for_knowledge_node_type(Some(t)), Untrusted, "{t}");
        }
        assert_eq!(tier_for_knowledge_node_type(None), Untrusted);
    }
}
