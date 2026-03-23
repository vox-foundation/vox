//! Semantic stacked-PR planner for CodeRabbit — **working-tree diff edition**.
//!
//! Unlike [`super::stack_planner`] (which reads `git ls-files` and groups the whole repo),
//! this planner reads `git diff HEAD --name-status` plus untracked `git status` entries to
//! discover only what has *changed* since the last push, then groups those files by coherent
//! semantic context.
//!
//! # Workflow
//!
//! 1. **Collect** – gather every modified/deleted tracked file and every untracked new file,
//!    filtering out build artifacts, generated HTML, scratch logs, and lock files.
//! 2. **Group** – map each file path to one of the [`SEMANTIC_GROUPS`] in a deterministic,
//!    context-preserving order (e.g. all `crates/vox-typeck/` files go in the typeck PR).
//! 3. **Baseline** – push `refs/heads/cr-baseline-*` to the same commit as `origin/<default_branch>`
//!    (after `git fetch`) so every PR has a real merge base.
//! 4. **Optional `--commit-main`** – legacy path: broad `git add -u` + manifest paths, commit, push default branch.
//! 5. **Worktrees + PR** – for each semantic group, add a git worktree from `origin/<baseline>`,
//!    overlay changed files from the main working tree, commit, push `cr/review-<name>`, open PR **into baseline**.
//!
//! Each PR targets the same baseline branch (independent topology). The main checkout is not switched for chunk work.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::limits;
use super::path_policy;

// ──────────────────────────────────────────────
// Constants
// ──────────────────────────────────────────────

/// Files per PR soft target (stays comfortably under 300-file Pro cap).
const DEFAULT_MAX_FILES_PER_PR: usize = 250;

/// Directories / filename suffixes that should never land in a review PR.
///
/// The `"target-"` prefix catches any `target-<name>/` variation (target-agent/, target-doc-inv2/,
/// target-ci/, etc.), mirroring the `.gitignore` `target-*/` rule.
static IGNORED_DIRS: &[&str] = &[
    "target/",
    "target-",   // catches target-agent/, target-doc-inv2/, target-ci/, target-toestub/, etc.
    "target_",   // catches target_debug/, target_release/, etc.
    ".cargo-targets/",
    "docs/book/",          // generated mdBook HTML (in .gitignore but sometimes tracked)
    ".vox-research-data/", // local SQLite cache
    ".vox/cache/",
    ".gemini/",
    ".cursor/",
    "node_modules/",
    "dist/",
    "dist1/",
    ".next/",
    "tmp_tools/",      // temporary scaffolding directories
    "vendor/",
    "vox-vscode/out/",
    // CodeRabbit tooling — worktrees, run-state, manifests.
    // Without this, a bare `.coderabbit` entry in the manifest triggers copy_dir_recursive
    // which recurses inside worktrees and hits Windows OS error 206 (path too long).
    ".coderabbit/",
    ".coderabbit",
];


static IGNORED_EXTENSIONS: &[&str] = &[
    ".db", ".db-wal", ".db-shm", ".png", ".jpg", ".jpeg", ".webp", ".ico", ".dll", ".exe", ".so",
    ".dylib", ".bin", ".svg", ".woff", ".woff2",
    ".csv", // data export / error dumps (errors.csv etc.) — not source code
    ".md",  // documentation — excluded from code review PRs
    ".txt", // text files — excluded from code review PRs
    ".hbs", // handlebars templates — excluded from code review PRs
];

/// Root-level files (no `/`) that are always excluded — scratch/agent artefacts.
static IGNORED_ROOT_EXACT: &[&str] = &[
    "Cargo.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    // known agent-generated scratch scripts
    "autofix.py",
    "check_lex.py",
    "debug_offset.py",
    "debug_sync.py",
    "find_missing_cli_paths.py",
    "gen_json.py",
    "list_tools.rs",
    "parse_toestub.py",
    "patch2.py",
    "summary.py",
    "sync_registry_final.py",
    "sync_registry_v2.py",
    "sync_registry_v3.py",
    "sync_registry.py",
    "test_clone.rs",
    "test_lexer.rs",
    "test_parse.rs",
    "tmp_parse_check.rs",
    "update_tool_entries.py",
    "update_trust_policy.py",
];

/// Root-level file suffix patterns that are excluded.
static IGNORED_ROOT_PATTERNS: &[&str] = &[
    ".log", ".txt", // only matched for root-level files (no slash)
    ".py",  // root-level Python scripts are scratch; sub-directory ones go to their group
];

