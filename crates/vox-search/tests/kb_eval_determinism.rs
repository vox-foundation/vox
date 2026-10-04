mod common;

// End-to-end fixture eval: BEIR files to a deterministic doc+chunk report.

use std::path::PathBuf;

use vox_search::kb_eval::bm25::Bm25Index;
use vox_search::kb_eval::golden::{GoldenSet, Split};
use vox_search::kb_eval::{Report, run_eval};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/kb_eval")
}

fn eval_fixture() -> Report {
    let golden = GoldenSet::load(&fixture_dir(), Split::Dev).expect("load fixture");
    let index = Bm25Index::build(&golden.corpus);
    run_eval(&golden, |q| index.rank(q)).expect("run_eval")
}

#[test]
fn fixture_eval_end_to_end() {
    let report = eval_fixture();
    let all = report.keys.get("dev/all").expect("dev/all key");
    assert_eq!(all.queries, 3);
    // q001's unique terms live only in alpha's Install section: doc rank 1, so MRR is 1.0
    // for q001 and the mean over three queries is 1.0 only if every query ranks its doc first.
    assert_eq!(all.doc.mrr_at_10, 1.0);
    assert!(all.doc.recall_at_10 > 0.0 && all.chunk.recall_at_10 > 0.0);
    assert!(report.render_text().contains("dev/all queries=3"));
}

#[test]
fn fixture_eval_is_byte_identical() {
    let a = eval_fixture().render_text();
    let b = eval_fixture().render_text();
    assert_eq!(a, b);
}
