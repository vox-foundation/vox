# Feature Research

**Domain:** Navigable codebase atlas (visual language for crates/files/symbols + `.vox` kinds) beside Chat
**Researched:** 2026-10-09
**Confidence:** MEDIUM
**Milestone:** v1.4 Code Atlas (`code-atlas` workstream)

## Already shipped — do not treat as new

These exist and are table-stakes *infrastructure*, not v1.4 features. Atlas must extend them.

| Existing surface | What it already does | Atlas role |
|------------------|----------------------|------------|
| Graphify status cards (`VoxGraphStatusPanel`, Search Index dock) | Fresh/stale, node/edge counts, Rebuild, TTL | Keep as freshness chrome *on* the atlas, not a second status product |
| Agent Flow SVG | Agent topology around Orchestrator; no pan/zoom/search | **Stay a separate surface.** Do not merge into Code Atlas |
| Repository isolation panel + strategy dropdown | Isolation strategy, per-agent assignment | Reuse worktree/branch *identity*; do not rebuild isolation |
| Chat dockview (`Chat`, `Execution`, `Search Index`, `Repository`, To-dos) | Dock / float / tab beside transcript | Add **Code Atlas** as one more panel type |
| CLI `vox graph query / coverage / crate-map / why-rebuilt / history` | Lexical search, coverage, crate metrics, rebuild causes, first-parent history | GUI consumes these; do not fork a second query language |
| `collapse_to_modules`, crate-map, overlay, BFS, history/lineage | Reader already has lenses | Semantic zoom *must* call these, not reimplement |
| Loquela `@` (`insertAt` / `insertAtFile`) | Agent chips + `@path` file context | Click-to-mention calls this inserter |
| `@xyflow/react` (`SubAgentGraph` only today) | Installed; MiniMap / Controls / `setCenter` / `onlyRenderVisibleElements` available | Render atlas here. Do not add a second graph library |

**Measured ceiling:** cached `repo-code-graph` is **47,111 nodes / ~49k links**, edges have `source`/`target`/`confidence` only — **no `kind`**. Communities are ~1 cluster per 2.3 symbols (meaningless). No chat→graph "agent touched X" event exists.

## How navigable atlases actually work

Products that feel like a *visual language* share one interaction model. Products that feel like a movie or a dashboard do not.

**Gold-standard pattern (Sourcetrail):** Search **activates** a symbol → graph shows **that node + typed edges to neighbors** → optional depth/trail slider → code pane lists locations → click node or edge to retarget. The full index is never drawn. Edge types are first-class (`call`, `include`, `inherit`, `override`, `type-use`) with a **legend**. Members expand/collapse.

**Folder-map pattern (CodeSee):** Start **collapsed**. File-browser search **filters** the canvas. Arrows are file/folder dependencies, not typed symbol edges. PR Review Maps **color changed files**. Geometric pan/zoom, not symbol-level semantic zoom.

**Local-graph pattern (Obsidian):** Global graph for small corpora; **local graph + depth slider** when large. Search **filters**; color **groups**; hover highlights neighbors. Zoom is geometric.

**Collapse/focus pattern (dependency-cruiser):** `focus` + `maxDepth` + `collapsePattern` / `archi` reporter. Click node → open file. Without collapse, repo-scale graphs hairball. VS Code extensions are **file-seeded**, not repo atlases.

**Not a code atlas (still steal one idea each):**

| Product | What it actually visualizes | Steal | Do not copy |
|---------|----------------------------|-------|-------------|
| GitKraken / GitLens | Commit/branch/worktree/WIP/agent-session graph | Worktree pills, agent-status overlay, history minimap | Treat commit graph as the codebase map |
| Sourcegraph + Cody | Search, go-to-def, blame; **no canvas** | `@`-mention chips for file/symbol into chat | Build SCIP-precise nav as v1.4 |
| Gource | Animated dir/file tree + author avatars over VCS time | Pulse-on-touch | Time-lapse as the primary UI; no symbols/types |
| CodeScene | Hierarchical circle-pack; color = hotspot/health/knowledge | Hierarchical zoom; coupling **on hover** | Health/knowledge product; always-on coupling hairballs |

