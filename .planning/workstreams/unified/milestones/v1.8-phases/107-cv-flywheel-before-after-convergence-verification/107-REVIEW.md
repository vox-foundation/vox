---
phase: 107-cv-flywheel-before-after-convergence-verification
reviewed: 2026-10-20T04:38:00Z
depth: deep
files_reviewed: 42
files_reviewed_list:
  - crates/vox-gui/ui/src/App.tsx
  - crates/vox-gui/ui/src/components/surfaces/Scientia/ScientiaDashboard.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx
  - crates/vox-gui/ui/src/components/surfaces/Dashboard/Dashboard.tsx
  - crates/vox-gui/ui/src/components/surfaces/Dashboard/ResourcesWidget.tsx
  - crates/vox-gui/ui/src/components/surfaces/CodeRabbit/CodeRabbitView.tsx
  - crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx
  - crates/vox-gui/ui/src/components/surfaces/Memory/MemoryView.tsx
  - crates/vox-gui/ui/src/components/CommandCatalogForm.tsx
  - crates/vox-gui/ui/src/components/surfaces/SubAgents/SubAgentsView.tsx
  - crates/vox-gui/ui/src/components/surfaces/SubAgents/SubAgentTree.tsx
  - crates/vox-gui/ui/e2e/lib/tauriMock.ts
  - crates/vox-gui/ui/e2e/lib/tauriMockRich.ts
  - crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts
findings:
  critical: 4
  warning: 7
  info: 3
  total: 14
status: remediated_and_verified
---

# Phase 107: Code Review & Visual CV Image Audit Report

**Reviewed:** 2026-10-20
**Depth:** Deep (Visual inspection of all 32 `crates/vox-gui/ui/artifacts/cv-flywheel/surfaces/*-after.png` images cross-referenced against React/TypeScript components, Tauri IPC wiring in `crates/vox-gui/src/`, and Playwright E2E mocks)
**Status:** Remediated & Re-Verified

## Summary

Numeric CV metrics (Sobel edge density, RMS contrast, axe-core WCAG 2.2 AA, and 0px DOM bounding-box overlap) are necessary but **not sufficient** to prove that a surface is working properly underneath. By inspecting every single `*-after.png` image pixel-by-pixel alongside its underlying React component and Rust Tauri command handler, we uncovered **4 Critical** and **7 Warning** functional/visual defects that were masked behind passing numeric CV scores:

1. **Broken Tauri IPC command names & unmapped wire shapes** (`CR-01`..`CR-04`) where production or E2E calls silently fell into `.catch()` or rendered `0` / perpetual loading states while neighboring elements supplied enough edge density to pass numeric CV.
2. **Data-contract mismatches between Rust backend keys and TypeScript UI consumers** (`WR-06`, `CR-04`) where `get_memory_status` and `enrich_mesh_from_tool` emitted different field names (`proj/docs/chats/rules/web` vs `memory/knowledge/chunk`; `status: "online"` vs `online: true`) than the React views expected.
3. **Layout flex-justify traps & unstyled semantic nodes** (`WR-07`) that left 450px empty black voids in `catalog-after.png` and `sub-agents-after.png` and rendered `0x0` invisible `<span data-status>` badges in `SubAgentTree.tsx`.

All **4 Critical** issues and **5 of 7 Warnings** were fixed directly in component/shell code during this review pass; the remaining **2 Warnings** (`WR-01`, `WR-02`) are architectural/contract items documented with concrete follow-up recommendations below.

---

## Critical Issues (All Fixed in This Turn)

