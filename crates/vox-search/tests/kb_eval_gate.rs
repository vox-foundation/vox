mod common;

// Fixture half of SC4: a degraded ranking must fail the regression gate, the real one must not.

use std::path::PathBuf;

use vox_search::kb_eval::baseline::{Baseline, check_regression, new_epoch};
use vox_search::kb_eval::bm25::Bm25Index;
use vox_search::kb_eval::golden::{GoldenSet, Split};
use vox_search::kb_eval::{RankedChunk, Report, run_eval};

fn setup() -> (GoldenSet, Bm25Index) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/kb_eval");
    let golden = GoldenSet::load(&dir, Split::Dev).expect("load fixture");
    let index = Bm25Index::build(&golden.corpus);
    (golden, index)
}

fn eval(
    golden: &GoldenSet,
    index: &Bm25Index,
    degrade: impl Fn(Vec<RankedChunk>) -> Vec<RankedChunk>,
) -> Report {
    run_eval(golden, &index.doc_chunk_ids(), |q| degrade(index.rank(q))).expect("run_eval")
}

fn baseline(golden: &GoldenSet, index: &Bm25Index) -> Baseline {
    new_epoch(None, &eval(golden, index, |r| r), "fixture").expect("baseline")
}

#[test]
fn real_retriever_passes_its_own_baseline() {
    let (golden, index) = setup();
    let base = baseline(&golden, &index);
    let errs = check_regression(&eval(&golden, &index, |r| r), &base);
    assert!(errs.is_empty(), "{errs:?}");
}

#[test]
fn reversed_ranking_fails_regression() {
    let (golden, index) = setup();
    let base = baseline(&golden, &index);
    let report = eval(&golden, &index, |mut r| {
        r.reverse();
        r
    });
    let errs = check_regression(&report, &base);
    assert!(!errs.is_empty(), "reversed ranking must regress");
    assert!(errs.iter().any(|e| e.contains("dev/all")), "{errs:?}");
}

#[test]
fn drop_top_hit_fails_regression() {
    let (golden, index) = setup();
    let base = baseline(&golden, &index);
    let report = eval(&golden, &index, |r| r.into_iter().skip(1).collect());
    let errs = check_regression(&report, &base);
    assert!(!errs.is_empty(), "dropping the top hit must regress");
    assert!(errs.iter().any(|e| e.contains("dev/all")), "{errs:?}");
}