**Scale consensus (MEDIUM, cross-checked):** Do not render 47k xyflow nodes. Semantic zoom = **levels** (crate → module/file → symbol) + **expand-on-demand** + viewport cull (`onlyRenderVisibleElements` *after* the subgraph is small). Tile pyramids / WebGL hairball engines are overkill for v1.4.

---

## Feature Landscape

### Table Stakes (Users Expect These)

Missing these = "this is still a status card, not an atlas."

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Semantic zoom (crate → file → symbol) | Every real atlas starts collapsed (CodeSee folders, CodeScene circles, depcruise `archi`, Sourcetrail member expand). 47k symbols cannot be the default view. | HIGH | Drive from `crate-map` + `collapse_to_modules` + on-demand BFS, not Leiden `community`. Zoom changes **what is drawn**, not just camera scale. |
| Search-to-focus | Sourcetrail/CodeSee/Obsidian/depcruise all treat search as the way in. Users will type a symbol and expect the camera to fly there. | MEDIUM | Reuse `vox graph query` (already lexical). `setCenter`/`fitView` on the hit. Do not invent a second search. |
| Minimap + fit/zoom chrome | GitLens Commit Graph and xyflow users expect a locator. Without it, a crate map feels lost. | LOW | Ship `@xyflow/react` `<MiniMap pannable zoomable />` + `<Controls />`. Color nodes by kind/level. |
| Typed-edge visual language | "How things relate" is the brief. Untyped arrows are CodeSee-at-file-level, not a language. | HIGH | **Data first:** extractor must emit `kind` on links (`call` / `import` / `impl` / `test`, plus `contains` for hierarchy). Then stroke/dash/glyph + tooltip + **legend** (Sourcetrail `legend` command). Bundle parallel edges. |
| Meaningful hierarchy | Users expect crates as **regions**, files as **containers**. Today's 20,653 communities fail that. | HIGH | Hierarchy = Cargo crate → rustc/vox module → file → symbol. Drop community-as-cluster from the default view. |
| Dockable / floating atlas beside Chat | The success picture is a widget next to the transcript, not a standalone IDE. | MEDIUM | New dockview panel `codeAtlas` next to Search Index / Repository. Same dock/float/pop-out behavior. |
| Branch / worktree picker | "Any branch that existed at a given time" is in the brief. Isolation dropdown is strategy, not a graph target. | MEDIUM | Picker of `main` + local worktrees/branches. Bind atlas to that snapshot. Rebuild/freshness already exists — show it on the picker. |
| Legend + hover tooltips | Sourcetrail had to add a legend because exported graphs were unreadable. A "visual language" without a key is decoration. | LOW | `?` / `legend` overlay: node kinds + edge kinds. Hover edge → kind + confidence. |
| Keyboard pan / zoom / activate | Sourcetrail added WASD/HJKL because mouse-only failed. | LOW | Arrow/HJKL pan, `+/-` zoom, Enter activate, Esc clear focus. Playwright-cover the stepper. |

### Differentiators (Competitive Advantage)

