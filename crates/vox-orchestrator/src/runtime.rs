//! Tokio/`vox-runtime` bridge: actor agents, task processors, and fleet scaling hooks.
//!
//! [`AgentFleet`](crate::runtime::AgentFleet) keeps [`ProcessHandle`](vox_runtime::ProcessHandle) values aligned with [`Orchestrator`](crate::orchestrator::Orchestrator) registrations
//! and applies [`ScalingAction`](crate::services::ScalingAction) decisions from the scaling service.

use std::sync::Arc;

use vox_runtime::{
    ProcessHandle, RegistryError, mailbox::MessagePayload, process::ProcessContext,
    scheduler::Scheduler, supervisor::ChildSpec, supervisor::RestartStrategy,
    supervisor::Supervisor,
};

use crate::events::AgentEventKind;
use crate::models::{ModelRouteBackend, route_backend_for_model};
use crate::orchestrator::Orchestrator;
use crate::services::{ScalingAction, ScalingService};
use crate::types::AgentId;
use crate::types::TaskId;
use futures_util::StreamExt;
use std::time::{Duration, Instant};

/// Message type sent to the ActorAgent to trigger task processing.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum AgentCommand {
    /// Drain the agent's queue once (used by supervisor ticks).
    ProcessQueue,
    /// Pause dequeueing new tasks.
    Pause,
    /// Resume after [`AgentCommand::Pause`].
    Resume,
    /// Remove a specific pending task id from the local queue.
    CancelTask(TaskId),
}

/// Pluggable executor invoked by [`ActorAgent`] for each dequeued [`AgentTask`](crate::types::AgentTask).
#[async_trait::async_trait]
pub trait TaskProcessor: Send + Sync {
    /// Runs `task` on behalf of `agent_id` and returns the finished task id on success.
    async fn process(
        &self,
        agent_id: crate::types::AgentId,
        task: crate::types::AgentTask,
    ) -> anyhow::Result<crate::types::TaskId>;
}

/// A default stub processor that simulates a short work slice (~50ms) then completes the task.
pub struct StubTaskProcessor;

#[async_trait::async_trait]
impl TaskProcessor for StubTaskProcessor {
    async fn process(
        &self,
        _agent_id: crate::types::AgentId,
        task: crate::types::AgentTask,
    ) -> anyhow::Result<crate::types::TaskId> {
        // Small delay so scaling/retirement tests and metrics have a non-racy window.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        Ok(task.id)
    }
}

/// A real AI-powered task processor that streams tokens back to the event bus.
pub struct AiTaskProcessor {
    client: vox_ludus::ai::FreeAiClient,
    event_bus: crate::events::EventBus,
    orchestrator: Arc<Orchestrator>,
    /// Provider name stored at construction time (e.g. "ollama", "google").
    provider: String,
    /// Model identifier stored at construction time.
    model: String,
}

#[derive(Debug, Clone, Copy)]
enum ExecutorPhase {
    Inspect,
    Localize,
    Hypothesize,
    Act,
    Verify,
    Decide,
}

impl ExecutorPhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Inspect => "inspect",
            Self::Localize => "localize",
            Self::Hypothesize => "hypothesize",
            Self::Act => "act",
            Self::Verify => "verify",
            Self::Decide => "decide",
        }
    }
}

impl AiTaskProcessor {
    /// Create a new AI processor that auto-discovers providers.
    pub async fn new(event_bus: crate::events::EventBus, orchestrator: Arc<Orchestrator>) -> Self {
        let client = vox_ludus::ai::FreeAiClient::auto_discover().await;
        // Reflect the active provider in costs/logs
        let (provider, model) = client.active_provider_info();
        Self {
            client,
            event_bus,
            orchestrator,
            provider,
            model,
        }
    }

