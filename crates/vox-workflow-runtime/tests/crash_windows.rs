#![allow(missing_docs)]

use serde_json::json;
use std::sync::Arc;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_db::{DbConfig, VoxDb};
use vox_workflow_runtime::VoxDbTracker;
use vox_workflow_runtime::workflow::counting_tracker::CountingTracker;
use vox_workflow_runtime::workflow::interpret_workflow_durable;

fn hir(src: &str) -> vox_compiler::hir::HirModule {
    lower_module(&parse(lex(src)).expect("parses"))
}

const SRC: &str = r#"
@uses(fs)
fn write_file(path: str, contents: str) {
    std.fs.write(path, contents)
}

activity write_one(path: str) to int {
    write_file(path, "one")
    return 1
}

activity write_two(path: str) to int {
    write_file(path, "two")
    return 2
}

workflow pair(dir: str) to int {
    let _a = write_one(dir + "/one.txt")
    let _b = write_two(dir + "/two.txt")
    return 3
}
"#;

#[tokio::test]
async fn window_a_kill_after_first_completed() {
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = tmp.path().to_string_lossy().to_string();
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let inner = VoxDbTracker::new(db.clone(), "crash-a");
    // fail_after counts tracker calls; tune so the first on_activity_completed
    // succeeds and the next tracker call (second start) faults.
    let mut tracker = CountingTracker::new(inner, /* fail_after */ 4);
    let first =
        interpret_workflow_durable(&hir(SRC), "pair", vec![json!(dir.clone())], &mut tracker).await;
    assert!(first.is_err(), "must crash after activity 1 completed");
    let one = std::fs::read_to_string(tmp.path().join("one.txt")).expect("one written");
    assert_eq!(one, "one");
    assert!(
        !tmp.path().join("two.txt").exists(),
        "activity 2 must not have run"
    );

    let mut resume = VoxDbTracker::new(db, "crash-a");
    resume.expire_lease_for_test().await.expect("expire");
    let journal = interpret_workflow_durable(&hir(SRC), "pair", vec![json!(dir)], &mut resume)
        .await
        .expect("resume");
    let one_again = std::fs::read_to_string(tmp.path().join("one.txt")).expect("one still");
    assert_eq!(one_again, "one", "activity 1 must not re-run");
    assert!(
        journal.iter().any(|e| {
            e["event"] == "ActivityReplayed" && e["activity"].as_str() == Some("write_one")
        }),
        "activity 1 must be replayed from its completed journal entry"
    );
    let two = std::fs::read_to_string(tmp.path().join("two.txt")).expect("two ran");
    assert_eq!(two, "two");
    assert!(journal.iter().any(|e| e["event"] == "WorkflowCompleted"));
}

#[tokio::test]
async fn window_b_started_without_completed_retries() {
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = tmp.path().to_string_lossy().to_string();
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let inner = VoxDbTracker::new(db.clone(), "crash-b");
    // fail_after tuned so on_activity_started persists and the body/completion faults.
    let mut tracker = CountingTracker::new(inner, /* fail_after */ 3);
    let first =
        interpret_workflow_durable(&hir(SRC), "pair", vec![json!(dir.clone())], &mut tracker).await;
    assert!(first.is_err());

    let mut resume = VoxDbTracker::new(db, "crash-b");
    resume.expire_lease_for_test().await.expect("expire");
    let journal = interpret_workflow_durable(&hir(SRC), "pair", vec![json!(dir)], &mut resume)
        .await
        .expect("retry");
    let attempt = journal
        .iter()
        .find(|e| e["activity"].as_str() == Some("write_one") && e.get("resume_attempt").is_some())
        .and_then(|e| e["resume_attempt"].as_u64())
        .unwrap_or(0);
    assert_eq!(
        attempt, 2,
        "started-without-completed must retry; got {journal:#?}"
    );
}
