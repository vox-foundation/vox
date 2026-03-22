//! Tokio/`vox-runtime` bridge: actor agents, task processors, and fleet scaling hooks.
//!
//! [`AgentFleet`](crate::runtime::AgentFleet) keeps [`ProcessHandle`](vox_runtime::ProcessHandle) values aligned with [`Orchestrator`](crate::orchestrator::Orchestrator) registrations
//! and applies [`ScalingAction`](crate::services::ScalingAction) decisions from the scaling service.

use std::sync::Arc;
use tokio::sync::Mutex;

use vox_runtime::{
    ProcessHandle, mailbox::MessagePayload, process::ProcessContext, scheduler::Scheduler,
    supervisor::ChildSpec, supervisor::RestartStrategy, supervisor::Supervisor,
};

use crate::events::AgentEventKind;
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
    client: vox_gamify::ai::FreeAiClient,
    event_bus: crate::events::EventBus,
    orchestrator: Arc<Mutex<Orchestrator>>,
    /// Provider name stored at construction time (e.g. "ollama", "google").
    provider: String,
    /// Model identifier stored at construction time.
    model: String,
}

impl AiTaskProcessor {
    /// Create a new AI processor that auto-discovers providers.
    pub async fn new(
        event_bus: crate::events::EventBus,
        orchestrator: Arc<Mutex<Orchestrator>>,
    ) -> Self {
        let client = vox_gamify::ai::FreeAiClient::auto_discover().await;
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
}

#[async_trait::async_trait]
impl TaskProcessor for AiTaskProcessor {
    async fn process(
        &self,
        agent_id: crate::types::AgentId,
        task: crate::types::AgentTask,
    ) -> anyhow::Result<crate::types::TaskId> {
        let prompt = format!(
            "Task: {}\n\nContext: {:?}\n\nAction: Execute this task and provide the output.",
            task.description, task.task_category
        );

        let mut stream = self.client.generate_stream(&prompt).await;
        let mut full_text = String::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(text) => {
                    full_text.push_str(&text);
                    // Emit token stream event
                    self.event_bus
                        .emit(AgentEventKind::TokenStreamed { agent_id, text });
                }
                Err(e) => tracing::error!("AI stream error: {}", e),
            }
        }

        // Estimate token counts (4 chars ≈ 1 token as a rough heuristic)
        let input_tokens = (prompt.len() / 4).max(1) as u32;
        let output_tokens = (full_text.len() / 4).max(1) as u32;
        // Approximate cost: $0.000001 per token (conservative free-tier estimate)
        let cost_usd = (input_tokens + output_tokens) as f64 * 0.000_001;

