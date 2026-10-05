mod common;

// Contract over the real checked-in golden set (contracts/kb/eval): both stored splits load
// through the strict loader, splits are disjoint and covering, and the canary is on every line.

use std::path::PathBuf;

use vox_search::kb_eval::golden::{GoldenSet, Origin, Split, load_query_texts, validate_splits};

fn real_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../contracts/kb/eval")
}

fn total(m: &std::collections::BTreeMap<Origin, usize>, pred: impl Fn(Origin) -> bool) -> usize {
    m.iter().filter(|(o, _)| pred(**o)).map(|(_, n)| n).sum()
}

#[test]
fn golden_set_loads_both_splits() {
    for split in [Split::Dev, Split::Test] {
        let g = GoldenSet::load(&real_dir(), split).unwrap_or_else(|e| panic!("{split:?}: {e:#}"));
        assert!(!g.queries.is_empty(), "{split:?} has no queries");
    }
}

#[test]
fn golden_set_has_40_seed_queries() {
    let s = validate_splits(&real_dir()).expect("validate_splits");
    let seeds = total(&s.dev, Origin::is_seed) + total(&s.test, Origin::is_seed);
    let expansion = total(&s.dev, |o| o == Origin::LlmExpansion)
        + total(&s.test, |o| o == Origin::LlmExpansion);
    assert!(seeds >= 40, "only {seeds} seed queries");
    assert!(expansion >= 1, "no llm-expansion queries");
}

#[test]
fn splits_disjoint_and_covering() {
    validate_splits(&real_dir()).expect("splits must be disjoint and cover queries.jsonl");
}

#[test]
fn test_fraction_in_range() {
    let s = validate_splits(&real_dir()).expect("validate_splits");
    let test = total(&s.test, |_| true) as f64;
    let all = test + total(&s.dev, |_| true) as f64;
    let frac = test / all;
    assert!((0.20..=0.35).contains(&frac), "test fraction {frac:.3}");
}

#[test]
fn every_query_line_carries_canary() {
    let dir = real_dir();
    let lines = std::fs::read_to_string(dir.join("queries.jsonl"))
        .expect("queries.jsonl")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();
    assert_eq!(load_query_texts(&dir).expect("texts").len(), lines);
    // The loader rejects any line whose metadata.canary differs from the README canary.
    let loaded: usize = [Split::Dev, Split::Test]
        .into_iter()
        .map(|s| GoldenSet::load(&dir, s).expect("load").queries.len())
        .sum();
    assert_eq!(loaded, lines);
}

// Real-data ratchet (EVAL-04): the committed baseline holds for the real retriever on the real
// dev split, and a degraded ranking fails it.

use vox_search::kb_eval::baseline::{Baseline, RATCHET_KEYS, check_regression};
use vox_search::kb_eval::bm25::Bm25Index;
use vox_search::kb_eval::metrics::{BUDGET_TOKENS, TOKEN_ESTIMATOR};
use vox_search::kb_eval::{RankedChunk, Report, run_eval};

fn real_baseline() -> Baseline {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/kb/retrieval-baseline.v1.json");
    let s = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    Baseline::from_json_str(&s).expect("parse baseline")
}

fn real_report(degrade: impl Fn(Vec<RankedChunk>) -> Vec<RankedChunk>) -> Report {
    let golden = GoldenSet::load(&real_dir(), Split::Dev).expect("load dev");
    let idx = Bm25Index::build(&golden.corpus);
    run_eval(&golden, &idx.doc_chunk_ids(), |q| degrade(idx.rank(q))).expect("run_eval")
}

#[test]
fn real_baseline_shape() {
    let b = real_baseline();
    assert_eq!(b.epoch, 1);
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
    let errs = check_regression(&real_report(|r| r), &real_baseline());
    assert!(errs.is_empty(), "{errs:?}");
}

#[test]
fn degraded_retriever_fails_real_baseline() {
    let report = real_report(|mut r| {
        r.reverse();
        r
    });
    let errs = check_regression(&report, &real_baseline());
    assert!(!errs.is_empty(), "reversed ranking must regress");
    assert!(errs.iter().any(|e| e.contains("dev/all")), "{errs:?}");
}
