use crate::types::AgentId;
use serde::{Deserialize, Serialize};

/// Result of a judge model's evaluation of another agent's output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeVerdict {
    pub task_id: String,
    pub target_agent_id: AgentId,
    pub judge_agent_id: AgentId,
    pub score: f64, // [0, 1]
    pub reason: String,
    pub identifies_hallucination: bool,
}

/// A policy for when to invoke a judge model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JudgePolicy {
    /// Never invoke a judge.
    Never,
    /// Invoke if Socrates confidence is below threshold.
    LowConfidence { threshold: f64 },
    /// Invoke for every Nth iteration.
    Sampling { rate: f64 },
    /// Always invoke (High stakes).
    Always,
}

impl JudgePolicy {
    pub fn to_verdict(&self) -> JudgeVerdict {
        JudgeVerdict {
            task_id: "0".to_string(),
            target_agent_id: AgentId(0),
            judge_agent_id: AgentId(0),
            score: 1.0,
            reason: "Default verdict from policy".to_string(),
            identifies_hallucination: false,
        }
    }
}

pub struct JudgeModel {
    pub policy: JudgePolicy,
}

impl JudgeModel {
    pub fn new(policy: JudgePolicy) -> Self {
        Self { policy }
    }

    /// Determine if a judge should be invoked for the given confidence.
    pub fn should_judge(&self, confidence: f64) -> bool {
        match self.policy {
            JudgePolicy::Never => false,
            JudgePolicy::LowConfidence { threshold } => confidence < threshold,
            JudgePolicy::Always => true,
            JudgePolicy::Sampling { rate } => {
                // Simplified sampling logic
                rand::random::<f64>() < rate
            }
        }
    }
}
