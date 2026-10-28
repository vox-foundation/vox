use vox_terminal_core::block::BlockOrigin;
use vox_terminal_core::host::{LocalHost, OpenSpec, TermControlMode, TermSecurityLevel};

#[tokio::test]
async fn security_levels_gate_agent_commands_and_autopilot_drops_on_user_input() {
    let host = LocalHost::default();
    let id = host.open(OpenSpec::default()).await.unwrap();

    // Default security level is L3StructuredDefault and default control mode is CoPilot
    assert_eq!(
        host.security_level(&id).unwrap(),
        TermSecurityLevel::L3StructuredDefault
    );
    assert_eq!(host.control_mode(&id).unwrap(), TermControlMode::CoPilot);

    // L1ReadOnly permits echo/ls/git status, rejects cargo/rm
    host.set_security_level(&id, TermSecurityLevel::L1ReadOnly)
        .unwrap();
    assert!(
        host.submit_with_origin(
            &id,
            "!echo allowed",
            BlockOrigin::Agent {
                agent_id: "a1".into(),
            },
        )
        .is_ok()
    );
    assert!(
        host.submit_with_origin(
            &id,
            "!cargo build",
            BlockOrigin::Agent {
                agent_id: "a1".into(),
            },
        )
        .is_err()
    );

    // L2WorkspaceBuild permits cargo/echo, rejects rm
    host.set_security_level(&id, TermSecurityLevel::L2WorkspaceBuild)
        .unwrap();
    assert!(
        host.submit_with_origin(
            &id,
            "!rm somefile",
            BlockOrigin::Agent {
                agent_id: "a1".into(),
            },
        )
        .is_err()
    );

    // Autopilot automatically drops to Manual on user submit or keystroke input
    host.set_control_mode(&id, TermControlMode::Autopilot)
        .unwrap();
    assert_eq!(host.control_mode(&id).unwrap(), TermControlMode::Autopilot);
    host.submit(&id, "!echo user-took-wheel").unwrap();
    assert_eq!(host.control_mode(&id).unwrap(), TermControlMode::Manual);

    host.set_control_mode(&id, TermControlMode::Autopilot)
        .unwrap();
    host.input(&id, b"a").unwrap();
    assert_eq!(host.control_mode(&id).unwrap(), TermControlMode::Manual);
}
