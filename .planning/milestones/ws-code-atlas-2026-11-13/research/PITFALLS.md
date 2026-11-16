# Pitfalls Research

**Domain:** Live code atlas in an existing Graphify + Tauri dockview GUI
**Researched:** 2026-10-09
**Confidence:** HIGH on Vox-measured hazards; MEDIUM on library/a11y limits (Context7 official docs + cross-checked web sources)

Proposed prevention phases (continue numbering after docs-freshness Phase 24):

| Phase | Name | Owns |
|-------|------|------|
| 25 | Graph data & identity | Typed edges, hierarchy, clustering floor, snapshot ≠ branch |
| 26 | Viewport-culled renderer | Semantic zoom, search-to-focus, minimap, xyflow LOD |
| 27 | Branch / worktree overlay | Picker, diff vs `main`, honest freshness |
| 28 | Dock widget chrome | Dockview persist, floating vs Tauri window, resurrection |
| 29 | Live chat pairing | File-touch events, pulses, browser-preview empty states |
| 30 | Mentions & Vox kinds | Click-to-`@`, kind taxonomy, path/secret hygiene |
| 31 | Verification | Playwright stepper, canvas a11y, synthetic fixtures |

Do **not** confuse this surface with Agent Flow (`AgentFlow` SVG ring). That stays a separate topology.

---

## Critical Pitfalls

### Pitfall 1: Drawing the 47k-node graph (or stuffing it into the React Flow store)

**What goes wrong:**
The cached `repo-code-graph` is **47,111 nodes / 49,775 edges**. Mounting that as `@xyflow/react` nodes freezes the UI. `onlyRenderVisibleElements` only skips *DOM mount* for off-screen nodes; the full array still lives in the React Flow store, still participates in hit-testing/layout, and still serializes on every store update. xyflow is SVG/DOM, not WebGL. A second library (Sigma, Cytoscape WebGL, deck.gl) does not fix a payload that should never leave the reader.

**Why it happens:**
"We already depend on `@xyflow/react`" (used today only by `SubAgentGraph`) reads as permission to feed it the corpus. Viewport culling looks like a scale solution in the docs. Demo graphs are hundreds of nodes.

**How to avoid:**
- Serve a **lens**, not the corpus. `vox-graph-reader::lens::collapse_to_modules` and `pick_lens(node_count, threshold)` already exist — default the first paint to crate/module tiles.
- Cap each fetch (comparable IDE graphs stay ≤ ~200 nodes per request). Drill-in loads children of one container.
- Positions come from a precomputed hierarchical layout on the server, not a client force sim on 47k nodes. xyflow does not layout; every node needs an explicit `position`.
- Keep `nodeTypes` / `edgeTypes` as **module-level constants**. A new object each render trips xyflow error `002` and remounts every custom node.
- Do not add a second graph library unless Phase 26 measures that viewport-culled *subgraphs* still miss the frame budget.

**Warning signs:**
- First paint > 200 ms or main thread blocked > 50 ms with the real corpus.
- React Flow store `nodes.length` ≈ 47k.
- `fitView` on the full graph; force-directed layout in the webview.
- New `sigma` / `cytoscape` / `deck.gl` dependency in `crates/vox-gui/ui/package.json`.

**Phase to address:**
Phase 26 (renderer). Phase 25 must ship the lens/aggregation API the renderer consumes. Phase 31 proves it with a **synthetic** 47k-shaped fixture, not the real cache file.

---

### Pitfall 2: Treating 20,653 Leiden communities as visual regions (hairball)

**What goes wrong:**
Communities are almost 1:2 with symbols (20,653 clusters / 47,111 nodes). Drawing each community as a region is a hairball with a legend. Leiden on software graphs is known to emit many small, poorly cohesive clusters; modularity maximization also overfits and invents modules in random graphs. A force layout that is not *constrained* by a real hierarchy will hide the crate/file structure the user already has.

**Why it happens:**
The JSON already has a `community` field, so it looks like a clustering product. `collapse_to_modules` currently stamps every module `community: "c_0"`, which trains the next author to ignore the field or to trust the raw one.

