//! Golden drift benchmark v2 (`LLM-HEAL-02`): real historical doc fixes, labelled by a reviewer,
//! scored against the real ground-truth resolver.
//!
//! The v1 benchmark compared `stale != fixed` and repeated 10 cases five times, so it measured
//! nothing. v2 differs in three ways:
//! - cases are mined from `git log -p` of real doc fixes ([`mine_candidates`]) — *unlabelled*;
//! - a reviewer sets `label` and `reviewed_by`; the resolver's opinion is only a stored hint, so
//!   the benchmark cannot be circular;
//! - promotion eligibility counts only `human:`-reviewed cases (>= 50), never agent or pending ones.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use super::extractor::extract_mentions;
use super::resolver::GroundTruthRegistry;

/// Minimum human-reviewed cases before the benchmark can license the bot.
pub const MIN_HUMAN_REVIEWED_CASES: usize = 50;

/// One historical doc edit: `stale_line` was replaced by `fixed_line` in `fix_commit`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenCaseV2 {
    pub id: String,
    pub fix_commit: String,
    pub doc_path: String,
    pub stale_line: String,
    pub fixed_line: String,
    /// `drift` (the stale line was wrong about the code), `benign` (editorial) or `unlabeled`.
    pub label: String,
    /// `pending`, `agent:<who>` or `human:<who>`.
    pub reviewed_by: String,
    /// What the resolver thought when the case was mined. A hint for the reviewer only.
    #[serde(default)]
    pub resolver_hint_stale_flagged: bool,
    #[serde(default)]
    pub resolver_hint_fixed_flagged: bool,
    #[serde(default)]
    pub notes: String,
}

/// The v2 evaluation set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenSetV2 {
    #[serde(rename = "x-vox-version", default = "v2")]
    pub version: u32,
    pub title: String,
    pub cases: Vec<GoldenCaseV2>,
}

fn v2() -> u32 {
    2
}

/// Benchmark result with provenance, so a number can never hide who produced the labels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoldenMetricsV2 {
    pub total_cases: usize,
    pub pending: usize,
    pub agent_reviewed: usize,
    pub human_reviewed: usize,
    /// Cases actually scored (labelled `drift`/`benign`, reviewer not `pending`).
    pub scored: usize,
    pub true_positives: usize,
    pub false_positives: usize,
    pub true_negatives: usize,
    pub false_negatives: usize,
    pub precision: f64,
    pub recall: f64,
    pub false_positive_rate: f64,
    /// `human_reviewed >= MIN_HUMAN_REVIEWED_CASES` and thresholds met on the human-reviewed subset.
    pub eligible_for_bot_promotion: bool,
    pub ineligible_reason: Option<String>,
}

/// Does `line` contain a prescriptive mention that the resolver reports as missing or retired?
#[must_use]
pub fn line_is_flagged(registry: &GroundTruthRegistry, doc_path: &str, line: &str) -> bool {
    extract_mentions(Path::new(doc_path), line)
        .into_iter()
        .map(|m| registry.resolve_mention(&m))
        .any(|r| r.is_actionable_deficit())
}

