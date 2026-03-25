use super::parse_cargo_error;


// ── helpers ────────────────────────────────────────────────────────────────

/// Build a minimal rustc "error[Exxxx]" block as cargo emits it.
fn ec(code: &str, msg: &str, detail: &str) -> String {
    format!(
        "error[{code}]: {msg}\n  --> src/main.rs:3:5\n   |\n3  |     {detail}\n   |     ^^^^\n"
    )
}

fn cargo_build_failed_plain(msg: &str) -> String {
    format!("error: {msg}\n\nerror: could not compile `vox-script` due to previous error\n")
}

// ── summary extraction ─────────────────────────────────────────────────────

#[test]
fn summary_extracts_first_error_line_native() {
    let stderr = ec("E0308", "mismatched types", "42");
    let (summary, _) = parse_cargo_error(&stderr, false);
    assert!(summary.starts_with("error[E0308]"), "got: {summary}");
}

#[test]
fn summary_extracts_first_error_line_wasi() {
    let stderr = ec("E0433", "failed to resolve: use of undeclared crate", "foo");
    let (summary, _) = parse_cargo_error(&stderr, true);
    assert!(summary.starts_with("error[E0433]"), "got: {summary}");
}

#[test]
fn summary_fallback_native_when_no_error_line() {
    let (summary, _) = parse_cargo_error(
        "Compiling vox-script v0.1.0\nwarning: unused import\n",
        false,
    );
    assert_eq!(summary, "Compilation failed");
}

#[test]
fn summary_fallback_wasi_when_no_error_line() {
    let (summary, _) = parse_cargo_error(
        "Compiling vox-script v0.1.0\nwarning: unused import\n",
        true,
    );
    assert_eq!(summary, "WASI compilation failed");
}

// ── suggestion branches ────────────────────────────────────────────────────

#[test]
fn suggestion_wasm_target_not_found() {
    let stderr = "error[E0463]: can't find crate for `std`\n  = note: the `wasm32-wasip1` target may not be installed\nerror: target 'wasm32-wasip1' not found\n";
    let (_, suggestion) = parse_cargo_error(stderr, true);
    assert!(
        suggestion.contains("rustup target add wasm32-wasip1"),
        "got: {suggestion}"
    );
}

#[test]
fn suggestion_unknown_target_triple() {
    let stderr = "error: unknown target triple `wasm32-wasip1`\n";
    let (_, suggestion) = parse_cargo_error(stderr, false);
    assert!(
        suggestion.contains("rustup target add wasm32-wasip1"),
        "got: {suggestion}"
    );
}

#[test]
fn suggestion_import_error_e0433() {
    let stderr = ec(
        "E0433",
        "failed to resolve: use of undeclared crate or module `serde`",
        "serde",
    );
    let (_, suggestion) = parse_cargo_error(&stderr, false);
    assert!(
        suggestion.contains("dependency or crate name"),
        "got: {suggestion}"
    );
}

#[test]
fn suggestion_import_error_e0432() {
    let stderr = ec("E0432", "unresolved import `tokio::runtime`", "tokio");
    let (_, suggestion) = parse_cargo_error(&stderr, false);
    assert!(
        suggestion.contains("dependency or crate name"),
        "got: {suggestion}"
    );
}

#[test]
fn suggestion_type_mismatch_e0308() {
    let stderr = ec(
        "E0308",
        "mismatched types: expected `i32`, found `&str`",
        "\"hello\"",
    );
    let (_, suggestion) = parse_cargo_error(&stderr, false);
    assert!(suggestion.contains("Type mismatch"), "got: {suggestion}");
}

#[test]
fn suggestion_compile_error_macro_is_empty() {
    let stderr = "error: compile_error!(\"actors are not supported in WASI scripts\")\n --> src/main.rs:2:1\n";
    let (_, suggestion) = parse_cargo_error(stderr, true);
    assert!(suggestion.is_empty(), "got: {suggestion}");
}

#[test]
fn suggestion_cargo_file_lock() {
    let stderr = "Blocking waiting for file lock on build directory\n";
    let (_, suggestion) = parse_cargo_error(stderr, false);
    assert!(
        suggestion.to_lowercase().contains("wait") || suggestion.contains("no-cache"),
        "got: {suggestion}"
    );
}

