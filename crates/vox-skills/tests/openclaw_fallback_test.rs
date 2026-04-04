use vox_skills::sandbox::{SandboxPolicy, resolve_policy};
use vox_skills::sandbox::fallback::{FallbackError, OpenClawSidecarSandbox};
use vox_skills::ars_shim::manifest::{SkillKind, TrustLevel};

#[test]
fn test_openclaw_sandbox_policy_resolution() {
    // Verified community document skills resolve to Container
    let policy1 = resolve_policy(SkillKind::Document, TrustLevel::Community);
    assert!(matches!(policy1, SandboxPolicy::Container));

    // Trusted tools resolve to Permissive
    let policy2 = resolve_policy(SkillKind::Tool, TrustLevel::Trusted);
    assert!(matches!(policy2, SandboxPolicy::Permissive));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_openclaw_fallback_connection_failure() {
    // Testing the OpenClaw adapter's unreachable fallback behavior.
    // Ensure that it gracefully fails and yields SidecarUnreachable instead of panicking.
    let res = OpenClawSidecarSandbox::connect().await;
    
    match res {
        Err(FallbackError::SidecarUnreachable(_)) => {
            // Expected since no sidecar is running in tests
        }
        _ => panic!("Expected SidecarUnreachable error when OpenClaw runtime sidecar is down"),
    }
}
