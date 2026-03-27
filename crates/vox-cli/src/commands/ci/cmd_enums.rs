//! Clap enums for `vox ci`.

use clap::{Subcommand, ValueEnum};
use std::path::PathBuf;

use super::release_build;

/// Command variations for Continuous Integration guards and internal codebase hygiene.
#[derive(Subcommand)]
pub enum CiCmd {
    /// `cargo metadata --locked --format-version 1 --no-deps` (workspace manifest resolves).
    Manifest,
    /// Documentation SSOT guard (required pages, doc-inventory schema, orphan inventory crate list).
    #[command(name = "check-docs-ssot")]
    CheckDocsSsot,
    /// Codex / Arca SSOT file and OpenAPI substring guard.
    #[command(name = "check-codex-ssot")]
    CheckCodexSsot,
    /// Validate `contracts/index.yaml` against JSON Schema and listed file paths.
    #[command(name = "contracts-index")]
    ContractsIndex,
    /// Validate `publication-worthiness.default.yaml` against its JSON Schema + numeric invariants.
    #[command(name = "scientia-worthiness-contract")]
    ScientiaWorthinessContract,
    /// Run documentation + Codex + command-compliance + contracts-index guards in one shot.
    #[command(name = "ssot-drift")]
    SsotDrift,
    /// VoxDB connect policy doc, telemetry JSONL parsing, and `research_metrics` NULL-vs-zero invariants.
    #[command(name = "data-ssot-guards")]
    DataSsotGuards,
    /// `cargo check -p vox-cli` for each supported feature set.
    #[command(name = "feature-matrix")]
    FeatureMatrix,
    /// Fail if `vox-cli` sources import `vox_dei::`.
    #[command(name = "no-dei-import", visible_alias = "no-vox-dei-import")]
    NoDeiImport,
    /// Run `vox-doc-pipeline --check` to verify SUMMARY.md matches docs/src
    CheckSummaryDrift,
    /// Build all documentation artifacts
    BuildDocs,
    /// Doc inventory (schema v3): generate or verify.
    DocInventory {
        /// Subcommand execution variant.
        #[command(subcommand)]
        cmd: DocInventoryCmd,
    },
    /// Milestone benchmark matrix (`contracts/eval/benchmark-matrix.json`).
    #[command(name = "eval-matrix")]
    EvalMatrix {
        /// Subcommand execution variant.
        #[command(subcommand)]
        cmd: EvalMatrixCmd,
    },
    /// Fail if workflow YAML references `scripts/` paths not in the allowlist file.
    #[command(name = "workflow-scripts")]
    WorkflowScripts {
        /// Allowlist path (one script path per line, repo-relative).
        #[arg(long, default_value = "docs/agents/workflow-script-allowlist.txt")]
        allowlist: PathBuf,
    },
    /// Fail if changed LF-policy text files contain CRLF / CR (`*.ps1` exempt). Forward-only unless `--all`.
    #[command(name = "line-endings")]
    LineEndings {
        /// Audit all tracked policy files (not just the diff).
        #[arg(long)]
        all: bool,
        /// Git ref for diff base (overrides `VOX_LINE_ENDINGS_BASE`; head defaults to `HEAD`).
        #[arg(long)]
        base: Option<String>,
    },
    /// Run mesh / Populi CI gate steps from `scripts/populi/gates.yaml` (with legacy fallback).
    #[command(name = "mesh-gate", visible_alias = "mens-gate")]
    MeshGate {
        /// Profile name: `m1m4` or `training`.
        #[arg(long, default_value = "m1m4")]
        profile: String,
        /// Build `vox-cli` to a side `--target-dir`, copy the `vox` binary to a temp path, then run the gate from that copy (avoids file locks when the workspace `vox` is busy). **Windows + Unix.**
        #[arg(long)]
        isolated_runner: bool,
        /// Back-compat for `--isolated-runner` (older docs / scripts).
        #[arg(long, hide = true)]
        windows_isolated_runner: bool,
        /// Cargo `--target-dir` for the isolated runner build. Default: `target/mens-gate-safe`.
        #[arg(long)]
        gate_build_target_dir: Option<PathBuf>,
        /// With `--isolated-runner`: tee child stdout/stderr to this file while printing to the console.
        #[arg(long)]
        gate_log_file: Option<PathBuf>,
    },
    /// Full-repo TOESTUB: `cargo build -p vox-toestub --release` then `cargo run -p vox-toestub --bin toestub` (replaces `scripts/toestub_self_apply.*`).
    #[command(name = "toestub-self-apply")]
    ToestubSelfApply,
    /// Scoped TOESTUB: `cargo run -p vox-toestub --bin toestub -- <ROOT>`.
    #[command(name = "toestub-scoped")]
    ToestubScoped {
        /// Root path for structural scope testing.
        #[arg(default_value = "crates/vox-repository")]
        root: PathBuf,
        /// Exit policy forwarded to `toestub --mode` (`legacy` keeps historical Error+ fail).
        #[arg(long, value_enum, default_value_t = ToestubCiMode::Legacy)]
        mode: ToestubCiMode,
    },
    /// Scaling SSOT: validate `contracts/scaling/policy.yaml`; optionally emit backlog + findings.
    #[command(name = "scaling-audit")]
    ScalingAudit {
        /// Subcommand.
        #[command(subcommand)]
        cmd: ScalingAuditCmd,
    },
    /// Optional CUDA feature compile checks when `nvcc` is on PATH (or skip via env).
    #[command(name = "cuda-features")]
    CudaFeatures,
    /// Release-build `vox` with `gpu,mens-candle-cuda`, tee output to `mens/runs/logs/cuda_build_<UTC>.log` (same intent as `cargo vox-cuda-release` + `cursor_background_cuda_build.ps1`).
    #[command(name = "cuda-release-build")]
    CudaReleaseBuild {
        /// Log directory (created if missing).
        #[arg(long, default_value = "mens/runs/logs")]
        log_dir: PathBuf,
    },
    /// Wall-clock timings for key `cargo check` lanes (default CLI, GPU+stub, optional CUDA).
    #[command(name = "build-timings")]
    BuildTimings {
        /// Print one JSON object per lane (machine-readable).
        #[arg(long)]
        json: bool,
        /// Also time isolated `cargo check -p <crate>` lanes (compiler vs data vs Oratio vs Mens train).
        #[arg(long)]
        crates: bool,
        /// Detailed per-crate telemetry persisted to Arca (V34+).
        #[arg(long)]
        deep: bool,
        /// Persist results to VoxDB (default: true if deep).
        #[arg(long)]
        persist: Option<bool>,
        /// Name for this build run (deep only).
        #[arg(long)]
        name: Option<String>,
        /// Profile: `dev` or `release` (deep only).
        #[arg(long, default_value = "dev")]
        profile: String,
    },
    /// Compare grammar taxonomy fingerprint (`generate_system_prompt` SHA-256) to `mens/data/grammar_fingerprint.txt`; update file on drift.
    #[command(name = "grammar-drift")]
    GrammarDrift {
        /// Emit machine-readable `drift=true|false` for CI (e.g. append to `GITHUB_OUTPUT`).
        #[arg(long, value_enum)]
        emit: Option<GrammarDriftEmit>,
    },
    /// Repository hygiene guards (`TypeVar(0)` in codegen crates only, filtered `opencode` refs, stray root files) — GitLab parity.
    #[command(name = "repo-guards")]
    RepoGuards,
    /// Fail when changed files add direct secret env reads outside Clavis-owned modules.
    #[command(name = "secret-env-guard")]
    SecretEnvGuard {
        /// Scan all crate Rust files instead of only changed files.
        #[arg(long)]
        all: bool,
    },
    /// Fail when unknown crates use `db.connection().query|execute(` (transitional allowlist in docs).
    #[command(name = "sql-surface-guard")]
    SqlSurfaceGuard {
        /// Scan all `crates/**/*.rs` instead of only `git diff` changed files.
        #[arg(long)]
        all: bool,
    },
    /// Verify Clavis SSOT parity between managed secret spec and docs/guards.
    #[command(name = "clavis-parity")]
    ClavisParity,
    /// Command registry parity: `contracts/cli/command-registry.yaml` vs `ref-cli`, reachability, compilerd, dei, MCP tools, script duals.
    #[command(name = "command-compliance")]
    CommandCompliance,
    /// Compare `cargo llvm-cov report --json --summary-only` to `.config/coverage-gates.toml`.
    #[command(name = "coverage-gates")]
    CoverageGates {
        /// Output path from `cargo llvm-cov report --json --summary-only`.
        #[arg(long)]
        summary_json: PathBuf,
        #[arg(long, value_enum, default_value_t = CoverageGateMode::Warn)]
        mode: CoverageGateMode,
        /// Gate policy TOML (repo-relative unless absolute).
        #[arg(long, default_value = ".config/coverage-gates.toml")]
        config: PathBuf,
    },
    /// Regenerate or verify `docs/src/reference/cli-command-surface.generated.md` from the registry.
    #[command(name = "command-sync")]
    CommandSync {
        /// Write generated Markdown; without this flag, verify it matches the registry.
        #[arg(long)]
        write: bool,
    },
    /// Validate `vox.pm.provenance/1` JSON files under `.vox_modules/provenance/` (from `vox pm publish`).
    #[command(name = "pm-provenance")]
    PmProvenance {
        /// Fail when the provenance directory is missing or contains no `*.json`.
        #[arg(long)]
        strict: bool,
        /// Directory to scan (relative to repo root unless absolute); default `.`.
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Fail if internal Markdown links are broken in `docs/src` or root-level guides.
    #[command(name = "check-links")]
    CheckLinks,
    /// Build and package release artifacts for a target triple (binary + checksum manifest).
    #[command(name = "release-build")]
    ReleaseBuild {
        /// Rust target triple (for example `x86_64-unknown-linux-gnu`).
        #[arg(long)]
        target: String,
        /// Version tag used in artifact names (defaults to package version).
        #[arg(long)]
        version: Option<String>,
        /// Output directory for packaged artifacts.
        #[arg(long, default_value = "dist")]
        out_dir: PathBuf,
        /// Which binary packages to produce.
        #[arg(long, value_enum, default_value = "vox")]
        package: release_build::ReleasePackage,
    },
}

/// Output channel for [`CiCmd::GrammarDrift`].
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum GrammarDriftEmit {
    /// One line: `drift=true` or `drift=false` (GitHub Actions / shell).
    Github,
    /// Writes `drift.env` in the repo root with `drift=true|false` (GitLab-style artifact).
    Gitlab,
}

/// Subcommands for the doc inventory schema verifier.
#[derive(Subcommand)]
pub enum DocInventoryCmd {
    /// Write `docs/agents/doc-inventory.json` (or `--output`).
    Generate {
        /// Optional path to override the default JSON inventory location.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Fail if committed inventory differs from a fresh generation (ignores `generated_at`).
    Verify,
}

/// `vox ci toestub-scoped --mode` ↔ `toestub --mode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum ToestubCiMode {
    #[default]
    Legacy,
    Audit,
    #[value(name = "enforce-warn")]
    EnforceWarn,
    #[value(name = "enforce-strict")]
    EnforceStrict,
}

impl ToestubCiMode {
    pub(crate) fn as_cli_str(self) -> &'static str {
        match self {
            ToestubCiMode::Legacy => "legacy",
            ToestubCiMode::Audit => "audit",
            ToestubCiMode::EnforceWarn => "enforce-warn",
            ToestubCiMode::EnforceStrict => "enforce-strict",
        }
    }
}

/// Subcommands for [`CiCmd::ScalingAudit`].
#[derive(Subcommand)]
pub enum ScalingAuditCmd {
    /// Validate scaling policy YAML against JSON Schema and `vox-scaling-policy` parse.
    Verify,
    /// Regenerate `contracts/reports/scaling-audit/**` (≥300 templated tasks + TOESTUB JSON on `crates/`).
    #[command(name = "emit-reports")]
    EmitReports,
}

/// `vox ci coverage-gates --mode` (warn = print only; enforce = fail CI).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum CoverageGateMode {
    /// Print gaps; exit 0 (visibility without blocking merges).
    #[default]
    Warn,
    /// Exit non-zero when a configured threshold is not met.
    Enforce,
}

/// Subcommands for [`CiCmd::EvalMatrix`].
#[derive(Subcommand)]
pub enum EvalMatrixCmd {
    /// Validate committed JSON against `contracts/eval/benchmark-matrix.schema.json`.
    Verify,
    /// Run `cargo` checks/tests mapped from `benchmark_classes` (deduped across milestones).
    Run {
        /// Restrict to one milestone `id` from the matrix (e.g. `m3-dei-contracts`).
        #[arg(long)]
        milestone: Option<String>,
    },
}
