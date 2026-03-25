//! Baseline ref publishing for semantic / full-repo flows.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

pub fn push_baseline_from_origin(
    repo: &Path,
    baseline_name: &str,
    default_branch: &str,
    empty: bool,
) -> Result<String> {
    let status = Command::new("git")
        .args(["fetch", "origin", default_branch])
        .current_dir(repo)
        .status()
        .context("git fetch origin")?;
    if !status.success() {
        anyhow::bail!("git fetch origin {default_branch} failed");
    }
    let remote_ref = format!("origin/{default_branch}");
    let out = Command::new("git")
        .args(["rev-parse", &remote_ref])
        .current_dir(repo)
        .output()
        .context("git rev-parse")?;
    if !out.status.success() {
        anyhow::bail!(
            "rev-parse {remote_ref}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();

    let push_sha = if empty {
        // Create an empty tree
        let empty_tree = "4b825dc642cb6eb9a060e54bf8d69288fbee4904"; // git mktree < NUL

        let commit_out = Command::new("git")
            .args([
                "commit-tree",
                empty_tree,
                "-p",
                &sha,
                "-m",
                "Empty baseline for full-repo review",
            ])
            .current_dir(repo)
            .output()
            .context("git commit-tree")?;

        if !commit_out.status.success() {
            anyhow::bail!(
                "commit-tree: {}",
                String::from_utf8_lossy(&commit_out.stderr)
            );
        }
        String::from_utf8_lossy(&commit_out.stdout)
            .trim()
            .to_string()
    } else {
        sha.clone()
    };

    let refspec = format!("{push_sha}:refs/heads/{baseline_name}");
    let status = Command::new("git")
        .args(["push", "-f", "origin", &refspec])
        .current_dir(repo)
        .status()
        .context("git push baseline")?;
    if !status.success() {
        anyhow::bail!("git push origin {refspec} failed");
    }
    eprintln!("[baseline] {baseline_name} -> {push_sha} (from {remote_ref}, empty={empty})");
    Ok(push_sha)
}
