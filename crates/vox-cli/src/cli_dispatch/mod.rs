//! Subcommand dispatch and fabrica / Latin lane helpers.

mod lanes;

use crate::command_catalog;
use crate::latin_cmd;
use crate::{Cli, CodexCmd, GlobalOpts, VoxCliRoot};

use lanes::{cli_top_level_into_fabrica_or_self, run_ars_cmd, run_diag_cmd, run_fabrica_cmd};
#[cfg(feature = "ars")]
use lanes::run_openclaw_subcommand;
#[cfg(feature = "coderabbit")]
use lanes::run_review_subcommand;
#[cfg(feature = "script-execution")]
use lanes::run_script_subcommand;

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
            crate::commands::clavis::run(cmd).await?;
        }
        #[cfg(feature = "coderabbit")]
        Cli::Recensio { cmd } => {
            run_review_subcommand(cmd).await?;
        }
        Cli::Ci { cmd } => {
            crate::commands::ci::run(cmd).await?;
        }
        #[cfg(feature = "script-execution")]
        Cli::Script { args } => {
            run_script_subcommand(&args, "top-level").await?;
        }
        #[cfg(feature = "live")]
        Cli::Live => {
            crate::commands::live::run().await?;
        }
        Cli::Install { package_name } => {
            crate::commands::install::run(Some(&package_name), false).await?;
        }
        Cli::Login {
            registry,
            token,
            username,
        } => {
            eprintln!(
                "warning: `vox login` is deprecated; use `vox clavis set <registry> <token>`."
            );
            crate::commands::login::run(token.as_deref(), registry.as_deref(), username.as_deref())
                .await?;
        }
        Cli::Logout { registry } => {
            eprintln!("warning: `vox logout` is deprecated; use `vox clavis` management commands.");
            crate::commands::logout::run(registry.as_deref()).await?;
        }
        Cli::Lsp => {
            crate::commands::lsp::run()?;
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
            crate::commands::db_cli::run(cmd).await?;
        }
        Cli::Scientia { cmd } => {
            crate::commands::scientia::run(cmd).await?;
        }
        #[cfg(feature = "dei")]
        Cli::Dei { cmd } => {
            crate::commands::dei::run(cmd).await?;
        }
        Cli::Codex { cmd } => match cmd {
            CodexCmd::Verify => {
                crate::commands::codex::verify().await?;
            }
            CodexCmd::ExportLegacy { output } => {
                crate::commands::codex::export_legacy(&output).await?;
            }
            CodexCmd::ImportLegacy { input } => {
                crate::commands::codex::import_legacy(&input).await?;
            }
            CodexCmd::ImportOrchestratorMemory {
                dir,
                agent_id,
                session_id,
            } => {
                crate::commands::codex::import_orchestrator_memory(dir, agent_id, session_id)
                    .await?;
            }
            CodexCmd::ImportSkillBundle { file } => {
                crate::commands::codex::import_skill_bundle(file).await?;
            }
            CodexCmd::Cutover {
                target_db,
                source_db,
                artifact_dir,
                force,
            } => {
                crate::commands::codex::cutover(artifact_dir, target_db, source_db, force).await?;
            }
            CodexCmd::SocratesMetrics {
                repository_id,
                limit,
            } => {
                crate::commands::codex::socrates_metrics(repository_id, limit).await?;
            }
            CodexCmd::SocratesEvalSnapshot {
                eval_id,
                repository_id,
                limit,
            } => {
                crate::commands::codex::socrates_eval_snapshot(eval_id, repository_id, limit)
                    .await?;
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
            crate::commands::mens::run(action, global.json, global.verbose).await?;
        }
        #[cfg(feature = "oratio")]
        Cli::Oratio { action } => {
            crate::commands::oratio_cmd::run(action, global.json)?;
        }
        #[cfg(feature = "coderabbit")]
        Cli::Review { cmd } => {
            run_review_subcommand(cmd).await?;
        }
        #[cfg(feature = "island")]
        Cli::Island { cmd } => {
            crate::commands::island::run(cmd).await?;
        }
        #[cfg(all(feature = "gpu", feature = "mens-dei"))]
        Cli::Train { args } => {
            crate::commands::ai::train::run(
                args.data_dir.clone(),
                args.output_dir.clone(),
                args.provider.clone(),
                args.native,
            )
            .await?;
        }
        #[cfg(feature = "populi")]
        Cli::Populi { cmd } => {
            crate::commands::populi_cli::run(cmd, global.json).await?;
        }
    }

    Ok(())
}