**How to avoid:**
- **Layout-second:** compute hierarchy first (crate → module/file → symbol; Vox: crate → `.vox` file → `component`/`table`/`workflow`/`actor`/…), then constrain positions to that tree.
- Apply a **size floor** (drop or fold communities with < 4 members) before any community is a visual region.
- Prefer `vox-graph-reader` crate-map + module lens over raw `community` IDs. Do not promote Leiden IDs to the visual language until a later research spike proves they beat path hierarchy.
- Semantic zoom changes *kind* of glyph (crate tile → file container → symbol), not just scale. Unexpected auto-expand of packages destroys the mental map.

**Warning signs:**
- Region count within an order of magnitude of node count.
- Color = community id, no crate/file chrome.
- "Cluster" UI that cannot name a crate.

**Phase to address:**
Phase 25 (data). Phase 26 only draws what 25 emits.

---

### Pitfall 3: Untyped edges — the visual language cannot show *how*

**What goes wrong:**
Edges today are `{source, target, confidence}` only. After `collapse_to_modules` they gain `weight` and still no `kind`. The atlas cannot distinguish calls vs imports vs implements vs crate-dep vs "agent just edited." Color-only edge styles then fail WCAG 1.4.1.

**Why it happens:**
The snapshot is treated as complete. UI authors invent stroke colors per guessed relation. A crate-edge `exceptions` row looks like the way to "just add a kind field" in the wrong crate.

**How to avoid:**
- Add a closed `edge.kind` enum in `vox-graph-reader` (and the graph JSON contract) before any canvas work: at minimum `calls | imports | implements | crate_dep | contains | mentions`.
- Live overlay uses a **different channel** (pulse, badge, dashed halo) — not a fake graph edge.
- Patterns + labels, not hue alone.
- New workspace crate edges: narrower `-types` crate or <50-line defactor. **`exceptions` ledger entries are user-authorized-only** — never self-add to ship the atlas.

**Warning signs:**
- Renderer switches on `confidence` or hex color to mean relation type.
- PR that edits `contracts/ci/crate-edges.allow.v1.json` exceptions without an operator-authored ledger note.

**Phase to address:**
Phase 25. Phase 26 consumes kinds; Phase 29 must not overload them for live pulses.

---

### Pitfall 4: Stale snapshot presented as the live / current-branch graph

**What goes wrong:**
The measured cache is keyed to `c5d162b9` (2026-09-30) — **9 days stale** vs a 2026-10-09 workspace. Graphify snapshots are **rebuild history**, not branches. A user picking `main` or a worktree while the atlas still shows that SHA will "watch" edits that cannot appear. `useFreshness` + `LIVE_EVENT_FRESH_MS` (10s) is an **orch-status event** window, not a graph-corpus clock. Corpus TTL is days (`ttl_days_default`); conflating the two makes a 9-day-old graph look "live."

**Why it happens:**
`VoxGraphStatusPanel` already shows built-at / Rebuild / TTL, so a canvas author assumes "status is handled." Branch picker is copied from `RepositoryView`'s strategy dropdown. Pulse animation is wired to "panel is open."

**How to avoid:**
- Every atlas payload carries `{corpus_id, git_sha, ref_name, worktree_id, built_at, lens}`. The chrome shows **ref + SHA + relative built-at** (reuse `relativeBuiltAt`) and a stale tone when SHA ≠ selected ref HEAD or age > TTL.
- Branch/worktree picker **selects a graph identity**, then loads or rebuilds that identity. Do not overlay a worktree diff on a `main`-SHA snapshot and call it that worktree.
- Rebuild remains an explicit affordance (`vox_search_rebuild`). Auto-rebuild on every chat turn is out of scope and will fight the TTL contract.
- Never map `freshnessTone(..., {usesPolling:false})` to graph corpus age.

**Warning signs:**
- Atlas header has no SHA / says "live" with a days-old `built_at`.
- Picker changes and node ids do not.
- Tests use "fresh" to mean "IPC succeeded."

**Phase to address:**
Phase 25 (identity fields). Phase 27 (picker + overlay). Phase 29 must not paint live over a mismatched SHA.

---

