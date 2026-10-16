# Roadmap: Vox Unified Holistic Stream (`unified`)

## Milestones

- ✅ **v1.0 Architectural Housekeeping** — Phases 1–6 (shipped 2026-10-01)
- ✅ **v1.3 Self-Maintaining Public Docs** — Phases 20–24 (shipped 2026-10-13)
- ✅ **v1.5 Unified Holistic Stream** — Phases 01–76 across Waves 0–6 (74 phases, 241 plans — shipped 2026-10-16)
- ✅ **v1.6 Production Stability, Zero-Mock Surface Reality & Mens Continuous Training Flywheel** — Phases 77–88 (12 phases, 25 plans — shipped 2026-10-16)

---

## Completed Milestone: v1.6 Production Stability, Zero-Mock Surface Reality & Mens Continuous Training Flywheel (`Phases 77–88`)

**Ordering Principle:** Strictly ordered by user/developer value — **Highest Value First**.

- [x] **Phase 77: Persistent Multi-Spoke Training Data Schema, Hygiene & `VoxDb` Ingestion** (`77-01`..`77-03` — completed 2026-10-16)
- [x] **Phase 78: Text-In / Text-Out Multi-Spoke Verification & Round-Trip Evals** (`78-01`..`78-02` — completed 2026-10-16)
- [x] **Phase 79: Core Daily Driver Stability, Daemon Auto-Bootstrap & Gateway/IPC Parity** (`P1 Highest` — `CORE-01..05` — completed 2026-10-16)
- [x] **Phase 80: Real-Backend E2E Verification Harness (Zero-Mock Surface Proof)** (`P2` — `REAL-01..04` — completed 2026-10-16)
- [x] **Phase 81: `.vox` Language, Compiler, Interpreter & Codegen End-to-End Execution Parity** (`P3` — `VOX-01..05` — completed 2026-10-16)
- [x] **Phase 82: Operational GUI Surfaces & CLI Side-Effect / Dedicated Floor Ratchet** (`P4` — `OPS-01..05` — completed 2026-10-16)
- [x] **Phase 83: Knowledge Base, Deep Research, Scientia, Code Atlas & Mesh Live Surface Hardening** (`P5` — `DATA-01..05` — completed 2026-10-16)
- [x] **Phase 84: Workspace CI Gate Parity, `vox doctor --fix` & Release Readiness** (`P6` — `GATE-01..04` — completed 2026-10-16)
- [x] **Phase 85: Native Mens QLoRA + DPO Pipeline, Curriculum Weighting & Adapter Hot-Swap** (`85-01`..`85-02` — `TRAIN-01..05` — completed 2026-10-16)
- [x] **Phase 86: Multi-Spoke Empirical Evaluation, Longitudinal Scorecards & Promotion Gates** (`86-01`..`86-02` — `EVAL-01..05` — completed 2026-10-16)
- [x] **Phase 87: Continuous Audit-to-Corpus Flywheel & Per-Spoke Regression Ratchets** (`87-01`..`87-02` — `AUDIT-01..05` — completed 2026-10-16)
- [x] **Phase 88: Full-Loop Autonomous Burn-In, Cross-Spoke Benchmark Proof & Milestone Closure** (`88-01`..`88-02` — `LOOP-01..04` — completed 2026-10-16)

---

## Phase Details (`v1.6` — Phases 77–88)

### Phase 77: Persistent Multi-Spoke Training Data Schema, Hygiene & `VoxDb` Ingestion
**Goal**: Establish a schema-validated, decontaminated, and database-persisted training corpus spanning all 5 Vox spokes (`VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`) in `crates/vox-corpus/`.
**Depends on**: Phase 76 (`v1.5` complete)
**Requirements**: `CORPUS-01`, `CORPUS-02`, `CORPUS-03`, `CORPUS-05`, `CORPUS-06`, `CORPUS-04`
**Plans**: 3/3 complete (`77-01`..`77-03`)

### Phase 78: Text-In / Text-Out Multi-Spoke Verification & Round-Trip Evals
**Goal**: Deliver a deterministic, in-process 5-spoke text-in / text-out round-trip verification harness (`SPOKE-01..06`) in `crates/vox-corpus` (`training::spoke_roundtrip`) emitting `MensSpokeScorecardV1`.
**Depends on**: Phase 77
**Requirements**: `SPOKE-01`, `SPOKE-02`, `SPOKE-03`, `SPOKE-04`, `SPOKE-05`, `SPOKE-06`
**Plans**: 2/2 complete (`78-01`..`78-02`)

