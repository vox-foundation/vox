# Project Research Summary

**Project:** Code Atlas (v1.4 `code-atlas` workstream)
**Domain:** Navigable visual codebase graph (Graphify) docked beside Chat
**Researched:** 2026-10-09
**Confidence:** HIGH on stack, seams, and measured hazards; MEDIUM on live-event producers and worktree overlay mechanics

## Executive Summary

Code Atlas is a **Sourcetrail interaction model on a CodeSee-style collapsed crate map**, docked like Search Index, with Cody-style `@` from a click and Gource-style pulses once a real touch event exists. Experts who ship usable code graphs never draw the full symbol corpus. They project a containment tree (crate → file → symbol) plus a typed reference overlay, then paint only the current zoom, focus, or neighborhood. Vox already has that projection home in `vox-graph-reader` (`crate_model`, `lens::collapse_to_modules`, `overlay`, `history/git`, `compare`). The GUI job is a budgeted subgraph canvas, not a second graph product.

**Recommended approach:** stay on viewport-culled `@xyflow/react` 12 subgraphs (already installed; used today only by `SubAgentGraph`). Extend `vox-graph-reader` — do not add a crate, a JS graph model, or a WebGL toolkit. Serve an `AtlasQuery` → `AtlasSubgraph` LOD API (hard cap, default ~400 nodes) over existing MCP. Dock a new opt-in `codeAtlas` panel on pinned `dockview@6.6.1`. Reuse Loquela `insertAtFile` for click-to-`@`. Extract `.vox` declaration-head kinds in a new `ast_vox.rs` beside `ast.rs` / `ast_ts.rs`, not via a `vox-compiler` crate edge. Layout is **deterministic packed regions from `crate_model`**, not elkjs and not a force sim. elkjs@0.12.0 stays an escape hatch after crate-level visual QA fails.

**Operator override (reconciles FEATURES P2 vs “ship all”):** this milestone includes **every scoped capability** — data, atlas, branch/worktree compare, dockable widget, live sync, click-to-`@`, Vox-native kinds. FEATURES correctly marked live pulses and vs-`main` coloring as P2 because they are blocked on a missing event and a missing overlay. Keep those P2 items **in this milestone as later phases**, not a v1.4.x / next-milestone cut. Do not fake either: a pulse on `isStreaming` or a color overlay on a `main`-SHA snapshot trains the wrong demo.

**Key risks:** (1) stuffing 47,111 nodes into the React Flow store — `onlyRenderVisibleElements` is a second cull, not a scale strategy; (2) treating 20,653 Leiden communities as regions; (3) painting untyped `{source,target,confidence}` edges as a visual language; (4) presenting a 9-day-stale SHA as the live / current-branch graph; (5) pulsing without a typed `code_touch` event; (6) dock resurrection / 0×0 pane; (7) OS `WebviewWindow` pop-out that cannot mention or pulse. Mitigate by shipping the lens API before the canvas, hierarchy before paint, identity chrome on every payload, the event before animation, opt-in `closedPanelIds` before a sixth core panel, and **same-webview floating only**.

## Key Findings

### Recommended Stack

See [STACK.md](./STACK.md). **Additions only.** Do not re-litigate `vox-graph-reader`, graphify corpora, Tauri 2, dockview Chat panels, or the existing `@xyflow/react` install.

A 47k-node / 50k-edge corpus is a **projection** problem. Sourcetrail, CodeSee/GitKraken Maps, CodeNib, and every atlas that stays usable change what “the graph” means at each zoom. Adding sigma, cytoscape, ngraph, or a force engine would fork the visual language, lose React `nodeTypes` (Vox kinds, pulses, click-to-`@`), and fight Playwright.

**elkjs contradiction (resolved):** STACK says do not add elkjs; the canvas still needs layout because xyflow does not assign positions. Use **precomputed hierarchical packed regions from `crate_model`** (stable crate geography across rebuilds). Add `elkjs@0.12.0` in a worker, cached by graph hash, **only if** crate-level edges are unreadable after visual QA. Never run ELK or any layout on the full corpus. Dagre is rejected (no compound graphs).