### Pitfall 5: False 'live' pulses (no file-touch event exists)

**What goes wrong:**
Chat has **no structured "agent touched file X / symbol Y" event** on the GUI bus. `vox-orchestrator-queue` oplog has `OperationKind` "agent edited files," but nothing reaches dockview. A CSS pulse on "the model is streaming" or on search-index poll will train users to distrust the atlas. Browser preview has no desktop backend — a pulse there is a lie.

**Why it happens:**
The success picture says "nodes pulse as the agent edits them." The shortest path is to pulse whatever is already ticking (`LIVE_EVENT_FRESH_MS`, MCP status refetch, token stream).

**How to avoid:**
- **No pulse without a typed event** `{path, symbol_id?, sha?, session_id, at}`. Ship the event (or an explicit "overlay unavailable") in Phase 29 *before* animation.
- Pulse only nodes in the **loaded lens** whose id matches the event. Unknown ids update a "touched off-screen" chip, not a random crate.
- `freshnessTone` for pulses: `live` only within `LIVE_EVENT_FRESH_MS` of that event; else `stale`. Polling the graph status is `poll`, never `live`.
- Browser / `backendAvailable() === false`: empty state from `BackendUnavailableError` copy — no fake nodes, no pulse. Reuse `sanitizeErrorForToast`.

**Warning signs:**
- Animation class bound to `isStreaming` or query `isFetching`.
- Playwright mock that pulses without emitting a touch event.
- Preview build showing a populated atlas.

**Phase to address:**
Phase 29. Phase 31 stepper must include "streaming but no touches → no pulse" and "preview → honest empty."

---

### Pitfall 6: Dockview layout persistence + panel resurrection

**What goes wrong:**
`DockWorkspaceShell` persists `${prefix}.dockview_layout.v3`, strips `params`, and `fromJSON`s on ready. Empirically, dockview-core@6.6.1 `fromJSON` **throws** on some older shapes (sessions-panel leftover). A new Code Atlas panel id that is auto-recreated on ChatSurface refresh will **resurrect** after the user closes it — already the documented bug this shell was extracted to stop. `closedPanelIds` is **host-local**, not in the shell. Adding a 6th "core" panel without that guard repeats the Flow-panel bug.

Dockview's inner `.dv-shell` computed **0×0** when height was percentage-only; `api.layout(w,h)` does not fix it. A WebGL/canvas atlas inside a 0×0 pane looks like a blank product.

**Why it happens:**
New surfaces get `addPanel` in the refresh effect "so it is always there." Layout key is not bumped when the panel catalog changes. Canvas authors assume the pane has pixels.

