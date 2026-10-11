//! Typed eval-corpus report (schema 2): honest statistics plus the single
//! run-completeness gate.
//!
//! Pure: no I/O. All intervals and floors come from [`crate::corpus_stats`];
//! all point estimates from [`crate::corpus_score`]. `run_complete` is decided
//! once, in [`EvalCorpusReport::assemble`], and re-derived on read by
//! [`EvalCorpusReport::recheck_complete`] so a hand-edited flag proves nothing.

use crate::corpus_score::{CorpusScore, FixtureOutcome, pass_at_k, score_corpus};
use crate::corpus_stats::{bootstrap_ci, min_detectable_difference};
use crate::provenance::RunProvenance;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Report schema version written by [`EvalCorpusReport::assemble`].
pub const REPORT_SCHEMA_VERSION: u32 = 2;
/// Default preregistered ceiling on the infra-error rate (5%).
pub const DEFAULT_MAX_INFRA_RATE: f64 = 0.05;
/// Default bootstrap resamples for confidence intervals.
pub const DEFAULT_BOOTSTRAP_REPS: usize = 2000;

/// Why a sample did or did not pass. Ordered so it can key a `BTreeMap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    /// Compiled and tests passed.
    Pass,
    /// Did not parse.
    ParseError,
    /// Parsed but failed checking (compile failure).
    CheckError,
    /// Compiled, then failed at runtime.
    RuntimeError,
    /// Compiled and ran, an assertion failed.
    WrongAnswer,
    /// Verifier timed out.
    Timeout,
    /// Candidate neutralized the scoring oracle.
    Cheated,
    /// Infrastructure failure; no candidate reached the verifier.
    InfraError,
    /// Generation failed before a candidate existed.
    GenerationError,
}

/// Verifier facts for one candidate (exit-code facts, never an LLM judgment).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct VerifyFacts {
    /// Candidate compiled.
    pub compiled: bool,
    /// Tests passed.
    pub tests_passed: bool,
    /// Oracle neutralized.
    pub cheated: bool,
    /// Verifier timed out.
    pub timed_out: bool,
    /// Candidate parsed.
    pub parse_ok: bool,
    /// An assertion failed at run time.
    pub assertion_failed: bool,
}

/// Fixed-precedence classification: cheated, timeout, pass, then parse vs check
/// error (compile failure), then wrong answer vs runtime error.
///
/// Never returns `InfraError` or `GenerationError`; callers assign those when
/// no candidate reached the verifier.
#[must_use]
pub fn classify_failure(f: &VerifyFacts) -> FailureClass {
    if f.cheated {
        FailureClass::Cheated
    } else if f.timed_out {
        FailureClass::Timeout
    } else if f.tests_passed {
        FailureClass::Pass
    } else if !f.compiled {
        if f.parse_ok {
            FailureClass::CheckError
        } else {
            FailureClass::ParseError
        }
    } else if f.assertion_failed {
        FailureClass::WrongAnswer
    } else {
        FailureClass::RuntimeError
    }
}

/// One generated sample and how it fared.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SampleRecord {
    /// Fixture this sample answers.
    pub fixture_id: String,
    /// Sample index within the fixture.
    pub sample_index: usize,
    /// Seed used for this sample.
    pub seed: u64,
    /// Candidate source, when one was produced.
    pub code: Option<String>,
    /// Classified outcome.
    pub failure_class: FailureClass,
    /// Candidate compiled.
    pub compiled: bool,
    /// Tests passed.
    pub tests_passed: bool,
    /// Oracle neutralized.
    pub cheated: bool,
    /// Tokens used.
    pub total_tokens: u32,
    /// Generation latency in milliseconds.
    pub latency_ms: i64,
    /// Cost, when known.
    pub cost_usd: Option<f64>,
    /// Model string the provider reported for this sample.
    pub provider_model: Option<String>,
    /// Cumulative pass flags: index 0 is the original attempt, index t is "passed by repair
    /// turn t". Empty when the sample never reached the verifier (or predates feedback turns).
    #[serde(default)]
    pub passed_by_turn: Vec<bool>,
    /// Tokens spent on repair turns (not part of `total_tokens`).
    #[serde(default)]
    pub repair_tokens: u32,
    /// Cost of repair turns, when known.
    #[serde(default)]
    pub repair_cost_usd: Option<f64>,
}

