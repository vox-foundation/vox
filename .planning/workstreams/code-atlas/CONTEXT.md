# code-atlas — Workstream Brief (v1.4 Code Atlas)

## Problem (user's words, paraphrased)

The Graphify codebase graph is trapped. You cannot navigate it. Most of the goal is to represent worktrees and graphs of the codebase — especially `main`, or any branch that existed at a given time — as a **purely visual language**. As a large language system changes a repository written in Vox, or works on the Vox codebase itself, visual links should show *how* that information is changing. Right now it is hard to read. We need better semantics and a better visual representation so it is actually useful. The atlas must become a **widget** you can pair or paste and keep open beside Chat, updating visually while the conversation talks.

## Decisions from scoping (2026-10-09)

| Decision | Choice |
|---|---|
| Placement | New GSD workstream `code-atlas`, milestone **v1.4**, phases **25+** (continue numbering after docs-freshness Phase 24) |
| Target | **Codebase graph (Graphify)** — crates / modules / files / symbols. Not the Agent Flow topology (that stays a separate surface). |
| Outcomes in scope | (1) graph data fixes: typed edges, real hierarchy/clustering, freshness; (2) navigable Code Atlas: semantic zoom, search-to-focus, minimap; (3) branch/worktree picker + diff overlay vs `main`; (4) dockable / floating / pop-out widget beside Chat; (5) live sync: graph reacts to what the chat agent is touching; (6) click node → `@`-mention into the chat composer; (7) Vox-native node kinds (`component`, `table`, `workflow`, `actor`, …) for `.vox` repos |
| Research | Run parallel Stack / Features / Architecture / Pitfalls research before requirements |
| Rendering | Prefer already-installed `@xyflow/react` + dockview; do not add a second graph library unless research proves 47k-node semantic zoom cannot be done by viewport-culled subgraphs |

## Measured baseline (2026-10-09)

- `VoxGraphStatusPanel` is a **freshness card grid** (nodes / edges / built-at / Rebuild / TTL). No canvas. It already docks beside Chat as the "Search Index" panel.
- `AgentFlow` is a fixed-position SVG ring of agents around Orchestrator — no pan, zoom, search, or minimap; it is **not** a code graph.
- `RepositoryView` is command buttons + a text isolation panel. Worktrees/branches appear only as a strategy dropdown.
- Cached `repo-code-graph` (`.vox/cache/graphify/repo-code-graph/graph.json`): **47,111 nodes**, **49,775 edges**, **20,653 communities** (~1 cluster per 2.3 symbols), keyed to `c5d162b9` (2026-09-30). Edges have `source` / `target` / `confidence` only — **no kind**.
- `vox-graph-reader` already has history, lineage, coverage, crate-map, what-if, overlay, and lens (`collapse_to_modules`) — CLI/MCP only.
- Chat already has dockview panels (`Chat`, `Execution`, `Search Index`, `Repository`, …). No structured "agent touched file X / symbol Y" event reaches the GUI.

## Existing machinery to extend (do not rebuild)

- `crates/vox-graph-reader` — graph load, BFS, coverage, crate model, history/lineage, overlay, lens.
- `crates/vox-cli/src/commands/graphify/` — CLI + rebuild pipeline.
- Graphify corpora under `.vox/cache/graphify/` (`repo-code-graph`, `crate-map`, `vox-gui-surface`, `vox-config-graph`).
- GUI: `VoxGraphStatusPanel`, `useVoxGraphStatus`, dockview in `ChatSurface`, `@xyflow/react` already a dependency (used today only by `SubAgentGraph`).
- MCP / Tauri: `vox_search_*` tools; GUI reads status through MCP, not a retired `vox_graphify_status` command.
- Chat composer already has `@` for agents/files — the click-to-mention path should reuse that, not invent a second mention syntax.

## Hard constraints (repo policy)

- LLM calls stay on `vox_actor_runtime::llm`. No vendor hostnames.
- New crate edges need layer compliance; `exceptions` are user-authorized-only.
- GUI visual changes need Playwright stepper specs + review-bundle screenshots.
- Automation is `.vox` via `vox run`. Secrets via `vox_secrets::resolve_secret`.
- Do not ingest `docs/src/archive/`.
- Test-first for new `pub fn`s.

## Biggest flaws this milestone exists to fix

1. No code-graph view exists in the GUI.
2. Clustering is meaningless (20,653 communities).
3. Edges have no type, so the visual language cannot show *how* things relate.
4. 47k nodes cannot be drawn raw — needs levels + on-demand drill-in.
5. No branch/worktree dimension; snapshots are rebuild history, not branches.
6. Nothing connects chat to the graph.

## Success picture

A user opens Chat, docks Code Atlas beside the transcript, picks `main` or a worktree, sees crates as regions and files as containers, searches a symbol and the camera flies there, watches nodes pulse as the agent edits them, and clicks a node to drop it into the composer as `@` context. The same widget works for a repo written in Vox.
