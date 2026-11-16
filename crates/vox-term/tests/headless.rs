//! Piped-stdin / `TERM=dumb` must not crash or emit ANSI escapes.
//!
//! Bug (2026-09-21): `printf 'x\n' | TERM=dumb vox term` emitted ANSI escapes
//! then errored `Failed to initialize input reader`, even though `vox term
//! --help` documents plain-text headless output under `TERM=dumb` /
//! non-tty. See `crates/vox-term/src/app.rs::is_headless`/`run_plain`.

use std::io::Write;
use std::process::{Command, Stdio};

fn run_headless(configure: impl FnOnce(&mut Command)) -> std::process::Output {
    let exe = env!("CARGO_BIN_EXE_vox-term");
    let mut cmd = Command::new(exe);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure(&mut cmd);

    let mut child = cmd.spawn().expect("failed to spawn vox-term");
    child
        .stdin
        .as_mut()
        .expect("stdin was piped")
        .write_all(b"x\n")
        .expect("failed to write to stdin");
    drop(child.stdin.take()); // EOF so the plain-mode loop can exit.

    child
        .wait_with_output()
        .expect("failed to wait on vox-term")
}

fn assert_headless_ok(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "vox-term exited with {:?}, stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stdout.contains(&0x1b),
        "expected no ANSI escapes in headless stdout, got: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Failed to initialize input reader"),
        "crossterm input reader should never be initialized in headless mode, stderr: {stderr}"
    );
}

#[test]
fn term_dumb_piped_stdin_no_ansi_no_crash() {
    let output = run_headless(|cmd| {
        cmd.env("TERM", "dumb");
    });
    assert_headless_ok(&output);
}

#[test]
fn non_tty_piped_stdin_no_ansi_no_crash() {
    // No TERM=dumb here — piped stdin/stdout alone (e.g. `cmd | vox term`)
    // must also be detected as headless.
    let output = run_headless(|cmd| {
        cmd.env_remove("TERM");
    });
    assert_headless_ok(&output);
}

#[test]
fn parse_cli_args_handles_attach_and_local() {
    let bare = vox_term::app::TermCliArgs::parse_iter(["vox-term", "--attach"]);
    assert_eq!(bare.attach.as_deref(), Some("latest"));
    assert!(!bare.force_local);

    let bare_with_local =
        vox_term::app::TermCliArgs::parse_iter(["vox-term", "--attach", "--local"]);
    assert_eq!(bare_with_local.attach.as_deref(), Some("latest"));
    assert!(bare_with_local.force_local);

    let empty_eq = vox_term::app::TermCliArgs::parse_iter(["vox-term", "--attach="]);
    assert_eq!(empty_eq.attach.as_deref(), Some("latest"));

    let args =
        vox_term::app::TermCliArgs::parse_iter(["vox-term", "--attach", "term-2", "--local"]);
    assert_eq!(args.attach.as_deref(), Some("term-2"));
    assert!(args.force_local);

    let eq_args = vox_term::app::TermCliArgs::parse_iter(["vox-term", "--attach=term-9"]);
    assert_eq!(eq_args.attach.as_deref(), Some("term-9"));
    assert!(!eq_args.force_local);
}

#[tokio::test]
async fn resolve_host_falls_back_to_local_when_daemon_unreachable() {
    let host = vox_term::app::resolve_host(true).await;
    let id = host
        .open(vox_terminal_core::OpenSpec::default())
        .await
        .expect("open local session");
    let attached = host.attach(&id, None).await.expect("attach local session");
    assert!(attached.blocks.is_empty());
    host.close(&id).await.expect("close local session");
}

#[tokio::test]
async fn attach_or_open_session_numeric_latest_reattach_and_missing_errors() {
    use vox_terminal_core::SessionHost;

    let empty_host = vox_terminal_core::LocalHost::default();
    let empty_err = vox_term::app::attach_or_open_session(&empty_host, Some("latest"), 120, 32)
        .await
        .err()
        .expect("attaching latest on empty host must fail");
    assert!(
        empty_err
            .to_string()
            .contains("no active terminal sessions"),
        "unexpected empty-host error: {empty_err}"
    );

    let missing_err = vox_term::app::attach_or_open_session(&empty_host, Some("term-999"), 120, 32)
        .await
        .err()
        .expect("attaching missing session must fail");
    assert!(
        missing_err.to_string().contains("term-999"),
        "missing-session error should name session id, got: {missing_err}"
    );

    // Numeric "latest" ordering: "term-10" must beat "term-9" despite lexicographic list order.
    let synth_host = vox_terminal_core::LocalHost::default();
    for _ in 0..10 {
        synth_host.open_synthetic(120, 32);
    }
    let listed = SessionHost::list(&synth_host).await.expect("list sessions");
    assert_eq!(
        vox_term::app::pick_latest_session_id(&listed).as_deref(),
        Some("term-10")
    );
    let (latest_sid, _att) =
        vox_term::app::attach_or_open_session(&synth_host, Some("latest"), 120, 32)
            .await
            .expect("attach latest");
    assert_eq!(latest_sid, "term-10");

    // Live PTY open -> detach -> submit -> explicit reattach preserves blocks and output.
    let live_host = vox_terminal_core::LocalHost::default();
    let (sid, first_att) = vox_term::app::attach_or_open_session(&live_host, None, 120, 32)
        .await
        .expect("open term-1");
    assert_eq!(sid, "term-1");
    drop(first_att); // Detach client without closing session.

    SessionHost::submit(&live_host, &sid, "!echo persisted")
        .await
        .expect("submit echo persisted");

    let (reattached_sid, reattached) =
        vox_term::app::attach_or_open_session(&live_host, Some("term-1"), 120, 32)
            .await
            .expect("reattach term-1");
    assert_eq!(reattached_sid, "term-1");
    assert!(
        reattached
            .blocks
            .iter()
            .any(|b| b.plain_output().contains("persisted")),
        "expected preserved block output containing 'persisted', got: {:?}",
        reattached.blocks
    );
    SessionHost::close(&live_host, &sid)
        .await
        .expect("close live session");
}

