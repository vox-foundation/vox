use anyhow::{Context, Result};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use vox_eval::reasoning_bench::{SLICE_NAMES, SLICE_SIZE};
use vox_eval::reasoning_slice::{
    THINK_INIT_SUFFIX, eval_reasoning_slice, reasoning_slice_of, reasoning_slices_json,
    require_reasoning_keys,
};
use vox_eval::{CollateralDamageConfig, CollateralDamageReport, eval_collateral_damage_suite};

/// Benchmark directories are looked up by name under this root, matching the
/// `--bench mens/data/<name>` convention `vox mens eval-local` uses.
const BENCH_ROOT: &str = "mens/data";

/// One scored benchmark. `n` is the item count for reasoning slices, `None` for
/// eval-local benches.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BenchScore {
    pub(crate) value: f64,
    pub(crate) n: Option<usize>,
}

/// Hand projection of one report (`CollateralDamageReport` is not `Serialize`).
/// Reasoning entries pass `Some(n)` and gain `slice` and `n`; generic entries gain neither.
fn report_json(r: &CollateralDamageReport, n: Option<usize>) -> serde_json::Value {
    let mut v = json!({
        "benchmark": r.benchmark_name,
        "pre": r.pre_training_score,
        "post": r.post_training_score,
        "degradation": r.degradation,
        "degradation_rate": r.degradation_rate,
        "exceeds_threshold": r.exceeds_threshold,
    });
    if let Some(n) = n {
        v["slice"] = json!(true);
        v["n"] = json!(n);
    }
    v
}

/// Scores every benchmark named in the baseline against the adapter and writes
/// the envelope the `vox mens serve` gate reads.
///
/// `score_bench` is a parameter so tests can drive a degradation without a GPU —
/// and, more importantly, so `post` has exactly one source. A scorer error is
/// propagated: a model that cannot be loaded must never leave a report behind.
pub(crate) fn run_collateral_damage_with(
    pre_score_path: &Path,
    run_dir: &Path,
    score_bench: &mut dyn FnMut(&str) -> Result<BenchScore>,
) -> Result<i32> {
    let pre = load_pre(pre_score_path, run_dir)?;

    let mut post: BTreeMap<&str, BenchScore> = BTreeMap::new();
    for bench in pre.keys() {
        // fails loudly; never falls back to `pre`
        post.insert(bench.as_str(), score_bench(bench)?);
    }

    let cfg = CollateralDamageConfig::default();
    let generic: Vec<(&str, f64, f64)> = pre
        .iter()
        .filter(|(bench, _)| reasoning_slice_of(bench).is_none())
        .map(|(bench, p)| (bench.as_str(), *p, post[bench.as_str()].value))
        .collect();
    let generic_result = eval_collateral_damage_suite(&generic, &cfg);

    let mut verdicts = Vec::with_capacity(SLICE_NAMES.len());
    for name in SLICE_NAMES {
        let init = format!("{name}{THINK_INIT_SUFFIX}");
        verdicts.push(slice_verdict(
            name,
            &pre,
            post[name],
            post[init.as_str()],
            &cfg,
        )?);
    }

    let failing_slice = verdicts.iter().find(|v| v.exceeds_threshold());
    let (status, failed_on, reports_json) = match (&generic_result, failing_slice) {
        (Err(bad), _) => (
            "fail",
            Some(bad.benchmark_name.clone()),
            vec![report_json(bad, None)],
        ),
        (Ok(_), Some(v)) => ("fail", v.failed_key(), vec![failing_report_json(v)]),
        (Ok(rs), None) => {
            let mut out: Vec<_> = rs.iter().map(|r| report_json(r, None)).collect();
            out.extend(verdicts.iter().flat_map(slice_reports_json));
            ("pass", None, out)
        }
    };

    write_report(
        run_dir,
        json!({
            "status": status,
            "failed_on": failed_on,
            "threshold": cfg.max_degradation_rate,
            "reports": reports_json,
            "reasoning_slices": reasoning_slices_json(&verdicts),
        }),
        &failed_on,
    )
}

