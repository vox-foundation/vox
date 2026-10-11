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
            "facet-missing-summary",
            "facet-missing-authorship"
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

/// CR-01: a doc that opens a `---` fence the splitter cannot read is a hard error, not "no facets",
/// even when the backlog excuses every `facet-missing-*` rule for it.
#[test]
fn unparseable_frontmatter_is_a_hard_parse_error() {
    for (name, doc) in [
        ("leading blank", "\n---\ntitle: T\n---\n# T\n"),
        ("leading space", " ---\ntitle: T\n---\n# T\n"),
        (
            "unterminated",
            "---\ntitle: T\ntraining_eligible: false\n# T\n",
        ),
        ("bom blank", "\u{feff}\n---\ntitle: T\n---\n# T\n"),
    ] {
        assert_eq!(ids(doc), ["facet-parse-error"], "{name}");
        let ctx = ctx_with(
            "\n  - path: docs/src/a.md\n    rules: [facet-missing-authorship, facet-missing-domain, facet-missing-summary, facet-missing-topic]",
        );
        let errs = run_lint(&ctx, "docs/src/a.md", doc);
        assert!(
            errs.iter().any(
                |e| matches!(&e.kind, LintKind::Facet(i) if i.rule_id() == "facet-parse-error" && i.is_hard())
            ),
            "{name}: backlogged doc must still fail hard"
        );
    }
    // a BOM before a real fence is fine
    assert_eq!(
        ids("\u{feff}---\ntitle: T\n---\n# T\n"),
        [
            "facet-missing-topic",
            "facet-missing-domain",
            "facet-missing-summary",
            "facet-missing-authorship"
        ]
    );
}

const FIXTURE_TAXONOMY: &str = "x-vox-version: 1
schema_version: 1
domains:
  - prefLabel: knowledge
  - prefLabel: platform
topics:
  - prefLabel: kb-index
    broader: knowledge
    altLabel: [vox-search]
    deprecatedAliases: [kb-store]
  - prefLabel: ci
    broader: platform
";

fn fx_ids(doc: &str) -> Vec<&'static str> {
    let t = Taxonomy::from_yaml_str(FIXTURE_TAXONOMY).unwrap();
    facet_issues(doc, &t)
        .iter()
        .map(|(_, i)| i.rule_id())
        .collect()
}

fn fx_msgs(doc: &str) -> Vec<String> {
    let t = Taxonomy::from_yaml_str(FIXTURE_TAXONOMY).unwrap();
    facet_issues(doc, &t)
        .iter()
        .map(|(_, i)| i.message())
        .collect()
}

/// A page that is clean except for what `extra_fm` / `body` add.
fn page(extra_fm: &str, summary_words: usize, body: &str) -> String {
    let w = vec!["w"; summary_words].join(" ");
    format!(
        "---\ntopic: kb-index\ndomain: knowledge\nauthorship: human\n{extra_fm}\n---\n# T\n\n<!-- kb:summary -->\n{w}\n<!-- /kb:summary -->\n{body}"
    )
}

#[test]
fn clean_page_has_no_issues() {
    assert!(fx_ids(&page("", 40, "")).is_empty());
}

#[test]
fn topic_and_domain_must_be_pref_labels() {
    let doc = |t: &str, d: &str| {
        let w = vec!["w"; 40].join(" ");
        format!(
            "---\ntopic: {t}\ndomain: {d}\nauthorship: human\n---\n# T\n\n<!-- kb:summary -->\n{w}\n<!-- /kb:summary -->\n"
        )
    };
    assert_eq!(fx_ids(&doc("nope", "knowledge")), ["facet-unknown-topic"]);
    assert_eq!(
        fx_ids(&doc("kb-indx", "knowledge")),
        ["facet-unknown-topic"]
    );
    assert!(fx_msgs(&doc("kb-indx", "knowledge"))[0].contains("kb-index"));
    assert_eq!(
        fx_ids(&doc("vox-search", "knowledge")),
        ["facet-unknown-topic"]
    );
    assert!(fx_msgs(&doc("vox-search", "knowledge"))[0].contains("kb-index"));
    assert_eq!(
        fx_ids(&doc("kb-store", "knowledge")),
        ["facet-deprecated-label"]
    );
    assert!(fx_msgs(&doc("kb-store", "knowledge"))[0].contains("kb-index"));
    assert_eq!(fx_ids(&doc("kb-index", "nope")), ["facet-unknown-domain"]);
    assert_eq!(
        fx_ids(&doc("kb-index", "platform")),
        ["facet-domain-mismatch"]
    );
    assert!(fx_msgs(&doc("kb-index", "platform"))[0].contains("knowledge"));
}

