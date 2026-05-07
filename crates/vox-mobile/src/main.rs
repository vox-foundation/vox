//! Entry point for the `vox-mobile` binary.

use anyhow::Result;
use clap::Parser;
use vox_mobile::cli::{Cli, Command};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Doctor => {
            println!("vox-mobile doctor: not yet implemented (Task 3)");
            Ok(())
        }
        Command::Build { platform, release } => {
            println!("vox-mobile build --platform={platform} --release={release}: not yet implemented (Task 5+)");
            Ok(())
        }
    }
}