/// Mine unlabeled candidate pairs from a `git log -p -U0 --format=@@COMMIT%h -- docs/src` patch.
///
/// Only `docs/src/**/*.md(x)` hunks whose `-` and `+` runs have the same small length are paired
/// line by line, and only when a line carries inline code (something the resolver can check).
#[must_use]
pub fn mine_candidates(patch: &str, registry: &GroundTruthRegistry) -> Vec<GoldenCaseV2> {
    let mut out = Vec::new();
    let mut commit = String::new();
    let mut file = String::new();
    let mut minus: Vec<String> = Vec::new();
    let mut plus: Vec<String> = Vec::new();

    let mut flush = |commit: &str, file: &str, minus: &mut Vec<String>, plus: &mut Vec<String>| {
        if !file.is_empty()
            && !minus.is_empty()
            && minus.len() == plus.len()
            && minus.len() <= 6
            && (file.ends_with(".md") || file.ends_with(".mdx"))
            && file.starts_with("docs/src/")
            && !file.contains("/archive/")
        {
            for (s, f) in minus.iter().zip(plus.iter()) {
                let (s, f) = (s.trim(), f.trim());
                if s == f || s.is_empty() || f.is_empty() || s.len() > 400 || f.len() > 400 {
                    continue;
                }
                if !(s.contains('`') || f.contains('`')) {
                    continue;
                }
                out.push(GoldenCaseV2 {
                    id: String::new(),
                    fix_commit: commit.to_string(),
                    doc_path: file.to_string(),
                    stale_line: s.to_string(),
                    fixed_line: f.to_string(),
                    label: "unlabeled".to_string(),
                    reviewed_by: "pending".to_string(),
                    resolver_hint_stale_flagged: line_is_flagged(registry, file, s),
                    resolver_hint_fixed_flagged: line_is_flagged(registry, file, f),
                    notes: String::new(),
                });
            }
        }
        minus.clear();
        plus.clear();
    };

    for line in patch.lines() {
        if let Some(h) = line.strip_prefix("@@COMMIT") {
            flush(&commit, &file, &mut minus, &mut plus);
            commit = h.trim().to_string();
            file.clear();
        } else if let Some(rest) = line.strip_prefix("diff --git ") {
            flush(&commit, &file, &mut minus, &mut plus);
            file = rest.split(" b/").nth(1).unwrap_or("").trim().to_string();
        } else if line.starts_with("@@ ") {
            flush(&commit, &file, &mut minus, &mut plus);
        } else if line.starts_with("---") || line.starts_with("+++") {
            // file header lines
        } else if let Some(m) = line.strip_prefix('-') {
            if !plus.is_empty() {
                flush(&commit, &file, &mut minus, &mut plus);
            }
            minus.push(m.to_string());
        } else if let Some(p) = line.strip_prefix('+') {
            plus.push(p.to_string());
        }
    }
    flush(&commit, &file, &mut minus, &mut plus);

    for (i, c) in out.iter_mut().enumerate() {
        c.id = format!("GD2-{:04}", i + 1);
    }
    out
}

/// Score the reviewed cases of a v2 set with the real resolver.
pub fn evaluate_golden_set_v2(
    path: &Path,
    registry: &GroundTruthRegistry,
) -> Result<GoldenMetricsV2> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("read golden drift set {}", path.display()))?;
    let set: GoldenSetV2 = serde_json::from_str(&raw).context("parse golden drift set v2")?;
    Ok(score(&set.cases, registry))
}

