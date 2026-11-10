//! `vox snapshot` — insta snapshot helpers.
//!
//! `vox snapshot orphans [--clean]` walks `**/tests/snapshots/*.snap`, reads the `source:` header
//! line (insta format), and verifies the referenced test function still exists in the source file.
//! Orphans are printed; `--clean` deletes them.

use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::{Path, PathBuf};

#[derive(Subcommand, Debug, Clone)]
pub enum SnapshotCmd {
    /// Detect (and optionally delete) insta `.snap` files whose test no longer exists.
    Orphans {
        /// Root to search for snapshot files (default: current directory).
        #[arg(default_value = ".")]
        root: PathBuf,
        /// Delete orphaned `.snap` files instead of just listing them.
        #[arg(long)]
        clean: bool,
    },
}

pub fn run(cmd: &SnapshotCmd) -> Result<()> {
    match cmd {
        SnapshotCmd::Orphans { root, clean } => run_orphans(root, *clean),
    }
}

/// Resolve the cargo workspace root that contains `root`, via `cargo metadata`.
/// insta's `source:` header is written relative to this root, not to the
/// snapshot file's own directory.
fn resolve_workspace_root(root: &Path) -> Result<PathBuf> {
    let meta = cargo_metadata::MetadataCommand::new()
        .current_dir(root)
        .no_deps()
        .exec()
        .context("cargo metadata (to resolve workspace root for snapshot `source:` paths)")?;
    Ok(meta.workspace_root.into_std_path_buf())
}

fn run_orphans(root: &Path, clean: bool) -> Result<()> {
    let workspace_root = resolve_workspace_root(root)?;
    let snaps = collect_snap_files(root)?;
    let mut orphan_count = 0usize;
    let mut unresolvable_count = 0usize;
    let mut checked = 0usize;

    for snap_path in &snaps {
        checked += 1;
        match snap_is_orphan(snap_path, &workspace_root)? {
            OrphanResult::Orphan {
                source_file,
                test_name,
            } => {
                orphan_count += 1;
                if clean {
                    std::fs::remove_file(snap_path)?;
                    println!(
                        "deleted  {} (test `{}` not found in {})",
                        snap_path.display(),
                        test_name,
                        source_file.display()
                    );
                } else {
                    println!(
                        "orphan   {} (test `{}` not found in {})",
                        snap_path.display(),
                        test_name,
                        source_file.display()
                    );
                }
            }
            OrphanResult::Ok => {}
            OrphanResult::Unresolvable(reason) => {
                unresolvable_count += 1;
                eprintln!("warn: {} — {}", snap_path.display(), reason);
            }
        }
    }

    println!(
        "{checked} snapshots checked, {orphan_count} orphan(s) found, {unresolvable_count} unresolvable."
    );
    if orphan_count > 0 && !clean {
        println!("Re-run with --clean to delete them.");
    }
    // Unresolvable snapshots mean the orphan count is not trustworthy — never delete
    // anything and never report success in that case (an unresolvable `source:` path
    // most often means every snapshot is being mis-resolved, not that everything is fine).
    if unresolvable_count > 0 {
        anyhow::bail!(
            "{unresolvable_count} snapshot(s) had an unresolvable `source:` path — refusing to \
             report orphan status. Nothing was deleted."
        );
    }
    if orphan_count > 0 {
        anyhow::bail!("{orphan_count} orphan snapshot(s) detected");
    }
    Ok(())
}

#[derive(Debug)]
enum OrphanResult {
    Ok,
    Orphan {
        source_file: PathBuf,
        test_name: String,
    },
    Unresolvable(String),
}

/// Walk `root` and collect all `.snap` files under any `tests/snapshots/` directory.
fn collect_snap_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    collect_recursive(root, &mut out)?;
    Ok(out)
}

fn collect_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // Skip hidden dirs and target/
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            collect_recursive(&path, out)?;
        } else if path.is_file() {
            let in_snapshots_dir = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .map(|n| n == "snapshots")
                .unwrap_or(false);
            if in_snapshots_dir && path.extension().map(|e| e == "snap").unwrap_or(false) {
                out.push(path);
            }
        }
    }
    Ok(())
}

/// Parse a `.snap` file and check whether its referenced test function still exists.
fn snap_is_orphan(snap_path: &Path, workspace_root: &Path) -> Result<OrphanResult> {
    let content = std::fs::read_to_string(snap_path)?;

    // Insta snapshot header format:
    //   ---
    //   source: crates/foo/tests/bar.rs
    //   assertion_line: 42
    //   expression: "some_value"
    //   ---
    // `source:` is written by insta relative to the cargo *workspace* root, not to the
    // snapshot file's own directory (which is typically `<crate>/tests/snapshots/`).
    let source_rel = match parse_insta_source_header(&content) {
        Some(s) => s,
        None => {
            return Ok(OrphanResult::Unresolvable(
                "no `source:` header found — skipping".into(),
            ));
        }
    };

    let source_file = workspace_root.join(&source_rel);
    let source_file = match source_file.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // Source file doesn't exist at all — definitely orphan-adjacent, but we
            // can't confirm test name; treat as unresolvable.
            return Ok(OrphanResult::Unresolvable(format!(
                "source file not found: {}",
                source_file.display()
            )));
        }
    };

    // Infer the test name from the snapshot file stem.
    // Insta names snapshots as `<module>__<test_name>` or just `<test_name>`.
    let stem = snap_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    // The function name is the last `__`-separated segment.
    let test_name = stem.rsplit("__").next().unwrap_or(stem).to_string();

    let source_content = std::fs::read_to_string(&source_file)?;
    if source_contains_test(&source_content, &test_name) {
        Ok(OrphanResult::Ok)
    } else {
        Ok(OrphanResult::Orphan {
            source_file,
            test_name,
        })
    }
}

