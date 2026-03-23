//! Load-based agent scaling and periodic orchestrator maintenance.
//!
//! - [`Orchestrator::rebalance`] — work-stealing across imbalanced agent queues.
//! - [`Orchestrator::tick`] — one maintenance cycle: stale lock release, load
//!   history, zombie detection, auto-continuation, and urgent-queue rebalance.

use crate::types::AgentId;

impl crate::orchestrator::Orchestrator {
    /// Rebalance tasks across agents using work-stealing.
    ///
    /// Moves tasks from overloaded agents to underloaded ones, respecting file
    /// affinity (only moves tasks whose write-files are not locked by a third agent).
    pub fn rebalance(&mut self) -> usize {
        let loads: Vec<(AgentId, f64)> = self
            .agents
            .iter()
            .map(|(id, q)| (*id, q.weighted_load()))
            .collect();

        if loads.len() < 2 {
            return 0;
        }

        let total_load: f64 = loads.iter().map(|(_, l)| l).sum();
        let avg = total_load / loads.len() as f64;
        let mut moved = 0;

        let overloaded: Vec<AgentId> = loads
            .iter()
            .filter(|(_, l)| *l > avg + 2.0)
            .map(|(id, _)| *id)
            .collect();
        let mut underloaded: Vec<AgentId> = loads
            .iter()
            .filter(|(_, l)| *l < avg)
            .map(|(id, _)| *id)
            .collect();

        if self.config.cost_preference == crate::config::CostPreference::Economy {
            let models = &self.models;
            underloaded.sort_by(|a, b| {
                let cost_a = models
                    .get_override(a.0)
                    .and_then(|id| models.get(&id))
                    .map(|m| m.cost_per_1k)
                    .unwrap_or(0.003);
                let cost_b = models
                    .get_override(b.0)
                    .and_then(|id| models.get(&id))
                    .map(|m| m.cost_per_1k)
                    .unwrap_or(0.003);
                cost_a
                    .partial_cmp(&cost_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        for over_id in &overloaded {
            for under_id in &underloaded {
                if let Some(queue) = self.agents.get_mut(over_id) {
                    let mut tasks = queue.drain_tasks();
                    tasks.sort_by_key(|t| match t.priority {
                        crate::types::TaskPriority::Background => 0u8,
                        crate::types::TaskPriority::Normal => 1,
                        crate::types::TaskPriority::Urgent => 2,
                    });
                    let steal_idx = tasks.iter().position(|t| {
                        t.write_files().iter().all(|path| {
                            match self.lock_manager.holder(path.as_path()) {
                                None => true,
                                Some((holder, _)) => holder == *over_id,
                            }
                        })
                    });
                    let stolen = steal_idx.map(|i| tasks.remove(i));
                    if let Some(queue) = self.agents.get_mut(over_id) {
                        for task in tasks {
                            queue.enqueue(task);
                        }
                    }
                    if let Some(task) = stolen {
                        if let Some(target) = self.agents.get_mut(under_id) {
                            self.task_assignments.insert(task.id, *under_id);
                            target.enqueue(task);
                            moved += 1;
                        }
                    }
                }
            }
        }

        if moved > 0 {
            tracing::info!("Rebalanced: moved {} tasks", moved);
            self.last_rebalance_at = Some(std::time::Instant::now());
            self.oplog.record(
                AgentId(0),
                crate::oplog::OperationKind::Rebalance,
                format!("Rebalanced {} tasks", moved),
                None,
                None,
                None,
                None,
                None,
                None,
            );
        }
        moved
    }

    /// Run one periodic maintenance cycle.
    ///
    /// - Refreshes system metrics (when `system-metrics` feature is on).
    /// - Force-releases stale locks older than `config.lock_timeout_ms`.
    /// - Records current weighted load for predictive scaling history.
    /// - Checks heartbeats and retires zombie dynamic agents.
    /// - Issues auto-continuation tasks for idle agents.
    /// - Triggers urgent-queue rebalance when a single agent exceeds the threshold.
    pub async fn tick(&mut self) {
        #[cfg(feature = "system-metrics")]
        {
            self.sys.refresh_cpu_all();
            self.sys.refresh_memory();
        }

        let timeout = self.config.lock_timeout_ms as u128;
        let released = self.lock_manager.force_release_stale(timeout);
        if released > 0 {
            tracing::warn!(
                "Tick: forcefully released {} stale orphaned lock(s) older than {}ms",
                released,
                timeout
            );
        }

        let current_load = self.status().total_weighted_load;
        self.load_history.push_back(current_load);
        if self.load_history.len() > self.config.scaling_lookback_ticks {
            self.load_history.pop_front();
        }

        let stale_ids = self.heartbeat_monitor.check_stale(&self.event_bus);
        for (id, level) in stale_ids {
            if self.dynamic_agents.contains(&id) {
                tracing::warn!(
                    "Tick: retiring zombie dynamic agent {} (level: {})",
                    id,
                    level
                );
                let _ = self.retire_agent(id);
            } else {
                tracing::error!(
                    "Tick: reserved agent {} is unresponsive at level {}! Immediate attention required.",
                    id,
                    level
                );
            }
        }

        if self.config.auto_continue_enabled {
            let active_agents: Vec<(AgentId, usize)> = self
                .agents
                .iter()
                .map(|(id, queue)| (*id, queue.len()))
                .collect();
            let intents = self
                .monitor
                .check_idle_agents(&active_agents, &self.event_bus);

            for (agent_id, prompt) in intents {
                let _ = self
                    .submit_task_with_agent(
                        format!("[Auto-Continuation] {}", prompt),
                        vec![],
                        Some(crate::types::TaskPriority::Background),
                        Some(
                            self.agents
                                .get(&agent_id)
                                .map(|q| q.name.clone())
                                .unwrap_or_default(),
                        ),
                        None,
                    )
                    .await;
            }
        }

        let urgent_threshold = self.config.urgent_rebalance_threshold;
        if urgent_threshold > 0 && self.agents.len() >= 2 {
            let cooldown_ms = self.config.scaling_cooldown_ms;
            let can_rebalance = self
                .last_rebalance_at
                .map(|t| t.elapsed().as_millis() >= cooldown_ms as u128)
                .unwrap_or(true);

            if can_rebalance {
                let overloaded_urgent: Vec<(AgentId, usize)> = self
                    .agents
                    .iter()
                    .map(|(id, q)| {
                        (*id, q.depth_by_priority(crate::types::TaskPriority::Urgent))
                    })
                    .filter(|(_, depth)| *depth > urgent_threshold)
                    .collect();

                if !overloaded_urgent.is_empty() {
                    for (agent_id, depth) in &overloaded_urgent {
                        tracing::warn!(
                            "Tick: agent {} has {} urgent tasks (threshold {}), triggering urgent rebalance",
                            agent_id,
                            depth,
                            urgent_threshold
                        );
                    }
                    let moved = self.rebalance();
                    if moved > 0 {
                        self.event_bus.emit(
                            crate::events::AgentEventKind::UrgentRebalanceTriggered { moved },
                        );
                    }
                }
            }
        }
    }
}
