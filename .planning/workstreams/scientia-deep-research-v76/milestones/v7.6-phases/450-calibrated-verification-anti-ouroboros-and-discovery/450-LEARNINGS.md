---
phase: 450
phase_name: "calibrated-verification-anti-ouroboros-and-discovery"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 1
  lessons: 2
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 450 Learnings: calibrated-verification-anti-ouroboros-and-discovery

## Decisions

### Ship calibrated 3-tier NLI/span verification (VERI-01), Clavis/mock-reti...
Ship calibrated 3-tier NLI/span verification (`VERI-01`), Clavis/mock-retirement/`vox-crypto` hygiene (`VERI-02`), Crossref retraction + `OfflineAttested` trust + `GcraLimiter` + OpenAlex Clavis auth (`VERI-03`, `DISC-03`), anti-Ouroboros taint (`VERI-04`), fused lexical+semantic `AtomicNoveltyScorer` (`DISC-02`), and two-stage Novelty × Utility continuous discovery (`DISC-04`). Purpose: Eliminate mock verification, self-ingestion loops, `None => Novel` false positives, and unthrottled/deadlocked scholarly lookups without adding Cargo crates or `crate-edges` entries. Output: Three file-disjoint Wave D tracks (`p450-verifier`, `p450-trust-ouroboros`, `p450-novelty-disc`), verified in batched Wave F.

**Rationale:** Primary phase objective established in plan.
**Source:** 450-01-PLAN.md

## Lessons

### Blocking Issues
**Blocking Issues:** `0`

**Context:** Surfaced in structured phase code review.
**Source:** 450-REVIEW.md

---

### Phase 450 Architectural / Policy Violations
**Phase 450 Architectural / Policy Violations:** `0`

**Context:** Surfaced in structured phase code review.
**Source:** 450-REVIEW.md

## Patterns

## Surprises