**Core technologies:**
- `@xyflow/react` **12.12.0** (repo `^12.11.6`; optional patch bump): canvas, `parentId` hierarchy, custom `nodeTypes`/`edgeTypes`, MiniMap, Controls, `setCenter`, `onlyRenderVisibleElements` — already the SubAgentGraph renderer; React 19.2.8 is a supported peer. Dual Zustand (app v5 + xyflow’s internal v4) is expected; do not dedupe.
- `dockview` **6.6.1** (pinned; latest 8.4.1 is a ChatSurface rewrite): new opt-in panel id; `addFloatingGroup` for in-webview float. Do not bump.
- `vox-graph-reader` (in-tree): typed edges, containment tree, LOD `pub fn`, branch overlay. GUI never receives `graph.json`.
- `@tanstack/react-query` **5.102.8**: `useAtlasSubgraph({ corpus, rev, zoom, focus, viewport })` beside existing `useVoxGraphStatus`.
- `zustand` **5.0.15**: tiny touched-id set for pulses. Do not subscribe overlay chrome to the full `nodes` array.
- Playwright `^1.62.1` + `installTauriMock`: required visual stepper + review-bundle.

**Stack additions this milestone (no new graph toolkit):**
- New reader modules: `lod.rs` (`AtlasQuery` / `AtlasSubgraph`), `ast_vox.rs`
- New MCP tool: `vox_search_atlas`
- New turn-event kind: `code_touch`
- New GUI folder: `components/surfaces/CodeAtlas/`
- Extracted `lib/composerMentions.ts` (Loquela + atlas share `insertAtFile`)
- Optional: `@xyflow/react` patch bump inside existing range
- **Not added:** elkjs, sigma, cytoscape, dagre, isomorphic-git, dockview 8, new Rust crate, new state library, new mention grammar

### Expected Features

See [FEATURES.md](./FEATURES.md). Gold standard is Sourcetrail (search → local typed graph → expand) on CodeSee collapsed folders, docked beside Chat.

**Already shipped — do not treat as new:** `VoxGraphStatusPanel` freshness cards, Agent Flow SVG (stays separate), Repository isolation dropdown (reuse identity only), Chat dockview, CLI `vox graph query/coverage/crate-map/history`, `collapse_to_modules` / crate-map / overlay, Loquela `@`, `@xyflow/react`.

**Must have (table stakes — missing these = still a status card):**
- Semantic zoom (crate → file → symbol) — zoom changes **what is drawn**
- Search-to-focus via existing `vox graph query` + `setCenter`
- Typed-edge visual language (`call` / `import` / `impl` / `test` + `contains`) + legend
- Meaningful hierarchy (Cargo crate → module/file → symbol; drop Leiden-as-cluster)
- Dockable / floating atlas beside Chat
- Branch / worktree picker bound to a graph identity + freshness chip
- Minimap + controls + keyboard pan/zoom/activate

**Should have (Vox differentiators — all in this milestone):**
- Click-to-`@`mention — reuse chips; no second syntax (P1, cheap)
- Vox-native node kinds on `.vox` extract (P1)
- Worktree vs `main` compare coloring (P2 **later phase**, not next milestone)
- Live agent-touch pulses + optional follow-camera (P2 **later phase**, not next milestone)

**Defer (v2+ / not in the approved seven capabilities):**
- Local-graph depth slider beyond a 1–3 cap if 1-hop is too tight
- Coverage / test-target lens (CLI already exists; not default paint)
- Gource replay, CodeSee tours, CodeScene health maps
- SCIP / cross-repo go-to-def
- WebGL / second renderer
- OS-level `WebviewWindow` pop-out (see pitfalls — not the same as “floating”)

**Anti-features (do not schedule):** draw all 47k nodes; force-layout the corpus; merge Agent Flow; new `@graph:` grammar; continuous force sim; infinite-depth trail; self-added crate-edge `exceptions`.

### Architecture Approach

See [ARCHITECTURE.md](./ARCHITECTURE.md). Code Atlas is a **read API + docked canvas** over the existing Graphify corpus. Layering: L5 GUI (`CodeAtlasView` on ChatSurface dockview) → L4 MCP (`vox_search_atlas` + `code_touch`) → L5 CLI graphify atlas subcommand → L1 `vox-graph-reader` (extend, do not fork) → `.vox/cache/graphify/`.

