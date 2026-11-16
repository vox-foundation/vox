//! `ResearchEvent` — the typed event bus for the SCIENTIA signal ladder.
//!
//! All cross-crate SCIENTIA events flow through this enum. Consumers subscribe via
//! `ResearchEventEmitter`; the orchestrator is the primary producer.
//!
//! Signal ladder mapping:
//!   T0 (observation) → `TelemetryObservation`
//!   T1 (aggregate)   → `AggregateComputed`
//!   T2 (atomic claim)→ `ClaimExtracted`, `ClaimVerified`, `NoveltyAssessed`
//!   T3 (finding)     → `FindingCandidateProposed`, `FindingApproved`
//!   T4 (publication) → `PublicationAttempted`, `PublicationSucceeded`, `PublicationFailed`

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

/// Coarse event category for filtering without deserializing the payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchEventKind {
    TelemetryObservation,
    AggregateComputed,
    ClaimExtracted,
    ClaimVerified,
    NoveltyAssessed,
    FindingCandidateProposed,
    FindingApproved,
    PreregistrationSubmitted,
    CampaignStarted,
    CampaignAborted,
    PublicationAttempted,
    PublicationSucceeded,
    PublicationFailed,
    RetractionEmitted,
    RightOfReplyWindowOpened,
    RightOfReplyCleared,
    WaveStarted,
    WaveCompleted,
    SubqueryDispatched,
    ContinuumEscalated,
    EpistemicDagDelta,
}

/// Typed SCIENTIA pipeline event.
///
/// Consumers match on this enum; serde tags allow JSON transport across
/// process boundaries (e.g. orchestrator → publisher).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "PascalCase")]
pub enum ResearchEvent {
    // ── T0 — Telemetry observation ────────────────────────────────────────────
    TelemetryObservation {
        provider: String,
        metric_type: String,
        value: f64,
        session_id: String,
        recorded_at_ms: i64,
    },

    // ── T1 — Aggregate computed ───────────────────────────────────────────────
    AggregateComputed {
        provider: String,
        metric_type: String,
        window_start_ms: i64,
        window_end_ms: i64,
        value: f64,
        sample_count: u64,
        session_id: String,
    },

    // ── T2 — Atomic claim extracted ───────────────────────────────────────────
    ClaimExtracted {
        claim_id: u64,
        text: String,
        verifiability_score: f64,
        session_id: String,
    },

    // ── T2 — Claim verified ───────────────────────────────────────────────────
    ClaimVerified {
        claim_id: u64,
        verdict: String, // "Supported" | "Contradicted" | "Contested" | "Unverified"
        confidence: f64,
        verifier_model: String,
        session_id: String,
    },

    // ── T2 — Novelty assessed ─────────────────────────────────────────────────
    NoveltyAssessed {
        claim_id: u64,
        is_novel: bool,
        /// Highest cosine similarity to prior corpus (SPECTER2).
        max_prior_similarity: f64,
        evidence_conflict: bool,
        session_id: String,
    },

    // ── T3 — Finding candidate proposed ──────────────────────────────────────
    FindingCandidateProposed {
        finding_id: String,
        claim_ids: Vec<u64>,
        worthiness_score: f64,
        session_id: String,
        /// Full `FindingCandidateV1` JSON when produced by the research pipeline.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        finding_candidate: Option<serde_json::Value>,
    },

    // ── T3 — Finding approved by dual-approver gate ───────────────────────────
    FindingApproved {
        finding_id: String,
        approved_by: Vec<String>,
        session_id: String,
    },

    // ── Pre-registration ──────────────────────────────────────────────────────
    PreregistrationSubmitted {
        prereg_id: String,
        /// SHA-256 of the hypothesis field (tamper evidence without full body).
        hypothesis_digest: String,
    },

    // ── Campaign lifecycle ────────────────────────────────────────────────────
    CampaignStarted {
        campaign_id: String,
        prereg_id: String,
        cost_cap_usd: f64,
    },
    CampaignAborted {
        campaign_id: String,
        reason: String,
        cost_incurred_usd: f64,
    },

    // ── T4 — Publication ──────────────────────────────────────────────────────
    PublicationAttempted {
        manifest_id: String,
        venue: String,
        finding_ids: Vec<String>,
    },
    PublicationSucceeded {
        manifest_id: String,
        doi: Option<String>,
        nanopub_uris: Vec<String>,
    },
    PublicationFailed {
        manifest_id: String,
        error: String,
    },

