//! P2-T2 acceptance: `workflow.version()` emits a WorkflowPatch journal event.
//!
//! Task 5.2 extends this with a SQL-resume acceptance test: `VoxDbTracker`
//! must persist the chosen patch version so a *second*, independently-run
//! workflow run (same `workflow_name`, different `run_id`) replays the same
//! decision instead of re-deciding it.

use std::sync::Arc;

use vox_compiler::hir::{HirModule, lower_module};
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_db::{DbConfig, VoxDb};
use vox_workflow_runtime::VoxDbTracker;
use vox_workflow_runtime::workflow::run::interpret_workflow_durable;
use vox_workflow_runtime::workflow::tracker::DefaultTracker;

fn hir_with_version() -> HirModule {
    let src = r#"
        workflow wf() to int {
            let v = workflow.version("split-step", 1, 2)
            return 0
        }
    "#;
    let module = parse(lex(src)).expect("parse");
    lower_module(&module)
}

#[tokio::test]
async fn workflow_patch_emits_journal_event_first_run() {
    let mut tracker = DefaultTracker;
    let journal = interpret_workflow_durable(&hir_with_version(), "wf", vec![], &mut tracker)
        .await
        .expect("run");

    let patch_event = journal
        .iter()
        .find(|v| v["event"].as_str() == Some("WorkflowPatch"))
        .expect("WorkflowPatch must be journaled");

    assert_eq!(patch_event["change_id"], "split-step");
    assert_eq!(patch_event["replayed"], serde_json::Value::Bool(false));
    assert_eq!(patch_event["version"], serde_json::json!(2));
    assert_eq!(patch_event["min_supported"], serde_json::json!(1));
    assert_eq!(patch_event["max_supported"], serde_json::json!(2));
}

#[tokio::test]
async fn sql_tracker_replays_a_recorded_patch() {
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let mut t1 = VoxDbTracker::new(db.clone(), "patch-sql");
    let j1 = interpret_workflow_durable(&hir_with_version(), "wf", vec![], &mut t1)
        .await
        .expect("first");
    assert!(
        j1.iter()
            .any(|e| e["event"] == "WorkflowPatch" && e["replayed"] == false)
    );

    let mut t2 = VoxDbTracker::new(db, "patch-sql-2");
    // Same workflow_name, new run still loads the patch by (workflow, change_id).
    let j2 = interpret_workflow_durable(&hir_with_version(), "wf", vec![], &mut t2)
        .await
        .expect("second");
    assert!(
        j2.iter()
            .any(|e| e["event"] == "WorkflowPatch" && e["replayed"] == true),
        "VoxDbTracker must persist patches; got {j2:#?}"
    );
}
