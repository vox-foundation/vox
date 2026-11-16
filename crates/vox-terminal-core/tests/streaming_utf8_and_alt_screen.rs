use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use serde_json::json;
use tokio::sync::broadcast;
use vox_terminal_core::block::{Block, BlockId, BlockKind, BlockStatus, OutputChunk, Stream};
use vox_terminal_core::daemon_host;
use vox_terminal_core::host::{LocalHost, OutputFrame, SessionHost, SessionInfo};
use vox_terminal_core::osc633::{Osc633Event, Osc633Parser};
use vox_terminal_core::pty::TermSignal;
use vox_terminal_core::session::{Session, SessionEvent};
use vox_terminal_core::utf8_stream::{Utf8StreamDecoder, normalize_cr_overwrites};
use vox_terminal_core::vt_shadow::VtShadow;

#[test]
fn prime_chunk_utf8_reassembly_zero_replacement_chars() {
    let rich_body = concat!(
        "╭──────────────────────────╮\r\n",
        "│ 🦀 Rust + 🚀 Warp + 🧪 Test │\r\n",
        "│ \x1b[38;5;208mcolored éλΩ text\x1b[0m       │\r\n",
        "╰──────────────────────────╯\r\n",
    );
    let stream_payload = format!(
        "\x1b]7;file://localhost/tmp/%F0%9F%A6%80-workspace\x07\
         \x1b]633;A\x07Prompt 🦀 > \x1b]633;B\x07\
         \x1b]633;E;echo 🦀╭─╮\x07\
         \x1b]633;C\x07\
         {rich_body}\
         \x1b]633;D;0\x07"
    );
    let raw_bytes = stream_payload.as_bytes();

    for chunk_size in [1usize, 7, 13, 31] {
        // 1. Direct Utf8StreamDecoder verification across prime chunk boundaries.
        let mut dec = Utf8StreamDecoder::new();
        let mut decoded = String::new();
        for chunk in raw_bytes.chunks(chunk_size) {
            decoded.push_str(&dec.push(chunk));
        }
        decoded.push_str(&dec.flush_lossy());
        assert!(
            !decoded.contains('\u{FFFD}'),
            "chunk_size={chunk_size} produced U+FFFD in Utf8StreamDecoder"
        );
        assert_eq!(decoded, stream_payload, "chunk_size={chunk_size} mismatch");

        // 2. Osc633Parser + Session verification across prime chunk boundaries.
        let mut parser = Osc633Parser::new();
        let mut events = Vec::new();
        let mut session = Session::new(format!("utf8-{chunk_size}"));
        for chunk in raw_bytes.chunks(chunk_size) {
            events.extend(parser.feed(chunk));
            session.on_pty_bytes(chunk);
        }

        for ev in &events {
            if let Osc633Event::Output(text) = ev {
                assert!(
                    !text.contains('\u{FFFD}'),
                    "chunk_size={chunk_size} produced U+FFFD in Osc633Event::Output: {text:?}"
                );
            }
        }
        assert!(events.contains(&Osc633Event::Osc7Cwd("/tmp/🦀-workspace".to_string())));
        assert!(events.contains(&Osc633Event::CommandLine("echo 🦀╭─╮".to_string())));
        assert!(events.contains(&Osc633Event::Exit(0)));

        assert_eq!(session.cwd(), Some("/tmp/🦀-workspace"));
        assert_eq!(session.blocks().len(), 1);
        let block = &session.blocks()[0];
        assert_eq!(block.input, "echo 🦀╭─╮");
        assert_eq!(block.status, BlockStatus::Ok);
        let raw_block_out: String = block.output.iter().map(|c| c.text.as_str()).collect();
        assert!(!raw_block_out.contains('\u{FFFD}'));
        assert_eq!(raw_block_out, rich_body);
    }

    // Genuine invalid UTF-8 emits U+FFFD and recovers cleanly on subsequent ASCII;
    // incomplete trailing UTF-8 flushed at EOF via flush_lossy() emits U+FFFD.
    let mut dec = Utf8StreamDecoder::new();
    assert_eq!(dec.push(b"pre-\xff-post"), "pre-\u{FFFD}-post");
    assert_eq!(dec.push(&"🦀".as_bytes()[..2]), "");
    assert_eq!(dec.flush_lossy(), "\u{FFFD}");
    assert_eq!(dec.push(b"recovered"), "recovered");
}