### Phase 79: Core Daily Driver Stability, Daemon Auto-Bootstrap & Gateway/IPC Parity
**Goal**: Ensure the daily-driver `Chat` (`vox chat` + `ChatSurface.tsx`/`Loquela.tsx`), `Terminal` (`vox term` + `Console.tsx`), and `vox-orchestrator-d` daemon lifecycle work seamlessly from cold start and across local/remote gateway modes without competing DB locks or unrouted RPCs.
**Depends on**: Phase 78
**Requirements**: `CORE-01`, `CORE-02`, `CORE-03`, `CORE-04`, `CORE-05`
**Success Criteria** (what must be TRUE):
  1. Launching `vox-gui`, `vox chat`, or `vox term` when `vox-orchestrator-d` is stopped automatically starts, health-checks (`orch.ping`), and connects to a supervised daemon instance, and automatically reconnects if the daemon restarts (`CORE-01`).
  2. `Loquela.tsx` and `ChatSurface.tsx` hydrate model tiers, active oversight level (`1..5`), session history, and context budget from live daemon RPCs on mount and resync on daemon reconnect (`CORE-02`).
  3. All previously unrouted endpoints in `contracts/gui/command-backends.v1.yaml` (`chat.rs` `secretary_confirm_task`, `chat_turn.rs` background `SUBMIT_TASK`, `control_plane.rs`, `mission_control.rs`, and `research.rs` start/persist) route cleanly over the daemon/gateway client (`CORE-03`).
  4. `crates/vox-gui/src/commands/gui_db_pool.rs` (`class: db`) is replaced by daemon-owned `VoxDb` RPCs so `vox-gui` never contends with `vox-orchestrator-d` for a local SQLite/Turso file lock (`CORE-04`).
  5. Axis Console (`Console.tsx` / `TerminalTab.tsx`) and `vox term` recover cleanly from shell exit or daemon restart via an inline `"Reconnect / Respawn Shell"` action with zero orphaned PTY processes (`CORE-05`).
**Plans**: 2/2 complete (`79-01`..`79-02`)
- [x] `79-01`: Supervised `vox-orchestrator-d` auto-bootstrap, health probe & exponential-backoff reconnect across `vox-gui`, `vox chat`, and `vox term` (`CORE-01`, `CORE-02`, `CORE-04`, `CORE-05`)
- [x] `79-02`: Close all unrouted gateway/daemon RPCs in `command-backends.v1.yaml` (`CORE-03`)

### Phase 80: Real-Backend E2E Verification Harness (Zero-Mock Surface Proof)
**Goal**: Prove that Vox's core GUI surfaces work end-to-end against a real running `vox-orchestrator-d` + `VoxDb` + Rust command dispatcher in automated Playwright tests, rather than relying solely on `installTauriMock` browser stubs.
**Depends on**: Phase 79
**Requirements**: `REAL-01`, `REAL-02`, `REAL-03`, `REAL-04`
**Success Criteria** (what must be TRUE):
  1. A hermetic real-backend test harness boots a live `vox-orchestrator-d` + temporary `VoxDb` + HTTP/IPC Tauri command bridge on an ephemeral localhost port (`REAL-01`).
  2. Playwright real-backend E2E specs pass for the `Chat` + `Loquela` workflow (`vox_chat_session_*`, `orch.oversight_get`/`set`, `@kb` scope toggle, `TurnCostReceipt` badge) against live Rust handlers (`REAL-02`).
  3. Playwright real-backend E2E specs pass for `Terminal` (`SessionHost` `term.open` → `term.submit` → `OSC-633` block completion → `"term.context"` redaction) and `⌘K` Universal Palette / `ExplorerView.tsx` read-only CLI execution against the live Rust backend (`REAL-03`).
  4. `honestyScan.ts` passes across all 32 GUI surfaces in `crates/vox-gui/ui/src/components/surfaces/`, enforcing zero dead handlers, zero placeholder prose, and zero unguarded mock data in production builds (`REAL-04`).
**Plans**: 2/2 complete (`80-01`..`80-02`)
- [x] `80-01`: Ephemeral real-backend (`vox-orchestrator-d` + `VoxDb` + HTTP/IPC command bridge) fixture and `honestyScan.ts` 32-surface audit (`REAL-01`, `REAL-04`)
- [x] `80-02`: Real-backend Playwright E2E suite for `Chat`/`Loquela`, `Terminal` (`SessionHost`), `Palette` (`⌘K`), `Explorer`, and `Library` (`REAL-02`, `REAL-03`)

