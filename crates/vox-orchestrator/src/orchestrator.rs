//! Central coordinator for queues, affinity, locks, scope, and JJ-style undo metadata.
//!
//! [`Orchestrator`] is the integration point for routing services, Codex-backed stores,
//! and runtime agent processes when the `runtime` feature is enabled.
//!
//! ## Sub-module structure (decomposed from the original god-object)
//!
//! | Sub-module | Responsibility |
//! |---|---|
//! | [`core`] | `new`, `with_groups`, `init_db`, `record_ai_usage` |
//! | [`agent_lifecycle`] | `spawn_agent`, `retire_agent`, `pause/resume`, `heartbeat`, `handoff` |
//! | [`scaling`] | `rebalance`, `tick` |
//! | [`vcs_ops`] | `capture_snapshot`, `take_db_snapshot`, `undo/redo_operation` |

mod core;
mod agent_lifecycle;
mod scaling;
mod vcs_ops;


use std::collections::HashMap;
use std::path::PathBuf;

use crate::affinity::FileAffinityMap;
use crate::bulletin::BulletinBoard;
use crate::config::OrchestratorConfig;
use crate::groups::AffinityGroupRegistry;
use crate::locks::{FileLockManager, LockKind};
use crate::oplog::OperationKind;
use crate::queue::AgentQueue;
use crate::scope::{ScopeEnforcement, ScopeGuard};
use crate::services::{
    MessageGateway, PolicyCheckResult, PolicyEngine, RouteResult, RoutingService,
};
use crate::types::{
    AccessKind, AgentId, AgentIdGenerator, AgentTask, FileAffinity, TaskId, TaskIdGenerator,
    TaskPriority, TaskStatus,
};

/// Error type for orchestrator operations.
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    /// Orchestrator is turned off via configuration.
    #[error("Orchestrator is disabled")]
    Disabled,
    /// No additional agent slots remain.
    #[error("Maximum agents ({max}) reached")]
    MaxAgentsReached {
        /// Configured hard cap on concurrent agents.
        max: usize,
    },
    /// Lookup failed for the given agent id.
    #[error("Agent {0} not found")]
    AgentNotFound(AgentId),
    /// Lookup failed for the given task id.
    #[error("Task {0} not found")]
    TaskNotFound(TaskId),
    /// File lock could not be acquired.
    #[error("Lock conflict: {0}")]
    LockConflict(#[from] crate::locks::LockConflict),
    /// Path violated scope / affinity rules.
    #[error("Scope denied: {0}")]
    ScopeDenied(String),
    /// Undo/redo referenced a missing oplog entry.
    #[error("Operation not found")]
    OperationNotFound,
    /// Persistent layer failure surfaced to callers.
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// One step in a task's lifecycle timeline (ingress → route → verification → outcome).
/// ORCH-01 SPLIT TARGET: Types (OrchestratorError, TaskTraceStep, OrchestratorStatus, AgentSummary)
/// → move to orchestrator/types.rs when decomposing this file into sub-modules.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskTraceStep {
    /// Pipeline stage name (submit, route, verify, complete, …).
    pub stage: String,
    /// When this step was recorded (Unix ms).
    pub timestamp_ms: u64,
    /// Optional structured payload or error text.
    pub detail: Option<String>,
}

const MAX_TASK_TRACES: usize = 200;

/// Snapshot of the orchestrator state for display.
#[derive(Debug, serde::Serialize)]
pub struct OrchestratorStatus {
    /// Whether the orchestrator accepts new work.
    pub enabled: bool,
    /// Registered agents (static + dynamic).
    pub agent_count: usize,
    /// Tasks waiting across all queues.
    pub total_queued: usize,
    /// Tasks currently executing.
    pub total_in_progress: usize,
    /// Tasks finished since start (approximate counter).
    pub total_completed: usize,
    /// Distinct paths under lock.
    pub locked_files: usize,
    /// Aggregate lock wait / conflict events (policy-specific).
    pub total_contention: usize,
    /// Sum of weighted queue depths for scaling heuristics.
    pub total_weighted_load: f64,
    /// Smoothed forecast of near-future load.
    pub predicted_load: f64,
    /// Agents pinned or reserved for scaling policy.
    pub reserved_agents: usize,
    /// Ephemeral agents spawned for burst handling.
    pub dynamic_agents: usize,
    /// Shared context keys visible to dashboards.
    pub context_entries: std::collections::HashMap<String, crate::context::ContextEntry>,
    /// Per-agent rollups for UI tables.
    pub agents: Vec<AgentSummary>,
}