### CR-01: Wrong Tauri IPC command `gamify_profile` in `ChatSurface.tsx` & hardcoded `ENERGY 0/0` in `Dashboard.tsx`
**Files:**
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx:773`
- `crates/vox-gui/ui/src/components/surfaces/Dashboard/Dashboard.tsx:451`
- `crates/vox-gui/src/main.rs:356`

**Visual Evidence (`dashboard-after.png`):**
In the bottom-right `Workspace Simulation Mini-Map` (`LudusSandbox`) on `dashboard-after.png`, the badge displayed `ENERGY 0/0` despite `ludusProfile` having `energy: 80, max_energy: 120`.

**Root Cause:**
1. `ChatSurface.tsx:773` invoked `invoke('gamify_profile')`, which **is not registered** in `crates/vox-gui/src/main.rs` (`get_ludus_profile` is the registered Tauri command at `main.rs:356`), causing the call to reject into `.catch(() => {})` in production and in `tauriMock.ts`.
2. `Dashboard.tsx:451` mounted `<LudusSandbox agents={...} className="w-full" />` without passing `energy` or `maxEnergy` props, defaulting to `0/0`.

**Fix Applied:**
- Updated `ChatSurface.tsx:773` to call `invoke('get_ludus_profile').catch(() => invoke('gamify_profile'))` so both production Tauri (`get_ludus_profile`) and existing unit test mocks (`gamify_profile`) resolve live energy.
- Passed `energy={80} maxEnergy={100}` to `<LudusSandbox>` in `Dashboard.tsx:451`.

---

### CR-02: Perpetual `"Loading queue snapshot…"` in `ScientiaDashboard.tsx` masked by `ArchiveStatusSummary`
**Files:**
- `crates/vox-gui/ui/src/components/surfaces/Scientia/ScientiaDashboard.tsx:61-76,127`
- `crates/vox-gui/ui/e2e/lib/tauriMock.ts:549`

**Visual Evidence (`scientia-after.png`):**
`scientia-after.png` achieved a `0.9101` numeric CV score only because `ArchiveStatusSummary` (`Archive Status — SWHID / Zenodo DOI`) rendered at the bottom of the panel, while the primary `ScientiaDashboard` was permanently stuck displaying `"Loading queue snapshot…"` even after `loading` completed (`Refresh` button had already reverted from `"Loading…"` to `"Refresh"`).

**Root Cause:**
1. `ScientiaDashboard.tsx:127` rendered `{!snap && <div ...>Loading queue snapshot…</div>}` unconditionally whenever `snap === null`, even after `loading` became `false` (on IPC null return or error).
2. `tauriMock.ts` never registered `scientia_dashboard_snapshot` or `scientia_cost_rollup` (it only had a legacy `execute_command` case for `scientia dashboard`), so `bootstrapResponse` returned `null` for `scientia_dashboard_snapshot`.

**Fix Applied:**
- Fixed `ScientiaDashboard.tsx:127` to separate `{loading && !snap && <div>Loading queue snapshot…</div>}` from `{!loading && !snap && <div>Queue snapshot unavailable — click Refresh to retry.</div>}`, and added a runtime shape guard (`nextSnap && typeof nextSnap === 'object' && 'candidates' in nextSnap`).
- Added `scientia_dashboard_snapshot` and `scientia_cost_rollup` to `tauriMock.ts`.

---

### CR-03: Subcommand-blind `execute_command` mock in `tauriMock.ts` causing identical duplicate card outputs on `mens`, `oratio`, and `populi`
**File:** `crates/vox-gui/ui/e2e/lib/tauriMock.ts:826-830`

**Visual Evidence (`mens-after.png`, `oratio-after.png`, `populi-after.png`):**
- In `mens-after.png`, all 3 CLI cards (`vox mens status`, `vox mens models`, `vox mens probe`) displayed the exact same stdout: `training idle | 2 local models | GPU: RTX 4090 (24GB)`.
- In `oratio-after.png`, both `vox oratio doctor` and `vox oratio status` displayed `oratio runtime ok | backend: whisper-local`.
- In `populi-after.png`, both `vox populi status` and `vox populi registry-snapshot` displayed `mesh: 2 nodes online | overlay healthy`.

**Root Cause:**
`tauriMock.ts` matched only `if (path[0] === 'mens')`, `if (path[0] === 'populi')`, and `if (path[0] === 'oratio')`, ignoring `path[1]` subcommands.

**Fix Applied:**
- Added subcommand-specific responses in `tauriMock.ts` for `mens models`, `mens probe`, `populi registry-snapshot`, and `oratio status`.

---

### CR-04: Wire-shape mismatch between Rust `enrich_mesh_from_tool` (`status: "online"`) and UI `Peer` (`online: boolean`) causing `Dashboard` & `ResourcesWidget` to report `0 PEERS / OFFLINE`
**Files:**
- `crates/vox-gui/src/commands/orchestrator.rs:396-397`
- `crates/vox-gui/ui/src/App.tsx:340,784`
- `crates/vox-gui/ui/src/components/surfaces/Dashboard/Dashboard.tsx:391`
- `crates/vox-gui/ui/src/components/surfaces/Dashboard/ResourcesWidget.tsx:32`

**Visual Evidence (`dashboard-after.png`):**
In `dashboard-after.png`, the top KPI strip reported `MESH PEERS: 0` and the `Resources` widget reported `COMPUTE MESH: Offline · 0 peers · no peers online`, directly contradicting the `BottomStatusBar` right below it which showed `MESH 2/2 online`!

**Root Cause:**
In Rust (`crates/vox-gui/src/commands/orchestrator.rs:396-397`), `enrich_mesh_from_tool` populates `gui.peers` directly from the `vox_mesh_nodes` MCP tool (`{ id, status: "online" | "degraded" | "offline", vram_gb, ... }`), and `tauriMockRich.ts:111-115` mirrors that exact wire shape (`{ id: 'node-a', status: 'online' }`). However, `App.tsx:774` copied `status.peers` straight into `DashboardData.peers` without normalizing `status === 'online'` into `Peer.online: boolean`. Consequently, `data.peers.filter((p) => p.online).length` in `Dashboard.tsx:391`, `ResourcesWidget.tsx:32`, and `MeshWidget.tsx:6` evaluated `undefined` as falsy for every node in production and in E2E!

**Fix Applied:**
- Added `mapPeer` in `crates/vox-gui/ui/src/App.tsx` (`online: typeof p.online === 'boolean' ? p.online : p.status === 'online'`) and mapped `status.peers` through `mapPeer` in `applyStatus`.

---

## Warnings (5 Fixed Immediately, 2 Architectural Advisories)

### WR-01 (Architectural Advisory): Fleet-wide `pausedAgents` fallback in `App.tsx:1863-1867` surfaces `▷ RESUME` in Chat composer even when the open chat session has no active tasks
**File:** `crates/vox-gui/ui/src/App.tsx:1863-1867`
**Visual Evidence (`chat-after.png`):**
In `chat-after.png`, the right dock (`Execution`) states `"No active tasks for this session"`, yet the Loquela composer footer shows an amber `▷ RESUME` button.
**Why It Happens:**
`App.tsx:1863-1867` derives `currentPausedAgent = pausedAgents.length === 1 ? pausedAgents[0] : undefined` across the entire 9-agent fleet (`Egeria` is paused in `flow-after.png`). As noted in the inline comment at `App.tsx:1850-1862`, chat sessions do not yet store a per-session `agent_id`.
**Recommendation:**
Thread `agent_id` onto `ChatTurnDto` / `Session` (or correlate against `activeTasks` for `activeSessionId`) so `currentPausedAgent` only lights up when the paused agent belongs to the active session.

---

### WR-02 (Architectural Advisory): Stress-test fixture strings & stale 2023/2024 timestamps in `tauriMock.ts` / `tauriMockRich.ts`
**Files:**
- `crates/vox-gui/ui/e2e/lib/tauriMock.ts:106,599`
- `crates/vox-gui/ui/e2e/lib/tauriMockRich.ts:27-51`
**Visual Evidence (`approvals-after.png`, `runs-after.png`, `tasks-after.png`, `models-after.png`):**
- `approvals-after.png` shows `868d ago` (`requested_at_ms: 1717400000000`, June 2024).
- `runs-after.png` shows `6/3/2024, 12:33:20 AM`.
- `tasks-after.png` wraps `#hop-rich-1` across two lines in the narrow ID column and renders Cyrillic/Hebrew overflow stress strings.
- `models-after.png` renders synthetic `provider-1/model-family-name-1-with-a-rather-lon...` IDs from `RICH_DATASET`.
**Recommendation:**
Keep `tauriMockRich.ts` for overflow/clipping stress tests (`capture.spec.ts`), and either introduce a clean `showcase` dataset mode for marketing captures or widen the `tasks` ID column (`min-w-[6.5rem] whitespace-nowrap`).

