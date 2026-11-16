//! Atomic-NEI novelty scoring over `NoveltyEvidenceBundle`.
//!
//! Fuses Stage-1 lexical overlap (`max_lexical_score`, `NormalizedHit.lexical_score`,
//! and shingle/MinHash text similarity) with Stage-2 semantic overlap (`max_semantic_score`),
//! while respecting `ChronoFilter` recency weighting and `EvidenceConflict` signals.

use crate::inspect_bridge::chronofact::ChronoFilter;
use crate::inspect_bridge::conflict::{ClaimPolarity, EvidenceConflictDetector, PolarizedHit};
use crate::inspect_bridge::lexical_shingles::{calibrated_lexical_overlap, text_overlap_score};
use vox_research_events::schema_types::{
    NormalizedHit, NoveltyEvidenceBundle, QueryTrace, RecencyBucket,
};

/// Whether a single prior-art query trace represents a successful retrieval.
///
/// A trace counts as successful if it was served from cache, or if it carries
/// an HTTP status in the 2xx success range. A trace with no status and no cache
/// hit is treated as a failure (we have no evidence the source answered).
fn trace_succeeded(trace: &QueryTrace) -> bool {
    if trace.cached == Some(true) {
        return true;
    }
    matches!(trace.http_status, Some(s) if (200..300).contains(&s))
}

/// The result of an atomic-NEI novelty assessment.
#[derive(Debug, Clone, PartialEq)]
pub enum NoveltyVerdict {
    /// No high-similarity prior art found (max_semantic_score < novel_threshold).
    Novel,
    /// Borderline: similarity above the novel threshold but below the not-novel threshold.
    PossiblyNovel { closest_score: f64 },
    /// Clear prior art found: similarity >= not_novel_threshold.
    NotNovel {
        closest_hit_uri: String,
        similarity: f64,
    },
    /// Prior-art retrieval itself failed (every queried source returned a
    /// non-success response), so no novelty determination can be made. This is
    /// distinct from `Novel`: a successful-but-empty search means "no prior art
    /// found"; a failed search means "we don't know".
    InsufficientEvidence { reason: String },
    /// The candidate claim directly conflicts with established prior art.
    /// Emitted by the contradiction detector (B5); defined here so the verdict
    /// surface is complete.
    Contradicted { conflicting_uri: String },
}

/// Thresholds controlling the novelty classification boundaries.
pub struct NoveltyConfig {
    /// Max semantic score below which the claim is considered Novel (default 0.5).
    pub novel_threshold: f64,
    /// Max semantic score at or above which the claim is NotNovel (default 0.8).
    pub not_novel_threshold: f64,
}

impl Default for NoveltyConfig {
    fn default() -> Self {
        Self {
            novel_threshold: 0.5,
            not_novel_threshold: 0.8,
        }
    }
}

/// Scores a `NoveltyEvidenceBundle` and returns a `NoveltyVerdict`.
#[derive(Default)]
pub struct AtomicNoveltyScorer {
    pub config: NoveltyConfig,
}

fn derive_hit_polarity(hit: &NormalizedHit) -> ClaimPolarity {
    let note = hit
        .overlap_note
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    if note.contains("contradict") || note.contains("dispute") || note.contains("refute") {
        ClaimPolarity::Negative
    } else {
        ClaimPolarity::Positive
    }
}

fn recency_weight(
    bucket: Option<RecencyBucket>,
    raw_overlap: f64,
    not_novel_threshold: f64,
) -> f64 {
    match bucket {
        // Keep >= not_novel_threshold hits unattenuated so established prior art is never cleared.
        Some(RecencyBucket::Stale) if raw_overlap < not_novel_threshold => 0.95,
        _ => 1.0,
    }
}

impl AtomicNoveltyScorer {
    pub fn new(config: NoveltyConfig) -> Self {
        Self { config }
    }

    /// Score a bundle using fused lexical + semantic overlap, recency weighting,
    /// and contradiction detection.
    pub fn score(&self, bundle: &NoveltyEvidenceBundle) -> NoveltyVerdict {
        self.score_with_context(bundle, None, None)
    }

