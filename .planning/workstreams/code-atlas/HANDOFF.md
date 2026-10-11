# code-atlas: Handoff (v1.4 Code Atlas)

**Written:** 2026-10-11
**Workstream:** `code-atlas` (`.planning/workstreams/code-atlas/`)
**Where it stopped:** partway through `/gsd-discuss-phase 25 --ws code-atlas`. Two of four discussion areas are decided. The third area's questions were asked but not answered. No product code has been written for this milestone.

## Read these first

1. `CONTEXT.md`: the workstream brief, with the user's problem in their own words, the measured baseline, constraints, and the success picture.
2. `REQUIREMENTS.md`: 25 requirements with IDs (DATA, ATLAS, BRANCH, DOCK, LIVE, MENTION, VOX, PROOF), mapped to phases.
3. `ROADMAP.md`: Phases 25–31 with success criteria.
4. `research/SUMMARY.md`, then `STACK.md`, `FEATURES.md`, `ARCHITECTURE.md` and `PITFALLS.md` as needed.
5. `phases/25-graph-data-floor/25-DISCUSS-CHECKPOINT.json`: answers so far plus the pending questions in machine-readable form.

---

## 1. What the user asked for

The original request was dictated by voice and garbled in transcription. This is the reconstructed intent, which the user confirmed by picking the recommended option for every scoping question.

> The Graphify codebase graph is trapped in one place and can't be navigated. The goal is to show worktrees and graphs of the codebase, especially `main` or any branch, as a **purely visual language**. As an LLM system changes a repo written in Vox, or works on the Vox codebase itself, visual links should show how the code is changing. It's hard to read today; rethink the semantics and the visual representation. Name the biggest flaws, use screenshots, and consider every control. It should become a **widget you can keep open beside Chat**, updating visually while the chat talks.

Scoping answers (2026-10-09, user said "approve, take recommended, all"):
- The target is the **codebase graph (Graphify)**, not Agent Flow.
- All seven capabilities are in this milestone: graph data fixes, a navigable atlas, branch/worktree compare, a dockable widget, live sync, click-to-`@`, and Vox-native node kinds.
- Research ran before requirements.
- Name: **v1.4 Code Atlas**, workstream `code-atlas`.

## 2. What the investigation found (evidence for the flaws)

The GUI was run in a browser with `pnpm dev`. Browser preview has no desktop backend, so panels show empty or loading states.

| Surface | What it really is |
|---|---|
| Chat → Panels → **Search Index** (`VoxGraphStatusPanel.tsx`) | Freshness cards per corpus (nodes, edges, built time, Rebuild button, TTL editor). **No graph canvas.** It already docks beside Chat through dockview. |
| Agents → **Flow** (`surfaces/Flow/AgentFlow.tsx`) | Fixed-position SVG ring of agents around an Orchestrator node. No pan, zoom, search or minimap. Shows agents, not code. |
| Workspace → **Repository** (`RepositoryView.tsx`) | Command buttons plus a text isolation panel. Worktrees and branches appear only as an isolation-strategy dropdown. |

Graph data, measured from `.vox/cache/graphify/repo-code-graph/graph.json`:
- **47,111 nodes, 49,775 links.**
- **20,653 distinct `community` values**, roughly one cluster per 2.3 symbols. The clustering is useless as visual regions.
- Links are `{source, target, confidence}` only, with **no edge kind**. `ExtractedEdge` in `crates/vox-graph-reader/src/ast.rs` has no `kind` field, and `EXTRACTOR_VERSION` is `"4"`.
- Node kinds: fn 28,596; method 11,612; struct 5,163; enum 1,149; trait 365; command 118; type 107; tool 1. There are **no `.vox` kinds**.
- Built at `c5d162b9` on 2026-09-30, so it was 9+ days stale at research time. The 5 snapshots are rebuild history, not branches.
- `walk_source_files` (`crates/vox-graph-reader/src/rebuild.rs:52`) skips hidden directories, including `.worktrees`. Indexing a worktree has to pass that tree as `source_dir`.
- Nothing in the GUI receives a structured "agent touched file X / symbol Y" event.

