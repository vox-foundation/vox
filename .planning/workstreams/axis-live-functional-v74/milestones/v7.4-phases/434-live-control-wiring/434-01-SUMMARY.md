---
phase: 434
plan: 01
title: "End-to-End Live Control, Rotary Knob, Brass Gauge & Interactive Telemetry Wiring"
status: complete
completed_at: "2026-11-14"
requirements_completed:
  - AXIS-LIVE-CONTROLS-01
  - AXIS-LIVE-CONTROLS-02
  - AXIS-LIVE-CONTROLS-03
  - AXIS-LIVE-CONTROLS-04
---

# Summary 434-01: End-to-End Live Control, Rotary Knob, Brass Gauge & Interactive Telemetry Wiring

## Delivered Capabilities

### 1. Prerequisite Primitive Upgrades (`Step 0`)
- **`crates/vox-gui/ui/src/components/ui/Select.tsx`**: Added `<VoxSelect>` wrapper over Radix `Select` with transparent empty-string (`"" <-> "__vox_empty__"`) sentinel translation, `size="sm" | "xs"` variants, `data-slot="select-trigger"`, and a test-compatible `onChange` bridge.
- **`crates/vox-gui/ui/src/components/ui/RomanTooltip.tsx` & `RomanSurfaceHeader.tsx`**: Added `active?: boolean` and `actionHint?: string` to `RomanBrassGauge` and `RomanSurfaceGaugeSpec`, rendering focusable `aria-pressed` interactive dial buttons with tooltip action guidance and explicit CSS property transitions (zero `transition-all`).

### 2. Valid Navigation, Typed Checkpoints & 11 Live Controls (`Slice 434-A` — `AXIS-LIVE-CONTROLS-01`, `AXIS-LIVE-CONTROLS-02`)
- **Valid `SURFACE_REGISTRY` Navigation**: Replaced dead `viewKey`s (`'vox-cost-analytics'`, `'vox-Ludus-view'`, `'vox-operations-hub'`, `'operations'`) across `HudPanels.tsx`, `LudusSandbox.tsx`, and `HarnessRiverPanel.tsx` with canonical `SURFACE_REGISTRY` routes (`'mercatus'`, `'tasks'`, `'gamify'`, `'mens'`, `'models'`, `'approvals'`, `'harness-health'`).
- **Typed VCS Checkpoints**: Removed `(...args: unknown[])` casts on `vcsCheckpointCapture` and `vcsCheckpointRestore` in `ImperiumInspector.tsx`, invoking strongly typed RPCs with `sanitizeErrorForToast(err)` error reporting.
- **11 Wired Controls with Optimistic Snapshot-Rollback**:
  1. `HarnessRiverPanel.tsx` time-window `<VoxSelect>` (`15m` / `1h` / `24h` / `session` -> `vox.harness.time_window.v1`).
  2. `HarnessRiverPanel.tsx` `Compact Context` -> `compactChatUpTo({ sessionId: 'default', direction: 'up_to' })`.
  3. `HarnessRiverPanel.tsx` `Claim Yield` -> `recordGuiEvent('aqueduct_yield_claimed')` + `setGuiPreference('vox.harness.claimed_yield_xp.v1')`.
  4. `HarnessRiverPanel.tsx` `Verification Gate` -> `setGuiPreference('vox.harness.interceptor_armed.v1')`.
  5. `HarnessRiverPanel.tsx` `Sluice Gates I–IV` & `Flush Sluice Gate` -> `setGuiPreference('vox.harness.sluice_gates.v1')`.
  6. `ImperiumInspector.tsx` `PRAETORIVM Approve` -> `invokeMcpTool('vox_resolve_approval', { approval_id, outcome: 'approved' })` + `onRefreshHarness()`.
  7. `ImperiumInspector.tsx` `FORVM` District Model Routing Tier -> `<RomanRotaryKnob>` persisting `vox.routing.exploration_pct.v1`.
  8. `LudusSandbox.tsx` `Pin Context` -> `getSessionContextPins` / `setSessionContextPins`.
  9. `LudusSandbox.tsx` `Yield` -> `recordGuiEvent('urbs_yield_claimed')` + `setGuiPreference('vox.urbs.claimed_yield_xp.v1')`.
  10. `SessionInteropModal.tsx` live `guestShareList` + per-submission `guestShareModerate` (`Approve` / `Reject`, zero `'gsub_demo_1'`) + `.jsonl`/`.json` file upload picker (`sessionImportBundle`).
  11. `Mercatus.tsx` per-source `<Switch>` toggles and inline `target_usd` threshold inputs persisted via `mercatusSaveConfig` with full rollback on error.

