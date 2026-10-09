//! Pure join: committed inputs -> [`CapabilityIndex`]. No IO, no clock.

use anyhow::Result;
use vox_capability_registry::CapabilityIndex;

use super::inputs::Inputs;

pub(crate) fn build_index(_inputs: &Inputs) -> Result<CapabilityIndex> {
    // RED stub: replaced in the GREEN commit.
    Ok(CapabilityIndex {
        schema_version: 1,
        generator: "vox ci capability-index".to_string(),
        coverage_snapshot: None,
        domains: Vec::new(),
        rows: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::super::inputs::{OperationRow, test_op};
    use super::*;
    use vox_capability_registry::RowKind;

    fn inputs(ops: Vec<OperationRow>) -> Inputs {
        Inputs { ops }
    }

    fn ids(idx: &CapabilityIndex) -> Vec<&str> {
        idx.rows.iter().map(|r| r.id.as_str()).collect()
    }

    #[test]
    fn builds_one_row_per_projection() {
        let idx = build_index(&inputs(vec![
            test_op("a.b", Some((&["a", "b"], "active")), None, "cli only"),
            test_op("x", None, Some("vox_x"), "mcp only"),
            test_op("p", Some((&["p"], "active")), Some("vox_p"), "paired"),
        ]))
        .expect("build");
        assert_eq!(
            ids(&idx),
            vec!["cli.a.b", "cli.p", "mcp.vox_p", "mcp.vox_x"]
        );
        let cli_p = idx.row("cli.p").expect("cli.p");
        assert_eq!(cli_p.kind, RowKind::Cli);
        assert_eq!(cli_p.equivalents, vec!["mcp.vox_p"]);
        assert_eq!(
            idx.row("mcp.vox_p").expect("mcp.vox_p").equivalents,
            vec!["cli.p"]
        );
        assert!(idx.row("cli.a.b").expect("cli.a.b").equivalents.is_empty());
        assert!(
            idx.row("mcp.vox_x")
                .expect("mcp.vox_x")
                .equivalents
                .is_empty()
        );
        let a = idx.row("cli.a.b").expect("cli.a.b");
        assert_eq!(a.cli.as_ref().expect("cli ref").invocation, "vox a b");
        let cov = a.coverage.as_ref().expect("coverage object");
        assert!(cov.kind.is_none());
        assert!(cov.reason.as_deref().is_some_and(|r| !r.is_empty()));
    }

    #[test]
    fn non_active_cli_row_keeps_status() {
        let idx = build_index(&inputs(vec![test_op(
            "old",
            Some((&["old"], "deprecated")),
            None,
            "d",
        )]))
        .expect("build");
        assert_eq!(idx.rows.len(), 1);
        assert_eq!(idx.rows[0].status.as_deref(), Some("deprecated"));
    }

    #[test]
    fn duplicate_id_is_an_error() {
        let err = build_index(&inputs(vec![
            test_op("one", Some((&["dup", "x"], "active")), None, "d"),
            test_op("two", Some((&["dup", "x"], "active")), None, "d"),
        ]))
        .expect_err("duplicate ids must fail");
        assert!(err.to_string().contains("cli.dup.x"), "got: {err}");
    }

    #[test]
    fn deterministic_and_sorted() {
        let make = || {
            inputs(vec![
                test_op("z", Some((&["z"], "active")), Some("vox_z"), "d"),
                test_op("a", Some((&["a"], "active")), Some("vox_a"), "d"),
            ])
        };
        let one =
            serde_json::to_string_pretty(&build_index(&make()).expect("build")).expect("json");
        let two =
            serde_json::to_string_pretty(&build_index(&make()).expect("build")).expect("json");
        assert_eq!(one, two);
        let idx = build_index(&make()).expect("build");
        assert_eq!(ids(&idx), vec!["cli.a", "cli.z", "mcp.vox_a", "mcp.vox_z"]);
    }

    #[test]
    fn summary_is_one_line() {
        let idx = build_index(&inputs(vec![test_op(
            "s",
            Some((&["s"], "active")),
            None,
            "line one\nline\ttwo\r\nthree \u{7}bell",
        )]))
        .expect("build");
        assert_eq!(idx.rows.len(), 1);
        let summary = idx.rows[0].summary.as_deref().expect("summary");
        assert!(!summary.chars().any(char::is_control), "got: {summary:?}");
        assert_eq!(summary, "line one line two three bell");
    }
}
