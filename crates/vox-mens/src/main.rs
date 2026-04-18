use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "vox-mens", version, about = "Vox ML, AI, and Telemetry CLI")]
pub struct VoxMensRoot {
    #[command(flatten)]
    pub global: vox_cli::GlobalOpts,

    #[command(subcommand)]
    pub command: MensSubcommand,
}

#[derive(Subcommand)]
pub enum MensSubcommand {
    #[cfg(any(feature = "mens-base", feature = "gpu"))]
    #[command(name = "mens")]
    Mens {
        #[command(subcommand)]
        action: vox_mens::commands::mens::PopuliAction,
    },
    #[cfg(feature = "gpu")]
    #[command(name = "schola")]
    Schola {
        #[command(subcommand)]
        cmd: vox_mens::commands::schola::ScholaCmd,
    },
    #[cfg(feature = "oratio")]
    #[command(name = "oratio", visible_alias = "speech")]
    Oratio {
        #[command(subcommand)]
        action: vox_mens::commands::oratio_cmd::OratioAction,
    },
    #[cfg(feature = "populi")]
    #[command(name = "populi")]
    Populi {
        #[command(subcommand)]
        cmd: vox_mens::commands::populi_cli::PopuliCli,
    },
    /// Fine-tune legacy entry (canonical is vox mens train)
    #[cfg(all(feature = "gpu", feature = "mens-dei"))]
    #[command(name = "train")]
    Train {
        #[command(flatten)]
        args: vox_cli::cli_args::TrainLegacyArgs,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    vox_cli::init_tracing_for_cli();
    let root = VoxMensRoot::parse();
    vox_cli::apply_global_opts(&root.global);

    match root.command {
        #[cfg(any(feature = "mens-base", feature = "gpu"))]
        MensSubcommand::Mens { action } => vox_mens::commands::mens::run(action, root.global.json, root.global.verbose).await,
        #[cfg(feature = "gpu")]
        MensSubcommand::Schola { cmd } => vox_mens::commands::schola::run_schola_cmd(cmd).await,
        #[cfg(feature = "oratio")]
        MensSubcommand::Oratio { action } => vox_mens::commands::oratio_cmd::run_oratio_action(action).await,
        #[cfg(feature = "populi")]
        MensSubcommand::Populi { cmd } => vox_mens::commands::populi_cli::run_populi_cli(cmd).await,
        #[cfg(all(feature = "gpu", feature = "mens-dei"))]
        MensSubcommand::Train { args } => vox_mens::commands::ai::train::run_legacy_train(args).await,
    }
}
