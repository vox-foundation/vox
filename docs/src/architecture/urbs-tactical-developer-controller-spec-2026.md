---
title: "URBS Tactical Developer Controller & Zero-Hop Code/Agent Governance Specification (2026)"
description: "Architecture specification for Milestone v4.9 (Phases 271–276): transforming VRBS from an ambient visualizer into a daily developer controller with zero-hop inline diff/diagnostic loupes, multi-agent collision radar, clickable viaduct contracts, hard agent scope fencing, milestone time-scrubbing, and split-dock workbench integration."
category: "Architecture SSOTs"
status: "current"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-6"
---

# URBS Tactical Developer Controller & Zero-Hop Code/Agent Governance Specification (2026)

<!-- kb:summary -->
Defines the Milestone v4.9 (Phases 271–276) architecture that elevates VRBS from a macro codebase visualizer into an indispensable daily developer controller. Covers six end-to-end capabilities: (1) Zero-Hop Inline Diff & Compiler Diagnostic Loupe (*Speculum Codicis*) with hunk stage/revert and exact error dispatch; (2) Multi-Agent Write-Set Collision & Blast-Radius Cone Radar (*Concursus Legionum*) with worktree isolation and agent serialization; (3) Clickable Viaduct Cross-Module Contract & Layer-Inversion Inspector (*Pons Foederis*); (4) Hard Agent Scope Quarantine Fencing (*Clausura Ferrea*) enforced in the backend MCP tool runner; (5) Interactive Milestone Road Turn Time-Scrubber (*Via Temporis* visual `git bisect`); and (6) Split-Dock Tactical Map (`⌘M` *Tabula Praetoria*) synchronized bi-directionally with Chat and Review.
<!-- /kb:summary -->

## 1. Problem Statement & Design Mandate

Even with authentic Vitruvian Roman mechanical semiotics and repo-agnostic polyglot discovery (`v4.7`, `Phases 259–264`), a codebase map remains **60% ambient telemetry and 40% controller** unless a developer can resolve daily engineering problems **without leaving the map**:

1. **Zero-Hop Code & Error Inspection:** Seeing `M parser.rs +42/-11` on a Treadwheel Crane or `2 ERR` on a Trip-Hammer Forge tells you *where* a change or failure occurred, not *what* changed or *why* it failed. Developers must be able to read the unified diff hunks and exact compiler diagnostics (`file:line:col`) directly in an inline **Parchment & Bronze Code Loupe (*Speculum Codicis*)** and stage, revert, or dispatch a targeted fix in one click.
2. **Predictive Multi-Agent Collision Prevention:** When 2+ LLM agents operate concurrently, their biggest failure mode is silent write-set overlap or upstream contract breakage. The map must project each agent's **claimed file write-set + downstream blast-radius cone** and raise a **Contested Viaduct / Shared-File Collision Standard (*Concursus Legionum*)** with 1-click `[Serialize Agents]` or `[Isolate to Git Worktree]`.
3. **First-Class Boundary & Contract Inspection:** Software bugs concentrate on **module boundaries** (viaducts). Clicking any stone viaduct between two districts must reveal the **exact cross-module imports/exports**, **broken downstream call sites**, and **architectural layer inversions (`L1 → L3`)**.
4. **Hard Backend Guardrails (Not UI-Only Locks):** Locking a district on the map must drop a **Wrought-Iron Quarantine Portcullis (*Clausura Ferrea*)** that is enforced inside `vox-orchestrator-mcp`'s tool execution gate—physically rejecting `write_file` / `edit_file` / destructive shell calls targeting quarantined directories.
5. **Visual `git bisect` Across Agent Turns:** When an agent breaks tests on Turn 3 of a 6-turn run, scrubbing the **Stone Milestone Road (`I → II → III → IV`)** must replay per-checkpoint file diffs, test shield states (`PASS` vs. `FAIL`), and token burn with 1-click worktree rollback.
6. **Always-Accessible Split-Dock Workbench Integration (`⌘M`):** A controller buried under `Settings → Gamify` is invisible during active coding. `VRBS` must mount as a **collapsible split-dock Tactical Map (`⌘M`)** alongside Chat and Review, where clicking a district filters the Chat/Review stream and every live tool call animates its corresponding mechanism on the map.

---