### Phase 81: `.vox` Language, Compiler, Interpreter & Codegen End-to-End Execution Parity
**Goal**: Guarantee that `.vox` automation scripts (`scripts/*.vox`), `vox init` templates, and full-stack `.vox` applications compile, typecheck, and execute identically across `--interp`, `--mode script`, `RustAxum`, `RustTauri`, and `TypeScript`.
**Depends on**: Phase 80
**Requirements**: `VOX-01`, `VOX-02`, `VOX-03`, `VOX-04`, `VOX-05`
**Success Criteria** (what must be TRUE):
  1. Every `.vox` script in `scripts/*.vox` passes `vox check` and executes cleanly via `vox run` (`VOX-01`).
  2. End-to-end parity tests verify that `.vox` modules using `@table`, `@query`, `@mutation`, `@tool`, `@form`, `Stream[T]`, `Channel[Send, Recv]`, `workflow`, `activity`, and `actor` compile and execute with matching semantics across interpreter and codegen targets (`VOX-02`).
  3. Every `vox init` template (`default`, `simulation-rpg`, `script`) scaffolds, passes `vox check`, and compiles cleanly out-of-the-box (`VOX-03`).
  4. `vox check` and `vox check --for-llm` emit span-accurate diagnostics with fix hints and never panic on malformed `.vox` input (`VOX-04`).
  5. `vox test` runs `.vox` `@test` blocks, `--ordering-gate` ranker invariants, and documentation `.vox` doctests deterministically (`VOX-05`).
**Plans**: 2/2 complete (`81-01`..`81-02`)
- [x] `81-01`: `scripts/*.vox` and `vox init` template compile/run verification + panic-free `vox check --for-llm` diagnostics (`VOX-01`, `VOX-03`, `VOX-04`)
- [x] `81-02`: End-to-end `--interp` ↔ `RustAxum`/`RustTauri`/`TypeScript` execution parity suite and `vox test` harness verification (`VOX-02`, `VOX-05`)

### Phase 82: Operational GUI Surfaces & CLI Side-Effect / Dedicated Floor Ratchet
**Goal**: Ensure all operational control surfaces (`Settings`/`Clavis`, `Models`/`Mens`, `Approvals`, `Policies`, `SubAgents`) are 100% live-wired and expand CLI side-effect classification and dedicated GUI coverage.
**Depends on**: Phase 81
**Requirements**: `OPS-01`, `OPS-02`, `OPS-03`, `OPS-04`, `OPS-05`
**Success Criteria** (what must be TRUE):
  1. `SettingsView.tsx` (`Clavis` secrets acquire/import/probe/rotate, OAuth login, `Oratio` STT probe, and `ServerConnectionSection`) executes real round-trips against `crates/vox-gui/src/commands/{secrets,oauth,stt_config,user_config,axis_remote}.rs` (`OPS-01`).
  2. `ModelsView.tsx` (`MensServePanel`, `MensTrainingView`, `RoutingExplainer`, `Matrix`) reflects live model catalog state and validates run directories before spawning `vox mens` (`OPS-02`).
  3. `ApprovalsView.tsx`, `PoliciesView.tsx`, `NeedsYouSurface.tsx`, and `SubAgentsView.tsx` execute real approval/denial, policy inspection, and sub-agent control round-trips against `vox-orchestrator-d` (`OPS-03`).
  4. At least 150 additional CLI rows in `contracts/operations/catalog.v1.yaml` receive handler-evidenced `side_effect_class`, ratcheting `contracts/capabilities/side-effect-ratchet.v1.json` from `906` to `<= 756` (`OPS-04`).
  5. High-value `cli-only` command families (`secrets`/`clavis`, `doctor`, `db`, `audit`) are wired to dedicated GUI workflows, raising `contracts/gui/dedicated-floor-ratchet.v1.json` from `12` to `>= 16` (`OPS-05`).
**Plans**: 2/2 complete (`82-01`..`82-02`)
- [x] `82-01`: Live-wire and verify `SettingsView` (`Clavis`, OAuth, `Oratio`), `ModelsView` (`Mens`), `ApprovalsView`, `PoliciesView`, and `SubAgentsView` (`OPS-01`, `OPS-02`, `OPS-03`)
- [x] `82-02`: Classify `>= 150` CLI `side_effect_class` rows (`906 -> 649 <= 756`) and promote `secrets`/`doctor`/`db`/`audit` to dedicated GUI surfaces (`12 -> 16 >= 16`) (`OPS-04`, `OPS-05`)