    // ── Right-of-reply ────────────────────────────────────────────────────────
    RightOfReplyWindowOpened {
        manifest_id: String,
        providers_notified: Vec<String>,
        deadline_ms: i64,
    },
    RightOfReplyCleared {
        manifest_id: String,
        replies_received: u32,
    },

    // ── Retraction ────────────────────────────────────────────────────────────
    RetractionEmitted {
        target_doi: String,
        reason: String,
        nanopub_uri: String,
    },

    // ── Continuum / Multi-Wave / Epistemic DAG deltas ─────────────────────────
    WaveStarted {
        session_id: String,
        wave: u8,
        max_waves: u8,
        tier: String,
        subquery_count: usize,
    },
    WaveCompleted {
        session_id: String,
        wave: u8,
        new_sources: usize,
        verified_claims: usize,
        stability_score: f64,
    },
    SubqueryDispatched {
        session_id: String,
        wave: u8,
        subquery_index: usize,
        subquery: String,
        provider_lane: String,
    },
    ContinuumEscalated {
        session_id: String,
        from_tier: String,
        to_tier: String,
        reason: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        trigger_score: Option<f64>,
    },
    EpistemicDagDelta {
        session_id: String,
        wave: u8,
        added_nodes: usize,
        added_edges: usize,
        contradicted_claims: usize,
    },
}

/// Monotonic watermarked wire envelope around [`ResearchEvent`] for SSE/WS resume.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequencedResearchEvent {
    pub seq: u64,
    pub session_id: String,
    pub recorded_at_ms: i64,
    pub event: ResearchEvent,
}

/// Lock-free monotonic sequence generator (`AtomicU64`, starting at `1`).
#[derive(Debug)]
pub struct ResearchEventSequencer {
    next_seq: AtomicU64,
}

impl Default for ResearchEventSequencer {
    fn default() -> Self {
        Self::new()
    }
}

impl ResearchEventSequencer {
    /// Construct a sequencer whose first emitted event receives `seq = 1`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next_seq: AtomicU64::new(1),
        }
    }

    /// Construct a sequencer resuming at `start_seq` (clamped to `>= 1`).
    #[must_use]
    pub const fn with_start_seq(start_seq: u64) -> Self {
        let initial = if start_seq == 0 { 1 } else { start_seq };
        Self {
            next_seq: AtomicU64::new(initial),
        }
    }

    /// Wrap `event` in a [`SequencedResearchEvent`] with the next monotonic `seq`.
    pub fn next_event(
        &self,
        session_id: impl Into<String>,
        recorded_at_ms: i64,
        event: ResearchEvent,
    ) -> SequencedResearchEvent {
        let seq = self.next_seq.fetch_add(1, Ordering::AcqRel);
        SequencedResearchEvent {
            seq,
            session_id: session_id.into(),
            recorded_at_ms,
            event,
        }
    }

    /// Highest sequence number emitted so far (`0` before the first event).
    #[must_use]
    pub fn current_watermark(&self) -> u64 {
        self.next_seq.load(Ordering::Acquire).saturating_sub(1)
    }
}

