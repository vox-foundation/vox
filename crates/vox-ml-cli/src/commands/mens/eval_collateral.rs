use anyhow::{Context, Result};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use vox_eval::reasoning_bench::{BenchOutcome, ReasoningItem, SLICE_NAMES, SLICE_SIZE};
use vox_eval::reasoning_slice::{
    ItemFlag, PairedMetric, THINK_INIT_SUFFIX, eval_reasoning_slice, pooled_accuracy,
    reasoning_slice_of, reasoning_slices_json, require_reasoning_keys,
};
use vox_eval::{CollateralDamageConfig, CollateralDamageReport, eval_collateral_damage_suite};

/// Benchmark directories are looked up by name under this root, matching the
/// `--bench mens/data/<name>` convention `vox mens eval-local` uses.
const BENCH_ROOT: &str = "mens/data";

/// The policy whose `collateral_damage.alpha` the reasoning slices are tested at
/// (relative to the repo root, like [`BENCH_ROOT`]). The number lives there, not here.
pub(crate) const LADDER_POLICY: &str = "mens/config/eval-gates-ladder.yaml";

/// One scored benchmark. `n` is the item count for reasoning slices, `None` for
/// eval-local benches. Reasoning keys also carry `paired`: per-item flags
/// `(base, adapter)` for the same items, decoded with the same settings.
#[derive(Debug, Clone)]
pub(crate) struct BenchScore {
    pub(crate) value: f64,
    pub(crate) n: Option<usize>,
    pub(crate) paired: Option<(Vec<ItemFlag>, Vec<ItemFlag>)>,
    /// BLAKE3 of the slice file a reasoning key was scored on, checked against its
    /// policy pin before scoring; the report records it for the gate.
    pub(crate) items_blake3: Option<String>,
}

/// `{slice: items_blake3}` for the slices that carry one.
fn slice_blake3<'a>(scores: impl Iterator<Item = (&'a str, &'a BenchScore)>) -> serde_json::Value {
    scores
        .filter_map(|(name, s)| Some((name.to_string(), json!(s.items_blake3.as_ref()?))))
        .collect::<serde_json::Map<_, _>>()
        .into()
}

/// The slice file at `path` checked against `pins[name]`; its BLAKE3.
fn pinned_slice_hash(
    name: &str,
    path: &Path,
    pins: &BTreeMap<String, super::eval_gate::SlicePin>,
) -> Result<String> {
    let pin = pins.get(name).ok_or_else(|| {
        anyhow::anyhow!("{LADDER_POLICY} pins no item set for reasoning slice '{name}'")
    })?;
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    pin.verify(&bytes).map_err(|e| {
        anyhow::anyhow!(
            "{}: {e}; refusing to score an unpinned item set",
            path.display()
        )
    })?;
    Ok(pin.blake3.clone())
}

/// `collateral_damage.alpha` from the ladder policy; absent or out of range is an error.
pub(crate) fn policy_alpha(policy: &Path) -> Result<f64> {
    let p = super::eval_gate::load_policy(policy)
        .with_context(|| format!("loading {}", policy.display()))?;
    let alpha = p.collateral_damage.alpha.ok_or_else(|| {
        anyhow::anyhow!(
            "{} sets no collateral_damage.alpha; the paired reasoning test needs one",
            policy.display()
        )
    })?;
    anyhow::ensure!(
        alpha.is_finite() && alpha > 0.0 && alpha < 1.0,
        "{}: collateral_damage.alpha {alpha} is not in (0, 1)",
        policy.display()
    );
    Ok(alpha)
}

/// One paired metric as a `reports` entry.
fn paired_report_json(m: &PairedMetric, slice: bool) -> serde_json::Value {
    let mut v = json!({
        "benchmark": m.key,
        "pre": m.pre,
        "post": m.post,
        "losses": m.losses,
        "gains": m.gains,
        "p_value": m.p_value,
        "regressed": m.regressed,
        "n": m.n,
    });
    if slice {
        v["slice"] = json!(true);
    }
    v
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

fn default_eval_settings() -> vox_eval::decode_protocol::DecodeSettings {
    vox_eval::decode_protocol::DecodeSettings::default()
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
    alpha: f64,
    score_bench: &mut dyn FnMut(&str) -> Result<BenchScore>,
) -> Result<i32> {
    let settings = default_eval_settings();
    run_collateral_damage_with_settings(pre_score_path, run_dir, alpha, &settings, score_bench)
}

pub(crate) fn run_collateral_damage_with_settings(
    pre_score_path: &Path,
    run_dir: &Path,
    alpha: f64,
    settings: &vox_eval::decode_protocol::DecodeSettings,
    score_bench: &mut dyn FnMut(&str) -> Result<BenchScore>,
) -> Result<i32> {
    let pre = load_pre(pre_score_path, run_dir, true, &settings.protocol)?;

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
            pre_score_path,
            &pre,
            &post[name],
            &post[init.as_str()],
            alpha,
        )?);
    }
    let pooled = pooled_accuracy(&verdicts, alpha).map_err(|e| anyhow::anyhow!(e))?;

    let failing_slice = verdicts.iter().find_map(|v| v.failing_metric());
    let failing = failing_slice.or(pooled.regressed.then_some(&pooled));
    let (status, failed_on, reports_json) = match (&generic_result, failing) {
        (Err(bad), _) => (
            "fail",
            Some(bad.benchmark_name.clone()),
            vec![report_json(bad, None)],
        ),
        (Ok(_), Some(m)) => (
            "fail",
            Some(m.key.clone()),
            vec![paired_report_json(m, m.key != pooled.key)],
        ),
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
            "reasoning_slices": reasoning_slices_json(&verdicts, Some(&pooled), alpha),
            "slice_blake3": slice_blake3(SLICE_NAMES.iter().map(|n| (*n, &post[*n]))),
            "grader": vox_eval::qwen_reasoning::GRADER_ID,
            "decode": settings.report_object(),
        }),
        &failed_on,
    )
}

/// Default fail-fast order: cheapest-to-fail-first for the mens ladder.
///
/// `gsm8k_think` historically fails first at small base accuracy (n=32, base≈0.5),
/// so scoring it before math500/vox avoids multi-hour full-suite runs after an
/// early failure. Operators can override via `--fail-fast a,b,c`.
#[must_use]
pub fn default_fail_fast_order() -> Vec<String> {
    ["gsm8k_think", "math500_think", "vox_think"]
        .map(String::from)
        .to_vec()
}

/// Validate a fail-fast order: non-empty, known slice names, no duplicates.
/// A **subset** of [`SLICE_NAMES`] is allowed (debug / single-slice smoke).
/// Hub-eligible `status=pass` still requires every slice in [`SLICE_NAMES`] to
/// have been scored and passed — see [`run_collateral_damage_fail_fast_with`].
pub(crate) fn validate_fail_fast_order(order: &[String]) -> Result<()> {
    if order.is_empty() {
        anyhow::bail!("fail-fast order is empty; pass known slices {SLICE_NAMES:?}");
    }
    let mut seen = std::collections::BTreeSet::new();
    for name in order {
        if !SLICE_NAMES.contains(&name.as_str()) {
            anyhow::bail!("unknown reasoning slice '{name}'; known {SLICE_NAMES:?}");
        }
        if !seen.insert(name.as_str()) {
            anyhow::bail!("duplicate slice '{name}' in fail-fast order");
        }
    }
    Ok(())
}

/// Fail-fast variant: scores the reasoning slices one at a time in `order` and
/// stops at the first slice over the threshold. Slices later in `order` are
/// recorded as `not_run` with reason `fail_fast_after_<slice>` and no score.
/// Known slices omitted from `order` are `not_run` with reason
/// `not_in_fail_fast_order`.
///
/// - **fail**: a scored slice exceeded the threshold.
/// - **pass**: every name in [`SLICE_NAMES`] was scored and passed (Hub-eligible).
/// - **incomplete**: requested slices all passed but at least one [`SLICE_NAMES`]
///   entry was never scored (debug subset) — exit 1, not Hub-eligible.
///
/// The baseline may be partial (only the slices a sliced compare scored);
/// reaching a slice it does not name is an error, before that slice is scored.
pub(crate) fn run_collateral_damage_fail_fast_with(
    pre_score_path: &Path,
    run_dir: &Path,
    order: &[String],
    alpha: f64,
    score_bench: &mut dyn FnMut(&str) -> Result<BenchScore>,
) -> Result<i32> {
    let settings = default_eval_settings();
    run_collateral_damage_fail_fast_with_settings(
        pre_score_path,
        run_dir,
        order,
        alpha,
        &settings,
        score_bench,
    )
}

pub(crate) fn run_collateral_damage_fail_fast_with_settings(
    pre_score_path: &Path,
    run_dir: &Path,
    order: &[String],
    alpha: f64,
    settings: &vox_eval::decode_protocol::DecodeSettings,
    score_bench: &mut dyn FnMut(&str) -> Result<BenchScore>,
) -> Result<i32> {
    validate_fail_fast_order(order)?;
    let pre = load_pre(pre_score_path, run_dir, false, &settings.protocol)?;
    if let Some(other) = pre.keys().find(|b| reasoning_slice_of(b).is_none()) {
        anyhow::bail!("fail-fast scores reasoning slices only; the baseline also names '{other}'");
    }

    let mut verdicts = Vec::with_capacity(order.len());
    let mut scored: Vec<(&str, BenchScore)> = Vec::with_capacity(order.len());
    let mut not_run = Vec::new();
    let mut stopped_early = false;
    for (i, name) in order.iter().enumerate() {
        let init = format!("{name}{THINK_INIT_SUFFIX}");
        if let Some(missing) = [name.as_str(), init.as_str()]
            .into_iter()
            .find(|k| !pre.contains_key(*k))
        {
            anyhow::bail!(
                "fail-fast reached slice '{name}' but the baseline {} has no '{missing}' \
                 (a partial baseline); score that slice in the compare first",
                pre_score_path.display()
            );
        }
        let (acc, think) = (score_bench(name)?, score_bench(&init)?);
        let v = slice_verdict(name, pre_score_path, &pre, &acc, &think, alpha)?;
        scored.push((name.as_str(), acc));
        let failed = v.exceeds_threshold();
        verdicts.push(v);
        if failed {
            stopped_early = true;
            not_run.extend(order[i + 1..].iter().map(|rest| {
                json!({
                    "benchmark": rest,
                    "status": "not_run",
                    "reason": format!("fail_fast_after_{name}"),
                })
            }));
            break;
        }
    }

    // Names actually scored = the prefix of `order` that produced verdicts.
    let scored_names: std::collections::BTreeSet<&str> = order
        .iter()
        .take(verdicts.len())
        .map(String::as_str)
        .collect();
    for name in SLICE_NAMES {
        if !scored_names.contains(name) && !not_run.iter().any(|e| e["benchmark"] == *name) {
            not_run.push(json!({
                "benchmark": name,
                "status": "not_run",
                "reason": if stopped_early {
                    // already covered by fail_fast_after_* for remaining order;
                    // only add canonical names not already listed.
                    format!(
                        "fail_fast_after_{}",
                        order.get(verdicts.len().saturating_sub(1)).map(String::as_str).unwrap_or("?")
                    )
                } else {
                    "not_in_fail_fast_order".to_string()
                },
            }));
        }
    }

    let full_coverage = scored_names.len() == SLICE_NAMES.len() && !stopped_early;
    // Pooled over every scored slice; it can decide the verdict only at full
    // coverage (a subset run is `incomplete` anyway).
    let pooled = pooled_accuracy(&verdicts, alpha).map_err(|e| anyhow::anyhow!(e))?;
    let failing = verdicts
        .iter()
        .find_map(|v| v.failing_metric())
        .or((full_coverage && pooled.regressed).then_some(&pooled));
    let (status, failed_on, reports_json) = match failing {
        Some(m) => (
            "fail",
            Some(m.key.clone()),
            vec![paired_report_json(m, m.key != pooled.key)],
        ),
        None if full_coverage => (
            "pass",
            None,
            verdicts.iter().flat_map(slice_reports_json).collect(),
        ),
        None => (
            // Subset order passed its slices but Hub needs the full suite.
            "incomplete",
            Some("incomplete_slice_coverage".to_string()),
            verdicts.iter().flat_map(slice_reports_json).collect(),
        ),
    };
    write_report(
        run_dir,
        json!({
            "status": status,
            "failed_on": failed_on,
            "reports": reports_json,
            "reasoning_slices": reasoning_slices_json(
                &verdicts,
                full_coverage.then_some(&pooled),
                alpha
            ),
            "slice_blake3": slice_blake3(scored.iter().map(|(n, s)| (*n, s))),
            "grader": vox_eval::qwen_reasoning::GRADER_ID,
            "decode": settings.report_object(),
            "fail_fast_order": order,
            "not_run": not_run,
            "fail_fast": true,
        }),
        &failed_on,
    )
}