#[test]
fn summary_word_count_boundaries() {
    for (n, bad) in [(39, true), (40, false), (120, false), (121, true)] {
        let got = fx_ids(&page("", n, ""));
        assert_eq!(
            got,
            if bad {
                vec!["facet-summary-word-count"]
            } else {
                vec![]
            },
            "{n} words"
        );
    }
    assert!(fx_msgs(&page("", 39, ""))[0].contains("39"));
}

#[test]
fn summary_placement_and_missing() {
    let w = vec!["w"; 40].join(" ");
    let block = format!("<!-- kb:summary -->\n{w}\n<!-- /kb:summary -->\n");
    let head = "---\ntopic: kb-index\ndomain: knowledge\nauthorship: human\n---\n# T\n\n";
    assert_eq!(
        fx_ids(&format!("{head}intro paragraph\n\n{block}")),
        ["facet-summary-placement"]
    );
    assert_eq!(
        fx_ids(&format!("{head}{block}\n{block}")),
        ["facet-summary-placement"]
    );
    assert_eq!(
        fx_ids(&format!("{head}<!-- kb:summary -->\n{w}\n")),
        ["facet-missing-summary"]
    );
    assert_eq!(
        fx_ids(&format!("{head}body only\n")),
        ["facet-missing-summary"]
    );
}

#[test]
fn authorship_model_required_for_llm_authorship() {
    let doc = |fm: &str| page(fm, 40, "").replace("authorship: human", "");
    assert_eq!(
        fx_ids(&doc("authorship: llm-assisted")),
        ["facet-missing-authorship-model"]
    );
    assert_eq!(
        fx_ids(&doc("authorship: llm-generated\nauthorship_model: \"  \"")),
        ["facet-missing-authorship-model"]
    );
    assert!(fx_ids(&doc("authorship: llm-assisted\nauthorship_model: m")).is_empty());
}

#[test]
fn training_eligible_contradiction() {
    let doc = |fm: &str| {
        page(fm, 40, "").replace(
            "authorship: human",
            "authorship: llm-generated\nauthorship_model: m",
        )
    };
    assert_eq!(
        fx_ids(&doc("training_eligible: true")),
        ["facet-training-eligible-contradiction"]
    );
    assert!(fx_ids(&doc("")).is_empty());
    assert!(fx_ids(&doc("training_eligible: true\nhuman_attested_by: alice")).is_empty());
}

#[test]
fn web_promoted_sources_are_required_and_complete() {
    let h = format!("blake3:{}", "a".repeat(64));
    let promoted = |fm: &str| {
        page(
            &format!("origin: web-promoted\nhuman_attested_by: a\n{fm}"),
            40,
            "",
        )
    };
    assert_eq!(fx_ids(&promoted("")), ["facet-missing-sources"]);
    assert_eq!(fx_ids(&promoted("sources: []")), ["facet-missing-sources"]);
    let no_hash = promoted("sources:\n  - url: https://e.x\n    date: 2026-10-10");
    assert_eq!(fx_ids(&no_hash), ["facet-source-field"]);
    assert!(fx_msgs(&no_hash)[0].contains("sources[0].hash"));
    assert_eq!(
        fx_ids(&promoted(&format!(
            "sources:\n  - url: \" \"\n    date: 2026-10-10\n    hash: {h}"
        ))),
        ["facet-source-field"]
    );
    assert!(
        fx_ids(&promoted(&format!(
            "sources:\n  - url: https://e.x\n    date: 2026-10-10\n    hash: {h}"
        )))
        .is_empty()
    );
}

