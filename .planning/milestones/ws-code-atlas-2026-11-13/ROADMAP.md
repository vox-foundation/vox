# Roadmap: Vox

## Milestones

- ✅ **v1.0 Architectural Housekeeping** — Phases 1–6 (shipped 2026-10-01)
- 🚧 **v1.1 Research Trial Flywheel** — Phases 7–11 (parallel workstream `research-trial-flywheel`)
- 🚧 **v1.2 Unified Autonomy Control** — Phases 12–19 (parallel workstream `autonomy-ux`)
- 🚧 **v1.3 Self-Maintaining Public Docs** — Phases 20–24 (parallel workstream `docs-freshness`)
- 🚧 **v1.4 Code Atlas** — Phases 25–31 (this workstream, `code-atlas`)

## Overview

v1.4 turns the trapped Graphify corpus into a navigable visual language: crates as regions, files as containers, symbols on drill-in — including first-class `.vox` kinds — on `main` or any listed worktree, docked beside Chat so the map updates as the conversation names and changes code. It starts with a typed, budgeted, identity-stamped subgraph (Phase 25) so the canvas never paints untyped edges, Leiden clusters, or a stale SHA as live. A viewport-culled xyflow atlas then ships without Chat chrome (26). Branch/worktree compare and vs-`main` coloring land later, once identity exists, so the overlay is honest (27). Only then does Code Atlas register as an opt-in same-webview Chat dock (28). A real `code_touch` event ships before any pulse (29). Click-to-`@` and Vox glyphs close the Chat↔graph loop (30). Visual steppers on synthetic fixtures prove empty / loaded / search / docked / vs-main / pulse without materializing the 47k cache (31). No new crate, no `vox-gui` → `vox-graph-reader` edge, no OS pop-out.

## Phases

**Phase Numbering:**

- Integer phases (25, 26, …): Planned milestone work
- Decimal phases (25.1, 25.2): Urgent insertions (marked with INSERTED)

### 🚧 v1.4 Code Atlas

- [ ] **Phase 25: Graph Data Floor** - Typed edges, crate→file→symbol containment, budgeted LOD + identity, MCP/CLI atlas query, first-class `.vox` kinds, extractor version bump
- [ ] **Phase 26: Viewport-Culled Atlas Canvas** - Semantic-zoom Code Atlas on `@xyflow/react` 12: search-to-focus, typed-edge legend, keyboard + a11y table, crate-tile minimap, subgraph-only store
- [ ] **Phase 27: Branch / Worktree Compare** - Atlas identity picker for `main` and listed worktrees, vs-`main` added/removed/changed overlay on stable `main` regions, honest stale tone
- [ ] **Phase 28: Dockable Widget Beside Chat** - Opt-in `codeatlas` dockview 6.6.1 panel, same-webview float, closed stays closed, non-zero pane before React Flow, Agent Flow stays separate
- [ ] **Phase 29: Live Sync** - Typed `code_touch` from writes and search hits; pulse + optional follow-camera; “overlay unavailable” when the event is missing
- [ ] **Phase 30: Click-to-@ and Vox-Native Chrome** - Node click inserts path-shaped `@` via Loquela; legend and glyphs distinguish Vox kinds from Rust/TS
- [ ] **Phase 31: Visual Proof** - Playwright steppers for empty / loaded / search-focus / docked / vs-main / pulse on synthetic fixtures; review-bundle screenshots; no 47k DOM

## Phase Details

### Phase 25: Graph Data Floor