/// Fail-fast variant: scores the reasoning slices one at a time in `order` and
/// stops at the first slice over the threshold. Slices after it are recorded as
/// `not_run` with reason `fail_fast_after_<slice>` and no score. A partial
/// report is always `fail`; when every slice passes it is a full report.
pub(crate) fn run_collateral_damage_fail_fast_with(
    pre_score_path: &Path,
    run_dir: &Path,
    order: &[String],
    score_bench: &mut dyn FnMut(&str) -> Result<BenchScore>,
) -> Result<i32> {
    let pre = load_pre(pre_score_path, run_dir)?;
    let mut want: Vec<&str> = order.iter().map(String::as_str).collect();
    want.sort_unstable();
    let mut all = SLICE_NAMES.to_vec();
    all.sort_unstable();
    if want != all {
        anyhow::bail!(
            "--fail-fast must name each reasoning slice exactly once ({SLICE_NAMES:?}), got {order:?}"
        );
    }
    if let Some(other) = pre.keys().find(|b| reasoning_slice_of(b).is_none()) {
        anyhow::bail!(
            "--fail-fast scores reasoning slices only; the baseline also names '{other}'"
        );
    }

    let cfg = CollateralDamageConfig::default();
    let mut verdicts = Vec::with_capacity(order.len());
    let mut not_run = Vec::new();
    for (i, name) in order.iter().enumerate() {
        let init = format!("{name}{THINK_INIT_SUFFIX}");
        let (acc, think) = (score_bench(name)?, score_bench(&init)?);
        let v = slice_verdict(name, &pre, acc, think, &cfg)?;
        let failed = v.exceeds_threshold();
        verdicts.push(v);
        if failed {
            not_run = order[i + 1..]
                .iter()
                .map(|rest| {
                    json!({
                        "benchmark": rest,
                        "status": "not_run",
                        "reason": format!("fail_fast_after_{name}"),
                    })
                })
                .collect();
            break;
        }
    }

    let (status, failed_on, reports_json) = match verdicts.iter().find(|v| v.exceeds_threshold()) {
        Some(v) => ("fail", v.failed_key(), vec![failing_report_json(v)]),
        None => (
            "pass",
            None,
            verdicts.iter().flat_map(slice_reports_json).collect(),
        ),
    };
    write_report(
        run_dir,
        json!({
            "status": status,
            "failed_on": failed_on,
            "threshold": cfg.max_degradation_rate,
            "reports": reports_json,
            "reasoning_slices": reasoning_slices_json(&verdicts),
            "fail_fast_order": order,
            "not_run": not_run,
        }),
        &failed_on,
    )
}

/// Removes any stale report, then reads and validates the baseline.
fn load_pre(pre_score_path: &Path, run_dir: &Path) -> Result<BTreeMap<String, f64>> {
    // A report from an earlier run must not survive a run that fails: serve
    // would read the old pass. Removed before anything else can fail.
    let report_path = run_dir.join("collateral_damage_report.json");
    match std::fs::remove_file(&report_path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(e).with_context(|| {
                format!("could not remove stale report {}", report_path.display())
            });
        }
    }
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
    require_reasoning_keys(pre.keys().map(String::as_str)).map_err(|e| anyhow::anyhow!(e))?;
    Ok(pre)
}

fn slice_verdict(
    name: &str,
    pre: &BTreeMap<String, f64>,
    acc: BenchScore,
    think: BenchScore,
    cfg: &CollateralDamageConfig,
) -> Result<vox_eval::reasoning_slice::ReasoningSliceVerdict> {
    let init = format!("{name}{THINK_INIT_SUFFIX}");
    let n = match (acc.n, think.n) {
        (Some(a), Some(t)) if a == t => a,
        (Some(a), Some(t)) => {
            anyhow::bail!("{name}: item counts differ ({name}: {a}, {init}: {t})")
        }
        _ => anyhow::bail!("{name}: scorer returned no item count for {name} or {init}"),
    };
    eval_reasoning_slice(
        name,
        (pre[name], acc.value),
        (pre[init.as_str()], think.value),
        n,
        cfg,
    )
    .map_err(|e| anyhow::anyhow!(e))
}

