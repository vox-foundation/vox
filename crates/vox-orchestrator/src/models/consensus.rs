//! Multi-Model Consensus Fan-Out Engine (`ConsensusFanOutEngine`, `T3-CONSENSUS-QA-01`).
//!
//! Adapted from `lulkebit/t3-cloneathon` (`src/app/api/chat/consensus/route.ts`):
//! Executes `2..=4` distinct models in parallel and multiplexes `Start`, `Update`, `Complete`,
//! `Error`, and `Final` (`ConsensusStreamEvent`) events over a single event stream.

use serde::{Deserialize, Serialize};

/// Minimum number of models required for a consensus turn.
pub const MIN_CONSENSUS_MODELS: usize = 2;

/// Maximum number of models permitted in a single consensus fan-out turn.
pub const MAX_CONSENSUS_MODELS: usize = 4;

/// Lifecycle status of a single model leg inside a consensus turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsensusModelLegStatus {
    Pending,
    Streaming,
    Completed,
    Error,
}

/// Snapshot of one model's response inside a multi-model consensus turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsensusModelLegState {
    pub model_id: String,
    pub status: ConsensusModelLegStatus,
    pub content: String,
    pub reasoning: String,
    pub response_time_ms: Option<u64>,
    pub quality_score: Option<f64>,
    pub error_message: Option<String>,
}

/// Multiplexed SSE/WebSocket event emitted by `ConsensusFanOutEngine`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConsensusStreamEvent {
    /// Initial event listing all participating models.
    Start { models: Vec<String> },
    /// Incremental token or reasoning update for a single model leg.
    Update {
        model_id: String,
        content_delta: String,
        reasoning_delta: Option<String>,
    },
    /// A single model leg finished generating.
    Complete {
        model_id: String,
        content: String,
        response_time_ms: u64,
        quality_score: f64,
    },
    /// A single model leg failed while sibling legs continue.
    Error {
        model_id: String,
        error_message: String,
    },
    /// Final settlement event containing all `2..=4` model leg states.
    Final {
        responses: Vec<ConsensusModelLegState>,
        total_elapsed_ms: u64,
    },
}

/// Validation errors for `ConsensusFanOutEngine`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConsensusError {
    /// Model count is outside `2..=4`.
    #[error(
        "consensus requires between {MIN_CONSENSUS_MODELS} and {MAX_CONSENSUS_MODELS} models (got {0})"
    )]
    InvalidModelCount(usize),
    /// Duplicate model IDs were supplied.
    #[error("consensus requires distinct model IDs (duplicate: {0})")]
    DuplicateModelId(String),
}

/// Coordinator for multi-model (`2..=4`) parallel consensus execution and stream multiplexing.
#[derive(Debug, Clone)]
pub struct ConsensusFanOutEngine {
    models: Vec<String>,
    legs: Vec<ConsensusModelLegState>,
}

impl ConsensusFanOutEngine {
    /// Validate `models` (`2..=4` distinct IDs) and initialize leg states.
    pub fn new(models: Vec<String>) -> Result<Self, ConsensusError> {
        if !(MIN_CONSENSUS_MODELS..=MAX_CONSENSUS_MODELS).contains(&models.len()) {
            return Err(ConsensusError::InvalidModelCount(models.len()));
        }
        let mut seen = std::collections::HashSet::with_capacity(models.len());
        for m in &models {
            if !seen.insert(m.clone()) {
                return Err(ConsensusError::DuplicateModelId(m.clone()));
            }
        }
        let legs = models
            .iter()
            .map(|m| ConsensusModelLegState {
                model_id: m.clone(),
                status: ConsensusModelLegStatus::Pending,
                content: String::new(),
                reasoning: String::new(),
                response_time_ms: None,
                quality_score: None,
                error_message: None,
            })
            .collect();
        Ok(Self { models, legs })
    }

    /// Emit the initial `ConsensusStreamEvent::Start` event.
    #[must_use]
    pub fn start_event(&self) -> ConsensusStreamEvent {
        ConsensusStreamEvent::Start {
            models: self.models.clone(),
        }
    }

    /// Apply an incremental delta for `model_id` and return the corresponding `ConsensusStreamEvent::Update`.
    pub fn apply_delta(
        &mut self,
        model_id: &str,
        content_delta: &str,
        reasoning_delta: Option<&str>,
    ) -> Option<ConsensusStreamEvent> {
        let leg = self.legs.iter_mut().find(|l| l.model_id == model_id)?;
        leg.status = ConsensusModelLegStatus::Streaming;
        leg.content.push_str(content_delta);
        if let Some(r) = reasoning_delta {
            leg.reasoning.push_str(r);
        }
        Some(ConsensusStreamEvent::Update {
            model_id: model_id.to_string(),
            content_delta: content_delta.to_string(),
            reasoning_delta: reasoning_delta.map(str::to_string),
        })
    }