**Major components:**
1. **`vox-graph-reader`** — typed `ExtractedEdge.kind`; `contains` from `id.split("::")` + crate path prefix; `lod::query(AtlasQuery)`; `ast_vox.rs` declaration-head scan; overlay attributes (`touched` / `added` / `removed` / `changed`). Leiden stays for CLI crate-map blast-radius only.
2. **`vox-orchestrator-mcp` `graph_tools`** — serve budgeted subgraphs; never clone 47k (~126 MB parsed) into the GUI. `from_ref` / `get_graph` cache; no `vox-gui` → `vox-graph-reader` crate edge.
3. **`CodeAtlasView` (NEW)** — xyflow canvas, semantic zoom, search-to-focus, minimap, pulse overlay. New folder; do not grow `VoxGraphStatusPanel` into a canvas; do not reuse `AgentFlow` / `SubAgentGraph`.
4. **ChatSurface dock panel `codeatlas`** — opt-in, same instance as Search Index; default right of transcript; layout via `DockWorkspaceShell` (`dockview_layout.v3`, bump to `v4` if the default set changes).
5. **`composerMentions` helper** — file nodes insert `@path`; symbol nodes insert `@path` + chip `ref` = graph id. Composer text stays path-shaped.
6. **`code_touch` turn event** — producers: agent loop / tool receipts for writes and search hits. Atlas maps `path` → file id (`id.split("::").next()`) and `symbol` → qualified id.

**Key patterns:** hierarchical page queries (LOD); contains-from-id; overlay don’t fork; MCP as the GUI/graph boundary. Snapshots (`snapshot.rs`) are **rebuild history**, not branches. Worktrees are skipped by `walk_source_files` (`.worktrees` is hidden) — indexing a worktree must pass that tree as `source_dir`.

### Critical Pitfalls

See [PITFALLS.md](./PITFALLS.md). Top hazards for the roadmapper:

1. **Drawing the 47k-node graph (or stuffing it into the React Flow store)** — `onlyRenderVisibleElements` still keeps every node in Zustand, hit-testing, and diffs. Serve a lens. Positions from server-side hierarchical layout. Keep `nodeTypes` / `edgeTypes` as module-level constants (xyflow error `002`).
2. **Leiden 20,653 communities as visual regions** — ~1 cluster per 2.3 symbols. Hierarchy-first: crate → file → symbol. Semantic zoom changes glyph kind, not just scale.
3. **Untyped edges** — cannot show *how*. Closed `edge.kind` enum before any canvas: `calls | imports | implements | crate_dep | contains | mentions` (plus `tests`). Color-only styles fail WCAG 1.4.1.
4. **Stale snapshot presented as live / current-branch** — measured cache is `c5d162b9` (2026-09-30), 9 days stale on research day. `LIVE_EVENT_FRESH_MS` is an orch-status window, not a corpus clock. Every payload carries `{corpus_id, git_sha, ref_name, worktree_id, built_at, lens}`.
5. **False live pulses** — no file/symbol-touch event reaches the GUI today. Ship `code_touch` (or an explicit “overlay unavailable”) **before** animation. Never pulse on `isStreaming` or status poll.
6. **Dock resurrection + 0×0 pane** — atlas is opt-in with `closedPanelIds`; assert pixel height before mounting React Flow; do not persist camera in dockview `params`.
7. **Floating ≠ Tauri `WebviewWindow`** — v1.4 = docked panel + `addFloatingGroup` in the **same** webview. OS pop-out isolates the JS heap, needs a capability not in `core:webview:default`, and breaks mentions/pulses.

Also watch: `@` injection of raw `path::symbol` ids; collapsing `.vox` kinds to `fn`; testing against the real 47k cache; canvas with no a11y equivalent; merging Agent Flow.

## Implications for Roadmap

**One milestone, seven phases (continue numbering after docs-freshness Phase 24).** Do not split pulses or vs-`main` coloring into a future milestone. Do not start the canvas before typed edges + LOD exist.

