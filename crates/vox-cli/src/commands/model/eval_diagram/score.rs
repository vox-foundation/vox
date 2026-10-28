//! Recording -> oracle inputs (which texts the Node Mermaid oracle must judge), per-attempt
//! verdicts, and the recording checks: turn structure and feedback consistency, which catch oracle
//! drift and hand-edited recordings.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};

use super::oracle::{Judged, extract, judge, oracle_map, vd_verdict};
use super::{Arm, Attempt, Line, MAX_REPAIR_TURNS, OracleInput, OracleOutput, Task, write_jsonl};

/// Attempts by (task, slot, arm, turn).
pub type AttemptIndex<'a> = BTreeMap<(&'a str, &'a str, Arm, u32), &'a Attempt>;

/// Index the attempts, refusing unknown tasks, turns above `MAX_REPAIR_TURNS`, a reply and a
/// transport error together (or neither), and duplicate (task, slot, arm, turn).
pub fn index_attempts<'a>(tasks: &[Task], lines: &'a [Line]) -> Result<AttemptIndex<'a>> {
    let ids: BTreeSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    let mut index = BTreeMap::new();
    for a in attempts(lines) {
        let id = a.oracle_id();
        ensure!(ids.contains(a.task_id.as_str()), "{id}: unknown task");
        ensure!(
            a.turn <= MAX_REPAIR_TURNS,
            "{id}: turn above {MAX_REPAIR_TURNS}"
        );
        ensure!(
            a.raw_response.is_some() != a.transport_error.is_some(),
            "{id}: exactly one of raw_response and transport_error must be set"
        );
        let key = (a.task_id.as_str(), a.slot.as_str(), a.arm, a.turn);
        ensure!(index.insert(key, a).is_none(), "{id}: duplicate attempt");
    }
    Ok(index)
}

/// Check a recording: header first, a stop line only last; turn k+1 exists only after an invalid
/// turn k and carries exactly the recomputed feedback of turn k; without a stop line every invalid
/// turn below the cap has its repair turn.
pub fn check_recording(
    tasks: &[Task],
    lines: &[Line],
    oracle_outputs: &[OracleOutput],
) -> Result<()> {
    ensure!(
        matches!(lines.first(), Some(Line::Run(_))),
        "line 1 must be the run header"
    );
    for (i, l) in lines.iter().enumerate().skip(1) {
        ensure!(
            !matches!(l, Line::Run(_)),
            "line {}: a second run header",
            i + 1
        );
        ensure!(
            !matches!(l, Line::Stop(_)) || i + 1 == lines.len(),
            "line {}: the stop line must be last",
            i + 1
        );
    }
    let stopped = matches!(lines.last(), Some(Line::Stop(_)));
    let oracle = oracle_map(oracle_outputs);
    let index = index_attempts(tasks, lines)?;
    for (&(task, slot, arm, turn), a) in &index {
        let id = a.oracle_id();
        if turn == 0 {
            ensure!(a.feedback.is_none(), "{id}: turn 0 carries feedback");
        } else {
            let prev = index
                .get(&(task, slot, arm, turn - 1))
                .with_context(|| format!("{id}: turn {} is missing", turn - 1))?;
            let Judged::Invalid(expected) = judge(prev, &oracle)? else {
                bail!("{id}: turn {} needed no repair", turn - 1);
            };
            ensure!(
                a.feedback.as_deref() == Some(expected.as_str()),
                "{id}: recorded feedback differs from the recomputed feedback of turn {}; expected {}",
                turn - 1,
                serde_json::to_string(&expected)?
            );
        }
        if !stopped && turn < MAX_REPAIR_TURNS && matches!(judge(a, &oracle)?, Judged::Invalid(_)) {
            ensure!(
                index.contains_key(&(task, slot, arm, turn + 1)),
                "{id}: invalid without repair turn {} and no stop line",
                turn + 1
            );
        }
    }
    Ok(())
}

/// The attempts of a recording, in recording order.
pub fn attempts(lines: &[Line]) -> impl Iterator<Item = &Attempt> {
    lines.iter().filter_map(|l| match l {
        Line::Attempt(a) => Some(a),
        _ => None,
    })
}