    /// Mark `model_id` as completed, automatically scoring its response with [`super::quality_analyzer::analyze_response_quality`].
    pub fn complete_leg(
        &mut self,
        model_id: &str,
        prompt: &str,
        response_time_ms: u64,
    ) -> Option<ConsensusStreamEvent> {
        let leg = self.legs.iter_mut().find(|l| l.model_id == model_id)?;
        leg.status = ConsensusModelLegStatus::Completed;
        leg.response_time_ms = Some(response_time_ms);
        let metrics = super::quality_analyzer::analyze_response_quality(prompt, &leg.content);
        leg.quality_score = Some(metrics.quality_score);
        Some(ConsensusStreamEvent::Complete {
            model_id: model_id.to_string(),
            content: leg.content.clone(),
            response_time_ms,
            quality_score: metrics.quality_score,
        })
    }

    /// Mark `model_id` as failed without aborting sibling legs.
    pub fn fail_leg(
        &mut self,
        model_id: &str,
        error_message: impl Into<String>,
    ) -> Option<ConsensusStreamEvent> {
        let leg = self.legs.iter_mut().find(|l| l.model_id == model_id)?;
        let msg = error_message.into();
        leg.status = ConsensusModelLegStatus::Error;
        leg.error_message = Some(msg.clone());
        Some(ConsensusStreamEvent::Error {
            model_id: model_id.to_string(),
            error_message: msg,
        })
    }

    /// Emit the terminal `ConsensusStreamEvent::Final` event containing all leg states.
    #[must_use]
    pub fn finalize(&self, total_elapsed_ms: u64) -> ConsensusStreamEvent {
        ConsensusStreamEvent::Final {
            responses: self.legs.clone(),
            total_elapsed_ms,
        }
    }

    /// Synthesize a unified consensus response combining the highest-quality leg's structure
    /// with key findings from all completed model legs (`T3-TOOLS-HARDENING-02`).
    #[must_use]
    pub fn synthesize_consensus_response(&self, prompt: &str) -> String {
        let completed: Vec<&ConsensusModelLegState> = self
            .legs
            .iter()
            .filter(|l| {
                l.status == ConsensusModelLegStatus::Completed && !l.content.trim().is_empty()
            })
            .collect();
        if completed.is_empty() {
            return format!("No consensus legs completed for prompt: {prompt}");
        }
        let mut best = completed[0];
        for leg in &completed[1..] {
            if leg.quality_score.unwrap_or(0.0) > best.quality_score.unwrap_or(0.0) {
                best = leg;
            }
        }
        let mut synthesis = format!(
            "### Unified Consensus Synthesis ({} models, lead: `{}`)\n\n{}\n\n#### Cross-Model Agreement & Perspectives\n",
            completed.len(),
            best.model_id,
            best.content.trim()
        );
        for leg in &completed {
            let preview_line = leg
                .content
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("");
            let cut = preview_line.floor_char_boundary(preview_line.len().min(160));
            synthesis.push_str(&format!(
                "- **`{}`** (quality {:.1}): {}\n",
                leg.model_id,
                leg.quality_score.unwrap_or(0.0),
                &preview_line[..cut]
            ));
        }
        synthesis
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ConsensusError, ConsensusFanOutEngine, ConsensusModelLegStatus, ConsensusStreamEvent,
    };

    #[test]
    fn consensus_engine_validates_2_to_4_models_and_multiplexes_events() {
        assert!(matches!(
            ConsensusFanOutEngine::new(vec!["m1".into()]),
            Err(ConsensusError::InvalidModelCount(1))
        ));
        assert!(matches!(
            ConsensusFanOutEngine::new(vec!["m1".into(), "m1".into()]),
            Err(ConsensusError::DuplicateModelId(_))
        ));

        let mut engine = ConsensusFanOutEngine::new(vec![
            "anthropic/claude-sonnet-4-5".into(),
            "openai/gpt-4o".into(),
            "mens-local-7b".into(),
        ])
        .expect("3 distinct models");

        assert!(matches!(
            engine.start_event(),
            ConsensusStreamEvent::Start { .. }
        ));
        engine.apply_delta(
            "anthropic/claude-sonnet-4-5",
            "First, we wire VoxIdAllocatorV2.",
            None,
        );
        let done_ev = engine
            .complete_leg(
                "anthropic/claude-sonnet-4-5",
                "How do we wire VoxIdAllocatorV2?",
                420,
            )
            .expect("complete leg");
        assert!(matches!(done_ev, ConsensusStreamEvent::Complete { .. }));
        engine.fail_leg("openai/gpt-4o", "429 rate limit");
        engine.apply_delta(
            "mens-local-7b",
            "Therefore, derive uses percent encoding.",
            None,
        );
        engine.complete_leg("mens-local-7b", "How do we wire VoxIdAllocatorV2?", 180);

        let ConsensusStreamEvent::Final { responses, .. } = engine.finalize(450) else {
            panic!("expected Final event");
        };
        assert_eq!(responses.len(), 3);
        assert_eq!(responses[0].status, ConsensusModelLegStatus::Completed);
        assert_eq!(responses[1].status, ConsensusModelLegStatus::Error);
        assert_eq!(responses[2].status, ConsensusModelLegStatus::Completed);
    }
}
