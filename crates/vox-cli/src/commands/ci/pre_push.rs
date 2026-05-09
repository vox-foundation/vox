//! `vox ci pre-push` — local aggregate that mirrors the merge-blocking CI subset.
//!
//! Runs in order: fmt --check, line-endings, ssot-drift, doc-inventory verify,
//! clippy (workspace, all-targets, -D warnings), scoped TOESTUB (changed paths).
//! `--quick` skips clippy + TOESTUB; `--full` also runs nextest on changed crates.
//! `--act` additionally runs the GitHub-hosted exception workflows through `act`
//! (nektos/act must be on PATH; Docker daemon must be running).

use anyhow::{Context, Result, anyhow, bail};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Clone, Copy)]
pub struct PrePushOpts {
    pub quick: bool,
    pub full: bool,
    pub dry_run: bool,
    /// Run the GH-hosted exception workflows through `act` after the Rust checks.
    pub act: bool,
}

/// Workflows that run on `ubuntu-latest` (GH-hosted exceptions).  These are
/// the only workflows that `act` meaningfully reproduces locally — self-hosted
/// lanes require the real fleet.
const ACT_WORKFLOWS: &[&str] = &[
    ".github/workflows/docs-quality.yml",
    ".github/workflows/link_checker.yml",
    ".github/workflows/ts-emit-noemit.yml",
];

pub fn run(root: &Path, opts: PrePushOpts) -> Result<()> {
    let steps = build_steps(opts);
    if opts.dry_run {
        for s in &steps {
            println!("DRY-RUN: {}", s.label);
        }
        if opts.act {
            run_act(root, true)?;
        }
        return Ok(());
    }
    let total = Instant::now();
    for s in steps {
        let started = Instant::now();
        println!("==> {}", s.label);
        (s.run)(root).with_context(|| format!("step `{}`", s.label))?;
        println!("    OK ({:.1?})", started.elapsed());
    }
    if opts.act {
        run_act(root, false)?;
    }
    println!("pre-push: all checks passed in {:.1?}", total.elapsed());
    Ok(())
}

struct Step {
    label: &'static str,
    run: fn(&Path) -> Result<()>,
}

fn build_steps(opts: PrePushOpts) -> Vec<Step> {
    let mut v = vec![
        Step {
            label: "cargo fmt --all -- --check",
            run: step_fmt,
        },
        Step {
            label: "vox ci line-endings",
            run: step_line_endings,
        },
        Step {
            label: "vox ci ssot-drift",
            run: step_ssot_drift,
        },
    ];
    if !opts.quick {
        v.push(Step {
            label: "vox ci doc-inventory verify",
            run: step_doc_inventory,
        });
        v.push(Step {
            label: "cargo clippy --workspace --all-targets -- -D warnings",
            run: step_clippy,
        });
        v.push(Step {
            label: "vox ci toestub-scoped --mode enforce-warn (changed paths)",
            run: step_toestub_changed,
        });
    }
    if opts.full {
        v.push(Step {
            label: "cargo nextest run --workspace --no-fail-fast",
            run: step_nextest,
        });
    }
    v
}

/// Run the GH-hosted exception workflows through `act`.
///
/// Each workflow is run independently so a failure in one does not suppress
/// output from the others.  All failures are collected and reported together.
pub fn run_act(root: &Path, dry_run: bool) -> Result<()> {
    let act_bin = which_act().context(
        "`act` not found on PATH — install nektos/act (https://nektosact.com) to use --act",
    )?;

    let mut failures: Vec<&str> = Vec::new();
    for &workflow in ACT_WORKFLOWS {
        println!("==> act: {workflow}");
        if dry_run {
            println!("    DRY-RUN: {act_bin} --workflows {workflow} push");
            continue;
        }
        let status = Command::new(&act_bin)
            .args(["push", "--workflows", workflow])
            .current_dir(root)
            .status()
            .with_context(|| format!("spawn act for {workflow}"))?;
        if status.success() {
            println!("    OK");
        } else {
            eprintln!("    FAILED ({workflow}): exit {:?}", status.code());
            failures.push(workflow);
        }
    }
    if !failures.is_empty() {
        bail!(
            "act: {} workflow(s) failed: {}",
            failures.len(),
            failures.join(", ")
        );
    }
    Ok(())
}

