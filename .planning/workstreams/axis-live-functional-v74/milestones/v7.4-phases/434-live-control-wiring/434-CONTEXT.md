# Phase 434 Context — End-to-End Live Control, Rotary Knob, Brass Gauge & Interactive Telemetry Wiring

- **Milestone**: `v7.4` (`axis-live-functional-v74`)
- **Requirements**: `AXIS-LIVE-CONTROLS-01`, `AXIS-LIVE-CONTROLS-02`, `AXIS-LIVE-CONTROLS-03`, `AXIS-LIVE-CONTROLS-04`
- **Status**: Ready for Execution (`Smart Discuss` 3/3 decisions accepted)

## Locked Decisions

### Area 1 — Valid `SURFACE_REGISTRY` Navigation, Typed Checkpoints & 11 Live Controls (`AXIS-LIVE-CONTROLS-01`, `AXIS-LIVE-CONTROLS-02`)
1. **Valid `viewKey` Navigation**:
   - `HudPanels.tsx`: map Budget gauge click (`vox-cost-analytics` -> `'mercatus'`) and Energy gauge click (`vox-operations-hub` -> `'tasks'`), and make Speed dial click cycle `0x -> 1x -> 3x` via `onSetSpeed`.
   - `HarnessRiverPanel.tsx`: map `handleJump('operations')` -> `'tasks'` (valid `SURFACE_REGISTRY` viewKey) and verify all quick-jump pills target real `SURFACE_REGISTRY` `viewKey`s (`'tasks'`, `'mens'`, `'models'`, `'approvals'`, `'harness-health'`).
   - `ImperiumInspector.tsx` & `LudusSandbox.tsx`: verify every `targetView` / `surface` string in monument callouts and inspector actions resolves to a live `SURFACE_REGISTRY` `viewKey`.
2. **Typed `vcsCheckpointCapture` / `vcsCheckpointRestore`**:
   - Remove all `(...args: unknown[])` casts in `ImperiumInspector.tsx` (`:851`, `:898`) and call `vcsCheckpointCapture('default', checkpoints.length + 1, \`porta-${headBranch}\`)` and `vcsCheckpointRestore('default', restoreTurnNum)` with strict TypeScript signatures and `sanitizeErrorForToast` error handling.
3. **11 Live Controls with Optimistic Snapshot-Rollback**:
   - `HarnessRiverPanel.tsx`: wire time-window filter (`15m` / `1h` / `24h` / `session` filtering live river events + persisting via `setGuiPreference`), `Compact Context` (`compactChatUpTo`), `Claim Yield` (`gamify_award_xp` / `invokeCmd` or `setGuiPreference` persisted ledger with rollback), `Verification Gate` (`setGuiPreference` + `vox_check` / gate toggle), and `Sluice Gates I–IV` (`setGuiPreference('vox.harness.sluice_gates.v1', ...)` with rollback on error).
   - `ImperiumInspector.tsx`: wire `PRAETORIVM Approve` to real `voxTransport.resolveApproval` / `invokeMcpTool('vox_resolve_approval', ...)` and replace the `FORVM` routing `<input type="range">` with `<RomanRotaryKnob>` committing real routing weight via `voxTransport.setRoutingExplorationWeight` / `setGuiPreference` (`onValueCommit`).
   - `LudusSandbox.tsx`: wire `Pin Context` to real `invokeCmd('set_session_context_pins', ...)` and `Yield` to real gamify/preference persistence with rollback on failure.
   - `SessionInteropModal.tsx`: replace hardcoded `'gsub_demo_1'` with live `guestShareList(sessionId)` loading invites & pending submissions, per-submission Approve/Reject buttons calling `guestShareModerate({ submissionId, approve })`, and a local `.jsonl`/`.json` file picker (`<input type="file" accept=".jsonl,.json">`) populating `rawJsonl` / importing bundles.
   - `Mercatus.tsx`: wire per-source enable/disable `<Switch>` controls and editable target USD thresholds to `voxTransport.mercatusSaveConfig(nextCfg)` with snapshot-rollback + `sanitizeErrorForToast` banner on save failure.

### Area 2 — Empirical `<RomanBrassGauge>` Dials & `ContextWindowMeter` (`AXIS-LIVE-CONTROLS-03`)
1. **Empirical Telemetry**:
   - Eliminate hardcoded magic gauge ratios (`0.85`, `0.78`, `92%`) across all `RomanSurfaceHeader` `gauges` arrays, `WaveformTierStrip.tsx`, `MensCorpusStudioPanel.tsx`, `ChatSurface.tsx`, and `ContextWindowMeter.tsx` (accept optional empirical `sliceBreakdown` and `cacheHitPct` props on `ContextWindowMeter`, deriving cache/slice ratios deterministically from real token/reclaimed metrics when backend slice breakdown is absent rather than fake strategy constants).
2. **Interactive Click-to-Filter / Focus**:
   - Add `active?: boolean` and `actionHint?: string` to `RomanBrassGauge` & `RomanSurfaceGaugeSpec` (`aria-pressed={active}` when interactive) and wire header gauge `onClick` handlers across surfaces so clicking a header dial filters or focuses the corresponding status tab, search/table filter, or active section.

### Area 3 — Native Control Purge, Interactive Recharts & Layout Persistence (`AXIS-LIVE-CONTROLS-04`)
1. **100% Native `<select>` (34) & `<input type="range">` (4) Replacement**:
   - Replace every `<select>` across `src/` with `<VoxSelect>` / Radix `<Select>` (`data-slot="select-trigger"`, empty-string sentinel safe) and every `<input type="range">` (`Loquela.tsx`, `PriorityChainEditor.tsx` ×2, `ImperiumInspector.tsx`) with `<RomanRotaryKnob>` or Radix `<Slider>` (`onValueCommit` debounced).
   - Enforce permanently via `src/guards/noRawNativeControlsGuard.test.ts`.
2. **Interactive Full-Bleed Recharts**:
   - Upgrade `chartWidgetShared.tsx`, `LineChartWidget.tsx`, `AreaChartWidget.tsx`, and `BarChartWidget.tsx` with `ResponsiveContainer width="100%" height="100%"` inside `flex-1 min-h-[160px]`, formatted `XAxis`/`YAxis`, optional `<ReferenceLine>` threshold cap, `<Brush>` timeline scrubber (`series.length >= 6`), and datum click drill-down readout.
3. **Backend-Persisted `DashboardGrid` & `DockWorkspaceShell`**:
   - Persist and hydrate `DashboardGrid` (`Dashboard.tsx` / `DashboardGrid.tsx`) and `DockWorkspaceShell.tsx` layouts through `voxTransport.getGuiPreference` / `voxTransport.setGuiPreference` (with synchronous `localStorage` write-through cache and schema validation).