#[test]
fn bad_source_date_or_hash_fails_on_any_page() {
    let d = page("sources:\n  - date: 2026/10/10", 40, "");
    assert_eq!(fx_ids(&d), ["facet-source-field"]);
    let hsh = page("sources:\n  - hash: sha256:abc", 40, "");
    assert_eq!(fx_ids(&hsh), ["facet-source-field"]);
    // absent fields are fine off the promoted path
    assert!(fx_ids(&page("sources:\n  - title: t", 40, "")).is_empty());
}

fn quote(words: usize) -> String {
    format!("> {}\n\n", vec!["q"; words].join(" "))
}

#[test]
fn quote_over_cap_per_quote_and_per_page_on_promoted_pages_only() {
    let h = format!("blake3:{}", "a".repeat(64));
    let src = format!(
        "origin: web-promoted\nhuman_attested_by: a\nsources:\n  - url: https://e.x\n    date: 2026-10-10\n    hash: {h}"
    );
    assert_eq!(
        fx_ids(&page(&src, 40, &quote(51))),
        ["facet-quote-over-cap"]
    );
    assert!(fx_msgs(&page(&src, 40, &quote(51)))[0].contains("quote cap"));
    assert!(fx_ids(&page(&src, 40, &quote(50))).is_empty());
    let five: String = (0..5).map(|_| quote(45)).collect();
    assert_eq!(fx_ids(&page(&src, 40, &five)), ["facet-quote-over-cap"]);
    assert!(fx_msgs(&page(&src, 40, &five))[0].contains("page cap"));
    assert!(fx_ids(&page("", 40, &five)).is_empty());
}

#[test]
fn invalid_review_date_is_reported() {
    assert_eq!(
        fx_ids(&page("review_by: soon", 40, "")),
        ["facet-invalid-date"]
    );
    assert!(fx_msgs(&page("review_by: soon", 40, ""))[0].contains("review_by"));
    assert!(fx_ids(&page("review_by: 2026-12-31", 40, "")).is_empty());
}

/// WR-02: `human_attested_by` must name a person.
#[test]
fn invalid_attestation_is_a_hard_lint_error() {
    for v in [
        "false", "none", "n/a", "TODO", "tbd", "claude", "\"\"", "\"  \"", "yes",
    ] {
        assert_eq!(
            fx_ids(&page(&format!("human_attested_by: {v}"), 40, "")),
            ["facet-invalid-attestation"],
            "{v}"
        );
    }
    assert!(fx_msgs(&page("human_attested_by: none", 40, ""))[0].contains("none"));
    assert!(fx_ids(&page("human_attested_by: Alice Smith", 40, "")).is_empty());
    assert!(
        fx_ids(&page("", 40, "")).is_empty(),
        "absent key is not an error"
    );
}

#[test]
fn rule_ids_are_stable() {
    let all = [
        FacetIssue::ContractUnreadable {
            path: String::new(),
            error: String::new(),
        },
        FacetIssue::ParseError { msg: String::new() },
        FacetIssue::MissingTopic,
        FacetIssue::MissingDomain,
        FacetIssue::UnknownTopic {
            value: String::new(),
            hint: None,
        },
        FacetIssue::UnknownDomain {
            value: String::new(),
            hint: None,
        },
        FacetIssue::DeprecatedLabel {
            field: "topic",
            value: String::new(),
            replaced_by: String::new(),
        },
        FacetIssue::DomainMismatch {
            topic: String::new(),
            domain: String::new(),
            expected: String::new(),
        },
        FacetIssue::MissingSummary,
        FacetIssue::SummaryPlacement {
            reason: String::new(),
        },
        FacetIssue::SummaryWordCount { words: 0 },
        FacetIssue::MissingAuthorship,
        FacetIssue::MissingAuthorshipModel,
        FacetIssue::TrainingEligibleContradiction,
        FacetIssue::MissingSources,
        FacetIssue::SourceField {
            index: 0,
            field: "url",
            problem: String::new(),
        },
        FacetIssue::QuoteOverCap {
            words: 0,
            cap: 0,
            scope: "page",
        },
        FacetIssue::InvalidDate {
            field: "review_by",
            value: String::new(),
        },
        FacetIssue::InvalidAttestation {
            value: String::new(),
        },
        FacetIssue::BacklogStale {
            rule: String::new(),
        },
    ];
    let mut ids: Vec<&str> = all.iter().map(FacetIssue::rule_id).collect();
    assert!(
        ids.iter()
            .all(|i| i.starts_with("facet-")
                && i.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'))
    );
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 20);
    let meta = ["facet-contract-unreadable", "facet-backlog-stale"];
    let mut expect: Vec<&str> = ids.iter().copied().filter(|i| !meta.contains(i)).collect();
    let mut got = BACKLOGGABLE_RULE_IDS.to_vec();
    expect.sort_unstable();
    got.sort_unstable();
    assert_eq!(got, expect);
    let soft: Vec<_> = all
        .iter()
        .filter(|i| !i.is_hard())
        .map(FacetIssue::rule_id)
        .collect();
    assert_eq!(soft, ["facet-backlog-stale"]);
}