    async fn run_phase_stream(
        &self,
        agent_id: crate::types::AgentId,
        task: &crate::types::AgentTask,
        phase: ExecutorPhase,
        usage_model: &str,
        prior_notes: &str,
        route: vox_ludus::StreamRoute<'_>,
    ) -> String {
        let prompt = format!(
            "Task: {}\n\nPhase: {}\nCategory: {:?}\nRouting model hint: {}\n\nKnown notes:\n{}\n\nAction contract:\n- Think step-by-step for this phase only.\n- If proposing tool usage, emit one line starting with `@tool` and a concrete tool name.\n- Keep output concise and executable.",
            task.description,
            phase.as_str(),
            task.task_category,
            usage_model,
            prior_notes
        );

        let mut stream = self.client.generate_stream_routed(&prompt, route).await;
        let mut phase_text = String::new();
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(text) => {
                    phase_text.push_str(&text);
                    self.event_bus
                        .emit(AgentEventKind::TokenStreamed { agent_id, text });
                }
                Err(e) => tracing::error!("AI stream error [{}]: {}", phase.as_str(), e),
            }
        }
        phase_text
    }
}

#[async_trait::async_trait]
impl TaskProcessor for AiTaskProcessor {
    async fn process(
        &self,
        agent_id: crate::types::AgentId,
        task: crate::types::AgentTask,
    ) -> anyhow::Result<crate::types::TaskId> {
        let cost_pref = crate::sync_lock::rw_read(&*self.orchestrator.config).cost_preference;
        let routed = {
            let registry = crate::sync_lock::rw_read(&*self.orchestrator.models);
            registry.best_for(task.task_category, task.estimated_complexity, cost_pref)
        };
        let (usage_provider, usage_model) = if let Some(ref mo) = task.model_override {
            ("task_override".to_string(), mo.clone())
        } else if let Some(m) = routed.as_ref() {
            (m.provider.clone(), m.id.clone())
        } else {
            (self.provider.clone(), self.model.clone())
        };

        let route = if let Some(mo) = task
            .model_override
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            vox_ludus::StreamRoute::UserModelOverride(mo)
        } else if let Some(m) = routed.as_ref() {
            match route_backend_for_model(m) {
                ModelRouteBackend::Ollama => vox_ludus::StreamRoute::Registry {
                    backend: vox_ludus::LudusStreamBackend::Ollama,
                    model: m.id.as_str(),
                },
                ModelRouteBackend::GeminiDirect => vox_ludus::StreamRoute::Registry {
                    backend: vox_ludus::LudusStreamBackend::Gemini,
                    model: m.id.as_str(),
                },
                ModelRouteBackend::OpenRouter => vox_ludus::StreamRoute::Registry {
                    backend: vox_ludus::LudusStreamBackend::OpenRouter,
                    model: m.id.as_str(),
                },
                ModelRouteBackend::CascadeFallback => vox_ludus::StreamRoute::Cascade,
            }
        } else {
            vox_ludus::StreamRoute::Cascade
        };

        if let Some(db) = self.orchestrator.db() {
            let repo = crate::lineage::repository_id();
            let has_model_override = task
                .model_override
                .as_deref()
                .map(str::trim)
                .is_some_and(|s| !s.is_empty());
            let ludus_fallback = !has_model_override && routed.is_none();
            let reason = vox_runtime::routing_telemetry::OrchestratorTaskRoutingReasonV1::new(
                format!("{:?}", task.task_category),
                task.estimated_complexity,
                usage_provider.clone(),
                usage_model.clone(),
                routed.is_some(),
                format!("{:?}", cost_pref),
                ludus_fallback,
                vox_runtime::routing_telemetry::unified_routing_rollout_enabled(),
                task.id.0,
            );
            let reason_s =
                reason.to_json_bounded(vox_runtime::routing_telemetry::ROUTING_REASON_JSON_MAX_BYTES);
            if let Err(e) = db
                .record_routing_decision(
                    None::<&str>,
                    repo.as_str(),
                    task.session_id.as_deref(),
                    "orchestrator_ai_task",
                    Some(usage_model.as_str()),
                    Some(reason_s.as_str()),
                )
                .await
            {
                tracing::debug!(error = %e, "record_routing_decision (orchestrator_ai_task) skipped");
            }
        }

