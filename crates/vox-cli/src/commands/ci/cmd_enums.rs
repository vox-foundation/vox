//! Clap enums for `vox ci`.

use clap::{Subcommand, ValueEnum};
use std::path::PathBuf;

use super::completion_quality::CompletionGateMode;
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
    /// Validate `contracts/terminal/exec-policy.v1.yaml` against schema (+ pwsh smoke when available).
    #[command(name = "exec-policy-contract")]
    ExecPolicyContract,
    /// Validate OpenClaw gateway protocol fixture contracts.
    #[command(name = "openclaw-contract")]
    OpenClawContract,
    /// Validate unified operations catalog parity across MCP + CLI registries.
    #[command(name = "operations-verify")]
    OperationsVerify,
    /// Sync or verify derived registry artifacts from unified operations catalog.
    #[command(name = "operations-sync")]
    OperationsSync {
        /// Target projection.
        #[arg(long, value_enum)]
        target: OperationsSyncTarget,
        /// Write generated output. Without this flag, verify current file matches.
        #[arg(long)]
        write: bool,
    },
    /// Validate `publication-worthiness.default.yaml` against its JSON Schema + numeric invariants.
    #[command(name = "scientia-worthiness-contract")]
    ScientiaWorthinessContract,
    /// Validate `scientia-heuristics.default.yaml` against its struct defaults.
    #[command(name = "scientia-heuristics-parity")]
    ScientiaHeuristicsParity,
    /// Validate SCIENTIA finding-candidate + novelty-evidence example JSON against v1 schemas.
    #[command(name = "scientia-novelty-ledger-contracts")]
    ScientiaNoveltyLedgerContracts,
    /// Run documentation + Codex + command-compliance + contracts-index guards in one shot.
    #[command(name = "ssot-drift")]
    SsotDrift,
    /// VoxDB connect policy doc, telemetry JSONL parsing, and `research_metrics` NULL-vs-zero invariants.
    #[command(name = "data-ssot-guards")]
    DataSsotGuards,
    /// `cargo check -p vox-cli` for each supported feature set.
    #[command(name = "feature-matrix")]
    FeatureMatrix,
    /// Ensures `vox-cli` sources do not reference the staging `vox-dei` crate via a Rust path import.
    #[command(name = "no-dei-import", visible_alias = "no-vox-dei-import")]
    NoDeiImport,
    /// Run `vox-doc-pipeline --check` to verify SUMMARY.md matches docs/src
    CheckSummaryDrift,
    /// Verify attention event tracking parity
    AttentionEventLedgerParity,
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
    /// Mens model scorecard harness (`contracts/eval/mens-scorecard*.json`).
    #[command(name = "mens-scorecard")]
    MensScorecard {
        /// Subcommand execution variant.
        #[command(subcommand)]
        cmd: MensScorecardCmd,
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
        /// Cargo `--target-dir` for the isolated runner build. Default: OS temp `…/vox-targets/<repo-hash>/mens-gate-safe`.
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
    /// Compare grammar taxonomy fingerprint (`emit_ebnf` SHA-256) to `mens/data/grammar_fingerprint.txt`; update file on drift.
    #[command(name = "grammar-drift")]
    GrammarDrift {
        /// Emit machine-readable `drift=true|false` for CI (e.g. append to `GITHUB_OUTPUT`).
        #[arg(long, value_enum)]
        emit: Option<GrammarDriftEmit>,
    },
    /// Audit K-complexity budget: verify compressed sizes of golden outputs vs `contracts/eval/complexity-budget.v1.json`.
    #[command(name = "k-complexity-budget")]
    KComplexityBudget {
        /// Fail if any fixture exceeds its budget by more than this percentage (default 0%).
        #[arg(long, default_value_t = 0.0)]
        tolerance_percent: f64,
        /// Update baseline budgets in `contracts/eval/complexity-budget.v1.json` (Wave 11 Task 211).
        #[arg(long)]
        update: bool,
    },
    /// Validate grammar export crate: emit all formats, verify rule counts are non-zero, assert semver alignment.
    #[command(name = "grammar-export-check")]
    GrammarExportCheck,
    /// Histogram of AST decl kinds across `examples/golden` (requires `vox-corpus/ast-extract`).
    #[command(name = "corpus-decl-coverage", visible_alias = "corpus-coverage")]
    CorpusDeclCoverage,
    /// Repository hygiene guards (`TypeVar(0)` in codegen crates only, filtered `opencode` refs, stray root files) — GitLab parity.
    #[command(name = "repo-guards")]
    RepoGuards,
    /// Fail when changed files add direct secret env reads outside Clavis-owned modules.
    /// Fail when changed files use environment variables not registered in Clavis or Operator Registry.
    #[command(name = "operator-env-guard")]
    OperatorEnvGuard {
        /// Scan all crate Rust files instead of only changed files.
        #[arg(long)]
        all: bool,
    },
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
    /// Fail when unknown crates call `query_all(` on Codex (transitional allowlist in docs).
    #[command(name = "query-all-guard")]
    QueryAllGuard {
        /// Scan all `crates/**/*.rs` instead of only `git diff` changed files.
        #[arg(long)]
        all: bool,
    },
    /// Fail when unknown crates use the Turso Rust path prefix (transitional allowlist in docs).
    #[command(name = "turso-import-guard")]
    TursoImportGuard {
        /// Scan all `crates/**/*.rs` instead of only `git diff` changed files.
        #[arg(long)]
        all: bool,
    },
    /// Verify Clavis SSOT parity between managed secret spec and docs/guards.
    #[command(name = "clavis-parity")]
    ClavisParity,
    /// Generate Clavis SSOT manifest.
    #[command(name = "clavis-contracts")]
    ClavisContracts,
    /// Machine-checkable Clavis cutover promotion/rollback gates for shadow/canary/enforce/decommission.
    #[command(name = "clavis-cutover-gates")]
    ClavisCutoverGates,
    /// Emit post-cutover policy-violation audit report for Clavis migration.
    #[command(name = "clavis-cutover-audit")]
    ClavisCutoverAudit {
        /// Scan all crate Rust files instead of only changed files.
        #[arg(long)]
        all: bool,
    },
    /// Enforce mapping between OrchestratorConfig, Vox Db and preferences for Attention Guarding.
    #[command(name = "attention-config-parity")]
    AttentionConfigParity,
    /// Command registry parity: `contracts/cli/command-registry.yaml` vs `ref-cli`, reachability, compilerd, dei, MCP tools, script duals.
    #[command(name = "command-compliance")]
    CommandCompliance,
    /// Scan for LLM premature-completion patterns; write `contracts/reports/completion-audit.v1.json`.
    #[command(name = "completion-audit")]
    CompletionAudit {
        /// Additional repo-relative or absolute directories to scan (must resolve under repo root). Default scan always includes `crates/`.
        #[arg(long = "scan-extra", value_name = "DIR")]
        scan_extra: Vec<PathBuf>,
    },
    /// Gate on the last completion audit (Tier A hard block; Tier B vs `completion-baseline.v1.json`).
    #[command(name = "completion-gates")]
    CompletionGates {
        #[arg(long, value_enum, default_value_t = CompletionGateMode::Enforce)]
        mode: CompletionGateMode,
    },
    /// Ingest a completion audit report into VoxDB `ci_completion_*` telemetry tables.
    #[command(name = "completion-ingest")]
    CompletionIngest {
        /// Audit JSON path (default: `contracts/reports/completion-audit.v1.json`).
        #[arg(long)]
        report: Option<PathBuf>,
        #[arg(long, default_value = "local")]
        workflow: String,
        #[arg(long, default_value = "completion-audit")]
        run_kind: String,
    },
    /// Run rust ecosystem support parity checks (`vox-compiler` contract + classifier test).
    #[command(name = "rust-ecosystem-policy")]
    RustEcosystemPolicy,
    /// Fast local smoke: orchestrator compile + command-compliance + rust ecosystem policy.
    #[command(name = "policy-smoke")]
    PolicySmoke,
    /// GUI smoke: `web_ir_lower_emit` always; optional Vite (`VOX_WEB_VITE_SMOKE=1`) and Playwright (`VOX_GUI_PLAYWRIGHT=1`) lanes.
    #[command(name = "gui-smoke")]
    GuiSmoke,
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
    /// Regenerate or verify generated CLI/reference docs from registry + code constants.
    #[command(name = "command-sync")]
    CommandSync {
        /// Write generated Markdown; without this flag, verify it matches the registry.
        #[arg(long)]
        write: bool,
    },
    /// Regenerate or verify `contracts/capability/model-manifest.generated.json` (Mens / external models).
    #[command(name = "capability-sync")]
    CapabilitySync {
        /// Write the generated JSON manifest.
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
    CheckLinks {
        /// Optional target file or directory to check.
        #[arg(long)]
        target: Option<PathBuf>,
    },
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
    /// Audit workspace artifacts for cleanup.
    #[command(name = "artifact-audit")]
    ArtifactAudit {
        #[arg(long)]
        json: bool,
    },
    /// Prune workspace artifacts cleanly.
    #[command(name = "artifact-prune")]
    ArtifactPrune {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        policy: Option<PathBuf>,
    },
    /// Nomenclature guard: fail when new Latin-only structural crate directories appear outside the allowlist (T189-T196).
    #[command(name = "nomenclature-guard")]
    NomenclatureGuard {
        /// Print a JSON array of violations instead of prose (for tooling).
        #[arg(long)]
        json: bool,
    },
    /// Scan for retired symbols inside `docs/` using the list in `contracts/documentation/retired-symbols.v1.yaml`.
    #[command(name = "retired-symbol-check")]
    RetiredSymbolCheck,
    /// **Placeholder:** prints a message only (no DB/corpus checks). Prefer `vox ci mesh-gate` and `vox mens corpus …` for real gates.
    #[command(name = "mens-corpus-health")]
    MensCorpusHealth {
        #[arg(long, default_value_t = 1000)]
        min_pairs: usize,
        #[arg(long, default_value_t = 0.15)]
        min_human_ratio: f64,
    },
    /// **Placeholder:** prints a message only (no GRPO validation).
    #[command(name = "grpo-reward-baseline")]
    GrpoRewardBaseline,
    /// **Placeholder:** prints a message only (no eval suite).
    #[command(name = "collateral-damage-gate")]
    CollateralDamageGate {
        #[arg(long, default_value_t = 0.05)]
        max_damage_rate: f64,
    },
    /// **Placeholder:** prints a message only (no constrained generation).
    #[command(name = "constrained-gen-smoke")]
    ConstrainedGenSmoke {
        #[arg(long, default_value_t = 50)]
        n_samples: usize,
    },
    /// Sync derived IDE ignore files (.cursorignore, .aiignore, .aiexclude) from .voxignore SSOT.
    #[command(name = "sync-ignore-files")]
    SyncIgnoreFiles {
        /// If true, fail CI if derived files are out of sync instead of regenerating them.
        #[arg(long)]
        verify: bool,
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

/// Projection target for `vox ci operations-sync`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum OperationsSyncTarget {
    /// Build or verify `contracts/operations/catalog.v1.yaml` from live registries.
    Catalog,
    /// Build or verify `contracts/mcp/tool-registry.canonical.yaml` from operations catalog.
    Mcp,
    /// Build or verify `contracts/cli/command-registry.yaml` from operations catalog.
    Cli,
    /// Build or verify `contracts/capability/capability-registry.yaml` from the catalog `capability` block.
    Capability,
    /// Verify or write MCP + CLI + capability registry projections (`mcp`, then `cli`, then `capability`).
    All,
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

/// Subcommands for [`CiCmd::MensScorecard`].
#[derive(Subcommand)]
pub enum MensScorecardCmd {
    /// Validate scorecard spec against `contracts/eval/mens-scorecard.schema.json`.
    Verify {
        /// Benchmark spec path (repo-relative unless absolute).
        #[arg(long, default_value = "contracts/eval/mens-scorecard.baseline.json")]
        spec: PathBuf,
    },
    /// Execute scorecard benchmark and emit artifacts (`events.jsonl`, `summary.json`).
    Run {
        /// Benchmark spec path (repo-relative unless absolute).
        #[arg(long, default_value = "contracts/eval/mens-scorecard.baseline.json")]
        spec: PathBuf,
        /// Optional output directory; default `mens/eval/runs/<utc_ts>`.
        #[arg(long)]
        out_dir: Option<PathBuf>,
    },
    /// Apply custom-model go/no-go threshold policy from one or more summary files.
    Decide {
        /// Summary JSON paths from prior `mens-scorecard run`.
        #[arg(long = "summary", required = true)]
        summaries: Vec<PathBuf>,
        /// Print machine-readable JSON only.
        #[arg(long)]
        json: bool,
    },
    /// Evaluate Burn R&D expected-value role against QLoRA summaries.
    #[command(name = "burn-rnd")]
    BurnRnd {
        /// Baseline QLoRA summary JSON.
        #[arg(long)]
        qlora_summary: PathBuf,
        /// Optional Burn/scratch summary JSON.
        #[arg(long)]
        burn_summary: Option<PathBuf>,
        /// Print machine-readable JSON only.
        #[arg(long)]
        json: bool,
    },
    /// Ingest `summary.json` from a scorecard run into VoxDb trust observations (needs Turso/Arca).
    #[command(name = "ingest-trust")]
    IngestTrust {
        /// Summary JSON path (repo-relative unless absolute).
        #[arg(long = "summary")]
        summary: PathBuf,
    },
}
