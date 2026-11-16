use super::warp::{parse_control_mode, parse_security_level, promote_bg_on_host};
use super::*;
use vox_terminal_core::pty::shell_integration_snippet;
use vox_terminal_core::{TermControlMode, TermSecurityLevel};

#[test]
fn default_shell_is_nonempty() {
    assert!(!default_shell().is_empty());
}

#[test]
fn manager_starts_empty() {
    let m = PtyManager::default();
    assert_eq!(m.count(), 0);
    assert!(!m.has("tab-1"));
}

#[test]
fn integration_snippet_supports_pwsh_and_bash() {
    assert!(shell_integration_snippet("pwsh").is_some());
    assert!(shell_integration_snippet("powershell.exe").is_some());
    // vox-arch-check: allow abs-path
    assert!(shell_integration_snippet("/usr/bin/bash").is_some());
}

#[test]
fn integration_snippet_skips_unknown_shells() {
    // zsh/fish gained OSC 633 integration in vox-terminal-core's Track 6
    // (Nushell effort) — see shell_integration_snippet's own doc comment
    // and test module in crates/vox-terminal-core/src/pty.rs. cmd.exe has
    // no OSC 633 support and stays unsupported.
    assert!(shell_integration_snippet("cmd.exe").is_none());
}

#[test]
fn integration_snippet_supports_zsh_and_fish() {
    assert!(shell_integration_snippet("zsh").is_some());
    assert!(shell_integration_snippet("fish").is_some());
}

#[test]
fn snippets_emit_633_markers() {
    assert!(shell_integration_snippet("pwsh").unwrap().contains("]633;"));
    assert!(shell_integration_snippet("bash").unwrap().contains("]633;"));
}

#[test]
fn pwsh_snippet_uses_real_last_exit_code() {
    let s = shell_integration_snippet("pwsh").unwrap();
    assert!(s.contains("$LASTEXITCODE"), "must read the real exit code");
    assert!(!s.contains("__VoxLastExit"), "stale exit var removed");
}

#[test]
fn bash_snippet_preserves_user_hooks() {
    let s = shell_integration_snippet("bash").unwrap();
    assert!(
        s.contains("__VOX_PREV_PROMPT_COMMAND"),
        "chains PROMPT_COMMAND"
    );
    assert!(s.contains("trap -p DEBUG"), "preserves the DEBUG trap");
}

#[test]
fn uppercase_exe_suffix_is_stripped() {
    assert!(shell_integration_snippet("PowerShell.EXE").is_some());
}

#[tokio::test]
async fn local_fallback_session_host_lifecycle() {
    let m = PtyManager::default();
    let host = m.resolve_local_host();
    let sid = host
        .open(OpenSpec {
            cols: 80,
            rows: 24,
            shell: None,
        })
        .await
        .unwrap();
    let listed = host.list().await.unwrap();
    assert!(listed.iter().any(|s| s.id == sid));

    let initial = host.attach(&sid, None).await.unwrap();
    assert!(initial.resync_required);
    let initial_seq = initial.seq;
    let initial_dto = term_attachment_dto_from_attachment(&sid, &initial);
    assert!(initial_dto.resync_required);
    assert_eq!(initial_dto.session_id, sid);

    host.submit(&sid, "!echo gui-delta-441").await.unwrap();
    let delta = host.attach(&sid, Some(initial_seq)).await.unwrap();
    assert!(!delta.resync_required);
    let delta_dto = term_attachment_dto_from_attachment(&sid, &delta);
    assert!(!delta_dto.resync_required);
    assert!(
        delta_dto.replay_utf8.contains("gui-delta-441")
            || delta
                .blocks
                .iter()
                .any(|b| b.plain_output().contains("gui-delta-441")),
        "expected post-seq delta output, got replay={:?}, blocks={:?}",
        delta_dto.replay_utf8,
        delta_dto.blocks
    );

    let dto = term_attachment_dto_from_parts(&sid, vec![], b"hello", 7);
    assert_eq!(dto.session_id, sid);
    assert_eq!(dto.replay_utf8, "hello");
    assert_eq!(dto.seq, 7);
    assert!(dto.resync_required);
    assert!(!dto.alt_screen);
    assert_eq!(dto.cwd, None);
    assert_eq!(dto.git_branch, None);

    host.close(&sid).await.unwrap();
}

