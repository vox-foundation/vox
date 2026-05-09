//! `vox audit` — umbrella command that iterates quality gates defined in
//! `contracts/ci/check-targets.v1.yaml`.
//!
//! # Usage
//!
//! ```text
//! vox audit                         # run all checks
//! vox audit --category lint         # run only lint checks
//! vox audit --list                  # print check IDs without running
//! vox audit --dry-run               # print commands without executing
//! ```
//!
//! The manifest at `contracts/ci/check-targets.v1.yaml` is the SSOT for every
//! quality gate.  See that file for per-check metadata: `blocking`, `runs_on`,
//! `rust_only`, and `command`.

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// Manifest types
// ---------------------------------------------------------------------------

/// Top-level structure of `contracts/ci/check-targets.v1.yaml`.
#[derive(Debug, Deserialize)]
pub struct CheckManifest {
    pub schema_version: u32,
    pub checks: Vec<CheckEntry>,
}

/// One quality-gate entry in the manifest.
#[derive(Debug, Deserialize, Clone)]
pub struct CheckEntry {
    pub id: String,
    pub description: String,
    pub category: String,
    pub blocking: bool,
    pub runs_on: Vec<String>,
    #[serde(default)]
    pub rust_only: bool,
    pub command: Vec<String>,
    /// When `true` this check is skipped by `--quick`.
    #[serde(default)]
    pub quick_skip: bool,
}

// ---------------------------------------------------------------------------
// CLI args
// ---------------------------------------------------------------------------

/// Run quality-gate checks defined in `contracts/ci/check-targets.v1.yaml`.
#[derive(Args, Debug, Clone)]
pub struct AuditArgs {
    /// Filter to a specific category (lint, test, audit, doc, arch).
    #[arg(long, value_name = "CATEGORY")]
    pub category: Option<String>,

    /// Print all matching check IDs and exit without running anything.
    #[arg(long)]
    pub list: bool,

    /// Print each command without executing it.
    #[arg(long)]
    pub dry_run: bool,

    /// Skip checks marked `quick_skip: true` in the manifest.
    #[arg(long)]
    pub quick: bool,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Entry point called from the CLI dispatch.
pub fn run(args: &AuditArgs) -> Result<()> {
    let root = vox_repository::resolve_repo_root_for_ci();
    let manifest = load_manifest(&root)?;
    let checks = filter_checks(&manifest.checks, args);

    if args.list {
        for check in &checks {
            println!("{}: {} [{}]", check.id, check.description, check.category);
        }
        return Ok(());
    }

    for check in &checks {
        println!("==> {}: {}", check.id, check.description);
        if args.dry_run {
            println!("    DRY-RUN: {}", check.command.join(" "));
            continue;
        }
        run_check(check, &root)?;
        println!("    OK");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Load and parse the manifest from `<root>/contracts/ci/check-targets.v1.yaml`.
fn load_manifest(root: &Path) -> Result<CheckManifest> {
    let path = root
        .join("contracts")
        .join("ci")
        .join("check-targets.v1.yaml");
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("read manifest at {}", path.display()))?;
    let manifest: CheckManifest =
        serde_yaml::from_str(&content).with_context(|| "parse check-targets.v1.yaml")?;
    if manifest.schema_version != 1 {
        bail!(
            "unsupported manifest schema_version {}; expected 1",
            manifest.schema_version
        );
    }
    Ok(manifest)
}

/// Return the subset of checks that match the supplied CLI filters.
fn filter_checks<'a>(checks: &'a [CheckEntry], args: &AuditArgs) -> Vec<&'a CheckEntry> {
    checks
        .iter()
        .filter(|c| {
            if let Some(ref cat) = args.category {
                if c.category != *cat {
                    return false;
                }
            }
            if args.quick && c.quick_skip {
                return false;
            }
            true
        })
        .collect()
}

/// Execute a single check; bail on non-zero exit.
fn run_check(check: &CheckEntry, root: &Path) -> Result<()> {
    let (program, rest) = check
        .command
        .split_first()
        .context("check command is empty")?;

    let status = Command::new(program)
        .args(rest)
        .current_dir(root)
        .status()
        .with_context(|| format!("spawn command for check `{}`", check.id))?;

    if !status.success() {
        bail!(
            "check `{}` failed with exit code {:?}",
            check.id,
            status.code()
        );
    }
    Ok(())
}