### Phase 25: Graph data floor
**Rationale:** Typed edges and a real containment tree unblock every later visual claim. Painting first is a lie.
**Delivers:** `ExtractedEdge.kind` in graphify output; `contains` + crate membership; `ast_vox.rs` + `.vox` in `walk_source_files`; `EXTRACTOR_VERSION` bump; `lod::query` (`AtlasQuery` / `AtlasSubgraph`, default limit 400); payload identity `{corpus_id, git_sha, ref_name, worktree_id, built_at, lens}`; `vox_search_atlas` MCP + optional `vox graph atlas` CLI. Leiden IDs stay off the atlas camera.
**Addresses:** Typed edges, meaningful hierarchy, Vox extract (data half), freshness identity.
**Avoids:** Pitfalls 2, 3, 4 (schema/identity); crate-edge `exceptions`; new `vox-code-atlas` crate.
**Stack:** extend `vox-graph-reader` only; adjacent tests on every new `pub fn`.

### Phase 26: Viewport-culled atlas canvas
**Rationale:** Once the lens exists, the Sourcetrail/CodeSee interaction model can ship without Chat chrome.
**Delivers:** `CodeAtlasView` on `@xyflow/react` 12; semantic zoom crate → file → symbol; search-to-focus (`vox graph query` → `setCenter` + 1-hop); MiniMap (crate tiles only) + Controls + legend; keyboard pan/zoom/activate; a11y summary + table/tree of the **loaded** subgraph; `toFlow` helpers; synthetic fixtures (tiny + ~1k shaped). Deterministic packed regions from `crate_model`. `onlyRenderVisibleElements` **after** the subgraph is small.
**Addresses:** Semantic zoom, search-to-focus, minimap, legend, keyboard.
**Avoids:** Pitfall 1 (47k in store); pitfall 11 (mouse-only canvas); force layout; second renderer.
**Uses:** xyflow 12 built-ins, react-query page keyed `[atlas, corpus, level, focus, tip]`.
**QA gate (not a stack add):** if crate-level DAG is unreadable, then and only then add `elkjs@0.12.0` worker + hash cache. Do not introduce dagre.

### Phase 27: Branch / worktree compare
**Rationale:** Picker without compare is still valuable; compare coloring is the approved P2 that must land in this milestone once identity exists. FEATURES deferred this to “after validation” — operator keeps it here as a later phase.
**Delivers:** Atlas picker of `main` + listed worktrees (not the Repository isolation dropdown); load or rebuild that **graph identity**; overlay vs `main` as node/edge data (`added` / `removed` / `changed`); layout stays `main` crate regions so the map does not jump; honest stale tone when SHA ≠ selected ref HEAD. Optional explicit rebuild with `source_dir` = worktree (`with_graph_lock`, one at a time).
**Addresses:** Branch/worktree picker + vs-`main` coloring (W5).
**Avoids:** Pitfall 4 (overlaying a worktree on a `main`-SHA snapshot and calling it that worktree).
**Implements:** `history/git.rs` (`rev_parse`, `file_deltas`, `ls_tree`) + `compare.rs` + `overlay.rs`. No JS git.

### Phase 28: Dockable widget beside Chat
**Rationale:** The success picture is a widget next to the transcript. Register only after the canvas has a sized parent story.
**Delivers:** Opt-in dockview panel `codeatlas` on 6.6.1; default right of transcript; `addFloatingGroup` in-webview float; `closedPanelIds` + resurrection test cloned from VoxGraph; layout key `v3` → `v4` if the default set changes; pixel-height assert before React Flow; Search Index / `VoxGraphStatusPanel` kept as freshness chrome (optional “Open Code Atlas” action). Same React tree as Chat — mentions and pulses still work.
**Addresses:** Dockable / floating atlas (W1).
**Avoids:** Pitfalls 6, 7, 12 (resurrection, OS pop-out, Agent Flow merge).
**Explicit non-goal this phase:** Tauri `WebviewWindow` / `addPopoutGroup` / `unstable` child webviews. “Pop-out” in the brief means floating-in-app, not a new OS window.

### Phase 29: Live sync
**Rationale:** Pulses are P2 because the event is missing — not because they are out of milestone. Ship the contract before CSS.
**Delivers:** `code_touch` in `contracts/gui/turn-event-kinds` + producers (write/edit/search receipts); zustand touched-id set; pulse + ~1.5s fade on on-page nodes; off-screen “touched” chip that focuses; optional follow-camera (pin/unpin); `freshnessTone` `live` only within `LIVE_EVENT_FRESH_MS` of **that event**; preview / `backendAvailable() === false` shows honest empty, never fake pulses.
**Addresses:** Live sync + chat-driven camera (W4).
**Avoids:** Pitfall 5 (false live); rebuild-on-every-turn.
**Research flag:** yes — no producer reaches dockview today.