#[test]
fn osc633_streams_live_output_between_c_and_d_before_exit() {
    let mut parser = Osc633Parser::new();
    let pre_exit =
        b"\x1b]633;A\x07\x1b]633;B\x07\x1b]633;E;cargo build\x07\x1b]633;C\x07line-one\n";
    let evs = parser.feed(pre_exit);
    assert!(
        evs.contains(&Osc633Event::Output("line-one\n".to_string())),
        "expected live Output before 633;D, got: {evs:?}"
    );
    assert!(!evs.iter().any(|e| matches!(e, Osc633Event::Exit(_))));

    let mut session = Session::new("live-stream");
    let mut rx = session.subscribe();
    session.on_pty_bytes(pre_exit);

    assert_eq!(session.blocks().len(), 1);
    assert_eq!(session.blocks()[0].status, BlockStatus::Running);
    assert_eq!(session.blocks()[0].plain_output(), "line-one\n");

    let mut saw_live_append = false;
    while let Ok(ev) = rx.try_recv() {
        if let SessionEvent::OutputAppended { chunk, .. } = ev
            && chunk.text == "line-one\n"
        {
            saw_live_append = true;
        }
    }
    assert!(
        saw_live_append,
        "SessionEvent::OutputAppended must fire before 633;D"
    );

    // Feed split 4-byte emoji before 633;D arrives.
    let crab = "🦀".as_bytes();
    session.on_pty_bytes(&crab[..2]);
    session.on_pty_bytes(&crab[2..]);
    session.on_pty_bytes(b" step-two\n");
    assert_eq!(session.blocks()[0].status, BlockStatus::Running);
    assert_eq!(
        session.blocks()[0].plain_output(),
        "line-one\n🦀 step-two\n"
    );

    session.on_pty_bytes(b"\x1b]633;D;0\x07");
    assert_eq!(session.blocks()[0].status, BlockStatus::Ok);
    assert_eq!(session.blocks()[0].exit_code, Some(0));
}

#[test]
fn cr_progress_overwrite_and_csi_k_normalization_preserves_split_crlf() {
    let mut block = Block::new(BlockId(1), BlockKind::Shell, "cargo build");
    for chunk in [
        "Compiling 1/10\r",
        "Compiling 2/10\rCompiling 10/10\r",
        "\nDone\r",
        "\n",
    ] {
        block.push(OutputChunk::text(Stream::Stdout, chunk));
    }
    assert_eq!(block.plain_output(), "Compiling 10/10\nDone\n");

    // CSI K (erase to end of line) and CSI 2K (erase entire line) with ANSI SGR colors.
    let mut spinner_block = Block::new(BlockId(2), BlockKind::Shell, "download");
    spinner_block.push(OutputChunk::text(
        Stream::Stdout,
        "\x1b[33mDownloading [====    ] 50% very long suffix\x1b[0m\r\x1b[32mshort\x1b[0m\x1b[K\r\n",
    ));
    spinner_block.push(OutputChunk::text(
        Stream::Stdout,
        "Temporary spinner frame\r\x1b[2KFinal frame\r\nIn-flight 99%\r",
    ));
    assert_eq!(
        spinner_block.plain_output(),
        "short\nFinal frame\nIn-flight 99%"
    );

    assert_eq!(
        normalize_cr_overwrites("alpha-long\rbeta\x1b[0K\r\ngamma\r"),
        "beta\ngamma"
    );

    // Block::cancel and optional metadata serde defaults.
    spinner_block.cancel();
    assert_eq!(spinner_block.status, BlockStatus::Cancelled);
    assert_eq!(spinner_block.exit_code, Some(130));
    assert_eq!(spinner_block.cwd, None);
    assert_eq!(spinner_block.git_branch, None);
}

