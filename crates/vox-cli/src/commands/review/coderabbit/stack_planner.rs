//! Size-first and Semantic PR Stack planner for CodeRabbit.
//!
//! Generates a sequence of dependent PRs (a Stack) to slip past CodeRabbit size limits
//! without sacrificing cross-referencing context.
//! See `super::limits` for tier-based constants.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use vox_forge::GitForgeProvider;
use vox_forge::github::GitHubProvider;
use vox_git::GitBridge;

use super::limits;
use super::path_policy;

/// Configuration for the Stack submit planner.
#[derive(Debug, Clone)]
pub struct StackPlanConfig {
    /// Max files per PR (default: 50, to keep CodeRabbit closely focused per chunk).
    pub max_files_per_pr: u32,
}

impl Default for StackPlanConfig {
    fn default() -> Self {
        Self {
            max_files_per_pr: 50,
        }
    }
}

/// A semantic grouping layer (e.g., scaffolds first, traits next, endpoints last).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackChunk {
    pub order: u32,
    pub name: String,
    pub files: Vec<String>,
}

/// The final generated artifact holding the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackManifest {
    pub generated_at: String,
    pub chunks: Vec<StackChunk>,
    pub total_files: usize,
}

/// Generates stacked PR manifests for CodeRabbit-friendly code review.
///
/// Partitions a repository's git-tracked files into semantically ordered
/// chunks that stay within CodeRabbit's per-PR file limit.
pub struct StackPlanner {
    config: StackPlanConfig,
}

impl StackPlanner {
    /// Create a new planner from the given configuration.
    pub fn new(config: StackPlanConfig) -> Self {
        Self { config }
    }

    /// Determines if a file should be hidden from CodeRabbit entirely.
    pub fn is_ignored(path: &str) -> bool {
        if path_policy::is_coderabbit_local_tool_path(path) {
            return true;
        }
        let ignored_dirs = [
            ".git",
            ".github",
            "target",
            "target-toestub", // TOESTUB build artifacts
            "node_modules",
            "dist",
            ".next",
            "tmp",
            ".gemini",
            ".cursor",
            ".vox-research-data", // local SQLite cache
        ];
        // Separately: generated mdBook HTML (in .gitignore but sometimes still tracked)
        let ignored_prefixes = ["docs/book/"];
        let ignored_exts = [
            ".png", ".jpg", ".jpeg", ".webp", ".ico", ".dll", ".exe", ".so", ".dylib", ".bin",
            ".lock", ".svg", ".db", ".db-wal", ".db-shm",
        ];
        let ignored_files = [
            "Cargo.lock",
            "package-lock.json",
            "pnpm-lock.yaml",
            "yarn.lock",
        ];

        let path_norm = path.replace('\\', "/");
        let path_lower = path_norm.to_lowercase();
        let path_parts: Vec<&str> = path_lower.split('/').collect();

        if ignored_dirs.iter().any(|d| path_parts.contains(d)) {
            return true;
        }
        if ignored_prefixes.iter().any(|p| path_norm.starts_with(p)) {
            return true;
        }
        if ignored_exts.iter().any(|e| path_lower.ends_with(e)) {
            return true;
        }
        let filename = path_norm.split('/').filter(|s| !s.is_empty()).last();
        if let Some(filename) = filename
            && ignored_files.contains(&filename)
        {
            return true;
        }

        false
    }