/// Summary info for one agent.
#[derive(Debug, serde::Serialize)]
pub struct AgentSummary {
    /// Agent id.
    pub id: AgentId,
    /// Display name.
    pub name: String,
    /// Tasks waiting in this agent's queue.
    pub queued: usize,
    /// Urgent-priority backlog depth.
    pub urgent_count: usize,
    /// Normal-priority backlog depth.
    pub normal_count: usize,
    /// Background-priority backlog depth.
    pub background_count: usize,
    /// Whether a task is actively running.
    pub in_progress: bool,
    /// Completed tasks attributed to this agent.
    pub completed: usize,
    /// Operator paused this agent.
    pub paused: bool,
    /// Files this agent currently owns for writing.
    pub owned_files: usize,
    /// True if spawned dynamically for overflow.
    pub dynamic: bool,
    /// Load score combining priorities and in-flight work.
    pub weighted_load: f64,
    /// Linked Codex session id when known.
    pub agent_session_id: Option<String>,
}

/// The central coordinator for the multi-agent file-affinity queue system.
pub struct Orchestrator {
    config: OrchestratorConfig,
    affinity_map: FileAffinityMap,
    lock_manager: FileLockManager,
    context_store: crate::context::ContextStore,
    budget_manager: crate::budget::BudgetManager,
    summary_manager: crate::summary::SummaryManager,
    models: crate::models::ModelRegistry,
    bulletin: BulletinBoard,
    agents: HashMap<AgentId, AgentQueue>,
    groups: AffinityGroupRegistry,
    task_id_gen: TaskIdGenerator,
    agent_id_gen: AgentIdGenerator,
    /// Maps task IDs to the agent they were assigned to.
    task_assignments: HashMap<TaskId, AgentId>,
    qa_router: crate::qa::QARouter,
    monitor: crate::monitor::AiMonitor,
    event_bus: crate::events::EventBus,
    message_bus: crate::a2a::MessageBus,
    /// IDs of agents that were dynamically spawned (transient).
    dynamic_agents: std::collections::HashSet<AgentId>,
    /// Handles to the running agent processes.
    agent_handles: HashMap<AgentId, vox_runtime::ProcessHandle>,
    heartbeat_monitor: crate::heartbeat::HeartbeatMonitor,
    /// System resource monitor.
    #[cfg(feature = "system-metrics")]
    sys: sysinfo::System,
    /// Historical system load for predictive scaling.
    load_history: std::collections::VecDeque<f64>,
    /// Scope guard for write boundaries (synced with affinity on assign/retire).
    scope_guard: ScopeGuard,
    /// Per-task timeline (ingress → route → outcome), capped at MAX_TASK_TRACES.
    task_traces: HashMap<TaskId, Vec<TaskTraceStep>>,
    /// **Codex** database handle (Turso/libSQL).
    db: Option<std::sync::Arc<vox_db::VoxDb>>,
    // -- JJ-inspired subsystems --
    /// Auto-snapshot store for tracking file state changes.
    snapshot_store: crate::snapshot::SnapshotStore,
    /// Operation log for universal undo/redo.
    oplog: crate::oplog::OpLog,
    /// First-class conflict tracking.
    conflict_manager: crate::conflicts::ConflictManager,
    /// Per-agent virtual workspaces and change tracking.
    workspace_manager: crate::workspace::WorkspaceManager,
    /// Timestamp of the last rebalance (for cooldown enforcement).
    last_rebalance_at: Option<std::time::Instant>,
    /// Last remote mesh snapshot hints (from MCP federation poller); read-only placement signals.
    remote_mesh_routing_hints: Vec<crate::mesh_federation::RemoteMeshRoutingHint>,
}

impl Orchestrator {
    /// Create a new orchestrator with the given configuration.
    // ORCH-01 SPLIT TARGET:
    //   new() / with_groups() / init_db() → orchestrator/core.rs
    //   submit_task*() / submit_batch() / resolve_route() / spawn_agent*() → orchestrator/task_dispatch.rs
    //   map_agent_session() / retire_agent() / heartbeat() / pause/resume_agent() → orchestrator/agent_state.rs
    //   All construction, lifecycle, scaling, and VCS methods are in sub-modules:
    //   core.rs, agent_lifecycle.rs, scaling.rs, vcs_ops.rs

    /// Submit a new task to the orchestrator (async).
    ///
    /// The orchestrator will:
    /// 1. Analyze the file manifest against the affinity map
    /// 2. Route the task to an existing agent or spawn a new one
    /// 3. Acquire file locks
    /// 4. Enqueue the task
    pub async fn submit_task(
        &mut self,
        description: impl Into<String>,
        file_manifest: Vec<FileAffinity>,
        priority: Option<TaskPriority>,
    ) -> Result<TaskId, OrchestratorError> {
        self.submit_task_with_agent(description, file_manifest, priority, None, None)
            .await
    }