/// Locate the `act` binary; returns its path or an error.
fn which_act() -> Result<String> {
    // `act` may be installed as a GitHub CLI extension (`gh act`) or standalone.
    // Prefer the standalone binary; fall back to `gh act`.
    let candidates = ["act", "gh act"];
    for candidate in candidates {
        let parts: Vec<&str> = candidate.split_whitespace().collect();
        if let Ok(out) = Command::new(parts[0])
            .args(&parts[1..])
            .arg("--version")
            .output()
        {
            if out.status.success() {
                return Ok(candidate.to_string());
            }
        }
    }
    Err(anyhow!("act binary not found"))
}

fn cargo() -> Command {
    Command::new(super::cargo_bin())
}

/// Run `cargo` with the given args; bail if it exits non-zero.
fn cargo_status(args: &[&str]) -> Result<()> {
    let status = cargo()
        .args(args)
        .status()
        .with_context(|| format!("spawn cargo {}", args.join(" ")))?;
    if !status.success() {
        bail!("cargo {} exited with {:?}", args.join(" "), status.code());
    }
    Ok(())
}

fn step_fmt(_root: &Path) -> Result<()> {
    cargo_status(&["fmt", "--all", "--", "--check"])
}

fn step_line_endings(_root: &Path) -> Result<()> {
    cargo_status(&["run", "-q", "-p", "vox-cli", "--", "ci", "line-endings"])
}

fn step_ssot_drift(_root: &Path) -> Result<()> {
    cargo_status(&["run", "-q", "-p", "vox-cli", "--", "ci", "ssot-drift"])
}

fn step_doc_inventory(_root: &Path) -> Result<()> {
    cargo_status(&[
        "run",
        "-q",
        "-p",
        "vox-cli",
        "--",
        "ci",
        "doc-inventory",
        "verify",
    ])
}

fn step_clippy(_root: &Path) -> Result<()> {
    cargo_status(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ])
}

fn step_toestub_changed(root: &Path) -> Result<()> {
    // Diff-base failure is a hard error — silently skipping would let
    // pre-push report success without ever running TOESTUB.
    let dirs = changed_dirs_under_crates(root)
        .context("compute changed crate paths for scoped TOESTUB")?;
    if dirs.is_empty() {
        // Real "no work" path: diff succeeded but no crates/ entries changed.
        println!("    (no crate changes vs. base — skipping scoped TOESTUB)");
        return Ok(());
    }
    let mut cmd = cargo();
    cmd.args([
        "run",
        "-q",
        "-p",
        "vox-cli",
        "--",
        "ci",
        "toestub-scoped",
        "--mode",
        "enforce-warn",
    ]);
    for d in &dirs {
        cmd.arg(d);
    }
    let status = cmd.status().context("spawn vox ci toestub-scoped")?;
    if !status.success() {
        bail!("toestub-scoped exited with {:?}", status.code());
    }
    Ok(())
}

fn step_nextest(_root: &Path) -> Result<()> {
    cargo_status(&["nextest", "run", "--workspace", "--no-fail-fast"])
}

/// Return a deduped list of `crates/<crate>` directories that have changes vs.
/// the diff base. Tries `VOX_PREPUSH_BASE` (default `origin/main`) first; if
/// that ref is unknown locally (e.g. shallow clone, no remote), falls back to
/// `HEAD~1`. Returns `Err` only when both attempts fail — callers must treat
/// that as a hard failure rather than "no changes".
fn changed_dirs_under_crates(root: &Path) -> Result<Vec<PathBuf>> {
    let primary = std::env::var("VOX_PREPUSH_BASE").unwrap_or_else(|_| "origin/main".into());
    let attempt = |base: &str| -> Result<String> {
        let out = Command::new("git")
            .args(["diff", "--name-only", &format!("{base}...HEAD")])
            .current_dir(root)
            .output()
            .with_context(|| format!("spawn git diff against {base}"))?;
        if !out.status.success() {
            return Err(anyhow!(
                "git diff against `{base}` failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    };
    let raw = match attempt(&primary) {
        Ok(s) => s,
        Err(primary_err) => {
            eprintln!(
                "pre-push: primary diff base `{primary}` unavailable ({primary_err}); trying HEAD~1"
            );
            attempt("HEAD~1").context("HEAD~1 fallback also failed")?
        }
    };
    let mut seen = std::collections::BTreeSet::new();
    for line in raw.lines() {
        let parts: Vec<&str> = line.splitn(3, '/').collect();
        if parts.len() >= 2 && parts[0] == "crates" {
            seen.insert(PathBuf::from("crates").join(parts[1]));
        }
    }
    Ok(seen.into_iter().collect())
}
