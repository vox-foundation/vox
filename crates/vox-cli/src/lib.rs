//! Shared library for the **`vox`** and **`vox-compilerd`** binaries.
//!
//! For the end-user CLI surface and subcommand map, see the `vox` binary crate root
//! (`src/main.rs`) and repository `docs/src/ref-cli.md`.

#![allow(clippy::collapsible_if)]
#![allow(clippy::drop_non_drop)]

pub mod benchmark_telemetry;
#[cfg(feature = "script-execution")]
mod build_lock;
mod build_service;
mod cli_actions;
mod cli_args;
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
#[cfg(any(feature = "codex", feature = "stub-check"))]
mod lock_telemetry;
#[cfg(feature = "mesh")]
mod mesh_codex_telemetry;
pub mod pipeline;
#[cfg(feature = "island")]
mod table;
pub mod templates;
mod training;
/// WASI preopen mode for `script-execution` / `execution-api` runners.
#[cfg(any(feature = "script-execution", feature = "execution-api"))]
mod wasi_dir_mode;
mod watcher;
#[cfg(feature = "workflow-runtime")]
mod workflow_journal_codex;

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
/// Uses [`tracing_subscriber::fmt`] with [`tracing_subscriber::EnvFilter`]. Safe to call once per
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
    /// Workshop lane — same as top-level `build` (`fabrica` = Latin *workshop*).
    #[command(name = "fabrica", visible_alias = "fab")]
    Fabrica {
        /// Subcommand.
        #[command(subcommand)]
        cmd: latin_cmd::FabricaCmd,
    },
    /// Mind / diagnostics lane — doctor, architect, stub-check (`mens`).
    #[command(name = "mens")]
    Mens {
        /// Subcommand.
        #[command(subcommand)]
        cmd: latin_cmd::MensCmd,
    },
    /// Craft / skills lane — snippet, share, skill, … (`ars`).
    #[command(name = "ars")]
    Ars {
        /// Subcommand.
        #[command(subcommand)]
        cmd: latin_cmd::ArsCmd,
    },
    /// Review lane — CodeRabbit flows (`recensio`; alias of `review` when built with `coderabbit`).
    #[cfg(feature = "coderabbit")]
    #[command(name = "recensio", visible_alias = "rec")]
    Recensio {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::review::ReviewCli,
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
    /// Install a component or package via vox-pm
    Install {
        /// Target package name.
        #[arg(required = true)]
        package_name: String,
    },
    /// Start the Vox Language Server
    Lsp,
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
    /// Snippet helpers (local Arca `CodeStore`; `VOX_DB_*` / Turso aliases or project `.vox/store.db`)
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
    /// Codex / Arca database tools (verify, legacy JSONL export/import)
    Codex {
        /// Subcommand.
        #[command(subcommand)]
        cmd: CodexCmd,
    },
    /// Local VoxDB: schema, samples, research ingest, preferences
    Db {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::db_cli::DbCli,
    },
    /// Vox Scientia — research / capability map facade (delegates to `vox db` tools)
    Scientia {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::scientia::ScientiaCmd,
    },
    /// OpenClaw / ClawHub gateway (skill import, approvals); requires `--features ars`
    #[cfg(feature = "ars")]
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
    /// Populi: train, serve, corpus, eval (`populi-base` default; native train needs `gpu`)
    #[cfg(any(feature = "populi-base", feature = "gpu"))]
    Populi {
        /// Action.
        #[command(subcommand)]
        action: commands::populi::PopuliAction,
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
    /// Fine-tune: legacy entry — **`--provider local`** bails with **`vox populi train --backend qlora …`**; Together API; **`--native`** Burn scratch (requires `gpu` + `populi-dei`). **Canonical native QLoRA:** `vox populi train`.
    #[cfg(all(feature = "gpu", feature = "populi-dei"))]
    Train {
        /// Arguments.
        #[command(flatten)]
        args: cli_args::TrainLegacyArgs,
    },
    /// Mesh registry + HTTP control plane (`--features mesh`).
    #[cfg(feature = "mesh")]
    Mesh {
        /// Subcommand.
        #[command(subcommand)]
        cmd: commands::mesh_cli::MeshCli,
    },
}

/// Subcommands for the legacy `vox codex` facade.
#[derive(Subcommand)]
pub enum CodexCmd {
    /// Print schema version and whether Codex reactivity (V8) tables exist
    Verify,
    /// Export configured legacy tables as JSONL (see `vox_db::codex_legacy::LEGACY_EXPORT_TABLES`)
    ExportLegacy {
        /// Output file path
        #[arg(long, short = 'o')]
        output: std::path::PathBuf,
    },
    /// Import JSONL produced by `export-legacy`
    ImportLegacy {
        /// Input file path
        #[arg(long, short = 'i')]
        input: std::path::PathBuf,
    },
    /// Aggregate MCP Socrates `research_metrics` rows and print JSON (`SocratesSurfaceAggregate`)
    #[command(name = "socrates-metrics")]
    SocratesMetrics {
        /// Restrict to sessions `mcp:<repository_id>` (omit to include all repos)
        #[arg(long)]
        repository_id: Option<String>,
        /// Max recent `socrates_surface` rows to scan
        #[arg(long, default_value_t = 500)]
        limit: i64,
    },
    /// Append one `eval_runs` summary from recent Socrates metrics (cron-friendly)
    #[command(name = "socrates-eval-snapshot")]
    SocratesEvalSnapshot {
        /// Stable id for this snapshot (e.g. `daily-2026-03-21` or CI build id)
        #[arg(long)]
        eval_id: String,
        /// Optional repository constraint.
        #[arg(long)]
        repository_id: Option<String>,
        /// Number of recent metrics to pull into the snapshot.
        #[arg(long, default_value_t = 500)]
        limit: i64,
    },
}

async fn run_doctor_command(args: &cli_args::DoctorArgs) -> anyhow::Result<()> {
    commands::diagnostics::doctor::run(
        args.auto_heal,
        args.test_health,
        args.build_perf,
        args.scope,
        args.json,
    )
    .await
}

#[cfg(feature = "stub-check")]
async fn run_stub_check_command(args: &cli_args::StubCheckArgs) -> anyhow::Result<()> {
    let scan_root = args
        .path
        .clone()
        .or(args.scan_pos.clone())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    commands::stub_check::run(
        &scan_root,
        args.format.as_deref(),
        args.severity.as_deref(),
        args.suggest_fixes,
        args.rules.as_deref(),
        &args.excludes,
        args.langs.as_deref(),
        args.baseline.as_deref(),
        args.save_baseline.as_deref(),
        args.task_list,
        args.import_suppressions,
        args.ingest_findings.as_deref(),
        args.fix_pipeline,
        args.fix_pipeline_apply,
        args.gate.as_deref(),
        args.gate_budget_path.as_deref(),
        args.verify_impacted,
        args.max_escalation,
        args.self_heal_safe_mode,
    )
    .await
}

#[cfg(feature = "script-execution")]
fn script_opts_for_cli(args: &cli_args::ScriptArgs) -> commands::runtime::run::script::ScriptOpts {
    commands::runtime::run::script::ScriptOpts {
        sandbox: args.sandbox,
        allow_mcp: false,
        no_cache: args.no_cache,
        isolation: args.isolation.clone(),
        trust_class: args.trust_class.clone(),
        wasi_dirs: Vec::new(),
    }
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
    dispatch_cli(root.cmd, &root.global).await
}

/// Run as `vox populi …` while the process argv is `vox-populi …` (inserts the `populi` subcommand).
///
/// Used by the **`vox-populi`** binary (`required-features = ["populi-base"]`).
pub async fn run_vox_cli_populi_prefixed() -> anyhow::Result<()> {
    let mut args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        anyhow::bail!(
            "usage: vox-populi <subcommand> …\n  Equivalent to: vox populi <subcommand> …\n  Native training needs: cargo build -p vox-cli --features gpu"
        );
    }
    args.insert(1, "populi".into());
    let root = VoxCliRoot::try_parse_from(&args).map_err(|e| anyhow::anyhow!("{e}"))?;
    run_vox_cli_from_parsed(root).await
}

