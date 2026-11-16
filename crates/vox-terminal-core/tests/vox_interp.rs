use vox_terminal_core::vox_interp::eval_line;

#[cfg(feature = "vox-lang")]
#[test]
fn evaluates_vox_expression() {
    let out = eval_line("fn main() -> Int { 40 + 2 }").unwrap();
    assert!(out.contains("42"), "got: {out}");
}

#[cfg(feature = "vox-lang")]
#[test]
fn surfaces_compile_errors() {
    assert!(eval_line("let = ").is_err());
}

#[cfg(not(feature = "vox-lang"))]
#[test]
fn returns_vox_lang_diagnostic_when_disabled() {
    let err = eval_line("fn main() -> Int { 40 + 2 }")
        .unwrap_err()
        .to_string();
    assert!(err.contains("vox-lang"), "got: {err}");
}
