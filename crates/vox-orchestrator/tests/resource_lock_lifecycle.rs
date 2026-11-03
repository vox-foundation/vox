use std::time::Duration;
use vox_db::{DbConfig, VoxDb};
use vox_orchestrator::events::AgentEventKind;
use vox_orchestrator::hopper::{IntakeSource, PriorityHint};
use vox_orchestrator::types::TaskId;
use vox_orchestrator::{CompletionAttestation, Orchestrator, OrchestratorConfig};

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
    // drift-allow(timeout-literal,duplicate-numeric-literal): test fixture duration, not an HTTP request timeout; a shared constant needs a vox-config edge
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

#[tokio::test]
async fn completing_the_holder_releases_the_lock_and_dispatches_the_waiter() {
    let orch = Orchestrator::new(OrchestratorConfig::for_testing());
    let agent_id = orch.spawn_agent("locks").expect("spawn_agent must succeed");

    let mut rx = orch.event_bus.subscribe();

    let hopper = orch.hopper();
    let item_a = hopper
        .submit_with_resource(
            "task A".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s3".into()),
            Some("db://orders/7".into()),
        )
        .await;

    let item_b = hopper
        .submit_with_resource(
            "task B".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s4".into()),
            Some("db://orders/7".into()),
        )
        .await;

    let task_id_a = TaskId(vox_orchestrator::orchestrator::dispatch::stable_hash(
        &item_a.item_id.0,
    ));
    let task_id_b = TaskId(vox_orchestrator::orchestrator::dispatch::stable_hash(
        &item_b.item_id.0,
    ));

    // Wait until A holds the lock and B's LockWaiting arrives (bounded poll, D_10S)
    let deadline = tokio::time::Instant::now() + vox_config::timeouts::D_10S;
    let mut b_waiting = false;
    loop {
        let is_locked = orch.resource_locks().is_locked("db://orders/7");
        let assigned = hopper.assigned().await;
        let a_assigned = assigned.iter().any(|i| i.item_id == item_a.item_id);

        while let Ok(event) = rx.try_recv() {
            if let AgentEventKind::LockWaiting {
                task_id,
                resource_id,
                ..
            } = &event.kind
                && *task_id == task_id_b
                && resource_id == "db://orders/7"
            {
                b_waiting = true;
            }
        }

        if is_locked && a_assigned && b_waiting {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for task A to hold the lock and B to wait"
        );
        tokio::time::sleep(vox_config::timeouts::D_20MS).await;
    }

    // Dequeue A's task and complete it
    orch.agent_queue(agent_id)
        .expect("agent queue")
        .write()
        .unwrap()
        .dequeue();

    let attestation = CompletionAttestation {
        checks_passed: vec!["human_review_approved".to_string()],
        ..Default::default()
    };
    orch.complete_task_with_attestation(task_id_a, Some(attestation))
        .await
        .expect("complete task A");

    // Assert LockReleased { session_id: Some("chat-s3"), task_id: Some(task_id_a), .. }
    let deadline = tokio::time::Instant::now() + vox_config::timeouts::D_10S;
    let mut lock_released = false;
    while tokio::time::Instant::now() < deadline {
        if let Ok(Ok(event)) = tokio::time::timeout(vox_config::timeouts::D_100MS, rx.recv()).await
            && let AgentEventKind::LockReleased {
                session_id,
                task_id,
                path,
                ..
            } = &event.kind
            && path.to_str() == Some("db://orders/7")
            && session_id.as_deref() == Some("chat-s3")
            && *task_id == Some(task_id_a)
        {
            lock_released = true;
            break;
        }
    }
    assert!(
        lock_released,
        "did not receive expected LockReleased event for task A"
    );

    // Then (bounded poll) B is assigned and resource_locks() holds db://orders/7 again
    let deadline = tokio::time::Instant::now() + vox_config::timeouts::D_10S;
    loop {
        let is_locked = orch.resource_locks().is_locked("db://orders/7");
        let assigned = hopper.assigned().await;
        let b_assigned = assigned.iter().any(|i| i.item_id == item_b.item_id);
        if is_locked && b_assigned {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for task B to be assigned and lock held"
        );
        tokio::time::sleep(vox_config::timeouts::D_20MS).await;
    }
}