#[cfg(feature = "script-execution")]
async fn run_script_subcommand(
    args: &cli_args::ScriptArgs,
    lane: &'static str,
) -> anyhow::Result<()> {
    tracing::info!(
        target: "vox.script",
        path = %args.file.display(),
        lane = lane,
        "script subcommand"
    );
    let opts = script_opts_for_cli(args);
    crate::commands::runtime::run::script::run(&args.file, &args.args, &opts).await
}

#[cfg(feature = "ars")]
async fn run_openclaw_subcommand(action: commands::openclaw::OpenClawAction) -> anyhow::Result<()> {
    commands::openclaw::run(action, false).await
}

#[cfg(feature = "coderabbit")]
async fn run_review_subcommand(cmd: commands::review::ReviewCli) -> anyhow::Result<()> {
    commands::review::run_coderabbit(cmd).await
}

/// Top-level `vox build` / `check` / … shims that map 1:1 onto [`latin_cmd::FabricaCmd`].
///
/// `Script` is not included: top-level `vox script` uses [`run_script_subcommand`] instead of `fabrica script`.
#[allow(clippy::result_large_err)] // `Err` carries the full `Cli` for re-dispatch; boxing would churn hot path.
fn cli_top_level_into_fabrica_or_self(cli: Cli) -> Result<latin_cmd::FabricaCmd, Cli> {
    use latin_cmd::FabricaCmd;
    match cli {
        Cli::Build { args } => Ok(FabricaCmd::Build(args)),
        Cli::Check { args } => Ok(FabricaCmd::Check(args)),
        Cli::Test { args } => Ok(FabricaCmd::Test(args)),
        Cli::Run { args } => Ok(FabricaCmd::Run(args)),
        Cli::Dev { args } => Ok(FabricaCmd::Dev(args)),
        Cli::Bundle { args } => Ok(FabricaCmd::Bundle(args)),
        Cli::Fmt { args } => Ok(FabricaCmd::Fmt(args)),
        other => Err(other),
    }
}

