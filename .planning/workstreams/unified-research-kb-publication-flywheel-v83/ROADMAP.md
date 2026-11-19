# Roadmap: Milestone v8.3 (`unified-research-kb-publication-flywheel-v83`) — Unified Active Research, Federated KB & Autonomous Discovery-to-Publication Flywheel

**Workstream:** `.planning/workstreams/unified-research-kb-publication-flywheel-v83/`
**Scope:** Unify the five-stage workflow bridging active Research (`ResearchView`), 3-layer Federated Knowledge Base (`vox_baseline`, `~/.vox/library/`, and repo-local `docs/wiki/` in `MemoryView`), Discovery Review (`DiscoveryInbox` / `DiscoveryReview` / `ScientiaSurface`), and 7-stage Publication Studio (`PublicationsView`) with zero split-brain ingest, intact candidate promotion prefixes, `EmpiricalScience` class preservation, and closed-loop publication-to-KB indexing (`Phases 489–494`, `24` requirements).

## Phase Checklist

- [x] **Phase 489: Workstream Initialization & Closed-Loop Research ↔ Federated KB ↔ Publication SSOT** (`FLY-SSOT-01..04`)
- [x] **Phase 490: Unified KB Ingest Scope & Dual Findings Quarantine Bridge** (`FLY-INGEST-01..04`)
- [x] **Phase 491: Research-to-Discovery Promotion Prefix & `EmpiricalScience` Class Fidelity** (`FLY-PROMO-01..04`)
- [x] **Phase 492: Closed-Loop Publication → KB Synthesis Back-Indexing (`pub:{id}`)** (`FLY-BACKINDEX-01..04`)
- [x] **Phase 493: Cross-Surface GUI Journey, 3-Layer Scope Selector & Studio Handoff** (`FLY-UX-01..04`)
- [x] **Phase 494: Rust, TypeScript, Vitest, Doc-Lint & Playwright CV Flywheel Verification** (`FLY-VERIFY-01..04`)

---

### Phase 489: Workstream Initialization & Closed-Loop Research ↔ Federated KB ↔ Publication SSOT

**Directory**: `.planning/workstreams/unified-research-kb-publication-flywheel-v83/phases/489-closed-loop-ssot/`
**Goal**: Initialize the `v8.3` workstream, update `.planning/PROJECT.md`, and author `docs/src/architecture/unified-research-kb-publication-flywheel-ssot-2026.md` documenting how active research, the 3-layer federated KB, and Scientia discovery-to-publication operate cleanly standalone or in closed-loop tandem.
**Depends on**: None
**Requirements**: FLY-SSOT-01, FLY-SSOT-02, FLY-SSOT-03, FLY-SSOT-04

### Phase 490: Unified KB Ingest Scope & Dual Findings Quarantine Bridge

**Directory**: `.planning/workstreams/unified-research-kb-publication-flywheel-v83/phases/490-unified-kb-ingest/`
**Goal**: Forward `target_scope` through `crates/vox-gui/src/commands/knowledge.rs` and `crates/vox-orchestrator-mcp/src/gui_ops/knowledge.rs`, record scope in node metadata, best-effort file companion `kb_findings` entries, preserve `ToolResult::ok(node_id)` API compatibility, and handle both `string` and `FindingFiled` responses in `MemoryView.tsx`.
**Depends on**: Phase 489
**Requirements**: FLY-INGEST-01, FLY-INGEST-02, FLY-INGEST-03, FLY-INGEST-04

### Phase 491: Research-to-Discovery Promotion Prefix & `EmpiricalScience` Class Fidelity

**Directory**: `.planning/workstreams/unified-research-kb-publication-flywheel-v83/phases/491-promotion-prefix-class/`
**Goal**: Update `promote_candidate_to_publication` in `crates/vox-scientia/src/review_flow.rs` to resolve `"finding-{session_id}"` prefixes against `"finding-{session_id}-{digest}"` candidate IDs and hydrate preserved `"candidate_class"` (e.g., `EmpiricalScience`) from `confidence_json`.
**Depends on**: Phase 490
**Requirements**: FLY-PROMO-01, FLY-PROMO-02, FLY-PROMO-03, FLY-PROMO-04

### Phase 492: Closed-Loop Publication → KB Synthesis Back-Indexing (`pub:{id}`)

**Directory**: `.planning/workstreams/unified-research-kb-publication-flywheel-v83/phases/492-publication-kb-backindex/`
**Goal**: Extend `advance_publication_stage_in_db` in `crates/vox-scientia/src/review_flow.rs` so publications reaching `approved` or `published` upsert a searchable `knowledge_nodes` entry (`node_id = "pub:{publication_id}"`, `node_type = Some("research_synthesis")`) closing the discovery-to-KB flywheel.
**Depends on**: Phase 491
**Requirements**: FLY-BACKINDEX-01, FLY-BACKINDEX-02, FLY-BACKINDEX-03, FLY-BACKINDEX-04

### Phase 493: Cross-Surface GUI Journey, 3-Layer Scope Selector & Studio Handoff

**Directory**: `.planning/workstreams/unified-research-kb-publication-flywheel-v83/phases/493-cross-surface-ux/`
**Goal**: Fix `DiscoveryInbox.tsx` review routing (`'activity'` → `'scientia'`), add `vox://scientia-open-studio` synchronization and `"Open in Publications Studio"` handoff in `DiscoveryReview.tsx` → `PublicationsView.tsx`, add the 3-layer KB scope strip + Research link in `MemoryView.tsx`, and add a compact 5-stage Discovery-to-Publication Journey Strip across `ResearchView.tsx` and `PublicationsView.tsx`.
**Depends on**: Phase 492
**Requirements**: FLY-UX-01, FLY-UX-02, FLY-UX-03, FLY-UX-04

### Phase 494: Rust, TypeScript, Vitest, Doc-Lint & Playwright CV Flywheel Verification

**Directory**: `.planning/workstreams/unified-research-kb-publication-flywheel-v83/phases/494-flywheel-verification/`
**Goal**: Format changed Rust files with `rustfmt --edition 2024`, pass `vox-doc-pipeline --lint-only`, Rust unit test suites (`vox-scientia`, `vox-orchestrator-mcp`, `vox-gui`), `tsc --noEmit`, affected Vitest specs, and Playwright `kb-research-cv-flywheel.spec.ts` (`CV_STAGE=after`) with zero defects, then mark Milestone `v8.3` complete.
**Depends on**: Phase 493
**Requirements**: FLY-VERIFY-01, FLY-VERIFY-02, FLY-VERIFY-03, FLY-VERIFY-04
