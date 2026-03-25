use crate::config::OrchestratorConfig;
use crate::planning::{PlanningMode, PlanningStrategy, RouterEvaluation};

pub fn evaluate_goal(
    config: &OrchestratorConfig,
    goal: &str,
    mode: Option<PlanningMode>,
) -> RouterEvaluation {
    let mode = mode.unwrap_or(PlanningMode::Auto);
    let gl = goal.to_ascii_lowercase();
    let complexity = complexity_heuristic(goal);

    let forced = match mode {
        PlanningMode::Direct => Some(PlanningStrategy::ImmediateAct),
        PlanningMode::ForcePlan => Some(PlanningStrategy::SequentialDag),
        PlanningMode::WorkflowOnly => Some(PlanningStrategy::WorkflowHandoff),
        PlanningMode::Auto => None,
    };
    if let Some(strategy) = forced {
        return RouterEvaluation {
            strategy,
            complexity,
            confidence: 0.99,
            workflow_match: if strategy == PlanningStrategy::WorkflowHandoff {
                Some("auto-detected".to_string())
            } else {
                None
            },
            rationale: "strategy forced by planning_mode".to_string(),
        };
    }

    if !config.planning_enabled || !config.planning_router_enabled {
        return RouterEvaluation {
            strategy: PlanningStrategy::ImmediateAct,
            complexity,
            confidence: 0.95,
            workflow_match: None,
            rationale: "planning router disabled".to_string(),
        };
    }

    if gl.contains("workflow") || gl.contains("pipeline") {
        return RouterEvaluation {
            strategy: PlanningStrategy::WorkflowHandoff,
            complexity: complexity.max(6),
            confidence: 0.75,
            workflow_match: Some("workflow".to_string()),
            rationale: "goal mentions workflow-like execution".to_string(),
        };
    }

    if complexity <= 3 {
        return RouterEvaluation {
            strategy: PlanningStrategy::ImmediateAct,
            complexity,
            confidence: 0.8,
            workflow_match: None,
            rationale: "low complexity goal".to_string(),
        };
    }

    RouterEvaluation {
        strategy: PlanningStrategy::SequentialDag,
        complexity,
        confidence: 0.7,
        workflow_match: None,
        rationale: "multi-step goal benefits from decomposition".to_string(),
    }
}

fn complexity_heuristic(goal: &str) -> u8 {
    let words = goal.split_whitespace().count() as u8;
    if words <= 6 {
        2
    } else if words <= 16 {
        5
    } else if words <= 30 {
        7
    } else {
        9
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_mode_forces_immediate() {
        let cfg = OrchestratorConfig::default();
        let r = evaluate_goal(&cfg, "do one tiny edit", Some(PlanningMode::Direct));
        assert_eq!(r.strategy, PlanningStrategy::ImmediateAct);
    }

    #[test]
    fn workflow_keyword_routes_to_handoff() {
        let cfg = OrchestratorConfig {
            planning_enabled: true,
            planning_router_enabled: true,
            ..OrchestratorConfig::default()
        };
        let r = evaluate_goal(&cfg, "run this workflow pipeline and report", None);
        assert_eq!(r.strategy, PlanningStrategy::WorkflowHandoff);
    }
}
