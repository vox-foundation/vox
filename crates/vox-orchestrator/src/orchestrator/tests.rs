use super::*;
use crate::config::OrchestratorConfig;
use crate::types::{AgentTask, CompletionAttestation, FileAffinity, TaskId, TaskPriority};

/// Shared assertions for affinity / scope / lock alignment with [`Orchestrator::task_assignments`].
#[cfg(test)]
fn assert_assignment_affinity_scope_locks_aligned(orch: &Orchestrator) {
    let assignments = orch.task_assignments.read().unwrap();
    let agents = orch.agents.read().unwrap();
    let scope = orch.scope_guard.read().unwrap();

    for (&task_id, &agent_id) in assignments.iter() {
        let queue_lock = agents.get(&agent_id).unwrap_or_else(|| {
            panic!(
                "task {} assigned to missing agent {}",
                task_id.0, agent_id.0
            )
        });
        let queue = queue_lock.read().unwrap();
        let task_ref = queue
            .tasks()
            .iter()
            .find(|t| t.id == task_id)
            .or_else(|| queue.current_task().filter(|t| t.id == task_id))
            .unwrap_or_else(|| {
                panic!(
                    "task {} assigned to agent {} but not present in that queue",
                    task_id.0, agent_id.0
                )
            });

        for path in task_ref.write_files() {
            assert_eq!(
                orch.affinity_map.lookup(path.as_path()),
                Some(agent_id),
                "affinity owner for task {} path {:?} should match assignment agent {}",
                task_id.0,
                path,
                agent_id.0
            );
            let in_scope = scope
                .agent_scope(agent_id)
                .map(|s| s.contains(path))
                .unwrap_or(false);
            assert!(
                in_scope,
                "agent {} scope should contain {:?} for task {}",
                agent_id.0, path, task_id.0
            );
            let holder = orch.lock_manager.holder(path.as_path());
            assert_eq!(
                holder.map(|(a, _)| a),
                Some(agent_id),
                "lock holder for {:?} should be agent {} (task {})",
                path,
                agent_id.0,
                task_id.0
            );
        }
    }
}

#[cfg(test)]
fn assert_path_has_no_writer_ownership(
    orch: &Orchestrator,
    path: &std::path::Path,
    former: crate::types::AgentId,
) {
    assert!(
        orch.affinity_map.lookup(path).is_none(),
        "affinity should be released for {:?}",
        path
    );
    assert!(
        !orch.lock_manager.is_locked(path),
        "lock should be released for {:?}",
        path
    );
    let scope = orch.scope_guard.read().unwrap();
    if let Some(files) = scope.agent_scope(former) {
        assert!(
            !files.iter().any(|p| p.as_path() == path),
            "former agent {} should not retain {:?} in scope",
            former.0,
            path
        );
    }
}

#[cfg(test)]
mod state_invariants {
    use super::*;
    use std::path::Path;