/// Expected canonical sets per task id, from the Node-derived reference outputs.
pub fn expected_sets<'a>(
    tasks: &[Task],
    reference_outputs: &'a [OracleOutput],
) -> Result<BTreeMap<String, &'a serde_json::Value>> {
    let by_id: BTreeMap<&str, &OracleOutput> = reference_outputs
        .iter()
        .map(|o| (o.id.as_str(), o))
        .collect();
    let mut out = BTreeMap::new();
    for t in tasks {
        let o = by_id
            .get(t.id.as_str())
            .with_context(|| format!("task {}: no reference output", t.id))?;
        ensure!(
            o.ok && o.sets.is_object() && o.sets.get("unsupported").is_none(),
            "task {}: reference export is not a supported Mermaid diagram: {:?}",
            t.id,
            o.error
        );
        out.insert(t.id.clone(), &o.sets);
    }
    Ok(out)
}

/// The text an attempt contributes to the Mermaid oracle: the extracted reply for the Mermaid arm,
/// the Mermaid export of a valid VoxDiagram reply; `None` without a reply or for invalid VoxDiagram.
#[must_use]
pub fn oracle_text(a: &Attempt) -> Option<String> {
    if a.transport_error.is_some() {
        return None;
    }
    let text = extract(a.raw_response.as_deref()?);
    match a.arm {
        Arm::Mermaid => Some(text),
        Arm::Voxdiagram => vd_verdict(&text).exported,
    }
}

