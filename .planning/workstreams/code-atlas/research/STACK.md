# Stack Research

**Domain:** Navigable visual codebase graph (Code Atlas) beside Chat
**Project:** Vox v1.4 `code-atlas`
**Researched:** 2026-10-09
**Confidence:** HIGH for versions and the renderer decision; MEDIUM for the elkjs deferral and Tauri popout fallback

> Additions only. Do not re-litigate `vox-graph-reader`, graphify corpora, Tauri 2, dockview Chat panels, or the existing `@xyflow/react` install. Those are validated.

## Bottom line

**Stay on viewport-culled `@xyflow/react` 12 subgraphs.** Do not add sigma, graphology, ngraph, cytoscape, or elkjs/dagre in this milestone.

A 47k-node / 50k-edge corpus is not a rendering problem. It is a **projection** problem. Sourcetrail, CodeSee/GitKraken Maps, CodeNib, and every code-graph that stays usable never put the full symbol graph on a canvas. They project a containment tree (crate → file → symbol) plus a typed reference overlay, then draw only the current zoom / focus / neighborhood. That projection already has a home: `vox-graph-reader` (`crate_model`, `lens::collapse_to_modules`, `overlay`, `history/git`, `compare`).

`@xyflow/react` is already installed and used by `SubAgentGraph`. dockview 6.6.1 already docks "Search Index" beside Chat. React 19.2.8 is a supported xyflow peer. Adding a second graph toolkit would fork the visual language, lose React custom nodes (Vox kinds, live pulse, click-to-`@`), and fight Playwright.

---

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `@xyflow/react` | **12.12.0** latest; repo has `^12.11.6` | Atlas canvas: pan/zoom, `parentId` hierarchy, custom `nodeTypes`/`edgeTypes`, MiniMap, Controls | Official 12 API includes `onlyRenderVisibleElements`, nested `parentId` + `extent: 'parent'`, `MiniMap`/`Controls`. Peer `react >= 17` — compatible with React 19.2.8. Already the SubAgentGraph renderer. Use X because the atlas needs **React node chrome**, not a WebGL dot cloud. |
| `dockview` | **6.6.1** (pinned; do not bump) | Dock / float the atlas beside Chat | ChatSurface is written against dockview-core 6.6.1 constraint types (`AddPanelOptions` + `setConstraints`). Latest npm is **8.4.1** (2026-10-05) — a major-version landmine, not an atlas prerequisite. 6.x already has `addFloatingGroup` / `addPopoutGroup`. Add a panel id, do not upgrade the layout manager. |
| React / react-dom | **19.2.8** (already `^19.2.8`) | GUI runtime | No change. xyflow 12 and dockview 6 both run on it. |
| `vox-graph-reader` | in-tree | Subgraph projection, typed edges, hierarchy, branch/worktree diff | The 47k graph stays in Rust. The GUI receives a **budgeted projection** (≤ a few hundred nodes), never `graph.json`. Extend `crate_model`, `lens`, `overlay`, `history/git.rs`, `compare.rs`. Do not add a JS graph model. |
| `@tanstack/react-query` | **5.102.8** (already) | Fetch / cache atlas projections | `useVoxGraphStatus` already polls `vox_search_status` this way. Add `useAtlasSubgraph({ corpus, rev, zoom, focus, viewport })` on the same transport. |
| `zustand` | **5.0.15** (already) | Live "agent is touching X" highlight set | xyflow docs: do not subscribe overlay chrome to the full `nodes` array. Keep touched-id set in a tiny store. Do not add Redux/Jotai. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `@xyflow/react` built-ins (`MiniMap`, `Controls`, `Background`, `useReactFlow`, `useOnViewportChange`) | same 12.x | Search-to-focus (`setCenter` / `fitView`), minimap, zoom-driven projection refresh | Always. Zero new deps. |
| `@tauri-apps/api` `WebviewWindow` | **2.11.1** (already `^2.11.1`) | True OS pop-out if dockview `addPopoutGroup` (`window.open` + `/popout.html`) fails inside Tauri | Only if floating-in-window is required **and** `addPopoutGroup` returns false. Prefer `addFloatingGroup` first. |
| `elkjs` | **0.12.0** (npm current; **do not add now**) | Layered compound layout for crate-level DAGs (~100–300 nodes) | Escape hatch **after** visual QA of deterministic packed regions fails. Worker-only, cached by graph hash. Never run on 47k nodes. |
| Playwright + existing `installTauriMock` | repo `^1.62.1` | GUI visual stepper + review-bundle screenshots | Required by the GUI visual invariant. Mock `@xyflow/react` the way `App.test.tsx` / `SubAgentGraph.test.tsx` already do. |

