//! Shared library for the **`vox`** and **`vox-compilerd`** binaries.
//!
//! For the end-user CLI surface and subcommand map, see the `vox` binary crate root
//! (`src/main.rs`) and repository `docs/src/reference/cli.md`.

#![allow(clippy::collapsible_if)]
#![allow(clippy::drop_non_drop)]

pub mod benchmark_telemetry;
#[cfg(feature = "script-execution")]
mod build_lock;
mod build_service;
pub mod cli_actions;
pub mod cli_args;
mod cli_dispatch;
mod codex_cmd;
mod command_contract;
mod command_registry_model;
use crate::codex_cmd::CodexCmd;
pub mod artifact_policy;
pub mod command_catalog;
pub mod commands;
pub mod compilerd;
pub mod config;
/// External `vox-dei-d` RPC boundary (method id SSOT).
pub mod dei_daemon;
/// Colored CLI output helpers (`print_info`, `print_success`, …).
pub mod diagnostics;
mod dispatch;
pub mod dispatch_protocol;
/// Vite/React scaffold helpers and shared **pnpm** executable resolution (`pnpm_executable`).
pub mod frontend;
pub mod fs_utils;
mod island_paths;
#[cfg(feature = "script-execution")]
mod isolation;
mod latin_cmd;
/// Lock-wait JSONL metrics (`vox lock-report`, recursive script guard).
#[cfg(any(
    feature = "codex",
    feature = "stub-check",
    feature = "script-execution"
))]
mod lock_telemetry;
pub mod pipeline;
#[cfg(feature = "populi")]
mod populi_codex_telemetry;
mod process_supervision;
/// Terminal Markdown renderer + human-in-the-loop prompt helpers (CLI SSOT).
pub(crate) mod render;
#[cfg(feature = "island")]
mod table;
mod telemetry_spool;
pub mod templates;
/// WASI preopen mode for `script-execution` / `execution-api` runners.
#[cfg(any(feature = "script-execution", feature = "execution-api"))]
mod wasi_dir_mode;
mod watcher;
pub mod workflow_journal_codex;
/// Workspace journey VoxDb connect for repo-scoped CLI subcommands.
pub mod workspace_db;

/// Legacy v0 integration helpers (external codegen API).
pub mod v0;
/// Normalize v0.dev TSX for Vox `routes:` named imports.
pub(crate) mod v0_tsx_normalize;

pub use dispatch_protocol::{DispatchPayload, DispatchRequest, DispatchResponse};

use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;

/// Build version string: `0.x.y+build.N (githash)`
pub const VOX_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "+build.",
    env!("VOX_BUILD_NUMBER"),
    " (",
    env!("VOX_GIT_HASH"),
    ")",
);

/// Initialize [`tracing`] for `vox` / `vox-compilerd`: respects `RUST_LOG`, defaults to `info`.
///
/// Uses `tracing_subscriber::fmt` with [`tracing_subscriber::EnvFilter`]. Safe to call once per
/// process; repeated calls are ignored (`try_init`).
pub fn init_tracing_for_cli() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

/// Global flags available before every subcommand (`vox --color never build …`).
#[derive(Args, Clone, Debug, Default)]
pub struct GlobalOpts {
    /// When to emit ANSI colors (`NO_COLOR` still disables).
    #[arg(long, global = true, value_name = "WHEN", value_enum)]
    pub color: Option<crate::diagnostics::ColorChoice>,
    /// Hint subcommands to prefer machine JSON where supported (`VOX_CLI_GLOBAL_JSON=1`).
    #[arg(long, global = true)]
    pub json: bool,
    /// More verbose logs (sets `RUST_LOG=debug` when unset — see [`run_vox_cli_from_parsed`] before tracing init).
    #[arg(long, global = true, short = 'v')]
    pub verbose: bool,
    /// Quieter stderr for supported subcommands (`VOX_CLI_QUIET=1`).
    #[arg(long, global = true, short = 'q')]
    pub quiet: bool,
}

/// Full `vox` invocation: global options + subcommand.
#[derive(Parser)]
#[command(
    name = "vox",
    about = "The Vox AI-native language compiler",
    long_about = "The Vox AI-native language compiler.\n\nDiscover commands dynamically:\n  vox commands --recommended\n  vox commands --format json --include-nested",
    version = VOX_VERSION
)]
pub struct VoxCliRoot {
    /// Global application options.
    #[command(flatten)]
    pub global: GlobalOpts,
    /// Core parsed CLI subcommand execution variant.
    #[command(subcommand)]
    pub cmd: Cli,
}

