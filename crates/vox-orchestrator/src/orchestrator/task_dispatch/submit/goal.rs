use crate::planning::{PlanningMode, PlanningStrategy};
use crate::types::{FileAffinity, TaskId, TaskPriority};

use super::super::super::{Orchestrator, OrchestratorError};

impl Orchestrator {
    /// If the context store holds a session-scoped retrieval envelope, attach it to the task.
    pub(crate) fn attach_session_retrieval_envelope_if_present(
        &self,
        task_id: TaskId,
        session_id: &Option<String>,
    ) {
        let Some(sid) = session_id.as_ref() else {
            return;
        };
        let key = crate::socrates::session_retrieval_envelope_key(sid);
        let raw_opt = crate::sync_lock::rw_read(&*self.context_store).get(&key);
        if let Some(raw) = raw_opt {
            if let Ok(env) = serde_json::from_str::<crate::socrates::SessionRetrievalEnvelope>(&raw)
            {
                if let Err(e) = self.attach_socrates_context(task_id, env.to_task_context()) {
                    tracing::debug!(
                        task_id = task_id.0,
                        error = %e,
                        "session retrieval envelope parse OK but Socrates attach failed"
                    );
                }
            }
        }
    }

    /// Submit a higher-level goal that may be routed through planning.
    pub async fn submit_goal(
        &self,
        goal: impl Into<String>,
        file_manifest: Vec<FileAffinity>,
        priority: Option<TaskPriority>,
        planning_mode: Option<PlanningMode>,
        session_id: Option<String>,
    ) -> Result<TaskId, OrchestratorError> {
        let goal = goal.into();
        let cfg = crate::sync_lock::rw_read(&*self.config).clone();
        if planning_mode.is_none()
            && (!cfg.planning_auto_mode_enabled || cfg.planning_rollout_percent == 0)
        {
            return self
                .submit_task_with_agent(goal, file_manifest, priority, None, None, session_id)
                .await;
        }
        if planning_mode.is_none() {
            let selector = xxhash_rust::xxh3::xxh3_64(goal.as_bytes()) % 100;
            if selector >= u64::from(cfg.planning_rollout_percent) {
                return self
                    .submit_task_with_agent(goal, file_manifest, priority, None, None, session_id)
                    .await;
            }
        }
        let eval = crate::planning::intake_router::evaluate_goal(&cfg, &goal, planning_mode);
        self.event_bus
            .emit(crate::events::AgentEventKind::PlanningRouted {
                strategy: format!("{:?}", eval.strategy),
                complexity: eval.complexity,
                confidence: eval.confidence,
                rationale: eval.rationale.clone(),
            });

        if cfg.planning_shadow_mode || eval.strategy == PlanningStrategy::ImmediateAct {
            return self
                .submit_task_with_agent(goal, file_manifest, priority, None, None, session_id)
                .await;
        }

        if eval.strategy == PlanningStrategy::WorkflowHandoff
            && cfg.planning_workflow_handoff_enabled
        {
            return self
                .submit_workflow_handoff_goal(goal, file_manifest, priority, session_id)
                .await;
        }

        let plan_session_id = format!("plan-{}", uuid::Uuid::new_v4());
        let plan_version = 1_u32;
        let nodes = crate::planning::synthesizer::synthesize_plan_nodes(&goal);
        if let Some(db) = self.db() {
            let strategy = format!("{:?}", eval.strategy);
            let _ = db
                .create_plan_session(&plan_session_id, session_id.as_deref(), &goal, &strategy)
                .await;
            let _ = db
                .append_plan_version(&plan_session_id, plan_version as i64, None, None, None)
                .await;
            for n in &nodes {
                let deps_json =
                    serde_json::to_string(&n.depends_on).unwrap_or_else(|_| "[]".to_string());
                let pol_json =
                    serde_json::to_string(&n.execution_policy).unwrap_or_else(|_| "{}".to_string());
                let _ = db
                    .upsert_plan_node(
                        &plan_session_id,
                        plan_version as i64,
                        &n.node_id,
                        &n.description,
                        &deps_json,
                        &pol_json,
                        "pending",
                        n.workflow_invocation.as_deref(),
                    )
                    .await;
            }
        }
        self.event_bus
            .emit(crate::events::AgentEventKind::PlanSessionCreated {
                plan_session_id: plan_session_id.clone(),
                strategy: format!("{:?}", eval.strategy),
                version: plan_version as i64,
            });

        let first = nodes
            .first()
            .cloned()
            .unwrap_or_else(|| crate::planning::PlanNode {
                node_id: "n1".to_string(),
                description: goal.clone(),
                depends_on: vec![],
                status: crate::planning::PlanStatus::Pending,
                execution_policy: crate::planning::ExecutionPolicy::default(),
                workflow_invocation: None,
            });
        crate::planning::executor_bridge::enqueue_plan_node(
            self,
            &first,
            &plan_session_id,
            plan_version,
            session_id,
        )
        .await
    }
}
