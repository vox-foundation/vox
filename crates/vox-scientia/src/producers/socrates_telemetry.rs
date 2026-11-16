//! Socrates-telemetry signal producer.
//!
//! 1. Reads `agent_telemetry_flat` rows of `event_kind = 'trust_obs'` and emits
//!    a `telemetry_trust` candidate when the trailing trust-score window shows
//!    a sustained improvement (mean trust delta ≥ [`MIN_TRUST_IMPROVEMENT`])
//!    with both windows containing at least [`MIN_SAMPLES_PER_WINDOW`] samples.
//! 2. Mines `research_metrics` (`socrates_surface`) for high-entropy abstention
//!    and contradiction clusters (`detect_socrates_anomaly_clusters`) and emits
//!    structured candidate signals into SCIENTIA discovery.

use std::collections::BTreeMap;

use async_trait::async_trait;
use vox_research_events::ResearchEvent;
use vox_research_events::schema_types::{
    DiscoverySignal, DiscoverySignalFamily, DiscoverySignalStrength, FindingCandidateClass,
    FindingCandidateConfidence, FindingCandidateV1, SignalProvenance,
};

use super::heuristics::date_slug;
use super::producer::{Producer, ProducerContext};

const PRODUCER_NAME: &str = "socrates_telemetry";
/// Minimum trust observations per window.
pub const MIN_SAMPLES_PER_WINDOW: usize = 30;
/// Minimum mean trust-score improvement (`trailing - prior`) to emit.
pub const MIN_TRUST_IMPROVEMENT: f64 = 0.05;
/// Minimum surface samples in a cluster before emitting an anomaly signal.
pub const MIN_CLUSTER_SAMPLES: usize = 5;
/// Abstention rate threshold when paired with high confidence entropy.
pub const MIN_CLUSTER_ABSTAIN_RATE: f64 = 0.30;
/// Normalized 5-bin confidence Shannon entropy threshold.
pub const MIN_CLUSTER_CONFIDENCE_ENTROPY: f64 = 0.55;
/// Mean contradiction ratio threshold for emitting a contradiction cluster.
pub const MIN_CLUSTER_CONTRADICTION_RATE: f64 = 0.30;

/// Parsed Socrates surface observation used for cluster mining.
#[derive(Debug, Clone, PartialEq)]
pub struct SocratesSurfaceSample {
    pub surface: String,
    pub cluster_key: String,
    pub is_abstain: bool,
    pub confidence_estimate: f64,
    pub contradiction_ratio: f64,
    pub hallucination_risk_proxy: f64,
}

/// Detected high-entropy abstention or contradiction cluster signal.
#[derive(Debug, Clone, PartialEq)]
pub struct SocratesClusterSignal {
    pub cluster_key: String,
    pub sample_count: usize,
    pub abstain_rate: f64,
    pub mean_contradiction: f64,
    pub confidence_entropy: f64,
    pub severity_score: f64,
}

fn normalized_confidence_entropy(confidences: &[f64]) -> f64 {
    if confidences.is_empty() {
        return 0.0;
    }
    const BINS: usize = 5;
    let mut counts = [0usize; BINS];
    for &raw in confidences {
        let c = if raw.is_finite() {
            raw.clamp(0.0, 1.0)
        } else {
            0.5
        };
        let idx = ((c * BINS as f64).floor() as usize).min(BINS - 1);
        counts[idx] += 1;
    }
    let total = confidences.len() as f64;
    let mut entropy = 0.0_f64;
    for count in counts {
        if count > 0 {
            let p = count as f64 / total;
            entropy -= p * p.ln();
        }
    }
    let max_entropy = (BINS as f64).ln();
    if max_entropy <= 0.0 {
        0.0
    } else {
        (entropy / max_entropy).clamp(0.0, 1.0)
    }
}

