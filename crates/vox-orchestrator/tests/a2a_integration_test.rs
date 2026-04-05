use std::sync::Arc;
use vox_orchestrator::Orchestrator;
use vox_orchestrator::a2a::A2AMessageType;
use vox_orchestrator::config::OrchestratorConfig;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_orchestrator_a2a_message_bus_integration() {
    let orch = Orchestrator::new(OrchestratorConfig::for_testing());
    let agent1 = orch.spawn_agent("agent-a").expect("spawn agent A");
    let agent2 = orch.spawn_agent("agent-b").expect("spawn agent B");

    // Agents should be self-registered into the message bus on spawn logic (if applicable)
    // Send a message via Orchestrator's internal message bus
    let msg_id = orch.message_bus.send(
        agent1,
        agent2,
        A2AMessageType::HelpRequest,
        "Need assistance with testing",
    );

    // Verify the message reached agent2's inbox
    let inbox2 = orch.message_bus.inbox(agent2);
    assert_eq!(
        inbox2.len(),
        1,
        "Agent B should have received the A2A message"
    );
    let msg = &inbox2[0];

    assert_eq!(msg.id, msg_id);
    assert_eq!(msg.sender, agent1);
    assert_eq!(msg.msg_type, A2AMessageType::HelpRequest);
    assert_eq!(msg.payload, "Need assistance with testing");

    // Verify audit trail consistency
    let trail = orch.message_bus.audit_trail();
    assert!(
        trail.iter().any(|m| m.id == msg_id),
        "Message must be in the audit trail"
    );
}
