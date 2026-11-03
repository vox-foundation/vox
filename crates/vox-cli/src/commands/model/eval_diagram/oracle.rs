//! VoxDiagram oracle: `vox_diagram::check` (via `validate`) decides validity, its diagnostics are
//! the repair feedback, and a valid diagram's Mermaid export feeds the shared fidelity path.
//!
//! The model repairs, never the tool: `repair` / `apply_fixes` are not called here, or the
//! "after repair" endpoint would measure the fixer instead of the agent.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use vox_doc_pipeline::diagram::{Emit, ValidateOpts, validate};

use super::{Arm, Attempt, OracleOutput};
use crate::commands::model::eval_corpus::extract_vox_code;

/// Verdict of one VoxDiagram text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VdVerdict {
    pub valid: bool,
    /// Compact JSON of the diagnostics (`[]` when there are none): the next turn's feedback.
    pub feedback: String,
    /// Mermaid export of a valid diagram.
    pub exported: Option<String>,
}

/// The text both arms score: the first fenced block of a reply, else the trimmed reply.
#[must_use]
pub fn extract(raw: &str) -> String {
    extract_vox_code(raw)
}

/// Judge an extracted VoxDiagram text.
#[must_use]
pub fn vd_verdict(text: &str) -> VdVerdict {
    let v = validate(
        text,
        ValidateOpts {
            from_mermaid: false,
            emit: Some(Emit::Mermaid),
        },
    );
    VdVerdict {
        valid: v.valid,
        feedback: serde_json::to_string(&v.diagnostics)
            .unwrap_or_else(|e| format!("[serialize: {e}]")),
        exported: v.emitted.filter(|_| v.valid),
    }
}

/// Oracle outputs by line id.
pub type OracleMap<'a> = BTreeMap<&'a str, &'a OracleOutput>;

/// Index oracle outputs by id.
#[must_use]
pub fn oracle_map(outputs: &[OracleOutput]) -> OracleMap<'_> {
    outputs.iter().map(|o| (o.id.as_str(), o)).collect()
}

/// What the oracles say about one attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Judged {
    /// No reply: an infra failure, never an invalid diagram.
    Transport,
    Valid,
    /// Invalid; the text is the next turn's feedback (VoxDiagram diagnostics JSON, Mermaid error).
    Invalid(String),
}

/// Judge an attempt: `check` for VoxDiagram, the recorded Node oracle output for Mermaid.
pub fn judge(a: &Attempt, oracle: &OracleMap<'_>) -> Result<Judged> {
    if a.transport_error.is_some() {
        return Ok(Judged::Transport);
    }
    let id = a.oracle_id();
    let raw = a
        .raw_response
        .as_deref()
        .with_context(|| format!("{id}: no reply"))?;
    Ok(match a.arm {
        Arm::Voxdiagram => {
            let v = vd_verdict(&extract(raw));
            if v.valid {
                Judged::Valid
            } else {
                Judged::Invalid(v.feedback)
            }
        }
        Arm::Mermaid => {
            let o = oracle
                .get(id.as_str())
                .with_context(|| format!("no oracle output for {id}"))?;
            match (o.ok, &o.error) {
                (true, _) => Judged::Valid,
                (false, Some(e)) => Judged::Invalid(e.clone()),
                (false, None) => bail!("{id}: oracle output is not ok and has no error"),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::model::eval_diagram::score::tests::{MM, VD_BAD, attempt};
    use vox_doc_pipeline::diagram::check;

    fn out(id: &str, ok: bool, error: Option<&str>) -> OracleOutput {
        let sets = serde_json::Value::Null;
        OracleOutput {
            id: id.into(),
            ok,
            error: error.map(Into::into),
            sets,
            matched: None,
        }
    }

    #[test]
    fn judge_uses_check_for_voxdiagram_and_the_recorded_oracle_for_mermaid() {
        let vd = attempt("t", Arm::Voxdiagram, "A", 0, Some(VD_BAD));
        let fb = vd_verdict(&extract(VD_BAD)).feedback;
        assert_eq!(judge(&vd, &BTreeMap::new()).unwrap(), Judged::Invalid(fb));
        let mm = attempt("t", Arm::Mermaid, "A", 0, Some(MM));
        let outs = [out("t:A:mermaid:0", false, Some("Parse error"))];
        assert_eq!(
            judge(&mm, &oracle_map(&outs)).unwrap(),
            Judged::Invalid("Parse error".into())
        );
        assert!(
            judge(&mm, &BTreeMap::new()).is_err(),
            "a reply without an oracle output"
        );
        let outs = [out("t:A:mermaid:0", false, None)];
        assert!(
            judge(&mm, &oracle_map(&outs)).is_err(),
            "not ok without an error text"
        );
        let mut lost = attempt("t", Arm::Mermaid, "A", 0, None);
        lost.transport_error = Some("reset".into());
        assert_eq!(judge(&lost, &BTreeMap::new()).unwrap(), Judged::Transport);
    }

    #[test]
    fn valid_graph_has_empty_feedback_and_an_export() {
        let v = vd_verdict("graph {\n    node a: \"A\"\n    node b: \"B\"\n    a -> b\n}\n");
        assert!(v.valid);
        assert_eq!(v.feedback, "[]");
        assert!(
            v.exported
                .as_deref()
                .is_some_and(|e| e.starts_with("flowchart")),
            "{v:?}"
        );
    }

    #[test]
    fn undeclared_node_is_invalid_with_check_diagnostics_as_feedback() {
        let src = "graph {\n    node a: \"A\"\n    a -> b\n}\n";
        let v = vd_verdict(src);
        assert!(!v.valid);
        assert_eq!(
            v.feedback,
            serde_json::to_string(&check(src).diagnostics).unwrap()
        );
        assert!(
            v.feedback.contains("\"severity\":\"error\""),
            "{}",
            v.feedback
        );
        assert!(v.exported.is_none());
    }

    #[test]
    fn extract_takes_the_first_fence_of_any_info_string() {
        assert_eq!(
            extract("Here:\n```mermaid\nflowchart TD\n  a --> b\n```\nok"),
            "flowchart TD\n  a --> b"
        );
        assert_eq!(extract("  graph {}  "), "graph {}");
    }
}
