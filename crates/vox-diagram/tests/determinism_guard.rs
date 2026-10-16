//! Guard: the IR must stay byte-deterministic, so no hash-based collection may exist in `src/`
//! and no sorted map may appear in an IR file. Needles are split with `concat!` so this file
//! cannot match itself.

use std::fs;
use std::path::Path;

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn determinism_guard_no_hash_collections() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    assert!(
        files.len() > 5,
        "walked {} files from {root:?}",
        files.len()
    );
    let hashed = [concat!("Hash", "Map"), concat!("Hash", "Set")];
    let sorted = concat!("BTree", "Map");
    for file in files {
        let text = fs::read_to_string(&file).unwrap_or_default();
        for needle in hashed {
            assert!(!text.contains(needle), "{file:?} uses {needle}");
        }
        if file.to_string_lossy().ends_with("ir.rs") {
            assert!(!text.contains(sorted), "{file:?} uses {sorted} in the IR");
        }
    }
}
