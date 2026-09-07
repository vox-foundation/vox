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

async fn expire_lease(db: &VoxDb, run_id: &str) {
    db.connection()
        .execute(
            "UPDATE workflow_run_log \
             SET lease_until_ms = 0, lease_owner = NULL \
             WHERE run_id = ?1",
            (run_id.to_string(),),
        )
        .await
        .expect("expire workflow lease");
    db.connection()
        .execute(
            "UPDATE workflow_activity_attempt_log \
             SET lease_until_ms = 0 \
             WHERE run_id = ?1 AND status = 'started'",
            (run_id.to_string(),),
        )
        .await
        .expect("expire activity attempt lease");
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
    // Calls 1-9 persist through the first on_activity_completed; call 10 is
    // the second activity's is_activity_completed read and faults.
    let mut tracker = CountingTracker::new(inner, /* fail_after */ 9);
    let first =
        interpret_workflow_durable(&hir(SRC), "pair", vec![json!(dir.clone())], &mut tracker).await;
    assert!(first.is_err(), "must crash after activity 1 completed");
    let one = std::fs::read_to_string(tmp.path().join("one.txt")).expect("one written");
    assert_eq!(one, "one");
    assert!(
        !tmp.path().join("two.txt").exists(),
        "activity 2 must not have run"
    );

    expire_lease(&db, "crash-a").await;
    let mut resume = VoxDbTracker::new(db.clone(), "crash-a");
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
    let write_two_id = journal
        .iter()
        .find(|e| e["event"] == "ActivityCompleted" && e["activity"] == "write_two")
        .and_then(|e| e["activity_id"].as_str())
        .expect("write_two completion has activity id");
    let mut completed = db
        .connection()
        .query(
            "SELECT 1 FROM workflow_activity_attempt_log \
             WHERE run_id = ?1 AND workflow_name = ?2 AND activity_id = ?3 \
             AND status = 'completed'",
            (
                "crash-a".to_string(),
                "pair".to_string(),
                write_two_id.to_string(),
            ),
        )
        .await
        .expect("query completed attempt");
    assert!(
        completed
            .next()
            .await
            .expect("read completed attempt")
            .is_some(),
        "successful attempt must be completed"
    );
}

#[tokio::test]
async fn window_b_started_without_completed_retries() {
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = tmp.path().to_string_lossy().to_string();
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("db"));
    let inner = VoxDbTracker::new(db.clone(), "crash-b");
    // Calls 1-7 persist the first attempt start; call 8 faults while recording
    // its successful attempt completion, leaving started-without-completed.
    let mut tracker = CountingTracker::new(inner, /* fail_after */ 7);
    let first =
        interpret_workflow_durable(&hir(SRC), "pair", vec![json!(dir.clone())], &mut tracker).await;
    assert!(first.is_err());

    expire_lease(&db, "crash-b").await;
    let mut resume = VoxDbTracker::new(db, "crash-b");
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
