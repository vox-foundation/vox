//! GitHub CodeRabbit helpers: semantic / batch PR planning, isolated worktree submits, ingest, tasks.
//!
//! Build with **`--features coderabbit`**. Set **`GITHUB_TOKEN`** or **`GH_TOKEN`**.
//!
//! ## Runbook (short)
//!
//! - **Plan only (no git remotes):** `vox review coderabbit semantic-submit` (omit `--execute`).
//! - **Open PRs:** same with `--execute`. Uses **`origin/<default>`** tip pushed to **`cr-baseline-*`**, then one PR per chunk from a **git worktree** under `.coderabbit/worktrees/` (main repo working tree is not checked out).
//! - **Resume:** `--resume` loads `.coderabbit/run-state.json` and reuses its baseline when you omit `--baseline-branch`; mismatched `--baseline-branch` fails fast. Completed chunks are skipped unless `--force-chunks`.
//! - **Exclusions:** `[review.coderabbit] exclude_prefixes = [\"populi/data/\"]` in `Vox.toml`.
//! - **Legacy broad commit to default branch:** `--commit-main` (stages `git add -u` + manifest paths — avoid unless you intend it).

pub mod config;
pub mod git;
pub mod github;
pub mod ingest;
pub mod limits;
pub mod path_policy;
pub mod planner;
pub mod run_state;
pub mod semantic_planner;
pub mod stack_planner;
pub mod tasks;
pub mod historical_planner;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Subcommand;

/// `vox review coderabbit …` subcommands.
#[derive(Subcommand, Debug)]
pub enum CodeRabbitAction {
    /// Group *changed* files by concern; plan or open independent PRs into a real baseline (recommended).
    ///
    /// By default collects only files that differ from HEAD (diffs + untracked). Pass `--full-repo`
    /// to instead collect **every** tracked file via `git ls-files` for a complete clean-slate review.
    #[command(name = "semantic-submit")]
    SemanticSubmit {
        /// Repo root (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Push baseline + create worktree PRs (default: plan + write manifest only).
        #[arg(long, default_value_t = false)]
        execute: bool,
        /// After baseline, run legacy `git add -u` + commit all manifest paths and push default branch.
        #[arg(long, default_value_t = false)]
        commit_main: bool,
        /// Reuse this baseline branch name (required for `--resume` across runs).
        #[arg(long)]
        baseline_branch: Option<String>,
        /// Merge completed chunks from `.coderabbit/run-state.json` and skip them.
        #[arg(long, default_value_t = false)]
        resume: bool,
        /// Re-run every chunk even if previously completed.
        #[arg(long, default_value_t = false)]
        force_chunks: bool,
        /// Tier: free, trial, oss, pro, enterprise (overrides `Vox.toml`).
        #[arg(long)]
        tier: Option<String>,
        /// Max files per PR before semantic split (clamped to tier cap; overrides `Vox.toml`).
        #[arg(long)]
        max_files_per_pr: Option<usize>,
        /// Seconds between PR triggers (0 = tier default; overrides `Vox.toml`).
        #[arg(long)]
        delay_secs: Option<u64>,
        /// Only keep chunk names containing this substring.
        #[arg(long)]
        group_filter: Option<String>,
        /// Review the **entire codebase** (`git ls-files`) instead of just changed files.
        /// Creates a fresh baseline at current `origin/main` and opens PRs for every tracked file.
        /// Skips the drift check (N/A when reviewing the whole repo).
        #[arg(long, default_value_t = false)]
        full_repo: bool,
    },
    /// Generate stacked PRs comparing a historical commit to the current local state safely.
    #[command(name = "historical-submit")]
    HistoricalSubmit {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Historical commit SHA or ref to use as the baseline.
        #[arg(long)]
        commit_id: String,
        /// Push baseline + create worktree PRs (default: plan + write manifest only).
        #[arg(long, default_value_t = false)]
        execute: bool,
        /// Tier: free, trial, oss, pro, enterprise (overrides `Vox.toml`).
        #[arg(long)]
        tier: Option<String>,
        /// Max files per PR before semantic split (clamped to tier cap; overrides `Vox.toml`).
        #[arg(long)]
        max_files_per_pr: Option<usize>,
        /// Seconds between PR triggers (0 = tier default; overrides `Vox.toml`).
        #[arg(long)]
        delay_secs: Option<u64>,
        /// Only keep chunk names containing this substring.
        #[arg(long)]
        group_filter: Option<String>,
    },
    /// Size-first batches from `git diff` (manifest `.coderabbit-batch-manifest.json`).
    #[command(name = "batch-submit")]
    BatchSubmit {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value_t = 250)]
        max_files_per_pr: u32,
        #[arg(long, default_value_t = 300)]
        hard_cap: u32,
        /// Tier for cap clamping (overrides `Vox.toml`; default: Pro).
        #[arg(long)]
        tier: Option<String>,
        #[arg(long)]
        base_ref: Option<String>,
        /// Write manifest (default: dry-run print only).
        #[arg(long, default_value_t = false)]
        execute: bool,
    },
    /// Full-repo stack planner (chains PRs on orphan baseline — destructive to checkout; prefer `semantic-submit`).
    #[command(name = "stack-submit")]
    StackSubmit {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value_t = 50)]
        max_files_per_pr: u32,
        #[arg(long)]
        tier: Option<String>,
        #[arg(long)]
        delay_secs: Option<u64>,
        #[arg(long, default_value_t = false)]
        execute: bool,
    },
    /// Single-branch submit (`git checkout -b` in **this** repo — prefer `semantic-submit` for safety).
    Submit {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        branch: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long, default_value_t = false)]
        no_commit: bool,
    },
    /// Fetch PR review comments and emit normalized JSON.
    Ingest {
        pr: u64,
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        persist: bool,
    },
    /// Build a markdown/json task checklist from a PR's CodeRabbit comments.
    Tasks {
        pr: u64,
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "markdown")]
        format: String,
        #[arg(long, default_value_t = false)]
        persist: bool,
    },
    /// Poll until CodeRabbit posts a review on the PR.
    Wait {
        pr: u64,
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value_t = 3600)]
        timeout_secs: u64,
    },
}

