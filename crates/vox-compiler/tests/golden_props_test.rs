//! Golden tests for expression-valued JSX props (Task 0.4 + 0.4-review).
//!
//! Verifies that `{expr}` in attribute values — arithmetic, object literals,
//! variable refs, boolean expressions, subscripts, member access, negation,
//! conditionals, and function-ref handlers — all compile and emit correctly.
//!
//! Fixture files:
//! - `props/rich.vox`       — arithmetic and object-literal prop values
//! - `props/edge_cases.vox` — boolean expression (`is`), mixed string+expr props
//! - `props/subscript.vox`  — subscript expressions (`obj[idx]`)
//! - `props/coverage.vox`   — member access, negation, conditional, function-ref

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

/// Subscript expressions (`items[0]`, `items[i]`, `items[i + 1]`) in JSX children.
#[test]
fn subscript_props_emit() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/props/subscript.vox"
    ))
    .unwrap();
    let expected = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/props/subscript.expected.tsx"
    ))
    .unwrap();

    let files = compile_components(&src);
    let actual = get_component(&files, "Indexed");

    assert_eq!(
        normalize_ws(&actual).trim().to_string(),
        normalize_ws(&expected).trim().to_string(),
        "subscript props golden snapshot mismatch\nACTUAL:\n{actual}"
    );
}

// Known limitation (deferred): Closures with explicit parameters in JSX prop values
// emit with IIFE wrapping that discards the parameter. Example:
//
//   <Btn on:click={fn(e) { e.preventDefault() }}/>
//
// Currently emits roughly:
//   onClick={() => { ((e) => (() => { ... })())(); }}
//
// The inner (e) => ... lambda is immediately invoked with no arguments,
// so `e` is undefined inside the body. This is a Lambda-in-Block emit issue,
// not a prop-value parser issue. The dashboard surfaces (mesh, runs, models,
// etc.) all use no-arg arrow handlers (`onClick={() => onSelect(id)}`), so
// this gap doesn't block Phase 1+. Track as a follow-up before any surface
// that needs `e.target` or `e.preventDefault()` is built.

/// Coverage: member access, negation, conditional, function-ref.
#[test]
fn coverage_member_access() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/props/coverage.vox"
    ))
    .unwrap();
    let expected = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/props/coverage.expected.tsx"
    ))
    .unwrap();

    let files = compile_components(&src);
    let actual_ma = get_component(&files, "MemberAccess");
    let actual_neg = get_component(&files, "Negation");
    let actual_cond = get_component(&files, "Conditional");
    let actual_fn = get_component(&files, "FunctionRef");
    let actual = format!("{actual_ma}\n{actual_neg}\n{actual_cond}\n{actual_fn}");

    assert_eq!(
        normalize_ws(&actual).trim().to_string(),
        normalize_ws(&expected).trim().to_string(),
        "coverage props golden snapshot mismatch\nACTUAL:\n{actual}"
    );
}

/// Arithmetic (`count * 16`) and object-literal (`{{ width: width, padding: 8 }}`) props.
#[test]
fn rich_props_compile() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/props/rich.vox"
    ))
    .unwrap();
    let expected = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/props/rich.expected.tsx"
    ))
    .unwrap();

    let files = compile_components(&src);
    let actual_box = get_component(&files, "Box");
    let actual_page = get_component(&files, "Page");
    let actual = format!("{actual_box}\n{actual_page}");

    assert_eq!(
        normalize_ws(&actual).trim().to_string(),
        normalize_ws(&expected).trim().to_string(),
        "rich props golden snapshot mismatch\nACTUAL:\n{actual}"
    );
}

/// Boolean-expression props (`{i is stages}`), mixed string+expression props.
#[test]
fn edge_case_props_compile() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/props/edge_cases.vox"
    ))
    .unwrap();
    let expected = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/props/edge_cases.expected.tsx"
    ))
    .unwrap();

    let files = compile_components(&src);
    let actual_pipeline = get_component(&files, "PipelineCard");
    let actual_stages = get_component(&files, "Stages");
    let actual_mixed = get_component(&files, "MixedProps");
    let actual = format!("{actual_pipeline}\n{actual_stages}\n{actual_mixed}");

    assert_eq!(
        normalize_ws(&actual).trim().to_string(),
        normalize_ws(&expected).trim().to_string(),
        "edge-case props golden snapshot mismatch\nACTUAL:\n{actual}"
    );
}
