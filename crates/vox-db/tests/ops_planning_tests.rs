#![cfg(feature = "local")]

use vox_db::{DbConfig, VoxDb};

#[tokio::test]
async fn planning_schema_and_ops_roundtrip() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    db.create_plan_session("p1", Some("s1"), "goal", "SequentialDag")
        .await
        .expect("session");
    db.append_plan_version("p1", 1, None, None, None)
        .await
        .expect("version");
    db.upsert_plan_node("p1", 1, "n1", "do work", "[]", "{}", "pending", None)
        .await
        .expect("node");
    let head = db.load_plan_head("p1").await.expect("head");
    assert_eq!(head, Some(1));
    let runnable = db.list_runnable_nodes("p1", 1).await.expect("runnable");
    assert_eq!(runnable.len(), 1);
    db.record_plan_node_attempt("p1", 1, "n1", 1, Some("T-1"), "completed", None, None)
        .await
        .expect("attempt");
}