    /// Maps a file to its semantic layer and chunk order (used for all-file stack scans).
    ///
    /// For changed-only reviews use [`super::semantic_planner::SemanticPlanner`] instead.
    pub fn get_chunk_id(path: &str) -> (u32, &'static str) {
        let p = path.replace('\\', "/");

        // ── Scaffolding / project metadata ───────────────────────────────────
        if p.contains("Cargo.toml") || p.contains("package.json") {
            return (5, "01_scaffold_manifests");
        }
        if p.ends_with(".md") || p.ends_with(".toml") && !p.contains("crates/") {
            return (08, "01_scaffold_docs");
        }

        // ── CI / agents ──────────────────────────────────────────────────────
        if p.contains(".github/") || p.contains(".agents/") {
            return (10, "02_github_agents");
        }

        // ── Dotfiles / config ────────────────────────────────────────────────
        if p.contains(".cargo/") || p.contains(".config/") {
            return (12, "03_dotfiles_config");
        }

        // ── Docs ─────────────────────────────────────────────────────────────
        if p.contains("docs/src/") {
            return (15, "04_docs_src");
        }
        if p.contains("docs/") {
            return (17, "04_docs_other");
        }

        // ── Frontend / UI ────────────────────────────────────────────────────
        if p.contains("frontend/") || p.contains("islands/") || p.contains("packages/") {
            return (20, "05_frontend");
        }
        if p.contains("vox-vscode/") {
            return (22, "05_vscode_ext");
        }

        // ── Examples ─────────────────────────────────────────────────────────
        if p.contains("examples/") {
            return (25, "06_examples");
        }

        // ── ML / Mens ──────────────────────────────────────────────────────
        if p.starts_with("crates/vox-mens/src/tensor/lora.rs")
            || p.starts_with("crates/vox-mens/src/tensor/model.rs")
        {
            return (30, "07_populi_lora_model");
        }
        if p.starts_with("crates/vox-mens/src/tensor/") {
            return (32, "07_populi_tensor");
        }
        if p.starts_with("crates/vox-mens/") || p.starts_with("mens/") {
            return (35, "07_populi_ml");
        }

        // ── Scripts / xtask ──────────────────────────────────────────────────
        if p.starts_with("scripts/")
            || p.starts_with("xtask/")
            || p.starts_with("fuzz/")
            || p.starts_with("infra/")
            || p.starts_with("tree-sitter-vox/")
        {
            return (38, "08_scripts_xtask");
        }

        // ── Tests / fixtures ─────────────────────────────────────────────────
        if p.contains("tests/") {
            return (40, "09_tests");
        }

        // ── Compiler front-end ───────────────────────────────────────────────
        if p.contains("crates/vox-parser/") || p.contains("crates/vox-lexer/") {
            return (50, "10_crate_parser_lexer");
        }
        if p.contains("crates/vox-ast/") {
            return (52, "10_crate_ast");
        }
        if p.contains("crates/vox-hir/") {
            return (54, "10_crate_hir");
        }
        if p.contains("crates/vox-typeck/") {
            return (56, "10_crate_typeck");
        }

        // ── Code generation ──────────────────────────────────────────────────
        if p.contains("crates/vox-codegen") {
            return (60, "11_crate_codegen");
        }

        // ── Runtime / LSP ────────────────────────────────────────────────────
        if p.contains("crates/vox-runtime/")
            || p.contains("crates/vox-lsp/")
            || p.contains("crates/vox-dap/")
            || p.contains("crates/vox-fmt/")
            || p.contains("crates/vox-toestub/")
            || p.contains("crates/vox-doc-pipeline/")
        {
            return (65, "12_crate_runtime_lsp");
        }

        // ── Agent / MCP ───────────────────────────────────────────────────────
        if p.contains("crates/vox-mcp/")
            || p.contains("crates/vox-dei/")
            || p.contains("crates/vox-codex")
            || p.contains("crates/vox-capability")
        {
            return (70, "13_crate_mcp_dei");
        }

        // ── CLI ───────────────────────────────────────────────────────────────
        if p == "crates/vox-cli/src/main.rs"
            || p == "crates/vox-cli/src/lib.rs"
            || p == "crates/vox-cli/src/diagnostics.rs"
        {
            return (75, "14_crate_cli_core");
        }
        if p.starts_with("crates/vox-cli/src/training/") {
            return (77, "14_crate_cli_training");
        }
        if p.starts_with("crates/vox-cli/src/commands/") {
            return (79, "14_crate_cli_commands");
        }
        if p.starts_with("crates/vox-cli/") {
            return (80, "14_crate_cli_other");
        }

        // ── All other Rust crates ─────────────────────────────────────────────
        if p.ends_with(".rs") || p.contains("crates/") {
            return (85, "15_crate_other");
        }

        (99, "99_unassigned")
    }

    /// Partition `all_files` from `git ls-files` into semantic PR chunks.
    ///
    /// Files matched by [`StackPlanner::is_ignored`] are excluded.
    /// Large chunks are sub-divided into parts of at most `max_files_per_pr` files.
    pub fn plan(&self, all_files: Vec<String>) -> StackManifest {
        let mut chunks_map = std::collections::HashMap::new();

        let mut total_files = 0;
        for file in all_files {
            if Self::is_ignored(&file) {
                continue;
            }
            total_files += 1;
            let (order, name) = Self::get_chunk_id(&file);

            let chunk = chunks_map
                .entry(name.to_string())
                .or_insert_with(|| StackChunk {
                    order,
                    name: name.to_string(),
                    files: Vec::new(),
                });
            chunk.files.push(file.clone());
        }

        let mut chunks: Vec<StackChunk> = chunks_map.into_values().collect();
        chunks.sort_by_key(|c| c.order);

        // Optional: Sub-divide massive chunks if they exceed self.config.max_files_per_pr
        let mut final_chunks = Vec::new();
        for chunk in chunks {
            for (i, sub_batch) in chunk
                .files
                .chunks(self.config.max_files_per_pr as usize)
                .enumerate()
            {
                let suffix = if chunk.files.len() > self.config.max_files_per_pr as usize {
                    format!("_part{}", i + 1)
                } else {
                    String::new()
                };
                final_chunks.push(StackChunk {
                    order: chunk.order, // Subparts maintain same general order sequence
                    name: format!("{}{}", chunk.name, suffix),
                    files: sub_batch.to_vec(),
                });
            }
        }

        StackManifest {
            generated_at: chrono::Utc::now().to_rfc3339(),
            chunks: final_chunks,
            total_files,
        }
    }
}