/// (order, group_name, inclusion_test_as_path_prefix_or_suffix)
///
/// Order matters: lower numbers appear in earlier PRs.
/// Each group is ≤ DEFAULT_MAX_FILES_PER_PR files; oversized groups are sub-divided.
static SEMANTIC_GROUPS: &[(u32, &str, SemanticMatcher)] = &[
    // ── Infrastructure ──────────────────────────────────────────────────────
    (
        10,
        "02_github_agents",
        SemanticMatcher::Prefix(&[".github/", ".agents/"]),
    ),
    (
        15,
        "03_dotfiles_config",
        SemanticMatcher::Prefix(&[
            ".cargo/",
            ".config/",
            ".gitlab-ci.yml",
            ".gitignore",
            ".aiignore",
            ".cursorignore",
            ".cursorrules",
            ".dockerignore",
            ".git-hooks/",
            "config/",
            "rust-toolchain.toml",
        ]),
    ),
    (
        17,
        "04_opencode_retire",
        SemanticMatcher::Prefix(&[".opencode/"]),
    ),
    // Root-level files (no path separator): Cargo.toml, README.md, AGENTS.md, CLAUDE.md, etc.
    // Must come AFTER the dotfile/prefix groups so .gitignore lands in 03_dotfiles_config.
    (19, "01_scaffold", SemanticMatcher::RootFile),
    (25, "05_contracts", SemanticMatcher::Prefix(&["contracts/"])),
    // ── Docs ────────────────────────────────────────────────────────────────
    (30, "06_docs_src", SemanticMatcher::Prefix(&["docs/src/"])),
    (35, "07_docs_other", SemanticMatcher::Prefix(&["docs/"])),
    // ── Frontend / UI ───────────────────────────────────────────────────────
    (
        40,
        "08_frontend",
        SemanticMatcher::Prefix(&["frontend/", "islands/", "packages/"]),
    ),
    (45, "09_examples", SemanticMatcher::Prefix(&["examples/"])),
    (
        50,
        "10_vscode_ext",
        SemanticMatcher::Prefix(&["vox-vscode/"]),
    ),
    // ── Scripts / Tooling ───────────────────────────────────────────────────
    (
        55,
        "11_scripts_xtask",
        SemanticMatcher::Prefix(&[
            "scripts/",
            "xtask/",
            "fuzz/",
            "infra/",
            "tree-sitter-vox/",
            "tools/",
            "editors/",
        ]),
    ),
    // ── ML / Populi ─────────────────────────────────────────────────────────
    (60, "12_populi_ml", SemanticMatcher::Prefix(&["populi/"])),
    // ── Tests / Fixtures ────────────────────────────────────────────────────
    (65, "13_tests", SemanticMatcher::Prefix(&["tests/"])),
    // ── Compiler front-end ──────────────────────────────────────────────────
    (
        70,
        "14_crate_parser_lexer",
        SemanticMatcher::Prefix(&["crates/vox-parser/", "crates/vox-lexer/", "crates/vox-ast/"]),
    ),
    (
        75,
        "15_crate_hir",
        SemanticMatcher::Prefix(&["crates/vox-hir/"]),
    ),
    (
        80,
        "16_crate_typeck",
        SemanticMatcher::Prefix(&["crates/vox-typeck/"]),
    ),
    // ── Code generation ─────────────────────────────────────────────────────
    (
        85,
        "17_crate_codegen",
        SemanticMatcher::Prefix(&[
            "crates/vox-codegen-rust/",
            "crates/vox-ssg/",
            "crates/vox-codegen-ts/",
            "crates/vox-codegen-common/",
            "crates/vox-codegen-ts-sdk/",
        ]),
    ),
    // ── Runtime / LSP / Tooling crates ──────────────────────────────────────
    (
        90,
        "18_crate_runtime_lsp",
        SemanticMatcher::Prefix(&[
            "crates/vox-runtime/",
            "crates/vox-lsp/",
            "crates/vox-dap/",
            "crates/vox-fmt/",
            "crates/vox-wasm/",
            "crates/vox-toestub/",
            "crates/vox-doc-pipeline/",
        ]),
    ),
    // ── Agent / MCP ─────────────────────────────────────────────────────────
    (
        95,
        "19_crate_mcp_dei",
        SemanticMatcher::Prefix(&[
            "crates/vox-mcp/",
            "crates/vox-dei/",
            "crates/vox-codex/",
            "crates/vox-capability-registry/",
            "crates/vox-codex-api/",
        ]),
    ),
    // ── CLI (big, always last of must-haves) ────────────────────────────────
    (
        100,
        "20_crate_cli",
        SemanticMatcher::Prefix(&["crates/vox-cli/"]),
    ),
    // ── All remaining crates (catch-all) ────────────────────────────────────
    (105, "21_crate_other", SemanticMatcher::Prefix(&["crates/"])),
    // ── Deployment / generated outputs / static assets ──────────────────────
    (
        110,
        "22_deploy_outputs",
        SemanticMatcher::Prefix(&["deploy/", "dist/", "dist1/", "lib/", "static/"]),
    ),
];

// ──────────────────────────────────────────────
// SemanticMatcher helper
// ──────────────────────────────────────────────

