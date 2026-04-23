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

/// Compact prompt-compiled contract for repository reconstruction campaigns.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RepoReconstructionSpec {
    /// Stable campaign id (same value used by task hints and lineage payloads).
    pub campaign_id: String,
    /// Human-provided objective (typically short prompt / tweet-sized request).
    pub objective: String,
    /// Optional constraints (runtime, security, language/version rails).
    #[serde(default)]
    pub constraints: Vec<String>,
    /// Optional acceptance checks that define "done".
    #[serde(default)]
    pub acceptance_tests: Vec<String>,
    /// Architecture assumptions captured during planning/research.
    #[serde(default)]
    pub architecture_assumptions: Vec<String>,
    /// Planned shard boundaries for fan-out execution.
    #[serde(default)]
    pub shard_boundaries: Vec<ReconstructionShardBoundary>,
}

/// One shard boundary in a reconstruction campaign.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReconstructionShardBoundary {
    /// Stable shard id scoped to [`RepoReconstructionSpec::campaign_id`].
    pub shard_id: String,
    /// Human-readable shard summary.
    pub summary: String,
    /// Paths targeted by this shard (glob-like patterns or exact paths).
    #[serde(default)]
    pub path_hints: Vec<String>,
    /// Optional role preference for routing (planner/builder/verifier/...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<AgentExecutionRole>,
}

/// Artifact categories for retrieval-first reconstruction state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ReconstructionArtifactKind {
    RepoSkeleton,
    CrateBoundary,
    SymbolGraph,
    ApiContract,
    DocsFact,
    TestExpectation,
    Hypothesis,
    Contradiction,
    PatchAttempt,
    VerificationEvidence,
    PlannerBrief,
}

impl ReconstructionArtifactKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RepoSkeleton => "repo_skeleton",
            Self::CrateBoundary => "crate_boundary",
            Self::SymbolGraph => "symbol_graph",
            Self::ApiContract => "api_contract",
            Self::DocsFact => "docs_fact",
            Self::TestExpectation => "test_expectation",
            Self::Hypothesis => "hypothesis",
            Self::Contradiction => "contradiction",
            Self::PatchAttempt => "patch_attempt",
            Self::VerificationEvidence => "verification_evidence",
            Self::PlannerBrief => "planner_brief",
        }
    }
}

/// Durable artifact row payload used by campaign storage and retrieval.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReconstructionArtifactRecord {
    /// Campaign that owns this artifact.
    pub campaign_id: String,
    /// Stable artifact id (for references from tasks and verifier outputs).
    pub artifact_id: String,
    /// Artifact category.
    pub kind: ReconstructionArtifactKind,
    /// Opaque artifact payload (JSON object recommended).
    pub payload: serde_json::Value,
    /// Optional tags to accelerate retrieval by lane (e.g. `crate:vox-orchestrator`).
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional source summary (tool/action that produced this artifact).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
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
    /// Optional typed verifier failures emitted by the repair gate.
    #[serde(default)]
    pub failures: Vec<VerificationFailureKind>,
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

/// Typed verifier failure classes used to generate repair tasks.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum VerificationFailureKind {
    Compile,
    Tests,
    Contract,
    DocsSsot,
    Regression,
    Grounding,
    Contradiction,
    Unknown,
}

/// Campaign-level benchmark KPIs for the reconstruction ladder.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReconstructionBenchmarkKpis {
    /// Wall clock latency from prompt ingest to latest checkpoint.
    pub elapsed_ms: u64,
    /// Fraction [0.0, 1.0] of autonomous recovery attempts that succeeded.
    pub autonomous_recovery_rate: f32,
    /// Fraction [0.0, 1.0] of regenerated files passing validation.
    pub regenerated_file_success_rate: f32,
    /// Cost in USD-like units per successful reconstruction step.
    pub cost_per_success_step: f64,
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
            failures: Vec::new(),
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

    #[test]
    fn artifact_kind_strings_are_stable() {
        assert_eq!(
            ReconstructionArtifactKind::VerificationEvidence.as_str(),
            "verification_evidence"
        );
    }
}