#[test]
fn split_utf8_chunks_and_attachment_metadata_roundtrip_losslessly() {
    let mut decoder = vox_terminal_core::Utf8StreamDecoder::new();
    let first = decode_stream_chunk(&mut decoder, &[0xF0, 0x9F]);
    assert!(first.is_empty(), "half-emoji must buffer without U+FFFD");
    let second = decode_stream_chunk(&mut decoder, &[0x9A, 0x80]);
    assert_eq!(second, "🚀");
    assert!(!second.contains('\u{FFFD}'));
    assert!(decoder.flush_lossy().is_empty());

    let local = LocalHost::default();
    let sid = local.open_synthetic(120, 32);
    local
        .feed_raw_output(
            &sid,
            b"\x1b]633;P;Cwd=/tmp/vox-441\x07\x1b]633;P;GitBranch=feat/441\x07\x1b[?1049h\xf0\x9f\x9a\x80",
        )
        .unwrap();
    let att = local.attach_since(&sid, None).unwrap();
    let dto = term_attachment_dto_from_attachment(&sid, &att);
    assert!(dto.resync_required);
    assert!(dto.alt_screen);
    // vox-arch-check: allow abs-path
    assert_eq!(dto.cwd.as_deref(), Some("/tmp/vox-441"));
    assert_eq!(dto.git_branch.as_deref(), Some("feat/441"));
    assert!(dto.replay_utf8.contains("🚀"));
    assert!(!dto.replay_utf8.contains('\u{FFFD}'));
}

#[tokio::test]
async fn attach_replay_to_live_seam_carries_split_utf8_without_replacement_chars() {
    let local = LocalHost::default();
    let sid = local.open_synthetic(120, 32);
    local
        .feed_raw_output(&sid, b"prefix-\xf0\x9f")
        .expect("feed prefix + half emoji");

    let mut att = local.attach_since(&sid, None).expect("attach at seam");
    let (dto, mut seeded_decoder) = term_attachment_dto_and_decoder(&sid, &att);
    assert_eq!(dto.seq, 1);
    assert_eq!(
        dto.replay_utf8, "prefix-",
        "incomplete trailing UTF-8 at attach seam must stay buffered instead of flushing U+FFFD"
    );
    assert!(!dto.replay_utf8.contains('\u{FFFD}'));

    local
        .feed_raw_output(&sid, b"\xa6\x80-suffix")
        .expect("feed remaining emoji bytes");
    let live_frame = att.output.recv().await.expect("live frame seq 2");
    assert_eq!(live_frame.seq, 2);
    let live_utf8 = decode_stream_chunk(&mut seeded_decoder, &live_frame.bytes);
    assert_eq!(live_utf8, "🦀-suffix");
    assert!(!live_utf8.contains('\u{FFFD}'));
}

#[test]
fn parse_security_level_and_control_mode_accept_canonical_and_reject_invalid() {
    for (raw, expected) in [
        ("l1_read_only", TermSecurityLevel::L1ReadOnly),
        ("1", TermSecurityLevel::L1ReadOnly),
        ("L1", TermSecurityLevel::L1ReadOnly),
        ("read_only", TermSecurityLevel::L1ReadOnly),
        ("l2_workspace_build", TermSecurityLevel::L2WorkspaceBuild),
        ("2", TermSecurityLevel::L2WorkspaceBuild),
        ("L2", TermSecurityLevel::L2WorkspaceBuild),
        ("workspace_build", TermSecurityLevel::L2WorkspaceBuild),
        ("safe_exec", TermSecurityLevel::L2WorkspaceBuild),
        (
            "l3_structured_default",
            TermSecurityLevel::L3StructuredDefault,
        ),
        ("3", TermSecurityLevel::L3StructuredDefault),
        ("L3", TermSecurityLevel::L3StructuredDefault),
        ("structured_default", TermSecurityLevel::L3StructuredDefault),
        ("workspace_write", TermSecurityLevel::L3StructuredDefault),
        ("l4_host_gated", TermSecurityLevel::L4HostGated),
        ("4", TermSecurityLevel::L4HostGated),
        ("L4", TermSecurityLevel::L4HostGated),
        ("host_gated", TermSecurityLevel::L4HostGated),
        ("system_exec", TermSecurityLevel::L4HostGated),
        ("l5_autopilot_floor", TermSecurityLevel::L5AutopilotFloor),
        ("5", TermSecurityLevel::L5AutopilotFloor),
        ("L5", TermSecurityLevel::L5AutopilotFloor),
        ("autopilot_floor", TermSecurityLevel::L5AutopilotFloor),
    ] {
        assert_eq!(parse_security_level(raw).unwrap(), expected, "raw={raw}");
    }
    for bad in ["", "   ", "0", "6", "L6", "god_mode"] {
        assert!(
            parse_security_level(bad).is_err(),
            "expected Err for {bad:?}"
        );
    }

    for (raw, expected) in [
        ("manual", TermControlMode::Manual),
        ("Manual", TermControlMode::Manual),
        ("co_pilot", TermControlMode::CoPilot),
        ("copilot", TermControlMode::CoPilot),
        ("autopilot", TermControlMode::Autopilot),
    ] {
        assert_eq!(parse_control_mode(raw).unwrap(), expected, "raw={raw}");
    }
    for bad in ["", "   ", "semi_auto", "unknown"] {
        assert!(parse_control_mode(bad).is_err(), "expected Err for {bad:?}");
    }
}