/// Mean over problems with a percentile-bootstrap CI and the power floor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmStats {
    /// Mean of per-problem estimates.
    pub point: f64,
    /// Lower CI bound.
    pub ci_low: f64,
    /// Upper CI bound.
    pub ci_high: f64,
    /// Problems the mean is over.
    pub n_problems: usize,
    /// Bootstrap resamples used.
    pub bootstrap_reps: usize,
    /// Seed of the bootstrap, so a reader can recompute.
    pub analysis_seed: u64,
    /// `min_detectable_difference(n_problems)`.
    pub min_detectable_difference: f64,
}

/// `None` for an empty slice; otherwise the arm's statistics (no new maths).
#[must_use]
pub fn arm_stats(per_problem: &[f64], reps: usize, seed: u64) -> Option<ArmStats> {
    if per_problem.is_empty() {
        return None;
    }
    let reps = reps.max(1);
    let (ci_low, ci_high) = bootstrap_ci(per_problem, reps, seed);
    Some(ArmStats {
        point: per_problem.iter().sum::<f64>() / per_problem.len() as f64,
        ci_low,
        ci_high,
        n_problems: per_problem.len(),
        bootstrap_reps: reps,
        analysis_seed: seed,
        min_detectable_difference: min_detectable_difference(per_problem.len()),
    })
}

/// `(fixture_id, pass@k)` for fixtures with at least `k` scored samples, sorted by id.
#[must_use]
pub fn per_problem_estimates(outcomes: &[FixtureOutcome], k: usize) -> Vec<(String, f64)> {
    let mut v: Vec<(String, f64)> = outcomes
        .iter()
        .filter(|f| f.n > 0 && f.n >= k)
        .map(|f| (f.fixture_id.clone(), pass_at_k(f.n, f.c, k)))
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

/// Everything [`EvalCorpusReport::assemble`] needs.
#[derive(Debug, Clone)]
pub struct ReportInputs {
    /// Model under test.
    pub model_id: String,
    /// Harness identifier.
    pub harness_id: String,
    /// Digest of the run configuration.
    pub config_digest: String,
    /// Training cutoff, if known.
    pub cutoff: Option<String>,
    /// Only held-out problems were scored.
    pub held_out_only: bool,
    /// Generation metrics exist (false for ingested solutions).
    pub measured: bool,
    /// k for pass@k.
    pub k: usize,
    /// Per-fixture outcomes.
    pub outcomes: Vec<FixtureOutcome>,
    /// Per-sample records.
    pub samples: Vec<SampleRecord>,
    /// Samples lost to infrastructure errors.
    pub n_infra_errors: usize,
    /// Samples the run set out to take.
    pub n_samples_planned: usize,
    /// Preregistered infra-rate ceiling.
    pub max_infra_rate: f64,
    /// The spend cap stopped the run.
    pub spend_cap_hit: bool,
    /// §5 provenance.
    pub provenance: RunProvenance,
    /// Bootstrap resamples.
    pub bootstrap_reps: usize,
    /// Bootstrap seed.
    pub analysis_seed: u64,
}

/// Schema-2 report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCorpusReport {
    /// [`REPORT_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Model under test.
    pub model_id: String,
    /// Harness identifier.
    pub harness_id: String,
    /// Digest of the run configuration.
    pub config_digest: String,
    /// Training cutoff, if known.
    pub cutoff: Option<String>,
    /// Only held-out problems were scored.
    pub held_out_only: bool,
    /// §5 provenance.
    pub provenance: RunProvenance,
    /// Point estimates over scorable fixtures.
    pub score: CorpusScore,
    /// pass@1 with CI; `None` when nothing was scorable.
    pub pass_at_1: Option<ArmStats>,
    /// pass@k with CI; `None` when nothing was scorable.
    pub pass_at_k: Option<ArmStats>,
    /// Entry `t - 1` is pass@1 after `t` compiler-feedback turns (cumulative, so
    /// non-decreasing in `t`). Empty without feedback turns. `pass_at_1` stays the
    /// original attempts only.
    #[serde(default)]
    pub pass_at_1_after_feedback: Vec<ArmStats>,
    /// Compiled samples over non-infra samples.
    pub sample_compile_rate: Option<f64>,
    /// Count of samples per failure class.
    pub failure_breakdown: BTreeMap<FailureClass, usize>,
    /// Samples lost to infrastructure errors.
    pub n_infra_errors: usize,
    /// Samples the run set out to take.
    pub n_samples_planned: usize,
    /// `n_infra_errors / n_samples_planned`.
    pub infra_rate: f64,
    /// Preregistered ceiling.
    pub max_infra_rate: f64,
    /// The spend cap stopped the run.
    pub spend_cap_hit: bool,
    /// Fixtures with fewer scored samples than k, excluded from pass@k.
    pub unscored_fixture_ids: Vec<String>,
    /// Decided only by the completeness gate.
    pub run_complete: bool,
    /// One reason per failed gate cause.
    pub incomplete_reasons: Vec<String>,
    /// Per-fixture outcomes, sorted by id.
    pub fixtures: Vec<FixtureOutcome>,
    /// Samples, sorted by (fixture_id, sample_index).
    pub samples: Vec<SampleRecord>,
}

