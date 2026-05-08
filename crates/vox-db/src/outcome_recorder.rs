//! Unified LLM outcome ingestion — one transaction for scoreboard + Socrates surface when both are present.
//!
//! See plan: Centralized Model Selection SSOT — single write path for `llm_interactions` / `model_scoreboard`
//! and `socrates_surface` (+ trust observations) to avoid partial failures.

use serde_json::Value;
use vox_orchestrator_types::socrates_policy::RiskDecision;

use crate::store::types::{ModelOutcome, StoreError};

/// Row ids returned from [`crate::VoxDb::record_unified_llm_turn`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UnifiedLlmTurnRowIds {
    pub llm_interaction_rowid: i64,
    pub socrates_research_metric_rowid: Option<i64>,
}

impl crate::VoxDb {
    /// Record an LLM turn and optional Socrates surface telemetry in **one** transaction.
    ///
    /// When `socrates` is `None`, this is equivalent to [`Self::record_llm_outcome`], but still runs
    /// inside a short transaction for API symmetry.
    pub async fn record_unified_llm_turn(
        &self,
        outcome: ModelOutcome<'_>,
        socrates: Option<(
            String,
            String,
            RiskDecision,
            f64,
            f64,
            Option<String>,
            Option<Value>,
        )>,
    ) -> Result<UnifiedLlmTurnRowIds, StoreError> {
        self.transaction(async {
            let llm_interaction_rowid = self.record_llm_outcome(outcome).await?;
            let socrates_research_metric_rowid = if let Some((
                repository_id,
                surface,
                decision,
                confidence_estimate,
                contradiction_ratio,
                model_used,
                retrieval,
            )) = socrates
            {
                Some(
                    self.record_socrates_surface_event(
                        repository_id.as_str(),
                        surface.as_str(),
                        decision,
                        confidence_estimate,
                        contradiction_ratio,
                        model_used.as_deref(),
                        retrieval,
                    )
                    .await?,
                )
            } else {
                None
            };
            Ok(UnifiedLlmTurnRowIds {
                llm_interaction_rowid,
                socrates_research_metric_rowid,
            })
        })
        .await
    }
}
