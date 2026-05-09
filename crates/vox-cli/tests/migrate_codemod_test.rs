//! Tests for the `vox migrate names` subcommand (VUV-9 Task 5).

use std::process::Command;

#[test]
fn migrate_help_lists_subcommand() {
    let output = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(["migrate", "--help"])
        .output()
        .expect("vox binary should be runnable");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("migrate") || stdout.contains("Migrate"),
        "help output should reference the migrate subcommand: {}",
        stdout
    );
    assert!(
        stdout.contains("rewrite")
            || stdout.contains("Rewrite")
            || stdout.contains("registry")
            || stdout.contains("rename")
            || stdout.contains("canonical"),
        "help output should describe what migrate does: {}",
        stdout
    );
}

#[test]
fn migrate_names_help_shows_dry_run() {
    let output = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(["migrate", "names", "--help"])
        .output()
        .expect("vox binary should be runnable");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("dry-run") || stdout.contains("dry_run"),
        "names --help should mention --dry-run: {}",
        stdout
    );
}

#[test]
fn migrate_names_dry_run_empty_dir() {
    use std::fs;
    let dir = tempfile::tempdir().expect("tempdir");

    let output = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(["migrate", "names", "--dry-run", dir.path().to_str().unwrap()])
        .output()
        .expect("vox binary should be runnable");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "migrate names --dry-run should succeed on empty dir: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("0 file(s)"),
        "should report 0 files updated on empty dir: {}",
        stdout
    );

    // cleanup
    drop(dir);
    let _ = fs::remove_dir_all(std::env::temp_dir().join("migrate_names_test"));
}
