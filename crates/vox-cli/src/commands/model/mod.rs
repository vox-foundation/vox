//! Subcommands for managing models, discovery, and orchestration health (`vox model`).

use clap::{Parser, Subcommand};

pub mod discover;
pub mod rollup;
pub mod scoreboard;
pub mod explain;
pub mod costs;
pub mod pricing;

/// Manage models: discovery, scoreboard, and explainability.
#[derive(Parser)]
pub struct ModelArgs {
    #[command(subcommand)]
    pub cmd: ModelCmd,
}

#[derive(Subcommand)]
pub enum ModelCmd {
    /// Refresh the model catalog from all sources (discovery).
    Discover(discover::DiscoverArgs),
    /// Perform batch aggregation of telemetry into the scoreboard.
    Rollup(rollup::RollupArgs),
    /// Show the model scoreboard (latency, success rate, cost).
    Scoreboard(scoreboard::ScoreboardArgs),
    /// Explain model selection for a given task description.
    Explain(explain::ExplainArgs),
    /// Show detailed cost reporting.
    Costs(costs::CostsArgs),
    /// Manage and view observed pricing SSOT.
    Pricing(pricing::PricingArgs),
}

pub async fn run(cmd: ModelCmd) -> anyhow::Result<()> {
    match cmd {
        ModelCmd::Discover(args) => discover::run(args).await,
        ModelCmd::Rollup(args) => rollup::run(args).await,
        ModelCmd::Scoreboard(args) => scoreboard::run(args).await,
        ModelCmd::Explain(args) => explain::run(args).await,
        ModelCmd::Costs(args) => costs::run(args).await,
        ModelCmd::Pricing(args) => pricing::run(args).await,
    }
}