async fn run_fabrica_cmd(cmd: latin_cmd::FabricaCmd) -> anyhow::Result<()> {
    use latin_cmd::FabricaCmd;
    match cmd {
        FabricaCmd::Build(a) => {
            commands::build::run(&a.file, &a.out_dir).await?;
        }
        FabricaCmd::Check(a) => {
            commands::check::run(&a.file, a.emit_training_jsonl.as_deref()).await?;
        }
        FabricaCmd::Test(a) => {
            commands::test::run(&a.file).await?;
        }
        FabricaCmd::Run(a) => {
            if let Some(p) = a.port {
                crate::config::set_process_vox_port(p);
            }
            commands::run::run(&a.file, &a.args, a.mode).await?;
        }
        FabricaCmd::Dev(a) => {
            commands::dev::run(&a.file, &a.out_dir, a.port, a.open).await?;
        }
        FabricaCmd::Bundle(a) => {
            commands::bundle::run(&a.file, &a.out_dir, a.target.as_deref(), a.release).await?;
        }
        FabricaCmd::Fmt(a) => {
            commands::fmt::run(&a.file, false)?;
        }
        #[cfg(feature = "script-execution")]
        FabricaCmd::Script(a) => {
            run_script_subcommand(&a, "fabrica").await?;
        }
    }
    Ok(())
}

async fn run_mens_cmd(cmd: latin_cmd::MensCmd) -> anyhow::Result<()> {
    use latin_cmd::MensCmd;
    match cmd {
        MensCmd::Doctor(a) => {
            run_doctor_command(&a).await?;
        }
        #[cfg(any(feature = "codex", feature = "stub-check"))]
        MensCmd::Architect { cmd } => {
            commands::diagnostics::tools::architect::run(cmd).await?;
        }
        #[cfg(feature = "stub-check")]
        MensCmd::StubCheck(a) => {
            run_stub_check_command(&a).await?;
        }
    }
    Ok(())
}

async fn run_ars_cmd(cmd: latin_cmd::ArsCmd) -> anyhow::Result<()> {
    use latin_cmd::ArsCmd;
    match cmd {
        ArsCmd::Snippet { cmd } => {
            commands::extras::snippet_cli::run(cmd).await?;
        }
        ArsCmd::Share { cmd } => {
            commands::extras::share_cli::run(cmd).await?;
        }
        #[cfg(feature = "ars")]
        ArsCmd::Skill { cmd } => {
            commands::extras::skill_cmd::run(cmd).await?;
        }
        #[cfg(feature = "ars")]
        ArsCmd::Openclaw { action } => {
            run_openclaw_subcommand(action).await?;
        }
        #[cfg(feature = "extras-ludus")]
        ArsCmd::Ludus { cmd } => {
            commands::extras::ludus_cli::run(cmd).await?;
        }
    }
    Ok(())
}