Not required to *look* like an atlas. Required to *be Vox*.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Click-to-`@`mention | Closes the loop Chat ↔ graph. Cody's `@file`/`@symbol` chips are the industry pattern; Vox already has `insertAtFile`. | LOW | Click node → same `@path` (file) or `@path::symbol` if composer later accepts symbols. **Reuse chips. No second syntax.** P1 differentiator — cheapest unique win. |
| Vox-native node kinds | `.vox` repos are first-class: `component`, `table`, `query`, `mutation`, `workflow`, `activity`, `actor`, `tool`, `resource`, `form`, `index`. Rust-only atlases cannot show this. | MEDIUM | Extractor + distinct glyphs/colors. Hierarchy still crate/file. Do not invent a parallel graph. |
| Live agent-touch pulses | Graph updates while the conversation talks (Gource pulse + GitLens agent pills). | HIGH | **Blocked** on a structured "agent touched file/symbol" event from chat/orchestrator → GUI. Pulse + fade; do not relayout. After the event exists, this is the signature demo. |
| Worktree vs `main` compare coloring | CodeSee Review Maps for branches: added / removed / changed nodes vs `main`. | HIGH | Needs two snapshots or a diff overlay (`vox-graph-reader` history/lineage is commit-keyed today, not worktree-keyed). Color only; keep one layout. |
| Local-graph depth slider | Obsidian local graph + Sourcetrail Custom Trail: 1-hop default, slider to N. | MEDIUM | Default = activated node + typed 1-hop. Slider 1–3. Infinite depth is an anti-feature at 47k. |
| Coverage / test overlay | CLI `vox graph coverage` + `overlay_test_targets` already exist; no GUI. | MEDIUM | Optional lens: surfaced / orphan / dead-end / `targeted_by`. Not default paint. |
| Chat-driven camera | Atlas follows whatever the agent is talking about without a click. | HIGH | Same event bus as pulses. Camera eases to the node; user can pin/unpin follow. |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Draw all 47k nodes | "Show the whole repo" | xyflow DOM nodes will jank; hairball is unreadable. Every successful product refuses this. | Semantic levels + local subgraph. `onlyRenderVisibleElements` is a *second* cull, not the first. |
| Force-layout the full graph | "Make it look like Obsidian / Gource" | Layout of 47k is a freeze; positions thrash on every rebuild; mental map dies. | Deterministic crate-region layout (crate-map / layered). Animate only pulses and camera. |
| Merge Agent Flow into Atlas | "One graph to rule them all" | Agents ≠ code. Mixing topologies destroys both languages. | Keep Agent Flow. Atlas may *badge* a crate the agent is in. |
| Second graph library (Sigma, Cytoscape, deck.gl tiles) | "47k needs WebGL" | Scoping decision: prove xyflow + culled subgraphs first. New lib = new a11y/test/theme surface. | Viewport-culled subgraphs on `@xyflow/react`. Revisit only if crate-level still janks. |
| New mention syntax / `@graph:` | "Atlas nodes are special" | Splits composer mental model; chips already exist. | Call `insertAtFile` / future symbol chip. |
| Gource-style time-lapse as home view | "See the repo evolve" | Pretty, not navigable; no types; no search-to-focus. | Optional later "replay last N agent touches." Not v1.4 home. |
| CodeSee tours / sticky annotations | Onboarding maps | Editorial product; persists layout as content; fights live rebuilds. | Skip. Chat transcript *is* the tour. |
| CodeScene health / knowledge maps | "Color = quality" | Different product (git mining + org data). Out of milestone. | Optional later lens; do not block atlas. |
| Always-on coupling / all edge kinds at once | "Show everything related" | CodeScene only shows coupling on hover for a reason. | Default: edges of the **active** node. Edge-kind filter chips. |
| Precise cross-repo SCIP / go-to-def IDE | Sourcegraph feature list | Years of indexer work; we already have Graphify + CLI. | Stay on Graphify confidence + BFS. Click can open file later; not v1.4. |
| Editable map as source of truth | CodeSee drag-to-reposition | Conflicts with live rebuild; two SSoTs. | Allow transient drag; persist **camera + focus + filters**, not node XY. |
| Continuous force simulation | "Alive" feeling | Burns CPU beside Chat; fights pulses. | Static layout; pulse CSS; camera ease. |
| Infinite-depth trail | Sourcetrail slider-to-top | Explodes to thousands of nodes from a hub (`god_nodes`). | Cap depth at 3; "open in query" for more. |