/// Score reasoning slices under a sequential McNemar stopping rule from checkpoints.
pub(crate) fn run_collateral_damage_sequential(
    ck_root: &Path,
    bench_root: &Path,
    pre_score_path: &Path,
    run_dir: &Path,
    base: &Path,
    order: &[String],
    rule: &vox_eval::sequential_mcnemar::SequentialRule,
    settings: &vox_eval::decode_protocol::DecodeSettings,
) -> Result<i32> {
    validate_fail_fast_order(order)?;
    let pre = load_pre(pre_score_path, run_dir, false, &settings.protocol)?;
    if let Some(other) = pre.keys().find(|b| reasoning_slice_of(b).is_none()) {
        anyhow::bail!("fail-fast scores reasoning slices only; the baseline also names '{other}'");
    }

    let mut verdicts: Vec<vox_eval::sequential_mcnemar::SequentialSliceVerdict> =
        Vec::with_capacity(order.len());
    let mut acc_all = Vec::new();
    let mut not_run = Vec::new();
    let mut stopped_early = false;
    let mut failed_slice_name = None;

    for (i, name) in order.iter().enumerate() {
        let file = bench_root.join(name).join("reasoning.jsonl");
        let items = load_slice_file(&file)?;
        let (acc_outcomes, think_outcomes, contiguous_len) =
            sequential_outcomes(ck_root, base, run_dir, name, &items, settings)?;

        let v = rule.judge_slice(name, &acc_outcomes, &think_outcomes);
        if v.stopped_at_look.is_some() {
            stopped_early = true;
            failed_slice_name = Some(name.clone());
            verdicts.push(v);
            for rest in &order[i + 1..] {
                not_run.push(json!({
                    "benchmark": rest,
                    "status": "not_run",
                    "reason": format!("fail_fast_after_{name}"),
                }));
            }
            break;
        } else {
            if contiguous_len != SLICE_SIZE {
                anyhow::bail!(
                    "{name}: slice did not stop early but has only {contiguous_len} of {SLICE_SIZE} items"
                );
            }
            acc_all.push((name.clone(), acc_outcomes));
            verdicts.push(v);
        }
    }

    let scored_names: std::collections::BTreeSet<&str> = order
        .iter()
        .take(verdicts.len())
        .map(String::as_str)
        .collect();
    for name in SLICE_NAMES {
        if !scored_names.contains(name) && !not_run.iter().any(|e| e["benchmark"] == *name) {
            not_run.push(json!({
                "benchmark": name,
                "status": "not_run",
                "reason": if stopped_early {
                    format!(
                        "fail_fast_after_{}",
                        order
                            .get(verdicts.len().saturating_sub(1))
                            .map(String::as_str)
                            .unwrap_or("?")
                    )
                } else {
                    "not_in_fail_fast_order".to_string()
                },
            }));
        }
    }

    let full_coverage = scored_names.len() == SLICE_NAMES.len() && !stopped_early;
    let pooled = if full_coverage {
        let total_n = SLICE_SIZE * SLICE_NAMES.len();
        let mut total_losses = 0;
        let mut total_gains = 0;
        for (_, outcomes) in &acc_all {
            for o in outcomes {
                match o {
                    vox_eval::sequential_mcnemar::PairOutcome::Loss => total_losses += 1,
                    vox_eval::sequential_mcnemar::PairOutcome::Gain => total_gains += 1,
                    vox_eval::sequential_mcnemar::PairOutcome::Tie => {}
                }
            }
        }
        let total_base_ok = (total_n - total_gains).max(total_losses);
        let p = vox_eval::reasoning_slice::PairedMetric::from_counts(
            vox_eval::reasoning_slice::POOLED_KEY,
            total_n,
            total_base_ok,
            total_losses,
            total_gains,
            rule.alpha(),
        )
        .map_err(|e| anyhow::anyhow!(e))?;
        Some(p)
    } else {
        None
    };

    let (status, failed_on, reports_json) = if stopped_early {
        let failed_name = failed_slice_name.unwrap_or_else(|| "unknown".to_string());
        let rep = json!({
            "benchmark": failed_name,
            "status": "fail",
            "stopped_at_look": verdicts.last().and_then(|v| v.stopped_at_look),
            "items_used": verdicts.last().map(|v| v.items_used).unwrap_or(0),
        });
        ("fail", Some(failed_name), vec![rep])
    } else if full_coverage {
        if let Some(ref p) = pooled {
            if p.regressed {
                (
                    "fail",
                    Some(p.key.clone()),
                    vec![paired_report_json(p, false)],
                )
            } else {
                let reps: Vec<serde_json::Value> = verdicts
                    .iter()
                    .map(|v| {
                        json!({
                            "benchmark": v.slice,
                            "status": "pass",
                            "stopped_at_look": v.stopped_at_look,
                            "items_used": v.items_used,
                        })
                    })
                    .collect();
                ("pass", None, reps)
            }
        } else {
            ("pass", None, Vec::new())
        }
    } else {
        (
            "incomplete",
            Some("incomplete_slice_coverage".to_string()),
            verdicts
                .iter()
                .map(|v| {
                    json!({
                        "benchmark": v.slice,
                        "status": "incomplete",
                        "items_used": v.items_used,
                    })
                })
                .collect(),
        )
    };

    write_report(
        run_dir,
        json!({
            "status": status,
            "failed_on": failed_on,
            "reports": reports_json,
            "reasoning_slices": vox_eval::sequential_mcnemar::sequential_slices_json(
                &verdicts,
                pooled.as_ref(),
                rule
            ),
            "grader": vox_eval::qwen_reasoning::GRADER_ID,
            "decode": settings.report_object(),
            "fail_fast_order": order,
            "not_run": not_run,
            "fail_fast": true,
        }),
        &failed_on,
    )
}

/// Validate that baseline pre-score file recorded protocol matches the run's protocol.
pub(crate) fn require_baseline_protocol(
    pre_file: &serde_json::Value,
    run: &vox_eval::decode_protocol::DecodeProtocol,
    pre_score_path: &Path,
) -> Result<()> {
    let base_proto = vox_eval::decode_protocol::DecodeProtocol::from_report(pre_file.get("decode"))
        .map_err(|e| anyhow::anyhow!("baseline {} decode object: {e}", pre_score_path.display()))?;
    if base_proto != *run {
        let base_tag = base_proto.tag();
        let run_tag = run.tag();
        anyhow::bail!(
            "baseline {} was decoded under {base_tag}; this run uses {run_tag}. \
             Regenerate it under the same protocol: eval-reasoning-compare, then \
             baseline-from-compare, with VOX_MENS_DECODE_PROTOCOL={run_tag}",
            pre_score_path.display()
        );
    }
    Ok(())
}

/// Removes any stale report, then reads and validates the baseline. With
/// `require_all` every reasoning key must be present.
fn load_pre(
    pre_score_path: &Path,
    run_dir: &Path,
    require_all: bool,
    run_protocol: &vox_eval::decode_protocol::DecodeProtocol,
) -> Result<BTreeMap<String, f64>> {
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
    require_baseline_protocol(&pre_file, run_protocol, pre_score_path)?;
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
    if require_all {
        require_reasoning_keys(pre.keys().map(String::as_str)).map_err(|e| anyhow::anyhow!(e))?;
    }
    Ok(pre)
}

/// The paired verdict for one slice. The scorer's per-item flags decide it; the
/// baseline file's numbers must agree with the base side's re-scored items, so a
/// baseline from another grader, token cap or base cannot pass silently.
fn slice_verdict(
    name: &str,
    pre_score_path: &Path,
    pre: &BTreeMap<String, f64>,
    acc: &BenchScore,
    think: &BenchScore,
    alpha: f64,
) -> Result<vox_eval::reasoning_slice::ReasoningSliceVerdict> {
    let init = format!("{name}{THINK_INIT_SUFFIX}");
    match (acc.n, think.n) {
        (Some(a), Some(t)) if a == t => {}
        (Some(a), Some(t)) => {
            anyhow::bail!("{name}: item counts differ ({name}: {a}, {init}: {t})")
        }
        _ => anyhow::bail!("{name}: scorer returned no item count for {name} or {init}"),
    }
    let (Some(acc_pair), Some(think_pair)) = (&acc.paired, &think.paired) else {
        anyhow::bail!(
            "{name}: scorer returned no per-item base/adapter outcomes for {name} or {init}"
        );
    };
    let v = eval_reasoning_slice(
        name,
        (&acc_pair.0, &acc_pair.1),
        (&think_pair.0, &think_pair.1),
        alpha,
    )
    .map_err(|e| anyhow::anyhow!(e))?;
    for (key, score, m) in [
        (name, acc, &v.accuracy),
        (init.as_str(), think, &v.think_initiation),
    ] {
        // `!(x <= tol)` so a NaN score is refused too.
        if !((score.value - m.post).abs() <= 1e-9) {
            anyhow::bail!(
                "{key}: scorer value {} disagrees with its own items ({}); refusing",
                score.value,
                m.post
            );
        }
        if !((pre[key] - m.pre).abs() <= 1e-9) {
            anyhow::bail!(
                "{key}: baseline {} says {} but the base re-scored per item gives {} \
                 (another grader, token cap or base?); regenerate the baseline",
                pre_score_path.display(),
                pre[key],
                m.pre
            );
        }
    }
    Ok(v)
}

