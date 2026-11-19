# Requirements: Vox — Milestone v8.7 (`workspace-doc-dock-and-e2e-closure-v87`)

**Defined:** 2026-11-18
**Workstream:** `.planning/workstreams/workspace-doc-dock-and-e2e-closure-v87/`
**Core Value:** Any Markdown, AsciiDoc, structured config, or source doc opened from Search, Memory, Research, or `+ New Doc` renders directly inside the non-modal docked workspace editor without leaving the active engine, double-prefixing paths, or blocking parallel workspace tools.

## v8.7 Requirements (`Phases 514–519`, `24` requirements)

### Workstream & SSOT (`DOCK-SSOT`) — Phase 514

- [x] **DOCK-SSOT-01**: Update `.planning/PROJECT.md` to register Active Milestone `v8.7`.
- [x] **DOCK-SSOT-02**: Create `.planning/workstreams/workspace-doc-dock-and-e2e-closure-v87/{ROADMAP.md,REQUIREMENTS.md,STATE.md,state.json}`.
- [x] **DOCK-SSOT-03**: Append Section 8 (`Milestone v8.7: Workspace-Wide Non-Modal Doc Dock Routing, Multi-Root Path Resolution & Research-to-Editor Seeding`) to `docs/src/architecture/research-click-tax-observability-and-control-ssot-2026.md`.
- [x] **DOCK-SSOT-04**: Lint updated SSOT cleanly via `vox-doc-pipeline -- --lint-only`.

### Multi-Root Path Resolution (`DOCK-PATH`) — Phase 515

- [x] **DOCK-PATH-01**: Preserve `resolve_path_under_root` while adding `resolve_workspace_doc_path` supporting `docs/src`, `docs/wiki`, `contracts`, `crates`, `scripts`, repo-root files, and `~/.vox/library`, without double `docs/src/docs/src/` prefixing.
- [x] **DOCK-PATH-02**: Widen `ALLOWED_DOC_EXTENSIONS` to include `rs`, `ts`, `tsx`, `sql`, reject `.env*` secrets (except `.env.example`), and block path traversal (`..`).
- [x] **DOCK-PATH-03**: Walk `docs/wiki` alongside `docs/src` in `vox_docs_index`.
- [x] **DOCK-PATH-04**: Add Rust unit tests in `docs_index.rs` verifying multi-root resolution, `.env` rejection, and traversal rejection.

### Universal `vox://open-doc` Bus & `+ New Doc` (`DOCK-BUS`) — Phase 516

- [x] **DOCK-BUS-01**: Extend `ActiveDoc` and `openDoc` in `useDocViewer.ts` with `initialContent` and `initialMode` (`read | split | edit`).
- [x] **DOCK-BUS-02**: Subscribe `App.tsx` to `vox://open-doc` forwarding `{ path, title, initialContent, initialMode }` to `openDocTab`.
- [x] **DOCK-BUS-03**: Forward `initialContent` and `initialMode` through `DocViewerDrawer.tsx` into `DocReader.tsx`, allowing unsaved synthesis/new-doc drafts to render in `Split`/`Edit` without read errors.
- [x] **DOCK-BUS-04**: Add `+ New Doc` (`doc-new-file-btn`) and editable path (`doc-path-input`) in `DocReader.tsx`.

### In-App Doc Dock Flow (`DOCK-FLOW`) — Phase 517

- [x] **DOCK-FLOW-01**: Route `MemoryView.tsx` `openHit` file locators (supported doc/config/code extensions) through `vox://open-doc` instead of escaping to external OS launch.
- [x] **DOCK-FLOW-02**: Add `Edit in Doc Dock` (`open-synthesis-in-doc-dock-btn`) in `ResearchView.tsx` seeding valid frontmatter + `kb:summary` + synthesized report + citations into `Split` editor mode.
- [x] **DOCK-FLOW-03**: Make matched KB preflight hits in `ResearchView.tsx` clickable buttons dispatching `vox://open-doc`.
- [x] **DOCK-FLOW-04**: Maintain zero hardcoded hex colors, zero emojis/dingbats, and uniform toolbar heights.

### Test & Guard Closure (`DOCK-TEST`) — Phase 518

- [x] **DOCK-TEST-01**: Repair stale fixtures in `coldStartJourneys.test.tsx` so all `3/3` journeys pass against live component contracts.
- [x] **DOCK-TEST-02**: Add Vitest specs in `useDocViewer.test.ts`, `DocReader.test.tsx`, `MemoryView.test.tsx`, and `ResearchView.test.tsx` covering `vox://open-doc`, `initialContent`, `doc-new-file-btn`, and `open-synthesis-in-doc-dock-btn`.
- [x] **DOCK-TEST-03**: Pass `tsc --noEmit`.
- [x] **DOCK-TEST-04**: Pass `cargo test -p vox-gui docs_index`, the 31-suite Vitest + guards run, and `v79CvFlywheelConvergence.test.ts`.

### E2E, CV Flywheel & Closeout (`DOCK-VERIFY`) — Phase 519

- [x] **DOCK-VERIFY-01**: Pass Playwright `e2e/workbench-tabs.spec.ts`.
- [x] **DOCK-VERIFY-02**: Pass Playwright `VOX_CV_FLYWHEEL=1 CV_STAGE=after` `e2e/review/kb-research-cv-flywheel.spec.ts` (`16/16` captures, `0` Axe/font/height violations).
- [x] **DOCK-VERIFY-03**: Inspect `ledger-after.json` and refreshed `research-arcane-after.png` & `doc-reader-arcane-after.png`.
- [x] **DOCK-VERIFY-04**: Close out `.planning/PROJECT.md` and workstream files for Milestone `v8.7`.
