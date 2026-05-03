//! Golden tests for JSX fragment `<>…</>` syntax in view blocks (Task 0.3).
//!
//! Each test compiles a `.vox` fixture and compares the emitted TSX with a
//! `.expected.tsx` snapshot.  Helpers are inlined (mirroring golden_for_loop_test.rs).

use std::collections::HashSet;

use vox_compiler::codegen_ts::reactive::{ReactiveViewBridgeStats, generate_reactive_component};
use vox_compiler::hir::lower::lower_module;
use vox_compiler::lexer::lex;
use vox_compiler::parser::parse;

/// Strip trailing whitespace from each line (normalizes emitter quirks).
fn normalize_ws(s: &str) -> String {
    s.lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

fn compile_components(src: &str) -> Vec<(String, String)> {
    let tokens = lex(src);
    let module = parse(tokens).expect("parse error");
    let hir = lower_module(&module);
    let island_names = HashSet::new();
    let mut stats = ReactiveViewBridgeStats::default();
    hir.components
        .iter()
        .map(|rc| generate_reactive_component(&hir, rc, &island_names, None, &mut stats))
        .collect()
}

fn get_component(files: &[(String, String)], name: &str) -> String {
    files
        .iter()
        .find(|(f, _)| *f == format!("{name}.tsx"))
        .map(|(_, c)| c.clone())
        .unwrap_or_else(|| panic!("component {name}.tsx not found"))
}

#[test]
fn fragment_pair_emits_fragment_wrapper() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/fragment/pair.vox"
    ))
    .unwrap();
    let expected = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/fragment/pair.expected.tsx"
    ))
    .unwrap();

    let files = compile_components(&src);
    let actual = get_component(&files, "Pair");

    assert_eq!(
        normalize_ws(&actual).trim().to_string(),
        normalize_ws(&expected).trim().to_string(),
        "Pair.tsx: fragment wrapper did not match golden snapshot\nACTUAL:\n{actual}"
    );
}

#[test]
fn fragment_empty_emits_empty_fragment() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/fragment/empty.vox"
    ))
    .unwrap();
    let expected = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/fragment/empty.expected.tsx"
    ))
    .unwrap();

    let files = compile_components(&src);
    let actual = get_component(&files, "EmptyFrag");

    assert_eq!(
        normalize_ws(&actual).trim().to_string(),
        normalize_ws(&expected).trim().to_string(),
        "EmptyFrag.tsx: empty fragment did not match golden snapshot\nACTUAL:\n{actual}"
    );
}

#[test]
fn fragment_nested_emits_nested_fragments() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/fragment/nested.vox"
    ))
    .unwrap();
    let expected = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/fragment/nested.expected.tsx"
    ))
    .unwrap();

    let files = compile_components(&src);
    let actual = get_component(&files, "Nested");

    assert_eq!(
        normalize_ws(&actual).trim().to_string(),
        normalize_ws(&expected).trim().to_string(),
        "Nested.tsx: nested fragments did not match golden snapshot\nACTUAL:\n{actual}"
    );
}