/// `oracle-inputs.jsonl`: one line per oracle text, in recording order, each with its task's
/// expected sets.
pub fn oracle_inputs(
    tasks: &[Task],
    reference_outputs: &[OracleOutput],
    lines: &[Line],
) -> Result<String> {
    let expected = expected_sets(tasks, reference_outputs)?;
    let mut out = Vec::new();
    for a in attempts(lines) {
        let exp = expected
            .get(&a.task_id)
            .with_context(|| format!("attempt for unknown task {}", a.task_id))?;
        if let Some(text) = oracle_text(a) {
            out.push(OracleInput {
                id: a.oracle_id(),
                text,
                expected: Some((*exp).clone()),
            });
        }
    }
    write_jsonl(&out)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::commands::model::eval_diagram::{Stop, read_jsonl};
    use serde_json::json;

    pub(crate) const VD_OK: &str =
        "```vox-diagram\ngraph {\n    node a: \"A\"\n    node b: \"B\"\n    a -> b\n}\n```";
    pub(crate) const VD_BAD: &str =
        "```vox-diagram\ngraph {\n    node a: \"A\"\n    a -> b\n}\n```";
    pub(crate) const MM: &str = "```mermaid\nflowchart TD\n  a[\"A\"] --> b[\"B\"]\n```";

    pub(crate) fn attempt(
        task: &str,
        arm: Arm,
        slot: &str,
        turn: u32,
        raw: Option<&str>,
    ) -> Attempt {
        Attempt {
            task_id: task.into(),
            arm,
            slot: slot.into(),
            turn,
            seed: 1,
            requested_model: "stub".into(),
            resolved_model: Some("stub".into()),
            feedback: None,
            raw_response: raw.map(Into::into),
            prompt_tokens: 10,
            completion_tokens: 5,
            cost_usd: 0.001,
            latency_ms: 7,
            transport_error: None,
            retries: 0,
            finish_reason: None,
            reasoning_tokens: 0,
        }
    }

    pub(crate) fn task(id: &str) -> Task {
        Task {
            id: id.into(),
            kind: "graph".into(),
            prompt: "p".into(),
            reference: String::new(),
        }
    }

    pub(crate) fn reference(id: &str) -> OracleOutput {
        OracleOutput {
            id: id.into(),
            ok: true,
            error: None,
            sets: json!({"kind": "graph", "direction": "down", "nodes": [], "edges": [], "containers": []}),
            matched: None,
        }
    }

    #[test]
    fn oracle_inputs_cover_mermaid_replies_and_valid_voxdiagram_exports_in_order() {
        let mut lost = attempt("t2", Arm::Mermaid, "A", 0, None);
        lost.transport_error = Some("timeout".into());
        let lines = vec![
            Line::Attempt(attempt("t1", Arm::Voxdiagram, "A", 0, Some(VD_OK))),
            Line::Attempt(attempt("t1", Arm::Mermaid, "A", 0, Some(MM))),
            Line::Attempt(attempt("t2", Arm::Voxdiagram, "A", 0, Some(VD_BAD))),
            Line::Attempt(lost),
            Line::Attempt(attempt("t2", Arm::Mermaid, "B", 1, Some("not fenced"))),
        ];
        let out = oracle_inputs(
            &[task("t1"), task("t2")],
            &[reference("t1"), reference("t2")],
            &lines,
        )
        .unwrap();
        let got: Vec<OracleInput> = read_jsonl(&out).unwrap();
        let ids: Vec<&str> = got.iter().map(|o| o.id.as_str()).collect();
        assert_eq!(
            ids,
            ["t1:A:voxdiagram:0", "t1:A:mermaid:0", "t2:B:mermaid:1"]
        );
        assert!(got[0].text.starts_with("flowchart"), "{}", got[0].text);
        assert_eq!(got[1].text, "flowchart TD\n  a[\"A\"] --> b[\"B\"]");
        assert_eq!(got[2].text, "not fenced");
        assert!(
            got.iter()
                .all(|o| o.expected.as_ref() == Some(&reference("t1").sets))
        );
    }

    #[test]
    fn oracle_inputs_refuse_unknown_tasks_and_unsupported_references() {
        let lines = vec![Line::Attempt(attempt("zz", Arm::Mermaid, "A", 0, Some(MM)))];
        assert!(oracle_inputs(&[task("t1")], &[reference("t1")], &lines).is_err());
        let mut bad = reference("t1");
        bad.sets = json!({"unsupported": "x"});
        let err = oracle_inputs(&[task("t1")], &[bad], &[]).unwrap_err();
        assert!(err.to_string().contains("t1"), "{err}");
    }

    pub(crate) fn header() -> Line {
        let h = r#"{"type":"run","schema_version":1,"synthetic":true,"freeze_commit":"x","tasks_hash":"x","card_hashes":{},"prompt_template_hash":"x","intent":"x","scope":"x","selection":null,"models":[],"temperature":0.0,"seed_base":42,"max_tokens":1,"max_spend_usd":0.0,"mermaid_version":"x","vox_version":"x","probes":[],"started_at":"x"}"#;
        serde_json::from_str(h).unwrap()
    }

    fn mm_out(id: &str, error: Option<&str>) -> OracleOutput {
        OracleOutput {
            id: id.into(),
            ok: error.is_none(),
            error: error.map(Into::into),
            sets: json!(null),
            matched: None,
        }
    }

    fn vd_feedback(raw: &str) -> String {
        vd_verdict(&extract(raw)).feedback
    }

    /// t1: VoxDiagram invalid then repaired at turn 1; Mermaid invalid at 0, 1 and 2.
    fn repaired() -> (Vec<Line>, Vec<OracleOutput>) {
        let mut vd1 = attempt("t1", Arm::Voxdiagram, "A", 1, Some(VD_OK));
        vd1.feedback = Some(vd_feedback(VD_BAD));
        let mut mm1 = attempt("t1", Arm::Mermaid, "A", 1, Some(MM));
        mm1.feedback = Some("err0".into());
        let mut mm2 = attempt("t1", Arm::Mermaid, "A", 2, Some(MM));
        mm2.feedback = Some("err1".into());
        let lines = vec![
            header(),
            Line::Attempt(attempt("t1", Arm::Voxdiagram, "A", 0, Some(VD_BAD))),
            Line::Attempt(attempt("t1", Arm::Mermaid, "A", 0, Some(MM))),
            Line::Attempt(vd1),
            Line::Attempt(mm1),
            Line::Attempt(mm2),
        ];
        let outs = vec![
            mm_out("t1:A:mermaid:0", Some("err0")),
            mm_out("t1:A:mermaid:1", Some("err1")),
            mm_out("t1:A:mermaid:2", Some("err2")),
        ];
        (lines, outs)
    }

    fn edit(lines: &mut [Line], i: usize, f: impl FnOnce(&mut Attempt)) {
        let Line::Attempt(a) = &mut lines[i] else {
            panic!("line {i} is not an attempt")
        };
        f(a);
    }

    #[test]
    fn check_recording_accepts_a_consistent_recording() {
        let (lines, outs) = repaired();
        check_recording(&[task("t1")], &lines, &outs).unwrap();
    }

    #[test]
    fn check_recording_rejects_a_repair_after_a_valid_turn() {
        let (mut lines, outs) = repaired();
        edit(&mut lines, 1, |a| a.raw_response = Some(VD_OK.into()));
        let err = check_recording(&[task("t1")], &lines, &outs).unwrap_err();
        assert!(err.to_string().contains("t1:A:voxdiagram:1"), "{err}");
    }

    #[test]
    fn check_recording_rejects_a_missing_previous_turn_and_a_turn_above_two() {
        let (mut lines, outs) = repaired();
        lines.remove(4); // Mermaid turn 1: turn 2 now has no turn 1
        assert!(check_recording(&[task("t1")], &lines, &outs).is_err());
        let (mut lines, outs) = repaired();
        edit(&mut lines, 5, |a| a.turn = 3);
        let err = check_recording(&[task("t1")], &lines, &outs).unwrap_err();
        assert!(err.to_string().contains("turn above 2"), "{err}");
    }

    #[test]
    fn check_recording_rejects_edited_feedback_and_names_the_expected_text() {
        let (mut lines, outs) = repaired();
        edit(&mut lines, 4, |a| a.feedback = Some("err1".into()));
        let err = check_recording(&[task("t1")], &lines, &outs).unwrap_err();
        assert!(err.to_string().contains("expected \"err0\""), "{err}");
        let (mut lines, outs) = repaired();
        edit(&mut lines, 3, |a| a.feedback = Some("[]".into()));
        assert!(check_recording(&[task("t1")], &lines, &outs).is_err());
    }

    #[test]
    fn check_recording_rejects_duplicates_and_unknown_tasks() {
        let (mut lines, outs) = repaired();
        lines.push(lines[2].clone());
        let err = check_recording(&[task("t1")], &lines, &outs).unwrap_err();
        assert!(err.to_string().contains("duplicate"), "{err}");
        let (lines, outs) = repaired();
        assert!(check_recording(&[task("t2")], &lines, &outs).is_err());
    }

    #[test]
    fn check_recording_requires_the_next_repair_turn_unless_the_run_stopped() {
        let (mut lines, outs) = repaired();
        lines.remove(5); // Mermaid turn 1 invalid, turn 2 missing
        let err = check_recording(&[task("t1")], &lines, &outs).unwrap_err();
        assert!(err.to_string().contains("t1:A:mermaid:1"), "{err}");
        lines.push(Line::Stop(Stop {
            reason: "spend cap".into(),
            spent_usd: 25.0,
        }));
        check_recording(&[task("t1")], &lines, &outs).unwrap();
    }

    #[test]
    fn check_recording_needs_no_repair_after_a_transport_error() {
        let mut lost = attempt("t1", Arm::Voxdiagram, "A", 0, None);
        lost.transport_error = Some("reset".into());
        let lines = vec![header(), Line::Attempt(lost)];
        check_recording(&[task("t1")], &lines, &[]).unwrap();
    }

    #[test]
    fn check_recording_wants_the_header_first_and_the_stop_last() {
        let (mut lines, outs) = repaired();
        lines.remove(0);
        assert!(check_recording(&[task("t1")], &lines, &outs).is_err());
        let (mut lines, outs) = repaired();
        lines.insert(
            1,
            Line::Stop(Stop {
                reason: "x".into(),
                spent_usd: 0.0,
            }),
        );
        assert!(check_recording(&[task("t1")], &lines, &outs).is_err());
    }
}
