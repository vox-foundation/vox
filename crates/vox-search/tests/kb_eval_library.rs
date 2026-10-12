//! Library-index retriever for the KB eval: determinism over the Phase 1 fixture, exact
//! resolution of the real golden set's chunk qrels onto Library chunk ids, and the real-data
//! ratchet (EVAL-04): the committed baseline holds for the production Library retriever on the
//! ratchet split, and a degraded ranking fails it.
#![cfg(feature = "tantivy-lexical")]

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use vox_search::kb_eval::baseline::{Baseline, RATCHET_KEYS, check_regression, ratchet_split};
use vox_search::kb_eval::golden::{GoldenSet, Split, load_corpus};
use vox_search::kb_eval::library::{EVAL_DEPTH, LIBRARY_RETRIEVER, LibraryRetriever, evaluate};
use vox_search::kb_eval::metrics::{BUDGET_TOKENS, TOKEN_ESTIMATOR};
use vox_search::kb_eval::run_eval;
use vox_search::kb_ids::{chunk_id, split_chunk_id};
use vox_search::library::chunk::CHUNKER_VERSION;

fn hasher(b: &[u8]) -> String {
    vox_crypto::hex_encode(&vox_crypto::secure_hash(b))
}

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/kb_eval")
}

fn real_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/kb/eval")
}

fn retriever(golden: &GoldenSet, depth: usize) -> LibraryRetriever {
    let work = tempfile::tempdir().unwrap();
    let queries: Vec<String> = golden.queries.iter().map(|q| q.text.clone()).collect();
    LibraryRetriever::build_blocking(&golden.corpus, &queries, depth, work.path(), &hasher)
        .expect("build library retriever")
}

fn eval_fixture() -> vox_search::kb_eval::Report {
    let golden = GoldenSet::load(&fixture_dir(), Split::Dev).expect("load fixture");
    evaluate(&golden, &retriever(&golden, 100)).expect("evaluate")
}

#[test]
fn library_retriever_is_deterministic() {
    let (a, b) = (eval_fixture(), eval_fixture());
    assert_eq!(a.to_json_string().unwrap(), b.to_json_string().unwrap());
    assert_eq!(a.render_text(), b.render_text());
    assert_eq!(a.retriever, LIBRARY_RETRIEVER);
    assert_eq!(a.retriever, "library-tantivy-v1");
    assert_eq!(a.chunker, CHUNKER_VERSION);
}

#[test]
fn library_retriever_ranks_the_unique_install_section_first() {
    // q001's unique installer terms live only in alpha's Install section.
    let report = eval_fixture();
    let all = &report.keys["dev/all"];
    assert_eq!(all.queries, 3);
    assert_eq!(all.doc.mrr_at_10, 1.0);
    assert!(all.chunk.recall_at_10 > 0.0);
}

#[test]
fn chunk_qrels_resolve_exactly_to_library_chunks() {
    // Reads only the corpus-id column of each qrels file: no metrics on the held-out split.
    let dir = real_dir();
    let corpus = load_corpus(&dir).expect("load real corpus");
    let work = tempfile::tempdir().unwrap();
    let idx = LibraryRetriever::build_blocking(&corpus, &[], 10, work.path(), &hasher)
        .expect("index the frozen snapshot");
    let known: BTreeSet<String> = idx
        .doc_chunk_ids()
        .values()
        .flat_map(|s| s.iter().cloned())
        .collect();
    let mut unresolved = Vec::new();
    for name in ["dev.chunks.tsv", "test.chunks.tsv"] {
        let text = std::fs::read_to_string(dir.join("qrels").join(name)).unwrap();
        for line in text.lines().skip(1).filter(|l| !l.trim().is_empty()) {
            let key = line.split('\t').nth(1).expect("corpus-id column");
            let ok = split_chunk_id(key)
                .map(|(doc, hp)| known.contains(&chunk_id(doc, hp)))
                .unwrap_or(false);
            if !ok {
                let doc = split_chunk_id(key).map(|(d, _)| d).unwrap_or(key);
                let have: Vec<&String> = idx
                    .doc_chunk_ids()
                    .get(doc)
                    .map(|s| s.iter().collect())
                    .unwrap_or_default();
                unresolved.push(format!("{name}: {key:?}; doc heading paths: {have:?}"));
            }
        }
    }
    assert!(
        unresolved.is_empty(),
        "chunk qrel keys with no exact Library chunk:\n{}",
        unresolved.join("\n")
    );
}

fn real_baseline() -> Baseline {
    let p =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/kb/retrieval-baseline.v1.json");
    let s = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    Baseline::from_json_str(&s).expect("parse baseline")
}

/// Ratchet split only (never the held-out test split), as in `vox kb eval --gate`.
fn ratchet_golden() -> GoldenSet {
    GoldenSet::load(&real_dir(), ratchet_split()).expect("load ratchet split")
}

#[test]
fn real_baseline_shape() {
    let b = real_baseline();
    // Config-aware instead of an exact-epoch pin: an exact pin re-breaks on every legitimate
    // re-epoch. These fail exactly when the scored retriever and the committed baseline diverge.
    assert_eq!(b.retriever, LIBRARY_RETRIEVER);
    assert_eq!(b.chunker, CHUNKER_VERSION);
    assert!(
        b.epoch >= 2,
        "Library-index baselines start at epoch 2, got {}",
        b.epoch
    );
    assert!(
        (b.tolerance - 0.01).abs() < 1e-12,
        "tolerance {}",
        b.tolerance
    );
    assert_eq!(
        b.metrics.keys().map(String::as_str).collect::<Vec<_>>(),
        RATCHET_KEYS
    );
    assert_eq!(b.budget_tokens, 1500);
    assert_eq!(b.budget_tokens, BUDGET_TOKENS);
    assert_eq!(b.token_estimator, "bytes/4");
    assert_eq!(b.token_estimator, TOKEN_ESTIMATOR);
}

#[test]
fn real_retriever_meets_real_baseline() {
    let golden = ratchet_golden();
    let lib = retriever(&golden, EVAL_DEPTH);
    let report = evaluate(&golden, &lib).expect("evaluate");
    let errs = check_regression(&report, &real_baseline());
    assert!(errs.is_empty(), "{errs:#?}");
}

#[test]
fn degraded_retriever_fails_real_baseline() {
    let golden = ratchet_golden();
    let lib = retriever(&golden, EVAL_DEPTH);
    let mut report = run_eval(&golden, lib.doc_chunk_ids(), |q| {
        let mut r = lib.rank(q);
        r.reverse();
        r
    })
    .expect("run_eval");
    // Like-for-like labels: this test must fail on quality, never on a config mismatch.
    report.retriever = LIBRARY_RETRIEVER;
    report.chunker = CHUNKER_VERSION;
    let errs = check_regression(&report, &real_baseline());
    assert!(!errs.is_empty(), "reversed ranking must regress");
    assert!(errs.iter().any(|e| e.contains("dev/all")), "{errs:?}");
    assert!(
        !errs.iter().any(|e| e.contains("not comparable")),
        "must fail on a metric drop, not a config label: {errs:?}"
    );
}
