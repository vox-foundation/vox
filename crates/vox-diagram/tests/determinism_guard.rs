//! Guard: the IR must stay byte-deterministic, so no hash-based collection may exist in `src/`
//! and no sorted map may appear in an IR file. The layout and SVG output must also stay free of
//! transcendental float calls (results differ across platforms), clocks and version reads, and
//! only `layout/dagre_impl.rs` may name the dagre crate. Needles are split or built at run time
//! so this file cannot match itself.

use std::fs;
use std::path::{Path, PathBuf};

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

fn src_files() -> (PathBuf, Vec<PathBuf>) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    assert!(
        files.len() > 5,
        "walked {} files from {root:?}",
        files.len()
    );
    (root, files)
}

#[test]
fn determinism_guard_no_transcendental_floats() {
    let names = [
        "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh", "powf",
        "powi", "exp", "exp2", "exp_m1", "ln", "ln_1p", "log", "log2", "log10", "hypot", "cbrt",
    ];
    let (_, files) = src_files();
    for file in files {
        let text = fs::read_to_string(&file).unwrap_or_default();
        for name in names {
            for needle in [format!(".{name}("), format!("::{name}(")] {
                assert!(!text.contains(&needle), "{file:?} calls {needle}");
            }
        }
    }
}

#[test]
fn determinism_guard_no_clock_or_version() {
    let needles = [
        concat!("System", "Time"),
        concat!("Inst", "ant"),
        concat!("CARGO_PKG", "_VERSION"),
        concat!("env", "!("),
    ];
    let (root, files) = src_files();
    let mut checked = 0;
    for file in files {
        let rel = file.strip_prefix(&root).unwrap_or(&file);
        if !(rel.starts_with("layout") || rel.starts_with("svg")) {
            continue;
        }
        checked += 1;
        let text = fs::read_to_string(&file).unwrap_or_default();
        for needle in needles {
            assert!(!text.contains(needle), "{file:?} reads {needle}");
        }
    }
    assert!(checked >= 8, "only {checked} layout/svg files walked");
}

#[test]
fn layout_seam_guard_dagre_only_in_adapter() {
    let path_needle = concat!("dagre", "::");
    let use_needle = concat!("use ", "dagre");
    let (root, files) = src_files();
    // `use dagre` only counts at the start of a line: prose such as "because dagre" contains it.
    let names_dagre = |text: &str| {
        text.contains(path_needle)
            || text.lines().any(|l| {
                let l = l.trim_start();
                l.starts_with(use_needle)
                    || l.strip_prefix("pub ")
                        .is_some_and(|r| r.starts_with(use_needle))
            })
    };
    let importers: Vec<PathBuf> = files
        .iter()
        .filter(|f| names_dagre(&fs::read_to_string(f).unwrap_or_default()))
        .map(|f| f.strip_prefix(&root).unwrap_or(f).to_path_buf())
        .collect();
    assert_eq!(importers, [PathBuf::from("layout").join("dagre_impl.rs")]);
}
