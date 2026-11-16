//! Integration and deterministic shim suite for `TERM-PTY-01` and `TERM-PTY-02`.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use vox_terminal_core::block::BlockStatus;
use vox_terminal_core::host::{LocalHost, OpenSpec};
use vox_terminal_core::osc633::{Osc633Event, Osc633Parser};
use vox_terminal_core::pty::TermSignal;
use vox_terminal_core::shell_init::{
    ShellInitDirs, prepare_shell_command, prepare_shell_command_with_dirs,
};

#[cfg(unix)]
fn available_unix_shells() -> Vec<String> {
    let mut out = Vec::new();
    if Path::new("/bin/zsh").exists() {
        out.push("/bin/zsh".to_string());
    }
    if Path::new("/opt/homebrew/bin/bash").exists() {
        out.push("/opt/homebrew/bin/bash".to_string());
    } else if Path::new("/bin/bash").exists() {
        out.push("/bin/bash".to_string());
    }
    for opt in ["fish", "pwsh", "nu"] {
        if std::process::Command::new(opt)
            .arg("--version")
            .output()
            .is_ok()
        {
            out.push(opt.to_string());
        }
    }
    out
}

#[test]
fn five_shell_shims_openspec_defaults_and_osc_parser_bounds() {
    let spec = OpenSpec::default();
    assert_eq!(spec.cols, 120, "OpenSpec default cols must be 120");
    assert_eq!(spec.rows, 32, "OpenSpec default rows must be 32");

    let fake_home = tempfile::tempdir().expect("fake home");
    let fake_nu = tempfile::tempdir().expect("fake nu");
    for shell in ["zsh", "bash", "fish", "pwsh", "nu"] {
        let shim = prepare_shell_command_with_dirs(
            shell,
            Some(fake_home.path()),
            ShellInitDirs {
                home_dir: Some(fake_home.path()),
                nu_config_dir: Some(fake_nu.path()),
            },
        )
        .expect("shim builds");
        assert!(shim.osc633_capable);
        let dir_path = shim
            .shim_dir
            .as_ref()
            .expect("tempdir")
            .path()
            .to_path_buf();
        assert!(dir_path.exists());
        drop(shim);
        assert!(!dir_path.exists(), "RAII drop must unlink shim directory");
    }
    assert!(!prepare_shell_command("sh", None).unwrap().osc633_capable);

    // Verify Osc633Parser PromptEnd, Property, Osc7Cwd, and 16 KiB DoS cap (T-441-03)
    let mut p = Osc633Parser::new();
    let evs = p.feed(
        b"\x1b]7;file://localhost/tmp/my%20folder\x07\x1b]633;P;Cwd=/tmp/my folder\x07\x1b]633;P;GitBranch=feat-441\x07\x1b]633;A\x07$ \x1b]633;B\x07",
    );
    assert!(evs.contains(&Osc633Event::Osc7Cwd("/tmp/my folder".to_string())));
    assert!(evs.contains(&Osc633Event::Property {
        key: "Cwd".to_string(),
        value: "/tmp/my folder".to_string(),
    }));
    assert!(evs.contains(&Osc633Event::Property {
        key: "GitBranch".to_string(),
        value: "feat-441".to_string(),
    }));
    assert!(evs.contains(&Osc633Event::PromptStart));
    assert!(evs.contains(&Osc633Event::PromptEnd));

    // T-441-03: Malformed unterminated OSC > 16 KiB must flush as Output rather than hoarding
    let mut malformed = b"\x1b]633;E;".to_vec();
    malformed.extend(std::iter::repeat_n(b'A', 20 * 1024));
    let flushed = p.feed(&malformed);
    assert!(
        flushed
            .iter()
            .any(|e| matches!(e, Osc633Event::Output(s) if s.len() > 16 * 1024)),
        "unterminated >16KiB OSC sequence must flush safely as Output"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn startup_zero_echo_spam_empty_initial_blocks_and_initial_cwd() {
    let shells = available_unix_shells();
    assert!(!shells.is_empty(), "expected at least zsh or bash on Unix");

    for shell in shells {
        let host = LocalHost::default();
        let id = host
            .open(OpenSpec {
                cols: 120,
                rows: 32,
                shell: Some(shell.clone()),
            })
            .await
            .unwrap_or_else(|e| panic!("open {shell} failed: {e}"));

        let attached = host.attach(&id).expect("attach");
        assert!(
            attached.blocks.is_empty(),
            "{shell}: initial prompt must not create phantom Running blocks, got: {:?}",
            attached.blocks
        );

        let replay_str = String::from_utf8_lossy(&attached.replay);
        for leaked in [
            "__vox_precmd",
            "__vox_preexec",
            "add-zsh-hook",
            "__VOX_PREV_PROMPT_COMMAND",
            "\x1b[7m%\x1b[27m",
        ] {
            assert!(
                !replay_str.contains(leaked),
                "{shell}: startup replay leaked {leaked:?}: {replay_str:?}"
            );
        }

        let cwd = host.session_cwd(&id).expect("session_cwd");
        assert!(
            cwd.as_deref().is_some_and(|s| !s.is_empty()),
            "{shell}: expected initial CWD from OSC 7 / 633;P;Cwd, got {cwd:?}"
        );

        host.close(&id).expect("close");
    }
}

#[cfg(unix)]
#[tokio::test]
async fn nonzero_exit_codes_false_and_top_level_exit_7() {
    for shell in ["/bin/zsh", "bash"] {
        if shell.starts_with('/') && !Path::new(shell).exists() {
            continue;
        }
        let host = LocalHost::default();
        let id = host
            .open(OpenSpec {
                cols: 120,
                rows: 32,
                shell: Some(shell.to_string()),
            })
            .await
            .expect("open");

        let bid_false = host.submit(&id, "!false").expect("submit false");
        let b_false = host.block(&id, bid_false).expect("false block");
        assert_eq!(
            b_false.status,
            BlockStatus::Failed,
            "{shell}: `false` must mark block Failed"
        );
        assert_eq!(
            b_false.exit_code,
            Some(1),
            "{shell}: `false` must report exit_code 1"
        );
        host.close(&id).expect("close");

        // Top-level `exit 7` must emit 633;D;7 via EXIT trap and finish cleanly without hanging
        let host2 = LocalHost::default();
        let id2 = host2
            .open(OpenSpec {
                cols: 120,
                rows: 32,
                shell: Some(shell.to_string()),
            })
            .await
            .expect("open 2");
        let bid_exit = host2.submit(&id2, "!exit 7").expect("submit exit 7");
        let b_exit = host2.block(&id2, bid_exit).expect("exit 7 block");
        assert_eq!(
            b_exit.status,
            BlockStatus::Failed,
            "{shell}: `exit 7` must mark block Failed"
        );
        assert_eq!(
            b_exit.exit_code,
            Some(7),
            "{shell}: `exit 7` must report exit_code 7"
        );
        let _ = host2.close(&id2);
    }
}

#[cfg(unix)]
#[tokio::test]
async fn stateful_cd_and_export_persist_across_submissions_and_update_cwd() {
    let raw_tmp = tempfile::tempdir().expect("tempdir");
    let canonical = fs::canonicalize(raw_tmp.path()).expect("canonicalize tempdir");
    // Synthesize a lightweight .git layout inside the temp dir to verify 633;P;GitBranch updates
    fs::create_dir_all(canonical.join(".git/objects")).unwrap();
    fs::create_dir_all(canonical.join(".git/refs/heads")).unwrap();
    fs::write(
        canonical.join(".git/HEAD"),
        "ref: refs/heads/feat-stateful-441\n",
    )
    .unwrap();
    let canonical_str = canonical.to_string_lossy().into_owned();

    for shell in ["/bin/zsh", "bash"] {
        if shell.starts_with('/') && !Path::new(shell).exists() {
            continue;
        }
        let host = LocalHost::default();
        let id = host
            .open(OpenSpec {
                cols: 120,
                rows: 32,
                shell: Some(shell.to_string()),
            })
            .await
            .expect("open");

        let cd_cmd = format!("!cd '{}'", canonical_str);
        let cd_bid = host.submit(&id, &cd_cmd).expect("submit cd");
        assert_eq!(host.block(&id, cd_bid).unwrap().status, BlockStatus::Ok);

        let exp_bid = host
            .submit(&id, "!export VOX_STATE_441=persisted_ok")
            .expect("submit export");
        assert_eq!(host.block(&id, exp_bid).unwrap().status, BlockStatus::Ok);

        let check_bid = host
            .submit(&id, "!pwd && echo $VOX_STATE_441")
            .expect("submit pwd && echo");
        let check_block = host.block(&id, check_bid).expect("check block");
        assert_eq!(check_block.status, BlockStatus::Ok);
        let out = check_block.plain_output();
        assert!(
            out.contains(&canonical_str),
            "{shell}: expected persistent pwd {canonical_str:?} in {out:?}"
        );
        assert!(
            out.contains("persisted_ok"),
            "{shell}: expected persistent env var in {out:?}"
        );

        assert_eq!(
            host.session_cwd(&id).unwrap().as_deref(),
            Some(canonical_str.as_str()),
            "{shell}: session_cwd must reflect stateful cd"
        );
        assert_eq!(
            host.session_git_branch(&id).unwrap().as_deref(),
            Some("feat-stateful-441"),
            "{shell}: session_git_branch must reflect 633;P;GitBranch"
        );

        host.close(&id).expect("close");
    }
}

#[cfg(unix)]
#[tokio::test]
async fn interactive_stdin_forwards_into_active_running_block() {
    let shell = if Path::new("/bin/zsh").exists() {
        "/bin/zsh"
    } else {
        "bash"
    };
    let host = LocalHost::default();
    let id = host
        .open(OpenSpec {
            cols: 120,
            rows: 32,
            shell: Some(shell.to_string()),
        })
        .await
        .expect("open");

    let running_id = host
        .submit(&id, "!read VOX_ANS && echo got:$VOX_ANS")
        .expect("submit interactive read");
    let initial = host.block(&id, running_id).expect("running block exists");
    assert_eq!(
        initial.status,
        BlockStatus::Running,
        "interactive `read` must remain Running awaiting stdin"
    );

    // Subsequent submit while Running must forward into the active Running block
    let forwarded_id = host.submit(&id, "approved").expect("forward stdin");
    assert_eq!(
        forwarded_id, running_id,
        "must reuse the active Running BlockId rather than opening an overlapping block"
    );

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let b = host.block(&id, running_id).expect("block");
        if b.status != BlockStatus::Running {
            assert_eq!(b.status, BlockStatus::Ok);
            assert!(
                b.plain_output().contains("got:approved"),
                "expected interactive reply output, got {:?}",
                b.plain_output()
            );
            break;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for interactive read block to finish"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    assert_eq!(
        host.attach(&id).unwrap().blocks.len(),
        1,
        "must have exactly 1 block after interactive stdin completion"
    );
    host.close(&id).expect("close");
}

#[cfg(unix)]
#[tokio::test]
async fn live_kernel_resize_updates_stty_size_and_sigint_interrupts_foreground_job() {
    let shell = if Path::new("/bin/zsh").exists() {
        "/bin/zsh"
    } else {
        "bash"
    };
    let host = LocalHost::default();
    let id = host
        .open(OpenSpec {
            cols: 120,
            rows: 32,
            shell: Some(shell.to_string()),
        })
        .await
        .expect("open");

    host.resize(&id, 132, 43).expect("resize 132x43");
    let stty_bid = host.submit(&id, "!stty size").expect("submit stty size");
    let stty_out = host
        .block(&id, stty_bid)
        .expect("stty block")
        .plain_output();
    assert!(
        stty_out.contains("43 132"),
        "expected kernel PTY stty size `43 132`, got {stty_out:?}"
    );

    let sleep_bid = host.submit(&id, "!sleep 5").expect("submit sleep 5");
    assert_eq!(
        host.block(&id, sleep_bid).unwrap().status,
        BlockStatus::Running
    );

    host.signal(&id, TermSignal::Interrupt)
        .expect("send Interrupt ETX 0x03");

    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let b = host.block(&id, sleep_bid).unwrap();
        if b.status != BlockStatus::Running {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "sleep 5 block did not close after TermSignal::Interrupt"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    let alive_bid = host
        .submit(&id, "!echo alive-after-sigint")
        .expect("submit after sigint");
    let alive_block = host.block(&id, alive_bid).expect("alive block");
    assert_eq!(alive_block.status, BlockStatus::Ok);
    assert!(
        alive_block.plain_output().contains("alive-after-sigint"),
        "shell must remain alive after SIGINT, got {:?}",
        alive_block.plain_output()
    );

    host.close(&id).expect("close");
}