/// Walk parent directories from `start` until a `Cargo.toml` is found.
/// Returns `None` if the filesystem root is reached without finding one.
///
/// This is a lightweight fallback for environments where
/// `vox_repository::resolve_repo_root_for_ci` is unavailable.
#[allow(dead_code)]
fn find_cargo_root(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        if dir.join("Cargo.toml").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// The manifest file must be present and parse without error.
    #[test]
    fn manifest_parses_from_repo_root() {
        // Walk from the manifest file's known location relative to this source
        // file: crates/vox-cli/src/commands/audit.rs → repo root is 5 levels up.
        let src = std::path::Path::new(file!());
        // file!() gives a relative path from repo root; convert to absolute.
        let abs = std::env::current_dir()
            .expect("cwd")
            .join(src)
            .canonicalize()
            .unwrap_or_else(|_| std::path::PathBuf::from(file!()));

        // Walk upward to find Cargo.toml (workspace root).
        let mut root = abs.as_path();
        let manifest_path = loop {
            let candidate = root.join("contracts/ci/check-targets.v1.yaml");
            if candidate.exists() {
                break candidate;
            }
            match root.parent() {
                Some(p) => root = p,
                None => {
                    // If we can't find it (e.g. in a detached worktree without the
                    // contracts dir), skip the test rather than fail.
                    eprintln!("skip: contracts/ci/check-targets.v1.yaml not found from {abs:?}");
                    return;
                }
            }
        };

        let content = std::fs::read_to_string(&manifest_path).expect("read check-targets.v1.yaml");
        let manifest: CheckManifest =
            serde_yaml::from_str(&content).expect("parse check-targets.v1.yaml");
        assert_eq!(manifest.schema_version, 1, "schema_version must be 1");
        assert!(
            !manifest.checks.is_empty(),
            "manifest must list at least one check"
        );
        // All checks must have non-empty ids and commands.
        for check in &manifest.checks {
            assert!(!check.id.is_empty(), "check id must not be empty");
            assert!(
                !check.command.is_empty(),
                "check `{}` command must not be empty",
                check.id
            );
        }
    }

    /// `filter_checks` by category must return only matching entries.
    #[test]
    fn filter_by_category() {
        let checks = vec![
            CheckEntry {
                id: "fmt".into(),
                description: "fmt".into(),
                category: "lint".into(),
                blocking: true,
                runs_on: vec!["ci".into()],
                rust_only: true,
                command: vec!["cargo".into(), "fmt".into()],
                quick_skip: false,
            },
            CheckEntry {
                id: "arch-check".into(),
                description: "arch".into(),
                category: "arch".into(),
                blocking: true,
                runs_on: vec!["ci".into()],
                rust_only: true,
                command: vec!["cargo".into(), "run".into()],
                quick_skip: false,
            },
        ];

        let args = AuditArgs {
            category: Some("lint".into()),
            list: false,
            dry_run: false,
            quick: false,
        };
        let filtered = filter_checks(&checks, &args);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "fmt");
    }

    /// `filter_checks` with `--quick` skips `quick_skip: true` entries.
    #[test]
    fn filter_quick_skip() {
        let checks = vec![
            CheckEntry {
                id: "slow".into(),
                description: "slow check".into(),
                category: "test".into(),
                blocking: false,
                runs_on: vec!["ci".into()],
                rust_only: true,
                command: vec!["cargo".into(), "test".into()],
                quick_skip: true,
            },
            CheckEntry {
                id: "fast".into(),
                description: "fast check".into(),
                category: "lint".into(),
                blocking: true,
                runs_on: vec!["ci".into()],
                rust_only: false,
                command: vec!["cargo".into(), "fmt".into()],
                quick_skip: false,
            },
        ];

        let args = AuditArgs {
            category: None,
            list: false,
            dry_run: false,
            quick: true,
        };
        let filtered = filter_checks(&checks, &args);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "fast");
    }
}
