//! Retrieval baseline file (`contracts/kb/retrieval-baseline.v1.json`): schema, regression
//! check (EVAL-04) and tighten-only ratchet. Pure functions; the CLI only gathers inputs.
//!
//! Every check returns human-readable errors, each starting with `[regression]` or
//! `[tighten-only]`, so a caller can print all failures at once.

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::golden::Split;
use super::metrics::round6;
use super::{Block, LevelMetrics, Report};

/// Report keys the ratchet gates on.
pub const RATCHET_KEYS: &[&str] = &["dev/all", "dev/seed"];
/// Tolerance written by the first epoch (AP-03).
pub const INITIAL_TOLERANCE: f64 = 0.01;

/// The only split a ratchet read or write may touch; held-out `test` is never tuned against.
#[must_use]
pub fn ratchet_split() -> Split {
    Split::Dev
}

/// Metrics of one ratchet key (no query count: it is not a quality metric).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineBlock {
    pub doc: LevelMetrics,
    pub chunk: LevelMetrics,
    pub answer_in_budget: f64,
}

impl From<&Block> for BaselineBlock {
    fn from(b: &Block) -> Self {
        Self {
            doc: b.doc,
            chunk: b.chunk,
            answer_in_budget: b.answer_in_budget,
        }
    }
}

impl BaselineBlock {
    fn flat(&self) -> [(&'static str, f64); 9] {
        let (d, c) = (&self.doc, &self.chunk);
        [
            ("doc recall@5", d.recall_at_5),
            ("doc recall@10", d.recall_at_10),
            ("doc mrr@10", d.mrr_at_10),
            ("doc ndcg@10", d.ndcg_at_10),
            ("chunk recall@5", c.recall_at_5),
            ("chunk recall@10", c.recall_at_10),
            ("chunk mrr@10", c.mrr_at_10),
            ("chunk ndcg@10", c.ndcg_at_10),
            ("answer_in_budget", self.answer_in_budget),
        ]
    }

    fn zip(&self, o: &Self, f: fn(f64, f64) -> f64) -> Self {
        let lv = |a: &LevelMetrics, b: &LevelMetrics| LevelMetrics {
            recall_at_5: f(a.recall_at_5, b.recall_at_5),
            recall_at_10: f(a.recall_at_10, b.recall_at_10),
            mrr_at_10: f(a.mrr_at_10, b.mrr_at_10),
            ndcg_at_10: f(a.ndcg_at_10, b.ndcg_at_10),
        };
        Self {
            doc: lv(&self.doc, &o.doc),
            chunk: lv(&self.chunk, &o.chunk),
            answer_in_budget: f(self.answer_in_budget, o.answer_in_budget),
        }
    }
}

/// Baseline file, schema v1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    pub schema_version: u32,
    pub epoch: u32,
    pub epoch_note: String,
    pub retriever: String,
    pub chunker: String,
    pub token_estimator: String,
    pub budget_tokens: usize,
    pub tolerance: f64,
    pub metrics: BTreeMap<String, BaselineBlock>,
}

impl Baseline {
    pub fn from_json_str(s: &str) -> Result<Self> {
        Ok(serde_json::from_str(s)?)
    }

    /// Pretty JSON, ordered keys, trailing newline.
    pub fn to_json_string(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)? + "\n")
    }
}

/// `[regression]` errors: a ratchet metric of `report` fell below the baseline by more than
/// the baseline tolerance (compared at 6 dp), or a gated key is missing.
#[must_use]
pub fn check_regression(report: &Report, baseline: &Baseline) -> Vec<String> {
    let mut errs = Vec::new();
    if report.budget_tokens != baseline.budget_tokens
        || report.token_estimator != baseline.token_estimator
    {
        errs.push(format!(
            "[regression] report budget {}/{} != baseline {}/{}: metrics are not comparable",
            report.budget_tokens,
            report.token_estimator,
            baseline.budget_tokens,
            baseline.token_estimator
        ));
    }
    for k in RATCHET_KEYS {
        if !baseline.metrics.contains_key(*k) {
            errs.push(format!("[regression] baseline lacks ratchet key {k}"));
        }
    }
    for (key, base) in &baseline.metrics {
        let Some(b) = report.keys.get(key) else {
            errs.push(format!("[regression] report has no {key} block"));
            continue;
        };
        let now = BaselineBlock::from(b);
        for ((name, was), (_, now)) in base.flat().into_iter().zip(now.flat()) {
            if round6(was - now) > round6(baseline.tolerance) {
                errs.push(format!(
                    "[regression] {key} {name}: {was:.6} -> {now:.6} (drop beyond tolerance {:.6})",
                    baseline.tolerance
                ));
            }
        }
    }
    errs
}

