# GUI Observability: Notices, Severity and the Notification Center — Implementation Plan

> **For agentic workers:** Execution is **Claude Code driving Gemini Flash via the `agy` CLI**, one task per headless
> run (`/drive-task docs/superpowers/plans/2026-09-29-gui-observability-notices.md <N>`), per
> [`docs/src/contributors/antigravity-driven-execution.md`](../../src/contributors/antigravity-driven-execution.md).
> The agent never stages or commits; Claude Code re-runs every check, runs the mutation proofs, reviews the diff and
> commits. Task 6 is re-anchored by Claude before it is driven; Tasks 12 and 13 are Claude's. Code blocks are
> transcribed exactly. Amended 2026-09-29 after a three-track review (correctness, tests, simplicity); the
> amendments are folded into the tasks below.

**Goal:** Make [`gui-observability-ssot-2026.md`](../../src/architecture/gui-observability-ssot-2026.md) true in the
code:
- Every message the GUI shows is a categorised **notice**.
- Engine events carry the engine's own **severity**.
- Nothing a toast says is lost after 5 seconds, because every toast is also recorded in a **notification center**.
- A failed data source shows as **degraded**, never as healthy.
- The Activity view refreshes when an activity row is written, not on every token.
- The chat status line shows the real phase and cost.

All of this is wired end to end (Rust → Tauri bridge → hooks → UI → Playwright). The chat text stays conversation only.

**Architecture.**
- **Rust (orchestrator).** `AgentEventKind::severity()` is an exhaustive match in a new `events_severity.rs`. There is
  no wildcard arm, so a new event kind cannot compile without a classification.
- **Rust (GUI bridge).** `spawn_agent_event_stream` passes each frame through `annotate_agent_event`. That call adds
  `severity` and reports whether the kind is activity-loggable; for loggable kinds the bridge emits
  `vox://activity-appended`, which today has a listener and no emitter.
- **TypeScript.** `lib/notices.ts` defines the notice model and the routing rule.
  - `hooks/useNoticeCenter.ts` holds a bounded, coalescing store.
  - `App.tsx`'s `pushToast` records every toast, and its agent-event listener records engine warnings and errors.
  - `NotificationCenter.tsx` is one self-contained bell + drawer that renders the store.
  - `useAttentionInbox` reports which sources failed, and the Sidebar shows that instead of a zero.

**Tech stack:** Rust (`vox-orchestrator`, `vox-gui`), TypeScript/React 19 + Tailwind 4, vitest 3 (jsdom), Playwright 1.62.

**Evidence** (survey 2026-09-29 at `f50e9fa26`; see the SSOT's "Current state"):
- Toast tones are `ok|warn|info` (`types/tauri.ts#Toast`). Toasts expire after 5 s (`App.tsx#pushToast`) and are
  kept nowhere else.
- `AgentEvent`/`ActivityRow` carry no severity (`events.rs#AgentEvent`, `activity/project.rs`).
- `vox://activity-appended` has no emitter. The GUI's `ActivitySurface` therefore re-queries on every
  `vox://agent-events` frame, token frames included (`ActivitySurface.tsx`, "Option B" comment).
- `mapAgentEvent` metadata has no `phase` or `costUsd`. `buildChatOnlyTimeline` reads both, so the status line always
  says "Working". Cost events carry no `task_id`, so the done-summary row never renders.
  - The timeline's unit tests pass only because they hand-build that metadata (`chatTranscriptTimeline.test.ts`
    helper `evt(…, { phase, costUsd, taskId })`).
- `useAttentionInbox.refresh` turns every failure into empty data. The Review badge then shows nothing, as if all
  were clear.
- An untyped `pushToast` appears in 12 files (13 sites: `(t: any)` in 11 files, `SettingsView` twice, and
  `TasksView`'s `(t: unknown)`), bypassing the required `cause`.

**Prerequisites:** these plans committed first, because they rewrite files this plan edits (`App.tsx`,
`BottomStatusBar.tsx`, `ChatTranscript.tsx`, `ModelsView.tsx`):
- `2026-09-28-chat-turn-trace.md`
- `2026-09-28-chat-surfaces-consolidation.md`
- `2026-09-28-chat-visual-language.md`
- the GUI tasks (7–11) of `2026-09-29-model-routing-self-maintaining.md`

Only Tasks 1 and 3 touch none of those plans' files and may be driven earlier. Execution Order says exactly when
every other task runs.

## Global Constraints

- No new crate, crate edge or dependency (no `tauri-plugin-notification`, no query library).
- No versioned cloud model id literal anywhere; fixtures use `acme/*`.
- Test-first: write the tests, run them, and save the failing output to `target/obs-t<N>-red.txt` before
  implementing. Claude runs the mutation proofs.
- Foreground, `timeout`-prefixed commands:
  - cargo: `timeout 1500s`, scoped with `-p`;
  - pnpm/vitest/playwright: `timeout 300s`;
  - format Rust with `rustfmt --edition 2024 <file>`, never `cargo fmt`.
- UI commands:
  - `pnpm --dir crates/vox-gui/ui exec vitest run <files>`
  - `pnpm --dir crates/vox-gui/ui typecheck`
  - `pnpm --dir crates/vox-gui/ui exec playwright test <spec> --project=chromium --reporter=line`
- tdd-guard scans whole files: every touched `.rs` file with a `pub fn` needs an in-file default-feature
  `#[cfg(test)]` test.
- Existing tests are not edited except where a task names the edit. Any other existing test that breaks is a STOP.
- Pre-flight for every task: `git status --short` must show no uncommitted change in a file the task edits or
  compiles. Another session's work in the shared tree is a STOP, not something to build on.
- Test files are not type-checked (`ui/tsconfig.json` excludes `*.test.ts(x)`); do not look for type errors in them.
- Files over 500 lines (`App.tsx` ~2100, `events.rs` ~1400, `vox-gui/src/commands/orchestrator.rs`) get only the
  edits shown; new code goes in new files.
- Colours only through `var(--color-*)` tokens: no raw hex, no new raw Tailwind palette classes.
- The agent never runs `git add` or `git commit`.

## File Structure

| File | Status | Task | Responsibility |
|---|---|---|---|
| `crates/vox-orchestrator/src/events_severity.rs` | create | 1 | `EventSeverity`, `AgentEventKind::severity` |
| `crates/vox-orchestrator/src/lib.rs` | modify | 1 | `pub mod events_severity;` |
| `crates/vox-gui/src/commands/event_annotate.rs` | create | 2 | `annotate_agent_event`, `ACTIVITY_APPENDED_EVENT` |
| `crates/vox-gui/src/commands/mod.rs`, `orchestrator.rs` | modify | 2 | module; the bridge calls the annotator and emits `activity-appended` |
| `crates/vox-gui/ui/src/lib/notices.ts` + `.test.ts` | create | 3 | notice model, `noticeFromToast`, `noticeFromAgentEvent` |
| `crates/vox-gui/ui/src/lib/noticeStore.ts` + `.test.ts` | create | 4 | pure reducer (coalesce, cap, unread) |
| `crates/vox-gui/ui/src/hooks/useNoticeCenter.ts` | create | 4 | hook over the reducer |
| `crates/vox-gui/ui/src/types/tauri.ts`, `components/ui/Toasts.tsx` + test | modify | 4 | `error` tone; `data-testid="toast-stack"` |
| `crates/vox-gui/ui/src/App.tsx` | modify | 4 | `pushToast` and the agent-event listener record notices |
| `crates/vox-gui/ui/src/components/ui/Icons.tsx` | modify | 5 | `bell` icon |
| `crates/vox-gui/ui/src/components/common/NotificationCenter.tsx` + `.test.tsx` | create | 5 | bell + drawer, one component that owns its open state and focus |
| status bar (surfaces plan's cards file) | modify | 6 | mount the bell (re-anchored) |
| `crates/vox-gui/ui/src/hooks/useAttentionInbox.ts` + test | modify | 7 | `degraded` sources |
| `crates/vox-gui/ui/src/components/layout/Sidebar.tsx` + test, `AppShell.tsx`, `App.tsx` | modify | 7 | degraded Review badge (App → AppShell → Sidebar) |
| `crates/vox-gui/ui/src/components/surfaces/Activity/ActivitySurface.tsx`, `ActivitySurface.container.test.tsx`, `config/constants.ts` | modify | 8 | refresh on `activity-appended` only, debounced |
| `crates/vox-gui/ui/src/lib/mapAgentEvent.ts`, `chatTranscriptTimeline.ts` + tests | modify | 9 | real phase and cost |
| — | — | 10 | moved out of this plan (see Deferred) |
| 12 files with an untyped `pushToast` + `lib/typedToasts.test.ts` | modify/create | 11 | typed toasts |
| `crates/vox-gui/ui/e2e/notices.spec.ts`, `e2e/lib/tauriMockShared.ts` | create/modify | 12 | Playwright + screenshots (Claude) |

---

### Task 1: The engine classifies every event's severity

**Files:** Create `crates/vox-orchestrator/src/events_severity.rs`; modify `crates/vox-orchestrator/src/lib.rs` (add
`pub mod events_severity;` directly after `pub mod events;`).

- [ ] **Step 1: Write the failing tests** — add `pub mod events_severity;` to `lib.rs` now (so the red run compiles
  this file), and create `events_severity.rs` with only:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::BudgetSignal;
    use crate::events::AgentEventKind;
    use crate::types::{AgentId, TaskId};

    fn budget(signal: BudgetSignal) -> AgentEventKind {
        AgentEventKind::BudgetAlert { agent_id: AgentId(1), signal }
    }

    #[test]
    fn failures_and_safety_events_are_errors() {
        assert_eq!(AgentEventKind::InjectionDetected { detail: "x".into() }.severity(), EventSeverity::Error);
        assert_eq!(budget(BudgetSignal::CostExceeded { cost_usd: 2.0, limit_usd: 1.0 }).severity(), EventSeverity::Error);
        let failed = AgentEventKind::TaskFailed {
            task_id: TaskId(7),
            agent_id: AgentId(1),
            error: "boom".into(),
            session_id: None,
            audit_report: None,
        };
        assert_eq!(failed.severity(), EventSeverity::Error);
    }

    #[test]
    fn budget_severity_follows_the_signal() {
        assert_eq!(budget(BudgetSignal::Normal { usage_ratio: 0.1 }).severity(), EventSeverity::Info);
        assert_eq!(
            budget(BudgetSignal::HighLoad { usage_ratio: 0.8, tokens_remaining: 100 }).severity(),
            EventSeverity::Warning
        );
        assert_eq!(
            budget(BudgetSignal::Critical { usage_ratio: 0.99, tokens_remaining: 1 }).severity(),
            EventSeverity::Error
        );
    }

    #[test]
    fn a_flagged_grounding_check_is_a_warning() {
        let check = |flagged| AgentEventKind::GroundingCheckCompleted {
            agent_id: AgentId(1),
            task_id: TaskId(1),
            confidence: 0.4,
            flagged,
        };
        assert_eq!(check(true).severity(), EventSeverity::Warning);
        assert_eq!(check(false).severity(), EventSeverity::Info);
    }

    #[test]
    fn routine_events_are_info() {
        assert_eq!(AgentEventKind::AttentionConfigReloaded.severity(), EventSeverity::Info);
        assert_eq!(
            AgentEventKind::LockWaiting { resource_id: "db://x".into(), task_id: TaskId(1), session_id: None }.severity(),
            EventSeverity::Info
        );
    }

    #[test]
    fn severity_serialises_lowercase_for_the_gui() {
        assert_eq!(serde_json::to_value(EventSeverity::Warning).unwrap(), serde_json::json!("warning"));
        assert_eq!(serde_json::to_value(EventSeverity::Debug).unwrap(), serde_json::json!("debug"));
    }
}
```

If `AgentId`/`TaskId` are imported from a different path in `events.rs`, use that path; if a variant's fields differ
from these literals (`rg -n "LockWaiting \{" -A5 crates/vox-orchestrator/src/events.rs`), adapt only the literal and
say so.

- [ ] **Step 2: Run to verify failure** —
  `timeout 1500s cargo test -p vox-orchestrator --lib events_severity > target/obs-t1-red.txt 2>&1; tail -15 target/obs-t1-red.txt`.
  Expected: fails to compile with `error[E0412]`/`error[E0599]` (`EventSeverity`, `severity` missing). A run that
  compiles and reports `0 passed` means the `lib.rs` line is missing: add it and re-run.
- [ ] **Step 3: Implement** (above the tests):

```rust
//! One severity per engine event, decided where the event is defined, so no surface classifies events by
//! string matching. The match below has no wildcard arm on purpose: a new `AgentEventKind` variant does
//! not compile until someone decides how loud it is.

use serde::{Deserialize, Serialize};

use crate::budget::BudgetSignal;
use crate::events::AgentEventKind;

/// How much an engine event matters to a person. `Debug` is kept out of every surface except the
/// activity log and the verbose trace (docs/src/architecture/gui-observability-ssot-2026.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventSeverity {
    Debug,
    Info,
    Warning,
    Error,
}

