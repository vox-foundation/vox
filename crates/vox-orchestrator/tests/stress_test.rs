use std::time::Duration;

use proptest::prelude::*;
use vox_orchestrator::{
    CompletionAttestation, FileAffinity, Orchestrator, OrchestratorConfig, TaskPriority,
};

fn stress_completion_attestation() -> CompletionAttestation {
    CompletionAttestation {
        checks_passed: vec!["peer_review_approved".to_string()],
        ..Default::default()
    }
}

/// Fails the suite fast if drain logic livelocks under load.
const STRESS_DRAIN_MAX_OUTER_ROUNDS: usize = 200_000;

/// Headroom for the 1k-task stress drain (`rebalance` can add extra sweeps).
const STRESS_DRAIN_CAP_1K: usize = 250_000;

fn test_config() -> OrchestratorConfig {
    let mut config = OrchestratorConfig::for_testing();
    config.max_agents = 10;
    config
}

async fn submit_and_drain(orch: &Orchestrator, task_count: usize) {
    let max_outer_rounds = STRESS_DRAIN_MAX_OUTER_ROUNDS.max(task_count.saturating_mul(500));

    for i in 0..task_count {
        orch.submit_task(
            format!("Task {i}"),
            vec![FileAffinity::write(format!("src/file_{}.rs", i % 5))],
            Some(TaskPriority::Normal),
            None,
        )
        .await
        .unwrap();
    }

    let mut outer = 0usize;
    loop {
        outer += 1;
        assert!(
            outer <= max_outer_rounds,
            "submit_and_drain: exceeded outer rounds ({max_outer_rounds}) for task_count={task_count}"
        );

        let mut progress = false;
        let ids = orch.agent_ids();
        for id in ids {
            let task_id = if let Some(queue) = orch.get_agent_queue_mut(id) {
                if let Some(task) = vox_orchestrator::sync_lock::rw_write(&*queue).dequeue() {
                    Some(task.id)
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(tid) = task_id {
                orch
                    .complete_task_with_attestation(tid, Some(stress_completion_attestation()))
                    .await
                    .unwrap();
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }
}

proptest! {
    #[test]
    fn submit_and_complete_n_tasks(n in 1usize..100) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let orch = Orchestrator::new(test_config());
            submit_and_drain(&orch, n).await;

            // Assert everything completed
            assert_eq!(orch.status().total_completed, n);
            assert_eq!(orch.status().total_queued, 0);
            assert_eq!(orch.status().total_in_progress, 0);
        });
    }

    #[test]
    fn rebalance_maintains_total_tasks(n in 1usize..50) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let orch = Orchestrator::new(test_config());

            // Submit tasks targeting the same agent
            for i in 0..n {
                orch.submit_task(
                    format!("Task {i}"),
                    vec![FileAffinity::write("src/hot_file.rs")],
                    Some(TaskPriority::Normal),
                    None,
                ).await.unwrap();
            }

            // Submit unrelated tasks
            for i in 0..n {
                orch.submit_task(
                    format!("Other Task {i}"),
                    vec![FileAffinity::write(format!("src/other_{i}.rs"))],
                    Some(TaskPriority::Normal),
                    None,
                ).await.unwrap();
            }

            let initial_queued = orch.status().total_queued;
            assert_eq!(initial_queued, n * 2);

            // Force a rebalance
            orch.rebalance();

            // Ensure tasks aren't lost or duplicated
            assert_eq!(orch.status().total_queued, initial_queued);
        });
    }
}

#[tokio::test]
async fn stress_test_1000_tasks_10_agents() {
    let stress_timeout = Duration::from_secs(120);
    let task_count: usize = 1000;
    let max_outer_rounds = STRESS_DRAIN_CAP_1K.max(task_count.saturating_mul(500));

    tokio::time::timeout(stress_timeout, async {
        let orch = Orchestrator::new(test_config());

        for i in 0..task_count {
            orch.submit_task(
                format!("Stress Task {i}"),
                vec![FileAffinity::write(format!("src/partition_{}.rs", i % 10))],
                Some(TaskPriority::Normal),
                None,
            )
            .await
            .unwrap();
        }

        let mut outer = 0usize;
        loop {
            outer += 1;
            assert!(
                outer <= max_outer_rounds,
                "stress_test_1000_tasks_10_agents: exceeded outer rounds ({max_outer_rounds})"
            );

            let mut progress = false;
            let ids = orch.agent_ids();
            for id in ids {
                let next_task = {
                    let queue = orch.get_agent_queue_mut(id).unwrap();
                    vox_orchestrator::sync_lock::rw_write(&*queue).dequeue()
                };

                if let Some(task) = next_task {
                    orch
                        .complete_task_with_attestation(
                            task.id,
                            Some(stress_completion_attestation()),
                        )
                        .await
                        .unwrap();
                    progress = true;
                }
            }
            if !progress {
                break;
            }
            orch.rebalance(); // Periodically rebalance under stress
        }

        let status = orch.status();
        assert_eq!(status.total_completed, task_count);
        assert_eq!(status.total_queued, 0);
    })
    .await
    .expect("stress_test_1000_tasks_10_agents timed out; possible hang");
}
