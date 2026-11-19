# Requirements: Vox — Milestone v8.3 (`unified-research-kb-publication-flywheel-v83`)

**Defined:** 2026-11-18
**Completed:** 2026-11-18
**Workstream:** `.planning/workstreams/unified-research-kb-publication-flywheel-v83/`
**Core Value:** Active Deep Research, 3-layer Federated Knowledge Base (`vox_baseline`, `~/.vox/library/`, and repo-local `docs/wiki/`), and autonomous Scientia Discovery-to-Publication must operate cleanly either standalone (offline/baseline) or in closed-loop tandem with zero split-brain ingest, lossless promotion, and publication back-indexing.

## v8.3 Requirements

Requirements for Milestone `v8.3` (`Phases 489–494`, `24/24` verified).

### Closed-Loop Research ↔ Federated KB ↔ Publication SSOT (`FLY-SSOT`) — Phase 489

- [x] **FLY-SSOT-01**: Initialize `.planning/workstreams/unified-research-kb-publication-flywheel-v83/{ROADMAP.md,REQUIREMENTS.md,STATE.md,state.json}` covering Phases `489–494`.
- [x] **FLY-SSOT-02**: Update `.planning/PROJECT.md` to activate Milestone `v8.3`.
- [x] **FLY-SSOT-03**: Author `docs/src/architecture/unified-research-kb-publication-flywheel-ssot-2026.md` with compliant YAML frontmatter and `<!-- kb:summary -->` specifying the 5-stage workflow and 3-layer federated KB (standalone vs. tandem usage).
- [x] **FLY-SSOT-04**: Validate the authored SSOT document via `vox-doc-pipeline -- --lint-only`.

### Unified KB Ingest Scope & Dual Findings Quarantine Bridge (`FLY-INGEST`) — Phase 490

- [x] **FLY-INGEST-01**: Add optional `target_scope: Option<String>` to `KbIngestTextParams` and `KbIngestUrlParams` in `crates/vox-orchestrator-mcp/src/gui_ops/knowledge.rs` and persist `target_scope` in `knowledge_nodes` JSON metadata.
- [x] **FLY-INGEST-02**: Best-effort file companion entries via `file_gui_finding` inside `kb_ingest_text` and `kb_ingest_url` without breaking `ToolResult::ok(node_id)` (`String`) return parity.
- [x] **FLY-INGEST-03**: Forward `target_scope: Option<String>` from Tauri commands `ingest_text` and `ingest_url` in `crates/vox-gui/src/commands/knowledge.rs` while keeping existing 3-arg and 2-arg `_impl` unit tests passing.
- [x] **FLY-INGEST-04**: Update `toastFiled` in `crates/vox-gui/ui/src/components/surfaces/Memory/MemoryView.tsx` to support both `string` node IDs returned by the live daemon and `FindingFiled` DTOs used in mocks.

### Research-to-Discovery Promotion Prefix & `EmpiricalScience` Class Fidelity (`FLY-PROMO`) — Phase 491

- [x] **FLY-PROMO-01**: Update `promote_candidate_to_publication` in `crates/vox-scientia/src/review_flow.rs` to match `"finding-{session_id}"` prefixes against `"finding-{session_id}-{digest_hex}"` rows returned by `list_finding_candidates`.
- [x] **FLY-PROMO-02**: Hydrate `"candidate_class"` from `confidence_json` (preserving `EmpiricalScience` and downstream aliases instead of losing it to coarse `FindingCandidateClass::Other` column storage).
- [x] **FLY-PROMO-03**: Add fallback hydration from `research_sessions` / research summary metadata when a `"finding-<sid>"` prefix match requires session metadata context, and unit-test `"finding-<sid>"` prefix promotion in `crates/vox-scientia/src/review_flow.rs`.
- [x] **FLY-PROMO-04**: Ensure promoted `publication_manifests` preserve empirical domain and classification metadata for `PublicationsView`.

### Closed-Loop Publication → KB Synthesis Back-Indexing (`FLY-BACKINDEX`) — Phase 492

- [x] **FLY-BACKINDEX-01**: Extend `advance_publication_stage_in_db` in `crates/vox-scientia/src/review_flow.rs` so advancing a publication to `PublicationLifecycleStage::Approved` or `PublicationLifecycleStage::Published` upserts a `knowledge_nodes` entry (`node_id = "pub:{publication_id}"`).
- [x] **FLY-BACKINDEX-02**: Persist `node_type = Some("research_synthesis")` and structured metadata (`publication_id`, `stage`, `arxiv_id`, `doi`, `candidate_class`, `publication_tier`, `arxiv_primary_category`) on back-indexed publication nodes.
- [x] **FLY-BACKINDEX-03**: Ensure back-indexed nodes are searchable via `MemoryView` and `vox-search` lexical/hybrid lookups without breaking stage progression on non-KB fixtures.
- [x] **FLY-BACKINDEX-04**: Add Rust unit tests in `crates/vox-scientia/src/review_flow.rs` verifying `knowledge_nodes` upsert when advancing to `Approved` / `Published`.

### Cross-Surface GUI Journey, 3-Layer Scope Selector & Studio Handoff (`FLY-UX`) — Phase 493

- [x] **FLY-UX-01**: Fix `DiscoveryInbox.tsx` `openReview` routing from `view: 'activity'` to `view: 'scientia'`, and update `DiscoveryReview.tsx` to listen for `vox://scientia-open-studio`.
- [x] **FLY-UX-02**: Add `"Open in Publications Studio"` CTA in `DiscoveryReview.tsx` dispatching `vox://navigate-surface` `{ view: 'publications', publicationId }` and persisting `vox_selected_publication_id` read by `PublicationsView.tsx`.
- [x] **FLY-UX-03**: Add interactive 3-Layer Federated KB Scope Selector (`data-testid="kb-layer-scope-bar"` with `All Layers`, `Repo-Local`, `Global Library`, `Vox Baseline Canon`) and `"To Research"` handoff button in `MemoryView.tsx`.
- [x] **FLY-UX-04**: Add compact 5-Stage Discovery-to-Publication Journey Strip (`data-testid="flywheel-journey-strip"`) across `ResearchView.tsx` and `PublicationsView.tsx` adhering to theme contract (`>= 11px` font sizes, `h-8` uniform button heights, zero hardcoded dark hexes, zero emojis or bracketed button labels).

### Rust, TypeScript, Vitest, Doc-Lint & Playwright CV Flywheel Verification (`FLY-VERIFY`) — Phase 494

- [x] **FLY-VERIFY-01**: Format changed `.rs` files with `rustfmt --edition 2024` and pass `cargo test -p vox-scientia --lib review_flow`, `cargo test -p vox-orchestrator-mcp --lib gui_ops::knowledge`, and `cargo test -p vox-gui commands::knowledge`.
- [x] **FLY-VERIFY-02**: Update/add Vitest assertions for new UX controls and pass `tsc --noEmit` + targeted Vitest specs (`7/7` files, `69/69` tests).
- [x] **FLY-VERIFY-03**: Pass `VOX_CV_FLYWHEEL=1 CV_STAGE=after` Playwright `e2e/review/kb-research-cv-flywheel.spec.ts` (`16/16` captures converged, `0` DOM/Axe/contrast/font/overflow defects across `arcane` and `void`).
- [x] **FLY-VERIFY-04**: Mark Phases `489–494` and all `24` requirements complete in `.planning/workstreams/unified-research-kb-publication-flywheel-v83/{ROADMAP.md,REQUIREMENTS.md,STATE.md,state.json}` and `.planning/PROJECT.md`.