impl AgentEventKind {
    /// The event's severity.
    #[must_use]
    pub fn severity(&self) -> EventSeverity {
        use AgentEventKind as K;
        use EventSeverity::{Debug, Error, Info, Warning};
        match self {
            K::TaskFailed { .. }
            | K::WorkflowFailed { .. }
            | K::EmergencyStop { .. }
            | K::InjectionDetected { .. }
            | K::ScopeViolation { .. }
            | K::TaskExpired { .. } => Error,

            K::ToolTimedOut { .. }
            | K::TaskDoubted { .. }
            | K::DoubtReported { .. }
            | K::ConflictDetected { .. }
            | K::PromptConflictDetected { .. }
            | K::AgentHandoffRejected { .. }
            | K::UrgentRebalanceTriggered { .. }
            | K::ReplanTriggered { .. }
            | K::ActivityRetried { .. }
            | K::AutoHealSuggested { .. }
            | K::AttentionBudgetAlert { .. }
            | K::TrustOverride { .. }
            | K::ContextTruncated { .. }
            | K::SemanticDriftDetected { .. } => Warning,

            K::GroundingCheckCompleted { flagged, .. } => {
                if *flagged { Warning } else { Info }
            }
            K::BudgetAlert { signal, .. } => match signal {
                BudgetSignal::Normal { .. } | BudgetSignal::ToolLatencyUnknown { .. } => Info,
                BudgetSignal::HighLoad { .. }
                | BudgetSignal::AttentionHigh { .. }
                | BudgetSignal::ToolLatencyHigh { .. }
                | BudgetSignal::DoomLoopSuspect { .. } => Warning,
                BudgetSignal::Critical { .. }
                | BudgetSignal::CostExceeded { .. }
                | BudgetSignal::AttentionCritical { .. }
                | BudgetSignal::HaltAgent { .. } => Error,
            },

            K::TokenStreamed { .. }
            | K::AgentHeartbeat { .. }
            | K::ActivityChanged { .. }
            | K::ToolCallDispatched { .. }
            | K::MessageSent { .. }
            | K::CostIncurred { .. }
            | K::AgentIdle { .. }
            | K::AgentBusy { .. }
            | K::LlmCallCompleted { .. }
            | K::ObservationRecorded { .. }
            | K::ThroughputTick { .. }
            | K::CostTick { .. }
            | K::FileDiagChanged { .. }
            | K::BuildStage { .. }
            | K::MensObserverObservation { .. }
            | K::EndpointReliabilityObservation { .. }
            | K::MeshNodeBudget { .. } => Debug,

            K::AgentSpawned { .. }
            | K::AgentRetired { .. }
            | K::OperatingModeChanged { .. }
            | K::FeedbackRequested { .. }
            | K::FeedbackResolved { .. }
            | K::TaskSubmitted { .. }
            | K::TaskStarted { .. }
            | K::TaskPhaseChanged { .. }
            | K::TaskCompleted { .. }
            | K::TaskDelegated { .. }
            | K::TaskResolved { .. }
            | K::LockAcquired { .. }
            | K::LockReleased { .. }
            | K::LockWaiting { .. }
            | K::ContinuationTriggered { .. }
            | K::PlanHandoff { .. }
            | K::CompactionTriggered { .. }
            | K::MemoryFlushed { .. }
            | K::SessionCreated { .. }
            | K::SessionReset { .. }
            | K::SnapshotCaptured { .. }
            | K::OperationUndone { .. }
            | K::OperationRedone { .. }
            | K::AgentHandoffAccepted { .. }
            | K::PlanningRouted { .. }
            | K::PlanSessionCreated { .. }
            | K::PlanVersionCreated { .. }
            | K::WorkflowHandoffRequested { .. }
            | K::WorkflowHandoffCompleted { .. }
            | K::WorkflowStarted { .. }
            | K::WorkflowCompleted { .. }
            | K::ActivityStarted { .. }
            | K::ActivityCompleted { .. }
            | K::ConflictResolved { .. }
            | K::WorkspaceCreated { .. }
            | K::OrchestratorIdle { .. }
            | K::AutoHealApplied { .. }
            | K::AttentionBudgetReset { .. }
            | K::AttentionConfigReloaded
            | K::OrientCompleted { .. }
            | K::ResearchExecuted { .. }
            | K::ResearchSynthesisExecuted { .. }
            | K::MeshTopologyChanged { .. }
            | K::TaskReprioritized { .. }
            | K::HopperItemAdmitted { .. }
            | K::HopperItemOverridden { .. }
            | K::HopperItemCancelled { .. }
            | K::MeshActionCommitted { .. }
            | K::PavPhaseChanged { .. } => Info,
        }
    }
}
```

If the compiler reports a variant missing from the match, classify it: an outright failure is `Error`; something a
person should look at is `Warning`; a high-frequency tick, stream or telemetry sample is `Debug`; anything else is
`Info`. Add it, list it in your report, and do **not** add a `_ =>` arm. If `BudgetSignal` has variants not listed,
classify them the same way.

- [ ] **Step 4: Run to verify pass** — `timeout 1500s cargo test -p vox-orchestrator --lib events_severity 2>&1 | tail -10 && timeout 1500s cargo test -p vox-orchestrator --lib 2>&1 | tail -5`.
- [ ] **Step 5 (Claude): mutation proofs** — (a) `InjectionDetected` moved to the Info arm → `failures_and_safety_events_are_errors`;
  (b) `if *flagged { Warning } else { Info }` → `Info` → `a_flagged_grounding_check_is_a_warning`; (c) add
  `_ => Info` as the last arm and delete `K::PavPhaseChanged { .. }` from the Info arm, then confirm that it compiles.
  That is the regression the no-wildcard rule prevents; restore, and record in the commit body that the guard is the
  compiler, not a test.
- [ ] **Step 6: Commit (Claude Code)** — `git add crates/vox-orchestrator/src/events_severity.rs crates/vox-orchestrator/src/lib.rs`;
  message "feat(orchestrator): every engine event carries a severity decided where it is defined".

---

### Task 2: The GUI bridge annotates severity and emits `activity-appended`

**Files:**
- Create `crates/vox-gui/src/commands/event_annotate.rs`.
- Modify `crates/vox-gui/src/commands/mod.rs` (add `pub mod event_annotate;`).
- Modify `crates/vox-gui/src/commands/orchestrator.rs`: only the `while let Some(value) = rx.recv().await { … }`
  loop inside `spawn_agent_event_stream`.

- [ ] **Step 1: Write the failing tests** — add `pub mod event_annotate;` to `commands/mod.rs` now, and create
  `event_annotate.rs` with only:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_token_frame_is_debug_and_not_logged_without_being_parsed() {
        let mut frame = json!({ "id": 1, "timestamp_ms": 0, "kind": { "type": "token_streamed", "anything": true } });
        assert!(!annotate_agent_event(&mut frame));
        assert_eq!(frame["severity"], json!("debug"));
    }

    #[test]
    fn a_loggable_event_is_annotated_and_reported() {
        let mut frame = json!({ "id": 2, "timestamp_ms": 5, "kind": {
            "type": "lock_waiting", "resource_id": "db://x", "task_id": 7 } });
        assert!(annotate_agent_event(&mut frame), "LockWaiting is activity-loggable");
        assert_eq!(frame["severity"], json!("info"));
    }

    #[test]
    fn an_error_event_carries_its_severity() {
        let mut frame = json!({ "id": 3, "timestamp_ms": 5, "kind": { "type": "injection_detected", "detail": "x" } });
        assert!(!annotate_agent_event(&mut frame));
        assert_eq!(frame["severity"], json!("error"));
    }

    #[test]
    fn the_token_fast_path_agrees_with_the_engine() {
        let kind: AgentEventKind =
            serde_json::from_value(json!({ "type": "token_streamed", "agent_id": 1, "text": "" })).unwrap();
        assert_eq!(serde_json::to_value(kind.severity()).unwrap(), json!("debug"));
    }

    #[test]
    fn a_replay_frame_refreshes_the_activity_view_and_is_left_alone() {
        // Shape of `orch_daemon::replay_frame_value`: rows written while the stream was down.
        let mut frame = json!({ "replay": true, "op_id": 4, "agent_id": 1, "timestamp_ms": 0,
            "description": "d", "kind": null });
        let before = frame.clone();
        assert!(annotate_agent_event(&mut frame));
        assert_eq!(frame, before);
    }

    #[test]
    fn a_frame_that_is_not_an_agent_event_is_left_alone() {
        let mut frame = json!({ "offset": 9, "kind": { "type": "not_a_kind" } });
        let before = frame.clone();
        assert!(!annotate_agent_event(&mut frame));
        assert_eq!(frame, before);
    }
}
```