### Existing backend seams to extend (not replace)

| Seam | Path | Atlas job |
|------|------|-----------|
| Typed edges | graphify `graph.json` `links[]` today: `source` / `target` / `confidence` only | Add `kind` (`contains`, `calls`, `imports`, `implements`, `tests`, `crate_dep`, …). Reader already accepts `links` or `edges`. |
| Hierarchy | `crate_model.rs`, `lens::collapse_to_modules` | Real containment tree: crate → module/file → symbol. Stop using Leiden communities as the atlas clustering (20,653 communities is the bug). Keep Leiden for CLI crate-map only. |
| Subgraph projection | **new** `pub fn` on `GraphifyReader` | Inputs: rev, zoom level, focus id, optional viewport AABB, `max_nodes`. Output: nodes + typed edges + parent ids. This is the API the canvas calls. |
| Branch / worktree | `history/git.rs` (`rev_parse`, `file_deltas`, `ls_tree`), `compare.rs`, `snapshot.rs` | Graphs stay commit-keyed. Diff overlay = node/edge data (`added` / `removed` / `changed`) vs `main`, not a second renderer. No `isomorphic-git`. |
| Overlay / live highlight | `overlay.rs` + new Tauri/MCP event | Reuse overlay targeting for tests; stream "agent touched file/symbol" into the zustand set. |
| Vox node kinds | `ast.rs` / `ast_ts.rs` + graphify CLI | `component`, `table`, `workflow`, `actor`, … become `nodeTypes` keys. |
| Click → `@` | existing Loquela composer `@` for agents/files | Call the composer insert path. Do not add a mention library or a second syntax. |
| Dock placement | `ChatSurface.tsx` `OPT_IN_PANEL_IDS` / `VoxGraphDockPanel` | New opt-in panel (or replace the freshness-card body of `voxgraph`). Same dockview instance as Chat. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| Vitest + Testing Library | Projection → xyflow `Node`/`Edge` unit tests | Keep `toFlow`-style pure helpers (see `SubAgentGraph.toFlow`). |
| Playwright stepper | Dock, float, zoom, click-to-`@`, live pulse | Mock IPC; capture `review-bundle/latest/`. |
| `vox ci crate-edges` | Any new Rust crate edge | Prefer extending `vox-graph-reader`. `exceptions` are user-authorized-only. |
| Context7 / npm | Re-verify xyflow / dockview on upgrade | xyflow latest checked 2026-10-09: **12.12.0**. dockview latest: **8.4.1** (ignore for this milestone). |

---

## Installation

```bash
# Do not add a renderer, layout engine, or WebGL graph lib for v1.4.
# Optional patch-level bump inside the existing range (already ^12.11.6):
pnpm --dir crates/vox-gui/ui update @xyflow/react

# Escape hatch only after crate-level layout QA fails (not now):
# pnpm --dir crates/vox-gui/ui add elkjs@0.12.0
```

