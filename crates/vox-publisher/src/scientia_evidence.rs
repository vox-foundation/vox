//! Optional `metadata_json.scientia_evidence` — ties publication manifests to Socrates, eval-gate, and benchmark artifacts.

use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::publication_worthiness::WorthinessInputs;
use crate::scientific_metadata::ScientificPublicationMetadata;

/// Root [`PublicationManifest::metadata_json`] key for this block.
pub const METADATA_KEY_SCIENTIA_EVIDENCE: &str = "scientia_evidence";

/// Inline snapshot compatible with [`vox_db::socrates_telemetry::SocratesSurfaceAggregate`] JSON.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct SocratesAggregateSnapshot {
    #[serde(default)]
    pub sample_size: usize,
    #[serde(default)]
    pub parsed_metadata_rows: usize,
    #[serde(default)]
    pub mean_hallucination_risk_proxy: f64,
    #[serde(default)]
    pub mean_confidence_estimate: f64,
    #[serde(default)]
    pub mean_contradiction_ratio: f64,
    #[serde(default)]
    pub answer_count: usize,
    #[serde(default)]
    pub ask_count: usize,
    #[serde(default)]
    pub abstain_count: usize,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct EvalGateSnapshot {
    pub passed: bool,
    #[serde(default)]
    pub gates_failed: usize,
    #[serde(default)]
    pub gates_total: usize,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct BenchmarkPairSnapshot {
    #[serde(default)]
    pub baseline_run_id: Option<String>,
    #[serde(default)]
    pub candidate_run_id: Option<String>,
    /// Repo-relative path to a benchmark manifest or run envelope (informational).
    #[serde(default)]
    pub manifest_repo_relative: Option<String>,
    #[serde(default)]
    pub pair_complete: bool,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiscoverySignalStrength {
    #[default]
    Supporting,
    Strong,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct DiscoverySignal {
    pub code: String,
    pub summary: String,
    #[serde(default)]
    pub strength: DiscoverySignalStrength,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct DraftPreparationHints {
    #[serde(default)]
    pub abstract_needed: bool,
    #[serde(default)]
    pub citations_needed: bool,
    #[serde(default)]
    pub reproducibility_details_needed: bool,
    #[serde(default)]
    pub ethics_statement_needed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_scholarly_venue: Option<String>,
}

/// Evidence bundle authors embed (or tools merge from live DB reads).
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct ScientiaEvidenceContext {
    #[serde(default)]
    pub socrates_aggregate: Option<SocratesAggregateSnapshot>,
    #[serde(default)]
    pub eval_gate: Option<EvalGateSnapshot>,
    #[serde(default)]
    pub benchmark: Option<BenchmarkPairSnapshot>,
    /// Machine-inferred reasons this draft may be worth preparing for publication.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discovery_signals: Vec<DiscoverySignal>,
    /// Preparation work the system can tee up before a human writes the full paper/package.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_preparation: Option<DraftPreparationHints>,
    /// Short machine-generated note summarizing why this draft was surfaced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_note: Option<String>,
    /// Repo-relative directory passed to `vox mens eval-gate` / `check_run` (CLI only when `with_worthiness`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eval_gate_run_dir_repo_relative: Option<String>,
    /// Repo-relative JSON file with an [`EvalGateSnapshot`] (CLI + MCP); applied when `eval_gate` is absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eval_gate_report_repo_relative: Option<String>,
    /// Repo-relative JSON file with a [`BenchmarkPairSnapshot`]; applied when `benchmark` is absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub benchmark_pair_report_repo_relative: Option<String>,
    /// Human attestation that there is a substantive novel result (never inferred from heuristics alone).
    #[serde(default)]
    pub human_meaningful_advance: bool,
    /// Human attestation that AI / generative-tool disclosure meets target-venue policy (sets compliance to 1.0 when true).
    #[serde(default)]
    pub human_ai_disclosure_complete: bool,
}

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

/// Read `scientia_evidence` from manifest metadata (best-effort).
#[must_use]
pub fn parse_scientia_evidence(metadata_json: Option<&str>) -> Option<ScientiaEvidenceContext> {
    let raw = metadata_json?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let root: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    let v = root.get(METADATA_KEY_SCIENTIA_EVIDENCE)?;
    serde_json::from_value(v.clone()).ok()
}

/// Best-effort markdown title inference: frontmatter `title:` first, then first `# Heading`, else `Untitled`.
#[must_use]
pub fn infer_markdown_title(content: &str) -> String {
    let content = content.trim();
    if let Some(rest) = content.strip_prefix("---") {
        let rest = rest.strip_prefix('\n').unwrap_or(rest);
        if let Some(idx) = rest.find("\n---") {
            let fm = &rest[..idx];
            for line in fm.lines() {
                if let Some(val) = line.trim().strip_prefix("title:") {
                    let inferred = val.trim().trim_matches('"').trim_matches('\'').trim();
                    if !inferred.is_empty() {
                        return inferred.to_string();
                    }
                }
            }
        }
    }
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            let inferred = heading.trim();
            if !inferred.is_empty() {
                return inferred.to_string();
            }
        }
    }
    "Untitled".to_string()
}

fn push_signal_unique(out: &mut Vec<DiscoverySignal>, signal: DiscoverySignal) {
    if out.iter().any(|existing| existing.code == signal.code) {
        return;
    }
    out.push(signal);
}

/// Infer a first-pass discovery signal set from structured evidence and source location.
#[must_use]
pub fn infer_discovery_signals(
    source_ref: Option<&str>,
    evidence: &ScientiaEvidenceContext,
) -> Vec<DiscoverySignal> {
    let mut out = Vec::new();
    let source_ref = source_ref.map(str::trim).filter(|s| !s.is_empty());
    if let Some(src) = source_ref {
        let normalized = src.replace('\\', "/").to_ascii_lowercase();
        if normalized.starts_with("docs/src/adr/") || normalized.contains("/docs/src/adr/") {
            push_signal_unique(
                &mut out,
                DiscoverySignal {
                    code: "adr_writeup_present".to_string(),
                    summary: "An ADR-style writeup already exists for this candidate.".to_string(),
                    strength: DiscoverySignalStrength::Supporting,
                    source_ref: Some(src.to_string()),
                },
            );
        }
        if normalized.starts_with("docs/src/research/")
            || normalized.contains("/docs/src/research/")
            || normalized.starts_with("docs/src/architecture/")
            || normalized.contains("/docs/src/architecture/")
            || normalized.starts_with("docs/src/reference/")
            || normalized.contains("/docs/src/reference/")
        {
            push_signal_unique(
                &mut out,
                DiscoverySignal {
                    code: "research_writeup_present".to_string(),
                    summary:
                        "A repository writeup already exists and can seed the publication draft."
                            .to_string(),
                    strength: DiscoverySignalStrength::Supporting,
                    source_ref: Some(src.to_string()),
                },
            );
        }
    }
    if evidence.eval_gate.as_ref().is_some_and(|g| g.passed) {
        push_signal_unique(
            &mut out,
            DiscoverySignal {
                code: "eval_gate_passed".to_string(),
                summary: "An attached eval-gate snapshot passed.".to_string(),
                strength: DiscoverySignalStrength::Strong,
                source_ref: evidence.eval_gate_report_repo_relative.clone(),
            },
        );
    }
    if evidence.benchmark.as_ref().is_some_and(|b| {
        b.pair_complete
            && b.baseline_run_id
                .as_ref()
                .is_some_and(|s| !s.trim().is_empty())
            && b.candidate_run_id
                .as_ref()
                .is_some_and(|s| !s.trim().is_empty())
    }) {
        push_signal_unique(
            &mut out,
            DiscoverySignal {
                code: "benchmark_pair_complete".to_string(),
                summary: "A baseline/candidate benchmark pair is attached and marked complete."
                    .to_string(),
                strength: DiscoverySignalStrength::Strong,
                source_ref: evidence.benchmark_pair_report_repo_relative.clone(),
            },
        );
    }
    if evidence.human_meaningful_advance {
        push_signal_unique(
            &mut out,
            DiscoverySignal {
                code: "operator_marked_meaningful_advance".to_string(),
                summary: "A human explicitly marked this work as a meaningful advance candidate."
                    .to_string(),
                strength: DiscoverySignalStrength::Strong,
                source_ref: None,
            },
        );
    }
    out
}

/// Infer factual preparation gaps the system can tee up before human drafting.
#[must_use]
pub fn infer_draft_preparation_hints(
    abstract_text: Option<&str>,
    citations_json: Option<&str>,
    scientific: Option<&ScientificPublicationMetadata>,
    evidence: &ScientiaEvidenceContext,
) -> DraftPreparationHints {
    let reproducibility_present = scientific
        .and_then(|s| s.reproducibility.as_ref())
        .is_some_and(|r| {
            r.code_repository_url
                .as_ref()
                .is_some_and(|s| !s.trim().is_empty())
                || r.data_repository_url
                    .as_ref()
                    .is_some_and(|s| !s.trim().is_empty())
                || r.artifact_checksum_note
                    .as_ref()
                    .is_some_and(|s| !s.trim().is_empty())
        });
    let ethics_present = scientific
        .and_then(|s| s.ethics_and_impact.as_ref())
        .is_some_and(|e| {
            e.broader_impact_statement
                .as_ref()
                .is_some_and(|s| !s.trim().is_empty())
                || e.irb_or_human_subjects_note
                    .as_ref()
                    .is_some_and(|s| !s.trim().is_empty())
        });
    let has_strong_signal = infer_discovery_signals(None, evidence)
        .iter()
        .any(|s| s.strength == DiscoverySignalStrength::Strong);
    DraftPreparationHints {
        abstract_needed: abstract_text.is_none_or(|s| s.trim().is_empty()),
        citations_needed: citations_json.is_none_or(|s| s.trim().is_empty()),
        reproducibility_details_needed: !reproducibility_present
            && (evidence.eval_gate.is_some() || evidence.benchmark.is_some()),
        ethics_statement_needed: !ethics_present && has_strong_signal,
        recommended_scholarly_venue: if has_strong_signal {
            Some("arxiv_assist".to_string())
        } else {
            None
        },
    }
}

/// Fill missing discovery signals / preparation hints / candidate note using current evidence and source context.
pub fn populate_candidate_context_defaults(
    source_ref: Option<&str>,
    abstract_text: Option<&str>,
    citations_json: Option<&str>,
    scientific: Option<&ScientificPublicationMetadata>,
    evidence: &mut ScientiaEvidenceContext,
) {
    if evidence.discovery_signals.is_empty() {
        evidence.discovery_signals = infer_discovery_signals(source_ref, evidence);
    }
    if evidence.draft_preparation.is_none() {
        evidence.draft_preparation = Some(infer_draft_preparation_hints(
            abstract_text,
            citations_json,
            scientific,
            evidence,
        ));
    }
    if evidence.candidate_note.is_none() && !evidence.discovery_signals.is_empty() {
        let summary = evidence
            .discovery_signals
            .iter()
            .take(3)
            .map(|s| s.summary.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        evidence.candidate_note = Some(format!(
            "Draft candidate surfaced from structured signals. {summary}"
        ));
    }
}

fn trim_repo_rel_path(s: &str) -> String {
    s.trim()
        .trim_start_matches('/')
        .trim_start_matches('\\')
        .to_string()
}

/// Read sidecar JSON files referenced under `scientia_evidence` and merge into `metadata_json` (no `check_run`).
///
/// Runs after live Socrates merge and (in the CLI) after eval-gate directory checks when those set `eval_gate`.
/// Fills `eval_gate` / `benchmark` only when the corresponding snapshot field is still `None`.
pub fn enrich_metadata_json_with_repo_files(
    metadata_json: Option<&str>,
    repo_root: &Path,
) -> anyhow::Result<Option<String>> {
    let Some(raw) = metadata_json else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let mut root: serde_json::Value = serde_json::from_str(trimmed)
        .with_context(|| "parse metadata_json for scientia_evidence file hydration")?;

    let mut ev: ScientiaEvidenceContext = root
        .get(METADATA_KEY_SCIENTIA_EVIDENCE)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let mut changed = false;

    if ev.eval_gate.is_none()
        && let Some(ref rel) = ev.eval_gate_report_repo_relative.clone()
    {
        let part = trim_repo_rel_path(rel);
        if !part.is_empty() {
            let p = repo_root.join(&part);
            if p.is_file() {
                let txt = crate::bounded_fs::read_utf8_path_capped(&p)
                    .with_context(|| format!("read eval_gate report {}", p.display()))?;
                let g: EvalGateSnapshot = serde_json::from_str(&txt)
                    .with_context(|| format!("parse eval_gate report JSON {}", p.display()))?;
                ev.eval_gate = Some(g);
                changed = true;
            }
        }
    }

    if ev.benchmark.is_none()
        && let Some(ref rel) = ev.benchmark_pair_report_repo_relative.clone()
    {
        let part = trim_repo_rel_path(rel);
        if !part.is_empty() {
            let p = repo_root.join(&part);
            if p.is_file() {
                let txt = crate::bounded_fs::read_utf8_path_capped(&p)
                    .with_context(|| format!("read benchmark pair report {}", p.display()))?;
                let b: BenchmarkPairSnapshot = serde_json::from_str(&txt)
                    .with_context(|| format!("parse benchmark pair report JSON {}", p.display()))?;
                ev.benchmark = Some(b);
                changed = true;
            }
        }
    }

    if !changed {
        return Ok(None);
    }

    root[METADATA_KEY_SCIENTIA_EVIDENCE] = serde_json::to_value(&ev)?;
    Ok(Some(serde_json::to_string(&root)?))
}

fn merge_socrates_aggregate(inputs: &mut WorthinessInputs, agg: &SocratesAggregateSnapshot) {
    if agg.parsed_metadata_rows == 0 && agg.sample_size == 0 {
        return;
    }
    let conf = agg.mean_confidence_estimate.clamp(0.0, 1.0);
    let risk = agg.mean_hallucination_risk_proxy.clamp(0.0, 1.0);
    let cr = agg.mean_contradiction_ratio.clamp(0.0, 1.0);

    let epistemic_signal = conf * (1.0 - 0.88 * risk);
    inputs.epistemic = clamp01(0.4 * inputs.epistemic + 0.6 * epistemic_signal);
    inputs.epistemic = clamp01(inputs.epistemic * (1.0 - 0.45 * cr));

    let reliability_signal = (1.0 - risk).max(0.0);
    inputs.reliability = clamp01(0.35 * inputs.reliability + 0.65 * reliability_signal);

    if agg.abstain_count >= 2 && cr > 0.2 {
        inputs.repeated_unresolved_contradiction = true;
    }
    if cr > 0.38
        && !inputs
            .red_line_violation_ids
            .iter()
            .any(|x| x == "unresolved_socrates_contradiction")
    {
        inputs
            .red_line_violation_ids
            .push("unresolved_socrates_contradiction".to_string());
    }
}

/// Fold structured evidence into heuristic [`WorthinessInputs`] (order: heuristic base first, then evidence).
#[must_use]
pub fn apply_scientia_evidence(
    mut inputs: WorthinessInputs,
    evidence: &ScientiaEvidenceContext,
) -> WorthinessInputs {
    if let Some(ref agg) = evidence.socrates_aggregate {
        merge_socrates_aggregate(&mut inputs, agg);
    }

    if let Some(ref g) = evidence.eval_gate {
        if g.passed {
            inputs.reliability = clamp01(inputs.reliability.max(0.9));
            inputs.metadata_policy = clamp01(inputs.metadata_policy.max(0.87));
        } else {
            inputs.reliability = clamp01(inputs.reliability * 0.68);
        }
    }

    if let Some(ref b) = evidence.benchmark
        && b.pair_complete
        && b.baseline_run_id
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty())
        && b.candidate_run_id
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty())
    {
        inputs.before_after_pair_integrity = clamp01(inputs.before_after_pair_integrity.max(0.9));
    }

    if evidence.human_ai_disclosure_complete {
        inputs.ai_disclosure_compliance = 1.0;
    }

    if evidence.human_meaningful_advance {
        let socrates_ok = evidence.socrates_aggregate.as_ref().is_some_and(|a| {
            a.parsed_metadata_rows > 0
                && a.mean_hallucination_risk_proxy < 0.36
                && a.mean_contradiction_ratio < 0.26
        });
        let gate_ok = evidence.eval_gate.as_ref().is_none_or(|g| g.passed);
        let bench_ok = evidence.benchmark.as_ref().is_none_or(|b| {
            b.pair_complete
                && b.baseline_run_id
                    .as_ref()
                    .is_some_and(|s| !s.trim().is_empty())
                && b.candidate_run_id
                    .as_ref()
                    .is_some_and(|s| !s.trim().is_empty())
        });
        if socrates_ok && gate_ok && bench_ok {
            inputs.meaningful_advance = true;
        }
    }

    inputs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_bumps_epistemic_when_socrates_clean() {
        let evidence = ScientiaEvidenceContext {
            socrates_aggregate: Some(SocratesAggregateSnapshot {
                sample_size: 12,
                parsed_metadata_rows: 10,
                mean_hallucination_risk_proxy: 0.08,
                mean_confidence_estimate: 0.82,
                mean_contradiction_ratio: 0.05,
                answer_count: 8,
                ask_count: 2,
                abstain_count: 0,
            }),
            eval_gate: Some(EvalGateSnapshot {
                passed: true,
                gates_failed: 0,
                gates_total: 3,
            }),
            benchmark: Some(BenchmarkPairSnapshot {
                baseline_run_id: Some("b1".into()),
                candidate_run_id: Some("c1".into()),
                manifest_repo_relative: Some("contracts/eval/benchmark-matrix.json".into()),
                pair_complete: true,
            }),
            discovery_signals: Vec::new(),
            draft_preparation: None,
            candidate_note: None,
            eval_gate_run_dir_repo_relative: None,
            eval_gate_report_repo_relative: None,
            benchmark_pair_report_repo_relative: None,
            human_meaningful_advance: true,
            human_ai_disclosure_complete: true,
        };
        let base = WorthinessInputs {
            red_line_violation_ids: vec![],
            repeated_unresolved_contradiction: false,
            claim_evidence_coverage: 0.92,
            artifact_replayability: 0.88,
            before_after_pair_integrity: 0.5,
            metadata_completeness: 0.9,
            ai_disclosure_compliance: 0.85,
            epistemic: 0.55,
            reproducibility: 0.7,
            novelty: 0.6,
            reliability: 0.6,
            metadata_policy: 0.75,
            meaningful_advance: false,
        };
        let merged = apply_scientia_evidence(base, &evidence);
        assert!(merged.meaningful_advance);
        assert_eq!(merged.ai_disclosure_compliance, 1.0);
        assert!(merged.epistemic > 0.65);
        assert!(merged.before_after_pair_integrity >= 0.88);
    }

    #[test]
    fn file_hydration_inlines_eval_gate_from_repo_relative_path() {
        let dir = tempfile::tempdir().unwrap();
        let report_path = dir.path().join("reports/eval_gate.json");
        std::fs::create_dir_all(report_path.parent().unwrap()).unwrap();
        std::fs::write(
            &report_path,
            r#"{"passed":true,"gates_failed":0,"gates_total":4}"#,
        )
        .unwrap();
        let meta = r#"{"repository_id":"r1","scientia_evidence":{"eval_gate_report_repo_relative":"reports/eval_gate.json"}}"#;
        let out = enrich_metadata_json_with_repo_files(Some(meta), dir.path())
            .unwrap()
            .unwrap();
        let ev = parse_scientia_evidence(Some(&out)).expect("evidence");
        let g = ev.eval_gate.as_ref().unwrap();
        assert!(g.passed);
        assert_eq!(g.gates_total, 4);
    }

    #[test]
    fn file_hydration_skips_when_sidecar_missing() {
        let dir = tempfile::tempdir().unwrap();
        let meta =
            r#"{"scientia_evidence":{"eval_gate_report_repo_relative":"nope/missing.json"}}"#;
        assert!(
            enrich_metadata_json_with_repo_files(Some(meta), dir.path())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn candidate_context_defaults_capture_discovery_signals_and_prep() {
        let scientific = ScientificPublicationMetadata::default();
        let mut evidence = ScientiaEvidenceContext {
            eval_gate: Some(EvalGateSnapshot {
                passed: true,
                gates_failed: 0,
                gates_total: 5,
            }),
            benchmark: Some(BenchmarkPairSnapshot {
                baseline_run_id: Some("baseline-1".into()),
                candidate_run_id: Some("candidate-1".into()),
                manifest_repo_relative: Some("reports/bench.json".into()),
                pair_complete: true,
            }),
            human_meaningful_advance: true,
            ..Default::default()
        };
        populate_candidate_context_defaults(
            Some("docs/src/adr/013-openclaw-ws-native-strategy.md"),
            None,
            None,
            Some(&scientific),
            &mut evidence,
        );
        assert!(
            evidence
                .discovery_signals
                .iter()
                .any(|s| s.code == "eval_gate_passed"
                    && s.strength == DiscoverySignalStrength::Strong)
        );
        assert!(
            evidence
                .discovery_signals
                .iter()
                .any(|s| s.code == "adr_writeup_present")
        );
        let prep = evidence.draft_preparation.as_ref().expect("draft prep");
        assert!(prep.abstract_needed);
        assert!(prep.citations_needed);
        assert!(prep.reproducibility_details_needed);
        assert_eq!(
            prep.recommended_scholarly_venue.as_deref(),
            Some("arxiv_assist")
        );
        assert!(
            evidence
                .candidate_note
                .as_ref()
                .is_some_and(|n| n.contains("structured signals"))
        );
    }
}
