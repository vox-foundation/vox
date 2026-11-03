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

fn started_activity_ids(journal: &[serde_json::Value], name: &str) -> Vec<String> {
    journal
        .iter()
        .filter(|e| e["event"].as_str() == Some("ActivityStarted"))
        .filter(|e| e["activity"].as_str() == Some(name))
        .filter_map(|e| e["activity_id"].as_str().map(str::to_string))
        .collect()
}

#[tokio::test]
async fn helper_calls_use_the_enclosing_function_in_activity_identity() {
    const SRC: &str = r#"
activity charge(n: int) to int { return n }
fn first() to int { return charge(1) }
fn second() to int { return charge(2) }
workflow wf() to int { return first() + second() }
"#;
    let mut tracker = DefaultTracker;
    let journal = interpret_workflow_durable(&hir(SRC), "wf", vec![], &mut tracker)
        .await
        .expect("workflow completes");
    let ids = started_activity_ids(&journal, "charge");
    assert_eq!(ids.len(), 2, "both helper calls must execute: {journal:#?}");
    assert_ne!(
        ids[0], ids[1],
        "different enclosing functions must produce different ids"
    );
}

// Distinct callees may share per-callee ordinal 0. The BLAKE3 input
// includes the callee name so they still get distinct activity ids.
#[tokio::test]
async fn aliased_callees_of_the_same_activity_get_distinct_ids() {
    const SRC: &str = r#"
activity step(n: int) to int { return n }
workflow wf() to int {
    let a = step
    let b = step
    return a(1) + b(2)
}
"#;
    let mut tracker = DefaultTracker;
    let journal = interpret_workflow_durable(&hir(SRC), "wf", vec![], &mut tracker)
        .await
        .expect("workflow completes");
    let ids = started_activity_ids(&journal, "step");
    assert_eq!(ids.len(), 2, "both aliases must execute: {journal:#?}");
    assert_ne!(
        ids[0], ids[1],
        "two aliases of the same activity must not share an activity_id: {journal:#?}"
    );
}

#[tokio::test]
async fn same_helper_called_twice_gets_distinct_ids() {
    const SRC: &str = r#"
activity charge(n: int) to int { return n }
fn helper() to int { return charge(1) }
workflow wf() to int {
    let a = helper()
    let b = helper()
    return a + b
}
"#;
    let mut tracker = DefaultTracker;
    let journal = interpret_workflow_durable(&hir(SRC), "wf", vec![], &mut tracker)
        .await
        .expect("workflow completes");
    let ids = started_activity_ids(&journal, "charge");
    assert_eq!(ids.len(), 2, "both helper calls must execute: {journal:#?}");
    assert_ne!(
        ids[0], ids[1],
        "two invocations of the same helper must not share an activity_id: {journal:#?}"
    );
}

#[tokio::test]
async fn helper_outside_a_loop_does_not_collide_with_first_iteration() {
    const SRC: &str = r#"
activity charge(n: int) to int { return n }
fn helper() to int { return charge(1) }
workflow wf() to int {
    let a = helper()
    let s = 0
    let n = 0
    while n < 1 {
        s = helper()
        n = n + 1
    }
    return a + s
}
"#;
    let mut tracker = DefaultTracker;
    let journal = interpret_workflow_durable(&hir(SRC), "wf", vec![], &mut tracker)
        .await
        .expect("workflow completes");
    let ids = started_activity_ids(&journal, "charge");
    assert_eq!(
        ids.len(),
        2,
        "outside-loop and first-iteration helper calls must both execute: {journal:#?}"
    );
    assert_ne!(
        ids[0], ids[1],
        "empty iteration stack must not hash as [0]: {journal:#?}"
    );
}

