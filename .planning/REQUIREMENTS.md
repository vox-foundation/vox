# Requirements: Vox v1.6 — Production Stability, Zero-Mock Surface Reality & Value-Ordered Reliability

**Defined:** 2026-10-16
**Core Value:** The compiler, orchestrator, runtime, knowledge base, and public documentation site (`voxlang.org`) that everything else depends on must keep building, staying verified, and passing CI from a single unified worktree (`/Users/brbrainerd/dev/vox`).
**Ordering Principle:** Strictly ordered by user/developer value — **Highest Value First** (`Phases 77–84`).

---

## v1.6 Requirements

### Foundation (Completed in-tree): Multi-Spoke Training Hygiene & Round-Trip Verification (`CORPUS` / `SPOKE` — Phases 77–78)

- [x] **CORPUS-01..06**: Schema-validated `TrainingRecordV2`, 5-spoke admission gates (`VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`), 3-gram Jaccard deduplication + holdout decontamination, and Trusty-URI dataset snapshots in `VoxDb` (`Phase 77`).
- [x] **SPOKE-01..06**: Deterministic 5-spoke text-in / text-out round-trip verification harness (`crates/vox-corpus/src/training/spoke_roundtrip.rs`) and `MensSpokeScorecardV1` (`contracts/reports/mens-spoke-scorecard.v1.json`) (`Phase 78`).

---

### Priority 1: Core Daily Driver Stability, Daemon Auto-Bootstrap & Gateway/IPC Parity (`CORE` — Phase 79)

- [x] **CORE-01**: Launching `vox-gui` (`Axis`), `vox chat`, or `vox term` on a machine where `vox-orchestrator-d` is not running automatically spawns, health-probes (`orch.ping`), and connects to a supervised local `vox-orchestrator-d` instance with exponential-backoff reconnect on daemon restart (`crates/vox-gui/src/commands/daemon.rs`, `crates/vox-cli/src/commands/chat.rs`, `crates/vox-term/`).
- [x] **CORE-02**: `Loquela.tsx` and `ChatSurface.tsx` hydrate model tiers, active oversight level (`1..5`), session history, and context budget from live daemon RPCs on startup and auto-resync when the daemon reconnects, eliminating silent cold-start degraded state (`crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx`, `ChatSurface.tsx`).
- [x] **CORE-03**: All unrouted daemon/gateway endpoints documented in `contracts/gui/command-backends.v1.yaml` are wired end-to-end: `chat.rs` (`secretary_confirm_task` / `SUBMIT_TASK`), `chat_turn.rs` (background `SUBMIT_TASK`), `control_plane.rs`, `mission_control.rs`, and `research.rs` (`start`/`persist` over gateway/daemon RPC).
- [x] **CORE-04**: `crates/vox-gui/src/commands/gui_db_pool.rs` (`class: db` in `command-backends.v1.yaml`) is migrated to daemon-owned `VoxDb` RPCs so `vox-gui` never opens a competing SQLite/Turso file lock against `vox-orchestrator-d`.
- [x] **CORE-05**: Axis Console (`Console.tsx`, `TerminalTab.tsx`, `BlockStreamView.tsx`) and `vox term` survive daemon restart and PTY shell exit cleanly with an inline `"Reconnect / Respawn Shell"` control and zero orphaned PTY child processes.

---

### Priority 2: Real-Backend E2E Verification Harness (Zero-Mock Surface Proof) (`REAL` — Phase 80)

- [x] **REAL-01**: A hermetic real-backend test harness (`crates/vox-gui/tests/` + `crates/vox-gui/ui/e2e/real-backend/`) boots a live `vox-orchestrator-d` + temporary `VoxDb` + live HTTP/IPC command bridge on an ephemeral localhost port without `installTauriMock` browser stubs.
- [x] **REAL-02**: Playwright real-backend E2E specs verify the daily-driver `Chat` + `Loquela` loop (`vox_chat_session_*`, `orch.oversight_get`/`set`, `@kb` scope toggle, `TurnCostReceipt` badge) against the live Rust daemon and `VoxDb`.
- [x] **REAL-03**: Playwright real-backend E2E specs verify the live `Terminal` (`SessionHost` `term.open` → `term.submit` → `OSC-633` block completion → `"term.context"` redaction) and `⌘K` Universal Palette / `ExplorerView.tsx` read-only CLI execution against the live Rust backend.
- [x] **REAL-04**: `honestyScan.ts` (`crates/vox-gui/ui/src/components/surfaces/__guards__/honestyScan.ts`) is expanded and gated across all 32 GUI surfaces in `crates/vox-gui/ui/src/components/surfaces/` to forbid dead click handlers, unguarded mock fallbacks in production builds, and placeholder prose.

