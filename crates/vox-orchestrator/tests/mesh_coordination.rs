use std::time::Duration;
use vox_orchestrator::types::{AgentId, A2AMessageType, MessagePriority};
use vox_orchestrator::a2a;
use vox_orchestrator::heartbeat;
use vox_orchestrator::locks;
use vox_orchestrator::oplog;

#[tokio::test]
async fn test_distributed_coordination_primitives() {
    let db = vox_db::VoxDb::from_store(vox_pm::CodeStore::open_memory().await.expect("in-memory store"));
    
    // 1. Schema application (coordination DDL)
    db.sync_schema_from_digest(&vox_orchestrator::schema::orchestrator_schema())
        .await
        .expect("sync schema");

    let repo_id = "test-repo";

    // 2. Heartbeats
    heartbeat::persist_heartbeat_with_breaker(&db, "node-1", AgentId(1), "writing".parse().unwrap(), repo_id)
        .await
        .expect("persist heartbeat");
    
    let live = heartbeat::live_nodes_from_db(db.store().connection(), 60000, repo_id)
        .await
        .expect("list live nodes");
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].0, "node-1");

    // 3. A2A Messaging
    let uuid = a2a::send_to_db_with_breaker(
        &db,
        AgentId(1),
        AgentId(2),
        A2AMessageType::ProgressUpdate,
        "payload",
        MessagePriority::Normal,
        None,
        repo_id,
    )
    .await
    .expect("send a2a");

    let inbox = a2a::poll_inbox_from_db(db.store().connection(), AgentId(2), repo_id)
        .await
        .expect("poll a2a");
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].message_uuid, uuid);

    a2a::acknowledge_db_message(db.store().connection(), &uuid)
        .await
        .expect("ack a2a");
    let inbox_after = a2a::poll_inbox_from_db(db.store().connection(), AgentId(2), repo_id)
        .await
        .expect("poll a2a after ack");
    assert_eq!(inbox_after.len(), 0);

    // 4. Distributed Locks
    let lock_res = locks::acquire_distributed_lock_with_breaker(&db, "file.rs", "node-1", AgentId(1), 60, repo_id)
        .await
        .expect("acquire lock");
    let fence = lock_res.expect("should get lock");
    assert_eq!(fence, 1);

    // Node 2 tries to acquire same lock
    let lock_res_2 = locks::acquire_distributed_lock_with_breaker(&db, "file.rs", "node-2", AgentId(2), 60, repo_id)
        .await
        .expect("acquire lock node 2");
    let holder = lock_res_2.expect_err("should be held");
    assert_eq!(holder, "node-1");

    locks::release_distributed_lock_with_breaker(&db, "file.rs", "node-1", repo_id)
        .await
        .expect("release lock");
    
    let lock_res_3 = locks::acquire_distributed_lock_with_breaker(&db, "file.rs", "node-2", AgentId(2), 60, repo_id)
        .await
        .expect("acquire lock node 2 again");
    assert!(lock_res_3.is_ok());

    // 5. OpLog
    let op_kind = oplog::OperationKind::Custom { label: "test".into() };
    oplog::append_to_db_with_breaker(
        &db,
        AgentId(1),
        "OP-000001",
        &op_kind,
        "test desc",
        None,
        None,
        None,
        123456789,
        repo_id,
    )
    .await
    .expect("append oplog");

    let ops = oplog::list_from_db(db.store().connection(), None, repo_id, 10)
        .await
        .expect("list oplog");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].description, "test desc");
}