        let mut notes = String::new();
        let phases = [
            ExecutorPhase::Inspect,
            ExecutorPhase::Localize,
            ExecutorPhase::Hypothesize,
            ExecutorPhase::Act,
            ExecutorPhase::Verify,
            ExecutorPhase::Decide,
        ];
        // Keep execution bounded: no infinite self-reflection or uncontrolled loops.
        for phase in phases {
            let phase_out = self
                .run_phase_stream(
                    agent_id,
                    &task,
                    phase,
                    usage_model.as_str(),
                    notes.as_str(),
                    route,
                )
                .await;
            if !notes.is_empty() {
                notes.push_str("\n\n");
            }
            notes.push_str(&format!("[{}]\n{}", phase.as_str(), phase_out));
            // Lightweight tool intent tracing: explicit breadcrumbs for future bridge adapters.
            if let Some(tool_line) = phase_out
                .lines()
                .map(str::trim)
                .find(|line| line.starts_with("@tool "))
            {
                tracing::info!(
                    agent_id = agent_id.0,
                    task_id = task.id.0,
                    phase = phase.as_str(),
                    tool_intent = %tool_line,
                    "bounded executor emitted tool intent"
                );
            }
        }
        let full_text = notes;

        // Estimate token counts (4 chars ≈ 1 token as a rough heuristic)
        let input_tokens = (task.description.len() / 4).max(1) as u32;
        let output_tokens = (full_text.len() / 4).max(1) as u32;
        // Approximate cost: $0.000001 per token (conservative free-tier estimate)
        let cost_usd = (input_tokens + output_tokens) as f64 * 0.000_001;

        // Record usage through the unified pipeline (event bus + budget + oplog)
        self.orchestrator
            .record_ai_usage(
                agent_id,
                usage_provider.as_str(),
                usage_model.as_str(),
                input_tokens,
                output_tokens,
                cost_usd,
            )
            .await;

        Ok(task.id)
    }
}

/// Actor process wrapping an `AgentQueue`.
///
/// Converts a reactive orchestrator queue into an active background worker
/// using `vox-runtime` actor primitives.
pub struct ActorAgent {
    /// Agent id managed by this process.
    pub agent_id: AgentId,
    /// Human-readable process/agent name.
    pub name: String,
}

impl ActorAgent {
    /// Spawn an active agent process from an `AgentQueue`.
    pub fn spawn(
        scheduler: &Scheduler,
        agent_id: AgentId,
        name: String,
        orchestrator: Arc<Orchestrator>,
        processor: Arc<dyn TaskProcessor>,
    ) -> Result<ProcessHandle, RegistryError> {
        let process_name = format!("agent-{}", name);

        scheduler.spawn_named(&process_name, move |mut ctx: ProcessContext| async move {
            tracing::info!("Agent {} ({}) process started", agent_id, name);

            loop {
                // Wait for commands
                let msg = ctx.receive().await;
                if let Some(envelope) = msg {
                    if let vox_runtime::mailbox::Envelope::Message(msg) = envelope {
                        if let MessagePayload::Json(json_data) = msg.payload {
                            if let Ok(cmd) = serde_json::from_str::<AgentCommand>(&json_data) {
                                Self::handle_command(cmd, agent_id, &orchestrator, &processor)
                                    .await;
                            }
                        }
                    }
                } else {
                    // Channel closed
                    break;
                }
            }
            tracing::info!("Agent {} ({}) process shutting down", agent_id, name);
        })
    }