    /// Score a bundle with optional candidate text (for direct 3-gram shingle /
    /// MinHash title overlap) and optional `ChronoFilter` temporal cutoff.
    pub fn score_with_context(
        &self,
        bundle: &NoveltyEvidenceBundle,
        candidate_text: Option<&str>,
        chrono_filter: Option<&ChronoFilter>,
    ) -> NoveltyVerdict {
        // Retrieval gate (must run BEFORE the similarity ladder): if we have
        // per-source query traces and *every* source failed, prior-art
        // retrieval did not succeed — we cannot conclude novelty.
        if let Some(traces) = bundle.query_traces.as_ref()
            && !traces.is_empty()
            && traces.iter().all(|t| !trace_succeeded(t))
        {
            return NoveltyVerdict::InsufficientEvidence {
                reason: "no prior-art source returned a successful response".into(),
            };
        }

        let hits: Vec<&NormalizedHit> = match chrono_filter {
            Some(cf) => cf.filter_hits(&bundle.normalized_hits),
            None => bundle.normalized_hits.iter().collect(),
        };

        let cand_clean = candidate_text.map(str::trim).filter(|s| !s.is_empty());

        let mut per_hit_scores: Vec<(&NormalizedHit, Option<f64>)> = Vec::with_capacity(hits.len());
        let mut hit_max_lexical: Option<f64> = None;
        let mut hit_max_semantic: Option<f64> = None;

        for hit in &hits {
            let text_sim = cand_clean
                .filter(|_| !hit.title.trim().is_empty())
                .map(|cand| text_overlap_score(cand, &hit.title));
            let lex_opt = if hit.lexical_score.is_some() || text_sim.is_some() {
                let lex = calibrated_lexical_overlap(hit.lexical_score, text_sim);
                hit_max_lexical = Some(hit_max_lexical.map_or(lex, |prev| prev.max(lex)));
                Some(lex)
            } else {
                None
            };
            let sem_opt = hit
                .semantic_score
                .filter(|v| v.is_finite())
                .map(|v| v.clamp(0.0, 1.0));
            if let Some(sem) = sem_opt {
                hit_max_semantic = Some(hit_max_semantic.map_or(sem, |prev| prev.max(sem)));
            }
            let hit_fused = match (sem_opt, lex_opt) {
                (Some(s), Some(l)) => Some(s.max(l)),
                (Some(s), None) => Some(s),
                (None, Some(l)) => Some(l),
                (None, None) => None,
            };
            per_hit_scores.push((hit, hit_fused));
        }

        // Derive summary lexical/semantic when chrono_filter did not strip all hits;
        // if chrono_filter removed all hits, do not leak unfiltered summary scores.
        let summary_allowed = chrono_filter.is_none() || !hits.is_empty();
        let summary_semantic = if summary_allowed {
            bundle
                .overlap_summary
                .as_ref()
                .and_then(|s| s.max_semantic_score)
                .filter(|v| v.is_finite())
                .map(|v| v.clamp(0.0, 1.0))
        } else {
            None
        };
        let summary_lexical = if summary_allowed {
            bundle
                .overlap_summary
                .as_ref()
                .and_then(|s| s.max_lexical_score)
                .filter(|v| v.is_finite())
                .map(|v| v.clamp(0.0, 1.0))
        } else {
            None
        };

        let max_semantic = summary_semantic.or(hit_max_semantic);
        let max_lexical = if summary_lexical.is_some() || hit_max_lexical.is_some() {
            Some(calibrated_lexical_overlap(summary_lexical, hit_max_lexical))
        } else {
            None
        };

        let raw_fused = match (max_semantic, max_lexical) {
            (Some(s), Some(l)) => Some(s.max(l)),
            (Some(s), None) => Some(s),
            (None, Some(l)) => Some(l),
            (None, None) => None,
        };

        let recency_bucket = bundle
            .overlap_summary
            .as_ref()
            .and_then(|s| s.recency_bucket);
        let fused_overlap = raw_fused.map(|raw| {
            let weight = recency_weight(recency_bucket, raw, self.config.not_novel_threshold);
            (raw * weight).clamp(0.0, 1.0)
        });

        let traces_absent = bundle
            .query_traces
            .as_ref()
            .map(|t| t.is_empty())
            .unwrap_or(true);

        if hits.is_empty() && fused_overlap.is_none() {
            if traces_absent {
                return NoveltyVerdict::InsufficientEvidence {
                    reason: "prior-art retrieval produced no evidence".into(),
                };
            }
            return NoveltyVerdict::Novel;
        }

        // Hits exist, yet none carry semantic, lexical, or text-overlap scores:
        // never treat unverified hits as `Novel`.
        if fused_overlap.is_none() {
            return NoveltyVerdict::InsufficientEvidence {
                reason: "prior-art hits carry neither semantic nor lexical overlap scores".into(),
            };
        }

        // Check for opposing polarity evidence conflicts among high-similarity hits.
        let polarized: Vec<PolarizedHit> = per_hit_scores
            .iter()
            .filter_map(|(h, fused)| {
                fused.map(|similarity| PolarizedHit {
                    work_uri: h.work_uri.clone(),
                    similarity,
                    polarity: derive_hit_polarity(h),
                    excerpt: h.overlap_note.clone(),
                })
            })
            .collect();
        let detector = EvidenceConflictDetector::new(self.config.not_novel_threshold);
        if let Some(conflict) = detector.detect(cand_clean.unwrap_or(""), &polarized)
            && let Some(contra) = conflict.contradicting_hits.first()
        {
            return NoveltyVerdict::Contradicted {
                conflicting_uri: contra.work_uri.clone(),
            };
        }

        match fused_overlap {
            None => NoveltyVerdict::InsufficientEvidence {
                reason: "no lexical or semantic overlap signal available".into(),
            },
            Some(score) if score < self.config.novel_threshold => NoveltyVerdict::Novel,
            Some(score) if score >= self.config.not_novel_threshold => {
                let closest_uri = per_hit_scores
                    .iter()
                    .max_by(|(a, sa), (b, sb)| {
                        sa.unwrap_or(0.0)
                            .partial_cmp(&sb.unwrap_or(0.0))
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then_with(|| a.year.unwrap_or(0).cmp(&b.year.unwrap_or(0)))
                    })
                    .map(|(h, _)| h.work_uri.clone())
                    .unwrap_or_default();
                NoveltyVerdict::NotNovel {
                    closest_hit_uri: closest_uri,
                    similarity: score,
                }
            }
            Some(score) => NoveltyVerdict::PossiblyNovel {
                closest_score: score,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_research_events::schema_types::{
        NormalizedHit, NoveltyEvidenceBundle, NoveltySource, OverlapSummary, QueryTrace,
    };

    fn make_bundle(hits: Vec<NormalizedHit>, max_semantic: Option<f64>) -> NoveltyEvidenceBundle {
        NoveltyEvidenceBundle {
            schema_version: 1,
            bundle_id: "B-test".to_string(),
            candidate_id: "C-test".to_string(),
            computed_at_ms: 0,
            query_digest_sha256: "a".repeat(64),
            sources: vec![NoveltySource::Manual],
            normalized_hits: hits,
            overlap_summary: max_semantic.map(|s| OverlapSummary {
                max_lexical_score: None,
                max_semantic_score: Some(s),
                recency_bucket: None,
            }),
            query_traces: None,
        }
    }

    fn hit(work_uri: &str, semantic_score: Option<f64>) -> NormalizedHit {
        NormalizedHit {
            source: NoveltySource::Manual,
            work_uri: work_uri.to_string(),
            title: "Test hit".to_string(),
            year: None,
            lexical_score: None,
            semantic_score,
            overlap_note: None,
            cited_by_count: None,
        }
    }

    #[test]
    fn empty_bundle_is_insufficient_evidence() {
        let bundle = make_bundle(vec![], None);
        let scorer = AtomicNoveltyScorer::default();
        assert!(matches!(
            scorer.score(&bundle),
            NoveltyVerdict::InsufficientEvidence { reason }
            if reason.contains("retrieval") || reason.contains("evidence")
        ));
    }

    #[test]
    fn successful_search_empty_hits_is_novel() {
        let mut bundle = make_bundle(vec![], None);
        bundle.query_traces = Some(vec![trace("openalex", Some(200), None)]);
        let scorer = AtomicNoveltyScorer::default();
        assert_eq!(scorer.score(&bundle), NoveltyVerdict::Novel);
    }

    #[test]
    fn low_score_is_novel() {
        let bundle = make_bundle(vec![hit("doi:10.1/low", Some(0.3))], Some(0.3));
        let scorer = AtomicNoveltyScorer::default();
        assert_eq!(scorer.score(&bundle), NoveltyVerdict::Novel);
    }

    #[test]
    fn high_score_is_not_novel() {
        let bundle = make_bundle(vec![hit("doi:10.x", Some(0.85))], Some(0.85));
        let scorer = AtomicNoveltyScorer::default();
        assert!(matches!(
            scorer.score(&bundle),
            NoveltyVerdict::NotNovel { closest_hit_uri, similarity }
            if closest_hit_uri == "doi:10.x" && (similarity - 0.85).abs() < 1e-9
        ));
    }

    #[test]
    fn mid_score_is_possibly_novel() {
        let bundle = make_bundle(vec![hit("doi:10.mid", Some(0.65))], Some(0.65));
        let scorer = AtomicNoveltyScorer::default();
        assert!(matches!(
            scorer.score(&bundle),
            NoveltyVerdict::PossiblyNovel { closest_score }
            if (closest_score - 0.65).abs() < 1e-9
        ));
    }

    fn trace(source: &str, http_status: Option<i32>, cached: Option<bool>) -> QueryTrace {
        QueryTrace {
            source: source.to_string(),
            request_fingerprint_sha256: "f".repeat(64),
            http_status,
            cached,
        }
    }

    #[test]
    fn all_sources_failed_is_insufficient_evidence() {
        // Empty bundle (max_semantic_score=0.0 → would map to Novel) but every
        // prior-art source returned a non-success status: retrieval failed.
        let mut bundle = make_bundle(vec![], Some(0.0));
        bundle.query_traces = Some(vec![
            trace("openalex", Some(503), None),
            trace("crossref", Some(500), None),
        ]);
        let scorer = AtomicNoveltyScorer::default();
        assert!(matches!(
            scorer.score(&bundle),
            NoveltyVerdict::InsufficientEvidence { .. }
        ));
    }

    #[test]
    fn partial_failure_is_not_insufficient() {
        // One source succeeded → not a failure; falls through the normal ladder.
        let mut bundle = make_bundle(vec![], Some(0.0));
        bundle.query_traces = Some(vec![
            trace("openalex", Some(503), None),
            trace("crossref", Some(200), None),
        ]);
        let scorer = AtomicNoveltyScorer::default();
        assert_eq!(scorer.score(&bundle), NoveltyVerdict::Novel);
    }

    #[test]
    fn cached_trace_counts_as_success() {
        let mut bundle = make_bundle(vec![], Some(0.0));
        bundle.query_traces = Some(vec![trace("openalex", None, Some(true))]);
        let scorer = AtomicNoveltyScorer::default();
        assert_eq!(scorer.score(&bundle), NoveltyVerdict::Novel);
    }

    #[test]
    fn no_summary_no_scores_is_insufficient_evidence() {
        let bundle = make_bundle(vec![hit("doi:10.test", None)], None);
        let scorer = AtomicNoveltyScorer::default();
        let verdict = scorer.score(&bundle);
        assert!(matches!(
            verdict,
            NoveltyVerdict::InsufficientEvidence { .. }
        ));
    }

    #[test]
    fn lexical_only_prior_art_without_semantic_is_not_novel() {
        let mut h = hit("https://openalex.org/W4390000001", None);
        h.title = "Exact Prior Art on Deterministic Continuum Routing".into();
        h.year = Some(2025);
        h.lexical_score = Some(0.95);
        let mut bundle = make_bundle(vec![h], None);
        bundle.overlap_summary = Some(OverlapSummary {
            max_lexical_score: Some(0.95),
            max_semantic_score: None,
            recency_bucket: Some(RecencyBucket::VeryRecent),
        });
        bundle.query_traces = Some(vec![trace("openalex", Some(200), Some(true))]);
        let scorer = AtomicNoveltyScorer::default();
        assert!(matches!(
            scorer.score(&bundle),
            NoveltyVerdict::NotNovel { closest_hit_uri, similarity }
            if closest_hit_uri == "https://openalex.org/W4390000001" && (similarity - 0.95).abs() < 1e-9
        ));
    }

    #[test]
    fn lexical_mid_overlap_without_semantic_is_possibly_novel() {
        let mut h = hit("https://openalex.org/Wmid", None);
        h.lexical_score = Some(0.62);
        let mut bundle = make_bundle(vec![h], None);
        bundle.query_traces = Some(vec![trace("openalex", Some(200), None)]);
        let scorer = AtomicNoveltyScorer::default();
        assert!(matches!(
            scorer.score(&bundle),
            NoveltyVerdict::PossiblyNovel { closest_score }
            if (closest_score - 0.62).abs() < 1e-9
        ));
    }

    #[test]
    fn candidate_text_shingle_overlap_blocks_scoreless_false_novel() {
        let mut h = hit("https://openalex.org/Wshingle", None);
        h.title = "Deterministic Continuum Routing for Multi Agent Tool Graphs".into();
        h.year = Some(2024);
        let mut bundle = make_bundle(vec![h], None);
        bundle.query_traces = Some(vec![trace("openalex", Some(200), None)]);
        let scorer = AtomicNoveltyScorer::default();
        let verdict = scorer.score_with_context(
            &bundle,
            Some("Deterministic Continuum Routing for Multi Agent Tool Graphs"),
            Some(&ChronoFilter::new(1_760_000_000)),
        );
        assert!(matches!(
            verdict,
            NoveltyVerdict::NotNovel { closest_hit_uri, similarity }
            if closest_hit_uri == "https://openalex.org/Wshingle" && similarity >= 0.99
        ));
    }

    #[test]
    fn conflict_hits_produce_contradicted_verdict() {
        let mut h1 = hit("https://openalex.org/W1", Some(0.9));
        h1.year = Some(2022);
        let mut h2 = hit("https://openalex.org/W2", Some(0.9));
        h2.year = Some(2021);
        h2.overlap_note = Some("this work contradicts the proposed approach".into());
        let mut bundle = make_bundle(vec![h1, h2], Some(0.9));
        bundle.query_traces = Some(vec![trace("openalex", Some(200), None)]);
        let scorer = AtomicNoveltyScorer::default();
        assert_eq!(
            scorer.score(&bundle),
            NoveltyVerdict::Contradicted {
                conflicting_uri: "https://openalex.org/W2".into()
            }
        );
    }
}