#[tokio::test]
async fn failing_the_holder_releases_the_lock_and_dispatches_the_waiter() {
    let orch = Orchestrator::new(OrchestratorConfig::for_testing());
    let agent_id = orch.spawn_agent("locks").expect("spawn_agent must succeed");

    let mut rx = orch.event_bus.subscribe();

    let hopper = orch.hopper();
    let item_a = hopper
        .submit_with_resource(
            "task A".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s3".into()),
            Some("db://orders/7".into()),
        )
        .await;

    let item_b = hopper
        .submit_with_resource(
            "task B".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s4".into()),
            Some("db://orders/7".into()),
        )
        .await;

    let task_id_a = TaskId(vox_orchestrator::orchestrator::dispatch::stable_hash(
        &item_a.item_id.0,
    ));
    let task_id_b = TaskId(vox_orchestrator::orchestrator::dispatch::stable_hash(
        &item_b.item_id.0,
    ));

    // Wait until A holds the lock and B's LockWaiting arrives (bounded poll, D_10S)
    let deadline = tokio::time::Instant::now() + vox_config::timeouts::D_10S;
    let mut b_waiting = false;
    loop {
        let is_locked = orch.resource_locks().is_locked("db://orders/7");
        let assigned = hopper.assigned().await;
        let a_assigned = assigned.iter().any(|i| i.item_id == item_a.item_id);

        while let Ok(event) = rx.try_recv() {
            if let AgentEventKind::LockWaiting {
                task_id,
                resource_id,
                ..
            } = &event.kind
                && *task_id == task_id_b
                && resource_id == "db://orders/7"
            {
                b_waiting = true;
            }
        }

        if is_locked && a_assigned && b_waiting {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for task A to hold the lock and B to wait"
        );
        tokio::time::sleep(vox_config::timeouts::D_20MS).await;
    }

    // Dequeue A's task and fail it
    orch.agent_queue(agent_id)
        .expect("agent queue")
        .write()
        .unwrap()
        .dequeue();

    orch.fail_task(task_id_a, "boom".to_string())
        .await
        .expect("fail task A");

    // Assert LockReleased { session_id: Some("chat-s3"), task_id: Some(task_id_a), .. }
    let deadline = tokio::time::Instant::now() + vox_config::timeouts::D_10S;
    let mut lock_released = false;
    while tokio::time::Instant::now() < deadline {
        if let Ok(Ok(event)) = tokio::time::timeout(vox_config::timeouts::D_100MS, rx.recv()).await
            && let AgentEventKind::LockReleased {
                session_id,
                task_id,
                path,
                ..
            } = &event.kind
            && path.to_str() == Some("db://orders/7")
            && session_id.as_deref() == Some("chat-s3")
            && *task_id == Some(task_id_a)
        {
            lock_released = true;
            break;
        }
    }
    assert!(
        lock_released,
        "did not receive expected LockReleased event for task A"
    );

    // Then (bounded poll) B is assigned and resource_locks() holds db://orders/7 again
    let deadline = tokio::time::Instant::now() + vox_config::timeouts::D_10S;
    loop {
        let is_locked = orch.resource_locks().is_locked("db://orders/7");
        let assigned = hopper.assigned().await;
        let b_assigned = assigned.iter().any(|i| i.item_id == item_b.item_id);
        if is_locked && b_assigned {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for task B to be assigned and lock held"
        );
        tokio::time::sleep(vox_config::timeouts::D_20MS).await;
    }
}

