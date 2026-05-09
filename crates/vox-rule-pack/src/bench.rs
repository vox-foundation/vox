//! Authoring-time bench: runs rules against fixtures, computes precision/recall.
//!
//! Pure function over RulePack + filesystem of fixtures. No network, no LLM.

use crate::pack::RulePack;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Per-rule precision/recall statistics from the fixture corpus.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuleBenchResult {
    pub rule_id: String,
    pub positive_total: u32,
    pub positive_matched: u32,
    pub negative_total: u32,
    pub negative_matched: u32,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
}

/// Full bench report across all rules in a pack.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchReport {
    pub generated_at_unix: u64,
    pub rules: Vec<RuleBenchResult>,
}

/// Runs the bench against `pack`, resolving fixture files relative to `fixtures_root`.
///
/// Fixture file naming convention: `<rule-sub-id>_pos_*.txt` and
/// `<rule-sub-id>_neg_*.txt` inside `<fixtures_root>/<rule-parent-id>/`.
/// For example, rule `victory-claim/premature` resolves to
/// `<fixtures_root>/victory-claim/premature_pos_*.txt`.
pub fn run_bench(pack: &RulePack, fixtures_root: &Path) -> BenchReport {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut results = Vec::with_capacity(pack.len());
    for rule in pack.rules() {
        results.push(score_rule(rule, fixtures_root));
    }
    BenchReport {
        generated_at_unix: now,
        rules: results,
    }
}

fn score_rule(rule: &crate::pack::CompiledRule, fixtures_root: &Path) -> RuleBenchResult {
    let parts: Vec<&str> = rule.id.splitn(2, '/').collect();
    let parent = parts[0];
    let sub = if parts.len() > 1 { parts[1] } else { "" };
    let dir = fixtures_root.join(parent);

    let mut pos_total = 0u32;
    let mut pos_match = 0u32;
    let mut neg_total = 0u32;
    let mut neg_match = 0u32;

    if let Ok(entries) = std::fs::read_dir(&dir) {
        let mut paths: Vec<_> = entries.flatten().collect();
        paths.sort_by_key(|e| e.file_name());
        for entry in paths {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_pos = if sub.is_empty() {
                name.contains("_pos")
            } else {
                name.starts_with(&format!("{sub}_pos"))
            };
            let is_neg = if sub.is_empty() {
                name.contains("_neg")
            } else {
                name.starts_with(&format!("{sub}_neg"))
            };
            if !is_pos && !is_neg {
                continue;
            }
            let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
            // Each fixture is a single line; take the first non-empty line.
            let line = content.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
            let matched = rule.matches_line(line);
            if is_pos {
                pos_total += 1;
                if matched {
                    pos_match += 1;
                }
            } else {
                neg_total += 1;
                if matched {
                    neg_match += 1;
                }
            }
        }
    }

    let tp = pos_match as f64;
    let fp = neg_match as f64;
    let fn_ = (pos_total.saturating_sub(pos_match)) as f64;
    let precision = if tp + fp == 0.0 { 1.0 } else { tp / (tp + fp) };
    let recall = if tp + fn_ == 0.0 {
        1.0
    } else {
        tp / (tp + fn_)
    };
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };

    RuleBenchResult {
        rule_id: rule.id.clone(),
        positive_total: pos_total,
        positive_matched: pos_match,
        negative_total: neg_total,
        negative_matched: neg_match,
        precision,
        recall,
        f1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_fixture(p: &Path, content: &str) {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(p).unwrap();
        writeln!(f, "{content}").unwrap();
    }

    #[test]
    fn perfect_classifier_reports_f1_one() {
        let yaml = r#"
version: 1
rules:
  - id: alpha/foo
    name: F
    description: F
    severity: warning
    languages: [rust]
    match: { kind: line-regex, pattern: "^foo$" }
    message: m
"#;
        let pack = RulePack::load_from_str(yaml).unwrap();
        let dir = TempDir::new().unwrap();
        write_fixture(&dir.path().join("alpha").join("foo_pos_a.txt"), "foo");
        write_fixture(&dir.path().join("alpha").join("foo_neg_a.txt"), "bar");
        let report = run_bench(&pack, dir.path());
        let r = &report.rules[0];
        assert_eq!(r.positive_total, 1);
        assert_eq!(r.positive_matched, 1);
        assert_eq!(r.negative_total, 1);
        assert_eq!(r.negative_matched, 0);
        assert!((r.f1 - 1.0).abs() < 1e-9, "expected f1=1.0, got {}", r.f1);
    }

    #[test]
    fn false_positive_lowers_precision() {
        let yaml = r#"
version: 1
rules:
  - id: beta/bar
    name: B
    description: B
    severity: warning
    languages: [rust]
    match: { kind: line-regex, pattern: "bar" }
    message: m
"#;
        let pack = RulePack::load_from_str(yaml).unwrap();
        let dir = TempDir::new().unwrap();
        write_fixture(&dir.path().join("beta").join("bar_pos_1.txt"), "bar");
        write_fixture(&dir.path().join("beta").join("bar_neg_1.txt"), "bar_extra");
        let report = run_bench(&pack, dir.path());
        let r = &report.rules[0];
        // Both pos and neg match "bar" → fp=1, tp=1 → precision=0.5
        assert_eq!(r.positive_matched, 1);
        assert_eq!(r.negative_matched, 1);
        assert!((r.precision - 0.5).abs() < 1e-9);
    }

    #[test]
    fn no_fixtures_reports_perfect() {
        let yaml = r#"
version: 1
rules:
  - id: gamma/baz
    name: G
    description: G
    severity: info
    languages: [rust]
    match: { kind: line-regex, pattern: "baz" }
    message: m
"#;
        let pack = RulePack::load_from_str(yaml).unwrap();
        let dir = TempDir::new().unwrap();
        let report = run_bench(&pack, dir.path());
        let r = &report.rules[0];
        // Zero fixtures → no FP, no FN → precision=1, recall=1, f1=1
        assert_eq!(r.positive_total, 0);
        assert_eq!(r.negative_total, 0);
        assert!((r.f1 - 1.0).abs() < 1e-9);
    }
}
