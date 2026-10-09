# Requirements — v1.4 Code Atlas (workstream `code-atlas`)

**Goal:** Represent the Graphify codebase graph — crates → modules → files → symbols, including `.vox` kinds — as a navigable visual language on any branch or worktree, docked beside Chat so the atlas updates as the conversation names and changes code.

Brief: `CONTEXT.md`. Research: `research/SUMMARY.md` (+ STACK, FEATURES, ARCHITECTURE, PITFALLS).

## Decisions (user, 2026-10-09)

- D1: Separate GSD workstream `code-atlas`, milestone **v1.4**, phases **25+** (continue after docs-freshness Phase 24).
- D2: Target is the **codebase graph**, not Agent Flow. Agent Flow stays a separate surface.
- D3: All seven capabilities in this milestone: data floor, navigable atlas, branch/worktree compare, dockable widget, live sync, click-to-`@`, Vox-native kinds.
- D4: Live pulses and vs-`main` coloring are **later phases in this milestone**, not a future milestone. Do not fake either.
- D5: Stay on `@xyflow/react` 12 + dockview 6.6.1. No second graph library unless crate-level visual QA fails (elkjs escape hatch only).
- D6: No new crate. No `vox-gui` → `vox-graph-reader` edge (GUI talks MCP). No `vox-graph-reader` → `vox-compiler` edge.
- D7: Floating means **same-webview** `addFloatingGroup`, not a Tauri `WebviewWindow` pop-out.

## v1.4 Requirements

### Graph data floor (DATA)

- [ ] **DATA-01**: Every extracted graph edge carries a closed `kind` (`calls`, `imports`, `implements`, `tests`, `crate_dep`, `contains`, `mentions`). Untyped `{source,target,confidence}` is not served to the atlas.
- [ ] **DATA-02**: The atlas hierarchy is Cargo crate → module/file → symbol, derived from path/`id` containment — not Leiden community IDs. Leiden remains available to CLI crate-map blast-radius only.
- [ ] **DATA-03**: A budgeted LOD query (`AtlasQuery` → `AtlasSubgraph`) returns at most a hard cap of nodes (default 400) for a zoom level, focus, and viewport. The GUI never receives the full `graph.json`.
- [ ] **DATA-04**: Every atlas payload includes identity `{corpus_id, git_sha, ref_name, worktree_id, built_at, lens}` so the UI can refuse to label a stale SHA as the live or selected ref.
- [ ] **DATA-05**: MCP tool `vox_search_atlas` (and optional `vox graph atlas`) serves the LOD subgraph. Rebuilding a worktree graph uses that tree as `source_dir` and the existing graph lock.
- [ ] **DATA-06**: Extractor version bumps when edge kinds or containment change, so stale caches fail closed instead of painting the old untyped graph.

### Navigable atlas (ATLAS)

- [ ] **ATLAS-01**: User can open a Code Atlas canvas that draws the current LOD subgraph with semantic zoom: crates as regions, files as containers, symbols on drill-in. Zoom changes what is drawn, not only scale.
- [ ] **ATLAS-02**: User can search a symbol or path and the camera focuses it plus a 1-hop neighborhood (`vox graph query` → `setCenter`).
- [ ] **ATLAS-03**: Typed edges have a visible language (color + dash/marker + legend). Meaning is not color-only (WCAG 1.4.1).
- [ ] **ATLAS-04**: User can pan, zoom, and activate nodes with keyboard as well as pointer. A textual table/tree of the **loaded** subgraph is available as the a11y equivalent of the canvas.
- [ ] **ATLAS-05**: Minimap shows crate-level tiles only. Controls (zoom/fit) are present. Layout is deterministic packed regions from `crate_model`, stable across rebuilds of the same crate set.
- [ ] **ATLAS-06**: The React Flow store holds only the loaded subgraph. `onlyRenderVisibleElements` is a second cull, not the scale strategy.

### Branch / worktree (BRANCH)

- [ ] **BRANCH-01**: User can pick `main` (or the default branch) or a listed worktree as the atlas identity. The picker is not the Repository isolation-strategy dropdown.
- [ ] **BRANCH-02**: User can see a vs-`main` overlay coloring nodes/edges `added` / `removed` / `changed` for the selected worktree or branch. Layout stays on `main` crate regions so the map does not jump.
- [ ] **BRANCH-03**: When the cached SHA does not match the selected ref HEAD, the atlas shows an honest stale tone and does not claim the overlay is that worktree.

