//! Smoke tests for `vox share` CLI subcommand (S1 task 6).

use std::process::Command;

#[test]
fn share_help_lists_subcommand() {
    let output = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(["share", "--help"])
        .output()
        .expect("vox binary should be runnable");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("share") || stdout.contains("public") || stdout.contains("tunnel"),
        "help output should describe share: {}",
        stdout
    );
}

#[test]
fn share_help_lists_backend_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(["share", "--help"])
        .output()
        .expect("vox binary should be runnable");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--backend"),
        "help should include --backend flag: {}",
        stdout
    );
}