fn slice_reports_json(
    v: &vox_eval::reasoning_slice::ReasoningSliceVerdict,
) -> [serde_json::Value; 2] {
    [
        paired_report_json(&v.accuracy, true),
        paired_report_json(&v.think_initiation, true),
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
    match (report.get("status").and_then(|s| s.as_str()), failed_on) {
        (Some("pass"), _) => {
            println!("Collateral damage check PASSED.");
            Ok(0)
        }
        (Some("incomplete"), _) => {
            eprintln!(
                "Collateral damage check INCOMPLETE: scored a slice subset only \
                 (not Hub-eligible). Re-run with the default fail-fast order \
                 (all of {SLICE_NAMES:?}) or --no-fail-fast."
            );
            Ok(1)
        }
        (_, Some(bench)) => {
            eprintln!("Collateral damage check FAILED on benchmark '{bench}'.");
            Ok(1)
        }
        _ => {
            eprintln!("Collateral damage check FAILED.");
            Ok(1)
        }
    }
}

fn load_slice_file(file: &Path) -> Result<Vec<ReasoningItem>> {
    let text =
        std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let n = text.lines().filter(|l| !l.trim().is_empty()).count();
    vox_eval::reasoning_bench::load_slice(file, n)
        .map_err(|e| anyhow::anyhow!("{}: {e}", file.display()))
}

/// Prefill base reasoning slices in `order` into checkpoints.
pub(crate) fn prefill_collateral_base(
    ck_root: &Path,
    bench_root: &Path,
    base: &Path,
    order: &[String],
    settings: &vox_eval::decode_protocol::DecodeSettings,
    force_restart_base: bool,
    base_infer: &mut dyn FnMut(&str) -> Result<String>,
) -> Result<()> {
    let mut benches: Vec<(String, Vec<ReasoningItem>)> = Vec::with_capacity(order.len());
    for name in order {
        let file = bench_root.join(name).join("reasoning.jsonl");
        let items = load_slice_file(&file)?;
        benches.push((name.clone(), items));
    }
    let bench_refs: Vec<(&str, &[ReasoningItem])> = benches
        .iter()
        .map(|(n, items)| (n.as_str(), items.as_slice()))
        .collect();

    // Base generations are prefilled for every pending slice in one stream because
    // they are cached per snapshot and shared by every adapter.
    super::reasoning_eval::prefill_benches(
        ck_root,
        base,
        &bench_refs,
        settings,
        force_restart_base,
        &mut |_, _| Ok(super::reasoning_eval::StreamControl::Continue),
        base_infer,
    )
}

/// Read the contiguous pinned-order prefix recorded in both base and adapter checkpoints
/// for `bench`, without invoking an inference model.
pub(crate) fn sequential_outcomes(
    ck_root: &Path,
    base: &Path,
    adapter: &Path,
    bench: &str,
    items: &[ReasoningItem],
    settings: &vox_eval::decode_protocol::DecodeSettings,
) -> Result<(
    Vec<vox_eval::sequential_mcnemar::PairOutcome>,
    Vec<vox_eval::sequential_mcnemar::PairOutcome>,
    usize,
)> {
    let mut base_ck =
        super::item_checkpoint::reasoning_checkpoint(ck_root, base, bench, items, settings, false)?;
    let mut adapter_ck = super::item_checkpoint::reasoning_checkpoint(
        ck_root, adapter, bench, items, settings, false,
    )?;

    let mut refuse = |_: &str| -> Result<String> {
        anyhow::bail!("infer must not be called during sequential checkpoint readout")
    };

    let mut acc_outcomes = Vec::with_capacity(items.len());
    let mut think_outcomes = Vec::with_capacity(items.len());
    let mut contiguous_len = 0usize;

    for item in items {
        let prompt = super::reasoning_eval::prompt_request(item, settings.max_tokens);
        if !adapter_ck.has(&prompt) {
            break;
        }
        if !base_ck.has(&prompt) {
            anyhow::bail!(
                "{bench}: item {} missing in base checkpoint {}",
                item.id,
                base.display()
            );
        }
        let base_resp = base_ck.infer(&prompt, &mut refuse)?;
        let adapter_resp = adapter_ck.infer(&prompt, &mut refuse)?;
        let base_text = super::reasoning_eval::usable_text(&base_resp)
            .ok_or_else(|| anyhow::anyhow!("{bench}: base item {} has no usable text", item.id))?;
        let adapter_text = super::reasoning_eval::usable_text(&adapter_resp).ok_or_else(|| {
            anyhow::anyhow!("{bench}: adapter item {} has no usable text", item.id)
        })?;

        let base_acc = vox_eval::qwen_reasoning::grade_final_answer(&base_text, &item.answer);
        let adapter_acc = vox_eval::qwen_reasoning::grade_final_answer(&adapter_text, &item.answer);
        let base_think = vox_eval::qwen_reasoning::opened_nonempty_think(&base_text);
        let adapter_think = vox_eval::qwen_reasoning::opened_nonempty_think(&adapter_text);

        acc_outcomes.push(vox_eval::sequential_mcnemar::PairOutcome::from_flags(
            base_acc,
            adapter_acc,
        ));
        think_outcomes.push(vox_eval::sequential_mcnemar::PairOutcome::from_flags(
            base_think,
            adapter_think,
        ));
        contiguous_len += 1;
    }

    Ok((acc_outcomes, think_outcomes, contiguous_len))
}

/// Prefill adapter reasoning slices in `order` into checkpoints with fail-fast cancellation.
pub(crate) fn prefill_collateral_adapter(
    ck_root: &Path,
    bench_root: &Path,
    pre_score_path: &Path,
    pre: &BTreeMap<String, f64>,
    base: &Path,
    adapter: &Path,
    order: &[String],
    fail_fast: bool,
    alpha: f64,
    settings: &vox_eval::decode_protocol::DecodeSettings,
    force_restart_adapter: bool,
    seq_rule: Option<&vox_eval::sequential_mcnemar::SequentialRule>,
    adapter_infer: &mut dyn FnMut(&str) -> Result<String>,
) -> Result<()> {
    let mut benches: Vec<(String, Vec<ReasoningItem>)> = Vec::with_capacity(order.len());
    for name in order {
        let file = bench_root.join(name).join("reasoning.jsonl");
        let items = load_slice_file(&file)?;
        benches.push((name.clone(), items));
    }
    let bench_refs: Vec<(&str, &[ReasoningItem])> = benches
        .iter()
        .map(|(n, items)| (n.as_str(), items.as_slice()))
        .collect();

    let mut completed = std::collections::BTreeSet::new();
    let mut rejected_slices = std::collections::BTreeMap::<String, usize>::new();
    let mut next_judge_idx = 0usize;

    let mut on_record =
        |bench: &str, complete: bool| -> Result<super::reasoning_eval::StreamControl> {
            if !fail_fast {
                return Ok(super::reasoning_eval::StreamControl::Continue);
            }
            if complete {
                completed.insert(bench.to_string());
            }

            if let Some(rule) = seq_rule {
                if let Some(b_idx) = order.iter().position(|s| s == bench) {
                    let items = &benches[b_idx].1;
                    let (acc_outcomes, think_outcomes, _prefix_len) =
                        sequential_outcomes(ck_root, base, adapter, bench, items, settings)?;
                    let v = rule.judge_slice(bench, &acc_outcomes, &think_outcomes);
                    if let Some(look_idx) = v.stopped_at_look {
                        rejected_slices.insert(bench.to_string(), look_idx);
                    }
                }

                while next_judge_idx < order.len() {
                    let slice_name = &order[next_judge_idx];
                    if rejected_slices.contains_key(slice_name) {
                        return Ok(super::reasoning_eval::StreamControl::Cancel);
                    }
                    if completed.contains(slice_name) {
                        next_judge_idx += 1;
                    } else {
                        break;
                    }
                }
            } else {
                while next_judge_idx < order.len() && completed.contains(&order[next_judge_idx]) {
                    let slice_name = &order[next_judge_idx];
                    let items = &benches[next_judge_idx].1;

                    let mut refuse = |_: &str| -> Result<String> {
                        anyhow::bail!("infer must not be called during judgment from checkpoint")
                    };

                    let (_, base_rows) = super::reasoning_eval::score_slice_checkpointed(
                        ck_root,
                        base,
                        slice_name,
                        items,
                        settings,
                        false,
                        &mut refuse,
                    )?;
                    let (adapter_outcome, adapter_rows) =
                        super::reasoning_eval::score_slice_checkpointed(
                            ck_root,
                            adapter,
                            slice_name,
                            items,
                            settings,
                            false,
                            &mut refuse,
                        )?;
                    let adapter_scored = match adapter_outcome {
                        BenchOutcome::Scored(s) => s,
                        _ => anyhow::bail!("{slice_name}: adapter not scored from checkpoint"),
                    };

                    let acc_base_flags: Vec<ItemFlag> = base_rows
                        .iter()
                        .map(|(id, _, c, _)| (id.clone(), *c))
                        .collect();
                    let acc_adapter_flags: Vec<ItemFlag> = adapter_rows
                        .iter()
                        .map(|(id, _, c, _)| (id.clone(), *c))
                        .collect();
                    let think_base_flags: Vec<ItemFlag> = base_rows
                        .iter()
                        .map(|(id, _, _, t)| (id.clone(), *t))
                        .collect();
                    let think_adapter_flags: Vec<ItemFlag> = adapter_rows
                        .iter()
                        .map(|(id, _, _, t)| (id.clone(), *t))
                        .collect();

                    let acc = BenchScore {
                        value: adapter_scored.accuracy,
                        n: Some(items.len()),
                        paired: Some((acc_base_flags, acc_adapter_flags)),
                        items_blake3: None,
                    };
                    let think = BenchScore {
                        value: adapter_scored.think_initiation_rate,
                        n: Some(items.len()),
                        paired: Some((think_base_flags, think_adapter_flags)),
                        items_blake3: None,
                    };

                    let v = slice_verdict(slice_name, pre_score_path, pre, &acc, &think, alpha)?;
                    if v.exceeds_threshold() {
                        return Ok(super::reasoning_eval::StreamControl::Cancel);
                    }
                    next_judge_idx += 1;
                }
            }
            Ok(super::reasoning_eval::StreamControl::Continue)
        };

    super::reasoning_eval::prefill_benches(
        ck_root,
        adapter,
        &bench_refs,
        settings,
        force_restart_adapter,
        &mut on_record,
        adapter_infer,
    )
}

/// Testable core of collateral prefill: runs base prefill, then adapter prefill
/// with fail-fast cancellation.
pub(crate) fn prefill_collateral_with(
    ck_root: &Path,
    bench_root: &Path,
    pre_score_path: &Path,
    pre: &BTreeMap<String, f64>,
    base: &Path,
    adapter: &Path,
    order: &[String],
    fail_fast: bool,
    alpha: f64,
    settings: &vox_eval::decode_protocol::DecodeSettings,
    force: crate::commands::mens::item_checkpoint::ForceRestart,
    seq_rule: Option<&vox_eval::sequential_mcnemar::SequentialRule>,
    base_infer: &mut dyn FnMut(&str) -> Result<String>,
    adapter_infer: &mut dyn FnMut(&str) -> Result<String>,
) -> Result<()> {
    prefill_collateral_base(
        ck_root,
        bench_root,
        base,
        order,
        settings,
        force.for_base(),
        base_infer,
    )?;
    prefill_collateral_adapter(
        ck_root,
        bench_root,
        pre_score_path,
        pre,
        base,
        adapter,
        order,
        fail_fast,
        alpha,
        settings,
        force.for_adapter(),
        seq_rule,
        adapter_infer,
    )?;
    Ok(())
}

/// CLI entry point. Builds the real, adapter-backed scorer and owns the exit
/// code — the `exit` lives here so the core above stays testable.
///
/// **Fail-fast is the default** (see [`default_fail_fast_order`]). Pass
/// `no_fail_fast = true` to score the full baseline suite without early stop.
pub fn run_collateral_damage(
    pre_score_path: PathBuf,
    post_adapter_path: PathBuf,
    fail_fast_order: Option<Vec<String>>,
    no_fail_fast: bool,
    force: crate::commands::mens::item_checkpoint::ForceRestart,
) -> Result<()> {
    println!(
        "Evaluating collateral damage against baseline: {}",
        pre_score_path.display()
    );
    println!("Using adapter: {}", post_adapter_path.display());
    let alpha = policy_alpha(Path::new(LADDER_POLICY))?;
    let base = base_snapshot_of(&post_adapter_path)?;
    println!(
        "Base (paired, same decode settings): {}  alpha={alpha} (from {LADDER_POLICY})",
        base.display()
    );

    let max_tokens = super::reasoning_eval::parse_max_tokens(
        super::reasoning_eval::max_tokens_env().as_deref(),
    )?;
    let settings = super::reasoning_eval::decode_settings(max_tokens)?;

    let run_order: Vec<String> = if no_fail_fast {
        SLICE_NAMES.iter().map(|s| s.to_string()).collect()
    } else {
        fail_fast_order
            .clone()
            .unwrap_or_else(default_fail_fast_order)
    };
    validate_fail_fast_order(&run_order)?;

    let pre_file: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&pre_score_path).with_context(|| {
            format!("Failed to read pre-score from {}", pre_score_path.display())
        })?)?;
    require_baseline_protocol(&pre_file, &settings.protocol, &pre_score_path)?;
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

    // 1. Base prefill (one model load)
    // Base generations are prefilled for every pending slice in one stream because
    // they are cached per snapshot and shared by every adapter.
    super::reasoning_eval::with_backend(&base, false, |base_infer| {
        prefill_collateral_base(
            Path::new(super::item_checkpoint::REASONING_CHECKPOINT_ROOT),
            Path::new(BENCH_ROOT),
            &base,
            &run_order,
            &settings,
            force.for_base(),
            base_infer,
        )
    })??;

    let policy = super::eval_gate::load_policy(Path::new(LADDER_POLICY))?;
    let seq_rule = policy.collateral_damage.sequential_rule()?;

    // 2. Adapter prefill (one model load)
    super::reasoning_eval::with_backend(&post_adapter_path, false, |adapter_infer| {
        prefill_collateral_adapter(
            Path::new(super::item_checkpoint::REASONING_CHECKPOINT_ROOT),
            Path::new(BENCH_ROOT),
            &pre_score_path,
            &pre,
            &base,
            &post_adapter_path,
            &run_order,
            !no_fail_fast,
            alpha,
            &settings,
            force.for_adapter(),
            seq_rule.as_ref(),
            adapter_infer,
        )
    })??;

    let pins = policy.collateral_damage.slices;
    // 3. Score from checkpoints (zero model loads)
    // The restart already happened in the prefill, so pass ForceRestart::none()
    // so records are never discarded again.
    let mut scorer = |bench: &str| {
        score_bench(
            bench,
            &base,
            &post_adapter_path,
            &settings,
            crate::commands::mens::item_checkpoint::ForceRestart {
                adapter: false,
                base: false,
            },
            &pins,
        )
    };
    let code = if let Some(ref rule) = seq_rule {
        println!("Mode: sequential McNemar v1 order={run_order:?}");
        run_collateral_damage_sequential(
            Path::new(super::item_checkpoint::REASONING_CHECKPOINT_ROOT),
            Path::new(BENCH_ROOT),
            &pre_score_path,
            &post_adapter_path,
            &base,
            &run_order,
            rule,
            &settings,
        )?
    } else if no_fail_fast {
        println!("Mode: full suite (--no-fail-fast)");
        run_collateral_damage_with_settings(
            &pre_score_path,
            &post_adapter_path,
            alpha,
            &settings,
            &mut scorer,
        )?
    } else {
        println!("Mode: fail-fast order={run_order:?}");
        run_collateral_damage_fail_fast_with_settings(
            &pre_score_path,
            &post_adapter_path,
            &run_order,
            alpha,
            &settings,
            &mut scorer,
        )?
    };
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

