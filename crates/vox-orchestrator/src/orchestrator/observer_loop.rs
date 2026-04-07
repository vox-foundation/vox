use crate::orchestrator::Orchestrator;
use crate::types::AgentId;
use std::sync::Arc;
use std::time::Duration;

/// Autonomous observer daemon that watches for workflow blockages
/// and orchestrates MENS anomaly reporting.
pub async fn run_observer_loop(orch: Arc<Orchestrator>) {
    let cfg = orch.config.read().unwrap().clone();
    if !cfg.observer_enabled {
        return;
    }

    let interval = cfg.observer_poll_interval_ms;
    let mut ticker = tokio::time::interval(Duration::from_millis(interval));

    loop {
        ticker.tick().await;

        let assignments = {
            let lock = orch.task_assignments.read().unwrap();
            lock.clone()
        };

        for (_, agent_id) in assignments {
            if let Some(queue) = orch.agent_queue(agent_id) {
                let q = queue.read().unwrap();
                let queue_depth = q.tasks().len();

                // If the queue is saturated...
                if queue_depth > 10 {
                    tracing::warn!(
                        "MENS Observer: Agent {} is saturated with {} tasks.",
                        agent_id,
                        queue_depth
                    );

                    // Fire a Socrates validation request on the secondary channel
                    let msg_payload = serde_json::json!({
                        "observation_type": "QueueSaturation",
                        "queue_depth": queue_depth
                    });

                    // Fallback sender ID
                    let system_id = AgentId(1);

                    orch.message_bus.send(
                        system_id,
                        agent_id,
                        crate::a2a::A2AMessageType::SocratesResearchRequest,
                        msg_payload.to_string(),
                    );

                    // Stream to MCP for realtime VS Code dashboarding
                    orch.event_bus()
                        .emit(crate::events::AgentEventKind::MensObserverObservation {
                            agent_id,
                            observation_type: "QueueSaturation".to_string(),
                            queue_depth,
                        });
                } else if queue_depth == 0 && agent_id.0 > 0 {
                    // OAPV Auto-scale down: If the agent is idle and isn't the primary agent (ID 0)
                    tracing::info!(
                        "MENS Observer: Agent {} is idle. Triggering scale-down evaluation.",
                        agent_id
                    );

                    let msg_payload = serde_json::json!({
                        "observation_type": "IdleAgentDetected",
                        "queue_depth": 0
                    });

                    orch.message_bus.send(
                        AgentId(1), // System ID
                        agent_id,
                        crate::a2a::A2AMessageType::SocratesResearchRequest,
                        msg_payload.to_string(),
                    );

                    orch.event_bus()
                        .emit(crate::events::AgentEventKind::MensObserverObservation {
                            agent_id,
                            observation_type: "IdleScaleDown".to_string(),
                            queue_depth: 0,
                        });
                }
            }
        }
    }
}
