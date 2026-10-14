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

#[tokio::test]
async fn early_resume_preserves_persisted_wake_at_ms() {
    const SRC: &str = r#"
activity after(n: int) to int { return n + 1 }
workflow delayed(n: int) to int {
    workflow_wait(3_600_000)
    return after(n)
}
"#;
    let hir = lower_module(&parse(lex(SRC)).expect("parses"));
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let mut tracker = VoxDbTracker::new(db.clone(), "timer-early");
    let first = interpret_workflow_durable(&hir, "delayed", vec![json!(4)], &mut tracker)
        .await
        .expect("first park is not a failure");
    let first_wake = first
        .iter()
        .find(|e| e["event"] == "TimerScheduled")
        .and_then(|e| e["wake_at_ms"].as_i64())
        .expect("first park must record wake_at_ms");
    let persisted = db
        .workflow_run_wake_at_ms("timer-early")
        .await
        .expect("read wake_at_ms");
    assert_eq!(persisted, Some(first_wake));

    // Wall clock must advance so a naive `now + deadline` rewrite cannot
    // accidentally match the original millisecond.
    const EARLY_RESUME_CLOCK_ADVANCE: std::time::Duration = std::time::Duration::from_millis(25);
    tokio::time::sleep(EARLY_RESUME_CLOCK_ADVANCE).await;

    let mut resume = VoxDbTracker::new(db.clone(), "timer-early");
    let second = interpret_workflow_durable(&hir, "delayed", vec![], &mut resume)
        .await
        .expect("early resume must re-park, not fail");
    assert!(
        second
            .iter()
            .any(|e| e["event"] == "WorkflowParked" || e["event"] == "TimerScheduled"),
        "early resume must park again; got {second:#?}"
    );
    assert!(
        second.iter().all(|e| e["event"] != "WorkflowCompleted"),
        "timer is not due; must not complete; got {second:#?}"
    );
    let persisted_after = db
        .workflow_run_wake_at_ms("timer-early")
        .await
        .expect("read wake_at_ms after resume");
    assert_eq!(
        persisted_after,
        Some(first_wake),
        "early resume must not recompute now+deadline"
    );
}
