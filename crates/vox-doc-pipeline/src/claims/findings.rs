//! Docs reality audit finding generation, deduplication, and metrics computation.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

use super::types::ResolvedMention;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingScores {
    pub impact: i32,
    pub blast_radius: i32,
    pub staleness: i32,
    pub enforcement_gap: i32,
    pub tractability: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealityAuditFinding {
    pub id: String,
    pub claim_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub doc_paths: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub code_paths: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub contract_paths: Vec<String>,
    pub claim_type: String,
    pub classification: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub secondary_tags: Vec<String>,
    pub scores: FindingScores,
    pub priority_score: i32,
    pub priority_band: String,
    pub recommended_action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_candidate: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detector: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FindingsFile {
    pub schema_version: u32,
    pub findings: Vec<RealityAuditFinding>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsFile {
    pub schema_version: u32,
    pub generated_at: String,
    pub inventory_claim_count: usize,
    pub findings_total: usize,
    pub findings_open: usize,
    pub findings_closed: usize,
    pub counts_by_classification: HashMap<String, usize>,
    pub counts_by_status: HashMap<String, usize>,
    pub counts_by_priority_band: HashMap<String, usize>,
    pub open_p0: usize,
    pub open_p1: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corpus_pages_scanned: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_mentions_extracted: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_mentions_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dead_mentions_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_status: Option<String>,
}

/// Compute a deterministic 64-character SHA-256 fingerprint for a mention.
pub fn compute_fingerprint(mention: &ResolvedMention) -> String {
    let mut hasher = Sha256::new();
    let key = format!(
        "{}:{}:{:?}:{}",
        mention.extracted.source_file,
        mention.extracted.raw_text,
        mention.extracted.kind,
        mention.extracted.line_number
    );
    hasher.update(key.as_bytes());
    let hash = hasher.finalize();
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Convert actionable deficits into deduplicated findings.
pub fn generate_findings(resolved: &[ResolvedMention]) -> Vec<RealityAuditFinding> {
    let mut findings = Vec::new();
    let mut seen_fps = HashSet::new();
    let mut id_counter = 1;

    for m in resolved {
        if !m.is_actionable_deficit() {
            continue;
        }

        let fp = compute_fingerprint(m);
        if !seen_fps.insert(fp.clone()) {
            continue;
        }

        let detector = format!("mention_extractor::{:?}", m.extracted.kind).to_lowercase();
        let recommended_action = if let Some(ref repl) = m.replacement {
            format!(
                "Replace retired mention '{}' with canonical replacement '{}'",
                m.extracted.raw_text, repl
            )
        } else {
            format!(
                "Update or remove missing mention '{}' in {}",
                m.extracted.raw_text, m.extracted.source_file
            )
        };

        let scores = FindingScores {
            impact: 2,
            blast_radius: 2,
            staleness: 2,
            enforcement_gap: 3,
            tractability: 3,
        };
        let priority_score = scores.impact * 2
            + scores.blast_radius * 2
            + scores.staleness
            + scores.enforcement_gap
            + scores.tractability;
        let priority_band = if priority_score >= 22 {
            "P0".to_string()
        } else if priority_score >= 14 {
            "P1".to_string()
        } else {
            "P2".to_string()
        };

        let claim_id = format!("claim-mention-{:04}", id_counter);
        let mut claim_ids = vec![m.extracted.raw_text.clone(), claim_id];
        if !m.extracted.normalized_target.is_empty()
            && m.extracted.normalized_target != m.extracted.raw_text
        {
            claim_ids.push(m.extracted.normalized_target.clone());
        }

        findings.push(RealityAuditFinding {
            id: format!("doc-deficit-{:04}", id_counter),
            claim_ids,
            doc_paths: vec![m.extracted.source_file.clone()],
            code_paths: vec![],
            contract_paths: vec![],
            claim_type: "normative_requirement".to_string(),
            classification: "DocDeficit".to_string(),
            secondary_tags: vec![format!("{:?}", m.extracted.kind).to_lowercase()],
            scores,
            priority_score,
            priority_band,
            recommended_action,
            owner_candidate: Some("docs-team".to_string()),
            status: "new".to_string(),
            notes: m.rationale.clone(),
            source: Some("mention_extractor".to_string()),
            fingerprint: Some(fp),
            detector: Some(detector),
        });

        id_counter += 1;
    }

    findings
}

/// Compute metrics file contents from findings and corpus scan stats.
pub fn compute_metrics(
    findings: &[RealityAuditFinding],
    corpus_pages_scanned: usize,
    total_mentions: usize,
    resolved_count: usize,
) -> MetricsFile {
    let mut counts_by_classification = HashMap::new();
    let mut counts_by_status = HashMap::new();
    let mut counts_by_priority_band = HashMap::new();
    let mut open_p0 = 0;
    let mut open_p1 = 0;

    for f in findings {
        *counts_by_classification
            .entry(f.classification.clone())
            .or_insert(0) += 1;
        *counts_by_status.entry(f.status.clone()).or_insert(0) += 1;
        *counts_by_priority_band
            .entry(f.priority_band.clone())
            .or_insert(0) += 1;

        if f.status != "closed" {
            if f.priority_band == "P0" {
                open_p0 += 1;
            } else if f.priority_band == "P1" {
                open_p1 += 1;
            }
        }
    }

    let health_status = if corpus_pages_scanned == 0 {
        "unpopulated".to_string()
    } else if findings.is_empty() {
        "healthy".to_string()
    } else {
        "deficits_found".to_string()
    };

    MetricsFile {
        schema_version: 1,
        generated_at: "2026-10-13T00:00:00Z".to_string(),
        inventory_claim_count: findings.len(),
        findings_total: findings.len(),
        findings_open: findings.iter().filter(|f| f.status != "closed").count(),
        findings_closed: findings.iter().filter(|f| f.status == "closed").count(),
        counts_by_classification,
        counts_by_status,
        counts_by_priority_band,
        open_p0,
        open_p1,
        corpus_pages_scanned: Some(corpus_pages_scanned),
        total_mentions_extracted: Some(total_mentions),
        resolved_mentions_count: Some(resolved_count),
        dead_mentions_count: Some(findings.len()),
        health_status: Some(health_status),
    }
}