fn failing_report_json(v: &vox_eval::reasoning_slice::ReasoningSliceVerdict) -> serde_json::Value {
    let r = if v.accuracy.exceeds_threshold {
        &v.accuracy
    } else {
        &v.think_initiation
    };
    report_json(r, Some(v.n))
}

fn slice_reports_json(
    v: &vox_eval::reasoning_slice::ReasoningSliceVerdict,
) -> [serde_json::Value; 2] {
    [
        report_json(&v.accuracy, Some(v.n)),
        report_json(&v.think_initiation, Some(v.n)),
    ]
}

fn write_report(
    run_dir: &Path,
    report: serde_json::Value,
    failed_on: &Option<String>,
) -> Result<i32> {
    std::fs::create_dir_all(run_dir)?;
    std::fs::write(
        run_dir.join("collateral_damage_report.json"),
        serde_json::to_string_pretty(&report)?,
    )?;
    if let Some(bench) = failed_on {
        eprintln!("Collateral damage check FAILED on benchmark '{bench}'.");
        Ok(1)
    } else {
        println!("Collateral damage check PASSED.");
        Ok(0)
    }
}

/// CLI entry point. Builds the real, adapter-backed scorer and owns the exit
/// code — the `exit` lives here so the core above stays testable.
pub fn run_collateral_damage(
    pre_score_path: PathBuf,
    post_adapter_path: PathBuf,
    fail_fast: Option<Vec<String>>,
) -> Result<()> {
    println!(
        "Evaluating collateral damage against baseline: {}",
        pre_score_path.display()
    );
    println!("Using adapter: {}", post_adapter_path.display());

    let mut scorer = |bench: &str| score_bench(bench, &post_adapter_path);
    let code = match &fail_fast {
        Some(order) => run_collateral_damage_fail_fast_with(
            &pre_score_path,
            &post_adapter_path,
            order,
            &mut scorer,
        )?,
        None => run_collateral_damage_with(&pre_score_path, &post_adapter_path, &mut scorer)?,
    };
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

fn cached_reasoning_report(
    bench: &str,
    items: &Path,
    adapter: &Path,
) -> Result<super::reasoning_eval::ReasoningReport> {
    use std::sync::Mutex;
    static CACHE: Mutex<Vec<(String, super::reasoning_eval::ReasoningReport)>> =
        Mutex::new(Vec::new());
    let key = format!("{}::{bench}", adapter.display());
    {
        let guard = CACHE.lock().unwrap_or_else(|err| err.into_inner());
        if let Some((_, report)) = guard.iter().find(|(k, _)| k == &key) {
            return Ok(*report);
        }
    }
    let report = super::reasoning_eval::score_reasoning_file(bench, items, adapter)?;
    CACHE
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .push((key, report));
    Ok(report)
}

/// Score one benchmark against the adapter by running the existing
/// `vox mens eval-local` harness over `mens/data/<bench>` and reading back its
/// pass@1. Reusing that harness is deliberate: the post-training score must come
/// from the same rubric the operator's baseline was produced with, and it is
/// what performs the `MlBackend` plugin dispatch and loads the adapter — so an
/// adapter that cannot be loaded surfaces here as an `Err`, never as a score.
fn score_bench(bench: &str, adapter: &Path) -> Result<BenchScore> {
    let (dir_name, want_think_rate) = match bench.strip_suffix("_init") {
        Some(base) => (base, true),
        None => (bench, false),
    };
    let reasoning = Path::new(BENCH_ROOT).join(dir_name).join("reasoning.jsonl");
    if reasoning.is_file() {
        let report = cached_reasoning_report(dir_name, &reasoning, adapter)?;
        if report.n != SLICE_SIZE {
            anyhow::bail!(
                "reasoning bench '{dir_name}': scored {} items, expected {SLICE_SIZE}",
                report.n
            );
        }
        let value = if want_think_rate {
            report.think_initiation_rate
        } else {
            report.accuracy
        };
        return Ok(BenchScore {
            value,
            n: Some(report.n),
        });
    }
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
        1,
        0,
        Some(out.clone()),
    )
    .with_context(|| format!("scoring benchmark '{bench}' against {}", adapter.display()))?;

    let report: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out)?)?;
    let value = report["pass_rate_at_1"].as_f64().ok_or_else(|| {
        anyhow::anyhow!("eval-local report for '{bench}' has no numeric pass_rate_at_1")
    })?;
    Ok(BenchScore { value, n: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The six reasoning keys at 20/32 (0.625) accuracy and think-initiation.
    const SIX: &str = r#""gsm8k_think":0.625,"gsm8k_think_init":0.625,"math500_think":0.625,"math500_think_init":0.625,"vox_think":0.625,"vox_think_init":0.625"#;

    /// A scorer result for `bench`: reasoning keys at 0.625 over 32 items, others `other`.
    fn flat(bench: &str, other: f64) -> BenchScore {
        if reasoning_slice_of(bench).is_some() {
            BenchScore {
                value: 0.625,
                n: Some(32),
            }
        } else {
            BenchScore {
                value: other,
                n: None,
            }
        }
    }

    fn read_report(dir: &Path) -> serde_json::Value {
        serde_json::from_str(
            &std::fs::read_to_string(dir.join("collateral_damage_report.json")).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn a_degraded_adapter_writes_a_failing_report() {
        let dir = tempfile::tempdir().unwrap();
        let pre = dir.path().join("pre.json");
        std::fs::write(
            &pre,
            format!(r#"{{"benchmarks":{{"general_bench":0.85,"code_bench":0.90,{SIX}}}}}"#),
        )
        .unwrap();

        let mut asked: Vec<String> = Vec::new();
        let mut scorer = |bench: &str| {
            asked.push(bench.to_string());
            // The degraded bench is the one that sorts FIRST, so a scorer loop
            // that short-circuits on the first failure is visible in `asked`.
            Ok(match bench {
                "code_bench" => BenchScore {
                    value: 0.40,
                    n: None,
                },
                _ => flat(bench, 0.84),
            })
        };
        let code = run_collateral_damage_with(&pre, dir.path(), &mut scorer).unwrap();

        assert_eq!(code, 1, "a 55% collapse must not pass");
        let report = read_report(dir.path());
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
            [
                "code_bench",
                "general_bench",
                "gsm8k_think",
                "gsm8k_think_init",
                "math500_think",
                "math500_think_init",
                "vox_think",
                "vox_think_init"
            ],
            "every bench in the baseline must be re-scored against the adapter"
        );
    }

    #[test]
    fn an_unloadable_adapter_fails_loudly_and_writes_no_passing_report() {
        let dir = tempfile::tempdir().unwrap();
        let pre = dir.path().join("pre.json");
        std::fs::write(
            &pre,
            format!(r#"{{"benchmarks":{{"general_bench":0.85,{SIX}}}}}"#),
        )
        .unwrap();
        let mut scorer = |_: &str| anyhow::bail!("adapter not found");
        assert!(run_collateral_damage_with(&pre, dir.path(), &mut scorer).is_err());
        assert!(
            !dir.path().join("collateral_damage_report.json").exists(),
            "a load failure must never leave a report the serve gate would accept"
        );
    }

    #[test]
    fn a_six_key_baseline_writes_the_slice_label_and_item_counts() {
        let dir = tempfile::tempdir().unwrap();
        let pre = dir.path().join("pre.json");
        std::fs::write(
            &pre,
            format!(r#"{{"benchmarks":{{"general_bench":0.85,{SIX}}}}}"#),
        )
        .unwrap();
        let mut scorer = |bench: &str| Ok(flat(bench, 0.84));
        let code = run_collateral_damage_with(&pre, dir.path(), &mut scorer).unwrap();
        assert_eq!(code, 0);
        let report = read_report(dir.path());
        assert_eq!(report["status"], "pass");
        let note = report["reasoning_slices"]["scope_note"].as_str().unwrap();
        assert!(
            note.contains("32-item") && note.contains("not full"),
            "{note}"
        );
        assert_eq!(report["reasoning_slices"]["slice_size"], 32);
        for name in SLICE_NAMES {
            assert_eq!(
                report["reasoning_slices"]["benches"][name]["n"], 32,
                "{name}"
            );
        }
        let reports = report["reports"].as_array().unwrap();
        assert_eq!(reports.len(), 7, "one generic plus two per slice");
        for r in reports {
            if reasoning_slice_of(r["benchmark"].as_str().unwrap()).is_some() {
                assert_eq!(r["slice"], true);
                assert_eq!(r["n"], 32);
            } else {
                assert!(r.get("slice").is_none() && r.get("n").is_none());
            }
        }
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
        let mut scorer = |_: &str| {
            Ok(BenchScore {
                value: 1.0,
                n: None,
            })
        };
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
        let mut scorer = |_: &str| {
            Ok(BenchScore {
                value: 1.0,
                n: None,
            })
        };
        let err = run_collateral_damage_with(&pre, dir.path(), &mut scorer)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("nothing to score"),
            "error must say there was nothing to score: {err}"
        );
        assert!(!dir.path().join("collateral_damage_report.json").exists());
    }

    const SIX_KEYS: [&str; 6] = [
        "gsm8k_think",
        "gsm8k_think_init",
        "math500_think",
        "math500_think_init",
        "vox_think",
        "vox_think_init",
    ];

    /// Write a baseline of the six reasoning keys at 0.625, minus `drop`, with
    /// `set` overriding or adding entries (for example `general_bench`).
    fn write_baseline(dir: &Path, drop: &[&str], set: &[(&str, f64)]) -> PathBuf {
        let mut m: BTreeMap<String, f64> = SIX_KEYS
            .iter()
            .filter(|k| !drop.contains(k))
            .map(|k| (k.to_string(), 0.625))
            .collect();
        for (k, v) in set {
            m.insert(k.to_string(), *v);
        }
        let path = dir.join("pre.json");
        std::fs::write(&path, json!({ "benchmarks": m }).to_string()).unwrap();
        path
    }

    /// A scorer that returns `flat` for every key except the `over` entries.
    fn scorer_over<'a>(
        over: &'a [(&'a str, BenchScore)],
        calls: &'a mut usize,
    ) -> impl FnMut(&str) -> Result<BenchScore> + 'a {
        move |bench: &str| {
            *calls += 1;
            Ok(over
                .iter()
                .find(|(k, _)| *k == bench)
                .map(|(_, s)| *s)
                .unwrap_or_else(|| flat(bench, 0.84)))
        }
    }

    fn score(value: f64, n: Option<usize>) -> BenchScore {
        BenchScore { value, n }
    }

    fn no_report(dir: &Path) {
        assert!(
            !dir.join("collateral_damage_report.json").exists(),
            "a refused run must leave no report"
        );
    }

    #[test]
    fn a_baseline_missing_a_think_initiation_key_is_a_hard_error() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &["gsm8k_think_init"], &[]);
        let mut calls = 0;
        let err = {
            let mut scorer = scorer_over(&[], &mut calls);
            run_collateral_damage_with(&pre, dir.path(), &mut scorer)
                .unwrap_err()
                .to_string()
        };
        assert!(err.contains("gsm8k_think_init"), "{err}");
        assert_eq!(
            calls, 0,
            "the scorer must never run on an incomplete baseline"
        );
        no_report(dir.path());
    }

    #[test]
    fn a_baseline_missing_a_whole_slice_is_a_hard_error() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &["vox_think", "vox_think_init"], &[]);
        let mut calls = 0;
        let mut scorer = scorer_over(&[], &mut calls);
        let err = run_collateral_damage_with(&pre, dir.path(), &mut scorer)
            .unwrap_err()
            .to_string();
        assert!(err.contains("vox_think, vox_think_init"), "{err}");
        no_report(dir.path());
    }

    #[test]
    fn a_zero_reasoning_baseline_refuses_and_writes_no_report() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[("math500_think", 0.0)]);
        let mut calls = 0;
        let mut scorer = scorer_over(&[], &mut calls);
        let err = run_collateral_damage_with(&pre, dir.path(), &mut scorer)
            .unwrap_err()
            .to_string();
        assert!(err.contains("no relative bar"), "{err}");
        no_report(dir.path());
    }

    #[test]
    fn a_non_finite_reasoning_score_refuses_and_writes_no_report() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let over = [("vox_think_init", score(f64::NAN, Some(32)))];
        let mut calls = 0;
        let mut scorer = scorer_over(&over, &mut calls);
        let err = run_collateral_damage_with(&pre, dir.path(), &mut scorer)
            .unwrap_err()
            .to_string();
        assert!(err.contains("non-finite"), "{err}");
        no_report(dir.path());
    }

    #[test]
    fn a_reasoning_score_without_an_item_count_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let over = [("gsm8k_think", score(0.625, None))];
        let mut calls = 0;
        let mut scorer = scorer_over(&over, &mut calls);
        assert!(run_collateral_damage_with(&pre, dir.path(), &mut scorer).is_err());
        no_report(dir.path());
    }

    #[test]
    fn a_reasoning_score_with_31_items_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let over = [
            ("gsm8k_think", score(0.625, Some(31))),
            ("gsm8k_think_init", score(0.625, Some(31))),
        ];
        let mut calls = 0;
        let mut scorer = scorer_over(&over, &mut calls);
        let err = run_collateral_damage_with(&pre, dir.path(), &mut scorer)
            .unwrap_err()
            .to_string();
        assert!(err.contains("expected 32"), "{err}");
        no_report(dir.path());
    }

    #[test]
    fn mismatched_item_counts_between_a_slice_and_its_init_key_refuse() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let over = [
            ("math500_think", score(0.625, Some(32))),
            ("math500_think_init", score(0.625, Some(31))),
        ];
        let mut calls = 0;
        let mut scorer = scorer_over(&over, &mut calls);
        assert!(run_collateral_damage_with(&pre, dir.path(), &mut scorer).is_err());
        no_report(dir.path());
    }

    #[test]
    fn a_stale_passing_report_is_removed_when_scoring_fails() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        std::fs::write(
            dir.path().join("collateral_damage_report.json"),
            r#"{"status":"pass"}"#,
        )
        .unwrap();
        let mut scorer = |_: &str| anyhow::bail!("adapter not found");
        assert!(run_collateral_damage_with(&pre, dir.path(), &mut scorer).is_err());
        no_report(dir.path());
    }

    #[test]
    fn a_stale_passing_report_is_removed_when_the_baseline_is_incomplete() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &["vox_think_init"], &[]);
        std::fs::write(
            dir.path().join("collateral_damage_report.json"),
            r#"{"status":"pass"}"#,
        )
        .unwrap();
        let mut calls = 0;
        let mut scorer = scorer_over(&[], &mut calls);
        assert!(run_collateral_damage_with(&pre, dir.path(), &mut scorer).is_err());
        no_report(dir.path());
    }

    fn assert_labelled(report: &serde_json::Value) {
        let note = report["reasoning_slices"]["scope_note"].as_str().unwrap();
        assert!(
            note.contains("32-item") && note.contains("not full"),
            "{note}"
        );
        assert_eq!(report["reasoning_slices"]["slice_size"], 32);
        for name in SLICE_NAMES {
            assert_eq!(
                report["reasoning_slices"]["benches"][name]["n"], 32,
                "{name}"
            );
        }
    }

    #[test]
    fn an_accuracy_only_drop_fails_and_names_the_slice() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let over = [("gsm8k_think", score(0.5625, Some(32)))];
        let mut calls = 0;
        let mut scorer = scorer_over(&over, &mut calls);
        let code = run_collateral_damage_with(&pre, dir.path(), &mut scorer).unwrap();
        assert_eq!(code, 1);
        let report = read_report(dir.path());
        assert_eq!(report["status"], "fail");
        assert_eq!(report["failed_on"], "gsm8k_think");
        assert_labelled(&report);
        let reports = report["reports"].as_array().unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0]["slice"], true);
        assert_eq!(reports[0]["n"], 32);
    }

    #[test]
    fn a_think_initiation_only_drop_fails_and_names_the_init_key() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[("vox_think_init", 0.5)]);
        let over = [("vox_think_init", score(0.46875, Some(32)))];
        let mut calls = 0;
        let mut scorer = scorer_over(&over, &mut calls);
        let code = run_collateral_damage_with(&pre, dir.path(), &mut scorer).unwrap();
        assert_eq!(code, 1);
        let report = read_report(dir.path());
        assert_eq!(report["status"], "fail");
        assert_eq!(report["failed_on"], "vox_think_init");
        assert_labelled(&report);
    }

    #[test]
    fn a_non_reasoning_bench_keeps_one_score_and_no_slice_field() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[("general_bench", 0.85)]);
        let mut calls = 0;
        let mut scorer = scorer_over(&[], &mut calls);
        assert_eq!(
            run_collateral_damage_with(&pre, dir.path(), &mut scorer).unwrap(),
            0
        );
        let report = read_report(dir.path());
        let entry = report["reports"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["benchmark"] == "general_bench")
            .unwrap();
        assert!(entry.get("slice").is_none() && entry.get("n").is_none());
        assert_eq!(entry["pre"], 0.85);
        assert_eq!(entry["post"], 0.84);
    }

    fn fail_fast_order() -> Vec<String> {
        ["vox_think", "gsm8k_think", "math500_think"]
            .map(String::from)
            .to_vec()
    }

    #[test]
    fn fail_fast_stops_at_the_first_failing_slice_and_marks_the_rest_not_run() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let mut asked: Vec<String> = Vec::new();
        let mut scorer = |bench: &str| {
            asked.push(bench.to_string());
            Ok(match bench {
                // gsm8k also degrades: it must never be scored, so it cannot be the verdict.
                "vox_think" | "gsm8k_think" => score(0.40625, Some(32)),
                _ => flat(bench, 0.84),
            })
        };
        let code =
            run_collateral_damage_fail_fast_with(&pre, dir.path(), &fail_fast_order(), &mut scorer)
                .unwrap();
        assert_eq!(code, 1);
        assert_eq!(asked, ["vox_think", "vox_think_init"]);
        let report = read_report(dir.path());
        assert_eq!(report["status"], "fail");
        assert_eq!(report["failed_on"], "vox_think");
        assert_eq!(report["fail_fast_order"], json!(fail_fast_order()));
        let not_run = report["not_run"].as_array().unwrap();
        assert_eq!(not_run.len(), 2);
        for (entry, name) in not_run.iter().zip(["gsm8k_think", "math500_think"]) {
            assert_eq!(entry["benchmark"], name);
            assert_eq!(entry["status"], "not_run");
            assert_eq!(entry["reason"], "fail_fast_after_vox_think");
            assert!(entry.get("post").is_none(), "no invented score");
        }
        let benches = report["reasoning_slices"]["benches"].as_object().unwrap();
        assert_eq!(benches.keys().collect::<Vec<_>>(), ["vox_think"]);
    }

    #[test]
    fn fail_fast_runs_every_slice_in_order_when_all_pass() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let mut asked: Vec<String> = Vec::new();
        let mut scorer = |bench: &str| {
            asked.push(bench.to_string());
            Ok(flat(bench, 0.84))
        };
        let code =
            run_collateral_damage_fail_fast_with(&pre, dir.path(), &fail_fast_order(), &mut scorer)
                .unwrap();
        assert_eq!(code, 0);
        assert_eq!(
            asked,
            [
                "vox_think",
                "vox_think_init",
                "gsm8k_think",
                "gsm8k_think_init",
                "math500_think",
                "math500_think_init"
            ]
        );
        let report = read_report(dir.path());
        assert_eq!(report["status"], "pass");
        assert_eq!(report["not_run"], json!([]));
        assert_labelled(&report);
        vox_eval::reasoning_slice::verify_reasoning_slices(
            &report,
            &CollateralDamageConfig::default(),
        )
        .unwrap();
    }

    #[test]
    fn fail_fast_refuses_an_order_that_skips_a_slice() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let mut calls = 0;
        {
            let mut scorer = scorer_over(&[], &mut calls);
            let order = ["vox_think", "gsm8k_think"].map(String::from).to_vec();
            assert!(
                run_collateral_damage_fail_fast_with(&pre, dir.path(), &order, &mut scorer)
                    .is_err()
            );
        }
        assert_eq!(calls, 0);
        no_report(dir.path());
    }
}