#[tokio::test]
async fn security_control_pin_and_promote_bg_roundtrip_on_local_host() {
    let m = PtyManager::default();
    let sid = m.local_host.open_synthetic(100, 30);
    let host = m.resolve_local_host();

    host.set_security_level(&sid, parse_security_level("l4_host_gated").unwrap())
        .await
        .expect("set_security_level");
    host.set_control_mode(&sid, parse_control_mode("autopilot").unwrap())
        .await
        .expect("set_control_mode");

    let listed = host.list().await.expect("list");
    let info = listed.iter().find(|s| s.id == sid).expect("session listed");
    assert_eq!(info.security_level, TermSecurityLevel::L4HostGated);
    assert_eq!(info.control_mode, TermControlMode::Autopilot);

    assert!(
        !promote_bg_on_host(host.as_ref(), &sid, None)
            .await
            .expect("idle promote_bg")
    );

    let completed_id = host.submit(&sid, "!echo pin-me").await.expect("submit");
    host.pin_block(&sid, completed_id, true)
        .await
        .expect("pin true");
    let att_pinned = host.attach(&sid, Some(u64::MAX)).await.expect("attach");
    assert!(
        att_pinned
            .blocks
            .iter()
            .any(|b| b.id == completed_id && b.pinned)
    );
    host.pin_block(&sid, completed_id, false)
        .await
        .expect("pin false");
    let att_unpinned = host.attach(&sid, Some(u64::MAX)).await.expect("attach");
    assert!(
        att_unpinned
            .blocks
            .iter()
            .any(|b| b.id == completed_id && !b.pinned)
    );

    m.local_host
        .feed_raw_output(&sid, b"\x1b]633;E;sleep 10\x07\x1b]633;C\x07working...")
        .expect("open running block");
    assert!(
        promote_bg_on_host(host.as_ref(), &sid, None)
            .await
            .expect("active promote_bg")
    );

    assert!(
        promote_bg_on_host(host.as_ref(), "no-such-session", None)
            .await
            .is_err()
    );
}

