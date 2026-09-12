use anyhow::{Context, Result};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use vox_eval::{CollateralDamageConfig, CollateralDamageReport, eval_collateral_damage_suite};

/// Benchmark directories are looked up by name under this root, matching the
/// `--bench mens/data/<name>` convention `vox mens eval-local` uses.
const BENCH_ROOT: &str = "mens/data";

/// Scores every benchmark named in the baseline against the adapter and writes
/// the envelope the `vox mens serve` gate reads.
///
/// `score_bench` is a parameter so tests can drive a degradation without a GPU —
/// and, more importantly, so `post` has exactly one source. A scorer error is
/// propagated: a model that cannot be loaded must never leave a report behind.
pub(crate) fn run_collateral_damage_with(
    pre_score_path: &Path,
    run_dir: &Path,
    score_bench: &mut dyn FnMut(&str) -> Result<f64>,
) -> Result<i32> {
    let pre_file: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(pre_score_path).with_context(|| {
            format!("Failed to read pre-score from {}", pre_score_path.display())
        })?)?;
    let pre: BTreeMap<String, f64> = serde_json::from_value(
        pre_file
            .get("benchmarks")
            .filter(|v| v.is_object())
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "pre-score file {} has no \"benchmarks\" object — refusing to guess which \
                     top-level keys are benchmark names",
                    pre_score_path.display()
                )
            })?,
    )?;

    if pre.is_empty() {
        anyhow::bail!(
            "pre-score file {} has an empty \"benchmarks\" object — nothing to score",
            pre_score_path.display()
        );
    }

    let mut scores: Vec<(String, f64, f64)> = Vec::with_capacity(pre.len());
    for (bench, pre_score) in &pre {
        let post = score_bench(bench)?; // fails loudly; never falls back to `pre`
        scores.push((bench.clone(), *pre_score, post));
    }

    let borrowed: Vec<(&str, f64, f64)> = scores
        .iter()
        .map(|(n, a, b)| (n.as_str(), *a, *b))
        .collect();
    let cfg = CollateralDamageConfig::default();
    let (status, failed_on, reports) = match eval_collateral_damage_suite(&borrowed, &cfg) {
        Ok(rs) => ("pass", None, rs),
        Err(bad) => ("fail", Some(bad.benchmark_name.clone()), vec![bad]),
    };

    // `CollateralDamageReport` is not `Serialize`; project it by hand.
    let reports_json: Vec<_> = reports
        .iter()
        .map(|r: &CollateralDamageReport| {
            json!({
                "benchmark": r.benchmark_name,
                "pre": r.pre_training_score,
                "post": r.post_training_score,
                "degradation": r.degradation,
                "degradation_rate": r.degradation_rate,
                "exceeds_threshold": r.exceeds_threshold,
            })
        })
        .collect();

    std::fs::create_dir_all(run_dir)?;
    std::fs::write(
        run_dir.join("collateral_damage_report.json"),
        serde_json::to_string_pretty(&json!({
            "status": status,
            "failed_on": failed_on,
            "threshold": cfg.max_degradation_rate,
            "reports": reports_json,
        }))?,
    )?;

    if let Some(bench) = &failed_on {
        eprintln!("Collateral damage check FAILED on benchmark '{bench}'.");
    } else {
        println!("Collateral damage check PASSED.");
    }
    Ok(if status == "pass" { 0 } else { 1 })
}

