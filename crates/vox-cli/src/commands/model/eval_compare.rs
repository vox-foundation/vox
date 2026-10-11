//! `vox model eval-compare` — pairwise comparison of eval-corpus reports.
//!
//! Thin wrapper over [`vox_eval::compare::compare`]: McNemar + Holm, paired
//! bootstrap for pass@k, and the resolution floor beside every claim. No verdict
//! here is derived from interval overlap.

use anyhow::{Context, bail};
use clap::Args;
use owo_colors::OwoColorize;
use std::path::PathBuf;
use vox_eval::compare::{ArmInput, CompareOptions, Verdict, compare};
use vox_eval::report::{ArmStats, EvalCorpusReport, REPORT_SCHEMA_VERSION};

#[derive(Args, Debug)]
pub struct EvalCompareArgs {
    /// Eval-corpus report JSON files (at least two, including the baseline).
    #[arg(required = true, num_args = 2..)]
    pub reports: Vec<PathBuf>,
    /// The baseline report; must be one of the REPORT paths, as written.
    #[arg(long)]
    pub baseline: PathBuf,
    /// Family-wise error rate for the Holm step-down.
    #[arg(long, default_value_t = 0.05)]
    pub alpha: f64,
    /// Bootstrap resamples for confidence intervals.
    #[arg(long, default_value_t = 2000)]
    pub bootstrap_reps: usize,
    /// Seed of the bootstrap, recorded so the comparison can be recomputed.
    #[arg(long, default_value_t = 0)]
    pub analysis_seed: u64,
    /// Write the full comparison as pretty JSON.
    #[arg(long)]
    pub output: Option<PathBuf>,
}

fn load(path: &std::path::Path) -> anyhow::Result<EvalCorpusReport> {
    let text = vox_bounded_fs::read_utf8_path_capped(path)?;
    let report: EvalCorpusReport = serde_json::from_str(&text)
        .with_context(|| format!("{} is not an eval-corpus report", path.display()))?;
    if report.schema_version != REPORT_SCHEMA_VERSION {
        bail!(
            "{}: report schema_version {} (expected {REPORT_SCHEMA_VERSION})",
            path.display(),
            report.schema_version
        );
    }
    Ok(report)
}

fn stats(s: &Option<ArmStats>) -> String {
    s.as_ref().map_or("n/a".to_string(), |s| {
        format!("{:.3} [{:.3}, {:.3}]", s.point, s.ci_low, s.ci_high)
    })
}

