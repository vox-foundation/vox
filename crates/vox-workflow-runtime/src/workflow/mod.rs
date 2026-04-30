//! Interpreted workflow planning and execution (internal).

pub mod plan;
pub mod populi;
pub mod run;
pub mod tracker;
pub mod types;

pub use plan::{plan_workflow_activities, plan_workflow_replay_ir};
#[cfg(feature = "mens")]
pub use populi::execute_populi_step;
pub use run::{WORKFLOW_JOURNAL_VERSION, interpret_workflow, interpret_workflow_durable};
pub use tracker::{DefaultTracker, WorkflowTracker};
pub use types::{PlannedActivity, PopuliActivity, PopuliHttpOp, ReplayNode, WorkflowReplayIr};

#[cfg(test)]
mod tests {
    // HirWorkflow / HirActivity types were retired in TASK-2.6 Path B.
    // The workflow planner is now a stub (plan_workflow_replay_ir returns empty IR).
    // Tests that constructed HirModule.workflows directly are removed.
    // Integration tests live in the golden corpus once @durable fn is introduced (TASK-4.1).

}