    /// Submit a new task to the orchestrator, potentially targeting a specific agent name (async).
    pub async fn submit_task_with_agent(
        &mut self,
        description: impl Into<String>,
        file_manifest: Vec<FileAffinity>,
        priority: Option<TaskPriority>,
        target_agent: Option<String>,
        capability_requirements: Option<crate::contract::TaskCapabilityHints>,
    ) -> Result<TaskId, OrchestratorError> {
        if !self.config.enabled {
            return Err(OrchestratorError::Disabled);
        }

        let task_id = self.task_id_gen.next();
        let priority = priority.unwrap_or(self.config.default_priority);

        let mut task = AgentTask::new(task_id, description, priority, file_manifest.clone());
        task.capability_requirements = capability_requirements.clone();

        // Route to the right agent via RoutingService
        let agent_id = self
            .resolve_route(
                &file_manifest,
                target_agent.as_deref(),
                capability_requirements.as_ref(),
            )
            .await?;

        // Pre-queue policy check (locks; scope when enforcement enabled)
        let scope_guard = (self.config.scope_enforcement != ScopeEnforcement::Disabled)
            .then_some(&self.scope_guard);
        match PolicyEngine::check_before_queue(
            &self.lock_manager,
            scope_guard,
            &self.event_bus,
            &file_manifest,
            agent_id,
        ) {
            PolicyCheckResult::Allowed => {}
            PolicyCheckResult::LockConflict(e) => return Err(OrchestratorError::LockConflict(e)),
            PolicyCheckResult::ScopeDenied(msg) => return Err(OrchestratorError::ScopeDenied(msg)),
        }

        // Try to acquire locks for write files
        for fa in &file_manifest {
            if fa.access == AccessKind::Write {
                let lock_kind = LockKind::Exclusive;
                // If lock fails, we still enqueue (the agent will retry when it picks up the task)
                let _ = self.lock_manager.try_acquire(&fa.path, agent_id, lock_kind);
            }
        }

        // Assign files to the agent in the affinity map and scope guard
        for fa in &file_manifest {
            if fa.access == AccessKind::Write {
                self.affinity_map.assign(&fa.path, agent_id);
                self.scope_guard.assign_file(agent_id, fa.path.clone());
            }
        }

        // Capture pre-task snapshot for version control (persisted to CodeStore)
        let snapshot_before = {
            let paths: Vec<PathBuf> = file_manifest.iter().map(|f| f.path.clone()).collect();
            let desc_str = task.description.clone();
            let snap_desc = format!("pre-task: {:.50}", desc_str);
            let snap_id = self
                .capture_snapshot(agent_id, &paths, snap_desc.clone())
                .await;
            self.event_bus
                .emit(crate::events::AgentEventKind::SnapshotCaptured {
                    agent_id,
                    snapshot_id: snap_id.to_string(),
                    file_count: paths.len(),
                    description: snap_desc,
                });
            snap_id
        };

        self.record_operation(
            agent_id,
            OperationKind::TaskSubmit { task_id: task_id.0 },
            format!("Submitted task {}", task_id),
            Some(snapshot_before),
            None,
            None,
            None,
        )
        .await;

        // Enqueue the task
        if let Some(queue) = self.agents.get_mut(&agent_id) {
            queue.enqueue(task);
            self.task_assignments.insert(task_id, agent_id);

            // Notify the agent process to wake up and process
            if let Some(handle) = self.agent_handles.get(&agent_id) {
                let json = serde_json::to_string(&crate::runtime::AgentCommand::ProcessQueue)
                    .unwrap_or_else(|e| {
                        tracing::warn!("serialize ProcessQueue: {e}");
                        "{}".to_string()
                    });
                let env = vox_runtime::mailbox::Envelope::Message(vox_runtime::mailbox::Message {
                    from: vox_runtime::Pid::new(),
                    payload: vox_runtime::mailbox::MessagePayload::Json(json),
                });
                let _ = handle.send(env).await;
            }

            tracing::info!(
                "Task {} routed to agent {} (queue len: {})",
                task_id,
                agent_id,
                queue.len()
            );
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        if self.task_traces.len() >= MAX_TASK_TRACES {
            if let Some(min_id) = self.task_traces.keys().min().copied() {
                self.task_traces.remove(&min_id);
            }
        }
        self.task_traces.insert(
            task_id,
            vec![
                TaskTraceStep {
                    stage: "ingress".to_string(),
                    timestamp_ms: now_ms,
                    detail: None,
                },
                TaskTraceStep {
                    stage: "routed".to_string(),
                    timestamp_ms: now_ms,
                    detail: Some(format!("agent {}", agent_id)),
                },
            ],
        );

        Ok(task_id)
    }

    /// Submit a batch of interdependent tasks (async).
    pub async fn submit_batch(
        &mut self,
        descriptors: Vec<crate::types::TaskDescriptor>,
    ) -> Result<Vec<TaskId>, OrchestratorError> {
        if !self.config.enabled {
            return Err(OrchestratorError::Disabled);
        }

        let mut assigned_ids: Vec<TaskId> = Vec::with_capacity(descriptors.len());

        // Pre-allocate task IDs
        for _ in 0..descriptors.len() {
            assigned_ids.push(self.task_id_gen.next());
        }

        let mut results = Vec::new();

        // Second pass: construct tasks with resolved IDs and submit
        for (i, mut desc) in descriptors.into_iter().enumerate() {
            let my_id = assigned_ids[i];

            // Resolve temporary deps into actual TaskIds
            for tmp_dep_idx in desc.temp_deps {
                if tmp_dep_idx < assigned_ids.len() {
                    desc.depends_on.push(assigned_ids[tmp_dep_idx]);
                } else {
                    tracing::warn!(
                        "Task descriptor {} referenced out-of-bounds temp dep {}",
                        i,
                        tmp_dep_idx
                    );
                }
            }

            let priority = desc.priority.unwrap_or(self.config.default_priority);
            let mut task = AgentTask::new(
                my_id,
                desc.description.clone(),
                priority,
                desc.file_manifest.clone(),
            );
            task.capability_requirements = desc.capability_requirements.clone();

            // Add all collected deps
            for dep in desc.depends_on {
                task = task.depends_on(dep);
            }

            // Route to best agent via RoutingService
            let agent_id = self
                .resolve_route(
                    &desc.file_manifest,
                    None,
                    desc.capability_requirements.as_ref(),
                )
                .await?;

            let scope_guard = (self.config.scope_enforcement != ScopeEnforcement::Disabled)
                .then_some(&self.scope_guard);
            match PolicyEngine::check_before_queue(
                &self.lock_manager,
                scope_guard,
                &self.event_bus,
                &desc.file_manifest,
                agent_id,
            ) {
                PolicyCheckResult::Allowed => {}
                PolicyCheckResult::LockConflict(e) => {
                    return Err(OrchestratorError::LockConflict(e));
                }
                PolicyCheckResult::ScopeDenied(msg) => {
                    return Err(OrchestratorError::ScopeDenied(msg));
                }
            }

            // Acquire locks and assign scope
            for fa in &desc.file_manifest {
                if fa.access == AccessKind::Write {
                    let _ = self
                        .lock_manager
                        .try_acquire(&fa.path, agent_id, LockKind::Exclusive);
                    self.affinity_map.assign(&fa.path, agent_id);
                    self.scope_guard.assign_file(agent_id, fa.path.clone());
                }
            }

            // Capture pre-task snapshot for version control
            let snapshot_before = {
                let paths: Vec<PathBuf> =
                    desc.file_manifest.iter().map(|f| f.path.clone()).collect();
                self.capture_snapshot(
                    agent_id,
                    &paths,
                    format!("pre-task batch: {:.50}", task.description),
                )
                .await
            };

            self.record_operation(
                agent_id,
                OperationKind::TaskSubmit { task_id: my_id.0 },
                format!("Submitted batch task {}", my_id),
                Some(snapshot_before),
                None,
                None,
                None,
            )
            .await;

            // Enqueue
            if let Some(queue) = self.agents.get_mut(&agent_id) {
                queue.enqueue(task);
                self.task_assignments.insert(my_id, agent_id);

                // Notify
                if let Some(handle) = self.agent_handles.get(&agent_id) {
                    let json = serde_json::to_string(&crate::runtime::AgentCommand::ProcessQueue)
                        .unwrap_or_else(|e| {
                            tracing::warn!("serialize ProcessQueue: {e}");
                            "{}".to_string()
                        });
                    let env =
                        vox_runtime::mailbox::Envelope::Message(vox_runtime::mailbox::Message {
                            from: vox_runtime::Pid::new(),
                            payload: vox_runtime::mailbox::MessagePayload::Json(json),
                        });
                    let _ = handle.send(env).await;
                }
            }

            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            if self.task_traces.len() >= MAX_TASK_TRACES {
                if let Some(min_id) = self.task_traces.keys().min().copied() {
                    self.task_traces.remove(&min_id);
                }
            }
            self.task_traces.insert(
                my_id,
                vec![
                    TaskTraceStep {
                        stage: "ingress".to_string(),
                        timestamp_ms: now_ms,
                        detail: None,
                    },
                    TaskTraceStep {
                        stage: "routed".to_string(),
                        timestamp_ms: now_ms,
                        detail: Some(format!("agent {}", agent_id)),
                    },
                ],
            );

            results.push(my_id);
        }

        tracing::info!("Submitted batch of {} tasks", results.len());
        Ok(results)
    }

    /// Resolve route via RoutingService and spawn if needed.
    async fn resolve_route(
        &mut self,
        manifest: &[FileAffinity],
        target_agent: Option<&str>,
        task_capability_requirements: Option<&crate::contract::TaskCapabilityHints>,
    ) -> Result<AgentId, OrchestratorError> {
        if let Some(agent_name) = target_agent {
            // First check if an agent with this name exists
            for (id, queue) in &self.agents {
                if queue.name == agent_name {
                    return Ok(*id);
                }
            }
            // Otherwise, spawn an agent with this name
            return self.spawn_agent(agent_name);
        }

        let reliability_map: Option<HashMap<AgentId, f64>> =
            if self.config.socrates_reputation_routing {
                self.db.as_ref().map(|db| {
                    db.store()
                        .block_on(async { db.store().list_agent_reliability().await })
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(id, r)| {
                            let numeric_id = id.parse::<u64>().unwrap_or(0);
                            (AgentId(numeric_id), r)
                        })
                        .collect()
                })
            } else {
                None
            };

        let remote = if self.remote_mesh_routing_hints.is_empty() {
            None
        } else {
            Some(self.remote_mesh_routing_hints.as_slice())
        };
        let result = RoutingService::route(
            manifest,
            &self.affinity_map,
            &self.groups,
            &self.agents,
            &self.config,
            reliability_map.as_ref(),
            task_capability_requirements,
            remote,
        );
        match result {
            RouteResult::Existing(id) => Ok(id),
            RouteResult::SpawnAgent(name) => self.spawn_agent(&name),
        }
    }