- [ ] **Step 2: Run to verify failure** — `timeout 1500s cargo test -p vox-gui --bin vox-gui event_annotate > target/obs-t2-red.txt 2>&1; tail -15 target/obs-t2-red.txt`.
  Expected: fails to compile. If the build fails inside `tauri-build` (missing sidecar or `ui/dist`), that is the
  known fresh-worktree issue in AGENTS.md: STOP and report.
- [ ] **Step 3: Implement** (above the tests):

```rust
//! Adds the engine's own severity to each agent-event frame the GUI receives, and says whether the
//! event is one the activity log records, so no GUI code classifies events by string matching.

use serde::Deserialize;
use serde_json::Value;
use vox_orchestrator::AgentEventKind;

/// Tauri event emitted when the Activity view should re-query (an activity-loggable event or a replay frame).
pub const ACTIVITY_APPENDED_EVENT: &str = "vox://activity-appended";

/// Set `severity` on an agent-event frame and return whether the Activity view should refresh: true
/// for an activity-loggable event and for a replay frame (rows written while the stream was down).
/// A replay frame, or a frame whose `kind` is not an `AgentEventKind`, is left unchanged.
pub fn annotate_agent_event(frame: &mut Value) -> bool {
    let Some(obj) = frame.as_object_mut() else {
        return false;
    };
    if obj.get("replay").and_then(Value::as_bool) == Some(true) {
        return true;
    }
    let Some(kind) = obj.get("kind") else {
        return false;
    };
    // Token frames are the high-frequency path: classify without parsing.
    if kind.get("type").and_then(Value::as_str) == Some("token_streamed") {
        obj.insert("severity".into(), Value::from("debug"));
        return false;
    }
    let Ok(kind) = AgentEventKind::deserialize(kind) else {
        return false;
    };
    if let Ok(severity) = serde_json::to_value(kind.severity()) {
        obj.insert("severity".into(), severity);
    }
    vox_orchestrator::activity::is_loggable(&kind)
}
```

  In `orchestrator.rs`, replace the body of `while let Some(value) = rx.recv().await { … }` with:

```rust
                if let Some(offset) = extract_offset(&value) {
                    last_offset = Some(offset);
                }
                let mut value = value;
                let loggable = super::event_annotate::annotate_agent_event(&mut value);
                let _ = app_handle.emit(AGENT_EVENTS_EVENT, value);
                if loggable {
                    let _ = app_handle.emit(super::event_annotate::ACTIVITY_APPENDED_EVENT, ());
                }
```

- [ ] **Step 4: Run to verify pass** — `timeout 1500s cargo test -p vox-gui --bin vox-gui event_annotate 2>&1 | tail -10 && timeout 1500s cargo clippy -p vox-gui --all-targets -- -D warnings 2>&1 | tail -3`.
- [ ] **Step 5 (Claude): mutation proofs** — (a) delete the token fast path → the token test fails, because
  `anything: true` does not parse as `TokenStreamed`; (b) always `return false` → `a_loggable_event_is_annotated_and_reported`;
  (c) insert `severity` before parsing (for every frame) → `a_frame_that_is_not_an_agent_event_is_left_alone`;
  (d) delete the `replay` early return → the replay test.
- [ ] **Step 6: Commit** — the three files; message "feat(gui): agent-event frames carry severity; activity-appended is emitted".

---

### Task 3: The notice model and its routing rule

**Files:** Create `crates/vox-gui/ui/src/lib/notices.ts` and `notices.test.ts`.

- [ ] **Step 1: Write the failing test** — `notices.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { noticeFromToast, noticeFromAgentEvent } from './notices';

describe('notices', () => {
  it('a toast becomes an action notice with a matching severity', () => {
    expect(noticeFromToast({ tone: 'ok', title: 'Saved', cause: 'backend-ok' }, 10)).toMatchObject(
      { severity: 'success', scope: 'action', source: 'backend-ok', title: 'Saved', groupKey: 'Saved', atMs: 10 });
    expect(noticeFromToast({ tone: 'warn', title: 'x', cause: 'backend-error' }, 1).severity).toBe('warning');
    expect(noticeFromToast({ tone: 'info', title: 'x', cause: 'external' }, 1).severity).toBe('info');
  });

  it('only engine warnings and errors become notices, grouped per task', () => {
    const frame = (type: string, severity?: string) =>
      ({ id: 1, timestamp_ms: 5, severity, kind: { type, task_id: 7 } });
    expect(noticeFromAgentEvent(frame('task_failed', 'error'))).toMatchObject(
      { severity: 'error', scope: 'engine', source: 'engine', groupKey: 'engine:task_failed:7', atMs: 5 });
    expect(noticeFromAgentEvent(frame('tool_timed_out', 'warning'))?.severity).toBe('warning');
    expect(noticeFromAgentEvent(frame('task_completed', 'info'))).toBeNull();
    expect(noticeFromAgentEvent(frame('token_streamed', 'debug'))).toBeNull();
    expect(noticeFromAgentEvent(frame('task_failed'))).toBeNull();
  });

  it('an engine notice the mapper has no body for shows the event\'s own detail', () => {
    const n = noticeFromAgentEvent({ id: 1, timestamp_ms: 5, severity: 'error',
      kind: { type: 'injection_detected', detail: 'prompt override in tool output' } });
    expect(n?.body).toBe('prompt override in tool output');
    expect(n?.groupKey).toBe('engine:injection_detected');
  });
});
```

- [ ] **Step 2: Run to verify failure** — `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/notices.test.ts > target/obs-t3-red.txt 2>&1; tail -12 target/obs-t3-red.txt`.
  Expected: fails (module missing).
- [ ] **Step 3: Implement** — `notices.ts`:

```ts
import type { Toast } from '../types/tauri';
import type { AgentEventFrame } from './chatCorrelation';
import { mapAgentEvent } from './mapAgentEvent';
import { toastGroupKey } from './toastQueue';

/** docs/src/architecture/gui-observability-ssot-2026.md — every message the GUI shows is a notice. */
export type NoticeSeverity = 'success' | 'info' | 'warning' | 'error';
/** Scopes this store holds. `turn` and `session` notices live in the chat trace and rail, never here. */
export type NoticeScope = 'action' | 'engine' | 'app';

export interface NoticeInput {
  severity: NoticeSeverity;
  scope: NoticeScope;
  /** Producer id: a toast's `cause`, or `engine`. */
  source: string;
  title: string;
  body?: string;
  cmd?: string;
  /** Coalescing identity; defaults to `source` + `title`. */
  groupKey?: string;
  atMs?: number;
}

/** An agent-event frame as the GUI bridge forwards it, with the engine's severity. */
export type SeverityFrame = AgentEventFrame & { severity?: string };

const TONE_SEVERITY: Record<Toast['tone'], NoticeSeverity> = {
  ok: 'success',
  info: 'info',
  warn: 'warning',
};

/** Event fields that carry a human-readable reason, in order of preference. */
const DETAIL_FIELDS = ['error', 'detail', 'reason', 'tool_key'] as const;

export function noticeFromToast(t: Toast, atMs: number = Date.now()): NoticeInput {
  return {
    severity: TONE_SEVERITY[t.tone],
    scope: 'action',
    source: t.cause,
    title: t.title,
    body: t.body,
    cmd: t.cmd,
    // Same identity the toast stack coalesces by.
    groupKey: toastGroupKey(t),
    atMs,
  };
}

/** Engine warnings and errors only; everything else stays in the trace and the activity log. */
export function noticeFromAgentEvent(frame: SeverityFrame): NoticeInput | null {
  if (frame.severity !== 'warning' && frame.severity !== 'error') return null;
  const item = mapAgentEvent(frame);
  const kind = frame.kind;
  const detail = DETAIL_FIELDS.map(f => kind[f]).find((v): v is string => typeof v === 'string' && v !== '');
  // One notice per event type *and* subject, so two different tasks' failures never merge into one.
  const subject = kind.task_id ?? kind.agent_id ?? kind.workflow_id;
  return {
    severity: frame.severity,
    scope: 'engine',
    source: 'engine',
    title: item.title,
    body: item.body || detail,
    groupKey: `engine:${kind.type}${subject != null ? `:${String(subject)}` : ''}`,
    atMs: frame.timestamp_ms || Date.now(),
  };
}
```

- [ ] **Step 4: Run to verify pass** — the Step 2 command plus `timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -3`.
- [ ] **Step 5 (Claude): mutation proofs** — (a) `frame.severity !== 'warning' && …` → `false` (always a notice) →
  the engine test; (b) drop the `subject` suffix from `groupKey` → the engine test; (c) `body: item.body || detail`
  → `body: item.body` → the detail test.
- [ ] **Step 6: Commit** — `notices.ts`, `notices.test.ts`; message "feat(gui): a notice model with one routing rule".

---

### Task 4: A notification store that remembers every toast and engine problem

**Files:**
- Create `crates/vox-gui/ui/src/lib/noticeStore.ts`, `noticeStore.test.ts` and `hooks/useNoticeCenter.ts`.
- Modify `types/tauri.ts`: `Toast.tone` gains `'error'`.
- Modify `components/ui/Toasts.tsx`: the `error` tone class and icon, `ToastItem.tone`, and a `data-testid` on the
  container. Modify `Toasts.test.tsx` (new test appended).
