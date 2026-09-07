#![allow(missing_docs)]

use std::sync::Arc;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_db::{DbConfig, VoxDb};
use vox_workflow_runtime::VoxDbTracker;
use vox_workflow_runtime::workflow::interpret_workflow_durable;

#[tokio::test]
async fn missing_signal_parks_the_run() {
    const SRC: &str = r#"
activity after() to int { return 1 }
workflow gated() to int {
    workflow_wait_signal("go")
    return after()
}
"#;
    let hir = lower_module(&parse(lex(SRC)).expect("parses"));
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let mut tracker = VoxDbTracker::new(db.clone(), "sig-1");
    let journal = interpret_workflow_durable(&hir, "gated", vec![], &mut tracker)
        .await
        .expect("park is not failed");
    assert!(
        journal
            .iter()
            .any(|e| e["event"] == "SignalAwaited" || e["event"] == "WorkflowParked"),
        "got {journal:#?}"
    );
    let status = db.workflow_run_status("sig-1").await.expect("status");
    assert_eq!(status, "parked", "absent signal must park, not fail");
}

#[tokio::test]
async fn late_signal_after_park_completes() {
    const SRC: &str = r#"
activity after() to int { return 1 }
workflow gated() to int {
    workflow_wait_signal("go")
    return after()
}
"#;
    let hir = lower_module(&parse(lex(SRC)).expect("parses"));
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let mut tracker = VoxDbTracker::new(db.clone(), "sig-2");
    let _ = interpret_workflow_durable(&hir, "gated", vec![], &mut tracker)
        .await
        .expect("park");
    db.record_workflow_signal("sig-2", "go", "{}")
        .await
        .expect("signal");
    let mut resume = VoxDbTracker::new(db, "sig-2");
    resume.expire_lease_for_test().await.ok();
    let journal = interpret_workflow_durable(&hir, "gated", vec![], &mut resume)
        .await
        .expect("resume");
    assert!(journal.iter().any(|e| e["event"] == "WorkflowCompleted"));
}