### Phase 83: Knowledge Base, Deep Research, Scientia, Code Atlas & Mesh Live Surface Hardening
**Goal**: Verify and harden all data-intensive and visual intelligence surfaces (`MemoryView`/`Library`, `ResearchView`, `ScientiaSurface`, `CodeAtlasCanvas`, `MeshView`, and `Dashboard`/`Runs`/`Tasks`/`Gamify`) against live `VoxDb` and `vox-orchestrator-d` state.
**Depends on**: Phase 82
**Requirements**: `DATA-01`, `DATA-02`, `DATA-03`, `DATA-04`, `DATA-05`
**Success Criteria** (what must be TRUE):
  1. `MemoryView.tsx` (`Library` search, facet filters, `@kb` Context Inspector drawer, `kb_findings` Inbox review, and KB Health tab) executes live queries, ingestion, and finding promotion/dismissal against `VoxDb` baseline 98 (`DATA-01`).
  2. `ResearchView.tsx` and `ScientiaSurface.tsx` run live multi-wave sessions, render epistemic DAGs, promote finding candidates, and preview IMRaD Markdown / LaTeX / Replay Receipts from `VoxDb` (`DATA-02`).
  3. `CodeAtlasCanvas.tsx` and `VoxGraphStatusPanel.tsx` render the live workspace `vox_search_atlas` graph (`400` nodes / `800` edges LOD), diff overlay, and click-to-`@` insertion into `Loquela.tsx` (`DATA-03`).
  4. `MeshView.tsx` reports live `Populi` iroh QUIC node status, A2A mailbox counts, and queue telemetry with clear single-node/offline guidance (`DATA-04`).
  5. `Dashboard.tsx`, `RunsView.tsx`, `TasksView.tsx`, `ActivitySurface.tsx`, and `GamifyView.tsx` stream live daemon events with bounded ring-buffer memory (`DATA-05`).
**Plans**: 2/2 complete (`83-01`..`83-02`)
- [x] `83-01`: Live-backend verification and hardening for `MemoryView` (`Library`/Inbox/Health), `ResearchView`, and `ScientiaSurface` (`DATA-01`, `DATA-02`)
- [x] `83-02`: Live-backend verification and bounded-memory streaming for `CodeAtlasCanvas`, `MeshView`, `Dashboard`, `RunsView`, `TasksView`, `ActivitySurface`, and `GamifyView` (`DATA-03`, `DATA-04`, `DATA-05`)

### Phase 84: Workspace CI Gate Parity, `vox doctor --fix` & Release Readiness
**Goal**: Provide a single `vox doctor --fix` self-heal command for local/runtime readiness and prove 100% simultaneous pass across all workspace CI, SSOT, architecture, and build gates.
**Depends on**: Phase 83
**Requirements**: `GATE-01`, `GATE-02`, `GATE-03`, `GATE-04`
**Success Criteria** (what must be TRUE):
  1. `vox doctor` and `vox doctor --fix` inspect and auto-remediate daemon reachability, `VoxDb` schema baseline `98`, Clavis keyring/vault health, `~/.vox/cache/model-catalog.v1.json`, and shell integration (`GATE-01`).
  2. `vox ci ssot-drift`, `vox ci gui-surface-coverage`, `vox ci capability-index`, `vox ci pipeline-parity`, `vox ci crate-edges`, `vox-arch-check`, and `vox-doc-pipeline --lint-only` all exit `0` with zero drift (`GATE-02`).
  3. `pnpm --dir crates/vox-gui/ui run build`, `pnpm --dir docs-astro run build`, and Rust compilation/tests across core crates complete with zero errors (`GATE-03`).
  4. Milestone `/gsd-code-review --fix` and `audit-open` pass with zero Critical/Warning issues and zero open deferred items (`GATE-04`).
**Plans**: 2/2 complete (`84-01`..`84-02`)
- [x] `84-01`: `vox doctor --fix` auto-remediation & full workspace SSOT/architecture/build gate verification (`GATE-01`, `GATE-02`, `GATE-03`)
- [x] `84-02`: Milestone-wide `/gsd-code-review --fix`, `audit-open` verification, and `v1.6` signoff (`GATE-04`)
