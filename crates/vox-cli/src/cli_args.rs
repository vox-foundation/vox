//! Shared [`clap::Args`] structs for top-level `vox` commands and Latin namespace groups.

use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// `vox build` / `vox fabrica build`
#[derive(Args, Clone, Debug)]
pub struct BuildArgs {
    /// Path to the `.vox` file
    #[arg(required = true)]
    pub file: PathBuf,
    /// Output directory for generated TypeScript
    #[arg(short, long, default_value = "dist")]
    pub out_dir: PathBuf,
    /// Native mobile build target (e.g., ios, android, native)
    #[arg(long)]
    pub target: Option<String>,
    /// Write one-shot user scaffold (`app/App.tsx`, Vite, Tailwind v4) next to output if files are missing.
    /// Same as `VOX_WEB_EMIT_SCAFFOLD=1` (flag takes precedence when either is set).
    #[arg(long)]
    pub scaffold: bool,
    /// Emit **WebIR** JSON (`web-ir.v1.json`) into the output directory (frontend IR only).
    /// For the full **VoxIrModule** bundle (HIR + embedded WebIR), use `vox check <file>.vox --emit-ir`.
    #[arg(long)]
    pub emit_ir: bool,
}

/// `vox check` / `vox fabrica check`
#[derive(Args, Clone, Debug)]
pub struct CheckArgs {
    #[arg(required = true)]
    pub file: PathBuf,
    /// Append successful check output as a training JSONL record
    #[arg(long, value_name = "PATH")]
    pub emit_training_jsonl: Option<PathBuf>,
    /// Set individual output format (overrides global --json)
    #[arg(long, value_name = "FORMAT", default_value = "text")]
    pub output_format: String,
    /// Emit the full **VoxIrModule** JSON next to the source file as `<stem>.vox-ir.json`
    /// (HIR module fields plus `module.web_ir` when present).
    #[arg(long)]
    pub emit_ir: bool,
}

/// `vox test` / `vox fabrica test`
#[derive(Args, Clone, Debug)]
pub struct TestArgs {
    #[arg(required = true)]
    pub file: PathBuf,
    /// Filter tests by label
    #[arg(long)]
    pub filter: Option<String>,
    /// Number of property testing iterations
    #[arg(long)]
    pub forall_iterations: Option<u32>,
    /// Instrument for branch coverage
    #[arg(long)]
    pub coverage: bool,
    /// Update snapshot golden files
    #[arg(long)]
    pub update_snapshots: bool,
}

/// `vox run` / `vox fabrica run`
#[derive(Args, Clone, Debug)]
pub struct RunArgs {
    #[arg(required = true)]
    pub file: PathBuf,
    /// Backend listen port (sets `VOX_PORT` for generated Axum and Vite proxy)
    #[arg(long)]
    pub port: Option<u16>,
    /// `app` = generated server; `script` = `fn main()` script lane; `auto` = heuristic.
    #[arg(long, value_enum, default_value_t = crate::commands::run::RunMode::Auto)]
    pub mode: crate::commands::run::RunMode,
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}

/// `vox script` / `vox fabrica script`
#[cfg(feature = "script-execution")]
#[derive(Args, Clone, Debug)]
pub struct ScriptArgs {
    #[arg(required = true)]
    pub file: PathBuf,
    #[arg(long, default_value_t = false)]
    pub sandbox: bool,
    #[arg(long, default_value_t = false)]
    pub no_cache: bool,
    #[arg(long)]
    pub isolation: Option<String>,
    #[arg(long)]
    pub trust_class: Option<String>,
    /// Optional target triple for cross-compilation (Wave 4).
    #[arg(long)]
    pub target_triple: Option<String>,
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}

/// `vox dev` / `vox fabrica dev`
#[derive(Args, Clone, Debug)]
pub struct DevArgs {
    #[arg(required = true)]
    pub file: PathBuf,
    #[arg(short, long, default_value = "dist")]
    pub out_dir: PathBuf,
    #[arg(long)]
    pub port: Option<u16>,
    #[arg(long, default_value = "false")]
    pub open: bool,
}

/// Bundling mode: `app` (web + backend) or `script` (binary only).
#[derive(Clone, Copy, Debug, ValueEnum, Default, Serialize, Deserialize)]
pub enum BundleMode {
    /// Web application with React frontend and Axum backend.
    #[default]
    App,
    /// Native binary script for mesh/CLI execution.
    Script,
}

/// `vox bundle` / `vox fabrica bundle`
#[derive(Args, Clone, Debug)]
pub struct BundleArgs {
    #[arg(required = true)]
    pub file: PathBuf,
    /// Bundling mode.
    #[arg(long, value_enum, default_value_t = BundleMode::App)]
    pub mode: BundleMode,
    #[arg(short, long, default_value = "dist")]
    pub out_dir: PathBuf,
    #[arg(long)]
    pub target: Option<String>,
    #[arg(long, default_value = "true")]
    pub release: bool,
}

