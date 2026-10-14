//! Integration tests for workspace crate API doc generator (GEN-04).

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

#[test]
fn test_crate_api_docs_verify_clean() {
    let root = repo_root();
    let res = vox_cli_ci::crate_api_docs::run(&root, false);
    assert!(
        res.is_ok(),
        "crate_api_docs::run verify failed on repo: {:?}",
        res.err()
    );
}

#[test]
fn test_crate_api_index_and_files_exist() {
    let root = repo_root();
    let index_path = root.join("docs/src/api/index.generated.md");
    assert!(index_path.is_file(), "api/index.generated.md should exist");

    let index_content = std::fs::read_to_string(&index_path).expect("read index");
    assert!(
        index_content.contains("Workspace Crate API Index (generated)"),
        "Missing title in crate API index"
    );
    assert!(
        index_content.contains("vox-bounded-fs"),
        "Missing vox-bounded-fs in index"
    );

    let bounded_doc = root.join("docs/src/api/vox-bounded-fs.generated.md");
    assert!(
        bounded_doc.is_file(),
        "vox-bounded-fs.generated.md should exist"
    );
    let doc_content = std::fs::read_to_string(&bounded_doc).expect("read doc");
    assert!(
        doc_content.contains("category: \"API Reference — Crates\""),
        "Missing valid category"
    );
    assert!(
        doc_content.contains("## Public API Surface"),
        "Missing public surface section"
    );
}

#[test]
fn test_crate_api_docs_dead_path_fails() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let res = vox_cli_ci::crate_api_docs::run(tmp.path(), false);
    assert!(
        res.is_err(),
        "crate_api_docs should fail on empty directory without Cargo.toml"
    );
}