- Modify `lib/notices.ts` and `notices.test.ts`: the `error` tone.
- Modify `App.tsx`: `pushToast` and the `listenAgentEvents` effect only.

- [ ] **Step 1: Write the failing test** — `noticeStore.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import {
  noticeReducer, EMPTY_NOTICES, MAX_NOTICES, MAX_BODY_CHARS, COALESCE_WINDOW_MS, unreadProblems,
} from './noticeStore';

const warn = (title: string, atMs: number) =>
  ({ type: 'record' as const, input: { severity: 'warning' as const, scope: 'engine' as const, source: 'engine', title, atMs } });

describe('noticeReducer', () => {
  it('records newest first and counts unread problems', () => {
    let s = noticeReducer(EMPTY_NOTICES, warn('a', 1));
    s = noticeReducer(s, { type: 'record', input: { severity: 'success', scope: 'action', source: 'backend-ok', title: 'ok', atMs: 2 } });
    expect(s.notices.map(n => n.title)).toEqual(['ok', 'a']);
    expect(unreadProblems(s.notices)).toBe(1);
  });

  it('coalesces a repeat inside the window into one notice with a count', () => {
    let s = noticeReducer(EMPTY_NOTICES, warn('Tool timed out', 1_000));
    s = noticeReducer(s, warn('Tool timed out', 1_000 + COALESCE_WINDOW_MS - 1));
    expect(s.notices).toHaveLength(1);
    expect(s.notices[0].count).toBe(2);
    s = noticeReducer(s, warn('Tool timed out', 1_000 + 3 * COALESCE_WINDOW_MS));
    expect(s.notices).toHaveLength(2);
  });

  it('a merged repeat shows the latest title and never lowers the severity', () => {
    const rec = (severity: 'error' | 'warning', title: string, atMs: number) =>
      ({ type: 'record' as const, input: { severity, scope: 'engine' as const, source: 'engine', title, groupKey: 'g', atMs } });
    let s = noticeReducer(EMPTY_NOTICES, rec('error', 'budget critical', 1));
    s = noticeReducer(s, rec('warning', 'budget high', 2));
    expect(s.notices).toHaveLength(1);
    expect(s.notices[0]).toMatchObject({ title: 'budget high', severity: 'error', count: 2 });
  });

  it('a repeat after reading brings the notice back as unread', () => {
    let s = noticeReducer(EMPTY_NOTICES, warn('x', 1));
    s = noticeReducer(s, { type: 'markAllRead' });
    expect(unreadProblems(s.notices)).toBe(0);
    s = noticeReducer(s, warn('x', 2));
    expect(unreadProblems(s.notices)).toBe(1);
  });

  it('keeps at most MAX_NOTICES, dropping the oldest', () => {
    let s = EMPTY_NOTICES;
    for (let i = 0; i < MAX_NOTICES + 5; i++) s = noticeReducer(s, warn(`n${i}`, i * 10 * COALESCE_WINDOW_MS));
    expect(s.notices).toHaveLength(MAX_NOTICES);
    expect(s.notices[s.notices.length - 1].title).toBe('n5');
  });

  it('caps a notice body at MAX_BODY_CHARS', () => {
    const s = noticeReducer(EMPTY_NOTICES, { type: 'record', input: {
      severity: 'error', scope: 'engine', source: 'engine', title: 't', body: 'x'.repeat(5000), atMs: 1 } });
    expect(s.notices[0].body).toHaveLength(MAX_BODY_CHARS);
  });
});
```

Append to `components/ui/Toasts.test.tsx`:

```tsx
describe('Toasts error tone', () => {
  it('renders an error toast in the fail status colour', () => {
    const { container } = render(
      <Toasts items={[{ id: 'e', tone: 'error', title: 'Save failed', cause: 'backend-error' }]} onClose={vi.fn()} />);
    expect(screen.getByText('Save failed')).toBeInTheDocument();
    expect(container.innerHTML).toContain('--color-status-fail');
    expect(screen.getByTestId('toast-stack')).toBeInTheDocument();
  });
});
```

Append to the first `it` in `lib/notices.test.ts`:
`expect(noticeFromToast({ tone: 'error', title: 'x', cause: 'backend-error' }, 1).severity).toBe('error');`

- [ ] **Step 2: Run to verify failure** — `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/noticeStore.test.ts > target/obs-t4-red.txt 2>&1; tail -12 target/obs-t4-red.txt`.
- [ ] **Step 3: Implement** — `noticeStore.ts`:

```ts
import type { NoticeInput, NoticeSeverity } from './notices';

export const MAX_NOTICES = 200;
/** A repeat of the same notice within this window merges into it (×N). */
export const COALESCE_WINDOW_MS = 60_000;
/** Longest body kept; a task-failed error can be a whole stack trace. */
export const MAX_BODY_CHARS = 1024;

const RANK: Record<NoticeSeverity, number> = { success: 0, info: 0, warning: 1, error: 2 };

export interface Notice {
  id: string;
  groupKey: string;
  severity: NoticeSeverity;
  scope: NoticeInput['scope'];
  source: string;
  title: string;
  body?: string;
  cmd?: string;
  count: number;
  lastAtMs: number;
  read: boolean;
}

export interface NoticeState {
  notices: Notice[];
  seq: number;
}

export const EMPTY_NOTICES: NoticeState = { notices: [], seq: 0 };

export type NoticeAction = { type: 'record'; input: NoticeInput } | { type: 'markAllRead' };

export function noticeReducer(state: NoticeState, action: NoticeAction): NoticeState {
  switch (action.type) {
    case 'markAllRead':
      return { ...state, notices: state.notices.map(n => (n.read ? n : { ...n, read: true })) };
    case 'record': {
      const input = action.input;
      const atMs = input.atMs ?? Date.now();
      const groupKey = input.groupKey ?? `${input.source}:${input.title}`;
      const body = input.body?.slice(0, MAX_BODY_CHARS);
      const hit = state.notices.findIndex(n => n.groupKey === groupKey && atMs - n.lastAtMs < COALESCE_WINDOW_MS);
      if (hit !== -1) {
        const prev = state.notices[hit];
        const merged: Notice = {
          ...prev,
          // Title and body describe the latest occurrence; the severity stays at the worst one seen.
          title: input.title,
          severity: RANK[input.severity] > RANK[prev.severity] ? input.severity : prev.severity,
          body,
          cmd: input.cmd,
          count: prev.count + 1,
          lastAtMs: atMs,
          read: false,
        };
        return { ...state, notices: [merged, ...state.notices.filter((_, i) => i !== hit)] };
      }
      const notice: Notice = {
        id: `notice-${state.seq + 1}`,
        groupKey,
        severity: input.severity,
        scope: input.scope,
        source: input.source,
        title: input.title,
        body,
        cmd: input.cmd,
        count: 1,
        lastAtMs: atMs,
        read: false,
      };
      return { seq: state.seq + 1, notices: [notice, ...state.notices].slice(0, MAX_NOTICES) };
    }
  }
}

/** Unread warnings and errors: the number the bell shows. */
export function unreadProblems(notices: Notice[]): number {
  return notices.filter(n => !n.read && (n.severity === 'warning' || n.severity === 'error')).length;
}
```

`hooks/useNoticeCenter.ts`:

```ts
import { useCallback, useReducer } from 'react';
import { EMPTY_NOTICES, noticeReducer, type Notice } from '../lib/noticeStore';
import type { NoticeInput } from '../lib/notices';

export interface NoticeCenter {
  notices: Notice[];
  record(input: NoticeInput): void;
  markAllRead(): void;
}

/** The app's notice store. `record` is stable, so listeners can capture it once. */
export function useNoticeCenter(): NoticeCenter {
  const [state, dispatch] = useReducer(noticeReducer, EMPTY_NOTICES);
  const record = useCallback((input: NoticeInput) => dispatch({ type: 'record', input }), []);
  const markAllRead = useCallback(() => dispatch({ type: 'markAllRead' }), []);
  return { notices: state.notices, record, markAllRead };
}
```

The remaining edits:
  - `types/tauri.ts`: `tone: 'ok' | 'warn' | 'info';` becomes `tone: 'ok' | 'warn' | 'info' | 'error';`.
  - `Toasts.tsx`:
    - `ToastItem.tone` becomes `tone: Toast['tone'];` (import `Toast` alongside `ToastCause`), so the two unions
      cannot drift.
    - `TONE_ICON_CLASS` gains `error: 'bg-(--color-status-fail)/15 text-(--color-status-fail)',`.
    - The icon ternary becomes
      `t.tone === "ok" ? <Icon.check …/> : t.tone === "warn" || t.tone === "error" ? <Icon.alert …/> : <Icon.bolt …/>`.
    - The container `div` (the one with `role="status"`) gains `data-testid="toast-stack"`.
  - `notices.ts`: `TONE_SEVERITY` gains `error: 'error',`.
  - `App.tsx`:
    1. Directly before `const toastTimers = useRef…` add
       `const noticeCenter = useNoticeCenter();` and `const recordNotice = noticeCenter.record;`.
    2. In `pushToast`, as its first statement, add `recordNotice(noticeFromToast(t));`, and change its deps from
       `[]` to `[recordNotice]`.
    3. In the `listenAgentEvents((frame) => { … })` callback, as its first statement, add
       `const engineNotice = noticeFromAgentEvent(frame as SeverityFrame); if (engineNotice) recordNotice(engineNotice);`,
       and change that effect's deps from `[]` to `[recordNotice]`.
    4. Add the imports (`useNoticeCenter`; `noticeFromToast`, `noticeFromAgentEvent`, `type SeverityFrame`).

  Change nothing else in `App.tsx`. `noticeCenter` is consumed in Task 6.
- [ ] **Step 4: Run to verify pass** — `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/noticeStore.test.ts src/lib/notices.test.ts src/components/ui/Toasts.test.tsx 2>&1 | tail -8 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -3 && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run 2>&1 | tail -6`.
- [ ] **Step 5 (Claude): mutation proofs** — (a) `atMs - n.lastAtMs < COALESCE_WINDOW_MS` → `true` → the coalesce
  test; (b) `read: false` in the merge → `read: prev.read` → the unread-again test; (c) `.slice(0, MAX_NOTICES)`
  removed → the cap test; (d) merged `severity: input.severity` → the merge test; (e) delete `title: input.title`
  from the merge → the merge test; (f) delete the `error` entry of `TONE_ICON_CLASS` → the Toasts error test.
  The `App.tsx` wiring has no unit test here; Task 6 adds one (the bell is mounted there).
