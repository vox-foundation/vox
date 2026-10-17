//! Doc-lint validation of ```` ```vox-diagram ```` fences: every exact-info fence goes through
//! `vox_diagram::check` and each diagnostic is reported at file line and column.

use std::path::Path;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag};
use vox_diagram::diag::Severity;
use vox_diagram::fence;

use crate::pipeline::types::{DiagramIssue, LintError, LintKind};

/// The only info string the scanner (and the renderer) accept.
const INFO: &str = "vox-diagram";

/// Every problem in the `vox-diagram` fences of `md`, ordered by file line.
pub(crate) fn fence_issues(md: &str) -> Vec<DiagramIssue> {
    let mut out = Vec::new();
    let lines: Vec<&str> = md.lines().collect();
    for f in fence::extract(md, INFO) {
        let open = f.open_line as usize;
        let body_lines: Vec<&str> = f.body.lines().collect();
        for d in vox_diagram::check(&f.body).diagnostics {
            let line = open + d.span.line as usize;
            // Container prefix ("  " of a list item, "> " of a quote) the scanner stripped.
            let prefix = match (
                lines.get(line - 1),
                body_lines.get((d.span.line as usize).saturating_sub(1)),
            ) {
                (Some(raw), Some(body)) if !body.is_empty() && raw.ends_with(body) => {
                    raw.chars().count() - body.chars().count()
                }
                _ => 0,
            };
            out.push(DiagramIssue::Check {
                line,
                col: d.span.col as usize + prefix,
                code: d.code,
                message: d.message,
                error: d.severity == Severity::Error,
            });
        }
    }
    out.extend(
        near_miss_fences(md, INFO)
            .into_iter()
            .map(|(line, info)| DiagramIssue::InfoString { line, info }),
    );
    out.sort_by_key(DiagramIssue::line);
    out
}