---

## Feature Dependencies

```
Typed edges (extractor + schema)
    └──requires──> Visual language (stroke/glyph/legend)
                       └──enhances──> Semantic zoom (edges bundle at crate/file)

Meaningful hierarchy (crate-map + collapse_to_modules)
    └──requires──> Semantic zoom
                       └──requires──> Search-to-focus (something to fly to)
                       └──enhances──> Minimap (regions, not 47k dots)

Dockview Chat panel
    └──requires──> Atlas widget
                       └──enhances──> Click-to-@mention (composer is next door)
                       └──enhances──> Live pulses (same session)

vox graph query
    └──requires──> Search-to-focus

Loquela insertAtFile
    └──requires──> Click-to-@mention

Chat/orchestrator "touched path/symbol" event   [MISSING TODAY]
    └──requires──> Live pulses
    └──requires──> Chat-driven camera

Per-worktree / per-branch snapshot (or overlay vs main)
    └──requires──> Branch picker binding
    └──requires──> Compare coloring

Vox AST kinds in extractor
    └──requires──> Vox-native glyphs

Agent Flow
    └──conflicts──> Merging into Atlas
```

### Dependency Notes

- **Typed edges require extractor work first.** Painting untyped links with four colors is a lie. This is the data-phase that unblocks the visual language.
- **Semantic zoom requires hierarchy, not communities.** `collapse_to_modules` + crate-map already exist; default view must use them.
- **Search-to-focus requires query + a laid-out subgraph.** Query exists; the camera is cheap once levels exist.
- **Live pulses conflict with nothing except missing telemetry.** Do not fake pulses from file-watcher noise.
- **Click-to-mention enhances the dock** and is blocked on nothing except a node→path mapping (IDs are already `path::symbol`).
- **Compare coloring conflicts with a single-snapshot cache.** Today's graph is keyed to one commit (`c5d162b9`). Picker without compare is still valuable.

---

## Expected user walkthroughs (v1.4)

These are the behaviors the atlas must support. If a walkthrough needs a missing event or a second snapshot, that feature is P2.

### W1 — Open beside Chat and read the repo (table stakes)

1. User opens Chat. Docks **Code Atlas** (not Search Index cards, not Agent Flow).
2. Default target is `main` (or the active worktree). Freshness chip shows built-at / stale / Rebuild (existing status API).
3. Default level = **crates as regions**, files hidden, symbols hidden. Edges between crates are bundled `import`/`call` with thickness = weight.
4. User zooms (or double-clicks a crate) → files appear inside that crate. Another zoom/click → symbols in the focused file.
5. Minimap tracks the crate map. Legend is one click.

### W2 — Search-to-focus

1. User types `persist_fact` in the atlas search (same scorer as `vox graph query`).
2. Autocomplete shows kind-colored hits. Enter selects one.
3. Camera eases to that node; local graph = node + typed 1-hop. Other crates dim.
4. User hits Esc → back to crate level, query cleared.

### W3 — Click-to-`@`mention (differentiator, P1)

1. Atlas is docked; composer is visible.
2. User clicks a file or symbol node (or "Add to chat" on the node toolbar).
3. Composer inserts `@crates/vox-orchestrator/src/memory/manager.rs` (existing `insertAtFile`) and a context chip.
4. User sends. Agent sees the same context path the graph showed. **No new `@` grammar.**

### W4 — Watch the agent work (differentiator, P2 — needs event)

1. User pins follow. Agent edits `manager.rs` then `ops_agents.rs`.
2. Those nodes pulse (Gource-style, ~1.5s fade). Minimap ticks the same crates.
3. Camera optionally eases to the latest touch. User can unpin.
4. If the graph is stale vs worktree, freshness chip warns; pulses still attach to last-known IDs.

### W5 — Compare a worktree to `main` (differentiator, P2 — needs second snapshot)

