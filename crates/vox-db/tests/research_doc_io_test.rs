use std::fs;
use tempfile::tempdir;
use vox_db::research_doc_io::{atomic_write_secure, update_research_index_md};

#[test]
fn test_atomic_write_and_idempotent_index_update() {
    let dir = tempdir().expect("tempdir");
    let test_file = dir.path().join("test-doc.md");
    atomic_write_secure(&test_file, b"# Hello World").expect("atomic write");
    assert_eq!(fs::read_to_string(&test_file).unwrap(), "# Hello World");

    let index_file = dir.path().join("research-index.md");
    let initial_index = "---\ntitle: \"Research Index\"\n---\n\n## Strategic & Value Proposition\n\n- [Old Doc](old-doc.md) — Existing description.\n\n## Data Storage\n";
    fs::write(&index_file, initial_index).unwrap();

    // 1. Insert new entry
    update_research_index_md(
        &index_file,
        "Strategic & Value Proposition",
        "new-doc.md",
        "New Doc Title",
        "A brand new architecture document.",
    )
    .expect("update index");

    let content = fs::read_to_string(&index_file).unwrap();
    assert!(content.contains("- [New Doc Title](new-doc.md) — A brand new architecture document."));

    // 2. Update existing entry idempotently (no duplicate rows)
    update_research_index_md(
        &index_file,
        "Strategic & Value Proposition",
        "new-doc.md",
        "New Doc Title Updated",
        "An updated description.",
    )
    .expect("update index again");

    let updated = fs::read_to_string(&index_file).unwrap();
    assert_eq!(
        updated.matches("new-doc.md").count(),
        1,
        "Must not duplicate links"
    );
    assert!(updated.contains("- [New Doc Title Updated](new-doc.md) — An updated description."));
}