/// `(1-based opener line, trimmed info)` of every fenced block whose first info token is `lang`
/// but whose info string is not exactly `lang`. Uses the parser options of `fence::extract`, so
/// both agree on nesting.
pub(crate) fn near_miss_fences(md: &str, lang: &str) -> Vec<(usize, String)> {
    let parser = Parser::new_ext(md, Options::ENABLE_YAML_STYLE_METADATA_BLOCKS).into_offset_iter();
    parser
        .filter_map(|(event, range)| match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                let info = info.trim();
                let near = info != lang && info.split([',', ' ', '\t']).next() == Some(lang);
                near.then(|| {
                    (
                        md[..range.start].matches('\n').count() + 1,
                        info.to_string(),
                    )
                })
            }
            _ => None,
        })
        .collect()
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

    fn fixture_fence(src: &str) -> String {
        format!("```vox-diagram\n{}\n```\n", src.trim_end())
    }

    fn checks(md: &str) -> Vec<(usize, usize, &'static str, bool)> {
        fence_issues(md)
            .into_iter()
            .filter_map(|i| match i {
                DiagramIssue::Check {
                    line,
                    col,
                    code,
                    error,
                    ..
                } => Some((line, col, code, error)),
                DiagramIssue::InfoString { .. } => None,
            })
            .collect()
    }

    #[test]
    fn doctest_runner_never_compiles_diagram_fences() {
        use crate::pipeline::doctest::check_doctests;
        let body = "graph {\n    node a: \"A\"\n    a -> b\n}\n";
        let p = Path::new("docs/src/x.md");
        let mut errs = Vec::new();
        let diagrams = format!("```vox-diagram\n{body}```\n\n```vox-diagram,foo\n{body}```\n");
        check_doctests(p, &diagrams, &mut errs);
        assert!(errs.is_empty(), "{errs:?}");
        check_doctests(p, &format!("```vox\n{body}```\n"), &mut errs);
        assert_eq!(errs.len(), 1, "control ```vox fence must be rejected");
        assert!(matches!(errs[0].kind, LintKind::DocTestFailed { .. }));
    }

    #[test]
    fn near_miss_info_strings_are_hard_errors() {
        let md = "```vox-diagram,foo\nx\n```\n\n```vox-diagram foo\ny\n```\n\n```  vox-diagram   \ngraph {\n    node a: \"A\"\n}\n```\n\n```vox-diagrams\nz\n```\n\n```vox\nw\n```\n";
        let issues = fence_issues(md);
        let infos: Vec<_> = issues
            .iter()
            .filter_map(|i| match i {
                DiagramIssue::InfoString { line, info } => Some((*line, info.as_str())),
                DiagramIssue::Check { .. } => None,
            })
            .collect();
        assert_eq!(infos, [(1, "vox-diagram,foo"), (5, "vox-diagram foo")]);
        assert_eq!(
            issues.len(),
            2,
            "padded fence is valid, others ignored: {issues:?}"
        );
        assert!(issues.iter().all(DiagramIssue::is_hard));
        let out = issues[0].render(Path::new("docs/src/x.md"));
        assert!(
            out.starts_with("docs/src/x.md:1:1: diagram-fence-info: "),
            "{out}"
        );
        // The scanner the renderer uses skips both near misses, so the lint must reject them.
        assert_eq!(fence::extract(md, INFO).len(), 1);
        assert_eq!(near_miss_fences(md, "mermaid"), []);
    }

    #[test]
    fn warnings_print_but_do_not_block() {
        let fx = include_str!("../../../vox-diagram/tests/fixtures/broken/18-shadowed-id.vdg");
        let text = doc(&fixture_fence(fx));
        let c = checks(&text);
        assert_eq!(c.len(), 1, "{c:?}");
        assert_eq!((c[0].2, c[0].3), ("diagram/shadowed-id", false));
        assert!(!fence_issues(&text)[0].is_hard());
        assert_eq!(lint_doc("warn", &text), 0);
    }

    #[test]
    fn every_error_in_a_fence_is_reported() {
        let fx = include_str!("../../../vox-diagram/tests/fixtures/broken/01-two-independent.vdg");
        let c = checks(&fixture_fence(fx));
        assert_eq!(c.len(), 2, "{c:?}");
        assert!(c.iter().all(|x| x.3));
        assert_ne!(c[0].0, c[1].0);
    }

    #[test]
    fn nested_fences_map_container_columns() {
        let list = "- item\n\n  ```vox-diagram\n  graph {\n      node a: \"A\"\n      a -> b\n  }\n  ```\n";
        let quote = "> ```vox-diagram\n> graph {\n>     node a: \"A\"\n>     a -> b\n> }\n> ```\n";
        assert_eq!(checks(list), [(6, 12, "diagram/undeclared-node", true)]);
        assert_eq!(checks(quote), [(4, 12, "diagram/undeclared-node", true)]);
    }

    #[test]
    fn fences_inside_four_backtick_examples_are_not_validated() {
        let md = format!("````markdown\n{BROKEN}````\n");
        assert!(fence_issues(&md).is_empty());
    }

    #[test]
    fn oversized_fence_is_one_limit_source_error() {
        let md = fixture_fence(&"x".repeat(70_000));
        let c = checks(&md);
        assert_eq!(c.len(), 1, "{c:?}");
        assert_eq!((c[0].2, c[0].3), ("diagram/limit-source", true));
    }

    #[test]
    fn committed_docs_diagram_fences_are_valid() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/src");
        let mut seen = 0;
        for e in walkdir::WalkDir::new(&root)
            .into_iter()
            .filter_map(Result::ok)
        {
            let p = e.path();
            let md = p.extension().is_some_and(|x| x == "md" || x == "mdx");
            if !md
                || p.strip_prefix(&root)
                    .is_ok_and(|r| r.starts_with("archive"))
            {
                continue;
            }
            let text =
                std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            let hard: Vec<_> = (fence_issues(&text).iter())
                .filter(|i| i.is_hard())
                .map(|i| i.render(p))
                .collect();
            assert!(hard.is_empty(), "{hard:?}");
            seen += 1;
        }
        assert!(seen > 0, "no docs found under {}", root.display());
    }
}