        // Record usage through the unified pipeline (event bus + budget + oplog)
        if let Ok(mut orch) = self.orchestrator.try_lock() {
            orch.record_ai_usage(
                agent_id,
                &self.provider,
                &self.model,
                input_tokens,
                output_tokens,
                cost_usd,
            );
        }

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
        orchestrator: Arc<Mutex<Orchestrator>>,
        processor: Arc<dyn TaskProcessor>,
    ) -> ProcessHandle {
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
        orchestrator_ref: &Arc<Mutex<Orchestrator>>,
        processor: &Arc<dyn TaskProcessor>,
    ) {
        match cmd {
            AgentCommand::ProcessQueue => {
                let task_to_run = {
                    let mut orch = orchestrator_ref.lock().await;
                    let task_to_run = if let Some(queue) = orch.get_agent_queue_mut(agent_id) {
                        if !queue.is_paused() {
                            queue.dequeue()
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some(ref task) = task_to_run {
                        orch.heartbeat(agent_id, crate::events::AgentActivity::Thinking);
                        orch.event_bus().emit(AgentEventKind::TaskStarted {
                            task_id: task.id,
                            agent_id,
                        });
                    } else {
                        orch.heartbeat(agent_id, crate::events::AgentActivity::Idle);
                    }
                    task_to_run
                };

                if let Some(task) = task_to_run {
                    let task_id = task.id;
                    tracing::info!("Agent {} processing task {}", agent_id, task_id);

                    match processor.process(agent_id, task).await {
                        Ok(completed_id) => {
                            let mut o2 = orchestrator_ref.lock().await;
                            let _ = o2.complete_task(completed_id).await;
                            o2.heartbeat(agent_id, crate::events::AgentActivity::Idle);
                        }
                        Err(e) => {
                            tracing::error!("Agent {} failed task {}: {}", agent_id, task_id, e);
                            let mut o2 = orchestrator_ref.lock().await;
                            if let Err(err) = o2.fail_task(task_id, e.to_string()).await {
                                tracing::error!(
                                    "fail_task after processor error: {} (task {})",
                                    err,
                                    task_id
                                );
                            }
                            o2.heartbeat(agent_id, crate::events::AgentActivity::Idle);
                        }
                    }
                }
            }
            AgentCommand::Pause => {
                let mut orch = orchestrator_ref.lock().await;
                orch.heartbeat(agent_id, crate::events::AgentActivity::Idle);
                let _ = orch.pause_agent(agent_id);
            }
            AgentCommand::Resume => {
                let mut orch = orchestrator_ref.lock().await;
                orch.heartbeat(agent_id, crate::events::AgentActivity::Idle);
                let _ = orch.resume_agent(agent_id);
            }
            AgentCommand::CancelTask(task_id) => {
                let mut orch = orchestrator_ref.lock().await;
                orch.heartbeat(agent_id, crate::events::AgentActivity::Idle);
                if let Some(q) = orch.get_agent_queue_mut(agent_id) {
                    q.cancel(task_id);
                }
            }
        }
    }
}

/// A fleet supervisor that manages multiple agent processes.
pub struct AgentFleet {
    supervisor: Supervisor,
    scheduler: Arc<Scheduler>,
    orchestrator: Arc<Mutex<Orchestrator>>,
    processor: Arc<dyn TaskProcessor>,
    /// Last time we performed a scale-up (for cooldown).
    #[allow(dead_code)]
    last_scale_up: std::sync::RwLock<Option<Instant>>,
    /// Number of agents spawned in the current tick (reset each check_scaling).
    #[allow(dead_code)]
    spawns_this_tick: std::sync::atomic::AtomicUsize,
}

impl AgentFleet {
    /// Wires the shared scheduler and orchestrator mutex with a task processor implementation.
    pub fn new(
        scheduler: Arc<Scheduler>,
        orchestrator: Arc<Mutex<Orchestrator>>,
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
            let orch: tokio::sync::MutexGuard<'_, Orchestrator> = self.orchestrator.lock().await;
            let ids = orch.agent_ids();
            ids.iter()
                .map(|id| (*id, orch.agent_queue(*id).unwrap().name.clone()))
                .collect()
        };

        // 1. Ensure all active agents have actors
        for (agent_id, name) in agent_info {
            let proc_name = format!("agent-{}", name);

            // Check if process is already running in the global registry
            if self.scheduler.registry().lookup_name(&proc_name).is_none() {
                // Not running, add it to supervisor
                let orchestrator_clone = self.orchestrator.clone();
                let scheduler_clone = self.scheduler.clone();
                let processor_clone = self.processor.clone();

                let spec = ChildSpec {
                    name: proc_name.clone(),
                    start: Box::new(move || {
                        ActorAgent::spawn(
                            &scheduler_clone,
                            agent_id,
                            name.clone(),
                            orchestrator_clone.clone(),
                            processor_clone.clone(),
                        )
                    }),
                };

                self.supervisor.add_child(spec).await;
            }
        }

        // 2. Remove actors for agents that are no longer active
        // This is a bit tricky with the current Supervisor as it doesn't expose a list of children
        // easily for selective termination without names.
        // However, we can use the scheduler registry to find agent processes and see if they belong
        // to our active IDs.
        // For now, sync_fleet mostly focuses on starting. The ActorAgent loop will also terminate
        // if the channel closes or if we implement a "Kill" command.
    }

    /// Check if agents need to be spawned or retired using ScalingService and profile limits.
    pub async fn check_scaling(&self) {
        let (status, idle_dynamic, config, budget_manager) = {
            let orch = self.orchestrator.lock().await;
            if !orch.config().scaling_enabled {
                return;
            }
            let status = orch.status();
            let idle_dynamic: Vec<_> = status
                .agents
                .iter()
                .filter(|a| a.dynamic && a.queued == 0 && !a.in_progress)
                .filter_map(|a| orch.agent_queue(a.id).map(|q| (a.id, q.last_active)))
                .collect();
            let config = orch.config().clone();
            let budget_manager = orch.budget_manager().clone();
            (status, idle_dynamic, config, budget_manager)
        };

        let load_history: Vec<f64> = Vec::new();
        let action = ScalingService::decide_scaling(
            &status,
            &config,
            &load_history,
            &idle_dynamic,
            &budget_manager,
        );

        let mut orch = self.orchestrator.lock().await;
        match action {
            ScalingAction::NoOp => {}
            ScalingAction::ScaleUp { name } => {
                let max_per_tick = config.max_spawn_per_tick;
                let cooldown_ms = config.scaling_cooldown_ms;
                let spawns = self
                    .spawns_this_tick
                    .load(std::sync::atomic::Ordering::Relaxed);
                let cooldown_ok = self
                    .last_scale_up
                    .read()
                    .unwrap()
                    .as_ref()
                    .map(|t| t.elapsed() >= Duration::from_millis(cooldown_ms))
                    .unwrap_or(true);
                if spawns < max_per_tick && cooldown_ok {
                    let _ = orch.spawn_dynamic_agent(&name);
                    self.spawns_this_tick
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    *self.last_scale_up.write().unwrap() = Some(Instant::now());
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
                    let _ = orch.retire_agent(id);
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
                let mut orch = self.orchestrator.lock().await;
                orch.rebalance();
                orch.tick().await;
            }

            // 4. Wait until next tick
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
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