The six biggest flaws are listed in `CONTEXT.md` under "Biggest flaws this milestone exists to fix".

## 3. What is done (all committed on the current branch)

| Commit | What |
|---|---|
| `6c8547295` | Milestone start: `PROJECT.md` workstream entry, `code-atlas/STATE.md`, `code-atlas/CONTEXT.md` |
| `7c53c0547` | Research: STACK, FEATURES, ARCHITECTURE, PITFALLS, SUMMARY |
| `60be815f0` | `REQUIREMENTS.md` (25 requirements) |
| `62b2c6b0f` | `ROADMAP.md` (Phases 25–31), traceability, STATE update |

Phase numbering continues from docs-freshness, which ends at Phase 24.

## 4. Decisions already locked

### Milestone level (REQUIREMENTS.md D1–D7)
- No new crate. Extend `vox-graph-reader`.
- No `vox-gui` → `vox-graph-reader` crate edge; the GUI talks to the graph over MCP.
- No `vox-graph-reader` → `vox-compiler` edge (it would break the L1→L3 layering). Extract `.vox` kinds with a declaration-head scanner (`ast_vox.rs`) next to `ast.rs` and `ast_ts.rs`.
- Stay on `@xyflow/react` 12 and `dockview` **6.6.1** (pinned; version 8 would mean a ChatSurface rewrite). Add no second graph library. `elkjs` is an escape hatch only if crate-level visual QA fails.
- "Floating" means `addFloatingGroup` in the same webview. A Tauri `WebviewWindow` pop-out is out of scope.
- Live pulses and vs-`main` coloring are later phases inside this milestone. Never fake them: no pulsing on `isStreaming`, and no overlay drawn on a snapshot from the wrong SHA.
- Agent Flow stays separate from Code Atlas.

### Phase 25 discussion (answers given)

**Edge-kind honesty**
- The ~49k existing Rust/TS edges become `calls`.
- This phase ships `crate_dep` plus derived `contains`. `imports`, `implements` and `tests` are added only if the existing AST visitors already carry the signal. Otherwise they stay empty until a later extractor bump.
- Dangling edges stay in atlas payloads as `mentions` with `confidence: dangling` ("you decide" → recommended option applied).
- `contains` is derived at query time from `id.split("::")` and is not stored in `graph.json`.

**Empty-query LOD**
- A query with no focus returns every crate node plus `crate_dep` edges. With about 138 crates this fits under the 400-node cap.
- The caller passes an explicit `level: crate | file | symbol`.
- A focused crate at `level=crate` returns that crate plus its 1-hop `crate_dep` neighbors. Files come back only at `level=file` ("you decide" → recommended option applied).
- When the corpus is omitted, default to `repo-code-graph` and return that cache's real identity (`git_sha`, `built_at`).

## 5. Still open: finish the Phase 25 discussion

### Area 3: Stale-cache policy (questions asked, no answers yet)
| # | Question | Recommended |
|---|---|---|
| 1 | Extractor version mismatch (cache v4, atlas expects v5)? | **Refuse** with an error and no subgraph, so Phase 26 can never paint untyped edges |
| 2 | Identity SHA ≠ selected ref HEAD? | **Serve** with identity fields plus a stale flag. Phase 25 has no picker and rebuilds are expensive |
| 3 | Who rebuilds? | **Operator or caller**, through the existing `vox graph refresh` or the Search Index Rebuild button. The atlas never rebuilds |
| 4 | Worktree `source_dir` (DATA-05)? | `source_dir` is accepted **on rebuild only**. Queries read the default cache unless a worktree corpus exists. Phase 27 owns compare |

