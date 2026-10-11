use super::*;
use std::path::PathBuf;

pub(super) const TAXONOMY: &str =
    include_str!("../../../../contracts/documentation/kb-taxonomy.v1.yaml");

pub(super) fn tax() -> Taxonomy {
    Taxonomy::from_yaml_str(TAXONOMY).unwrap()
}

fn ids(doc: &str) -> Vec<&'static str> {
    facet_issues(doc, &tax())
        .iter()
        .map(|(_, i)| i.rule_id())
        .collect()
}

#[test]
fn facet_lint_rel_path_scopes_to_authored_md_under_docs_src() {
    let root = PathBuf::from("/r");
    let rel = |p: &str| facet_lint_rel_path(&root.join(p), &root);
    assert_eq!(rel("docs/src/x/y.md").as_deref(), Some("docs/src/x/y.md"));
    assert_eq!(rel("docs/src/archive/z.md"), None);
    assert_eq!(rel("docs/src/a/archive/z.md"), None);
    assert_eq!(rel("docs/src/reference/cli.generated.md"), None);
    assert_eq!(rel("docs/src/index.mdx"), None);
    assert_eq!(rel("README.md"), None);
    assert!(rel("docs/src/archive-notes.md").is_some());
}

#[test]
fn facet_issues_unfaceted_doc_reports_the_four_required_ids() {
    let doc = "---\ntitle: \"T\"\n---\n# T\n\nbody\n";
    assert_eq!(
        ids(doc),
        [
            "facet-missing-topic",
            "facet-missing-domain",
            "facet-missing-authorship",
            "facet-missing-summary"
        ]
    );
}

#[test]
fn facet_issues_invalid_value_is_parse_error_only() {
    assert_eq!(
        ids("---\nauthorship: robot\n---\n# T\n"),
        ["facet-parse-error"]
    );
}