/// Parse one `list_socrates_surface_events` metadata JSON row into a sample.
#[must_use]
pub fn parse_socrates_surface_sample(
    metric_value: Option<f64>,
    metadata_json: Option<&str>,
) -> Option<SocratesSurfaceSample> {
    let raw = metadata_json?.trim();
    if raw.is_empty() {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    let surface = v
        .get("surface")
        .and_then(|s| s.as_str())
        .unwrap_or("unknown")
        .to_string();
    let task_class = v
        .get("task_class")
        .and_then(|s| s.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let first_tag = v
        .get("domain_tags")
        .and_then(|t| t.as_array())
        .and_then(|arr| arr.first())
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let cluster_key = task_class
        .or(first_tag)
        .map(|k| format!("{surface}:{k}"))
        .unwrap_or_else(|| surface.clone());

    let risk_str = v
        .get("risk_decision")
        .and_then(|d| d.as_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let is_abstain = risk_str == "abstain" || risk_str == "ask" || risk_str == "disambiguate";
    let confidence_estimate = v
        .get("confidence_estimate")
        .and_then(|c| c.as_f64())
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    let contradiction_ratio = v
        .get("contradiction_ratio")
        .and_then(|c| c.as_f64())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let hallucination_risk_proxy = v
        .get("hallucination_risk_proxy")
        .and_then(|c| c.as_f64())
        .or(metric_value)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);

    Some(SocratesSurfaceSample {
        surface,
        cluster_key,
        is_abstain,
        confidence_estimate,
        contradiction_ratio,
        hallucination_risk_proxy,
    })
}

/// Group Socrates surface samples by cluster key and detect high-entropy
/// abstention and contradiction clusters.
#[must_use]
pub fn detect_socrates_anomaly_clusters(
    samples: &[SocratesSurfaceSample],
) -> Vec<SocratesClusterSignal> {
    let mut by_cluster: BTreeMap<&str, Vec<&SocratesSurfaceSample>> = BTreeMap::new();
    for s in samples {
        by_cluster
            .entry(s.cluster_key.as_str())
            .or_default()
            .push(s);
    }

    let mut signals = Vec::new();
    for (cluster_key, group) in by_cluster {
        if group.len() < MIN_CLUSTER_SAMPLES {
            continue;
        }
        let n = group.len() as f64;
        let abstain_count = group.iter().filter(|s| s.is_abstain).count();
        let abstain_rate = abstain_count as f64 / n;
        let mean_contradiction = group.iter().map(|s| s.contradiction_ratio).sum::<f64>() / n;
        let confs: Vec<f64> = group.iter().map(|s| s.confidence_estimate).collect();
        let confidence_entropy = normalized_confidence_entropy(&confs);

        let high_entropy_abstention = abstain_rate >= MIN_CLUSTER_ABSTAIN_RATE
            && confidence_entropy >= MIN_CLUSTER_CONFIDENCE_ENTROPY;
        let contradiction_spike = mean_contradiction >= MIN_CLUSTER_CONTRADICTION_RATE;

        if !high_entropy_abstention && !contradiction_spike {
            continue;
        }

        let severity_score =
            (0.45 * abstain_rate + 0.35 * mean_contradiction + 0.20 * confidence_entropy)
                .clamp(0.0, 1.0);
        signals.push(SocratesClusterSignal {
            cluster_key: cluster_key.to_string(),
            sample_count: group.len(),
            abstain_rate,
            mean_contradiction,
            confidence_entropy,
            severity_score,
        });
    }

    signals.sort_by(|a, b| {
        b.severity_score
            .partial_cmp(&a.severity_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.cluster_key.cmp(&b.cluster_key))
    });
    signals
}

pub struct SocratesTelemetryProducer {
    codex: vox_db::VoxDb,
}

impl SocratesTelemetryProducer {
    pub fn new(codex: vox_db::VoxDb) -> Self {
        Self { codex }
    }
}

#[async_trait]
impl Producer for SocratesTelemetryProducer {
    fn name(&self) -> &'static str {
        PRODUCER_NAME
    }

    async fn observe(&self, ctx: &ProducerContext) -> Vec<ResearchEvent> {
        let mut events = Vec::new();
        let slug = date_slug(ctx.now_ms);

        match list_trust_scores(&self.codex).await {
            Ok(scores) if scores.len() >= 2 * MIN_SAMPLES_PER_WINDOW => {
                let mid = scores.len() / 2;
                let prior = &scores[..mid];
                let trailing = &scores[mid..];
                if prior.len() >= MIN_SAMPLES_PER_WINDOW && trailing.len() >= MIN_SAMPLES_PER_WINDOW
                {
                    let prior_mean: f64 = prior.iter().sum::<f64>() / prior.len() as f64;
                    let trailing_mean: f64 = trailing.iter().sum::<f64>() / trailing.len() as f64;
                    let delta = trailing_mean - prior_mean;
                    if delta >= MIN_TRUST_IMPROVEMENT {
                        let mut payload = Vec::new();
                        payload.extend_from_slice(PRODUCER_NAME.as_bytes());
                        payload.extend_from_slice(b"::trust::");
                        payload.extend_from_slice(&scores.len().to_le_bytes());
                        let sha7: String = crate::compliance_hash_hex(&payload)
                            .chars()
                            .take(8)
                            .collect();
                        events.push(ResearchEvent::FindingCandidateProposed {
                            finding_id: format!("teltr-{slug}-trust-{sha7}"),
                            claim_ids: vec![],
                            worthiness_score: delta.clamp(0.0, 1.0),
                            session_id: ctx.session_id.clone(),
                            finding_candidate: None,
                        });
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                tracing::debug!(error = %e, "socrates_telemetry: trust_obs query failed");
            }
        }

        match self.codex.list_socrates_surface_events(None, 500).await {
            Ok(rows) => {
                let samples: Vec<SocratesSurfaceSample> = rows
                    .iter()
                    .filter_map(|(_sess, val, meta)| {
                        parse_socrates_surface_sample(*val, meta.as_deref())
                    })
                    .collect();
                for cluster in detect_socrates_anomaly_clusters(&samples)
                    .into_iter()
                    .take(3)
                {
                    let payload = format!(
                        "{PRODUCER_NAME}::socrates::{}::{}",
                        cluster.cluster_key, cluster.sample_count
                    );
                    let sha8: String = crate::compliance_hash_hex(payload.as_bytes())
                        .chars()
                        .take(8)
                        .collect();
                    let finding_id = format!("teltr-{slug}-socrates-{sha8}");
                    let candidate = FindingCandidateV1 {
                        schema_version: 1,
                        candidate_id: finding_id.clone(),
                        candidate_class: FindingCandidateClass::TelemetryTrust,
                        internal_signals: vec![DiscoverySignal {
                            code: "SOC-CLUSTER-01".to_string(),
                            summary: format!(
                                "Socrates cluster '{}' abstain_rate={:.2} contradiction={:.2} entropy={:.2} (n={})",
                                cluster.cluster_key,
                                cluster.abstain_rate,
                                cluster.mean_contradiction,
                                cluster.confidence_entropy,
                                cluster.sample_count
                            ),
                            strength: DiscoverySignalStrength::Strong,
                            family: DiscoverySignalFamily::TelemetryAggregate,
                            source_ref: Some(cluster.cluster_key.clone()),
                            provenance: SignalProvenance {
                                origin: Some("socrates_telemetry.rs".to_string()),
                                repo_path: None,
                                metric_type: Some("socrates_surface".to_string()),
                                run_id: Some(ctx.session_id.clone()),
                                recorded_at_ms: Some(ctx.now_ms),
                                digest: Some(sha8.clone()),
                            },
                        }],
                        created_at_ms: ctx.now_ms,
                        publication_id: None,
                        title_hint: Some(format!(
                            "High-entropy Socrates cluster: {}",
                            cluster.cluster_key
                        )),
                        novelty_evidence_bundle_id: None,
                        worthiness_decision_ref: None,
                        confidence: Some(FindingCandidateConfidence {
                            signal_strength: Some(cluster.severity_score),
                            contradiction_risk: Some(cluster.mean_contradiction),
                            reproducibility_support: Some(
                                (cluster.sample_count as f64 / 20.0).clamp(0.0, 1.0),
                            ),
                        }),
                        updated_at_ms: None,
                    };
                    events.push(ResearchEvent::FindingCandidateProposed {
                        finding_id,
                        claim_ids: vec![],
                        worthiness_score: cluster.severity_score,
                        session_id: ctx.session_id.clone(),
                        finding_candidate: serde_json::to_value(candidate).ok(),
                    });
                }
            }
            Err(e) => {
                tracing::debug!(error = %e, "socrates_telemetry: socrates_surface query failed");
            }
        }

        events
    }
}

async fn list_trust_scores(codex: &vox_db::VoxDb) -> Result<Vec<f64>, vox_db::StoreError> {
    let mut rows = codex
        .connection()
        .query(
            "SELECT trust_score FROM agent_telemetry_flat \
             WHERE event_kind = 'trust_obs' AND trust_score IS NOT NULL \
             ORDER BY recorded_at_ms ASC",
            (),
        )
        .await
        .map_err(vox_db::StoreError::Turso)?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await.map_err(vox_db::StoreError::Turso)? {
        let v: Option<f64> = r.get(0).map_err(vox_db::StoreError::Turso)?;
        if let Some(v) = v {
            out.push(v);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(
        cluster_key: &str,
        is_abstain: bool,
        confidence: f64,
        contradiction: f64,
    ) -> SocratesSurfaceSample {
        SocratesSurfaceSample {
            surface: "vox_chat_message".into(),
            cluster_key: cluster_key.into(),
            is_abstain,
            confidence_estimate: confidence,
            contradiction_ratio: contradiction,
            hallucination_risk_proxy: if is_abstain { 0.9 } else { 0.1 },
        }
    }

    #[test]
    fn detects_high_entropy_abstention_and_contradiction_clusters() {
        let mut samples = vec![
            sample("vox_chat:codegen", true, 0.05, 0.10),
            sample("vox_chat:codegen", true, 0.25, 0.15),
            sample("vox_chat:codegen", false, 0.45, 0.05),
            sample("vox_chat:codegen", true, 0.65, 0.20),
            sample("vox_chat:codegen", false, 0.92, 0.10),
            // Contradiction spike cluster even with low abstention
            sample("vox_chat:research", false, 0.85, 0.45),
            sample("vox_chat:research", false, 0.86, 0.40),
            sample("vox_chat:research", false, 0.84, 0.35),
            sample("vox_chat:research", false, 0.88, 0.38),
            sample("vox_chat:research", false, 0.82, 0.32),
        ];
        // Calm cluster should not fire
        for _ in 0..6 {
            samples.push(sample("vox_chat:steady", false, 0.95, 0.02));
        }

        let detected = detect_socrates_anomaly_clusters(&samples);
        let keys: Vec<&str> = detected.iter().map(|c| c.cluster_key.as_str()).collect();
        assert!(keys.contains(&"vox_chat:codegen"));
        assert!(keys.contains(&"vox_chat:research"));
        assert!(!keys.contains(&"vox_chat:steady"));
    }

    #[test]
    fn parses_socrates_surface_metadata_json() {
        let meta = serde_json::json!({
            "surface": "vox_chat_message",
            "repository_id": "repo-1",
            "risk_decision": "Abstain",
            "confidence_estimate": 0.41,
            "contradiction_ratio": 0.38,
            "hallucination_risk_proxy": 0.95,
            "task_class": "scholarly_claim"
        })
        .to_string();
        let parsed = parse_socrates_surface_sample(Some(0.95), Some(&meta)).expect("parsed");
        assert_eq!(parsed.cluster_key, "vox_chat_message:scholarly_claim");
        assert!(parsed.is_abstain);
        assert!((parsed.contradiction_ratio - 0.38).abs() < 1e-9);
    }
}