#[test]
fn vt_shadow_decset_decrst_ris_and_zero_dimension_clamping() {
    let mut shadow = VtShadow::new(0, 0);
    assert!(!shadow.is_alt_screen());
    shadow.resize(0, 0);
    assert_eq!(shadow.feed(b"normal output\r\n"), None);
    assert!(!shadow.is_alt_screen());

    // Split ?1049h across two chunks.
    assert_eq!(shadow.feed(b"\x1b[?10"), None);
    assert_eq!(shadow.feed(b"49h"), Some(true));
    assert!(shadow.is_alt_screen());
    assert_eq!(shadow.feed(b"still in alt screen"), None);

    // ?1049l exits alternate screen.
    assert_eq!(shadow.feed(b"\x1b[?1049l"), Some(false));
    assert!(!shadow.is_alt_screen());

    // Compound DECSET ?1049;2004h enters alt screen; RIS (\x1bc) resets to primary.
    assert_eq!(shadow.feed(b"\x1b[?1049;2004h"), Some(true));
    assert!(shadow.is_alt_screen());
    assert_eq!(shadow.feed(b"\x1bc"), Some(false));
    assert!(!shadow.is_alt_screen());

    // ?47h / ?47l and ?1047h / ?1047l transitions.
    assert_eq!(shadow.feed(b"\x1b[?47h"), Some(true));
    assert_eq!(shadow.feed(b"\x1b[?47l"), Some(false));
    assert_eq!(shadow.feed(b"\x1b[?1047h"), Some(true));
    assert_eq!(shadow.feed(b"\x1b[?1047l"), Some(false));
    assert!(!shadow.is_alt_screen());
}

#[test]
fn session_alt_screen_emits_events_suppresses_nested_block_splits_and_stamps_cwd() {
    let mut session = Session::with_size("alt-session", 0, 0);
    session.resize(0, 0);
    session.resize(120, 32);
    let mut rx = session.subscribe();

    session.on_pty_bytes(
        b"\x1b]7;file://localhost/workspace\x07\
          \x1b]633;P;GitBranch=main\x07\
          \x1b]633;A\x07$ \x1b]633;B\x07\
          \x1b]633;E;vim notes.md\x07\
          \x1b]633;C\x07",
    );
    assert_eq!(session.blocks().len(), 1);
    assert_eq!(session.blocks()[0].cwd.as_deref(), Some("/workspace"));
    assert_eq!(session.blocks()[0].git_branch.as_deref(), Some("main"));
    assert!(!session.is_alt_screen());

    // Enter alternate screen.
    session.on_pty_bytes(b"\x1b[?1049h");
    assert!(session.is_alt_screen());

    let mut saw_enter_alt = false;
    while let Ok(ev) = rx.try_recv() {
        if matches!(ev, SessionEvent::AltScreenChanged { active: true }) {
            saw_enter_alt = true;
        }
    }
    assert!(saw_enter_alt, "expected AltScreenChanged(true)");

    // Nested TUI sub-process emits OSC 633 markers while alt_screen is active.
    session.on_pty_bytes(
        b"\x1b]633;A\x07sub$ \x1b]633;B\x07\
          \x1b]633;E;nested-cmd\x07\
          \x1b]633;C\x07nested output\n\
          \x1b]633;D;42\x07",
    );
    assert_eq!(
        session.blocks().len(),
        1,
        "OSC 633 inside alt_screen must not split or open blocks"
    );
    assert_eq!(session.blocks()[0].input, "vim notes.md");
    assert_eq!(
        session.blocks()[0].status,
        BlockStatus::Running,
        "nested 633;D inside alt_screen must not close outer block"
    );

    // Exit alternate screen AND emit outer shell's 633;D;0 in the SAME chunk.
    session.on_pty_bytes(b"\x1b[?1049l\x1b]633;D;0\x07");
    assert!(!session.is_alt_screen());
    assert_eq!(session.blocks().len(), 1);
    assert_eq!(session.blocks()[0].status, BlockStatus::Ok);
    assert_eq!(session.blocks()[0].exit_code, Some(0));

    let mut saw_exit_alt = false;
    let mut saw_outer_closed = false;
    while let Ok(ev) = rx.try_recv() {
        match ev {
            SessionEvent::AltScreenChanged { active: false } => saw_exit_alt = true,
            SessionEvent::BlockClosed { id, .. } if id == BlockId(1) => saw_outer_closed = true,
            _ => {}
        }
    }
    assert!(saw_exit_alt, "expected AltScreenChanged(false)");
    assert!(saw_outer_closed, "expected outer BlockClosed after ?1049l");
}

