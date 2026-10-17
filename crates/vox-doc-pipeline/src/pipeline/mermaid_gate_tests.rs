//! Fixture-tree tests of the legacy-Mermaid ratchet through `lint_in`.

use std::path::{Path, PathBuf};

use super::*;
use crate::pipeline::lint_in;
use crate::pipeline::tests::{BASE_FM, write_facet_contracts, write_readme_sync_fixture};

const FENCE: &str = "```mermaid\ngraph TD\n  A --> B\n```\n";

/// A facet-clean doc followed by `body`; the body starts on line 15.
fn doc(body: &str) -> String {
    let words = vec!["word"; 40].join(" ");
    format!(
        "---\n{BASE_FM}\ntopic: search-retrieval\ndomain: knowledge\nauthorship: human\n---\n# Doc\n\n<!-- kb:summary -->\n{words}\n<!-- /kb:summary -->\n\n{body}"
    )
}

fn root(name: &str) -> PathBuf {
    let r = std::env::temp_dir().join(format!("vox-doc-mermaid-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&r);
    std::fs::create_dir_all(r.join("docs/src/archive")).unwrap();
    write_facet_contracts(&r, " []");
    write_readme_sync_fixture(&r, "Same text.", "Same text.");
    r
}

fn put(r: &Path, rel: &str, text: &str) {
    std::fs::write(r.join(rel), text).unwrap();
}

fn baseline(r: &Path, files: &str) {
    let text = format!("schema_version: 1\nfiles:{files}\n");
    std::fs::create_dir_all(r.join("contracts/documentation")).unwrap();
    put(r, MERMAID_BASELINE_REL, &text);
}

fn run(r: &Path, paths: &str) -> i32 {
    lint_in(r, &[format!("--paths={paths}")])
}

fn issues(r: &Path) -> Vec<(usize, MermaidIssue)> {
    let mut errors = Vec::new();
    lint(r, &mut errors);
    errors
        .into_iter()
        .map(|e| match e.kind {
            LintKind::LegacyMermaid(i) => (e.line, i),
            other => panic!("unexpected {other:?}"),
        })
        .collect()
}

#[test]
fn tracer_extra_fence_fails_lint() {
    let r = root("tracer");
    baseline(&r, "\n  docs/src/old.md: 1");
    put(&r, "docs/src/old.md", &doc(FENCE));
    assert_eq!(run(&r, "old.md"), 0);
    assert!(issues(&r).is_empty());
    put(&r, "docs/src/old.md", &doc(&format!("{FENCE}\n{FENCE}")));
    assert_eq!(run(&r, "old.md"), 1);
    // the second fence opens on line 15 + 5
    assert_eq!(
        issues(&r),
        [(
            20,
            MermaidIssue::Growth {
                count: 2,
                allowed: 1
            }
        )]
    );
    std::fs::remove_dir_all(&r).unwrap();
}

#[test]
fn unlisted_file_fails_even_when_another_file_is_linted() {
    let r = root("unlisted");
    baseline(&r, " {}");
    put(&r, "docs/src/old.md", &doc("clean\n"));
    put(&r, "docs/src/new.md", &doc(FENCE));
    assert_eq!(run(&r, "old.md"), 1);
    assert_eq!(
        issues(&r),
        [(
            15,
            MermaidIssue::Growth {
                count: 1,
                allowed: 0
            }
        )]
    );
    std::fs::remove_dir_all(&r).unwrap();
}

#[test]
fn examples_prose_and_archive_are_not_counted() {
    let r = root("examples");
    baseline(&r, " {}");
    let body = format!("Never add ```mermaid fences.\n\n````markdown\n{FENCE}````\n");
    put(&r, "docs/src/example.md", &doc(&body));
    put(&r, "docs/src/archive/a.md", FENCE);
    assert!(issues(&r).is_empty());
    assert_eq!(run(&r, "example.md"), 0);
    std::fs::remove_dir_all(&r).unwrap();
}

#[test]
fn missing_baseline_admits_nothing() {
    let r = root("missing");
    put(&r, "docs/src/a.md", &doc(FENCE));
    assert!(!r.join(MERMAID_BASELINE_REL).exists());
    assert_eq!(
        issues(&r),
        [(
            15,
            MermaidIssue::Growth {
                count: 1,
                allowed: 0
            }
        )]
    );
    assert_eq!(run(&r, "a.md"), 1);
    std::fs::remove_dir_all(&r).unwrap();
}

#[test]
fn invalid_baseline_is_one_hard_error() {
    let r = root("invalid");
    std::fs::create_dir_all(r.join("contracts/documentation")).unwrap();
    put(&r, MERMAID_BASELINE_REL, "{{ not yaml");
    put(&r, "docs/src/a.md", &doc(FENCE));
    let got = issues(&r);
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(matches!(got[0].1, MermaidIssue::Unreadable { .. }));
    assert_eq!(run(&r, "a.md"), 1);
    std::fs::remove_dir_all(&r).unwrap();
}

#[test]
fn near_miss_mermaid_info_is_a_hard_error() {
    let r = root("nearmiss");
    baseline(&r, " {}");
    put(&r, "docs/src/a.md", &doc("```mermaid,foo\ngraph TD\n```\n"));
    assert_eq!(
        issues(&r),
        [(
            15,
            MermaidIssue::InfoString {
                info: "mermaid,foo".into()
            }
        )]
    );
    assert_eq!(run(&r, "a.md"), 1);
    std::fs::remove_dir_all(&r).unwrap();
}

#[test]
fn stale_entry_warns_but_exits_zero() {
    let r = root("stale");
    baseline(&r, "\n  docs/src/a.md: 2");
    put(&r, "docs/src/a.md", &doc(FENCE));
    assert_eq!(
        issues(&r),
        [(
            0,
            MermaidIssue::Stale {
                count: 1,
                allowed: 2
            }
        )]
    );
    assert_eq!(run(&r, "a.md"), 0);
    std::fs::remove_dir_all(&r).unwrap();
}

fn flag(r: &Path, f: &str) -> i32 {
    lint_in(r, &[f.to_string()])
}

fn read(r: &Path) -> String {
    std::fs::read_to_string(r.join(MERMAID_BASELINE_REL)).unwrap()
}

fn files(r: &Path) -> Vec<(String, usize)> {
    parse(&read(r)).unwrap().files.into_iter().collect()
}

#[test]
fn seed_then_tighten_then_refuse_growth() {
    let r = root("flow");
    put(&r, "docs/src/a.md", &doc(&format!("{FENCE}\n{FENCE}")));
    put(&r, "docs/src/b.md", &doc(FENCE));
    put(&r, "docs/src/archive/old.md", FENCE);
    assert_eq!(flag(&r, "--seed-mermaid-baseline"), 0);
    let first = read(&r);
    assert!(first.starts_with("# GENERATED"), "{first}");
    assert_eq!(render(&parse(&first).unwrap()).unwrap(), first);
    assert_eq!(seed(&r).unwrap(), "seeded 3 fences in 2 files");
    assert_eq!(read(&r), first, "re-seeding must write identical bytes");
    assert_eq!(
        files(&r),
        [("docs/src/a.md".into(), 2), ("docs/src/b.md".into(), 1)]
    );

    put(&r, "docs/src/a.md", &doc(FENCE));
    assert_eq!(flag(&r, "--tighten-mermaid-baseline"), 0);
    assert_eq!(
        files(&r),
        [("docs/src/a.md".into(), 1), ("docs/src/b.md".into(), 1)]
    );
    std::fs::remove_file(r.join("docs/src/b.md")).unwrap();
    assert_eq!(
        tighten(&r).unwrap(),
        "tightened: 2 -> 1 fences, 2 -> 1 files"
    );
    assert_eq!(files(&r), [("docs/src/a.md".into(), 1)]);
    assert_eq!(
        tighten(&r).unwrap(),
        "legacy-Mermaid baseline already tight"
    );

    put(&r, "docs/src/a.md", &doc(&format!("{FENCE}\n{FENCE}")));
    let before = read(&r);
    let err = tighten(&r).unwrap_err();
    assert!(err.contains("docs/src/a.md: 2 > 1"), "{err}");
    assert!(err.contains("USER-AUTHORIZED-ONLY"), "{err}");
    assert_eq!(flag(&r, "--tighten-mermaid-baseline"), 1);
    assert_eq!(
        read(&r),
        before,
        "a refused tighten must not touch the file"
    );
    std::fs::remove_dir_all(&r).unwrap();
}

#[test]
fn tighten_without_baseline_is_an_error() {
    let r = root("notighten");
    let err = tighten(&r).unwrap_err();
    assert!(err.contains("is missing"), "{err}");
    assert_eq!(flag(&r, "--tighten-mermaid-baseline"), 1);
    assert!(!r.join(MERMAID_BASELINE_REL).exists());
    std::fs::remove_dir_all(&r).unwrap();
}