/// Pattern type for semantic group matching.
#[derive(Copy, Clone)]
pub enum SemanticMatcher {
    /// File path starts with any of the given prefixes.
    Prefix(&'static [&'static str]),
    /// File has no directory separator (root-level file: Cargo.toml, README.md, etc.).
    RootFile,
}

impl SemanticMatcher {
    fn matches(&self, path: &str) -> bool {
        match self {
            SemanticMatcher::Prefix(prefixes) => prefixes.iter().any(|p| path.starts_with(p)),
            SemanticMatcher::RootFile => {
                // True only for files directly in the repo root (no '/' in path)
                !path.contains('/')
            }
        }
    }
}

// ──────────────────────────────────────────────
// Data types
// ──────────────────────────────────────────────

/// A single named semantic group of files (≤ max_files_per_pr).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticChunk {
    /// Ordering key (lower = earlier PR).
    pub order: u32,
    /// Human-readable group name (stable, used for branch names).
    pub name: String,
    /// Relative repository paths.
    pub files: Vec<String>,
}

/// Manifest produced by [`SemanticPlanner::plan`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticManifest {
    pub generated_at: String,
    pub baseline_branch: String,
    pub total_files: usize,
    pub chunks: Vec<SemanticChunk>,
}

// ──────────────────────────────────────────────
// SemanticPlanner
// ──────────────────────────────────────────────

/// Diff-based semantic PR planner.
pub struct SemanticPlanner {
    max_files_per_pr: usize,
}

impl SemanticPlanner {
    pub fn new(max_files_per_pr: usize) -> Self {
        Self { max_files_per_pr }
    }

    /// Returns `true` if the file should be excluded from review PRs.
    pub fn is_ignored(path: &str) -> bool {
        let p = path.replace('\\', "/");

        if path_policy::is_coderabbit_local_tool_path(&p) {
            return true;
        }

        if IGNORED_DIRS.iter().any(|d| p.starts_with(d)) {
            return true;
        }
        if IGNORED_EXTENSIONS.iter().any(|e| p.ends_with(e)) {
            return true;
        }
        // Root-level scratch patterns and exact names (no directory component)
        if !p.contains('/') {
            if IGNORED_ROOT_EXACT.contains(&p.as_str()) {
                return true;
            }
            if IGNORED_ROOT_PATTERNS.iter().any(|pat| p.ends_with(pat)) {
                return true;
            }
        }
        false
    }

    /// Map a file path to its `(order, group_name)`.
    pub fn get_group(path: &str) -> (u32, &'static str) {
        let p = path.replace('\\', "/");
        for (order, name, matcher) in SEMANTIC_GROUPS {
            if matcher.matches(&p) {
                return (*order, name);
            }
        }
        (199, "99_unassigned")
    }

    /// Plan semantic chunks from a list of files.
    ///
    /// Large groups are sub-divided into `_part1`, `_part2`, … ensuring no chunk
    /// exceeds [`SemanticPlanner::max_files_per_pr`].
    pub fn plan(&self, files: Vec<String>, baseline_branch: &str) -> SemanticManifest {
        use std::collections::BTreeMap;

        let mut groups: BTreeMap<(u32, &'static str), Vec<String>> = BTreeMap::new();
        let mut total = 0usize;

        for f in files {
            if Self::is_ignored(&f) {
                continue;
            }
            total += 1;
            let (order, name) = Self::get_group(&f);
            groups.entry((order, name)).or_default().push(f);
        }

        let mut chunks: Vec<SemanticChunk> = Vec::new();
        for ((order, name), mut group_files) in groups {
            group_files.sort(); // stable, alphabetical
            if group_files.len() <= self.max_files_per_pr {
                chunks.push(SemanticChunk {
                    order,
                    name: name.to_string(),
                    files: group_files,
                });
            } else {
                // Sub-divide
                for (i, batch) in group_files.chunks(self.max_files_per_pr).enumerate() {
                    chunks.push(SemanticChunk {
                        order,
                        name: format!("{}_part{}", name, i + 1),
                        files: batch.to_vec(),
                    });
                }
            }
        }

        chunks.sort_by_key(|c| c.order);

        SemanticManifest {
            generated_at: chrono::Utc::now().to_rfc3339(),
            baseline_branch: baseline_branch.to_string(),
            total_files: total,
            chunks,
        }
    }
}

// ──────────────────────────────────────────────
// File collection
// ──────────────────────────────────────────────

/// Gather all files that differ from HEAD *or* are untracked new files.
///
/// - Modified/deleted tracked files come from `git diff HEAD --name-only`
/// - New files (untracked) come from `git status --short` (`??` prefix)
///
/// Returns deduplicated, forward-slash paths relative to repo root.
pub async fn collect_changed_files(repo: &Path) -> Result<Vec<String>> {
    // Resolve relative paths (e.g. ".") to absolute without using canonicalize(),
    // which produces UNC-style paths on Windows that break some git invocations.
    let cwd = std::env::current_dir().context("get current directory")?;
    let normalized: std::path::PathBuf = if repo.is_absolute() {
        repo.components()
            .filter(|c| !matches!(c, std::path::Component::CurDir))
            .collect()
    } else {
        cwd.join(repo)
            .components()
            .filter(|c| !matches!(c, std::path::Component::CurDir))
            .collect()
    };
    let repo = normalized.as_path();

    // 1. Tracked modifications (modified/deleted tracked files)
    // -c core.autocrlf=false suppresses CRLF warnings that fill the stderr pipe and deadlock .output()
    let diff_out = tokio::process::Command::new("git")
        .args([
            "-c",
            "core.autocrlf=false",
            "diff",
            "HEAD",
            "--name-only",
            "--diff-filter=ACDMRT",
        ])
        .current_dir(repo)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null()) // ← prevent CRLF warning deadlock
        .output()
        .await
        .context("git diff HEAD --name-only")?;
    let diff_str = String::from_utf8_lossy(&diff_out.stdout);