    /// Handle a command sent to this agent process.
    async fn handle_command(
        cmd: AgentCommand,
        agent_id: AgentId,
        orchestrator_ref: &Arc<Orchestrator>,
        processor: &Arc<dyn TaskProcessor>,
    ) {
        match cmd {
            AgentCommand::ProcessQueue => {
                let task_to_run = {
                    let dequeued = if let Some(queue_lock) = orchestrator_ref.agent_queue(agent_id)
                    {
                        let mut queue = crate::sync_lock::rw_write(&queue_lock);
                        if !queue.is_paused() {
                            let t = queue.dequeue();
                            if t.is_some() {
                                orchestrator_ref
                                    .heartbeat(agent_id, crate::events::AgentActivity::Thinking);
                            } else {
                                orchestrator_ref
                                    .heartbeat(agent_id, crate::events::AgentActivity::Idle);
                            }
                            t
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    if let Some(ref task) = dequeued {
                        orchestrator_ref
                            .event_bus()
                            .emit(AgentEventKind::TaskStarted {
                                task_id: task.id,
                                agent_id,
                                session_id: task.session_id.clone(),
                            });
                    }
                    dequeued
                };

                if let Some(task) = task_to_run {
                    let task_id = task.id;
                    tracing::info!("Agent {} processing task {}", agent_id, task_id);

                    match processor.process(agent_id, task).await {
                        Ok(completed_id) => {
                            let _ = orchestrator_ref.complete_task(completed_id).await;
                            orchestrator_ref
                                .heartbeat(agent_id, crate::events::AgentActivity::Idle);
                        }
                        Err(e) => {
                            tracing::error!("Agent {} failed task {}: {}", agent_id, task_id, e);
                            if let Err(err) =
                                orchestrator_ref.fail_task(task_id, e.to_string()).await
                            {
                                tracing::error!(
                                    "fail_task after processor error: {} (task {})",
                                    err,
                                    task_id
                                );
                            }
                            orchestrator_ref
                                .heartbeat(agent_id, crate::events::AgentActivity::Idle);
                        }
                    }
                }
            }
            AgentCommand::Pause => {
                orchestrator_ref.heartbeat(agent_id, crate::events::AgentActivity::Idle);
                let _ = orchestrator_ref.pause_agent(agent_id);
            }
            AgentCommand::Resume => {
                orchestrator_ref.heartbeat(agent_id, crate::events::AgentActivity::Idle);
                let _ = orchestrator_ref.resume_agent(agent_id);
            }
            AgentCommand::CancelTask(task_id) => {
                orchestrator_ref.heartbeat(agent_id, crate::events::AgentActivity::Idle);
                if let Some(q_lock) = orchestrator_ref.agent_queue(agent_id) {
                    crate::sync_lock::rw_write(&q_lock).cancel(task_id);
                }
            }
        }
    }
}

/// A fleet supervisor that manages multiple agent processes.
pub struct AgentFleet {
    supervisor: Supervisor,
    scheduler: Arc<Scheduler>,
    orchestrator: Arc<Orchestrator>,
    processor: Arc<dyn TaskProcessor>,
    /// Last time we performed a scale-up (for cooldown).
    #[allow(dead_code)]
    last_scale_up: std::sync::RwLock<Option<Instant>>,
    /// Number of agents spawned in the current tick (reset each check_scaling).
    #[allow(dead_code)]
    spawns_this_tick: std::sync::atomic::AtomicUsize,
}

impl AgentFleet {
    /// Wires the shared scheduler and shared [`Arc<Orchestrator>`] with a task processor implementation.
    pub fn new(
        scheduler: Arc<Scheduler>,
        orchestrator: Arc<Orchestrator>,
        processor: Arc<dyn TaskProcessor>,
    ) -> Self {
        Self {
            supervisor: Supervisor::new(RestartStrategy::RestForOne),
            scheduler,
            orchestrator,
            processor,
            last_scale_up: std::sync::RwLock::new(None),
            spawns_this_tick: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Watch the orchestrator state and ensure an actor exists for every
    /// agent registered in the orchestrator. Also stops processes for retired agents.
    pub async fn sync_fleet(&self) {
        let agent_info: Vec<(AgentId, String)> = {
            let ids = self.orchestrator.agent_ids();
            ids.iter()
                .map(|id| {
                    (
                        *id,
                        crate::sync_lock::rw_read(
                            &*self.orchestrator.agent_queue(*id).expect("agent queue"),
                        )
                        .name
                        .clone(),
                    )
                })
                .collect()
        };
        let active_agent_ids: std::collections::HashSet<AgentId> =
            agent_info.iter().map(|(id, _)| *id).collect();

        // 1. Ensure all active agents have actors
        for (agent_id, name) in agent_info {
            let proc_name = format!("agent-{}", name);

            // Check if process is already running in the global registry
            let already_running = match self.scheduler.registry().lookup_name(&proc_name) {
                Ok(opt) => opt.is_some(),
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        proc_name = %proc_name,
                        "process registry poisoned during fleet sync; aborting sync_fleet"
                    );
                    return;
                }
            };
            if !already_running {
                // Not running, add it to supervisor
                let orchestrator_clone = self.orchestrator.clone();
                let scheduler_clone = self.scheduler.clone();
                let processor_clone = self.processor.clone();

                let spec = ChildSpec {
                    name: proc_name.clone(),
                    start: Box::new(move || {
                        let h = ActorAgent::spawn(
                            &scheduler_clone,
                            agent_id,
                            name.clone(),
                            orchestrator_clone.clone(),
                            processor_clone.clone(),
                        )?;
                        orchestrator_clone.register_agent_handle(agent_id, h.clone());
                        Ok(h)
                    }),
                };

                self.supervisor.add_child(spec).await;
            }
        }

        // 2. Prune stale handles for retired agents so runtime state converges.
        let mut handles = crate::sync_lock::rw_write(&*self.orchestrator.agent_handles);
        let stale_ids: Vec<AgentId> = handles
            .keys()
            .copied()
            .filter(|id| !active_agent_ids.contains(id))
            .collect();
        for id in stale_ids {
            handles.remove(&id);
            tracing::debug!("Removed stale runtime handle for retired agent {}", id);
        }
        drop(handles);
    }

    /// Check if agents need to be spawned or retired using ScalingService and profile limits.
    pub async fn check_scaling(&self) {
        let (status, idle_dynamic, config, budget_manager, remote_gpu_capacity) = {
            let orch = &*self.orchestrator;
            let config_arc = orch.config_handle();
            let config = crate::sync_lock::rw_read(&config_arc).clone();
            if !config.scaling_enabled {
                return;
            }
            let status = orch.status();
            let idle_dynamic: Vec<_> = status
                .agents
                .iter()
                .filter(|a| a.dynamic && a.queued == 0 && !a.in_progress)
                .filter_map(|a| {
                    orch.agent_queue(a.id)
                        .map(|q| (a.id, crate::sync_lock::rw_read(&*q).last_active))
                })
                .collect();
            let budget_manager = orch.budget_manager_handle();
            let remote_gpu_capacity = crate::sync_lock::rw_read(&*orch.remote_populi_routing_hints)
                .iter()
                .filter(|h| {
                    h.capabilities.gpu_cuda
                        || h.capabilities.gpu_metal
                        || h.capabilities.gpu_vulkan
                        || h.capabilities.gpu_webgpu
                        || h.capabilities.npu
                })
                .count();
            (
                status,
                idle_dynamic,
                config,
                budget_manager,
                remote_gpu_capacity,
            )
        };

        let load_history: Vec<f64> = Vec::new();
        let action = ScalingService::decide_scaling(
            &status,
            &config,
            &load_history,
            remote_gpu_capacity,
            &idle_dynamic,
            &crate::sync_lock::rw_read(&budget_manager),
        );

        match action {
            ScalingAction::NoOp => {}
            ScalingAction::ScaleUp { name } => {
                let max_per_tick = config.max_spawn_per_tick;
                let cooldown_ms = config.scaling_cooldown_ms;
                let spawns = self
                    .spawns_this_tick
                    .load(std::sync::atomic::Ordering::Relaxed);
                let cooldown_ok = crate::sync_lock::rw_read(&self.last_scale_up)
                    .as_ref()
                    .map(|t| t.elapsed() >= Duration::from_millis(cooldown_ms))
                    .unwrap_or(true);
                if spawns < max_per_tick && cooldown_ok {
                    let _ = self.orchestrator.spawn_dynamic_agent_with_parent(
                        &name,
                        None,
                        Some("scaling_load"),
                        None,
                    );
                    self.spawns_this_tick
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    *crate::sync_lock::rw_write(&self.last_scale_up) = Some(Instant::now());
                    tracing::info!(
                        "Scaling up: spawning '{}' (load: {:.2}, profile: {:?})",
                        name,
                        status.total_weighted_load,
                        config.scaling_profile
                    );
                }
            }
            ScalingAction::ScaleDown { agent_ids } => {
                if !agent_ids.is_empty() {
                    tracing::info!(
                        "Scaling down: retiring {} idle dynamic agent(s)",
                        agent_ids.len()
                    );
                }
                for id in agent_ids {
                    let _ = self.orchestrator.retire_agent(id);
                }
            }
        }

        self.spawns_this_tick
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }

    /// Start the main orchestrator loop: rebalancing, maintenance, and fleet syncing.
    pub async fn run(&self) {
        loop {
            // 1. Scaling checks
            self.check_scaling().await;

            // 2. Sync fleet (ensure all agents have actors)
            self.sync_fleet().await;

            // 3. Perform orchestrator maintenance (rebalance and tick)
            {
                self.orchestrator.rebalance();
                self.orchestrator.tick().await;
            }

            // 4. Wait until next tick
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}

/// When truthy (default if unset), MCP / `vox-orchestrator-d` spawn [`AgentFleet`] with [`StubTaskProcessor`].
///
/// Disable with **`VOX_MCP_AGENT_FLEET`**=`0`, `false`, `no`, or `off`.
#[must_use]
pub fn agent_fleet_env_enabled() -> bool {
    match std::env::var("VOX_MCP_AGENT_FLEET") {
        Ok(v) => {
            let v = v.trim();
            if v.is_empty() {
                return true;
            }
            !(v == "0"
                || v.eq_ignore_ascii_case("false")
                || v.eq_ignore_ascii_case("no")
                || v.eq_ignore_ascii_case("off"))
        }
        Err(_) => true,
    }
}

/// Background [`AgentFleet`] loop (sync fleet + tick + scaling). No-op when [`agent_fleet_env_enabled`] is false.
pub fn spawn_stub_agent_fleet_if_enabled(orchestrator: Arc<Orchestrator>) {
    if !agent_fleet_env_enabled() {
        tracing::info!(
            target: "vox_orchestrator::runtime",
            "VOX_MCP_AGENT_FLEET disabled: task queues will not auto-drain via AgentFleet"
        );
        return;
    }
    let scheduler = Arc::new(Scheduler::new());
    let fleet = AgentFleet::new(
        scheduler,
        orchestrator,
        Arc::new(StubTaskProcessor),
    );
    tokio::spawn(async move {
        tracing::info!(
            target: "vox_orchestrator::runtime",
            "AgentFleet loop running (stub processor; MCP / orchestrator-d)"
        );
        fleet.run().await;
    });
}

#[cfg(test)]
mod stub_processor_tests {
    use super::{StubTaskProcessor, TaskProcessor};
    use crate::types::{AgentId, AgentTask, TaskId, TaskPriority};

    #[tokio::test]
    async fn stub_task_processor_returns_same_task_id() {
        let p = StubTaskProcessor;
        let task = AgentTask::new(TaskId(42), "test", TaskPriority::Normal, vec![]);
        let out = p.process(AgentId(1), task.clone()).await.expect("ok");
        assert_eq!(out, task.id);
    }
}
