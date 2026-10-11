//! `vox model classify` — L2 of the model-autonomic system.
//!
//! Build the classifier prompt for a given model id and print it (default) or
//! invoke the classifier LLM (when `--invoke` is set and a classifier key is
//! configured). The classifier itself is read from
//! `contracts/orchestration/model-pins.v1.yaml` (`classifier.primary`).
//!
//! See `docs/src/architecture/model-autonomic-system-2026.md` §3.

use anyhow::{Context, Result};
use clap::Args;
use vox_orchestrator::models::ModelRegistry;
use vox_orchestrator::models::autonomic::{build_classifier_prompt, record_classification};

#[derive(Args, Debug)]
pub struct ClassifyArgs {
    /// Model id to classify (e.g. `anthropic/claude-future-1`).
    pub model_id: String,

    /// Print the classifier prompt that *would* be sent, without calling
    /// the classifier LLM. Default mode — safe to run offline.
    #[arg(long, default_value_t = true)]
    pub dry_run: bool,

    /// Override the classifier model id (defaults to
    /// `model-pins.v1.yaml::classifier.primary`).
    #[arg(long)]
    pub classifier: Option<String>,

    /// Classify from measurement instead: run a graded `vox model bench` on the model and
    /// derive tier and strengths from its suite scores.
    #[arg(long)]
    pub measure: bool,

    /// Spend cap for `--measure`.
    #[arg(long, default_value_t = 0.25)]
    pub max_usd: f64,
}

async fn run_measured(args: &ClassifyArgs) -> Result<()> {
    let report = super::bench::execute(super::bench::BenchArgs {
        models: vec![args.model_id.clone()],
        provisional: 0,
        latest: 0,
        suites: Vec::new(),
        repeats: 1,
        max_usd: args.max_usd,
        concurrency: 1,
        dry_run: false,
        no_write: false,
        report: None,
        timeout_ms: 120_000,
    })
    .await?;
    let summary = first_summary(&report)?;
    let registry = ModelRegistry::from_cache();
    let catalog_tier = registry
        .get(&args.model_id)
        .map_or(vox_orchestrator::models::ModelTier::Unknown, |s| {
            s.capabilities.tier
        });
    let judgement = super::bench::classify_from_summary(&summary, catalog_tier);
    println!(
        "tier: {:?}  strengths: {:?}  confidence: {:.2}",
        judgement.tier, judgement.strengths, judgement.confidence
    );
    record_classification(&args.model_id, "vox-model-bench", &judgement);
    Ok(())
}

fn first_summary(report: &serde_json::Value) -> Result<super::bench::ModelSummary> {
    report["summaries"]
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .map(serde_json::from_value)
        .transpose()?
        .context("bench produced no summary for this model")
}

pub async fn run(args: ClassifyArgs) -> Result<()> {
    if args.measure {
        return run_measured(&args).await;
    }
    let registry = ModelRegistry::from_cache();
    let pins = vox_config::load_model_pins_config().unwrap_or_default();
    let classifier_id = args
        .classifier
        .clone()
        .or(pins.classifier.primary)
        .context(
            "no classifier model configured (set --classifier or pins.yaml::classifier.primary)",
        )?;

    // Pull what we know about the target model from the local catalog, if present.
    let (description, supported_params, sample_cost): (Option<String>, Vec<String>, Option<f64>) =
        match registry.get(&args.model_id) {
            Some(spec) => {
                let cost = if spec.cost_per_1k_input > 0.0 {
                    Some(spec.cost_per_1k_input)
                } else if spec.cost_per_1k > 0.0 {
                    Some(spec.cost_per_1k)
                } else {
                    None
                };
                // ModelSpec doesn't currently surface `description` or
                // `supported_parameters`; the autonomic prompt is intentionally
                // tolerant of either being empty.
                (None, Vec::new(), cost)
            }
            None => (None, Vec::new(), None),
        };

    let prompt = build_classifier_prompt(
        &args.model_id,
        description.as_deref(),
        &supported_params,
        sample_cost,
    );

    if args.dry_run {
        println!("# vox model classify — dry-run\n");
        println!("Classifier (primary pin): {classifier_id}");
        println!("Target model:             {}", args.model_id);
        println!("\n----- PROMPT BEGIN -----\n{prompt}\n----- PROMPT END -----");
        println!(
            "\nNote: run with `--no-dry-run` to actually invoke the classifier. Live invocation is gated until the council ratifies promotion thresholds (see model-autonomic-system-2026.md §6 Phase F)."
        );
        return Ok(());
    }

    // Live-invocation path is scaffolded but disabled by default until the
    // council ratifies budget + thresholds. Emit a placeholder telemetry
    // record so downstream consumers see the call signal even in scaffold mode.
    eprintln!(
        "Live classifier invocation not yet wired (council gate). Emitting a placeholder ClassificationEvent so downstream consumers can verify the telemetry pipeline."
    );
    let placeholder = vox_orchestrator::models::autonomic::ClassificationJudgement {
        tier: vox_orchestrator::models::ModelTier::Unknown,
        strengths: Vec::new(),
        confidence: 0.0,
        rationale: Some("scaffold placeholder — live invocation not yet enabled".into()),
    };
    record_classification(&args.model_id, &classifier_id, &placeholder);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::bench::ModelSummary;
    use super::first_summary;
    use std::collections::BTreeMap;

    #[test]
    fn measured_classify_reads_the_first_summary_of_a_bench_report() {
        let s = ModelSummary {
            model_id: "acme/x-3".into(),
            served_model: None,
            cases: 22,
            passed: 20,
            pass_rate: 20.0 / 22.0,
            pass_ci95: Some((0.72, 0.97)),
            mean_score: 0.93,
            by_suite: BTreeMap::from([("reasoning".to_string(), 0.9)]),
            error_rate: 0.0,
            p50_ttft_ms: Some(300),
            p50_latency_ms: Some(900),
            p95_latency_ms: Some(2000),
            mean_tokens_per_sec: Some(60.0),
            total_cost_usd: 0.02,
            cost_per_solve_usd: Some(0.001),
            tokens_per_solve: Some(250.0),
            score_stddev: None,
            effective_context: None,
        };
        let report = serde_json::json!({ "summaries": [s] });
        let got = first_summary(&report).expect("summary");
        assert_eq!(
            (got.model_id.as_str(), got.cases, got.passed),
            ("acme/x-3", 22, 20)
        );
        assert_eq!(got.by_suite.get("reasoning"), Some(&0.9));

        let empty = first_summary(&serde_json::json!({ "summaries": [] })).unwrap_err();
        assert!(empty.to_string().contains("no summary"), "{empty}");
    }
}
