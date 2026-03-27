use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::Path;

use crate::commands::ci::bounded_read::read_utf8_path_capped;

use super::matrix::visit_rs_files;

pub(crate) fn run_repo_guards(root: &Path) -> Result<()> {
    guard_no_typevar_zero(root)?;
    guard_no_opencode_refs(root)?;
    guard_no_stray_root_files(root)?;
    println!("repo-guards OK");
    Ok(())
}

/// Fail when Rust sources outside the Arca SQL home (`vox-db`) use Codex's
/// raw SQL entrypoints on `connection()`: the `query` / `execute` methods (see nomenclature doc).
///
/// See `docs/agents/database-nomenclature.md` and `docs/agents/sql-connection-api-allowlist.txt`.
/// Detects dot-`connection()` chains ending in `query` / `execute` call parentheses, including
/// splits across lines (`.` + method on the next line). Test fixtures avoid spelling the banned
/// substring literally so this module does not trip the repo-wide guard.
#[must_use]
pub(crate) fn sql_surface_contains_raw_connection_api(text: &str) -> bool {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"\.connection\(\)\s*\.\s*(?:query|execute)\s*\(")
            .expect("sql surface regex")
    })
    .is_match(text)
}

pub(crate) fn run_sql_surface_guard(root: &Path, all: bool) -> Result<()> {
    let allow = load_sql_connection_allowlist(root)?;
    let mut offenders = Vec::new();
    for rel in scan_targets(root, all)? {
        let rel_norm = rel.replace('\\', "/");
        if sql_connection_path_allowed(&rel_norm, &allow) {
            continue;
        }
        let path = root.join(&rel);
        if !path.exists() {
            continue;
        }
        let text = read_utf8_path_capped(&path)?;
        if sql_surface_contains_raw_connection_api(&text) {
            offenders.push(rel_norm);
        }
    }
    if !offenders.is_empty() {
        return Err(anyhow!(
            "sql-surface-guard: disallowed Codex connection SQL API (query/execute) outside allowlist in {} file(s): {} — add vox_db::VoxDb methods in store/ops_*.rs (see docs/agents/database-nomenclature.md)",
            offenders.len(),
            offenders.join(", ")
        ));
    }
    println!("sql-surface-guard OK");
    Ok(())
}

