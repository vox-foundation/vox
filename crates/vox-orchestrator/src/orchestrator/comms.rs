use crate::types::AgentId;
use super::Orchestrator;

impl Orchestrator {
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