/// `vox fmt` / `vox fabrica fmt`
#[derive(Args, Clone, Debug)]
pub struct FmtArgs {
    #[arg(required = true)]
    pub file: PathBuf,
    /// Exit with error if the file would be reformatted (does not write).
    #[arg(long, default_value_t = false)]
    pub check: bool,
}

/// `vox doctor` / `vox mens doctor`
#[derive(Args, Clone, Debug)]
pub struct DoctorArgs {
    #[arg(long, default_value_t = false)]
    pub auto_heal: bool,
    #[arg(long, default_value_t = false)]
    pub test_health: bool,
    #[arg(long, default_value_t = false)]
    pub build_perf: bool,
    #[arg(long, default_value_t = false)]
    pub scope: bool,
    #[arg(long, default_value_t = false)]
    pub json: bool,
    /// OCI / automation: run default doctor checks and exit with non-zero status if any fail (no banner; stable for HEALTHCHECK).
    #[arg(long, default_value_t = false)]
    pub probe: bool,
}

/// `vox train` (legacy; canonical: `vox mens train`)
#[cfg(all(feature = "gpu", feature = "mens-dei"))]
#[derive(Args, Clone, Debug)]
pub struct TrainLegacyArgs {
    #[arg(long)]
    pub data_dir: Option<PathBuf>,
    #[arg(long)]
    pub output_dir: Option<PathBuf>,
    #[arg(long)]
    pub provider: Option<String>,
    #[arg(long, default_value = "false")]
    pub native: bool,
}

/// `vox stub-check` / `vox mens stub-check`
#[cfg(feature = "stub-check")]
#[derive(Args, Clone, Debug)]
pub struct StubCheckArgs {
    #[arg(long, short = 'p', value_name = "PATH", conflicts_with = "scan_pos")]
    pub path: Option<PathBuf>,
    #[arg(value_name = "PATH", conflicts_with = "path")]
    pub scan_pos: Option<PathBuf>,
    #[arg(short = 'f', long)]
    pub format: Option<String>,
    #[arg(short = 's', long)]
    pub severity: Option<String>,
    #[arg(long, default_value = "true")]
    pub suggest_fixes: bool,
    #[arg(long)]
    pub rules: Option<String>,
    #[arg(long)]
    pub excludes: Vec<String>,
    #[arg(long)]
    pub langs: Option<String>,
    #[arg(long)]
    pub baseline: Option<String>,
    #[arg(long)]
    pub save_baseline: Option<String>,
    #[arg(long)]
    pub task_list: bool,
    #[arg(long)]
    pub import_suppressions: bool,
    #[arg(long)]
    pub ingest_findings: Option<PathBuf>,
    #[arg(long)]
    pub fix_pipeline: bool,
    #[arg(long)]
    pub fix_pipeline_apply: bool,
    #[arg(long, value_name = "MODE")]
    pub gate: Option<String>,
    #[arg(long, value_name = "PATH")]
    pub gate_budget_path: Option<PathBuf>,
    #[arg(long)]
    pub verify_impacted: bool,
    #[arg(long, default_value = "1", value_name = "N")]
    pub max_escalation: u8,
    #[arg(long)]
    pub self_heal_safe_mode: bool,
}

/// `vox add` — manifest dependency declaration.
#[derive(Args, Clone, Debug)]
pub struct AddDependencyArgs {
    /// Dependency package name.
    #[arg(required = true)]
    pub name: String,
    /// Version requirement (default `*`).
    #[arg(long)]
    pub version: Option<String>,
    /// Local path dependency.
    #[arg(long)]
    pub path: Option<String>,
}

/// `vox remove`
#[derive(Args, Clone, Debug)]
pub struct RemoveDependencyArgs {
    #[arg(required = true)]
    pub name: String,
}

/// `vox lock`
#[derive(Args, Clone, Debug)]
pub struct LockArgs {
    /// Verify `vox.lock` is current without rewriting.
    #[arg(long)]
    pub locked: bool,
}

/// `vox sync`
#[derive(Args, Clone, Debug)]
pub struct SyncArgs {
    #[arg(long)]
    pub registry: Option<String>,
    /// Fail when the lockfile does not strictly match `Vox.toml`.
    #[arg(long)]
    pub frozen: bool,
}

