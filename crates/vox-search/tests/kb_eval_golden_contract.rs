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
