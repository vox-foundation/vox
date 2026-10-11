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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}