fn resolve_repo(path: &PathBuf) -> Result<std::path::PathBuf> {
    if path.as_os_str().is_empty() || path.as_path() == Path::new(".") {
        return std::env::current_dir().context("current_dir");
    }
    path.canonicalize()
        .with_context(|| format!("canonicalize {}", path.display()))
}

/// Dispatch `vox review coderabbit …`.
pub async fn run(action: CodeRabbitAction) -> Result<()> {
    match action {
        CodeRabbitAction::SemanticSubmit {
            path,
            execute,
            commit_main,
            baseline_branch,
            resume,
            force_chunks,
            tier,
            max_files_per_pr,
            delay_secs,
            group_filter,
            full_repo,
        } => {
            let repo = resolve_repo(&path)?;
            let vox = config::load_from_dir(&repo);
            let mut cfg = semantic_planner::SemanticSubmitConfig::default();
            if let Some(ref t) = vox.tier {
                if let Ok(parsed) = t.parse::<limits::CodeRabbitTier>() {
                    cfg.tier = parsed;
                }
            }
            if let Some(t) = tier {
                cfg.tier = t
                    .parse::<limits::CodeRabbitTier>()
                    .map_err(|e| anyhow::anyhow!(e))?;
            }
            if let Some(d) = vox.delay_between_prs_secs {
                cfg.delay_secs = d;
            }
            if let Some(d) = delay_secs {
                cfg.delay_secs = d;
            }
            if let Some(m) = vox.max_files_per_pr {
                cfg.max_files_per_pr = m as usize;
            }
            if let Some(m) = max_files_per_pr {
                cfg.max_files_per_pr = m;
            }
            cfg.execute = execute;
            cfg.commit_main = commit_main;
            cfg.baseline_branch = baseline_branch;
            cfg.resume = resume;
            cfg.force_chunks = force_chunks;
            cfg.group_filter = group_filter;
            cfg.full_repo = full_repo;
            semantic_planner::run_semantic_submit(&repo, &cfg).await?;
        }
        CodeRabbitAction::HistoricalSubmit {
            path,
            commit_id,
            execute,
            tier,
            max_files_per_pr,
            delay_secs,
            group_filter,
        } => {
            let repo = resolve_repo(&path)?;
            let vox = config::load_from_dir(&repo);
            let mut cfg = semantic_planner::SemanticSubmitConfig::default();
            if let Some(ref t) = vox.tier {
                if let Ok(parsed) = t.parse::<limits::CodeRabbitTier>() {
                    cfg.tier = parsed;
                }
            }
            if let Some(t) = tier {
                cfg.tier = t.parse::<limits::CodeRabbitTier>().map_err(|e| anyhow::anyhow!(e))?;
            }
            if let Some(d) = vox.delay_between_prs_secs {
                cfg.delay_secs = d;
            }
            if let Some(d) = delay_secs {
                cfg.delay_secs = d;
            }
            if let Some(m) = vox.max_files_per_pr {
                cfg.max_files_per_pr = m as usize;
            }
            if let Some(m) = max_files_per_pr {
                cfg.max_files_per_pr = m;
            }
            cfg.execute = execute;
            cfg.group_filter = group_filter;
            historical_planner::run_historical_submit(&repo, &commit_id, &cfg).await?;
        }
        CodeRabbitAction::BatchSubmit {
            path,
            max_files_per_pr,
            hard_cap,
            tier,
            base_ref,
            execute,
        } => {
            let repo = resolve_repo(&path)?;
            let vox = config::load_from_dir(&repo);
            let tier_e = tier
                .as_deref()
                .or(vox.tier.as_deref())
                .and_then(|s| s.parse::<limits::CodeRabbitTier>().ok())
                .unwrap_or(limits::CodeRabbitTier::Pro);
            let (max_c, hard_c) = limits::clamp_batch_caps(tier_e, max_files_per_pr, hard_cap);
            if max_c != max_files_per_pr || hard_c != hard_cap {
                eprintln!(
                    "[batch-submit] Tier {tier_e}: clamped max_files_per_pr {max_files_per_pr}→{max_c}, hard_cap {hard_cap}→{hard_c}"
                );
            }
            planner::run_batch_submit(&repo, max_c, hard_c, base_ref.as_deref(), !execute).await?;
        }
        CodeRabbitAction::StackSubmit {
            path,
            max_files_per_pr,
            tier,
            delay_secs,
            execute,
        } => {
            let repo = resolve_repo(&path)?;
            let vox = config::load_from_dir(&repo);
            let tier_s = tier.as_deref().or(vox.tier.as_deref());
            let tier_e = tier_s
                .and_then(|s| s.parse::<limits::CodeRabbitTier>().ok())
                .unwrap_or(limits::CodeRabbitTier::Pro);
            let max_c = limits::clamp_max_files_per_pr(tier_e, max_files_per_pr);
            if max_c != max_files_per_pr {
                eprintln!(
                    "[stack-submit] Tier {tier_e}: clamped max_files_per_pr {max_files_per_pr}→{max_c}"
                );
            }
            let delay = delay_secs
                .or(vox.delay_between_prs_secs)
                .unwrap_or_else(|| tier_e.min_delay_between_prs_secs());
            stack_planner::run_stack_submit(&repo, max_c, tier_s, delay, !execute).await?;
        }
        CodeRabbitAction::Submit {
            path,
            branch,
            title,
            no_commit,
        } => {
            let repo = resolve_repo(&path)?;
            github::submit(&repo, branch.as_deref(), title.as_deref(), no_commit).await?;
        }
        CodeRabbitAction::Ingest {
            pr,
            path,
            output,
            persist,
        } => {
            let repo = resolve_repo(&path)?;
            ingest::run_ingest(pr, output.as_deref(), persist, &repo).await?;
        }
        CodeRabbitAction::Tasks {
            pr,
            path,
            format,
            persist,
        } => {
            let repo = resolve_repo(&path)?;
            tasks::run_tasks(Some(pr), &format, persist, &repo).await?;
        }
        CodeRabbitAction::Wait {
            pr,
            path,
            timeout_secs,
        } => {
            let repo = resolve_repo(&path)?;
            github::wait_for_review(pr, timeout_secs, &repo).await?;
        }
    }
    Ok(())
}
