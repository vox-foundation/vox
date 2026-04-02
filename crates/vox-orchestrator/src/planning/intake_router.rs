use crate::config::OrchestratorConfig;
use crate::planning::plan_adequacy::estimate_goal_word_complexity;
use crate::planning::{PlanningMode, PlanningStrategy, RouterEvaluation};

pub fn evaluate_goal(
    config: &OrchestratorConfig,
    goal: &str,
    mode: Option<PlanningMode>,
) -> RouterEvaluation {
    let mode = mode.unwrap_or(PlanningMode::Auto);
    let gl = goal.to_ascii_lowercase();
    let search_plan = vox_db::heuristic_search_plan(goal, false, None);
    let mut complexity = estimate_goal_word_complexity(goal);
    if matches!(
        search_plan.intent,
        vox_db::SearchIntent::BroadResearch | vox_db::SearchIntent::RepoStructure
    ) {
        complexity = complexity.max(6);
    }

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
            rationale: format!(
                "goal mentions workflow-like execution; search intent {:?}",
                search_plan.intent
            ),
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
        rationale: format!(
            "multi-step goal benefits from decomposition; search intent {:?}",
            search_plan.intent
        ),
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
