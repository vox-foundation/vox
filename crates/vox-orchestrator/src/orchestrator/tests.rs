use super::*;
use crate::config::OrchestratorConfig;
use crate::types::{AgentTask, FileAffinity, TaskId, TaskPriority};

#[cfg(test)]
mod orch_smoke {
    use super::*;

    fn test_orchestrator() -> Orchestrator {
        Orchestrator::new(OrchestratorConfig::for_testing())
    }

    #[tokio::test]
    async fn spawn_agent() {
        let orch = test_orchestrator();
        let id = orch.spawn_agent("parser").expect("spawn");
        assert_eq!(orch.status().agent_count, 1);
        assert!(orch.agent_queue(id).is_some());
    }

    #[tokio::test]
    async fn max_agents_enforced() {
        let orch = Orchestrator::new(OrchestratorConfig {
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
        let orch = test_orchestrator();
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
        assert!(orch.task_assignments.read().unwrap().contains_key(&task_id));
    }

    #[tokio::test]
    async fn submit_seeds_socrates_from_session_retrieval_envelope_key() {
        let orch = test_orchestrator();
        let sid = "orch-test-session";
        let key = crate::socrates::session_retrieval_envelope_key(sid);
        let env = crate::socrates::SessionRetrievalEnvelope {
            retrieval_tier: "hybrid".to_string(),
            memory_hit_count: 2,
            knowledge_hit_count: 1,
            used_vector: true,
            used_bm25: true,
            used_lexical_fallback: false,
            contradiction_count: 0,
        };
        let json = serde_json::to_string(&env).unwrap();
        orch.context_store
            .write()
            .unwrap()
            .set(crate::types::AgentId(0), key, json, 0);

        let tid = orch
            .submit_task(
                "task with session",
                vec![FileAffinity::read("README.md")],
                None,
                Some(sid.to_string()),
            )
            .await
            .expect("submit");

        let aid = *orch.task_assignments.read().unwrap().get(&tid).unwrap();
        let q_lock = orch.agent_queue(aid).unwrap();
        let q = q_lock.read().unwrap();
        let t = q
            .tasks()
            .iter()
            .find(|t| t.id == tid)
            .expect("queued task");
        let soc = t.socrates.as_ref().expect("socrates from context store");
        assert_eq!(soc.retrieval_tier.as_deref(), Some("hybrid"));
        assert_eq!(soc.evidence_count, 3);
        assert!(soc.retrieval_used_vector);
    }

    #[tokio::test]
    async fn same_file_routes_to_same_agent() {
        let orch = test_orchestrator();
        let t1 = orch
            .submit_task(
                "Task 1",
                vec![FileAffinity::write("src/lib.rs")],
                None,
                None,
            )
            .await
            .unwrap();
        let t2 = orch
            .submit_task(
                "Task 2",
                vec![FileAffinity::write("src/lib.rs")],
                None,
                None,
            )
            .await
            .unwrap();

        // Both tasks should be assigned to the same agent
        let assignments = orch.task_assignments.read().unwrap();
        assert_eq!(
            assignments.get(&t1),
            assignments.get(&t2),
            "tasks touching the same file should route to the same agent"
        );
    }

    #[tokio::test]
    async fn different_files_can_route_to_different_agents() {
        let orch = test_orchestrator();
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
        let orch = test_orchestrator();
        let task_id = orch
            .submit_task(
                "Test task",
                vec![FileAffinity::write("test.rs")],
                None,
                None,
            )
            .await
            .unwrap();

        let agent_id = *orch.task_assignments.read().unwrap().get(&task_id).unwrap();

        // Dequeue the task (simulating an agent picking it up)
        orch.agent_queue(agent_id)
            .unwrap()
            .write()
            .unwrap()
            .dequeue();

        // Complete it
        orch.complete_task(task_id).await.expect("complete");
        assert_eq!(orch.status().total_completed, 1);
    }

    #[tokio::test]
    async fn retire_agent_returns_tasks() {
        let orch = test_orchestrator();
        let agent_id = orch.spawn_agent("temp").unwrap();

        // Manually enqueue a task
        let task = AgentTask::new(TaskId(99), "leftover", TaskPriority::Normal, vec![]);
        orch.agent_queue(agent_id)
            .unwrap()
            .write()
            .unwrap()
            .enqueue(task);

        let remaining = orch.retire_agent(agent_id).unwrap();
        assert_eq!(remaining.len(), 1);
        assert!(orch.agent_queue(agent_id).is_none());
    }

    #[tokio::test]
    async fn pause_resume_agent() {
        let orch = test_orchestrator();
        let agent_id = orch.spawn_agent("test").unwrap();

        orch.pause_agent(agent_id).unwrap();
        assert!(
            orch.agent_queue(agent_id)
                .unwrap()
                .read()
                .unwrap()
                .is_paused()
        );

        orch.resume_agent(agent_id).unwrap();
        assert!(
            !orch
                .agent_queue(agent_id)
                .unwrap()
                .read()
                .unwrap()
                .is_paused()
        );
    }

    #[tokio::test]
    async fn disabled_orchestrator_rejects_tasks() {
        let orch = Orchestrator::new(OrchestratorConfig {
            enabled: false,
            ..OrchestratorConfig::for_testing()
        });
        let err = orch
            .submit_task("test", vec![], None, None)
            .await
            .unwrap_err();
        assert!(matches!(err, OrchestratorError::Disabled));
    }

    #[tokio::test]
    async fn status_snapshot() {
        let orch = test_orchestrator();
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
        let orch = test_orchestrator();
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
        let orch = test_orchestrator();
        let task_id = orch
            .submit_task("Complete me", vec![FileAffinity::write("y.rs")], None, None)
            .await
            .unwrap();
        let agent_id = *orch.task_assignments.read().unwrap().get(&task_id).unwrap();
        orch.agent_queue(agent_id)
            .unwrap()
            .write()
            .unwrap()
            .dequeue();
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
        let orch = test_orchestrator();
        let task_id = orch
            .submit_task("Fail me", vec![FileAffinity::write("z.rs")], None, None)
            .await
            .unwrap();
        let agent_id = *orch.task_assignments.read().unwrap().get(&task_id).unwrap();
        orch.agent_queue(agent_id)
            .unwrap()
            .write()
            .unwrap()
            .dequeue();
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
        let orch = Orchestrator::new(cfg);
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
            retrieval_tier: None,
            retrieval_used_vector: false,
            retrieval_used_lexical_fallback: false,
        });
        {
            let queue_lock = orch.agent_queue(agent_id).expect("queue");
            let mut queue = queue_lock.write().unwrap();
            queue.enqueue(task);
            let _ = queue.dequeue();
        }
        orch.task_assignments
            .write()
            .unwrap()
            .insert(task_id, agent_id);

        orch.complete_task(task_id).await.expect("gate path");

        let q_lock = orch.agent_queue(agent_id).expect("queue snapshot");
        let q = q_lock.read().unwrap();
        assert_eq!(q.completed_count(), 0);
        assert!(!q.is_empty());
    }