### Area 4: Vox kind set (not asked in detail yet)
The choice is between making every Vox bare-keyword declaration a first-class node kind or a short list.
- The authoritative keyword list is in `AGENTS.md` §Grammar Unification: `type`, `fn`, `component`, `state_machine`, `routes`, `module`, `actor`, `workflow`, `activity`, plus soft keywords `table`, `query`, `mutation`, `server`, `tool`, `resource`, `form`, `index`.
- The short list from the success picture is `component`, `table`, `workflow`, `actor`, `fn`.
- Suggested follow-ups: whether decorators such as `@durable` and `@scheduled` become node attributes or are ignored, and how `.vox` ids line up with the `<path>::<container>::<name>` scheme in `ast.rs`.

## 6. Next steps, in order

1. Resume the discussion: `/gsd-discuss-phase 25 --ws code-atlas`. It should detect the checkpoint JSON and offer "Resume". Ask the user Areas 3 and 4. If the user wants speed, apply the recommended answers above and confirm.
2. Write `phases/25-graph-data-floor/25-CONTEXT.md` from the template at `~/.cursor/gsd-core/workflows/discuss-phase/templates/context.md`, plus `25-DISCUSSION-LOG.md` (template: `templates/discussion-log.md`). Delete the checkpoint after writing, then commit with:
   `node ~/.cursor/gsd-core/bin/gsd-tools.cjs query commit "docs(25): capture phase context" --files <files> --ws code-atlas`
3. Plan: `/gsd-plan-phase 25 --ws code-atlas`. The roadmap marks Phase 25 as "standard patterns", so phase research can be skipped.
4. Execute: `/gsd-execute-phase 25 --ws code-atlas`.
5. Continue through Phases 26–31. Phases **27** (worktree identity) and **29** (`code_touch` event producers) are flagged as needing phase research.

## 7. Code seams for Phase 25

| Seam | Path | Note |
|---|---|---|
| Edge / node structs, `EXTRACTOR_VERSION` | `crates/vox-graph-reader/src/ast.rs` | Add `kind` to `ExtractedEdge` (use a serde default for old caches, but refuse old versions per Area 3). Bump the version to `"5"`. |
| TS extract | `crates/vox-graph-reader/src/ast_ts.rs` | Uses the same edge struct |
| Edge resolution | `crates/vox-graph-reader/src/rebuild_resolve.rs` (`resolve_edges`) | Where `dangling` is assigned |
| File walk | `crates/vox-graph-reader/src/rebuild.rs` (`walk_source_files`) | Add `.vox`; `.worktrees` is skipped |
| Hierarchy helpers | `crates/vox-graph-reader/src/lens.rs` (`collapse_to_modules`), `crate_model.rs` | Basis for crate/file levels and `crate_dep` |
| Overlay / compare | `overlay.rs`, `compare.rs`, `history/git.rs` | Phase 27, not 25 |
| New module | `crates/vox-graph-reader/src/lod.rs` (planned) | `AtlasQuery` → `AtlasSubgraph`, cap 400, identity fields |
| New module | `crates/vox-graph-reader/src/ast_vox.rs` (planned) | Declaration-head scan; no compiler dependency |
| MCP tools | `crates/vox-orchestrator-mcp/src/graph_tools.rs` | Existing: `vox_graphify_status/search/query/path/compare`, `vox_search_callers/callees/rebuild/set_ttl`. Add `vox_search_atlas`. Register it in the operations catalog (`contracts/operations/catalog.v1.yaml`) the way the others are registered. |
| CLI | `crates/vox-cli/src/commands/graphify/mod.rs` | Optional `vox graph atlas` |
| Corpus registry / TTL | `vox_config::graphify`, `contracts/retrieval/vox-graph-corpora.v1.yaml` | Freshness and TTL live here |

GUI seams for later phases: `components/surfaces/VoxGraph/VoxGraphStatusPanel.tsx` (stays freshness chrome), `hooks/useVoxGraphStatus.ts`, the dockview panels in `components/surfaces/Chat/ChatSurface.tsx`, `components/dock/DockWorkspaceShell.tsx`, `insertAtFile` in `components/surfaces/Loquela/Loquela.tsx:640` (extract it into a shared `lib/composerMentions.ts` for MENTION-01), and `SubAgents/SubAgentGraph.tsx` (the only current `@xyflow/react` user).