    fn orch_named_pair() -> Orchestrator {
        let orch = Orchestrator::new(OrchestratorConfig {
            max_agents: 8,
            ..OrchestratorConfig::for_testing()
        });
        orch.spawn_agent("heavy").expect("spawn heavy");
        orch.spawn_agent("light").expect("spawn light");
        orch
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn rebalance_transfer_keeps_assignments_affinity_scope_locks_coherent() {
        let orch = orch_named_pair();
        for i in 0..15 {
            orch.submit_task_with_agent(
                format!("heavy {i}"),
                vec![FileAffinity::write(format!("state_inv/rebalance/h{i}.rs"))],
                None,
                Some("heavy".to_string()),
                None,
                None,
                None,
            )
            .await
            .expect("submit heavy");
        }
        orch.submit_task_with_agent(
            "light sole",
            vec![FileAffinity::write("state_inv/rebalance/light_only.rs")],
            None,
            Some("light".to_string()),
            None,
            None,
            None,
        )
        .await
        .expect("submit light");

        super::assert_assignment_affinity_scope_locks_aligned(&orch);

        let moved = orch.rebalance();
        assert!(
            moved > 0,
            "expected at least one stolen task when heavy is overloaded vs light"
        );

        super::assert_assignment_affinity_scope_locks_aligned(&orch);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn cancel_unique_write_does_not_leak_affinity_scope_or_lock() {
        let orch = Orchestrator::new(OrchestratorConfig::for_testing());
        let path = Path::new("state_inv/cancel_unique.rs");
        let tid = orch
            .submit_task("cancel me", vec![FileAffinity::write(path)], None, None)
            .await
            .expect("submit");
        let aid = *orch
            .task_assignments
            .read()
            .unwrap()
            .get(&tid)
            .expect("assignment");
        orch.cancel_task(tid).expect("cancel");
        assert!(
            !orch.task_assignments.read().unwrap().contains_key(&tid),
            "cancel should drop task_assignments entry"
        );
        super::assert_path_has_no_writer_ownership(&orch, path, aid);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn fail_in_progress_unique_write_does_not_leak_affinity_scope_or_lock() {
        let orch = Orchestrator::new(OrchestratorConfig::for_testing());
        let path = Path::new("state_inv/fail_unique.rs");
        let tid = orch
            .submit_task("fail me", vec![FileAffinity::write(path)], None, None)
            .await
            .expect("submit");
        let aid = *orch
            .task_assignments
            .read()
            .unwrap()
            .get(&tid)
            .expect("assignment");
        orch.agent_queue(aid)
            .expect("queue")
            .write()
            .unwrap()
            .dequeue();
        orch.fail_task(tid, "boom".to_string()).await.expect("fail");
        super::assert_path_has_no_writer_ownership(&orch, path, aid);
    }
}

#[cfg(test)]
mod orch_smoke {
    use super::*;

    fn test_orchestrator() -> Orchestrator {
        Orchestrator::new(OrchestratorConfig::for_testing())
    }

    fn complete_attestation_for_tests() -> CompletionAttestation {
        CompletionAttestation {
            checks_passed: vec!["human_review_approved".to_string()],
            ..Default::default()
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn spawn_agent() {
        let orch = test_orchestrator();
        let id = orch.spawn_agent("parser").expect("spawn");
        assert_eq!(orch.status().agent_count, 1);
        assert!(orch.agent_queue(id).is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn submit_and_route() {
        let orch = test_orchestrator();
        let task_id = orch
            .submit_task(
                "Fix parser bug",
                vec![FileAffinity::write("grammar.rs")],
                None,
                None,
            )
            .await
            .expect("submit");
        assert_eq!(orch.status().total_queued, 1);
        assert_eq!(orch.status().agent_count, 1); // auto-spawned
        assert!(orch.task_assignments.read().unwrap().contains_key(&task_id));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn submit_seeds_socrates_from_session_context_envelope_key() {
        let orch = test_orchestrator();
        let sid = "orch-test-session";
        let key = crate::socrates::session_context_envelope_key(sid);
        let env = crate::socrates::SessionRetrievalEnvelope {
            retrieval_tier: "hybrid".to_string(),
            memory_hit_count: 2,
            knowledge_hit_count: 1,
            chunk_hit_count: 0,
            repo_hit_count: 0,
            rrf_fused_hit_count: 0,
            used_vector: true,
            used_bm25: true,
            used_lexical_fallback: false,
            contradiction_count: 0,
            source_diversity: 2,
            evidence_quality: 0.8,
            citation_coverage: 1.0,
            verification_performed: false,
            verification_reason: None,
            recommended_next_action: None,
        };
        let context = crate::ContextEnvelope::from_session_retrieval("repo-orch-test", sid, &env);
        let json = serde_json::to_string(&context).unwrap();
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
        let t = q.tasks().iter().find(|t| t.id == tid).expect("queued task");
        let soc = t.socrates.as_ref().expect("socrates from context store");
        assert_eq!(soc.retrieval_tier.as_deref(), Some("hybrid"));
        assert_eq!(soc.evidence_count, 3);
        assert!(soc.retrieval_used_vector);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

        // Complete it (writes can classify as Review/Confirm; attestation satisfies gates)
        orch.complete_task_with_attestation(task_id, Some(complete_attestation_for_tests()))
            .await
            .expect("complete");
        assert_eq!(orch.status().total_completed, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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
        orch.complete_task_with_attestation(task_id, Some(complete_attestation_for_tests()))
            .await
            .unwrap();
        let steps = orch.task_trace(task_id).expect("trace exists");
        let outcome = steps
            .iter()
            .find(|s| s.stage == "outcome")
            .expect("outcome step");
        assert_eq!(outcome.detail.as_deref(), Some("completed"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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
            ..Default::default()
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

    fn grounding_violating_attestation() -> CompletionAttestation {
        CompletionAttestation {
            completion_summary: Some(
                "The handler returns HTTP 403 for anonymous users when auth middleware is disabled."
                    .to_string(),
            ),
            checks_passed: vec!["human_review_approved".to_string()],
            ..Default::default()
        }
    }

    async fn orch_memory_with_init(cfg: OrchestratorConfig) -> (std::sync::Arc<vox_db::VoxDb>, Orchestrator) {
        let db = std::sync::Arc::new(
            vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
                .await
                .expect("memory db"),
        );
        let orch = Orchestrator::new(cfg).with_db(db.clone());
        orch.init_db(db.clone())
            .await
            .expect("orchestrator init_db");
        (db, orch)
    }

    async fn seed_agent_reliability_high(db: &vox_db::VoxDb, agent_id: AgentId) {
        let sid = agent_id.0.to_string();
        // Five successive outcomes reach (5+2)/(5+3) ≈ 0.875 ≥ default relax floor 0.85.
        for _ in 0..5 {
            db.record_task_reliability_observation(&sid, true)
                .await
                .expect("record reliability");
        }
        let r = db
            .get_agent_reliability(&sid)
            .await
            .expect("read reliability");
        assert!(
            r.is_some_and(|x| x >= 0.85),
            "expected Laplace reliability >= 0.85 after five successes, got {r:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn trust_relax_allows_completion_under_grounding_enforce_when_agent_reliable() {
        let mut cfg = OrchestratorConfig::for_testing();
        cfg.socrates_gate_enforce = false;
        cfg.completion_grounding_enforce = true;
        cfg.completion_grounding_shadow = false;
        cfg.trust_gate_relax_enabled = true;
        cfg.trust_gate_relax_min_reliability = 0.85;
        cfg.max_socrates_debug_iterations = 2;
        let (_db, orch) = orch_memory_with_init(cfg).await;

        let agent_id = orch.spawn_agent("ground-relax").expect("spawn");
        seed_agent_reliability_high(&_db, agent_id).await;

        let task_id = TaskId(9021);
        let mut task = AgentTask::new(
            task_id,
            "factual summary without declared citations",
            TaskPriority::Normal,
            vec![FileAffinity::write("facts.md")],
        );
        task.socrates = Some(crate::socrates::SocratesTaskContext {
            factual_mode: true,
            required_citations: 2,
            evidence_count: 0,
            contradiction_hints: 0,
            risk_budget: "normal".to_string(),
            retrieval_tier: None,
            retrieval_used_vector: false,
            retrieval_used_lexical_fallback: false,
            ..Default::default()
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

        orch.complete_task_with_attestation(task_id, Some(grounding_violating_attestation()))
            .await
            .expect("complete without grounding requeue when trust relax applies");

        let q_lock = orch.agent_queue(agent_id).expect("queue snapshot");
        let q = q_lock.read().unwrap();
        assert_eq!(
            q.completed_count(),
            1,
            "task should complete, not requeue for grounding"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn completion_grounding_enforce_requeues_when_trust_relax_disabled_even_if_reliable() {
        let mut cfg = OrchestratorConfig::for_testing();
        cfg.socrates_gate_enforce = false;
        cfg.completion_grounding_enforce = true;
        cfg.completion_grounding_shadow = false;
        cfg.trust_gate_relax_enabled = false;
        cfg.trust_gate_relax_min_reliability = 0.85;
        cfg.max_socrates_debug_iterations = 2;
        let (_db, orch) = orch_memory_with_init(cfg).await;

        let agent_id = orch.spawn_agent("ground-strict").expect("spawn");
        seed_agent_reliability_high(&_db, agent_id).await;

        let task_id = TaskId(9022);
        let mut task = AgentTask::new(
            task_id,
            "factual summary without declared citations (strict)",
            TaskPriority::Normal,
            vec![FileAffinity::write("facts2.md")],
        );
        task.socrates = Some(crate::socrates::SocratesTaskContext {
            factual_mode: true,
            required_citations: 2,
            evidence_count: 0,
            contradiction_hints: 0,
            risk_budget: "normal".to_string(),
            retrieval_tier: None,
            retrieval_used_vector: false,
            retrieval_used_lexical_fallback: false,
            ..Default::default()
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

        orch.complete_task_with_attestation(task_id, Some(grounding_violating_attestation()))
            .await
            .expect("completion returns ok while requeueing");

        let q_lock = orch.agent_queue(agent_id).expect("queue snapshot");
        let q = q_lock.read().unwrap();
        assert_eq!(q.completed_count(), 0);
        assert!(!q.is_empty());
        let requeued = q
            .tasks()
            .iter()
            .chain(q.current_task())
            .find(|t| t.id == task_id)
            .expect("requeued task");
        assert!(
            requeued.description.contains("[GROUNDING GATE]"),
            "expected grounding requeue banner, got {:?}",
            requeued.description
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn submit_goal_falls_back_to_direct_when_planning_disabled() {
        let orch = test_orchestrator();
        let task_id = orch
            .submit_goal(
                "small direct change",
                vec![FileAffinity::write("src/lib.rs")],
                None,
                None,
                None,
                None,
            )
            .await
            .expect("submit goal");
        assert!(orch.task_assignments.read().unwrap().contains_key(&task_id));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn submit_goal_force_plan_attaches_plan_metadata() {
        let orch = Orchestrator::new(OrchestratorConfig {
            planning_enabled: true,
            planning_router_enabled: true,
            ..OrchestratorConfig::for_testing()
        });
        let task_id = orch
            .submit_goal(
                "Refactor authentication helpers and add regression tests for session handling",
                vec![FileAffinity::write("src/lib.rs")],
                None,
                Some(crate::planning::PlanningMode::ForcePlan),
                Some("s1".to_string()),
                None,
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn plan_dag_unblocks_next_node_on_complete() {
        let db = std::sync::Arc::new(
            vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
                .await
                .expect("memory db"),
        );
        let orch = Orchestrator::new(OrchestratorConfig {
            planning_enabled: true,
            planning_router_enabled: true,
            ..OrchestratorConfig::for_testing()
        })
        .with_db(db.clone());

        let tid = orch
            .submit_goal(
                "Implement alpha feature scaffolding in crates/demo and verify beta integration paths",
                vec![FileAffinity::read("Cargo.toml")],
                None,
                Some(crate::planning::PlanningMode::ForcePlan),
                Some("plan-dag-test".into()),
                None,
            )
            .await
            .expect("planned goal");

        let agent_id = *orch
            .task_assignments
            .read()
            .unwrap()
            .get(&tid)
            .expect("assignment");

        let plan_session_id = {
            let ql = orch.agent_queue(agent_id).expect("queue");
            let q = ql.read().unwrap();
            q.tasks()
                .iter()
                .find(|t| t.id == tid)
                .and_then(|t| t.plan_session_id.clone())
                .expect("plan session on task")
        };

        {
            let ql = orch.agent_queue(agent_id).expect("queue");
            let mut q = ql.write().unwrap();
            let _ = q.dequeue();
        }

        orch.complete_task_with_attestation(tid, Some(complete_attestation_for_tests()))
            .await
            .expect("complete first node");

        let rows = db
            .load_plan_nodes_with_status(&plan_session_id, 1)
            .await
            .expect("nodes");
        let n1 = rows.iter().find(|r| r.node_id == "n1").expect("n1 row");
        let n2 = rows.iter().find(|r| r.node_id == "n2").expect("n2 row");
        assert_eq!(n1.status, "completed");
        assert_eq!(n2.status, "queued");

        let st = orch.status();
        assert!(
            st.total_queued >= 1,
            "successor plan node should be enqueued (total_queued={})",
            st.total_queued
        );
    }
}

/// Single-owner Populi lease-gated submission and remote-hold completion paths.
#[cfg(test)]
mod populi_single_owner {
    use super::*;
    use crate::a2a::{
        REMOTE_TASK_CANCEL_TYPE, REMOTE_TASK_ENVELOPE_TYPE, REMOTE_TASK_RESULT_TYPE,
        RemoteTaskEnvelope, RemoteTaskResult,
    };
    #[cfg(feature = "populi-transport")]
    use crate::a2a::populi_remote_worker_tick_once;
    use crate::config::OrchestratorConfig;
    use crate::reconstruction::AgentExecutionRole;
    use crate::types::{
        AgentTask, FileAffinity, PopuliRemoteDelegate, TaskEnqueueHints, TaskId, TaskPriority,
    };

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn lease_gated_relay_failure_falls_back_to_local_queue() {
        let mut cfg = OrchestratorConfig::for_testing();
        cfg.populi_remote_execute_experimental = true;
        cfg.populi_control_url = Some("http://127.0.0.1:9".to_string());
        cfg.populi_remote_execute_receiver_agent = Some("2".to_string());
        cfg.populi_remote_lease_gating_enabled = true;
        cfg.populi_remote_lease_gated_roles = vec![AgentExecutionRole::Builder];
        let orch = Orchestrator::new(cfg);
        orch.spawn_agent("worker").expect("spawn");
        let hints = TaskEnqueueHints {
            execution_role: Some(AgentExecutionRole::Builder),
            ..Default::default()
        };
        let tid = orch
            .submit_task_with_agent(
                "leased-class task",
                vec![],
                None,
                None,
                None,
                Some(hints),
                None,
            )
            .await
            .expect("submit");
        let aid = *orch
            .task_assignments
            .read()
            .unwrap()
            .get(&tid)
            .expect("assignment");
        let ql = orch.agent_queue(aid).expect("queue");
        let q = ql.read().unwrap();
        assert!(
            !q.has_in_progress(),
            "relay failure must not leave a remote hold in progress"
        );
        assert_eq!(q.len(), 1, "task should be queued for local execution");
        let t = q.tasks().iter().find(|t| t.id == tid).expect("task");
        assert!(
            t.populi_remote_delegate.is_none(),
            "fallback task must not carry remote delegate"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn populi_remote_hold_completes_via_complete_task() {
        let orch = Orchestrator::new(OrchestratorConfig::for_testing());
        orch.spawn_agent("solo").expect("spawn");
        let aid = orch.agent_ids()[0];
        let mut task = AgentTask::new(TaskId(901), "remote-only", TaskPriority::Normal, vec![]);
        task.populi_remote_delegate = Some(PopuliRemoteDelegate {
            idempotency_key: "orch-remote-901-test".into(),
            lease_id: None,
            claimer_node_id: None,
        });
        {
            let ql = orch.agent_queue(aid).expect("queue");
            let mut q = ql.write().unwrap();
            q.hold_for_populi_remote(task).expect("hold");
        }
        orch.task_assignments
            .write()
            .unwrap()
            .insert(TaskId(901), aid);
        orch.complete_task(TaskId(901))
            .await
            .expect("complete remote-held task");
        let ql = orch.agent_queue(aid).expect("queue");
        let q = ql.read().unwrap();
        assert!(!q.has_in_progress());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancel_populi_remote_delegated_clears_assignment() {
        let orch = Orchestrator::new(OrchestratorConfig::for_testing());
        orch.spawn_agent("solo").expect("spawn");
        let aid = orch.agent_ids()[0];
        let path = std::path::Path::new("populi_single_owner/cancel_test.rs");
        let mut task = AgentTask::new(
            TaskId(902),
            "cancel-remote",
            TaskPriority::Normal,
            vec![FileAffinity::write(path)],
        );
        task.populi_remote_delegate = Some(PopuliRemoteDelegate {
            idempotency_key: "k902".into(),
            lease_id: None,
            claimer_node_id: None,
        });
        {
            let ql = orch.agent_queue(aid).expect("queue");
            let mut q = ql.write().unwrap();
            q.hold_for_populi_remote(task).expect("hold");
        }
        let _ = orch
            .lock_manager
            .try_acquire(path, aid, crate::locks::LockKind::Exclusive);
        orch.affinity_map.assign(path, aid);
        orch.task_assignments
            .write()
            .unwrap()
            .insert(TaskId(902), aid);
        orch.cancel_task(TaskId(902)).expect("cancel");
        assert!(
            !orch
                .task_assignments
                .read()
                .unwrap()
                .contains_key(&TaskId(902))
        );
    }

    #[cfg(feature = "populi-transport")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn lease_gated_submit_holds_then_completes_via_populi_result_poll() {
        let state = vox_populi::transport::PopuliTransportState::new();
        let seed = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind seed");
        let bound = seed.local_addr().expect("local addr");
        drop(seed);
        let server = tokio::spawn(async move {
            vox_populi::transport::serve(bound, state)
                .await
                .expect("serve");
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let base = format!("http://{bound}");
        let http = vox_populi::http_client::PopuliHttpClient::new(&base);

        let mut cfg = OrchestratorConfig::for_testing();
        cfg.populi_remote_execute_experimental = true;
        cfg.populi_control_url = Some(base.clone());
        cfg.populi_remote_execute_receiver_agent = Some("2".to_string());
        cfg.populi_remote_execute_sender_agent = Some("1".to_string());
        cfg.populi_remote_lease_gating_enabled = true;
        cfg.populi_remote_lease_gated_roles = vec![AgentExecutionRole::Builder];

        let orch = Orchestrator::new(cfg);
        orch.spawn_agent("worker").expect("spawn");
        let hints = TaskEnqueueHints {
            execution_role: Some(AgentExecutionRole::Builder),
            ..Default::default()
        };
        let tid = orch
            .submit_task_with_agent(
                "leased-e2e task",
                vec![],
                None,
                None,
                None,
                Some(hints),
                None,
            )
            .await
            .expect("submit");
        let aid = *orch
            .task_assignments
            .read()
            .unwrap()
            .get(&tid)
            .expect("assignment");

        // Lease-gated path holds remote in progress, not local queue.
        {
            let ql = orch.agent_queue(aid).expect("queue");
            let q = ql.read().unwrap();
            assert!(q.has_in_progress(), "expected remote-held in-progress task");
            assert_eq!(q.len(), 0, "expected no local queued copy");
        }
        let delegate_idempotency = {
            let ql = orch.agent_queue(aid).expect("queue");
            let q = ql.read().unwrap();
            q.current_task()
                .and_then(|t| t.populi_remote_delegate.as_ref())
                .map(|d| d.idempotency_key.clone())
                .expect("delegate idempotency")
        };

        let payload = serde_json::to_string(&RemoteTaskResult {
            idempotency_key: delegate_idempotency,
            success: false,
            result: None,
            error: Some("remote execution failed (test)".to_string()),
            task_id: Some(tid.0),
        })
        .expect("serialize remote result");
        http.relay_a2a(&vox_populi::transport::A2ADeliverRequest {
            sender_agent_id: "2".into(),
            receiver_agent_id: "1".into(),
            message_type: REMOTE_TASK_RESULT_TYPE.to_string(),
            payload,
            idempotency_key: Some(format!("remote-result-{}", tid.0)),
            privacy_class: None,
            payload_blake3_hex: None,
            worker_ed25519_sig_b64: None,
            jwe_payload: None,
        })
        .await
        .expect("relay result row");

        let mut cleared = false;
        for _ in 0..10 {
            crate::a2a::populi_remote_result_poll_once(&orch).await;
            let ql = orch.agent_queue(aid).expect("queue");
            let q = ql.read().unwrap();
            if !q.has_in_progress() {
                cleared = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert!(
            cleared,
            "in-progress slot should be cleared after remote completion"
        );
        let ql = orch.agent_queue(aid).expect("queue");
        let q = ql.read().unwrap();
        assert!(
            !q.has_in_progress(),
            "in-progress slot should be cleared after remote completion"
        );
        let inbox_after = http.relay_a2a_inbox("1").await.expect("inbox after");
        assert!(
            inbox_after
                .messages
                .iter()
                .all(|m| m.message_type != REMOTE_TASK_RESULT_TYPE),
            "remote_task_result row should be acked after terminal transition"
        );

        server.abort();
    }

    #[cfg(feature = "populi-transport")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn lease_gated_submit_relays_context_envelope_in_payload() {
        let state = vox_populi::transport::PopuliTransportState::new();
        let seed = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind seed");
        let bound = seed.local_addr().expect("local addr");
        drop(seed);
        let server = tokio::spawn(async move {
            vox_populi::transport::serve(bound, state)
                .await
                .expect("serve");
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let base = format!("http://{bound}");
        let http = vox_populi::http_client::PopuliHttpClient::new(&base);

        let mut cfg = OrchestratorConfig::for_testing();
        cfg.populi_remote_execute_experimental = true;
        cfg.populi_control_url = Some(base.clone());
        cfg.populi_remote_execute_receiver_agent = Some("2".to_string());
        cfg.populi_remote_execute_sender_agent = Some("1".to_string());
        cfg.populi_remote_lease_gating_enabled = true;
        cfg.populi_remote_lease_gated_roles = vec![AgentExecutionRole::Builder];
        let orch = Orchestrator::new(cfg);
        orch.spawn_agent("worker").expect("spawn");

        let sid = "lease-payload-session";
        let retrieval = crate::SessionRetrievalEnvelope {
            retrieval_tier: "hybrid".to_string(),
            memory_hit_count: 2,
            knowledge_hit_count: 1,
            chunk_hit_count: 0,
            repo_hit_count: 0,
            rrf_fused_hit_count: 1,
            used_vector: true,
            used_bm25: true,
            used_lexical_fallback: false,
            contradiction_count: 0,
            source_diversity: 2,
            evidence_quality: 0.8,
            citation_coverage: 0.9,
            verification_performed: false,
            verification_reason: None,
            recommended_next_action: None,
        };
        let context = crate::ContextEnvelope::from_session_retrieval("repo-test", sid, &retrieval);
        let context_json = serde_json::to_string(&context).expect("serialize context envelope");
        let key = crate::socrates::session_context_envelope_key(sid);
        crate::sync_lock::rw_write(&*orch.context_store).set(
            crate::types::AgentId(0),
            key,
            &context_json,
            3600,
        );
        let harness = crate::AgentHarnessSpec::minimal_contract_first(
            "repo-test",
            "leased-context-payload",
            Some(sid),
            Some("thread-lease"),
            &["artifacts/result.md".to_string()],
        );
        let harness_json = serde_json::to_string(&harness).expect("serialize harness");

        let hints = TaskEnqueueHints {
            execution_role: Some(AgentExecutionRole::Builder),
            thread_id: Some("thread-lease".to_string()),
            harness_spec_json: Some(harness_json.clone()),
            ..Default::default()
        };
        let _tid = orch
            .submit_task_with_agent(
                "leased-context-payload",
                vec![],
                None,
                None,
                None,
                Some(hints),
                Some(sid.to_string()),
            )
            .await
            .expect("submit");

        let mut relayed_payload: Option<serde_json::Value> = None;
        for _ in 0..20 {
            let inbox = http.relay_a2a_inbox("2").await.expect("inbox");
            if let Some(msg) = inbox
                .messages
                .iter()
                .find(|m| m.message_type == REMOTE_TASK_ENVELOPE_TYPE)
            {
                let env: RemoteTaskEnvelope =
                    serde_json::from_str(&msg.payload).expect("remote envelope parse");
                relayed_payload =
                    Some(serde_json::from_str::<serde_json::Value>(&env.payload).expect("payload parse"));
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        let payload = relayed_payload.expect("expected remote_task_envelope relay");
        assert_eq!(payload["session_id"], serde_json::json!(sid));
        assert_eq!(payload["thread_id"], serde_json::json!("thread-lease"));
        assert_eq!(payload["context_envelope_json"], serde_json::json!(context_json));
        assert_eq!(payload["harness_spec_json"], serde_json::json!(harness_json));

        server.abort();
    }

    #[cfg(feature = "populi-transport")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_worker_tick_once_seeds_context_and_attaches_socrates_when_task_assigned() {
        let state = vox_populi::transport::PopuliTransportState::new();
        let seed = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind seed");
        let bound = seed.local_addr().expect("local addr");
        drop(seed);
        let server = tokio::spawn(async move {
            vox_populi::transport::serve(bound, state)
                .await
                .expect("serve");
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let base = format!("http://{bound}");
        let http = vox_populi::http_client::PopuliHttpClient::new(&base);

        let mut cfg = OrchestratorConfig::for_testing();
        cfg.populi_remote_execute_experimental = true;
        cfg.populi_control_url = Some(base.clone());
        cfg.populi_remote_execute_receiver_agent = Some("2".to_string());
        cfg.populi_remote_execute_sender_agent = Some("1".to_string());
        cfg.populi_remote_worker_poll_interval_secs = 1;
        let orch = Orchestrator::new(cfg);
        orch.spawn_agent("worker").expect("spawn");
        let aid = orch.agent_ids()[0];

        let remote_task_id = TaskId(9944);
        let mut task = AgentTask::new(remote_task_id, "remote-worker-seed", TaskPriority::Normal, vec![]);
        task.populi_remote_delegate = Some(PopuliRemoteDelegate {
            idempotency_key: "k9944".into(),
            lease_id: None,
            claimer_node_id: None,
        });
        {
            let ql = orch.agent_queue(aid).expect("queue");
            let mut q = ql.write().unwrap();
            q.hold_for_populi_remote(task).expect("hold");
        }
        orch.task_assignments
            .write()
            .unwrap()
            .insert(remote_task_id, aid);

        let sid = "worker-seed-session";
        let retrieval = crate::SessionRetrievalEnvelope {
            retrieval_tier: "hybrid".to_string(),
            memory_hit_count: 2,
            knowledge_hit_count: 1,
            chunk_hit_count: 0,
            repo_hit_count: 0,
            rrf_fused_hit_count: 1,
            used_vector: true,
            used_bm25: true,
            used_lexical_fallback: false,
            contradiction_count: 0,
            source_diversity: 2,
            evidence_quality: 0.8,
            citation_coverage: 0.9,
            verification_performed: false,
            verification_reason: None,
            recommended_next_action: None,
        };
        let context = crate::ContextEnvelope::from_session_retrieval("repo-worker-test", sid, &retrieval);
        let context_json = serde_json::to_string(&context).expect("serialize context envelope");
        let key = crate::socrates::session_context_envelope_key(sid);
        assert!(
            crate::sync_lock::rw_read(&*orch.context_store)
                .get(&key)
                .is_none(),
            "context key should not be pre-seeded"
        );

        let inner_payload = serde_json::json!({
            "task_description": "remote worker task",
            "assigned_agent_id": aid.0,
            "session_id": sid,
            "context_envelope_json": context_json,
        })
        .to_string();
        let envelope = RemoteTaskEnvelope {
            idempotency_key: "remote-worker-9944".to_string(),
            task_id: remote_task_id.0,
            repository_id: "repo-worker-test".to_string(),
            capability_requirements_json: "{}".to_string(),
            payload: inner_payload,
            privacy_class: None,
            populi_scope_id: None,
            submitted_unix_ms: Some(crate::types::now_unix_ms()),
            exec_lease_id: Some("orchestrator-lease".to_string()),
            campaign_id: None,
            artifact_refs_json: None,
            session_id: Some(sid.to_string()),
            thread_id: None,
            context_envelope_json: Some(serde_json::to_string(&context).expect("serialize context")),
            harness_spec_json: None,
        };
        http.relay_a2a(&vox_populi::transport::A2ADeliverRequest {
            sender_agent_id: "1".into(),
            receiver_agent_id: "2".into(),
            message_type: REMOTE_TASK_ENVELOPE_TYPE.to_string(),
            payload: serde_json::to_string(&envelope).expect("serialize envelope"),
            idempotency_key: Some("inject-remote-worker-9944".to_string()),
            privacy_class: None,
            payload_blake3_hex: None,
            worker_ed25519_sig_b64: None,
            jwe_payload: None,
        })
        .await
        .expect("deliver remote envelope");

        populi_remote_worker_tick_once(&orch).await;

        let stored = crate::sync_lock::rw_read(&*orch.context_store)
            .get(&key)
            .expect("worker should seed context envelope key");
        assert_eq!(stored, serde_json::to_string(&context).expect("serialize"));

        let ql = orch.agent_queue(aid).expect("queue");
        let q = ql.read().unwrap();
        let soc = q
            .current_task()
            .and_then(|t| t.socrates.as_ref())
            .expect("worker should attach Socrates context when task is assigned");
        assert_eq!(soc.retrieval_tier.as_deref(), Some("hybrid"));

        server.abort();
    }

    #[cfg(feature = "populi-transport")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_worker_tick_once_accepts_object_context_envelope_payload() {
        let state = vox_populi::transport::PopuliTransportState::new();
        let seed = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind seed");
        let bound = seed.local_addr().expect("local addr");
        drop(seed);
        let server = tokio::spawn(async move {
            vox_populi::transport::serve(bound, state)
                .await
                .expect("serve");
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let base = format!("http://{bound}");
        let http = vox_populi::http_client::PopuliHttpClient::new(&base);

        let mut cfg = OrchestratorConfig::for_testing();
        cfg.populi_remote_execute_experimental = true;
        cfg.populi_control_url = Some(base.clone());
        cfg.populi_remote_execute_receiver_agent = Some("2".to_string());
        cfg.populi_remote_execute_sender_agent = Some("1".to_string());
        cfg.populi_remote_worker_poll_interval_secs = 1;
        let orch = Orchestrator::new(cfg);
        orch.spawn_agent("worker").expect("spawn");

        let sid = "worker-object-context-session";
        let retrieval = crate::SessionRetrievalEnvelope {
            retrieval_tier: "hybrid".to_string(),
            memory_hit_count: 1,
            knowledge_hit_count: 1,
            chunk_hit_count: 0,
            repo_hit_count: 0,
            rrf_fused_hit_count: 0,
            used_vector: true,
            used_bm25: true,
            used_lexical_fallback: false,
            contradiction_count: 0,
            source_diversity: 2,
            evidence_quality: 0.7,
            citation_coverage: 0.8,
            verification_performed: false,
            verification_reason: None,
            recommended_next_action: None,
        };
        let context = crate::ContextEnvelope::from_session_retrieval("repo-object-worker", sid, &retrieval);
        let context_value = serde_json::to_value(&context).expect("serialize context");
        let key = crate::socrates::session_context_envelope_key(sid);
        assert!(
            crate::sync_lock::rw_read(&*orch.context_store)
                .get(&key)
                .is_none(),
            "context key should not be pre-seeded"
        );

        let inner_payload = serde_json::json!({
            "task_description": "remote worker object payload",
            "session_id": sid,
            "context_envelope_json": context_value,
        })
        .to_string();
        let envelope = RemoteTaskEnvelope {
            idempotency_key: "remote-worker-object-9955".to_string(),
            task_id: 9955,
            repository_id: "repo-object-worker".to_string(),
            capability_requirements_json: "{}".to_string(),
            payload: inner_payload,
            privacy_class: None,
            populi_scope_id: None,
            submitted_unix_ms: Some(crate::types::now_unix_ms()),
            exec_lease_id: Some("orchestrator-lease".to_string()),
            campaign_id: None,
            artifact_refs_json: None,
            session_id: Some(sid.to_string()),
            thread_id: None,
            context_envelope_json: Some(serde_json::to_string(&context).expect("serialize context")),
            harness_spec_json: None,
        };
        http.relay_a2a(&vox_populi::transport::A2ADeliverRequest {
            sender_agent_id: "1".into(),
            receiver_agent_id: "2".into(),
            message_type: REMOTE_TASK_ENVELOPE_TYPE.to_string(),
            payload: serde_json::to_string(&envelope).expect("serialize envelope"),
            idempotency_key: Some("inject-remote-worker-object-9955".to_string()),
            privacy_class: None,
            payload_blake3_hex: None,
            worker_ed25519_sig_b64: None,
            jwe_payload: None,
        })
        .await
        .expect("deliver remote envelope");

        populi_remote_worker_tick_once(&orch).await;

        let stored = crate::sync_lock::rw_read(&*orch.context_store)
            .get(&key)
            .expect("worker should seed context envelope key");
        let parsed: crate::ContextEnvelope = serde_json::from_str(&stored).expect("stored context json");
        assert_eq!(parsed.envelope_type, crate::ContextEnvelopeType::RetrievalEvidence);

        server.abort();
    }

    #[cfg(feature = "populi-transport")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancel_populi_remote_delegated_relays_remote_cancel_message() {
        let state = vox_populi::transport::PopuliTransportState::new();
        let seed = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind seed");
        let bound = seed.local_addr().expect("local addr");
        drop(seed);
        let server = tokio::spawn(async move {
            vox_populi::transport::serve(bound, state)
                .await
                .expect("serve");
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let base = format!("http://{bound}");
        let http = vox_populi::http_client::PopuliHttpClient::new(&base);

        let mut cfg = OrchestratorConfig::for_testing();
        cfg.populi_remote_execute_experimental = true;
        cfg.populi_control_url = Some(base);
        cfg.populi_remote_execute_receiver_agent = Some("2".to_string());
        cfg.populi_remote_execute_sender_agent = Some("1".to_string());
        let orch = Orchestrator::new(cfg);
        orch.spawn_agent("solo").expect("spawn");
        let aid = orch.agent_ids()[0];

        let mut task = AgentTask::new(
            TaskId(9902),
            "cancel-remote-net",
            TaskPriority::Normal,
            vec![],
        );
        task.populi_remote_delegate = Some(PopuliRemoteDelegate {
            idempotency_key: "k9902".into(),
            lease_id: None,
            claimer_node_id: None,
        });
        {
            let ql = orch.agent_queue(aid).expect("queue");
            let mut q = ql.write().unwrap();
            q.hold_for_populi_remote(task).expect("hold");
        }
        orch.task_assignments
            .write()
            .unwrap()
            .insert(TaskId(9902), aid);
        orch.cancel_task(TaskId(9902)).expect("cancel");

        let mut saw_cancel = false;
        for _ in 0..20 {
            let inbox = http.relay_a2a_inbox("2").await.expect("inbox");
            if inbox
                .messages
                .iter()
                .any(|m| m.message_type == REMOTE_TASK_CANCEL_TYPE)
            {
                saw_cancel = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        assert!(saw_cancel, "expected remote_task_cancel delivery");

        server.abort();
    }

    #[cfg(feature = "populi-transport")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn lease_renew_loss_requeues_locally_and_relays_cancel() {
        let state = vox_populi::transport::PopuliTransportState::new();
        let seed = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind seed");
        let bound = seed.local_addr().expect("local addr");
        drop(seed);
        let server = tokio::spawn(async move {
            vox_populi::transport::serve(bound, state)
                .await
                .expect("serve");
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let base = format!("http://{bound}");
        let http = vox_populi::http_client::PopuliHttpClient::new(&base);

        let mut cfg = OrchestratorConfig::for_testing();
        cfg.populi_remote_execute_experimental = true;
        cfg.populi_control_url = Some(base.clone());
        cfg.populi_remote_execute_receiver_agent = Some("2".to_string());
        cfg.populi_remote_execute_sender_agent = Some("1".to_string());
        cfg.populi_remote_lease_gating_enabled = true;
        cfg.populi_remote_lease_gated_roles = vec![AgentExecutionRole::Builder];
        let orch = Orchestrator::new(cfg);
        orch.spawn_agent("worker").expect("spawn");
        let hints = TaskEnqueueHints {
            execution_role: Some(AgentExecutionRole::Builder),
            ..Default::default()
        };
        let tid = orch
            .submit_task_with_agent(
                "leased-renew-loss",
                vec![],
                None,
                None,
                None,
                Some(hints),
                None,
            )
            .await
            .expect("submit");
        let aid = *orch
            .task_assignments
            .read()
            .unwrap()
            .get(&tid)
            .expect("assignment");
        let delegate = {
            let ql = orch.agent_queue(aid).expect("queue");
            let q = ql.read().unwrap();
            q.current_task()
                .and_then(|t| t.populi_remote_delegate.clone())
                .expect("delegate")
        };
        let lease_id = delegate.lease_id.clone().expect("lease_id");
        let claimer_node_id = delegate.claimer_node_id.clone().expect("claimer");
        http.exec_lease_release(&vox_populi::transport::RemoteExecLeaseReleaseRequest {
            lease_id,
            claimer_node_id,
        })
        .await
        .expect("force lease loss");

        crate::a2a::populi_remote_result_poll_once(&orch).await;

        let ql = orch.agent_queue(aid).expect("queue");
        let q = ql.read().unwrap();
        assert!(!q.has_in_progress(), "lease-loss should clear remote hold");
        let task = q
            .tasks()
            .iter()
            .find(|t| t.id == tid)
            .expect("requeued task");
        assert!(
            task.populi_remote_delegate.is_none(),
            "fallback requeue should remove remote delegate"
        );
        drop(q);

        let mut saw_cancel = false;
        for _ in 0..20 {
            let inbox = http.relay_a2a_inbox("2").await.expect("inbox");
            if inbox
                .messages
                .iter()
                .any(|m| m.message_type == REMOTE_TASK_CANCEL_TYPE)
            {
                saw_cancel = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        assert!(saw_cancel, "lease-loss fallback should relay cancel");

        server.abort();
    }

    #[cfg(feature = "populi-transport")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_result_poll_respects_max_messages_per_poll() {
        let state = vox_populi::transport::PopuliTransportState::new();
        let seed = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind seed");
        let bound = seed.local_addr().expect("local addr");
        drop(seed);
        let server = tokio::spawn(async move {
            vox_populi::transport::serve(bound, state)
                .await
                .expect("serve");
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let base = format!("http://{bound}");
        let http = vox_populi::http_client::PopuliHttpClient::new(&base);
        let mut cfg = OrchestratorConfig::for_testing();
        cfg.populi_remote_execute_experimental = true;
        cfg.populi_control_url = Some(base.clone());
        cfg.populi_remote_execute_sender_agent = Some("1".to_string());
        cfg.populi_remote_result_max_messages_per_poll = 1;
        let orch = Orchestrator::new(cfg);

        for i in 0..3u64 {
            let aid = orch.spawn_agent(&format!("w{i}")).expect("spawn");
            let tid = TaskId(12_000 + i);
            let mut task = AgentTask::new(tid, format!("held-{i}"), TaskPriority::Normal, vec![]);
            task.populi_remote_delegate = Some(PopuliRemoteDelegate {
                idempotency_key: format!("orch-remote-{}-t", tid.0),
                lease_id: None,
                claimer_node_id: None,
            });
            {
                let ql = orch.agent_queue(aid).expect("queue");
                let mut q = ql.write().unwrap();
                q.hold_for_populi_remote(task).expect("hold");
            }
            orch.task_assignments.write().unwrap().insert(tid, aid);
            let payload = serde_json::to_string(&RemoteTaskResult {
                idempotency_key: format!("orch-remote-{}-t", tid.0),
                success: false,
                result: None,
                error: Some("fail".to_string()),
                task_id: Some(tid.0),
            })
            .expect("serialize");
            http.relay_a2a(&vox_populi::transport::A2ADeliverRequest {
                sender_agent_id: "2".into(),
                receiver_agent_id: "1".into(),
                message_type: REMOTE_TASK_RESULT_TYPE.to_string(),
                payload,
                idempotency_key: Some(format!("remote-result-{}", tid.0)),
                privacy_class: None,
                payload_blake3_hex: None,
                worker_ed25519_sig_b64: None,
                jwe_payload: None,
            })
            .await
            .expect("relay result");
        }

        crate::a2a::populi_remote_result_poll_once(&orch).await;
        let after_one = orch
            .agent_ids()
            .into_iter()
            .filter(|aid| {
                let ql = orch.agent_queue(*aid).expect("queue");
                ql.read().unwrap().has_in_progress()
            })
            .count();
        assert_eq!(
            after_one, 2,
            "max-per-poll=1 should clear one held task per tick"
        );
        let inbox_after_one = http.relay_a2a_inbox("1").await.expect("inbox");
        let remaining_after_one = inbox_after_one
            .messages
            .iter()
            .filter(|m| m.message_type == REMOTE_TASK_RESULT_TYPE)
            .count();
        assert_eq!(remaining_after_one, 2);

        crate::a2a::populi_remote_result_poll_once(&orch).await;
        crate::a2a::populi_remote_result_poll_once(&orch).await;
        let after_three = orch
            .agent_ids()
            .into_iter()
            .filter(|aid| {
                let ql = orch.agent_queue(*aid).expect("queue");
                ql.read().unwrap().has_in_progress()
            })
            .count();
        assert_eq!(
            after_three, 0,
            "all held tasks should clear after three polls"
        );
        let inbox_after_three = http.relay_a2a_inbox("1").await.expect("inbox");
        let remaining_after_three = inbox_after_three
            .messages
            .iter()
            .filter(|m| m.message_type == REMOTE_TASK_RESULT_TYPE)
            .count();
        assert_eq!(remaining_after_three, 0);

        server.abort();
    }

    #[cfg(feature = "populi-transport")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn non_lease_remote_relay_includes_session_and_context_payload() {
        let state = vox_populi::transport::PopuliTransportState::new();
        let seed = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind seed");
        let bound = seed.local_addr().expect("local addr");
        drop(seed);
        let server = tokio::spawn(async move {
            vox_populi::transport::serve(bound, state)
                .await
                .expect("serve");
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let base = format!("http://{bound}");
        let http = vox_populi::http_client::PopuliHttpClient::new(&base);

        let mut cfg = OrchestratorConfig::for_testing();
        cfg.populi_remote_execute_experimental = true;
        cfg.populi_control_url = Some(base.clone());
        cfg.populi_remote_execute_receiver_agent = Some("2".to_string());
        cfg.populi_remote_execute_sender_agent = Some("1".to_string());
        cfg.populi_remote_lease_gating_enabled = false;
        let orch = Orchestrator::new(cfg);
        orch.spawn_agent("worker").expect("spawn");

        let sid = "non-lease-relay-session";
        let retrieval = crate::SessionRetrievalEnvelope {
            retrieval_tier: "hybrid".to_string(),
            memory_hit_count: 1,
            knowledge_hit_count: 1,
            chunk_hit_count: 0,
            repo_hit_count: 0,
            rrf_fused_hit_count: 0,
            used_vector: true,
            used_bm25: true,
            used_lexical_fallback: false,
            contradiction_count: 0,
            source_diversity: 2,
            evidence_quality: 0.8,
            citation_coverage: 0.8,
            verification_performed: false,
            verification_reason: None,
            recommended_next_action: None,
        };
        let context = crate::ContextEnvelope::from_session_retrieval("repo-non-lease", sid, &retrieval);
        let context_json = serde_json::to_string(&context).expect("serialize context envelope");
        let key = crate::socrates::session_context_envelope_key(sid);
        crate::sync_lock::rw_write(&*orch.context_store).set(
            crate::types::AgentId(0),
            key,
            &context_json,
            3600,
        );

        let _tid = orch
            .submit_task_with_agent(
                "non-lease remote relay context",
                vec![],
                None,
                None,
                None,
                None,
                Some(sid.to_string()),
            )
            .await
            .expect("submit");

        let mut relayed_payload: Option<serde_json::Value> = None;
        for _ in 0..25 {
            let inbox = http.relay_a2a_inbox("2").await.expect("inbox");
            if let Some(msg) = inbox
                .messages
                .iter()
                .find(|m| m.message_type == REMOTE_TASK_ENVELOPE_TYPE)
            {
                let env: RemoteTaskEnvelope =
                    serde_json::from_str(&msg.payload).expect("remote envelope parse");
                relayed_payload =
                    Some(serde_json::from_str::<serde_json::Value>(&env.payload).expect("payload parse"));
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        let payload = relayed_payload.expect("expected non-lease relay payload");
        assert_eq!(payload["session_id"], serde_json::json!(sid));
        assert_eq!(payload["context_envelope_json"], serde_json::json!(context_json));

        server.abort();
    }
}

#[cfg(test)]
mod route_replay_tests {
    use super::*;
    use crate::groups::{AffinityGroup, AffinityGroupRegistry};
    use crate::types::{FileAffinity, PopuliRemoteDelegate};

    fn agent_id_named(orch: &Orchestrator, name: &str) -> crate::types::AgentId {
        let agents = orch.agents.read().unwrap();
        for (id, q) in agents.iter() {
            if q.read().unwrap().name == name {
                return *id;
            }
        }
        panic!("no agent named {name}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn replay_moves_queued_task_to_group_default_agent() {
        let orch = Orchestrator::new(OrchestratorConfig::for_testing());
        orch.spawn_agent("heavy").expect("spawn heavy");
        let light_id = orch.spawn_agent("light").expect("spawn light");
        {
            let mut groups = orch.groups.write().unwrap();
            *groups = AffinityGroupRegistry::new(vec![AffinityGroup {
                name: "route-replay-fixture".to_string(),
                patterns: vec!["**/route_replay_fixture/**".to_string()],
                default_agent: Some(light_id),
            }]);
        }
        let path = FileAffinity::write("route_replay_fixture/task.rs");
        let tid = orch
            .submit_task_with_agent(
                "affinity replay",
                vec![path],
                None,
                Some("heavy".into()),
                None,
                None,
                None,
            )
            .await
            .expect("submit");
        let heavy_id = agent_id_named(&orch, "heavy");
        assert_eq!(
            *orch
                .task_assignments
                .read()
                .unwrap()
                .get(&tid)
                .expect("assignment"),
            heavy_id
        );

        let moved = orch
            .replay_queued_routes_after_populi_schedulable_drop()
            .await;
        assert!(
            moved >= 1,
            "expected route replay to move at least one queued task toward group default"
        );
        assert_eq!(
            *orch
                .task_assignments
                .read()
                .unwrap()
                .get(&tid)
                .expect("assignment after replay"),
            light_id
        );
        let q_heavy = orch.agent_queue(heavy_id).expect("heavy queue");
        assert!(
            !q_heavy
                .read()
                .unwrap()
                .tasks()
                .iter()
                .any(|t| t.id == tid),
            "task should leave heavy pending queue"
        );
        let q_light = orch.agent_queue(light_id).expect("light queue");
        assert!(
            q_light
                .read()
                .unwrap()
                .tasks()
                .iter()
                .any(|t| t.id == tid),
            "task should land on light"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn replay_skips_tasks_with_populi_remote_delegate() {
        let orch = Orchestrator::new(OrchestratorConfig::for_testing());
        orch.spawn_agent("heavy").expect("spawn heavy");
        let light_id = orch.spawn_agent("light").expect("spawn light");
        {
            let mut groups = orch.groups.write().unwrap();
            *groups = AffinityGroupRegistry::new(vec![AffinityGroup {
                name: "route-replay-fixture".to_string(),
                patterns: vec!["**/route_replay_fixture/**".to_string()],
                default_agent: Some(light_id),
            }]);
        }
        let tid = orch
            .submit_task_with_agent(
                "delegate hold",
                vec![FileAffinity::write(
                    "route_replay_fixture/delegate_skip.rs",
                )],
                None,
                Some("heavy".into()),
                None,
                None,
                None,
            )
            .await
            .expect("submit");
        let heavy_id = agent_id_named(&orch, "heavy");
        let mut task = {
            let q = orch.agent_queue(heavy_id).expect("heavy queue");
            let mut w = q.write().unwrap();
            w.cancel(tid).expect("task on heavy queue")
        };
        task.populi_remote_delegate = Some(PopuliRemoteDelegate {
            idempotency_key: "replay-delegate-skip".into(),
            lease_id: None,
            claimer_node_id: None,
        });
        {
            let q = orch.agent_queue(heavy_id).expect("heavy queue");
            q.write().unwrap().enqueue(task);
        }

        let moved = orch
            .replay_queued_routes_after_populi_schedulable_drop()
            .await;
        assert_eq!(moved, 0, "delegate tasks must not be replay-routed");
        assert_eq!(
            *orch
                .task_assignments
                .read()
                .unwrap()
                .get(&tid)
                .expect("assignment"),
            heavy_id
        );
        let q_heavy = orch.agent_queue(heavy_id).expect("heavy queue");
        assert!(
            q_heavy
                .read()
                .unwrap()
                .tasks()
                .iter()
                .any(|t| t.id == tid),
            "task stays pending on heavy"
        );
    }
}