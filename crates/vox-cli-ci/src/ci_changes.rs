//! `vox ci ci-changes` — per-job boolean outputs from changed paths (t3code ci-changes.cjs).

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn classify(path: &str) -> Vec<&'static str> {
    let p = path.replace('\\', "/");
    let mut jobs = Vec::new();
    if p.starts_with("crates/vox-gui/") || p.starts_with("crates/vox-gui/ui/") {
        jobs.extend(["ui", "typecheck"]);
    }
    if p.starts_with("crates/") || p == "Cargo.toml" || p == "Cargo.lock" {
        jobs.extend(["clippy", "nextest"]);
    }
    if p.starts_with("docs/") || p.starts_with(".planning/") {
        jobs.push("docs");
    }
    if p.starts_with(".github/workflows/") || p.starts_with("contracts/") {
        jobs.extend(["ssot", "clippy"]);
    }
    if p.contains("sidecar") || p.contains("tauri.conf") {
        jobs.push("sidecar_fingerprint");
    }
    if jobs.is_empty() {
        jobs.push("misc");
    }
    jobs
}

fn git_names(root: &Path, base: &str) -> Result<Vec<String>> {
    let out = Command::new("git")
        .args([
            "-C",
            &root.to_string_lossy(),
            "diff",
            "--name-only",
            "--no-renames",
            base,
        ])
        .output()
        .context("git diff")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!(
            "git diff --name-only {base} failed (status={}): {stderr}",
            out.status
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect())
}

pub struct CiChangesArgs {
    pub base: String,
    pub paths: Option<PathBuf>,
    pub github_output: Option<PathBuf>,
}

pub fn run(root: &Path, args: CiChangesArgs) -> Result<()> {
    let files = if let Some(list) = args.paths {
        let p = if list.is_absolute() {
            list
        } else {
            root.join(list)
        };
        std::fs::read_to_string(p)?
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
    } else {
        git_names(root, &args.base)?
    };

    let mut flags: BTreeMap<&str, bool> = [
        ("ui", false),
        ("typecheck", false),
        ("clippy", false),
        ("nextest", false),
        ("docs", false),
        ("ssot", false),
        ("sidecar_fingerprint", false),
        ("misc", false),
    ]
    .into_iter()
    .collect();

    for f in &files {
        for j in classify(f) {
            if let Some(slot) = flags.get_mut(j) {
                *slot = true;
            }
        }
    }

    println!("ci-changes: files={}", files.len());
    for (k, v) in &flags {
        println!("ci-changes: {k}={v}");
    }
    if let Some(out) = args.github_output {
        let mut body = String::new();
        for (k, v) in &flags {
            body.push_str(&format!("{k}={}\n", if *v { "true" } else { "false" }));
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::write(&out, body).with_context(|| format!("write {}", out.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_gui_and_crates() {
        assert!(classify("crates/vox-gui/ui/src/App.tsx").contains(&"ui"));
        assert!(classify("crates/vox-cli/src/main.rs").contains(&"clippy"));
        assert!(classify("docs/src/ci/foo.md").contains(&"docs"));
    }
}
