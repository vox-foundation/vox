---
phase: 53
phase_name: "scientia-publication-pipeline-closure"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 53 Learnings: scientia-publication-pipeline-closure

## Decisions

### Close Gap 2 from [scientia-publication-and-deep-research-claims-vs-reali...
Close Gap 2 from [`scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md`](../../../../docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md): 1. **Rich Candidate Classification in `discovery_bridge.rs` (`crates/vox-research-shim/src/research/discovery_bridge.rs`)**: Replace the hardcoded `EventCandidateClass::Other`, `title_hint: None`, and `novelty_evidence_bundle_id: None` in `finding_candidate_from_research_result` with deterministic classification (`BenchmarkDelta`, `ArchitectureDecision`, `ReproducibilityAudit`, `CorpusFinding`, `VerificationProof`, `Other`) and a clean `title_hint` extracted from the research report H1 or query. 2. **1-Click / 1-Command Candidate & Research Session Promotion (`promote_candidate_to_publication`)**: Implement `promote_candidate_to_publication` in `crates/vox-orchestrator-mcp/src/gui_ops/scientia.rs` (`POST /api/v2/scientia/candidates/:id/promote`), Tauri command `scientia_promote_candidate` in `crates/vox-gui/src/commands/scientia.rs`, and CLI `vox scientia candidate-promote --id <candidate_or_session_id> [--venue <venue>]`. Promotion creates a canonical `publication_manifests` row (`pub-<slug>-<short_id>`), updates `scientia_discovery_inbox.publication_id` and `scientia_finding_candidates.status = "promoted"`, and seeds `scientia_claims` from the research session's verified claims so `DiscoveryReview.tsx` and `ArchivePanel.tsx` never fail with `"not_found"`.

**Rationale:** Primary phase objective established in plan.
**Source:** 53-01-PLAN.md

---

### Close Gap 3 from [scientia-publication-and-deep-research-claims-vs-reali...
Close Gap 3 from [`scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md`](../../../../docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md): 1. **DB-Backed `assemble_scaffold_from_db` & `assemble_finding_page_from_db`**: Build a deterministic assembler that hydrates `vox_scientia_manuscript::scaffold::ScaffoldInput` and `vox_scientia_HTML::finding_page::FindingPageInput` directly from `VoxDb` (`publication_manifests` + `scientia_claims` + `novelty_evidence_bundles` + `ro_crate` metadata) given only a `publication_id` and optional `VenueStyle`. 2. **CLI & Daemon HTTP Endpoints Without Offline JSON Files**: - Update `vox scientia publication-manuscript-draft`, `publication-render-latex`, `publication-arxiv-bundle`, and `publication-finding-page-render` so `--publication-id <id>` is the primary input (keeping `--scaffold <path>` / `--page <path>` as optional overrides). - Expose `POST /api/v2/scientia/publications/:id/manuscript`, `POST /api/v2/scientia/publications/:id/latex`, `POST /api/v2/scientia/publications/:id/arxiv-bundle`, and `POST /api/v2/scientia/publications/:id/finding-page` in `crates/vox-orchestrator-mcp/src/gui_ops/scientia.rs` and `crates/vox-gui/src/commands/scientia.rs`.

**Rationale:** Primary phase objective established in plan.
**Source:** 53-02-PLAN.md

## Lessons

## Patterns

## Surprises

