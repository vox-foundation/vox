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

    /// `retriever/chunker/budget_tokens/token_estimator`: what the metrics were measured with.
    fn config(&self) -> String {
        let (r, c, b, e) = (
            &self.retriever,
            &self.chunker,
            self.budget_tokens,
            &self.token_estimator,
        );
        format!("{r}/{c}/{b}/{e}")
    }

    /// Pretty JSON, ordered keys, trailing newline.
    pub fn to_json_string(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)? + "\n")
    }
}

/// `[regression]` errors: a ratchet metric of `report` fell below the baseline by more than
/// the baseline tolerance (compared at 6 dp), a gated key is missing, or the report was
/// measured with a different retrieval config (retriever, chunker, budget, estimator).
#[must_use]
pub fn check_regression(report: &Report, baseline: &Baseline) -> Vec<String> {
    let mut errs = Vec::new();
    let (r, c, b, e) = (
        report.retriever,
        report.chunker,
        report.budget_tokens,
        report.token_estimator,
    );
    let now = format!("{r}/{c}/{b}/{e}");
    if now != baseline.config() {
        errs.push(format!(
            "[regression] report config {now} != baseline {}: metrics are not comparable; \
             re-epoch with --write-baseline",
            baseline.config()
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
/// - golden set and retrieval config (retriever, chunker, budget, token estimator) unchanged:
///   epoch fixed, no metric lowered, no base key dropped;
/// - golden set or retrieval config changed: epoch must rise and `epoch_note` must be new and
///   non-empty; metrics may move (a new yardstick), but every ratchet key must still exist.
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
    let what = match (golden_changed, head.config() != base.config()) {
        (true, _) => Some("golden set".to_string()),
        (false, true) => Some(format!(
            "retrieval config ({} -> {})",
            base.config(),
            head.config()
        )),
        (false, false) => None,
    };
    if let Some(what) = what {
        if head.epoch <= base.epoch {
            errs.push(t(format!(
                "{what} changed but epoch not bumped ({} -> {})",
                base.epoch, head.epoch
            )));
        }
        let note = head.epoch_note.trim();
        if note.is_empty() || note == base.epoch_note.trim() {
            errs.push(t(format!(
                "{what} changed but epoch_note is empty or unchanged"
            )));
        }
        return errs;
    }
    if head.epoch != base.epoch {
        errs.push(t(format!(
            "epoch changed {} -> {} without a golden-set or retrieval-config change",
            base.epoch, head.epoch
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
mod tests;
