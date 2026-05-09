use vox_codegen::codegen_ts::emitter::generate;
use vox_compiler::{parser::parse, lexer::cursor::lex, hir::lower_module};

fn emit(src: &str) -> String {
    let m = parse(lex(src)).expect("parse");
    let hir = lower_module(&m);
    generate(&hir).expect("emit").files.iter()
        .map(|(_, content)| content.clone())
        .collect::<Vec<_>>().join("\n")
}

#[test]
fn safe_area_top_emits_env_padding() {
    let src = r#"
component C() {
    view: stack(safe_area="top") {
        text() { "hi" }
    }
}
"#;
    let ts = emit(src);
    assert!(ts.contains("env(safe-area-inset-top)"), "got:\n{ts}");
}

#[test]
fn safe_area_all_emits_four_paddings() {
    let src = r#"
component C() {
    view: stack(safe_area="all") {
        text() { "hi" }
    }
}
"#;
    let ts = emit(src);
    assert!(ts.contains("env(safe-area-inset-top)"), "got:\n{ts}");
    assert!(ts.contains("env(safe-area-inset-bottom)"), "got:\n{ts}");
    assert!(ts.contains("env(safe-area-inset-left)"), "got:\n{ts}");
    assert!(ts.contains("env(safe-area-inset-right)"), "got:\n{ts}");
}

#[test]
fn safe_area_top_with_surface_preserves_both() {
    let src = r#"
component C() {
    view: stack(safe_area="top", surface="primary") {
        text() { "hi" }
    }
}
"#;
    let ts = emit(src);
    assert!(ts.contains("env(safe-area-inset-top)"), "safe-area lost, got:\n{ts}");
}
