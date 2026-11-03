//! Honest pairwise comparison of eval-corpus reports (BENCH-03).
//!
//! Each non-baseline arm is compared with the baseline on the fixtures both
//! scored: exact McNemar on paired first-attempt outcomes, Holm across the
//! claim family (arms in the order given), and a paired bootstrap CI of the
//! pass@k difference. The resolution floor ([`min_detectable_difference`]) is
//! reported beside every claim, and a difference below it is `Unresolved`,
//! never "no difference". No verdict is derived from interval overlap.
//!
//! Pure: no I/O. Eligibility is re-derived with
//! [`EvalCorpusReport::recheck_complete`], never read from the stored flag.

use crate::corpus_stats::{bootstrap_ci, holm_reject, min_detectable_difference, paired_compare};
use crate::report::{ArmStats, EvalCorpusReport, per_problem_estimates};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One report to compare, labelled by the caller (the CLI uses the path).
#[derive(Debug, Clone)]
pub struct ArmInput {
    /// Caller-chosen unique label.
    pub label: String,
    /// The typed report.
    pub report: EvalCorpusReport,
}

/// Analysis parameters, echoed into the output so a comparison can be recomputed.
#[derive(Debug, Clone)]
pub struct CompareOptions {
    /// Family-wise error rate for Holm.
    pub alpha: f64,
    /// Bootstrap resamples for the pass@k difference interval.
    pub bootstrap_reps: usize,
    /// Bootstrap seed.
    pub analysis_seed: u64,
}

/// Outcome of one claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Holm rejected the null and the difference is at or above the floor.
    Significant,
    /// Resolvable difference, Holm did not reject.
    NotSignificant,
    /// Difference is below the resolution floor: the data cannot tell.
    Unresolved,
}

/// An included arm's own statistics (from its report).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmSummary {
    /// Caller label.
    pub label: String,
    /// Model under test.
    pub model_id: String,
    /// Prompt condition; never merged across arms.
    pub condition: Option<String>,
    /// Problems the arm scored.
    pub n_problems: usize,
    /// pass@1 with bootstrap CI.
    pub pass_at_1: Option<ArmStats>,
    /// pass@k with bootstrap CI.
    pub pass_at_k: Option<ArmStats>,
}

/// One arm vs the baseline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimResult {
    /// Challenger label.
    pub arm: String,
    /// Baseline label.
    pub baseline: String,
    /// Challenger prompt condition.
    pub condition_arm: Option<String>,
    /// Baseline prompt condition.
    pub condition_baseline: Option<String>,
    /// Fixtures both arms scored.
    pub n_aligned: usize,
    /// First-attempt pass rate of the arm minus the baseline's.
    pub first_attempt_difference: f64,
    /// Exact two-sided McNemar p on the paired first-attempt outcomes.
    pub mcnemar_p: f64,
    /// Holm decision across the claim family.
    pub holm_rejected: bool,
    /// `min_detectable_difference(n_aligned)`.
    pub min_detectable_difference: f64,
    /// Mean aligned pass@k difference (arm minus baseline).
    pub pass_at_k_difference: f64,
    /// Paired-bootstrap CI of the pass@k difference.
    pub pass_at_k_difference_ci: (f64, f64),
    /// Decision: floor first, then Holm.
    pub verdict: Verdict,
}

/// A report that did not enter the comparison, and why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExcludedArm {
    /// Caller label.
    pub label: String,
    /// Reasons it is ineligible.
    pub reasons: Vec<String>,
}

/// The full comparison.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comparison {
    /// The one problem set every included arm scored.
    pub problem_set_hash: String,
    /// The one k every included arm used.
    pub k: usize,
    /// Holm alpha.
    pub alpha: f64,
    /// Bootstrap resamples.
    pub bootstrap_reps: usize,
    /// Bootstrap seed.
    pub analysis_seed: u64,
    /// Arms that entered, in input order.
    pub included: Vec<ArmSummary>,
    /// Arms that did not.
    pub excluded: Vec<ExcludedArm>,
    /// One claim per included non-baseline arm, in input order.
    pub claims: Vec<ClaimResult>,
}

fn exclusion_reasons(r: &EvalCorpusReport) -> Vec<String> {
    if r.run_complete {
        vec!["stored run_complete contradicts its provenance".to_string()]
    } else if r.incomplete_reasons.is_empty() {
        vec!["run is marked incomplete".to_string()]
    } else {
        r.incomplete_reasons.clone()
    }
}

