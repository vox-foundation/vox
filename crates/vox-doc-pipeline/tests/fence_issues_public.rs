//! `fence_issues` is reachable from outside the crate (the self-heal bot's pre-PR check
//! calls it at the next merge of `main`; Phase 87 SC3).

use vox_doc_pipeline::pipeline::fence_issues;
use vox_doc_pipeline::pipeline::types::DiagramIssue;

#[test]
fn fence_issues_is_public_and_reports_broken_fences() {
    let broken = "# T\n\n```vox-diagram\ngraph {\n    a -> ghost\n}\n```\n";
    let issues = fence_issues(broken);
    assert!(
        issues.iter().any(
            |i| matches!(i, DiagramIssue::Check { code, .. } if *code == "diagram/undeclared-node")
        ),
        "expected an undeclared-node issue, got {issues:?}"
    );

    let valid = "# T\n\n```vox-diagram\ngraph {\n    node a: \"A\"\n}\n```\n";
    assert!(fence_issues(valid).is_empty());
}
