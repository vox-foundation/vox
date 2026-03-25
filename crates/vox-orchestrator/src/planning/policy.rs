use crate::planning::{ExecutionPolicy, ReplanTrigger};

pub fn default_policy_for_goal(goal: &str) -> ExecutionPolicy {
    let mut policy = ExecutionPolicy::default();
    let gl = goal.to_ascii_lowercase();
    if gl.contains("test") {
        policy
            .replan_triggers
            .push(ReplanTrigger::TestFailureNewRegression);
    }
    if gl.contains("build") || gl.contains("compile") {
        policy
            .replan_triggers
            .push(ReplanTrigger::CompilerErrorUnresolved);
    }
    policy
}
