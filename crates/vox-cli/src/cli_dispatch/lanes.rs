//! Fabrica / `diag` / `ars` lane dispatch helpers.

use crate::Cli;
use crate::cli_args;
use crate::commands;
use crate::latin_cmd;

pub(super) async fn run_doctor_command(args: &cli_args::DoctorArgs) -> anyhow::Result<()> {
    commands::diagnostics::doctor::run(
        args.auto_heal,
        args.test_health,
        args.build_perf,
        args.scope,
        args.json,
        args.probe,
        args.fix_cuda_path,
    )
    .await
}

#[cfg(feature = "stub-check")]
pub(super) async fn run_stub_check_command(args: &cli_args::StubCheckArgs) -> anyhow::Result<()> {
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
pub(super) fn script_opts_for_cli(
    args: &cli_args::ScriptArgs,
) -> commands::runtime::run::script::ScriptOpts {
    commands::runtime::run::script::ScriptOpts {
        sandbox: args.sandbox,
        allow_mcp: false,
        no_cache: args.no_cache,
        isolation: args.isolation.clone(),
        trust_class: args.trust_class.clone(),
        wasi_dirs: Vec::new(),
        target_triple: args.target_triple.clone(),
    }
}

#[cfg(feature = "script-execution")]
pub(super) async fn run_script_subcommand(
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
pub(super) async fn run_openclaw_subcommand(
    action: commands::openclaw::OpenClawAction,
) -> anyhow::Result<()> {
    commands::openclaw::run(action, false).await
}

#[cfg(feature = "coderabbit")]
pub(super) async fn run_review_subcommand(cmd: commands::review::ReviewCli) -> anyhow::Result<()> {
    commands::review::run_coderabbit(cmd).await
}

/// Top-level `vox build` / `check` / … shims that map 1:1 onto [`latin_cmd::FabricaCmd`].
///
/// `Script` is not included: top-level `vox script` uses [`run_script_subcommand`] instead of `fabrica script`.
#[allow(clippy::result_large_err)]
pub(super) fn cli_top_level_into_fabrica_or_self(
    cli: Cli,
) -> std::result::Result<latin_cmd::FabricaCmd, Cli> {
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

pub(super) async fn run_fabrica_cmd(cmd: latin_cmd::FabricaCmd) -> anyhow::Result<()> {
    use latin_cmd::FabricaCmd;
    match cmd {
        FabricaCmd::Build(a) => {
            commands::build::run(&a.file, &a.out_dir, a.target.clone(), a.scaffold, a.emit_ir, a.mode)
                .await?;
        }
        FabricaCmd::Check(a) => {
            commands::check::run(&a).await?;
        }
        FabricaCmd::Test(a) => {
            commands::test::run(&a).await?;
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
            commands::bundle::run(&a.file, &a.out_dir, a.target.as_deref(), a.release, a.mode)
                .await?;
        }
        FabricaCmd::Fmt(a) => {
            commands::fmt::run(&a.file, a.check)?;
        }
        #[cfg(feature = "script-execution")]
        FabricaCmd::Script(a) => {
            run_script_subcommand(&a, "fabrica").await?;
        }
    }
    Ok(())
}

pub(super) async fn run_diag_cmd(cmd: latin_cmd::DiagCmd) -> anyhow::Result<()> {
    use latin_cmd::DiagCmd;
    match cmd {
        DiagCmd::Doctor(a) => {
            run_doctor_command(&a).await?;
        }
        #[cfg(any(feature = "codex", feature = "stub-check"))]
        DiagCmd::Architect { cmd } => {
            commands::diagnostics::tools::architect::run(cmd).await?;
        }
        #[cfg(feature = "stub-check")]
        DiagCmd::StubCheck(a) => {
            run_stub_check_command(&a).await?;
        }
    }
    Ok(())
}

pub(super) async fn run_ars_cmd(cmd: latin_cmd::ArsCmd) -> anyhow::Result<()> {
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