#[test]
fn terminal_llm_config_resolution_and_clavis_slugs() {
    use super::agent::{CLAVIS_UNCONFIGURED_DIAG, resolve_terminal_llm_config_with};
    use vox_orchestrator_mcp::gui_ops::dto::models::ProviderStatusDto;
    use vox_secrets::SecretId;

    let none_sec = |_: SecretId| None;
    let err = resolve_terminal_llm_config_with(None, None, &[], none_sec, "fix build").unwrap_err();
    assert_eq!(err, CLAVIS_UNCONFIGURED_DIAG);
    assert!(err.contains("[clavis-unconfigured]"));

    let mens = resolve_terminal_llm_config_with(None, Some(11435), &[], none_sec, "fix").unwrap();
    assert_eq!(mens.provider, "ollama");
    assert_eq!(mens.model, "mens-local");
    assert_eq!(
        mens.base_url.as_deref(),
        Some("http://127.0.0.1:11435/v1/chat/completions")
    );

    let local_sec = resolve_terminal_llm_config_with(
        Some("qwen2.5-coder"),
        None,
        &[],
        |id| (id == SecretId::OllamaUrl).then(|| "http://127.0.0.1:11434".to_string()),
        "fix",
    )
    .unwrap();
    assert_eq!(local_sec.provider, "ollama");
    assert_eq!(local_sec.model, "qwen2.5-coder");
    assert_eq!(
        local_sec.base_url.as_deref(),
        Some("http://127.0.0.1:11434/v1/chat/completions")
    );

    let st = [ProviderStatusDto {
        provider: "VoxLocal".into(),
        key_present: false,
        is_local: true,
        local_reachable: Some(true),
        local_models: vec!["mens-7b".into()],
    }];
    let probed = resolve_terminal_llm_config_with(None, None, &st, none_sec, "fix").unwrap();
    assert_eq!(probed.provider, "voxlocal");
    assert_eq!(probed.model, "mens-7b");

    let gemini = resolve_terminal_llm_config_with(
        None,
        None,
        &[],
        |id| (id == SecretId::GeminiApiKey).then(|| "gem-test-key".to_string()),
        "explain error",
    )
    .unwrap();
    assert_eq!(gemini.provider, "gemini", "never use 'google' slug");
    assert_eq!(gemini.api_key.as_deref(), Some("gem-test-key"));

    for (model, want_prov, key_id) in [
        ("claude-sonnet-4-5", "anthropic", SecretId::AnthropicApiKey),
        (
            "anthropic/claude-sonnet-4-5",
            "anthropic",
            SecretId::AnthropicApiKey,
        ),
        ("gemini/gemini-2.5-flash", "gemini", SecretId::GeminiApiKey),
        ("gpt-4o", "openai", SecretId::OpenaiApiKey),
        ("o3-mini", "openai", SecretId::OpenaiApiKey),
        ("openrouter/auto", "openrouter", SecretId::OpenRouterApiKey),
        ("groq/llama-3.3-70b-versatile", "groq", SecretId::GroqApiKey),
    ] {
        let cfg = resolve_terminal_llm_config_with(
            Some(model),
            Some(11435),
            &st,
            |id| (id == key_id).then(|| "cloud-key".to_string()),
            "fix",
        )
        .unwrap();
        assert_eq!(cfg.provider, want_prov, "cloud {model} must beat local");
        assert_eq!(cfg.model, model);
        assert_eq!(cfg.api_key.as_deref(), Some("cloud-key"));
        assert!(cfg.base_url.is_none());
    }
    let or_fb = resolve_terminal_llm_config_with(
        Some("anthropic/claude-sonnet-4-5"),
        Some(11435),
        &st,
        |id| (id == SecretId::OpenRouterApiKey).then(|| "or-key".into()),
        "fix",
    )
    .unwrap();
    assert_eq!(or_fb.provider, "openrouter");
}

#[test]
fn explain_fix_scrubs_caps_and_refuses_credential_blocks() {
    use super::agent::{
        build_explain_fix_messages, extract_proposed_command, take_safe_stream_flush,
    };
    use vox_terminal_core::{Block, BlockId, BlockKind, OutputChunk, Stream};

    let mut cred = Block::new(BlockId(1), BlockKind::Shell, "sudo su");
    cred.credential_prompt = true;
    assert!(build_explain_fix_messages("explain", &cred).is_err());

    let tok = ["ghp", "_1234567890abcdefghijklmnopqrstuvwx"].concat();
    let mut blk = Block::new(BlockId(2), BlockKind::Shell, format!("curl -H {tok}"));
    blk.exit_code = Some(22);
    blk.push(OutputChunk::text(
        Stream::Stdout,
        format!(
            "\x1b[31m{tok}\x1b[0m {} tail_marker: error[E0382]: borrow",
            "a".repeat(12_000)
        ),
    ));
    blk.push(OutputChunk::text(
        Stream::Stderr,
        format!("err {tok} {} stderr_tail_marker", "b".repeat(26_000)),
    ));
    let msgs = build_explain_fix_messages("fix", &blk).expect("scrubbed explain/fix");
    assert_eq!(msgs.len(), 2);
    assert!(!msgs[1].content.contains(&tok));
    assert!(!msgs[1].content.contains("\x1b[31m"));
    assert!(msgs[1].content.contains("...[truncated]..."));
    assert!(
        msgs[1]
            .content
            .contains("tail_marker: error[E0382]: borrow")
    );
    assert!(msgs[1].content.contains("stderr_tail_marker"));
    assert!(msgs[1].content.len() <= 17_000);

    let mut buf = format!(
        "{} Authorization: Bearer sk-proj-0123456789",
        "x ".repeat(128)
    );
    let flushed = take_safe_stream_flush(&mut buf).expect("whitespace flush");
    assert!(!flushed.contains("Bearer") && buf.starts_with("Bearer sk-proj-"));
    let mut nl_buf = "line1\npartial_secret".to_string();
    assert_eq!(
        take_safe_stream_flush(&mut nl_buf).as_deref(),
        Some("line1\n")
    );
    assert_eq!(nl_buf, "partial_secret");

    assert_eq!(
        extract_proposed_command("Run:\n```sh\n$ cargo check -p vox-gui\n```"),
        Some("cargo check -p vox-gui".to_string())
    );
    assert_eq!(
        extract_proposed_command("VOX_PROPOSED_COMMAND: $ vox doctor"),
        Some("vox doctor".to_string())
    );
}