    // 2. Staged (already added with git add)
    let staged_out = tokio::process::Command::new("git")
        .args([
            "-c",
            "core.autocrlf=false",
            "diff",
            "--cached",
            "--name-only",
            "--diff-filter=ACDMRT",
        ])
        .current_dir(repo)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .await
        .context("git diff --cached --name-only")?;
    let staged_str = String::from_utf8_lossy(&staged_out.stdout);

    // 3. Untracked new files/directories
    let status_out = tokio::process::Command::new("git")
        .args([
            "-c",
            "core.autocrlf=false",
            "status",
            "--short",
            "--porcelain",
        ])
        .current_dir(repo)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .await
        .context("git status --short")?;
    let status_str = String::from_utf8_lossy(&status_out.stdout);

    let mut files: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Tracked changes
    for line in diff_str.lines().chain(staged_str.lines()) {
        let p = line.trim().replace('\\', "/");
        if !p.is_empty() {
            files.insert(p);
        }
    }

    // Untracked new files/dirs
    for line in status_str.lines() {
        if !line.starts_with("??") {
            continue;
        }
        let raw = line[3..].trim().replace('\\', "/");
        // Directories end with '/' — include the prefix, actual staging will be recursive
        let p = raw.trim_end_matches('/').to_string();
        if !p.is_empty() {
            files.insert(p);
        }
    }

    let mut sorted: Vec<String> = files.into_iter().collect();
    sorted.sort();
    Ok(sorted)
}

/// Gather **every tracked file** in the repository plus untracked new files.
///
/// Use this for a full-codebase review (`--full-repo`) regardless of commit history.
/// Unlike [`collect_changed_files`] this uses `git ls-files` so even files with no
/// working-tree modifications are included.
pub async fn collect_all_files(repo: &Path) -> Result<Vec<String>> {
    let cwd = std::env::current_dir().context("get current directory")?;
    let normalized: std::path::PathBuf = if repo.is_absolute() {
        repo.components()
            .filter(|c| !matches!(c, std::path::Component::CurDir))
            .collect()
    } else {
        cwd.join(repo)
            .components()
            .filter(|c| !matches!(c, std::path::Component::CurDir))
            .collect()
    };
    let repo = normalized.as_path();

    // 1. All tracked files.
    let ls_out = tokio::process::Command::new("git")
        .args(["-c", "core.autocrlf=false", "ls-files"])
        .current_dir(repo)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .await
        .context("git ls-files")?;
    let ls_str = String::from_utf8_lossy(&ls_out.stdout);

    // 2. Untracked new files (same as collect_changed_files).
    let status_out = tokio::process::Command::new("git")
        .args(["-c", "core.autocrlf=false", "status", "--short", "--porcelain"])
        .current_dir(repo)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .await
        .context("git status --short")?;
    let status_str = String::from_utf8_lossy(&status_out.stdout);

    let mut files: std::collections::HashSet<String> = std::collections::HashSet::new();

    for line in ls_str.lines() {
        let p = line.trim().replace('\\', "/");
        if !p.is_empty() {
            files.insert(p);
        }
    }

    for line in status_str.lines() {
        if !line.starts_with("??") {
            continue;
        }
        let raw = line[3..].trim().replace('\\', "/");
        let p = raw.trim_end_matches('/').to_string();
        if !p.is_empty() {
            files.insert(p);
        }
    }

    let mut sorted: Vec<String> = files.into_iter().collect();
    sorted.sort();
    Ok(sorted)
}

// ──────────────────────────────────────────────
// run_semantic_submit
// ──────────────────────────────────────────────