### Phase 30: Click-to-`@` and Vox-native chrome
**Rationale:** Mentions are the cheapest Chat↔graph loop; Vox glyphs are why this atlas is not a Rust-only costume. Extractor landed in 25; chrome lands here so legend/icons are keyed by `(source_lang, kind)`.
**Delivers:** Shared `composerMentions` helper; click inserts `@<repo-relative-path>` + chip; no `::` ids, no absolute paths, no auto-submit; Vox glyphs for `component` / `table` / `workflow` / `actor` / …; legend tables; fixture `.vox` must not render as `fn`.
**Addresses:** Click-to-`@` (W3), Vox-native kinds (W6).
**Avoids:** Pitfalls 8, 9 (mention injection, kind confusion).

### Phase 31: Honesty and visual proof
**Rationale:** GUI visual-verification invariant + the 47k freeze must be proven with synthetic fixtures, not the developer cache.
**Delivers:** Playwright `e2e/code-atlas.spec.ts` + review-bundle screenshots via `installTauriMock`; steppers for dock, search-to-focus, close-and-refresh (no resurrection), streaming-without-touch (no pulse), preview empty, keyboard search + `role="status"`, mention path shape, Vox glyphs; reader tests with `serde_json` blobs; shaped ~1k fixture + 47k **count** that never materializes 47k DOM nodes; store-size assert `nodes.length` ≪ corpus `node_count`.
**Addresses:** Verification of W1–W6; pitfall 10.
**Avoids:** Checking in `.vox/cache/graphify/graph.json`; e2e against live Tauri / real cache.

### Phase Ordering Rationale

- **Data → canvas → identity chrome → dock → live → mentions/glyphs → proof.** Dependencies in FEATURES.md are real: typed edges before the visual language; hierarchy before semantic zoom; LOD before xyflow; `code_touch` before pulses; same-webview dock before click-to-`@`.
- P2 pulses and vs-`main` coloring stay **inside** this milestone (Phases 27 and 29) because the user approved all seven capabilities. They stay **later** because faking them is worse than shipping them last.
- Dock is after the canvas so React Flow mounts into a known shell, but before live/mentions so those features attach to a real panel.
- Vox extract is in 25 (walker/schema) and Vox chrome is in 30 (glyphs/legend) so Phase 26 can render kinds generically.
- Coverage lens and depth slider are not in the approved seven — do not inflate the roadmap.

### Research Flags

Phases likely needing `/gsd-plan-phase --research-phase` during planning:
- **Phase 27:** worktree-as-identity vs commit-keyed corpus; `ls_tree` overlay vs opt-in `source_dir` rebuild; how to color removed nodes without a second full graph.
- **Phase 29:** which orchestrator/tool receipts become `code_touch`; session scoping; mapping path → node when the corpus SHA ≠ disk.

Phases with standard patterns (skip research-phase):
- **Phase 25:** in-tree seams (`ast.rs`, `lens`, `crate_model`, MCP `graph_tools`) are read and opinionated.
- **Phase 26:** xyflow 12 + SubAgentGraph `toFlow` + react-query status hook are established. Re-open only if the elkjs QA gate trips.
- **Phase 28:** ChatSurface opt-in panels + `closedPanelIds` + DockWorkspaceShell are documented and tested.
- **Phase 30:** Loquela `@` / `insertAtFile` / `addContextRef` already exist; Vox keyword list is the compiler declaration-head set.
- **Phase 31:** Playwright `installTauriMock` + review-bundle is repo policy, not a new domain.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | npm 2026-10-09 versions + in-repo `package.json` + Context7 xyflow/dockview. MEDIUM only on elkjs deferral and Tauri pop-out (both resolved as non-adds). |
| Features | MEDIUM | Cross-product consensus (Sourcetrail / CodeSee / Cody / Gource) is consistent. FEATURES author deferred pulses and compare; **operator overrode** to keep them in-milestone as later phases. |
| Architecture | HIGH | Seams read in-tree (`vox-graph-reader`, MCP, ChatSurface, Loquela, turn-event kinds). MEDIUM on dockview OS pop-out (disabled / `pointer` DnD). |
| Pitfalls | HIGH | Vox-measured: 47,111 / 49,775 / 20,653; SHA `c5d162b9`; no GUI touch event; dock `fromJSON` / resurrection / 0×0. Library a11y/WebGL limits MEDIUM. |

