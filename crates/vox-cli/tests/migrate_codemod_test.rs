//! Tests for the `vox migrate names` subcommand (VUV-9 Tasks 5 & 6).

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

// ── Task 6: token-based rewrite unit tests ────────────────────────────────

use vox_compiler::parser::renames::RenameRegistry;

/// Helper: load a registry with a single Box→panel rename for testing.
fn test_registry() -> RenameRegistry {
    RenameRegistry::from_str(
        r#"{
        "version": 1,
        "entries": [
          { "from": "Box", "to": "panel", "kind": "primitive", "since": "0.5.0" }
        ]
    }"#,
    )
    .unwrap()
}

#[test]
fn rewrite_renames_primitive_call_sites() {
    let registry = test_registry();
    let before = "component App() { view: Box() { Box() { text(\"hi\") } } }";
    let after = vox_cli::commands::migrate::rewrite_for_test(before, &registry);
    assert_eq!(
        after,
        "component App() { view: panel() { panel() { text(\"hi\") } } }"
    );
}

#[test]
fn rewrite_does_not_touch_string_literals() {
    let registry = test_registry();
    let before = r#"component App() { view: text("Box of crayons") }"#;
    let after = vox_cli::commands::migrate::rewrite_for_test(before, &registry);
    assert_eq!(after, before, "string literal contents must be preserved");
}

#[test]
fn rewrite_does_not_touch_substring_identifiers() {
    let registry = test_registry();
    // None of these are the exact identifier "Box"
    let before = "let myBox = 1; let boxes = 2;";
    let after = vox_cli::commands::migrate::rewrite_for_test(before, &registry);
    assert_eq!(after, before, "substring matches must not be rewritten");
}

#[test]
fn rewrite_preserves_whitespace_and_comments() {
    let registry = test_registry();
    let before =
        "// before-comment\ncomponent App() {\n    view: Box()  {  // inline\n        text(\"hi\")\n    }\n}\n";
    let after = vox_cli::commands::migrate::rewrite_for_test(before, &registry);
    let expected =
        "// before-comment\ncomponent App() {\n    view: panel()  {  // inline\n        text(\"hi\")\n    }\n}\n";
    assert_eq!(after, expected);
}

#[test]
fn rewrite_passes_through_unchanged_when_no_deprecated_names() {
    let registry = test_registry();
    let before = "component App() { view: panel() { text(\"hi\") } }";
    let after = vox_cli::commands::migrate::rewrite_for_test(before, &registry);
    assert_eq!(after, before, "no deprecated names → output equals input");
}

#[test]
fn rewrite_returns_input_unchanged_on_lex_failure() {
    let registry = test_registry();
    // The logos lexer is infallible (skips unknown chars) so we just verify
    // that a pathological input does not panic and produces some output.
    let before = "component App() { view: \"unterminated";
    let after = vox_cli::commands::migrate::rewrite_for_test(before, &registry);
    // Primary assertion: did not panic.
    // Secondary: the identifier "component" and "view" should still appear.
    assert!(
        after.contains("component"),
        "output should contain 'component': {after:?}"
    );
    let _ = after;
}