- [ ] **Step 6: Commit** — the files listed; message "feat(gui): every toast and engine problem is kept in a notice store".

---

### Task 5: The notification center and its bell (standalone)

**Files:** Modify `components/ui/Icons.tsx` (a `bell` entry after `alert`). Create
`components/common/NotificationCenter.tsx` and `NotificationCenter.test.tsx`.

- [ ] **Step 1: Write the failing test** — `NotificationCenter.test.tsx`:

```tsx
// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, within } from '@testing-library/react';
import React from 'react';
import { NotificationCenter } from './NotificationCenter';
import type { Notice } from '../../lib/noticeStore';

const n = (id: string, severity: Notice['severity'], title: string, extra: Partial<Notice> = {}): Notice => ({
  id, groupKey: id, severity, scope: 'engine', source: 'engine', title,
  count: 1, lastAtMs: 0, read: false, ...extra,
});

const notices = [
  n('1', 'error', 'FAILED · task 7', { count: 2, body: 'error: boom' }),
  n('2', 'success', 'Saved', { scope: 'action', source: 'backend-ok', read: true }),
];

const bell = (name: string | RegExp = /^Notifications/) => screen.getByRole('button', { name });

describe('NotificationCenter', () => {
  it('the bell names the number of unread problems', () => {
    render(<NotificationCenter notices={notices} onMarkAllRead={vi.fn()} />);
    expect(bell('Notifications, 1 need attention').getAttribute('aria-expanded')).toBe('false');
    expect(screen.getByTestId('notification-unread').textContent).toBe('1');
  });

  it('shows no count when nothing needs attention, keeping the count slot so the bell does not shift', () => {
    render(<NotificationCenter notices={[notices[1]]} onMarkAllRead={vi.fn()} />);
    expect(screen.queryByTestId('notification-unread')).toBeNull();
    expect(bell('Notifications').querySelector('[data-slot="count"]')).not.toBeNull();
  });

  it('opens a focused drawer that lists notices with counts and filters to problems', () => {
    render(<NotificationCenter notices={notices} onMarkAllRead={vi.fn()} />);
    fireEvent.click(bell());
    expect(bell().getAttribute('aria-expanded')).toBe('true');
    const dialog = screen.getByRole('dialog', { name: 'Notifications' });
    expect(document.activeElement).toBe(dialog);
    const items = within(dialog).getAllByRole('listitem');
    expect(items).toHaveLength(2);
    expect(items[0].getAttribute('data-severity')).toBe('error');
    expect(within(items[0]).getByText('×2')).toBeTruthy();
    fireEvent.click(within(dialog).getByRole('radio', { name: 'Problems' }));
    expect(within(dialog).getAllByRole('listitem')).toHaveLength(1);
  });

  it('marks all read; Escape closes and returns focus to the bell', () => {
    const onMarkAllRead = vi.fn();
    render(<NotificationCenter notices={notices} onMarkAllRead={onMarkAllRead} />);
    fireEvent.click(bell());
    fireEvent.click(screen.getByRole('button', { name: 'Mark all read' }));
    expect(onMarkAllRead).toHaveBeenCalled();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.activeElement).toBe(bell());
  });

  it('closes on a click outside', () => {
    render(<div><span data-testid="outside" /><NotificationCenter notices={notices} onMarkAllRead={vi.fn()} /></div>);
    fireEvent.click(bell());
    fireEvent.mouseDown(screen.getByTestId('outside'));
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('says so when there is nothing to report', () => {
    render(<NotificationCenter notices={[]} onMarkAllRead={vi.fn()} />);
    fireEvent.click(bell());
    expect(screen.getByText('Nothing to report')).toBeTruthy();
  });
});
```

- [ ] **Step 2: Run to verify failure** — `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/common/NotificationCenter.test.tsx > target/obs-t5-red.txt 2>&1; tail -12 target/obs-t5-red.txt`.
- [ ] **Step 3: Implement.** `Icons.tsx`, after the `alert` entry:

```tsx
  bell: (p: React.SVGProps<SVGSVGElement>) => (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" {...p}>
      <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" />
      <path d="M10.3 21a1.94 1.94 0 0 0 3.4 0" />
    </svg>
  ),
```

`NotificationCenter.tsx`:

```tsx
import React, { useEffect, useRef, useState } from 'react';
import { Icon } from '../ui/Icons';
import { unreadProblems, type Notice } from '../../lib/noticeStore';

const SEVERITY_COLOR: Record<Notice['severity'], string> = {
  error: 'var(--color-status-fail)',
  warning: 'var(--color-status-warn)',
  success: 'var(--color-status-pass)',
  info: 'var(--color-status-info)',
};

interface Props {
  notices: Notice[];
  onMarkAllRead(): void;
}

/**
 * The notification bell and its drawer (docs/src/architecture/gui-observability-ssot-2026.md).
 * Popover behaviour — outside click, document-level Escape, focus return — mirrors StatusBarCluster.
 */
export function NotificationCenter({ notices, onMarkAllRead }: Props) {
  const [open, setOpen] = useState(false);
  const [filter, setFilter] = useState<'all' | 'problems'>('all');
  const panelRef = useRef<HTMLElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const unread = unreadProblems(notices);
  const unreadError = notices.some(n => !n.read && n.severity === 'error');

  useEffect(() => {
    if (!open) return;
    panelRef.current?.focus();
    const handleOutside = (e: MouseEvent) => {
      const target = e.target as Node;
      if (panelRef.current?.contains(target) || triggerRef.current?.contains(target)) return;
      setOpen(false);
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        setOpen(false);
        triggerRef.current?.focus();
      }
    };
    document.addEventListener('mousedown', handleOutside);
    document.addEventListener('keydown', handleKey);
    return () => {
      document.removeEventListener('mousedown', handleOutside);
      document.removeEventListener('keydown', handleKey);
    };
  }, [open]);

  const shown = filter === 'all' ? notices : notices.filter(n => n.severity === 'warning' || n.severity === 'error');
  return (
    <div className="relative inline-flex items-center">
      <button
        ref={triggerRef}
        type="button"
        onClick={() => setOpen(o => !o)}
        aria-expanded={open}
        aria-controls="notification-center"
        aria-label={unread > 0 ? `Notifications, ${unread} need attention` : 'Notifications'}
        className="flex items-center gap-1 px-2 text-text-muted hover:text-text-primary"
      >
        <Icon.bell className="size-3.5" aria-hidden="true" />
        {/* The slot is always rendered at a fixed width so the bar does not shift when the count appears. */}
        <span data-slot="count" data-testid={unread > 0 ? 'notification-unread' : undefined}
          className="min-w-[3ch] font-mono text-[11px] tabular-nums"
          style={{ color: unreadError ? 'var(--color-status-fail)' : 'var(--color-status-warn)' }}>
          {unread > 0 ? unread : ''}
        </span>
      </button>
      {/* Engine problems never toast, so announce the count politely for screen-reader users. */}
      <span className="sr-only" aria-live="polite">{unread > 0 ? `${unread} notifications need attention` : ''}</span>
      {open && (
        <section
          ref={panelRef}
          id="notification-center"
          role="dialog"
          aria-label="Notifications"
          tabIndex={-1}
          className="absolute bottom-full right-0 z-50 mb-1 flex max-h-[60vh] w-[360px] max-w-[calc(100vw-2rem)] flex-col rounded-xl border border-border-subtle bg-bg-base p-3 text-xs shadow-2xl"
        >
          <header className="mb-2 flex items-center gap-2">
            <span className="flex-1 text-text-secondary">Notifications</span>
            <span role="radiogroup" aria-label="Show" className="flex gap-1">
              {(['all', 'problems'] as const).map(v => (
                <label key={v} className="flex items-center gap-1 text-text-muted">
                  <input type="radio" name="notice-filter" checked={filter === v} onChange={() => setFilter(v)} />
                  {v === 'all' ? 'All' : 'Problems'}
                </label>
              ))}
            </span>
            <button type="button" onClick={onMarkAllRead} className="text-text-muted hover:text-text-primary">
              Mark all read
            </button>
          </header>
          {shown.length === 0 ? (
            <p className="py-6 text-center text-text-muted">Nothing to report</p>
          ) : (
            <ul className="flex min-h-0 flex-col gap-1 overflow-y-auto">
              {shown.map(n => (
                <li key={n.id} data-severity={n.severity} data-read={n.read ? 'true' : undefined}
                  className="rounded-lg border-l-2 px-2 py-1"
                  style={{ borderColor: SEVERITY_COLOR[n.severity] }}>
                  <div className="flex items-baseline gap-1">
                    <span className={n.read ? 'text-text-muted' : 'text-text-primary'}>{n.title}</span>
                    {n.count > 1 && <span className="font-mono text-[11px] text-text-muted">×{n.count}</span>}
                    <span className="ml-auto text-[11px] text-text-muted">{n.source}</span>
                  </div>
                  {n.body && <div className="break-words text-text-muted">{n.body}</div>}
                  {n.cmd && <div className="font-mono text-[11px] text-text-muted">▸ {n.cmd}</div>}
                </li>
              ))}
            </ul>
          )}
        </section>
      )}
    </div>
  );
}
```

- [ ] **Step 4: Run to verify pass** — Step 2's command plus `typecheck`.
- [ ] **Step 5 (Claude): mutation proofs** — (a) the Problems filter returns `notices` → the filter assertion;
  (b) `aria-label` always `'Notifications'` → the first test; (c) delete the Escape branch → the Escape test;
  (d) delete `triggerRef.current?.focus()` → the focus assertion; (e) delete the `mousedown` listener → the
  outside-click test; (f) delete `panelRef.current?.focus()` → the focused-drawer assertion.
- [ ] **Step 6: Commit** — the three files; message "feat(gui): a notification center that keeps what toasts drop".

---

### Task 6: Mount the bell and center in the status bar (re-anchor before driving)

