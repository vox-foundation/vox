use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "vox-mens", version, about = "Vox ML, AI, and Telemetry CLI")]
pub struct VoxMensRoot {
    #[command(flatten)]
    pub global: vox_cli_core::GlobalOpts,

    #[command(subcommand)]
    pub command: MensSubcommand,
}

#[derive(Subcommand)]
pub enum MensSubcommand {
    #[cfg(any(feature = "mens-base", feature = "gpu"))]
    #[command(name = "mens")]
    Mens {
        #[command(subcommand)]
        action: Box<vox_mens::commands::mens::PopuliAction>,
    },
    #[cfg(feature = "oratio")]
    #[command(name = "oratio", visible_alias = "speech")]
    Oratio {
        #[command(subcommand)]
        action: Box<vox_mens::commands::oratio_cmd::OratioAction>,
    },
    #[cfg(feature = "populi")]
    #[command(name = "populi")]
    Populi {
        #[command(subcommand)]
        cmd: Box<vox_mens::commands::populi_cli::PopuliCli>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    vox_cli_core::init_tracing_for_cli();
    let root = VoxMensRoot::parse();
    vox_cli_core::apply_global_opts(&root.global);

    match root.command {
        #[cfg(any(feature = "mens-base", feature = "gpu"))]
        MensSubcommand::Mens { action } => {
            vox_mens::commands::mens::run(*action, root.global.json, root.global.verbose > 0).await
        }
        #[cfg(feature = "oratio")]
        MensSubcommand::Oratio { action } => {
            vox_mens::commands::oratio_cmd::run(*action, root.global.json).await
        }
        #[cfg(feature = "populi")]
        MensSubcommand::Populi { cmd } => {
            vox_mens::commands::populi_cli::run(*cmd, root.global.json).await
        }
    }
}