**Goal**: Atlas consumers receive a typed, budgeted, identity-stamped subgraph — including first-class Vox declaration kinds — and never the raw untyped corpus.
**Depends on**: Nothing (first v1.4 phase; independent of v1.1–v1.3)
**Requirements**: DATA-01, DATA-02, DATA-03, DATA-04, DATA-05, DATA-06, VOX-01
**Research flag**: Standard patterns — skip phase research (in-tree seams in `ast.rs`, `lens`, `crate_model`, MCP `graph_tools` already read).
**Success Criteria** (what must be TRUE):

  1. Every edge served to the atlas carries a closed `kind` (`calls`, `imports`, `implements`, `tests`, `crate_dep`, `contains`, `mentions`); an untyped `{source,target,confidence}` payload is not served.
  2. The atlas hierarchy is Cargo crate → module/file → symbol from path/`id` containment; Leiden community IDs are not atlas regions (they remain CLI crate-map blast-radius only).
  3. An `AtlasQuery` returns at most the hard cap of nodes (default 400) plus identity `{corpus_id, git_sha, ref_name, worktree_id, built_at, lens}`; the GUI never receives the full `graph.json`.
  4. MCP `vox_search_atlas` (and optional `vox graph atlas`) serves that LOD subgraph; a worktree rebuild uses that tree as `source_dir` and the existing graph lock.
  5. `.vox` files are walked and extracted; declaration-head kinds (`component`, `table`, `query`, `mutation`, `workflow`, `activity`, `actor`, `fn`, …) are first-class node kinds, not collapsed to `fn`. After a kind or containment change, an old cache is refused (extractor version bump).

**Plans**: TBD

### Phase 26: Viewport-Culled Atlas Canvas

**Goal**: Users can navigate a Sourcetrail/CodeSee-style code map of the current LOD subgraph without Chat chrome.
**Depends on**: Phase 25
**Requirements**: ATLAS-01, ATLAS-02, ATLAS-03, ATLAS-04, ATLAS-05, ATLAS-06
**Research flag**: Standard patterns — skip phase research (xyflow 12 + SubAgentGraph `toFlow` + react-query status hook). Re-open only if crate-level visual QA trips the elkjs escape hatch.
**Success Criteria** (what must be TRUE):

  1. A user opens a Code Atlas canvas and sees crates as regions, files as containers, and symbols on drill-in; changing zoom changes what is drawn, not only scale.
  2. A user searches a symbol or path and the camera focuses that node plus a 1-hop neighborhood.
  3. Typed edges are readable by color and dash/marker plus a legend — meaning is not color-only.
  4. A user can pan, zoom, and activate nodes with keyboard as well as pointer, and a textual table/tree of the **loaded** subgraph is available as the a11y equivalent of the canvas.
  5. The minimap shows crate-level tiles only, zoom/fit controls work, layout is deterministic packed regions from `crate_model` (stable across rebuilds of the same crate set), and the React Flow store holds only the loaded subgraph.

**Plans**: TBD
**UI hint**: yes

### Phase 27: Branch / Worktree Compare

**Goal**: Users can inspect `main` or a listed worktree as a graph identity and see an honest vs-`main` overlay that does not jump the map.
**Depends on**: Phase 26 (canvas + identity fields from Phase 25)
**Requirements**: BRANCH-01, BRANCH-02, BRANCH-03
**Research flag**: Needs phase research — worktree-as-identity vs commit-keyed corpus; `ls_tree` overlay vs opt-in `source_dir` rebuild; how to color removed nodes without a second full graph.
**Success Criteria** (what must be TRUE):

  1. A user picks `main` (or the default branch) or a listed worktree from an atlas-specific picker — not the Repository isolation-strategy dropdown — and the atlas loads that graph identity.
  2. A user sees a vs-`main` overlay coloring nodes/edges `added` / `removed` / `changed` for the selected worktree or branch; crate regions stay on `main` so the map does not jump.
  3. When the cached SHA does not match the selected ref HEAD, the atlas shows an honest stale tone and does not claim the overlay is that worktree.

**Plans**: TBD
**UI hint**: yes

### Phase 28: Dockable Widget Beside Chat

**Goal**: Users can keep Code Atlas open beside the Chat transcript as an opt-in same-webview widget.
**Depends on**: Phase 26 (sized-parent canvas story; Phase 27 may proceed in parallel after 26)
**Requirements**: DOCK-01, DOCK-02, DOCK-03, DOCK-04
**Research flag**: Standard patterns — skip phase research (ChatSurface opt-in panels, `closedPanelIds`, DockWorkspaceShell already documented and tested).
**Success Criteria** (what must be TRUE):

  1. A user opens Code Atlas as an opt-in Chat dockview panel (`codeatlas`) beside the transcript on pinned dockview 6.6.1.
  2. A user can float the atlas in the same webview (`addFloatingGroup`) while Chat stays open; closing it stays closed (`closedPanelIds`) and it does not resurrect on layout restore.
  3. React Flow does not mount until the pane has a non-zero pixel height; Search Index / `VoxGraphStatusPanel` remains freshness chrome and may offer “Open Code Atlas”.
  4. Agent Flow remains a separate surface; there is no OS-level `WebviewWindow` / `addPopoutGroup` in this milestone.