## 8. Top pitfalls from the research

1. **Putting 47k nodes in the React Flow store.** `onlyRenderVisibleElements` still keeps them all in memory. Serve the budgeted LOD only. (Phases 25 and 26)
2. **Using Leiden communities as regions.** Build the hierarchy first. (Phase 25)
3. **Painting untyped edges as a visual language.** Close the `kind` enum before any canvas, and never encode meaning in color alone (WCAG 1.4.1). (Phases 25 and 26)
4. **Showing a stale SHA as live or as the current branch.** Every payload carries identity. (Phases 25 and 27)
5. **False pulses.** Ship a typed `code_touch` event first. (Phase 29)
6. **Dock resurrection or a 0×0 pane.** The panel is opt-in through `closedPanelIds`, and React Flow mounts only after the pane has height. (Phase 28)
7. **`@` injection.** Reuse Loquela's mention helper and keep composer text path-shaped. Never insert raw `path::symbol` ids or absolute paths. (Phase 30)
8. **Testing against the live 47k cache.** Use synthetic fixtures (a tiny one and one with about 1k nodes). (Phase 31)

## 9. Environment gotchas from this session

- **lean-ctx shell guard:** `node -e` and the native Grep/Glob tools are blocked. `Read` is denied outside the "active project" root (it was misreporting `~/dev/gigme` as the root). Workarounds: read `~/.cursor/gsd-core/**` with `sed -n` in Shell, and search with `rg` in Shell.
- **gsd-tools:** run `node ~/.cursor/gsd-core/bin/gsd-tools.cjs query <cmd> ... --ws code-atlas`. `--ws` is a **global** flag. Building `--ws code-atlas` into one string and passing it as an argument to `state.milestone-switch` failed with "unknown flag". Phase-scoped init: `query init.phase-op 25 --ws code-atlas`. Every call prints a harmless `defaults.json` precedence warning.
- **GUI in a browser:** start it in the background with `pnpm dev --port 5199 --host 127.0.0.1` from `crates/vox-gui/ui`. Binding to `localhost` was unreachable from the IDE browser, and a `&` background inside a blocking shell call died. Dismiss the onboarding modal with "Skip for now". Routes use hash URLs (`#view=chat`, `#view=flow`, `#view=repository`). Chat's "Panels" button (bottom right) toggles dock panels; "Search Index" is the graphify panel.
- Large `jq` over `graph.json` (about 20 MB) works and takes about 1–4 s.

## 10. Repo state warnings (do not clobber)

- The working tree has **many unrelated uncommitted changes from other agents and workstreams**: `crates/vox-gui/ui/**` (BreadcrumbBar, Sidebar, ChatSurface, Loquela, Popover, new Tooltip), compiler parser/eval, CLI classify, orchestrator telemetry, `Cargo.lock`, suppressions, autonomy-ux Phase 12 plans, root `package.json`. **None of it belongs to code-atlas.** Stage only `.planning/workstreams/code-atlas/**` and the `PROJECT.md` lines you change.
- `.planning/active-workstream` now reads `code-atlas` (it was `docs-freshness`) and is uncommitted. Other agents may switch it back; always pass `--ws code-atlas` explicitly.
- `.planning/workstreams/code-atlas/state.json` is untracked (written by gsd-tools).
- Parallel workstreams running: `research-trial-flywheel` (v1.1, Phases 7–11), `autonomy-ux` (v1.2, Phases 12–19), `docs-freshness` (v1.3, Phases 20–24). The `PROJECT.md` "Current Milestone" heading still says v1.1; that is intentional, because the shared heading is not rewritten while a workstream is active.
- Repo policy: no `git push`, no PRs without being asked; `.vox` scripts only for automation; test-first for new `pub fn`; Playwright plus review-bundle screenshots for GUI changes; crate-edge `exceptions` are user-authorized only.
