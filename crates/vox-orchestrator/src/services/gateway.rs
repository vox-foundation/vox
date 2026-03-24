//! Message gateway: unified fan-out to bulletin, A2A bus, and event bus.
//!
//! Provides a single API to publish notifications so that dashboard,
//! monitors, and other agents see consistent updates.

use crate::a2a::MessageBus;
use crate::bulletin::BulletinBoard;
use crate::events::{AgentEventKind, EventBus};
use crate::types::A2AMessageType;
use crate::types::{AgentId, AgentMessage, TaskId};

/// Unified message gateway for orchestrator notifications.
///
/// Use the associated functions to publish to bulletin, A2A, and event bus
/// in one place so all consumers stay in sync.
pub struct MessageGateway;

impl MessageGateway {
    /// Publish task completion to bulletin, A2A audit, and event stream.
    pub fn publish_task_completed(
        bulletin: &BulletinBoard,
        message_bus: &parking_lot::RwLock<MessageBus>,
        event_bus: &EventBus,
        task_id: TaskId,
        agent_id: AgentId,
        session_id: Option<String>,
    ) {
        bulletin.publish(AgentMessage::TaskCompleted { task_id, agent_id });
        let _ = message_bus.write().broadcast(
            agent_id,
            A2AMessageType::CompletionNotice,
            format!("Task {} completed", task_id),
        );
        event_bus.emit(AgentEventKind::TaskCompleted {
            task_id,
            agent_id,
            session_id,
        });
    }

    /// Publish task failure to bulletin and event stream.
    pub fn publish_task_failed(
        bulletin: &BulletinBoard,
        event_bus: &EventBus,
        task_id: TaskId,
        agent_id: AgentId,
        error: String,
        session_id: Option<String>,
    ) {
        bulletin.publish(AgentMessage::TaskFailed {
            agent_id,
            task_id,
            error: error.clone(),
        });
        event_bus.emit(AgentEventKind::TaskFailed {
            task_id,
            agent_id,
            error,
            session_id,
        });
    }

    /// Publish agent spawned to bulletin and event stream.
    pub fn publish_agent_spawned(
        bulletin: &BulletinBoard,
        event_bus: &EventBus,
        agent_id: AgentId,
        name: String,
    ) {
        bulletin.publish(AgentMessage::AgentSpawned {
            agent_id,
            name: name.clone(),
        });
        event_bus.emit(AgentEventKind::AgentSpawned { agent_id, name });
    }

    /// Publish agent retired to event stream (bulletin has no AgentRetired variant; event is primary).
    pub fn publish_agent_retired(event_bus: &EventBus, agent_id: AgentId) {
        event_bus.emit(AgentEventKind::AgentRetired { agent_id });
    }

    /// Publish Q&A or broadcast [`AgentMessage`] to the bulletin and mirror a short summary on the event bus.
    pub fn publish_bulletin_inter_agent(
        bulletin: &BulletinBoard,
        event_bus: &EventBus,
        msg: AgentMessage,
    ) {
        let preview = match &msg {
            AgentMessage::Question {
                from,
                to,
                question,
                correlation_id,
            } => Some((
                *from,
                Some(*to),
                format!(
                    "Q[{}]: {}",
                    correlation_id,
                    question.chars().take(100).collect::<String>()
                ),
            )),
            AgentMessage::Answer {
                from,
                to,
                answer,
                correlation_id,
            } => Some((
                *from,
                Some(*to),
                format!(
                    "A[{}]: {}",
                    correlation_id,
                    answer.chars().take(100).collect::<String>()
                ),
            )),
            AgentMessage::Broadcast { from, message } => Some((
                *from,
                None,
                format!(
                    "Broadcast: {}",
                    message.chars().take(100).collect::<String>()
                ),
            )),
            _ => None,
        };

        bulletin.publish(msg);

        if let Some((from, to, summary)) = preview {
            event_bus.emit(AgentEventKind::MessageSent { from, to, summary });
        }
    }
}