/// Collection of subcommands exposing all features of the `vox` binary.
#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Cli {
    /// Emit shell completions for `vox` (bash, zsh, fish, powershell, elvish).
    Completions {
        /// Target shell.
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Print a dynamic command catalog generated from the clap command tree.
    Commands {
        /// Output format.
        #[arg(long, value_enum, default_value_t = command_catalog::CatalogFormat::Text)]
        format: command_catalog::CatalogFormat,
        /// Show only commands recommended for first-time users.
        #[arg(long)]
        recommended: bool,
        /// Include nested subcommands (default shows top-level only).
        #[arg(long)]
        include_nested: bool,
    },
    /// Workshop lane — same as top-level `build` (`fabrica` = Latin *workshop*).
    #[command(name = "fabrica", visible_alias = "fab")]
    Fabrica {
        /// Subcommand.
        #[command(subcommand)]
        cmd: latin_cmd::FabricaCmd,
    },
    /// Diagnostics lane — doctor, architect, stub-check (`diag`).
    #[command(name = "diag")]
    Diag {
        /// Subcommand.
        #[command(subcommand)]
        cmd: latin_cmd::DiagCmd,
    },
    /// Craft / skills lane — snippet, share, skill, … (`ars`).
    #[command(name = "ars")]
    Ars {
        /// Subcommand.
        #[command(subcommand)]
        cmd: latin_cmd::ArsCmd,
    },
    /// Central secret lifecycle and diagnostics (`clavis`; alias: `secrets`).
    #[command(name = "clavis", visible_alias = "secrets")]
    Clavis {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::clavis::ClavisCmd,
    },
    /// Review lane — CodeRabbit flows (`recensio`; alias of `review` when built with `coderabbit`).
    #[cfg(feature = "coderabbit")]
    #[command(name = "recensio", visible_alias = "rec")]
    Recensio {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::review::ReviewCli,
    },
    /// Manage global configuration and preferences.
    Config {
        /// Subcommand
        #[command(subcommand)]
        cmd: commands::config::ConfigCmd,
    },
    /// Identity and master key integration (`vox auth`).
    Auth {
        /// Subcommand
        #[command(subcommand)]
        cmd: commands::auth::AuthCmd,
    },
    /// Build a Vox source file, producing TypeScript output
    Build {
        /// Arguments.
        #[command(flatten)]
        args: cli_args::BuildArgs,
    },
    /// Type-check a Vox source file without producing output
    Check {
        /// Arguments.
        #[command(flatten)]
        args: cli_args::CheckArgs,
    },
    /// Run tests for the Vox program
    Test {
        /// Arguments.
        #[command(flatten)]
        args: cli_args::TestArgs,
    },
    /// Run a Vox source file (build + cargo run in generated project)
    Run {
        /// Arguments.
        #[command(flatten)]
        args: cli_args::RunArgs,
    },
    /// Run a `.vox` script (`fn main()`) via the native script cache (needs `--features script-execution`).
    #[cfg(feature = "script-execution")]
    Script {
        /// Arguments.
        #[command(flatten)]
        args: cli_args::ScriptArgs,
    },
    #[cfg(not(feature = "script-execution"))]
    #[command(name = "script")]
    ScriptStub {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        _args: Vec<String>,
    },
    /// Watch and rebuild via `vox-compilerd` (install daemon next to `vox` or on PATH)
    Dev {
        /// Arguments.
        #[command(flatten)]
        args: cli_args::DevArgs,
    },
    /// In-process orchestrator event dashboard (requires `--features live`)
    #[cfg(feature = "live")]
    Live,
    /// Bundle a Vox source file into a complete web application
    Bundle {
        /// Arguments.
        #[command(flatten)]
        args: cli_args::BundleArgs,
    },
    /// Format a Vox source file in place
    Fmt {
        /// Arguments.
        #[command(flatten)]
        args: cli_args::FmtArgs,
    },
    /// Add a dependency to `Vox.toml` (manifest only; run `vox lock` then `vox sync` to materialize).
    Add {
        #[command(flatten)]
        args: cli_args::AddDependencyArgs,
    },
    /// Remove a dependency from `Vox.toml`.
    Remove {
        #[command(flatten)]
        args: cli_args::RemoveDependencyArgs,
    },
    /// Refresh `vox.lock` from the local PM index (project graph — not the Vox toolchain).
    Update,
    /// Resolve `Vox.toml` and write `vox.lock` without downloading artifacts.
    Lock {
        #[command(flatten)]
        args: cli_args::LockArgs,
    },
    /// Materialize registry packages from `vox.lock` into `.vox_modules/dl/`.
    Sync {
        #[command(flatten)]
        args: cli_args::SyncArgs,
    },
    /// Deploy from `Vox.toml` `[deploy]` (OCI build/push, compose, Kubernetes, or bare-metal SSH).
    Deploy {
        #[command(flatten)]
        args: cli_args::DeployArgs,
    },
    /// Advanced package manager / registry commands (`search`, `publish`, `vendor`, …).
    Pm {
        #[command(subcommand)]
        cmd: commands::pm::PmCli,
    },
    /// Agentic Planning tools: Create, replan, and bypass planning steps (`vox plan`).
    Plan {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::plan::PlanCmd,
    },
    /// Vox Visus: Voice of Vision. Agentic GUI visual intelligence and bug detection.
    #[cfg(feature = "dei")]
    Visus {
        #[command(subcommand)]
        cmd: commands::visus::VisusCmd,
    },
    /// Toolchain upgrade: `--source release` (checksums.txt binary) or `--source repo` (git + `cargo install --locked`); never edits `Vox.toml` / `vox.lock`.
    Upgrade {
        #[command(flatten)]
        args: cli_args::UpgradeToolchainArgs,
    },
    /// Scaffold a new Vox project (`Vox.toml`, `src/main.vox`, `.vox_modules/`, or `<name>.skill.md`).
    Init {
        /// Project / package name (defaults to current directory name).
        name: Option<String>,
        /// Package kind: `application`, `skill`, `agent`, `workflow`, `chatbot`, `library`, …
        #[arg(long)]
        kind: Option<String>,
        /// Application template: `chatbot`, `dashboard`, or `api` (with `--kind application` or default).
        #[arg(long)]
        template: Option<String>,
    },
    /// Deprecated compatibility command; use `vox clavis set` instead.
    Login {
        /// Registry name (for example `google`, `openrouter`, `voxpm`).
        #[arg(long)]
        registry: Option<String>,
        /// Token to store (omit for interactive prompt).
        token: Option<String>,
        /// Optional username for registry flows.
        #[arg(long)]
        username: Option<String>,
    },
    /// Deprecated compatibility command; use `vox clavis` instead.
    Logout {
        /// Registry to remove (default `voxpm`).
        #[arg(long)]
        registry: Option<String>,
    },
    /// Start the Vox Language Server
    Lsp,
    /// Source migrations for React interop / retired web syntax (`migrate web`, …).
    Migrate {
        #[command(subcommand)]
        cmd: commands::migrate::MigrateCmd,
    },
    /// Start the Vox MCP (Model Context Protocol) server
    Mcp,
    /// Export the Vox language grammar in various formats for MENS training.
    Grammar {
        /// Arguments.
        #[command(flatten)]
        args: commands::grammar::GrammarParams,
    },
    /// Check toolchain and local environment readiness (`--build-perf` / `--json` need `--features codex`)
    Doctor {
        /// Arguments.
        #[command(flatten)]
        args: cli_args::DoctorArgs,
    },
    /// Workspace layout validation + god-object scan (needs `--features codex` or `stub-check`)
    #[cfg(any(feature = "codex", feature = "stub-check"))]
    Architect {
        /// Subcommand.
        #[command(subcommand)]
        cmd: cli_actions::ArchitectAction,
    },
    /// Snippet helpers (local Arca `VoxDb`; `VOX_DB_*` / Turso aliases or project `.vox/store.db`)
    Snippet {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::extras::snippet_cli::SnippetCli,
    },
    /// Share / search packages via local Arca index
    Share {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::extras::share_cli::ShareCli,
    },
    /// Interactive shell or PowerShell AST exec-policy check (`shell check`, `shell repl`).
    Shell {
        /// Subcommand (default: `repl`).
        #[command(subcommand)]
        cmd: Option<commands::runtime::shell::ShellCmd>,
    },
    /// Codex / Arca database tools (verify, legacy JSONL export/import)
    Codex {
        /// Subcommand.
        #[command(subcommand)]
        cmd: CodexCmd,
    },
    /// Manage the Vox attention-budgeting system and A2A thresholds.
    #[cfg(feature = "dei")]
    Attention {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::attention::AttentionCommand,
    },
    /// Repository discovery status, catalog (`.vox/repositories.yaml`), and cross-repo queries.
    Repo {
        /// Subcommand (`Option` so bare `vox repo` defaults to status in dispatch).
        #[command(subcommand)]
        cmd: Option<commands::repo::RepoCmd>,
    },
    /// Local VoxDB: schema, samples, research ingest, preferences
    Db {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::db_cli::DbCli,
    },
    /// Manage the `.vox/repositories.yaml` cross-repo catalog.
    Catalog {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::catalog::CatalogCmd,
    },
    /// Vox Scientia — research / capability map facade (delegates to `vox db` tools)
    Scientia {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::scientia::ScientiaCmd,
    },
    /// Manage the DEI (Distributed Execution Intelligence) orchestrator.
    #[cfg(feature = "dei")]
    #[command(visible_alias = "orchestrator")]
    Dei {
        /// Subcommand.
        #[command(subcommand)]
        cmd: crate::commands::dei::DeiCli,
    },
    /// OpenClaw / ClawHub gateway (skill import, approvals); requires `--features ars`
    #[cfg(feature = "ars")]
    #[command(visible_alias = "oc")]
    Openclaw {
        /// Action.
        #[command(subcommand)]
        action: commands::openclaw::OpenClawAction,
    },
    /// ARS skill registry + promote / context (`--features ars`)
    #[cfg(feature = "ars")]
    Skill {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::extras::skill_cmd::SkillCmd,
    },
    /// Ludus gamification (`--features extras-ludus`)
    #[cfg(feature = "extras-ludus")]
    Ludus {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::extras::ludus_cli::LudusCli,
    },
    /// TOESTUB scan + Codex baselines / Ludus rewards (`--features stub-check`)
    #[cfg(feature = "stub-check")]
    StubCheck {
        /// Arguments.
        #[command(flatten)]
        args: cli_args::StubCheckArgs,
    },
    /// CI guards: manifest, SSOT checks, feature matrix, doc inventory (no shell/Python required).
    Ci {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::ci::CiCmd,
    },
    /// Mens: train, serve, corpus, eval (delegated to vox-mens)
    #[command(name = "mens")]
    Mens {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Unified research operations: infrastructure (up/down/status) and eval.
    Research {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::research::ResearchCmd,
    },
    /// Oratio: speech-to-text / transcripts (delegated to vox-mens).
    #[command(name = "oratio", visible_alias = "speech")]
    Oratio {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// CodeRabbit batch PRs + ingest (`--features coderabbit`).
    #[cfg(feature = "coderabbit")]
    Review {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::review::ReviewCli,
    },
    /// v0.dev React islands under `islands/` (`--features island`; needs `V0_API_KEY` for generate/upgrade).
    #[cfg(feature = "island")]
    Island {
        /// Subcommand.
        #[command(subcommand)]
        cmd: cli_actions::IslandCli,
    },
    /// Fine-tune: legacy entry (delegated to vox-mens)
    #[command(name = "train")]
    Train {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Populi registry + HTTP control plane (delegated to vox-mens)
    #[command(name = "populi")]
    Populi {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Training tools (delegated to vox-mens)
    #[command(name = "schola")]
    Schola {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Emergency stop the orchestrator (MCP/daemon local stop request)
    Stop {
        /// Reason for stopping
        reason: Option<String>,
    },
    /// Optional telemetry upload queue (local spool + explicit upload; ADR 023).
    Telemetry {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::telemetry::TelemetryCmd,
    },
}

/// Apply [`GlobalOpts`] (color, JSON hint, quiet) before dispatching a subcommand.
#[allow(unsafe_code)]
pub fn apply_global_opts(g: &GlobalOpts) {
    if let Some(c) = g.color {
        crate::diagnostics::set_color_choice(c);
    }
    if g.json {
        // SAFETY: CLI startup is single-threaded before Tokio worker threads spawn env readers.
        unsafe {
            crate::config::set_process_env("VOX_CLI_GLOBAL_JSON", "1");
        }
    }
    if g.quiet {
        unsafe {
            crate::config::set_process_env("VOX_CLI_QUIET", "1");
        }
    }
}

/// Run the `vox` CLI (parsed from `std::env::args`).
pub async fn run_vox_cli() -> anyhow::Result<()> {
    let root = VoxCliRoot::parse();
    run_vox_cli_from_parsed(root).await
}

/// Run after parsing a [`VoxCliRoot`]: optional `RUST_LOG=debug` for `--verbose`, [`init_tracing_for_cli`], then dispatch.
#[allow(unsafe_code)]
pub async fn run_vox_cli_from_parsed(root: VoxCliRoot) -> anyhow::Result<()> {
    if root.global.verbose && std::env::var_os("RUST_LOG").is_none() {
        // SAFETY: CLI startup is single-threaded before Tokio workers read `RUST_LOG`.
        unsafe {
            crate::config::set_process_env("RUST_LOG", "debug");
        }
    }
    init_tracing_for_cli();
    apply_global_opts(&root.global);
    cli_dispatch::dispatch_cli(root.cmd, &root.global).await
}


