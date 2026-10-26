//! `vox ci hit-every-surface` — advisory checklist when a diff spans surface classes.

use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

const CHECKLIST: &[&str] = &[
    "entry points (CLI / Tauri / MCP registration)",
    "clients (GUI hooks / IPC)",
    "providers / adapters (LLM facade, plugins)",
    "MCP / agent reachability",
    "contracts (+ schema regen)",
    "reverse / error states",
    "connection modes (local / remote / MCP-only)",
    "docs (AGENTS / architecture)",
];

fn classify(path: &str) -> Option<&'static str> {
    let p = path.replace('\\', "/");
    if p.starts_with("crates/vox-gui/") {
        Some("gui")
    } else if p.starts_with("crates/vox-orchestrator-mcp/") || p.contains("/mcp") {
        Some("mcp")
    } else if p.starts_with("crates/vox-orchestrator/") || p.starts_with("crates/vox-actor-runtime/")
    {
        Some("harness")
    } else if p.starts_with("contracts/") {
        Some("contracts")
    } else if p.starts_with("docs/") || p == "AGENTS.md" {
        Some("docs")
    } else {
        None
    }
}

fn git_diff_names(root: &Path, base: &str) -> Result<Vec<String>> {
    let out = Command::new("git")
        .args(["-C", &root.to_string_lossy(), "diff", "--name-only", base])
        .output()
        .context("git diff")?;
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect())
}

pub struct HitEverySurfaceArgs {
    pub base: String,
    pub paths: Option<PathBuf>,
    pub strict: bool,
}

pub fn run(root: &Path, args: HitEverySurfaceArgs) -> Result<()> {
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
            .collect()
    } else {
        git_diff_names(root, &args.base).unwrap_or_default()
    };

    let mut classes = BTreeSet::new();
    for f in &files {
        if let Some(c) = classify(f) {
            classes.insert(c);
        }
    }

    let ack = std::env::var("HIT_EVERY_SURFACE").ok().as_deref() == Some("1");
    println!("hit-every-surface: classes={classes:?} files={}", files.len());
    if classes.len() >= 2 && !ack {
        println!("hit-every-surface: advisory — complete the multi-surface checklist:");
        for (i, row) in CHECKLIST.iter().enumerate() {
            println!("  {}. {row}", i + 1);
        }
        println!("  (set HIT_EVERY_SURFACE=1 after acknowledging)");
        if args.strict {
            anyhow::bail!("multi-surface diff without HIT_EVERY_SURFACE=1 acknowledgement");
        }
    } else {
        println!("hit-every-surface: ok");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_gui_and_mcp() {
        assert_eq!(classify("crates/vox-gui/ui/src/App.tsx"), Some("gui"));
        assert_eq!(
            classify("crates/vox-orchestrator-mcp/src/lib.rs"),
            Some("mcp")
        );
        assert_eq!(classify("contracts/ci/foo.yaml"), Some("contracts"));
    }
}
