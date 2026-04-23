//! Review-related helpers: DeI daemon review is invoked from **`vox mens review`** (`mens-dei`); GitHub CodeRabbit flows are **`vox review coderabbit`** (`coderabbit`).

#[cfg(feature = "dei")]
mod dei;
#[cfg(feature = "dei")]
pub use dei::run;

#[cfg(feature = "coderabbit")]
pub mod coderabbit;

/// Dispatch `vox review …` when built with **`coderabbit`**.
#[cfg(feature = "coderabbit")]
pub async fn run_coderabbit(cli: ReviewCli) -> anyhow::Result<()> {
    match cli {
        ReviewCli::Coderabbit { action } => coderabbit::run(action).await,
    }
}

/// Top-level `vox review …` CLI (CodeRabbit when `coderabbit` feature is enabled).
#[cfg(feature = "coderabbit")]
#[derive(clap::Subcommand, Debug)]
pub enum ReviewCli {
    /// CodeRabbit: semantic PR batches, ingest, tasks (`vox review coderabbit …`).
    Coderabbit {
        #[command(subcommand)]
        action: coderabbit::CodeRabbitAction,
    },
}