    /// Mark a task as completed (async).
    pub async fn complete_task(&mut self, task_id: TaskId) -> Result<(), OrchestratorError> {
        let agent_id = self
            .task_assignments
            .get(&task_id)
            .copied()
            .ok_or(OrchestratorError::TaskNotFound(task_id))?;

        let queue = self
            .agents
            .get_mut(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?;

        // Get the task's file manifest before completing
        let write_files: Vec<PathBuf> = queue
            .current_task()
            .map(|t| t.write_files().into_iter().cloned().collect())
            .unwrap_or_default();

        let mut auto_debug_requeue = None;

        #[cfg(feature = "toestub-gate")]
        {
            if self.config.toestub_gate {
                if let Some(mut task_clone) = queue.current_task().cloned() {
                    let vr = crate::validation::post_task_validate(&task_clone);
                    if !crate::validation::quality_gate(&vr)
                        && task_clone.debug_iterations < self.config.max_debug_iterations
                    {
                        task_clone.debug_iterations += 1;
                        task_clone.description.push_str(&format!("\n\n[AUTO-DEBUG ITERATION {}]\nValidation failed with diagnostic issues. Please fix the following:\n{}", task_clone.debug_iterations, vr.report));
                        task_clone.status = TaskStatus::Queued;
                        auto_debug_requeue = Some((task_clone, vr.report.clone()));
                    }
                }
            }
        }

        if let Some((requeue_task, err_report)) = auto_debug_requeue {
            // Log it
            tracing::warn!(
                "Task {} failed validation. Auto-debugging (iteration {}/{})",
                task_id,
                requeue_task.debug_iterations,
                self.config.max_debug_iterations
            );
            queue.mark_failed(
                task_id,
                format!("Auto-debug validation failure:\n{}", err_report),
            );

            // Requeue the modified task back to the *same* queue
            queue.enqueue(requeue_task);
            return Ok(());
        }

        let mut socrates_requeue: Option<AgentTask> = None;
        {
            let policy = self.config.effective_socrates_policy();
            if let Some(task) = queue.current_task() {
                if let Some(ref ctx) = task.socrates {
                    let outcome = crate::socrates::evaluate_socrates_gate(ctx, &policy);
                    if self.config.socrates_gate_shadow {
                        tracing::info!(
                            target: "vox_orchestrator::socrates",
                            task_id = task_id.0,
                            agent_id = agent_id.0,
                            decision = ?outcome.decision,
                            confidence = outcome.confidence,
                            contradiction = outcome.contradiction_ratio,
                            "socrates gate (shadow)"
                        );
                    }
                    if self.config.socrates_gate_enforce
                        && outcome.decision != vox_socrates_policy::RiskDecision::Answer
                        && task.debug_iterations < self.config.max_debug_iterations
                    {
                        let mut t = task.clone();
                        t.debug_iterations += 1;
                        t.description.push_str(&format!(
                            "\n\n[SOCRATES GATE]\nRisk decision {:?} (confidence {:.2}, contradiction {:.2}). Improve grounding (citations, evidence) or resolve contradictions before completing.\n",
                            outcome.decision, outcome.confidence, outcome.contradiction_ratio
                        ));
                        t.status = TaskStatus::Queued;
                        socrates_requeue = Some(t);
                    }
                }
            }
        }

        if let Some(requeue_task) = socrates_requeue {
            tracing::warn!(
                task_id = task_id.0,
                "Socrates gate blocked completion; requeueing"
            );
            queue.mark_failed(task_id, "Socrates risk gate blocked completion".to_string());
            queue.enqueue(requeue_task);
            return Ok(());
        }

        let desc = queue
            .current_task()
            .map(|t| t.description.clone())
            .unwrap_or_default();
        queue.mark_complete(task_id);

        // Capture post-task snapshot and record in oplog (persisted to CodeStore)
        let snap_desc = format!("post-task complete: {:.50}", desc);
        let snapshot_after = self
            .capture_snapshot(agent_id, &write_files, snap_desc.clone())
            .await;

        self.event_bus
            .emit(crate::events::AgentEventKind::SnapshotCaptured {
                agent_id,
                snapshot_id: snapshot_after.to_string(),
                file_count: write_files.len(),
                description: snap_desc,
            });

        // Find pre-task snapshots from the oplog to link this completion
        let (snap_before, db_snap_before) = self.oplog.find_task_snapshots(task_id.0);
        let db_snap_after = self
            .take_db_snapshot(agent_id, format!("post-task-complete: {}", task_id))
            .await;

        self.record_operation(
            agent_id,
            OperationKind::TaskComplete { task_id: task_id.0 },
            format!("Completed task {}", task_id),
            snap_before,
            Some(snapshot_after),
            db_snap_before,
            db_snap_after,
        )
        .await;

        // Auto-detect conflicts: check if any other agent's workspace overlaps
        let other_agents: Vec<AgentId> = self
            .agents
            .keys()
            .filter(|&&id| id != agent_id && self.workspace_manager.has_workspace(id))
            .copied()
            .collect();
        for other_id in other_agents {
            let overlaps = self.workspace_manager.overlapping_paths(agent_id, other_id);
            for overlap_path in overlaps {
                let conflict_id = self.conflict_manager.record_conflict(
                    overlap_path.clone(),
                    Some(snapshot_after),
                    vec![(agent_id, snapshot_after), (other_id, snapshot_after)],
                );
                self.event_bus
                    .emit(crate::events::AgentEventKind::ConflictDetected {
                        path: overlap_path,
                        agent_ids: vec![agent_id, other_id],
                        conflict_id: conflict_id.to_string(),
                    });
                tracing::warn!(
                    "Conflict {} detected between {} and {}",
                    conflict_id,
                    agent_id,
                    other_id
                );
            }
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        if let Some(steps) = self.task_traces.get_mut(&task_id) {
            steps.push(TaskTraceStep {
                stage: "outcome".to_string(),
                timestamp_ms: now_ms,
                detail: Some("completed".to_string()),
            });
        }

        // Release file locks for this task's write files
        for path in &write_files {
            self.lock_manager.release(path, agent_id);
        }

        MessageGateway::publish_task_completed(
            &mut self.bulletin,
            &mut self.message_bus,
            &self.event_bus,
            task_id,
            agent_id,
        );

        // Unblock dependent tasks across ALL agents
        for queue in self.agents.values_mut() {
            queue.unblock(task_id);
        }

        if let Some(db) = &self.db {
            let _ = db.store().block_on(db.store().record_task_reliability_observation(&agent_id.0.to_string(), true));
        }

        tracing::info!("Task {} completed by agent {}", task_id, agent_id);
        Ok(())
    }

    /// Mark a task as failed (async).
    pub async fn fail_task(
        &mut self,
        task_id: TaskId,
        reason: String,
    ) -> Result<(), OrchestratorError> {
        let agent_id = self
            .task_assignments
            .get(&task_id)
            .copied()
            .ok_or(OrchestratorError::TaskNotFound(task_id))?;

        let queue = self
            .agents
            .get_mut(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?;

        queue.mark_failed(task_id, reason.clone());

        if let Some(db) = &self.db {
            let _ = db.store().block_on(db.store().record_task_reliability_observation(&agent_id.0.to_string(), false));
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        if let Some(steps) = self.task_traces.get_mut(&task_id) {
            steps.push(TaskTraceStep {
                stage: "outcome".to_string(),
                timestamp_ms: now_ms,
                detail: Some(format!("failed: {}", reason)),
            });
        }

        // Release locks
        self.lock_manager.release_all(agent_id);

        // Find pre-task snapshots to link this failure
        let (snap_before, db_snap_before) = self.oplog.find_task_snapshots(task_id.0);

        // Record failure in oplog (async to support DB snapshot)
        self.record_operation(
            agent_id,
            OperationKind::TaskFail {
                task_id: task_id.0,
                reason: reason.clone(),
            },
            format!("Failed task {}", task_id),
            snap_before,
            None,
            db_snap_before,
            None,
        )
        .await;

        MessageGateway::publish_task_failed(
            &mut self.bulletin,
            &self.event_bus,
            task_id,
            agent_id,
            reason.clone(),
        );

        tracing::warn!("Task {} failed: {}", task_id, reason);
        Ok(())
    }

    /// Get a snapshot of the orchestrator's current state.
    pub fn status(&self) -> OrchestratorStatus {
        let agents: Vec<AgentSummary> = self
            .agents
            .iter()
            .map(|(id, queue)| AgentSummary {
                id: *id,
                name: queue.name.clone(),
                queued: queue.len(),
                urgent_count: queue.depth_by_priority(crate::types::TaskPriority::Urgent),
                normal_count: queue.depth_by_priority(crate::types::TaskPriority::Normal),
                background_count: queue.depth_by_priority(crate::types::TaskPriority::Background),
                in_progress: queue.has_in_progress(),
                completed: queue.completed_count(),
                paused: queue.is_paused(),
                owned_files: self.affinity_map.files_for_agent(*id).len(),
                dynamic: self.dynamic_agents.contains(id),
                weighted_load: queue.weighted_load(),
                agent_session_id: queue.agent_session_id.clone(),
            })
            .collect();

        let dynamic_count = self.dynamic_agents.len();
        let reserved_count = self.agents.len().saturating_sub(dynamic_count);

        #[allow(unused_mut)]
        let mut total_weighted_load: f64 = agents.iter().map(|a| a.weighted_load).sum();

        // Integrate system resources if configured
        #[cfg(feature = "system-metrics")]
        if self.config.resource_weight > 0.0 {
            let cpu_usage = self.sys.global_cpu_usage() as f64 / 100.0;
            let mem_usage = self.sys.used_memory() as f64 / self.sys.total_memory().max(1) as f64;
            let mut resource_factor = cpu_usage * self.config.resource_cpu_multiplier
                + mem_usage * self.config.resource_mem_multiplier;
            if self.config.resource_exponent != 1.0 {
                resource_factor = resource_factor.powf(self.config.resource_exponent);
            }
            total_weighted_load *= 1.0 + (resource_factor * self.config.resource_weight);
        }

        let predicted_load = if self.load_history.is_empty() {
            total_weighted_load
        } else {
            let avg: f64 =
                self.load_history.iter().copied().sum::<f64>() / self.load_history.len() as f64;
            if self.load_history.len() >= 2 {
                let last = *self.load_history.back().unwrap();
                let trend = last - avg;
                (last + trend).max(0.0)
            } else {
                avg
            }
        };

        OrchestratorStatus {
            enabled: self.config.enabled,
            agent_count: self.agents.len(),
            total_queued: agents.iter().map(|a| a.queued).sum(),
            total_in_progress: agents.iter().filter(|a| a.in_progress).count(),
            total_completed: agents.iter().map(|a| a.completed).sum(),
            locked_files: self.lock_manager.active_lock_count(),
            total_contention: self.lock_manager.contention_count(),
            total_weighted_load,
            predicted_load,
            reserved_agents: reserved_count,
            dynamic_agents: dynamic_count,
            context_entries: self.context_store.entries(),
            agents,
        }
    }

    /// Get a reference to an agent's queue.
    pub fn agent_queue(&self, agent_id: AgentId) -> Option<&AgentQueue> {
        self.agents.get(&agent_id)
    }

    /// Get a mutable reference to an agent's queue.
    pub fn get_agent_queue_mut(&mut self, agent_id: AgentId) -> Option<&mut AgentQueue> {
        self.agents.get_mut(&agent_id)
    }

    /// Get a reference to the budget manager.
    pub fn budget_manager(&self) -> &crate::budget::BudgetManager {
        &self.budget_manager
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &OrchestratorConfig {
        &self.config
    }

    /// Get a mutable reference to the configuration to allow run-time tuning.
    pub fn config_mut(&mut self) -> &mut OrchestratorConfig {
        &mut self.config
    }

    /// Get a reference to the bulletin board (for subscribing).
    pub fn bulletin(&self) -> &BulletinBoard {
        &self.bulletin
    }

    /// Get a mutable reference to the bulletin board (for publishing).
    pub fn bulletin_mut(&mut self) -> &mut BulletinBoard {
        &mut self.bulletin
    }

    /// Access the file affinity map.
    pub fn affinity_map(&self) -> &FileAffinityMap {
        &self.affinity_map
    }

    /// Access the QA Router.
    pub fn qa_router(&self) -> &crate::qa::QARouter {
        &self.qa_router
    }

    /// Access the file affinity map mutably.
    pub fn affinity_map_mut(&mut self) -> &mut FileAffinityMap {
        &mut self.affinity_map
    }

    /// Get a reference to the lock manager.
    pub fn lock_manager(&self) -> &FileLockManager {
        &self.lock_manager
    }

    /// List all agent IDs.
    pub fn agent_ids(&self) -> Vec<AgentId> {
        self.agents.keys().copied().collect()
    }

    /// Get a reference to task → agent assignment map.
    pub fn task_assignments(&self) -> &HashMap<TaskId, AgentId> {
        &self.task_assignments
    }

    /// Get the lifecycle timeline for a task (ingress → route → outcome), if recorded.
    pub fn task_trace(&self, task_id: TaskId) -> Option<&Vec<TaskTraceStep>> {
        self.task_traces.get(&task_id)
    }

    /// Get a reference to the shared context store.
    pub fn context(&self) -> &crate::context::ContextStore {
        &self.context_store
    }

    /// Get a reference to the budget manager.
    pub fn budget(&self) -> &crate::budget::BudgetManager {
        &self.budget_manager
    }

    /// Get a reference to the summary manager.
    pub fn summary(&self) -> &crate::summary::SummaryManager {
        &self.summary_manager
    }

    /// Access the model registry.
    pub fn models(&self) -> &crate::models::ModelRegistry {
        &self.models
    }

    /// Mutable access for updating model registry overrides at runtime.
    pub fn models_mut(&mut self) -> &mut crate::models::ModelRegistry {
        &mut self.models
    }

    /// Access the event bus
    pub fn event_bus(&self) -> &crate::events::EventBus {
        &self.event_bus
    }

    /// Access the A2A message bus
    pub fn message_bus(&self) -> &crate::a2a::MessageBus {
        &self.message_bus
    }

    /// Access the A2A message bus mutably (for ack, etc.)
    pub fn message_bus_mut(&mut self) -> &mut crate::a2a::MessageBus {
        &mut self.message_bus
    }

    // -- JJ-inspired subsystem accessors --

    /// Access the auto-snapshot store.
    pub fn snapshot_store(&self) -> &crate::snapshot::SnapshotStore {
        &self.snapshot_store
    }

    /// Access the auto-snapshot store mutably.
    pub fn snapshot_store_mut(&mut self) -> &mut crate::snapshot::SnapshotStore {
        &mut self.snapshot_store
    }

    /// Access the operation log.
    pub fn oplog(&self) -> &crate::oplog::OpLog {
        &self.oplog
    }

    /// Access the operation log mutably.
    pub fn oplog_mut(&mut self) -> &mut crate::oplog::OpLog {
        &mut self.oplog
    }

    /// Access the conflict manager.
    pub fn conflict_manager(&self) -> &crate::conflicts::ConflictManager {
        &self.conflict_manager
    }

    /// Access the conflict manager mutably.
    pub fn conflict_manager_mut(&mut self) -> &mut crate::conflicts::ConflictManager {
        &mut self.conflict_manager
    }

    /// Access the workspace manager.
    pub fn workspace_manager(&self) -> &crate::workspace::WorkspaceManager {
        &self.workspace_manager
    }

    /// Access the workspace manager mutably.
    pub fn workspace_manager_mut(&mut self) -> &mut crate::workspace::WorkspaceManager {
        &mut self.workspace_manager
    }

    /// Send a structured A2A message and publish to bulletin.
    pub fn send_a2a(
        &mut self,
        sender: AgentId,
        receiver: AgentId,
        msg_type: crate::types::A2AMessageType,
        payload: impl Into<String>,
    ) -> crate::types::MessageId {
        let payload_str = payload.into();

        // Native VCS integration: When an agent hands off a plan to another, automatically
        // start tracking a logical Change in the workspace manager for provenance visibility.
        if msg_type == crate::types::A2AMessageType::PlanHandoff {
            self.workspace_manager.create_change(
                receiver,
                format!("Plan handoff from {}: {:.100}", sender, payload_str),
            );
        }

        let msg_id = self
            .message_bus
            .send(sender, receiver, msg_type, payload_str);
        if let Some(msg) = self.message_bus.audit_trail().last() {
            self.bulletin
                .publish(crate::types::AgentMessage::A2A(msg.clone()));

            self.event_bus
                .emit(crate::events::AgentEventKind::MessageSent {
                    from: msg.sender,
                    to: msg.receiver,
                    summary: format!("{:?}: {}", msg.msg_type, msg.payload),
                });
        }
        msg_id
    }

    /// Broadcast a structured A2A message to all and publish to bulletin.
    pub fn broadcast_a2a(
        &mut self,
        sender: AgentId,
        msg_type: crate::types::A2AMessageType,
        payload: impl Into<String>,
    ) -> crate::types::MessageId {
        let msg_id = self.message_bus.broadcast(sender, msg_type, payload);
        if let Some(msg) = self.message_bus.audit_trail().last() {
            self.bulletin
                .publish(crate::types::AgentMessage::A2A(msg.clone()));

            self.event_bus
                .emit(crate::events::AgentEventKind::MessageSent {
                    from: msg.sender,
                    to: None, // Broadcast
                    summary: format!("{:?}: {}", msg.msg_type, msg.payload),
                });
        }
        msg_id
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
            .submit_task("Task 1", vec![FileAffinity::write("src/lib.rs")], None)
            .await
            .unwrap();
        let t2 = orch
            .submit_task("Task 2", vec![FileAffinity::write("src/lib.rs")], None)
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
        )
        .await
        .unwrap();
        orch.submit_task(
            "Codegen work",
            vec![FileAffinity::write("crates/vox-codegen-rust/src/lib.rs")],
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
            .submit_task("Test task", vec![FileAffinity::write("test.rs")], None)
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
        let err = orch.submit_task("test", vec![], None).await.unwrap_err();
        assert!(matches!(err, OrchestratorError::Disabled));
    }

    #[tokio::test]
    async fn status_snapshot() {
        let mut orch = test_orchestrator();
        orch.submit_task("t1", vec![FileAffinity::write("a.rs")], None)
            .await
            .unwrap();
        orch.submit_task("t2", vec![FileAffinity::write("b.rs")], None)
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
            .submit_task("Trace me", vec![FileAffinity::write("x.rs")], None)
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
            .submit_task("Complete me", vec![FileAffinity::write("y.rs")], None)
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
            .submit_task("Fail me", vec![FileAffinity::write("z.rs")], None)
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
