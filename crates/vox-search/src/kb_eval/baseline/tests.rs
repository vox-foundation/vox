use super::*;
use crate::kb_eval::{CHUNKER, RETRIEVER, metrics::BUDGET_TOKENS, metrics::TOKEN_ESTIMATOR};

fn level(v: f64) -> LevelMetrics {
    LevelMetrics {
        recall_at_5: v,
        recall_at_10: v,
        mrr_at_10: v,
        ndcg_at_10: v,
    }
}

fn report(v: f64) -> Report {
    let b = Block {
        queries: 2,
        doc: level(v),
        chunk: level(v),
        answer_in_budget: v,
    };
    Report {
        split: "dev".into(),
        retriever: RETRIEVER,
        chunker: CHUNKER,
        budget_tokens: BUDGET_TOKENS,
        token_estimator: TOKEN_ESTIMATOR,
        queries: 4,
        keys: [("dev/all", b.clone()), ("dev/seed", b)]
            .into_iter()
            .map(|(k, b)| (k.to_string(), b))
            .collect(),
    }
}

fn base() -> Baseline {
    new_epoch(None, &report(0.5), "init").unwrap()
}

#[test]
fn regression_flags_drop_beyond_tolerance() {
    let b = base();
    assert!(check_regression(&report(0.5), &b).is_empty());
    let mut r = report(0.5);
    r.keys.get_mut("dev/all").unwrap().chunk.recall_at_5 = 0.48;
    let errs = check_regression(&r, &b);
    assert_eq!(errs.len(), 1, "{errs:?}");
    for needle in [
        "[regression]",
        "dev/all",
        "chunk recall@5",
        "0.500000",
        "0.480000",
    ] {
        assert!(errs[0].contains(needle), "{needle} missing in {}", errs[0]);
    }
    // Exactly the tolerance is allowed.
    r.keys.get_mut("dev/all").unwrap().chunk.recall_at_5 = 0.49;
    assert!(check_regression(&r, &b).is_empty());
    r.keys.remove("dev/seed");
    let errs = check_regression(&r, &b);
    assert!(errs.iter().any(|e| e.contains("dev/seed")), "{errs:?}");
}

#[test]
fn tighten_only_rejects_lowered_metric() {
    let b = base();
    let mut h = b.clone();
    h.metrics.get_mut("dev/all").unwrap().doc.mrr_at_10 = 0.4;
    let errs = check_tighten_only(Some(&b), Some(&h), false);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(errs[0].starts_with("[tighten-only]") && errs[0].contains("doc mrr@10"));
    h.metrics.get_mut("dev/all").unwrap().doc.mrr_at_10 = 0.5;
    assert!(check_tighten_only(Some(&b), Some(&h), false).is_empty());
    h.metrics.get_mut("dev/all").unwrap().doc.mrr_at_10 = 0.9;
    assert!(check_tighten_only(Some(&b), Some(&h), false).is_empty());
}

#[test]
fn tighten_only_rejects_raised_tolerance() {
    let b = base();
    let mut h = b.clone();
    h.tolerance = 0.05;
    assert_eq!(check_tighten_only(Some(&b), Some(&h), false).len(), 1);
    // Even a legitimate re-epoch may not loosen the tolerance.
    h.epoch = 2;
    h.epoch_note = "new golden".into();
    let errs = check_tighten_only(Some(&b), Some(&h), true);
    assert!(errs.iter().any(|e| e.contains("tolerance")), "{errs:?}");
}

#[test]
fn tighten_only_requires_epoch_bump_on_golden_change() {
    let b = base();
    let mut h = b.clone();
    h.metrics.get_mut("dev/all").unwrap().chunk.ndcg_at_10 = 0.1;
    assert!(
        !check_tighten_only(Some(&b), Some(&h), true).is_empty(),
        "same epoch"
    );
    h.epoch = 2;
    assert!(
        !check_tighten_only(Some(&b), Some(&h), true).is_empty(),
        "same note"
    );
    h.epoch_note = "  ".into();
    assert!(
        !check_tighten_only(Some(&b), Some(&h), true).is_empty(),
        "empty note"
    );
    h.epoch_note = "golden grew to 120 queries".into();
    assert!(
        check_tighten_only(Some(&b), Some(&h), true).is_empty(),
        "bump + note"
    );
    // An epoch bump with an unchanged golden set is not a licence to lower values.
    assert!(!check_tighten_only(Some(&b), Some(&h), false).is_empty());
}

#[test]
fn tighten_only_rejects_deleted_baseline() {
    let errs = check_tighten_only(Some(&base()), None, false);
    assert_eq!(errs.len(), 1);
    assert!(errs[0].contains("deleted"));
}