### 3. 100% Native `<select>` & `<input type="range">` Purge + Guard (`Slice 434-B` — `AXIS-LIVE-CONTROLS-04`)
- Replaced all 37 native `<select>` elements across 25 source files (`CommandCatalogForm`, `ActivitySurface`, `BrowserView`, `Console`, `InputEditor`, `SendToAgent`, `ExplorerRail`, `Loquela`, `IntentPanel`, `MemoryView`, `MeshView`, `ModelsView`, `IsolationPanel`, `LiveSourceProber`, `ResearchView`, `SandboxReplModal`, `HarnessIssuesPanel`, `PriorityChainEditor`, `SettingsView`, `TaskPolicySection`, `TasksView`, `CodeAtlasCanvas`, `HarnessRiverPanel`, `LudusSandbox`) with `<VoxSelect>`.
- Replaced all 4 native `<input type="range">` controls (`Loquela.tsx`, `PriorityChainEditor.tsx` x2, `ImperiumInspector.tsx`) with accessible `<Slider>` and `<RomanRotaryKnob>` controls.
- Added `crates/vox-gui/ui/src/guards/noRawNativeControlsGuard.test.ts` enforcing zero `<select>` and zero `type="range"` JSX tags across `crates/vox-gui/ui/src`.

### 4. Empirical `<RomanBrassGauge>` Dials, Click-to-Filter & `ContextWindowMeter` (`Slices 434-B & 434-C` — `AXIS-LIVE-CONTROLS-03`)
- Hydrated all ~65 `<RomanBrassGauge>` dials across 26 surfaces (`ApprovalsView`, `BrowserView`, `Catalog`, `CodeRabbitView`, `CommandCardsView`, `Console`, `DiscoverySurface`, `ExplorerView`, `HarnessHealthView`, `HudPanels`, `MemoryView`, `MensCorpusStudioPanel`, `Mercatus`, `MeshView`, `ModelsView`, `NeedsYouSurface`, `PoliciesView`, `PublicationsView`, `RepositoryView`, `ResearchView`, `RunsView`, `ScientiaDashboard`, `SettingsView`, `SkillsPluginsView`, `SubAgentsView`, `TasksView`, `WaveformTierStrip`) from empirical DTO counts (`0` when empty; zero `0.78` / `0.85` / `99.8%` constants) and wired `onClick` + `active` + `actionHint` to filter or focus each surface's active lane, table, or inspector.
- Upgraded `ContextWindowMeter.tsx` to compute empirical `kvCacheHitPct` (`0% cached` on zero usage) and proportional BPE token slices (`Sys`, `KB`, `AST`, `Hist`) with sibling `<button>` slice filter pills (`axe nested-interactive === 0`) and Limes 11px type-scale compliance (`chatTypeScale.test.ts`).

### 5. Interactive Recharts Scrubbing/Drill-Down & Layout Persistence (`Slice 434-D` — `AXIS-LIVE-CONTROLS-04`)
- Upgraded `chartWidgetShared.tsx`, `LineChartWidget.tsx`, `AreaChartWidget.tsx`, and `BarChartWidget.tsx` with full-bleed `ResponsiveContainer width="100%" height="100%" minHeight={156}`, formatted `XAxis`/`YAxis`, optional `<ReferenceLine>` budget/threshold caps, automatic `<Brush>` timeline scrubber (`series.length >= 6`), and datum click selection (`Selected:` filter pill + `Clear`).
- Wired `DashboardGrid.tsx` (`vox.dashboard.layout.v2` + `Dashboard.tsx`) and `DockWorkspaceShell.tsx` (`vox.dockview.layout.v3`) to hydrate from `voxTransport.getGuiPreference` on mount and persist debounced layout mutations via `voxTransport.setGuiPreference`.
