//! Gate evaluation against run artifacts.

use anyhow::Result;
use std::path::{Path, PathBuf};

use super::io::{read_jsonl_nonempty_lines, read_utf8_path_capped};
use super::leakage::assert_no_text_leakage;
use super::policy::load_policy;

/// Gate check result.
#[derive(Debug)]
pub struct GateResult {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub block: bool,
}

/// Walk upward from the current working directory looking for `rel`. Used to
/// locate the repo-relative `mens/data/heldout_bench/manifest.json` (and
/// sibling corpus directories) regardless of which directory within the
/// workspace `check_run` happens to be invoked from — `vox mens eval-gate`
/// is documented to run from the repo root, but tests run from the crate
/// directory, and this still finds the real file there.
fn find_upward(rel: &str) -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join(rel);
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Task D3 (2026-09-12-mens-end-to-end-completion), part 3: `assert_no_leakage`
/// (tool-name Jaccard) had zero non-test callers and can't see code-body
/// leakage anyway. This wires its text-n-gram sibling, `assert_no_text_leakage`,
/// in as a hard precondition — bench-answer/corpus-completion leakage always
/// blocks (`block: true`, not policy-gated) regardless of every other gate's
/// configuration, because a leaked bench invalidates every other gate result
/// that depends on pass@k. When the bench file can't be found relative to the
/// current working directory at all (e.g. a packaged binary run outside the
/// workspace), the precondition can't be evaluated and is reported as
/// not-applicable rather than failing closed on an environment difference.
pub(crate) fn check_leakage_precondition(results: &mut Vec<GateResult>) {
    const BENCH_REL: &str = "mens/data/heldout_bench/manifest.json";
    let Some(bench_path) = find_upward(BENCH_REL) else {
        results.push(GateResult {
            name: "leakage".to_string(),
            passed: true,
            message: format!(
                "{BENCH_REL} not found relative to cwd — leakage precondition not applicable"
            ),
            block: false,
        });
        return;
    };
    // bench_path = <repo_root>/mens/data/heldout_bench/manifest.json
    let repo_root = bench_path
        .parent() // heldout_bench
        .and_then(Path::parent) // mens/data
        .and_then(Path::parent) // mens
        .and_then(Path::parent); // repo_root
    let corpus_dirs: Vec<PathBuf> = match repo_root {
        Some(root) => vec![root.join("mens/data"), root.join("target/dogfood")],
        None => vec![],
    };
    let corpus_dir_refs: Vec<&Path> = corpus_dirs.iter().map(PathBuf::as_path).collect();

    match assert_no_text_leakage(&bench_path, &corpus_dir_refs) {
        Ok(()) => results.push(GateResult {
            name: "leakage".to_string(),
            passed: true,
            message: "no bench-answer / corpus-completion n-gram overlap detected".to_string(),
            block: true,
        }),
        Err(e) => results.push(GateResult {
            name: "leakage".to_string(),
            passed: false,
            message: e.to_string(),
            block: true,
        }),
    }
}