    #[tokio::test]
    async fn submit_goal_falls_back_to_direct_when_planning_disabled() {
        let orch = test_orchestrator();
        let task_id = orch
            .submit_goal(
                "small direct change",
                vec![FileAffinity::write("src/lib.rs")],
                None,
                None,
                None,
            )
            .await
            .expect("submit goal");
        assert!(orch.task_assignments.read().unwrap().contains_key(&task_id));
    }

    #[tokio::test]
    async fn submit_goal_force_plan_attaches_plan_metadata() {
        let orch = Orchestrator::new(OrchestratorConfig {
            planning_enabled: true,
            planning_router_enabled: true,
            ..OrchestratorConfig::for_testing()
        });
        let task_id = orch
            .submit_goal(
                "refactor and add tests",
                vec![FileAffinity::write("src/lib.rs")],
                None,
                Some(crate::planning::PlanningMode::ForcePlan),
                Some("s1".to_string()),
            )
            .await
            .expect("submit planned goal");
        let agent_id = *orch
            .task_assignments
            .read()
            .unwrap()
            .get(&task_id)
            .expect("assignment");
        let queue_lock = orch.agent_queue(agent_id).expect("queue");
        let queue = queue_lock.read().unwrap();
        let has_meta = queue
            .tasks()
            .iter()
            .any(|t| t.id == task_id && t.plan_session_id.is_some() && t.plan_node_id.is_some());
        assert!(has_meta, "planned task should contain planning metadata");
    }

    #[tokio::test]
    async fn fail_task_with_replan_trigger_enqueues_recovery_work() {
        let orch = Orchestrator::new(OrchestratorConfig {
            planning_enabled: true,
            planning_replan_enabled: true,
            ..OrchestratorConfig::for_testing()
        });
        let agent_id = orch.spawn_agent("planner").expect("spawn");
        let mut t = AgentTask::new(
            TaskId(777),
            "run tests",
            TaskPriority::Normal,
            vec![FileAffinity::write("src/lib.rs")],
        );
        t.plan_session_id = Some("p1".to_string());
        t.plan_node_id = Some("n1".to_string());
        t.plan_version = Some(1);
        t.execution_policy_json = Some(
            serde_json::json!({
                "replan_triggers": ["test_failure_new_regression"]
            })
            .to_string(),
        );
        {
            let queue_lock = orch.agent_queue(agent_id).expect("queue");
            let mut queue = queue_lock.write().unwrap();
            queue.enqueue(t);
            let _ = queue.dequeue();
        }
        orch.task_assignments
            .write()
            .unwrap()
            .insert(TaskId(777), agent_id);
        orch.fail_task(TaskId(777), "test failure in suite".to_string())
            .await
            .expect("fail");
        // Recovery is enqueued via `enqueue_plan_node` (manifest `Cargo.toml`), which routes to
        // whichever agent owns that affinity — not necessarily the agent that failed.
        let st = orch.status();
        assert!(
            st.total_queued >= 1,
            "replan should enqueue recovery work on some agent (total_queued={}, per_agent={:?})",
            st.total_queued,
            st.agents
                .iter()
                .map(|a| (a.id, a.queued))
                .collect::<Vec<_>>()
        );
    }
}