1. User picks worktree `feat/atlas` in the atlas picker (not the isolation strategy dropdown).
2. Overlay vs `main`: added nodes green, removed muted/struck, changed amber. Layout stays `main`'s crate regions so the map does not jump.
3. Click a green node → `@`mention that path from the worktree.

### W6 — Vox-native repo

1. User opens a `.vox` app (or `examples/golden`).
2. File expand shows `component` / `workflow` / `actor` / `table` glyphs, not only `fn`/`struct`.
3. Edges: `call` into activities, `uses`/`auth` as typed edges if extracted, `test` from `@test` overlays.
4. Same search / mention / dock behavior as Rust.

---

## MVP Definition

### Launch With (v1.4)

Minimum that makes the success picture true *without* the missing event bus or dual snapshots.

- [ ] **Typed edges in Graphify output** — without this there is no visual language
- [ ] **Hierarchical semantic zoom** — crate regions → file containers → symbols; never paint 47k
- [ ] **Search-to-focus** — `vox graph query` → camera + 1-hop local graph
- [ ] **Dockable Code Atlas panel** — beside Chat via existing dockview
- [ ] **Minimap + legend + zoom controls** — xyflow primitives
- [ ] **Branch/worktree picker bound to one snapshot** — `main` + listed worktrees; freshness chip reused
- [ ] **Click-to-`@`mention** — reuse `insertAtFile`
- [ ] **Vox-native kinds on `.vox` extract** — glyphs in the same canvas

### Add After Validation (v1.4.x / next phase)

- [ ] **Live agent-touch pulses** — trigger: structured chat/orchestrator touch event ships
- [ ] **Chat-driven camera (follow mode)** — same event
- [ ] **Worktree vs `main` compare coloring** — trigger: per-ref graph or overlay API
- [ ] **Local-graph depth slider (1–3)** — trigger: 1-hop feels too tight on hubs
- [ ] **Coverage / test-target lens** — trigger: users ask "what's orphan / tested"

### Future Consideration (v2+)

- [ ] Gource-style replay of agent touches
- [ ] CodeScene-like hotspot/health color (git-mining product)
- [ ] CodeSee tours / sticky annotations
- [ ] Precise SCIP / cross-repo go-to-def
- [ ] WebGL / second renderer if crate-level still janks (unproven)
- [ ] Opening the file in an editor pane from a node (nice; not the Chat-widget job)

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Typed edges (data + paint + legend) | HIGH | HIGH | P1 |
| Semantic zoom (crate/file/symbol) | HIGH | HIGH | P1 |
| Search-to-focus | HIGH | MEDIUM | P1 |
| Dockable atlas beside Chat | HIGH | MEDIUM | P1 |
| Minimap + controls | MEDIUM | LOW | P1 |
| Branch/worktree picker (single snapshot) | HIGH | MEDIUM | P1 |
| Click-to-`@`mention | HIGH | LOW | P1 |
| Vox-native node kinds | HIGH | MEDIUM | P1 |
| Local-graph depth slider | MEDIUM | MEDIUM | P2 |
| Live agent-touch pulses | HIGH | HIGH | P2 |
| Compare coloring vs `main` | HIGH | HIGH | P2 |
| Coverage / test overlay | MEDIUM | MEDIUM | P2 |
| Chat-driven camera | MEDIUM | HIGH | P2 |
| Replay / tours / health maps | LOW | HIGH | P3 |

**Priority key:**
- P1: Must have for v1.4 launch (validates "visual language beside Chat")
- P2: Should have once P1 data/events exist
- P3: Nice to have; different product or unproven renderer

**Opinionated v1.4 cut:** ship the Sourcetrail interaction model (search → local typed graph → expand) on a CodeSee-style collapsed crate map, docked like Search Index, with Cody-style `@` from a click. Defer Gource pulses and Review-Map colors until the event and the second snapshot exist — faking either will train the wrong demo.

---

## Competitor Feature Analysis

