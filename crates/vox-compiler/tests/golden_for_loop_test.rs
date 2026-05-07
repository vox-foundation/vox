//! Golden tests for `for x, i in arr { … }` loops in view blocks (Task 0.2).
//!
//! Each test compiles a `.vox` fixture and compares the emitted TSX with a
//! `.expected.tsx` snapshot.  Helpers are inlined (mirroring golden_svg_snake_case_test.rs).


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
    let mut stats = ReactiveViewBridgeStats::default();
    hir.components
        .iter()
        .map(|rc| generate_reactive_component(&hir, rc, None, &mut stats))
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
#[ignore = "VUV-9 retired JSX angle-bracket syntax; view-call coverage lives in reactive_smoke_test"]
fn for_loop_emits_array_map_with_index() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/for/runs_table.vox"
    ))
    .unwrap();
    let expected = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/for/runs_table.expected.tsx"
    ))
    .unwrap();

    let files = compile_components(&src);
    let actual = get_component(&files, "RunsTable");

    assert_eq!(
        normalize_ws(&actual).trim().to_string(),
        normalize_ws(&expected).trim().to_string(),
        "RunsTable.tsx: for-loop with index did not lower to .map()\nACTUAL:\n{actual}"
    );
}

#[test]
#[ignore = "VUV-9 retired JSX angle-bracket syntax; view-call coverage lives in reactive_smoke_test"]
fn for_loop_no_index_emits_underscore_i() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/for/for_no_index.vox"
    ))
    .unwrap();
    let expected = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/for/for_no_index.expected.tsx"
    ))
    .unwrap();

    let files = compile_components(&src);
    let actual = get_component(&files, "NoIndex");

    assert_eq!(
        normalize_ws(&actual).trim().to_string(),
        normalize_ws(&expected).trim().to_string(),
        "NoIndex.tsx: for-loop without index should use _i\nACTUAL:\n{actual}"
    );
}

#[test]
#[ignore = "VUV-9 retired JSX angle-bracket syntax; view-call coverage lives in reactive_smoke_test"]
fn for_loop_nested_emits_nested_maps() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/for/for_nested.vox"
    ))
    .unwrap();
    let expected = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/for/for_nested.expected.tsx"
    ))
    .unwrap();

    let files = compile_components(&src);
    let actual = get_component(&files, "Matrix");

    assert_eq!(
        normalize_ws(&actual).trim().to_string(),
        normalize_ws(&expected).trim().to_string(),
        "Matrix.tsx: nested for-loops did not lower to nested .map()\nACTUAL:\n{actual}"
    );
}

#[test]
#[ignore = "VUV-9 retired JSX angle-bracket syntax; view-call coverage lives in reactive_smoke_test"]
fn for_loop_minimal_body_compiles() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/for/for_minimal_body.vox"
    ))
    .unwrap();
    let expected = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/for/for_minimal_body.expected.tsx"
    ))
    .unwrap();

    let files = compile_components(&src);
    let actual = get_component(&files, "Empty");

    assert_eq!(
        normalize_ws(&actual).trim().to_string(),
        normalize_ws(&expected).trim().to_string(),
        "Empty.tsx: minimal-body for-loop did not match golden snapshot\nACTUAL:\n{actual}"
    );
}
