//! Watchdog for Populi remote-delegated tasks.
//!
//! Identifies tasks that have exceeded their remote authoritative lease duration
//! and recovers them to the local agent queue.

use crate::orchestrator::Orchestrator;
use crate::types::now_unix_ms;

impl Orchestrator {
    /// Identifies and recovers expired Populi remote leases.
    ///
    /// Iterates through all agent queues and checks in-progress tasks.
    /// If a task is held for remote execution and its lease has expired,
    /// it triggers a local fallback.
    pub fn tick_populi_remote_lease_watchdog(&self) {
        let (lease_timeout_ms, lease_gating_enabled) = {
            let config = crate::sync_lock::rw_read(&*self.config);
            (
                config.populi_remote_lease_timeout_ms,
                config.populi_remote_lease_gating_enabled,
            )
        };

        // If lease gating is disabled globally, we skip the authoritative check.
        if !lease_gating_enabled {
            return;
        }

        let mut expired_tasks = Vec::new();
        let now = now_unix_ms();

        {
            let agents = crate::sync_lock::rw_read(&*self.agents);
            for (&agent_id, queue_lock) in agents.iter() {
                let queue = crate::sync_lock::rw_read(&**queue_lock);
                if let Some(task) = queue.current_task() {
                    if let Some(delegate) = &task.populi_remote_delegate {
                        if let Some(started_at) = task.started_at_ms {
                            let age_ms = now.saturating_sub(started_at);
                            if age_ms >= lease_timeout_ms {
                                tracing::warn!(
                                    task_id = task.id.0,
                                    agent_id = agent_id.0,
                                    lease_id = ?delegate.lease_id,
                                    age_ms,
                                    timeout_ms = lease_timeout_ms,
                                    "Populi remote lease expired; triggering local fallback"
                                );
                                expired_tasks.push(task.id);
                            }
                        }
                    }
                }
            }
        }

        for task_id in expired_tasks {
            let reason = format!("Populi remote lease expired after {lease_timeout_ms}ms");
            if let Err(e) = self.fallback_populi_remote_task_locally(task_id, &reason) {
                tracing::error!(
                    task_id = task_id.0,
                    error = %e,
                    "Failed to fallback expired remote task locally"
                );
            } else {
                self.event_bus
                    .emit(crate::events::AgentEventKind::TaskExpired {
                        task_id,
                        agent_id: crate::types::AgentId(0), // Orchestrator-level reaper event
                        age_ms: lease_timeout_ms,
                    });
            }
        }
    }
}
