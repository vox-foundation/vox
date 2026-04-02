//! Campaign-level shard scheduling helpers above file-affinity routing.

use crate::types::FileAffinity;

/// High-level campaign mode selected from objective hints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignSchedulingMode {
    ResearchFirst,
    ScaffoldFirst,
    ContractFirst,
    RepairFirst,
}

impl CampaignSchedulingMode {
    #[must_use]
    pub fn as_tag(self) -> &'static str {
        match self {
            Self::ResearchFirst => "research_first",
            Self::ScaffoldFirst => "scaffold_first",
            Self::ContractFirst => "contract_first",
            Self::RepairFirst => "repair_first",
        }
    }
}

/// Deterministic campaign schedule output used by submit helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignSchedulePlan {
    pub mode: CampaignSchedulingMode,
    pub shard_order: Vec<usize>,
}

/// Lightweight scheduler: infer mode from objective text and produce deterministic shard order.
pub struct CampaignScheduler;

impl CampaignScheduler {
    #[must_use]
    pub fn plan(objective: &str, shard_count: usize) -> CampaignSchedulePlan {
        let mode = Self::infer_mode(objective);
        let mut shard_order: Vec<usize> = (0..shard_count).collect();
        match mode {
            CampaignSchedulingMode::ResearchFirst => {
                // Keep stable order so early shards feed context before broad fan-out.
            }
            CampaignSchedulingMode::ScaffoldFirst => {
                // Stable ascending order favors deterministic repository skeleton layout.
            }
            CampaignSchedulingMode::ContractFirst => {
                // Front-load likely API/contract shards by lexical stability (lower index first).
            }
            CampaignSchedulingMode::RepairFirst => {
                // Start from tail shards to prioritize likely "hot" changed areas in iterative repairs.
                shard_order.reverse();
            }
        }
        CampaignSchedulePlan { mode, shard_order }
    }

    #[must_use]
    pub fn infer_mode(objective: &str) -> CampaignSchedulingMode {
        let o = objective.to_ascii_lowercase();
        if o.contains("repair") || o.contains("fix") || o.contains("regression") {
            CampaignSchedulingMode::RepairFirst
        } else if o.contains("contract") || o.contains("openapi") || o.contains("schema") {
            CampaignSchedulingMode::ContractFirst
        } else if o.contains("scaffold") || o.contains("bootstrap") || o.contains("skeleton") {
            CampaignSchedulingMode::ScaffoldFirst
        } else {
            CampaignSchedulingMode::ResearchFirst
        }
    }

    /// Optional helper to score shard rough size for future budget-aware scheduling.
    #[must_use]
    pub fn shard_weight(manifest: &[FileAffinity]) -> usize {
        manifest
            .iter()
            .map(|fa| if fa.path.extension().is_some() { 2 } else { 1 })
            .sum()
    }
}