#[test]
fn bootstrap_is_allowed() {
    assert!(check_tighten_only(None, Some(&base()), false).is_empty());
    assert!(check_tighten_only(None, Some(&base()), true).is_empty());
    assert!(check_tighten_only(None, None, false).is_empty());
}

#[test]
fn tighten_only_rejects_dropped_key_and_budget_change() {
    let b = base();
    let mut h = b.clone();
    h.metrics.remove("dev/seed");
    assert!(!check_tighten_only(Some(&b), Some(&h), false).is_empty());
    let mut h = b.clone();
    h.budget_tokens = 3000;
    assert!(!check_tighten_only(Some(&b), Some(&h), false).is_empty());
    let mut h = b.clone();
    h.token_estimator = "words".into();
    assert!(!check_tighten_only(Some(&b), Some(&h), false).is_empty());
}

#[test]
fn tightened_never_lowers() {
    let b = base();
    let mut r = report(0.9);
    r.keys.get_mut("dev/all").unwrap().doc.recall_at_5 = 0.1;
    let t = tightened(&b, &r);
    let all = &t.metrics["dev/all"];
    assert_eq!(
        all.doc.recall_at_5, 0.5,
        "lower report value must not lower baseline"
    );
    assert_eq!(all.chunk.ndcg_at_10, 0.9);
    assert_eq!(t.metrics["dev/seed"].answer_in_budget, 0.9);
    assert_eq!((t.epoch, t.tolerance), (b.epoch, b.tolerance));
    assert!(check_tighten_only(Some(&b), Some(&t), false).is_empty());
}

#[test]
fn improvements_lists_gains_beyond_tolerance() {
    let b = base();
    assert!(improvements(&report(0.505), &b).is_empty());
    let imp = improvements(&report(0.6), &b);
    assert_eq!(imp.len(), 18, "9 metrics x 2 keys: {imp:?}");
}

#[test]
fn new_epoch_rules() {
    let e1 = new_epoch(None, &report(0.5), "fixture").unwrap();
    assert_eq!((e1.epoch, e1.tolerance), (1, INITIAL_TOLERANCE));
    let mut prev = e1.clone();
    prev.tolerance = 0.005;
    let e2 = new_epoch(Some(&prev), &report(0.6), "bigger").unwrap();
    assert_eq!((e2.epoch, e2.tolerance), (2, 0.005));
    assert!(new_epoch(None, &report(0.5), " ").is_err());
    let mut r = report(0.5);
    r.keys.remove("dev/seed");
    assert!(new_epoch(None, &r, "x").is_err());
}

#[test]
fn ratchet_split_is_dev() {
    assert_eq!(ratchet_split(), Split::Dev);
    assert_eq!(RATCHET_KEYS, ["dev/all", "dev/seed"]);
}

#[test]
fn json_round_trips_and_rejects_unknown_fields() {
    let s = base().to_json_string().unwrap();
    assert!(s.ends_with("}\n"));
    let back = Baseline::from_json_str(&s).unwrap();
    assert_eq!(back, base());
    assert_eq!(back.to_json_string().unwrap(), s);
    let bad = s.replacen("\"epoch\"", "\"extra\": 1,\n  \"epoch\"", 1);
    assert!(Baseline::from_json_str(&bad).is_err());
}

#[test]
fn config_change_licenses_reepoch_and_regression_compares_it() {
    let b = base();
    let edits: [fn(&mut Baseline); 4] = [
        |h| h.retriever = "dense-v2".into(),
        |h| h.chunker = "md-h2-512-v2".into(),
        |h| h.budget_tokens = 3000,
        |h| h.token_estimator = "words".into(),
    ];
    for edit in edits {
        let mut h = b.clone();
        edit(&mut h);
        h.metrics.get_mut("dev/all").unwrap().doc.mrr_at_10 = 0.1;
        assert!(!check_tighten_only(Some(&b), Some(&h), false).is_empty());
        h.epoch = 2;
        assert!(
            !check_tighten_only(Some(&b), Some(&h), false).is_empty(),
            "same note"
        );
        h.epoch_note = "new retriever".into();
        let errs = check_tighten_only(Some(&b), Some(&h), false);
        assert!(errs.is_empty(), "{errs:?}");
    }
    let mut r = report(0.5);
    r.retriever = "dense-v2";
    assert!(
        check_regression(&r, &b)
            .iter()
            .any(|e| e.contains("dense-v2"))
    );
    let mut r = report(0.5);
    r.chunker = "md-h2-512-v2";
    assert!(
        check_regression(&r, &b)
            .iter()
            .any(|e| e.contains("md-h2-512-v2"))
    );
}
