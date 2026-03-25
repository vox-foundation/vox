//! Interpreted workflow runner: plan → journal.

use serde_json::{Value, json};
use vox_compiler::hir::HirModule;

use super::plan::plan_workflow_activities;
use super::tracker::{DefaultTracker, WorkflowTracker};
use super::types::PopuliActivity;

/// Execute a planned workflow and append journal entries.
pub async fn interpret_workflow(
    hir: &HirModule,
    workflow_name: &str,
) -> anyhow::Result<Vec<Value>> {
    let mut tracker = DefaultTracker;
    interpret_workflow_durable(hir, workflow_name, &mut tracker).await
}

/// Execute a planned workflow with a durable state engine, returning journal entries.
pub async fn interpret_workflow_durable(
    hir: &HirModule,
    workflow_name: &str,
    tracker: &mut impl WorkflowTracker,
) -> anyhow::Result<Vec<Value>> {
    let plan = plan_workflow_activities(hir, workflow_name)?;
    let mut journal = Vec::new();
    tracker
        .on_workflow_started(workflow_name, plan.len())
        .await?;
    journal.push(json!({
        "event": "WorkflowStarted",
        "workflow": workflow_name,
        "steps": plan.len(),
    }));
    for (idx, step) in plan.iter().enumerate() {
        let activity_id = step
            .activity_id
            .clone()
            .unwrap_or_else(|| format!("{workflow_name}-{idx}"));

        if tracker
            .is_activity_completed(workflow_name, &activity_id)
            .await?
        {
            journal.push(json!({
                "event": "ActivitySkipped",
                "workflow": workflow_name,
                "activity": step.name,
                "activity_id": activity_id,
                "reason": "already completed in prior durable run",
            }));
            continue;
        }

        tracker
            .on_activity_started(workflow_name, &step.name, &activity_id)
            .await?;
        journal.push(json!({
            "event": "ActivityStarted",
            "workflow": workflow_name,
            "activity": step.name,
            "activity_id": activity_id,
        }));

        let entry = if step.mens {
            #[cfg(feature = "mens")]
            {
                let m = PopuliActivity {
                    name: step.name.clone(),
                    populi_op: step.populi_op,
                    timeout_ms: step.timeout_ms,
                    activity_id: activity_id.clone(),
                };
                super::populi::execute_populi_step(&m).await?
            }
            #[cfg(not(feature = "mens"))]
            {
                json!({
                    "event": "MeshActivitySkipped",
                    "activity": step.name,
                    "activity_id": activity_id,
                    "reason": "vox-workflow-runtime built without mens feature",
                })
            }
        } else {
            json!({
                "event": "LocalActivity",
                "activity": step.name,
                "activity_id": activity_id,
                "status": "noop",
            })
        };

        tracker
            .on_activity_completed(workflow_name, &step.name, &activity_id, &entry)
            .await?;
        journal.push(entry);

        journal.push(json!({
            "event": "ActivityCompleted",
            "workflow": workflow_name,
            "activity": step.name,
            "activity_id": activity_id,
        }));
    }
    tracker.on_workflow_completed(workflow_name).await?;
    journal.push(json!({
        "event": "WorkflowCompleted",
        "workflow": workflow_name,
    }));
    Ok(journal)
}
