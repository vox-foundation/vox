# Requirements: Vox — Milestone v8.6 (`research-above-fold-synthesis-and-discovery-dedup-v86`)

**Defined:** 2026-11-18
**Workstream:** `.planning/workstreams/research-above-fold-synthesis-and-discovery-dedup-v86/`
**Core Value:** Operators immediately read synthesized research answers above the fold without scrolling past control strips, filter prior sessions by keyword, switch domain extractors in 1 click, inspect Discovery feeds without stacked double headers or manual polling buttons, scan dense Memory citations cleanly on single-row action bars, and read/edit Markdown and structured docs in a non-modal docked pane without blocking the rest of the Axis workspace.

## v8.6 Requirements (`Phases 507–513`, `28` requirements)

### Workstream & SSOT (`FOLD-SSOT`) — Phase 507

- [x] **FOLD-SSOT-01**: Update `.planning/PROJECT.md` to register Active Milestone `v8.6`.
- [x] **FOLD-SSOT-02**: Create `.planning/workstreams/research-above-fold-synthesis-and-discovery-dedup-v86/{ROADMAP.md,REQUIREMENTS.md,STATE.md,state.json}`.
- [x] **FOLD-SSOT-03**: Append Section 7 (`Milestone v8.6: Above-the-Fold Synthesis First, Discovery Dedup & Non-Modal Multi-Format Doc Editor/Parser`) to `docs/src/architecture/research-click-tax-observability-and-control-ssot-2026.md`.
- [x] **FOLD-SSOT-04**: Lint updated SSOT cleanly via `vox-doc-pipeline -- --lint-only`.

### Discovery & Activity Dedup (`DISC-DEDUP`) — Phase 508

- [x] **DISC-DEDUP-01**: Wrap preset sub-views inside `EmbeddedSurfaceProvider value={true}` in `DiscoverySurface.tsx` and replace hex color literals (`#F3E5AB`, `#1C140C`, `#4A3718`) with theme tokens.
- [x] **DISC-DEDUP-02**: Add 1-click flywheel jumps (`Open Research`, `Open Memory`, `Open Publications`) to `Archival Provenance Gate` in `DiscoverySurface.tsx`.
- [x] **DISC-DEDUP-03**: Render compact header in `ActivitySurface.tsx` when embedded inside `DiscoverySurface` while preserving `RomanSurfaceHeader` when standalone.
- [x] **DISC-DEDUP-04**: Replace manual `Refresh` button in `ActivitySurface.tsx` with passive `activity-live-sync-pill` and add `visibilitychange` auto-refresh listener.

### Above-the-Fold Synthesis, Session Search & Domain Mode (`RES-FOLD`) — Phase 509

- [x] **RES-FOLD-01**: Add `research-domain-mode-pill` cycling `general -> codegen -> scholarly -> shopping` inline with `research-kb-preflight-panel` inside `ResearchView.tsx`.
- [x] **RES-FOLD-02**: Add instant `research-history-filter-input` filtering prior sessions by query/status and display active topic in session header.
- [x] **RES-FOLD-03**: Move `HeadlineVerdictBanner` and `ResearchReportMarkdown` (`max-h-[165px]` in `overview`) above `research-completed-summary-pill`, `research-process-control-strip`, and `research-storage-control-bar`.
- [x] **RES-FOLD-04**: Preserve existing `ResearchView` diagnostics, DAG, and steering/storage contracts intact.

### Memory Compaction & Provenance (`MEM-COMPACT`) — Phase 510

- [x] **MEM-COMPACT-01**: Add provenance layer pill (`Memory Shard` / `Code Chunk` / `Codex Canon`) with explanatory tooltip on `HitCard` in `MemoryView.tsx`.
- [x] **MEM-COMPACT-02**: Tighten `HitCard` button padding/label (`Research`) so `Expand`, `Research`, `Open`, and `Pin` fit on one single unwrapped row.
- [x] **MEM-COMPACT-03**: Convert `INVARIANT` Codex Specification card into a compact collapsed-by-default `<details>` drawer preserving 1-line summary metrics.
- [x] **MEM-COMPACT-04**: Preserve zero-leak theme/emoji guards and existing `MemoryView.test.tsx` contracts.

### Non-Modal Docked DocViewer & Multi-Format Markdown Editor/Parser (`DOC-EDIT`) — Phase 511

- [x] **DOC-EDIT-01**: Allow common doc extensions (`md`, `mdx`, `markdown`, `txt`, `adoc`, `asciidoc`, `json`, `jsonl`, `yaml`, `yml`, `toml`, `csv`, `tsv`, `vox`) and add `write_doc_markdown` Tauri IPC + `voxTransport.writeDocMarkdown`.
- [x] **DOC-EDIT-02**: Convert `DocViewerDrawer.tsx` from a dimming modal (`drawer-backdrop`, `aria-modal="true"`, Tab focus trap) to a non-modal docked right pane (`aria-modal="false"`, `Width: Split / Wide / Full`, `Minimize / Expand`) so the rest of the Axis engine remains interactive.
- [x] **DOC-EDIT-03**: Upgrade `ResearchReportMarkdown.tsx` to parse YAML/TOML frontmatter, `<!-- kb:summary -->`, GFM pipe tables, GitHub/MDX/AsciiDoc alert callouts, task lists, AsciiDoc headings, JSON/JSONL, YAML/TOML key-value configs, CSV/TSV grids, and line-numbered source docs.
- [x] **DOC-EDIT-04**: Add `Read | Split | Edit` modes, quick-insert toolbar, heading TOC bar, live monospace editor, format switcher, Save/Revert controls, and collapsible facet drawer in `DocReader.tsx` and `DocReaderPanels.tsx`.

### TypeScript & Vitest Verification (`V86-TEST`) — Phase 512

- [x] **V86-TEST-01**: Add Vitest assertions covering `ResearchView`, `DocViewerDrawer`, and `DocReader`.
- [x] **V86-TEST-02**: Pass `$ tsc --noEmit` (`pnpm --dir crates/vox-gui/ui typecheck`).
- [x] **V86-TEST-03**: Pass the 19-file Vitest run across `Memory`, `Research`, `Discovery`, `Activity`, `Scientia`, `Publications`, `VoxGraph`, `DocReader`, `DocViewerDrawer`, `themeContract`, and `zeroEmojiAndDingbat`.
- [x] **V86-TEST-04**: Confirm zero new Playwright/DOM lint regressions.

### Visual CV Flywheel & Closeout (`V86-VERIFY`) — Phase 513

- [x] **V86-VERIFY-01**: Pass `VOX_CV_FLYWHEEL=1 CV_STAGE=after` Playwright `kb-research-cv-flywheel.spec.ts` (`16/16` captures, `0` Axe/font/height/overflow violations).
- [x] **V86-VERIFY-02**: Visually inspect refreshed `doc-reader-arcane-after.png`, `activity-arcane-after.png`, `research-arcane-after.png`, and `memory-arcane-after.png` via `read_file`.
- [x] **V86-VERIFY-03**: Mark Phases `507–513` and `28/28` requirements completed in `.planning/PROJECT.md` and workstream files.
- [x] **V86-VERIFY-04**: Report concrete before/after findings and verified test metrics cleanly.
