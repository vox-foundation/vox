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
