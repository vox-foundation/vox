//! VoxDiagram oracle: `vox_diagram::check` (via `validate`) decides validity, its diagnostics are
//! the repair feedback, and a valid diagram's Mermaid export feeds the shared fidelity path.
//!
//! The model repairs, never the tool: `repair` / `apply_fixes` are not called here, or the
//! "after repair" endpoint would measure the fixer instead of the agent.

use vox_doc_pipeline::diagram::{Emit, ValidateOpts, validate};

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

#[cfg(test)]
mod tests {
    use super::*;
    use vox_doc_pipeline::diagram::check;

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
