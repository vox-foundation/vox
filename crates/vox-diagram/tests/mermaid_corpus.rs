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