No new Rust crates. No new Cargo features. Graph work stays in `vox-graph-reader` + graphify CLI + one Tauri/MCP read + one event.

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| Viewport-culled xyflow **subgraphs** (project in Rust, draw ≤ few hundred nodes) | Dump 47k nodes into ReactFlow + `onlyRenderVisibleElements` | Never. Visibility culling still keeps every node in the Zustand store and still diffs them. xyflow's own performance guide says collapse large trees and memoize; it does not claim 50k-node React graphs. |
| `@xyflow/react` 12 custom nodes | `sigma` 4.0.0 + `graphology` 0.26.0 | If a future "galaxy" far-zoom must paint tens of thousands of unlabelled dots at 60 fps **and** crate/file collapse has failed. Loses React nodeTypes, Playwright-friendly DOM, and click-to-`@` chrome. |
| `@xyflow/react` 12 | `cytoscape` 3.34.3 | Cytoscape is a second scene graph with its own stylesheet language. CodeNib uses it for a file overview; we already have xyflow and need React nodes. |
| `@xyflow/react` 12 | `ngraph` / `ngraph.pixel` | Low-level, no React integration, no typed custom nodes. |
| `@xyflow/react` 12 | `react-force-graph` / Three / Pixi / WebGPU (CodeGraphy) | Right tool for Obsidian-style global force hairballs. Wrong tool for a typed, hierarchical, chat-synced atlas. Heavier Playwright/WebGL surface. |
| Deterministic packed regions from `crate_model` (stable positions across rebuilds) | `elkjs` 0.12.0 layered/compound | Use elkjs only for the crate-level DAG if packed regions look tangled. ELK mutates positions every run unless seeded; a code atlas wants **geographic stability** (crates stay put). 1.5 MB + worker. Never on the full corpus. |
| Deterministic packed regions | `@dagrejs/dagre` 3.1.1 | Dagre has no real compound/parent layout. Hierarchy is the product. Skip. |
| dockview **6.6.1** + `addFloatingGroup` | dockview **8.4.1** / `dockview-react` | Only in a dedicated layout-manager milestone. ChatSurface comments encode 6.6.1 constraint types. 6→8 is unrelated risk. |
| dockview `addPopoutGroup` | Tauri `WebviewWindow` | If `window.open` is blocked or `/popout.html` is missing in the Tauri bundle. Floating-in-app is the default. |
| Rust `history/git.rs` | `isomorphic-git` / JS git | Violates "git stays in Rust" and the no-new-JS-git policy. Worktree/branch picker reads existing repo/worktree commands. |
| zustand touched-id set + Tauri event | WebSocket / new event bus | Orchestrator already reaches the GUI via Tauri/MCP. Add one event name, not a transport. |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `sigma` 4 + `graphology` | WebGL point/line renderer. No first-class React custom nodes. Second visual language vs SubAgentGraph. | xyflow subgraphs + Rust projection |
| `cytoscape` (+ cose-bilkent / fcose) | Full second toolkit; CSS-like style, not React 19 node components. | xyflow `nodeTypes` / `edgeTypes` |
| `ngraph`, `vis-network`, `@antv/g6` | Extra graph runtime for a problem we will not solve by drawing 47k dots. | Projection budget |
| `react-force-graph-2d`, Pixi, Three, WebGPU graph renderers | Built for global force-directed clouds (Obsidian, Gource). Force layout of a monorepo is the hairball. Unstable positions, hard to test. | Containment tree + DOI / neighborhood |
| `elkjs` in v1 | Not needed if the canvas never holds the full graph. Adds worker + bundle + layout flicker. | Packed regions from `crate_model`; revisit if crate-level edges are unreadable |
| `@dagrejs/dagre` | No compound graphs; worse than ELK for crate⊃file. | Same as elkjs: don't, then ELK if anything |
| `d3-force` in the UI thread | Obsidian-plugin lesson: physics on the main thread melts large graphs; even worker force is the wrong metaphor for a code map. | Deterministic hierarchy layout |
| dockview 8 / `dockview-react` split package | Chat is on `dockview@6.6.1`. A bump is a ChatSurface rewrite, not an atlas feature. | New opt-in panel on 6.6.1 |
| `isomorphic-git`, JS worktree libs | Duplicate of `history/git.rs`; secrets/process policy. | Rust git + existing Repository picker data |
| New mention / slash-command library | Composer already `@`s agents/files. | Call the existing insert path from `onNodeClick` |
| New state library (Redux, Jotai, Valtio) | zustand 5 and react-query 5 are already in `package.json`. xyflow bundles its **own** zustand 4 — do not hoist or unify. | Tiny `useAtlasLive` slice |
| New Rust crate for "atlas types" | Layer-edge ratchet; <50-line shapes can live next to the reader. | Extend `vox-graph-reader` |
| Leiden-as-atlas-clusters | Measured: 20,653 communities ≈ 1 cluster / 2.3 symbols. Meaningless. | Path/crate containment |
| Drawing Agent Flow as the atlas | Agent Flow is a fixed SVG ring. Out of scope. | Code graph only |

