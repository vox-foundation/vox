//! `validate_document_with_hir` runs full frontend checks including HIR.

use vox_lsp::{validate_document, validate_document_with_hir};

#[test]
fn hir_path_matches_lsp_on_parse_failures() {
    let src = "this is not valid vox syntax {{{";
    let base = validate_document(src);
    let with_hir = validate_document_with_hir(src);
    assert!(!base.is_empty(), "sanity: parse errors");
    assert_eq!(base.len(), with_hir.len(), "HIR must not change parse errors");
}

#[test]
fn validate_with_hir_smoke_empty_module() {
    assert!(validate_document_with_hir("").is_empty());
}
