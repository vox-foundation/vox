#![allow(missing_docs)]

//! Validates frozen speech-to-code `.vox` fixtures against HIR parity (compile gate for benchmarks).

use std::fs;
use std::path::PathBuf;

use tower_lsp::lsp_types::DiagnosticSeverity;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn speech_fixture_expected_vox_passes_hir_gate() {
    let root = workspace_root();
    let path = root.join("tests/speech-to-code/fixtures/p001_expected.vox");
    let src = fs::read_to_string(&path).expect("read fixture vox");
    let diags = vox_lsp::validate_document_with_hir(&src);
    let errs: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .collect();
    assert!(
        errs.is_empty(),
        "HIR validation failed for {}: {:?}",
        path.display(),
        errs
    );
}
