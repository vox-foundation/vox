//! Plan-mode vs. ReAct mode decision trigger (D2).
//!
//! Decides whether to use ReAct (act-first) or PlanAndExecute (plan-first) based
//! on task signals. Thresholds mirror `contracts/orchestration/plan-mode-trigger.v1.yaml`.
//! All logic is pure: no async, no I/O.

use serde::{Deserialize, Serialize};

/// The execution mode chosen for this task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanModeDecision {
    /// Act directly using ReAct loop — no upfront planning step.
    React,
    /// Emit a structured plan first, then execute step by step.
    PlanAndExecute,
}

impl std::fmt::Display for PlanModeDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::React => write!(f, "react"),
            Self::PlanAndExecute => write!(f, "plan-and-execute"),
        }
    }
}

/// Signals used to pick execution mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanModeSignal {
    /// Task complexity 0–10.
    pub complexity: u8,
    /// Number of inter-task dependencies detected in the task graph.
    pub dependency_count: u32,
    /// How many distinct tool types the task description hints at.
    pub tool_hint_count: u32,
    /// Score in `[0, 1]` from the prior adequacy evaluator; 1.0 = prior plan fully adequate.
    pub prior_adequacy_score: f64,
}

impl Default for PlanModeSignal {
    fn default() -> Self {
        Self {
            complexity: 4,
            dependency_count: 0,
            tool_hint_count: 1,
            prior_adequacy_score: 1.0,
        }
    }
}

/// Thresholds loaded from contract YAML. Defaults mirror contract defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanModeTriggerConfig {
    /// Complexity ≥ this → PlanAndExecute.
    pub complexity_threshold: u8,
    /// Dependency count ≥ this → PlanAndExecute.
    pub dependency_threshold: u32,
    /// Tool hint count ≥ this → PlanAndExecute.
    pub tool_hint_threshold: u32,
    /// Prior adequacy score < this → PlanAndExecute (prior plan not reusable).
    pub prior_adequacy_threshold: f64,
}

impl Default for PlanModeTriggerConfig {
    fn default() -> Self {
        Self {
            complexity_threshold: 6,
            dependency_threshold: 3,
            tool_hint_threshold: 4,
            prior_adequacy_threshold: 0.60,
        }
    }
}

/// Pure plan-mode trigger.
pub struct PlanModeTrigger {
    config: PlanModeTriggerConfig,
}

impl PlanModeTrigger {
    pub fn new(config: PlanModeTriggerConfig) -> Self {
        Self { config }
    }

    /// Return the execution mode for the given signals.
    ///
    /// Any signal breaching its threshold selects `PlanAndExecute`; otherwise `React`.
    #[must_use]
    #[inline]
    pub fn decide(&self, signal: &PlanModeSignal) -> PlanModeDecision {
        if signal.complexity >= self.config.complexity_threshold
            || signal.dependency_count >= self.config.dependency_threshold
            || signal.tool_hint_count >= self.config.tool_hint_threshold
            || signal.prior_adequacy_score < self.config.prior_adequacy_threshold
        {
            PlanModeDecision::PlanAndExecute
        } else {
            PlanModeDecision::React
        }
    }
}

/// Metric payload emitted when a plan-mode decision is made.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanModeEvent {
    pub metric_type: &'static str,
    pub decision: String,
    pub complexity: u8,
    pub dependency_count: u32,
    pub tool_hint_count: u32,
    pub prior_adequacy_score: f64,
    pub session_id: Option<String>,
}

impl PlanModeEvent {
    pub fn new(decision: PlanModeDecision, signal: &PlanModeSignal, session_id: Option<String>) -> Self {
        Self {
            metric_type: vox_db::research_metrics_contract::METRIC_TYPE_PLAN_MODE_DECISION,
            decision: decision.to_string(),
            complexity: signal.complexity,
            dependency_count: signal.dependency_count,
            tool_hint_count: signal.tool_hint_count,
            prior_adequacy_score: signal.prior_adequacy_score,
            session_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trigger() -> PlanModeTrigger {
        PlanModeTrigger::new(PlanModeTriggerConfig::default())
    }

    #[test]
    fn low_complexity_simple_task_uses_react() {
        let t = trigger();
        let sig = PlanModeSignal {
            complexity: 3,
            dependency_count: 1,
            tool_hint_count: 2,
            prior_adequacy_score: 0.90,
        };
        assert_eq!(t.decide(&sig), PlanModeDecision::React);
    }

    #[test]
    fn high_complexity_triggers_plan_and_execute() {
        let t = trigger();
        let sig = PlanModeSignal {
            complexity: 8,
            ..Default::default()
        };
        assert_eq!(t.decide(&sig), PlanModeDecision::PlanAndExecute);
    }

    #[test]
    fn high_dependency_count_triggers_plan_and_execute() {
        let t = trigger();
        let sig = PlanModeSignal {
            complexity: 3,
            dependency_count: 3,
            tool_hint_count: 1,
            prior_adequacy_score: 0.90,
        };
        assert_eq!(t.decide(&sig), PlanModeDecision::PlanAndExecute);
    }

    #[test]
    fn high_tool_hint_count_triggers_plan_and_execute() {
        let t = trigger();
        let sig = PlanModeSignal {
            complexity: 3,
            dependency_count: 1,
            tool_hint_count: 4,
            prior_adequacy_score: 0.90,
        };
        assert_eq!(t.decide(&sig), PlanModeDecision::PlanAndExecute);
    }

    #[test]
    fn low_prior_adequacy_triggers_plan_and_execute() {
        let t = trigger();
        let sig = PlanModeSignal {
            complexity: 3,
            dependency_count: 1,
            tool_hint_count: 2,
            prior_adequacy_score: 0.50,
        };
        assert_eq!(t.decide(&sig), PlanModeDecision::PlanAndExecute);
    }

    #[test]
    fn at_exact_complexity_threshold_uses_plan_and_execute() {
        let t = trigger();
        let sig = PlanModeSignal {
            complexity: 6,
            dependency_count: 0,
            tool_hint_count: 0,
            prior_adequacy_score: 1.0,
        };
        assert_eq!(t.decide(&sig), PlanModeDecision::PlanAndExecute);
    }

    #[test]
    fn just_below_all_thresholds_uses_react() {
        let t = trigger();
        let sig = PlanModeSignal {
            complexity: 5,
            dependency_count: 2,
            tool_hint_count: 3,
            prior_adequacy_score: 0.61,
        };
        assert_eq!(t.decide(&sig), PlanModeDecision::React);
    }

    #[test]
    fn plan_mode_event_has_correct_metric_type() {
        let sig = PlanModeSignal::default();
        let ev = PlanModeEvent::new(PlanModeDecision::React, &sig, None);
        assert_eq!(ev.metric_type, "orch.plan.mode_decision");
    }
}