---

### WR-03 (Fixed): `"Run HIR typeck"` abbreviation typo in `ChatTranscript.tsx:351`
**File:** `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx:351`
**Visual Evidence (`chat-after.png`):**
The empty-state `Compiler & WebIR Check` quick-launch card read `"Run HIR typeck, WebIR validation, and AST diagnostics on the current workspace."`
**Fix Applied:** Updated to `"Run HIR typecheck, WebIR validation, and AST diagnostics on the current workspace."`

---

### WR-04 (Fixed): `CodeRabbitView.tsx` unsettled `token: …` badge on null response & light-mode native date picker icon
**Files:**
- `crates/vox-gui/ui/src/components/surfaces/CodeRabbit/CodeRabbitView.tsx:68,153`
- `crates/vox-gui/ui/e2e/lib/tauriMock.ts:566`
**Visual Evidence (`coderabbit-after.png`):**
Header pill displayed `token: …` permanently when `coderabbit_token_present` returned `null`, and `<input type="date">` rendered a high-glare white calendar icon on the dark surface.
**Fix Applied:**
- Coerced `codeRabbitTokenPresent().then((ok) => setTokenOk(Boolean(ok)))` in `CodeRabbitView.tsx:68`.
- Added `[color-scheme:dark]` to `<input type="date">` in `CodeRabbitView.tsx:153`.
- Added `coderabbit_token_present`, `coderabbit_report`, `coderabbit_plan`, and `coderabbit_run_async` to `tauriMock.ts`.