### Dockable widget (DOCK)

- [ ] **DOCK-01**: User can open Code Atlas as an opt-in Chat dockview panel (`codeatlas`) beside the transcript, using pinned dockview 6.6.1.
- [ ] **DOCK-02**: User can float the atlas in the same webview (`addFloatingGroup`) while Chat stays open. Closing it stays closed (`closedPanelIds`); it does not resurrect on layout restore.
- [ ] **DOCK-03**: React Flow does not mount until the pane has a non-zero pixel height. Search Index / `VoxGraphStatusPanel` remains freshness chrome and may offer “Open Code Atlas”.
- [ ] **DOCK-04**: Agent Flow is not merged into Code Atlas. OS-level `WebviewWindow` / `addPopoutGroup` is out of this milestone.

### Live sync (LIVE)

- [ ] **LIVE-01**: A typed `code_touch` turn event exists with `path` and optional `symbol`, produced from agent writes and search hits — not inferred from `isStreaming` or status polls.
- [ ] **LIVE-02**: When `code_touch` arrives, matching atlas nodes pulse (and optional follow-camera can track them). If the event is unavailable, the atlas shows “overlay unavailable” rather than a fake pulse.

### Click-to-mention (MENTION)

- [ ] **MENTION-01**: Clicking a file node inserts `@path` into the Chat composer via the shared Loquela mention helper. Symbol nodes insert `@path` with chip `ref` = graph id. Composer text stays path-shaped — no raw `path::symbol` ids and no new `@graph:` grammar.

### Vox-native kinds (VOX)

- [ ] **VOX-01**: `.vox` files are walked and extracted. Declaration-head kinds (`component`, `table`, `query`, `mutation`, `workflow`, `activity`, `actor`, `fn`, …) are first-class node kinds, not collapsed to `fn`.
- [ ] **VOX-02**: The atlas legend and node chrome distinguish Vox kinds from Rust/TS kinds (`struct`, `trait`, `method`, …) with glyphs as well as color.

### Visual proof (PROOF)

- [ ] **PROOF-01**: Playwright visual-stepper specs cover atlas empty / loaded / search-focus / docked-beside-chat / vs-main overlay / pulse (mocked `code_touch`) and write screenshots to the review bundle. Fixtures are synthetic (tiny + ~1k shaped), never the live 47k cache.

## Out of Scope

- Merging Agent Flow / SubAgentGraph into Code Atlas — different domain (agents vs code).
- Drawing the full 47k-node corpus or adding sigma / cytoscape / ngraph / react-force-graph / WebGL.
- bumping dockview 6.6.1 → 8.x (ChatSurface rewrite).
- OS-level pop-out window (`WebviewWindow`).
- New `@graph:` mention syntax.
- SCIP / cross-repo go-to-definition.
- Gource time-lapse, CodeSee tours, CodeScene health maps as the home view.
- Coverage / test-target lens as default paint (CLI already exists).
- New crate or user-unauthorized crate-edge `exceptions`.
- `vox-graph-reader` importing `vox-compiler`.

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| DATA-01 | — | Pending |
| DATA-02 | — | Pending |
| DATA-03 | — | Pending |
| DATA-04 | — | Pending |
| DATA-05 | — | Pending |
| DATA-06 | — | Pending |
| ATLAS-01 | — | Pending |
| ATLAS-02 | — | Pending |
| ATLAS-03 | — | Pending |
| ATLAS-04 | — | Pending |
| ATLAS-05 | — | Pending |
| ATLAS-06 | — | Pending |
| BRANCH-01 | — | Pending |
| BRANCH-02 | — | Pending |
| BRANCH-03 | — | Pending |
| DOCK-01 | — | Pending |
| DOCK-02 | — | Pending |
| DOCK-03 | — | Pending |
| DOCK-04 | — | Pending |
| LIVE-01 | — | Pending |
| LIVE-02 | — | Pending |
| MENTION-01 | — | Pending |
| VOX-01 | — | Pending |
| VOX-02 | — | Pending |
| PROOF-01 | — | Pending |
