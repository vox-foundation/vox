# Milestones

## v5.5 T3 CI/TOESTUB Deep Enforcement — Trust Gates, Live Budgets, Host/Test Detectors & Fingerprint Hashing (Planning: 2026-10-25)

**Phases:** 307–312 · **Requirements:** 0/24

**Goal:** Extract the next tier of t3code CI/oxlint/agent-ops gems beyond v5.4 — PR vouch trust, privileged preview trust-split, live transfer-budget measurement + job-matrix outputs, wall-clock/real-wait/test-in-loop/frame-wait detectors, host-runtime + schema-hoist + knip dual deepen + DOM-env discipline, and true sidecar fingerprint hashing.

**Key deliverables (planned):**
- **Trust (`Phase 307`):** VOUCHED SSOT + pr-vouch labels
- **Preview policy (`Phase 308`):** deploy-vs-cleanup trust-split CI audit
- **Live CI (`Phase 309`):** real transfer artifacts + per-job change outputs
- **Test detectors (`Phase 310`):** wall-clock / real-wait / loop / frame-wait
- **Host/schema (`Phase 311`):** process/host boundary, schema hoist, knip dual, DOM-env
- **Closure (`Phase 312`):** fingerprint hash compare, surface-graph regen, scorecard

---


## v5.4 T3 CI/TOESTUB Enforcement Deep-Extraction & Harness Policy Hardening (Shipped: 2026-10-25)

**Phases planned:** 6 phases (`Phases 301–306`), 24 requirements
**Status:** Shipped — 6/6 phases, 24/24 requirements verified

**Goal:** Extract the remaining t3code CI/oxlint/agent-ops gems (change-gated checks, transfer-budget PR reporting, knip, PR size/evidence hygiene, kill-safety + multi-surface checklist, thin-transport TOESTUB, fingerprint/cache/graph closure) into Vox harness enforcement without redoing shipped v3.9/Wave runtime features.

**Key planned accomplishments:**

- **Change-gated CI (`Phase 301`):** Owner map + `vox ci check-changed` + inert-path job skipping
- **Transfer budgets & shard weights (`Phase 302`):** CI artifacts, trusted PR comments, weighted nextest/e2e shards
- **Hygiene (`Phase 303`):** Knip dual gate, PR size labels, evidence-asset ban
- **Agent-ops (`Phase 304`):** Kill-safety triad, hit-every-surface checklist, worktree isolation, suppression review
- **TOESTUB (`Phase 305`):** Thin-transport detector, suppression ratchet, snapshot interning, oxlint mapping
- **Closure (`Phase 306`):** Tauri fingerprint advisory, GUI cache layers, surface-graph regen, scorecard

---

## v4.7 Universal Repo-Agnostic Codebase Civilization Simulator & Attention-Weighted Developer Control Surface (Shipped: 2026-10-25)

**Phases completed:** 6 phases (`Phases 259–264`), 6 plans, 24/24 requirements verified (`100%`)
**Archive:** [`milestones/v4.7-ROADMAP.md`](milestones/v4.7-ROADMAP.md) | [`milestones/v4.7-REQUIREMENTS.md`](milestones/v4.7-REQUIREMENTS.md) | [`.planning/v4.7-MILESTONE-AUDIT.md`](v4.7-MILESTONE-AUDIT.md)

**Key accomplishments:**