/// CLI entry point. Builds the real, adapter-backed scorer and owns the exit
/// code — the `exit` lives here so the core above stays testable.
pub fn run_collateral_damage(pre_score_path: PathBuf, post_adapter_path: PathBuf) -> Result<()> {
    println!(
        "Evaluating collateral damage against baseline: {}",
        pre_score_path.display()
    );
    println!("Using adapter: {}", post_adapter_path.display());

    let mut scorer = |bench: &str| score_bench(bench, &post_adapter_path);
    let code = run_collateral_damage_with(&pre_score_path, &post_adapter_path, &mut scorer)?;
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

/// Score one benchmark against the adapter by running the existing
/// `vox mens eval-local` harness over `mens/data/<bench>` and reading back its
/// pass@1. Reusing that harness is deliberate: the post-training score must come
/// from the same rubric the operator's baseline was produced with, and it is
/// what performs the `MlBackend` plugin dispatch and loads the adapter — so an
/// adapter that cannot be loaded surfaces here as an `Err`, never as a score.
fn score_bench(bench: &str, adapter: &Path) -> Result<f64> {
    let bench_root = Path::new(BENCH_ROOT).join(bench);
    if !bench_root.join("manifest.json").exists() {
        anyhow::bail!(
            "benchmark '{bench}' named in the baseline has no manifest at {}",
            bench_root.join("manifest.json").display()
        );
    }
    // Keep each bench's eval report next to the adapter so the operator can see
    // what the collateral-damage verdict was computed from.
    let out_dir = adapter.join("collateral_eval").join(bench);
    std::fs::create_dir_all(&out_dir)?;
    let out = out_dir.join("eval_local_report.json");
    super::eval_local::run_eval_local(
        Some(adapter.to_path_buf()),
        None,
        bench_root,
        512,
        0.0,
        1,
        0,
        Some(out.clone()),
    )
    .with_context(|| format!("scoring benchmark '{bench}' against {}", adapter.display()))?;

    let report: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out)?)?;
    report["pass_rate_at_1"].as_f64().ok_or_else(|| {
        anyhow::anyhow!("eval-local report for '{bench}' has no numeric pass_rate_at_1")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_degraded_adapter_writes_a_failing_report() {
        let dir = tempfile::tempdir().unwrap();
        let pre = dir.path().join("pre.json");
        std::fs::write(
            &pre,
            r#"{"benchmarks":{"general_bench":0.85,"code_bench":0.90}}"#,
        )
        .unwrap();

        let mut asked: Vec<String> = Vec::new();
        let mut scorer = |bench: &str| {
            asked.push(bench.to_string());
            // The degraded bench is the one that sorts FIRST, so a scorer loop
            // that short-circuits on the first failure is visible in `asked`.
            Ok(match bench {
                "code_bench" => 0.40,
                _ => 0.84,
            })
        };
        let code = run_collateral_damage_with(&pre, dir.path(), &mut scorer).unwrap();

        assert_eq!(code, 1, "a 55% collapse must not pass");
        let report: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("collateral_damage_report.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(report["status"], "fail");
        assert_eq!(
            report["failed_on"], "code_bench",
            "operator must know what broke"
        );
        // THE assertion that catches "never loaded the adapter". Exact sequence, not `>= 1`:
        // deleting a bench or short-circuiting the loop fails it. (Benchmarks are
        // scored in sorted order — the baseline is read into a BTreeMap.)
        assert_eq!(
            asked,
            ["code_bench", "general_bench"],
            "every bench in the baseline must be re-scored against the adapter"
        );
    }

    #[test]
    fn an_unloadable_adapter_fails_loudly_and_writes_no_passing_report() {
        let dir = tempfile::tempdir().unwrap();
        let pre = dir.path().join("pre.json");
        std::fs::write(&pre, r#"{"benchmarks":{"general_bench":0.85}}"#).unwrap();
        let mut scorer = |_: &str| anyhow::bail!("adapter not found");
        assert!(run_collateral_damage_with(&pre, dir.path(), &mut scorer).is_err());
        assert!(
            !dir.path().join("collateral_damage_report.json").exists(),
            "a load failure must never leave a report the serve gate would accept"
        );
    }

    #[test]
    fn a_pre_score_file_without_the_benchmarks_key_is_a_hard_error() {
        // The exact shape of the bug this format decision prevents: handing the
        // parser an eval-local report (or anything else with top-level f64 keys)
        // must not silently fabricate benchmark names from unrelated fields.
        let dir = tempfile::tempdir().unwrap();
        let pre = dir.path().join("pre.json");
        std::fs::write(
            &pre,
            r#"{"model":"mens/demo","max_tokens":256,"pass_rate_at_1":0.7}"#,
        )
        .unwrap();
        let mut scorer = |_: &str| Ok(1.0);
        let err = run_collateral_damage_with(&pre, dir.path(), &mut scorer)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("benchmarks"),
            "error must name the missing key: {err}"
        );
        assert!(!dir.path().join("collateral_damage_report.json").exists());
    }

    #[test]
    fn an_empty_benchmarks_object_is_a_hard_error() {
        // {"benchmarks": {}} passes the `is_object()` filter but has nothing to
        // score. Without this guard, the loop below never runs, the suite call
        // returns `Ok(vec![])` for an empty slice, and a vacuous "pass" report
        // gets written having loaded the adapter zero times.
        let dir = tempfile::tempdir().unwrap();
        let pre = dir.path().join("pre.json");
        std::fs::write(&pre, r#"{"benchmarks":{}}"#).unwrap();
        let mut scorer = |_: &str| Ok(1.0);
        let err = run_collateral_damage_with(&pre, dir.path(), &mut scorer)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("nothing to score"),
            "error must say there was nothing to score: {err}"
        );
        assert!(!dir.path().join("collateral_damage_report.json").exists());
    }
}