/// Generate a semantic stacked-PR manifest and optionally submit each chunk as a GitHub PR.
///
/// Writes `.coderabbit-stack-manifest.json` to `path` and prints a dry-run summary
/// or actually creates PRs when `dry_run = false`.
pub async fn run_stack_submit(
    path: &Path,
    max_files_per_pr: u32,
    tier: Option<&str>,
    delay_between_prs_secs: u64,
    dry_run: bool,
) -> Result<()> {
    // 1. Gather files via git bridge or CLI wrapper
    let output = tokio::process::Command::new("git")
        .args(["ls-files"])
        .current_dir(path)
        .output()
        .await
        .context("Failed to run git ls-files")?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut all_files: Vec<String> = output_str.lines().map(|s| s.to_string()).collect();

    if all_files.is_empty() {
        anyhow::bail!("No files found or not in a git repository.");
    }

    let vox_cfg = super::config::load_from_dir(path);
    if !vox_cfg.exclude_prefixes.is_empty() {
        all_files.retain(|f| !path_policy::is_excluded_by_prefixes(f, &vox_cfg.exclude_prefixes));
    }

    let guard = super::git::WorkspaceGuard::new(path).await?;
    let res = run_stack_submit_core(path, max_files_per_pr, tier, delay_between_prs_secs, dry_run, all_files).await;
    guard.restore().await?;
    res
}

