#![allow(missing_docs)]

use serde_json::json;
use std::sync::Arc;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_db::{DbConfig, VoxDb};
use vox_workflow_runtime::VoxDbTracker;
use vox_workflow_runtime::workflow::interpret_workflow_durable;
use vox_workflow_runtime::workflow::waker::WorkflowWaker;

#[tokio::test]
async fn wait_parks_and_waker_resumes_after_backdate() {
    const SRC: &str = r#"
activity after(n: int) to int { return n + 1 }
workflow delayed(n: int) to int {
    workflow_wait(3_600_000)
    return after(n)
}
"#;
    let hir = lower_module(&parse(lex(SRC)).expect("parses"));
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let mut tracker = VoxDbTracker::new(db.clone(), "timer-1");
    let journal = interpret_workflow_durable(&hir, "delayed", vec![json!(4)], &mut tracker)
        .await
        .expect("park is not a failure");
    assert!(
        journal
            .iter()
            .any(|e| e["event"] == "WorkflowParked" || e["event"] == "TimerScheduled"),
        "must emit a park/timer event; got {journal:#?}"
    );
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../contracts/workflow/workflow-journal.v1.schema.json"
    ))
    .expect("journal schema");
    let validator = jsonschema::validator_for(&schema).expect("valid journal schema");
    for event in &journal {
        validator
            .validate(event)
            .unwrap_or_else(|err| panic!("timer event must conform: {err}; event={event}"));
    }
    let status = db.workflow_run_status("timer-1").await.expect("status");
    assert_eq!(status, "parked");
    let owner = db.workflow_run_lease_owner("timer-1").await.expect("lease");
    assert!(owner.is_none(), "park must clear lease_owner");

    db.workflow_run_backdate_wake_at_ms("timer-1", 0)
        .await
        .expect("backdate");
    let waker = WorkflowWaker::new(db.clone());
    let resumed = waker.tick().await.expect("tick");
    assert_eq!(resumed, vec!["timer-1".to_string()]);

    let mut resume = VoxDbTracker::new(db, "timer-1");
    let journal = interpret_workflow_durable(&hir, "delayed", vec![], &mut resume)
        .await
        .expect("waker args come from args_json, vec![] is a runner bug if this loses n");
    let done = journal
        .iter()
        .rev()
        .find(|e| e["event"] == "WorkflowCompleted")
        .unwrap();
    assert_eq!(done["return_value"], json!(5));
}
