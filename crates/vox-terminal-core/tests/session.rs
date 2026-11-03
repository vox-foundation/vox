use vox_terminal_core::block::{BlockKind, BlockOrigin, BlockStatus};
use vox_terminal_core::host::{LocalHost, OpenSpec};
use vox_terminal_core::session::Session;

#[test]
fn shell_block_lifecycle_from_osc_events() {
    let mut s = Session::new("s1");
    s.on_pty_bytes(b"\x1b]633;A\x07\x1b]633;E;ls\x07\x1b]633;C\x07out\n\x1b]633;D;0\x07");
    let blocks = s.blocks();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].input, "ls");
    assert_eq!(blocks[0].plain_output(), "out\n");
    assert_eq!(blocks[0].status, BlockStatus::Ok);
}

#[test]
fn multiple_blocks_sequential() {
    let mut s = Session::new("s2");
    s.on_pty_bytes(b"\x1b]633;A\x07\x1b]633;E;echo a\x07\x1b]633;C\x07a\n\x1b]633;D;0\x07");
    s.on_pty_bytes(b"\x1b]633;A\x07\x1b]633;E;echo b\x07\x1b]633;C\x07b\n\x1b]633;D;1\x07");
    let blocks = s.blocks();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].input, "echo a");
    assert_eq!(blocks[0].status, BlockStatus::Ok);
    assert_eq!(blocks[1].input, "echo b");
    assert_eq!(blocks[1].status, BlockStatus::Failed);
}

#[test]
fn block_ids_are_unique() {
    let mut s = Session::new("s3");
    s.on_pty_bytes(b"\x1b]633;A\x07\x1b]633;D;0\x07");
    s.on_pty_bytes(b"\x1b]633;A\x07\x1b]633;D;0\x07");
    let ids: Vec<_> = s.blocks().iter().map(|b| b.id).collect();
    assert_ne!(ids[0], ids[1]);
}

#[tokio::test]
async fn submit_with_agent_origin_tags_block() {
    let host = LocalHost::default();
    let id = host.open(OpenSpec::default()).await.unwrap();
    let block_id = host
        .submit_with_origin(
            &id,
            "!echo agent-hi",
            BlockOrigin::Agent {
                agent_id: "agent-7".to_string(),
            },
        )
        .unwrap();
    let b = host.block(&id, block_id).expect("block exists");
    assert_eq!(
        b.origin,
        BlockOrigin::Agent {
            agent_id: "agent-7".to_string()
        }
    );
    assert!(b.plain_output().contains("agent-hi"));
    host.close(&id).unwrap();
}

#[tokio::test]
async fn submit_default_remains_user_origin() {
    let host = LocalHost::default();
    let id = host.open(OpenSpec::default()).await.unwrap();
    let block_id = host.submit(&id, "!echo user-hi").unwrap();
    let b = host.block(&id, block_id).expect("block exists");
    assert_eq!(b.origin, BlockOrigin::User);
    host.close(&id).unwrap();
}

#[tokio::test]
async fn organic_submit_evaluates_vox_shell_and_routed_agent_prompts() {
    use vox_terminal_core::block::BlockKind;

    let host = LocalHost::default();
    let id = host.open(OpenSpec::default()).await.unwrap();

    // 1. Vox-native expression evaluation
    let vox_id = host.submit(&id, "40 + 2").unwrap();
    let vox_block = host.block(&id, vox_id).unwrap();
    assert_eq!(vox_block.kind, BlockKind::VoxNative);
    assert_eq!(vox_block.status, BlockStatus::Ok);
    assert!(vox_block.plain_output().contains("42"));

    // 2. Unprefixed shell command (as submitted from GUI Console InputEditor)
    let sh_id = host.submit(&id, "echo organic-console-line").unwrap();
    let sh_block = host.block(&id, sh_id).unwrap();
    assert_eq!(sh_block.kind, BlockKind::Shell);
    assert_eq!(sh_block.status, BlockStatus::Ok);
    assert!(sh_block.plain_output().contains("organic-console-line"));

    // 3. ?prompt shorthand routes intelligently through vox_orchestrator::models
    let ai_id = host
        .submit(&id, "?fix the failing test with exit code 1")
        .unwrap();
    let ai_block = host.block(&id, ai_id).unwrap();
    assert_eq!(ai_block.kind, BlockKind::AgentTurn);
    assert_eq!(ai_block.status, BlockStatus::Ok);
    assert!(ai_block.plain_output().contains("[routed model="));
    assert!(ai_block.plain_output().contains("task=Debugging"));

    // 4. /model route slash command
    let cmd_id = host
        .submit(&id, "/model route architect a terminal session bridge")
        .unwrap();
    let cmd_block = host.block(&id, cmd_id).unwrap();
    assert_eq!(cmd_block.kind, BlockKind::SlashCommand);
    assert_eq!(cmd_block.status, BlockStatus::Ok);
    assert!(cmd_block.plain_output().contains("task=Planning"));

    host.close(&id).unwrap();
}

#[tokio::test]
async fn openspec_custom_shell_and_structured_shell_submission() {
    let host = LocalHost::default();
    let spec = OpenSpec {
        shell: Some("sh".to_string()),
        ..Default::default()
    };
    let id = host.open(spec).await.unwrap();

    let block_id = host.submit(&id, "!nu echo structured-test").unwrap();
    let block = host.block(&id, block_id).expect("block exists");
    assert_eq!(block.kind, BlockKind::Shell);
    assert!(block.plain_output().contains("structured-test"));

    host.close(&id).unwrap();
}
