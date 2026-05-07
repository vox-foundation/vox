//! Golden tests for SVG snake_case attribute and tag aliasing (Task 0.1).
//!
//! Vox source uses snake_case SVG attrs (view_box, stroke_width, etc.) and
//! tag names (radial_gradient, etc.); the compiler lowers them to React-required
//! camelCase. Back-compat with existing camelCase usage is preserved.


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
fn svg_snake_case_attrs_lower_to_camel() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/svg/snake_case.vox"
    ))
    .unwrap();
    let expected_play = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/svg/play_icon.expected.tsx"
    ))
    .unwrap();
    let expected_halo = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/svg/halo.expected.tsx"
    ))
    .unwrap();

    let files = compile_components(&src);

    let actual_play = get_component(&files, "PlayIcon");
    assert_eq!(
        normalize_ws(&actual_play).trim().to_string(),
        normalize_ws(&expected_play).trim().to_string(),
        "PlayIcon.tsx: snake_case attrs did not lower to camelCase"
    );

    let actual_halo = get_component(&files, "Halo");
    assert_eq!(
        normalize_ws(&actual_halo).trim().to_string(),
        normalize_ws(&expected_halo).trim().to_string(),
        "Halo.tsx: snake_case attrs/tags did not lower to camelCase"
    );
}

#[test]
#[ignore = "VUV-9 retired JSX angle-bracket syntax; view-call coverage lives in reactive_smoke_test"]
fn svg_remaining_tag_aliases_lower() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/svg/uncovered_tags.vox"
    ))
    .unwrap();
    let expected = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/svg/uncovered_tags.expected.tsx"
    ))
    .unwrap();

    let files = compile_components(&src);
    let actual = get_component(&files, "Filtered");
    assert_eq!(
        normalize_ws(&actual).trim().to_string(),
        normalize_ws(&expected).trim().to_string(),
        "Filtered.tsx: linearGradient/feGaussianBlur/foreignObject tags did not lower to camelCase"
    );
}

#[test]
#[ignore = "VUV-9 retired JSX angle-bracket syntax; view-call coverage lives in reactive_smoke_test"]
fn svg_camel_case_still_works() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/svg/svg_camel_still_works.vox"
    ))
    .unwrap();
    let expected = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/svg/svg_camel_still_works.expected.tsx"
    ))
    .unwrap();

    let files = compile_components(&src);
    let actual = get_component(&files, "IconCamel");
    assert_eq!(
        normalize_ws(&actual).trim().to_string(),
        normalize_ws(&expected).trim().to_string(),
        "IconCamel.tsx: camelCase passthrough broken"
    );
}
