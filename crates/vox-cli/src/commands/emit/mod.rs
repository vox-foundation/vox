//! Narrow codegen entrypoints (`vox emit …`) without running a full multi-target build.

use anyhow::Result;
use clap::Subcommand;

mod client;

/// Subcommands under `vox emit`.
#[derive(Subcommand)]
pub enum EmitCmd {
    /// Emit Library-shaped TypeScript (`vox-client.ts`, types, schemas, `openapi.json`).
    Client(crate::cli_args::EmitClientArgs),
}

pub async fn run(cmd: EmitCmd) -> Result<()> {
    match cmd {
        EmitCmd::Client(args) => client::run(&args).await,
    }
}
