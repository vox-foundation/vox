//! `vox ci pre-push` — local aggregate that mirrors the merge-blocking CI subset.
//!
//! Runs in order: fmt --check, line-endings, ssot-drift, doc frontmatter lint,
//! doctest-md extraction, doc-inventory verify, workspace drift check,
//! clippy (workspace, all-targets, -D warnings), scoped TOESTUB (changed paths).
//! `--quick` skips clippy + TOESTUB; `--full` also runs workspace **`cargo nextest`**
//! with **`--profile ci`** (same profile as GitHub `ci.yml` tests job — timeouts/retries).
//! `--act` additionally runs the GitHub-hosted exception workflows through `act`
//! (nektos/act must be on PATH; Docker daemon must be running).
//!
//! Previously the pre-push did not run doc frontmatter lint, doctest extraction,
//! or the workspace drift check — those only ran in CI.  The gap meant a green
//! pre-push could still produce a red docs-quality job.  All three are now always
//! included (they're fast; drift-check may take a few seconds on large trees).

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
        // Doc checks: match the docs-quality CI job so a green pre-push
        // implies a green docs-quality workflow.  Both are fast (<10 s total).
        Step {
            label: "vox-doc-pipeline --lint-only (frontmatter + code fences)",
            run: step_doc_frontmatter,
        },
        Step {
            label: "vox ci doctest-md --strict",
            run: step_doctest_md,
        },
        // Workspace drift check: was pre-push only via lefthook; now also
        // mirrored here so `vox ci pre-push` covers it even without lefthook.
        Step {
            label: "vox-drift-check workspace",
            run: step_drift_check,
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
            label: "cargo nextest run --workspace --profile ci --no-fail-fast",
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
    let act_cmd = if dry_run {
        which_act().unwrap_or_else(|_| ActCommand::new("act", vec![]))
    } else {
        which_act().context(
            "`act` not found on PATH — install nektos/act (https://nektosact.com) to use --act",
        )?
    };

    let mut failures: Vec<&str> = Vec::new();
    for &workflow in ACT_WORKFLOWS {
        println!("==> act: {workflow}");
        if dry_run {
            println!(
                "    DRY-RUN: {}",
                act_cmd.display_with_args(&["push", "--workflows", workflow])
            );
            continue;
        }
        let status = Command::new(&act_cmd.executable)
            .args(&act_cmd.base_args)
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
fn which_act() -> Result<ActCommand> {
    // `act` may be installed as a GitHub CLI extension (`gh act`) or standalone.
    // Prefer the standalone binary; fall back to `gh act`.
    let candidates = [
        ActCommand::new("act", vec![]),
        ActCommand::new("gh", vec!["act"]),
    ];
    for candidate in candidates {
        if let Ok(out) = Command::new(&candidate.executable)
            .args(&candidate.base_args)
            .arg("--version")
            .output()
        {
            if out.status.success() {
                return Ok(candidate);
            }
        }
    }
    Err(anyhow!("act binary not found"))
}

#[derive(Clone, Debug)]
struct ActCommand {
    executable: String,
    base_args: Vec<String>,
}

impl ActCommand {
    fn new(executable: &str, base_args: Vec<&str>) -> Self {
        Self {
            executable: executable.to_string(),
            base_args: base_args.into_iter().map(ToString::to_string).collect(),
        }
    }

    fn display_with_args(&self, runtime_args: &[&str]) -> String {
        let mut parts = Vec::with_capacity(1 + self.base_args.len() + runtime_args.len());
        parts.push(self.executable.clone());
        parts.extend(self.base_args.iter().cloned());
        parts.extend(runtime_args.iter().map(|arg| (*arg).to_string()));
        parts.join(" ")
    }
}

fn cargo() -> Command {
    Command::new(super::cargo_bin())
}

/// Run `cargo` with the given args; bail if it exits non-zero.
fn cargo_status(root: &Path, args: &[&str]) -> Result<()> {
    let status = cargo()
        .args(args)
        .current_dir(root)
        .status()
        .with_context(|| format!("spawn cargo {}", args.join(" ")))?;
    if !status.success() {
        bail!("cargo {} exited with {:?}", args.join(" "), status.code());
    }
    Ok(())
}

fn step_fmt(root: &Path) -> Result<()> {
    cargo_status(root, &["fmt", "--all", "--", "--check"])
}

fn step_line_endings(root: &Path) -> Result<()> {
    cargo_status(root, &["run", "-q", "-p", "vox-cli", "--", "ci", "line-endings"])
}

fn step_ssot_drift(root: &Path) -> Result<()> {
    cargo_status(root, &["run", "-q", "-p", "vox-cli", "--", "ci", "ssot-drift"])
}

fn step_doc_inventory(root: &Path) -> Result<()> {
    cargo_status(root, &[
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

fn step_clippy(root: &Path) -> Result<()> {
    cargo_status(root, &[
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
    let status = cmd
        .current_dir(root)
        .status()
        .context("spawn vox ci toestub-scoped")?;
    if !status.success() {
        bail!("toestub-scoped exited with {:?}", status.code());
    }
    Ok(())
}

fn step_doc_frontmatter(root: &Path) -> Result<()> {
    cargo_status(root, &["run", "-q", "-p", "vox-doc-pipeline", "--", "--lint-only"])
}

fn step_doctest_md(root: &Path) -> Result<()> {
    cargo_status(root, &[
        "run",
        "-q",
        "-p",
        "vox-cli",
        "--",
        "ci",
        "doctest-md",
        "--strict",
    ])
}

fn step_drift_check(root: &Path) -> Result<()> {
    // Mirror the lefthook pre-push drift-check and the CI lints job step so
    // `vox ci pre-push` is the authoritative local gate.
    let status = std::process::Command::new(super::cargo_bin())
        .args([
            "run",
            "-q",
            "-p",
            "vox-drift-check",
            "--",
            ".",
            "--severity",
            "warning",
            "--fail-on",
            "warning",
        ])
        .current_dir(root)
        .status()
        .context("spawn vox-drift-check")?;
    if !status.success() {
        bail!("vox-drift-check exited with {:?}", status.code());
    }
    Ok(())
}

fn step_nextest(root: &Path) -> Result<()> {
    cargo_status(root, &[
        "nextest",
        "run",
        "--workspace",
        "--profile",
        "ci",
        "--no-fail-fast",
    ])
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
