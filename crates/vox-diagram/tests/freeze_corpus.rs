//! One-shot freezer for the Mermaid corpus. This is the only code in the crate allowed to write
//! from `docs/`: it snapshots every live ```` ```mermaid ```` fence into `tests/fixtures/corpus/`
//! with a `MANIFEST.json` of provenance. Run by hand, then commit:
//! `cargo nextest run -p vox-diagram --run-ignored only -E 'binary(freeze_corpus)'`.
//! The second reader of `docs/` is `migration_fidelity`: read-only and ignored, it checks the
//! frozen fences against the docs during the Phase 90 migration. Do not re-run the freezer
//! mid-migration: it would freeze the migrated docs and drop the Mermaid originals.

use std::fs;
use std::path::Path;

use serde_json::json;
use vox_diagram::fence::{extract, normalize};

/// Markdown files under `docs/src`, relative to `root`, sorted. Plain directory walk: the arch
/// guard forbids raw `git` invocations, and the freezer runs from a clean checkout of HEAD.
fn tracked_docs(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            let Ok(rel) = path.strip_prefix(root) else {
                continue;
            };
            let rel = rel.to_string_lossy().replace('\\', "/");
            // Archival protocol: tombstoned docs are never ingested.
            if rel == "docs/src/archive" {
                continue;
            }
            if path.is_dir() {
                walk(&path, root, out);
            } else if rel.ends_with(".md") || rel.ends_with(".mdx") {
                out.push(rel);
            }
        }
    }
    let mut files = Vec::new();
    walk(&root.join("docs/src"), root, &mut files);
    files.sort();
    files
}

#[test]
#[ignore = "one-shot: reads docs/src at HEAD; run by hand to refreeze"]
fn freeze_corpus() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join("crates/vox-diagram/tests/fixtures/corpus");
    let _ = fs::create_dir_all(&dir);
    for old in fs::read_dir(&dir).into_iter().flatten().flatten() {
        if old.path().extension().and_then(|e| e.to_str()) == Some("mmd") {
            let _ = fs::remove_file(old.path());
        }
    }
    let mut manifest = Vec::new();
    for rel in tracked_docs(&root) {
        let Ok(text) = fs::read_to_string(root.join(&rel)) else {
            continue;
        };
        let stem = Path::new(&rel)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("doc");
        for fence in extract(&text, "mermaid") {
            let n = manifest.len();
            let name = format!("{n:02}-{stem}-l{}.mmd", fence.open_line);
            let body = normalize(&fence.body);
            let header = body
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or_default()
                .trim()
                .to_owned();
            if let Err(e) = fs::write(dir.join(&name), format!("{body}\n")) {
                panic!("cannot write {name}: {e}");
            }
            manifest.push(json!({
                "fixture": name,
                "source": rel,
                "open_line": fence.open_line,
                "header": header,
            }));
        }
    }
    let mut out = serde_json::to_string_pretty(&manifest).unwrap_or_default();
    out.push('\n');
    if let Err(e) = fs::write(dir.join("MANIFEST.json"), out) {
        panic!("cannot write MANIFEST.json: {e}");
    }
}
