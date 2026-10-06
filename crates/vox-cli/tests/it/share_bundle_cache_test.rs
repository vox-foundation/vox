// crates/vox-cli/tests/share_bundle_cache_test.rs
//! Integration-style tests for `vox share` bundle/dev CLI surface (S8).

use std::process::Command;

/// FILE positional arg should appear in `vox share --help`.
#[test]
fn share_help_mentions_file_arg() {
    let output = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(["share", "--help"])
        .output()
        .expect("vox binary should be runnable");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("FILE") || stdout.contains("file"),
        "share --help should mention FILE argument: {}",
        stdout
    );
}

/// `--dev` flag should appear in `vox share --help`.
#[test]
fn share_help_mentions_dev_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(["share", "--help"])
        .output()
        .expect("vox binary should be runnable");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--dev"),
        "share --help should mention --dev flag: {}",
        stdout
    );
}