**Overall confidence:** HIGH enough to roadmap; MEDIUM on the two P2 producer/overlay designs (Phases 27 and 29).

### Gaps to Address

- **`code_touch` producer list:** no file/symbol event reaches dockview today. Phase 29 planning must name the exact receipts (`vox_write_file`, edits, search hits) and fail closed when absent (“overlay unavailable”), not pulse on stream ticks.
- **Worktree graph identity:** snapshots are rebuild history. Phase 27 must choose overlay-from-`ls_tree` vs explicit worktree rebuild as `source_dir`, and refuse to label a `main` SHA as that worktree.
- **elkjs QA bar:** define “crate-level edges unreadable” before anyone adds 1.5 MB + a worker. Default remains packed regions.
- **Vox edge kinds beyond node kinds:** `uses` / `auth` / `@test` overlays are nice if obvious at extract time; do not block glyphs on a full Vox call-graph.
- **Floating vs pop-out copy:** UI chrome should say “float,” not “pop out to a window,” so we do not promise `WebviewWindow`.
- **Coverage / depth slider:** not in the approved seven. If a phase has leftover budget after 1-hop, a 1–3 depth cap is the only in-scope extra.

## Sources

### Primary (HIGH confidence)
- `crates/vox-gui/ui/package.json` — `@xyflow/react ^12.11.6`, `dockview 6.6.1`, `react ^19.2.8`, react-query 5, zustand 5, Tauri API 2.11.1
- npm registry 2026-10-09 — xyflow 12.12.0; dockview latest 8.4.1 (ignore); elkjs 0.12.0; sigma 4 / cytoscape 3.34.3 / dagre 3.1.1 (rejected)
- In-repo seams — `vox-graph-reader` (`lens`, `crate_model`, `overlay`, `history/git`, `compare`, `snapshot`); `ChatSurface` / `DockWorkspaceShell`; `Loquela.tsx` `insertAtFile`; `VoxGraphStatusPanel`; `useFreshness` / `LIVE_EVENT_FRESH_MS`; turn-event kinds; cached `repo-code-graph` schema (47,111 / 49,775 / 20,653, untyped links, SHA `c5d162b9`)
- [CONTEXT.md](../CONTEXT.md) — seven approved capabilities; Agent Flow ≠ atlas; no GUI touch event
- xyflow performance guide — collapse trees, memo nodes, do not subscribe to full `nodes`; `onlyRenderVisibleElements` is render-only
- AGENTS.md — Playwright visual invariant; crate-edge exceptions user-authorized-only; model-agnostic LLM facade (atlas does not call models)

### Secondary (MEDIUM confidence)
- Context7 `/xyflow/xyflow`, `/dockview/dockview`, `/kieler/elkjs`
- Sourcetrail `DOCUMENTATION.md` — search / typed edges / legend / neighborhood
- CodeSee / GitKraken Maps — collapsed folders, PR Review Map colors
- Sourcegraph Cody — `@` file/symbol chips (no canvas)
- CodeNib — containment tree + reference overlay + `max_nodes`
- GitLens / GitKraken — worktree pills, history minimap (steal picker, not the commit DAG)
- Gource — pulse-on-touch only; not the home view
- Obsidian / Logseq / lxDIG — local/focus + visible-node budget; do not steal global WebGL force
- Tauri 2 `WebviewWindow` capability isolation
- WCAG 2.1 + canvas a11y notes — color-not-only, keyboard, summary/`role="status"`
- Leiden-on-software-graphs (arXiv:2102.04710); hairball / layout-second literature

### Tertiary (LOW confidence)
- Plugin-specific Obsidian/Logseq FPS claims — pattern only
- Travsr ~200 nodes/fetch — anecdotal LOD budget, aligns with our ≤400 cap

---
*Research completed: 2026-10-09*
*Ready for roadmap: yes*
