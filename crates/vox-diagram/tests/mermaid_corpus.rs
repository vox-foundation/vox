//! Completeness of the frozen Mermaid corpus. Reads only `tests/fixtures/corpus/`, never
//! `docs/`; the `#[ignore]` freezer in `freeze_corpus.rs` is the only code that reads docs.

mod support;

use std::collections::BTreeSet;
use std::fs;

use serde_json::Value;
use support::roundtrip::{fixtures, stems};

fn text<'a>(entry: &'a Value, key: &str) -> &'a str {
    entry.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[test]
fn corpus_manifest_matches_fixtures() {
    let dir = fixtures().join("corpus");
    let Ok(raw) = fs::read_to_string(dir.join("MANIFEST.json")) else {
        panic!("corpus/MANIFEST.json is missing; run the ignored freeze_corpus test");
    };
    let Ok(Value::Array(entries)) = serde_json::from_str::<Value>(&raw) else {
        panic!("MANIFEST.json is not a JSON array");
    };
    assert_eq!(entries.len(), 33, "corpus must hold 33 fences");

    let names: Vec<&str> = entries.iter().map(|e| text(e, "fixture")).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted, "manifest must be sorted by fixture");
    let on_disk: Vec<String> = stems(&dir, "mmd")
        .into_iter()
        .map(|s| format!("{s}.mmd"))
        .collect();
    assert_eq!(
        on_disk, names,
        "corpus/*.mmd must be exactly the manifest's fixtures"
    );

    let sources: BTreeSet<&str> = entries.iter().map(|e| text(e, "source")).collect();
    assert_eq!(sources.len(), 21, "fences must come from 21 files");
    assert!(
        sources
            .iter()
            .all(|s| s.starts_with("docs/src/") && !s.starts_with("docs/src/archive/")),
        "sources must be live docs/src files: {sources:?}"
    );

    let mut census = std::collections::BTreeMap::<String, usize>::new();
    for e in &entries {
        let fixture = text(e, "fixture");
        let Ok(body) = fs::read_to_string(dir.join(fixture)) else {
            panic!("cannot read {fixture}");
        };
        let first = body
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or_default()
            .trim();
        assert_eq!(
            first,
            text(e, "header"),
            "{fixture}: header differs from the manifest"
        );
        let word = first.split_whitespace().next().unwrap_or_default();
        let kind = if word == "graph" { "flowchart" } else { word };
        *census.entry(kind.to_owned()).or_default() += 1;
    }
    let want: std::collections::BTreeMap<String, usize> = [
        ("flowchart", 30),
        ("stateDiagram-v2", 1),
        ("sequenceDiagram", 1),
        ("gantt", 1),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    assert_eq!(census, want, "header census");

    for (source, line) in [
        ("docs/src/how-to/how-to-rust-crate-imports.md", 53),
        ("docs/src/reference/populi.md", 211),
    ] {
        assert!(
            entries.iter().any(|e| text(e, "source") == source
                && e.get("open_line").and_then(Value::as_u64) == Some(line)),
            "missing manifest entry {source}:{line}"
        );
    }
}

const GANTT: (&str, u64) = (
    "docs/src/architecture/deep-research-full-surface-audit-and-roadmap-2026.md",
    485,
);
const MARKDOWN: (&str, u64) = ("docs/src/how-to/how-to-rust-crate-imports.md", 53);
const SEQUENCE: (&str, u64) = (
    "docs/src/architecture/scientia-automated-research-historical-extension-research-2026.md",
    112,
);
const DIRECTIONS: (&str, u64) = (
    "docs/src/architecture/deep-research-full-surface-audit-and-roadmap-2026.md",
    238,
);

/// 84-03/84-04 expectations: 29 of the 30 flowchart blocks and the state block import; the
/// sequence and gantt blocks report their header until the next tasks; the markdown-string
/// block is rejected by the real parser.
#[test]
fn corpus_blocks_import_or_fail_closed_as_expected() {
    use vox_diagram::mermaid::{ImportOpts, import};
    use vox_diagram::{check, fmt};
    let dir = fixtures().join("corpus");
    let Ok(raw) = fs::read_to_string(dir.join("MANIFEST.json")) else {
        panic!("corpus/MANIFEST.json is missing");
    };
    let Ok(Value::Array(entries)) = serde_json::from_str::<Value>(&raw) else {
        panic!("MANIFEST.json is not a JSON array");
    };
    let mut failed_closed = 0;
    for e in &entries {
        let key = (
            text(e, "source"),
            e.get("open_line").and_then(Value::as_u64).unwrap_or(0),
        );
        let fixture = text(e, "fixture");
        let Ok(body) = fs::read_to_string(dir.join(fixture)) else {
            panic!("cannot read {fixture}");
        };
        let report = import(&body, ImportOpts::default());
        let got: Vec<(u32, &str)> = report
            .unsupported
            .iter()
            .map(|u| (u.line, u.construct.as_str()))
            .collect();
        let want_header = [
            (GANTT, "header:gantt"),
            (SEQUENCE, "header:sequenceDiagram"),
        ]
        .into_iter()
        .find(|(k, _)| *k == key);
        if let Some((_, header)) = want_header {
            assert_eq!(got, vec![(1, header)], "{fixture}");
            assert_eq!((&report.source, &report.sequence), (&None, &None));
            failed_closed += 1;
        } else if key == MARKDOWN {
            assert!(
                !got.is_empty(),
                "{fixture}: expected markdown-string entries"
            );
            assert!(
                got.iter().all(|(_, c)| *c == "markdown-string"),
                "{fixture}: {got:?}"
            );
            assert_eq!(report.source, None);
            failed_closed += 1;
        } else {
            assert!(got.is_empty(), "{fixture}: {got:?}");
            let Some(src) = report.source else {
                panic!("{fixture}: no source");
            };
            assert_eq!(
                fmt(&src).as_deref(),
                Some(src.as_str()),
                "{fixture}: not a fixed point"
            );
            assert!(
                check(&src).valid,
                "{fixture}: imported source does not check"
            );
        }
        let dropped: Vec<&str> = report
            .dropped
            .iter()
            .map(|d| d.construct.as_str())
            .collect();
        if key == DIRECTIONS {
            assert_eq!(dropped, vec!["subgraph-direction"; 2], "{fixture}");
        } else {
            assert!(dropped.is_empty(), "{fixture}: {dropped:?}");
        }
    }
    assert_eq!(failed_closed, 3, "exactly three blocks fail closed");
}