**How to avoid:**
- Atlas is an **opt-in dock panel** (like VoxGraph / Repository), not a 6th always-on core panel — *or* it gets the same `closedPanelIds` + `onDidRemovePanel` guard and a ChatSurface test cloned from "does not resurrect the VoxGraph panel."
- Bump layout key (`v3` → `v4`) if default panel set changes; keep the try/catch fallback to defaults.
- Do not persist atlas **graph camera / selection** inside dockview `params` (already stripped). Persist camera in atlas-owned state keyed by `{corpus, sha, lens}`.
- Assert non-zero pane box before mounting React Flow (reuse the shell's `pixelHeight` pattern).
- `dndStrategy="pointer"` stays. Do not switch back to HTML5 DnD for "pop-out drag."

**Warning signs:**
- Close Atlas, send a chat message, Atlas tab returns.
- `fromJSON` warnings in console on every load after the panel ships.
- Canvas at 0×0 in CDP; works in jsdom.

**Phase to address:**
Phase 28. Phase 31 includes the resurrection stepper.

---

### Pitfall 7: Floating window ≠ Tauri `WebviewWindow`

**What goes wrong:**
Dockview **floating groups** are in-webview (already styled via `--dv-floating-box-shadow`). Dockview **popout** opens a browser window and restores **asynchronously** (`api.popoutRestorationPromise`). Tauri `WebviewWindow` is a **new JS heap** plus a capability boundary: create needs `core:webview:allow-create-webview-window` (not in `core:webview:default`); if the label is not on a capability, the window has **no IPC**. Child `Webview` requires the `unstable` Cargo feature. React context, the chat store, and `useVoxGraphStatus` do not cross windows. `dndStrategy="pointer"` already documents **no popout-drag flows**.

A pop-out atlas that cannot hear file-touch events or mention-insert into the composer will look broken; one that is granted a wide capability is a privilege bug.

**Why it happens:**
The brief says "dockable / floating / pop-out." Those three are different products. VS Code webview popout intuition does not map to Tauri 2.

**How to avoid:**
- **v1.4 default:** docked panel + dockview floating group in the **same** webview. Same React tree, same mention callback, same event bus.
- Defer OS-level `WebviewWindow` until a later milestone. If it ships: dedicated capability with an exact label; rehydrate graph identity + subscribe to the same MCP/events; mention-insert via `emit` back to `main`; await popout restoration before claiming ready.
- Do not enable `unstable` child webviews for this milestone.

**Warning signs:**
- `WebviewWindow` / `allow-create-webview-window` in the atlas PR.
- Pop-out that cannot `@`-mention or pulse.
- HTML5 drag between windows (disabled by `pointer` strategy).

**Phase to address:**
Phase 28. Mention/live pairing (29–30) assumed same-webview.

---

### Pitfall 8: `@`-mention injection and path leaks

**What goes wrong:**
Loquela already owns `@`: `/@([^\s]*)$/` opens a picker that `invoke`s `list_repo_files` (debounce 200 ms, cap `LOQUELA_FILE_PICKER_LIMIT` = 20). Graph node ids are `path::container::name` (see `lens::module_of`). Inserting the raw id after `@` breaks the file picker (it is path-like because of `/` and `::`) and can dump **absolute paths**, worktree prefixes, or secret-looking filenames into the composer **and** into toasts (`attachContext` already joins raw paths into toast bodies). A second mention syntax splits the product. Untrusted node labels pasted into the composer are LLM context.

**Why it happens:**
Click-to-mention is implemented as `composer.value += '@' + node.id`. Graph ids look like convenient handles.

**How to avoid:**
- Reuse Loquela: insert `@<repo-relative-path>` (and a chip via `addContextRef` when the node is a file) through one shared helper. Symbols: path + fragment the picker already understands, or a chip `kind: 'symbol'`, **not** a new `@symbol::` dialect in v1.4.
- Never insert `::` ids, absolute paths, or `file://` URLs.
- Sanitize toast/status text with `sanitizeErrorForToast`; do not echo full path lists.
- Click-to-mention is a **composer edit**, not an auto-submit.
- Browser preview: `list_repo_files` fails closed (already `catch → []`); atlas click must not invent paths.

**Warning signs:**
- `@crates/foo/src/lib.rs::Foo::bar` in the textarea.
- Toast containing `$HOME` or `.vox/`.
- New mention parser beside the `/@([^\s]*)$/` matcher.

**Phase to address:**
Phase 30. Phase 29 must not emit paths on the pulse bus that 30 would then inject.

---

### Pitfall 9: Vox vs Rust kind confusion

**What goes wrong:**
Graphify AST extraction is tree-sitter TypeScript/Rust-shaped (`call_expression`, `method_definition`). Vox surface kinds are **bare-keyword scopes**: `component`, `table`, `workflow`, `actor`, `fn`, plus data-layer `query`/`mutation`/…. Collapsing `.vox` files to `kind: "fn"` or treating `actor` as a Rust module makes the atlas lie about the language the milestone exists to show. Conversely, painting Rust crates with Vox glyphs is costume jewelry.

**Why it happens:**
One `kind` string is reused for renderer icons. `collapse_to_modules` emits `kind: "module"` for every file, including `.vox`.

**How to avoid:**
- Closed unions: `source_lang: rust | vox | ts | other` **and** `kind` per language. Vox kinds are the grammar keywords, not Rust's.
- `.vox` drill-in uses compiler/graphify Vox extractors, not the TS walker.
- Icon/legend tables are keyed by `(source_lang, kind)`.
- Tests: a fixture `.vox` with `workflow`/`actor`/`component` must not render as `fn`.

**Warning signs:**
- Single switch on `kind === "fn"` for all files.
- Vox repo demo that looks like a Rust crate map.

**Phase to address:**
Phase 25 (schema). Phase 30 (glyphs + legend). Phase 31 fixtures in both languages.

---

### Pitfall 10: Testing the atlas against the 47k cache (or not at all)

**What goes wrong:**
Checking the real `.vox/cache/graphify/repo-code-graph/graph.json` into CI is huge, stale, and machine-specific. Skipping scale tests means the 47k freeze ships. jsdom cannot see the dockview 0×0 CSS bug or WebGL. Playwright without `installTauriMock` will either hang on IPC or paint a dishonest populated graph. New GUI surfaces **require** a visual-stepper spec + review-bundle screenshots (repo policy).

**Why it happens:**
The cache is already on disk locally. "We'll run it manually in Tauri" skips the gate.

**How to avoid:**
- **Synthetic fixtures:** tiny (10–30 nodes) for unit/lens/kind tests; **shaped** (same depth, community/node ratio, untyped vs typed edges) at ~1k and a streamed 47k *count* that never materializes 47k DOM nodes.
- Reader tests stay in `vox-graph-reader` with `serde_json` blobs (see existing `coverage.rs` fixtures).
- GUI: vitest for lens adapters and mention helper; Playwright stepper for dock, search-to-focus, pulse honesty, empty preview — `installTauriMock` / `installTauriMockRich` only.
- Do not read the developer's cache from tests.

**Warning signs:**
- Test opens `.vox/cache/graphify/**`.
- e2e without `installTauriMock`.
- No `crates/vox-gui/ui/e2e/*atlas*` spec.

**Phase to address:**
Phase 31. Fixtures designed in 25–26 so later phases do not invent a third shape.

---

### Pitfall 11: Canvas / WebGL with no accessible equivalent

**What goes wrong:**
A pan-zoom canvas is an image to AT. `role="img"` on an interactive surface hides children. Hover-only tooltips, color-only kinds, and a running force layout fail WCAG 1.1.1, 1.3.1, 1.4.1, 2.1.1, 2.1.2, 2.4.7, 4.1.3. Proxy-per-node DOM at 47k is its own freeze. Playwright click-only steppers will not catch this.

**Why it happens:**
xyflow nodes are focusable *if mounted*; viewport culling unmounts them. WebGL backends have no nodes at all.

**How to avoid:**
- Graph accessibility **layer** independent of renderer: summary ("N crates, M files, lens=modules, sha=…"), searchable table/tree of the **loaded** subgraph, one tab stop + neighbor traversal + `/` search, `role="status"` for filter/lod changes (not every hover).
- `aria-hidden` on the canvas **only** when the equivalent is present.
- Visible focus ring drawn in flow coordinates; Escape leaves the widget.
- Edge/node type in words or patterns, not color alone.
- Keyboard search-to-focus is the same path as the mouse search box.

**Warning signs:**
- Tab skips the atlas or traps inside it.
- Screen reader says "image" or "canvas."
- No table/tree toggle in the panel chrome.

**Phase to address:**
Phase 26 (do not ship a mouse-only canvas). Phase 31 adds an a11y stepper (keyboard search + status text).

---

### Pitfall 12: Shipping Agent Flow as the atlas (or merging the two)

**What goes wrong:**
`AgentFlow` is a fixed SVG ring of agents around Orchestrator — no pan, zoom, search, or minimap. It is registered next to `VoxGraphStatusPanel` in `surfaceComponents.tsx`. Reusing it, its store, or its "Flow" panel id produces a surface that cannot show crates/files/symbols and trains users that "the graph" is the agent topology.

**Why it happens:**
Both are "graphs in Chat." `@xyflow/react` is already imported. The Flow panel close/resurrect tests are the nearest template.

**How to avoid:**
- New panel id, new components, new query keys. Copy **only** the closed-panel test pattern.
- Docs and UI copy say **Code Atlas** / codebase graph. Never "Flow."
- No shared node type union with agents.

**Warning signs:**
- Atlas PR edits `AgentFlow.tsx`.
- One React Flow instance with mixed agent + symbol nodes.

**Phase to address:**
Phase 28 (panel registration). Called out in every phase's scope line.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| `onlyRenderVisibleElements` on the full 47k array | Looks like virtualization | Store/layout still O(n); freeze remains | Never as the only scale strategy |
| Draw raw Leiden `community` | Clustering appears "done" | Hairball; false architecture | Never without size floor + hierarchy |
| Pulse on `isStreaming` | Demo "liveness" | Permanent distrust | Never |
| Persist camera in dockview `params` | One storage key | Already stripped; silent data loss | Never |
| New graph library beside xyflow | WebGL fps in a prototype | Two renderers, a11y, e2e, bundle | Only after Phase 26 measurement against culled subgraphs |
| Self-added crate-edge `exceptions` | Compiles | Policy break; CI ratchet lies | **Never** (user-authorized-only) |
| `WebviewWindow` pop-out in v1.4 | Matches the word "pop-out" | Isolated heap, capability, no mentions | Defer; same-webview floating only |
| Check in `graph.json` (47k) | Realistic CI | Stale, huge, non-portable | Never |
| Collapse all Vox kinds to `fn`/`module` | One icon set | Atlas useless on `.vox` repos | Never |
| Skip Playwright stepper | Faster PR | Violates GUI visual-verification invariant | Never for this surface |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| `vox-graph-reader` | Reimplement BFS/lens/crate-map in the GUI | GUI calls reader/MCP; lenses stay in the reader |
| `vox_search_*` MCP | Revive retired `vox_graphify_status` | Status/rebuild/TTL already on `vox_search_*` (`VoxGraphStatusPanel`) |
| Chat composer `@` | Second mention grammar / raw node id | Reuse Loquela `/@([^\s]*)$/` + `list_repo_files` + `addContextRef` |
| Dockview / ChatSurface | Auto-recreate Atlas without `closedPanelIds` | Opt-in panel + resurrection test template |
| `useFreshness` / `LIVE_EVENT_FRESH_MS` | Use as corpus freshness | Orch-status / touch-event window only; corpus uses `built_at` + SHA + TTL |
| Tauri preview | Populated atlas in the browser | `backendAvailable()` empty state; `BackendUnavailableError` copy |
| File-touch / oplog | Assume GUI already listens | New typed event; oplog kind does not reach dockview today |
| Crate edges | GUI → graph-reader edge or exceptions edit | Layer check; `-types` / defactor; no self-exceptions |
| Agent Flow | Shared canvas / panel id | Separate surface forever |
| Playwright | Live Tauri / real cache | `installTauriMock` + synthetic graph |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Full corpus in React Flow store | Tab jank, frozen input | Lens + fetch cap; drill-in | ~1k–2k SVG nodes; **certain** at 47k |
| Client force-directed layout | Multi-second first paint; jumping nodes | Precomputed hierarchical positions | Hundreds of nodes with a running sim |
| `onlyRenderVisibleElements` without LOD | Memory high; pan still hitchy | Do not put 47k in the store | Immediately at Vox scale |
| `nodeTypes` recreated each render | Full remount; xyflow `002` | Module-level map | Any custom node |
| `fitView` on huge graph | Camera jumps to unreadable scale | Fit the **loaded** subgraph only | First paint |
| Pulse / hover → full store copy | Jank while chatting | Immerse pulse in node `data` for ≤ N highlighted ids | A few streaming highlights |
| Proxy-per-node a11y layer | AT tree = 47k buttons | Traverse loaded LOD + table | Thousands of proxies |
| Rebuild-on-every-turn | UI stalls; TTL contract fights you | Explicit rebuild; overlay for live | First long chat |
| Minimap drawing all edges | Second hairball | Minimap = crate tiles only | Same as main canvas |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Absolute / worktree / `.vox/` paths in composer, toasts, or screenshots | Path + machine disclosure; secret-like filenames | Repo-relative ids; `sanitizeErrorForToast`; review-bundle redaction |
| Raw graph id (`path::sym`) as `@` text | Prompt injection / confusing tool routing | Shared mention helper; chips for symbols |
| Auto-submit on node click | Unintended agent run with attached context | Insert only; user sends |
| `WebviewWindow` with `windows: ["*"]` capability | Pop-out inherits full IPC | Exact label; defer pop-out |
| Logging full `graph.json` in GUI | Huge PII-ish corpus in logs | Log counts + sha + lens |
| Treating node labels as trusted HTML | XSS in custom xyflow nodes | Text children only |
| Rebuild/TTL write from preview | No-op or surprise contract edit | Disabled when `!backendAvailable()`; TTL already dirties a tracked contract |
| New LLM vendor call for "graph summary" | Breaks model-agnostic facade | `vox_actor_runtime::llm` only; v1.4 should not need a new LLM call |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Hairball first paint | "This is useless" | Crate tiles + search; drill-in |
| Stale graph, live chrome | Wrong edits trusted | SHA + ref + built-at; stale tone |
| Pulse while streaming | Cries wolf | Pulse only on touch events |
| Atlas = Agent Flow | Cannot find code | Separate panel and name |
| 0×0 or blank canvas | Looks broken | Pixel height shell; honest empty in preview |
| Close Atlas, it returns | Feels haunted | `closedPanelIds` |
| OS pop-out that cannot `@` | Broken pairing | Same-webview float |
| Color-only edge kinds | Unreadable relations | Labels + dash/width + legend |
| Zoom unexpectedly expands packages | Lost mental map | Explicit drill; stable camera |
| `@id` that picker rejects | Mention does nothing | Repo-relative path / chip |

---

## "Looks Done But Isn't" Checklist

- [ ] **LOD:** First paint is crate/module tiles, not 47k symbols — verify store `nodes.length` is far below corpus `node_count`.
- [ ] **Identity:** Header shows ref + SHA + relative `built_at`; picker change changes payload — verify SHA ≠ "live."
- [ ] **Edge kinds:** Legend lists relation types; live pulse is not an edge kind — verify contract field exists.
- [ ] **Live pulse:** Streaming without touches does not animate — verify event fixture.
- [ ] **Preview honesty:** Browser build shows `BackendUnavailableError` empty state, not a demo graph.
- [ ] **Dock:** Close Atlas, trigger ChatSurface refresh — panel stays gone (`closedPanelIds`).
- [ ] **Layout:** Cold start after schema change falls back (no stuck `fromJSON` throw).
- [ ] **Mentions:** Click inserts `@relative/path` via Loquela helper; no `::` ids; no auto-send.
- [ ] **Vox kinds:** `.vox` `workflow`/`actor`/`component` render as themselves.
- [ ] **A11y:** Summary + table/tree + keyboard search-to-focus; one tab stop; Escape exits.
- [ ] **Playwright:** `e2e` stepper + review-bundle screenshots; `installTauriMock` only.
- [ ] **Crate edges:** No exceptions row unless the user wrote it.
- [ ] **Not Flow:** `AgentFlow.tsx` untouched.

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| 47k freeze shipped | HIGH | Feature-flag atlas; ship modules lens only; delete client force layout |
| Hairball communities | MEDIUM | Hide community layer; draw crate-map; add size floor |
| False live chrome | MEDIUM | Strip pulse CSS; show SHA/stale; add touch event or "overlay unavailable" |
| Dock resurrection | LOW | Add `closedPanelIds`; bump layout key to `v4`; clone VoxGraph test |
| `fromJSON` throw loop | LOW | Catch already present; bump key; wipe that localStorage key |
| Path leak in mentions | MEDIUM | Switch helper to relative paths; sanitize toasts; rotate any leaked secrets if real keys appeared |
| Accidental `WebviewWindow` | HIGH | Remove window + capability; keep dockview float |
| Crate-edge exception landed | HIGH | Revert exceptions; defactor or `-types`; do not tighten-over a bad row |
| Agent Flow merge | HIGH | Split panel ids; revert shared store |
| 47k fixture in git | MEDIUM | `git rm` cache; replace with synthetic shaped fixture |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 47k raw draw / store | 26 (API in 25) | Store size assert; 1k shaped fixture; no freeze stepper |
| Leiden hairball | 25 | Community count ≪ nodes after floor; crate tiles on first paint |
| Untyped edges | 25 | Contract + legend test; pulse ≠ edge kind |
| Stale as live / snapshot ≠ branch | 25 + 27 | Header SHA/ref; picker changes payload |
| False pulses | 29 | Streaming-without-touch stepper |
| Dock persist / resurrection / 0×0 | 28 | Close-and-refresh test; CDP pane size |
| Tauri pop-out / capability | 28 | No `WebviewWindow` in v1.4; float stays in-webview |
| `@` injection / path leak | 30 | Mention helper unit tests; toast sanitizer |
| Vox vs Rust kinds | 25 + 30 | `.vox` fixture glyphs |
| 47k real cache in CI | 31 | Fixture policy test (path deny) |
| Canvas a11y | 26 + 31 | Keyboard search + summary role=status |
| Atlas = Agent Flow | 28 | `AgentFlow.tsx` untouched; distinct panel id |
| Crate-edge exceptions | 25 (any Rust API) | `vox ci crate-edges` clean without new exceptions |
| Preview lies | 29 + 31 | Mock-less preview empty state |
| Second graph library | 26 | Measurement note before any new dep |

---

## Sources

**Vox-measured (HIGH):**

- `.planning/workstreams/code-atlas/CONTEXT.md` — 47,111 / 49,775 / 20,653; untyped edges; SHA `c5d162b9`; Agent Flow ≠ atlas; no GUI file-touch event; `@xyflow/react` + dockview already installed.
- `crates/vox-gui/ui/src/components/dock/DockWorkspaceShell.tsx` — `dockview_layout.v3`, `fromJSON` try/catch, `params` stripped, `dndStrategy="pointer"`, 0×0 `.dv-shell`, no popout-drag, `closedPanelIds` is host-local.
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.test.tsx` — resurrection tests (VoxGraph, Flow, Repository, …); `fromJSON` throw on leftover sessions layout.
- `crates/vox-gui/ui/src/hooks/useFreshness.ts` + `config/constants.ts` — `LIVE_EVENT_FRESH_MS = 10_000` (orch-status, not corpus).
- `crates/vox-gui/ui/src/lib/backendGuard.ts` — preview empty states; `sanitizeErrorForToast`.
- `crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx` — `@` matcher, `list_repo_files`, `LOQUELA_FILE_PICKER_LIMIT`, `addContextRef`.
- `crates/vox-graph-reader/src/lens.rs`, `gc.rs` — `collapse_to_modules`, `pick_lens`.
- AGENTS.md — Playwright visual-stepper invariant; crate-edge exceptions user-authorized-only; no `docs/src/archive/` ingest.

**Libraries (MEDIUM, Context7):**

- xyflow `onlyRenderVisibleElements`, `getNodesInside`, error `002` (`nodeTypes` identity): https://github.com/xyflow/xyflow
- dockview `toJSON`/`fromJSON`, floating vs async popout (`popoutRestorationPromise`): https://github.com/dockview/dockview
- Tauri 2 `WebviewWindow` permissions + capability isolation: https://tauri.app/reference/javascript/api/namespacewebviewwindow

**Ecosystem (MEDIUM when cross-checked; do not treat unverified Exa as sole authority):**

- Hairball / layout-second: Peixoto, "Untangling the hairball using statistical inference" (https://skewed.de/lab/posts/hairball/)
- Leiden vs Infomap on software graphs: arXiv:2102.04710
- Semantic zoom + minimap: ExplorViz arXiv:2510.00003
- Hierarchical community tiles vs hairball: Uncharted "Multi-Scale Community Visualization of Massive Graph Data"
- IDE LOD / ~200 nodes per fetch: Travsr graph webview redesign (github.com/Travsr-com/travsr#319)
- Canvas/graph a11y: WCAG 2.1 (w3.org/TR/WCAG21); Accessible Data Interfaces "Accessible Network & Graph Diagrams"; IBM canvas_content_described

---
*Pitfalls research for: v1.4 Code Atlas (Graphify + Tauri dockview)*
*Researched: 2026-10-09*