#[test]
fn suggestion_wasi_generic_fallback_when_no_other_match() {
    let stderr = cargo_build_failed_plain("aborting due to previous error");
    let (_, suggestion) = parse_cargo_error(&stderr, true);
    assert!(
        suggestion.contains("WASI scripts cannot use actors"),
        "got: {suggestion}"
    );
}

#[test]
fn suggestion_empty_for_plain_native_error() {
    let stderr = cargo_build_failed_plain("aborting due to previous error");
    let (_, suggestion) = parse_cargo_error(&stderr, false);
    assert!(suggestion.is_empty(), "got: {suggestion}");
}

// ── priority: most-specific rule wins ─────────────────────────────────────

#[test]
fn target_not_found_takes_priority_over_wasi_generic() {
    let stderr = "error: target 'wasm32-wasip1' not found\nerror: could not compile\n";
    let (_, suggestion) = parse_cargo_error(stderr, true);
    assert!(
        suggestion.contains("rustup target add"),
        "got: {suggestion}"
    );
    assert!(
        !suggestion.contains("actors"),
        "WASI generic should NOT fire: {suggestion}"
    );
}

#[test]
fn compile_error_macro_takes_priority_over_wasi_generic() {
    let stderr = "error: compile_error!(\"async fn main is not supported in WASI scripts\")\n --> src/main.rs:1:1\n";
    let (_, suggestion) = parse_cargo_error(stderr, true);
    assert!(suggestion.is_empty(), "got: {suggestion}");
}

// ── real-world cargo stderr sample ────────────────────────────────────────

#[test]
fn wasm_target_missing_sample() {
    let stderr = r#"error: target 'wasm32-wasip1' not found in channel
  |
  = help: run `rustup target add wasm32-wasip1`
"#;
    let (_, suggestion) = parse_cargo_error(stderr, true);
    assert!(suggestion.contains("rustup target add wasm32-wasip1"));
}

#[test]
fn wasi_generic_fallback_sample() {
    let stderr = "error: some weird wasm error\nerror: could not compile `vox-script`";
    let (summary, suggestion) = parse_cargo_error(stderr, true);
    assert_eq!(summary, "error: some weird wasm error");
    assert!(suggestion.contains("WASI scripts cannot use actors"));
}

#[test]
fn file_lock_blocking_sample() {
    let stderr = "    Blocking waiting for file lock on build directory";
    let (_, suggestion) = parse_cargo_error(stderr, false);
    assert!(suggestion.contains("Another `vox run` is compiling"));
}

#[test]
fn compile_error_guardrail_sample() {
    let stderr = r#"error: custom error from compile_error!
  --> src/main.rs:2:1
   |
2  | compile_error!("Actors are not supported in WASI");
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
"#;
    let (_, suggestion) = parse_cargo_error(stderr, true);
    assert!(suggestion.is_empty());
}

#[test]
fn real_world_e0308_sample() {
    let stderr = r#"error[E0308]: mismatched types
  --> src/main.rs:5:5
   |
5  |     42.0
   |     ^^^^ expected `i32`, found `f64`

For more information about this error, try `rustc --explain E0308`.
error: could not compile `vox-script` (bin "vox-script") due to 1 previous error
"#;
    let (summary, suggestion) = parse_cargo_error(stderr, false);
    assert!(summary.starts_with("error[E0308]"), "summary: {summary}");
    assert!(
        suggestion.contains("Type mismatch"),
        "suggestion: {suggestion}"
    );
}

#[test]
fn real_world_e0433_sample() {
    let stderr = r#"error[E0433]: failed to resolve: use of undeclared crate or module `uuid`
  --> src/main.rs:1:5
   |
1  |     uuid::Uuid::new_v4()
   |     ^^^^ use of undeclared crate or module `uuid`

For more information about this error, try `rustc --explain E0433`.
error: could not compile `vox-script` due to 1 previous error
"#;
    let (summary, suggestion) = parse_cargo_error(stderr, false);
    assert!(summary.starts_with("error[E0433]"), "summary: {summary}");
    assert!(
        suggestion.contains("dependency or crate name"),
        "suggestion: {suggestion}"
    );
}
}
