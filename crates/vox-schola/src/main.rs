//! `vox-schola` — Standalone binary for Vox Scientia and Scholarship.
//!
//! Handles scientific publication, finding candidates, and novelty ledger management.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "vox-schola",
    version,
    about = "Vox Scientia and Scholarship CLI"
)]
pub struct VoxScholaRoot {
    #[command(flatten)]
    pub global: vox_cli_core::GlobalOpts,

    #[command(subcommand)]
    pub command: ScholaSubcommand,
}

#[derive(Subcommand)]
pub enum ScholaSubcommand {
    #[command(name = "scientia")]
    Scientia {
        #[command(subcommand)]
        command: vox_cli_core::scientia::ScientiaCmd,
    },
    #[command(name = "schola")]
    Schola {
        #[command(subcommand)]
        action: vox_cli_core::scientia::ScientiaCmd, // Alias for now
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    vox_cli_core::init_tracing_for_cli();
    let root = VoxScholaRoot::parse();
    vox_cli_core::apply_global_opts(&root.global);

    let mut args: Vec<String> = std::env::args().collect();
    if args.is_empty() {
        anyhow::bail!("No arguments provided");
    }

    // Determine the target binary. Prefer 'vox' on path, fallback to cargo run.
    let target_bin = if which::which("vox").is_ok() {
        "vox".to_string()
    } else {
        // Fallback to dev launcher or cargo run
        "cargo".to_string()
    };

    let mut cmd_args = Vec::new();
    if target_bin == "cargo" {
        cmd_args.push("run".to_string());
        cmd_args.push("-p".to_string());
        cmd_args.push("vox-cli".to_string());
        cmd_args.push("--quiet".to_string());
        cmd_args.push("--".to_string());
    }

    // Append everything after the binary name (args[0])
    cmd_args.extend(args.into_iter().skip(1));

    println!("Proxying to {}: {:?}", target_bin, cmd_args);

    let mut child = tokio::process::Command::new(target_bin)
        .args(cmd_args)
        .spawn()
        .context("Failed to spawn vox-cli proxy")?;

    let status = child.wait().await?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}
