use crate::planning::{PlanningMode, PlanningStrategy};
use crate::types::{AccessKind, FileAffinity, TaskId, TaskPriority};
use std::collections::HashSet;

use super::super::super::{Orchestrator, OrchestratorError};

fn merge_file_affinities(into: &mut Vec<FileAffinity>, extra: &[FileAffinity]) {
    let mut have: HashSet<(std::path::PathBuf, AccessKind)> =
        into.iter().map(|f| (f.path.clone(), f.access)).collect();
    for f in extra {
        let key = (f.path.clone(), f.access);
        if have.insert(key) {
            into.push(f.clone());
        }
    }
}

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
        enqueue_hints: Option<crate::types::TaskEnqueueHints>,
    ) -> Result<TaskId, OrchestratorError> {
        let goal = goal.into();
        let cfg = crate::sync_lock::rw_read(&*self.config).clone();
        if planning_mode.is_none()
            && (!cfg.planning_auto_mode_enabled || cfg.planning_rollout_percent == 0)
        {
            return self
                .submit_task_with_agent(
                    goal,
                    file_manifest,
                    priority,
                    None,
                    None,
                    enqueue_hints.clone(),
                    session_id,
                )
                .await;
        }
        if planning_mode.is_none() {
            let selector = xxhash_rust::xxh3::xxh3_64(goal.as_bytes()) % 100;
            if selector >= u64::from(cfg.planning_rollout_percent) {
                return self
                    .submit_task_with_agent(
                        goal,
                        file_manifest,
                        priority,
                        None,
                        None,
                        enqueue_hints.clone(),
                        session_id,
                    )
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
                .submit_task_with_agent(
                    goal,
                    file_manifest,
                    priority,
                    None,
                    None,
                    enqueue_hints.clone(),
                    session_id,
                )
                .await;
        }

        if eval.strategy == PlanningStrategy::WorkflowHandoff
            && cfg.planning_workflow_handoff_enabled
        {
            return self
                .submit_workflow_handoff_goal(
                    goal,
                    file_manifest,
                    priority,
                    session_id,
                    enqueue_hints,
                )
                .await;
        }

        let plan_session_id = format!("plan-{}", uuid::Uuid::new_v4());
        let plan_version = 1_u32;
        let mut nodes = crate::planning::synthesizer::synthesize_plan_nodes(&goal);
        for n in &mut nodes {
            if let Some(ref h) = enqueue_hints {
                n.execution_policy.enqueue_hints = Some(h.clone());
            }
            if !file_manifest.is_empty() {
                merge_file_affinities(&mut n.execution_policy.file_manifest, &file_manifest);
            }
        }
        crate::planning::quality_gate::validate_plan_nodes(&nodes)?;
        let db_opt = self.db();
        if let Some(db) = db_opt.as_ref() {
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

        if crate::lineage::orchestration_lineage_persist_enabled() {
            if let Some(db) = self.db() {
                let repo = crate::lineage::repository_id();
                let mut payload = serde_json::json!({
                    "strategy": format!("{:?}", eval.strategy),
                    "plan_version": plan_version,
                    "node_count": nodes.len(),
                    "goal_preview": goal.chars().take(240).collect::<String>(),
                });
                if let Some(cid) = crate::lineage::orchestration_campaign_id() {
                    payload["campaign_id"] = serde_json::Value::String(cid);
                }
                let payload_str = payload.to_string();
                let _ = db
                    .append_orchestration_lineage_event(
                        &repo,
                        "plan_session_created",
                        0_i64,
                        None,
                        session_id.as_deref(),
                        None,
                        Some(plan_session_id.as_str()),
                        None,
                        Some(payload_str.as_str()),
                    )
                    .await;
            }
        }

        if db_opt.is_some() {
            let enqueued = crate::planning::schedule::enqueue_runnable_plan_nodes(
                self,
                &plan_session_id,
                plan_version,
                session_id.clone(),
            )
            .await?;
            return enqueued.into_iter().next().ok_or_else(|| {
                OrchestratorError::DatabaseError(
                    "planning produced no initial runnable nodes".into(),
                )
            });
        }

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