fn rate(n: usize, of: usize) -> f64 {
    if of == 0 { 0.0 } else { n as f64 / of as f64 }
}

/// The single completeness rule: one reason per failed cause, empty when complete.
fn gate_reasons(
    prov: &RunProvenance,
    spend_cap_hit: bool,
    planned: usize,
    infra_rate: f64,
    max_infra_rate: f64,
    has_scored: bool,
) -> Vec<String> {
    let mut r = Vec::new();
    if spend_cap_hit {
        r.push("spend cap hit".to_string());
    }
    if planned == 0 {
        r.push("no samples planned".to_string());
    }
    if infra_rate > max_infra_rate {
        r.push(format!(
            "infra error rate {infra_rate:.4} exceeds ceiling {max_infra_rate:.4}"
        ));
    }
    r.extend(
        prov.missing_required()
            .into_iter()
            .map(|m| format!("missing provenance: {m}")),
    );
    if prov.verifier_matches_harness == Some(false) {
        r.push("verifier binary does not match harness".to_string());
    }
    if !has_scored {
        r.push("no scored problems".to_string());
    }
    r
}

impl EvalCorpusReport {
    /// Re-derive completeness from stored provenance and counts; never trusts
    /// the stored `run_complete` alone (a hand-edited flag proves nothing).
    #[must_use]
    pub fn recheck_complete(&self) -> bool {
        self.run_complete
            && gate_reasons(
                &self.provenance,
                self.spend_cap_hit,
                self.n_samples_planned,
                rate(self.n_infra_errors, self.n_samples_planned),
                self.max_infra_rate,
                self.pass_at_k.is_some(),
            )
            .is_empty()
    }

