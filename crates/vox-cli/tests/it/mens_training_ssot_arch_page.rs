//! `docs/src/architecture/mens-training-ssot.md` must stay a short pointer to `reference/mens-training.md`.

use std::fs;
use std::path::PathBuf;

#[test]
fn mens_training_architecture_page_is_pointer_only() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let p = root.join("docs/src/architecture/mens-training-ssot.md");
    let s = fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));

    assert!(
        s.contains("reference/mens-training.md"),
        "{} must link reference/mens-training.md",
        p.display()
    );
    let line_count = s.lines().count();
    assert!(
        line_count <= 32,
        "{} must stay short (pointer page); got {line_count} lines — move detail to reference/mens-training.md",
        p.display()
    );
    assert!(
        !s.contains("\n### "),
        "{} must not add `###` sections (keep procedural detail on the reference page)",
        p.display()
    );
}