---

### Priority 3: `.vox` Language, Compiler, Interpreter & Codegen End-to-End Execution Parity (`VOX` — Phase 81)

- [x] **VOX-01**: Every first-party `.vox` script under `scripts/*.vox` passes `vox check` and executes cleanly under `vox run` with structured exit status and zero panics or parser/typeck regressions.
- [x] **VOX-02**: End-to-end compilation and execution parity tests verify that canonical `.vox` programs using `@table`, `@query`, `@mutation`, `@tool`, `@form`, `Stream[T]`, `Channel[Send, Recv]`, `workflow`, `activity`, and `actor` produce identical observable results across `--interp` and compiled `RustAxum` / `RustTauri` / `TypeScript` codegen targets.
- [x] **VOX-03**: `vox init` project templates (`default`, `simulation-rpg`, `script`) scaffold, pass `vox check`, and build cleanly out-of-the-box using the Tier 1/2/2.5 runtime dependency resolver (`crates/vox-codegen/src/codegen_rust/emit/mod.rs`).
- [x] **VOX-04**: Compiler diagnostic messages emitted by `vox check` and `vox check --for-llm` include actionable span locations, fix hints, and zero internal compiler panics (`unwrap()` on user syntax errors) across malformed input fuzz/corpus cases.
- [x] **VOX-05**: `vox test` executes `.vox` `@test` declarations, `@ranker` ordering gates (`--ordering-gate`), and markdown `.vox` doctests deterministically with clear pass/fail summaries.

---

### Priority 4: High-Value Operational GUI Surfaces & CLI Side-Effect / Dedicated Floor Ratchet (`OPS` — Phase 82)

- [x] **OPS-01**: `SettingsView.tsx` (`Clavis` secret status, `vox secrets acquire/import/probe/rotate`, OAuth login with fallback URL, `Oratio` STT/microphone probe, and `ServerConnectionSection`) operates end-to-end against real `crates/vox-gui/src/commands/{secrets,oauth,stt_config,user_config,axis_remote}.rs` handlers with actionable validation states.
- [x] **OPS-02**: `ModelsView.tsx` (`MensServePanel.tsx`, `MensTrainingView.tsx`, `RoutingExplainer.tsx`, and `Matrix.tsx`) reads live `~/.vox/cache/model-catalog.v1.json` and `vox-mens` run telemetry, clearly distinguishing live catalog routing from offline fallback scales and validating run directories before spawning `vox mens`.
- [x] **OPS-03**: `ApprovalsView.tsx`, `PoliciesView.tsx`, `NeedsYouSurface.tsx`, and `SubAgentsView.tsx` (`mission_control.rs` + `hopper.rs` + `policy.rs`) execute real approval/denial, policy inspection, and sub-agent task control round-trips against `vox-orchestrator-d`.
- [x] **OPS-04**: Handler-evidenced `side_effect_class` (`read_only`, `none`, `writes_files`, `spawns_process`, `network`, `destructive`) is classified across an additional `>= 150` CLI rows in `contracts/operations/catalog.v1.yaml`, ratcheting `contracts/capabilities/side-effect-ratchet.v1.json` downward from `906` to `649` (`<= 756`).
- [x] **OPS-05**: High-value `cli-only` command families (`secrets`/`clavis`, `doctor`, `db`, `audit`) are wired to dedicated GUI actions/surfaces, ratcheting `contracts/gui/dedicated-floor-ratchet.v1.json` upward from `12` to `16` (`>= 16`) dedicated top-level groups.

---

### Priority 5: Knowledge Base, Deep Research, Scientia, Code Atlas & Mesh Live Surface Hardening (`DATA` — Phase 83)