#[tokio::test]
async fn sqlite_hopper_contention_releases_and_dispatches_the_waiter() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("open db");
    let db = std::sync::Arc::new(db);

    let orch = Orchestrator::new(OrchestratorConfig::for_testing());
    let agent_id = orch.spawn_agent("locks").expect("spawn_agent must succeed");
    orch.init_db(db.clone())
        .await
        .expect("init_db must succeed");

    let mut rx = orch.event_bus.subscribe();

    let hopper = orch.hopper();
    let item_a = hopper
        .submit_with_resource(
            "task A".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s3".into()),
            Some("db://orders/9".into()),
        )
        .await;

    let item_b = hopper
        .submit_with_resource(
            "task B".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s4".into()),
            Some("db://orders/9".into()),
        )
        .await;

    let task_id_a = TaskId(vox_orchestrator::orchestrator::dispatch::stable_hash(
        &item_a.item_id.0,
    ));
    let task_id_b = TaskId(vox_orchestrator::orchestrator::dispatch::stable_hash(
        &item_b.item_id.0,
    ));

    // Wait until A holds the lock and B's LockWaiting arrives (bounded poll, D_10S)
    let deadline = tokio::time::Instant::now() + vox_config::timeouts::D_10S;
    let mut b_waiting = false;
    loop {
        let is_locked = orch.resource_locks().is_locked("db://orders/9");
        let assigned = hopper.assigned().await;
        let a_assigned = assigned.iter().any(|i| i.item_id == item_a.item_id);

        while let Ok(event) = rx.try_recv() {
            if let AgentEventKind::LockWaiting {
                task_id,
                resource_id,
                ..
            } = &event.kind
                && *task_id == task_id_b
                && resource_id == "db://orders/9"
            {
                b_waiting = true;
            }
        }

        if is_locked && a_assigned && b_waiting {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for task A to hold the lock and B to wait"
        );
        tokio::time::sleep(vox_config::timeouts::D_20MS).await;
    }

    // Dequeue A's task and complete it
    orch.agent_queue(agent_id)
        .expect("agent queue")
        .write()
        .unwrap()
        .dequeue();

    let attestation = CompletionAttestation {
        checks_passed: vec!["human_review_approved".to_string()],
        ..Default::default()
    };
    orch.complete_task_with_attestation(task_id_a, Some(attestation))
        .await
        .expect("complete task A");

    // Assert LockReleased { session_id: Some("chat-s3"), task_id: Some(task_id_a), .. }
    let deadline = tokio::time::Instant::now() + vox_config::timeouts::D_10S;
    let mut lock_released = false;
    while tokio::time::Instant::now() < deadline {
        if let Ok(Ok(event)) = tokio::time::timeout(vox_config::timeouts::D_100MS, rx.recv()).await
            && let AgentEventKind::LockReleased {
                session_id,
                task_id,
                path,
                ..
            } = &event.kind
            && path.to_str() == Some("db://orders/9")
            && session_id.as_deref() == Some("chat-s3")
            && *task_id == Some(task_id_a)
        {
            lock_released = true;
            break;
        }
    }
    assert!(
        lock_released,
        "did not receive expected LockReleased event for task A"
    );

    // Then (bounded poll) B is assigned and resource_locks() holds db://orders/9 again
    let deadline = tokio::time::Instant::now() + vox_config::timeouts::D_10S;
    loop {
        let is_locked = orch.resource_locks().is_locked("db://orders/9");
        let assigned = hopper.assigned().await;
        let b_assigned = assigned.iter().any(|i| i.item_id == item_b.item_id);
        if is_locked && b_assigned {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for task B to be assigned and lock held"
        );
        tokio::time::sleep(vox_config::timeouts::D_20MS).await;
    }
}

#[tokio::test]
async fn cancelling_the_holder_releases_the_lock() {
    let orch = Orchestrator::new(OrchestratorConfig::for_testing());
    let _agent_id = orch.spawn_agent("locks").expect("spawn_agent must succeed");

    let mut rx = orch.event_bus.subscribe();

    let hopper = orch.hopper();
    let item_a = hopper
        .submit_with_resource(
            "task A".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s3".into()),
            Some("db://orders/8".into()),
        )
        .await;

    let item_b = hopper
        .submit_with_resource(
            "task B".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s4".into()),
            Some("db://orders/8".into()),
        )
        .await;

    let task_id_a = TaskId(vox_orchestrator::orchestrator::dispatch::stable_hash(
        &item_a.item_id.0,
    ));
    let task_id_b = TaskId(vox_orchestrator::orchestrator::dispatch::stable_hash(
        &item_b.item_id.0,
    ));

    // Wait until A holds the lock and B's LockWaiting arrives (bounded poll, D_10S)
    let deadline = tokio::time::Instant::now() + vox_config::timeouts::D_10S;
    let mut b_waiting = false;
    loop {
        let is_locked = orch.resource_locks().is_locked("db://orders/8");
        let assigned = hopper.assigned().await;
        let a_assigned = assigned.iter().any(|i| i.item_id == item_a.item_id);

        while let Ok(event) = rx.try_recv() {
            if let AgentEventKind::LockWaiting {
                task_id,
                resource_id,
                ..
            } = &event.kind
                && *task_id == task_id_b
                && resource_id == "db://orders/8"
            {
                b_waiting = true;
            }
        }

        if is_locked && a_assigned && b_waiting {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for task A to hold the lock and B to wait"
        );
        tokio::time::sleep(vox_config::timeouts::D_20MS).await;
    }

    // Cancel A's task directly (it is queued, not dequeued)
    orch.cancel_task(task_id_a).expect("cancel task A");

    // Assert LockReleased { session_id: Some("chat-s3"), task_id: Some(task_id_a), .. }
    let deadline = tokio::time::Instant::now() + vox_config::timeouts::D_10S;
    let mut lock_released = false;
    while tokio::time::Instant::now() < deadline {
        if let Ok(Ok(event)) = tokio::time::timeout(vox_config::timeouts::D_100MS, rx.recv()).await
            && let AgentEventKind::LockReleased {
                session_id,
                task_id,
                path,
                ..
            } = &event.kind
            && path.to_str() == Some("db://orders/8")
            && session_id.as_deref() == Some("chat-s3")
            && *task_id == Some(task_id_a)
        {
            lock_released = true;
            break;
        }
    }
    assert!(
        lock_released,
        "did not receive expected LockReleased event for task A"
    );

    // Then (bounded poll) B is assigned and resource_locks() holds db://orders/8 again
    let deadline = tokio::time::Instant::now() + vox_config::timeouts::D_10S;
    loop {
        let is_locked = orch.resource_locks().is_locked("db://orders/8");
        let assigned = hopper.assigned().await;
        let b_assigned = assigned.iter().any(|i| i.item_id == item_b.item_id);
        if is_locked && b_assigned {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for task B to be assigned and lock held"
        );
        tokio::time::sleep(vox_config::timeouts::D_20MS).await;
    }
}

