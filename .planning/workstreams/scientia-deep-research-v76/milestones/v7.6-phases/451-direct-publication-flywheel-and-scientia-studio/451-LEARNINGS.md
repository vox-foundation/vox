---
phase: 451
phase_name: "direct-publication-flywheel-and-scientia-studio"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 1
  lessons: 3
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 451 Learnings: direct-publication-flywheel-and-scientia-studio

## Decisions

### Implement the full DB-hydrated Direct Publication Flywheel (PUBL-01..04)...
Implement the full DB-hydrated Direct Publication Flywheel (`PUBL-01..04`) and zero-`VoxDb` Axis Scientia/Publications Studio (`STUD-01..02`), resolving `artifact_json` wipes, manual scaffold JSON bottlenecks, LaTeX/arXiv hardening gaps, and CLI-DB subprocess spawns. Purpose: Let operators go from research session or `DiscoveryInbox` candidate to verified SSOT doc, IMRaD/LaTeX/arXiv/RO-Crate/Finding Page, Trusty-URI nanopub, and archival receipt behind a 4-layer human `ApprovalToken` gate inside one unified GUI. Output: Two file-disjoint Wave E tracks (`p451-flywheel` and `p451-studio-ui`), verified in batched Wave F.

**Rationale:** Primary phase objective established in plan.
**Source:** 451-01-PLAN.md

## Lessons

### Blocking Issues
**Blocking Issues:** `0`

**Context:** Surfaced in structured phase code review.
**Source:** 451-REVIEW.md

---

### Phase 451 Architectural / Policy Violations
**Phase 451 Architectural / Policy Violations:** `0`

**Context:** Surfaced in structured phase code review.
**Source:** 451-REVIEW.md

---

### Seam Resolved in Coordination & Wave F
**Seam Resolved in Coordination & Wave F:** Coordinator wired `pub mod doc_synthesizer;` into `crates/vox-research-shim/src/research/mod.rs`; `rust-verifier` aligned `flywheel_build_worthiness` to `vox_db_types::ScientiaClaimWithVerdict`, fixed `QueueSnapshot` field assertions in `review_flow.rs`, and migrated `scientia_phase_handlers.rs` lines 370–376 onto `vox_secrets::resolve_secret`.

**Context:** Surfaced in structured phase code review.
**Source:** 451-REVIEW.md

## Patterns

## Surprises