## 2. Six-Phase Architecture (`Phases 271–276`)

| Phase | Mechanism (*Vitruvian Name*) | Backend (`vox-gui` / `vox-orchestrator-mcp`) | Frontend (`crates/vox-gui/ui/src/components/gamify/urbs/`) |
|---|---|---|---|
| **Phase 271** | **Inline Diff & Compiler Diagnostic Loupe (*Speculum Codicis*)** | `workspace_file_diff_and_diagnostics` Tauri command (`git diff HEAD -- <path>` unified hunks + `git checkout -- <path>` revert + `git add <path>` stage + structured compiler/LSP diagnostics `line`, `col`, `code`, `message`). | `SpeculumLoupePopover.tsx`: anchored over any clicked/hovered Treadwheel Crane or red error building; renders syntax-colored `+added / -deleted` hunks, exact `file:line:col` compiler errors, and `[✓ Stage File]`, `[↺ Revert File]`, `[⚒ Fix Exact Error]`. |
| **Phase 272** | **Multi-Agent Write-Set Collision & Cone Radar (*Concursus Legionum*)** | Extend `vox_orchestrator_status` / `agentTasks` with `planned_write_paths: Vec<String>` and expose `vox_resolve_agent_collision` (`serialize` via `ThreadRunQueue` or `isolate_worktree` via `DelegationWorktree`). | `collisionRadar.ts`: computes pairwise agent write-set + downstream blast-cone intersections (`computeAgentCollisions`); `worldRenderer.ts` draws intersecting amber/crimson surveyor cones and a **Hammered-Bronze Clash Standard (`⚠ COLLISION`)** with `[⇄ Serialize]` and `[🌿 Split Worktree]`. |
| **Phase 273** | **Clickable Viaduct Contract & Layer-Inversion Inspector (*Pons Foederis*)** | Extend `workspace_town_scan` with `edges: Vec<TownDependencyEdge>` (`from`, `to`, `import_files`, `imported_symbols`, `is_layer_violation`: `from_layer < to_layer`). | Hit-test viaduct bezier/line segments in `LudusSandbox.tsx` (`selection: { kind: 'viaduct', from, to }`); `worldRenderer.ts` renders cracked red arch keystones on illegal layer inversions; `ImperiumInspector.tsx` renders the `PONS CONTRACTVS` boundary inspector. |
| **Phase 274** | **Hard Agent Scope Quarantine Fencing (*Clausura Ferrea*)** | Add `quarantined_paths: Arc<RwLock<HashSet<String>>>` on `ServerState` (`vox_quarantine_set` / `vox_quarantine_list`) and enforce pre-execution rejection in `agent_loop.rs` whenever `write_file`, `edit_file`, or `patch` targets a quarantined module root. | `Alt+Click` on any district (or clicking `[🔒 Quarantine District]` in `ImperiumInspector`) drops a **3D Wrought-Iron Perimeter Portcullis & Bronze Padlock** around the district's curtain wall and syncs with `vox_quarantine_set`. |
| **Phase 275** | **Milestone Road Turn Time-Scrubber (*Via Temporis* Visual `git bisect`)** | Enrich `vcsCheckpointList` (`GitPlumbingCheckpointStore`) with per-checkpoint `touched_files`, `diff_added`, `diff_deleted`, `test_passed: Option<bool>`, and `cost_usd` delta (`vcs_checkpoint_diff_summary`). | `ViaTemporisScrubber.tsx`: bottom paved Roman Milestone timeline bar (`I`, `II`, `III`, `IV`) with a draggable bronze sundial pointer that previews historical building diffs & test shield pass/fail states on the canvas, with `[↺ Rollback Worktree to Milestone]`. |
| **Phase 276** | **Split-Dock Tactical Map Workbench Integration (*Tabula Praetoria `⌘M`*)** | Emit real-time `urbs-tool-activity` events from `chat_turn.rs` / `agent_loop.rs` (`read_file` → Scroll Armarium, `edit_file` → Treadwheel Crane, `cargo check` → Trip-Hammer Forge, `cargo test` → Proving Parapet). | Register `urbs-tactical-map` as a first-class dockable panel in `DockWorkspaceShell.tsx` (`⌘M` toggle); clicking a district filters Chat/Review by module affinity, and live chat tool calls animate the target district's Vitruvian mechanism. |