- [x] **DATA-01**: `MemoryView.tsx` (`Library` search, facet filter bar, `@kb` Context Inspector drawer in `ChatSurface.tsx`, `kb_findings` Inbox review, and KB Health tab) executes live queries, URL/note ingestion, and finding promotion/dismissal against `VoxDb` (schema baseline 98) with zero UI crashes on empty or large corpora.
- [x] **DATA-02**: `ResearchView.tsx` and `ScientiaSurface.tsx` (`DiscoveryInbox.tsx`, `DiscoveryReview.tsx`, `PublicationsView.tsx`, `ResearchDagCanvas.tsx`) run multi-wave research sessions, render live epistemic DAGs, promote finding candidates, and preview IMRaD Markdown / LaTeX / Replay Receipts against live `VoxDb` rows.
- [x] **DATA-03**: `CodeAtlasCanvas.tsx` and `VoxGraphStatusPanel.tsx` render the live workspace `vox_search_atlas` graph (`EXTRACTOR_VERSION = "5"`, 400-node / 800-edge LOD budget), branch diff overlay, and click-to-`@` insertion into `Loquela.tsx` with sub-200ms query latency.
- [x] **DATA-04**: `MeshView.tsx` and `crates/vox-gui/src/commands/mesh.rs` report live `Populi` iroh QUIC node status, A2A mailbox counts, and queue telemetry (`PopuliHttpOp`), displaying clear offline/single-node guidance when no remote mesh peers are joined.
- [x] **DATA-05**: `Dashboard.tsx`, `RunsView.tsx`, `TasksView.tsx`, `ActivitySurface.tsx`, and `GamifyView.tsx` (`HarnessRiverPanel.tsx` + `Urbs Map`) stay synchronized over live daemon event streams with bounded memory (ring-buffered event lists) during long-running sessions.

---

### Priority 6: Workspace CI Gate Parity, `vox doctor --fix` & Release Readiness (`GATE` — Phase 84)

- [x] **GATE-01**: `vox doctor` and `vox doctor --fix` verify and auto-remediate local developer & runtime prerequisites (daemon reachability, `VoxDb` schema baseline `98`, Clavis keyring/vault health, `~/.vox/cache/model-catalog.v1.json`, and Nushell/shell integration).
- [x] **GATE-02**: All repository SSOT and architecture gates pass simultaneously with zero drift: `vox ci ssot-drift`, `vox ci gui-surface-coverage`, `vox ci capability-index`, `vox ci pipeline-parity`, `vox ci crate-edges`, `vox-arch-check`, and `vox-doc-pipeline --lint-only`.
- [x] **GATE-03**: Frontend production build (`pnpm --dir crates/vox-gui/ui run build` and `pnpm --dir docs-astro run build`) and Rust workspace check (`vox-cli`, `vox-gui`, `vox-orchestrator-d`, `vox-orchestrator-mcp`, `vox-compiler`, `vox-codegen`) complete with zero errors and verified bundle-chunk budgets.
- [x] **GATE-04**: Full milestone `/gsd-code-review --fix` and `audit-open` verification confirm zero Critical/Warning defects and zero unresolved deferred items across `Phases 77–84`.

---

## Traceability

| Requirement | Phase | Priority Tier | Status |
|-------------|-------|---------------|--------|
| **CORPUS-01..06** | Phase 77 | Foundation (Multi-Spoke Training Data Schema & `VoxDb` Ingestion) | Complete |
| **SPOKE-01..06** | Phase 78 | Foundation (5-Spoke Text-In / Text-Out Round-Trip Verification) | Complete |
| **CORE-01..05** | Phase 79 | P1 (Highest — Core Daily Driver & Daemon/Gateway Parity) | Complete |
| **REAL-01..04** | Phase 80 | P2 (Real-Backend E2E Verification Harness) | Complete |
| **VOX-01..05** | Phase 81 | P3 (`.vox` Compiler, Interpreter & Codegen E2E Parity) | Complete |
| **OPS-01..05** | Phase 82 | P4 (Operational GUI Surfaces & CLI Ratchets) | Complete |
| **DATA-01..05** | Phase 83 | P5 (KB, Research, Scientia, Atlas & Mesh Live Hardening) | Complete |
| **GATE-01..04** | Phase 84 | P6 (Workspace CI Gate Parity, `vox doctor --fix` & Release Signoff) | Complete |
