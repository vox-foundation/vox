use super::*;
use crate::config::OrchestratorConfig;
use crate::types::{AgentTask, TaskPriority, FileAffinity, TaskId};

#[cfg(test)]
mod tests {
    use super::*;

    fn test_orchestrator() -> Orchestrator {
        Orchestrator::new(OrchestratorConfig::for_testing())
    }

    #[tokio::test]
    async fn spawn_agent() {
        let mut orch = test_orchestrator();
        let id = orch.spawn_agent("parser").expect("spawn");
        assert_eq!(orch.status().agent_count, 1);
        assert!(orch.agent_queue(id).is_some());
    }

    #[tokio::test]
    async fn max_agents_enforced() {
        let mut orch = Orchestrator::new(OrchestratorConfig {
            max_agents: 2,
            ..OrchestratorConfig::for_testing()
        });
        orch.spawn_agent("a").unwrap();
        orch.spawn_agent("b").unwrap();
        let err = orch.spawn_agent("c").unwrap_err();
        assert!(matches!(
            err,
            OrchestratorError::MaxAgentsReached { max: 2 }
        ));
    }

    #[tokio::test]
    async fn submit_and_route() {
        let mut orch = test_orchestrator();
        let task_id = orch
            .submit_task(
                "Fix parser bug",
                vec![FileAffinity::write("crates/vox-parser/src/grammar.rs")],
                None,
                None,
            )
            .await
            .expect("submit");
        assert_eq!(orch.status().total_queued, 1);
        assert_eq!(orch.status().agent_count, 1); // auto-spawned
        assert!(orch.task_assignments().contains_key(&task_id));
    }

    #[tokio::test]
    async fn same_file_routes_to_same_agent() {
        let mut orch = test_orchestrator();
        let t1 = orch
            .submit_task("Task 1", vec![FileAffinity::write("src/lib.rs")], None, None)
            .await
            .unwrap();
        let t2 = orch
            .submit_task("Task 2", vec![FileAffinity::write("src/lib.rs")], None, None)
            .await
            .unwrap();

        // Both tasks should be assigned to the same agent
        assert_eq!(
            orch.task_assignments().get(&t1),
            orch.task_assignments().get(&t2),
            "tasks touching the same file should route to the same agent"
        );
    }

    #[tokio::test]
    async fn different_files_can_route_to_different_agents() {
        let mut orch = test_orchestrator();
        orch.submit_task(
            "Parser work",
            vec![FileAffinity::write("crates/vox-parser/src/lib.rs")],
            None,
            None,
        )
        .await
        .unwrap();
        orch.submit_task(
            "Codegen work",
            vec![FileAffinity::write("crates/vox-codegen-rust/src/lib.rs")],
            None,
            None,
        )
        .await
        .unwrap();

        // Should have spawned at least one agent (may be 1 or 2 depending on routing)
        assert!(orch.status().agent_count >= 1);
    }

    #[tokio::test]
    async fn complete_task_flow() {
        let mut orch = test_orchestrator();
        let task_id = orch
            .submit_task("Test task", vec![FileAffinity::write("test.rs")], None, None)
            .await
            .unwrap();

        let agent_id = *orch.task_assignments().get(&task_id).unwrap();

        // Dequeue the task (simulating an agent picking it up)
        orch.get_agent_queue_mut(agent_id).unwrap().dequeue();

        // Complete it
        orch.complete_task(task_id).await.expect("complete");
        assert_eq!(orch.status().total_completed, 1);
    }

    #[tokio::test]
    async fn retire_agent_returns_tasks() {
        let mut orch = test_orchestrator();
        let agent_id = orch.spawn_agent("temp").unwrap();

        // Manually enqueue a task
        let task = AgentTask::new(TaskId(99), "leftover", TaskPriority::Normal, vec![]);
        orch.get_agent_queue_mut(agent_id).unwrap().enqueue(task);

        let remaining = orch.retire_agent(agent_id).unwrap();
        assert_eq!(remaining.len(), 1);
        assert!(orch.agent_queue(agent_id).is_none());
    }

    #[tokio::test]
    async fn pause_resume_agent() {
        let mut orch = test_orchestrator();
        let agent_id = orch.spawn_agent("test").unwrap();

        orch.pause_agent(agent_id).unwrap();
        assert!(orch.agent_queue(agent_id).unwrap().is_paused());

        orch.resume_agent(agent_id).unwrap();
        assert!(!orch.agent_queue(agent_id).unwrap().is_paused());
    }