**Owner of the re-anchor: Claude.** The status bar is rewritten by the surfaces plan's Task 2. After that lands, Claude
reads it and rewrites this task with exact code. The decided behaviour:
- **Bell placement:** `<NotificationCenter notices={noticeCenter.notices} onMarkAllRead={noticeCenter.markAllRead} />`
  is the last status-bar item. The component owns its open state and focus (Task 5); `App.tsx` passes the store
  down through the status bar's props.
- Opening the drawer does **not** mark notices read; "Mark all read" does.
- **The Needs-you status card** (surfaces plan) must not render `0` when a source failed: with the Task 7 `degraded`
  list non-empty and a zero count it shows "—" and "couldn't load …" (SSOT rule 3).
- **Status-card dots are not part of this task** (see Deferred): every engine notice has `source: 'engine'`, so there is
  no per-card source yet, and the routing plan's Task 10 makes activating the Routing card open the Models surface.
- **Tests:**
  - an `App.test.tsx` case, using its existing render harness. It triggers an existing toast path (for example the
    budget-exceeded toast), then asserts that the bell's name is "Notifications, 1 need attention". This proves
    `pushToast` → store → bell. Mutation: delete `recordNotice(noticeFromToast(t))` → it fails.
  - a status-bar test that the Needs-you card shows "—" when `degraded` is `['approvals']` and the count is 0.
  - Task 12's Playwright spec covers the rest.

---

### Task 7: A failed source reads as degraded, not as all-clear

**Files:**
- Modify `crates/vox-gui/ui/src/hooks/useAttentionInbox.ts` and `useAttentionInbox.test.ts` (new tests appended).
- Modify `crates/vox-gui/ui/src/components/layout/Sidebar.tsx`: the `needsYouCount` prop area and the `runs` badge
  expression.
- Modify `Sidebar.test.tsx` (new `describe` appended).
- Modify `components/layout/AppShell.tsx`: `AppShellProps` gains `needsYouDegraded?: string[];` next to
  `needsYouCount`; destructure it; pass `needsYouDegraded={needsYouDegraded}` to `<Sidebar>` next to
  `needsYouCount={needsYouCount}`.
- Modify `App.tsx`: one prop at the `<AppShell` call (App renders `AppShell`, which renders `Sidebar`).

- [ ] **Step 1: Write the failing tests.** Append to `useAttentionInbox.test.ts` (reuse its `invokeMock` and
  transport mocks):

```ts
describe('useAttentionInbox degraded sources', () => {
  it('names each source that failed instead of reporting it as empty', async () => {
    invokeMock.mockImplementation((cmd: string) =>
      cmd === 'hopper_list' ? Promise.reject(new Error('down')) : Promise.resolve(null));
    const { result } = renderHook(() => useAttentionInbox());
    await waitFor(() => expect(result.current.degraded).toEqual(['tasks']));
    expect(result.current.hopperTasks).toEqual([]);
  });

  it('reports no degraded source when every fetch succeeds', async () => {
    const { result } = renderHook(() => useAttentionInbox());
    await waitFor(() => expect(result.current.approvals.length).toBe(1));
    expect(result.current.degraded).toEqual([]);
  });

  it('treats an MCP error reply as a failed source, not as no approvals', async () => {
    vi.mocked(voxTransport.invokeMcpTool).mockResolvedValueOnce(
      { tool: 'vox_pending_approvals', is_error: true, result: null } as never);
    const { result } = renderHook(() => useAttentionInbox());
    await waitFor(() => expect(result.current.degraded).toEqual(['approvals']));
    expect(result.current.approvals).toEqual([]);
  });
});
```

Append to `Sidebar.test.tsx` (it uses the file's `renderSidebar` helper; the `beforeEach` stubs `scrollIntoView`,
which `Sidebar` calls on mount):

```tsx
describe('Sidebar degraded Review badge', () => {
  beforeEach(() => {
    Element.prototype.scrollIntoView = vi.fn();
    window.localStorage.clear();
  });

  it('shows a degraded badge instead of all-clear when a source failed', () => {
    renderSidebar({ needsYouCount: 0, needsYouDegraded: ['approvals'] } as never);
    const review = screen.getByRole('button', { name: "Review, couldn't load approvals" });
    expect(review.textContent).toContain('!');
  });

  it('keeps the count and names the failed source when other sources have items', () => {
    renderSidebar({ needsYouCount: 2, needsYouDegraded: ['feedback'] } as never);
    expect(screen.getByRole('button', { name: "Review, 2 items need you (couldn't load feedback)" })).toBeDefined();
  });
});
```

- [ ] **Step 2: Run to verify failure** — `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/hooks/useAttentionInbox.test.ts src/components/layout/Sidebar.test.tsx > target/obs-t7-red.txt 2>&1; tail -15 target/obs-t7-red.txt`.
- [ ] **Step 3: Implement.** In `useAttentionInbox.ts`:
  1. Add `degraded: string[];` to `AttentionInbox`, with the doc comment "Sources whose last fetch failed
     (`approvals`, `feedback`, `tasks`); their lists are empty because they are unknown, not because they are clear."
  2. Add `const [degraded, setDegraded] = useState<string[]>([]);`.
  3. Replace the body of `refresh` with:

```ts
    const emptyFeedback = { needsYou: [] as FeedbackRow[], withheld: [] as FeedbackRow[] };
    const [approvalRes, feedback, tasks] = await Promise.allSettled([
      Promise.resolve(voxTransport.invokeMcpTool('vox_pending_approvals', {})),
      Promise.resolve(feedbackList()),
      Promise.resolve(hopperList()),
    ]);
    // An MCP error reply resolves instead of rejecting; it is a failed source all the same.
    const approvalReply = approvalRes.status === 'fulfilled' && !approvalRes.value?.is_error ? approvalRes.value : undefined;
    setDegraded([
      approvalRes.status === 'rejected' || approvalRes.value?.is_error ? 'approvals' : null,
      feedback.status === 'rejected' ? 'feedback' : null,
      tasks.status === 'rejected' ? 'tasks' : null,
    ].filter((s): s is string => s !== null));
    const safeFeedback = (feedback.status === 'fulfilled' && feedback.value) || emptyFeedback;
    const safeTasks = (tasks.status === 'fulfilled' && tasks.value) || [];
    setApprovals(approvalReply ? parsePendingApprovals(approvalReply as McpInvokeResult) : []);
    setNeedsYou(safeFeedback.needsYou ?? []);
    setWithheld(safeFeedback.withheld ?? []);
    const gates = new Set<number>((safeFeedback.needsYou ?? []).flatMap((f) => f.gates ?? []));
    setBlockedTasksCount(safeTasks.filter((t) => gates.has(t.task_id)).length);
    setHopperTasks(safeTasks);
```

  4. Add `degraded` to the returned object.

  In `Sidebar.tsx`:
  - Add the prop `needsYouDegraded?: string[];` next to `needsYouCount`, and destructure it.
  - Replace the `runs` branches of `badge` and `navAriaLabel` with:

```tsx
            const hasReviewItems = key === 'runs' && needsYouCount != null && needsYouCount > 0;
            const couldNotLoad =
              key === 'runs' && (needsYouDegraded?.length ?? 0) > 0 ? `couldn't load ${needsYouDegraded!.join(', ')}` : '';
            const badge =
              key === 'agents' ? agentsCount
              : hasReviewItems ? needsYouCount
              : couldNotLoad ? '!'
              : undefined;
            const navAriaLabel =
              key === 'runs'
                ? hasReviewItems
                  ? `Review, ${needsYouCount} items need you${couldNotLoad ? ` (${couldNotLoad})` : ''}`
                  : couldNotLoad
                    ? `Review, ${couldNotLoad}`
                    : 'Review'
                : undefined;
