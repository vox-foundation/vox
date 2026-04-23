//! Round-trip TCP daemon for `orch.ping` / `orch.task_status`.

use std::sync::Arc;

use tokio::net::TcpListener;
use vox_orchestrator::{
    AgentTask, Orchestrator, OrchestratorConfig, TaskId, TaskPriority, orch_daemon,
};

#[tokio::test]
async fn orchestrator_daemon_ping_and_task_status() {
    let orch = Arc::new(Orchestrator::new(OrchestratorConfig::for_testing()));
    let aid = orch.spawn_agent("d1").expect("spawn");
    let tid = TaskId(4242);
    let task = AgentTask::new(tid, "daemon probe", TaskPriority::Normal, vec![]);
    {
        let ql = orch.agent_queue(aid).expect("queue");
        let mut q = ql.write().unwrap();
        q.enqueue(task);
        let _ = q.dequeue();
    }
    orch.task_assignments.write().unwrap().insert(tid, aid);

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let o = orch.clone();
    let bind_label = addr.to_string();
    let server = tokio::spawn(orch_daemon::serve_listener(
        listener,
        bind_label,
        "ut-repo".to_string(),
        o,
    ));

    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    let client = orch_daemon::OrchDaemonClient::new(addr.to_string());
    let ping = client.ping().await.expect("ping");
    assert_eq!(ping["repository_id"], "ut-repo");
    assert_eq!(ping["protocol"], "vox.orchestrator_daemon/v1");

    let st = client.orchestrator_status().await.expect("status");
    assert!(st.get("agent_count").is_some());

    let ts = client.task_status(4242).await.expect("task_status");
    assert_eq!(ts["status"], "InProgress");

    let spawned = client
        .spawn_agent_named("rpc-spawn")
        .await
        .expect("spawn_agent");
    assert!(spawned["agent_id"].as_u64().is_some());

    let ids = client.agent_ids().await.expect("agent_ids");
    assert!(ids["agent_ids"].as_array().is_some());

    let wj = client.workspace_journey().await.expect("workspace_journey");
    assert_eq!(wj["daemon_repository_id"], "ut-repo");
    assert!(wj.get("workspace_journey_store_mode").is_some());

    server.abort();
}

#[tokio::test]
async fn orchestrator_daemon_task_and_agent_write_methods() {
    let orch = Arc::new(Orchestrator::new(OrchestratorConfig::for_testing()));
    let aid = orch.spawn_agent("writer").expect("spawn");
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(orch_daemon::serve_listener(
        listener,
        addr.to_string(),
        "ut-repo".to_string(),
        orch.clone(),
    ));
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    let client = orch_daemon::OrchDaemonClient::new(addr.to_string());

    let submitted = client
        .submit_task(serde_json::json!({
            "description": "rpc submit task",
            "file_manifest": [],
            "priority": "Normal",
            "target_agent": "writer",
        }))
        .await
        .expect("submit_task");
    let task_id = submitted["task_id"].as_u64().expect("task_id");
    let _ = client
        .reorder_task(task_id, "urgent")
        .await
        .expect("reorder_task");
    let _ = client.cancel_task(task_id).await.expect("cancel_task");

    let submitted2 = client
        .submit_task(serde_json::json!({
            "description": "rpc submit fail task",
            "file_manifest": [],
            "priority": "Normal",
            "target_agent": "writer",
        }))
        .await
        .expect("submit_task_2");
    let task_id2 = submitted2["task_id"].as_u64().expect("task_id2");
    let _ = client
        .fail_task(task_id2, "expected fail".to_string())
        .await
        .expect("fail_task");

    let submitted3 = client
        .submit_task(serde_json::json!({
            "description": "rpc submit complete task",
            "file_manifest": [],
            "priority": "Normal",
            "target_agent": "writer",
        }))
        .await
        .expect("submit_task_3");
    let task_id3 = submitted3["task_id"].as_u64().expect("task_id3");
    let _ = client
        .complete_task(task_id3, None)
        .await
        .expect("complete_task");

    let drained = client.drain_agent(aid.0).await.expect("drain_agent");
    assert!(drained["drained_count"].as_u64().is_some());
    let rebalance = client.rebalance().await.expect("rebalance");
    assert!(rebalance["rebalanced"].as_u64().is_some());

    let dyn_spawned = client
        .spawn_agent_ext(serde_json::json!({
            "name": "dyn-rpc",
            "dynamic": true,
            "parent_agent_id": aid.0,
            "delegation_reason": "unit-test",
            "source_task_id": task_id3,
        }))
        .await
        .expect("spawn_agent_ext");
    let dyn_id = dyn_spawned["agent_id"].as_u64().expect("dyn_id");
    let _ = client.pause_agent(dyn_id).await.expect("pause_agent");
    let _ = client.resume_agent(dyn_id).await.expect("resume_agent");
    let retired = client.retire_agent(dyn_id).await.expect("retire_agent");
    assert!(retired["remaining_tasks"].as_u64().is_some());

    server.abort();
}
