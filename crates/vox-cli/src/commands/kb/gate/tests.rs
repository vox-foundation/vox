use super::super::eval::{run_query, run_report};
use super::*;
use vox_search::kb_eval::baseline::BaselineBlock;
use vox_search::kb_eval::golden::Split;
use vox_search::kb_eval::golden::read_canary_guid;

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let dst = to.join(e.file_name());
        if e.path().is_dir() {
            copy_dir(&e.path(), &dst);
        } else {
            std::fs::copy(e.path(), dst).unwrap();
        }
    }
}

/// Tempdir repo whose `contracts/kb/eval/` is a copy of the vox-search fixture.
pub(crate) fn temp_repo() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../vox-search/tests/fixtures/kb_eval");
    copy_dir(&fixture, &tmp.path().join(EVAL_DIR));
    tmp
}

/// Write a baseline whose every metric is `v`.
fn baseline_with(root: &Path, v: f64) -> std::path::PathBuf {
    let mut b = new_epoch(None, &dev_report(&root.join(EVAL_DIR)).unwrap(), "t").unwrap();
    let l = serde_json::json!({"recall@5": v, "recall@10": v, "mrr@10": v, "ndcg@10": v});
    let all = serde_json::json!({"doc": l, "chunk": l, "answer_in_budget": v});
    let blk: BaselineBlock = serde_json::from_value(all).unwrap();
    b.metrics.values_mut().for_each(|m| *m = blk);
    let path = root.join(BASELINE_REL);
    std::fs::write(&path, b.to_json_string().unwrap()).unwrap();
    path
}

#[test]
fn tighten_never_lowers_file() {
    let tmp = temp_repo();
    let report = dev_report(&tmp.path().join(EVAL_DIR)).unwrap();
    // Baseline below the computed values: raised to exactly the computed ones.
    let path = baseline_with(tmp.path(), 0.0);
    run_tighten(tmp.path()).unwrap();
    let got = read_baseline(&path).unwrap().unwrap();
    for (k, m) in &got.metrics {
        assert_eq!(m, &BaselineBlock::from(&report.keys[k]), "{k}");
    }
    // Baseline above the computed values: file left byte-identical.
    let path = baseline_with(tmp.path(), 2.0);
    let before = std::fs::read(&path).unwrap();
    run_tighten(tmp.path()).unwrap();
    assert_eq!(before, std::fs::read(&path).unwrap());
    // No baseline at all is an error, not a silent bootstrap.
    std::fs::remove_file(&path).unwrap();
    assert!(run_tighten(tmp.path()).is_err());
}

#[test]
fn write_baseline_needs_note_then_bootstraps_epoch_one() {
    let tmp = temp_repo();
    assert!(run_write_baseline(tmp.path(), "  ").is_err());
    assert!(!tmp.path().join(BASELINE_REL).exists());
    run_write_baseline(tmp.path(), "fixture bootstrap").unwrap();
    let load = || {
        read_baseline(&tmp.path().join(BASELINE_REL))
            .unwrap()
            .unwrap()
    };
    let b = load();
    assert_eq!((b.epoch, b.tolerance), (1, 0.01));
    assert!(b.metrics.contains_key("dev/all") && b.metrics.contains_key("dev/seed"));
    run_write_baseline(tmp.path(), "second").unwrap();
    assert_eq!(load().epoch, 2);
}

#[test]
fn index_build_rejects_corpus_with_guid() {
    let tmp = temp_repo();
    let dir = tmp.path().join(EVAL_DIR);
    let guid = read_canary_guid(&dir).unwrap();
    let p = dir.join("corpus.jsonl");
    let mut lines: Vec<serde_json::Value> = std::fs::read_to_string(&p)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let id = lines[0]["_id"].as_str().unwrap().to_string();
    let text = format!("{} {guid}", lines[0]["text"].as_str().unwrap());
    lines[0]["text"] = text.into();
    let body: String = lines.iter().map(|v| format!("{v}\n")).collect();
    std::fs::write(&p, body).unwrap();
    let mut out = Vec::new();
    for err in [
        run_report(&dir, Split::Dev, false, &mut out).unwrap_err(),
        run_query(&dir, "zephyrine", 3, &mut out).unwrap_err(),
        dev_report(&dir).unwrap_err(),
        run_write_baseline(tmp.path(), "x").unwrap_err(),
    ] {
        let msg = format!("{err:#}");
        assert!(msg.contains(&id), "{msg}");
        assert!(!msg.contains(&guid), "GUID must never be printed: {msg}");
    }
    assert!(out.is_empty(), "nothing scored before the assert");
}