/// Configuration for a semantic-submit run.
#[derive(Debug, Clone)]
pub struct SemanticSubmitConfig {
    /// CodeRabbit tier (affects rate-limit delay).
    pub tier: limits::CodeRabbitTier,
    /// Max files per review PR chunk (clamped to tier file cap).
    pub max_files_per_pr: usize,
    /// Seconds between PR triggers (0 = use tier default).
    pub delay_secs: u64,
    /// If false, only plan + write manifest (no git remotes, no PRs).
    pub execute: bool,
    /// Legacy: commit all planned files to `default_branch` and push before opening chunk PRs.
    pub commit_main: bool,
    /// Reuse baseline branch name (enables `--resume` across invocations).
    pub baseline_branch: Option<String>,
    /// Skip chunks already marked `completed` in `.coderabbit/run-state.json`.
    pub resume: bool,
    /// Re-run every chunk even if previously completed.
    pub force_chunks: bool,
    /// Optional pattern filter — only process groups whose name contains this.
    pub group_filter: Option<String>,
    /// Review the **entire tracked repository** from scratch (`git ls-files`) instead of
    /// only files that differ from HEAD. When true, the drift check is skipped (N/A).
    pub full_repo: bool,
}

impl Default for SemanticSubmitConfig {
    fn default() -> Self {
        let tier = limits::CodeRabbitTier::Pro;
        let delay = tier.min_delay_between_prs_secs();
        Self {
            tier,
            max_files_per_pr: DEFAULT_MAX_FILES_PER_PR,
            delay_secs: delay,
            execute: false,
            commit_main: false,
            baseline_branch: None,
            resume: false,
            force_chunks: false,
            group_filter: None,
            full_repo: false,
        }
    }
}

/// Entry point: collect files, plan groups, optionally push baseline + isolated worktree PRs.
pub async fn run_semantic_submit(repo: &Path, cfg: &SemanticSubmitConfig) -> Result<()> {
    // ── 1. Collect files ────────────────────────────────────────────────────
    let mode_label = if cfg.full_repo {
        "full-repo (git ls-files)"
    } else {
        "diff-based (changes since HEAD)"
    };
    eprintln!("[semantic-submit] Collecting files ({mode_label})…");
    let mut all_files = if cfg.full_repo {
        collect_all_files(repo)
            .await
            .context("collect all tracked files")?
    } else {
        collect_changed_files(repo)
            .await
            .context("collect changed files")?
    };
    let dropped_tool = path_policy::retain_non_coderabbit_tool_paths(&mut all_files);
    if dropped_tool > 0 {
        eprintln!(
            "[semantic-submit] Dropped {dropped_tool} path(s) under `.coderabbit/` (tooling worktrees/state; not reviewed)."
        );
    }
    if all_files.is_empty() {
        if cfg.full_repo {
            anyhow::bail!("No tracked files found in the repository.");
        } else {
            anyhow::bail!("No changed or untracked files found. Working tree is already clean.");
        }
    }
    eprintln!(
        "[semantic-submit] Raw file count (before ignore filter): {}",
        all_files.len()
    );

    let vox_cfg = super::config::load_from_dir(repo);
    if !vox_cfg.exclude_prefixes.is_empty() {
        all_files.retain(|f| !path_policy::is_excluded_by_prefixes(f, &vox_cfg.exclude_prefixes));
        eprintln!(
            "[semantic-submit] After Vox.toml exclude_prefixes: {} paths",
            all_files.len()
        );
    }

    let mut plan_snapshot = all_files.clone();
    plan_snapshot.sort();

    // Secure the workspace before messing with branches and run_state
    let guard = super::git::WorkspaceGuard::new(repo).await?;

    let res = run_semantic_submit_core(repo, cfg, all_files, plan_snapshot).await;

    guard.restore().await?;
    res
}

