//! Subcommand dispatch and fabrica / latin lane helpers.

use crate::cli_args;
use crate::command_catalog;
use crate::commands;
use crate::latin_cmd;
use crate::{Cli, CodexCmd, GlobalOpts, VoxCliRoot};

async fn run_doctor_command(args: &cli_args::DoctorArgs) -> anyhow::Result<()> {
    commands::diagnostics::doctor::run(
        args.auto_heal,
        args.test_health,
        args.build_perf,
        args.scope,
        args.json,
        args.probe,
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

async fn run_diag_cmd(cmd: latin_cmd::DiagCmd) -> anyhow::Result<()> {
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

pub(crate) async fn dispatch_cli(cli: Cli, global: &GlobalOpts) -> anyhow::Result<()> {
    #[cfg(not(any(feature = "mens-base", feature = "gpu")))]
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
        Cli::Commands {
            format,
            recommended,
            include_nested,
        } => {
            let catalog = command_catalog::build_catalog();
            let selected =
                command_catalog::select_entries(catalog.entries, recommended, include_nested);
            match format {
                command_catalog::CatalogFormat::Text => {
                    println!("{}", command_catalog::render_text(&selected));
                }
                command_catalog::CatalogFormat::Json => {
                    let out = command_catalog::CommandCatalog {
                        generated_from: catalog.generated_from,
                        entries: selected,
                    };
                    println!("{}", serde_json::to_string_pretty(&out)?);
                }
            }
        }
        Cli::Fabrica { cmd } => {
            run_fabrica_cmd(cmd).await?;
        }
        Cli::Diag { cmd } => {
            run_diag_cmd(cmd).await?;
        }
        Cli::Ars { cmd } => {
            run_ars_cmd(cmd).await?;
        }
        Cli::Clavis { cmd } => {
            commands::clavis::run(cmd).await?;
        }
        #[cfg(feature = "coderabbit")]
        Cli::Recensio { cmd } => {
            run_review_subcommand(cmd).await?;
        }
        Cli::Ci { cmd } => {
            commands::ci::run(cmd).await?;
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
        Cli::Login {
            registry,
            token,
            username,
        } => {
            eprintln!(
                "warning: `vox login` is deprecated; use `vox clavis set <registry> <token>`."
            );
            commands::login::run(token.as_deref(), registry.as_deref(), username.as_deref())
                .await?;
        }
        Cli::Logout { registry } => {
            eprintln!("warning: `vox logout` is deprecated; use `vox clavis` management commands.");
            commands::logout::run(registry.as_deref()).await?;
        }
        Cli::Lsp => {
            commands::lsp::run()?;
        }
        Cli::Doctor { args } => {
            run_diag_cmd(latin_cmd::DiagCmd::Doctor(args)).await?;
        }
        #[cfg(any(feature = "codex", feature = "stub-check"))]
        Cli::Architect { cmd } => {
            run_diag_cmd(latin_cmd::DiagCmd::Architect { cmd }).await?;
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
        #[cfg(feature = "dei")]
        Cli::Dei { cmd } => {
            commands::dei::run(cmd).await?;
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
            CodexCmd::ImportOrchestratorMemory {
                dir,
                agent_id,
                session_id,
            } => {
                commands::codex::import_orchestrator_memory(dir, agent_id, session_id).await?;
            }
            CodexCmd::ImportSkillBundle { file } => {
                commands::codex::import_skill_bundle(file).await?;
            }
            CodexCmd::Cutover {
                target_db,
                source_db,
                artifact_dir,
                force,
            } => {
                commands::codex::cutover(artifact_dir, target_db, source_db, force).await?;
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
            run_diag_cmd(latin_cmd::DiagCmd::StubCheck(args)).await?;
        }
        #[cfg(any(feature = "mens-base", feature = "gpu"))]
        Cli::Mens { action } => {
            commands::mens::run(action, global.json, global.verbose).await?;
        }
        #[cfg(feature = "oratio")]
        Cli::Oratio { action } => {
            commands::oratio_cmd::run(action, global.json)?;
        }
        #[cfg(feature = "coderabbit")]
        Cli::Review { cmd } => {
            run_review_subcommand(cmd).await?;
        }
        #[cfg(feature = "island")]
        Cli::Island { cmd } => {
            commands::island::run(cmd).await?;
        }
        #[cfg(all(feature = "gpu", feature = "mens-dei"))]
        Cli::Train { args } => {
            commands::ai::train::run(
                args.data_dir.clone(),
                args.output_dir.clone(),
                args.provider.clone(),
                args.native,
            )
            .await?;
        }
        #[cfg(feature = "populi")]
        Cli::Populi { cmd } => {
            commands::populi_cli::run(cmd, global.json).await?;
        }
    }

    Ok(())
}
