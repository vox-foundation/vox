//! Recording -> oracle inputs: which texts the Node Mermaid oracle must judge.

use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};

use super::oracle::{extract, vd_verdict};
use super::{Arm, Attempt, Line, OracleInput, OracleOutput, Task, write_jsonl};

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
    use crate::commands::model::eval_diagram::read_jsonl;
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
}