- **Polyglot Repository Scanner (`workspace_town.rs`) + Live Git Diff (`+LoC / -LoC`) Enrichment (`Phase 259`):** Decoupled `workspace_town.rs` from Cargo-only prefixes to partition any repo structure (Cargo `crates/*`, TypeScript `packages/*`/`apps/*`/`services/*`, Go `cmd/*`/`internal/*`, Python `src/<pkg>`, single-app directory layouts) and enriched `TownScanDto`, `TownCrateDto`, and `TownFileDto` with live git working-tree diff telemetry (`git_status`, `diff_added_lines`, `diff_deleted_lines`, `dirty_file_count`, `total_churn_lines`, `primary_language`).
- **Zero-Hardcoding Graph Topology & Structural Archetype Classifier (`Phase 260`):** Implemented `computeTopologicalLayers` (Kahn's algorithm + Tarjan SCCs), `classifyDistrictArchetype`, `deriveCivicTitle`, and `computeBlastRadius` in `topology.ts`, eliminating all hardcoded `vox-*` crate name conditionals and static title lookups across `layout.ts`, `worldRenderer.ts`, and `LudusSandbox.tsx`.
- **Attention-Weighted Dynamic Real-Estate Layout (`Phase 261`):** Implemented `computeAttentionMultiplier` in `layout.ts` to inflate active, dirty, failing, and blast-radius districts (`1.0x`–`3.2x`) while compacting dormant modules (`0.55x`), allocated `>= 65%` of worksite interior plot footprint to modified/active files as interactive file slabs, and wired Real-Estate Focus Mode (`Attention-Weighted (Auto)` | `Worksite Only` | `Proportional Raw LoC`).
- **Interactive Foreground Worksite Cards & Priority Dock (`Phase 262`):** Rendered Live File Diff Slabs with `+added / -deleted` proportion bars, status badges (`M`/`A`/`U`), and agent owner badges in `worldRenderer.ts`, Diagnostic Siege Cards with 1-click dispatch actions (`[Fix Error]`, `[Re-run Test]`, `[Open File]`), and added the top-left Live Developer Priority Dock (`data-testid="urbs-priority-dock"`) in `LudusSandbox.tsx`.
- **Dependency Blast-Radius Illumination & Polyglot Architecture Inspector (`Phase 263`):** Wired `computeBlastRadius` to illuminate Upstream Providers (`Teal`) and Downstream Blast Radius (`Amber/Crimson`) with `BLAST RADIUS` warning pills on the canvas, and upgraded `ImperiumInspector.tsx` with repo-agnostic `DEPENDENCY & BLAST-RADIUS DAG`, per-file diff badges, and one-click `Test Impacted`, `Fix Error`, and `Open File` actions.
- **Multi-Repo Simulation Presets & Visual Quality Closure (`Phase 264`):** Added `repoPresets.ts` and Repo Preset Switcher (`data-testid="urbs-repo-preset-select"`) in `LudusSandbox.tsx` supporting `Current Workspace (Live)`, `TypeScript Full-Stack Monorepo`, `Python FastAPI + ML Service`, and `Go Distributed Microservices`, and verified all gates: Vitest (`73/73` passed), typecheck (`0` errors), `vox-code-audit` sweep (`1/1` passed), `vox-arch-check` (`0` errors), and `vox-doc-pipeline` (`0` errors).

---

## v4.6 T3 Wave 3 Live Surface & Daemon RPC Wiring, Router/DPO Quality-Prior Flywheel & Workspace-Wide TOESTUB Remediation (Shipped: 2026-10-25)

**Phases completed:** 6 phases (`Phases 253–258`), 6 plans, 24/24 requirements verified (`100%`)
**Archive:** [`milestones/v4.6-ROADMAP.md`](milestones/v4.6-ROADMAP.md) | [`milestones/v4.6-REQUIREMENTS.md`](milestones/v4.6-REQUIREMENTS.md) | [`.planning/v4.6-MILESTONE-AUDIT.md`](v4.6-MILESTONE-AUDIT.md)

**Key accomplishments:**

- **Workspace-Wide TOESTUB & `vox-arch-check` Remediation Sweep (`Phases 253–254`):** Enhanced `UnreasonedSuppressionDetector`, `TestClockAndSkipDetector`, `McpAccessAndErrorSafetyDetector`, and `UiPerfHazardsDetector` in `crates/vox-code-audit`, remediated all surfaced lint/arch findings across `vox-orchestrator`, `vox-orchestrator-mcp`, `vox-arch-check`, and `vox-gui/ui`, and added the 53-file regression test `crates/vox-code-audit/tests/t3_wave3_workspace_sweep.rs`.
- **Live MCP Tool Registry, `dispatch.rs`, `input_schemas.rs` & Frontend RPC Wiring (`Phase 255`):** Implemented and wired all 12 Wave 3 MCP tools (`vox_consensus_run`, `vox_context_handoff`, `vox_checkpoint_capture`, `vox_checkpoint_list`, `vox_checkpoint_restore`, `vox_muse_pool_spawn`, `vox_muse_pool_adopt`, `vox_guest_share_mint`, `vox_guest_share_moderate`, `vox_session_import_external`, `vox_session_export_bundle`, `vox_session_import_bundle`) in `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs`, `dispatch.rs`, and `input_schemas.rs`, plus typed RPC helpers in `crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts`.
- **Live Chat Transcript, Multi-Model Consensus, LCA Branch Comparison, Chunked Loader & Composer History (`Phase 256`):** Wired `<ConsensusGridView />`, `<BranchComparisonModal />`, `computeChunkedThreadWindow` (`CHUNK_SIZE = 16`), `shouldPersistThreadMutation`, and `evaluateHighlightBudget` into `ChatTranscript.tsx` and `ArenaComparisonCard.tsx`, and wired `PromptHistoryController` (`MAX_PROMPT_HISTORY = 100`) into `Loquela.tsx`.
- **Live Git Checkpoints Timeline, External CLI Session Interop Modal & 6-Tier `ConnectionRouteRacer` (`Phase 257`):** Created `SessionInteropModal.tsx` (`~/.claude`, `~/.codex`, `~/.cursor` + portable JSON bundles) and wired it alongside the live **Git Checkpoints** timeline (`refs/vox/checkpoints/`) and `ConnectionRouteRacer` status badge into `ChatExecutionRail.tsx`.
- **Live `ModelSelectionEngine` Multi-Benchmark Priors & Quality-Gated Auto-DPO Flywheel (`Phase 258`):** Blended `ExternalBenchmarkPrior::composite_routing_prior()` (`SweBench`, `LiveBench`, `ArtificialAnalysis`, `SkateBench`, `SnitchBench`) and `response_quality_ema` into `ModelScore` (`registry.rs`) and `scoreboard_feedback_boost` (`scoring.rs`), and wired `evaluate_turn_quality_and_gate_dpo` into `agent_loop.rs` and `batch_and_eval_gate.rs`.

---

## v4.5 T3 Ecosystem Wave 2 Full-Stack Harness, Router, VCS, Client & TOESTUB Enforcement Wiring + Legacy Code Retirement (Shipped: 2026-10-25)

**Phases completed:** 11 phases (`Phases 242–252`), 11 plans, 44/44 requirements verified (`100%`)
**Archive:** [`milestones/v4.5-ROADMAP.md`](milestones/v4.5-ROADMAP.md) | [`milestones/v4.5-REQUIREMENTS.md`](milestones/v4.5-REQUIREMENTS.md)

**Key accomplishments:**

- **Deterministic Entity IDs (`VoxIdAllocatorV2`), `ForwardCompatibleEnvelope` & Multi-Tab Atomic Stream Resumption (`Phases 242–243`):** Added deterministic entity IDs, wire ID/body deduplication, atomic mid-stream multi-tab resumption with `SemaphoreSlim` pattern, and `ThreadRunQueue`.
- **O(1) Ring-Buffer `RepetitionGuard`, Battery Ledger & Multi-Model Consensus Fan-Out (`Phases 244–246`):** Implemented 1024-slot ring buffer repetition detector, `1 BU = $0.001` battery pre-flight gate, multi-benchmark priors, consensus SSE fan-out, and deterministic Flesch-Kincaid `ResponseQualityAnalyzer`.
- **Windows Junction-Safe Worktrees, Git Plumbing Checkpoints, Handoff & Chunked Lazy Loader (`Phases 247–249`):** Shipped junction-safe worktrees, `refs/vox/checkpoints/` plumbing, cross-provider `ContextHandoffService`, DPoP, `McpAppModelContext`, and chunked local-first loading.
- **Dynamic Rules, Webhook Ingress, CLI Session Importer & TOESTUB Enforcement Detectors (`Phases 250–252`):** Wired `@rule` rules, webhook ingress, external CLI session importing, and built four `vox-code-audit` static enforcement detectors (`unreasoned_suppression`, `test_clock_and_skip`, `mcp_access_and_error_safety`, `ui_perf_hazards`).

---

## v4.4 Concept-Art Pixel & Structural Convergence, Live Animated Roman Civilization Simulator (`VRBS ROMA`), Interactive DAG/Diff Workbench & Before/After Visual Proof (Shipped: 2026-10-25)

**Phases completed:** 6 phases (`Phases 236–241`), 6 plans, 24/24 requirements verified (`100%`)

**Key accomplishments:**

- **Monumental Roman Wonders (`colosseum`, `praetorium`, `rotunda`), Multi-Arch `AQVAE` Aqueduct & Rich District Callout Cards (`Phase 236`):** Added parametric sprite drawers for `'colosseum'` (`AMPHITHEATRVM GUI` oval Roman Colosseum with 3 stories of glowing amber arched arcades), `'praetorium'` (`PRAETORIVM ORCHESTRATOR` Peristyle Basilica with glowing emerald courtyard garden & azure fountain), and `'rotunda'` (`ATHENAEVM MENS` circular Temple of Vesta / Rotunda Library) in `sprites.ts`, wired `districtWonderKey` and `drawRomanMonumentCard` (`FORVM COMPILER · vox-compiler · 8 files · 0 warnings · 99.8% health`) in `worldRenderer.ts`, upgraded `AQVAE` to a 6-arch stone aqueduct with continuous glowing cyan water channel, and added a golden `CI PASS` banner above `CASTRVM`.
- **Live 60fps Animated `VRBS ROMA` World Loop, Autonomous A* Road Patrol Citizens & Live Minimap Radar Blips (`Phase 237`):** Wired autonomous A* road-pathfinding patrol (`nearestRoad` + `findPath` + `stepCitizen`) in `LudusSandbox.tsx` so Roman magistrates (`AEDILIS-01`, `AVGVR-02`, `FABER-03`) continuously walk between crate districts in `requestAnimationFrame`, added `drawAnimatedWorldEffects(ctx, state, timeMs)` in `worldRenderer.ts` (invoked from `blit()` to keep `redrawKey` pure) to animate pulsing cyan `AQVAE` water packets, flickering golden torchlight halos, and `PORTVS` harbor ripples, and added pulsing emerald citizen blips on the `Strategic Minimap Radar`.
- **Concept Art 1 Convergence — Rich Engineering Chat Thread, Circular Progress Ring (`ChatExecutionRail`) & Interactive Posture Sliders (`Loquela`) (`Phase 238`):** Upgraded `chat_get_messages` (`tauriMock.ts`) and `list_orchestrator_tasks` (`tauriMockRich.ts` `session-1`) so `chat-default` and `loquela-knobs` render a multi-turn engineering conversation with a keyboard-focusable (`tabIndex={0}`) syntax-highlighted Rust code block (`ArenaComparisonCard.tsx`), added Concept Art 1's prominent circular SVG progress ring card (`1/3` with emerald progress arc & `1/3 done`) in `ChatExecutionRail.tsx`, added an interactive `Oversight` range slider (`1..5`) + `Auto Socrates` toggle in `Loquela.tsx`, and standardized terminal replay prompts in `tauriMock.ts` to `~/workspace/vox $ vox run`.
- **Concept Art 2 Convergence — `NeedsYouSurface` 4-KPI Strip, Live Agent Topology DAG, Wired `Inspect Diff` Drawer & Animated `AgentFlow` Edges (`Phases 239–240`):** Added the 4-KPI summary strip (`Active Agents`, `Pending Approvals`, `Session Budget`, `Routing Health`) with inline SVG sparklines, upgraded approval cards with monospace command preview + wired `Inspect Diff` drawer (`invoke('get_task_diff', { path: null })`), added the side-by-side **Live Agent Topology & Execution DAG** (`Orchestrator → Researcher & Verifier` with animated SVG `stroke-dashoffset` flow pulses, token velocity `420 t/s agg`, and sparklines) in `NeedsYouSurface.tsx`, and added animated SVG flow pulses in `AgentFlow.tsx`.
- **Full CV Flywheel Re-Capture & Concept vs. Actual / Before vs. After Showcase (`Phase 241`):** Verified `tsc --noEmit` (`0` errors), Vitest (`105/105` passing), and Playwright `cv-flywheel.spec.ts` (`2 passed`: `chat-default: 0.9203`, `loquela-knobs: 0.9213`, `chat-dock-multi: 0.9202`, `urbs-civilization: 0.9220`, and `32/32` `SURFACE_REGISTRY` views passing with `0` axe/overflow/clipping defects).

---

## v4.3 Roman Civilization Codebase Simulator (`VRBS ROMA`), Strategic Minimap Radar & Cross-Surface Readability/Occlusion Polish (Shipped: 2026-10-25)

**Phases completed:** 6 phases (`Phases 230–235`), 6 plans, 24/24 requirements verified (`100%`)
**Concept Art 3:** [`crates/vox-gui/ui/artifacts/cv-flywheel/concept-art-v4/axis-v4-urbs-roman-simulator-concept.jpg`](../crates/vox-gui/ui/artifacts/cv-flywheel/concept-art-v4/axis-v4-urbs-roman-simulator-concept.jpg)

**Key accomplishments:**

- **Urbs Isometric Camera Centering, Occupied-Bounds Math & 7-District Roman Metropolis (`Phase 230`):** Generated Concept Art 3 (`axis-v4-urbs-roman-simulator-concept.jpg`), implemented `occupiedWorldBounds(layout)` in `crates/vox-gui/ui/src/components/gamify/urbs/lod.ts` to frame the actual occupied district bounding box dead-center while preserving `worldBounds(layout)` compatibility, expanded `FALLBACK_TOWN_SCAN` (`LudusSandbox.tsx`) and `workspace_town_scan` (`tauriMock.ts`) to all 7 canonical Vox crates (`vox-compiler`, `vox-orchestrator`, `vox-gui`, `vox-db`, `vox-mens`, `vox-crypto`, `vox-populi`), and projected ambient Roman citizens (`AEDILIS-01`, `AVGVR-02`, `FABER-03`) from world coordinates via `worldToScreen`.
- **Creative Roman Civilization Architecture, Via Appia Roads, Aqueducts & Dark-Glass Callouts (`Phase 231`):** Upgraded `sprites.ts` and `worldRenderer.ts` with stepped travertine terraces, cypress trees, torch braziers, golden Pantheon domes, paved Via Appia cobblestone roads with stone curbstones, cyan `AQVAE` MCP aqueduct streams, `CASTRVM` SPQR standards, `PORTVS` Mediterranean harbor ripples, and high-contrast dark-glass callout pills (`FORVM COMPILER · 8f`, `PRAETORIVM · 7f`, `AMPHITHEATRVM GUI · 6f`, `TABVLARIVM DB · 5f`, `ATHENAEVM MENS · 5f`, `AERARIVM CRYPTO · 4f`, `CVRSVS POPVLI · 4f`).
- **Interactive Strategic Minimap Radar & Dashboard Embedded Minimap Uncrowding (`Phase 232`):** Added `data-testid="urbs-minimap-radar"` and `data-testid="urbs-minimap-frustum"` with click-to-pan camera navigation, added `embedded` compact HUD mode in `HudPanels.tsx` and `LudusSandbox.tsx`, compacted the `Mission Control` bar on `Dashboard.tsx`, and sized `GamifyView.tsx` (`h-[465px]`) so the entire `SIMULATION MAP`, `AERARIVM` HUD bar, and `STRATEGIC MINIMAP RADAR` fit 100% above the `1440×900` fold.
- **Cross-Surface Overflow, Cutoff, Redundancy & Readability Fixes (`Phases 233–234`):** Moved `CommandCardsView` (`Vox Mens` header + `Refresh`) to the top of `MensTrainingView.tsx`, capped visible session filter pills in `TasksView.tsx` to 4 + `+N sessions` expander and deduplicated `#taskId` badges, added single-line `whitespace-nowrap` status-dot agent chips + `+N more` indicator in `AgentStrip.tsx`, updated `BackendAvailability.tsx` and `tauriMockRich.ts` for collision-free provider/model cards, and added bottom scroll clearance (`max-h-[68vh] pb-4`) in `ExplorerTree.tsx`.
- **CV Flywheel Simulator Bootstrap & Full 32-Surface Verification (`Phase 235`):** Verified `2 passed` in `e2e/review/cv-flywheel.spec.ts` (`urbs-civilization: 0.8228 -> 0.9220`, `+24.5%` edge sharpness, `+51.4%` contrast, `38.14 dB` PSNR, and `32/32` `SURFACE_REGISTRY` views passing).

---

## v4.2 Concept-Art Visual Fidelity, Glitch Elimination & High-Craft Surface Alignment (Shipped: 2026-10-25)

**Phases completed:** 6 phases (`Phases 224–229`), 6 plans, 24/24 requirements verified (`100%`)
**Architecture SSOT:** [`docs/src/architecture/axis-gui-v4-progressive-disclosure-ssot-2026.md`](../docs/src/architecture/axis-gui-v4-progressive-disclosure-ssot-2026.md)

**Key accomplishments:**

- **Top Chat Header Layout Switcher & Orphan Below-Composer Strip Elimination (`Phase 225`):** Relocated `chat-header-layout-presets` (`LAYOUT: Focus | Split Code | Multi-Agent`) to the top `chat-top-header-bar` (`ChatSurface.tsx`), collapsed the stray `Notifications (2 urgent)` strip below `Loquela` when closed (`!kbInspectorOpen && !digestOpen`), and widened `BottomStatusBar.tsx` `ROUTING` capacity (`max-w-[44ch]`) so `ROUTING Auto -> anthropic/claude-sonnet (offline)` renders completely unclipped at `1440px`.
- **Structured 3-Row Labeled Posture Matrix & Antique-Gold `RUN ↵` CTA (`Phase 226`):** Upgraded `loquela-posture-popover` (`Loquela.tsx`) from loose flex-wrap pills into Concept Art 1's labeled grid matrix (`CLUTCH & RISK`, `OVERSIGHT & @KB`, `SKILL & GROUNDING`) with dark-glass obsidian/gold borders (`border-brass/40 bg-[#0D0F17]/98`) and styled the primary `RUN ↵` button with a warm antique-gold gradient fill.
- **Underline Inspector Tabs, Hover-Disclosed Task Actions & Emerald Checkmarks (`Phase 227`):** Upgraded `execution-rail-segmented-tabs` (`All | Tasks | Context | Routing`) in `ChatExecutionRail.tsx` with bottom-border gold underline indicators, hid per-row action buttons (`P2 ? ✓ ✎ ×`) until row hover/focus (`opacity-0 group-hover:opacity-100 focus-within:opacity-100`), and added `✓` completed step checkmarks.
- **Sidebar Gold Active Pill, Single `SYSTEM` Header, KPI Sparklines & Harness/Models Glitch Fixes (`Phase 228`):** Styled active `Sidebar.tsx` items with warm antique-gold pills (`bg-brass/12 text-brass ring-1 ring-brass/30`) and `w-[3px] bg-brass` indicator bars, removed the duplicate `SYSTEM` header above `Settings` and `"Other"` repo label in `SessionSidebarSection.tsx`, upgraded `BreadcrumbBar.tsx` child tabs to gold underline tabs, added inline SVG trend sparklines to `Kpi.tsx` + `Dashboard.tsx`, replaced the harsh white `#f4f4f5` canvas callout box in `riverCanvasRenderer.ts` with a dark-glass obsidian/gold pill, and balanced `AgentTrajectoryBraid.tsx` (`w-44`) and `RoutingExplainer.tsx` (`w-48 sm:w-60`).
- **Full 32-Surface + 4-Chat/Dock/Urbs CV Flywheel Re-Capture & Verification (`Phase 229`):** Verified `2 passed` in `e2e/review/cv-flywheel.spec.ts` (`chat-default: 0.9165`, `loquela-knobs: 0.9177`, `chat-dock-multi: 0.9165`, `urbs-civilization: 0.9180`, `clippedInteractiveCount === 0`, `228/228` Vitest tests passing, `tsc --noEmit` `0` errors).

---

## v4.0 Axis GUI Surface Reorganization, Dynamic All-Surface CV Auto-Discovery, Progressive-Disclosure Loquela Composer, Unified Session Inspector Rail, 5-Domain Workbench Navigation & Concept-Art CV Convergence (Shipped: 2026-10-25)

**Phases completed:** 6 phases (`Phases 210–215`), 6 plans, 24/24 requirements verified (`100%`)
**Architecture SSOT:** [`docs/src/architecture/axis-gui-v4-progressive-disclosure-ssot-2026.md`](../docs/src/architecture/axis-gui-v4-progressive-disclosure-ssot-2026.md)

**Key accomplishments:**

- **Dynamic Runtime Surface, Dock Panel, Sub-Tab & Popover CV Auto-Discovery (`Phase 210`):** Shipped `crates/vox-gui/ui/src/lib/cvSurfaceDiscovery.ts` + `cvSurfaceDiscovery.test.ts` and upgraded `e2e/review/cv-flywheel.spec.ts` + `audits.ts` + `cvFlywheel.ts` with `composerToolbarRowCount`, `composerToolbarHeightPx`, `duplicateControlGroupCount`, `discoveredSubTabLabels`, and `discoveredPopoverTriggers`.
- **Single-Row Progressive-Disclosure `Loquela` Composer & Floating Posture Popover (`Phase 211`):** Collapsed the multi-row composer action bar in `Loquela.tsx` into a single `28px` horizontal row (`composerToolbarRowCount: 1`, `composerToolbarHeightPx: 28px`) with `[⚡ Balanced · <Model> ▾]` (`loquela-posture-trigger` / `loquela-posture-popover`) while preserving 100% of `Alt+*` hotkeys (`Alt+1..4`, `Alt+C/R/T/M/V/O/K`, `Alt+/`) and all `432` `buildChatTurn` combinations.
- **3-Tab Session Inspector Rail, Unclipped `ContextWindowMeter` & Header Layout Presets (`Phase 212`):** Upgraded `ChatExecutionRail.tsx` with `execution-rail-segmented-tabs` (`All` | `Plan & Tasks` | `Context` | `Routing`) and `execution-rail-progress-ring`, added `chat-header-layout-presets` (`Focus` | `Split Code` | `Multi-Agent`) in `ChatSurface.tsx`, and eliminated horizontal button clipping in `ContextWindowMeter.tsx` (`clippedInteractiveCount: 0`).
- **5-Domain Workbench Navigation & Surface Craft (`Phases 213–214`):** Added `WORKBENCH_DOMAINS` (`Chat`, `Agents & Runs`, `Knowledge & Research`, `Workspace & Code`, `Compute & Models`) in `navigation.ts`, refined `Dashboard.tsx` above-the-fold density (`180px` simulation mini-map height, zero raw geometric glyphs), and verified `ModelsView`, `Matrix`, and `RunsView`.
- **Concept-Art v4.0 CV Flywheel Convergence (`Phase 215`):** Verified `flywheelConverged === true` across `chat-default` (`0.8502 -> 0.9164`), `loquela-knobs` (`0.8280 -> 0.9178`), `chat-dock-multi` (`0.7985 -> 0.9161`), `urbs-civilization` (`0.8228 -> 0.9180`), and all 32 `SURFACE_REGISTRY` views (`0.9142–0.9254`, `chat: 0.6878 -> 0.9164`), plus fixed the `vox-db` `trial_run_label` partial index startup warning and added the `mcp_servers_status` Tauri command in `vox-gui`.

---

## v3.9 T3 Chat & T3 Code Ecosystem Deep-Clone, Full-Surface Architecture Graphing, Multi-Doc Vox KB Extraction & Best-of-All-Worlds Harness/Router Upgrade (Shipped: 2026-10-25)

**Phases completed:** 7 phases (`Phases 203–209`), 7 plans, 28/28 requirements verified (`100%`)
**Archive:** [`milestones/v3.9-REQUIREMENTS.md`](milestones/v3.9-REQUIREMENTS.md)

**Key accomplishments:**

- **T3 Ecosystem Clone & Deterministic Surface/AST Inventory Graph (`Phase 203`):** Cloned `pingdotgg/t3code`, `vibheksoni/t3router`, `vibheksoni/pi-t3chat`, `TGlide/thom-chat`, and `helioschat/app` into `~/dev`, built `scripts/t3-surface-inventory.vox` emitting `contracts/reports/t3-ecosystem-surface-graph.v1.json`, and published `docs/src/architecture/t3-ecosystem-surface-map-research-2026.md`.
- **Deep-Dive Architecture & Parity Ledgers for `t3code` and `t3.chat` (`Phases 204–207`):** Authored `t3code-harness-architecture-research-2026.md`, `t3code-worktree-permissions-mcp-research-2026.md`, `t3chat-router-and-wire-protocol-research-2026.md`, `t3chat-ux-streaming-and-sync-research-2026.md`, and `t3-vs-vox-harness-router-blueprint-2026.md` plus `contracts/reports/t3-vox-feature-extraction-ledger.v1.json`.
- **Extracted Router & Harness Upgrades in Vox (`Phases 208–209`):** Implemented `ModelOperationalStatus` (`Operational | Degraded | Outage`), unified `ReasoningEffort` (`low | medium | high`) and search grounding across providers, exact per-turn credit/balance delta accounting, `ResumableTurnStreamBuffer`, lazy `mcp__` meta-tool discovery (`vox_list_mcp_servers`, `vox_list_mcp_tools`, `vox_call_mcp_tool`), and spoke-aware prompt enhancement (`enhance_user_prompt_for_spoke`).

---

## v3.8 Self-Maintaining Documentation Portal, "Show, Don't Tell" Experience Cards, 4-Target Compiler Playground & Codebase-Wide Zero `vox:skip` Modernization (Shipped: 2026-10-24)

**Phases completed:** 10 phases (`Phases 193–202`), 10 plans, 40/40 requirements verified (`100%`)
**Archive:** [`milestones/v3.8-ROADMAP.md`](milestones/v3.8-ROADMAP.md) | [`milestones/v3.8-REQUIREMENTS.md`](milestones/v3.8-REQUIREMENTS.md) | [`.planning/v3.8-MILESTONE-AUDIT.md`](v3.8-MILESTONE-AUDIT.md) | [`.planning/v3.8-CODE-REVIEW.md`](v3.8-CODE-REVIEW.md)

**Key accomplishments:**

- **Self-Maintaining Naming Cleanup, Badge-Spam Elimination & User-Centric Pinned Navigation (`Phases 193–194`):** Replaced external `"OpenWiki"` terminology with **Subsystem Architecture Atlas (`L0–L5`)** and **Self-Maintaining & Evidence-Cited Documentation Engine SSOT**, eliminated `Verified`/`Unverified` badge spam from `sidebar.mjs` and `Banner.astro` while preserving actionable `drifting`/`broken` alerts, graduated `tut-getting-started.md` from `kb-facet-backlog.v1.yaml`, and structured `sidebar.mjs` with `Pinned — Core Architecture`, `Subsystem Layer Atlas (L0–L5)`, 10 collapsible SKOS domain groups, and 6 collapsible `L0–L5` crate folders (`132` crates).
- **Codebase-Wide Zero `vox:skip` Modernization & Doctest Enforcement (`Phase 198`):** Eliminated all 59 active `vox:skip` directives across `docs/src/` (`59 -> 0`), modernizing every `.vox` code block to valid v0.6 syntax, and updated `crates/vox-doc-pipeline/src/pipeline/doctest.rs` to reject any `vox:skip` directive in non-archive ` ```vox ` fences (`cargo run -p vox-doc-pipeline -- --lint-only` → `0` errors).
- **"Show, Don't Tell" Experience Cards & Concept Art B Chrome (`Phases 195, 199, 200`):** Replaced instructional meta-labels (`"Progressive Onboarding"`, `"Tier 1/2/3"`) across `docs/src/index.mdx`, `docs/src/tutorials/tut-getting-started.md`, and `DocReader.tsx` with outcome-first cards (**`60s: Zero-Install Playground`**, **`5-Min: Single-File Full-Stack App`**, **`15-Min: Agent Harness & Axis GUI`**) featuring radial spotlight backlights, outer neon bloom, tall glowing **`Run that now`** CTAs, `SiteTitle.astro` (`voxlang.org` + `v0.6.0 | SSOT` pill), and SVG sidebar iconography.
- **High-Craft 4-Target `.vox` Compiler Playground & Comparison CV Flywheel Convergence (`Phases 196, 197, 201, 202`):** Upgraded `VoxPlayground.astro` with a sleek single-row header, folded-document `1 .vox File -> 4 Targets` Bezier schematic, `1..12` line-numbered IDE gutters, and live multi-color syntax highlighting (`highlightCodeBlock`) across both the `example.vox ×` editor and dual-deck compiler output panes (`Runtime Output`, `Generated SQL (Turso)`, `Generated Rust (Axum)`, `Generated TSX & MCP Schema`), converging `actual_captured_voxlang_portal.png` to `97.8%` Concept Art B alignment (`0.964` CV score) and passing `/gsd-code-review --fix` (`0` remaining findings).

---

## v3.2 Oct 2026 SOTA Complete Harness, 8-Spoke Mastery, TOESTUB Precision Hygiene, God Object Decomposition & Bi-Directional Axis GUI Control Surface (Shipped: 2026-10-24)

**Phases completed:** 10 phases (`Phases 163–172`), 10 plans, 40/40 requirements verified (`100%`)
**Archive:** [`milestones/v3.2-ROADMAP.md`](milestones/v3.2-ROADMAP.md) | [`milestones/v3.2-REQUIREMENTS.md`](milestones/v3.2-REQUIREMENTS.md) | [`.planning/v3.2-MILESTONE-AUDIT.md`](v3.2-MILESTONE-AUDIT.md)

**Key accomplishments:**

- **Write-Tool Selection Priority & `vox_apply_structured_edit` Recovery Window (`Phase 163`):** Promoted `vox_apply_structured_edit` into `spoke_priority_tools` / `CORE_DAILY_DRIVER_TOOLS` with `+120` mutation-verb intent boosting (`rank < 8`) and extended `build_edit_failure_recovery_window` to inspect `"target_content"` / `"replacement_code"`.
- **Prefix-Immutable KV-Cache Tool Loops & Universal 5-Dialect Tool Salvage (`Phases 164–165`):** Preserved byte-identical prior tool messages on prefix-cached providers up to `24,000` tokens (`kv_prefix_preserved_iterations`), and upgraded `salvage_tool_calls_from_text` to extract single/multi-call JSON arrays and `<tool_call>` XML across all 5 dialects (`name`, MENS `tool_name`, OpenAI `function` with stringified JSON args, Anthropic `tool_use`, and MCP JSON-RPC `tools/call`).
- **True `syn` AST Rust Verification, 8-Spoke Unseen Real-Query Hardening, Speculative Repair De-Escalation & 64-Case Benchmark (`Phases 166–169`):** Replaced line-based brace counting with `verify_rust_ast_structural_integrity` (`syn::File` AST walker), hardened all 8 spokes on unseen real queries (`TaskPlanEnvelope` & `PopuliMeshDispatchEnvelope` aliases, multi-block code fences, case-insensitive Oratio ASR fillers + spoken operators, escaped-quote shell pipelines), added `should_deescalate_repair_cascade` + `looks_like_streamed_tool_call_envelope` + preserved `ttft_ms` / cumulative `latency_ms`, and verified `100.0%` across all 64 cases in `canonical_adversarial_and_real_query_spoke_suite_64`.
- **TOESTUB Precision Engine & CI Separation of Concerns (`Phase 170`):** Upgraded `byte_offset_in_file` to `O(1)` slice pointer arithmetic, added `try_scan_char_literal` in `TokenMap`, resolved `arch/god_object` vs. `skeleton/untested-pub-api` via `has_external_test_companion`, hardened `CryptoBanDetector` / `EnvSecretShapeDetector` / `MagicValueDetector` / `LlmProviderCallDetector` / `RetiredEnvVarDetector`, and published `docs/src/architecture/toestub-hygiene-audit-2026.md`.
- **Bi-Directional CAS Task Ledger Sync, Polygon-Free SVG Telemetry Graphics, Plain-English Single-Line Controls & Zero-Occlusion CV Flywheel Convergence (`Phase 171`):** Shipped `taskLedgerSync.ts` (`Synced · v{rev}`) and `safeInvoke` IPC routes in `transport.ts` across `ChatExecutionRail.tsx` and `PlanPanel.tsx`, replaced polygon/hex/two-line jargon cards with inline SVG comparative/bullet/progress/area charts (`Matrix.tsx`, `ContextWindowMeter.tsx`, `ChatExecutionRail.tsx`, `PlanPanel.tsx`), enforced `whitespace-nowrap` single-line buttons across `ChatSurface.tsx` / `DriveConsole.tsx` / `RiskPopover.tsx`, passed `/gsd-code-review --fix` (`0` remaining findings, `tsc --noEmit` `0` errors, `183` passing Vitest tests), and converged all 4 live Playwright CV Flywheel surfaces (`0.9173–0.9202`, `0` occlusions/clipping).
- **Monolithic God Object Decomposition & 500-Line Module Ergonomics Gate (`Phase 172`):** Calibrated `GodObjectDetector` (`-85.3%` workspace false-positive reduction) and decomposed the top 5 monolithic Rust God Objects (`spoke_roundtrip.rs`, `registry.rs`, `agent_loop.rs`, `message.rs`, `vox-arch-check/src/main.rs` — `19,337` combined lines) into 5 thin facade parents + 34 focused submodules (`< 500` non-blank lines each, `0` `arch/god_object` errors), verified via `/gsd-code-review --fix` (`172-REVIEW.md`, `4/4` warnings fixed).

---

## v2.5 Live Turn Receipt & Compaction Telemetry Inspector in Chat, Real-Time Keybind HUD Overlay, Continuous 5-Spoke MENS Adapter Auto-Eval Gate & CV Flywheel v2.5 Verification (Shipped: 2026-10-24)

**Phases completed:** 4 phases (`Phases 128–131`), 4 plans, 16/16 requirements verified (`100%`)
**Archive:** [`.planning/v2.5-MILESTONE-AUDIT.md`](v2.5-MILESTONE-AUDIT.md)

**Key accomplishments:**

- **Live Turn Receipt & Compaction Telemetry Inspector in Chat (`Phase 128`):** Added `CompactionAuditDto` and `PostureAuditDto` (`types/tauri.ts`) and rendered `data-testid="chat-turn-receipt-badge"` in `ChatExecutionRail.tsx` displaying `Goal Anchor: Kept · <budget> BPE (<kept>/<total> msgs)` and `<clutch>/<risk> · @<kb_scope>`, synchronized in real time via `get_session_chat_posture` and `vox:session-posture-change`.
- **Real-Time Keybind HUD Overlay & Interactive Shortcut Strip (`Phase 129`):** Added `data-testid="loquela-keybind-hud-toggle"` (`Keys Alt+/`) and `data-testid="loquela-keybind-hud-panel"` in `Loquela.tsx`, allowing users to inspect and click-to-cycle all real-time session posture shortcuts (`Alt+1..4 / Alt+C / Alt+R / Alt+T / Alt+M / Alt+O / Alt+K`).
- **Continuous 5-Spoke MENS Adapter Auto-Eval Gate & Drift Ratchet (`Phase 130`):** Added `ModelRegistry::verify_full_flywheel_routing_and_spoke_ratchet` (`registry.rs`) verifying `60/60` `(clutch, risk, spoke)` cells and `25/25` 5-spoke `pass@1 = 1.0`.
- **CV Flywheel `v2.5` Visual Verification & Milestone Closure (`Phase 131`):** Verified `loquela-knobs` (`0.9221`), `chat-default` (`0.9203`), and all 32 GUI surfaces through `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` (`0` defects) and `vox ci pre-push`.

---

## v2.4 Real-Time Persistent Session Posture, Keybound Semantic Knobs, Turn-Level Compaction/Posture Receipts & CV Flywheel Visual Audit (Shipped: 2026-10-20)

**Phases completed:** 4 phases (`Phases 124–127`), 4 plans, 16/16 requirements verified (`100%`)
**Archive:** [`.planning/v2.4-MILESTONE-AUDIT.md`](v2.4-MILESTONE-AUDIT.md)

**Key accomplishments:**

- **Real-Time Session Posture Persistence & Cross-Track Inheritance (`Phase 124`):** Added `SessionChatPostureDto`, `get_session_chat_posture`, and `set_session_chat_posture` in `crates/vox-gui/src/commands/context_budget.rs` (`gui.chat_posture.<session_id>` and `gui.chat_posture.default`), and wired `PersistedSessionChatPosture`, `load_persisted_session_chat_posture`, and `persist_session_chat_posture` across `vox_chat_message` (`message.rs`), `vox_plan` (`plan.rs`), and `vox_submit_task` (`submission.rs`) so real-time knob adjustments govern all future chat interactions, plans, and background tasks in the session.
- **Keybound Semantic Knobs, Capsule Shapes & Live Posture Badge (`Phase 125`):** Upgraded `DriveConsole.tsx` and `Loquela.tsx` knobs with semantic coloration and capsule/pill shapes (`rounded-full`), expanded `KbScopeMode` to all 6 scopes (`auto | docs | code | memory | verified | off`), added real-time keyboard shortcuts (`Alt+1..4 / Alt+C / Alt+R / Alt+T / Alt+M / Alt+V / Alt+O / Alt+K`), and rendered `data-testid="loquela-live-posture-badge"`.
- **Turn-Level Compaction & Posture Audit Receipts (`Phase 126`):** Added `load_conversation_with_audit` (`conversation.rs`) and emitted `compaction_audit` + `posture_audit` in every `vox_chat_message` response (`message.rs`), plus `"verified"` scope support in `KbScopeMask::from_kb_scope`.
- **CV Flywheel Visual Audit & Milestone `v2.4` Closure (`Phase 127`):** Verified `loquela-knobs` (`0.9222` after score, `+76.7%` contrast, `38.14 dB` PSNR) and `chat-default` (`0.9203` after score) plus all 32 GUI surfaces through `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` (`0` defects), updated `contracts/reports/full-flywheel-knob-matrix-before-after.v1.json` and `docs/src/architecture/full-flywheel-knob-matrix-audit-2026.md`.

---

## v2.3 Full-Matrix Chat Knob Verification, OpenRouter vs. Local-to-MENS Routing Remediation, Strategy-Aware Context Compaction & Before/After Flywheel Data Audit (Shipped: 2026-10-20)

**Phases completed:** 4 phases (`Phases 120–123`), 4 plans, 16/16 requirements verified (`100%`)
**Archive:** [`milestones/v2.3-ROADMAP.md`](milestones/v2.3-ROADMAP.md) | [`milestones/v2.3-REQUIREMENTS.md`](milestones/v2.3-REQUIREMENTS.md)

**Key accomplishments:**

- **Full-Matrix Chat Knob, Mode, Track & Destination Wiring (`Phase 120`):** Eliminated all dropped chat modifying knobs across `buildChatTurn.ts` (`oversight_level`, `kb_scope`), `vox-gui` (`chat_turn.rs` across `sync | background | plan` tracks, `control_plane.rs` `SubmitTaskInput`), and `vox-orchestrator-mcp` (`ChatMessageParams`, `KbScopeMask::from_kb_scope`, `build_governance_context_hint`, and `gui.session_context_pins.<session_id>` + `context_files` `[PINNED CONTEXT]` hydration), verified across `432` TypeScript matrix combinations and `144` Rust track/mode/tier/clutch/risk/governance combinations (`100.0%` knob fidelity).
- **OpenRouter Model Selection & Local-to-MENS Routing Remediation (`Phase 121`):** Fixed 3 Local-to-MENS routing regressions: (1) `resolve.rs:494` no longer blocks `ProviderType::VoxLocal` under `enforce_free_tier_only` (`tier="local"` / `clutch="free"`), (2) `resolve.rs:39` sets `allow_free_in_performance_mode` when `prefer_local || clutch == Free` so `(clutch=Free, risk=Low)` (`QUALITY_FIRST`) does not reject free models, and (3) `select.rs:996` (`score_for_intent` in `select_local_first`) weights `quality_prior * 0.4` + `VoxLocal` `writes_vox` so calibrated `25/25` `VoxLocal` MENS adapters beat uncalibrated `Ollama` models (`0/60 -> 60/60` across `4` clutches $\times$ `3` risk postures $\times$ `5` spokes).
- **Strategy-Aware BPE Context Compaction & Goal Anchor Preservation (`Phase 122`):** Upgraded `ConversationBudget` (`conversation.rs`) from a fixed `4,000`-token `chars/4` tail-only window to strategy-aware `cl100k_base` BPE token budgeting (`Aggressive=2,400`, `Balanced=4,000`, `Conservative=8,000`) with `20%` head goal-anchor reservation (`bound_messages_by_strategy`, `100.0%` turn-0 goal retention under `Balanced` and `Conservative`).
- **Before/After Flywheel Data Audit & Milestone `v2.3` Closure (`Phase 123`):** Published `contracts/reports/full-flywheel-knob-matrix-before-after.v1.json` and `docs/src/architecture/full-flywheel-knob-matrix-audit-2026.md`, and verified all test suites and `vox ci pre-push`.

---

## v2.2 GitHub CI Green Sweep, Persistent Chat Task/Context Backend RPCs, Zero-Finding Drift Audit & Full-Stack Closure (Shipped: 2026-10-20)

**Phases completed:** 4 phases (`Phases 116–119`), 4 plans, 16/16 requirements verified (`100%`)
**Archive:** [`milestones/v2.2-ROADMAP.md`](milestones/v2.2-ROADMAP.md) | [`milestones/v2.2-REQUIREMENTS.md`](milestones/v2.2-REQUIREMENTS.md)

**Key accomplishments:**

- **GitHub CI Red Gate & DB Baseline Digest Remediation (`Phase 116`):** Synchronized `repository_baseline_digest_hex` in `contracts/db/baseline-version-policy.yaml` (`0` baseline digest warnings), fixed `distribution-parity` (`crates/voxup/src/profiles.rs` `[[crates]]` parsing + Tier 2 entries in `crates/_public.toml`, `10/10` tests passing), fixed `Build Starlight` (`docs/src/community/leaderboard.mdx` + `VoxLeaderboardArena.astro`, `855` pages built) and `Internal Markdown link check` (`5402` internal links PASS), and replaced the 14-minute `vox-cli` build in `.github/workflows/cr-l8-corpus-feedback.yml` with lightweight `vox-langtool`.
- **Persistent Backend-Synchronized Chat Task & Session Context RPCs (`Phase 117`):** Shipped `hopper_rename` in `crates/vox-gui/src/commands/hopper.rs` (`vox_hopper_rename` tool route), plus `set_context_budget_strategy`, `get_session_context_pins`, and `set_session_context_pins` in `crates/vox-gui/src/commands/context_budget.rs`, registered in `crates/vox-gui/src/main.rs` and wired in `ChatExecutionRail.tsx`, `ChatExecutionRail.test.tsx`, and `tauriMock.ts`.
- **Zero-Finding `vox-drift-check` Sweep (`Phase 118`):** Eliminated all 23 workspace `vox-drift-check` warning findings (`serde-default-dup`, `version-string`, `reqwest-bypass`) and updated `/Users/brbrainerd/.cargo/bin/vox-drift-check` (`✓ No drift findings at Warning level or above.`).
- **Full-Stack Verification & Milestone `v2.2` Closure (`Phase 119`):** Verified `cargo test -p vox-gui -- context_budget hopper plan_panel` (`22/22`), Vitest (`107/107` tests across 4 suites), and `vox ci pre-push` (`all checks passed in 24795ms` with `0` errors and `0` warnings).

---

## v2.1 Editable Chat Task/Context Controls, 32-Surface Mechanical & WCAG 2.2 AA Hardening, Self-Healing Pre-Push Automation Chain & Universal CV Flywheel Convergence (Shipped: 2026-10-20)

**Phases completed:** 4 phases (`Phases 112–115`), 4 plans, 16/16 requirements verified (`100%`)
**Archive:** [`milestones/v2.1-ROADMAP.md`](milestones/v2.1-ROADMAP.md) | [`milestones/v2.1-REQUIREMENTS.md`](milestones/v2.1-REQUIREMENTS.md)

**Key accomplishments:**

- **Editable Chat Task List, Session Context Pinning & Token Budget Controls (`Phase 112`):** Upgraded `ChatExecutionRail.tsx` (`execution-rail-add-task-input`, `execution-rail-task-toggle-${id}`, `execution-rail-task-edit-${id}`, `execution-rail-task-remove-${id}`, `execution-rail-context-pins`), `PlanPanel.tsx` (multi-status inline editing, completion toggle, removal, and standalone `+ Add step`), `ChatSurface.tsx` (`kb-context-inspector-drawer` with `@kb` scope buttons, numeric token budget cap, and inline citation editing), `Loquela.tsx` (`loquela-chip-edit-${id}`), and `ContextWindowMeter.tsx` (`used / max · %`, `auto-compact @ 80%`, and sibling compaction strategy button `context-meter-strategy-btn`).
- **All-32 GUI Surface Mechanical & Visual Hardening, Capability Index Schema Parity & WCAG 2.2 AA Enforcement (`Phase 113`):** Upgraded `MeshView.tsx`, `App.tsx` (`runs`, `tasks`, `capabilities`, `browser`), `SubAgentsView.tsx`, `ScientiaDashboard.tsx`, `ArchiveStatusSummary.tsx`, `RepositoryView.tsx`, `PoliciesView.tsx`, `ApprovalsView.tsx`, `CommandCatalogForm.tsx`, and `CodeRabbitView.tsx`; restored `CapabilityGroup`/`docOrigin` in `capabilityIndex.generated.ts`; eliminated all WAI-ARIA `nested-interactive` and `axe-core` WCAG 2.2 AA violations across `ResearchView.tsx`, `LeaderboardArenaPanel.tsx`, `ChatSurface.tsx`, and `ContextWindowMeter.tsx`; and verified `178/178` unit/guard tests across 8 Vitest suites.
- **Self-Healing Pre-Push & SSOT Automation Chain (`Phase 114`):** Wired in-process `VOX_AUTO_HEAL` across `pre_push.rs` and `docs.rs` (`rustfmt` chunks, `line-endings`, and 10 SSOT generators), configured the `lefthook.yml` auto-heal and retry chain, and synchronized `/Users/brbrainerd/.cargo/bin/vox` with the workspace `vox-cli` binary.
- **Universal 32-Surface `@ngi/platform-ssot` CV Flywheel Convergence (`Phase 115`):** Verified `flywheelConverged === true` across `chat-default` (`0.8502 -> 0.9200`), `loquela-knobs` (`0.8280 -> 0.9216`), `chat-dock-multi` (`0.7985 -> 0.9212`), and `urbs-civilization` (`0.8228 -> 0.9180`), and `combinedScore: 0.9142–0.9254` across all 32 `SURFACE_REGISTRY` views with `0` axe/overflow/clipping defects.

---

## v2.0 Persistent Organic Multi-Spoke Search-to-Compilation Benchmark, Live Execution Oracles & Longitudinal Evaluation Flywheel (Shipped: 2026-10-19)

**Phases completed:** 4 phases (`Phases 108–111`), 4 plans, 16/16 requirements verified (`100%`)
**Archive:** [`milestones/v2.0-ROADMAP.md`](milestones/v2.0-ROADMAP.md) | [`milestones/v2.0-REQUIREMENTS.md`](milestones/v2.0-REQUIREMENTS.md)

**Key accomplishments:**

- **15-Task 5-Spoke Organic Search-to-Compilation Suite & Decontamination Guard (`Phase 108`):** Shipped `canonical_organic_search_to_compilation_suite_15` (`3` organic developer search-to-compilation tasks per spoke across `VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`), `discover_workspace_evidence`, `verify_organic_benchmark_decontamination`, and `contracts/reports/mens-organic-search-spoke-scorecard.v1.json` (`100.0%` Recall@5, `1.000` MRR, `100.0%` Grounding, `100.0%` Compilation, `100.0%` E2E `pass@1`).
- **Deep Compilation & Functional Runtime Oracles Across All 5 Spokes (`Phase 109`):** Upgraded `verify_deep_compilation_for_spoke` with `vox_compiler` (`lex -> parse -> HIR -> typeck -> fmt -> Interpreter::run_module`) for `VoxLang`, `syn::parse_file` + `quote::ToTokens` + policy detectors for `RustWorkspace`, live directory inventory verification for `StructuredShell`, `vox-mcp-registry` + inline `vox_compiler::pipeline::check_file` (`0` errors) for `McpToolChain`, and `TaskPlanEnvelope` file existence + `classify_shell_safety_floor` gate verification for `AgentHarness`.
- **Longitudinal `VoxDb` Persistence, `ModelBenchCase` Telemetry & Multi-Metric Ratchets (`Phase 110`):** Shipped `persist_organic_scorecard_to_db`, `load_organic_scorecard_from_db`, `list_organic_scorecard_history_from_db`, `verify_organic_scorecard_non_decreasing_trend`, `record_organic_scorecard_telemetry_to_db` (`vox.model.bench_case` verified via `VoxDb::audit_research_metric_shapes`), and expanded `contracts/eval/spoke-regression-ratchet.v1.json` + `verify_organic_search_ratchet` in `vox-audit`.
- **CLI & Axis GUI Longitudinal Scorecard Surfaces (`Phase 111`):** Added `vox model eval --organic-search-spokes` (`execute_organic_search_spoke_evaluation_in_dir`), `OrganicSearchSpokeScorecardDto` in `vox-gui` (`mens_run_reports.rs`), and `data-testid="mens-organic-scorecard"` in `MensTrainingView.tsx`.

---

## v1.9 Chat Default Surfaces CV & Code Audit Remediation, Progressive-Disclosure Composer, Rich Markdown Transcript, Full-Height Execution Rail & CV Flywheel Convergence (Shipped: 2026-10-19)

**Phases completed:** 4 phases (`Phases 104–107`), 4 plans, 14/14 requirements verified (`100%`)

---

## v1.8 Shared-Package CV Flywheel, Chat/Loquela Knob & Datatype Unification, VS Code Multi-Panel Docking, Urbs Civilization Live Taps, Browser Songhand/Playwright Harness & `ngi.video` Marketing Copy/Screenshot CV Flywheel Promotion (Shipped: 2026-10-19)

**Phases completed:** 6 phases (`Phases 98–103`), 6 plans, 32/32 requirements verified (`100%`)

---

## v1.7 Centralized Postgres Telemetry, Private `vox-ops` Extraction, Ludus 3-Way SoC, Zero-Cheating Leaderboard, Codebase/History Scrub, Live Helios Deployment & 5-Spoke MENS Safetensors Evaluation (Shipped: 2026-10-19)

**Phases completed:** 9 phases (Phases 89–97), 18 plans, 70/70 requirements verified (`100%`)

**Key accomplishments:**

- **Telemetry Surface Audit Remediation, Strong Typing, Multi-Surface Model Router & Local `VoxDb` Shape Audit (`Phase 89`):** Wired `init_process_telemetry` across `vox-orchestrator-d` and `vox-gui`, expanded `ResearchMetricsSink` to all 26 `TelemetryEvent` variants with `VoxDb::audit_research_metric_shapes` (`TypedResearchMetricRow`), implemented batched OTLP HTTP uploader `upload_pending_otlp` (`<= 100` records / `<= 256 KiB`) + `vox telemetry preview`, removed prototype `server/telemetry/`, and verified the model router across CLI, daemon, MCP, and GUI surfaces.
- **Private `vox-ops` Repository & Three-Role Fenced Postgres on Live `helios-postgres` (`Phase 90`):** Bootstrapped the 6-crate `~/dev/vox-ops` workspace (`vox-ops-db`, `vox-ops-ingest`, `vox-ops-syndication`, `vox-ops-ludus`, `vox-ops-intelligence`, `vox-ops-server`) and provisioned the three-role fenced PostgreSQL 17 database (`vox_ops_owner`, `vox_ingest_app`, `vox_ops_app`) on live `helios-postgres` (`q40ck888ggkkcsosgcwgc0oo`), verified with privilege vector `t|f|f|t|f|f|t` and $k \ge 20$ `SECURITY DEFINER` analytical views.
- **Social Media Syndication, Multi-User Ludus Extraction, 3-Way Gamification SoC Split & Zero-Cheating Leaderboard Surfaces (`Phase 91`):** Extracted all 9 live social media adapters to `vox-ops-syndication` (`syndication.outbox`), added `SyndicationHandoffPayloadV1` (`contracts/scientia/syndication-handoff.v1.schema.json`), moved Tier 1 operational `agent_telemetry` (`agent_events`, `cost_records`, `agent_sessions`) to `crates/vox-db/src/store/ops_agent_telemetry.rs`, eliminated shadow telemetry (`hint_telemetry.rs`, `lsp_telemetry.rs`), extracted multi-user Ludus to `vox-ops-ludus`, and shipped `VerifiedLeaderboardSnapshotV1` (`BLAKE3` receipt hash chain + `Ed25519` envelope signature) with `VoxLeaderboardArena.astro` (`voxlang.org`) and `LeaderboardArenaPanel.tsx` (`vox-gui`).
- **Community Intelligence Flywheel, Live Helios Deployment & Git History Scrub (`Phase 92`):** Shipped `CommunityIntelligenceBundleV1` (`#[serde(deny_unknown_fields)]`, `Ed25519` verification via `vox_crypto::verify_signature_hex`, Empirical Bayes prior blending clamped to $\pm 15\%$ in `ModelRegistry::apply_community_intelligence_bundle`), `vox telemetry intelligence {status,sync,inspect}` CLI + GUI Settings controls, deployed container `vox-ops-server` live on `helios` (`127.0.0.1:8420`, verified live `GET /healthz`, `POST /v1/logs` -> `HTTP 202 Accepted` as `vox_ingest_app`, `GET /v1/intelligence/manifest.json`, and `GET /v1/ludus/leaderboard`), and verified working-tree/history scrub compliance via `scripts/verify-history-scrub.vox`.
- **5-Spoke MENS Gap Closure, 25-Problem Benchmark, Axis Harness Routing, GUI Scorecard & Live QLoRA Safetensors Evaluation (`Phases 93–97`):** Closed `88-REVIEW.md` findings (`MENS-GAP-01..04`), expanded the 5-spoke benchmark to 25 problems (`24.0% -> 72.0% -> 100.0% pass@1`), wired `ModelRegistry::register_calibrated_mens_adapter` + `route_spoke_task` + `vox model eval --mens-five-spokes`, added `MensSpokeScorecardDto` in `MensTrainingView.tsx`, and verified live `.safetensors` adapter checkpoints across all 25 problems (`LIVE-QLORA-01..04`).

---

## v1.6 Production Stability, Zero-Mock Surface Reality & Mens Continuous Training Flywheel (Shipped: 2026-10-16)

**Phases completed:** 12 phases (Phases 77–88), 25 plans, 100% requirements verified

**Key accomplishments:**

- **Multi-Spoke Training Hygiene & Round-Trip Verification (`Phases 77–78`):** Shipped schema-validated `TrainingRecordV2` across all 5 Vox spokes (`VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`), 3-gram Jaccard deduplication + holdout decontamination, `VoxDb` Trusty dataset snapshots (`trusty://vox-dataset/<hash>`), and the deterministic in-process 5-spoke round-trip harness emitting `MensSpokeScorecardV1` (`1.0` pass@1 across all 5 spokes).
- **Core Daily Driver Stability, Daemon Auto-Bootstrap & Gateway/IPC Parity (`Phase 79`):** Added supervised `vox-orchestrator-d` auto-bootstrap, health probes (`orch.ping`), and exponential-backoff reconnect across `vox-gui`, `vox chat`, and `vox term`; wired all remaining unrouted gateway/daemon RPCs in `contracts/gui/command-backends.v1.yaml`; migrated `gui_db_pool.rs` to daemon-owned `VoxDb` RPCs; and added inline PTY/daemon reconnect recovery in Axis Console (`Console.tsx` / `TerminalTab.tsx`).
- **Real-Backend E2E Verification Harness (`Phase 80`):** Built the hermetic real-backend test harness (`crates/vox-gui/tests/real_backend_harness.rs` + `crates/vox-gui/ui/e2e/real-backend/`) booting a live `vox-orchestrator-d` + temporary `VoxDb` + HTTP/IPC Tauri command bridge, verified Playwright E2E journeys (`Chat`/`Loquela`, `Terminal` `SessionHost`, `⌘K` `Palette`, `Explorer`, `Library`), and enforced `honestyScan.ts` across all 32 GUI surfaces.
- **`.vox` Language, Compiler, Interpreter & Codegen End-to-End Parity (`Phase 81`):** Verified `vox check` and `vox run` across `scripts/*.vox`, all `vox init` templates (`default`, `simulation-rpg`, `script`), span-accurate `vox check --for-llm` diagnostics, and full execution parity across `--interp`, `RustAxum`, `RustTauri`, and `TypeScript`.
- **Operational & Data-Intensive GUI Surface Hardening (`Phases 82–83`):** Live-wired `SettingsView` (`Clavis`, OAuth, `Oratio`), `ModelsView` (`Mens`), `ApprovalsView`, `PoliciesView`, `SubAgentsView`, `MemoryView` (`Library`/Inbox/Health), `ResearchView`, `ScientiaSurface`, `CodeAtlasCanvas`, `MeshView`, and ring-buffered `Dashboard`/`Runs`/`Tasks`/`Activity`/`Gamify` event streams; ratcheted CLI `side_effect_class` (`906 -> 647 <= 756`) and dedicated GUI surfaces (`12 -> 16 >= 16`).
- **Workspace CI Gate Parity, `vox doctor --fix` & Mens Continuous Flywheel Closure (`Phases 84–88`):** Shipped `vox doctor --fix` auto-remediation, curriculum-weighted QLoRA + DPO batch pipeline (`floor 0.10`), longitudinal 5-spoke scorecards and promotion gates (`vox-eval`), audit-to-corpus harvesting + verified repair DPO pairing + spoke regression ratchets (`vox-audit`), and the 2-cycle autonomous burn-in (`pass@1` `0.40 -> 0.80 -> 1.00`, `0` spoke regressions) persisted in `VoxDb` CAS.

---

## v1.5 Unified Holistic Stream (Shipped: 2026-10-16)

**Phases completed:** 74 phases (Waves 0–6, Phases 01–76), 241 plans, 100% requirements verified

**Key accomplishments:**

- **Wave 0 (`Phase 05` — Convergence):** Converged all 6 parallel GSD workstreams (`kb`, `discoverability`, `docs-freshness` v1.3/v1.4, `autonomy-ux`, `code-atlas`, `research-trial-flywheel`) and 42 git worktrees onto `feat/unified-2026-q4` with zero worktree deletions and zero `ssot-drift` or `crate-edges` violations.
- **Wave 1 (`Phases 12, 07, 07.1, 25, 32` — Safety, Trial & Graph Floors):** Shipped the action-only Autonomy Safety Floor (`permission-modes.v2.yaml` + `classify_floor`, 0/34 surviving mutants), `trials` domain at VoxDb schema baseline 97 (`TrialCampaignId` Trusty URI, atomic persistence with `PersistFailpoint`, code-gen benchmark provenance), `vox-graph-reader` v5 (`EXTRACTOR_VERSION = "5"`, `AtlasQuery` 400-node/800-edge LOD budget), and 100% docs trust signal calibration (`810/810 verified`, `0 drifting`, `0 broken`, `0 unanchored`).
- **Wave 2 (`Phases 33–34, 13–17, 37–42, 46, 49, 52–53, 55–56, 58, 60–62` — Core Backend Engines):** Delivered OpenWiki Subsystem Hubs & deep crate API pages with validated Mermaid diagrams, consolidated 29 duplicate doc clusters (`954 -> 7` baseline deficits), shipped `OversightLevel` (`1..5`) + `AttentionInbox` at DB baseline 98, local Metal/CPU KB embeddings + `@kb` context injection + `kb_findings` inbox + Top-25 page-one rewrites + staleness detector, Research Trial Race Harness (`vox-eval::trial_flywheel`), `vox-term` CLI attach + `TermDispatch::run_agent_command` + redacted `"term.context"`, LLM-first Nushell + Vox-Shell structured pipelines, Deep Research + SCIENTIA publication pipeline closure, autonomous Mermaid validation & LLM self-repair (`vox_validate_mermaid`), continuous `README.md`/`voxlang.org` sync, Open Collective + NLnet/Rust/Compute grant pack, Tier 2.5 `VOX_GIT_HASH` codegen fallback, and 7-crate standalone `crates.io` unbundling (`crates/_public.toml`).
- **Wave 3 (`Phases 06.4, 26–31, 43–45, 18–19, 47–48, 50–51, 54, 57` — Sequenced GUI & `voxlang.org` Unification):** Built the Capability Explorer (`ExplorerView.tsx`, 17 domains, 4 status badges, 4-tab coverage audit, `gui-coverage-gate` CI gate with 294/294 CLI families and 349/349 MCP tools covered or audited), Code & Architecture Atlas (`CodeAtlasCanvas.tsx` + diff overlay + chat dock + click-to-`@`), KB Library (`Library`, per-turn Context Inspector drawer, Inbox/Health tabs), `Oversight: 1..5` composer control + `autonomy-claims.v1.yaml` ledger, Axis Console Warp-Parity WebGL terminal (`TerminalTab.tsx` + `BlockStreamView.tsx`), Axis Harness River (`HarnessRiverPanel.tsx` + 4-scenario Playwright dogfood suite), bidirectional Chat ↔ Terminal co-execution (`L1..L5` security levels + `@term` chips), Unified 7-source IntelliSense (`intellisenseEngine.ts` + `IntelliSensePopover.tsx`), Scientia Publication Studio GUI (`ResearchDagCanvas.tsx`), and the `voxlang.org` brand overhaul, interactive playground, and collapsible sidebars (`36/36` Playwright tests).
- **Wave 4 (`Phases 35–36, 06.6, 59, 63` — Self-Healing Automation & Verification):** Shipped the constrained LLM doc claim verifier (`llm_verifier`), guarded self-healing docs draft-PR bot (`self_heal_bot`), 5 cold-start time-to-find journeys (`<= 60s`, `<= 3` interactions), full 34-mutant safety floor gate + milestone-wide `/gsd-code-review --fix` + resolution of all 22 `audit-open` items, and the `N=100` `.vox` vs. TypeScript empirical benchmark preprint + `CR-L8` diagnostic→repair→corpus flywheel.
- **Wave 5 (`Phases 64–69` — Cross-Portfolio Runtime Completeness):** Delivered polyglot `@tool` & `@form` auto-surface (`<ToolForm />`, CLI, MCP, OpenAPI 3.1), `Stream[T]` SSE + `Channel[Send, Recv]` WebSockets + reactive `db.Table.subscribe` + managed `ProcessRun`, algebraic `uses { Spend, Write, ExternalApi }` effect guards + `<WaitingNotice>` + `@ranker`/`@eval_shadow` ordering gates, `@embed`/`@searchable` hybrid search + partial `db.Table.patch` + `@table(scoped_by: tenant)`, presigned `Upload[T]`/`BlobStore` + `@webhook` verification + `@rate_limit`/`@metered` credit ledger, and the deterministic tick/skill engine reference template (`vox init --kind=simulation-rpg`).
- **Wave 6 (`Phases 70–76` — Worktree Land Wave, Clavis Next, Mens ML, Populi Mesh, SOTA Routing & Humane UX):** Harvested `land/w1..w5` and closed all 12 tasks of the Nightly-Green plan + `vox fmt` `layout_only` token guard, shipped Clavis Next Parts 1 & 2 (`clavis://` URI resolver, `vox secrets import/acquire/probe/rotate/sync`, zero-trust `vox_secrets::broker` with opaque `clavis-handle://` handles), integrated Apple Silicon unified-memory Qwen 3.8 27B + hybrid GatedDeltaNet linear-attention multi-token decode in `vox-mens`, ported `PopuliHttpOp` + durable A2A mailbox onto `vox-mesh-transport` iroh QUIC with fail-closed dispatch, unified MENS 2026 `.vox` script writer prompts/generators with 5-tier SOTA `vox chat` routing (`free`, `auto`, `efficient`, `balanced`, `genius`), and shipped humane zero-markup `TurnCostReceipt`, anti-FOMO `RestDayPolicy`, anti-sycophancy directness, and `FatigueMonitor` session wrap-up UX.

---

## v1.3 Self-Maintaining Public Docs (Shipped: 2026-10-13)

**Phases completed:** 6 phases (Phases 20, 20.1, 21, 22, 23, 24), 33 plans, 32 verified

**Key accomplishments:**

- Restored `voxlang.org` Cloudflare Pages deployments with fast local staging, added single-issue failure escalation and `ci-liveness` checks, fixed git-derived page dates across symlinked content, added frontmatter status banners and `noindex` for non-current pages, and isolated research/roadmap pages into the `Internals` sidebar section.
- Shipped collapsible left hover-rail and right table-of-contents sidebars with responsive wide-article layout on `voxlang.org`, verified by 12/12 Playwright visual-stepper tests across mobile, tablet, and desktop viewports in light and dark modes.
- Built deterministic claim extraction (`vox-doc-pipeline::claims`), `vox audit docs` CLI command, machine-populated Docs Reality Audit findings, and the corpus prune report (`prune-report.v1.json`) auditing all 972 doc pages.
- Shipped the code-anchored freshness ledger (`ledger.v1.json`), tighten-only baseline ratchet (`baseline.v1.json`), expiring suppressions, PR impact reverse-index comment (`vox ci pr-impact`), and executable `.vox` tutorial test harness (`TUT-02`).
- Generated `env-vars.generated.md`, `config-keys.generated.md`, `mcp-tools.generated.md`, and 127 crate API documentation pages (`docs/src/api/*.generated.md`) under `ssot-drift`, plus CLI `build_catalog()` compliance verification and retired-decorator prose checks.
- Delivered reader trust banners and sidebar badges driven by `ledger.v1.json`, consolidated `llms.txt` and `/llms-full.txt`, committed the 10-case golden drift evaluation set (`golden-drift-eval-set.v1.json`), and ratified the Decision D9 in-house build verdict (`docs-bot-evaluation-and-pivot-rule-2026.md`).

---

## v1.0 Architectural Housekeeping (Shipped: 2026-09-30)

**Phases completed:** 6 phases, 23 plans, 11 tasks

**Key accomplishments:**

- D-03 verification pass confirms all 10 REQ-dead-crate-delete crates and their prerequisite type/logic migrations already landed; repointed the stale all-features CI matrix entry; operator ratified SC#3's actual host-crate disposition.
- Verified catalog.toml's execution-api/stub-check ghost-entry bookkeeping is already accurate and added a KEEP-FROZEN disposition comment for the three frozen crates, closing ROADMAP Phase 1 Success Criterion 4.
- Ratified ADR-045 as Accepted across all three GSD artifacts (doc body, intel classification, decisions index) and recorded ADR-037's desktop-convergence clause as implemented with three cited, verified code paths — zero code changes, pure documentation ratification.
- Audited vox-gui's command surface (clean — no violations), recorded the finding and one documented exemption in the surface-ownership contract, wrote the Vox-native/React authoring boundary in a machine-readable field plus matching prose, and fixed the one reference page still naming a deleted crate as the canonical GUI.