/// The base snapshot an adapter was trained on: `base_model` in its
/// `adapter_manifest.json`. It is the same directory `eval-reasoning-compare`
/// scores as the base, so both share one base checkpoint per decode setting.
pub(crate) fn base_snapshot_of(adapter: &Path) -> Result<PathBuf> {
    let path = adapter.join("adapter_manifest.json");
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?,
    )
    .with_context(|| format!("parsing {}", path.display()))?;
    let base = manifest["base_model"]
        .as_str()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("{} has no base_model", path.display()))?;
    anyhow::ensure!(
        base.is_dir(),
        "{}: base_model {} is not a local snapshot directory",
        path.display(),
        base.display()
    );
    Ok(base)
}

/// One side of one reasoning bench, scored once per process. `base` sides are
/// scored without writing a report into the (read-only) base snapshot.
fn cached_reasoning_report(
    bench: &str,
    items: &Path,
    model: &Path,
    settings: &vox_eval::decode_protocol::DecodeSettings,
    is_base: bool,
    force_restart: bool,
) -> Result<super::reasoning_eval::ReasoningReport> {
    use std::sync::Mutex;
    static CACHE: Mutex<Vec<(String, super::reasoning_eval::ReasoningReport)>> =
        Mutex::new(Vec::new());
    let key = format!("{}::{bench}", model.display());
    {
        let guard = CACHE.lock().unwrap_or_else(|err| err.into_inner());
        if let Some((_, report)) = guard.iter().find(|(k, _)| k == &key) {
            return Ok(report.clone());
        }
    }
    let report = if is_base {
        super::reasoning_eval::score_base_file_with_settings(
            bench,
            items,
            model,
            settings,
            force_restart,
        )?
    } else {
        super::reasoning_eval::score_reasoning_file_with_settings(
            bench,
            items,
            model,
            settings,
            Path::new(super::item_checkpoint::REASONING_CHECKPOINT_ROOT),
            force_restart,
        )?
    };
    CACHE
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .push((key, report.clone()));
    Ok(report)
}

/// Score one benchmark against the adapter by running the existing
/// `vox mens eval-local` harness over `mens/data/<bench>` and reading back its
/// pass@1. Reusing that harness is deliberate: the post-training score must come
/// from the same rubric the operator's baseline was produced with, and it is
/// what performs the `MlBackend` plugin dispatch and loads the adapter — so an
/// adapter that cannot be loaded surfaces here as an `Err`, never as a score.
fn score_bench(
    bench: &str,
    base: &Path,
    adapter: &Path,
    settings: &vox_eval::decode_protocol::DecodeSettings,
    force: crate::commands::mens::item_checkpoint::ForceRestart,
    pins: &BTreeMap<String, super::eval_gate::SlicePin>,
) -> Result<BenchScore> {
    let (dir_name, want_think_rate) = match bench.strip_suffix("_init") {
        Some(base) => (base, true),
        None => (bench, false),
    };
    let reasoning = Path::new(BENCH_ROOT).join(dir_name).join("reasoning.jsonl");
    if reasoning.is_file() {
        let items_blake3 = pinned_slice_hash(dir_name, &reasoning, pins)?;
        // Base first: it is the side shared across adapters, so it is the one most
        // often already checkpointed. `--force-restart` is about this adapter and
        // never discards the shared base generations (base generations are discarded
        // only by `--force-restart-base`).
        let pre =
            cached_reasoning_report(dir_name, &reasoning, base, settings, true, force.for_base())?;
        let report = cached_reasoning_report(
            dir_name,
            &reasoning,
            adapter,
            settings,
            false,
            force.for_adapter(),
        )?;
        for r in [&pre, &report] {
            if r.n != SLICE_SIZE {
                anyhow::bail!(
                    "reasoning bench '{dir_name}': scored {} items, expected {SLICE_SIZE}",
                    r.n
                );
            }
        }
        let flags = |r: &super::reasoning_eval::ReasoningReport| -> Vec<ItemFlag> {
            r.items
                .iter()
                .map(|(id, correct, think)| {
                    (id.clone(), if want_think_rate { *think } else { *correct })
                })
                .collect()
        };
        let value = if want_think_rate {
            report.think_initiation_rate
        } else {
            report.accuracy
        };
        return Ok(BenchScore {
            value,
            n: Some(report.n),
            paired: Some((flags(&pre), flags(&report))),
            items_blake3: Some(items_blake3),
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
        force.for_adapter(),
    )
    .with_context(|| format!("scoring benchmark '{bench}' against {}", adapter.display()))?;

    let report: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out)?)?;
    let value = report["pass_rate_at_1"].as_f64().ok_or_else(|| {
        anyhow::anyhow!("eval-local report for '{bench}' has no numeric pass_rate_at_1")
    })?;
    Ok(BenchScore {
        value,
        n: None,
        paired: None,
        items_blake3: None,
    })
}