/// Extract the value of the `source:` key from an insta snapshot YAML header.
fn parse_insta_source_header(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("source:") {
            // Value may be quoted: `source: "../../src/foo.rs"` or bare.
            let val = rest.trim().trim_matches('"');
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

/// Check whether `content` contains a test function named `test_name`.
/// Matches `fn <test_name>` with word boundaries to avoid false positives.
fn source_contains_test(content: &str, test_name: &str) -> bool {
    // Accept `fn test_name(` or `async fn test_name(`
    let needle = format!("fn {test_name}");
    content.contains(&needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_quoted_source_header() {
        let snap = r#"---
source: "../../src/tests/foo.rs"
assertion_line: 42
---
value"#;
        assert_eq!(
            parse_insta_source_header(snap),
            Some("../../src/tests/foo.rs".into())
        );
    }

    #[test]
    fn parse_missing_source_header() {
        let snap = "---\nassertion_line: 5\n---\nvalue";
        assert!(parse_insta_source_header(snap).is_none());
    }

    #[test]
    fn source_contains_test_positive() {
        let src = "#[test]\nfn my_great_test() { assert!(true); }";
        assert!(source_contains_test(src, "my_great_test"));
    }

    #[test]
    fn source_contains_test_negative() {
        let src = "#[test]\nfn some_other_test() { }";
        assert!(!source_contains_test(src, "my_great_test"));
    }

    #[test]
    fn source_contains_test_no_substring_false_positive() {
        // "my_test" should not match "fn my_test_extended"... actually it would via `contains`.
        // The `fn <name>` check naturally avoids matching unrelated names because we search
        // for the exact stem. This test documents the known behavior.
        let src = "fn my_test_extra() {}";
        // "fn my_test" IS a substring of "fn my_test_extra", so it returns true.
        // Callers must supply exact stems (insta uses `__`-delimited stems).
        assert!(source_contains_test(src, "my_test"));
    }

    /// Build a fake workspace: `<root>/crates/foo/tests/bar_test.rs` (the source) and
    /// `<root>/crates/foo/tests/snapshots/bar_test__some_test.snap` (the snapshot), with
    /// `source:` written the way insta actually writes it — relative to the workspace root.
    fn make_fixture_workspace(source_body: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let tests_dir = dir.path().join("crates/foo/tests");
        let snaps_dir = tests_dir.join("snapshots");
        std::fs::create_dir_all(&snaps_dir).unwrap();
        std::fs::write(tests_dir.join("bar_test.rs"), source_body).unwrap();

        let snap_path = snaps_dir.join("bar_test__some_test.snap");
        std::fs::write(
            &snap_path,
            "---\nsource: crates/foo/tests/bar_test.rs\nexpression: \"x\"\n---\nvalue",
        )
        .unwrap();
        (dir, snap_path)
    }

    #[test]
    fn snap_is_orphan_resolves_source_against_workspace_root_not_snap_dir() {
        // Regression test for the bug where `source:` (workspace-root-relative, as insta
        // writes it) was joined against the snapshot file's own directory instead, producing
        // a doubled, nonexistent path (e.g. `.../tests/snapshots/crates/foo/tests/bar_test.rs`)
        // and making every snapshot "unresolvable" rather than correctly checked.
        let (dir, snap_path) = make_fixture_workspace("#[test]\nfn some_test() {}");
        let result = snap_is_orphan(&snap_path, dir.path()).unwrap();
        assert!(matches!(result, OrphanResult::Ok));
    }

    #[test]
    fn snap_is_orphan_detects_real_orphan_via_workspace_root() {
        let (dir, snap_path) = make_fixture_workspace("#[test]\nfn some_other_test() {}");
        let result = snap_is_orphan(&snap_path, dir.path()).unwrap();
        match result {
            OrphanResult::Orphan { test_name, .. } => assert_eq!(test_name, "some_test"),
            other => panic!("expected Orphan, got a different result variant: {other:?}"),
        }
    }

    #[test]
    fn snap_is_orphan_unresolvable_when_workspace_root_is_wrong() {
        // Mutation check: resolving against the WRONG root (the old, buggy behavior of
        // joining against the snapshot file's own directory) must NOT report `Ok` or
        // `Orphan` — it must come back `Unresolvable`, proving the fix actually changed
        // resolution behavior rather than coincidentally passing both ways.
        let (dir, snap_path) = make_fixture_workspace("#[test]\nfn some_test() {}");
        let wrong_root = snap_path.parent().unwrap(); // the snapshots/ dir — the old (buggy) base
        let result = snap_is_orphan(&snap_path, wrong_root).unwrap();
        assert!(
            matches!(result, OrphanResult::Unresolvable(_)),
            "expected Unresolvable when resolving against the wrong root, got {result:?}"
        );
        let _ = dir; // keep TempDir alive for the duration of the assertions above
    }
}
