//! Validation tests for the golden-set loader (split out to keep `golden.rs` small).

use super::*;

// Test-only placeholder GUIDs; the real canary only lives in contracts/kb/eval/README.md.
const G: &str = "11111111-2222-3333-4444-555555555555";
const G2: &str = "66666666-7777-8888-9999-aaaaaaaaaaaa";

fn qline(id: &str, origin: &str, intent: &str, canary: &str) -> String {
    format!(
        r#"{{"_id":"{id}","text":"query {id}","metadata":{{"origin":"{origin}","author":"t","approved_by":"t","created":"2026-10-04","intent":"{intent}","canary":"{canary}"}}}}"#
    )
}

/// A valid golden set: dev = q1 (human), q2 (llm-expansion); test = q3 (human).
fn fx() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let docs = [
        (
            "docs/src/a.md",
            "---\ntitle: A\n---\n## Sec\nbody\n### Sub\nmore\n",
        ),
        ("docs/src/b.md", "# B\n## Two\nx\n"),
    ];
    let corpus: Vec<String> = docs
        .iter()
        .map(|(id, text)| {
            serde_json::to_string(&CorpusDoc {
                id: (*id).into(),
                title: "t".into(),
                text: (*text).into(),
            })
            .unwrap()
        })
        .collect();
    let queries = [
        qline("q1", "human", "where", G),
        qline("q2", "llm-expansion", "how-to", G),
        qline("q3", "human", "why", G),
    ];
    let w = |rel: &str, body: String| {
        let p = d.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    };
    let h = QRELS_HEADER;
    w("README.md", format!("{CANARY_MARK}{G}\n"));
    w("corpus.jsonl", corpus.join("\n") + "\n");
    w("queries.jsonl", queries.join("\n") + "\n");
    w(
        "qrels/dev.tsv",
        format!("{h}\nq1\tdocs/src/a.md\t2\nq2\tdocs/src/b.md\t1\n"),
    );
    w(
        "qrels/dev.chunks.tsv",
        format!("{h}\nq1\tdocs/src/a.md#A › Sec › Sub\t2\nq2\tdocs/src/b.md#B › Two\t2\n"),
    );
    w("qrels/test.tsv", format!("{h}\nq3\tdocs/src/b.md\t2\n"));
    w(
        "qrels/test.chunks.tsv",
        format!("{h}\nq3\tdocs/src/b.md#B › Two\t2\n"),
    );
    d
}

fn put(d: &tempfile::TempDir, rel: &str, body: &str) {
    std::fs::write(d.path().join(rel), body).unwrap();
}

fn load_err(d: &tempfile::TempDir) -> String {
    format!("{:#}", GoldenSet::load(d.path(), Split::Dev).unwrap_err())
}

#[test]
fn valid_fixture_loads_both_splits() {
    let d = fx();
    let dev = GoldenSet::load(d.path(), Split::Dev).unwrap();
    assert_eq!(dev.queries.len(), 2);
    assert_eq!(dev.canary, G);
    assert_eq!(
        GoldenSet::load(d.path(), Split::Test)
            .unwrap()
            .queries
            .len(),
        1
    );
}

#[test]
fn agent_seed_origin_loads_and_counts() {
    let d = fx();
    put(&d, "queries.jsonl", &{
        let q = [
            qline("q1", "agent-seed", "where", G),
            qline("q2", "llm-expansion", "how-to", G),
            qline("q3", "human", "why", G),
        ];
        q.join("\n") + "\n"
    });
    let dev = GoldenSet::load(d.path(), Split::Dev).unwrap();
    assert_eq!(dev.queries[0].metadata.origin, Origin::AgentSeed);
    let s = validate_splits(d.path()).unwrap();
    assert_eq!(s.dev.get(&Origin::AgentSeed), Some(&1));
}

#[test]
fn dev_load_never_opens_test_qrels() {
    let d = fx();
    put(&d, "qrels/test.tsv", "not a qrels file");
    put(&d, "qrels/test.chunks.tsv", "not a qrels file");
    assert!(GoldenSet::load(d.path(), Split::Dev).is_ok());
    assert!(GoldenSet::load(d.path(), Split::Test).is_err());
}