/// Fixtures shared with the serve-boundary tests in `populi::dispatch`.
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    pub(crate) const ALPHA: f64 = 0.05;

    /// A reasoning score over `n` items: base right on the first `base_ok`,
    /// adapter on the first `post_ok`, so `losses = base_ok - post_ok` when the
    /// adapter is worse and `gains = post_ok - base_ok` when it is better.
    pub(crate) fn paired_n(n: usize, base_ok: usize, post_ok: usize) -> BenchScore {
        let side = |ok: usize| -> Vec<ItemFlag> {
            (0..n).map(|i| (format!("item-{i:04}"), i < ok)).collect()
        };
        BenchScore {
            value: post_ok as f64 / n as f64,
            n: Some(n),
            paired: Some((side(base_ok), side(post_ok))),
            items_blake3: None,
        }
    }

    /// [`paired_n`] at the slice size.
    pub(crate) fn paired(base_ok: usize, post_ok: usize) -> BenchScore {
        paired_n(SLICE_SIZE, base_ok, post_ok)
    }

    /// Reasoning keys at 80/128 (0.625) on both sides (no change), other benches `other`.
    pub(crate) fn flat(bench: &str, other: f64) -> BenchScore {
        if reasoning_slice_of(bench).is_some() {
            paired(80, 80)
        } else {
            BenchScore {
                value: other,
                n: None,
                paired: None,
                items_blake3: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{ALPHA, flat, paired, paired_n};
    use super::*;

    /// The six reasoning keys at 80/128 (0.625) accuracy and think-initiation.
    const SIX: &str = r#""gsm8k_think":0.625,"gsm8k_think_init":0.625,"math500_think":0.625,"math500_think_init":0.625,"vox_think":0.625,"vox_think_init":0.625"#;

    fn generic(value: f64) -> BenchScore {
        BenchScore {
            value,
            n: None,
            paired: None,
            items_blake3: None,
        }
    }

    fn read_report(dir: &Path) -> serde_json::Value {
        serde_json::from_str(
            &std::fs::read_to_string(dir.join("collateral_damage_report.json")).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn writer_output_satisfies_shared_check() {
        let dir = tempfile::tempdir().unwrap();
        let pre = dir.path().join("pre.json");
        std::fs::write(&pre, format!(r#"{{"benchmarks":{{{SIX}}}}}"#)).unwrap();
        let mut scorer = |bench: &str| {
            let mut s = paired(80, 80);
            s.items_blake3 = Some(format!("h-{}", bench.trim_end_matches(THINK_INIT_SUFFIX)));
            Ok(s)
        };
        let code = run_collateral_damage_with(&pre, dir.path(), ALPHA, &mut scorer).unwrap();
        assert_eq!(code, 0, "writer must produce exit code 0 for passing runs");

        let report = read_report(dir.path());
        let pins_val = report.get("slice_blake3").unwrap().as_object().unwrap();
        let mut slices = std::collections::HashMap::new();
        for (k, v) in pins_val {
            slices.insert(k.clone(), v.as_str().unwrap().to_string());
        }
        let req = vox_eval::collateral_evidence::CollateralRequirement {
            slices,
            rule: vox_eval::reasoning_slice::PAIRED_RULE.to_string(),
            alpha: ALPHA,
            grader: vox_eval::qwen_reasoning::GRADER_ID.to_string(),
        };

        let res = vox_eval::collateral_evidence::check_collateral_report(&report, &req);
        assert!(
            res.is_ok(),
            "writer output must pass shared check: {:?}",
            res.err()
        );
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
                "code_bench" => generic(0.40),
                _ => flat(bench, 0.84),
            })
        };
        let code = run_collateral_damage_with(&pre, dir.path(), ALPHA, &mut scorer).unwrap();

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
        assert!(run_collateral_damage_with(&pre, dir.path(), ALPHA, &mut scorer).is_err());
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
        let code = run_collateral_damage_with(&pre, dir.path(), ALPHA, &mut scorer).unwrap();
        assert_eq!(code, 0);
        let report = read_report(dir.path());
        assert_eq!(report["status"], "pass");
        assert_labelled(&report);
        let reports = report["reports"].as_array().unwrap();
        assert_eq!(reports.len(), 7, "one generic plus two per slice");
        for r in reports {
            if reasoning_slice_of(r["benchmark"].as_str().unwrap()).is_some() {
                assert_eq!(r["slice"], true);
                assert_eq!(r["n"], SLICE_SIZE);
                assert_eq!(r["losses"], 0);
            } else {
                assert!(r.get("slice").is_none() && r.get("n").is_none());
            }
        }
        vox_eval::reasoning_slice::verify_reasoning_slices(&report, ALPHA).unwrap();
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
        let mut scorer = |_: &str| Ok(generic(1.0));
        let err = run_collateral_damage_with(&pre, dir.path(), ALPHA, &mut scorer)
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
        // score. Without this guard, the loop below never runs and a vacuous
        // "pass" report gets written having loaded the adapter zero times.
        let dir = tempfile::tempdir().unwrap();
        let pre = dir.path().join("pre.json");
        std::fs::write(&pre, r#"{"benchmarks":{}}"#).unwrap();
        let mut scorer = |_: &str| Ok(generic(1.0));
        let err = run_collateral_damage_with(&pre, dir.path(), ALPHA, &mut scorer)
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
                .map(|(_, s)| s.clone())
                .unwrap_or_else(|| flat(bench, 0.84)))
        }
    }

    fn no_report(dir: &Path) {
        assert!(
            !dir.join("collateral_damage_report.json").exists(),
            "a refused run must leave no report"
        );
    }

    fn full_check(pre: &Path, dir: &Path, over: &[(&str, BenchScore)]) -> Result<i32> {
        let mut calls = 0;
        let mut scorer = scorer_over(over, &mut calls);
        run_collateral_damage_with(pre, dir, ALPHA, &mut scorer)
    }

    #[test]
    fn a_baseline_missing_a_think_initiation_key_is_a_hard_error() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &["gsm8k_think_init"], &[]);
        let mut calls = 0;
        let err = {
            let mut scorer = scorer_over(&[], &mut calls);
            run_collateral_damage_with(&pre, dir.path(), ALPHA, &mut scorer)
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
        let err = full_check(&pre, dir.path(), &[]).unwrap_err().to_string();
        assert!(err.contains("vox_think, vox_think_init"), "{err}");
        no_report(dir.path());
    }

    /// Phase 335 root cause 2: the baseline float came from another run (another
    /// grader or token cap) than the per-item base the pairing uses.
    #[test]
    fn a_baseline_that_disagrees_with_the_re_scored_base_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[("math500_think", 0.5)]);
        let err = full_check(&pre, dir.path(), &[]).unwrap_err().to_string();
        assert!(
            err.contains("math500_think") && err.contains("regenerate the baseline"),
            "{err}"
        );
        no_report(dir.path());
    }

    #[test]
    fn a_non_finite_reasoning_score_refuses_and_writes_no_report() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let mut nan = paired(80, 80);
        nan.value = f64::NAN;
        let err = full_check(&pre, dir.path(), &[("vox_think_init", nan)])
            .unwrap_err()
            .to_string();
        assert!(err.contains("disagrees"), "{err}");
        no_report(dir.path());
    }

    #[test]
    fn a_reasoning_score_without_an_item_count_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let mut no_n = paired(80, 80);
        no_n.n = None;
        assert!(full_check(&pre, dir.path(), &[("gsm8k_think", no_n)]).is_err());
        no_report(dir.path());
    }

    #[test]
    fn a_reasoning_score_without_per_item_outcomes_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let mut bare = paired(80, 80);
        bare.paired = None;
        let err = full_check(&pre, dir.path(), &[("gsm8k_think", bare)])
            .unwrap_err()
            .to_string();
        assert!(err.contains("per-item"), "{err}");
        no_report(dir.path());
    }

    #[test]
    fn a_reasoning_score_one_item_short_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let over = [
            ("gsm8k_think", paired_n(SLICE_SIZE - 1, 80, 80)),
            ("gsm8k_think_init", paired_n(SLICE_SIZE - 1, 80, 80)),
        ];
        let err = full_check(&pre, dir.path(), &over).unwrap_err().to_string();
        assert!(err.contains(&format!("expected {SLICE_SIZE}")), "{err}");
        no_report(dir.path());
    }

    #[test]
    fn mismatched_item_counts_between_a_slice_and_its_init_key_refuse() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let over = [("math500_think_init", paired_n(SLICE_SIZE - 1, 80, 80))];
        assert!(full_check(&pre, dir.path(), &over).is_err());
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
        assert!(run_collateral_damage_with(&pre, dir.path(), ALPHA, &mut scorer).is_err());
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
        assert!(full_check(&pre, dir.path(), &[]).is_err());
        no_report(dir.path());
    }

    fn assert_labelled(report: &serde_json::Value) {
        let block = &report["reasoning_slices"];
        let note = block["scope_note"].as_str().unwrap();
        assert!(
            note.contains(&format!("{SLICE_SIZE}-item")) && note.contains("not full"),
            "{note}"
        );
        assert_eq!(block["slice_size"], SLICE_SIZE);
        assert_eq!(block["alpha"], ALPHA);
        assert_eq!(block["rule"], vox_eval::reasoning_slice::PAIRED_RULE);
        for (name, bench) in block["benches"].as_object().unwrap() {
            assert_eq!(bench["accuracy"]["n"], SLICE_SIZE, "{name}");
        }
    }

    /// Phase 335 root cause 1: one lost item of 32 failed the old 5% relative
    /// bar. Under the paired test one lost item is not evidence of a regression.
    #[test]
    fn one_lost_item_on_a_slice_passes() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[("gsm8k_think", 0.5)]);
        let code = full_check(&pre, dir.path(), &[("gsm8k_think", paired(64, 63))]).unwrap();
        assert_eq!(code, 0);
        let report = read_report(dir.path());
        assert_eq!(report["status"], "pass");
        let acc = &report["reasoning_slices"]["benches"]["gsm8k_think"]["accuracy"];
        assert_eq!(
            (acc["losses"].as_u64(), acc["gains"].as_u64()),
            (Some(1), Some(0))
        );
    }

    #[test]
    fn an_accuracy_regression_fails_and_names_the_slice() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let code = full_check(&pre, dir.path(), &[("gsm8k_think", paired(80, 75))]).unwrap();
        assert_eq!(code, 1);
        let report = read_report(dir.path());
        assert_eq!(report["status"], "fail");
        assert_eq!(report["failed_on"], "gsm8k_think");
        assert_labelled(&report);
        let reports = report["reports"].as_array().unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0]["slice"], true);
        assert_eq!(reports[0]["n"], SLICE_SIZE);
        assert_eq!(reports[0]["losses"], 5);
        assert_eq!(reports[0]["regressed"], true);
    }

    #[test]
    fn a_think_initiation_regression_fails_and_names_the_init_key() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[("vox_think_init", 0.5)]);
        let code = full_check(&pre, dir.path(), &[("vox_think_init", paired(64, 58))]).unwrap();
        assert_eq!(code, 1);
        let report = read_report(dir.path());
        assert_eq!(report["status"], "fail");
        assert_eq!(report["failed_on"], "vox_think_init");
        assert_labelled(&report);
    }

    #[test]
    fn a_small_loss_on_every_slice_fails_on_the_pooled_test() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let over = [
            ("gsm8k_think", paired(80, 78)),
            ("math500_think", paired(80, 78)),
            ("vox_think", paired(80, 78)),
        ];
        let code = full_check(&pre, dir.path(), &over).unwrap();
        assert_eq!(
            code, 1,
            "2 losses per slice pass alone; 6-0 pooled does not"
        );
        let report = read_report(dir.path());
        assert_eq!(report["failed_on"], vox_eval::reasoning_slice::POOLED_KEY);
        assert_eq!(report["reasoning_slices"]["pooled"]["losses"], 6);
        assert_eq!(report["reasoning_slices"]["pooled"]["regressed"], true);
    }

    #[test]
    fn a_non_reasoning_bench_keeps_one_score_and_no_slice_field() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[("general_bench", 0.85)]);
        assert_eq!(full_check(&pre, dir.path(), &[]).unwrap(), 0);
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
        // Starts with vox_think for the historical "first failure" fixture.
        ["vox_think", "gsm8k_think", "math500_think"]
            .map(String::from)
            .to_vec()
    }

    fn fail_fast(
        pre: &Path,
        dir: &Path,
        order: &[String],
        scorer: &mut dyn FnMut(&str) -> Result<BenchScore>,
    ) -> Result<i32> {
        run_collateral_damage_fail_fast_with(pre, dir, order, ALPHA, scorer)
    }

    #[test]
    fn default_fail_fast_order_is_gsm8k_first() {
        assert_eq!(
            default_fail_fast_order(),
            ["gsm8k_think", "math500_think", "vox_think"]
        );
        validate_fail_fast_order(&default_fail_fast_order()).unwrap();
    }

    #[test]
    fn fail_fast_stops_at_the_first_failing_slice_and_marks_the_rest_not_run() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let mut asked: Vec<String> = Vec::new();
        let mut scorer = |bench: &str| {
            asked.push(bench.to_string());
            Ok(match bench {
                // gsm8k also regresses: it must never be scored, so it cannot be the verdict.
                "vox_think" | "gsm8k_think" => paired(80, 73),
                _ => flat(bench, 0.84),
            })
        };
        let code = fail_fast(&pre, dir.path(), &fail_fast_order(), &mut scorer).unwrap();
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
        assert!(
            report["reasoning_slices"]["pooled"].is_null(),
            "no pooled on a partial run"
        );
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
        let code = fail_fast(&pre, dir.path(), &fail_fast_order(), &mut scorer).unwrap();
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
        vox_eval::reasoning_slice::verify_reasoning_slices(&report, ALPHA).unwrap();
    }

    #[test]
    fn fail_fast_full_coverage_fails_on_a_pooled_regression() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let mut scorer = |bench: &str| {
            Ok(match bench {
                "gsm8k_think" | "math500_think" | "vox_think" => paired(80, 78),
                _ => flat(bench, 0.84),
            })
        };
        let code = fail_fast(&pre, dir.path(), &default_fail_fast_order(), &mut scorer).unwrap();
        assert_eq!(code, 1);
        let report = read_report(dir.path());
        assert_eq!(report["status"], "fail");
        assert_eq!(report["failed_on"], vox_eval::reasoning_slice::POOLED_KEY);
    }

    #[test]
    fn fail_fast_subset_order_that_all_pass_is_incomplete_not_hub_pass() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let mut asked: Vec<String> = Vec::new();
        let mut scorer = |bench: &str| {
            asked.push(bench.to_string());
            Ok(flat(bench, 0.84))
        };
        let order = ["gsm8k_think"].map(String::from).to_vec();
        let code = fail_fast(&pre, dir.path(), &order, &mut scorer).unwrap();
        assert_eq!(code, 1, "subset pass is incomplete, exit 1");
        assert_eq!(asked, ["gsm8k_think", "gsm8k_think_init"]);
        let report = read_report(dir.path());
        assert_eq!(report["status"], "incomplete");
        assert_eq!(report["failed_on"], "incomplete_slice_coverage");
        let not_run = report["not_run"].as_array().unwrap();
        for name in ["math500_think", "vox_think"] {
            assert!(
                not_run
                    .iter()
                    .any(|e| e["benchmark"] == name && e["reason"] == "not_in_fail_fast_order"),
                "{name}"
            );
        }
    }

    #[test]
    fn fail_fast_subset_stops_early_on_fail_without_scoring_later_slices() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        let mut asked: Vec<String> = Vec::new();
        let mut scorer = |bench: &str| {
            asked.push(bench.to_string());
            Ok(match bench {
                "gsm8k_think" => paired(80, 73),
                _ => flat(bench, 0.84),
            })
        };
        let code = fail_fast(&pre, dir.path(), &default_fail_fast_order(), &mut scorer).unwrap();
        assert_eq!(code, 1);
        assert_eq!(asked, ["gsm8k_think", "gsm8k_think_init"]);
        let report = read_report(dir.path());
        assert_eq!(report["status"], "fail");
        assert_eq!(report["failed_on"], "gsm8k_think");
        assert_eq!(report["fail_fast"], true);
    }

    #[test]
    fn fail_fast_refuses_unknown_or_duplicate_slice_names() {
        assert!(validate_fail_fast_order(&["nope".into()]).is_err());
        assert!(validate_fail_fast_order(&[]).is_err());
        assert!(validate_fail_fast_order(&["gsm8k_think".into(), "gsm8k_think".into()]).is_err());
    }

    const PARTIAL_DROP: [&str; 4] = [
        "gsm8k_think",
        "gsm8k_think_init",
        "math500_think",
        "math500_think_init",
    ];

    #[test]
    fn fail_fast_accepts_a_partial_baseline_when_its_slice_fails() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &PARTIAL_DROP, &[]);
        let over = [("vox_think", paired(80, 73))];
        let mut calls = 0;
        let mut scorer = scorer_over(&over, &mut calls);
        let code = fail_fast(&pre, dir.path(), &fail_fast_order(), &mut scorer).unwrap();
        assert_eq!(code, 1);
        let report = read_report(dir.path());
        assert_eq!(report["failed_on"], "vox_think");
        assert_eq!(report["not_run"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn fail_fast_refuses_to_reach_a_slice_the_baseline_does_not_name() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &PARTIAL_DROP, &[]);
        let mut asked: Vec<String> = Vec::new();
        let mut scorer = |bench: &str| {
            asked.push(bench.to_string());
            Ok(flat(bench, 0.84))
        };
        let err = fail_fast(&pre, dir.path(), &fail_fast_order(), &mut scorer)
            .expect_err("gsm8k_think has no baseline");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("gsm8k_think") && msg.contains("baseline"),
            "{msg}"
        );
        assert_eq!(
            asked,
            ["vox_think", "vox_think_init"],
            "gsm8k_think is never scored"
        );
        no_report(dir.path());
    }

    #[test]
    fn the_full_check_still_refuses_a_partial_baseline() {
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &PARTIAL_DROP, &[]);
        let mut calls = 0;
        {
            let mut scorer = scorer_over(&[], &mut calls);
            assert!(run_collateral_damage_with(&pre, dir.path(), ALPHA, &mut scorer).is_err());
        }
        assert_eq!(calls, 0);
    }

    #[test]
    fn the_ladder_policy_states_alpha_and_a_policy_without_it_is_refused() {
        let ladder = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(LADDER_POLICY);
        assert_eq!(policy_alpha(&ladder).unwrap(), 0.05);
        let dir = tempfile::tempdir().unwrap();
        for (body, needle) in [
            (
                "version: \"1\"\ncollateral_damage:\n  block: true\n",
                "sets no",
            ),
            (
                "version: \"1\"\ncollateral_damage:\n  block: true\n  alpha: 1.5\n",
                "not in (0, 1)",
            ),
        ] {
            let p = dir.path().join("p.yaml");
            std::fs::write(&p, body).unwrap();
            let e = format!("{:#}", policy_alpha(&p).unwrap_err());
            assert!(e.contains(needle), "{e}");
        }
    }

    #[test]
    fn the_base_side_is_the_snapshot_the_adapter_names() {
        let snap = tempfile::tempdir().unwrap();
        let adapter = tempfile::tempdir().unwrap();
        let manifest = adapter.path().join("adapter_manifest.json");
        assert!(base_snapshot_of(adapter.path()).is_err(), "no manifest");
        std::fs::write(&manifest, r#"{"rank":8}"#).unwrap();
        assert!(
            format!("{:#}", base_snapshot_of(adapter.path()).unwrap_err())
                .contains("no base_model")
        );
        std::fs::write(
            &manifest,
            json!({"base_model": "/no/such/snapshot"}).to_string(),
        )
        .unwrap();
        assert!(base_snapshot_of(adapter.path()).is_err(), "not a dir");
        std::fs::write(&manifest, json!({"base_model": snap.path()}).to_string()).unwrap();
        assert_eq!(base_snapshot_of(adapter.path()).unwrap(), snap.path());
    }

    /// A scorer whose reasoning keys carry the item set `h-<slice>` they were scored on.
    fn hashed(bench: &str) -> Result<BenchScore> {
        let mut s = flat(bench, 0.84);
        if reasoning_slice_of(bench).is_some() {
            s.items_blake3 = Some(format!("h-{}", bench.trim_end_matches(THINK_INIT_SUFFIX)));
        }
        Ok(s)
    }

    #[test]
    fn a_report_records_the_item_set_each_slice_was_scored_on() {
        let want: serde_json::Value = SLICE_NAMES
            .iter()
            .map(|s| (s.to_string(), json!(format!("h-{s}"))))
            .collect::<serde_json::Map<_, _>>()
            .into();
        let dir = tempfile::tempdir().unwrap();
        let pre = write_baseline(dir.path(), &[], &[]);
        assert_eq!(
            run_collateral_damage_with(&pre, dir.path(), ALPHA, &mut hashed).unwrap(),
            0
        );
        assert_eq!(read_report(dir.path())["slice_blake3"], want);
        // ...and the grader that judged the generations, which the checkpoint key
        // deliberately does not cover (a checkpoint stores generations only).
        assert_eq!(
            read_report(dir.path())["grader"],
            vox_eval::qwen_reasoning::GRADER_ID
        );
        let order = default_fail_fast_order();
        assert_eq!(fail_fast(&pre, dir.path(), &order, &mut hashed).unwrap(), 0);
        assert_eq!(read_report(dir.path())["slice_blake3"], want);
        assert_eq!(
            read_report(dir.path())["grader"],
            vox_eval::qwen_reasoning::GRADER_ID
        );
    }

    #[test]
    fn an_unpinned_or_changed_slice_file_is_refused_before_scoring() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("reasoning.jsonl");
        std::fs::write(&path, "{\"id\":\"a\"}\n{\"id\":\"b\"}\n").unwrap();
        let none = BTreeMap::new();
        let e = format!("{:#}", pinned_slice_hash("s", &path, &none).unwrap_err());
        assert!(
            e.contains("pins no item set for reasoning slice 's'"),
            "{e}"
        );
        let hash = |b: &[u8]| vox_crypto::hex_encode(&vox_crypto::secure_hash(b));
        let pin = super::super::eval_gate::SlicePin {
            blake3: hash(b"{\"id\":\"a\"}\n{\"id\":\"b\"}\n"),
            prefix_n: 1,
            prefix_blake3: hash(b"{\"id\":\"a\"}\n"),
        };
        let pins: BTreeMap<_, _> = [("s".to_string(), pin)].into_iter().collect();
        assert_eq!(
            pinned_slice_hash("s", &path, &pins).unwrap(),
            hash(b"{\"id\":\"a\"}\n{\"id\":\"b\"}\n")
        );
        std::fs::write(&path, "{\"id\":\"a\"}\n{\"id\":\"c\"}\n").unwrap();
        let e = format!("{:#}", pinned_slice_hash("s", &path, &pins).unwrap_err());
        assert!(e.contains("refusing to score an unpinned item set"), "{e}");
    }

    #[test]
    fn a_baseline_from_another_protocol_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        // Baseline has no decode object -> defaults to f32.
        let pre = write_baseline(dir.path(), &[], &[]);
        let bf16_settings = vox_eval::decode_protocol::DecodeSettings {
            protocol: vox_eval::decode_protocol::DecodeProtocol::Bf16 { gemm_rows: 32 },
            max_tokens: 2048,
            temperature: 0.0,
            seed: 42,
            thinking: true,
        };
        let mut calls = 0;
        let mut scorer = scorer_over(&[], &mut calls);
        let err = run_collateral_damage_with_settings(
            &pre,
            dir.path(),
            ALPHA,
            &bf16_settings,
            &mut scorer,
        )
        .unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("baseline")
                && msg.contains("was decoded under f32; this run uses bf16")
                && msg.contains("VOX_MENS_DECODE_PROTOCOL=bf16"),
            "unexpected error message: {msg}"
        );
        drop(scorer);
        assert_eq!(
            calls, 0,
            "scorer must not be called when baseline is refused"
        );
        no_report(dir.path());
    }

    #[test]
    fn an_f32_run_with_baseline_without_decode_proceeds() {
        let dir = tempfile::tempdir().unwrap();
        // Baseline has no decode object -> defaults to f32.
        let pre = write_baseline(dir.path(), &[], &[]);
        let f32_settings = vox_eval::decode_protocol::DecodeSettings {
            protocol: vox_eval::decode_protocol::DecodeProtocol::F32,
            max_tokens: 2048,
            temperature: 0.0,
            seed: 42,
            thinking: true,
        };
        let mut calls = 0;
        let mut scorer = scorer_over(&[], &mut calls);
        let code = run_collateral_damage_with_settings(
            &pre,
            dir.path(),
            ALPHA,
            &f32_settings,
            &mut scorer,
        )
        .unwrap();
        assert_eq!(code, 0);
        let report = read_report(dir.path());
        assert_eq!(report["status"], "pass");
        assert_eq!(report["decode"]["protocol"], "f32");
    }

    fn setup_test_collateral_env(
        dir: &Path,
    ) -> (PathBuf, PathBuf, PathBuf, PathBuf, BTreeMap<String, f64>) {
        let bench_root = dir.join("data");
        let base = dir.join("base");
        let adapter = dir.join("adapter");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&adapter).unwrap();
        let mut pre_map = BTreeMap::new();
        for name in ["gsm8k_think", "math500_think", "vox_think"] {
            let bdir = bench_root.join(name);
            std::fs::create_dir_all(&bdir).unwrap();
            let items: Vec<serde_json::Value> = (0..SLICE_SIZE)
                .map(|i| {
                    serde_json::json!({
                        "id": format!("{name}_{i}"),
                        "question": format!("question_{name}_{i}"),
                        "answer": "42"
                    })
                })
                .collect();
            let jsonl = items
                .iter()
                .map(|it| it.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            std::fs::write(bdir.join("reasoning.jsonl"), jsonl).unwrap();

            pre_map.insert(name.to_string(), 1.0);
            pre_map.insert(format!("{name}{THINK_INIT_SUFFIX}"), 1.0);
        }
        let pre_path = dir.join("pre.json");
        std::fs::write(
            &pre_path,
            serde_json::json!({ "benchmarks": pre_map }).to_string(),
        )
        .unwrap();
        (bench_root, base, adapter, pre_path, pre_map)
    }

    fn fake_batch_infer_collateral<'a>(
        batch_requests: &'a std::sync::atomic::AtomicUsize,
        batch_cancels: &'a std::sync::atomic::AtomicUsize,
        prompts_received: &'a std::sync::Mutex<Vec<String>>,
        mut get_answer: impl FnMut(usize, &str) -> String + 'a,
    ) -> impl FnMut(&str) -> Result<String> + 'a {
        let queue = std::sync::Mutex::new(Vec::new());
        let current_prompts = std::sync::Mutex::new(Vec::new());
        move |p: &str| {
            let v: serde_json::Value = serde_json::from_str(p)?;
            if v.get("batch_cancel").is_some() {
                batch_cancels.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Ok(serde_json::json!({"cancelled": true}).to_string());
            }
            if let Some(arr) = v.get("batch").and_then(|b| b.as_array()) {
                batch_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let mut pr = prompts_received.lock().unwrap();
                let mut cp = current_prompts.lock().unwrap();
                cp.clear();
                for item in arr {
                    let prompt = item["prompt"].as_str().unwrap_or_default().to_string();
                    pr.push(prompt.clone());
                    cp.push(prompt);
                }
                let len = arr.len();
                let mut q = queue.lock().unwrap();
                q.clear();
                for i in 0..len {
                    q.push(i);
                }
                return Ok(serde_json::json!({
                    "batch_started": len,
                    "batch_size": len,
                })
                .to_string());
            }
            if v.get("batch_next").is_some() {
                let mut q = queue.lock().unwrap();
                if q.is_empty() {
                    return Ok(r#"{"done":true}"#.to_string());
                }
                let i = q.remove(0);
                let cp = current_prompts.lock().unwrap();
                let prompt = &cp[i];
                let ans = get_answer(i, prompt);
                return Ok(serde_json::json!({
                    "index": i,
                    "response": serde_json::json!({
                        "generated_text": format!("<think>\nthink\n</think>\n\n\\boxed{{{ans}}}")
                    })
                    .to_string()
                })
                .to_string());
            }
            Ok(serde_json::json!({
                "generated_text": "<think>\nthink\n</think>\n\n\\boxed{42}"
            })
            .to_string())
        }
    }

    #[test]
    fn collateral_decodes_one_stream_per_side() {
        let dir = tempfile::tempdir().unwrap();
        let ck_root = tempfile::tempdir().unwrap();
        let (bench_root, base, adapter, pre_path, pre_map) = setup_test_collateral_env(dir.path());
        let order: Vec<String> = vec![
            "gsm8k_think".into(),
            "math500_think".into(),
            "vox_think".into(),
        ];
        let settings = vox_eval::decode_protocol::DecodeSettings::default();

        let base_batch_requests = std::sync::atomic::AtomicUsize::new(0);
        let base_batch_cancels = std::sync::atomic::AtomicUsize::new(0);
        let base_prompts = std::sync::Mutex::new(Vec::new());
        let mut base_infer = fake_batch_infer_collateral(
            &base_batch_requests,
            &base_batch_cancels,
            &base_prompts,
            |_, _| "42".into(),
        );

        let adapter_batch_requests = std::sync::atomic::AtomicUsize::new(0);
        let adapter_batch_cancels = std::sync::atomic::AtomicUsize::new(0);
        let adapter_prompts = std::sync::Mutex::new(Vec::new());
        let mut adapter_infer = fake_batch_infer_collateral(
            &adapter_batch_requests,
            &adapter_batch_cancels,
            &adapter_prompts,
            |_, _| "42".into(),
        );

        prefill_collateral_with(
            ck_root.path(),
            &bench_root,
            &pre_path,
            &pre_map,
            &base,
            &adapter,
            &order,
            false, // fail_fast off
            ALPHA,
            &settings,
            crate::commands::mens::item_checkpoint::ForceRestart {
                adapter: false,
                base: false,
            },
            None,
            &mut base_infer,
            &mut adapter_infer,
        )
        .expect("prefill passes");

        assert_eq!(
            base_batch_requests.load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        assert_eq!(base_prompts.lock().unwrap().len(), 3 * SLICE_SIZE);
        assert_eq!(
            adapter_batch_requests.load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        assert_eq!(adapter_prompts.lock().unwrap().len(), 3 * SLICE_SIZE);

        // Build report over the checkpoints alone
        let mut calls = 0;
        let mut checkpoint_scorer = |bench: &str| -> Result<BenchScore> {
            calls += 1;
            let (dir_name, want_think) = match bench.strip_suffix("_init") {
                Some(b) => (b, true),
                None => (bench, false),
            };
            let items = super::load_slice_file(&bench_root.join(dir_name).join("reasoning.jsonl"))?;
            let mut refuse =
                |_: &str| -> Result<String> { anyhow::bail!("infer must not be called") };
            let (_, base_rows) = crate::commands::mens::reasoning_eval::score_slice_checkpointed(
                ck_root.path(),
                &base,
                dir_name,
                &items,
                &settings,
                false,
                &mut refuse,
            )?;
            let (adapter_outcome, adapter_rows) =
                crate::commands::mens::reasoning_eval::score_slice_checkpointed(
                    ck_root.path(),
                    &adapter,
                    dir_name,
                    &items,
                    &settings,
                    false,
                    &mut refuse,
                )?;
            let adapter_scored = match adapter_outcome {
                BenchOutcome::Scored(s) => s,
                _ => anyhow::bail!("not scored"),
            };
            let side_flags = |rows: &[(String, String, bool, bool)]| -> Vec<ItemFlag> {
                rows.iter()
                    .map(|(id, _, c, t)| (id.clone(), if want_think { *t } else { *c }))
                    .collect()
            };
            Ok(BenchScore {
                value: if want_think {
                    adapter_scored.think_initiation_rate
                } else {
                    adapter_scored.accuracy
                },
                n: Some(items.len()),
                paired: Some((side_flags(&base_rows), side_flags(&adapter_rows))),
                items_blake3: None,
            })
        };

        let code = run_collateral_damage_with_settings(
            &pre_path,
            &adapter,
            ALPHA,
            &settings,
            &mut checkpoint_scorer,
        )
        .expect("report from checkpoints passes");

        assert_eq!(code, 0);
        let report = read_report(&adapter);
        assert_eq!(report["status"], "pass");
    }

    #[test]
    fn a_failing_first_slice_cancels_the_adapter_stream() {
        let dir = tempfile::tempdir().unwrap();
        let ck_root = tempfile::tempdir().unwrap();
        let (bench_root, base, adapter, pre_path, pre_map) = setup_test_collateral_env(dir.path());
        let order: Vec<String> = vec![
            "gsm8k_think".into(),
            "math500_think".into(),
            "vox_think".into(),
        ];
        let settings = vox_eval::decode_protocol::DecodeSettings::default();

        let base_batch_requests = std::sync::atomic::AtomicUsize::new(0);
        let base_batch_cancels = std::sync::atomic::AtomicUsize::new(0);
        let base_prompts = std::sync::Mutex::new(Vec::new());
        let mut base_infer = fake_batch_infer_collateral(
            &base_batch_requests,
            &base_batch_cancels,
            &base_prompts,
            |_, _| "42".into(),
        );

        let adapter_batch_requests = std::sync::atomic::AtomicUsize::new(0);
        let adapter_batch_cancels = std::sync::atomic::AtomicUsize::new(0);
        let adapter_prompts = std::sync::Mutex::new(Vec::new());
        // Adapter answers wrong ("0") on the first slice ("gsm8k_think")
        let mut adapter_infer = fake_batch_infer_collateral(
            &adapter_batch_requests,
            &adapter_batch_cancels,
            &adapter_prompts,
            |_, prompt| {
                if prompt.contains("question_gsm8k_think") {
                    "0".into()
                } else {
                    "42".into()
                }
            },
        );

        prefill_collateral_with(
            ck_root.path(),
            &bench_root,
            &pre_path,
            &pre_map,
            &base,
            &adapter,
            &order,
            true, // fail_fast on
            ALPHA,
            &settings,
            crate::commands::mens::item_checkpoint::ForceRestart {
                adapter: false,
                base: false,
            },
            None,
            &mut base_infer,
            &mut adapter_infer,
        )
        .expect("prefill passes");

        assert_eq!(
            adapter_batch_cancels.load(std::sync::atomic::Ordering::Relaxed),
            1,
            "adapter stream was cancelled on failing first slice"
        );

        // Report built from checkpoints alone
        let mut checkpoint_scorer = |bench: &str| -> Result<BenchScore> {
            let (dir_name, want_think) = match bench.strip_suffix("_init") {
                Some(b) => (b, true),
                None => (bench, false),
            };
            let items = super::load_slice_file(&bench_root.join(dir_name).join("reasoning.jsonl"))?;
            let mut refuse =
                |_: &str| -> Result<String> { anyhow::bail!("infer must not be called") };
            let (_, base_rows) = crate::commands::mens::reasoning_eval::score_slice_checkpointed(
                ck_root.path(),
                &base,
                dir_name,
                &items,
                &settings,
                false,
                &mut refuse,
            )?;
            let (adapter_outcome, adapter_rows) =
                crate::commands::mens::reasoning_eval::score_slice_checkpointed(
                    ck_root.path(),
                    &adapter,
                    dir_name,
                    &items,
                    &settings,
                    false,
                    &mut refuse,
                )?;
            let adapter_scored = match adapter_outcome {
                BenchOutcome::Scored(s) => s,
                _ => anyhow::bail!("not scored"),
            };
            let side_flags = |rows: &[(String, String, bool, bool)]| -> Vec<ItemFlag> {
                rows.iter()
                    .map(|(id, _, c, t)| (id.clone(), if want_think { *t } else { *c }))
                    .collect()
            };
            Ok(BenchScore {
                value: if want_think {
                    adapter_scored.think_initiation_rate
                } else {
                    adapter_scored.accuracy
                },
                n: Some(items.len()),
                paired: Some((side_flags(&base_rows), side_flags(&adapter_rows))),
                items_blake3: None,
            })
        };

        let code = run_collateral_damage_fail_fast_with_settings(
            &pre_path,
            &adapter,
            &order,
            ALPHA,
            &settings,
            &mut checkpoint_scorer,
        )
        .expect("fail-fast report produced");

        assert_eq!(code, 1);
        let report = read_report(&adapter);
        assert_eq!(report["status"], "fail");
        assert_eq!(report["failed_on"], "gsm8k_think");
        let benches = report["reasoning_slices"]["benches"].as_object().unwrap();
        assert_eq!(benches.keys().collect::<Vec<_>>(), ["gsm8k_think"]);
        assert_eq!(
            report["reasoning_slices"]["benches"]["gsm8k_think"]["accuracy"]["regressed"],
            true
        );
        let not_run = report["not_run"].as_array().unwrap();
        assert_eq!(not_run.len(), 2);
        assert_eq!(not_run[0]["benchmark"], "math500_think");
        assert_eq!(not_run[0]["status"], "not_run");
        assert_eq!(not_run[1]["benchmark"], "vox_think");
        assert_eq!(not_run[1]["status"], "not_run");
    }

    #[test]
    fn a_restart_discards_once() {
        let dir = tempfile::tempdir().unwrap();
        let ck_root = tempfile::tempdir().unwrap();
        let (bench_root, base, adapter, pre_path, pre_map) = setup_test_collateral_env(dir.path());
        let order: Vec<String> = vec![
            "gsm8k_think".into(),
            "math500_think".into(),
            "vox_think".into(),
        ];
        let settings = vox_eval::decode_protocol::DecodeSettings::default();

        let base_batch_requests = std::sync::atomic::AtomicUsize::new(0);
        let base_batch_cancels = std::sync::atomic::AtomicUsize::new(0);
        let base_prompts = std::sync::Mutex::new(Vec::new());
        let mut base_infer = fake_batch_infer_collateral(
            &base_batch_requests,
            &base_batch_cancels,
            &base_prompts,
            |_, _| "42".into(),
        );

        let adapter_batch_requests = std::sync::atomic::AtomicUsize::new(0);
        let adapter_batch_cancels = std::sync::atomic::AtomicUsize::new(0);
        let adapter_prompts = std::sync::Mutex::new(Vec::new());
        let mut adapter_infer = fake_batch_infer_collateral(
            &adapter_batch_requests,
            &adapter_batch_cancels,
            &adapter_prompts,
            |_, _| "42".into(),
        );

        // Populate initial checkpoints
        prefill_collateral_with(
            ck_root.path(),
            &bench_root,
            &pre_path,
            &pre_map,
            &base,
            &adapter,
            &order,
            false,
            ALPHA,
            &settings,
            crate::commands::mens::item_checkpoint::ForceRestart {
                adapter: false,
                base: false,
            },
            None,
            &mut base_infer,
            &mut adapter_infer,
        )
        .expect("initial prefill passes");

        assert_eq!(
            adapter_batch_requests.load(std::sync::atomic::Ordering::Relaxed),
            1
        );

        // Run with ForceRestart { adapter: true, base: false }:
        // Adapter checkpoint is discarded once during prefill.
        // Base checkpoint is NOT discarded.
        let mut restart_base_infer = fake_batch_infer_collateral(
            &base_batch_requests,
            &base_batch_cancels,
            &base_prompts,
            |_, _| "42".into(),
        );
        let mut restart_adapter_infer = fake_batch_infer_collateral(
            &adapter_batch_requests,
            &adapter_batch_cancels,
            &adapter_prompts,
            |_, _| "42".into(),
        );

        prefill_collateral_with(
            ck_root.path(),
            &bench_root,
            &pre_path,
            &pre_map,
            &base,
            &adapter,
            &order,
            false,
            ALPHA,
            &settings,
            crate::commands::mens::item_checkpoint::ForceRestart {
                adapter: true,
                base: false,
            },
            None,
            &mut restart_base_infer,
            &mut restart_adapter_infer,
        )
        .expect("restart prefill passes");

        // Base requests did not increment (it was not restarted)
        assert_eq!(
            base_batch_requests.load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        // Adapter requests incremented from 1 to 2 (restarted once)
        assert_eq!(
            adapter_batch_requests.load(std::sync::atomic::Ordering::Relaxed),
            2
        );

        // Now run the scoring pass with ForceRestart { adapter: false, base: false } (or refuse infer):
        // It must find all 9 records in checkpoint without discarding or generating anything.
        let mut refuse =
            |_: &str| -> Result<String> { anyhow::bail!("scoring pass must not generate") };
        for name in &order {
            let items = super::load_slice_file(&bench_root.join(name).join("reasoning.jsonl"))
                .expect("slice loads");
            let (out, rows) = crate::commands::mens::reasoning_eval::score_slice_checkpointed(
                ck_root.path(),
                &adapter,
                name,
                &items,
                &settings,
                false, // force_restart = false: scoring pass keeps every record
                &mut refuse,
            )
            .expect("reads from checkpoint without generating");
            assert_eq!(rows.len(), SLICE_SIZE);
            match out {
                BenchOutcome::Scored(s) => assert_eq!(s.n, SLICE_SIZE),
                _ => panic!("expected scored"),
            }
        }
    }

    #[test]
    fn a_crossed_boundary_cancels_the_stream() {
        let dir = tempfile::tempdir().unwrap();
        let ck_root = tempfile::tempdir().unwrap();
        let (bench_root, base, adapter, pre_path, pre_map) = setup_test_collateral_env(dir.path());
        let order: Vec<String> = vec![
            "gsm8k_think".into(),
            "math500_think".into(),
            "vox_think".into(),
        ];
        let settings = vox_eval::decode_protocol::DecodeSettings::default();

        let base_batch_requests = std::sync::atomic::AtomicUsize::new(0);
        let base_batch_cancels = std::sync::atomic::AtomicUsize::new(0);
        let base_prompts = std::sync::Mutex::new(Vec::new());
        let mut base_infer = fake_batch_infer_collateral(
            &base_batch_requests,
            &base_batch_cancels,
            &base_prompts,
            |_, _| "42".into(),
        );

        let adapter_batch_requests = std::sync::atomic::AtomicUsize::new(0);
        let adapter_batch_cancels = std::sync::atomic::AtomicUsize::new(0);
        let adapter_prompts = std::sync::Mutex::new(Vec::new());
        // Adapter wrong on 11 of the first 32 items of gsm8k_think (0..=10), right on rest
        let mut adapter_infer = fake_batch_infer_collateral(
            &adapter_batch_requests,
            &adapter_batch_cancels,
            &adapter_prompts,
            |_, prompt| {
                if prompt.contains("question_gsm8k_think_") {
                    let id_str = prompt
                        .split("question_gsm8k_think_")
                        .nth(1)
                        .and_then(|s| s.split(|c: char| !c.is_ascii_digit()).next())
                        .unwrap_or("");
                    if let Ok(idx) = id_str.parse::<usize>() {
                        if idx < 11 {
                            return "0".into();
                        }
                    }
                }
                "42".into()
            },
        );

        // Committed sequential schedule from ladder policy
        let rule = vox_eval::sequential_mcnemar::SequentialRule::new(
            vec![32, 64, 96, 128],
            vec![0.0005, 0.002, 0.005, 0.04795],
            ALPHA,
        )
        .unwrap();

        prefill_collateral_with(
            ck_root.path(),
            &bench_root,
            &pre_path,
            &pre_map,
            &base,
            &adapter,
            &order,
            true, // fail_fast on
            ALPHA,
            &settings,
            crate::commands::mens::item_checkpoint::ForceRestart {
                adapter: false,
                base: false,
            },
            Some(&rule),
            &mut base_infer,
            &mut adapter_infer,
        )
        .expect("prefill completes with early cancel");

        assert_eq!(
            adapter_batch_cancels.load(std::sync::atomic::Ordering::Relaxed),
            1,
            "adapter stream received batch_cancel at look 1"
        );

        let code = run_collateral_damage_sequential(
            ck_root.path(),
            &bench_root,
            &pre_path,
            &adapter,
            &base,
            &order,
            &rule,
            &settings,
        )
        .expect("report generated");

        assert_eq!(code, 1);
        let report = read_report(&adapter);
        assert_eq!(report["status"], "fail");
        assert_eq!(report["failed_on"], "gsm8k_think");
        assert_eq!(
            report["reasoning_slices"]["rule"],
            vox_eval::sequential_mcnemar::SEQUENTIAL_RULE
        );
        let gsm8k = &report["reasoning_slices"]["benches"]["gsm8k_think"];
        assert_eq!(gsm8k["stopped_at_look"], 1);
        assert_eq!(gsm8k["items_used"], 32);

        let not_run = report["not_run"].as_array().unwrap();
        assert!(not_run.iter().any(|n| n["benchmark"] == "math500_think"));
        assert!(not_run.iter().any(|n| n["benchmark"] == "vox_think"));
    }

    #[test]
    fn replay_equals_live() {
        let dir = tempfile::tempdir().unwrap();
        let ck_root = tempfile::tempdir().unwrap();
        let (bench_root, base, adapter, pre_path, pre_map) = setup_test_collateral_env(dir.path());
        let order: Vec<String> = vec![
            "gsm8k_think".into(),
            "math500_think".into(),
            "vox_think".into(),
        ];
        let settings = vox_eval::decode_protocol::DecodeSettings::default();

        let base_batch_requests = std::sync::atomic::AtomicUsize::new(0);
        let base_batch_cancels = std::sync::atomic::AtomicUsize::new(0);
        let base_prompts = std::sync::Mutex::new(Vec::new());
        let mut base_infer = fake_batch_infer_collateral(
            &base_batch_requests,
            &base_batch_cancels,
            &base_prompts,
            |_, _| "42".into(),
        );

        let adapter_batch_requests = std::sync::atomic::AtomicUsize::new(0);
        let adapter_batch_cancels = std::sync::atomic::AtomicUsize::new(0);
        let adapter_prompts = std::sync::Mutex::new(Vec::new());
        let mut adapter_infer = fake_batch_infer_collateral(
            &adapter_batch_requests,
            &adapter_batch_cancels,
            &adapter_prompts,
            |_, prompt| {
                if prompt.contains("question_gsm8k_think_") {
                    let id_str = prompt
                        .split("question_gsm8k_think_")
                        .nth(1)
                        .and_then(|s| s.split(|c: char| !c.is_ascii_digit()).next())
                        .unwrap_or("");
                    if let Ok(idx) = id_str.parse::<usize>() {
                        if idx < 11 {
                            return "0".into();
                        }
                    }
                }
                "42".into()
            },
        );

        let rule = vox_eval::sequential_mcnemar::SequentialRule::new(
            vec![32, 64, 96, 128],
            vec![0.0005, 0.002, 0.005, 0.04795],
            ALPHA,
        )
        .unwrap();

        prefill_collateral_with(
            ck_root.path(),
            &bench_root,
            &pre_path,
            &pre_map,
            &base,
            &adapter,
            &order,
            true,
            ALPHA,
            &settings,
            crate::commands::mens::item_checkpoint::ForceRestart {
                adapter: false,
                base: false,
            },
            Some(&rule),
            &mut base_infer,
            &mut adapter_infer,
        )
        .expect("live prefill passes");

        run_collateral_damage_sequential(
            ck_root.path(),
            &bench_root,
            &pre_path,
            &adapter,
            &base,
            &order,
            &rule,
            &settings,
        )
        .expect("live report generated");

        let live_report = read_report(&adapter);

        // Replay: generate report again from checkpoints only (zero model infer calls)
        run_collateral_damage_sequential(
            ck_root.path(),
            &bench_root,
            &pre_path,
            &adapter,
            &base,
            &order,
            &rule,
            &settings,
        )
        .expect("replay report generated");

        let replay_report = read_report(&adapter);
        assert_eq!(
            live_report["reasoning_slices"],
            replay_report["reasoning_slices"]
        );

        // Verification of report with verify_reasoning_slices checks against stopped look
        let err =
            vox_eval::reasoning_slice::verify_reasoning_slices(&replay_report, ALPHA).unwrap_err();
        assert!(err.contains("gsm8k_think"), "{err}");
        assert!(err.contains("look 1"), "{err}");
    }

    #[test]
    fn fixed_n_reports_are_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let (_bench_root, _base, adapter, pre_path, _pre_map) =
            setup_test_collateral_env(dir.path());
        let order: Vec<String> = vec![
            "gsm8k_think".into(),
            "math500_think".into(),
            "vox_think".into(),
        ];
        let settings = vox_eval::decode_protocol::DecodeSettings::default();

        let mut scorer = |bench: &str| -> Result<BenchScore> {
            let n = SLICE_SIZE;
            let (dir_name, _want_think) = match bench.strip_suffix("_init") {
                Some(b) => (b, true),
                None => (bench, false),
            };
            let flags: Vec<ItemFlag> = (0..n).map(|i| (format!("{dir_name}_{i}"), true)).collect();
            Ok(BenchScore {
                value: 1.0,
                n: Some(n),
                paired: Some((flags.clone(), flags)),
                items_blake3: None,
            })
        };

        run_collateral_damage_fail_fast_with_settings(
            &pre_path,
            &adapter,
            &order,
            ALPHA,
            &settings,
            &mut scorer,
        )
        .expect("fixed-n report generated");

        let report = read_report(&adapter);
        assert_eq!(report["status"], "pass");
        assert_eq!(
            report["reasoning_slices"]["rule"],
            vox_eval::reasoning_slice::PAIRED_RULE
        );
        assert!(report["reasoning_slices"].get("sequential").is_none());
        assert!(
            report["reasoning_slices"]["benches"]["gsm8k_think"]
                .get("stopped_at_look")
                .is_none()
        );
    }

    #[test]
    fn a_look_is_judged_only_on_a_contiguous_pinned_order_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let ck_root = tempfile::tempdir().unwrap();
        let (bench_root, base, adapter, _pre_path, _pre_map) =
            setup_test_collateral_env(dir.path());
        let settings = vox_eval::decode_protocol::DecodeSettings::default();
        let items = super::load_slice_file(&bench_root.join("gsm8k_think").join("reasoning.jsonl"))
            .unwrap();

        let rule = vox_eval::sequential_mcnemar::SequentialRule::new(
            vec![32, 64, 96, 128],
            vec![0.0005, 0.002, 0.005, 0.04795],
            ALPHA,
        )
        .unwrap();

        // Populate base with all 128 items
        let mut base_infer = |_: &str| -> Result<String> {
            Ok(serde_json::json!({
                "generated_text": "<think>\nthink\n</think>\n\n\\boxed{42}"
            })
            .to_string())
        };
        super::super::reasoning_eval::score_slice_checkpointed(
            ck_root.path(),
            &base,
            "gsm8k_think",
            &items,
            &settings,
            false,
            &mut base_infer,
        )
        .unwrap();

        // Populate adapter with only items 0..30 (31 items, not yet 32)
        let mut adapter_infer = |_: &str| -> Result<String> {
            Ok(serde_json::json!({
                "generated_text": "<think>\nthink\n</think>\n\n\\boxed{42}"
            })
            .to_string())
        };
        super::super::reasoning_eval::score_slice_checkpointed(
            ck_root.path(),
            &adapter,
            "gsm8k_think",
            &items[..31],
            &settings,
            false,
            &mut adapter_infer,
        )
        .unwrap();

        let (acc, think, contiguous_len) = sequential_outcomes(
            ck_root.path(),
            &base,
            &adapter,
            "gsm8k_think",
            &items,
            &settings,
        )
        .unwrap();

        assert_eq!(contiguous_len, 31);
        let v = rule.judge_slice("gsm8k_think", &acc, &think);
        // Look 1 is at 32 items, so look 1 has NOT been judged yet
        assert_eq!(v.stopped_at_look, None);
        assert_eq!(v.items_used, 31);
        assert_eq!(v.accuracy.looks.len(), 0);

        // Now record item 31 (making it 32 items)
        super::super::reasoning_eval::score_slice_checkpointed(
            ck_root.path(),
            &adapter,
            "gsm8k_think",
            &items[..32],
            &settings,
            false,
            &mut adapter_infer,
        )
        .unwrap();

        let (acc2, think2, contiguous_len2) = sequential_outcomes(
            ck_root.path(),
            &base,
            &adapter,
            "gsm8k_think",
            &items,
            &settings,
        )
        .unwrap();
        assert_eq!(contiguous_len2, 32);
        let v2 = rule.judge_slice("gsm8k_think", &acc2, &think2);
        assert_eq!(v2.accuracy.looks.len(), 1);
        assert_eq!(v2.accuracy.looks[0].n, 32);
    }
}