#[test]
fn alt_screen_entry_exit_and_single_chunk_frames_never_leak_into_plain_output() {
    let mut session = Session::new("wr01-alt-leak");
    session.on_pty_bytes(
        b"\x1b]633;A\x07$ \x1b]633;B\x07\
          \x1b]633;E;run-tui\x07\
          \x1b]633;C\x07",
    );

    // 1. Entry chunk mixes primary text + ?1049h + initial full-screen TUI frame.
    session.on_pty_bytes(b"before-tui\n\x1b[?1049h\x1b[2J\x1b[HTUI_ENTRY_LEAK");
    assert!(session.is_alt_screen());

    // 2. Exit chunk mixes final TUI frame + ?1049l + post-TUI primary text.
    session.on_pty_bytes(b"TUI_EXIT_LEAK\x1b[?1049lafter-tui\n");
    assert!(!session.is_alt_screen());

    // 3. Single-chunk ?1049h ... TUI ... ?1049l round-trip followed by primary text.
    session
        .on_pty_bytes(b"\x1b[?1049hSINGLE_CHUNK_TUI_LEAK\x1b[?1049lfinal-line\n\x1b]633;D;0\x07");
    assert!(!session.is_alt_screen());

    assert_eq!(session.blocks().len(), 1);
    let plain = session.blocks()[0].plain_output();
    assert_eq!(
        plain, "before-tui\nafter-tui\nfinal-line\n",
        "TUI alt-screen frames must never leak into Block::plain_output(), got: {plain:?}"
    );
}

#[tokio::test]
async fn localhost_after_seq_delta_slicing_session_info_and_trait_methods() {
    let host = LocalHost::with_replay_cap(1024);
    let id = host.open_synthetic(120, 32);

    // Feed 5 gap-free frames including OSC 7 CWD, GitBranch, and ALT_SCREEN toggle.
    host.feed_raw_output(&id, b"\x1b]7;file://localhost/repo\x07")
        .unwrap(); // seq 1
    host.feed_raw_output(&id, b"\x1b]633;P;GitBranch=feat-441\x07")
        .unwrap(); // seq 2
    host.feed_raw_output(&id, b"frame-3\n").unwrap(); // seq 3
    host.feed_raw_output(&id, b"frame-4\n").unwrap(); // seq 4
    host.feed_raw_output(&id, b"\x1b[?1049hframe-5").unwrap(); // seq 5

    // Initial attach (after_seq = None): full snapshot + resync_required = true.
    let full = host.attach_since(&id, None).unwrap();
    assert!(full.resync_required);
    assert_eq!(full.seq, 5);
    assert!(full.alt_screen);
    assert_eq!(full.cwd.as_deref(), Some("/repo"));
    assert_eq!(full.git_branch.as_deref(), Some("feat-441"));
    assert!(
        full.replay
            .ends_with(b"frame-3\nframe-4\n\x1b[?1049hframe-5")
    );

    // Delta attach after seq 2: only frames 3..=5, resync_required = false.
    let delta = SessionHost::attach(&host, &id, Some(2)).await.unwrap();
    assert!(!delta.resync_required);
    assert_eq!(delta.seq, 5);
    assert_eq!(delta.replay, b"frame-3\nframe-4\n\x1b[?1049hframe-5");
    assert!(delta.alt_screen);

    // Caught-up attach at seq 5: empty replay, resync_required = false.
    let caught_up = host.attach_since(&id, Some(5)).unwrap();
    assert!(!caught_up.resync_required);
    assert_eq!(caught_up.seq, 5);
    assert!(caught_up.replay.is_empty());

    // SessionInfo carries Default + alt_screen + cwd + git_branch.
    let default_info = SessionInfo::default();
    assert!(!default_info.alt_screen);
    assert_eq!(default_info.cwd, None);
    let listed = SessionHost::list(&host).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert!(listed[0].alt_screen);
    assert_eq!(listed[0].cwd.as_deref(), Some("/repo"));
    assert_eq!(listed[0].git_branch.as_deref(), Some("feat-441"));

    // Synthetic session has no PTY so signal returns clean closed error via SessionHost trait.
    assert!(
        SessionHost::signal(&host, &id, TermSignal::Interrupt)
            .await
            .is_err()
    );
}

