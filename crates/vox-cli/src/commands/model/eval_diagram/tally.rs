//! Recording + oracle outputs -> tallies under the pre-registered rule. Only `BTreeMap`, `Vec` and
//! sums in recording order, so `tallies.json` is byte-reproducible.
//!
//! Per model slot (A primary, B replication, never pooled): a task with a transport error in any
//! turn of either arm is an infra exclusion from both arms; the rest are scored on first-pass and
//! final (last recorded turn) validity, the paired 2x2 table with its Newcombe interval and verdict,
//! exact McNemar p, fidelity, per-kind counts, tokens and cost.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};

pub use super::stats::{
    ArmTally, Fidelity, KindCount, Paired, Primary, Rate, SlotTally, Tallies, verdict_key,
};

use super::oracle::{Judged, OracleMap, judge, oracle_map};
use super::score::{AttemptIndex, check_recording, index_attempts};
use super::{Arm, Attempt, Line, MAX_REPAIR_TURNS, OracleOutput, Rule, Task};

/// What one arm did on one scored task.
#[derive(Debug, Clone, Copy)]
struct Outcome {
    first: bool,
    last: bool,
    matched: bool,
}

fn outcome(turns: &[&Attempt], oracle: &OracleMap<'_>) -> Result<Outcome> {
    let (Some(first), Some(last)) = (turns.first(), turns.last()) else {
        anyhow::bail!("no attempts");
    };
    let first = judge(first, oracle)? == Judged::Valid;
    let last_valid = judge(last, oracle)? == Judged::Valid;
    let matched = last_valid && {
        let id = last.oracle_id();
        oracle
            .get(id.as_str())
            .with_context(|| format!("no oracle output for {id}"))?
            .matched
            == Some(true)
    };
    Ok(Outcome {
        first,
        last: last_valid,
        matched,
    })
}