fn ctx_with(rows: &str) -> FacetCtx {
    FacetCtx {
        taxonomy: Taxonomy::from_yaml_str(FIXTURE_TAXONOMY).unwrap(),
        backlog: Backlog::from_yaml_str(&format!(
            "schema_version: 1\nepoch: 1\nepoch_note: t\ndocs:{rows}\n"
        ))
        .unwrap(),
    }
}

fn run_lint(ctx: &FacetCtx, rel: &str, content: &str) -> Vec<LintError> {
    let root = PathBuf::from("/r");
    let mut errs = Vec::new();
    lint_facets(&root.join(rel), content, &root, ctx, &mut errs);
    errs
}

#[test]
fn stale_backlog_rule_is_a_soft_warning() {
    let ctx = ctx_with("\n  - path: docs/src/a.md\n    rules: [facet-missing-topic]");
    let errs = run_lint(&ctx, "docs/src/a.md", &page("", 40, ""));
    assert_eq!(errs.len(), 1);
    assert!(
        matches!(&errs[0].kind, LintKind::Facet(i) if i.rule_id() == "facet-backlog-stale" && !i.is_hard())
    );
}

#[test]
fn backlog_excuses_only_its_listed_rules() {
    let ctx = ctx_with("\n  - path: docs/src/a.md\n    rules: [facet-missing-topic]");
    // missing topic AND domain: topic is excused, domain still fails
    let doc = "---\nauthorship: human\n---\n# T\n\n<!-- kb:summary -->\n".to_string()
        + &vec!["w"; 40].join(" ")
        + "\n<!-- /kb:summary -->\n";
    let errs = run_lint(&ctx, "docs/src/a.md", &doc);
    let ids: Vec<_> = errs
        .iter()
        .map(|e| match &e.kind {
            LintKind::Facet(i) => i.rule_id(),
            _ => "?",
        })
        .collect();
    assert_eq!(ids, ["facet-missing-domain"]);
    // a renamed doc is not in the backlog and gets the full rules
    let renamed = run_lint(&ctx, "docs/src/b.md", &doc);
    assert_eq!(renamed.len(), 2);
    // archive and generated docs are out of scope
    assert!(run_lint(&ctx, "docs/src/archive/a.md", &doc).is_empty());
    assert!(run_lint(&ctx, "docs/src/x.generated.md", &doc).is_empty());
}

#[test]
fn facet_errors_do_not_depend_on_thread_order() {
    use rayon::prelude::*;
    let ctx = ctx_with(" []");
    let docs: Vec<String> = (0..64)
        .map(|i| {
            page(
                if i % 2 == 0 { "review_by: soon" } else { "" },
                39 + i % 3,
                "",
            )
        })
        .collect();
    let serial: Vec<usize> = docs
        .iter()
        .map(|d| run_lint(&ctx, "docs/src/a.md", d).len())
        .collect();
    let par: Vec<usize> = docs
        .par_iter()
        .map(|d| run_lint(&ctx, "docs/src/a.md", d).len())
        .collect();
    assert_eq!(serial, par);
}