#[tokio::test]
async fn cancelling_the_hopper_item_releases_the_lock() {
    let orch = Orchestrator::new(OrchestratorConfig::for_testing());
    let _agent_id = orch.spawn_agent("locks").expect("spawn_agent must succeed");

    let mut rx = orch.event_bus.subscribe();

    let hopper = orch.hopper();
    let item_a = hopper
        .submit_with_resource(
            "task A".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s3".into()),
            Some("db://orders/8".into()),
        )
        .await;

    let item_b = hopper
        .submit_with_resource(
            "task B".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s4".into()),
            Some("db://orders/8".into()),
        )
        .await;

    let task_id_a = TaskId(vox_orchestrator::orchestrator::dispatch::stable_hash(
        &item_a.item_id.0,
    ));
    let task_id_b = TaskId(vox_orchestrator::orchestrator::dispatch::stable_hash(
        &item_b.item_id.0,
    ));

    // Wait until A holds the lock and B's LockWaiting arrives (bounded poll, D_10S)
    let deadline = tokio::time::Instant::now() + vox_config::timeouts::D_10S;
    let mut b_waiting = false;
    loop {
        let is_locked = orch.resource_locks().is_locked("db://orders/8");
        let assigned = hopper.assigned().await;
        let a_assigned = assigned.iter().any(|i| i.item_id == item_a.item_id);

        while let Ok(event) = rx.try_recv() {
            if let AgentEventKind::LockWaiting {
                task_id,
                resource_id,
                ..
            } = &event.kind
                && *task_id == task_id_b
                && resource_id == "db://orders/8"
            {
                b_waiting = true;
            }
        }

        if is_locked && a_assigned && b_waiting {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for task A to hold the lock and B to wait"
        );
        tokio::time::sleep(vox_config::timeouts::D_20MS).await;
    }

    // Cancel A through the hopper
    orch.hopper()
        .cancel(&item_a.item_id)
        .await
        .expect("cancel hopper item A");

    // Assert LockReleased { session_id: Some("chat-s3"), task_id: Some(task_id_a), .. }
    let deadline = tokio::time::Instant::now() + vox_config::timeouts::D_10S;
    let mut lock_released = false;
    while tokio::time::Instant::now() < deadline {
        if let Ok(Ok(event)) = tokio::time::timeout(vox_config::timeouts::D_100MS, rx.recv()).await
            && let AgentEventKind::LockReleased {
                session_id,
                task_id,
                path,
                ..
            } = &event.kind
            && path.to_str() == Some("db://orders/8")
            && session_id.as_deref() == Some("chat-s3")
            && *task_id == Some(task_id_a)
        {
            lock_released = true;
            break;
        }
    }
    assert!(
        lock_released,
        "did not receive expected LockReleased event for task A"
    );

    // Then (bounded poll) B is assigned and resource_locks() holds db://orders/8 again
    let deadline = tokio::time::Instant::now() + vox_config::timeouts::D_10S;
    loop {
        let is_locked = orch.resource_locks().is_locked("db://orders/8");
        let assigned = hopper.assigned().await;
        let b_assigned = assigned.iter().any(|i| i.item_id == item_b.item_id);
        if is_locked && b_assigned {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for task B to be assigned and lock held"
        );
        tokio::time::sleep(vox_config::timeouts::D_20MS).await;
    }
}