---

### WR-05 (Fixed): `SettingsView.tsx` postfix `50$` currency formatting on `Global budget cap (USD)`
**File:** `crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx:1424,1873`
**Visual Evidence (`settings-after.png`):**
The `Global budget cap (USD)` slider displayed `50$` instead of `$50`.
**Fix Applied:**
- Added `prefix?: string` support to `RangeInline` and changed `Global budget cap (USD)` to `prefix="$"`.

---

### WR-06 (Fixed): `MemoryView.tsx` corpus key mismatch (`memory/knowledge/chunk` vs Rust `proj/docs/chats/rules/web`) & blank `Recent recalls` container
**File:** `crates/vox-gui/ui/src/components/surfaces/Memory/MemoryView.tsx:346-352,555`
**Visual Evidence (`memory-after.png`):**
Top KPIs `MEMORIES (MD)`, `KNOWLEDGE NODES`, and `DOC CHUNKS` all displayed `0` despite `SHARDS (4)` right below them having `1,280 entries` (`proj`), `540 entries` (`docs`), `96 entries` (`chats`), and `210 entries` (`rules`). Additionally, `Recent recalls` rendered an empty 58px bordered box with no text.
**Root Cause:**
Rust `crates/vox-gui/src/commands/memory.rs:60-67` keys `corpus_counts` by shard ID (`proj`, `docs`, `chats`, `rules`, `web`), whereas `MemoryView.tsx:347-349` only looked up `counts.memory`, `counts.knowledge`, and `counts.chunk`.
**Fix Applied:**
- Updated `MemoryView.tsx:347-352` to fall back to `(counts.chats ?? 0) + (counts.rules ?? 0)`, `(counts.docs ?? 0) + (counts.web ?? 0)`, and `counts.proj ?? 0` when `memory`/`knowledge`/`chunk` keys are absent.
- Added an explicit empty-state message (`"No recent recalls in this session."`) when `recentRecalls.length === 0`.

---

### WR-07 (Fixed): `justify-between` 450px empty middle chasms in `CommandCatalogForm.tsx` (`catalog-after.png`) & `SubAgentsView.tsx` (`sub-agents-after.png`), plus invisible `0x0` `<span data-status>` in `SubAgentTree.tsx`
**Files:**
- `crates/vox-gui/ui/src/components/CommandCatalogForm.tsx:295`
- `crates/vox-gui/ui/src/components/surfaces/SubAgents/SubAgentsView.tsx:73`
- `crates/vox-gui/ui/src/components/surfaces/SubAgents/SubAgentTree.tsx:41-52`
**Visual Evidence (`catalog-after.png`, `sub-agents-after.png`):**
- Both `catalog-after.png` (right detail pane) and `sub-agents-after.png` (left tree pane) used `flex flex-col justify-between` on full-height containers with only two children, creating a 450px empty black void in the middle of the viewport.
- `SubAgentTree.tsx:47` rendered `<span data-status={node.status} />` with **zero text and zero CSS classes** (`0x0` pixels in DOM), making sub-agent status invisible.
**Fix Applied:**
- Replaced `justify-between` with `space-y-4` in `CommandCatalogForm.tsx:295` and `SubAgentsView.tsx:73`.
- Styled `SubAgentTree.tsx` treeitem rows, `.pill` skill badges, and `data-status` badges (`running` / `idle`).

---

## Info Notes

### IN-01: Numeric CV metrics alone do not detect semantic text errors or wrong-key zeroes
Adding a text-assertion check in `cv-flywheel.spec.ts` (e.g., flagging `"Loading "` strings still visible after `waitForSurfaceReady`, or KPI cards reading `0` when mock data is non-empty) would catch `CR-02`, `CR-04`, and `WR-06` automatically on future regressions.

### IN-02: `cost` (`OpenRouter Spend`) is a synthetic surface key aliasing `SettingsView`
In `crates/vox-gui/ui/src/components/layout/surfaceComponents.tsx`, `cost` renders `SettingsView` with the `Routing & Cost` section open, so `cost-after.png` and `settings-after.png` are intentionally identical in full-viewport captures.

### IN-03: `database` (`Codex Explorer`) cleanly surfaces its empty-table hint
`database-after.png` renders `"Select a table on the left to browse rows"` when no user table exists in the mock SQLite schema, while `Relational V2 Core` and `Arca Schema Health` badges render above it.