---

## How large-graph products actually scale

Opinionated takeaway: **nobody successful draws 47k labelled, clickable, typed nodes.** They change what "the graph" means at each zoom.

| Product | Renderer | Scale strategy | Steal | Do not steal |
|---------|----------|----------------|-------|--------------|
| **Sourcetrail** | Custom Qt/OpenGL | Active symbol + N-hop neighborhood; graph + code split | Neighborhood / DOI as the default symbol view | Custom OpenGL engine |
| **CodeSee → GitKraken Maps** | Hierarchical map | Services/files as regions; drill-in, not a force hairball | Region-as-crate, file-as-container | Separate maps product |
| **GitKraken / GitLens** | Virtualized commit DAG + canvas minimap | Commit graph is a **list+lanes** problem, not a file graph. Worktree/WIP watches are first-class | Branch/worktree picker + diff overlay | Their commit-graph widget |
| **Sourcegraph Cody** | Chat + search, **no** code-graph canvas | `@`-mention files/repos/symbols into the composer | Click-node → existing `@` | Building a Cody clone |
| **Gource** | Native OpenGL (web ports: WebGL2/WASM) | Time-lapse **file tree**, not typed deps | Nothing for atlas v1 | Particle history viz |
| **Obsidian graph** | WebGL (Pixi). Local graph vs global | Global 10k+ force graph is the famous hairball; **local/focus** is what people use. Community "advanced" plugins add WebGL + workers + filters to survive 5k–50k notes | Filters + follow-active + N-hop focus | Global force-directed WebGL |
| **CodeNib** (closest analog) | Hierarchy tree + optional Cytoscape file overview | Containment tree + reference overlay; `max_nodes` default **40**; semantic zoom (labels, not more nodes); diff mode | This data model | Cytoscape |
| **lxDIG Visual** | React 19 + D3 force worker | Docs: "hundreds to a few thousand **visible** nodes"; large monorepos **expand-by-depth** | Visible budget + expand | Force layout, Memgraph |
| **Logseq graph** | Pixi | Hard caps (e.g. fast layout ≥2500; large all-pages ≤2200 nodes / 3600 edges); viewport culling is render-only | Caps + culling-does-not-move-layout | Pixi |

**Decision rule:** if a zoom level still exceeds ~400 nodes after crate/file collapse and neighborhood, **tighten the projection** (DOI, depth, max_nodes). Do not switch renderers.

---

## Stack Patterns by Variant

**If the user is at crate zoom (default):**
- Project `build_crate_map` / crate-level nodes + `crate_dep` edges.
- xyflow `group` / `parentId` regions. MiniMap on. Target **≤ ~200** nodes.
- Because that is the only scale at which a monorepo is a map, not a glitter bomb.

**If the user zooms or searches into a crate:**
- Expand files as children (`parentId` = crate). Hide other crates' files.
- Because xyflow's own guide says collapse large trees; `onlyRenderVisibleElements` is extra, not sufficient.

**If the user focuses a symbol (search, click, or chat-touch):**
- Sourcetrail mode: focus + N-hop typed edges (`calls` / `imports` / `implements`) + ancestor chain (file, crate).
- `max_nodes` ~ 80–300. Camera `setCenter`.
- Because this is the view that answers "what is the agent changing?"

**If the user picks a worktree / branch:**
- Load the commit-keyed corpus (or rebuild) for that rev; overlay `compare` vs `main` as node/edge data (`added` / `removed` / `changed`).
- Same canvas, same nodeTypes. Because a second graph lib for "diff" is how you get two atlases.

**If dockview `addPopoutGroup` fails in Tauri:**
- Keep the atlas as a floating group (`addFloatingGroup`) inside the Chat dock.
- Only then open a `WebviewWindow` that mounts the same `CodeAtlas` component.
- Because `window.open` + `/popout.html` is a web-origin API, not a Tauri guarantee.

**If crate-level edges are unreadable after QA:**
- Add `elkjs@0.12.0`, run in a worker, cache by graph hash, apply x/y onto the existing xyflow nodes.
- Do not replace xyflow. Do not introduce dagre.

