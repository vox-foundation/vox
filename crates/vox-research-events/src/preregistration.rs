//! `PreregistrationV1` — pre-registration as a typed object, not a Google Doc.
//!
//! Per SCIENTIA plan §5.1: the orchestrator refuses to run a measurement campaign
//! without a signed prereg. Modifications post-collection require a new prereg
//! with an explicit `supersedes` reference.

use serde::{Deserialize, Serialize};

/// Reference to the eval substrate (code + data) for reproducibility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubstrateRef {
    /// Software Heritage identifier for the repository snapshot (ISO/IEC 18670).
    pub repo_swhid: String,
    /// SWHID for the evaluation dataset snapshot.
    pub eval_set_swhid: String,
    /// Optional UK AISI Inspect task identifier.
    pub inspect_task_id: Option<String>,
}

/// What to measure and how to aggregate it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricSpec {
    pub name: String,
    pub aggregation: String,
    pub units: String,
}

/// Frequentist or Bayesian test family.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatisticalTest {
    Frequentist,
    Bayesian,
}

/// Full test specification including priors (Bayesian) or significance level (frequentist).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestSpec {
    pub kind: StatisticalTest,
    /// Bayesian prior expression (e.g. "Beta(1,1)"); None for frequentist.
    pub prior: Option<String>,
    /// Posterior probability threshold for Bayesian; None for frequentist.
    pub threshold: Option<f64>,
    /// Significance level α for frequentist; None for Bayesian.
    pub alpha: Option<f64>,
}

/// Sequential stopping rule — pre-declared to prevent p-hacking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopRule {
    /// Hard cap on sample size.
    pub max_n: u64,
    /// Frequentist α (optional).
    pub alpha: Option<f64>,
    /// Bayesian posterior threshold (optional).
    pub threshold: Option<f64>,
}

/// Explicit decision rule — what "reject" and "fail to reject" mean in context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionRule {
    pub description: String,
}

/// Signed, content-addressable pre-registration record.
///
/// The id field holds a Nanopub Trusty URI (content-hash-in-URI) once the
/// pre-registration is published; during drafting it is a provisional local ID.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreregistrationV1 {
    /// Nanopub Trusty URI — content hash embedded in the URI for integrity.
    pub id: String,
    /// Direction-inclusive hypothesis (not just "we will measure X").
    pub hypothesis: String,
    pub eval_substrate: SubstrateRef,
    pub metric: MetricSpec,
    pub statistical_test: TestSpec,
    pub stopping_rule: StopRule,
    pub decision_rule: DecisionRule,
    /// Hard cost cap in USD; orchestrator aborts the campaign when reached.
    pub cost_cap_usd: f64,
    /// Unix timestamp (seconds) when the prereg was signed.
    pub signed_at: i64,
    /// Ed25519 public key that produced the signature (hex-encoded).
    pub signing_key: String,
    /// Trusty URI of the prereg this supersedes (modifications post-collection).
    pub supersedes: Option<String>,
    /// Git commit hash of the analysis plan code; deviations flag the publication.
    pub analysis_tree_commit: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preregistration_round_trips_json() {
        let prereg = PreregistrationV1 {
            id: "np:RA5x1y2z".to_string(),
            hypothesis: "p95 latency for provider X increases >10ms after model update".to_string(),
            eval_substrate: SubstrateRef {
                repo_swhid: "swh:1:rev:abc123".to_string(),
                eval_set_swhid: "swh:1:dir:def456".to_string(),
                inspect_task_id: Some("task-001".to_string()),
            },
            metric: MetricSpec {
                name: "p95_latency_ms".to_string(),
                aggregation: "percentile_95".to_string(),
                units: "milliseconds".to_string(),
            },
            statistical_test: TestSpec {
                kind: StatisticalTest::Bayesian,
                prior: Some("Beta(1,1)".to_string()),
                threshold: Some(0.95),
                alpha: None,
            },
            stopping_rule: StopRule {
                max_n: 1000,
                alpha: Some(0.05),
                threshold: Some(0.95),
            },
            decision_rule: DecisionRule {
                description: "if posterior P(direction) > 0.95, conclude X".to_string(),
            },
            cost_cap_usd: 50.0,
            signed_at: chrono::Utc::now().timestamp(),
            signing_key: "ed25519:abcdef01234567890".to_string(),
            supersedes: None,
            analysis_tree_commit: None,
        };
        let json = serde_json::to_string(&prereg).unwrap();
        let back: PreregistrationV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, prereg.id);
        assert_eq!(back.cost_cap_usd, 50.0);
        assert_eq!(back.statistical_test.kind, StatisticalTest::Bayesian);
    }
}
