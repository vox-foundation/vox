//! TAX-06 / D-05 / D-06: the MENS doc extractor shares one eligibility rule with the facet lint
//! and never emits page-one summary-block text.

use super::*;
use std::fs;

const BODY: &str = "# Title\n\n## Concept section\n\nThis paragraph is long enough to clear the minimum section length so the \
section is emitted as a question and answer pair by the extractor under test.\n\n```vox\nfn add(a: int, b: int) -> int {\n    return a + b\n}\n```\n";

fn doc(frontmatter: &str) -> String {
    format!("---\n{frontmatter}\n---\n{BODY}")
}

fn extract(content: &str) -> Vec<DocTrainingPair> {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("page.md");
    fs::write(&p, content).unwrap();
    extract_from_md_file(&p, &ExtractDocsConfig::default()).unwrap()
}

#[test]
fn web_promoted_page_with_flag_true_yields_no_pairs() {
    let fm = "title: T\norigin: web-promoted\ntraining_eligible: true";
    assert!(extract(&doc(fm)).is_empty());
}

#[test]
fn web_promoted_page_with_attestation_yields_pairs() {
    let fm = "title: T\norigin: web-promoted\ntraining_eligible: true\nhuman_attested_by: alice";
    assert!(!extract(&doc(fm)).is_empty());
}

#[test]
fn llm_authorship_without_attestation_yields_no_pairs() {
    for a in ["llm-generated", "llm-assisted"] {
        assert!(
            extract(&doc(&format!("title: T\nauthorship: {a}"))).is_empty(),
            "{a}"
        );
    }
}

#[test]
fn human_authorship_and_absent_flag_yield_pairs() {
    assert!(!extract(&doc("title: T\nauthorship: human")).is_empty());
    assert!(!extract(&doc("title: T")).is_empty());
}

#[test]
fn explicit_flag_false_without_authorship_is_excluded() {
    assert!(extract(&doc("title: T\ntraining_eligible: false")).is_empty());
}

#[test]
fn facet_parse_error_excludes_doc() {
    assert!(extract(&doc("title: T\nauthorship: robot")).is_empty());
    assert!(extract(&doc("title: T\norigin: web-promotd")).is_empty());
}

#[test]
fn deprecated_status_still_short_circuits() {
    assert!(extract(&doc("title: T\nstatus: deprecated")).is_empty());
}

#[test]
fn staleness_penalty_still_parsed_from_typed_frontmatter() {
    let content = doc("title: Old\nlast_updated: 2020-01-01");
    let (fm, eligible, penalty) = parse_frontmatter(&content, Path::new("page.md"));
    assert!(eligible);
    assert_eq!(fm.title.as_deref(), Some("Old"));
    assert_eq!(penalty, 3);
}