/// Metrics where `report` beats the baseline by more than the tolerance (the "run
/// `vox kb eval --tighten`" hint). Informational, never an error.
#[must_use]
pub fn improvements(report: &Report, baseline: &Baseline) -> Vec<String> {
    let mut out = Vec::new();
    for (key, base) in &baseline.metrics {
        let Some(b) = report.keys.get(key) else {
            continue;
        };
        let now = BaselineBlock::from(b);
        for ((name, was), (_, now)) in base.flat().into_iter().zip(now.flat()) {
            if round6(now - was) > round6(baseline.tolerance) {
                out.push(format!("{key} {name}: {was:.6} -> {now:.6}"));
            }
        }
    }
    out
}

/// `baseline` with every value raised to `max(baseline, report)`; never lowers anything.
/// Epoch, note and tolerance are untouched.
#[must_use]
pub fn tightened(baseline: &Baseline, report: &Report) -> Baseline {
    let mut out = baseline.clone();
    for (key, was) in &baseline.metrics {
        if let Some(b) = report.keys.get(key) {
            out.metrics
                .insert(key.clone(), was.zip(&BaselineBlock::from(b), f64::max));
        }
    }
    out
}

/// A fresh epoch from `report`: epoch 1 / tolerance 0.01 with no `prev`, else `prev.epoch + 1`
/// keeping `prev.tolerance`. Needs a non-empty `note` and every ratchet key in the report.
pub fn new_epoch(prev: Option<&Baseline>, report: &Report, note: &str) -> Result<Baseline> {
    if note.trim().is_empty() {
        bail!("a new baseline epoch needs a non-empty epoch_note");
    }
    let mut metrics = BTreeMap::new();
    for k in RATCHET_KEYS {
        let Some(b) = report.keys.get(*k) else {
            bail!("report has no {k} block; cannot write an epoch");
        };
        metrics.insert((*k).to_string(), BaselineBlock::from(b));
    }
    Ok(Baseline {
        schema_version: 1,
        epoch: prev.map_or(1, |p| p.epoch + 1),
        epoch_note: note.trim().to_string(),
        retriever: report.retriever.to_string(),
        chunker: report.chunker.to_string(),
        token_estimator: report.token_estimator.to_string(),
        budget_tokens: report.budget_tokens,
        tolerance: prev.map_or(INITIAL_TOLERANCE, |p| p.tolerance),
        metrics,
    })
}

/// `[tighten-only]` errors for a baseline change between `base` and `head`.
///
/// Agents never self-author a loosening; a re-epoch is human-reviewed (CODEOWNERS /contracts/).
/// `golden_changed` is decided by the caller (AP-15). Rules:
/// - absent at base is a bootstrap (allowed); present at base but absent at head is an error;
/// - tolerance may never rise;
/// - golden unchanged: epoch fixed, no metric lowered, no base key dropped, budget and token
///   estimator fixed;
/// - golden changed: epoch must rise and `epoch_note` must be new and non-empty; metrics may
///   move (the golden set defines a new yardstick), but every ratchet key must still exist.
#[must_use]
pub fn check_tighten_only(
    base: Option<&Baseline>,
    head: Option<&Baseline>,
    golden_changed: bool,
) -> Vec<String> {
    let mut errs = Vec::new();
    let t = |m: String| format!("[tighten-only] {m}");
    let (base, head) = match (base, head) {
        (None, _) => return errs,
        (Some(_), None) => return vec![t("baseline deleted at head but present at base".into())],
        (Some(b), Some(h)) => (b, h),
    };
    if round6(head.tolerance) > round6(base.tolerance) {
        errs.push(t(format!(
            "tolerance raised {:.6} -> {:.6}",
            base.tolerance, head.tolerance
        )));
    }
    for k in RATCHET_KEYS {
        if !head.metrics.contains_key(*k) {
            errs.push(t(format!("head lacks ratchet key {k}")));
        }
    }
    if golden_changed {
        if head.epoch <= base.epoch {
            errs.push(t(format!(
                "golden set changed but epoch not bumped ({} -> {})",
                base.epoch, head.epoch
            )));
        }
        let note = head.epoch_note.trim();
        if note.is_empty() || note == base.epoch_note.trim() {
            errs.push(t(
                "golden set changed but epoch_note is empty or unchanged".into()
            ));
        }
        return errs;
    }
    if head.epoch != base.epoch {
        errs.push(t(format!(
            "epoch changed {} -> {} without a golden-set change",
            base.epoch, head.epoch
        )));
    }
    if head.budget_tokens != base.budget_tokens || head.token_estimator != base.token_estimator {
        errs.push(t(format!(
            "budget/estimator changed {}/{} -> {}/{}",
            base.budget_tokens, base.token_estimator, head.budget_tokens, head.token_estimator
        )));
    }
    for (key, was) in &base.metrics {
        let Some(now) = head.metrics.get(key) else {
            errs.push(t(format!("key {key} removed")));
            continue;
        };
        for ((name, was), (_, now)) in was.flat().into_iter().zip(now.flat()) {
            if round6(now) < round6(was) {
                errs.push(t(format!("{key} {name} lowered {was:.6} -> {now:.6}")));
            }
        }
    }
    errs
}

#[cfg(test)]
mod tests {
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
}
