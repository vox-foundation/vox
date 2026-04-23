#![allow(missing_docs)]

//! Ensures frozen paths listed in `contracts/speech-to-code/benchmark-fixtures.manifest.txt` exist.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn speech_benchmark_manifest_paths_exist() {
    let root = workspace_root();
    let manifest = root.join("contracts/speech-to-code/benchmark-fixtures.manifest.txt");
    let raw = fs::read_to_string(&manifest)
        .unwrap_or_else(|e| panic!("read {}: {e}", manifest.display()));

    for (line_no, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let rel = Path::new(line);
        let abs = root.join(rel);
        assert!(
            abs.exists(),
            "benchmark manifest line {}: missing path {} (resolved: {})",
            line_no + 1,
            line,
            abs.display()
        );
    }
}
