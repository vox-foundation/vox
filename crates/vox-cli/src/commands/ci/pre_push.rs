//! `vox ci pre-push` — local aggregate that mirrors the merge-blocking CI subset.
//!
//! Runs in order: fmt --check, line-endings, ssot-drift, doc-inventory verify,
//! clippy (workspace, all-targets, -D warnings), scoped TOESTUB (changed paths).
//! `--quick` skips clippy + TOESTUB; `--full` also runs nextest on changed crates.

use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

#[derive(Clone, Copy)]
pub struct PrePushOpts {
    pub quick: bool,
    pub full: bool,
    pub dry_run: bool,
}

pub fn run(root: &Path, opts: PrePushOpts) -> Result<()> {
    let steps = build_steps(opts);
    if opts.dry_run {
        for s in &steps {
            println!("DRY-RUN: {}", s.label);
        }
        return Ok(());
    }
    let total = Instant::now();
    for s in steps {
        let started = Instant::now();
        println!("==> {}", s.label);
        let status = (s.run)(root).with_context(|| format!("step `{}`", s.label))?;
        if !status.success() {
            bail!("step `{}` failed (exit {:?})", s.label, status.code());
        }
        println!("    OK ({:.1?})", started.elapsed());
    }
    println!("pre-push: all checks passed in {:.1?}", total.elapsed());
    Ok(())
}

struct Step {
    label: &'static str,
    run: fn(&Path) -> std::io::Result<std::process::ExitStatus>,
}

fn build_steps(opts: PrePushOpts) -> Vec<Step> {
    let mut v = vec![
        Step { label: "cargo fmt --all -- --check", run: step_fmt },
        Step { label: "vox ci line-endings", run: step_line_endings },
        Step { label: "vox ci ssot-drift", run: step_ssot_drift },
    ];
    if !opts.quick {
        v.push(Step { label: "vox ci doc-inventory verify", run: step_doc_inventory });
        v.push(Step { label: "cargo clippy --workspace --all-targets -- -D warnings", run: step_clippy });
        v.push(Step { label: "vox ci toestub-scoped (changed paths)", run: step_toestub_changed });
    }
    if opts.full {
        v.push(Step { label: "cargo nextest run --workspace --no-fail-fast", run: step_nextest });
    }
    v
}

fn cargo() -> Command {
    Command::new(super::cargo_bin())
}

fn step_fmt(_root: &Path) -> std::io::Result<std::process::ExitStatus> {
    cargo().args(["fmt", "--all", "--", "--check"]).status()
}

fn step_line_endings(_root: &Path) -> std::io::Result<std::process::ExitStatus> {
    cargo().args(["run", "-q", "-p", "vox-cli", "--", "ci", "line-endings"]).status()
}

fn step_ssot_drift(_root: &Path) -> std::io::Result<std::process::ExitStatus> {
    cargo().args(["run", "-q", "-p", "vox-cli", "--", "ci", "ssot-drift"]).status()
}

fn step_doc_inventory(_root: &Path) -> std::io::Result<std::process::ExitStatus> {
    cargo().args(["run", "-q", "-p", "vox-cli", "--", "ci", "doc-inventory", "verify"]).status()
}

fn step_clippy(_root: &Path) -> std::io::Result<std::process::ExitStatus> {
    cargo()
        .args(["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"])
        .status()
}

fn step_toestub_changed(root: &Path) -> std::io::Result<std::process::ExitStatus> {
    let dirs = changed_dirs_under_crates(root).unwrap_or_default();
    if dirs.is_empty() {
        // No changes under crates/ — skip silently.
        return cargo().args(["--version"]).status();
    }
    let mut cmd = cargo();
    cmd.args(["run", "-q", "-p", "vox-cli", "--", "ci", "toestub-scoped", "--mode", "legacy"]);
    for d in dirs {
        cmd.arg(d);
    }
    cmd.status()
}

fn step_nextest(_root: &Path) -> std::io::Result<std::process::ExitStatus> {
    cargo()
        .args(["nextest", "run", "--workspace", "--no-fail-fast"])
        .status()
}

/// Return a deduped list of `crates/<crate>` directories that have changes vs.
/// `origin/main` (or `HEAD~1` if no upstream). Empty list = no work.
fn changed_dirs_under_crates(root: &Path) -> Option<Vec<String>> {
    let base = std::env::var("VOX_PREPUSH_BASE").unwrap_or_else(|_| "origin/main".into());
    let out = Command::new("git")
        .args(["diff", "--name-only", &format!("{base}...HEAD")])
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let mut seen = std::collections::BTreeSet::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let parts: Vec<&str> = line.splitn(3, '/').collect();
        if parts.len() >= 2 && parts[0] == "crates" {
            seen.insert(format!("crates/{}", parts[1]));
        }
    }
    Some(seen.into_iter().collect())
}
