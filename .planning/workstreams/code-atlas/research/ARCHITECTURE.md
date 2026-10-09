ARCHITECTURE.md 499L
---
... [lean-ctx: omitted 1 lines]
description: "How Code Atlas integrates into vox-graph-reader, graphify, and the Vox GUI without a new crate or illegal crate edges."
category: "architecture"
... [lean-ctx: omitted 2 lines]
# Architecture Research
**Domain:** Codebase-graph atlas (Graphify) inside Vox GUI + CLI
**Researched:** 2026-10-09
**Confidence:** HIGH (seams read in-tree); MEDIUM (dockview pop-out — currently disabled)
## Standard Architecture
Code Atlas is a **read API + docked canvas** over the existing Graphify corpus. It is not a second graph product and not Agent Flow.
```
+------------------------------------------------------------------+
| L5 GUI (vox-gui / Tauri)                                         |
|  ChatSurface dockview                                            |
|  +------------+  +----------------+  +-------------------------+ |
|  | Transcript |  | CodeAtlasDock  |  | Search Index            | |
|  | + Loquela  |  | (NEW xyflow)   |  | VoxGraphStatusPanel     | |
|  +-----+------+  +--------+-------+  +-------------------------+ |
|        | @-mention        | subgraph + focus + pulse             |
+--------+------------------+--------------------------------------+
| L4 MCP (vox-orchestrator-mcp) -- EXISTING edge to graph-reader   |
|  vox_search_*  +  NEW vox_search_atlas                           |
|  chat turn events  +  NEW code_touch                             |
+------------------------------------------------------------------+
| L5 CLI (vox-cli graphify) -- EXISTING edge                       |
|  rebuild / query / history / NEW atlas subcommand                |
+------------------------------------------------------------------+
| L1 vox-graph-reader -- EXTEND, do not fork                       |
|  ast (+ ast_vox) | lod | lens | crate_model | history/lineage    |
|  overlay | snapshot (rebuild history, not branches)              |
+------------------------------------------------------------------+
| On-disk: .vox/cache/graphify/{repo-code-graph,crate-map,...}     |
|  graph.json + snapshots/ + history store s{N}-e{EXTRACTOR_VER}   |
+------------------------------------------------------------------+
```
... [lean-ctx: omitted 1 lines]
### Component Responsibilities
| Component | Responsibility | Typical Implementation |
... [lean-ctx: omitted 1 lines]
| `vox-graph-reader` | Typed edges, contains-hierarchy, LOD subgraph, Vox decl kinds, overlay vs tip | Extend existing crate (L1) |
| `vox-cli` `graphify` | Rebuild pipeline, atlas CLI, history ingest | Modify `commands/graphify/` |
| `vox-orchestrator-mcp` | Serve subgraph/search/history to GUI; emit `code_touch` | Modify `graph_tools.rs` + turn-event contract |
| `CodeAtlasView` (NEW) | Semantic zoom canvas, search-to-focus, minimap, pulse | `@xyflow/react` already installed |
... [lean-ctx: omitted 4 lines]
## Decision answers (the seven questions)
### 1. Typed edges + hierarchy: extend `vox-graph-reader`, do not add a crate
**Use the existing crate.** It already owns extraction (`ast.rs`, `ast_ts.rs`), rebuild, clustering, lens collapse, crate-map, overlay, and history. A new crate would need a `layers.toml` row and a **user-authorized** crate-edge exception for every consumer (`vox-cli`, `vox-orchestrator-mcp`).
... [lean-ctx: omitted 4 lines]
3. Keep **calls** as today’s directed `source→target` (caller→callee). Add kinds as extractors can prove them: `calls`, `contains`, `imports` (TS), `implements` (trait impl), `boundary` (`invoke` / `callTool` already special-cased).
4. Stop using Leiden-at-symbol as the atlas community. 20,653 communities (~1 per 2.3 symbols) is not a visual language. Atlas clusters are **crates → modules/files**. Reuse `crate_model::build_crate_map` for the top level and `lens::collapse_to_modules` as the mid level. Leiden stays for `vox graph crate-map` blast-radius, not the atlas camera.
... [lean-ctx: omitted 1 lines]
### 2. Subgraph / LOD API: server-side, viewport-culled, four levels
**Never send the full graph to the GUI.** 47k nodes / 50k edges is ~126 MB parsed; `from_ref` exists specifically to avoid cloning that.
... [lean-ctx: omitted 8 lines]
```rust
pub struct AtlasQuery {
    pub level: AtlasLevel,          // crate | module | file | symbol
    pub focus: Vec<String>,         // node ids, crate names, or search seeds
    pub branch_tip: Option<String>, // git SHA or worktree id
    pub kinds: Vec<String>,         // edge-kind filter; empty = all
    pub limit: usize,               // hard cap (default 400)
}

pub struct AtlasSubgraph {
    pub nodes: Vec<AtlasNode>,      // id, label, kind, parent, pulse?
    pub links: Vec<AtlasLink>,      // source, target, kind, weight?
    pub parent: Option<String>,     // drill-out target
    pub truncated: bool,
}
```
... [lean-ctx: omitted 3 lines]
### 3. Branch / worktree: overlay diffs, not N full graphs
**Snapshots are rebuild history** (`snapshot.rs`: copy `graph.json` + manifest, `gc` keeps newest N). They are not branches. History (`history/`) is a first-parent commit chain with `ChangeRec` (path + optional `SymbolDelta`) and `LineageRec` (rename/split/merge/move). `Git::ls_tree(tip)` already answers “what exists at this SHA.”
**Default atlas dimension:** pick a tip (`main` HEAD, or a worktree HEAD). Paint the **current corpus graph** (keyed to a SHA today, e.g. `c5d162b9`) plus an **overlay** of added/removed/changed files and symbols from `history` between `main` and that tip.
... [lean-ctx: omitted 3 lines]
| Rebuild from a worktree `source_dir` | Opt-in “index this worktree” | Full rebuild; already supported by `Index` / `RebuildMeta.scope_path` |
... [lean-ctx: omitted 3 lines]
Worktrees are skipped by `walk_source_files` (`.worktrees` is hidden). Indexing a worktree must pass that tree as `source_dir`, not walk the main checkout.
### 4. Dockview panel + optional floating / pop-out
**New dock panel, do not replace Search Index.**
... [lean-ctx: omitted 2 lines]
- Default layout: atlas right of transcript (same “pair beside Chat” slot Search Index already occupies). User can drag; layout persists via `DockWorkspaceShell` (`dockview_layout.v3`).
... [lean-ctx: omitted 1 lines]
- **Pop-out:** `DockWorkspaceShell` explicitly uses `dndStrategy="pointer"` and documents **no popout-drag flows**. Treat OS pop-out as a later phase; do not block the widget on it.
`VoxGraphStatusPanel` stays the freshness card grid (`export function VoxGraphStatusPanel`). Optional header action: “Open Code Atlas” → `addPanel({ id: 'codeatlas' })`.
... [lean-ctx: omitted 1 lines]
### 5. Live sync from orchestrator / chat stream
**No file/symbol-touch event reaches the GUI today.** Known turn-event kinds (`contracts/gui/turn-event-kinds` / `turnEvents.ts`): `skill_activated`, `delegation_spawned`, `research_milestone`, `tool_receipt`, `receipt_claims`, `routing_decision`, `research_trace`. Tasks have `write_files` but that is hopper state, not a stream.
... [lean-ctx: omitted 4 lines]
Producers: orchestrator agent loop / tool receipts for `vox_write_file`, edits, and search hits. Atlas maps `path` → file node (`id.split("::").next()`) and `symbol` → qualified id, then **pulses** those nodes on the current subgraph. If the node is off-page, show a “touched off-screen” chip that focuses.
... [lean-ctx: omitted 1 lines]
Until `code_touch` exists, a thin adapter can watch `tool_receipt` + task `write_files` — enough for a first pulse, not enough for symbol-accurate camera moves.
### 6. `@`-mention reuse
Chat already embeds Loquela as `composer`. Loquela owns `@`:
... [lean-ctx: omitted 1 lines]
- Agents → `insertAt` (`@${agent.id} ` + chip)
- Files → `list_repo_files` + `insertAtFile` (`@${path} ` + `addContextRef`)
**Extract** `insertAtFile` / chip append into a shared helper (e.g. `lib/composerMentions.ts`) used by Loquela and `CodeAtlasView.onNodeClick`. File nodes insert `@path`; symbol nodes insert `@path` plus a chip whose `ref` is the graph id (composer text stays path-shaped so existing send payload `context: chips[]` still works).
... [lean-ctx: omitted 1 lines]
### 7. Vox-native kinds: declaration-head extractor in graph-reader, not `vox-compiler`
Graphify does **not** parse `.vox` today. `walk_source_files` allows `rs|ts|tsx|js|jsx|py` only. Node kinds from Rust are `fn|method|trait|struct|enum|type`. Compiler `ast_eval` already counts `component|table|workflow|actor|fn|type|…` — that is the vocabulary to mirror, not a crate to import.
... [lean-ctx: omitted 1 lines]
**Do:** add `ast_vox.rs` beside `ast.rs` / `ast_ts.rs`. Scan declaration-head keywords the compiler already uses (`component`, `table`, `query`, `mutation`, `server`, `tool`, `resource`, `form`, `index`, `fn`, `type`, `actor`, `workflow`, `activity`, `state_machine`, `module`, `routes`). Emit those as `ExtractedNode.kind` and `contains`/`calls` when a call site is obvious. Include `.vox` in the walk. Bump `EXTRACTOR_VERSION`.
... [lean-ctx: omitted 1 lines]
## Recommended Project Structure
```
crates/vox-graph-reader/src/
├── ast.rs              # MODIFY: ExtractedEdge.kind; keep id scheme
├── ast_ts.rs           # MODIFY: emit edge kinds
├── ast_vox.rs          # NEW: .vox declaration-head kinds
├── rebuild.rs          # MODIFY: walk .vox; persist kinds; hierarchy edges
├── lens.rs             # KEEP/EXTEND: collapse_to_modules as LOD mid
├── crate_model.rs      # KEEP: crate-level atlas page
├── lod.rs              # NEW: AtlasQuery → AtlasSubgraph
├── overlay.rs          # EXTEND: tip-vs-main file/symbol overlay (today: test targets)
├── history/            # KEEP: lineage + ChangeRec for branch overlay
├── snapshot.rs         # KEEP: rebuild history only
└── lib.rs              # MODIFY: reader indexes node/edge kind

crates/vox-cli/src/commands/graphify/
├── mod.rs              # MODIFY: GraphifyCmd::Atlas { … }
└── history.rs          # KEEP

crates/vox-orchestrator-mcp/src/
├── graph_tools.rs      # MODIFY: vox_search_atlas
├── dispatch.rs         # MODIFY: route
└── chat_tools/chat/turn_events.rs  # MODIFY: code_touch

crates/vox-gui/ui/src/
├── components/surfaces/CodeAtlas/          # NEW
│   ├── CodeAtlasView.tsx
│   ├── CodeAtlasView.test.tsx
│   ├── CodeAtlasToolbar.tsx                # level, branch, search
│   └── atlasTypes.ts
├── components/surfaces/Chat/ChatSurface.tsx  # MODIFY: register panel
├── components/surfaces/VoxGraph/             # KEEP status panel
├── components/surfaces/Loquela/Loquela.tsx   # MODIFY: use shared mention helper
├── lib/composerMentions.ts                   # NEW (extracted)
└── e2e/code-atlas.spec.ts                    # NEW Playwright

contracts/gui/turn-event-kinds.v1.json        # MODIFY: add code_touch
```
### Structure Rationale
- **No new crate:** L1 already owns graph IO; splitting would multiply edges.
... [lean-ctx: omitted 1 lines]
- **New UI folder `CodeAtlas/`:** do not grow `VoxGraphStatusPanel` into a canvas (it is a freshness grid). Do not reuse `AgentFlow` or `SubAgentGraph`.
- **Mentions in `lib/`:** composer is Loquela-owned; atlas only calls the helper.
## Architectural Patterns
### Pattern 1: Hierarchical page queries (LOD)
**What:** Each camera level is a bounded subgraph computed on the reader, not a filter of a downloaded giant graph.
... [lean-ctx: omitted 3 lines]
```typescript
const page = await voxTransport.invokeMcpTool('vox_search_atlas', {
  level: 'module',
  focus: ['crates/vox-compiler'],
  limit: 400,
});
setNodes(toFlowNodes(page.nodes));
```
### Pattern 2: Contains-from-id (hierarchy without a second extract)
**What:** `id.split("::")` is file then containers then name (`lens::module_of`, overlay bare-name). Materialize `contains` edges at rebuild from that, plus crate membership from path prefix `crates/<name>/`.
... [lean-ctx: omitted 1 lines]
**Trade-offs:** Nested Rust `mod` vs file path can disagree; file path wins (already documented in `lens.rs`).
### Pattern 3: Overlay, don’t fork the graph
**What:** Live pulses and branch diffs are attributes on nodes (`touched`, `added`, `removed`) on the current page.
... [lean-ctx: omitted 1 lines]
**Trade-offs:** Overlay is wrong if the corpus SHA ≠ the files on disk; surface freshness from `useVoxGraphStatus` on the toolbar.
### Pattern 4: MCP as the GUI/graph boundary
**What:** GUI talks MCP/Tauri only. `vox-gui` does **not** depend on `vox-graph-reader`.
... [lean-ctx: omitted 1 lines]
**Trade-offs:** Cannot share Rust types with the UI; DTO in the MCP payload is the contract. Avoids a new crate-edge allowlist row.
## Data Flow
### Request flow (open atlas / drill-in)
```
User opens Code Atlas dock
    ↓
CodeAtlasView → invokeMcpTool('vox_search_atlas', { level: crate })
    ↓
vox-orchestrator-mcp::graph_tools (get_graph cache)
    ↓
vox_graph_reader::lod::query → AtlasSubgraph
    ↓
xyflow nodes/edges (≤ limit)
    ↓
User double-clicks crate → same tool, level: module, focus: [crate]
```
### Live pulse
```
Agent tool writes crates/foo/src/lib.rs
    ↓
orchestrator emits turn event code_touch
    ↓
Chat stream → CodeAtlasView (same session)
    ↓
Map path → node id; if on page, pulse; else off-screen chip
```
### Click-to-mention
```
Node click → composerMentions.insertAtFile(path) | insertSymbolChip(id)
    ↓
Loquela text/chips (existing send payload context[])
```
### Branch overlay
```
Toolbar picks worktree / main
    ↓
history ChangeRec + Git ls_tree(tip) vs main
    ↓
AtlasSubgraph nodes annotated added|removed|changed
    ↓
Optional: Rebuild with source_dir = worktree (explicit)
```
### State management
```
React Query: atlas page keyed [atlas, corpus, level, focus, tip]
useVoxGraphStatus: freshness (existing)
Session events: code_touch overlay (ephemeral, not persisted)
Dockview layout: localStorage v3 (existing)
```
### Key data flows
1. **Rebuild:** walk → extract (rs/ts/py/vox) → typed edges + contains → cluster crates/files → `graph.json` + snapshot. Invalidates `EXTRACTOR_VERSION` caches.
... [lean-ctx: omitted 1 lines]
3. **Search-to-focus:** `vox_graphify_search` / `vox_search_structural` → seed ids → atlas query at symbol/file level.
... [lean-ctx: omitted 2 lines]
## Scaling Considerations
| Scale | Architecture |
... [lean-ctx: omitted 2 lines]
| External indexed repo (`vox graph index`) | Same LOD; corpus id already in registry |
| Many worktrees | Overlay only; one opt-in rebuild at a time (`with_graph_lock`) |
| Chat-hot path | Pulse overlay; rebuild stays user/MCP `vox_search_rebuild` |
### Scaling priorities
1. **First bottleneck:** shipping 47k nodes to React. Prevention: LOD API + cap.
2. **Second bottleneck:** Leiden-at-symbol as visual clusters. Prevention: hierarchy levels replace community color for atlas.
... [lean-ctx: omitted 1 lines]
`GraphifyReader` construction is O(N+E) and already cached in `get_graph`. LOD must use that cache, not re-`from_ref` per wheel tick.
## Anti-Patterns
### Anti-Pattern 1: New `vox-code-atlas` crate
[… truncated at ~4056 of 5832 tokens — use ctx_read with lines= parameter to see specific sections]
