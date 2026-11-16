//! Generic held-out suite report (`vox_mens_suite_report_v1`) and paired verdict logic.

use crate::reasoning_slice::mcnemar_one_sided_p;
use crate::report::EvalCorpusReport;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const SUITE_REPORT_SCHEMA_V1: &str = "vox_mens_suite_report_v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupCount {
    pub n: usize,
    pub passed: usize,
    pub pass_rate: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuiteItem {
    pub id: String,
    pub group: String,
    pub passed: bool,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuiteReport {
    pub schema: String,
    pub suite: String,
    pub subject: String,
    pub n: usize,
    pub passed: usize,
    pub pass_rate: f64,
    #[serde(default)]
    pub groups: BTreeMap<String, GroupCount>,
    #[serde(default)]
    pub items: Vec<SuiteItem>,
    #[serde(default)]
    pub decode: Value,
    #[serde(default)]
    pub inputs: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SuiteFloors {
    pub min_pass_rate: f64,
    pub min_group_pass_rate: Option<f64>,
    pub alpha: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SuiteVerdict {
    pub passed: bool,
    pub reasons: Vec<String>,
    pub improvement_p: Option<f64>,
}

impl SuiteReport {
    /// Build a `SuiteReport` from scored episode items.
    #[must_use]
    pub fn from_episode_scores(
        suite: impl Into<String>,
        subject: impl Into<String>,
        items: Vec<SuiteItem>,
        decode: Value,
        inputs: BTreeMap<String, String>,
    ) -> Self {
        let n = items.len();
        let passed = items.iter().filter(|i| i.passed).count();
        let pass_rate = if n == 0 {
            0.0
        } else {
            passed as f64 / n as f64
        };
        let mut group_counts: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        for item in &items {
            let e = group_counts.entry(item.group.clone()).or_insert((0, 0));
            e.0 += 1;
            if item.passed {
                e.1 += 1;
            }
        }
        let groups = group_counts
            .into_iter()
            .map(|(k, (gn, gp))| {
                (
                    k,
                    GroupCount {
                        n: gn,
                        passed: gp,
                        pass_rate: if gn == 0 { 0.0 } else { gp as f64 / gn as f64 },
                    },
                )
            })
            .collect();

        Self {
            schema: SUITE_REPORT_SCHEMA_V1.to_string(),
            suite: suite.into(),
            subject: subject.into(),
            n,
            passed,
            pass_rate,
            groups,
            items,
            decode,
            inputs,
        }
    }

    /// Convert a HumanEval-Vox [`EvalCorpusReport`] into a generic [`SuiteReport`].
    #[must_use]
    pub fn from_eval_corpus_report(r: &EvalCorpusReport, suite: &str) -> Self {
        let items: Vec<SuiteItem> = r
            .fixtures
            .iter()
            .map(|f| SuiteItem {
                id: f.fixture_id.clone(),
                group: "humaneval_vox".to_string(),
                passed: f.c > 0,
                reason: if f.c > 0 {
                    "ok".to_string()
                } else {
                    "failed hidden tests".to_string()
                },
            })
            .collect();
        let mut inputs = BTreeMap::new();
        if let Some(hash) = &r.provenance.problem_set_hash {
            inputs.insert("items_hash".to_string(), hash.clone());
        }
        inputs.insert("model_id".to_string(), r.model_id.clone());
        Self::from_episode_scores(
            suite,
            r.model_id.clone(),
            items,
            serde_json::json!({
                "harness_id": r.harness_id,
                "config_digest": r.config_digest,
            }),
            inputs,
        )
    }
}

/// Evaluate whether `adapter` meets `floors` and statistically improves on paired `base`.
#[must_use]
pub fn suite_verdict(
    adapter: &SuiteReport,
    base: Option<&SuiteReport>,
    floors: &SuiteFloors,
) -> SuiteVerdict {
    let mut reasons = Vec::new();
    if adapter.pass_rate + 1e-12 < floors.min_pass_rate {
        reasons.push(format!(
            "overall pass_rate {:.4} below min_pass_rate {:.4}",
            adapter.pass_rate, floors.min_pass_rate
        ));
    }
    if let Some(group_floor) = floors.min_group_pass_rate {
        for (group, count) in &adapter.groups {
            if count.pass_rate + 1e-12 < group_floor {
                reasons.push(format!(
                    "group '{group}' pass_rate {:.4} below min_group_pass_rate {:.4}",
                    count.pass_rate, group_floor
                ));
            }
        }
    }

    let improvement_p = match base {
        None => {
            reasons.push("no base control run".to_string());
            None
        }
        Some(ctrl) => {
            let ctrl_map: BTreeMap<&str, bool> = ctrl
                .items
                .iter()
                .map(|i| (i.id.as_str(), i.passed))
                .collect();
            let mut adapter_only = 0usize;
            let mut base_only = 0usize;
            for item in &adapter.items {
                if let Some(&base_ok) = ctrl_map.get(item.id.as_str()) {
                    if item.passed && !base_ok {
                        adapter_only += 1;
                    } else if !item.passed && base_ok {
                        base_only += 1;
                    }
                }
            }
            // Test for gains over base: probability of >= adapter_only wins given discordant pairs.
            let p = mcnemar_one_sided_p(adapter_only, base_only);
            if p >= floors.alpha {
                reasons.push(format!(
                    "paired improvement over base not significant (gains={adapter_only}, losses={base_only}, p={p:.5} >= alpha={:.5})",
                    floors.alpha
                ));
            }
            Some(p)
        }
    };

    SuiteVerdict {
        passed: reasons.is_empty(),
        reasons,
        improvement_p,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_report(subject: &str, results: &[(&str, &str, bool)]) -> SuiteReport {
        let items = results
            .iter()
            .map(|(id, g, ok)| SuiteItem {
                id: (*id).to_string(),
                group: (*g).to_string(),
                passed: *ok,
                reason: if *ok { "ok".into() } else { "fail".into() },
            })
            .collect();
        SuiteReport::from_episode_scores(
            "test_suite",
            subject,
            items,
            serde_json::json!({}),
            BTreeMap::new(),
        )
    }

    #[test]
    fn suite_verdict_pins_improvement_direction_and_floors() {
        let adapter_wins: Vec<(String, String, bool)> = (0..10)
            .map(|i| (format!("item_{i}"), "g1".to_string(), true))
            .collect();
        let base_loses: Vec<(String, String, bool)> = (0..10)
            .map(|i| (format!("item_{i}"), "g1".to_string(), false))
            .collect();
        let a_refs: Vec<(&str, &str, bool)> = adapter_wins
            .iter()
            .map(|(a, b, c)| (a.as_str(), b.as_str(), *c))
            .collect();
        let b_refs: Vec<(&str, &str, bool)> = base_loses
            .iter()
            .map(|(a, b, c)| (a.as_str(), b.as_str(), *c))
            .collect();

        let adapter = mk_report("adapter", &a_refs);
        let base = mk_report("base", &b_refs);
        let floors = SuiteFloors {
            min_pass_rate: 0.80,
            min_group_pass_rate: Some(0.70),
            alpha: 0.05,
        };

        let good = suite_verdict(&adapter, Some(&base), &floors);
        assert!(good.passed, "{:?}", good.reasons);
        assert!(good.improvement_p.unwrap() < 0.01);

        // Reverse: adapter loses 10 of 10 discordant pairs -> p > 0.5
        let bad = suite_verdict(&base, Some(&adapter), &floors);
        assert!(!bad.passed);
        assert!(bad.improvement_p.unwrap() > 0.5);

        let no_base = suite_verdict(&adapter, None, &floors);
        assert!(!no_base.passed);
        assert!(no_base.reasons.iter().any(|r| r == "no base control run"));
    }

    #[test]
    fn suite_report_serde_round_trip() {
        let rep = mk_report("subj", &[("i1", "g1", true)]);
        assert_eq!(rep.schema, "vox_mens_suite_report_v1");
        let s = serde_json::to_string(&rep).unwrap();
        let back: SuiteReport = serde_json::from_str(&s).unwrap();
        assert_eq!(rep, back);
    }
}