#[test]
fn vt_grid_clamps_zero_dimensions_tracks_alt_screen_and_resets() {
    let mut grid = vox_term::vt::VtGrid::new(0, 0);
    assert!(!grid.is_alt_screen());
    grid.resize(0, 0);
    let _ = grid.render_lines();

    grid.feed(b"\x1b[?1049h");
    assert!(grid.is_alt_screen());

    grid.reset();
    assert!(!grid.is_alt_screen());
}

#[tokio::test]
async fn apply_session_event_syncs_peer_block_metadata_and_appends_output_in_place() {
    use vox_terminal_core::{
        BlockId, BlockStatus, LocalHost, OutputChunk, SessionEvent, SessionHost, Stream,
    };

    let host = LocalHost::default();
    let sid = host.open_synthetic(120, 32);
    let mut att = SessionHost::attach(&host, &sid, None)
        .await
        .expect("initial attach");
    let last_seq = att.seq;
    let mut blocks_state = att.blocks;
    let mut agent_lines = Vec::new();
    let mut alt_screen = att.alt_screen;

    // Peer submits a failing command (`!exit 7`) on the shared session.
    let block_id = SessionHost::submit(&host, &sid, "!exit 7")
        .await
        .expect("peer submit !exit 7");

    while let Ok(ev) = att.events.try_recv() {
        vox_term::app::apply_session_event(
            &host,
            &sid,
            last_seq,
            &mut blocks_state,
            &mut agent_lines,
            &mut alt_screen,
            ev,
        )
        .await;
    }

    let synced = blocks_state
        .iter()
        .find(|b| b.id == block_id)
        .expect("synced peer block present");
    assert_eq!(synced.input, "exit 7");
    assert_eq!(synced.exit_code, Some(7));
    assert_eq!(synced.status, BlockStatus::Failed);

    // OutputAppended mutates blocks_state in-place without requiring a host round-trip.
    vox_term::app::apply_session_event(
        &host,
        "nonexistent-session-proves-no-rpc",
        last_seq,
        &mut blocks_state,
        &mut agent_lines,
        &mut alt_screen,
        SessionEvent::OutputAppended {
            id: BlockId(block_id.0),
            chunk: OutputChunk::text(Stream::Stdout, "streamed-chunk"),
        },
    )
    .await;
    let updated = blocks_state.iter().find(|b| b.id == block_id).unwrap();
    assert!(
        updated
            .output
            .iter()
            .any(|c| c.text.contains("streamed-chunk"))
    );

    // Verify in-place [BlockOpened, OutputAppended, BlockClosed] sequence never calls host.attach
    // (proven by passing a nonexistent session id) and never duplicates streamed output chunks.
    let mut clean_blocks = Vec::new();
    let mut clean_agent = Vec::new();
    let mut clean_alt = false;
    for ev in [
        SessionEvent::BlockOpened {
            id: BlockId(42),
            kind: Some(vox_terminal_core::BlockKind::Shell),
            input: "echo once".to_string(),
            origin: Some(vox_terminal_core::BlockOrigin::User),
            cwd: Some("/workspace".to_string()),
            git_branch: Some("main".to_string()),
        },
        SessionEvent::OutputAppended {
            id: BlockId(42),
            chunk: OutputChunk::text(Stream::Stdout, "once\n"),
        },
        SessionEvent::BlockClosed {
            id: BlockId(42),
            exit_code: Some(0),
            status: Some(BlockStatus::Ok),
        },
    ] {
        vox_term::app::apply_session_event(
            &host,
            "nonexistent-session-no-attach-rpc",
            0,
            &mut clean_blocks,
            &mut clean_agent,
            &mut clean_alt,
            ev,
        )
        .await;
    }
    assert_eq!(clean_blocks.len(), 1);
    assert_eq!(clean_blocks[0].input, "echo once");
    assert_eq!(clean_blocks[0].plain_output(), "once\n");
    assert_eq!(clean_blocks[0].status, BlockStatus::Ok);
    assert_eq!(clean_blocks[0].exit_code, Some(0));

    // SnapshotResync replaces blocks_state and updates alt_screen in-place.
    vox_term::app::apply_session_event(
        &host,
        "nonexistent-session-no-attach-rpc",
        0,
        &mut clean_blocks,
        &mut clean_agent,
        &mut clean_alt,
        SessionEvent::SnapshotResync {
            blocks: vec![],
            resync_required: true,
            alt_screen: true,
            cwd: None,
            git_branch: None,
        },
    )
    .await;
    assert!(clean_blocks.is_empty());
    assert!(clean_alt);
}

#[test]
fn key_event_to_vt_bytes_ignores_release_events() {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    let press =
        KeyEvent::new_with_kind(KeyCode::Char('x'), KeyModifiers::NONE, KeyEventKind::Press);
    let repeat =
        KeyEvent::new_with_kind(KeyCode::Char('x'), KeyModifiers::NONE, KeyEventKind::Repeat);
    let release = KeyEvent::new_with_kind(
        KeyCode::Char('x'),
        KeyModifiers::NONE,
        KeyEventKind::Release,
    );

    assert_eq!(
        vox_term::app::key_event_to_vt_bytes(&press),
        Some(b"x".to_vec())
    );
    assert_eq!(
        vox_term::app::key_event_to_vt_bytes(&repeat),
        Some(b"x".to_vec())
    );
    assert_eq!(vox_term::app::key_event_to_vt_bytes(&release), None);
}