#[test]
fn localhost_replay_ring_eviction_and_oversized_frame_cap() {
    // 18-byte cap: each 6-byte frame ("seg-N\n") fits 3 frames max.
    let host = LocalHost::with_replay_cap(18);
    let id = host.open_synthetic(80, 24);

    host.feed_raw_output(&id, b"seg-1\n").unwrap(); // seq 1 (6B)
    host.feed_raw_output(&id, b"seg-2\n").unwrap(); // seq 2 (12B)
    host.feed_raw_output(&id, b"seg-3\n").unwrap(); // seq 3 (18B)
    host.feed_raw_output(&id, b"seg-4\n").unwrap(); // seq 4 -> evicts seq 1
    host.feed_raw_output(&id, b"seg-5\n").unwrap(); // seq 5 -> evicts seq 2 (retains 3, 4, 5 = 18B)

    // after_seq = Some(2): oldest retained is seq 3 (2 + 1 >= 3), so gap-free delta!
    let boundary_ok = host.attach_since(&id, Some(2)).unwrap();
    assert!(!boundary_ok.resync_required);
    assert_eq!(boundary_ok.seq, 5);
    assert_eq!(boundary_ok.replay, b"seg-3\nseg-4\nseg-5\n");

    // after_seq = Some(1): seq 2 was evicted (1 + 1 < 3), so resync_required = true + full retained window!
    let evicted = host.attach_since(&id, Some(1)).unwrap();
    assert!(evicted.resync_required);
    assert_eq!(evicted.seq, 5);
    assert_eq!(evicted.replay, b"seg-3\nseg-4\nseg-5\n");

    // Single oversized frame > replay_cap (26 bytes > 18 bytes) retains trailing 18 bytes within budget.
    host.feed_raw_output(&id, b"abcdefghijklmnopqrstuvwxyz")
        .unwrap(); // seq 6
    let after_oversized = host.attach_since(&id, Some(5)).unwrap();
    assert!(
        after_oversized.resync_required,
        "oversized frame dropped prefix bytes so after_seq=5 must resync"
    );
    assert_eq!(after_oversized.seq, 6);
    assert_eq!(after_oversized.replay.len(), 18);
    assert_eq!(after_oversized.replay, b"ijklmnopqrstuvwxyz");
}

#[test]
fn daemon_host_wire_routing_maps_alt_screen_cwd_and_signal_constant() {
    assert_eq!(daemon_host::SIGNAL, "term.signal");

    let (ev_tx, mut ev_rx) = broadcast::channel(16);
    let (out_tx, mut out_rx) = broadcast::channel(16);
    let mut last_seq = 0u64;

    assert!(daemon_host::route(
        &json!({"type": "alt_screen", "active": true}),
        &ev_tx,
        &out_tx,
        &mut last_seq,
    ));
    assert!(matches!(
        ev_rx.try_recv().unwrap(),
        SessionEvent::AltScreenChanged { active: true }
    ));

    assert!(daemon_host::route(
        &json!({"type": "cwd_changed", "cwd": "/tmp", "git_branch": "main"}),
        &ev_tx,
        &out_tx,
        &mut last_seq,
    ));
    match ev_rx.try_recv().unwrap() {
        SessionEvent::CwdChanged { cwd, git_branch } => {
            assert_eq!(cwd, "/tmp");
            assert_eq!(git_branch.as_deref(), Some("main"));
        }
        other => panic!("expected CwdChanged, got {other:?}"),
    }

    assert!(daemon_host::route(
        &json!({"type": "output", "seq": 42, "bytes_b64": B64.encode(b"delta")}),
        &ev_tx,
        &out_tx,
        &mut last_seq,
    ));
    assert_eq!(last_seq, 42);
    assert_eq!(
        out_rx.try_recv().unwrap(),
        OutputFrame {
            seq: 42,
            bytes: b"delta".to_vec(),
        }
    );
}