    #[tokio::test]
    async fn disabled_orchestrator_rejects_tasks() {
        let mut orch = Orchestrator::new(OrchestratorConfig {
            enabled: false,
            ..OrchestratorConfig::for_testing()
        });
        let err = orch.submit_task("test", vec![], None, None).await.unwrap_err();
        assert!(matches!(err, OrchestratorError::Disabled));
    }

    #[tokio::test]
    async fn status_snapshot() {
        let mut orch = test_orchestrator();
        orch.submit_task("t1", vec![FileAffinity::write("a.rs")], None, None)
            .await
            .unwrap();
        orch.submit_task("t2", vec![FileAffinity::write("b.rs")], None, None)
            .await
            .unwrap();

        let status = orch.status();
        assert!(status.enabled);
        assert!(status.total_queued >= 2);
    }

    #[tokio::test]
    async fn task_trace_after_submit() {
        let mut orch = test_orchestrator();
        let task_id = orch
            .submit_task("Trace me", vec![FileAffinity::write("x.rs")], None, None)
            .await
            .unwrap();
        let steps = orch.task_trace(task_id).expect("trace exists");
        assert!(steps.len() >= 2);
        assert_eq!(steps[0].stage, "ingress");
        assert_eq!(steps[1].stage, "routed");
        assert!(
            steps[1]
                .detail
                .as_ref()
                .map(|d| d.starts_with("agent "))
                .unwrap_or(false)
        );
    }

    #[tokio::test]
    async fn task_trace_after_complete() {
        let mut orch = test_orchestrator();
        let task_id = orch
            .submit_task("Complete me", vec![FileAffinity::write("y.rs")], None, None)
            .await
            .unwrap();
        let agent_id = *orch.task_assignments().get(&task_id).unwrap();
        orch.get_agent_queue_mut(agent_id).unwrap().dequeue();
        orch.complete_task(task_id).await.unwrap();
        let steps = orch.task_trace(task_id).expect("trace exists");
        let outcome = steps
            .iter()
            .find(|s| s.stage == "outcome")
            .expect("outcome step");
        assert_eq!(outcome.detail.as_deref(), Some("completed"));
    }

    #[tokio::test]
    async fn task_trace_after_fail() {
        let mut orch = test_orchestrator();
        let task_id = orch
            .submit_task("Fail me", vec![FileAffinity::write("z.rs")], None, None)
            .await
            .unwrap();
        let agent_id = *orch.task_assignments().get(&task_id).unwrap();
        orch.get_agent_queue_mut(agent_id).unwrap().dequeue();
        orch.fail_task(task_id, "timeout".to_string())
            .await
            .unwrap();
        let steps = orch.task_trace(task_id).expect("trace exists");
        let outcome = steps
            .iter()
            .find(|s| s.stage == "outcome")
            .expect("outcome step");
        assert!(
            outcome
                .detail
                .as_deref()
                .map(|d| d.starts_with("failed: "))
                .unwrap_or(false)
        );
    }

    #[tokio::test]
    async fn socrates_enforced_gate_requeues_low_confidence_task() {
        let mut cfg = OrchestratorConfig::for_testing();
        cfg.socrates_gate_enforce = true;
        cfg.socrates_gate_shadow = true;
        cfg.max_debug_iterations = 2;
        let mut orch = Orchestrator::new(cfg);
        let agent_id = orch.spawn_agent("socrates").expect("spawn");

        let task_id = TaskId(9001);
        let mut task = AgentTask::new(
            task_id,
            "grounded answer required",
            TaskPriority::Normal,
            vec![FileAffinity::write("facts.md")],
        );
        task.socrates = Some(crate::socrates::SocratesTaskContext {
            factual_mode: true,
            required_citations: 3,
            evidence_count: 0,
            contradiction_hints: 0,
            risk_budget: "high".to_string(),
        });
        {
            let queue = orch.get_agent_queue_mut(agent_id).expect("queue");
            queue.enqueue(task);
            let _ = queue.dequeue();
        }
        orch.task_assignments.insert(task_id, agent_id);

        orch.complete_task(task_id).await.expect("gate path");

        let q = orch.agent_queue(agent_id).expect("queue snapshot");
        assert_eq!(q.completed_count(), 0);
        assert!(!q.is_empty());
    }
}
