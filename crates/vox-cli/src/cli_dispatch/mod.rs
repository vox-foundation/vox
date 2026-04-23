//! Subcommand dispatch and fabrica / Latin lane helpers.

mod lanes;

use crate::codex_cmd::CodexCmd;
use crate::command_catalog;
// use crate::latin_cmd; // Unused after alias retirement
use crate::{Cli, GlobalOpts, VoxCliRoot};

#[cfg(feature = "ars")]
pub(crate) use lanes::run_openclaw_subcommand;
#[cfg(feature = "coderabbit")]
pub(crate) use lanes::run_review_subcommand;
#[cfg(feature = "script-execution")]
pub(crate) use lanes::run_script_subcommand;
#[cfg(feature = "stub-check")]
pub(crate) use lanes::run_stub_check_command;
pub(crate) use lanes::{
    cli_top_level_into_fabrica_or_self, run_ars_cmd, run_diag_cmd, run_doctor_command,
    run_fabrica_cmd,
};

pub(crate) async fn dispatch_cli(cli: Cli, global: &GlobalOpts) -> anyhow::Result<()> {
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
        Cli::Ext { cmd } => {
            crate::commands::ext::run(cmd).await?;
        }
        Cli::Ars { cmd } => {
            run_ars_cmd(cmd).await?;
        }
        #[cfg(feature = "extras-ludus")]
        Cli::Ludus { cmd } => {
            crate::commands::extras::ludus_cli::run(cmd).await?;
        }
        Cli::Clavis { cmd } => {
            crate::commands::clavis::run(cmd).await?;
        }
        Cli::Auth { cmd } => {
            crate::commands::auth::run(cmd).await?;
        }
        Cli::Config { cmd } => {
            crate::commands::config::run(cmd).await?;
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
        #[cfg(not(feature = "script-execution"))]
        Cli::ScriptStub { .. } => {
            vox_build_meta::require(
                "script-execution",
                "cargo build -p vox-cli --features script-execution",
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;
            unreachable!()
        }
        #[cfg(feature = "live")]
        Cli::Live => {
            crate::commands::live::run().await?;
        }
        Cli::Add { args } => {
            crate::commands::add::run(&args.name, args.version.as_deref(), args.path.as_deref())
                .await?;
        }
        Cli::Remove { args } => {
            crate::commands::remove::run(&args.name).await?;
        }
        Cli::Update => {
            crate::commands::update::run().await?;
        }
        Cli::Lock { args } => {
            crate::commands::lock::run(args.locked).await?;
        }
        Cli::Sync { args } => {
            crate::commands::sync::run(args.registry.as_deref(), args.frozen).await?;
        }
        Cli::Login => {
            eprintln!("vox login is deprecated. Use `vox auth connect` instead.");
            std::process::exit(1);
        }
        Cli::Logout => {
            eprintln!("vox logout is deprecated. Use `vox auth` instead.");
            std::process::exit(1);
        }
        Cli::Share { cmd } => {
            crate::commands::extras::share_cli::run(cmd).await?;
        }
        Cli::Train { .. } => {
            eprintln!("vox train is deprecated. Use `vox mens train` instead.");
            std::process::exit(1);
        }
        Cli::Snippet { cmd } => {
            crate::commands::extras::snippet_cli::run(cmd).await?;
        }
        #[cfg(feature = "ars")]
        Cli::Skill { cmd } => {
            crate::commands::extras::skill_cmd::run(cmd).await?;
        }
        Cli::Deploy { args } => {
            crate::commands::deploy::run(args).await?;
        }
        Cli::Pm { cmd } => {
            crate::commands::pm::run(cmd).await?;
        }
        Cli::Doctor { args } => {
            run_doctor_command(&args).await?;
        }
        #[cfg(any(feature = "codex", feature = "stub-check"))]
        Cli::Architect { cmd } => {
            crate::commands::diagnostics::tools::architect::run(cmd).await?;
        }
        #[cfg(feature = "stub-check")]
        Cli::StubCheck { args } => {
            run_stub_check_command(&args).await?;
        }
        Cli::Upgrade { args } => {
            crate::commands::upgrade::run(&args, global.json).await?;
        }
        Cli::Init {
            name,
            kind,
            template,
        } => {
            crate::commands::init::run(name.as_deref(), kind.as_deref(), template.as_deref())
                .await?;
        }
        Cli::New { cmd } => {
            crate::commands::new::run(cmd).await?;
        }
        Cli::Play { args } => {
            crate::commands::play::run(args).await?;
        }
        Cli::Repair { args } => {
            crate::commands::repair::run(args).await?;
        }
        Cli::Lsp => {
            crate::commands::lsp::run()?;
        }
        Cli::Migrate { cmd } => {
            crate::commands::migrate::run(cmd)?;
        }
        Cli::Mcp => {
            crate::commands::mcp::run()?;
        }
        Cli::Shell { cmd } => {
            crate::commands::runtime::shell::run(cmd).await?;
        }
        Cli::Db { cmd } => {
            crate::commands::db_cli::run(cmd).await?;
        }
        Cli::Scientia { cmd } => {
            crate::commands::scientia::run(cmd).await?;
        }
        Cli::Model { cmd } => {
            crate::commands::model::run(cmd).await?;
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
        #[cfg(feature = "dei")]
        Cli::Attention { cmd } => {
            crate::commands::attention::handle_attention_command(
                cmd,
                &std::env::current_dir().map_err(|e| anyhow::anyhow!("{}", e))?,
            )
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        Cli::Repo { cmd } => {
            let cmd = cmd.unwrap_or(crate::commands::repo::RepoCmd::Status { json: false });
            crate::commands::repo::run(cmd).await?;
        }
        #[cfg(feature = "dei")]
        Cli::Safety { cmd } => {
            crate::commands::safety::handle_safety_command(
                cmd,
                &std::env::current_dir().map_err(|e| anyhow::anyhow!("{}", e))?,
            )
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        Cli::Catalog { cmd } => {
            crate::commands::catalog::run(cmd).await?;
        }
        #[cfg(feature = "ars")]
        Cli::Openclaw { action } => {
            run_openclaw_subcommand(action).await?;
        }
        Cli::Stop { reason } => {
            #[cfg(feature = "dei")]
            crate::commands::dei::stop(reason).await?;
            #[cfg(not(feature = "dei"))]
            {
                let _ = reason;
                eprintln!("Feature 'dei' is not enabled.");
            }
        }

        Cli::Plan { cmd } => {
            crate::commands::plan::dispatch(cmd).await?;
        }
        Cli::Llm { cmd } => {
            crate::commands::llm::run(cmd).await?;
        }
        #[cfg(feature = "dei")]
        Cli::Visus { cmd } => {
            crate::commands::visus::dispatch(cmd)
                .await
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        }
        #[cfg(feature = "dashboard")]
        Cli::Dashboard { args } => {
            crate::commands::dashboard::run(args).await?;
        }
        Cli::Research { cmd } => crate::commands::research::run(cmd).await?,
        #[cfg(feature = "coderabbit")]
        Cli::Review { cmd } => {
            run_review_subcommand(cmd).await?;
        }
        #[cfg(feature = "island")]
        Cli::Island { cmd } => {
            crate::commands::island::run(cmd).await?;
        }
        Cli::Telemetry { cmd } => {
            crate::commands::telemetry::run(cmd).await?;
        }
        Cli::Grammar { args } => {
            crate::commands::grammar::handle(args);
        }
        Cli::Mens { .. } | Cli::Populi { .. } | Cli::Oratio { .. } | Cli::Schola { .. } => {
            std::unreachable!(
                "ML/AI commands are intercepted in main.rs and delegated to external binaries"
            )
        }
    }

    Ok(())
}