fn score(cases: &[GoldenCaseV2], registry: &GroundTruthRegistry) -> GoldenMetricsV2 {
    let (mut pending, mut agent, mut human) = (0usize, 0usize, 0usize);
    let (mut tp, mut fp, mut tn, mut fneg) = (0usize, 0usize, 0usize, 0usize);
    let (mut h_tp, mut h_fp, mut h_tn) = (0usize, 0usize, 0usize);
    let mut scored = 0usize;

    for c in cases {
        let is_human = c.reviewed_by.starts_with("human:");
        let is_agent = c.reviewed_by.starts_with("agent:");
        if is_human {
            human += 1;
        } else if is_agent {
            agent += 1;
        } else {
            pending += 1;
        }
        if c.label != "drift" && c.label != "benign" || !(is_human || is_agent) {
            continue;
        }
        scored += 1;
        let stale_flagged = line_is_flagged(registry, &c.doc_path, &c.stale_line);
        let fixed_flagged = line_is_flagged(registry, &c.doc_path, &c.fixed_line);
        let is_drift = c.label == "drift";
        // Stale line: should be flagged iff it was real drift.
        let (a_tp, a_fp, a_tn, a_fn) = match (is_drift, stale_flagged) {
            (true, true) => (1, 0, 0, 0),
            (true, false) => (0, 0, 0, 1),
            (false, true) => (0, 1, 0, 0),
            (false, false) => (0, 0, 1, 0),
        };
        // Fixed line: should never be flagged.
        let (b_fp, b_tn) = if fixed_flagged { (1, 0) } else { (0, 1) };
        tp += a_tp;
        fp += a_fp + b_fp;
        tn += a_tn + b_tn;
        fneg += a_fn;
        if is_human {
            h_tp += a_tp;
            h_fp += a_fp + b_fp;
            h_tn += a_tn + b_tn;
        }
    }

    let ratio = |n: usize, d: usize, empty: f64| {
        if d == 0 { empty } else { n as f64 / d as f64 }
    };
    let h_precision = ratio(h_tp, h_tp + h_fp, 0.0);
    let h_fpr = ratio(h_fp, h_fp + h_tn, 1.0);
    let ineligible_reason = if human < MIN_HUMAN_REVIEWED_CASES {
        Some(format!(
            "only {human} human-reviewed cases; {MIN_HUMAN_REVIEWED_CASES} required"
        ))
    } else if h_precision < 0.80 {
        Some(format!("human-reviewed precision {h_precision:.2} < 0.80"))
    } else if h_fpr > 0.10 {
        Some(format!(
            "human-reviewed false-positive rate {h_fpr:.2} > 0.10"
        ))
    } else {
        None
    };
    GoldenMetricsV2 {
        total_cases: cases.len(),
        pending,
        agent_reviewed: agent,
        human_reviewed: human,
        scored,
        true_positives: tp,
        false_positives: fp,
        true_negatives: tn,
        false_negatives: fneg,
        precision: ratio(tp, tp + fp, 0.0),
        recall: ratio(tp, tp + fneg, 0.0),
        false_positive_rate: ratio(fp, fp + tn, 0.0),
        eligible_for_bot_promotion: ineligible_reason.is_none(),
        ineligible_reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> GroundTruthRegistry {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        GroundTruthRegistry::load(root).expect("registry")
    }

    #[test]
    fn mines_equal_length_runs_in_docs_src_only() {
        let patch = "\
@@COMMIT abc1234
diff --git a/docs/src/reference/x.md b/docs/src/reference/x.md
--- a/docs/src/reference/x.md
+++ b/docs/src/reference/x.md
@@ -3,1 +3,1 @@
-Run `vox-nonexistent-zzz` to start.
+Run `vox-cli` to start.
@@ -9,2 +9,1 @@
-two
-lines
+one
diff --git a/crates/x/src/lib.rs b/crates/x/src/lib.rs
@@ -1 +1 @@
-let a = `x`;
+let a = `y`;
";
        let got = mine_candidates(patch, &registry());
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0].fix_commit, "abc1234");
        assert_eq!(got[0].doc_path, "docs/src/reference/x.md");
        assert_eq!(got[0].label, "unlabeled");
        assert_eq!(got[0].reviewed_by, "pending");
        assert_eq!(got[0].id, "GD2-0001");
    }

    #[test]
    fn eligibility_ignores_pending_and_agent_labels() {
        let reg = registry();
        let mut cases = Vec::new();
        for i in 0..60 {
            cases.push(GoldenCaseV2 {
                id: format!("c{i}"),
                fix_commit: "x".into(),
                doc_path: "docs/src/reference/x.md".into(),
                stale_line: "Run `crates/vox-nonexistent-zzz/src/lib.rs` now.".into(),
                fixed_line: "Run `crates/vox-doc-pipeline/src/lib.rs` now.".into(),
                label: "drift".into(),
                reviewed_by: if i % 2 == 0 { "agent:test" } else { "pending" }.into(),
                resolver_hint_stale_flagged: false,
                resolver_hint_fixed_flagged: false,
                notes: String::new(),
            });
        }
        let m = score(&cases, &reg);
        assert_eq!(m.human_reviewed, 0);
        assert_eq!(m.agent_reviewed, 30);
        assert_eq!(m.pending, 30);
        assert_eq!(m.scored, 30);
        assert!(!m.eligible_for_bot_promotion, "{m:?}");
        assert!(m.ineligible_reason.is_some());
    }

    #[test]
    fn a_missing_file_in_the_stale_line_is_flagged_and_the_fixed_line_is_not() {
        let reg = registry();
        let doc = "docs/src/reference/x.md";
        assert!(line_is_flagged(
            &reg,
            doc,
            "Open `crates/vox-nonexistent-zzz/src/lib.rs` first."
        ));
        assert!(!line_is_flagged(
            &reg,
            doc,
            "Open `crates/vox-doc-pipeline/src/lib.rs` first."
        ));
    }
}