/// First-attempt outcome per scored fixture; fixtures with no attempts or
/// fewer than `k` samples are not scored.
fn first_attempts(r: &EvalCorpusReport, k: usize) -> BTreeMap<&str, bool> {
    r.fixtures
        .iter()
        .filter(|f| f.n > 0 && f.n >= k)
        .filter_map(|f| {
            let a = f.attempts.first()?;
            Some((f.fixture_id.as_str(), a.tests_passed && !a.cheated))
        })
        .collect()
}

/// Compare every included arm with `baseline`.
///
/// # Errors
/// Missing or ineligible baseline, no eligible challenger, differing problem
/// sets or k, the same run supplied twice, or no fixture scored by both arms.
pub fn compare(
    arms: &[ArmInput],
    baseline: &str,
    opts: &CompareOptions,
) -> Result<Comparison, String> {
    let mut seen = std::collections::BTreeSet::new();
    if let Some(dup) = arms.iter().find(|a| !seen.insert(a.label.as_str())) {
        return Err(format!("arm label {:?} supplied twice", dup.label));
    }
    let (included, excluded): (Vec<&ArmInput>, Vec<&ArmInput>) =
        arms.iter().partition(|a| a.report.recheck_complete());
    let excluded: Vec<ExcludedArm> = excluded
        .into_iter()
        .map(|a| ExcludedArm {
            label: a.label.clone(),
            reasons: exclusion_reasons(&a.report),
        })
        .collect();

    if !arms.iter().any(|a| a.label == baseline) {
        return Err(format!("baseline {baseline:?} is not among the reports"));
    }
    if let Some(e) = excluded.iter().find(|e| e.label == baseline) {
        return Err(format!(
            "baseline {baseline:?} is not an eligible run: {}",
            e.reasons.join("; ")
        ));
    }
    let base = included
        .iter()
        .find(|a| a.label == baseline)
        .expect("baseline is included: it exists and is not excluded");
    let challengers: Vec<&&ArmInput> = included.iter().filter(|a| a.label != baseline).collect();
    if challengers.is_empty() {
        return Err("no eligible arm besides the baseline".to_string());
    }

    let mut runs = std::collections::BTreeMap::new();
    for a in &included {
        let id = a
            .report
            .provenance
            .identity
            .as_ref()
            .map(|i| i.run_id.as_str());
        if let Some(prev) = id.and_then(|id| runs.insert(id, a.label.as_str())) {
            return Err(format!(
                "same run supplied twice: {prev:?} and {:?} share run_id {:?}",
                a.label,
                id.unwrap_or_default()
            ));
        }
    }

    let hash_of = |a: &ArmInput| {
        a.report
            .provenance
            .problem_set_hash
            .clone()
            .unwrap_or_default()
    };
    let k = base.report.score.k;
    let hash = hash_of(base);
    for a in &challengers {
        if hash_of(a) != hash {
            return Err(format!(
                "different problem sets: {baseline:?} has {hash:?} but {:?} has {:?}",
                a.label,
                hash_of(a)
            ));
        }
        if a.report.score.k != k {
            return Err(format!(
                "different k: {baseline:?} has k={k} but {:?} has k={}",
                a.label, a.report.score.k
            ));
        }
    }

    let base_first = first_attempts(&base.report, k);
    let base_k: BTreeMap<String, f64> = per_problem_estimates(&base.report.fixtures, k)
        .into_iter()
        .collect();

    let mut claims = Vec::new();
    for a in challengers {
        let arm_first = first_attempts(&a.report, k);
        let arm_k: BTreeMap<String, f64> = per_problem_estimates(&a.report.fixtures, k)
            .into_iter()
            .collect();
        // BTreeMap iteration fixes the alignment order: fixture id, ascending.
        let aligned: Vec<&str> = arm_first
            .keys()
            .copied()
            .filter(|id| base_first.contains_key(id))
            .collect();
        if aligned.is_empty() {
            return Err(format!(
                "no fixtures scored by both {baseline:?} and {:?}",
                a.label
            ));
        }
        let arm_bools: Vec<bool> = aligned.iter().map(|id| arm_first[id]).collect();
        let base_bools: Vec<bool> = aligned.iter().map(|id| base_first[id]).collect();
        let paired = paired_compare(&arm_bools, &base_bools);
        let diffs: Vec<f64> = aligned.iter().map(|id| arm_k[*id] - base_k[*id]).collect();
        let n = aligned.len();
        claims.push(ClaimResult {
            arm: a.label.clone(),
            baseline: baseline.to_string(),
            condition_arm: a.report.provenance.condition.clone(),
            condition_baseline: base.report.provenance.condition.clone(),
            n_aligned: n,
            // One division of an exact integer (arm-only minus baseline-only
            // passes): `rate(a) - rate(b)` can land one ulp under the floor
            // when the true difference equals it.
            first_attempt_difference: (paired.b_only as f64 - paired.c_only as f64) / n as f64,
            mcnemar_p: paired.p_value,
            holm_rejected: false,
            min_detectable_difference: min_detectable_difference(n),
            pass_at_k_difference: diffs.iter().sum::<f64>() / n as f64,
            pass_at_k_difference_ci: bootstrap_ci(
                &diffs,
                opts.bootstrap_reps.max(1),
                opts.analysis_seed,
            ),
            verdict: Verdict::NotSignificant,
        });
    }

    let ps: Vec<f64> = claims.iter().map(|c| c.mcnemar_p).collect();
    for (c, rejected) in claims.iter_mut().zip(holm_reject(&ps, opts.alpha)) {
        c.holm_rejected = rejected;
        c.verdict = if c.first_attempt_difference.abs() < c.min_detectable_difference {
            Verdict::Unresolved
        } else if rejected {
            Verdict::Significant
        } else {
            Verdict::NotSignificant
        };
    }

    let summary = |a: &&ArmInput| ArmSummary {
        label: a.label.clone(),
        model_id: a.report.model_id.clone(),
        condition: a.report.provenance.condition.clone(),
        n_problems: a.report.score.n_fixtures,
        pass_at_1: a.report.pass_at_1.clone(),
        pass_at_k: a.report.pass_at_k.clone(),
    };
    Ok(Comparison {
        problem_set_hash: hash,
        k,
        alpha: opts.alpha,
        bootstrap_reps: opts.bootstrap_reps,
        analysis_seed: opts.analysis_seed,
        included: included.iter().map(summary).collect(),
        excluded,
        claims,
    })
}

