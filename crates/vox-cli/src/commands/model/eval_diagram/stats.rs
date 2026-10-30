//! The shape of `tallies.json` and its statistics: Wilson rates, the paired 2x2 table with its
//! Newcombe interval and pre-registered verdict, fidelity, per-kind counts, per slot and arm.

use std::collections::BTreeMap;

use serde::Serialize;
use vox_eval::corpus_stats::{ConfidenceInterval, Z_95, mcnemar_exact_p, wilson_interval};
use vox_eval::paired_ni::{NiVerdict, ni_verdict, paired_difference_ci};

use super::Rule;

/// A count with its Wilson 95% interval.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rate {
    pub valid: usize,
    pub n: usize,
    pub ci: ConfidenceInterval,
}

impl Rate {
    pub(crate) fn new(valid: usize, n: usize) -> Self {
        Self {
            valid,
            n,
            ci: wilson_interval(valid, n, Z_95),
        }
    }
}

/// Exact canonical-set matches among final-valid outputs and over all scored tasks.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Fidelity {
    pub matched: usize,
    pub valid: usize,
    pub scored: usize,
}

/// The paired 2x2 table VoxDiagram (arm 1) vs Mermaid (arm 2) and what the rule makes of it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Paired {
    /// Both valid.
    pub a: usize,
    /// VoxDiagram only.
    pub b: usize,
    /// Mermaid only.
    pub c: usize,
    /// Neither.
    pub d: usize,
    /// Newcombe method-10 95% interval for p(VoxDiagram) - p(Mermaid).
    pub ci: ConfidenceInterval,
    pub verdict: NiVerdict,
    /// Descriptive: L > 0.
    pub superior: bool,
    /// Descriptive: U < 0 and L > -margin.
    pub worse_within_margin: bool,
    pub mcnemar_p: f64,
}

impl Paired {
    pub(crate) fn new(pairs: &[(bool, bool)], margin: f64) -> Self {
        let count = |v: bool, m: bool| pairs.iter().filter(|p| **p == (v, m)).count();
        let (a, b, c, d) = (
            count(true, true),
            count(true, false),
            count(false, true),
            count(false, false),
        );
        let ci = paired_difference_ci(a, b, c, d, Z_95);
        Self {
            a,
            b,
            c,
            d,
            verdict: ni_verdict(&ci, margin),
            superior: ci.low > 0.0,
            worse_within_margin: ci.high < 0.0 && ci.low > -margin,
            mcnemar_p: mcnemar_exact_p(b, c),
            ci,
        }
    }
}

/// First-pass and final valid counts of one kind and arm.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct KindCount {
    pub n: usize,
    pub first_pass: usize,
    pub after_repair: usize,
}

/// One arm of one model slot.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ArmTally {
    pub first_pass: Rate,
    pub after_repair: Rate,
    pub fidelity: Fidelity,
    /// Over every recorded attempt of the arm, excluded tasks included (money was spent).
    pub attempts: usize,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cost_usd: f64,
}

/// One model slot.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlotTally {
    /// Distinct provider-reported model ids; more than one means the model drifted mid-run.
    pub resolved_models: Vec<String>,
    pub model_drift: bool,
    pub scored_tasks: usize,
    /// Infra exclusions: a transport error in some turn of either arm.
    pub excluded: Vec<String>,
    /// Dataset tasks without a turn-0 attempt in both arms.
    pub missing: Vec<String>,
    pub complete: bool,
    pub incomplete_reason: Option<String>,
    pub arms: BTreeMap<String, ArmTally>,
    /// `first_pass` and `after_repair`.
    pub paired: BTreeMap<String, Paired>,
    /// kind -> arm -> counts.
    pub per_kind: BTreeMap<String, BTreeMap<String, KindCount>>,
}

/// The pre-registered primary endpoint's verdict, or why it is withheld.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Primary {
    /// `non_inferior`, `inconclusive`, `inferior` or `not_determined`.
    pub verdict: String,
    pub reason: Option<String>,
}

/// Everything the report tables show, derived from the recording alone.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tallies {
    pub synthetic: bool,
    pub corpus_size: usize,
    pub rule: Rule,
    /// Reason of a `stop` line: the run ended early.
    pub stopped: Option<String>,
    pub slots: BTreeMap<String, SlotTally>,
    pub primary: Primary,
}

/// `non_inferior`, `inconclusive` or `inferior`.
#[must_use]
pub fn verdict_key(v: NiVerdict) -> &'static str {
    match v {
        NiVerdict::NonInferior => "non_inferior",
        NiVerdict::Inconclusive => "inconclusive",
        NiVerdict::Inferior => "inferior",
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// The pre-registered rule with a chosen infra ceiling.
    pub(crate) fn rule(max_infra_rate: f64) -> Rule {
        Rule {
            primary_endpoint: "first_pass_validity_slot_a".into(),
            interval: "newcombe_method_10".into(),
            confidence: 0.95,
            margin: 0.10,
            max_repair_turns: 2,
            max_infra_rate,
        }
    }

    #[test]
    fn rate_carries_the_wilson_interval() {
        let r = Rate::new(2, 3);
        assert_eq!((r.valid, r.n), (2, 3));
        assert_eq!(r.ci, wilson_interval(2, 3, Z_95));
    }

    #[test]
    fn worse_within_margin_flag_and_verdict_keys() {
        let worse: Vec<(bool, bool)> = (0..200).map(|i| (i >= 8, true)).collect();
        let p = Paired::new(&worse, 0.10);
        assert!(p.ci.high < 0.0 && p.ci.low > -0.10, "{:?}", p.ci);
        assert!(p.worse_within_margin && !p.superior);
        assert_eq!(verdict_key(NiVerdict::NonInferior), "non_inferior");
        assert_eq!(verdict_key(NiVerdict::Inferior), "inferior");
    }

    #[test]
    fn paired_counts_the_four_cells() {
        let p = Paired::new(
            &[
                (true, true),
                (true, false),
                (false, true),
                (false, false),
                (true, false),
            ],
            0.10,
        );
        assert_eq!((p.a, p.b, p.c, p.d), (1, 2, 1, 1));
        assert_eq!(p.ci, paired_difference_ci(1, 2, 1, 1, Z_95));
        assert_eq!(p.mcnemar_p, mcnemar_exact_p(2, 1));
    }
}
