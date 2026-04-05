use vox_skills::ars_shim::manifest::{SkillKind, TrustLevel};
use vox_skills::sandbox::fallback::{FallbackError, OpenClawSidecarSandbox};
use vox_skills::sandbox::{SandboxPolicy, resolve_policy};

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_execute_skill_deny_by_default() {
    use serde_json::json;
    use std::sync::Arc;
    use vox_skills::ars_shim::domain::ArsSkill;
    use vox_skills::ars_shim::runtime::{ArsRuntime, ArsRuntimeError};

    let db = Arc::new(
        vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
            .await
            .unwrap(),
    );
    let hooks = Arc::new(vox_skills::ars_shim::hooks::HookRegistry::new());
    let runtime = ArsRuntime::new(db, hooks);

    let skill = ArsSkill {
        id: "test_denied_secret".into(),
        namespace: "test".into(),
        name: "test".into(),
        version: "1.0.0".into(),
        content_hash: "hash".into(),
        description: None,
        author: None,
        body: None,
        kind: vox_skills::ars_shim::manifest::SkillKind::Tool,
        trust: vox_skills::ars_shim::manifest::TrustLevel::Trusted,
        resource_limits: vox_skills::ars_shim::manifest::ResourceLimits::default(),
        metadata: json!({
            "requested_secrets": ["FAKE_UNATHORIZED_KEY"]
        }),
    };

    let result = runtime.execute_skill("run-123", &skill, json!({})).await;

    // We expect it to be unauthorized/invalid since 'FAKE_UNATHORIZED_KEY' isn't available
    assert!(
        matches!(result, Err(ArsRuntimeError::InvalidRun(_))),
        "Expected InvalidRun error due to deny-by-default secret gating, but got {:?}",
        result
    );
}