/// Per-spoke paired delta and win/tie/loss summary (`EVAL-02`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpokeDeltaSummary {
    /// Target spoke.
    pub spoke: crate::mens::MensEvalSpoke,
    /// Baseline `pass@1`.
    pub baseline_pass_at_1: f64,
    /// Candidate `pass@1`.
    pub candidate_pass_at_1: f64,
    /// `candidate_pass_at_1 - baseline_pass_at_1`.
    pub delta_pass_at_1: f64,
    /// Baseline `pass@k`.
    pub baseline_pass_at_k: f64,
    /// Candidate `pass@k`.
    pub candidate_pass_at_k: f64,
    /// `candidate_pass_at_k - baseline_pass_at_k`.
    pub delta_pass_at_k: f64,
    /// Paired problems where candidate passed first attempt and baseline failed.
    pub wins: usize,
    /// Paired problems where both passed or both failed first attempt.
    pub ties: usize,
    /// Paired problems where baseline passed first attempt and candidate failed.
    pub losses: usize,
    /// Whether this spoke satisfies non-inferiority (`delta_pass_at_1 >= -max_spoke_regression`).
    pub non_inferior: bool,
}

/// Automated adapter promotion/rollback gate verdict (`EVAL-04`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionGateVerdict {
    /// All 5 spokes meet non-inferiority and overall `pass@1` strictly improved.
    Promote,
    /// Blocked: one or more spokes regressed beyond `max_spoke_regression`.
    RejectRegression { regressed_spokes: Vec<String> },
    /// Blocked: no spoke regressed beyond tolerance, but overall `pass@1` did not strictly improve.
    HoldNoImprovement,
}

