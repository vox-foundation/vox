//! End-to-end: parse `.vox` with `import rust:…`, lower, script codegen includes `Cargo.toml` deps.

use vox_compiler_emit::codegen_rust::{ScriptTarget, generate_script_with_target};
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::lex;
use vox_compiler::parser::parse;

#[test]
fn parse_lower_script_codegen_includes_rust_dep() {
    let source = r#"
import rust:chrono(version: "0.4") as ch
fn main() {}
"#;
    let module = parse(lex(source)).expect("parse");
    let hir = lower_module(&module);
    let out = generate_script_with_target(&hir, "vox-script", None, ScriptTarget::Native)
        .expect("codegen");
    let cargo = out.files.get("Cargo.toml").expect("Cargo.toml");
    assert!(
        cargo.contains("chrono") && cargo.contains("0.4"),
        "script Cargo.toml should list rust import:\n{cargo}"
    );
}
