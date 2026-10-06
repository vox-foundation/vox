//! TRUST-01 assumption-delta invariant: the only non-test code that INSERTs into
//! `search_documents` or `kb_findings` is a tier-requiring writer. A new writer elsewhere would
//! bypass the compile-time tier requirement, so it fails here until it is routed through one.

use std::path::{Path, PathBuf};

/// The tier-requiring writers (`upsert_search_document`, `apply_library_document`, the
/// kb_findings capture path). Paths are relative to the workspace `crates/` directory.
const TIERED_WRITERS: [&str; 3] = [
    "vox-db/src/store/ops_memory/search.rs",
    "vox-db/src/store/ops_memory/kb_findings.rs",
    "vox-db/src/store/ops_memory/library.rs",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if p.is_dir() {
            if name != "target" && name != "node_modules" && name != ".git" {
                rust_files(&p, out);
            }
        } else if name.ends_with(".rs") && !name.ends_with("_tests.rs") {
            out.push(p);
        }
    }
}

/// True when `src` contains a (case/whitespace-insensitive) INSERT/REPLACE into a guarded table.
fn writes_guarded_table(src: &str) -> bool {
    let norm = src
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    norm.contains("into search_documents") || norm.contains("into kb_findings")
}

/// `(files scanned, offending paths relative to crates/)`.
fn scan(crates_dir: &Path) -> (usize, Vec<String>) {
    let mut files = Vec::new();
    rust_files(crates_dir, &mut files);
    let (mut scanned, mut bad) = (0, Vec::new());
    for f in files {
        let rel = f
            .strip_prefix(crates_dir)
            .unwrap_or(&f)
            .to_string_lossy()
            .replace('\\', "/");
        if rel.contains("/tests/") {
            continue;
        }
        scanned += 1;
        let Ok(src) = std::fs::read_to_string(&f) else {
            continue;
        };
        if writes_guarded_table(&src) && !TIERED_WRITERS.contains(&rel.as_str()) {
            bad.push(rel);
        }
    }
    bad.sort();
    (scanned, bad)
}

#[test]
fn search_corpus_and_inbox_have_only_tiered_writers() {
    let crates_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let (scanned, bad) = scan(&crates_dir);
    assert!(
        scanned > 50,
        "walk looks vacuous: only {scanned} files scanned"
    );
    assert!(
        bad.is_empty(),
        "raw INSERT into search_documents/kb_findings outside the tiered writers {TIERED_WRITERS:?}: {bad:?}"
    );
}

#[test]
fn guarded_table_detector_matches_spaced_and_cased_inserts() {
    assert!(writes_guarded_table("INSERT INTO\n   Search_Documents (a)"));
    assert!(writes_guarded_table("insert or replace into kb_findings"));
    assert!(!writes_guarded_table("SELECT * FROM search_documents"));
}