/// Paired 5-spoke candidate-vs-baseline evaluation comparison (`EVAL-02`, `EVAL-04`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiSpokeComparisonV1 {
    /// Schema discriminator (`"mens_multi_spoke_comparison_v1"`).
    pub schema_version: String,
    /// Baseline model ID.
    pub baseline_model_id: String,
    /// Candidate model ID.
    pub candidate_model_id: String,
    /// Shared dataset snapshot URI.
    pub snapshot_uri: String,
    /// Candidate overall `pass@1` minus baseline overall `pass@1`.
    pub overall_delta_pass_at_1: f64,
    /// Candidate overall `pass@k` minus baseline overall `pass@k`.
    pub overall_delta_pass_at_k: f64,
    /// Per-spoke deltas in canonical [`crate::mens::MensEvalSpoke::ALL`] order.
    pub spoke_deltas: Vec<SpokeDeltaSummary>,
    /// Automated promotion/rollback gate decision (`EVAL-04`).
    pub verdict: PromotionGateVerdict,
}

impl MultiSpokeComparisonV1 {
    /// Canonical schema discriminator.
    pub const SCHEMA_VERSION: &'static str = "mens_multi_spoke_comparison_v1";
}

/// Compare a candidate 5-spoke scorecard against a baseline scorecard and paired problem outcomes,
/// computing per-spoke deltas, win/tie/loss counts, and the [`PromotionGateVerdict`] (`EVAL-02`, `EVAL-04`).
#[must_use]
pub fn compare_mens_five_spokes(
    baseline: &crate::mens::MensFiveSpokeScorecardV1,
    candidate: &crate::mens::MensFiveSpokeScorecardV1,
    baseline_problems: &[crate::mens::SpokeProblemInput],
    candidate_problems: &[crate::mens::SpokeProblemInput],
    max_spoke_regression: f64,
) -> MultiSpokeComparisonV1 {
    use crate::mens::MensEvalSpoke;

    let tol = max_spoke_regression.max(0.0);
    let mut spoke_deltas = Vec::with_capacity(MensEvalSpoke::ALL.len());
    let mut regressed_spokes = Vec::new();

    for spoke in MensEvalSpoke::ALL {
        let b_metric = baseline.spokes.iter().find(|s| s.spoke == spoke);
        let c_metric = candidate.spokes.iter().find(|s| s.spoke == spoke);

        let b_p1 = b_metric.map(|m| m.pass_at_1).unwrap_or(0.0);
        let c_p1 = c_metric.map(|m| m.pass_at_1).unwrap_or(0.0);
        let delta_p1 = c_p1 - b_p1;

        let b_pk = b_metric.map(|m| m.pass_at_k).unwrap_or(0.0);
        let c_pk = c_metric.map(|m| m.pass_at_k).unwrap_or(0.0);
        let delta_pk = c_pk - b_pk;

        let b_map: BTreeMap<&str, bool> = baseline_problems
            .iter()
            .filter(|p| p.spoke == spoke)
            .filter_map(|p| Some((p.case_id.as_str(), p.attempts.first()?.is_full_pass())))
            .collect();
        let c_map: BTreeMap<&str, bool> = candidate_problems
            .iter()
            .filter(|p| p.spoke == spoke)
            .filter_map(|p| Some((p.case_id.as_str(), p.attempts.first()?.is_full_pass())))
            .collect();

        let mut wins = 0usize;
        let mut ties = 0usize;
        let mut losses = 0usize;
        for (case_id, &b_ok) in &b_map {
            if let Some(&c_ok) = c_map.get(case_id) {
                match (b_ok, c_ok) {
                    (false, true) => wins += 1,
                    (true, false) => losses += 1,
                    _ => ties += 1,
                }
            }
        }

        let non_inferior = delta_p1 + 1e-9 >= -tol && losses == 0;
        if !non_inferior {
            regressed_spokes.push(spoke.as_str().to_string());
        }

        spoke_deltas.push(SpokeDeltaSummary {
            spoke,
            baseline_pass_at_1: b_p1,
            candidate_pass_at_1: c_p1,
            delta_pass_at_1: delta_p1,
            baseline_pass_at_k: b_pk,
            candidate_pass_at_k: c_pk,
            delta_pass_at_k: delta_pk,
            wins,
            ties,
            losses,
            non_inferior,
        });
    }

    let overall_delta_pass_at_1 = candidate.overall_pass_at_1 - baseline.overall_pass_at_1;
    let overall_delta_pass_at_k = candidate.overall_pass_at_k - baseline.overall_pass_at_k;

    let verdict = if !regressed_spokes.is_empty() {
        PromotionGateVerdict::RejectRegression { regressed_spokes }
    } else if overall_delta_pass_at_1 > 1e-9 {
        PromotionGateVerdict::Promote
    } else {
        PromotionGateVerdict::HoldNoImprovement
    };

    MultiSpokeComparisonV1 {
        schema_version: MultiSpokeComparisonV1::SCHEMA_VERSION.to_string(),
        baseline_model_id: baseline.model_id.clone(),
        candidate_model_id: candidate.model_id.clone(),
        snapshot_uri: candidate.snapshot_uri.clone(),
        overall_delta_pass_at_1,
        overall_delta_pass_at_k,
        spoke_deltas,
        verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus_score::{AttemptOutcome, FixtureOutcome};
    use crate::provenance::test_complete_external;
    use crate::report::{DEFAULT_MAX_INFRA_RATE, ReportInputs};

    const K: usize = 2;

    fn attempt(pass: bool) -> AttemptOutcome {
        AttemptOutcome {
            compiled: true,
            tests_passed: pass,
            cheated: false,
            total_tokens: 10,
            latency_ms: 5,
            cost_usd: None,
        }
    }

    /// `(id, first_attempt_passes)`; the second attempt always fails, so pass@2
    /// equals the first-attempt outcome.
    fn fx(id: &str, first: bool) -> FixtureOutcome {
        FixtureOutcome {
            fixture_id: id.to_string(),
            n: K,
            c: usize::from(first),
            attempts: vec![attempt(first), attempt(false)],
        }
    }

    /// Fixtures `f00..f{n}`; the first `passes` pass their first attempt.
    fn fixtures(n: usize, passes: usize) -> Vec<FixtureOutcome> {
        (0..n)
            .map(|i| fx(&format!("f{i:02}"), i < passes))
            .collect()
    }

    fn report(run_id: &str, condition: &str, outcomes: Vec<FixtureOutcome>) -> EvalCorpusReport {
        let mut provenance = test_complete_external();
        provenance.identity.as_mut().expect("identity").run_id = run_id.to_string();
        provenance.condition = Some(condition.to_string());
        EvalCorpusReport::assemble(ReportInputs {
            model_id: format!("model-{run_id}"),
            harness_id: "h".into(),
            config_digest: "d".into(),
            cutoff: None,
            held_out_only: true,
            measured: true,
            k: K,
            n_samples_planned: outcomes.len() * K,
            outcomes,
            samples: vec![],
            n_infra_errors: 0,
            max_infra_rate: DEFAULT_MAX_INFRA_RATE,
            spend_cap_hit: false,
            provenance,
            bootstrap_reps: 200,
            analysis_seed: 1,
        })
    }

    fn arm(label: &str, r: EvalCorpusReport) -> ArmInput {
        ArmInput {
            label: label.to_string(),
            report: r,
        }
    }

    fn opts() -> CompareOptions {
        CompareOptions {
            alpha: 0.05,
            bootstrap_reps: 200,
            analysis_seed: 3,
        }
    }

    #[test]
    fn significant_paired_difference() {
        let a = arm("A", report("run-a", "zero-shot", fixtures(40, 10)));
        let b = arm("B", report("run-b", "zero-shot", fixtures(40, 30)));
        let c = compare(&[a, b], "A", &opts()).expect("comparison");
        assert_eq!(c.claims.len(), 1);
        let claim = &c.claims[0];
        assert_eq!((claim.arm.as_str(), claim.baseline.as_str()), ("B", "A"));
        assert_eq!(claim.n_aligned, 40);
        assert!(claim.mcnemar_p < 0.05, "p = {}", claim.mcnemar_p);
        assert!(claim.holm_rejected);
        assert!((claim.first_attempt_difference - 0.5).abs() < 1e-12);
        assert_eq!(claim.min_detectable_difference, 0.25);
        assert_eq!(claim.verdict, Verdict::Significant);
        let (lo, hi) = claim.pass_at_k_difference_ci;
        assert!(lo <= claim.pass_at_k_difference && claim.pass_at_k_difference <= hi);
        assert_eq!(c.included.len(), 2);
        assert!(
            c.included
                .iter()
                .all(|s| s.pass_at_1.is_some() && s.pass_at_k.is_some())
        );
        assert_eq!((c.k, c.problem_set_hash.as_str()), (K, "ps-hash"));
    }

    #[test]
    fn compare_excludes_forged_complete_reports() {
        let mut forged = report("run-f", "zero-shot", fixtures(40, 40));
        forged.provenance.identity = None;
        forged.run_complete = true; // hand-edited flag
        assert!(!forged.recheck_complete());
        let base = arm("A", report("run-a", "zero-shot", fixtures(40, 10)));
        let good = arm("B", report("run-b", "zero-shot", fixtures(40, 30)));
        let c = compare(&[base, good, arm("F", forged.clone())], "A", &opts()).unwrap();
        assert_eq!(c.excluded.len(), 1);
        assert_eq!(c.excluded[0].label, "F");
        assert!(c.excluded[0].reasons[0].contains("contradicts"));
        assert_eq!(c.claims.len(), 1, "forged arm makes no claim");

        let good = arm("B", report("run-b", "zero-shot", fixtures(40, 30)));
        let err = compare(&[arm("F", forged), good], "F", &opts()).unwrap_err();
        assert!(err.contains("not an eligible run"), "{err}");
    }

    #[test]
    fn compare_refuses_mismatched_problem_sets_and_k() {
        let a = arm("A", report("run-a", "zero-shot", fixtures(40, 10)));
        let mut other = report("run-b", "zero-shot", fixtures(40, 30));
        other.provenance.problem_set_hash = Some("other-hash".into());
        let err = compare(&[a.clone(), arm("B", other)], "A", &opts()).unwrap_err();
        assert!(
            err.contains("different problem sets")
                && err.contains("\"A\"")
                && err.contains("\"B\"")
        );

        let mut other = report("run-c", "zero-shot", fixtures(40, 30));
        other.score.k = K + 1;
        let err = compare(&[a, arm("C", other)], "A", &opts()).unwrap_err();
        assert!(err.contains("different k"), "{err}");
    }

    #[test]
    fn compare_refuses_duplicate_run() {
        let a = arm("A", report("run-x", "zero-shot", fixtures(40, 10)));
        let b = arm("B", report("run-x", "zero-shot", fixtures(40, 30)));
        let err = compare(&[a, b], "A", &opts()).unwrap_err();
        assert!(err.contains("same run supplied twice"), "{err}");
    }

    #[test]
    fn compare_empty_and_single_fixture_edges() {
        let a = arm("A", report("run-a", "zero-shot", fixtures(40, 10)));
        let b = arm("B", report("run-b", "zero-shot", fixtures(40, 30)));
        assert!(compare(&[a.clone(), b.clone()], "Z", &opts()).is_err());
        assert!(
            compare(std::slice::from_ref(&a), "A", &opts())
                .unwrap_err()
                .contains("no eligible arm")
        );
        assert!(compare(&[], "A", &opts()).is_err());

        let disjoint = report("run-d", "zero-shot", vec![fx("zz", true)]);
        let err = compare(&[a.clone(), arm("D", disjoint)], "A", &opts()).unwrap_err();
        assert!(err.contains("no fixtures scored by both"), "{err}");

        let one_a = arm("A", report("run-a", "zero-shot", vec![fx("f00", false)]));
        let one_b = arm("B", report("run-b", "zero-shot", vec![fx("f00", true)]));
        let c = compare(&[one_a, one_b], "A", &opts()).unwrap();
        assert_eq!(c.claims[0].n_aligned, 1);
        assert_eq!(c.claims[0].min_detectable_difference, 0.25);
        // 1.0 is far above the floor; p = 1.0 so Holm keeps the null.
        assert_eq!(c.claims[0].verdict, Verdict::NotSignificant);
    }

    #[test]
    fn compare_floor_boundary_is_inclusive() {
        // n_aligned = 200 -> floor 0.10. The arm passes 20 first attempts, the
        // baseline none: difference is exactly the floor and is resolved.
        let base = arm("A", report("run-a", "zero-shot", fixtures(200, 0)));
        let at = arm("B", report("run-b", "zero-shot", fixtures(200, 20)));
        let c = compare(&[base.clone(), at], "A", &opts()).unwrap();
        let r = &c.claims[0];
        assert_eq!(r.min_detectable_difference, 0.10);
        assert_eq!(r.first_attempt_difference, 0.10);
        assert_eq!(r.verdict, Verdict::Significant);

        // The same difference arrived at as 120/200 - 100/200 (0.0999..98 in
        // f64 subtraction) is still at the floor, not below it.
        let base2 = arm("A", report("run-a", "zero-shot", fixtures(200, 100)));
        let at2 = arm("B", report("run-b", "zero-shot", fixtures(200, 120)));
        let c = compare(&[base2, at2], "A", &opts()).unwrap();
        assert_eq!(c.claims[0].verdict, Verdict::Significant);

        let below = arm("B", report("run-c", "zero-shot", fixtures(200, 10)));
        let c = compare(&[base, below], "A", &opts()).unwrap();
        assert_eq!(c.claims[0].first_attempt_difference, 0.05);
        assert_eq!(c.claims[0].verdict, Verdict::Unresolved);
    }

    #[test]
    fn compare_claim_order_is_input_order() {
        let a = arm("A", report("run-a", "zero-shot", fixtures(40, 10)));
        let c_arm = arm("C", report("run-c", "zero-shot", fixtures(40, 30)));
        let b_arm = arm("B", report("run-b", "zero-shot", fixtures(40, 30)));
        let c = compare(&[a, c_arm, b_arm], "A", &opts()).unwrap();
        let order: Vec<_> = c.claims.iter().map(|r| r.arm.as_str()).collect();
        assert_eq!(order, ["C", "B"]);
        assert_eq!(c.claims[0].mcnemar_p, c.claims[1].mcnemar_p);
        assert!(c.claims.iter().all(|r| r.holm_rejected));
    }

    #[test]
    fn compare_keeps_conditions_separate() {
        let a = arm("A", report("run-a", "C0", fixtures(40, 10)));
        let b = arm("B", report("run-b", "C1", fixtures(40, 30)));
        let c = compare(&[a, b], "A", &opts()).unwrap();
        assert_eq!(c.claims[0].condition_baseline.as_deref(), Some("C0"));
        assert_eq!(c.claims[0].condition_arm.as_deref(), Some("C1"));
        let conds: Vec<_> = c.included.iter().map(|s| s.condition.as_deref()).collect();
        assert_eq!(conds, [Some("C0"), Some("C1")]);
    }

    #[test]
    fn compare_mens_five_spokes_enforces_non_inferiority_and_promotion_gate() {
        use crate::mens::{
            MensEvalSpoke, SpokeAttemptOutcome, SpokeProblemInput, evaluate_mens_five_spokes,
        };

        let make_probs = |pass_flags: [bool; 5]| -> Vec<SpokeProblemInput> {
            MensEvalSpoke::ALL
                .iter()
                .enumerate()
                .map(|(idx, &spoke)| SpokeProblemInput {
                    case_id: format!("c_{}", spoke.as_str()),
                    spoke,
                    attempts: vec![SpokeAttemptOutcome {
                        case_id: format!("c_{}", spoke.as_str()),
                        spoke,
                        syntax_valid: true,
                        semantic_valid: pass_flags[idx],
                        execution_passed: pass_flags[idx],
                        diagnostic_summary: None,
                    }],
                })
                .collect()
        };

        let base_probs = make_probs([true, true, false, false, false]);
        let cand_promote_probs = make_probs([true, true, true, true, false]);
        let cand_regress_probs = make_probs([false, true, true, true, true]);

        let base_card = evaluate_mens_five_spokes(
            "mens-base",
            "trusty://vox-dataset/s1",
            "trusty://blake3/a0",
            1,
            &base_probs,
        );
        let promote_card = evaluate_mens_five_spokes(
            "mens-cand-1",
            "trusty://vox-dataset/s1",
            "trusty://blake3/a1",
            1,
            &cand_promote_probs,
        );
        let regress_card = evaluate_mens_five_spokes(
            "mens-cand-2",
            "trusty://vox-dataset/s1",
            "trusty://blake3/a2",
            1,
            &cand_regress_probs,
        );

        let comp_ok = compare_mens_five_spokes(
            &base_card,
            &promote_card,
            &base_probs,
            &cand_promote_probs,
            0.0,
        );
        assert_eq!(comp_ok.verdict, PromotionGateVerdict::Promote);
        assert!((comp_ok.overall_delta_pass_at_1 - 0.40).abs() < 1e-9);

        let comp_reg = compare_mens_five_spokes(
            &base_card,
            &regress_card,
            &base_probs,
            &cand_regress_probs,
            0.0,
        );
        assert_eq!(
            comp_reg.verdict,
            PromotionGateVerdict::RejectRegression {
                regressed_spokes: vec!["vox_lang".to_string()],
            }
        );
    }
}