| Feature | Sourcetrail | CodeSee | Obsidian | depcruise / VS Code | GitLens | Sourcegraph Cody | Gource | CodeScene | Our approach |
|---------|-------------|---------|----------|---------------------|---------|------------------|--------|-----------|--------------|
| Default view | Active symbol + neighbors | Collapsed folders | Global or local | File-seeded or collapsed `archi` | Commit graph | No canvas | Whole file tree animating | Circle-pack folders | **Collapsed crates** (CodeSee) + **activate-to-local** (Sourcetrail) |
| Semantic zoom | Expand members / trail depth | Folder toggle | Geometric only | `collapsePattern` / `maxDepth` | N/A | N/A | Camera zoom | Hierarchical circles | **Level switch** crate→file→symbol |
| Search-to-focus | Fuzzy symbol activate | File-browser filter | Filter + highlight | `--focus` | Commit search | Search is the product | Highlight user/dir | Filter by team | **`vox graph query` → `setCenter`** |
| Minimap | No (recenter on activate) | Zoom controls | No | No | Yes (history) | N/A | Optional key | No | **xyflow MiniMap** |
| Typed edges | Yes (call/include/inherit/…) | File deps only | Link / backlink | Import + rule flags | N/A | N/A | Tree edges only | Coupling on hover | **`call`/`import`/`impl`/`test` + `contains`** |
| Branch / worktree | Project = one index | PR Review Maps | N/A | N/A | Worktree pills | Repo @mention | One log | Analysis window | **Picker + later color-vs-main** |
| Live agent pulses | No | No | No | No | Agent session pills | Chat cites files | Author avatars | No | **P2 pulse on touch event** |
| Click → chat context | Show-in-IDE | View code | Open note | Open file | Open diff | **`@` chips** | No | Virtual review | **`insertAtFile`** |
| Language-native kinds | C++/Java node types | File/folder | Note/tag | Module | N/A | Symbol | File ext color | File | **Rust + Vox kinds** |
| Renders whole corpus | Never | Collapsed | Hurts when large | Hairballs if you let it | History rows | N/A | Yes (files only) | Aggregated circles | **Never 47k** |

---

## Sources

- Sourcetrail official `DOCUMENTATION.md` (CoatiSoftware/Sourcetrail) — search / graph / typed edges / custom trail / legend [MEDIUM, official docs via Exa]
- CodeSee docs: Explore Your Map, First Map Best Practices, Service Maps — collapsed folders, file-browser search, PR Review Maps [MEDIUM]
- GitLens / GitKraken help: Commit Graph, worktrees, minimap, agent sessions [MEDIUM]
- Sourcegraph docs: Code Navigation; Cody Chat / Context `@`-mentions [MEDIUM]
- Gource official site + GitHub README/wiki — file/dir tree + author pulses [MEDIUM]
- CodeScene enterprise docs 7.4 — hotspot system maps, hierarchical zoom, coupling-on-hover [MEDIUM]
- Obsidian Help — Graph view / Local graph / filters / groups [MEDIUM]
- dependency-cruiser options + CLI docs; VS Code extension READMEs — focus/collapse/maxDepth; click-to-open [MEDIUM]
- Large-graph LOD/culling: Cytoscape LOD, expand-collapse mental-map papers, MSAGLJS tile pyramids (overkill for v1.4) [MEDIUM]
- React Flow docs (`/websites/reactflow_dev`) — MiniMap, Controls, `setCenter`, `onlyRenderVisibleElements` [MEDIUM, Context7]
- In-repo: `.planning/workstreams/code-atlas/CONTEXT.md`; `vox-graph-reader` (`lens`, `overlay`, `coverage`, `crate_model`); `Loquela.tsx` `insertAtFile`; `ChatSurface.tsx` dockview; cached `repo-code-graph` schema [HIGH]

---
*Feature research for: Code Atlas (v1.4)*
*Researched: 2026-10-09*
