//! Subcommands for managing models, discovery, and orchestration health (`vox model`).

use clap::{Parser, Subcommand};

pub mod bench;
pub mod cas;
pub mod classify;
pub mod costs;
pub mod council_report;
pub mod discover;
pub mod eval;
pub mod eval_bundle;
pub mod eval_compare;
pub mod eval_corpus;
pub mod eval_diagram;
pub mod explain;
pub mod list;
pub mod preferences;
pub mod pricing;
pub mod rollup;
pub mod scoreboard;
pub mod shadow;
pub mod show;

/// Manage models: discovery, scoreboard, and explainability.
#[derive(Parser)]
pub struct ModelArgs {
    #[command(subcommand)]
    pub cmd: ModelCmd,
}

#[derive(Subcommand)]
pub enum ModelCmd {
    /// Mesh CAS model bundles (SafeTensors; Mn-T8). Not implemented — hidden until Mn-T8.
    #[command(hide = true)]
    Cas {
        #[command(subcommand)]
        cmd: cas::CasCmd,
    },
    /// Refresh the model catalog from all sources (discovery).
    Discover(discover::DiscoverArgs),
    /// Perform batch aggregation of telemetry into the scoreboard.
    Rollup(rollup::RollupArgs),
    /// Show the model scoreboard (latency, success rate, cost).
    Scoreboard(scoreboard::ScoreboardArgs),
    /// List model ids from the local catalog cache.
    List(list::ListArgs),
    /// Show JSON for one model from the cache.
    Show(show::ShowArgs),
    /// Set or reset Clavis capability routing preferences.
    Preferences(preferences::PreferencesArgs),
    /// Explain model selection for a given task description.
    Explain(explain::ExplainArgs),
    /// Graded live benchmark (intelligence, responsiveness, efficiency, reliability) under a hard
    /// `--max-usd` cap; results feed routing confidence and measured quality.
    Bench(Box<bench::BenchArgs>),
    /// Run a small built-in benchmark to measure intelligence/tokens/latency and write back.
    Eval(eval::EvalArgs),
    /// Score a model or external harness against the held-out HumanEval-Vox corpus.
    EvalCorpus(Box<eval_corpus::EvalCorpusArgs>),
    /// Compare eval-corpus reports: McNemar + Holm, paired bootstrap, resolution floor.
    EvalCompare(Box<eval_compare::EvalCompareArgs>),
    /// Show detailed cost reporting.
    Costs(costs::CostsArgs),
    /// Manage and view observed pricing SSOT.
    Pricing(pricing::PricingArgs),
    /// L2 — Build/inspect the classifier prompt for a model id (autonomic system).
    Classify(classify::ClassifyArgs),
    /// L2.5 — Shadow-eval a provisional model with a graded bench; confidence follows the verdict.
    Shadow(shadow::ShadowArgs),
    /// L3 — Render the quarterly model-council report (autonomic system).
    CouncilReport(council_report::CouncilReportArgs),
}

pub async fn run(cmd: ModelCmd) -> anyhow::Result<()> {
    match cmd {
        ModelCmd::Cas { cmd } => cas::run(cmd).await,
        ModelCmd::Discover(args) => discover::run(args).await,
        ModelCmd::Rollup(args) => rollup::run(args).await,
        ModelCmd::Scoreboard(args) => scoreboard::run(args).await,
        ModelCmd::List(args) => list::run(args).await,
        ModelCmd::Show(args) => show::run(args).await,
        ModelCmd::Preferences(args) => preferences::run(args).await,
        ModelCmd::Explain(args) => explain::run(args).await,
        ModelCmd::Bench(args) => bench::run(*args).await,
        ModelCmd::Eval(args) => eval::run(args).await,
        ModelCmd::EvalCorpus(args) => eval_corpus::run(*args).await,
        ModelCmd::EvalCompare(args) => eval_compare::run(*args).await,
        ModelCmd::Costs(args) => costs::run(args).await,
        ModelCmd::Pricing(args) => pricing::run(args).await,
        ModelCmd::Classify(args) => classify::run(args).await,
        ModelCmd::Shadow(args) => shadow::run(args).await,
        ModelCmd::CouncilReport(args) => council_report::run(args).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_args_route_a_subcommand_and_its_flags() {
        let args = ModelArgs::try_parse_from(["model", "list", "--limit", "3"]).unwrap();
        assert!(matches!(args.cmd, ModelCmd::List(ref l) if l.limit == 3));
        assert!(ModelArgs::try_parse_from(["model", "no-such-subcommand"]).is_err());
    }
}