    /// Build the report and decide `run_complete`.
    #[must_use]
    pub fn assemble(mut inp: ReportInputs) -> Self {
        inp.outcomes.sort_by(|a, b| a.fixture_id.cmp(&b.fixture_id));
        inp.samples
            .sort_by(|a, b| (&a.fixture_id, a.sample_index).cmp(&(&b.fixture_id, b.sample_index)));
        let (scorable, short): (Vec<FixtureOutcome>, Vec<FixtureOutcome>) = inp
            .outcomes
            .iter()
            .cloned()
            .partition(|f| f.n > 0 && f.n >= inp.k);
        let unscored_fixture_ids = short.into_iter().map(|f| f.fixture_id).collect();

        let mut score = score_corpus(&scorable, inp.k, inp.measured);
        score.n_infra_errors = inp.n_infra_errors;

        let at = |k| {
            let v: Vec<f64> = per_problem_estimates(&scorable, k)
                .into_iter()
                .map(|(_, p)| p)
                .collect();
            arm_stats(&v, inp.bootstrap_reps, inp.analysis_seed)
        };
        let (pass_at_1, pass_at_k) = (at(1), at(inp.k));

        // pass@1 after t feedback turns: per scorable fixture, the share of its verified
        // samples whose trajectory is true at index t.
        let feedback_turns = inp
            .provenance
            .sampling
            .as_ref()
            .map_or(0, |s| s.feedback_turns as usize);
        let pass_at_1_after_feedback: Vec<ArmStats> = (1..=feedback_turns)
            .filter_map(|t| {
                let v: Vec<f64> = scorable
                    .iter()
                    .filter_map(|f| {
                        let verified: Vec<&SampleRecord> = inp
                            .samples
                            .iter()
                            .filter(|s| {
                                s.fixture_id == f.fixture_id && !s.passed_by_turn.is_empty()
                            })
                            .collect();
                        let passed = verified
                            .iter()
                            .filter(|s| s.passed_by_turn.get(t) == Some(&true))
                            .count();
                        (!verified.is_empty()).then(|| passed as f64 / verified.len() as f64)
                    })
                    .collect();
                arm_stats(&v, inp.bootstrap_reps, inp.analysis_seed)
            })
            .collect();

        let mut failure_breakdown = BTreeMap::new();
        for s in &inp.samples {
            *failure_breakdown.entry(s.failure_class).or_insert(0) += 1;
        }
        let non_infra: Vec<&SampleRecord> = inp
            .samples
            .iter()
            .filter(|s| s.failure_class != FailureClass::InfraError)
            .collect();
        let sample_compile_rate = (!non_infra.is_empty()).then(|| {
            rate(
                non_infra.iter().filter(|s| s.compiled).count(),
                non_infra.len(),
            )
        });

        let infra_rate = rate(inp.n_infra_errors, inp.n_samples_planned);
        let incomplete_reasons = gate_reasons(
            &inp.provenance,
            inp.spend_cap_hit,
            inp.n_samples_planned,
            infra_rate,
            inp.max_infra_rate,
            pass_at_k.is_some(),
        );

        Self {
            schema_version: REPORT_SCHEMA_VERSION,
            model_id: inp.model_id,
            harness_id: inp.harness_id,
            config_digest: inp.config_digest,
            cutoff: inp.cutoff,
            held_out_only: inp.held_out_only,
            provenance: inp.provenance,
            score,
            pass_at_1,
            pass_at_k,
            pass_at_1_after_feedback,
            sample_compile_rate,
            failure_breakdown,
            n_infra_errors: inp.n_infra_errors,
            n_samples_planned: inp.n_samples_planned,
            infra_rate,
            max_infra_rate: inp.max_infra_rate,
            spend_cap_hit: inp.spend_cap_hit,
            unscored_fixture_ids,
            run_complete: incomplete_reasons.is_empty(),
            incomplete_reasons,
            fixtures: inp.outcomes,
            samples: inp.samples,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus_score::AttemptOutcome;
    use crate::provenance::test_complete_external;

    pub(super) fn fixture(id: &str, n: usize, c: usize) -> FixtureOutcome {
        FixtureOutcome {
            fixture_id: id.to_string(),
            n,
            c,
            attempts: (0..n)
                .map(|i| AttemptOutcome {
                    compiled: true,
                    tests_passed: i < c,
                    cheated: false,
                    total_tokens: 10,
                    latency_ms: 5,
                    cost_usd: None,
                })
                .collect(),
        }
    }

    pub(super) fn sample(id: &str, i: usize, class: FailureClass) -> SampleRecord {
        SampleRecord {
            fixture_id: id.to_string(),
            sample_index: i,
            seed: 1,
            code: None,
            failure_class: class,
            compiled: !matches!(class, FailureClass::ParseError | FailureClass::InfraError),
            tests_passed: class == FailureClass::Pass,
            cheated: false,
            total_tokens: 10,
            latency_ms: 5,
            cost_usd: None,
            provider_model: None,
            passed_by_turn: Vec::new(),
            repair_tokens: 0,
            repair_cost_usd: None,
        }
    }

    pub(super) fn inputs() -> ReportInputs {
        let samples = vec![
            sample("b", 0, FailureClass::Pass),
            sample("a", 1, FailureClass::Pass),
            sample("a", 0, FailureClass::ParseError),
            sample("c", 0, FailureClass::InfraError),
        ];
        ReportInputs {
            model_id: "openai/gpt-x".into(),
            harness_id: "h".into(),
            config_digest: "d".into(),
            cutoff: None,
            held_out_only: true,
            measured: true,
            k: 5,
            outcomes: vec![fixture("c", 5, 0), fixture("a", 5, 5), fixture("b", 5, 2)],
            samples,
            n_infra_errors: 1,
            n_samples_planned: 100,
            max_infra_rate: DEFAULT_MAX_INFRA_RATE,
            spend_cap_hit: false,
            provenance: test_complete_external(),
            bootstrap_reps: 200,
            analysis_seed: 42,
        }
    }

    #[test]
    fn assemble_complete_external_run() {
        let r = EvalCorpusReport::assemble(inputs());
        assert!(r.run_complete, "{:?}", r.incomplete_reasons);
        assert!(r.incomplete_reasons.is_empty());
        let a = r.pass_at_k.as_ref().expect("pass_at_k");
        assert!(a.ci_low <= a.point && a.point <= a.ci_high);
        assert_eq!(a.min_detectable_difference, 0.25);
        assert_eq!((a.bootstrap_reps, a.analysis_seed), (200, 42));
        assert_eq!(
            r.score.n_infra_errors, 1,
            "input count, not score_corpus's 0"
        );
        assert_eq!(r.failure_breakdown[&FailureClass::Pass], 2);
        assert_eq!(r.failure_breakdown.values().sum::<usize>(), 4);
        // 3 non-infra samples, 2 compiled (ParseError did not).
        assert!((r.sample_compile_rate.unwrap() - 2.0 / 3.0).abs() < 1e-9);
        assert_eq!(r.schema_version, 2);
        let ids: Vec<_> = r.fixtures.iter().map(|f| f.fixture_id.as_str()).collect();
        assert_eq!(ids, ["a", "b", "c"]);
        let order: Vec<_> = r
            .samples
            .iter()
            .map(|s| (s.fixture_id.as_str(), s.sample_index))
            .collect();
        assert_eq!(order, [("a", 0), ("a", 1), ("b", 0), ("c", 0)]);

        let json = serde_json::to_value(&r).unwrap();
        let back: EvalCorpusReport = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(&back).unwrap(), json);
    }

    fn complete_then(f: impl FnOnce(&mut ReportInputs)) -> EvalCorpusReport {
        let mut i = inputs();
        f(&mut i);
        EvalCorpusReport::assemble(i)
    }

    #[test]
    fn gate_rejects_each_cause() {
        let cases: Vec<(&str, EvalCorpusReport)> = vec![
            ("spend cap hit", complete_then(|i| i.spend_cap_hit = true)),
            (
                "no samples planned",
                complete_then(|i| i.n_samples_planned = 0),
            ),
            ("infra error rate", complete_then(|i| i.n_infra_errors = 6)),
            (
                "missing provenance: tier",
                complete_then(|i| i.provenance.tier = None),
            ),
            (
                "verifier binary does not match harness",
                complete_then(|i| i.provenance.verifier_matches_harness = Some(false)),
            ),
            ("no scored problems", complete_then(|i| i.outcomes.clear())),
        ];
        for (reason, r) in cases {
            assert!(!r.run_complete, "{reason}");
            assert!(
                r.incomplete_reasons.iter().any(|x| x.contains(reason)),
                "{reason} not in {:?}",
                r.incomplete_reasons
            );
        }
    }

    #[test]
    fn infra_ceiling_boundary_is_inclusive() {
        let at = complete_then(|i| i.n_infra_errors = 5); // 5/100 == 0.05 ceiling
        assert!(at.run_complete, "{:?}", at.incomplete_reasons);
        assert!(at.recheck_complete());
        let over = complete_then(|i| i.n_infra_errors = 6);
        assert!(!over.run_complete);
        assert!(!over.recheck_complete());
    }

    #[test]
    fn recheck_rejects_a_forged_complete_flag() {
        let r = EvalCorpusReport::assemble(inputs());
        assert!(r.recheck_complete());

        let forge = |edit: fn(&mut serde_json::Value)| {
            let mut v = serde_json::to_value(&r).unwrap();
            edit(&mut v);
            v["run_complete"] = true.into();
            serde_json::from_value::<EvalCorpusReport>(v).unwrap()
        };
        let no_identity = forge(|v| v["provenance"]["identity"] = serde_json::Value::Null);
        assert!(no_identity.run_complete && !no_identity.recheck_complete());
        let capped = forge(|v| v["spend_cap_hit"] = true.into());
        assert!(!capped.recheck_complete());
        let mismatch = forge(|v| v["provenance"]["verifier_matches_harness"] = false.into());
        assert!(!mismatch.recheck_complete());
        let unscored = forge(|v| v["pass_at_k"] = serde_json::Value::Null);
        assert!(!unscored.recheck_complete());
        let rate = forge(|v| v["n_infra_errors"] = 50.into());
        assert!(!rate.recheck_complete());
    }

    #[test]
    fn empty_problem_set_has_no_stats() {
        assert!(arm_stats(&[], 100, 1).is_none());
        let r = complete_then(|i| i.outcomes.clear());
        assert!(r.pass_at_k.is_none() && r.pass_at_1.is_none());
        assert!(
            r.incomplete_reasons
                .iter()
                .any(|x| x == "no scored problems")
        );
    }

    #[test]
    fn single_problem_has_degenerate_ci_and_floor_025() {
        let a = arm_stats(&[0.4], 100, 1).unwrap();
        assert_eq!((a.point, a.ci_low, a.ci_high), (0.4, 0.4, 0.4));
        assert_eq!((a.n_problems, a.min_detectable_difference), (1, 0.25));
        let r = complete_then(|i| i.outcomes = vec![fixture("a", 5, 2)]);
        let a = r.pass_at_k.unwrap();
        assert_eq!(a.n_problems, 1);
        assert_eq!(a.ci_low, a.ci_high);
        assert_eq!(a.min_detectable_difference, 0.25);
    }

    #[test]
    fn short_fixtures_are_unscored_not_panicking() {
        let r = complete_then(|i| {
            i.k = 3;
            i.outcomes = vec![fixture("a", 2, 2), fixture("b", 5, 3), fixture("z", 0, 0)];
        });
        assert_eq!(r.unscored_fixture_ids, ["a", "z"]);
        assert_eq!(r.pass_at_k.unwrap().n_problems, 1);
        assert_eq!(r.fixtures.len(), 3, "all fixtures stay recorded");
    }

    #[test]
    fn classify_failure_precedence() {
        let f = |c, t, ch, to, p, a| VerifyFacts {
            compiled: c,
            tests_passed: t,
            cheated: ch,
            timed_out: to,
            parse_ok: p,
            assertion_failed: a,
        };
        use FailureClass::*;
        // cheated beats everything, timeout beats pass.
        assert_eq!(
            classify_failure(&f(true, true, true, true, true, true)),
            Cheated
        );
        assert_eq!(
            classify_failure(&f(true, true, false, true, true, false)),
            Timeout
        );
        assert_eq!(
            classify_failure(&f(true, true, false, false, true, false)),
            Pass
        );
        assert_eq!(
            classify_failure(&f(false, false, false, false, false, false)),
            ParseError
        );
        assert_eq!(
            classify_failure(&f(false, false, false, false, true, false)),
            CheckError
        );
        assert_eq!(
            classify_failure(&f(true, false, false, false, true, true)),
            WrongAnswer
        );
        assert_eq!(
            classify_failure(&f(true, false, false, false, true, false)),
            RuntimeError
        );
    }

    #[test]
    fn bootstrap_is_seed_deterministic() {
        let v: Vec<f64> = (0..40).map(|i| f64::from((i * 7) % 11) / 10.0).collect();
        let (a, b) = (
            arm_stats(&v, 500, 1).unwrap(),
            arm_stats(&v, 500, 1).unwrap(),
        );
        assert_eq!(a, b);
        let c = arm_stats(&v, 500, 99).unwrap();
        assert_ne!((a.ci_low, a.ci_high), (c.ci_low, c.ci_high));
        assert_eq!(c.analysis_seed, 99);
    }

    // ---- pass@1 after compiler-feedback turns (12-08) ----

    fn with_trajectories(turns: u32, traj: &[(&str, usize, &[bool])]) -> EvalCorpusReport {
        complete_then(|i| {
            i.k = 2;
            i.outcomes = vec![fixture("a", 2, 0), fixture("b", 2, 1), fixture("z", 1, 0)];
            i.provenance.sampling.as_mut().unwrap().feedback_turns = turns;
            i.samples = traj
                .iter()
                .map(|(id, idx, t)| SampleRecord {
                    passed_by_turn: t.to_vec(),
                    ..sample(id, *idx, FailureClass::CheckError)
                })
                .collect();
        })
    }

    #[test]
    fn pass_after_feedback_per_turn() {
        let r = with_trajectories(
            1,
            &[
                ("a", 0, &[false, true]),
                ("a", 1, &[false, false]),
                ("b", 0, &[true, true]),
                ("b", 1, &[false, true]),
                // "z" has n = 1 < k = 2: unscored, so excluded from the per-turn stats too.
                ("z", 0, &[false, true]),
            ],
        );
        assert_eq!(r.pass_at_1_after_feedback.len(), 1);
        let a = &r.pass_at_1_after_feedback[0];
        assert!(
            (a.point - 0.75).abs() < 1e-12,
            "mean(1/2, 2/2), got {}",
            a.point
        );
        assert_eq!(a.n_problems, 2);
        assert!(a.ci_low <= a.point && a.point <= a.ci_high);
        assert_eq!(a.min_detectable_difference, min_detectable_difference(2));
    }

    #[test]
    fn pass_after_feedback_is_monotone() {
        let r = with_trajectories(
            3,
            &[
                ("a", 0, &[false, false, true, true]),
                ("a", 1, &[false, false, false, false]),
                ("b", 0, &[false, true, true, true]),
                ("b", 1, &[false, false, false, true]),
            ],
        );
        let pts: Vec<f64> = r.pass_at_1_after_feedback.iter().map(|a| a.point).collect();
        assert_eq!(pts.len(), 3);
        assert!(pts.windows(2).all(|w| w[0] <= w[1]), "{pts:?}");
        assert_eq!(pts, [0.25, 0.5, 0.75]);
    }

    #[test]
    fn no_feedback_turns_no_extra_stats() {
        let r = with_trajectories(0, &[("a", 0, &[false]), ("b", 0, &[true])]);
        assert!(r.pass_at_1_after_feedback.is_empty());
        assert_eq!(r.pass_at_1.as_ref().unwrap().n_problems, 2);
    }
}
