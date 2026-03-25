use super::context_assemble_bundle;

#[tokio::test]
async fn context_assemble_invalid_tier_errors() {
    let r = context_assemble_bundle("invalid_tier", None, None, None).await;
    assert!(r.is_err(), "invalid tier should error");
}

#[tokio::test]
async fn context_assemble_invalid_policy_json_errors() {
    let r = context_assemble_bundle("standard", Some("not valid json"), None, None).await;
    assert!(r.is_err(), "invalid policy JSON should error");
}

#[cfg(feature = "ars")]
#[tokio::test(flavor = "multi_thread")]
async fn context_assemble_valid_tier_succeeds() {
    use vox_db::{Codex, DbConfig};

    let db = Codex::connect(DbConfig::Memory)
        .await
        .expect("in-memory Codex");
    let r = context_assemble_bundle("standard", None, None, Some(&db)).await;
    assert!(r.is_ok(), "valid tier should succeed");
    assert!(r.unwrap().items.is_empty(), "no memories => empty bundle");
}

#[cfg(feature = "codex")]
#[tokio::test(flavor = "multi_thread")]
async fn context_assemble_with_agent_id_succeeds() {
    use vox_db::{Codex, CodexConfig};

    let db = Codex::connect(CodexConfig::Memory)
        .await
        .expect("in-memory Codex");
    let r = context_assemble_bundle("shallow", None, Some("test-agent"), Some(&db)).await;
    assert!(r.is_ok(), "valid tier with agent_id should succeed");
}

#[cfg(feature = "ars")]
#[tokio::test(flavor = "multi_thread")]
async fn context_assemble_with_memory_data_returns_non_empty_bundle() {
    use vox_db::MemoryParams;
    use vox_db::{Codex, DbConfig};

    let db = Codex::connect(DbConfig::Memory)
        .await
        .expect("in-memory Codex for test");
    let params = MemoryParams {
        agent_id: "test-agent",
        session_id: "test-session",
        memory_type: "session_turn",
        content: "User said hello",
        metadata: None,
        importance: 0.5,
        vcs_snapshot_id: None,
    };
    db.store_memory(params).await.expect("store one memory");
    let bundle = context_assemble_bundle("standard", None, None, Some(&db))
        .await
        .expect("assemble with test Codex");
    assert!(
        !bundle.items.is_empty(),
        "bundle should contain the stored memory, got {} items",
        bundle.items.len()
    );
}