async fn run_semantic_submit_core(
    repo: &Path,
    cfg: &SemanticSubmitConfig,
    all_files: Vec<String>,
    plan_snapshot: Vec<String>,
) -> Result<()> {
    // ── 2. Build semantic plan ──────────────────────────────────────────────
    let baseline_branch = if cfg.resume && !cfg.force_chunks {
        let prev = super::run_state::CoderabbitRunState::load(repo)?.ok_or_else(|| {
            anyhow::anyhow!(
                "`--resume` requires `.coderabbit/run-state.json` in the repo root. \
                 Run once with `--execute`, or pass `--baseline-branch` matching a prior run and use a compatible run-state."
            )
        })?;
        match &cfg.baseline_branch {
            None => {
                eprintln!(
                    "[resume] reusing baseline branch `{}` from run-state",
                    prev.baseline_branch
                );
                prev.baseline_branch.clone()
            }
            Some(want) if want != &prev.baseline_branch => {
                anyhow::bail!(
                    "`--resume` run-state baseline is `{}` but `--baseline-branch` is `{}`. \
                     Omit `--baseline-branch` to reuse the run-state baseline, or align the flag with the saved baseline.",
                    prev.baseline_branch,
                    want
                );
            }
            Some(want) => want.clone(),
        }
    } else if let Some(b) = cfg.baseline_branch.clone() {
        b
    } else {
        format!("cr-baseline-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S"))
    };
    let tier_cap = cfg.tier.files_per_review() as usize;
    let max_per = cfg.max_files_per_pr.max(1).min(tier_cap);
    if cfg.max_files_per_pr > tier_cap {
        eprintln!(
            "[semantic-submit] Tier {}: clamped max_files_per_pr {} → {}",
            cfg.tier, cfg.max_files_per_pr, max_per
        );
    }
    let planner = SemanticPlanner::new(max_per);
    let mut manifest = planner.plan(all_files, &baseline_branch);

    // Apply group filter if requested
    if let Some(ref filter) = cfg.group_filter {
        manifest.chunks.retain(|c| c.name.contains(filter.as_str()));
    }

    // ── 3. Print plan summary ───────────────────────────────────────────────
    eprintln!("\n══════════════════════════════════════════════");
    eprintln!("  SEMANTIC-SUBMIT plan");
    eprintln!("══════════════════════════════════════════════");
    eprintln!(
        "  Mode         : {}",
        if cfg.execute { "EXECUTE" } else { "PLAN ONLY" }
    );
    eprintln!(
        "  Tier         : {} ({} files/PR, {}s delay)",
        cfg.tier,
        cfg.tier.files_per_review(),
        cfg.tier.min_delay_between_prs_secs()
    );
    eprintln!(
        "  Files/PR cap : {} (effective {})",
        cfg.max_files_per_pr, max_per
    );
    eprintln!("  Baseline     : {}", baseline_branch);
    eprintln!("  Total files  : {}", manifest.total_files);
    eprintln!("  Chunks       : {}", manifest.chunks.len());
    eprintln!("──────────────────────────────────────────────");
    for chunk in &manifest.chunks {
        eprintln!("  {:.<30} {} files", chunk.name, chunk.files.len());
    }
    eprintln!("══════════════════════════════════════════════\n");

    // Write manifest to disk (always, even in dry-run mode — useful for resumption)
    let cr_dir = repo.join(".coderabbit");
    std::fs::create_dir_all(&cr_dir).ok();
    let manifest_path = cr_dir.join("semantic-manifest.json");
    let json = serde_json::to_string_pretty(&manifest).context("serialize manifest")?;
    std::fs::write(&manifest_path, &json).context("write manifest")?;
    eprintln!("[manifest] Written to: {}", manifest_path.display());

    if !cfg.execute {
        eprintln!(
            "\n[PLAN ONLY] No git mutations. Re-run with `--execute` to push baseline + open PRs from isolated worktrees."
        );
        return Ok(());
    }

    // ── 4. Resolve default branch + publish baseline at origin tip ───────────
    use vox_forge::GitForgeProvider;
    use vox_forge::github::GitHubProvider;
    use vox_git::GitBridge;

    let bridge = GitBridge::open(repo).context("open git repo")?;
    let remote_url = bridge.remote_url().context("get remote URL")?;
    let (owner, repo_name) = super::github::parse_github_owner_repo(&remote_url)
        .context("parse owner/repo from remote URL")?;
    let token = super::github::github_token()?;
    let provider = GitHubProvider::new(&token).map_err(|e| anyhow::anyhow!("{e}"))?;
    let repo_info = provider
        .repo_info(&owner, &repo_name)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let default_branch = repo_info.default_branch.clone();

    eprintln!(
        "\n[phase-1] Publishing baseline `{}` at origin/{default_branch} tip",
        baseline_branch
    );
    let _baseline_sha = super::github::push_baseline_from_origin(
        repo,
        &baseline_branch,
        &default_branch,
        cfg.full_repo,
    )
    .context("push baseline from origin")?;

    // Optional legacy: commit everything to default branch (broad `git add -u`).
    if cfg.commit_main {
        eprintln!("\n[legacy] --commit-main: staging all manifest paths + push {default_branch}");
        let _ = Command::new("git")
            .args(["rm", "--cached", "-r", "--ignore-unmatch", "docs/book/"])
            .current_dir(repo)
            .status();
        let add_all = Command::new("git")
            .args(["add", "-u"])
            .current_dir(repo)
            .status()
            .context("git add -u")?;
        if !add_all.success() {
            anyhow::bail!("git add -u failed");
        }
        let all_new: Vec<_> = manifest
            .chunks
            .iter()
            .flat_map(|c| c.files.iter().cloned())
            .collect();
        for batch in all_new.chunks(200) {
            let mut args = vec!["add", "--"];
            args.extend(batch.iter().map(|s| s.as_str()));
            let _ = Command::new("git").args(&args).current_dir(repo).status();
        }
        let timestamp = chrono::Utc::now().format("%Y-%m-%d");
        let commit_msg = format!(
            "feat: batch-commit {timestamp} — {} files across {} semantic groups\n\n\
             Committed via `vox review coderabbit semantic-submit --commit-main`.",
            manifest.total_files,
            manifest.chunks.len(),
        );
        let status = Command::new("git")
            .args(["commit", "-m", &commit_msg])
            .current_dir(repo)
            .status()
            .context("git commit")?;
        if !status.success() {
            anyhow::bail!("git commit failed — nothing staged or commit error");
        }
        let push = Command::new("git")
            .args(["push", "origin", &default_branch])
            .current_dir(repo)
            .status()
            .context("git push origin default branch")?;
        if !push.success() {
            anyhow::bail!("git push origin {default_branch} failed");
        }
        eprintln!("[legacy] ✓ pushed {default_branch}");
    }

    // ── 5. Resume state ─────────────────────────────────────────────────────
    let source_tree = repo.to_path_buf();
    let mut run_state = super::run_state::CoderabbitRunState {
        baseline_branch: baseline_branch.clone(),
        default_branch: default_branch.clone(),
        started_at: chrono::Utc::now().to_rfc3339(),
        chunks: manifest
            .chunks
            .iter()
            .map(|c| super::run_state::ChunkRunRecord {
                name: c.name.clone(),
                branch: format!("cr/review-{}", c.name),
                pr_number: None,
                status: "pending".to_string(),
                error: None,
            })
            .collect(),
    };

    if cfg.resume && !cfg.force_chunks {
        if let Some(prev) = super::run_state::CoderabbitRunState::load(repo)? {
            if prev.baseline_branch == baseline_branch {
                for rec in &mut run_state.chunks {
                    if let Some(o) = prev
                        .chunks
                        .iter()
                        .find(|p| p.name == rec.name && p.status == "completed")
                    {
                        *rec = o.clone();
                    }
                }
            }
        }
    }
    run_state.save(repo).context("write initial run-state")?;

    // Drift check is obsolete: WorkspaceGuard safely wraps the user's local state 
    // in a `wip:` commit. No uncommitted modifications can alter our tracked files mid-flight.

    // ── 6. Isolated worktree PRs (independent base = baseline) ─────────────

    eprintln!(
        "\n[phase-2] Creating {} review PR(s) via worktrees…",
        manifest.chunks.len()
    );
    let delay = if cfg.delay_secs > 0 {
        cfg.delay_secs
    } else {
        cfg.tier.min_delay_between_prs_secs()
    };

    for (i, chunk) in manifest.chunks.iter().enumerate() {
        if !cfg.force_chunks && run_state.chunks[i].status == "completed" {
            eprintln!(
                "[resume] skip completed chunk {} (PR #{})",
                chunk.name,
                run_state.chunks[i].pr_number.unwrap_or(0)
            );
            continue;
        }

        let cr_branch = format!("cr/review-{}", chunk.name);
        eprintln!(
            "\n[{}/{}] {} ({} files) → branch: {}",
            i + 1,
            manifest.chunks.len(),
            chunk.name,
            chunk.files.len(),
            cr_branch
        );

        let res = super::github::create_chunk_pr_via_worktree(
            repo,
            &source_tree,
            &default_branch,
            &baseline_branch,
            &cr_branch,
            &chunk.files,
            true,
        )
        .await;

        match res {
            Ok(pr_num) => {
                run_state.chunks[i].status = "completed".to_string();
                run_state.chunks[i].pr_number = Some(pr_num);
                run_state.chunks[i].error = None;
            }
            Err(e) => {
                run_state.chunks[i].status = "failed".to_string();
                run_state.chunks[i].error = Some(format!("{e:#}"));
                run_state.save(repo)?;
                return Err(e).with_context(|| format!("chunk {}", chunk.name));
            }
        }
        run_state.save(repo)?;

        if i + 1 < manifest.chunks.len() && delay > 0 {
            eprintln!("[rate-limit] waiting {delay}s before next PR…");
            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
        }
    }

    eprintln!("\n══════════════════════════════════════════════");
    let n_done = run_state
        .chunks
        .iter()
        .filter(|c| c.pr_number.is_some())
        .count();
    eprintln!("  DONE — {n_done} PR(s) with numbers recorded");
    for (chunk, rec) in manifest.chunks.iter().zip(&run_state.chunks) {
        if let Some(pr_num) = rec.pr_number {
            eprintln!("  PR #{pr_num:>6}  {}", chunk.name);
        } else if rec.status == "failed" {
            eprintln!("  (failed)         {}", chunk.name);
        }
    }
    eprintln!("══════════════════════════════════════════════");
    eprintln!("Next: monitor PRs, then ingest findings:");
    eprintln!("  vox review coderabbit ingest <pr_number>");
    eprintln!("  vox review coderabbit tasks <pr_number> --format markdown");

    Ok(())
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_ignored_build_artifacts() {
        assert!(SemanticPlanner::is_ignored("target/debug/vox"));
        assert!(SemanticPlanner::is_ignored("target-toestub/x"));
        assert!(SemanticPlanner::is_ignored("target-agent/debug/vox-cli"));
        assert!(SemanticPlanner::is_ignored("target-agent2/debug/foo.exe"));
        assert!(SemanticPlanner::is_ignored("target-doc-inv2/.rustc_info.json"));
        assert!(SemanticPlanner::is_ignored("target-ci/debug/build/addr2line/output"));
        assert!(SemanticPlanner::is_ignored("target_debug/vox"));
        assert!(SemanticPlanner::is_ignored("docs/book/index.html"));
        assert!(SemanticPlanner::is_ignored(".vox-research-data/vox.db"));
        assert!(SemanticPlanner::is_ignored("Cargo.lock"));
    }

    #[test]
    fn is_ignored_root_scratch_files() {
        assert!(SemanticPlanner::is_ignored("build_error.log"));
        assert!(SemanticPlanner::is_ignored("check_all.txt"));
        // Files with slashes are NOT root files — should not be ignored by txt rule
        assert!(!SemanticPlanner::is_ignored("docs/src/quickstart.txt"));
    }

    #[test]
    fn is_ignored_coderabbit_worktrees() {
        assert!(SemanticPlanner::is_ignored(
            ".coderabbit/worktrees/cr__review-01_scaffold"
        ));
    }

    #[test]
    fn is_ignored_real_files() {
        assert!(!SemanticPlanner::is_ignored("crates/vox-cli/src/main.rs"));
        assert!(!SemanticPlanner::is_ignored("AGENTS.md"));
        assert!(!SemanticPlanner::is_ignored("Vox.toml"));
        assert!(!SemanticPlanner::is_ignored(
            "docs/src/reference/lexicon.md"
        ));
        assert!(!SemanticPlanner::is_ignored(".github/workflows/ci.yml"));
    }

    #[test]
    fn group_assignment_canonical() {
        let g = |p: &str| SemanticPlanner::get_group(p).1;
        assert_eq!(g("AGENTS.md"), "01_scaffold");
        assert_eq!(g("Cargo.toml"), "01_scaffold");
        assert_eq!(g(".github/workflows/ci.yml"), "02_github_agents");
        assert_eq!(g(".agents/workflows/cargo-safety.md"), "02_github_agents");
        assert_eq!(g(".gitignore"), "03_dotfiles_config");
        assert_eq!(g(".opencode/README.md"), "04_opencode_retire");
        assert_eq!(g("contracts/api-registry.json"), "05_contracts");
        assert_eq!(g("docs/src/reference/lexicon.md"), "06_docs_src");
        assert_eq!(g("docs/SUMMARY.md"), "07_docs_other");
        assert_eq!(g("frontend/App.tsx"), "08_frontend");
        assert_eq!(g("examples/hello.vox"), "09_examples");
        assert_eq!(g("vox-vscode/src/extension.ts"), "10_vscode_ext");
        assert_eq!(g("scripts/unlock.ps1"), "11_scripts_xtask");
        assert_eq!(g("populi/data/sft_pairs.jsonl"), "12_populi_ml");
        assert_eq!(g("tests/fixtures/minimal.vox"), "13_tests");
        assert_eq!(g("crates/vox-parser/src/lib.rs"), "14_crate_parser_lexer");
        assert_eq!(g("crates/vox-hir/src/hir/nodes.rs"), "15_crate_hir");
        assert_eq!(g("crates/vox-typeck/src/lib.rs"), "16_crate_typeck");
        assert_eq!(g("crates/vox-codegen-rust/src/lib.rs"), "17_crate_codegen");
        assert_eq!(g("crates/vox-lsp/src/lib.rs"), "18_crate_runtime_lsp");
        assert_eq!(g("crates/vox-mcp/src/lib.rs"), "19_crate_mcp_dei");
        assert_eq!(g("crates/vox-cli/src/main.rs"), "20_crate_cli");
        assert_eq!(g("crates/vox-arca/src/lib.rs"), "21_crate_other");
    }

    #[test]
    fn plan_subdivides_large_groups() {
        let planner = SemanticPlanner::new(3);
        let files: Vec<String> = (0..7)
            .map(|i| format!("crates/vox-cli/src/file{i}.rs"))
            .collect();
        let manifest = planner.plan(files, "cr-baseline");
        // All go in crate_cli (20_crate_cli), should be split into 3 chunks (3+3+1)
        assert!(
            manifest
                .chunks
                .iter()
                .any(|c| c.name.starts_with("20_crate_cli"))
        );
        assert!(manifest.chunks.iter().all(|c| c.files.len() <= 3));
    }

    #[test]
    fn plan_filters_ignored_files() {
        let planner = SemanticPlanner::new(250);
        let files = vec![
            "target/debug/vox".to_string(),
            "build_error.log".to_string(),
            "Cargo.lock".to_string(),
            "crates/vox-cli/src/main.rs".to_string(),
        ];
        let manifest = planner.plan(files, "cr-baseline");
        assert_eq!(manifest.total_files, 1);
        assert_eq!(manifest.chunks[0].files[0], "crates/vox-cli/src/main.rs");
    }
}
