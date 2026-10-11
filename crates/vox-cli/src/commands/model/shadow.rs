//! `vox model shadow` — L2.5 shadow evaluation for provisional models: a real graded
//! `vox model bench` run on one model, after which routing confidence follows the Wilson
//! verdict on the recorded cases (`models::bench_evidence`) instead of a hand-set promotion.

use clap::Args;
use owo_colors::OwoColorize;

use super::bench::{BenchArgs, execute};

#[derive(Args, Debug)]
pub struct ShadowArgs {
    /// Model id to shadow-evaluate (catalog id or `provider:model`, as for `vox model bench`).
    pub model_id: String,
    /// Suites to run (default: all).
    #[arg(long, value_delimiter = ',')]
    pub suites: Vec<String>,
    /// Hard spend cap in USD for this evaluation.
    #[arg(long, default_value_t = 0.25)]
    pub max_usd: f64,
}

impl ShadowArgs {
    fn bench_args(&self) -> BenchArgs {
        BenchArgs {
            models: vec![self.model_id.clone()],
            provisional: 0,
            latest: 0,
            suites: self.suites.clone(),
            repeats: 1,
            max_usd: self.max_usd,
            concurrency: 1,
            dry_run: false,
            no_write: false,
            report: None,
            timeout_ms: 120_000,
        }
    }
}

pub async fn run(args: ShadowArgs) -> anyhow::Result<()> {
    println!(
        "{} Shadow-evaluating {} with a graded bench (cap ${:.2})",
        " INFO ".on_blue().white().bold(),
        args.model_id.cyan(),
        args.max_usd
    );
    let report = execute(args.bench_args()).await?;
    let verdict = report["confidence"]
        .as_array()
        .and_then(|a| a.first())
        .map(|c| {
            format!(
                "{} -> {}",
                c["from"].as_str().unwrap_or("?"),
                c["to"].as_str().unwrap_or("?")
            )
        })
        .unwrap_or_else(|| "unchanged".to_string());
    println!("  {} routing confidence: {verdict}", "✓".green());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_runs_a_persisted_single_model_bench_under_its_cap() {
        let args = ShadowArgs {
            model_id: "test/model".into(),
            suites: vec!["reasoning".into()],
            max_usd: 0.1,
        };
        let b = args.bench_args();
        assert_eq!(b.models, vec!["test/model".to_string()]);
        assert_eq!(b.suites, vec!["reasoning".to_string()]);
        assert!(!b.no_write, "shadow evidence must reach routing");
        assert!(!b.dry_run);
        assert_eq!(b.max_usd, 0.1);
    }
}