/// `vox deploy` — apply `Vox.toml` `[deploy]` via container / compose / Kubernetes / bare-metal.
#[derive(Args, Clone, Debug)]
pub struct DeployArgs {
    /// Deployment environment label (image tag suffix, e.g. `production`).
    #[arg(default_value = "production")]
    pub environment: String,
    /// Override `[deploy].target` (`container`, `compose`, `kubernetes`, `bare-metal`, …).
    #[arg(long)]
    pub target: Option<String>,
    /// Override `[deploy].runtime` for OCI builds (`auto`, `docker`, `podman`).
    #[arg(long)]
    pub runtime: Option<String>,
    /// Print actions without mutating remote systems or registries (best-effort).
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
    /// For compose targets: run `up` detached (`-d`).
    #[arg(long, default_value_t = false)]
    pub detach: bool,
    /// Require `vox.lock` to exist (CI / reproducibility gate).
    #[arg(long, default_value_t = false)]
    pub locked: bool,
}

/// Binary release host for `vox upgrade --source release` (`VOX_UPGRADE_PROVIDER`).
#[derive(Clone, Copy, Debug, Default, ValueEnum, PartialEq, Eq)]
pub enum UpgradeReleaseProvider {
    /// GitHub Releases (default for upstream).
    #[default]
    Github,
    /// GitLab Releases API.
    Gitlab,
    /// Static HTTP mirror using the binary release URL layout (`…/releases/download/<tag>/…`).
    Http,
}

/// `vox upgrade` lane: release binary vs local repository checkout.
#[derive(Clone, Copy, Debug, Default, ValueEnum, PartialEq, Eq)]
pub enum UpgradeLane {
    /// Checksums-verified release archive into `CARGO_HOME/bin` (default).
    #[default]
    Release,
    /// Fetch / fast-forward (or explicit `--ref`) then `cargo install --locked --path crates/vox-cli`.
    Repo,
}

/// `vox upgrade` — toolchain only (never `Vox.toml` / `vox.lock`).
#[derive(Args, Clone, Debug)]
pub struct UpgradeToolchainArgs {
    /// `release` = binary lane; `repo` = git + source install lane.
    #[arg(long = "source", value_enum, default_value_t = UpgradeLane::Release)]
    pub lane: UpgradeLane,
    /// Repository root for `--source repo`. Defaults to `VOX_REPO_ROOT` or walk-up (same as `vox ci`).
    #[arg(long, value_name = "PATH")]
    pub repo_root: Option<PathBuf>,
    /// After fetch, check out this tag, branch, or SHA. When omitted on `--source repo`, fast-forwards the current branch to upstream (or `--remote`/`--branch`).
    #[arg(long = "ref", value_name = "REF")]
    pub git_ref: Option<String>,
    /// When the current branch has no upstream, use this remote with `--branch` instead.
    #[arg(long)]
    pub remote: Option<String>,
    /// When the current branch has no upstream, fast-forward to `remote/branch`.
    #[arg(long)]
    pub branch: Option<String>,
    /// Allow `git fetch` / `merge` / `checkout` when the worktree is not clean.
    #[arg(long, default_value_t = false)]
    pub allow_dirty: bool,
    /// Check for updates only (default). Use `--apply` to mutate (install binary or update repo + reinstall).
    #[arg(long)]
    pub apply: bool,
    /// Channel: `stable` (no prereleases unless `--allow-prerelease`) or `next` (prereleases allowed).
    #[arg(long, default_value = "stable")]
    pub channel: String,
    /// Pin a release tag (e.g. `v1.2.3` or `1.2.3`). Skips “latest” discovery.
    #[arg(long = "version", value_name = "TAG")]
    pub version: Option<String>,
    /// Where to fetch releases (`VOX_UPGRADE_PROVIDER`).
    #[arg(long, value_enum)]
    pub provider: Option<UpgradeReleaseProvider>,
    /// Repository `owner/name` (GitHub) or `namespace/project` (GitLab). Default: `vox-foundation/vox`.
    #[arg(long, value_name = "OWNER/REPO")]
    pub repo: Option<String>,
    /// For `--provider http`: base URL such as `https://github.com/org/repo/releases`.
    #[arg(long, value_name = "URL")]
    pub base_url: Option<String>,
    /// For `--provider gitlab`: API host (default `https://gitlab.com`). `VOX_UPGRADE_GITLAB_HOST`.
    #[arg(long, value_name = "URL")]
    pub gitlab_host: Option<String>,
    /// Custom GitHub API root (Enterprise/CN mirror). `VOX_UPGRADE_GITHUB_API_URL`.
    #[arg(long, value_name = "URL")]
    pub github_api_url: Option<String>,
    /// Allow major / semver-incompatible jumps (language may ship breaking `vox` releases).
    #[arg(long)]
    pub allow_breaking: bool,
    /// Allow prerelease versions on the `stable` channel (normally `next` only).
    #[arg(long)]
    pub allow_prerelease: bool,
}
