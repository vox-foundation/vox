use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::Path;

use super::matrix::visit_rs_files;

pub(crate) fn run_repo_guards(root: &Path) -> Result<()> {
    guard_no_typevar_zero(root)?;
    guard_no_opencode_refs(root)?;
    guard_no_stray_root_files(root)?;
    println!("repo-guards OK");
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
            let text = fs::read_to_string(p)?;
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
        let text = fs::read_to_string(p)?;
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