async fn dispatch_cli(cli: Cli, global: &GlobalOpts) -> anyhow::Result<()> {
    #[cfg(not(any(feature = "populi-base", feature = "gpu")))]
    {
        let _ = global;
    }
    let cli = match cli_top_level_into_fabrica_or_self(cli) {
        Ok(cmd) => return run_fabrica_cmd(cmd).await,
        Err(cli) => cli,
    };
    match cli {
        // Compiler cannot narrow `Cli` after [`cli_top_level_into_fabrica_or_self`]; these are unreachable.
        Cli::Build { .. }
        | Cli::Check { .. }
        | Cli::Test { .. }
        | Cli::Run { .. }
        | Cli::Dev { .. }
        | Cli::Bundle { .. }
        | Cli::Fmt { .. } => {
            std::unreachable!("top-level fabrica shims are routed before this match")
        }
        Cli::Completions { shell } => {
            use clap::CommandFactory;
            let mut cmd = VoxCliRoot::command();
            clap_complete::generate(shell, &mut cmd, "vox", &mut std::io::stdout());
        }
        Cli::Fabrica { cmd } => {
            run_fabrica_cmd(cmd).await?;
        }
        Cli::Mens { cmd } => {
            run_mens_cmd(cmd).await?;
        }
        Cli::Ars { cmd } => {
            run_ars_cmd(cmd).await?;
        }
        #[cfg(feature = "coderabbit")]
        Cli::Recensio { cmd } => {
            run_review_subcommand(cmd).await?;
        }
        Cli::Ci { cmd } => {
            commands::ci::run(cmd)?;
        }
        #[cfg(feature = "script-execution")]
        Cli::Script { args } => {
            run_script_subcommand(&args, "top-level").await?;
        }
        #[cfg(feature = "live")]
        Cli::Live => {
            commands::live::run().await?;
        }
        Cli::Install { package_name } => {
            commands::install::run(Some(&package_name), false).await?;
        }
        Cli::Lsp => {
            commands::lsp::run()?;
        }
        Cli::Doctor { args } => {
            run_mens_cmd(latin_cmd::MensCmd::Doctor(args)).await?;
        }
        #[cfg(any(feature = "codex", feature = "stub-check"))]
        Cli::Architect { cmd } => {
            run_mens_cmd(latin_cmd::MensCmd::Architect { cmd }).await?;
        }
        Cli::Snippet { cmd } => {
            run_ars_cmd(latin_cmd::ArsCmd::Snippet { cmd }).await?;
        }
        Cli::Share { cmd } => {
            run_ars_cmd(latin_cmd::ArsCmd::Share { cmd }).await?;
        }
        Cli::Db { cmd } => {
            commands::db_cli::run(cmd).await?;
        }
        Cli::Scientia { cmd } => {
            commands::scientia::run(cmd).await?;
        }
        Cli::Codex { cmd } => match cmd {
            CodexCmd::Verify => {
                commands::codex::verify().await?;
            }
            CodexCmd::ExportLegacy { output } => {
                commands::codex::export_legacy(&output).await?;
            }
            CodexCmd::ImportLegacy { input } => {
                commands::codex::import_legacy(&input).await?;
            }
            CodexCmd::SocratesMetrics {
                repository_id,
                limit,
            } => {
                commands::codex::socrates_metrics(repository_id, limit).await?;
            }
            CodexCmd::SocratesEvalSnapshot {
                eval_id,
                repository_id,
                limit,
            } => {
                commands::codex::socrates_eval_snapshot(eval_id, repository_id, limit).await?;
            }
        },
        #[cfg(feature = "ars")]
        Cli::Openclaw { action } => {
            run_openclaw_subcommand(action).await?;
        }
        #[cfg(feature = "ars")]
        Cli::Skill { cmd } => {
            run_ars_cmd(latin_cmd::ArsCmd::Skill { cmd }).await?;
        }
        #[cfg(feature = "extras-ludus")]
        Cli::Ludus { cmd } => {
            run_ars_cmd(latin_cmd::ArsCmd::Ludus { cmd }).await?;
        }
        #[cfg(feature = "stub-check")]
        Cli::StubCheck { args } => {
            run_mens_cmd(latin_cmd::MensCmd::StubCheck(args)).await?;
        }
        #[cfg(any(feature = "populi-base", feature = "gpu"))]
        Cli::Populi { action } => {
            commands::populi::run(action, global.json, global.verbose).await?;
        }
        #[cfg(feature = "coderabbit")]
        Cli::Review { cmd } => {
            run_review_subcommand(cmd).await?;
        }
        #[cfg(feature = "island")]
        Cli::Island { cmd } => {
            commands::island::run(cmd).await?;
        }
        #[cfg(all(feature = "gpu", feature = "populi-dei"))]
        Cli::Train { args } => {
            commands::ai::train::run(
                args.data_dir.clone(),
                args.output_dir.clone(),
                args.provider.clone(),
                args.native,
            )
            .await?;
        }
        #[cfg(feature = "mesh")]
        Cli::Mesh { cmd } => {
            commands::mesh_cli::run(cmd, global.json).await?;
        }
    }

    Ok(())
}
