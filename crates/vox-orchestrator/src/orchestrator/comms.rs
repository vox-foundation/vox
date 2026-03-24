use crate::types::AgentId;
use super::Orchestrator;

impl Orchestrator {
    pub fn send_a2a(
        &self,
        sender: AgentId,
        receiver: AgentId,
        msg_type: crate::types::A2AMessageType,
        payload: impl Into<String>,
    ) -> crate::types::MessageId {
        let payload_str = payload.into();

        // Native VCS integration: When an agent hands off a plan to another, automatically
        // start tracking a logical Change in the workspace manager for provenance visibility.
        if msg_type == crate::types::A2AMessageType::PlanHandoff {
            self.workspace_manager.write().create_change(
                receiver,
                format!("Plan handoff from {}: {:.100}", sender, payload_str),
            );
        }

        let mut bus = self.message_bus.write();
        let msg_id = bus.send(sender, receiver, msg_type, payload_str);
        
        if let Some(msg) = bus.audit_trail().last() {
            let msg_cloned = msg.clone();
            drop(bus); // Release lock before emitting events to avoid potential deadlocks
            
            self.bulletin
                .publish(crate::types::AgentMessage::A2A(msg_cloned.clone()));

            self.event_bus
                .emit(crate::events::AgentEventKind::MessageSent {
                    from: msg_cloned.sender,
                    to: msg_cloned.receiver,
                    summary: format!("{:?}: {}", msg_cloned.msg_type, msg_cloned.payload),
                });
        }
        msg_id
    }

    /// Broadcast a structured A2A message to all and publish to bulletin.
    pub fn broadcast_a2a(
        &self,
        sender: AgentId,
        msg_type: crate::types::A2AMessageType,
        payload: impl Into<String>,
    ) -> crate::types::MessageId {
        let mut bus = self.message_bus.write();
        let msg_id = bus.broadcast(sender, msg_type, payload);
        
        if let Some(msg) = bus.audit_trail().last() {
            let msg_cloned = msg.clone();
            drop(bus); // Release lock before emitting events
            
            self.bulletin
                .publish(crate::types::AgentMessage::A2A(msg_cloned.clone()));

            self.event_bus
                .emit(crate::events::AgentEventKind::MessageSent {
                    from: msg_cloned.sender,
                    to: None, // Broadcast
                    summary: format!("{:?}: {}", msg_cloned.msg_type, msg_cloned.payload),
                });
        }
        msg_id
    }
}