```

  (`NavItemProps.badge` is already `number | string | null`.) In `App.tsx`, at the `<AppShell` call, add
  `needsYouDegraded={attention.degraded.filter((s) => s !== 'tasks')}` after `needsYouCount={attention.totalCount}`,
  with the comment `{/* tasks are not part of the Review count */}` if the surrounding props carry comments.
- [ ] **Step 4: Run to verify pass** — the Step 2 command, then `typecheck` and the full `vitest run`.
- [ ] **Step 5 (Claude): mutation proofs** — (a) `tasks.status === 'rejected'` → `false` → the degraded test;
  (b) drop `|| approvalRes.value?.is_error` → the MCP-error test; (c) `couldNotLoad ? '!'` removed → the first
  Sidebar test; (d) drop the `(couldn't load …)` suffix in the count branch → the second Sidebar test; (e) drop the
  `AppShell` forward → `typecheck` still passes, so Claude checks it by reading the diff and by Task 12's test 4.
- [ ] **Step 6: Commit** — the files; message "fix(gui): a failed inbox source reads as degraded, not as all-clear".

---

### Task 8: The Activity view refreshes when a row is written, not on every token

**Precondition:** Task 2 committed (the bridge emits `vox://activity-appended`).

**Files:**
- Modify `crates/vox-gui/ui/src/components/surfaces/Activity/ActivitySurface.tsx`: delete the `listenAgentEvents`
  effect and its import; debounce the `listenActivityAppended` effect.
- Modify `crates/vox-gui/ui/src/config/constants.ts`: append `ACTIVITY_REFRESH_DEBOUNCE_MS`.
- Modify `ActivitySurface.container.test.tsx`: **named edit** — its test "refetches activity when vox://agent-events
  fires" asserts the behaviour this task removes; replace that one test with the one below, and capture the
  `listenActivityAppended` callback in the mock. Leave the null-safety test as it is.

- [ ] **Step 1: Write the failing test.** In `ActivitySurface.container.test.tsx`:
  1. Next to `let capturedAgentEventsCb …` add `let capturedAppendedCb: (() => void) | null = null;`.
  2. In the `vi.mock('../../../transport', …)` factory, replace `listenActivityAppended: vi.fn().mockResolvedValue(() => {}),` with:

```tsx
  listenActivityAppended: vi.fn().mockImplementation((cb: () => void) => {
    capturedAppendedCb = cb;
    return Promise.resolve(() => { capturedAppendedCb = null; });
  }),
```

  3. In `beforeEach`, add `capturedAppendedCb = null;`. Add
     `import { ACTIVITY_REFRESH_DEBOUNCE_MS } from '../../../config/constants';` after the other imports.
  4. Replace the test `it('refetches activity when vox://agent-events fires', …)` with:

```tsx
  it('re-queries once per burst of activity-appended and ignores other engine events', async () => {
    vi.useFakeTimers();
    try {
      const activityQueryMock = vi.mocked(transport.activityQuery);
      activityQueryMock.mockResolvedValue([]);
      render(<Probe><ActivitySurface pushToast={vi.fn()} /></Probe>);
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
      });
      const afterMount = activityQueryMock.mock.calls.length;
      expect(afterMount).toBeGreaterThanOrEqual(1);
      expect(capturedAppendedCb).not.toBeNull();

      // Token and other engine frames no longer re-query.
      await act(async () => {
        capturedAgentEventsCb?.();
        capturedAgentEventsCb?.();
        await vi.advanceTimersByTimeAsync(1000);
      });
      expect(activityQueryMock.mock.calls.length).toBe(afterMount);

      // A burst of appended rows becomes one query after the debounce.
      await act(async () => {
        capturedAppendedCb!();
        capturedAppendedCb!();
        capturedAppendedCb!();
        await vi.advanceTimersByTimeAsync(ACTIVITY_REFRESH_DEBOUNCE_MS + 10);
      });
      expect(activityQueryMock.mock.calls.length).toBe(afterMount + 1);
    } finally {
      vi.useRealTimers();
    }
  });
```

- [ ] **Step 2: Run to verify failure** — `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Activity/ActivitySurface.container.test.tsx > target/obs-t8-red.txt 2>&1; tail -12 target/obs-t8-red.txt`.
  Expected: the count after the two agent events is `afterMount + 2`, not `afterMount`.
- [ ] **Step 3: Implement.**
  - `config/constants.ts`, appended:

```ts
/** Trailing debounce for Activity view refreshes: a burst of rows becomes one query, and the wait gives the
 *  daemon's activity sink time to commit the row before the view reads it. */
export const ACTIVITY_REFRESH_DEBOUNCE_MS = 250;
```

  - `ActivitySurface.tsx`: delete the effect whose comment begins "Also refresh on "vox://agent-events"", and the
    now-unused `listenAgentEvents` import. Replace the remaining `listenActivityAppended` effect (keep its comment
    about `listen()` rejecting outside Tauri) with:

```tsx
  // Reactive updates on "vox://activity-appended", which the GUI bridge emits for every activity-loggable
  // event and replay frame (crates/vox-gui/src/commands/event_annotate.rs).
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    // listen() rejects when the Tauri event bridge is unavailable (bare
    // browser, tests, headless capture) — guard so nothing leaks an
    // unhandled rejection and cleanup still resolves.
    const unlistenPromise = listenActivityAppended(() => {
      clearTimeout(timer);
      timer = setTimeout(fetchLogs, ACTIVITY_REFRESH_DEBOUNCE_MS);
    }).catch(() => undefined);
    return () => {
      clearTimeout(timer);
      unlistenPromise.then((unlisten) => unlisten?.());
    };
  }, [fetchLogs]);
```

    and import `ACTIVITY_REFRESH_DEBOUNCE_MS` from `'../../../config/constants'`.
- [ ] **Step 4: Run to verify pass** — the Step 2 command, `typecheck`, and
  `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Activity`.
- [ ] **Step 5 (Claude): mutation proofs** — (a) restore the deleted agent-events effect → the test fails;
  (b) call `fetchLogs()` directly instead of debouncing → the burst assertion fails (`afterMount + 3`).- [ ] **Step 6: Commit** — the three files; message "fix(gui): the activity view refreshes on new rows, not on every token".

---

### Task 9: The chat status line shows the real phase and the done row shows the real cost

**Files:** Modify `crates/vox-gui/ui/src/lib/mapAgentEvent.ts` (the `metadata` object) and
`lib/chatTranscriptTimeline.ts` (inside `buildChatOnlyTimeline`: the loop and the `lastCostByTask` map). Append a
test to `lib/chatTranscriptTimeline.test.ts`.

- [ ] **Step 1: Write the failing test** — append to `chatTranscriptTimeline.test.ts`:

```ts
import { mapAgentEvent } from './mapAgentEvent';

describe('buildChatOnlyTimeline through the real event mapper', () => {
  // The existing tests hand-build metadata; these go through mapAgentEvent, the producer the app uses.
  const frame = (id: number, type: string, extra: Record<string, unknown>) =>
    mapAgentEvent({ id, timestamp_ms: id * 1000, kind: { type, ...extra } });

  it('shows the phase the engine reported', () => {
    const rows = buildChatOnlyTimeline([], [
      frame(1, 'task_started', { task_id: 7, agent_id: 3 }),
      frame(2, 'task_phase_changed', { task_id: 7, agent_id: 3, phase: 'Act' }),
    ], { nowMs: 5000 });
    expect(rows.find(r => r.kind === 'status')).toMatchObject({ taskId: 7, phase: 'Act' });
  });

  it('attributes each agent cost to that agent task and shows the total when the task completes', () => {
    const rows = buildChatOnlyTimeline([], [
      frame(1, 'task_started', { task_id: 7, agent_id: 3 }),
      frame(2, 'cost_incurred', { agent_id: 3, cost_usd: 0.25, provider: 'acme', model: 'acme/widget' }),
      frame(3, 'cost_incurred', { agent_id: 3, cost_usd: 0.5, provider: 'acme', model: 'acme/widget' }),
      frame(4, 'task_completed', { task_id: 7, agent_id: 3 }),
    ], { nowMs: 5000 });
    expect(rows.find(r => r.kind === 'summary')).toMatchObject({ taskId: 7, costUsd: 0.75 });
  });
});
```

- [ ] **Step 2: Run to verify failure** — `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/chatTranscriptTimeline.test.ts > target/obs-t9-red.txt 2>&1; tail -12 target/obs-t9-red.txt`.
  Expected: the phase is `'Working'`, and there is no summary row.
- [ ] **Step 3: Implement.** In `mapAgentEvent.ts`, add to the returned `metadata` object after `timestampMs`:

```ts
      ...(typeof kind.phase === 'string' ? { phase: kind.phase } : {}),
      ...(typeof kind.cost_usd === 'number' ? { costUsd: kind.cost_usd } : {}),
```

  In `buildChatOnlyTimeline`, directly before `for (const item of agentItems) {` add
  `const taskByAgent = new Map<string, number>();`, and replace the first three lines of that loop body (from
  `const eventType = …` through `if (taskId == null) continue;`) with:

```ts
    const eventType = item.metadata?.eventType;
    const agentId = item.metadata?.agentId as string | undefined;
    let taskId = item.taskId ?? (item.metadata?.taskId as number | undefined);
    if (taskId != null && agentId) taskByAgent.set(agentId, taskId);
    // Cost events carry only the agent; attribute them to that agent's current task.
    if (taskId == null && eventType === 'cost_incurred' && agentId) taskId = taskByAgent.get(agentId);
    if (taskId == null) continue;
```

  Rename `lastCostByTask` to `costByTask` throughout `buildChatOnlyTimeline`, and make its `set` sum each call:
  `costByTask.set(taskId, (costByTask.get(taskId) ?? 0) + costUsd);` (one `cost_incurred` per LLM call; the done
  row shows the task's total). Update the comment above the map to say "total cost".

- [ ] **Step 4: Run to verify pass** — the Step 2 command plus `src/lib/mapAgentEvent.test.ts` and `typecheck`.
- [ ] **Step 5 (Claude): mutation proofs** — (a) drop the `phase` spread → the phase test; (b) drop the cost
  attribution line → the cost test; (c) `set(taskId, costUsd)` (last, not total) → the cost test.
  Note: `CostIncurred` bus emission is off by default when a DB is attached (`events.rs`, `CostIncurred` doc), so the
  done row may still not render in production; Task 13's live check records whether it does.
- [ ] **Step 6: Commit** — the three files; message "fix(gui): the chat status line shows the engine's phase and cost".

---

### Task 10: Moved out of this plan

"One poll per fact" (SSOT rule 4) is not about notices, and it touches files the trace and surfaces plans rewrite
(`ChatTranscript.tsx`, the status bar, `DashboardView`, `ApprovalsView`, `HarnessIssuesPanel`). It becomes its own
plan after those land (see Deferred). The decided shape stays: `useAttentionInbox` is the only approvals poller;
harness issues are polled once and shared through `@tanstack/react-query` (already installed and mounted in
`main.tsx`) with one query key and `refetchInterval`, not a new context; the proof counts transport calls under
`vi.useFakeTimers()`.

---

### Task 11: Toasts are typed everywhere, so every toast carries an honest cause

**Files:** each file below, where `pushToast: (t: any) => void` (or the inline `{ pushToast: (t: any) => void }`, or
`TasksView`'s `pushToast?: (t: unknown) => void`) becomes `(t: Toast) => void` (keeping any `?`) with
`import type { Toast } from '<relative>/types/tauri';`:
- `components/layout/Sidebar.tsx`
- `surfaces/Approvals/ApprovalsView.tsx`
- `surfaces/Chat/ChatSurface.tsx`
- `surfaces/Matrix/Matrix.tsx`
- `surfaces/Memory/MemoryView.tsx`
- `surfaces/Models/ModelsView.tsx`
- `surfaces/Policies/PoliciesView.tsx`
- `surfaces/Runs/RunsView.tsx`
- `surfaces/Settings/PriorityChainEditor.tsx`
- `surfaces/Settings/SettingsView.tsx` (both `RuntimeConfigSection` and `LlmSettingsSection`)
- `surfaces/SkillsPlugins/SkillsPluginsView.tsx`
- `surfaces/Tasks/TasksView.tsx`

Create `crates/vox-gui/ui/src/lib/typedToasts.test.ts`.

- [ ] **Step 1: Write the failing test** — `typedToasts.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

// Same walk as components/surfaces/__guards__/surfaceHonesty.guard.test.ts (vitest runs from crates/vox-gui/ui).
function walk(d: string): string[] {
  return readdirSync(d).flatMap(n => {
    const p = join(d, n);
    return statSync(p).isDirectory() ? walk(p) : /\.tsx?$/.test(p) && !/\.test\.tsx?$/.test(p) ? [p] : [];
  });
}

describe('typed toasts', () => {
  it('no component takes an untyped pushToast', () => {
    const offenders = walk('src')
      .filter(f => /pushToast\??\s*:\s*\(\s*\w+\s*:\s*(any|unknown)\s*\)/.test(readFileSync(f, 'utf8')));
    expect(offenders).toEqual([]);
  });
});
```

- [ ] **Step 2: Run to verify failure** — `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/typedToasts.test.ts > target/obs-t11-red.txt 2>&1; tail -12 target/obs-t11-red.txt`.
  Expected: 12 offenders.
- [ ] **Step 3: Implement** — change the types. Then run `typecheck`: each call that now fails for a missing `cause`
  gets the honest cause:
  - `backend-error` for a failed invoke;
  - `backend-ok` for a successful mutation;
  - `validation` for rejected input;
  - `clipboard` / `external` where that is what happened.

  Known sites without a cause: `MemoryView` (~194, 208, 249, 296, 332-333), `ModelsView` (~69, 88, 90), `PoliciesView`
  (~61, 86, 98) and `RunsView` (~76). Where a failed invoke is a real error, use `tone: 'error'` (added in Task 4)
  rather than `warn`. Do not change any toast's title or body.
- [ ] **Step 4: Run to verify pass** — the Step 2 command, `typecheck`, and the full `vitest run`.
- [ ] **Step 5 (Claude): mutation proof** — revert one file's type to `(t: any)` → the source-scan test fails.
- [ ] **Step 6: Commit** — the files; message "fix(gui): every toast is typed and carries its cause".

---

### Task 12: Playwright — notices end to end (Owner: Claude, after Task 6)

The selectors depend on Task 6's mount, so Claude writes the spec code after Task 6 lands; it is not driven as prose.
**Files:** create `crates/vox-gui/ui/e2e/notices.spec.ts`; modify `e2e/lib/tauriMockShared.ts` so its `unlisten`
removes the handler (today it does not, and `__TAURI_EMIT__` calls every registered id, so StrictMode's double
effect in the dev server makes each emit count twice).

Facts the spec must respect (verified in the review):
- Rejecting a source needs a custom init script keyed on `cmd === 'invoke_mcp_tool' && args.tool === 'vox_pending_approvals'`
  (or the surfaces plan's `e2e/lib/invokeOverrides.ts` if it has landed). `installTrustOverrides` is local to
  `chat-trust-chips.spec.ts`, keys on `cmd` only and cannot reject.
- Toasts are located by `data-testid="toast-stack"` (Task 4); `getByRole('status')` matches ten elements.
- `activity_query` is also polled every 5 s by `useChatExecutionData` with a `session_id`; count only calls whose
  filter has no `session_id`.
- The Activity view mounts under Discovery with the `timeline` preset; navigate there explicitly.

Tests, each saving a screenshot to `review-bundle/latest/` at 1440×900:
1. **An engine error appears in the center, not as a toast** → `notices-engine-error.png`. Emit
   `vox://agent-events` with `{ id: 1, timestamp_ms: 1, severity: 'error', kind: { type: 'task_failed', task_id: 7, error: 'boom' } }`;
   the bell reads "Notifications, 1 need attention"; `toast-stack` has no child; opening the bell shows one
   `data-severity="error"` item whose text contains "boom".
2. **Repeated engine warnings coalesce** → `notices-coalesced.png`: three identical `tool_timed_out` warnings for
   one agent → one item with `×3`.
3. **A toast survives in the center after it expires:** make one invoke used by a surface action reject (the
   same init-script pattern), trigger the action, wait for `toast-stack` to empty, open the center; the item is there.
4. **A failed inbox source shows a degraded Review badge** → `notices-degraded-review.png`: the Review nav item is
   named "Review, couldn't load approvals".
5. **The Activity view refreshes only on `activity-appended`:** emitting `vox://agent-events` adds no
   (session-less) `activity_query` call; emitting `vox://activity-appended` adds exactly one after the debounce.
6. **The drawer at narrow width** → `notices-narrow.png`, at 390×844: the drawer fits and the page has no horizontal
   scroll.

- [ ] Run: `timeout 300s pnpm --dir crates/vox-gui/ui exec playwright test e2e/notices.spec.ts --project=chromium --reporter=line`.
  Expected: 6 passed. Re-run the chat-trust and models specs after the `tauriMockShared.ts` fix.
- [ ] Open each screenshot and check it against the SSOT's rules.
- [ ] Commit: "test(gui): visual verification of notices, severity and degraded states".

---

### Task 13: Verification sweep — Owner: Claude

- [ ] `cargo test -p vox-orchestrator --lib events_severity`
- [ ] `cargo test -p vox-gui --bin vox-gui event_annotate`
- [ ] clippy on both crates
- [ ] the full `vitest run`, `typecheck`, and the notices, models and chat-trust Playwright specs
- [ ] A live check with the real daemon:
  - start the app, submit a task that fails;
  - the failure appears in the center with severity `error`;
  - the Activity view gains a row without re-querying per token. Pass criterion: 0 session-less
    `activity_query` calls while a reply streams, and one per burst of loggable events.
  - the chat's done row shows a cost (if it does not, `CostIncurred` is not on the bus; record that under Deferred).
- [ ] `where-things-live.md` rows:
  - Notice model and routing: `lib/notices.ts`
  - Notification center: `components/common/NotificationCenter.tsx`, `lib/noticeStore.ts`, `hooks/useNoticeCenter.ts`
  - Engine event severity: `events_severity.rs`
  - Agent-event annotation and `activity-appended`: `vox-gui/src/commands/event_annotate.rs`
- [ ] Doc lint on the SSOT page and the rows.
- [ ] `vox ci pre-push --complete`, or the standalone fallback when the 25-minute budget is exceeded.

## Decisions (resolved 2026-09-29; open decisions delegated to Claude)

1. **Severity is decided in Rust, exhaustively.** A TypeScript string table would rot silently as event kinds are
   added. The compiler is the guard.
2. **Annotate in the GUI bridge, not in the durable event.** Adding a field to `AgentEvent` changes the Tier-A journal
   format. The bridge already parses nothing, and parses only non-token frames here.
3. **Toasts keep working as they do today.** Every toast is additionally recorded in the center. Moving existing
   toasts to center-only is a later, per-surface decision, because the SSOT allows toasts for `action` scope.
4. **The center is in memory**, bounded at 200 and coalescing within 60 s. Persisting it across restarts is not needed
   while the Activity log holds the durable history.
5. **Only engine warnings and errors become notices.** Info engine events stay in the trace and the Activity log;
   routing them to the center would recreate the spam this plan removes.
6. **No new dependency.** No OS notifications and no new query library (`@tanstack/react-query` is already installed).
7. **Every engine warning and error goes to the center, including a chat turn's own failure.** Deriving `turn`
   scope from `session_id` would hide it from the center while the trace plan already shows it in the transcript;
   the center is a history, not the chat, so one copy there is not spam. De-duplicating is Deferred.
8. **Observation streams stay `Debug`/`Info`** even when a payload looks bad (`MensObserverObservation`,
   `ToolCallDispatched { ok: false }`, `EndpointReliabilityObservation` with a failure outcome,
   `TaskResolved { validated: false }`). Their consequences (`TaskFailed`, `ToolTimedOut`, budget signals) carry the
   warning; promoting the observations would repeat each problem several times.
9. **The center filters by severity only.** Filtering by source waits until more than `engine` and the toast causes
   produce notices (see Deferred).

## Deferred

- **Diagnostics in the GUI:** `vox doctor` checks, including routing health, as an Engine-card popover section.
- **OS notifications** for `needs: decision` while the window is unfocused (`tauri-plugin-notification`).
- **Migrating the ~145 `tone: 'warn', cause: 'backend-error'` toasts to `tone: 'error'`**, per surface, alongside a
  review of which should become center-only.
- **Routing-health violations as engine notices:** they are computed in the orchestrator's refresh, not emitted as
  events. Add an `AgentEventKind::RoutingHealthChanged` event when the routing plan's Task 10 lands.
- **The main toast stack's z-order** (`z-40`) against the onboarding overlay (`z-70`).
- **Severity on durable activity rows** (a schema change).
- **One poll per fact** (former Task 10): its own plan after the trace and surfaces plans land.
- **Source filter in the center** and **status-card dots** keyed by a notice's source (needs per-card sources, e.g.
  `routing`, `budget`, `mesh`).
- **App banners recorded in the center** (`BackendBanner`, `VersionMismatchBanner`: SSOT `app` scope).
- **The research popover's hard-coded "Online"** (`StatusBarCluster.tsx`), which says "Online" whether or not the
  status fetch succeeded (SSOT rule 3).
- **De-duplicating a chat turn's failure** between the transcript trace and the center (Decision 7).
- **Daemon/GUI version skew:** an unknown event kind gets no severity and no refresh until both are upgraded.

## Execution Order

- **May run now:** Task 1 (once `git status` is clean for `vox-orchestrator`), then Task 3.
- **After the routing plan's Task 7** (both edit `vox-gui/src/commands/mod.rs`): Task 2.
- **After visual-language Task 1** (it edits `chatTranscriptTimeline.ts` and `App.tsx`): Task 9.
- **After visual-language Task 1 and surfaces Tasks 2 and 5** (`App.tsx`, `AppShell.tsx`): Task 7.
- **After Task 2 and visual-language Task 7** (`ActivitySurface.tsx`): Task 8.
- **After the trace, surfaces and visual-language plans, and the routing plan's GUI tasks:** 4 → 5 → 6, then 11
  (it edits `ModelsView.tsx` after routing Task 9, and `ChatSurface.tsx` after surfaces Task 4 and visual Tasks 4–6),
  then 12 → 13.
- **Shared files:**
  - `App.tsx`: visual 1, surfaces 2/4/5 → Task 7 → Task 4 → Task 6.
  - `AppShell.tsx`: surfaces 2 → Task 7.
  - `commands/mod.rs`: routing 7 → Task 2.
  - `ActivitySurface.tsx`: visual 7 → Task 8.
  - `chatTranscriptTimeline.ts`: visual 1 → Task 9.
  - `types/tauri.ts`: trace 3, routing 7 → Task 4.
  - `notices.ts`: Task 3 → Task 4.
  - `Sidebar.tsx`: Task 7 → Task 11.
  - `ModelsView.tsx`: routing 9 → Task 11. `ChatSurface.tsx`: surfaces 4, visual 4–6 → Task 11.
  - the status bar: surfaces 2, visual 4–6, routing 10 → Task 6.

  All of these are sequential and settled.
  All of these are sequential and settled.
