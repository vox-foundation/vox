//! Git helpers for batch planner (`git diff` / numstat).

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};

/// One changed file with a sort key (insertions+deletions from `git diff --numstat`).
pub struct DiffEntry {
    pub path: PathBuf,
    pub weight: u64,
}

/// Collect changed paths vs `base_ref` (default `HEAD` = working tree vs last commit).
///
/// Uses `git diff --numstat` so the batch planner can sort by change size.
pub fn collect_git_diffs(repo: &std::path::Path, base_ref: Option<&str>) -> Result<Vec<DiffEntry>> {
    let base = base_ref.unwrap_or("HEAD");
    let out = Command::new("git")
        .args(["-c", "core.autocrlf=false", "diff", "--numstat", base])
        .current_dir(repo)
        .output()
        .context("git diff --numstat")?;

    if !out.status.success() {
        anyhow::bail!("git diff failed: {}", String::from_utf8_lossy(&out.stderr));
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let mut entries = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(3, '\t');
        let a = parts.next().unwrap_or("");
        let b = parts.next().unwrap_or("");
        let path = parts.next().unwrap_or("").replace('\\', "/");
        if path.is_empty() {
            continue;
        }
        // Binary or rename lines may show `-`
        let add = a.parse::<u64>().unwrap_or(1);
        let del = b.parse::<u64>().unwrap_or(1);
        entries.push(DiffEntry {
            path: PathBuf::from(path),
            weight: add.saturating_add(del),
        });
    }
    Ok(entries)
}
