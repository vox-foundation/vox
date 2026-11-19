# Requirements: Vox — Milestone v8.5 (`research-click-tax-observability-and-control-v85`)

**Defined:** 2026-11-18
**Workstream:** `.planning/workstreams/research-click-tax-observability-and-control-v85/`
**Core Value:** Operators must see active synthesis and live execution telemetry immediately above the fold, steer research mid-flight, control exact KB storage scope without mandatory extra clicks, and jump across search, claims, discovery review, and publications with zero raw ID copy-pasting.

## v8.5 Requirements (`Phases 501–506`, `24` requirements)

### Workstream & Visual Audit SSOT (`CTRL-SSOT`) — Phase 501

- [x] **CTRL-SSOT-01**: Update `.planning/PROJECT.md` to activate Milestone `v8.5`.
- [x] **CTRL-SSOT-02**: Create `.planning/workstreams/research-click-tax-observability-and-control-v85/{ROADMAP.md,REQUIREMENTS.md,STATE.md,state.json}`.
- [x] **CTRL-SSOT-03**: Author `docs/src/architecture/research-click-tax-observability-and-control-ssot-2026.md` documenting screenshot audit findings, click-tax removal, research observability/steering, scoped storage controls, and search enhancements.
- [x] **CTRL-SSOT-04**: Lint the SSOT doc cleanly via `vox-doc-pipeline -- --lint-only`.

### Above-the-Fold Compression, Observability & Steering (`RES-CTRL`) — Phase 502

- [x] **RES-CTRL-01**: Remove duplicate `flywheel-journey-strip` bars in `ResearchView.tsx` and `PublicationsView.tsx`, and compress idle `research-kb-preflight-panel` into an inline status pill inside the archetype row so reports render above the fold.
- [x] **RES-CTRL-02**: Collapse `PipelineTimeline` on completed sessions into a compact status summary while keeping live `PipelineTimeline` visible during running/incomplete states.
- [x] **RES-CTRL-03**: Render `data-testid="research-process-control-strip"` showing observed subqueries, citation source domains, dropped low-novelty count, `research-steer-input`, and `+1 Wave Refinement` (`research-steer-wave-btn`).
- [x] **RES-CTRL-04**: Render `data-testid="research-storage-control-bar"` with `research-storage-scope-toggle` (`Repo-Local KB` / `Global Library`) and `direct-index-kb-btn` (`Index Now`) calling `ingest_text` while keeping `save-to-kb-button` intact.

### Search & Storage Control (`KB-CTRL`) — Phase 503

- [x] **KB-CTRL-01**: Deduplicate `CorpusChip` buttons and inline `Top-K` number box from the top search bar in `MemoryView.tsx` (deferring to the left taxonomy tree and rotary dial cleanly).
- [x] **KB-CTRL-02**: Add `Expand` / `Collapse` (`expand-hit-{i}`) inline chunk reader toggle to `HitCard` in `MemoryView.tsx`.
- [x] **KB-CTRL-03**: Add `Research Topic` (`research-hit-{i}`) action on `HitCard` seeding `vox_research_seed_query` and navigating to `research` in 1 click.
- [x] **KB-CTRL-04**: Replace contradictory missing-graph empty state in `VoxSearchView.tsx` with a live Mounted Shard Architecture Summary, `TTL`/`LOD`/`SYNC GRAPH` tooltips, and 1-click `Search Shard in Memory`.

### Zero-ID-Typing Discovery & Scientia Navigation (`SCI-CTRL`) — Phase 504

- [x] **SCI-CTRL-01**: Add `claims-manifest-quick-picker` 1-click recent manuscript pills in `ClaimsView.tsx` that populate `pubId` and immediately load claims.
- [x] **SCI-CTRL-02**: Add `review-manifest-quick-picker` 1-click recent manuscript pills in `DiscoveryReview.tsx`.
- [x] **SCI-CTRL-03**: Make candidate, reply-window manifest, and stall rows in `ScientiaDashboard.tsx` clickable buttons dispatching `vox://scientia-open-studio`.
- [x] **SCI-CTRL-04**: Preserve existing `ClaimsView`, `DiscoveryReview`, and `ScientiaDashboard` unit test contracts cleanly.

### TypeScript & Vitest Verification (`CTRL-TEST`) — Phase 505

- [x] **CTRL-TEST-01**: Pass `tsc --noEmit` (`$ pnpm --dir crates/vox-gui/ui typecheck`).
- [x] **CTRL-TEST-02**: Update `PublicationsView.test.tsx` and add Vitest specs covering `research-process-control-strip`, `research-steer-wave-btn`, `direct-index-kb-btn`, `expand-hit-0`, and `research-hit-0`.
- [x] **CTRL-TEST-03**: Pass the 11-file Vitest surface, contract, and source-guard run.
- [x] **CTRL-TEST-04**: Verify zero theme/emoji/contrast violations via `themeContract.test.ts` and `zeroEmojiAndDingbat.test.ts`.

### Visual CV Flywheel Gate & Closeout (`CTRL-VERIFY`) — Phase 506

- [x] **CTRL-VERIFY-01**: Pass `VOX_CV_FLYWHEEL=1 CV_STAGE=after` Playwright `kb-research-cv-flywheel.spec.ts` (`16/16` captures, `0` Axe/font/height/overflow defects).
- [x] **CTRL-VERIFY-02**: Visually inspect refreshed `after` screenshots (`research-arcane-after.png`, `memory-arcane-after.png`, `scientia-arcane-after.png`, `scientia-claims-arcane-after.png`, `vox-search-arcane-after.png`, `publications-arcane-after.png`) to confirm above-the-fold layout and zero visual regressions.
- [x] **CTRL-VERIFY-03**: Mark Phases `501–506` and `24/24` requirements completed in `.planning/workstreams/research-click-tax-observability-and-control-v85/` and `.planning/PROJECT.md`.
- [x] **CTRL-VERIFY-04**: Present concrete click-tax, observability, and control audit summary with exact verified results.
