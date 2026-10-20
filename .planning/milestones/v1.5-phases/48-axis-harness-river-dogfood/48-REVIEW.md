---
phase: 48-axis-harness-river-dogfood
reviewed_at: "2026-10-14T01:11:00Z"
depth: standard
status: resolved
findings:
  critical: 0
  warning: 0
  info: 0
resolved_findings:
  warning: 2
  info: 2
---

# Phase 48 Code Review: Axis Harness River & Live Developer Dogfooding (`RIVER-01..04`)

## Scope Reviewed

- `crates/vox-gui/ui/src/components/surfaces/Harness/riverTypes.ts`
- `crates/vox-gui/ui/src/components/surfaces/Harness/riverMath.ts`
- `crates/vox-gui/ui/src/components/surfaces/Harness/riverMath.test.ts`
- `crates/vox-gui/ui/src/components/surfaces/Harness/riverCanvasRenderer.ts`
- `crates/vox-gui/ui/src/components/surfaces/Harness/riverCanvasRenderer.test.ts`
- `crates/vox-gui/ui/src/components/surfaces/Harness/WaveformTierStrip.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Harness/AgentTrajectoryBraid.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Harness/VerificationFunnelFooter.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Harness/HarnessRiverPanel.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Harness/HarnessRedirect.tsx`
- `crates/vox-gui/ui/src/lib/driveConsole.ts`
- `crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Gamify/GamifyView.tsx`
- `crates/vox-gui/ui/src/components/layout/Sidebar.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Matrix/Matrix.tsx`
- `crates/vox-gui/ui/src/components/surfaces/NeedsYou/NeedsYouSurface.tsx`
- `crates/vox-gui/ui/e2e/lib/tauriMock.ts`
- `crates/vox-gui/ui/e2e/axis-developer-dogfood.spec.ts`

---

## Summary

| Severity | Count | Status |
|----------|-------|--------|
| **Critical** | 0 | None |
| **Warning** | 2 | Actionable (`W-01`, `W-02`) |
| **Info** | 2 | Minor polish (`I-01`, `I-02`) |

---

## Findings

### Warnings

#### `W-01`: `deriveRiverSnapshot` double-counts in-progress tasks and counts completed tasks in `activeAgents`
- **File:** [`crates/vox-gui/ui/src/components/surfaces/Harness/riverMath.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Harness/riverMath.ts#L107-L110)
- **Lines:** `107–110`, `205–206`
- **Problem:**
  ```ts
  const inProgressTasks = tasks.filter(
    (t) => t.status === 'in_progress' || t.status === 'running' || t.status === 'queued',
  ).length;
  const activeAgents = Math.max(1, inProgressTasks + streamingCount + Math.min(8, tasks.length));
  ```
  Because `inProgressTasks` is a subset of `tasks`, any active task is counted twice (`inProgressTasks` + `tasks.length`), and completed/failed tasks in `tasks` still inflate `activeAgents` even when 0 tasks are running and 0 messages are streaming. Additionally, line 206 (`task.title.slice(0, 18)`) assumes `task.title` is always a non-null string.
- **Recommendation:** Compute `activeAgents = Math.min(15, Math.max(1, inProgressTasks + streamingCount))` so completed tasks do not inflate active agent counts and active tasks are counted once, and guard `task.title` with `(task?.title || task?.id || \`agent-0\${idx + 1}\`).slice(0, 18)`.

#### `W-02`: 60fps `getBoundingClientRect()` layout read & missing `prefers-reduced-motion` / `document.hidden` guard in `HarnessRiverPanel`
- **File:** [`crates/vox-gui/ui/src/components/surfaces/Harness/HarnessRiverPanel.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Harness/HarnessRiverPanel.tsx#L91-L134)
- **Lines:** `91–134`
- **Problem:**
  1. Inside the `requestAnimationFrame` callback `draw(nowMs)`, `canvas.getBoundingClientRect()` is invoked on every animation frame (60fps), forcing a synchronous layout measurement per frame.
  2. The 60fps `requestAnimationFrame` loop runs continuously even when the user has `prefers-reduced-motion: reduce` enabled or when the window/tab is hidden (`document.hidden === true`).
- **Recommendation:** Cache canvas dimensions via `ResizeObserver` (falling back to `canvas.clientWidth` / `canvas.clientHeight`), render only a single static frame when `window.matchMedia?.('(prefers-reduced-motion: reduce)').matches` is true, and skip drawing frames when `document.hidden` is true.

---

### Info

#### `I-01`: `WaveformTierStrip` tier cards render interactive hover affordances when neither `onOpenRouting` nor `onFocusComposer` is wired
- **File:** [`crates/vox-gui/ui/src/components/surfaces/Harness/WaveformTierStrip.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Harness/WaveformTierStrip.tsx#L60-L85), [`crates/vox-gui/ui/src/components/surfaces/Harness/HarnessRiverPanel.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Harness/HarnessRiverPanel.tsx#L136-L142)
- **Lines:** `WaveformTierStrip.tsx:61–85`, `HarnessRiverPanel.tsx:136–142`
- **Problem:** When `HarnessRiverPanel` is mounted inside `GamifyView` (`<HarnessRiverPanel gamifyEnabled />`), neither `onOpenRouting` nor `onFocusComposer` is passed, but `handleSelectTier` is still passed to `WaveformTierStrip`, giving the tier `<button>` cards hover border highlights even though clicking them is a no-op.
- **Recommendation:** Only pass `onSelectTier` when `Boolean(onOpenRouting || onFocusComposer)` is true, and apply `cursor-default` when `onSelectTier` is undefined.

#### `I-02`: Cold-start `latestModelOverride` in `driveConsole.ts` does not seed from `SHELL_PREFERENCE_KEYS.chatModelOverride`
- **File:** [`crates/vox-gui/ui/src/lib/driveConsole.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/lib/driveConsole.ts#L53-L65)
- **Lines:** `53–65`
- **Problem:** `latestModelOverride` starts as `null` in module state; while `HarnessRiverPanel` falls back to `readStoredModelOverride()`, initializing `latestModelOverride` from `SHELL_PREFERENCE_KEYS.chatModelOverride` in `driveConsole.ts` (when available) keeps `getLatestModelOverride()` consistent before `Loquela` mounts.
- **Recommendation:** Seed `latestModelOverride` from `localStorage` (`SHELL_PREFERENCE_KEYS.chatModelOverride`) in `driveConsole.ts` with a safe `try/catch` helper.