async fn run_stack_submit_core(
    path: &Path,
    max_files_per_pr: u32,
    tier: Option<&str>,
    delay_between_prs_secs: u64,
    dry_run: bool,
    all_files: Vec<String>,
) -> Result<()> {
    // 2. Plan semantic chunks
    let config = StackPlanConfig { max_files_per_pr };
    let planner = StackPlanner::new(config);
    let manifest = planner.plan(all_files);

    let tier_info = tier
        .and_then(|s| s.parse::<limits::CodeRabbitTier>().ok())
        .unwrap_or(limits::CodeRabbitTier::Pro);

    let manifest_path = path.join(".coderabbit-stack-manifest.json");
    let json = serde_json::to_string_pretty(&manifest).context("Serialize stack manifest")?;
    std::fs::write(&manifest_path, &json).context("Write stack manifest generated JSON")?;

    eprintln!("============ STACKED PR PLANNER ============");
    eprintln!(
        "[Mode]     : {}",
        if dry_run {
            "DRY RUN (No mutation)"
        } else {
            "LIVE"
        }
    );
    eprintln!("[Total]    : {} valid files", manifest.total_files);
    eprintln!("[Chunks]   : {}", manifest.chunks.len());
    eprintln!(
        "[Tier]     : {} ({} files/PR, {} reviews/hour)",
        tier_info,
        tier_info.files_per_review(),
        tier_info.reviews_per_hour()
    );
    eprintln!(
        "[Min delay]: {}s between PR triggers (stay under rate limit)",
        tier_info.min_delay_between_prs_secs()
    );
    eprintln!("============================================");

    for chunk in &manifest.chunks {
        eprintln!(
            "  -> Chunk: {:<20} ({} files)",
            chunk.name,
            chunk.files.len()
        );
    }

    eprintln!("\nArtifact written to: {}", manifest_path.display());

    if dry_run {
        eprintln!("\n[DRY RUN] Orchestration Simulation logic:");
        let mut current_base = "cr-empty-baseline".to_string();

        eprintln!("  1. CREATE orphan branch: `{}`", current_base);
        eprintln!("  2. COMMIT boilerplate README to `{}`", current_base);

        for chunk in &manifest.chunks {
            let new_branch = format!("cr-review-{}", chunk.name);
            eprintln!("  3. BRANCH `{}` from `{}`", new_branch, current_base);
            eprintln!("     a. CHECKOUT {} files from `main`", chunk.files.len());
            eprintln!(
                "     b. COMMIT 'feat: CodeRabbit review chunk {}'",
                chunk.name
            );
            eprintln!("     c. PUSH `{}`", new_branch);
            eprintln!(
                "     d. PR CREATE: base=`{}` head=`{}`",
                current_base, new_branch
            );

            current_base = new_branch;
        }

        eprintln!("\nDone. Pass `--execute` on `stack-submit` to run these commands.");
        return Ok(());
    }

    // Save current branch before destructive orphan checkout; restore even if chunk loop fails.
    let current_branch = tokio::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(path)
        .output()
        .await
        .context("git rev-parse HEAD")?;
    let current_branch = String::from_utf8_lossy(&current_branch.stdout)
        .trim()
        .to_string();

    let live_result = async {
        let bridge = GitBridge::open(path).context("Open git repo")?;
        let remote_url = bridge.remote_url().context("Get remote URL")?;
        let (owner, repo_name) = super::github::parse_github_owner_repo(&remote_url)
            .context("Parse owner/repo from remote URL")?;
        let token = super::github::github_token()?;
        let provider = GitHubProvider::new(&token).map_err(|e| anyhow::anyhow!("{e}"))?;
        let repo_info = provider
            .repo_info(&owner, &repo_name)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let default_branch = repo_info.default_branch;

        let baseline = "cr-empty-baseline";
        eprintln!("Creating orphan baseline: {}", baseline);
        super::github::create_orphan_baseline(path, baseline).await?;

        let mut pr_numbers = Vec::new();
        let mut current_base = baseline.to_string();

        for (i, chunk) in manifest.chunks.iter().enumerate() {
            let new_branch = format!("cr-review-{}", chunk.name);
            eprintln!(
                "\nChunk {}/{}: {} ({} files)",
                i + 1,
                manifest.chunks.len(),
                chunk.name,
                chunk.files.len()
            );

            let pr = super::github::create_stack_chunk_pr(
                path,
                &default_branch,
                &current_base,
                &new_branch,
                &chunk.files,
                true,
            )
            .await?;

            pr_numbers.push(pr);
            current_base = new_branch;

            if i + 1 < manifest.chunks.len() && delay_between_prs_secs > 0 {
                eprintln!(
                    "Waiting {}s before next chunk (rate limit)...",
                    delay_between_prs_secs
                );
                tokio::time::sleep(std::time::Duration::from_secs(delay_between_prs_secs)).await;
            }
        }
        Ok::<Vec<u64>, anyhow::Error>(pr_numbers)
    }
    .await;

    let restore_st = tokio::process::Command::new("git")
        .args(["checkout", &current_branch])
        .current_dir(path)
        .status()
        .await
        .context("git checkout to restore branch")?;
    if !restore_st.success() {
        eprintln!(
            "[warn] could not restore branch {}: exit {:?}. Run `git checkout {}` manually.",
            current_branch,
            restore_st.code(),
            current_branch
        );
    }

    let pr_numbers = live_result?;

    eprintln!(
        "\nDone. {} PR(s) created: {:?}",
        pr_numbers.len(),
        pr_numbers
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filtration_ignores_locks_and_targets() {
        assert!(StackPlanner::is_ignored("Cargo.lock"));
        assert!(StackPlanner::is_ignored("target/debug/vox.exe"));
        assert!(StackPlanner::is_ignored(".git/config"));
        assert!(StackPlanner::is_ignored(".cursor/workspace.json"));
        assert!(StackPlanner::is_ignored("images/logo.png"));

        // Should NOT ignore code
        assert!(!StackPlanner::is_ignored("crates/vox-cli/src/main.rs"));
        assert!(!StackPlanner::is_ignored("README.md"));
        assert!(!StackPlanner::is_ignored("frontend/package.json"));
    }

    #[test]
    fn test_chunk_assignment_logic() {
        let (order, name) = StackPlanner::get_chunk_id("Cargo.toml");
        assert_eq!(name, "01_scaffold_manifests");
        assert_eq!(order, 5);

        let (_, name) = StackPlanner::get_chunk_id("crates/vox-mens/src/tensor/lora.rs");
        assert_eq!(name, "07_populi_lora_model");

        let (_, name) = StackPlanner::get_chunk_id("crates/vox-cli/src/commands/mens/mod.rs");
        assert_eq!(name, "14_crate_cli_commands");
    }

    #[test]
    fn test_planner_subdivision() {
        let planner = StackPlanner::new(StackPlanConfig {
            max_files_per_pr: 2,
        });

        // 3 rust core files -> should split into 2 batches
        let files = vec![
            "crates/vox-cli/src/main.rs".to_string(),
            "crates/vox-cli/src/lib.rs".to_string(),
            "crates/vox-cli/src/diagnostics.rs".to_string(),
        ];

        let manifest = planner.plan(files);
        assert_eq!(manifest.chunks.len(), 2);
        assert!(manifest.chunks[0].name.starts_with("14_crate_cli_core"));
        assert_eq!(manifest.chunks[0].files.len(), 2);
        assert!(manifest.chunks[1].name.starts_with("14_crate_cli_core"));
        assert_eq!(manifest.chunks[1].files.len(), 1);
    }
}