fn load_sql_connection_allowlist(root: &Path) -> Result<Vec<String>> {
    let mut out = vec![
        "crates/vox-db/".to_string(),
        "crates/vox-compiler/".to_string(),
    ];
    let p = root.join("docs/agents/sql-connection-api-allowlist.txt");
    if p.is_file() {
        let text = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let norm = line.replace('\\', "/");
            let norm = if norm.ends_with('/') {
                norm
            } else {
                format!("{norm}/")
            };
            out.push(norm);
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn sql_connection_path_allowed(rel: &str, allow: &[String]) -> bool {
    allow.iter().any(|prefix| rel.starts_with(prefix.as_str()))
}

fn path_is_allowed(rel_norm: &str) -> bool {
    rel_norm.starts_with("crates/vox-clavis/")
        || rel_norm == "crates/vox-config/src/inference.rs"
        || rel_norm == "crates/vox-db/src/config.rs"
}

fn scan_targets(root: &Path, all: bool) -> Result<Vec<String>> {
    if all {
        let mut out = Vec::new();
        visit_rs_files(&root.join("crates"), &mut |p: &Path| {
            let rel = p
                .strip_prefix(root)
                .map_err(|e| anyhow!("strip prefix for {}: {e}", p.display()))?
                .to_string_lossy()
                .replace('\\', "/");
            out.push(rel);
            Ok(())
        })?;
        return Ok(out);
    }
    // CI: set `VOX_SECRET_GUARD_GIT_REF` to a two-dot or three-dot range (e.g.
    // `origin/main...HEAD` on pull requests, `${{ github.event.before }}...${{ github.sha }}` on push).
    // Default `git diff HEAD` is empty on clean checkouts — avoid a no-op guard.
    if let Ok(spec) = std::env::var("VOX_SECRET_GUARD_GIT_REF") {
        let spec = spec.trim();
        if !spec.is_empty() {
            let output = std::process::Command::new("git")
                .current_dir(root)
                .args(["diff", "--name-only", "--diff-filter=AMR", spec])
                .output()
                .with_context(|| format!("git diff for secret-env-guard ({spec})"))?;
            if !output.status.success() {
                return Err(anyhow!(
                    "git diff failed while checking secret env usage (range={spec})"
                ));
            }
            return Ok(String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .filter(|l| l.ends_with(".rs"))
                .map(std::string::ToString::to_string)
                .collect());
        }
    }

    let output = std::process::Command::new("git")
        .current_dir(root)
        .args(["diff", "--name-only", "--diff-filter=AMR", "HEAD"])
        .output()
        .context("run git diff HEAD for secret guard")?;
    if !output.status.success() {
        return Err(anyhow!("git diff failed while checking secret env usage"));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| l.ends_with(".rs"))
        .map(std::string::ToString::to_string)
        .collect())
}

pub(crate) fn run_secret_env_guard(root: &Path, all: bool) -> Result<()> {
    let mut names: Vec<String> = vox_clavis::managed_secret_env_names()
        .into_iter()
        .map(regex::escape)
        .collect();
    names.sort();
    names.dedup();
    let disallowed = regex::Regex::new(&format!(
        r#"std::env::var(?:_os)?\("(?:(?:{}))"\)"#,
        names.join("|")
    ))?;
    let mut offenders = Vec::new();
    for rel in scan_targets(root, all)? {
        let rel_norm = rel.replace('\\', "/");
        if path_is_allowed(&rel_norm) {
            continue;
        }
        let path = root.join(&rel);
        if !path.exists() {
            continue;
        }
        let text = read_utf8_path_capped(&path)?;
        if disallowed.is_match(&text) {
            offenders.push(rel);
        }
    }
    if !offenders.is_empty() {
        return Err(anyhow!(
            "secret-env-guard: direct secret env reads found outside Clavis in changed files: {}",
            offenders.join(", ")
        ));
    }
    println!("secret-env-guard OK");
    Ok(())
}

pub(crate) fn run_clavis_parity(root: &Path) -> Result<()> {
    let docs = root
        .join("docs")
        .join("src")
        .join("reference")
        .join("clavis-ssot.md");
    if !docs.exists() {
        return Err(anyhow!(
            "clavis-parity: missing docs/src/reference/clavis-ssot.md"
        ));
    }
    let content = read_utf8_path_capped(&docs)?;
    let missing: Vec<&str> = vox_clavis::managed_secret_env_names()
        .into_iter()
        .filter(|name| !content.contains(name))
        .collect();
    if !missing.is_empty() {
        return Err(anyhow!(
            "clavis-parity: docs/src/reference/clavis-ssot.md missing managed env names: {}",
            missing.join(", ")
        ));
    }
    let missing_bundles: Vec<&str> = vox_clavis::all_bundle_doc_names()
        .iter()
        .copied()
        .filter(|name| !content.contains(name))
        .collect();
    if !missing_bundles.is_empty() {
        return Err(anyhow!(
            "clavis-parity: docs/src/reference/clavis-ssot.md missing bundle names: {}",
            missing_bundles.join(", ")
        ));
    }
    if !content.contains("DeprecatedAliasUsed") {
        return Err(anyhow!(
            "clavis-parity: docs/src/reference/clavis-ssot.md must document DeprecatedAliasUsed lifecycle"
        ));
    }
    println!("clavis-parity OK");
    Ok(())
}

fn guard_no_typevar_zero(root: &Path) -> Result<()> {
    // The typechecker legitimately references `TypeVar(0)`; guard codegen emitters only.
    let re = regex::Regex::new(r"TypeVar\(0\)")?;
    for rel in ["crates/vox-codegen-rust/src", "crates/vox-codegen-ts/src"] {
        let dir = root.join(rel);
        if !dir.is_dir() {
            continue;
        }
        visit_rs_files(&dir, &mut |p: &Path| {
            let text = read_utf8_path_capped(p)?;
            if re.is_match(&text) {
                return Err(anyhow!(
                    "TypeVar(0) must not appear in codegen sources — use fresh inference vars ({})",
                    p.display()
                ));
            }
            Ok(())
        })?;
    }
    Ok(())
}

fn guard_no_opencode_refs(root: &Path) -> Result<()> {
    let crates = root.join("crates");
    let needle = regex::Regex::new(r"opencode")?;
    visit_rs_files(&crates, &mut |p: &Path| {
        let text = read_utf8_path_capped(p)?;
        if !needle.is_match(&text) {
            return Ok(());
        }
        for (idx, line) in text.lines().enumerate() {
            if !line.contains("opencode") {
                continue;
            }
            if line.contains("tests_agent_session")
                || line.contains("// formerly")
                || line.contains("how-to-opencode")
            {
                continue;
            }
            return Err(anyhow!(
                "disallowed opencode reference in {}:{} — {}",
                p.display(),
                idx + 1,
                line.trim()
            ));
        }
        Ok(())
    })?;
    Ok(())
}

fn root_file_is_stray(name: &str) -> bool {
    if name.ends_with(".txt") || name.ends_with(".log") || name.ends_with(".err") {
        return true;
    }
    if (name.starts_with("patch_") || name.starts_with("fix_")) && name.ends_with(".py") {
        return true;
    }
    if name.ends_with(".vox")
        && (name.starts_with("temp") || name.starts_with("test_") || name.starts_with("debug_"))
    {
        return true;
    }
    false
}

fn guard_no_stray_root_files(root: &Path) -> Result<()> {
    let mut offenders = Vec::new();
    for entry in fs::read_dir(root).with_context(|| format!("read {}", root.display()))? {
        let entry = entry?;
        let name_s = entry.file_name().to_string_lossy().into_owned();
        if !entry.file_type()?.is_file() {
            continue;
        }
        if root_file_is_stray(&name_s) {
            offenders.push(name_s);
        }
    }
    if !offenders.is_empty() {
        return Err(anyhow!(
            "stray files at repository root: {}",
            offenders.join(", ")
        ));
    }
    Ok(())
}

#[cfg(test)]
mod sql_surface_tests {
    use super::sql_surface_contains_raw_connection_api;

    #[test]
    fn detects_single_line_connection_query() {
        let src = concat!("db", ".connection().", "query", "(\"SELECT 1\", ()).await");
        assert!(sql_surface_contains_raw_connection_api(src));
    }

    #[test]
    fn detects_multiline_connection_query_chain() {
        let head = "foo\n            store\n                .connection()";
        let tail = "\n                .query(&sql, params)\n                .await\n        ";
        let src = format!("{head}{tail}");
        assert!(sql_surface_contains_raw_connection_api(&src));
    }

    #[test]
    fn detects_multiline_connection_execute() {
        let src = concat!("x", ".connection()", "\n.execute(\"VACUUM\", ())");
        assert!(sql_surface_contains_raw_connection_api(src));
    }

    #[test]
    fn ignores_execute_batch() {
        let src = concat!("db", ".connection().", "execute_batch", "(\"PRAGMA x\")");
        assert!(!sql_surface_contains_raw_connection_api(src));
    }

    #[test]
    fn allowlist_parser_ignores_comments_and_blank_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let agents = tmp.path().join("docs").join("agents");
        std::fs::create_dir_all(&agents).unwrap();
        let p = agents.join("sql-connection-api-allowlist.txt");
        std::fs::write(&p, "# comment\n\ncrates/vox-foo/\n  crates/vox-bar/  \n").unwrap();
        let list = super::load_sql_connection_allowlist(tmp.path()).unwrap();
        assert!(list.iter().any(|e| e == "crates/vox-db/"));
        assert!(list.iter().any(|e| e == "crates/vox-compiler/"));
        assert!(list.iter().any(|e| e == "crates/vox-foo/"));
        assert!(list.iter().any(|e| e == "crates/vox-bar/"));
    }
}
