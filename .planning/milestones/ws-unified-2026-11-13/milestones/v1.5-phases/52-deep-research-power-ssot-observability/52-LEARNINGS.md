---
phase: 52
phase_name: "deep-research-power-ssot-observability"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 52 Learnings: deep-research-power-ssot-observability

## Decisions

### Close Gap 1 from [scientia-publication-and-deep-research-claims-vs-reali...
Close Gap 1 from [`scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md`](../../../../docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md): 1. **`vox-doc-pipeline`-Compliant Frontmatter & `<!-- kb:summary -->` (`crates/vox-scientia/src/manuscript/scaffold/architecture_ssot.rs`)**: Update `ArchitectureSsotInput` and `render_architecture_ssot` so generated `docs/src/architecture/<slug>-2026.md` files include valid taxonomy facets (`topic: "scientia-research"`, `domain: "knowledge"`, `authorship: "llm-assisted"`, `authorship_model`) and a 40–120 word `<!-- kb:summary -->` block immediately under the H1, and omit `training_eligible: true` when `authorship` is `"llm-assisted"` without `human_attested_by` (avoiding `facet-training-eligible-contradiction`). 2. **Real `VoxDb` Hydration in `render_doc_draft` (`crates/vox-orchestrator-mcp/src/gui_ops/research.rs`)**: Replace the hardcoded empty-vector stub in `render_doc_draft` with real `VoxDb` hydration (`db.get_research_session(session_id)`, `db.get_research_artifact(session_id)`, and `db.list_publication_claims(session_id)`), populating `verified_claims`, `codebase_refs`, `competitive_matrix`, `sandbox_probes`, `gaps_and_recommendations`, `roadmap_phases`, and `stability_score`.

**Rationale:** Primary phase objective established in plan.
**Source:** 52-01-PLAN.md

---

### Close Gaps 6 and 8 from [scientia-publication-and-deep-research-claims-v...
Close Gaps 6 and 8 from [`scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md`](../../../../docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md): 1. **Full `ResearchQuery` Controls (`crates/vox-gui/src/commands/research.rs` + `gui_ops/dto/research.rs`)**: Extend `start_research_async` and `research_run_params` to forward `site_scope: Option<String>` alongside `domain_mode`, `scope`, `waves`, `max_sources`, `verify_claims`, and `lane` into `dei_method::RESEARCH_RUN` (`ResearchStartParams`). 2. **True 4-Layer Epistemic Graph Builder (`build_epistemic_dag` in `gui_ops/research.rs`)**: Construct hierarchical `EpistemicDagNodeDto` (`query`, `subquery`, `source_domain`, `claim`, `sandbox_probe`) and `EpistemicDagEdgeDto` (`decomposes_to`, `retrieves_from`, `grounds_claim`, `contradicts`, `verifies_compile`) from `artifact_json`, returning `epistemic_dag` + `wave_count` + `wave_stability` + `usage` on `ResearchDetailDto`.

**Rationale:** Primary phase objective established in plan.
**Source:** 52-02-PLAN.md

## Lessons

## Patterns

## Surprises

