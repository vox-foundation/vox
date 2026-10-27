//! Recording + oracle outputs -> tallies. Only `BTreeMap` and `Vec`, so `tallies.json` is
//! byte-reproducible.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::Serialize;
use vox_eval::corpus_stats::{ConfidenceInterval, Z_95, wilson_interval};

use super::oracle::{extract, vd_verdict};
use super::score::attempts;
use super::{Arm, Attempt, Line, OracleOutput, Rule, Task};

/// A count with its Wilson 95% interval.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rate {
    pub valid: usize,
    pub n: usize,
    pub ci: ConfidenceInterval,
}

impl Rate {
    fn new(valid: usize, n: usize) -> Self {
        Self {
            valid,
            n,
            ci: wilson_interval(valid, n, Z_95),
        }
    }
}

/// One arm of one model slot.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ArmTally {
    pub first_pass: Rate,
}

/// One model slot.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlotTally {
    pub scored_tasks: usize,
    pub arms: BTreeMap<String, ArmTally>,
}

/// Everything the report tables show, derived from the recording alone.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tallies {
    pub synthetic: bool,
    pub corpus_size: usize,
    pub margin: f64,
    pub slots: BTreeMap<String, SlotTally>,
}

/// Validity of one attempt with a reply: `check` for VoxDiagram, the Node oracle for Mermaid.
pub fn attempt_valid(a: &Attempt, oracle: &BTreeMap<&str, &OracleOutput>) -> Result<bool> {
    let Some(raw) = a
        .raw_response
        .as_deref()
        .filter(|_| a.transport_error.is_none())
    else {
        return Ok(false);
    };
    Ok(match a.arm {
        Arm::Voxdiagram => vd_verdict(&extract(raw)).valid,
        Arm::Mermaid => {
            let id = a.oracle_id();
            oracle
                .get(id.as_str())
                .with_context(|| format!("no oracle output for {id}"))?
                .ok
        }
    })
}

/// Tally a recording under the pre-registered rule.
pub fn tally(
    rule: &Rule,
    tasks: &[Task],
    lines: &[Line],
    oracle_outputs: &[OracleOutput],
) -> Result<Tallies> {
    let oracle: BTreeMap<&str, &OracleOutput> =
        oracle_outputs.iter().map(|o| (o.id.as_str(), o)).collect();
    let synthetic = matches!(lines.first(), Some(Line::Run(h)) if h.synthetic);
    // slot -> task -> arm -> turn-0 validity
    let mut first: BTreeMap<&str, BTreeMap<&str, BTreeMap<Arm, bool>>> = BTreeMap::new();
    for a in attempts(lines).filter(|a| a.turn == 0) {
        let v = attempt_valid(a, &oracle)?;
        first
            .entry(a.slot.as_str())
            .or_default()
            .entry(a.task_id.as_str())
            .or_default()
            .insert(a.arm, v);
    }
    let mut slots = BTreeMap::new();
    for (slot, per_task) in first {
        let paired: Vec<&BTreeMap<Arm, bool>> =
            per_task.values().filter(|arms| arms.len() == 2).collect();
        let arms = Arm::BOTH
            .iter()
            .map(|arm| {
                let valid = paired.iter().filter(|m| m[arm]).count();
                (
                    arm.as_str().to_string(),
                    ArmTally {
                        first_pass: Rate::new(valid, paired.len()),
                    },
                )
            })
            .collect();
        slots.insert(
            slot.to_string(),
            SlotTally {
                scored_tasks: paired.len(),
                arms,
            },
        );
    }
    Ok(Tallies {
        synthetic,
        corpus_size: tasks.len(),
        margin: rule.margin,
        slots,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::model::eval_diagram::score::tests::{MM, VD_BAD, VD_OK, attempt, task};

    fn rule() -> Rule {
        Rule {
            primary_endpoint: "first_pass_validity_slot_a".into(),
            interval: "newcombe_method_10".into(),
            confidence: 0.95,
            margin: 0.10,
            max_repair_turns: 2,
            max_infra_rate: 0.05,
        }
    }

    fn out(id: &str, ok: bool) -> OracleOutput {
        OracleOutput {
            id: id.into(),
            ok,
            error: None,
            sets: serde_json::Value::Null,
            matched: None,
        }
    }

    #[test]
    fn first_pass_counts_and_wilson_per_slot_and_arm() {
        let lines = vec![
            Line::Attempt(attempt("t1", Arm::Voxdiagram, "A", 0, Some(VD_OK))),
            Line::Attempt(attempt("t1", Arm::Mermaid, "A", 0, Some(MM))),
            Line::Attempt(attempt("t2", Arm::Voxdiagram, "A", 0, Some(VD_BAD))),
            Line::Attempt(attempt("t2", Arm::Mermaid, "A", 0, Some(MM))),
        ];
        let outs = [out("t1:A:mermaid:0", true), out("t2:A:mermaid:0", false)];
        let t = tally(&rule(), &[task("t1"), task("t2")], &lines, &outs).unwrap();
        let a = &t.slots["A"];
        assert_eq!(a.scored_tasks, 2);
        assert_eq!(a.arms["voxdiagram"].first_pass, Rate::new(1, 2));
        assert_eq!(a.arms["mermaid"].first_pass.valid, 1);
        assert_eq!(a.arms["mermaid"].first_pass.ci, wilson_interval(1, 2, Z_95));
    }

    #[test]
    fn a_mermaid_reply_without_an_oracle_output_is_an_error() {
        let lines = vec![Line::Attempt(attempt("t1", Arm::Mermaid, "A", 0, Some(MM)))];
        let err = tally(&rule(), &[task("t1")], &lines, &[]).unwrap_err();
        assert!(err.to_string().contains("t1:A:mermaid:0"), "{err}");
    }
}
