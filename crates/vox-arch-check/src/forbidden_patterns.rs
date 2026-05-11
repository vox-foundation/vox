//! Rule 11 (P3-T7): forbid raw `Command::new("git")` outside the wrapper.
//!
//! Implementation: compile `pattern` as a regex; for every file under `file_glob`
//! that is NOT in `exempt_files`, scan line-by-line for matches. If a match is
//! preceded (within 2 lines) or followed (within 1 line) by `allow_annotation`,
//! it is suppressed.
//!
//! False positives we tolerate: string literals in doc comments. The annotation
//! suppression is the escape hatch.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use globset::Glob;
use regex::Regex;

/// A `[[forbidden_pattern]]` rule entry from `layers.toml`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForbiddenPatternRule {
    pub name: String,
    pub pattern: String,
    pub file_glob: String,
    #[serde(default)]
    pub exempt_files: Vec<String>,
    pub allow_annotation: Option<String>,
    pub reason: String,
}

/// A single match produced by [`scan`].
#[derive(Debug)]
pub struct ForbiddenPatternHit {
    pub rule: String,
    pub file: PathBuf,
    pub line: usize,
    pub matched: String,
}

/// Scan every file under `repo_root` that matches `rule.file_glob` for the
/// forbidden regex pattern. Returns all hits that are not suppressed by an
/// `allow_annotation` within a ±2 line window.
pub fn scan(repo_root: &Path, rule: &ForbiddenPatternRule) -> Result<Vec<ForbiddenPatternHit>> {
    let regex = Regex::new(&rule.pattern).context("compile forbidden_pattern regex")?;
    let glob = Glob::new(&rule.file_glob)?.compile_matcher();
    let mut hits = Vec::new();

    for path in super::walkdir(repo_root) {
        if !path.is_file() {
            continue;
        }
        let rel = path.strip_prefix(repo_root).unwrap_or(&path);
        if !glob.is_match(rel) {
            continue;
        }
        let rel_unix = rel.to_string_lossy().replace('\\', "/");
        if rule.exempt_files.iter().any(|e| rel_unix == *e) {
            continue;
        }
        let body = match std::fs::read_to_string(&path) {
            Ok(b) => b,
            Err(_) => continue, // skip binary / unreadable files
        };
        let lines: Vec<&str> = body.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if !regex.is_match(line) {
                continue;
            }
            if let Some(ann) = rule.allow_annotation.as_deref() {
                let lo = i.saturating_sub(2);
                let hi = (i + 1).min(lines.len().saturating_sub(1));
                if (lo..=hi).any(|j| lines[j].contains(ann)) {
                    continue;
                }
            }
            hits.push(ForbiddenPatternHit {
                rule: rule.name.clone(),
                file: rel.to_path_buf(),
                line: i + 1,
                matched: regex
                    .find(line)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default(),
            });
        }
    }
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn make_rule() -> ForbiddenPatternRule {
        ForbiddenPatternRule {
            name: "raw-git-exec".into(),
            pattern: r#"Command::new\("git"\)"#.into(),
            file_glob: "crates/**/*.rs".into(),
            exempt_files: vec!["crates/vox-vcs-git/src/git_exec.rs".into()],
            allow_annotation: Some("// vox-arch-check: allow git-exec".into()),
            reason: "All git invocations must go through GitExec.".into(),
        }
    }

    fn write_fixture(dir: &tempfile::TempDir, rel_path: &str, content: &str) {
        let path = dir.path().join(rel_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn raw_git_outside_git_exec_is_flagged() {
        let dir = tempfile::tempdir().unwrap();
        write_fixture(
            &dir,
            "crates/my-crate/src/main.rs",
            r#"fn bad() { let _ = Command::new("git"); }"#,
        );
        let rule = make_rule();
        let hits = scan(dir.path(), &rule).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].rule, "raw-git-exec");
        assert!(hits[0].matched.contains("Command::new(\"git\")"));
    }

    #[test]
    fn exempt_file_is_not_flagged() {
        let dir = tempfile::tempdir().unwrap();
        write_fixture(
            &dir,
            "crates/vox-vcs-git/src/git_exec.rs",
            r#"fn run() { let _ = Command::new("git"); }"#,
        );
        let rule = make_rule();
        let hits = scan(dir.path(), &rule).unwrap();
        assert_eq!(hits.len(), 0, "exempt file must not produce hits");
    }

    #[test]
    fn allow_annotation_on_preceding_line_suppresses_hit() {
        let dir = tempfile::tempdir().unwrap();
        write_fixture(
            &dir,
            "crates/my-crate/src/lib.rs",
            "// vox-arch-check: allow git-exec\nlet _ = Command::new(\"git\");\n",
        );
        let rule = make_rule();
        let hits = scan(dir.path(), &rule).unwrap();
        assert_eq!(hits.len(), 0, "annotated call must be suppressed");
    }

    #[test]
    fn allow_annotation_on_same_line_suppresses_hit() {
        let dir = tempfile::tempdir().unwrap();
        write_fixture(
            &dir,
            "crates/my-crate/src/lib.rs",
            "let _ = Command::new(\"git\"); // vox-arch-check: allow git-exec\n",
        );
        let rule = make_rule();
        let hits = scan(dir.path(), &rule).unwrap();
        assert_eq!(hits.len(), 0, "inline annotation must be suppressed");
    }

    #[test]
    fn non_rs_file_under_crates_is_not_scanned() {
        let dir = tempfile::tempdir().unwrap();
        // .toml file should not match the `crates/**/*.rs` glob.
        write_fixture(&dir, "crates/my-crate/Cargo.toml", r#"[package]"#);
        let rule = make_rule();
        // No .rs files → no hits.
        let hits = scan(dir.path(), &rule).unwrap();
        assert_eq!(hits.len(), 0);
    }

    #[test]
    fn multiple_violations_in_same_file_all_reported() {
        let dir = tempfile::tempdir().unwrap();
        write_fixture(
            &dir,
            "crates/my-crate/src/util.rs",
            "let a = Command::new(\"git\");\nlet b = Command::new(\"git\");\n",
        );
        let rule = make_rule();
        let hits = scan(dir.path(), &rule).unwrap();
        assert_eq!(hits.len(), 2);
    }
}
