use std::time::Duration;
use vox_orchestrator::hopper::{IntakeSource, PriorityHint};
use vox_orchestrator::{Orchestrator, OrchestratorConfig};

#[tokio::test]
async fn real_dispatcher_holds_the_lock_and_parks_a_contender() {
    let orch = Orchestrator::new(OrchestratorConfig::for_testing());
    let _agent_id = orch.spawn_agent("locks").expect("spawn_agent must succeed");

    let hopper = orch.hopper();
    let item_a = hopper
        .submit_with_resource(
            "task A".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s1".into()),
            Some("db://orders/9".into()),
        )
        .await;

    // Poll until orch.resource_locks().is_locked("db://orders/9") and A is assigned
    let deadline = tokio::time::Instant::now() + vox_config::timeouts::D_10S;
    loop {
        let is_locked = orch.resource_locks().is_locked("db://orders/9");
        let assigned = hopper.assigned().await;
        let a_assigned = assigned.iter().any(|i| i.item_id == item_a.item_id);
        if is_locked && a_assigned {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for task A to hold the lock and be assigned"
        );
        tokio::time::sleep(vox_config::timeouts::D_20MS).await;
    }

    // Submit B on the same resource
    let item_b = hopper
        .submit_with_resource(
            "task B".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s2".into()),
            Some("db://orders/9".into()),
        )
        .await;

    // Poll ~500 ms (bounded) and assert B is still in hopper().inbox() and was never assigned
    let poll_end = tokio::time::Instant::now() + Duration::from_millis(500);
    while tokio::time::Instant::now() < poll_end {
        let assigned = hopper.assigned().await;
        assert!(
            !assigned.iter().any(|i| i.item_id == item_b.item_id),
            "task B should not be assigned while resource lock is held"
        );
        let inbox = hopper.inbox().await;
        assert!(
            inbox.iter().any(|i| i.item_id == item_b.item_id),
            "task B must remain in inbox"
        );
        tokio::time::sleep(vox_config::timeouts::D_20MS).await;
    }
}
