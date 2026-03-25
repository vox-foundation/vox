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
    let output = std::process::Command::new("git")
        .current_dir(root)
        .args(["diff", "--name-only", "--diff-filter=AMR", "HEAD"])
        .output()
        .context("run git diff for secret guard")?;
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
