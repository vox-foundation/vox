//! Guardrail: every `examples/golden/*.vox` file must parse and lower with no `legacy_ast_nodes`.
//!
//! Goldens are rewritten to match the core recursive-descent grammar (see parser `parse_decl`).

use std::path::Path;

use vox_compiler::hir::lower_module;
use vox_compiler::lexer::lex;
use vox_compiler::parser::parse;

fn assert_golden_file(path: &Path) {
    let src =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let tokens = lex(&src);
    let module = parse(tokens).unwrap_or_else(|errs| {
        panic!("parse {} failed: {errs:?}", path.display());
    });
    let hir = lower_module(&module);
    assert!(
        hir.legacy_ast_nodes.is_empty(),
        "{}: expected no legacy_ast_nodes after lowering, got {:?}",
        path.display(),
        hir.legacy_ast_nodes
    );
}

#[test]
fn all_golden_vox_examples_parse_and_lower() {
    let golden_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/golden");
    let read = std::fs::read_dir(&golden_dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", golden_dir.display()));

    let mut count = 0u32;
    for entry in read {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("vox") {
            continue;
        }
        assert_golden_file(&path);
        count += 1;
    }
    assert!(count > 0, "no .vox files under {}", golden_dir.display());
}