---

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `@xyflow/react@12.12.0` | `react@19.2.8` / `react-dom@19.2.8` | Peer is `react >= 17`. Verified on npm 2026-10-09. Repo range `^12.11.6` already admits 12.12.0. |
| `@xyflow/react@12.x` | app `zustand@5.0.15` | xyflow depends on **zustand ^4.4.0 internally** (`@xyflow/system` 0.0.83). Dual Zustand is expected. Do not dedupe to v5. |
| `dockview@6.6.1` | `react@19.2.8` | In use today on ChatSurface. Latest dockview is **8.4.1** — incompatible upgrade, out of scope. |
| `dockview@6.6.1` | `@xyflow/react@12` | Fine. Atlas is a panel child; ReactFlow needs a sized parent (`height: 100%`). Same constraint as SubAgentGraph. |
| `@tanstack/react-query@5.102.8` | Tauri invoke / MCP | Existing `useVoxGraphStatus` pattern. |
| `elkjs@0.12.0` (optional later) | xyflow 12 | Layout-only. Feed `{x,y,width,height}` into nodes. Worker recommended (`workerUrl`). |
| `sigma@4.0.0` / `cytoscape@3.34.3` | React 19 | Technically fine; **rejected** for product reasons, not version skew. |

---

## Integration constraints (repo policy)

- LLM calls stay on `vox_actor_runtime::llm`. The atlas does not call models.
- No new `VOX_*` env vars without `contracts/config/env-vars.v1.yaml`.
- No `turso::Connection` outside `vox-db`. Graph cache stays `.vox/cache/graphify/` via `vox_config::paths`.
- New `pub fn`s in `vox-graph-reader` need adjacent tests (`skeleton/untested-pub-api`).
- GUI changes need Playwright steppers + review-bundle screenshots.
- Do not ingest `docs/src/archive/`.
- Build-toolchain invariant: no dep that pulls cmake/nasm/Go/perl/libclang. (Another reason to skip native graph binaries.)

---

## Sources

- npm registry 2026-10-09 — `@xyflow/react@12.12.0`, `dockview@8.4.1` (latest) vs pinned `6.6.1`, `elkjs@0.12.0`, `sigma@4.0.0`, `graphology@0.26.0`, `cytoscape@3.34.3`, `@dagrejs/dagre@3.1.1` — **HIGH**
- `crates/vox-gui/ui/package.json` — installed `@xyflow/react ^12.11.6`, `dockview 6.6.1`, `react ^19.2.8`, `@tanstack/react-query ^5.102.8`, `zustand ^5.0.15`, `@tauri-apps/api ^2.11.1` — **HIGH**
- Context7 `/xyflow/xyflow` — `onlyRenderVisibleElements`, `parentId` / `extent` / `expandParent`, MiniMap/Controls — **MEDIUM** (seam); cross-checked with https://reactflow.dev/learn/advanced-use/performance — collapse trees, memo nodes, do not subscribe to full `nodes` — **HIGH**
- Context7 `/dockview/dockview` + dockview.dev popout/floating docs — `addFloatingGroup`, `addPopoutGroup` — **MEDIUM**; ChatSurface already documents 6.6.1 constraint APIs — **HIGH**
- Context7 `/kieler/elkjs` — `elk.algorithm: layered`, compound JSON, worker — **MEDIUM**
- CodeNib hierarchy model (official docs): containment tree + reference overlay + `max_nodes` — **MEDIUM**
- lxDIG Visual README: visible-node budget, expand-by-depth — **MEDIUM**
- Gource official (OpenGL time-lapse file tree) — **MEDIUM**
- Obsidian / Logseq / advanced-graph-view: WebGL + local/focus vs global hairball — **MEDIUM** for the pattern, **LOW** for plugin-specific FPS claims
- Sourcetrail neighborhood UX, CodeSee hierarchical maps, Sourcegraph Cody `@` mentions — **MEDIUM** (widely documented product behavior; no need to copy their stacks)

---

*Stack research for: Code Atlas (v1.4 `code-atlas`)*
*Researched: 2026-10-09*