#[tokio::test]
async fn distinct_activity_callees_in_the_same_function_get_distinct_ids() {
    const SRC: &str = r#"
activity fresh() to int { return 1 }
activity old() to int { return 2 }
workflow wf() to int {
    let a = fresh()
    let b = old()
    return a + b
}
"#;
    let mut tracker = DefaultTracker;
    let journal = interpret_workflow_durable(&hir(SRC), "wf", vec![], &mut tracker)
        .await
        .expect("workflow completes");
    let fresh_ids = started_activity_ids(&journal, "fresh");
    let old_ids = started_activity_ids(&journal, "old");
    assert_eq!(fresh_ids.len(), 1, "fresh() must execute: {journal:#?}");
    assert_eq!(old_ids.len(), 1, "old() must execute: {journal:#?}");
    assert_ne!(
        fresh_ids[0], old_ids[0],
        "two different activities called in the same enclosing function and \
         iteration must not derive the same activity_id: {journal:#?}"
    );
}

#[tokio::test]
async fn nested_loop_activity_identity_uses_the_full_iteration_stack() {
    const SRC: &str = r#"
activity charge(n: int) to int { return n }
workflow wf() to int {
    let outer = 0
    while outer < 2 {
        let inner = 0
        while inner < 2 {
            let _ = charge(outer * 10 + inner)
            inner = inner + 1
        }
        outer = outer + 1
    }
    return 0
}
"#;
    let mut tracker = DefaultTracker;
    let journal = interpret_workflow_durable(&hir(SRC), "wf", vec![], &mut tracker)
        .await
        .expect("workflow completes");
    let ids = started_activity_ids(&journal, "charge");
    assert_eq!(
        ids.len(),
        4,
        "all nested iterations must execute: {journal:#?}"
    );
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), 4, "nested iterations must have unique ids");
}

#[tokio::test]
async fn while_condition_activity_uses_the_current_iteration() {
    const SRC: &str = r#"
activity more(n: int) to bool { return n < 2 }
workflow wf() to int {
    let n = 0
    while more(n) {
        n = n + 1
    }
    return n
}
"#;
    let mut tracker = DefaultTracker;
    let journal = interpret_workflow_durable(&hir(SRC), "wf", vec![], &mut tracker)
        .await
        .expect("workflow completes");
    let ids = started_activity_ids(&journal, "more");
    assert_eq!(ids.len(), 3, "condition runs for iterations 0, 1, and 2");
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(
        unique.len(),
        3,
        "each while-condition evaluation must use its current iteration"
    );
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
    let inner = VoxDbTracker::with_owner(db.clone(), "upgrade-1", "upgrade-owner", 30_000);
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

    let mut t2 = VoxDbTracker::with_owner(db, "upgrade-1", "upgrade-owner", 30_000);
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

#[tokio::test]
async fn explicit_activity_id_replays_on_resume() {
    const SRC: &str = r#"
activity step() to int { return 1 }
workflow wf() to int {
    let n = step() with { activity_id: "same-on-resume" }
    workflow_wait_signal("continue")
    return n
}
"#;
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let mut first = VoxDbTracker::with_owner(db.clone(), "explicit-resume", "resume-owner", 30_000);
    let journal = interpret_workflow_durable(&hir(SRC), "wf", vec![], &mut first)
        .await
        .expect("first invocation parks");
    assert!(
        journal
            .iter()
            .any(|event| event["event"] == "WorkflowParked")
    );

    let mut resumed = VoxDbTracker::with_owner(db, "explicit-resume", "resume-owner", 30_000);
    let resumed_journal = interpret_workflow_durable(&hir(SRC), "wf", vec![], &mut resumed)
        .await
        .expect("resume must replay the completed explicit id, then park again");
    assert!(
        resumed_journal
            .iter()
            .any(|event| event["event"] == "ActivityReplayed"
                && event["activity_id"] == "same-on-resume"),
        "expected replay of explicit id, got {resumed_journal:#?}"
    );
    assert!(
        resumed_journal
            .iter()
            .any(|event| event["event"] == "WorkflowParked"),
        "signal still absent, so resume must park: {resumed_journal:#?}"
    );
}