impl ResearchEvent {
    pub fn kind(&self) -> ResearchEventKind {
        match self {
            Self::TelemetryObservation { .. } => ResearchEventKind::TelemetryObservation,
            Self::AggregateComputed { .. } => ResearchEventKind::AggregateComputed,
            Self::ClaimExtracted { .. } => ResearchEventKind::ClaimExtracted,
            Self::ClaimVerified { .. } => ResearchEventKind::ClaimVerified,
            Self::NoveltyAssessed { .. } => ResearchEventKind::NoveltyAssessed,
            Self::FindingCandidateProposed { .. } => ResearchEventKind::FindingCandidateProposed,
            Self::FindingApproved { .. } => ResearchEventKind::FindingApproved,
            Self::PreregistrationSubmitted { .. } => ResearchEventKind::PreregistrationSubmitted,
            Self::CampaignStarted { .. } => ResearchEventKind::CampaignStarted,
            Self::CampaignAborted { .. } => ResearchEventKind::CampaignAborted,
            Self::PublicationAttempted { .. } => ResearchEventKind::PublicationAttempted,
            Self::PublicationSucceeded { .. } => ResearchEventKind::PublicationSucceeded,
            Self::PublicationFailed { .. } => ResearchEventKind::PublicationFailed,
            Self::RightOfReplyWindowOpened { .. } => ResearchEventKind::RightOfReplyWindowOpened,
            Self::RightOfReplyCleared { .. } => ResearchEventKind::RightOfReplyCleared,
            Self::RetractionEmitted { .. } => ResearchEventKind::RetractionEmitted,
            Self::WaveStarted { .. } => ResearchEventKind::WaveStarted,
            Self::WaveCompleted { .. } => ResearchEventKind::WaveCompleted,
            Self::SubqueryDispatched { .. } => ResearchEventKind::SubqueryDispatched,
            Self::ContinuumEscalated { .. } => ResearchEventKind::ContinuumEscalated,
            Self::EpistemicDagDelta { .. } => ResearchEventKind::EpistemicDagDelta,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_event_kind_round_trips() {
        let evt = ResearchEvent::ClaimExtracted {
            claim_id: 42,
            text: "latency increased".to_string(),
            verifiability_score: 0.85,
            session_id: "sess-001".to_string(),
        };
        let json = serde_json::to_string(&evt).unwrap();
        let back: ResearchEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            back,
            ResearchEvent::ClaimExtracted { claim_id: 42, .. }
        ));
    }

    #[test]
    fn preregistration_submitted_event_serializes() {
        let evt = ResearchEvent::PreregistrationSubmitted {
            prereg_id: "np:test".to_string(),
            hypothesis_digest: "sha256:abc".to_string(),
        };
        let json = serde_json::to_string(&evt).unwrap();
        assert!(json.contains("PreregistrationSubmitted"));
    }

    #[test]
    fn event_kind_matches_variant() {
        assert_eq!(
            ResearchEvent::ClaimExtracted {
                claim_id: 1,
                text: "t".to_string(),
                verifiability_score: 0.5,
                session_id: "s".to_string(),
            }
            .kind(),
            ResearchEventKind::ClaimExtracted
        );
    }

    #[test]
    fn continuum_wave_dag_variants_and_sequencer_round_trip() {
        use ResearchEventKind as K;

        let seq = ResearchEventSequencer::default();
        assert_eq!(seq.current_watermark(), 0);
        let payloads = [
            (
                r#"{"event_type":"WaveStarted","session_id":"s-1","wave":1,"max_waves":3,"tier":"deep","subquery_count":4}"#,
                K::WaveStarted,
            ),
            (
                r#"{"event_type":"WaveCompleted","session_id":"s-1","wave":1,"new_sources":8,"verified_claims":5,"stability_score":0.82}"#,
                K::WaveCompleted,
            ),
            (
                r#"{"event_type":"SubqueryDispatched","session_id":"s-1","wave":1,"subquery_index":0,"subquery":"q","provider_lane":"scholarly"}"#,
                K::SubqueryDispatched,
            ),
            (
                r#"{"event_type":"ContinuumEscalated","session_id":"s-1","from_tier":"local_rag","to_tier":"quick_web","reason":"low_conf","trigger_score":0.41}"#,
                K::ContinuumEscalated,
            ),
            (
                r#"{"event_type":"EpistemicDagDelta","session_id":"s-1","wave":1,"added_nodes":6,"added_edges":9,"contradicted_claims":1}"#,
                K::EpistemicDagDelta,
            ),
        ];
        for (i, (raw, kind)) in payloads.into_iter().enumerate() {
            let evt: ResearchEvent = serde_json::from_str(raw).unwrap();
            assert_eq!(evt.kind(), kind);
            let wrapped = seq.next_event("s-1", 1000 + i as i64, evt);
            assert_eq!(
                (wrapped.seq, seq.current_watermark()),
                ((i + 1) as u64, (i + 1) as u64)
            );
            let wire = serde_json::to_string(&wrapped).unwrap();
            assert_eq!(
                serde_json::from_str::<SequencedResearchEvent>(&wire).unwrap(),
                wrapped
            );
        }
        let resumed = ResearchEventSequencer::with_start_seq(42);
        let evt = ResearchEvent::RightOfReplyCleared {
            manifest_id: "m".into(),
            replies_received: 1,
        };
        let next = resumed.next_event("s-2", 2000, evt);
        assert_eq!((next.seq, resumed.current_watermark()), (42, 42));
    }
}