/// Run gate checks against a run directory.
pub fn check_run(run_dir: &Path, policy_path: &Path) -> Result<Vec<GateResult>> {
    let policy = load_policy(policy_path)?;
    let mut results = Vec::new();

    // Hard precondition (Task D3): must run before any other gate result is
    // trusted. Always evaluated, never policy-gated.
    check_leakage_precondition(&mut results);

    // Trainer writes `training_manifest.json`; `manifest.json` is legacy/hand-written.
    // Prefer the canonical trainer output; fall back to the legacy name for older runs.
    let manifest_path = {
        let canonical = run_dir.join("training_manifest.json");
        let legacy = run_dir.join("manifest.json");
        if canonical.exists() {
            canonical
        } else {
            legacy
        }
    };
    let _manifest: Option<serde_json::Value> = if manifest_path.exists() {
        let content = read_utf8_path_capped(&manifest_path)?;
        serde_json::from_str(&content).ok()
    } else {
        None
    };

    // Trainer writes `telemetry.jsonl`; `metrics.jsonl` is the legacy alias.
    // Mirror the fallback used in `vox mens status` (status.rs).
    let metrics_path = {
        let legacy = run_dir.join("metrics.jsonl");
        let canonical = run_dir.join("telemetry.jsonl");
        if legacy.exists() { legacy } else { canonical }
    };
    let metrics_lines: Vec<String> = if metrics_path.exists() {
        read_jsonl_nonempty_lines(&metrics_path)?
    } else {
        Vec::new()
    };

    if policy.supervised_ratio.block || policy.supervised_ratio.min_pct > 0.0 {
        let sup_values: Vec<f64> = metrics_lines
            .iter()
            .filter_map(|l| {
                serde_json::from_str::<serde_json::Value>(l)
                    .ok()
                    .and_then(|v| {
                        let from_payload = v.get("payload").and_then(|p| {
                            p.get("supervised_ratio_pct")
                                .and_then(|x| x.as_f64())
                                .or_else(|| {
                                    let valid = p.get("valid_tokens").and_then(|x| x.as_u64())?;
                                    let theoretical =
                                        p.get("theoretical_tokens").and_then(|x| x.as_u64())?;
                                    if theoretical == 0 {
                                        Some(0.0)
                                    } else {
                                        Some((valid as f64 / theoretical as f64) * 100.0)
                                    }
                                })
                        });
                        from_payload
                            .or_else(|| v.get("supervised_ratio_pct").and_then(|x| x.as_f64()))
                            .or_else(|| {
                                let valid = v.get("valid_tokens").and_then(|x| x.as_u64())?;
                                let theoretical =
                                    v.get("theoretical_tokens").and_then(|x| x.as_u64())?;
                                if theoretical == 0 {
                                    Some(0.0)
                                } else {
                                    Some((valid as f64 / theoretical as f64) * 100.0)
                                }
                            })
                    })
            })
            .collect();
        if !sup_values.is_empty() {
            let sup_pct = sup_values.iter().sum::<f64>() / sup_values.len() as f64;
            let passed = sup_pct >= policy.supervised_ratio.min_pct;
            results.push(GateResult {
                name: "supervised_ratio".to_string(),
                passed,
                message: format!(
                    "supervised_ratio {:.1}% {} {:.1}%",
                    sup_pct,
                    if passed { ">=" } else { "<" },
                    policy.supervised_ratio.min_pct
                ),
                block: policy.supervised_ratio.block,
            });
        } else {
            results.push(GateResult {
                name: "supervised_ratio".to_string(),
                passed: !policy.supervised_ratio.block,
                message: "supervised_ratio missing from telemetry".to_string(),
                block: policy.supervised_ratio.block,
            });
        }
    }

    // --- anti_stub gate --------------------------------------------------------

    // anti_stub metrics (anti_stub_task_success, placeholder_event_rate,
    // trivial_placeholder_event_rate, construct_richness_mean) are written by
    // `vox mens eval-local` into eval_local_report.json — NOT by the corpus
    // evaluator (eval_results.json) or the QLoRA trainer.  Read from the
    // canonical eval-local output file so the gate can actually be satisfied.
    // Mirrors the pass_at_k pattern: missing file → failing GateResult whose
    // block flag is taken from the policy (block: true in full policy, false in
    // the post-train bootstrap policy).
    {
        let anti_stub_cfg = &policy.anti_stub;
        let anti_stub_active = anti_stub_cfg.block
            || anti_stub_cfg.min_pass_rate > 0.0
            || anti_stub_cfg.max_placeholder_event_rate > 0.0
            || anti_stub_cfg.max_trivial_placeholder_event_rate > 0.0
            || anti_stub_cfg.min_construct_richness_mean > 0.0;
        if anti_stub_active {
            let eval_local_report_path = run_dir.join("eval_local_report.json");
            if !eval_local_report_path.exists() {
                results.push(GateResult {
                    name: "anti_stub".to_string(),
                    passed: false,
                    message: format!(
                        "eval_local_report.json missing — run `vox mens eval-local \
                         --model {}/candle_qlora_adapter.safetensors \
                         --bench mens/data/heldout_bench \
                         -o {}/eval_local_report.json` first",
                        run_dir.display(),
                        run_dir.display()
                    ),
                    block: anti_stub_cfg.block,
                });
            } else {
                let content = read_utf8_path_capped(&eval_local_report_path)?;
                if let Ok(el) = serde_json::from_str::<serde_json::Value>(&content) {
                    // Recompute from per-sample checks when present so older reports
                    // (which folded parse failures into anti_stub_pass and averaged
                    // richness over unparseable zeros) are judged with current rules.
                    let m =
                        crate::commands::mens::metrics::anti_stub_metrics_from_eval_local_report(
                            &el,
                        );
                    let anti_stub_pass = m.anti_stub_task_success;
                    let placeholder_rate = m.placeholder_event_rate;
                    let trivial_placeholder_rate = m.trivial_placeholder_event_rate;
                    let construct_richness_mean = m.construct_richness_mean;
                    let pass_rate_ok = anti_stub_pass >= anti_stub_cfg.min_pass_rate;
                    let placeholder_ok = anti_stub_cfg.max_placeholder_event_rate <= 0.0
                        || placeholder_rate <= anti_stub_cfg.max_placeholder_event_rate;
                    let trivial_ok = anti_stub_cfg.max_trivial_placeholder_event_rate <= 0.0
                        || trivial_placeholder_rate
                            <= anti_stub_cfg.max_trivial_placeholder_event_rate;
                    let richness_ok = anti_stub_cfg.min_construct_richness_mean <= 0.0
                        || construct_richness_mean >= anti_stub_cfg.min_construct_richness_mean;
                    let passed = pass_rate_ok && placeholder_ok && trivial_ok && richness_ok;
                    results.push(GateResult {
                        name: "anti_stub".to_string(),
                        passed,
                        message: format!(
                            "anti_stub_pass={:.3} (min={:.3}) placeholder_rate={:.3} \
                             (max={:.3}) trivial_rate={:.3} (max={:.3}) \
                             richness_mean={:.3} (min={:.3}) [metrics:{:?}]",
                            anti_stub_pass,
                            anti_stub_cfg.min_pass_rate,
                            placeholder_rate,
                            anti_stub_cfg.max_placeholder_event_rate,
                            trivial_placeholder_rate,
                            anti_stub_cfg.max_trivial_placeholder_event_rate,
                            construct_richness_mean,
                            anti_stub_cfg.min_construct_richness_mean,
                            m.source,
                        ),
                        block: anti_stub_cfg.block,
                    });
                }
            }
        }
    }

    // --- pass_at_k gate --------------------------------------------------------
    let passk_cfg = &policy.pass_at_k;
    let passk_active = passk_cfg.block
        || passk_cfg.min_pass_rate_at_1 > 0.0
        || passk_cfg.min_pass_rate_at_k > 0.0
        || passk_cfg.baseline_file.is_some();
    if passk_active {
        let metrics_name = std::path::Path::new(&passk_cfg.metrics_file)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let metrics_path = run_dir.join(if metrics_name.is_empty() {
            "benchmark_passatk.json"
        } else {
            &metrics_name
        });
        if !metrics_path.exists() {
            results.push(GateResult {
                name: "pass_at_k".to_string(),
                passed: false,
                message: format!(
                    "metrics file missing: {} — run `vox mens eval-local \
                     --model {}/candle_qlora_adapter.safetensors \
                     --bench mens/data/heldout_bench \
                     --samples <k> -o {}/eval_local_report.json` first",
                    metrics_path.display(),
                    run_dir.display(),
                    run_dir.display()
                ),
                block: passk_cfg.block,
            });
        } else {
            let content = read_utf8_path_capped(&metrics_path)?;
            let v: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| anyhow::anyhow!("{}: invalid JSON ({})", metrics_path.display(), e))?;
            let p1 = v
                .get("pass_rate_at_1")
                .and_then(|x| x.as_f64())
                .unwrap_or(0.0);
            let pk = v
                .get("pass_rate_at_k")
                .and_then(|x| x.as_f64())
                .unwrap_or(0.0);
            // With samples=1 / k=1, pass@k collapses to pass@1. Requiring a higher
            // pass@k floor then is a wiring trap, not a quality signal. Keep the
            // stricter pass@k floor when k > 1.
            let k = v.get("k").and_then(|x| x.as_u64()).unwrap_or(1);
            let min_pk = if k <= 1 {
                passk_cfg.min_pass_rate_at_1
            } else {
                passk_cfg.min_pass_rate_at_k
            };
            let mut pass = p1 >= passk_cfg.min_pass_rate_at_1 && pk >= min_pk;
            let mut msg = if k <= 1 {
                format!(
                    "pass@1={:.3} (min={:.3}) pass@k={:.3} (min={:.3}; k=1 collapses to pass@1)",
                    p1, passk_cfg.min_pass_rate_at_1, pk, min_pk
                )
            } else {
                format!(
                    "pass@1={:.3} (min={:.3}) pass@k={:.3} (min={:.3})",
                    p1, passk_cfg.min_pass_rate_at_1, pk, min_pk
                )
            };
            if let Some(baseline_name) = passk_cfg.baseline_file.as_deref() {
                let baseline_path = run_dir.join(
                    std::path::Path::new(baseline_name)
                        .file_name()
                        .unwrap_or_default(),
                );
                if baseline_path.exists() {
                    let bcontent = read_utf8_path_capped(&baseline_path)?;
                    if let Ok(bv) = serde_json::from_str::<serde_json::Value>(&bcontent) {
                        let b1 = bv
                            .get("pass_rate_at_1")
                            .and_then(|x| x.as_f64())
                            .unwrap_or(0.0);
                        let bk = bv
                            .get("pass_rate_at_k")
                            .and_then(|x| x.as_f64())
                            .unwrap_or(0.0);
                        let drop1 = b1 - p1;
                        let dropk = bk - pk;
                        if drop1 > passk_cfg.max_regression_drop
                            || dropk > passk_cfg.max_regression_drop
                        {
                            pass = false;
                        }
                        msg.push_str(&format!(
                            " baseline(pass@1={:.3},pass@k={:.3}) max_drop={:.3}",
                            b1, bk, passk_cfg.max_regression_drop
                        ));
                    }
                } else {
                    // A3 (c): a *configured* baseline_file that is absent must hard-fail
                    // the gate, not silently skip the regression check — the same defect
                    // class Task A1/A2 fixed elsewhere (a policy reading a file nobody
                    // produced yet reading as "pass"/"not applicable" instead of "broken").
                    pass = false;
                    msg.push_str(&format!(
                        " baseline file missing: {} — beat-base comparison required but not \
                         found (run `vox mens eval-local --base ...` to produce it)",
                        baseline_path.display()
                    ));
                }
            }
            results.push(GateResult {
                name: "pass_at_k".to_string(),
                passed: pass,
                message: msg,
                block: passk_cfg.block,
            });
        }
    }

    // --- collateral_damage gate ----------------------------------------------
    // Same artifact Hub publish reads. Active when block/require_report is set
    // (ladder policy) or when a report file is already present (surface status).
    {
        let cd = &policy.collateral_damage;
        let report_path = run_dir.join("collateral_damage_report.json");
        let req_rep = cd.require_report.unwrap_or(false);
        let active = cd.block || req_rep || report_path.exists();
        if active {
            if !report_path.exists() {
                results.push(GateResult {
                    name: "collateral_damage".to_string(),
                    passed: false,
                    message: "collateral_damage_report.json missing — run `vox mens eval-collateral-damage` (fail-fast default: gsm8k→math500→vox)".to_string(),
                    block: cd.block || req_rep,
                });
            } else {
                let content = read_utf8_path_capped(&report_path)?;
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(v) => {
                        let status = v.get("status").and_then(|s| s.as_str()).unwrap_or("");
                        let failed_on = v.get("failed_on").and_then(|x| x.as_str()).unwrap_or("-");
                        let fail_fast = v
                            .get("fail_fast")
                            .and_then(|x| x.as_bool())
                            .unwrap_or(false);
                        // The stored status is not trusted: verified via check_collateral_report
                        // against current slice pins, rule, alpha, and canonical grader.
                        let req_res = cd.to_requirement();
                        let (passed, detail) = match req_res {
                            Ok(req) => {
                                match vox_eval::collateral_evidence::check_collateral_report(
                                    &v, &req,
                                ) {
                                    Ok(()) => (
                                        true,
                                        format!(
                                            "paired McNemar alpha={} on the pinned item sets: no regression",
                                            req.alpha
                                        ),
                                    ),
                                    Err(refusal) => (false, format!("refused: {refusal}")),
                                }
                            }
                            Err(e) => (false, format!("refused: {e}")),
                        };
                        results.push(GateResult {
                            name: "collateral_damage".to_string(),
                            passed,
                            message: format!(
                                "status={status} failed_on={failed_on} fail_fast={fail_fast} {detail}"
                            )
                            .trim_end()
                            .to_string(),
                            block: cd.block || req_rep,
                        });
                    }
                    Err(e) => results.push(GateResult {
                        name: "collateral_damage".to_string(),
                        passed: false,
                        message: format!("invalid collateral_damage_report.json: {e}"),
                        block: cd.block || req_rep,
                    }),
                }
            }
        }
    }

    Ok(results)
}
