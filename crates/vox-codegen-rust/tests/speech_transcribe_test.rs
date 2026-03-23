//! `Speech.transcribe` lowers to `vox_oratio::transcribe_path` + Vox `Ok`/`Error` variants.
use vox_ast::span::Span;
use vox_codegen_rust::emit::emit_expr;
use vox_hir::{HirArg, HirExpr};
use vox_test_harness::spans::dummy_span;

fn str_arg(s: &str) -> HirArg {
    HirArg {
        name: None,
        value: HirExpr::StringLit(s.to_string(), dummy_span()),
    }
}

fn ident(name: &str) -> Box<HirExpr> {
    Box::new(HirExpr::Ident(name.to_string(), dummy_span()))
}

#[test]
fn speech_transcribe_emits_oratio_match() {
    let expr = HirExpr::MethodCall(
        ident("Speech"),
        "transcribe".into(),
        vec![str_arg("/tmp/a.txt")],
        dummy_span(),
    );
    let out = emit_expr(&expr);
    assert!(
        out.contains("vox_oratio::transcribe_path"),
        "expected vox_oratio path, got: {out}"
    );
    assert!(
        out.contains("display_text()"),
        "expected refined display_text, got: {out}"
    );
    assert!(
        out.contains("Ok(") && out.contains("Error("),
        "expected Ok/Error arms: {out}"
    );
}
