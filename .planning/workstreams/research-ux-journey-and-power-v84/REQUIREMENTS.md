# Requirements: Vox — Milestone v8.4 (`research-ux-journey-and-power-v84`)

**Defined:** 2026-11-18
**Completed:** 2026-11-18
**Workstream:** `.planning/workstreams/research-ux-journey-and-power-v84/`
**Core Value:** Research, Knowledge Base, Discovery Review, and Publications Studio surfaces must minimize clutter, explain every epistemic gate and parameter in plain language on hover/inline, and launch statistical researcher workflows with live KB prior-art grounding.

## v8.4 Requirements (`Phases 495–500`, `24/24` verified)

### Workstream & Statistical Archetypes SSOT (`UX-SSOT`) — Phase 495

- [x] **UX-SSOT-01**: Update `.planning/PROJECT.md` to activate Milestone `v8.4`.
- [x] **UX-SSOT-02**: Create `.planning/workstreams/research-ux-journey-and-power-v84/{ROADMAP.md,REQUIREMENTS.md,STATE.md,state.json}`.
- [x] **UX-SSOT-03**: Author `docs/src/architecture/research-ux-journey-and-statistical-archetypes-ssot-2026.md` documenting prior research archetypes, unexplained vs. pruned UI elements, and preflight/confidence-gate upgrades.
- [x] **UX-SSOT-04**: Lint the new SSOT doc cleanly via `vox-doc-pipeline -- --lint-only`.

### Prune Redundant & Unwired UI Chrome (`UX-PRUNE`) — Phase 496

- [x] **UX-PRUNE-01**: Remove duplicate `CODEX SYNCED` wax seal and non-interactive slide rule track inside `MemoryView.tsx`.
- [x] **UX-PRUNE-02**: Move dynamic `LOW EVIDENCE` / `VERIFICATA` wax seal onto `RomanSurfaceHeader` in `ResearchView.tsx` and remove console duplicate.
- [x] **UX-PRUNE-03**: Replace the dead read-only `Stability` knob in `ResearchView.tsx` with an interactive `Confidence Gate` knob (`50..95% min`, comparing live against `stabilityPercent`).
- [x] **UX-PRUNE-04**: Deduplicate `DirectPublicationDrawer.tsx` when `compact={true}` inside `PublicationsView.tsx` while keeping full standalone controls in `DiscoveryReview.tsx`.

### Self-Documenting Surface Explanations (`UX-EXPLAIN`) — Phase 497

- [x] **UX-EXPLAIN-01**: Add plain-English novelty × utility gate formula legend and tooltip in `WorthinessRadarPanel.tsx`.
- [x] **UX-EXPLAIN-02**: Add explanatory tooltips/captions to `Reply window`, `Neg-result quota`, `Critic allowed`, and `Atlas gate` in `PublicationsView.tsx`.
- [x] **UX-EXPLAIN-03**: Explain corpora, `Top-K Depth`, and `BM25 + 768d Vector RRF (k=60)` cleanly in `MemoryView.tsx`.
- [x] **UX-EXPLAIN-04**: Add hover explanations across `Fast`, `Deep Research`, `Mode`, `Scope`, `Waves`, `Judge`, `Save to KB`, and `Promote` in `ResearchView.tsx`.

### Statistical Archetype Launchpad & Differentiated Workspace Tabs (`RES-ARCHETYPE`) — Phase 498

- [x] **RES-ARCHETYPE-01**: Add `data-testid="research-archetype-launchpad"` with the 4 statistical journey presets (`Prior-Art & KB Survey (35%)`, `Architecture Audit (25%)`, `Claim & Contradiction Check (25%)`, `Empirical Preprint (15%)`).
- [x] **RES-ARCHETYPE-02**: Configure lane/domainMode/scope/waves and clean query seeds in 1 click without overwriting non-empty queries.
- [x] **RES-ARCHETYPE-03**: Differentiate `Overview`, `Synthesis` (expanded report + height), `DAG & Claims` (`dag-claims-focus-banner`), and `Diagnostics` in `ResearchView.tsx`.
- [x] **RES-ARCHETYPE-04**: Ensure theme contract compliance (`h-8`, `>= 11px` font sizes, zero dark hexes/emojis/bracketed labels).

### Live Federated KB Prior-Art Preflight (`RES-PREFLIGHT`) — Phase 499

- [x] **RES-PREFLIGHT-01**: Add debounced preflight query against `vox_search_query` (`limit: 3`) inside `ResearchView.tsx` when `query.trim().length >= 4` and not `embedded`.
- [x] **RES-PREFLIGHT-02**: Render `data-testid="research-kb-preflight-panel"` showing matched baseline/published node previews or federated scope guidance.
- [x] **RES-PREFLIGHT-03**: Expose 1-click `Synthesize KB + Web Gaps` CTA (`data-testid="run-kb-gap-synthesis-btn"`) launching `deep` `both`-scope investigation.
- [x] **RES-PREFLIGHT-04**: Degrade silently inside unit tests / offline mocks without uncaught rejections.

### Verification & Milestone Closeout (`UX-VERIFY`) — Phase 500

- [x] **UX-VERIFY-01**: Add Vitest assertions for Archetype Launchpad, Confidence Gate knob, KB Prior-Art Preflight, and non-duplicated wax seals in `ResearchView.test.tsx`.
- [x] **UX-VERIFY-02**: Pass `tsc --noEmit` and the 9-file Vitest surface/contract/guard suite (`9/9` files, `77/77` tests).
- [x] **UX-VERIFY-03**: Pass `VOX_CV_FLYWHEEL=1 CV_STAGE=after` Playwright `kb-research-cv-flywheel.spec.ts` (`16/16` converged, `0` Axe defects).
- [x] **UX-VERIFY-04**: Mark Phases `495–500` and `24/24` requirements completed in `.planning/` and deliver final audit + changes summary.
