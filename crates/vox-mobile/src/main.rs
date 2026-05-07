//! Entry point for the `vox-mobile` binary.

use anyhow::Result;
use clap::Parser;
use vox_mobile::cli::{Cli, Command};
use vox_mobile::doctor;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Doctor => {
            let exit_code = doctor::run();
            std::process::exit(exit_code);
        }
        Command::Build { platform, release } => {
            println!("vox-mobile build --platform={platform} --release={release}: not yet implemented (Task 5+)");
            Ok(())
        }
    }
}