#[test]
fn malformed_inputs_are_rejected_with_file_and_line() {
    let h = QRELS_HEADER;
    let q1 = qline("q1", "human", "where", G);
    let cases: Vec<(&str, String, &str)> = vec![
        (
            "queries.jsonl",
            q1.replace("\"text\"", "\"extra\":1,\"text\""),
            "queries.jsonl:1",
        ),
        (
            "queries.jsonl",
            q1.replace("human", "robot"),
            "queries.jsonl:1",
        ),
        ("queries.jsonl", q1.replace(G, G2), "queries.jsonl:1"),
        (
            "queries.jsonl",
            q1.replace("2026-10-04", "yesterday"),
            "queries.jsonl:1",
        ),
        (
            "queries.jsonl",
            q1.replace("\"t\"", "\"\""),
            "queries.jsonl:1",
        ),
        (
            "qrels/dev.tsv",
            format!("{h}\nq1\tdocs/src/a.md\tx\n"),
            "qrels/dev.tsv:2",
        ),
        (
            "qrels/dev.tsv",
            format!("{h}\nq1\tdocs/src/a.md\t3\n"),
            "qrels/dev.tsv:2",
        ),
        (
            "qrels/dev.tsv",
            format!("{h}\nq1\tdocs/src/a.md\t2\nq1\tdocs/src/a.md\t1\n"),
            "qrels/dev.tsv:3",
        ),
        (
            "qrels/dev.tsv",
            format!("{h}\nq1\tdocs/src/zzz.md\t2\n"),
            "qrels/dev.tsv:2",
        ),
        (
            "qrels/dev.tsv",
            format!("{h}\nq1\tdocs/src/a.md\t0\n"),
            "no doc qrel with score > 0",
        ),
        (
            "qrels/dev.chunks.tsv",
            format!("{h}\nq1\tdocs/src/a.md#A › Nope\t2\n"),
            "qrels/dev.chunks.tsv:2",
        ),
        (
            "qrels/dev.chunks.tsv",
            format!("{h}\nq1\tdocs/src/a.md#A › Sec\t1\nq2\tdocs/src/b.md#B › Two\t2\n"),
            "no answer-bearing chunk",
        ),
        (
            "corpus.jsonl",
            r#"{"_id":"docs/src/archive/z.md","title":"z","text":"z"}"#.to_string(),
            "corpus.jsonl:1",
        ),
    ];
    for (file, body, needle) in cases {
        let d = fx();
        put(&d, file, &(body + "\n"));
        let e = load_err(&d);
        assert!(e.contains(needle), "{file}: want {needle:?}, got {e}");
    }
}

#[test]
fn validate_splits_reports_counts_and_catches_overlap_and_gaps() {
    let d = fx();
    let s = validate_splits(d.path()).unwrap();
    assert_eq!(
        s.dev,
        BTreeMap::from([(Origin::Human, 1), (Origin::LlmExpansion, 1)])
    );
    assert_eq!(s.test, BTreeMap::from([(Origin::Human, 1)]));
    let h = QRELS_HEADER;
    put(
        &d,
        "qrels/test.tsv",
        &format!("{h}\nq3\tdocs/src/b.md\t2\nq1\tdocs/src/a.md\t2\n"),
    );
    assert!(
        validate_splits(d.path())
            .unwrap_err()
            .to_string()
            .contains("both")
    );
    put(&d, "qrels/test.tsv", &format!("{h}\n"));
    assert!(
        validate_splits(d.path())
            .unwrap_err()
            .to_string()
            .contains("neither")
    );
}

#[test]
fn snapshot_is_sorted_filtered_normalized_and_deterministic() {
    let d = tempfile::tempdir().unwrap();
    for (rel, body) in [
        ("docs/src/z.md", "# Z\r\nbody\r\n"),
        ("docs/src/a.md", "\u{feff}# A\n"),
    ] {
        let p = d.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }
    let paths: Vec<String> = [
        "docs/src/z.md",
        "docs/src/a.md",
        "docs/src/archive/x.md",
        "../etc/passwd.md",
        "docs/src/a.txt",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let a = snapshot_corpus_jsonl(d.path(), &paths).unwrap();
    assert_eq!(a, snapshot_corpus_jsonl(d.path(), &paths).unwrap());
    let docs: Vec<CorpusDoc> = a
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let ids: Vec<&str> = docs.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(ids, ["docs/src/a.md", "docs/src/z.md"]);
    assert_eq!(
        (docs[0].title.as_str(), docs[0].text.as_str()),
        ("A", "# A\n")
    );
    assert_eq!(docs[1].text, "# Z\nbody\n");
}

#[test]
fn corpus_and_query_loaders_are_independent_of_qrels() {
    let d = fx();
    std::fs::remove_dir_all(d.path().join("qrels")).unwrap();
    assert_eq!(load_corpus(d.path()).unwrap().len(), 2);
    let q = load_query_texts(d.path()).unwrap();
    assert_eq!(q.len(), 3);
    assert_eq!(q[0], ("q1".to_string(), "query q1".to_string()));
}

#[test]
fn canary_guid_is_read_from_readme_and_shape_checked() {
    let d = fx();
    assert_eq!(read_canary_guid(d.path()).unwrap(), G);
    put(&d, "README.md", &format!("{CANARY_MARK}NOT-A-GUID\n"));
    assert!(read_canary_guid(d.path()).is_err());
    put(&d, "README.md", "no canary here\n");
    assert!(read_canary_guid(d.path()).is_err());
}
