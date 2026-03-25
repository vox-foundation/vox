#![allow(missing_docs)]
//! Integration test for session ↔ agent lifecycle.
use vox_orchestrator::{Orchestrator, OrchestratorConfig};

#[tokio::test]
async fn agent_session_lifecycle_tracks_session_id() {
    let config = OrchestratorConfig::default();
    let orchestrator = Orchestrator::new(config);

    // Map session to a new agent.
    let agent_id = orchestrator.spawn_agent("TestAgentSession").unwrap();
    let session_id = "vox_sess_123456";

    assert!(
        orchestrator
            .map_agent_session(agent_id, session_id.to_string())
            .is_ok(),
        "map_agent_session should succeed"
    );

    // Verify the session is tracked in the orchestrator status.
    let status = orchestrator.status();
    let agent_has_session = status
        .agents
        .iter()
        .any(|a| a.agent_session_id.as_deref() == Some(session_id));

    assert!(
        agent_has_session,
        "Agent session was not tracked by agent status"
    );
}