fn tally_slot(
    rule: &Rule,
    tasks: &[Task],
    slot: &str,
    index: &AttemptIndex<'_>,
    oracle: &OracleMap<'_>,
) -> Result<SlotTally> {
    let mine: Vec<&Attempt> = index.values().copied().filter(|a| a.slot == slot).collect();
    let (mut excluded, mut missing, mut scored) = (Vec::new(), Vec::new(), Vec::new());
    for t in tasks {
        let turns = |arm: Arm| -> Vec<&Attempt> {
            (0..=MAX_REPAIR_TURNS)
                .filter_map(|k| index.get(&(t.id.as_str(), slot, arm, k)).copied())
                .collect()
        };
        let (vd, mm) = (turns(Arm::Voxdiagram), turns(Arm::Mermaid));
        if vd.iter().chain(&mm).any(|a| a.transport_error.is_some()) {
            excluded.push(t.id.clone());
        } else if vd.first().is_none_or(|a| a.turn != 0) || mm.first().is_none_or(|a| a.turn != 0) {
            missing.push(t.id.clone());
        } else {
            scored.push((t, [outcome(&vd, oracle)?, outcome(&mm, oracle)?]));
        }
    }
    let n = scored.len();
    // Arm::BOTH order: index 0 is VoxDiagram (arm 1 of the paired table), 1 is Mermaid.
    let arms = Arm::BOTH.iter().enumerate().map(|(i, &arm)| {
        let outs: Vec<Outcome> = scored.iter().map(|(_, o)| o[i]).collect();
        let spent: Vec<&&Attempt> = mine.iter().filter(|a| a.arm == arm).collect();
        let (first, valid) = (
            outs.iter().filter(|o| o.first).count(),
            outs.iter().filter(|o| o.last).count(),
        );
        let matched = outs.iter().filter(|o| o.matched).count();
        let tally = ArmTally {
            first_pass: Rate::new(first, n),
            after_repair: Rate::new(valid, n),
            fidelity: Fidelity {
                matched,
                valid,
                scored: n,
            },
            attempts: spent.len(),
            prompt_tokens: spent.iter().map(|a| a.prompt_tokens).sum(),
            completion_tokens: spent.iter().map(|a| a.completion_tokens).sum(),
            cost_usd: spent.iter().map(|a| a.cost_usd).sum(),
        };
        (arm.as_str().to_string(), tally)
    });
    let arms = arms.collect();
    let pairs = |f: fn(&Outcome) -> bool| -> Vec<(bool, bool)> {
        scored.iter().map(|(_, o)| (f(&o[0]), f(&o[1]))).collect()
    };
    let paired = [
        ("first_pass", pairs(|o| o.first)),
        ("after_repair", pairs(|o| o.last)),
    ]
    .into_iter()
    .map(|(k, p)| (k.to_string(), Paired::new(&p, rule.margin)))
    .collect();
    let mut per_kind: BTreeMap<String, BTreeMap<String, KindCount>> = BTreeMap::new();
    for (t, outs) in &scored {
        for (arm, o) in Arm::BOTH.iter().zip(outs) {
            let k = per_kind
                .entry(t.kind.clone())
                .or_default()
                .entry(arm.as_str().to_string())
                .or_default();
            k.n += 1;
            k.first_pass += usize::from(o.first);
            k.after_repair += usize::from(o.last);
        }
    }
    let resolved_models: Vec<String> = mine
        .iter()
        .filter_map(|a| a.resolved_model.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut reasons = Vec::new();
    if !missing.is_empty() {
        reasons.push(format!(
            "{} of {} tasks lack a first-pass attempt in an arm",
            missing.len(),
            tasks.len()
        ));
    }
    if excluded.len() as f64 > rule.max_infra_rate * tasks.len() as f64 {
        reasons.push(format!(
            "{} infra exclusions exceed {:.1}% of {} tasks",
            excluded.len(),
            rule.max_infra_rate * 100.0,
            tasks.len()
        ));
    }
    Ok(SlotTally {
        model_drift: resolved_models.len() > 1,
        resolved_models,
        scored_tasks: n,
        excluded,
        missing,
        complete: reasons.is_empty(),
        incomplete_reason: (!reasons.is_empty()).then(|| reasons.join("; ")),
        arms,
        paired,
        per_kind,
    })
}

/// Tally a recording under the pre-registered rule; refuses a recording `check_recording` rejects.
pub fn tally(
    rule: &Rule,
    tasks: &[Task],
    lines: &[Line],
    oracle_outputs: &[OracleOutput],
) -> Result<Tallies> {
    check_recording(tasks, lines, oracle_outputs)?;
    let oracle = oracle_map(oracle_outputs);
    let index = index_attempts(tasks, lines)?;
    let slot_ids: BTreeSet<&str> = index.keys().map(|k| k.1).collect();
    let mut slots = BTreeMap::new();
    for slot in slot_ids {
        slots.insert(
            slot.to_string(),
            tally_slot(rule, tasks, slot, &index, &oracle)?,
        );
    }
    let primary = match slots.get("A") {
        Some(a) if a.complete => Primary {
            verdict: verdict_key(a.paired["first_pass"].verdict).into(),
            reason: None,
        },
        Some(a) => Primary {
            verdict: "not_determined".into(),
            reason: Some(format!(
                "model A incomplete: {}",
                a.incomplete_reason.clone().unwrap_or_default()
            )),
        },
        None => Primary {
            verdict: "not_determined".into(),
            reason: Some("no model A attempts".into()),
        },
    };
    Ok(Tallies {
        synthetic: matches!(lines.first(), Some(Line::Run(h)) if h.synthetic),
        corpus_size: tasks.len(),
        rule: rule.clone(),
        stopped: match lines.last() {
            Some(Line::Stop(s)) => Some(s.reason.clone()),
            _ => None,
        },
        slots,
        primary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::model::eval_diagram::Stop;
    use crate::commands::model::eval_diagram::oracle::{extract, vd_verdict};
    use crate::commands::model::eval_diagram::score::tests::{
        MM, VD_BAD, VD_OK, attempt, header, task as score_task,
    };
    use crate::commands::model::eval_diagram::stats::tests::rule;
    use vox_eval::corpus_stats::{Z_95, mcnemar_exact_p};
    use vox_eval::paired_ni::{ni_verdict, paired_difference_ci};
    use vox_eval::report::DEFAULT_MAX_INFRA_RATE;

    fn task(id: &str, kind: &str) -> Task {
        Task {
            kind: kind.into(),
            ..score_task(id)
        }
    }

    fn out(id: String, ok: bool, matched: Option<bool>) -> OracleOutput {
        let error = (!ok).then(|| format!("err {id}"));
        OracleOutput {
            id,
            ok,
            error,
            sets: serde_json::Value::Null,
            matched,
        }
    }

    /// Turn-0 pair of `task` in `slot`: `vd` / `mm` say which arm is valid; valid outputs match.
    fn pair(
        lines: &mut Vec<Line>,
        outs: &mut Vec<OracleOutput>,
        task: &str,
        slot: &str,
        vd: bool,
        mm: bool,
    ) {
        let v = attempt(
            task,
            Arm::Voxdiagram,
            slot,
            0,
            Some(if vd { VD_OK } else { VD_BAD }),
        );
        if vd {
            outs.push(out(v.oracle_id(), true, Some(true)));
        }
        let m = attempt(task, Arm::Mermaid, slot, 0, Some(MM));
        outs.push(out(m.oracle_id(), mm, Some(mm)));
        lines.extend([Line::Attempt(v), Line::Attempt(m)]);
    }

    #[test]
    fn the_rule_ceiling_matches_the_house_infra_ceiling() {
        assert_eq!(rule(0.05).max_infra_rate, DEFAULT_MAX_INFRA_RATE);
    }

    #[test]
    fn paired_table_interval_verdict_flags_and_primary() {
        let tasks: Vec<Task> = (0..20)
            .map(|i| task(&format!("t{i:02}"), "graph"))
            .collect();
        let (mut lines, mut outs) = (vec![header()], vec![]);
        for (i, t) in tasks.iter().enumerate() {
            pair(
                &mut lines,
                &mut outs,
                &t.id,
                "A",
                i != 19,
                i < 17 || i == 19,
            );
        }
        assert!(
            tally(&rule(0.05), &tasks, &lines, &outs).is_err(),
            "invalid turns lack repairs"
        );
        lines.push(Line::Stop(Stop {
            reason: "spend cap".into(),
            spent_usd: 25.0,
        }));
        let t = tally(&rule(0.05), &tasks, &lines, &outs).unwrap();
        let a = &t.slots["A"];
        let p = &a.paired["first_pass"];
        assert_eq!((p.a, p.b, p.c, p.d), (17, 2, 1, 0));
        assert_eq!(p.ci, paired_difference_ci(17, 2, 1, 0, Z_95));
        assert_eq!(p.verdict, ni_verdict(&p.ci, 0.10));
        assert_eq!(p.mcnemar_p, mcnemar_exact_p(2, 1));
        assert_eq!((p.superior, p.worse_within_margin), (p.ci.low > 0.0, false));
        assert_eq!(a.arms["voxdiagram"].first_pass, Rate::new(19, 20));
        assert_eq!(a.arms["mermaid"].after_repair, Rate::new(18, 20));
        assert_eq!(
            a.arms["mermaid"].fidelity,
            Fidelity {
                matched: 18,
                valid: 18,
                scored: 20
            }
        );
        assert_eq!(t.stopped.as_deref(), Some("spend cap"));
        assert!(
            a.complete && t.primary.reason.is_none(),
            "{:?}",
            a.incomplete_reason
        );
        assert_eq!(t.primary.verdict, verdict_key(p.verdict));
    }

    #[test]
    fn a_transport_error_excludes_the_task_from_both_arms() {
        let tasks = [
            task("t1", "graph"),
            task("t2", "graph"),
            task("t3", "sequence"),
        ];
        let (mut lines, mut outs) = (vec![header()], vec![]);
        pair(&mut lines, &mut outs, "t1", "A", true, true);
        let mut lost = attempt("t2", Arm::Voxdiagram, "A", 0, None);
        lost.transport_error = Some("reset".into());
        lines.push(Line::Attempt(lost));
        pair(&mut lines, &mut outs, "t2", "A", true, true);
        lines.remove(lines.len() - 2); // keep t2's Mermaid attempt, drop its valid VoxDiagram one
        pair(&mut lines, &mut outs, "t3", "A", true, true);
        let t = tally(&rule(0.05), &tasks, &lines, &outs).unwrap();
        let a = &t.slots["A"];
        assert_eq!(
            (a.excluded.clone(), a.scored_tasks),
            (vec!["t2".to_string()], 2)
        );
        assert_eq!(a.arms["mermaid"].first_pass.n, 2);
        assert_eq!(
            a.arms["voxdiagram"].attempts, 3,
            "spend counts excluded attempts"
        );
        assert!(!a.complete && a.incomplete_reason.as_deref().unwrap().contains("infra"));
        assert_eq!(t.primary.verdict, "not_determined");
        assert_eq!(
            a.per_kind["sequence"]["mermaid"],
            KindCount {
                n: 1,
                first_pass: 1,
                after_repair: 1
            }
        );
        assert!(tally(&rule(0.5), &tasks, &lines, &outs).unwrap().slots["A"].complete);
    }

    #[test]
    fn a_repair_turn_counts_after_repair_fidelity_and_cost() {
        let tasks = [task("t1", "graph")];
        let (mut lines, mut outs) = (vec![header()], vec![]);
        pair(&mut lines, &mut outs, "t1", "A", false, true);
        let mut fix = attempt("t1", Arm::Voxdiagram, "A", 1, Some(VD_OK));
        fix.feedback = Some(vd_verdict(&extract(VD_BAD)).feedback);
        outs.push(out(fix.oracle_id(), true, Some(true)));
        lines.push(Line::Attempt(fix));
        let t = tally(&rule(0.05), &tasks, &lines, &outs).unwrap();
        let vd = &t.slots["A"].arms["voxdiagram"];
        assert_eq!((vd.first_pass.valid, vd.after_repair.valid), (0, 1));
        assert_eq!(
            vd.fidelity,
            Fidelity {
                matched: 1,
                valid: 1,
                scored: 1
            }
        );
        assert_eq!(
            (vd.attempts, vd.prompt_tokens, vd.completion_tokens),
            (2, 20, 10)
        );
        assert!((vd.cost_usd - 0.002).abs() < 1e-12);
        let p = &t.slots["A"].paired;
        assert_eq!((p["first_pass"].c, p["after_repair"].a), (1, 1));
    }

    #[test]
    fn missing_first_passes_model_drift_and_no_slot_a_are_reported() {
        let tasks = [task("t1", "graph"), task("t2", "graph")];
        let (mut lines, mut outs) = (vec![header()], vec![]);
        pair(&mut lines, &mut outs, "t1", "B", true, true);
        if let Line::Attempt(a) = &mut lines[2] {
            a.resolved_model = Some("other".into());
        }
        let t = tally(&rule(0.05), &tasks, &lines, &outs).unwrap();
        let b = &t.slots["B"];
        assert_eq!(b.missing, ["t2"]);
        assert!(
            !b.complete
                && b.incomplete_reason
                    .as_deref()
                    .unwrap()
                    .contains("first-pass")
        );
        assert_eq!(b.resolved_models, ["other", "stub"]);
        assert!(b.model_drift);
        assert_eq!(t.primary.reason.as_deref(), Some("no model A attempts"));
    }
}
