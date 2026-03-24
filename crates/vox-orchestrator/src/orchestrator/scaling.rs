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
    pub fn rebalance(&self) -> usize {
        let loads: Vec<(AgentId, f64)> = self
            .agents
            .iter()
            .map(|pair| (*pair.key(), pair.value().weighted_load()))
            .collect();

        if loads.len() < 2 {
            return 0;
        }

        let config = self.config.read();
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

        if config.cost_preference == crate::config::CostPreference::Economy {
            let models = self.models();
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
                if let Some(mut over_queue) = self.agents.get_mut(over_id) {
                    let mut tasks = over_queue.drain_tasks();
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
                    
                    if let Some(stolen_task) = steal_idx.map(|i| tasks.remove(i)) {
                        if let Some(mut under_queue) = self.agents.get_mut(under_id) {
                            self.task_assignments.insert(stolen_task.id, *under_id);
                            under_queue.enqueue(stolen_task);
                            moved += 1;
                        } else {
                            // Put it back if we can't find the underloaded agent
                            tasks.push(stolen_task);
                        }
                    }

                    // Put remaining tasks back
                    for task in tasks {
                        over_queue.enqueue(task);
                    }
                }
            }
        }

        if moved > 0 {
            tracing::info!("Rebalanced: moved {} tasks", moved);
            *self.last_rebalance_at.write() = Some(std::time::Instant::now());
            self.oplog.write().record(
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
    pub async fn tick(&self) {
        #[cfg(feature = "system-metrics")]
        {
            let mut sys = self.sys.write();
            sys.refresh_cpu_all();
            sys.refresh_memory();
        }

        let config = self.config.read();
        let timeout = config.lock_timeout_ms as u128;
        let released = self.lock_manager.force_release_stale(timeout);
        if released > 0 {
            tracing::warn!(
                "Tick: forcefully released {} stale orphaned lock(s) older than {}ms",
                released,
                timeout
            );
        }

        let current_load = self.status().total_weighted_load;
        {
            let mut load_history = self.load_history.write();
            load_history.push_back(current_load);
            if load_history.len() > config.scaling_lookback_ticks {
                load_history.pop_front();
            }
        }

        // Global Idle Check
        let now_ms = crate::types::now_unix_ms();
        let global_idle_ms = now_ms.saturating_sub(self.last_activity_ms());
        if global_idle_ms >= config.idle_timeout_ms {
            self.event_bus.emit(crate::events::AgentEventKind::OrchestratorIdle {
                idle_ms: global_idle_ms,
            });
            if global_idle_ms < config.idle_timeout_ms + 10_000 {
                tracing::info!("Orchestrator has been idle for {}ms", global_idle_ms);
            }
        }

        // Task Expiration Check
        let task_timeout = std::time::Duration::from_millis(config.task_timeout_ms);
        for mut pair in self.agents.iter_mut() {
            let agent_id = *pair.key();
            let queue = pair.value_mut();
            let expired = queue.drain_timed_out(task_timeout);
            for task in expired {
                let age = now_ms.saturating_sub(task.created_at_ms);
                self.event_bus.emit(crate::events::AgentEventKind::TaskExpired {
                    task_id: task.id,
                    agent_id,
                    age_ms: age,
                });
                tracing::warn!("Task {} on agent {} expired (age: {}ms)", task.id, agent_id, age);
            }
        }

        let stale_ids = self.heartbeat_monitor.write().check_stale(&self.event_bus);
        for (id, level) in stale_ids {
            if self.dynamic_agents.contains_key(&id) {
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

        if config.auto_continue_enabled {
            let active_agents: Vec<(AgentId, usize)> = self
                .agents
                .iter()
                .map(|pair| (*pair.key(), pair.value().len()))
                .collect();
            let intents = self
                .monitor
                .write()
                .check_idle_agents(&active_agents, &self.event_bus);

            for (agent_id, prompt) in intents {
                let name = self.agents.get(&agent_id).map(|q| q.name.clone());
                let _ = self
                    .submit_task_with_agent(
                        format!("[Auto-Continuation] {}", prompt),
                        vec![],
                        Some(crate::types::TaskPriority::Background),
                        name,
                        None,
                        None,
                    )
                    .await;
            }
        }

        let urgent_threshold = config.urgent_rebalance_threshold;
        if urgent_threshold > 0 && self.agents.len() >= 2 {
            let cooldown_ms = config.scaling_cooldown_ms;
            let can_rebalance = self
                .last_rebalance_at
                .read()
                .map(|t| t.elapsed().as_millis() >= cooldown_ms as u128)
                .unwrap_or(true);

            if can_rebalance {
                let overloaded_urgent: Vec<(AgentId, usize)> = self
                    .agents
                    .iter()
                    .map(|pair| {
                        (*pair.key(), pair.value().depth_by_priority(crate::types::TaskPriority::Urgent))
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
