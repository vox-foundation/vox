//! Entry point for the `vox-mobile` binary.

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use std::env;
use vox_mobile::build;
use vox_mobile::cli::{Cli, Command};
use vox_mobile::doctor;
use vox_mobile::manifest_resolve;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Doctor => {
            let exit_code = doctor::run();
            std::process::exit(exit_code);
        }
        Command::Build { platform, release } => {
            let project_dir = env::current_dir()?;
            let manifest = manifest_resolve::load(&project_dir)?;
            let mobile = manifest
                .mobile
                .as_ref()
                .expect("manifest_resolve::load guarantees [mobile] is present when target=mobile");

            match platform.as_str() {
                "android" => {
                    let android = mobile
                        .android
                        .as_ref()
                        .ok_or_else(|| anyhow!("missing [mobile.android] section"))?;
                    build::android::build(&project_dir, android, release)?;
                }
                "ios" => {
                    let ios = mobile
                        .ios
                        .as_ref()
                        .ok_or_else(|| anyhow!("missing [mobile.ios] section"))?;
                    build::ios::build(&project_dir, ios, release)?;
                }
                "all" => {
                    bail!("--platform=all not yet implemented (Task 7)");
                }
                other => bail!("unknown platform '{other}'; use android, ios, or all"),
            }
            Ok(())
        }
    }
}
