//! TAX-06 / D-05 / D-06: the MENS doc extractor shares one eligibility rule with the facet lint
//! and never emits page-one summary-block text.

use super::*;
use std::fs;

const SENTINEL: &str = "SENTINELSUMMARYWORD";

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

/// CR-01: a `---` fence the splitter does not recognise must never default to eligible, and the
/// `training_eligible: false` opt-out must survive in every frontmatter shape.
#[test]
fn unrecognised_or_shifted_frontmatter_never_trains_an_opted_out_page() {
    for fm in [
        "title: T\ntraining_eligible: false",
        "title: T\norigin: web-promoted",
        "title: T\nauthorship: llm-generated",
    ] {
        let shapes = [
            ("bom", format!("\u{feff}---\n{fm}\n---\n{BODY}")),
            ("bom+crlf", format!("\u{feff}---\r\n{fm}\r\n---\r\n{BODY}")),
            ("leading blank", format!("\n---\n{fm}\n---\n{BODY}")),
            ("leading space", format!("  ---\n{fm}\n---\n{BODY}")),
            ("unterminated", format!("---\n{fm}\n{BODY}")),
            (
                "cr only",
                format!("---\r{fm}\r---\r{BODY}").replace('\n', "\r"),
            ),
            (
                "cr only unterminated",
                format!("---\r{fm}\r{BODY}").replace('\n', "\r"),
            ),
        ];
        for (name, doc) in shapes {
            assert!(extract(&doc).is_empty(), "{name} with `{fm}` still trains");
        }
    }
}

#[test]
fn bom_and_cr_only_frontmatter_without_opt_out_still_trains() {
    assert!(!extract(&format!("\u{feff}---\ntitle: T\n---\n{BODY}")).is_empty());
    // CR-only bodies yield no pairs (the section scanner reads `\n` lines); eligibility is what
    // matters here.
    let cr = format!("---\rtitle: T\r---\r{BODY}").replace('\n', "\r");
    assert!(parse_frontmatter(&cr, Path::new("page.md")).1);
}

#[test]
fn fenceless_file_keeps_the_legacy_substring_opt_out() {
    let body_only = BODY.to_string();
    assert!(!extract(&body_only).is_empty());
    assert!(extract(&format!("{body_only}\ntraining_eligible: false\n")).is_empty());
    assert!(extract(&format!("{body_only}\ntraining_eligible:false\n")).is_empty());
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

#[test]
fn summary_text_never_reaches_any_pair() {
    let block = format!(
        "<!-- kb:summary -->\nA {SENTINEL} sentence about the page.\nMore text.\n<!-- /kb:summary -->\n"
    );
    let shapes = [
        // whole-line block after the heading
        format!(
            "---\ntitle: T\n---\n# Title\n\n{block}\n## Concept section\n\nThis paragraph is long enough to clear the minimum section length so the section is emitted by the extractor under test.\n\n```vox\nfn a() {{}}\n```\n"
        ),
        // inline block
        format!(
            "---\ntitle: T\n---\n# Title\n\n<!-- kb:summary -->{SENTINEL} inline<!-- /kb:summary -->\n\n## Concept section\n\nThis paragraph is long enough to clear the minimum section length so the section is emitted by the extractor under test.\n\n```vox\nfn a() {{}}\n```\n"
        ),
        // unterminated: everything after the open marker is dropped
        format!(
            "---\ntitle: T\n---\n{BODY}\n<!-- kb:summary -->\n{SENTINEL} never closed\n\n## Later\n\nThis paragraph is long enough to clear the minimum section length so the section is emitted by the extractor under test.\n"
        ),
    ];
    for (i, s) in shapes.iter().enumerate() {
        let pairs = extract(s);
        assert!(!pairs.is_empty(), "shape {i} should still yield pairs");
        for p in &pairs {
            assert!(
                !p.to_jsonl().contains(SENTINEL),
                "shape {i} leaked: {}",
                p.to_jsonl()
            );
        }
    }
}

/// Verbatim pre-change eligibility rule (HEAD~ `parse_frontmatter`, 04-04 plan base).
fn legacy_eligible(content: &str) -> bool {
    if content.contains("status: deprecated")
        || content.contains("status: \"deprecated\"")
        || content.contains("status: 'deprecated'")
    {
        return false;
    }
    let substr = !(content.contains("training_eligible: false")
        || content.contains("training_eligible:false"));
    if !content.starts_with("---") {
        return substr;
    }
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return substr;
    }
    #[derive(Deserialize)]
    #[serde(default)]
    struct Legacy {
        training_eligible: bool,
    }
    impl Default for Legacy {
        fn default() -> Self {
            Self {
                training_eligible: true,
            }
        }
    }
    serde_yaml::from_str::<Legacy>(parts[1])
        .unwrap_or_default()
        .training_eligible
}

fn collect_md(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == "archive") {
                continue;
            }
            collect_md(&p, out);
        } else if p.extension().is_some_and(|x| x == "md") {
            out.push(p);
        }
    }
}

#[test]
fn real_docs_eligibility_unchanged_for_facetless_docs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|a| a.join("docs/src").is_dir())
        .expect("repo root with docs/src")
        .join("docs/src");
    let mut files = Vec::new();
    collect_md(&root, &mut files);

    let (mut examined, mut parse_failures, mut mismatches) = (0, Vec::new(), Vec::new());
    for p in &files {
        let Ok(content) = fs::read_to_string(p) else {
            continue;
        };
        let facets = match vox_kb_facets::parse_facets(&content) {
            Ok(f) => f,
            Err(e) => {
                parse_failures.push(format!("{}: {e}", p.display()));
                continue;
            }
        };
        if facets.authorship.is_some()
            || facets.origin.is_some()
            || facets.human_attested_by.is_some()
        {
            continue;
        }
        examined += 1;
        let new = parse_frontmatter(&content, p).1;
        if new != legacy_eligible(&content) {
            mismatches.push(p.display().to_string());
        }
    }
    assert!(
        parse_failures.is_empty(),
        "docs failing the facet parse:\n{}",
        parse_failures.join("\n")
    );
    assert!(
        mismatches.is_empty(),
        "eligibility changed for:\n{}",
        mismatches.join("\n")
    );
    assert!(examined > 600, "only {examined} docs examined");
}
