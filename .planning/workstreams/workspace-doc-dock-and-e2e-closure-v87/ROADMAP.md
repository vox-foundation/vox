# Roadmap: Milestone v8.7 (`workspace-doc-dock-and-e2e-closure-v87`) — Workspace-Wide Non-Modal Doc Dock Routing, Multi-Root Path Resolution & Research-to-Editor Seeding

**Workstream:** `.planning/workstreams/workspace-doc-dock-and-e2e-closure-v87/`
**Scope:** Connect the non-modal docked `DocViewerDrawer` and multi-format `DocReader` editor/parser end-to-end across `docs_index.rs` (`docs/src`, `docs/wiki`, repo root, `contracts`, `crates`, `~/.vox/library`), `useDocViewer` / `App.tsx` (`vox://open-doc` bus + `initialContent` seeding + `+ New Doc`), `MemoryView` (`HitCard` in-app doc open), `ResearchView` (`Edit in Doc Dock` synthesis seeding + clickable KB preflight chips), and `coldStartJourneys.test.tsx` (`Phases 514–519`, `24` requirements).

## Phase Checklist

- [x] **Phase 514: Workstream Initialization & SSOT Section 8** (`DOCK-SSOT-01..04`)
- [x] **Phase 515: Multi-Root Document Path Resolution in `docs_index.rs`** (`DOCK-PATH-01..04`)
- [x] **Phase 516: Universal `vox://open-doc` Bus, Unsaved Draft Seeding & `+ New Doc` Creation** (`DOCK-BUS-01..04`)
- [x] **Phase 517: In-App Doc Dock Integration in `MemoryView` & `ResearchView`** (`DOCK-FLOW-01..04`)
- [x] **Phase 518: `coldStartJourneys.test.tsx` Repair & Extended Vitest / TypeScript Suites** (`DOCK-TEST-01..04`)
- [x] **Phase 519: Rust, Guard, E2E Workbench, Dual-Theme CV Flywheel & Milestone Closeout** (`DOCK-VERIFY-01..04`)
