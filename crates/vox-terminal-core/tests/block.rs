use vox_terminal_core::block::{
    Block, BlockId, BlockKind, BlockOrigin, BlockStatus, OutputChunk, Stream,
};

#[test]
fn block_accumulates_output_and_finishes() {
    let mut b = Block::new(BlockId(1), BlockKind::Shell, "echo hi");
    assert_eq!(b.id, BlockId(1));
    assert_eq!(b.status, BlockStatus::Running);
    b.push(OutputChunk::text(Stream::Stdout, "hi\n"));
    b.finish(0);
    assert_eq!(b.status, BlockStatus::Ok);
    assert_eq!(b.exit_code, Some(0));
    assert_eq!(b.plain_output(), "hi\n");
}

#[test]
fn plain_output_strips_ansi() {
    let mut b = Block::new(BlockId(2), BlockKind::Shell, "ls");
    b.push(OutputChunk::text(Stream::Stdout, "\x1b[31mred\x1b[0m\n"));
    assert_eq!(b.plain_output(), "red\n");
}

#[test]
fn nonzero_exit_marks_failed() {
    let mut b = Block::new(BlockId(3), BlockKind::Shell, "false");
    b.finish(1);
    assert_eq!(b.status, BlockStatus::Failed);
    assert_eq!(b.exit_code, Some(1));
}

#[test]
fn block_defaults_to_user_origin() {
    let b = Block::new(BlockId(4), BlockKind::Shell, "ls");
    assert_eq!(b.origin, BlockOrigin::User);
}

#[test]
fn block_origin_roundtrips_and_defaults_on_missing_field() {
    let legacy_json =
        r#"{"id":1,"kind":"Shell","input":"ls","output":[],"status":"Ok","exit_code":0}"#;
    let parsed: Block = serde_json::from_str(legacy_json).expect("legacy json parses");
    assert_eq!(parsed.origin, BlockOrigin::User);

    let agent_block =
        Block::new(BlockId(5), BlockKind::Shell, "cargo check").with_origin(BlockOrigin::Agent {
            agent_id: "orch-1".to_string(),
        });
    let encoded = serde_json::to_string(&agent_block).expect("serialize");
    let decoded: Block = serde_json::from_str(&encoded).expect("deserialize");
    assert_eq!(
        decoded.origin,
        BlockOrigin::Agent {
            agent_id: "orch-1".to_string()
        }
    );
}