pub async fn run(args: EvalCompareArgs) -> anyhow::Result<()> {
    let baseline = args.baseline.display().to_string();
    if !args.reports.contains(&args.baseline) {
        bail!("--baseline {baseline} is not one of the supplied reports");
    }
    let arms = args
        .reports
        .iter()
        .map(|p| {
            Ok(ArmInput {
                label: p.display().to_string(),
                report: load(p)?,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let opts = CompareOptions {
        alpha: args.alpha,
        bootstrap_reps: args.bootstrap_reps,
        analysis_seed: args.analysis_seed,
    };
    let c = compare(&arms, &baseline, &opts).map_err(anyhow::Error::msg)?;

    println!(
        "{} problem set {} · k={} · alpha={} · {} bootstrap reps · seed {}",
        " COMPARE ".on_blue().white().bold(),
        c.problem_set_hash,
        c.k,
        c.alpha,
        c.bootstrap_reps,
        c.analysis_seed
    );
    for a in &c.included {
        println!(
            "  {} {} model={} condition={} n={} pass@1 {} pass@{} {}",
            "arm".cyan(),
            a.label,
            a.model_id,
            a.condition.as_deref().unwrap_or("-"),
            a.n_problems,
            stats(&a.pass_at_1),
            c.k,
            stats(&a.pass_at_k)
        );
    }
    for e in &c.excluded {
        println!(
            "  {} {} ({})",
            "excluded".yellow(),
            e.label,
            e.reasons.join("; ")
        );
    }
    for r in &c.claims {
        let verdict = match r.verdict {
            Verdict::Significant => "significant".green().to_string(),
            Verdict::NotSignificant => "not significant".to_string(),
            Verdict::Unresolved => "unresolved (below resolution floor)".yellow().to_string(),
        };
        println!(
            "  {} {} [{}] vs {} [{}] n={} Δfirst-attempt {:+.3} McNemar p={:.4} Holm={} \
             Δpass@{} {:+.3} [{:+.3}, {:+.3}] MDD={:.2} -> {}",
            "claim".magenta(),
            r.arm,
            r.condition_arm.as_deref().unwrap_or("-"),
            r.baseline,
            r.condition_baseline.as_deref().unwrap_or("-"),
            r.n_aligned,
            r.first_attempt_difference,
            r.mcnemar_p,
            if r.holm_rejected { "reject" } else { "keep" },
            c.k,
            r.pass_at_k_difference,
            r.pass_at_k_difference_ci.0,
            r.pass_at_k_difference_ci.1,
            r.min_detectable_difference,
            verdict
        );
    }
    if let Some(out) = &args.output {
        std::fs::write(out, serde_json::to_string_pretty(&c)?)
            .with_context(|| format!("write {}", out.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_eval::corpus_score::{AttemptOutcome, FixtureOutcome};
    use vox_eval::provenance::{RunIdentity, RunProvenance, SamplingRecord};
    use vox_eval::report::{DEFAULT_MAX_INFRA_RATE, ReportInputs};

    fn provenance(run_id: &str) -> RunProvenance {
        let s = |v: &str| Some(v.to_string());
        RunProvenance {
            syntax_fingerprint: s("fp"),
            syntax_fingerprint_kind: s("grammar-v1"),
            cargo_version: s("cargo 1.0"),
            vox_git_sha: s("abc123"),
            vox_binary_sha256: s("bin-sha"),
            verifier_version: s("0.6.0"),
            verifier_matches_harness: Some(true),
            tier: s("smoke"),
            problem_set_hash: s("ps-hash"),
            corpus_manifest_hash: s("cm-hash"),
            prompt_template_hash: s("pt-hash"),
            condition: s("zero-shot"),
            context_hash: s("ctx-hash"),
            harness_version: s("h1"),
            model_id: s("openai/gpt-x"),
            provider: s("openai"),
            request_date: s("2026-10-11"),
            provider_reported_models: vec!["gpt-x-2026-10-01".to_string()],
            mens: None,
            sampling: Some(SamplingRecord {
                temperature: 0.2,
                top_p: 0.95,
                n: 1,
                k: 1,
                seed: 7,
                max_tokens: 1024,
                seed_enforced: false,
            }),
            runner: s("local"),
            os: s("macos"),
            arch: s("aarch64"),
            verifier_timeout_secs: Some(30),
            identity: Some(RunIdentity {
                campaign_id: "camp".to_string(),
                run_id: run_id.to_string(),
                replicate_id: 0,
                attempt_index: 0,
                retry_of_run_id: None,
            }),
            preregistration_id: s("prereg-1"),
        }
    }

    /// 40 single-attempt fixtures, the first `passes` passing.
    fn report(run_id: &str, passes: usize) -> EvalCorpusReport {
        let outcomes = (0..40)
            .map(|i| FixtureOutcome {
                fixture_id: format!("f{i:02}"),
                n: 1,
                c: usize::from(i < passes),
                attempts: vec![AttemptOutcome {
                    compiled: true,
                    tests_passed: i < passes,
                    cheated: false,
                    total_tokens: 10,
                    latency_ms: 5,
                    cost_usd: None,
                }],
            })
            .collect();
        EvalCorpusReport::assemble(ReportInputs {
            model_id: format!("model-{run_id}"),
            harness_id: "h".into(),
            config_digest: "d".into(),
            cutoff: None,
            held_out_only: true,
            measured: true,
            k: 1,
            outcomes,
            samples: vec![],
            n_infra_errors: 0,
            n_samples_planned: 40,
            max_infra_rate: DEFAULT_MAX_INFRA_RATE,
            spend_cap_hit: false,
            provenance: provenance(run_id),
            bootstrap_reps: 100,
            analysis_seed: 1,
        })
    }

    #[tokio::test]
    async fn run_writes_one_claim() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b, out) = (
            dir.path().join("a.json"),
            dir.path().join("b.json"),
            dir.path().join("out.json"),
        );
        for (p, r) in [(&a, report("run-a", 10)), (&b, report("run-b", 30))] {
            std::fs::write(p, serde_json::to_string(&r).unwrap()).unwrap();
        }
        run(EvalCompareArgs {
            reports: vec![a.clone(), b],
            baseline: a,
            alpha: 0.05,
            bootstrap_reps: 100,
            analysis_seed: 0,
            output: Some(out.clone()),
        })
        .await
        .unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(out).unwrap()).unwrap();
        let claims = v["claims"].as_array().unwrap();
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0]["verdict"], "significant");
        assert_eq!(v["alpha"], 0.05);
    }

    #[tokio::test]
    async fn baseline_must_be_a_supplied_report() {
        let err = run(EvalCompareArgs {
            reports: vec!["a.json".into(), "b.json".into()],
            baseline: "c.json".into(),
            alpha: 0.05,
            bootstrap_reps: 10,
            analysis_seed: 0,
            output: None,
        })
        .await
        .unwrap_err();
        assert!(err.to_string().contains("not one of the supplied reports"));
    }
}