fn strings(s: &str) -> Vec<String> {
    s.split_whitespace().map(String::from).collect()
}

fn write(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body).unwrap();
}

fn section(guid: &str) -> String {
    format!(
        "# Planted\n\n## Leaky section\n\n{}\nmarker {guid}\n",
        "This section body is deliberately long enough to be extracted as a pair. ".repeat(4)
    )
}

#[test]
fn mens_extract_scan_flags_planted_guid_then_clears() {
    let tmp = temp_repo();
    let root = tmp.path();
    let guid = read_canary_guid(&root.join(EVAL_DIR)).unwrap();
    write(root, "docs/src/planted.md", &section(&guid));
    assert!(root.join("docs/src/planted.md").exists());
    let hits = mens_extract_guid_hits(root, &guid).unwrap();
    assert!(hits.iter().any(|h| h.ends_with("planted.md")), "{hits:?}");
    std::fs::remove_file(root.join("docs/src/planted.md")).unwrap();
    assert!(!root.join("docs/src/planted.md").exists());
    assert!(mens_extract_guid_hits(root, &guid).unwrap().is_empty());
}

#[test]
fn fingerprint_doc_paths_filters_scope() {
    let ls = strings(
        "docs/src/a.md crates/x/README.md docs/src/archive/old.md crates/x/docs/archive/old.md \
         contracts/kb/eval/README.md docs/src/data.json scripts/notes.md",
    );
    assert_eq!(
        fingerprint_doc_paths(&ls),
        strings("docs/src/a.md crates/x/README.md")
    );
}

#[test]
fn resolve_base_rejects_missing_ref() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let err = resolve_base(&repo, "refs/heads/definitely-missing-kb-eval").unwrap_err();
    assert!(err.to_string().contains("cannot resolve base ref"), "{err}");
    assert!(resolve_base(&repo, "--output=x").is_err());
    assert!(resolve_base(&repo, "HEAD").is_ok());
}

#[test]
fn gate_failures_sorted() {
    let got = sorted_unique(strings(
        "[tighten-only]_b [canary]_z [regression]_a [canary]_z [canary]_a",
    ));
    assert_eq!(
        got,
        strings("[canary]_a [canary]_z [regression]_a [tighten-only]_b")
    );
}

#[test]
fn canary_sweep_names_every_source_and_never_the_guid() {
    let tmp = temp_repo();
    let root = tmp.path();
    let eval = root.join(EVAL_DIR);
    let guid = read_canary_guid(&eval).unwrap();
    let pasted = "zephyrine installer setup walkthrough for the quokka daemon beta channel";
    let q = std::fs::read_to_string(eval.join("queries.jsonl")).unwrap();
    std::fs::write(
        eval.join("queries.jsonl"),
        q.replacen("zephyrine installer setup", pasted, 1),
    )
    .unwrap();
    write(root, "notes/leak.txt", &format!("oops {guid}\n"));
    write(root, "docs/src/planted.md", &section(&guid));
    write(root, "docs/src/pasted.md", &format!("# P\n\n{pasted}\n"));
    write(
        root,
        "mens/data/train.jsonl",
        &format!("{{\"t\":\"{guid}\"}}\n"),
    );
    let tracked = strings(
        "contracts/kb/eval/README.md notes/leak.txt docs/src/planted.md docs/src/pasted.md",
    );
    let out = canary_failures(root, &tracked).join("\n");
    for want in [
        "repo-scan: notes/leak.txt",
        "repo-scan: docs/src/planted.md",
        "mens-extractor: docs pair from docs/src/planted.md",
        "mens-jsonl: mens/data/train.jsonl",
        "query-fingerprint: q001 in docs/src/pasted.md",
    ] {
        assert!(out.contains(want), "missing {want:?} in:\n{out}");
    }
    assert!(!out.contains("contracts/kb/eval/README.md"), "{out}");
    assert!(!out.contains(&guid), "GUID must never be printed");
    for rel in
        "notes/leak.txt docs/src/planted.md docs/src/pasted.md mens/data/train.jsonl".split(' ')
    {
        std::fs::remove_file(root.join(rel)).unwrap();
        assert!(!root.join(rel).exists());
    }
    let clean = canary_failures(root, &["contracts/kb/eval/README.md".to_string()]);
    assert!(clean.is_empty(), "{clean:?}");
}
