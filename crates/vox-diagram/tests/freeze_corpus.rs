//! One-shot freezer for the Mermaid corpus. This is the only code in the crate allowed to write
//! from `docs/`: it snapshots every live ```` ```mermaid ```` fence into `tests/fixtures/corpus/`
//! with a `MANIFEST.json` of provenance. Run by hand, then commit:
//! `cargo nextest run -p vox-diagram --run-ignored only -E 'binary(freeze_corpus) & test(freeze_corpus)'`.
//! The second reader of `docs/` is `migration_fidelity`: read-only and ignored, it checks the
//! frozen fences against the docs.
//!
//! The corpus is closed: Phase 90 migrated 32 of its 33 fences to `vox-diagram`, and the frozen
//! `.mmd` files are the only copy of their Mermaid source (the Node oracle, `mermaid_roundtrip` and
//! `migration_fidelity` read them). So the freezer refuses, before touching anything, to write a
//! MANIFEST with fewer fences than the one on disk, and it fails on any I/O error.

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

/// Freeze every Mermaid fence of `root/docs/src` into `dir`; the number frozen. Refuses, leaving
/// `dir` untouched, when that is fewer than `dir/MANIFEST.json` already lists.
fn freeze(root: &Path, dir: &Path) -> Result<usize, String> {
    let mut frozen = Vec::new();
    let mut manifest = Vec::new();
    for rel in tracked_docs(root) {
        let text = fs::read_to_string(root.join(&rel)).map_err(|e| format!("{rel}: {e}"))?;
        let stem = Path::new(&rel)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("doc");
        for fence in extract(&text, "mermaid") {
            let name = format!("{:02}-{stem}-l{}.mmd", manifest.len(), fence.open_line);
            let body = normalize(&fence.body);
            let header = body
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or_default()
                .trim()
                .to_owned();
            manifest.push(json!({
                "fixture": name,
                "source": rel,
                "open_line": fence.open_line,
                "header": header,
            }));
            frozen.push((name, format!("{body}\n")));
        }
    }
    let listed = match fs::read_to_string(dir.join("MANIFEST.json")) {
        Ok(t) => serde_json::from_str::<Vec<serde_json::Value>>(&t)
            .map_err(|e| format!("MANIFEST.json: {e}"))?
            .len(),
        Err(_) => 0,
    };
    if frozen.len() < listed {
        return Err(format!(
            "freezer would drop {} of {listed} frozen fences: the docs are migrated and the corpus is closed",
            listed - frozen.len()
        ));
    }
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for old in fs::read_dir(dir).map_err(|e| e.to_string())?.flatten() {
        if old.path().extension().and_then(|e| e.to_str()) == Some("mmd") {
            fs::remove_file(old.path()).map_err(|e| e.to_string())?;
        }
    }
    for (name, body) in &frozen {
        fs::write(dir.join(name), body).map_err(|e| format!("{name}: {e}"))?;
    }
    let mut out = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    out.push('\n');
    fs::write(dir.join("MANIFEST.json"), out).map_err(|e| e.to_string())?;
    Ok(frozen.len())
}

#[test]
#[ignore = "one-shot: reads docs/src at HEAD; run by hand to refreeze"]
fn freeze_corpus() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if let Err(e) = freeze(
        &root,
        &root.join("crates/vox-diagram/tests/fixtures/corpus"),
    ) {
        panic!("{e}");
    }
}

/// A scratch tree under the system temp dir, removed first so a rerun starts clean.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("vd-freeze-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| panic!("{}: {e}", parent.display()));
    }
    fs::write(path, text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}

const DOC: &str = "# Doc\n\n```mermaid\ngraph TD\n  A --> B\n```\n";

/// Phase 90 review WR-06: with the docs migrated, a run would have deleted every frozen `.mmd` and
/// kept only the fences still in Mermaid. It must refuse and leave the corpus as it was.
#[test]
fn the_freezer_refuses_to_shrink_the_corpus_and_touches_nothing() {
    let root = scratch("shrink");
    write(&root.join("docs/src/a.md"), DOC);
    let dir = root.join("corpus");
    let manifest = r#"[{"fixture": "00-x.mmd"}, {"fixture": "01-y.mmd"}]"#;
    write(&dir.join("MANIFEST.json"), manifest);
    write(&dir.join("00-x.mmd"), "graph TD\n  X --> Y\n");
    write(&dir.join("01-y.mmd"), "graph TD\n  Y --> Z\n");
    let r = freeze(&root, &dir);
    assert!(matches!(&r, Err(m) if m.contains("drop 1 of 2")), "{r:?}");
    assert_eq!(
        fs::read_to_string(dir.join("MANIFEST.json"))
            .ok()
            .as_deref(),
        Some(manifest)
    );
    assert!(dir.join("00-x.mmd").exists() && dir.join("01-y.mmd").exists());
    assert!(!dir.join("00-a-l3.mmd").exists());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_freezer_freezes_a_corpus_that_does_not_shrink() {
    let root = scratch("grow");
    write(&root.join("docs/src/a.md"), DOC);
    let dir = root.join("corpus");
    assert_eq!(freeze(&root, &dir), Ok(1));
    assert_eq!(
        fs::read_to_string(dir.join("00-a-l3.mmd")).ok().as_deref(),
        Some("graph TD\n  A --> B\n")
    );
    let _ = fs::remove_dir_all(&root);
}
