#![allow(missing_docs)]

use std::sync::Arc;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_db::{DbConfig, VoxDb};
use vox_workflow_runtime::VoxDbTracker;
use vox_workflow_runtime::workflow::counting_tracker::CountingTracker;
use vox_workflow_runtime::workflow::{DefaultTracker, interpret_workflow_durable};

fn hir(src: &str) -> vox_compiler::hir::HirModule {
    lower_module(&parse(lex(src)).expect("parses"))
}

fn activity_ids(journal: &[serde_json::Value], name: &str) -> Vec<String> {
    journal
        .iter()
        .filter(|e| e["activity"].as_str() == Some(name))
        .filter_map(|e| e["activity_id"].as_str().map(str::to_string))
        .collect()
}

#[tokio::test]
async fn inserted_leading_activity_does_not_rename_the_old_id() {
    const V1: &str = r#"
activity old() to int { return 1 }
workflow wf() to int { return old() }
"#;
    const V2: &str = r#"
activity fresh() to int { return 0 }
activity old() to int { return 1 }
workflow wf() to int {
    let _ = fresh()
    return old()
}
"#;
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let inner = VoxDbTracker::new(db.clone(), "upgrade-1");
    // Crash after `old` completed so the run is in-flight, not completed.
    let mut t1 = CountingTracker::new(inner, /* fail_after */ 9);
    let first = interpret_workflow_durable(&hir(V1), "wf", vec![], &mut t1).await;
    assert!(first.is_err(), "V1 must stop before WorkflowCompleted");
    let v1_journal = first.err().map(|e| format!("{e:#}")).unwrap_or_default();
    let _ = v1_journal;
    // Re-read persisted rows for `old` rather than depending on a completed journal.
    let v1_ids = db
        .workflow_activity_ids("upgrade-1", "old")
        .await
        .expect("v1 ids");
    assert_eq!(v1_ids.len(), 1, "V1 recorded one old activity");

    let mut t2 = VoxDbTracker::new(db, "upgrade-1");
    t2.expire_lease_for_test().await.expect("expire");
    let j2 = interpret_workflow_durable(&hir(V2), "wf", vec![], &mut t2)
        .await
        .expect("resume V2 on the in-flight run");
    let v2_old = activity_ids(&j2, "old");
    assert_eq!(
        v2_old.first(),
        v1_ids.first(),
        "inserting `fresh` must not rename `old`; V1={v1_ids:?} V2={v2_old:?}"
    );
    assert!(
        j2.iter().any(|e| e["activity"] == "fresh"),
        "V2 must also run the new leading activity"
    );
}

#[tokio::test]
async fn completed_run_resume_is_refused() {
    const SRC: &str = r#"
activity old() to int { return 1 }
workflow wf() to int { return old() }
"#;
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let mut t1 = VoxDbTracker::new(db.clone(), "done-1");
    let _ = interpret_workflow_durable(&hir(SRC), "wf", vec![], &mut t1)
        .await
        .expect("completes");
    let mut t2 = VoxDbTracker::new(db, "done-1");
    let err = interpret_workflow_durable(&hir(SRC), "wf", vec![], &mut t2)
        .await
        .expect_err("completed run must refuse");
    assert!(format!("{err:#}").contains("completed"), "got {err:#}");
}

#[tokio::test]
async fn duplicate_explicit_activity_id_is_an_error() {
    const SRC: &str = r#"
activity step(n: int) to int { return n }
workflow wf() to int {
    let a = step(1) with { activity_id: "same" }
    let b = step(2) with { activity_id: "same" }
    return a + b
}
"#;
    let mut tracker = DefaultTracker;
    let err = interpret_workflow_durable(&hir(SRC), "wf", vec![], &mut tracker)
        .await
        .expect_err("duplicate explicit id");
    assert!(format!("{err:#}").contains("same"));
}
