//! Doc-lint validation of ```` ```vox-diagram ```` fences: every exact-info fence goes through
//! `vox_diagram::check` and each diagnostic is reported at file line and column.

use std::path::Path;

use vox_diagram::diag::Severity;
use vox_diagram::fence;

use crate::pipeline::types::{DiagramIssue, LintError, LintKind};

/// The only info string the scanner (and the renderer) accept.
const INFO: &str = "vox-diagram";

/// Every problem in the `vox-diagram` fences of `md`, ordered by file line.
pub(crate) fn fence_issues(md: &str) -> Vec<DiagramIssue> {
    let mut out = Vec::new();
    for f in fence::extract(md, INFO) {
        let open = f.open_line as usize;
        for d in vox_diagram::check(&f.body).diagnostics {
            out.push(DiagramIssue::Check {
                line: open + d.span.line as usize,
                col: d.span.col as usize,
                code: d.code,
                message: d.message,
                error: d.severity == Severity::Error,
            });
        }
    }
    out.sort_by_key(DiagramIssue::line);
    out
}

/// Append one [`LintError`] per [`fence_issues`] finding of `md`.
pub(crate) fn check_diagram_fences(path: &Path, md: &str, errors: &mut Vec<LintError>) {
    for issue in fence_issues(md) {
        errors.push(LintError {
            file: path.to_owned(),
            line: issue.line(),
            kind: LintKind::Diagram(issue),
        });
    }
}

impl DiagramIssue {
    fn line(&self) -> usize {
        match self {
            Self::Check { line, .. } | Self::InfoString { line, .. } => *line,
        }
    }

    /// The diagnostic code, or `diagram-fence-info`.
    pub(crate) fn rule_id(&self) -> &'static str {
        match self {
            Self::Check { code, .. } => code,
            Self::InfoString { .. } => "diagram-fence-info",
        }
    }

    /// Errors and info-string problems block; warnings only print.
    pub(crate) fn is_hard(&self) -> bool {
        match self {
            Self::Check { error, .. } => *error,
            Self::InfoString { .. } => true,
        }
    }

    pub(crate) fn workflow(&self) -> &'static str {
        match self {
            Self::Check { .. } => {
                "fix the ```vox-diagram fence at the reported line:col (syntax: crates/vox-diagram/grammar-card.txt)"
            }
            Self::InfoString { .. } => "use exactly ```vox-diagram as the fence info string",
        }
    }

    /// `{rel}:{line}:{col}: {code}: {message}` with `/` separators in `rel`.
    pub(crate) fn render(&self, rel: &Path) -> String {
        let rel = rel.to_string_lossy().replace('\\', "/");
        match self {
            Self::Check {
                line,
                col,
                code,
                message,
                ..
            } => format!("{rel}:{line}:{col}: {code}: {message}"),
            Self::InfoString { line, info } => format!(
                "{rel}:{line}:1: diagram-fence-info: info string {info:?} is not exactly \
                 `vox-diagram`; only an exact `vox-diagram` info string is validated and rendered"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::lint_in;
    use crate::pipeline::tests::{BASE_FM, write_facet_contracts, write_readme_sync_fixture};

    const BROKEN: &str = "```vox-diagram\ngraph {\n    node a: \"A\"\n    a -> b\n}\n```\n";
    const FIXED: &str =
        "```vox-diagram\ngraph {\n    node a: \"A\"\n    node b: \"B\"\n    a -> b\n}\n```\n";

    /// A facet-clean doc whose first fence opens on line 15.
    fn doc(fence: &str) -> String {
        let words = vec!["word"; 40].join(" ");
        format!(
            "---\n{BASE_FM}\ntopic: search-retrieval\ndomain: knowledge\nauthorship: human\n---\n# Doc\n\n<!-- kb:summary -->\n{words}\n<!-- /kb:summary -->\n\n{fence}"
        )
    }

    pub(super) fn lint_doc(name: &str, text: &str) -> i32 {
        let r =
            std::env::temp_dir().join(format!("vox-doc-diagrams-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&r);
        std::fs::create_dir_all(r.join("docs/src")).unwrap();
        write_facet_contracts(&r, " []");
        write_readme_sync_fixture(&r, "Same text.", "Same text.");
        std::fs::write(r.join("docs/src/diag.md"), text).unwrap();
        let code = lint_in(&r, &["--paths=diag.md".to_string()]);
        std::fs::remove_dir_all(&r).unwrap();
        code
    }

    #[test]
    fn tracer_broken_fence_fails_lint_with_file_line_col() {
        let text = doc(BROKEN);
        let issues = fence_issues(&text);
        assert_eq!(issues.len(), 1, "{issues:?}");
        let DiagramIssue::Check {
            line,
            col,
            code,
            error,
            ..
        } = &issues[0]
        else {
            panic!("expected Check, got {:?}", issues[0]);
        };
        assert_eq!(
            (*line, *col, *code, *error),
            (18, 10, "diagram/undeclared-node", true)
        );
        let out = issues[0].render(Path::new("docs/src/diag.md"));
        assert!(
            out.starts_with("docs/src/diag.md:18:10: diagram/undeclared-node: "),
            "{out}"
        );
        assert_eq!(lint_doc("tracer", &text), 1);
    }

    #[test]
    fn valid_fence_passes_lint() {
        let text = doc(FIXED);
        assert!(fence_issues(&text).is_empty());
        assert_eq!(lint_doc("valid", &text), 0);
    }
}