**Plans**: TBD
**UI hint**: yes

### Phase 29: Live Sync

**Goal**: Atlas nodes react only to real code-touch events from the conversation, never to stream ticks.
**Depends on**: Phase 28 (same-webview dock so pulses reach the open atlas)
**Requirements**: LIVE-01, LIVE-02
**Research flag**: Needs phase research — exact `code_touch` producers (`vox_write_file`, edits, search hits) and fail-closed “overlay unavailable” when absent.
**Success Criteria** (what must be TRUE):

  1. A typed `code_touch` turn event exists with `path` and optional `symbol`, produced from agent writes and search hits — not inferred from `isStreaming` or status polls.
  2. When `code_touch` arrives, matching on-page atlas nodes pulse and an optional follow-camera can track them.
  3. If the event is unavailable, the atlas shows “overlay unavailable” rather than a fake pulse.

**Plans**: TBD
**UI hint**: yes

### Phase 30: Click-to-@ and Vox-Native Chrome

**Goal**: Users can drop atlas nodes into Chat as path-shaped `@` mentions, and Vox kinds are visually first-class.
**Depends on**: Phase 28 (same React tree as Chat; extractor kinds from Phase 25)
**Requirements**: MENTION-01, VOX-02
**Research flag**: Standard patterns — skip phase research (Loquela `insertAtFile` / `composerMentions`; kinds already in the Phase 25 payload).
**Success Criteria** (what must be TRUE):

  1. Clicking a file node inserts `@path` into the Chat composer via the shared Loquela mention helper.
  2. Clicking a symbol node inserts `@path` with chip `ref` = graph id; composer text stays path-shaped — no raw `path::symbol` ids and no new `@graph:` grammar.
  3. The atlas legend and node chrome distinguish Vox kinds (`component`, `table`, `workflow`, `actor`, …) from Rust/TS kinds (`struct`, `trait`, `method`, …) with glyphs as well as color.

**Plans**: TBD
**UI hint**: yes

### Phase 31: Visual Proof

**Goal**: A reviewer can see every atlas state on synthetic fixtures and trust that the 47k corpus was never stuffed into the canvas.
**Depends on**: Phases 26–30 (all visual states exist to prove)
**Requirements**: PROOF-01
**Research flag**: Standard patterns — skip phase research (`installTauriMock` + review-bundle steppers are the GUI visual-verification invariant).
**Success Criteria** (what must be TRUE):

  1. A reviewer can step empty and loaded atlas states on synthetic fixtures (tiny + ~1k shaped) and find review-bundle screenshots; the live 47k cache is never the fixture.
  2. Search-focus and docked-beside-chat steppers produce screenshots; the React Flow store holds only the loaded subgraph (`nodes.length` ≪ corpus `node_count`).
  3. vs-`main` overlay and mocked `code_touch` pulse steppers produce screenshots; streaming without a touch event shows no fake pulse.
  4. Closing the panel and refreshing does not resurrect it; preview / unavailable backend shows an honest empty state.

**Plans**: TBD
**UI hint**: yes

## Parallelism

- 25 → 26 is the critical path (typed LOD before any canvas).
- 27 (vs-`main`) and 28 (dock) both depend on 26; 27 is a later phase by design (do not fake the overlay) and may run after or beside 28 once 26 is true.
- 29 depends on 28 (same-webview dock) and on a real `code_touch` contract — event before CSS.
- 30 depends on 28 (mentions) and 25 (Vox kinds); can overlap 29.
- 31 closes after 26–30 so every stepper state exists.

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 25. Graph Data Floor | 0/TBD | Not started | - |
| 26. Viewport-Culled Atlas Canvas | 0/TBD | Not started | - |
| 27. Branch / Worktree Compare | 0/TBD | Not started | - |
| 28. Dockable Widget Beside Chat | 0/TBD | Not started | - |
| 29. Live Sync | 0/TBD | Not started | - |
| 30. Click-to-@ and Vox-Native Chrome | 0/TBD | Not started | - |
| 31. Visual Proof | 0/TBD | Not started | - |
