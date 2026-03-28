//! Reconstruction campaign contracts, scoring, and memory snapshots.
//!
//! This module keeps moonshot-facing concepts lightweight and serializable so
//! they can be attached to tasks, lineage payloads, and context state without
//! introducing a heavyweight subsystem.

use serde::{Deserialize, Serialize};

/// Benchmark ladder tiers for progressive repository reconstruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ReconstructionBenchmarkTier {
    IssueRepair,
    SubsystemRegen,
    CrateRegen,
    RepoRegen,
}

impl ReconstructionBenchmarkTier {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IssueRepair => "issue_repair",
            Self::SubsystemRegen => "subsystem_regen",
            Self::CrateRegen => "crate_regen",
            Self::RepoRegen => "repo_regen",
        }
    }

    #[must_use]
    pub fn next(self) -> Option<Self> {
        match self {
            Self::IssueRepair => Some(Self::SubsystemRegen),
            Self::SubsystemRegen => Some(Self::CrateRegen),
            Self::CrateRegen => Some(Self::RepoRegen),
            Self::RepoRegen => None,
        }
    }
}

/// Explicit agent role for reconstruction-specialized multi-agent handoffs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AgentExecutionRole {
    Planner,
    Builder,
    Verifier,
    Reproducer,
    Researcher,
}

impl AgentExecutionRole {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planner => "planner",
            Self::Builder => "builder",
            Self::Verifier => "verifier",
            Self::Reproducer => "reproducer",
            Self::Researcher => "researcher",
        }
    }
}

/// Coarse verification layers used to explain trust decisions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerificationLayerStatus {
    pub structural_ok: bool,
    pub behavioral_ok: bool,
    pub contract_ok: bool,
    pub docs_ssot_ok: bool,
    pub grounding_ok: bool,
}

impl VerificationLayerStatus {
    #[must_use]
    pub fn passed_layers(&self) -> usize {
        [
            self.structural_ok,
            self.behavioral_ok,
            self.contract_ok,
            self.docs_ssot_ok,
            self.grounding_ok,
        ]
        .into_iter()
        .filter(|ok| *ok)
        .count()
    }

    #[must_use]
    pub fn score(&self) -> f32 {
        self.passed_layers() as f32 / 5.0
    }
}

/// Evidence used to score a reconstruction attempt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReconstructionEvidence {
    pub compile_ok: bool,
    pub targeted_tests_ok: bool,
    pub contract_checks_ok: bool,
    pub docs_ssot_ok: bool,
    pub regression_checks_ok: bool,
}

impl ReconstructionEvidence {
    /// Weighted campaign score:
    /// compile/test correctness dominates, docs/contract/regression provide the rest.
    #[must_use]
    pub fn score(&self) -> f32 {
        let mut score = 0.0_f32;
        if self.compile_ok {
            score += 0.30;
        }
        if self.targeted_tests_ok {
            score += 0.30;
        }
        if self.contract_checks_ok {
            score += 0.15;
        }
        if self.docs_ssot_ok {
            score += 0.10;
        }
        if self.regression_checks_ok {
            score += 0.15;
        }
        score
    }

    #[must_use]
    pub fn passes_gate(&self) -> bool {
        self.compile_ok && self.targeted_tests_ok && self.score() >= 0.80
    }
}

/// Resumable campaign state for long-horizon reconstruction runs.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CampaignMemorySnapshot {
    pub campaign_id: String,
    #[serde(default)]
    pub stable_facts: Vec<String>,
    #[serde(default)]
    pub hypotheses: Vec<String>,
    #[serde(default)]
    pub contradictions: Vec<String>,
    #[serde(default)]
    pub unresolved_questions: Vec<String>,
    #[serde(default)]
    pub milestone_summary: Option<String>,
}

/// Shared context-store key prefix for campaign memory entries.
#[must_use]
pub fn campaign_context_prefix(campaign_id: &str) -> String {
    format!("campaign:{campaign_id}:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_tier_progression_is_linear() {
        assert_eq!(
            ReconstructionBenchmarkTier::IssueRepair.next(),
            Some(ReconstructionBenchmarkTier::SubsystemRegen)
        );
        assert_eq!(ReconstructionBenchmarkTier::RepoRegen.next(), None);
    }

    #[test]
    fn evidence_score_weights_compile_and_tests() {
        let e = ReconstructionEvidence {
            compile_ok: true,
            targeted_tests_ok: true,
            contract_checks_ok: false,
            docs_ssot_ok: false,
            regression_checks_ok: false,
        };
        assert!(e.score() >= 0.60);
        assert!(!e.passes_gate());
    }

    #[test]
    fn verification_layer_score_counts_passed_layers() {
        let v = VerificationLayerStatus {
            structural_ok: true,
            behavioral_ok: true,
            contract_ok: false,
            docs_ssot_ok: false,
            grounding_ok: true,
        };
        assert_eq!(v.passed_layers(), 3);
        assert!(v.score() > 0.5);
    }
}
