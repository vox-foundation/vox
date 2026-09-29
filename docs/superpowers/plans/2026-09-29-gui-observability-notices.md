# GUI Observability: Notices, Severity and the Notification Center — Implementation Plan

> **For agentic workers:** Execution is **Claude Code driving Gemini Flash via the `agy` CLI**, one task per headless
> run (`/drive-task docs/superpowers/plans/2026-09-29-gui-observability-notices.md <N>`), per
> [`docs/src/contributors/antigravity-driven-execution.md`](../../src/contributors/antigravity-driven-execution.md).
> The agent never stages or commits; Claude Code re-runs every check, runs the mutation proofs, reviews the diff and
> commits. Tasks 6 and 10 are re-anchored by Claude before they are driven; Task 13 is Claude's. Code blocks are
> transcribed exactly.

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
  - `NotificationCenter.tsx` and a bell render the store.
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
- `pushToast: (t: any)` appears in 12 files, bypassing the required `cause`.

**Prerequisites:** these plans committed first, because they rewrite files this plan edits (`App.tsx`,
`BottomStatusBar.tsx`, `ChatTranscript.tsx`, `ModelsView.tsx`):
- `2026-09-28-chat-turn-trace.md`
- `2026-09-28-chat-surfaces-consolidation.md`
- `2026-09-28-chat-visual-language.md`
- the GUI tasks (7–11) of `2026-09-29-model-routing-self-maintaining.md`

Tasks 1, 2, 3, 7, 8 and 9 touch none of those files and may be driven earlier. The list in Execution Order says
exactly which.

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
- Files over 500 lines (`App.tsx` ~2100, `events.rs` ~1400, `vox-gui/src/commands/orchestrator.rs`, `Sidebar.tsx`)
  get only the edits shown; new code goes in new files.
- Colours only through `var(--color-*)` tokens: no raw hex, no new raw Tailwind palette classes.
- The agent never runs `git add` or `git commit`.

## File Structure

| File | Status | Task | Responsibility |
|---|---|---|---|
| `crates/vox-orchestrator/src/events_severity.rs` | create | 1 | `EventSeverity`, `AgentEventKind::severity` |
| `crates/vox-orchestrator/src/lib.rs` | modify | 1 | `pub mod events_severity;` |
| `crates/vox-gui/src/commands/event_annotate.rs` | create | 2 | `annotate_agent_event`, `ACTIVITY_APPENDED_EVENT` |
| `crates/vox-gui/src/commands/mod.rs`, `orchestrator.rs` | modify | 2 | module; the bridge calls the annotator and emits `activity-appended` |
| `crates/vox-gui/ui/src/lib/notices.ts` + `.test.ts` | create | 3 | notice model, `noticeFromToast`, `noticeFromAgentEvent`, `noticeSurfaces` |
| `crates/vox-gui/ui/src/lib/noticeStore.ts` + `.test.ts` | create | 4 | pure reducer (coalesce, cap, unread) |
| `crates/vox-gui/ui/src/hooks/useNoticeCenter.ts` | create | 4 | hook over the reducer |
| `crates/vox-gui/ui/src/types/tauri.ts`, `components/ui/Toasts.tsx` | modify | 4 | `error` tone |
| `crates/vox-gui/ui/src/App.tsx` | modify | 4 | `pushToast` and the agent-event listener record notices |
| `crates/vox-gui/ui/src/components/ui/Icons.tsx` | modify | 5 | `bell` icon |
| `crates/vox-gui/ui/src/components/common/NotificationCenter.tsx` + `.test.tsx` | create | 5 | bell button and drawer |
| status bar (surfaces plan's cards file) | modify | 6 | mount the bell (re-anchored) |
| `crates/vox-gui/ui/src/hooks/useAttentionInbox.ts` + test | modify | 7 | `degraded` sources |
| `crates/vox-gui/ui/src/components/layout/Sidebar.tsx` + test, `App.tsx` | modify | 7 | degraded Review badge |
| `crates/vox-gui/ui/src/components/surfaces/Activity/ActivitySurface.tsx` + test | modify | 8 | refresh on `activity-appended` only |
| `crates/vox-gui/ui/src/lib/mapAgentEvent.ts`, `chatTranscriptTimeline.ts` + tests | modify | 9 | real phase and cost |
| shared poll hooks (re-anchored) | create/modify | 10 | one poll per fact |
| 12 files with `pushToast: (t: any)` + `lib/typedToasts.test.ts` | modify/create | 11 | typed toasts |
| `crates/vox-gui/ui/e2e/notices.spec.ts` | create | 12 | Playwright + screenshots |

---

### Task 1: The engine classifies every event's severity

**Files:** Create `crates/vox-orchestrator/src/events_severity.rs`; modify `crates/vox-orchestrator/src/lib.rs` (add
`pub mod events_severity;` directly after `pub mod events;`).

- [ ] **Step 1: Write the failing tests** — create `events_severity.rs` with only:

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
  Expected: fails to compile (`EventSeverity`, `severity` missing).
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

- [ ] **Step 1: Write the failing tests** — create `event_annotate.rs` with only:

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
    fn a_frame_that_is_not_an_agent_event_is_left_alone() {
        let mut frame = json!({ "offset": 9, "replay": true });
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

use serde_json::Value;
use vox_orchestrator::AgentEvent;

/// Tauri event emitted when an activity-loggable engine event arrives (the Activity view refreshes on it).
pub const ACTIVITY_APPENDED_EVENT: &str = "vox://activity-appended";

/// Set `severity` on an agent-event frame; return whether the event is activity-loggable. A frame
/// that is not an `AgentEvent` (e.g. a replay envelope) is left unchanged.
pub fn annotate_agent_event(frame: &mut Value) -> bool {
    let Some(obj) = frame.as_object_mut() else {
        return false;
    };
    // Token frames are the high-frequency path: classify without parsing.
    if obj.get("kind").and_then(|k| k.get("type")).and_then(Value::as_str) == Some("token_streamed") {
        obj.insert("severity".into(), Value::from("debug"));
        return false;
    }
    let Ok(event) = serde_json::from_value::<AgentEvent>(Value::Object(obj.clone())) else {
        return false;
    };
    if let Ok(severity) = serde_json::to_value(event.kind.severity()) {
        obj.insert("severity".into(), severity);
    }
    vox_orchestrator::activity::is_loggable(&event.kind)
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
  (c) insert `severity` before parsing (for every frame) → `a_frame_that_is_not_an_agent_event_is_left_alone`.
- [ ] **Step 6: Commit** — the three files; message "feat(gui): agent-event frames carry severity; activity-appended is emitted".

---

### Task 3: The notice model and its routing rule

**Files:** Create `crates/vox-gui/ui/src/lib/notices.ts` and `notices.test.ts`.

- [ ] **Step 1: Write the failing test** — `notices.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { noticeFromToast, noticeFromAgentEvent, noticeSurfaces } from './notices';

describe('notices', () => {
  it('a toast becomes an action notice with a matching severity', () => {
    expect(noticeFromToast({ tone: 'ok', title: 'Saved', cause: 'backend-ok' }, 10)).toMatchObject(
      { severity: 'success', scope: 'action', source: 'backend-ok', title: 'Saved', atMs: 10 });
    expect(noticeFromToast({ tone: 'warn', title: 'x', cause: 'backend-error' }, 1).severity).toBe('warning');
    expect(noticeFromToast({ tone: 'error', title: 'x', cause: 'backend-error' }, 1).severity).toBe('error');
    expect(noticeFromToast({ tone: 'info', title: 'x', cause: 'external' }, 1).severity).toBe('info');
  });

  it('only engine warnings and errors become notices', () => {
    const frame = (type: string, severity?: string) =>
      ({ id: 1, timestamp_ms: 5, severity, kind: { type, task_id: 7 } });
    expect(noticeFromAgentEvent(frame('task_failed', 'error'))).toMatchObject(
      { severity: 'error', scope: 'engine', source: 'engine', groupKey: 'engine:task_failed', atMs: 5 });
    expect(noticeFromAgentEvent(frame('tool_timed_out', 'warning'))?.severity).toBe('warning');
    expect(noticeFromAgentEvent(frame('task_completed', 'info'))).toBeNull();
    expect(noticeFromAgentEvent(frame('token_streamed', 'debug'))).toBeNull();
    expect(noticeFromAgentEvent(frame('task_failed'))).toBeNull();
  });

  it('routes by scope and severity as the observability SSOT says', () => {
    expect(noticeSurfaces({ severity: 'success', scope: 'action', source: 's', title: 't' }))
      .toEqual({ toast: true, center: true, badge: false });
    expect(noticeSurfaces({ severity: 'error', scope: 'engine', source: 's', title: 't' }))
      .toEqual({ toast: false, center: true, badge: true });
    expect(noticeSurfaces({ severity: 'warning', scope: 'turn', source: 's', title: 't' }))
      .toEqual({ toast: false, center: false, badge: false });
  });
});
```

- [ ] **Step 2: Run to verify failure** — `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/notices.test.ts > target/obs-t3-red.txt 2>&1; tail -12 target/obs-t3-red.txt`.
  Expected: fails (module missing). Also expect a type error on `tone: 'error'`; Task 4 adds that tone, and until then
  the test file carries `// @ts-expect-error` on that one line. Remove that comment in Task 4.
- [ ] **Step 3: Implement** — `notices.ts`:

```ts
import type { Toast } from '../types/tauri';
import type { AgentEventFrame } from './chatCorrelation';
import { mapAgentEvent } from './mapAgentEvent';

/** docs/src/architecture/gui-observability-ssot-2026.md — every message the GUI shows is a notice. */
export type NoticeSeverity = 'success' | 'info' | 'warning' | 'error';
export type NoticeScope = 'action' | 'turn' | 'session' | 'engine' | 'app';
export type NoticeNeeds = 'none' | 'review' | 'decision';

export interface NoticeInput {
  severity: NoticeSeverity;
  scope: NoticeScope;
  needs?: NoticeNeeds;
  /** Producer id (a toast's cause, `engine`, `routing`, …). */
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
  error: 'error',
};

export function noticeFromToast(t: Toast, atMs: number = Date.now()): NoticeInput {
  return {
    severity: TONE_SEVERITY[t.tone],
    scope: 'action',
    source: t.cause,
    title: t.title,
    body: t.body,
    cmd: t.cmd,
    groupKey: t.groupKey,
    atMs,
  };
}

/** Engine warnings and errors only; everything else stays in the trace and the activity log. */
export function noticeFromAgentEvent(frame: SeverityFrame): NoticeInput | null {
  if (frame.severity !== 'warning' && frame.severity !== 'error') return null;
  const item = mapAgentEvent(frame);
  return {
    severity: frame.severity,
    scope: 'engine',
    source: 'engine',
    title: item.title,
    body: item.body || undefined,
    groupKey: `engine:${frame.kind?.type ?? 'unknown'}`,
    atMs: frame.timestamp_ms || Date.now(),
  };
}

/** Where a notice may appear. Turn and session notices belong to the chat trace and rail, not here. */
export function noticeSurfaces(n: NoticeInput): { toast: boolean; center: boolean; badge: boolean } {
  const recorded = n.scope === 'action' || n.scope === 'engine' || n.scope === 'app';
  return {
    toast: n.scope === 'action',
    center: recorded,
    badge: recorded && (n.severity === 'warning' || n.severity === 'error'),
  };
}
```

- [ ] **Step 4: Run to verify pass** — the Step 2 command plus `timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -3`.
  The `TONE_SEVERITY` record fails typecheck until Task 4 adds `'error'` to `Toast['tone']`, so type it as
  `Record<string, NoticeSeverity>` in this task and say so. Task 4 narrows it back.
- [ ] **Step 5 (Claude): mutation proofs** — (a) `frame.severity !== 'warning' && …` → `false` (always a notice) →
  the engine test; (b) `toast: n.scope === 'action'` → `true` → the routing test.
- [ ] **Step 6: Commit** — `notices.ts`, `notices.test.ts`; message "feat(gui): a notice model with one routing rule".

---

### Task 4: A notification store that remembers every toast and engine problem

**Files:**
- Create `crates/vox-gui/ui/src/lib/noticeStore.ts`, `noticeStore.test.ts` and `hooks/useNoticeCenter.ts`.
- Modify `types/tauri.ts`: `Toast.tone` gains `'error'`.
- Modify `components/ui/Toasts.tsx`: the `error` tone class and icon.
- Modify `lib/notices.ts`: narrow `TONE_SEVERITY` and remove the Task 3 `@ts-expect-error`.
- Modify `App.tsx`: `pushToast` and the `listenAgentEvents` effect only.

- [ ] **Step 1: Write the failing test** — `noticeStore.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { noticeReducer, EMPTY_NOTICES, MAX_NOTICES, COALESCE_WINDOW_MS, unreadProblems } from './noticeStore';

const warn = (title: string, atMs: number) =>
  ({ type: 'record' as const, input: { severity: 'warning' as const, scope: 'engine' as const, source: 'engine', title, atMs } });

describe('noticeReducer', () => {
  it('records newest first and counts unread problems', () => {
    let s = noticeReducer(EMPTY_NOTICES, warn('a', 1));
    s = noticeReducer(s, { type: 'record', input: { severity: 'success', scope: 'action', source: 'backend-ok', title: 'ok', atMs: 2 } });
    expect(s.notices.map(n => n.title)).toEqual(['ok', 'a']);
    expect(unreadProblems(s)).toBe(1);
  });

  it('coalesces a repeat inside the window into one notice with a count', () => {
    let s = noticeReducer(EMPTY_NOTICES, warn('Tool timed out', 1_000));
    s = noticeReducer(s, warn('Tool timed out', 1_000 + COALESCE_WINDOW_MS - 1));
    expect(s.notices).toHaveLength(1);
    expect(s.notices[0].count).toBe(2);
    s = noticeReducer(s, warn('Tool timed out', 1_000 + 3 * COALESCE_WINDOW_MS));
    expect(s.notices).toHaveLength(2);
  });

  it('a repeat after reading brings the notice back as unread', () => {
    let s = noticeReducer(EMPTY_NOTICES, warn('x', 1));
    s = noticeReducer(s, { type: 'markAllRead' });
    expect(unreadProblems(s)).toBe(0);
    s = noticeReducer(s, warn('x', 2));
    expect(unreadProblems(s)).toBe(1);
  });

  it('keeps at most MAX_NOTICES, dropping the oldest', () => {
    let s = EMPTY_NOTICES;
    for (let i = 0; i < MAX_NOTICES + 5; i++) s = noticeReducer(s, warn(`n${i}`, i * 10 * COALESCE_WINDOW_MS));
    expect(s.notices).toHaveLength(MAX_NOTICES);
    expect(s.notices[s.notices.length - 1].title).toBe('n5');
  });

  it('never records turn or session notices', () => {
    const s = noticeReducer(EMPTY_NOTICES, { type: 'record', input: { severity: 'error', scope: 'turn', source: 'chat', title: 'x', atMs: 1 } });
    expect(s.notices).toHaveLength(0);
  });
});
```

- [ ] **Step 2: Run to verify failure** — `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/noticeStore.test.ts > target/obs-t4-red.txt 2>&1; tail -12 target/obs-t4-red.txt`.
- [ ] **Step 3: Implement** — `noticeStore.ts`:

```ts
import { noticeSurfaces, type NoticeInput } from './notices';

export const MAX_NOTICES = 200;
/** A repeat of the same notice within this window merges into it (×N). */
export const COALESCE_WINDOW_MS = 60_000;

export interface Notice extends Required<Pick<NoticeInput, 'severity' | 'scope' | 'source' | 'title'>> {
  id: string;
  groupKey: string;
  needs: NonNullable<NoticeInput['needs']>;
  body?: string;
  cmd?: string;
  count: number;
  firstAtMs: number;
  lastAtMs: number;
  read: boolean;
}

export interface NoticeState {
  notices: Notice[];
  seq: number;
}

export const EMPTY_NOTICES: NoticeState = { notices: [], seq: 0 };

export type NoticeAction = { type: 'record'; input: NoticeInput } | { type: 'markAllRead' } | { type: 'clear' };

export function noticeReducer(state: NoticeState, action: NoticeAction): NoticeState {
  switch (action.type) {
    case 'markAllRead':
      return { ...state, notices: state.notices.map(n => (n.read ? n : { ...n, read: true })) };
    case 'clear':
      return { ...state, notices: [] };
    case 'record': {
      const input = action.input;
      if (!noticeSurfaces(input).center) return state;
      const atMs = input.atMs ?? Date.now();
      const groupKey = input.groupKey ?? `${input.source}:${input.title}`;
      const hit = state.notices.findIndex(n => n.groupKey === groupKey && atMs - n.lastAtMs < COALESCE_WINDOW_MS);
      if (hit !== -1) {
        const prev = state.notices[hit];
        const merged: Notice = {
          ...prev,
          severity: input.severity,
          body: input.body,
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
        needs: input.needs ?? 'none',
        source: input.source,
        title: input.title,
        body: input.body,
        cmd: input.cmd,
        count: 1,
        firstAtMs: atMs,
        lastAtMs: atMs,
        read: false,
      };
      return { seq: state.seq + 1, notices: [notice, ...state.notices].slice(0, MAX_NOTICES) };
    }
  }
}

/** Unread warnings and errors: the number the bell shows. */
export function unreadProblems(state: NoticeState): number {
  return state.notices.filter(n => !n.read && (n.severity === 'warning' || n.severity === 'error')).length;
}
```

`hooks/useNoticeCenter.ts`:

```ts
import { useCallback, useReducer } from 'react';
import { EMPTY_NOTICES, noticeReducer, unreadProblems, type Notice } from '../lib/noticeStore';
import type { NoticeInput } from '../lib/notices';

export interface NoticeCenter {
  notices: Notice[];
  unread: number;
  record(input: NoticeInput): void;
  markAllRead(): void;
  clear(): void;
}

/** The app's notice store. `record` is stable, so listeners can capture it once. */
export function useNoticeCenter(): NoticeCenter {
  const [state, dispatch] = useReducer(noticeReducer, EMPTY_NOTICES);
  const record = useCallback((input: NoticeInput) => dispatch({ type: 'record', input }), []);
  const markAllRead = useCallback(() => dispatch({ type: 'markAllRead' }), []);
  const clear = useCallback(() => dispatch({ type: 'clear' }), []);
  return { notices: state.notices, unread: unreadProblems(state), record, markAllRead, clear };
}
```

The remaining edits:
  - `types/tauri.ts`: `tone: 'ok' | 'warn' | 'info';` becomes `tone: 'ok' | 'warn' | 'info' | 'error';`.
  - `Toasts.tsx`:
    - `ToastItem.tone` gets the same union.
    - `TONE_ICON_CLASS` gains `error: 'bg-(--color-status-fail)/15 text-(--color-status-fail)',`.
    - The icon ternary becomes
      `t.tone === "ok" ? <Icon.check …/> : t.tone === "warn" || t.tone === "error" ? <Icon.alert …/> : <Icon.bolt …/>`.
  - `notices.ts`: type `TONE_SEVERITY` back to `Record<Toast['tone'], NoticeSeverity>`, and delete the
    `// @ts-expect-error` line in `notices.test.ts`.
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
  removed → the cap test; (d) delete `if (!noticeSurfaces(input).center) return state;` → the turn-notice test.
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
import { NotificationBell, NotificationCenter } from './NotificationCenter';
import type { Notice } from '../../lib/noticeStore';

const n = (id: string, severity: Notice['severity'], title: string, extra: Partial<Notice> = {}): Notice => ({
  id, groupKey: id, severity, scope: 'engine', needs: 'none', source: 'engine', title,
  count: 1, firstAtMs: 0, lastAtMs: 0, read: false, ...extra,
});

describe('NotificationBell', () => {
  it('names the number of problems and controls the drawer', () => {
    const onToggle = vi.fn();
    render(<NotificationBell unread={3} open={false} onToggle={onToggle} />);
    const bell = screen.getByRole('button', { name: 'Notifications, 3 need attention' });
    expect(bell.getAttribute('aria-expanded')).toBe('false');
    expect(bell.getAttribute('aria-controls')).toBe('notification-center');
    fireEvent.click(bell);
    expect(onToggle).toHaveBeenCalled();
  });

  it('shows no count when nothing needs attention', () => {
    render(<NotificationBell unread={0} open={false} onToggle={vi.fn()} />);
    expect(screen.getByRole('button', { name: 'Notifications' })).toBeTruthy();
    expect(screen.queryByTestId('notification-unread')).toBeNull();
  });
});

describe('NotificationCenter', () => {
  const notices = [
    n('1', 'error', 'FAILED · task 7', { count: 2, body: 'error: boom' }),
    n('2', 'success', 'Saved', { scope: 'action', source: 'backend-ok', read: true }),
  ];

  it('lists notices newest first with counts, and filters to problems', () => {
    render(<NotificationCenter notices={notices} open onClose={vi.fn()} onMarkAllRead={vi.fn()} />);
    const dialog = screen.getByRole('dialog', { name: 'Notifications' });
    const items = within(dialog).getAllByRole('listitem');
    expect(items).toHaveLength(2);
    expect(items[0].getAttribute('data-severity')).toBe('error');
    expect(within(items[0]).getByText('×2')).toBeTruthy();
    fireEvent.click(within(dialog).getByRole('radio', { name: 'Problems' }));
    expect(within(dialog).getAllByRole('listitem')).toHaveLength(1);
  });

  it('marks all read and closes on Escape', () => {
    const onMarkAllRead = vi.fn();
    const onClose = vi.fn();
    render(<NotificationCenter notices={notices} open onClose={onClose} onMarkAllRead={onMarkAllRead} />);
    fireEvent.click(screen.getByRole('button', { name: 'Mark all read' }));
    expect(onMarkAllRead).toHaveBeenCalled();
    fireEvent.keyDown(screen.getByRole('dialog', { name: 'Notifications' }), { key: 'Escape' });
    expect(onClose).toHaveBeenCalled();
  });

  it('says so when there is nothing to report, and renders nothing when closed', () => {
    const { rerender } = render(<NotificationCenter notices={[]} open onClose={vi.fn()} onMarkAllRead={vi.fn()} />);
    expect(screen.getByText('Nothing to report')).toBeTruthy();
    rerender(<NotificationCenter notices={notices} open={false} onClose={vi.fn()} onMarkAllRead={vi.fn()} />);
    expect(screen.queryByRole('dialog')).toBeNull();
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
import React, { useState } from 'react';
import { Icon } from '../ui/Icons';
import type { Notice } from '../../lib/noticeStore';

const SEVERITY_COLOR: Record<Notice['severity'], string> = {
  error: 'var(--color-status-fail)',
  warning: 'var(--color-status-warn)',
  success: 'var(--color-status-pass)',
  info: 'var(--color-status-info)',
};

export function NotificationBell({ unread, open, onToggle }: { unread: number; open: boolean; onToggle(): void }) {
  return (
    <button
      type="button"
      onClick={onToggle}
      aria-expanded={open}
      aria-controls="notification-center"
      aria-label={unread > 0 ? `Notifications, ${unread} need attention` : 'Notifications'}
      className="relative flex items-center gap-1 px-2 text-text-muted hover:text-text-primary"
    >
      <Icon.bell className="size-3.5" aria-hidden="true" />
      {unread > 0 && (
        <span data-testid="notification-unread" className="font-mono text-[10px] tabular-nums"
          style={{ color: 'var(--color-status-warn)' }}>
          {unread}
        </span>
      )}
    </button>
  );
}

interface CenterProps {
  notices: Notice[];
  open: boolean;
  onClose(): void;
  onMarkAllRead(): void;
}

export function NotificationCenter({ notices, open, onClose, onMarkAllRead }: CenterProps) {
  const [filter, setFilter] = useState<'all' | 'problems'>('all');
  if (!open) return null;
  const shown = filter === 'all' ? notices : notices.filter(n => n.severity === 'warning' || n.severity === 'error');
  return (
    <section
      id="notification-center"
      role="dialog"
      aria-label="Notifications"
      tabIndex={-1}
      onKeyDown={e => { if (e.key === 'Escape') onClose(); }}
      className="fixed bottom-10 right-4 z-50 flex max-h-[60vh] w-[360px] flex-col rounded-xl border border-border-subtle bg-bg-base p-3 text-xs"
    >
      <header className="mb-2 flex items-center gap-2">
        <span className="flex-1 text-text-secondary">Notifications</span>
        <span role="radiogroup" aria-label="Show" className="flex gap-1">
          {(['all', 'problems'] as const).map(v => (
            <label key={v} className="flex items-center gap-1 text-text-muted">
              <input type="radio" name="notice-filter" checked={filter === v} onChange={() => setFilter(v)}
                aria-label={v === 'all' ? 'All' : 'Problems'} />
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
        <ul className="flex flex-col gap-1 overflow-y-auto">
          {shown.map(n => (
            <li key={n.id} data-severity={n.severity} data-read={n.read ? 'true' : undefined}
              className="rounded-lg border-l-2 px-2 py-1"
              style={{ borderColor: SEVERITY_COLOR[n.severity] }}>
              <div className="flex items-baseline gap-1">
                <span className={n.read ? 'text-text-muted' : 'text-text-primary'}>{n.title}</span>
                {n.count > 1 && <span className="font-mono text-[10px] text-text-muted">×{n.count}</span>}
                <span className="ml-auto text-[10px] text-text-muted">{n.source}</span>
              </div>
              {n.body && <div className="text-text-muted">{n.body}</div>}
              {n.cmd && <div className="font-mono text-[10px] text-text-muted">▸ {n.cmd}</div>}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
```

- [ ] **Step 4: Run to verify pass** — Step 2's command plus `typecheck`.
- [ ] **Step 5 (Claude): mutation proofs** — (a) the Problems filter returns `notices` → the filter assertion;
  (b) `aria-label` always `'Notifications'` → the first bell test; (c) delete the Escape handler → the Escape test.
- [ ] **Step 6: Commit** — the three files; message "feat(gui): a notification center that keeps what toasts drop".

---

### Task 6: Mount the bell and center in the status bar (re-anchor before driving)

**Owner of the re-anchor: Claude.** The status bar is rewritten by the surfaces plan's Task 2. After that lands, Claude
reads it and rewrites this task with exact code. The decided behaviour:
- **Bell placement:** the bell is the last status-bar item. Its drawer opens above it.
- **State:** `App.tsx` owns `noticeCenter` (Task 4) and an `open` state.
  - Opening the drawer does **not** mark notices read; "Mark all read" does.
  - Closing the drawer returns focus to the bell.
- **Status cards:** a card whose fact has an unread `engine` problem (for example, Routing with a routing-health
  violation) shows its status dot. Clicking the dot opens the center, filtered to that source.
- **Tests:** a vitest test in the status-bar test file checks that the bell shows the store's unread count. Task 12's
  Playwright spec covers the rest.

---

### Task 7: A failed source reads as degraded, not as all-clear

**Files:**
- Modify `crates/vox-gui/ui/src/hooks/useAttentionInbox.ts` and `useAttentionInbox.test.ts` (new tests appended).
- Modify `crates/vox-gui/ui/src/components/layout/Sidebar.tsx`: the `needsYouCount` prop area and the `runs` badge
  expression.
- Modify `Sidebar.test.tsx` (new test appended).
- Modify `App.tsx`: one prop at the `Sidebar` call.

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
});
```

Append to `Sidebar.test.tsx` (mirror how its existing tests render `Sidebar`, adding `needsYouCount={0}` and
`needsYouDegraded={['approvals']}`):

```tsx
it('shows a degraded Review badge instead of all-clear when a source failed', () => {
  // render <Sidebar … needsYouCount={0} needsYouDegraded={['approvals']} /> as the other tests in this file do
  const review = screen.getByRole('button', { name: "Review, couldn't load approvals" });
  expect(review.textContent).toContain('!');
});
```

(If the existing `Sidebar.test.tsx` renders through a helper, call that helper with the two extra props. The test
body above is the contract.)

- [ ] **Step 2: Run to verify failure** — `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/hooks/useAttentionInbox.test.ts src/components/layout/Sidebar.test.tsx > target/obs-t7-red.txt 2>&1; tail -15 target/obs-t7-red.txt`.
- [ ] **Step 3: Implement.** In `useAttentionInbox.ts`:
  1. Add `degraded: string[];` to `AttentionInbox`, with the doc comment "Sources whose last fetch failed
     (`approvals`, `feedback`, `tasks`); their lists are empty because they are unknown, not because they are clear."
  2. Add `const [degraded, setDegraded] = useState<string[]>([]);`.
  3. Replace the body of `refresh` with:

```ts
    const FAILED = Symbol('failed');
    const emptyFeedback = { needsYou: [] as FeedbackRow[], withheld: [] as FeedbackRow[] };
    const [approvalRes, feedback, tasks] = await Promise.all([
      Promise.resolve(voxTransport.invokeMcpTool('vox_pending_approvals', {})).catch(() => FAILED),
      Promise.resolve(feedbackList()).catch(() => FAILED),
      Promise.resolve(hopperList()).catch(() => FAILED),
    ]);
    setDegraded([
      approvalRes === FAILED ? 'approvals' : null,
      feedback === FAILED ? 'feedback' : null,
      tasks === FAILED ? 'tasks' : null,
    ].filter((s): s is string => s !== null));
    const safeFeedback = feedback === FAILED || !feedback ? emptyFeedback : feedback;
    const safeTasks = tasks === FAILED || !tasks ? [] : tasks;
    setApprovals(approvalRes && approvalRes !== FAILED ? parsePendingApprovals(approvalRes as McpInvokeResult) : []);
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
            const reviewDegraded = key === 'runs' && (needsYouCount ?? 0) === 0 && (needsYouDegraded?.length ?? 0) > 0;
            const badge =
              key === 'agents' ? agentsCount
              : key === 'runs' && needsYouCount != null && needsYouCount > 0 ? needsYouCount
              : reviewDegraded ? '!'
              : undefined;
            const navAriaLabel =
              key === 'runs'
                ? needsYouCount != null && needsYouCount > 0
                  ? `Review, ${needsYouCount} items need you`
                  : reviewDegraded
                    ? `Review, couldn't load ${needsYouDegraded!.join(', ')}`
                    : 'Review'
                : undefined;
```

  If `NavItemProps.badge` is typed `number`, widen it to `number | string` and say so. In `App.tsx`, at the `Sidebar`
  call, add `needsYouDegraded={attention.degraded}` after `needsYouCount={attention.totalCount}`.
- [ ] **Step 4: Run to verify pass** — the Step 2 command, then `typecheck` and the full `vitest run`.
- [ ] **Step 5 (Claude): mutation proofs** — (a) `.catch(() => FAILED)` on `hopperList` → `.catch(() => [])` → the
  degraded test; (b) `reviewDegraded ? '!'` removed → the Sidebar test.
- [ ] **Step 6: Commit** — the files; message "fix(gui): a failed inbox source reads as degraded, not as all-clear".

---

### Task 8: The Activity view refreshes when a row is written, not on every token

**Precondition:** Task 2 committed (the bridge emits `vox://activity-appended`).

**Files:** Modify `crates/vox-gui/ui/src/components/surfaces/Activity/ActivitySurface.tsx`: delete the
`listenAgentEvents` effect and its import. Modify `ActivitySurface.test.tsx` (new test appended; add the mocks at the
top).

- [ ] **Step 1: Write the failing test.** Add at the top of `ActivitySurface.test.tsx`, after the imports:

```tsx
import { vi } from 'vitest';
import { waitFor } from '@testing-library/react';

const listeners: { appended?: () => void; agent?: () => void } = {};
const activityQueryMock = vi.fn(() => Promise.resolve([]));
vi.mock('../../../transport', async (orig) => ({
  ...(await orig<typeof import('../../../transport')>()),
  activityQuery: (...a: unknown[]) => activityQueryMock(...(a as [])),
  listenActivityAppended: vi.fn((cb: () => void) => { listeners.appended = cb; return Promise.resolve(() => {}); }),
  listenAgentEvents: vi.fn((cb: () => void) => { listeners.agent = cb; return Promise.resolve(() => {}); }),
}));
```

and append:

```tsx
import { ActivitySurface } from './ActivitySurface';

describe('ActivitySurface refresh', () => {
  it('re-queries on activity-appended and ignores other engine events', async () => {
    render(<ActivitySurface pushToast={vi.fn()} />);
    await waitFor(() => expect(activityQueryMock).toHaveBeenCalledTimes(1));
    listeners.agent?.();
    listeners.agent?.();
    await new Promise(r => setTimeout(r, 20));
    expect(activityQueryMock).toHaveBeenCalledTimes(1);
    listeners.appended?.();
    await waitFor(() => expect(activityQueryMock).toHaveBeenCalledTimes(2));
  });
});
```

(Move the `ActivitySurface` import up with the others if the linter requires it.)

- [ ] **Step 2: Run to verify failure** — `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Activity/ActivitySurface.test.tsx > target/obs-t8-red.txt 2>&1; tail -12 target/obs-t8-red.txt`.
  Expected: the call count is 3, not 1, after the two agent events.
- [ ] **Step 3: Implement** — delete the effect whose comment begins "Also refresh on "vox://agent-events"", and the
  now-unused `listenAgentEvents` import. Update the remaining effect's comment to say that the bridge emits
  `vox://activity-appended` for every activity-loggable event (`crates/vox-gui/src/commands/event_annotate.rs`).
- [ ] **Step 4: Run to verify pass** — the Step 2 command plus `typecheck`.
- [ ] **Step 5 (Claude): mutation proof** — restore the deleted effect → the refresh test fails.
- [ ] **Step 6: Commit** — the two files; message "fix(gui): the activity view refreshes on new rows, not on every token".

---

### Task 9: The chat status line shows the real phase and the done row shows the real cost

**Files:** Modify `crates/vox-gui/ui/src/lib/mapAgentEvent.ts` (the `metadata` object) and
`lib/chatTranscriptTimeline.ts` (inside `buildChatOnlyTimeline`'s loop). Append a test to
`lib/chatTranscriptTimeline.test.ts`.

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
      frame(2, 'task_phase_changed', { task_id: 7, agent_id: 3, phase: 'Planning' }),
    ], { nowMs: 5000 });
    expect(rows.find(r => r.kind === 'status')).toMatchObject({ taskId: 7, phase: 'Planning' });
  });

  it('attributes an agent cost to that agent task and shows it when the task completes', () => {
    const rows = buildChatOnlyTimeline([], [
      frame(1, 'task_started', { task_id: 7, agent_id: 3 }),
      frame(2, 'cost_incurred', { agent_id: 3, cost_usd: 0.02, provider: 'acme', model: 'acme/widget-5.5' }),
      frame(3, 'task_completed', { task_id: 7, agent_id: 3 }),
    ], { nowMs: 5000 });
    expect(rows.find(r => r.kind === 'summary')).toMatchObject({ taskId: 7, costUsd: 0.02 });
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

- [ ] **Step 4: Run to verify pass** — the Step 2 command plus `src/lib/mapAgentEvent.test.ts` and `typecheck`.
- [ ] **Step 5 (Claude): mutation proofs** — (a) drop the `phase` spread → the phase test; (b) drop the cost
  attribution line → the cost test.
- [ ] **Step 6: Commit** — the three files; message "fix(gui): the chat status line shows the engine's phase and cost".

---

### Task 10: One poll per fact (re-anchor before driving)

**Owner of the re-anchor: Claude**, after the trace and surfaces plans land. They change `ChatTranscript.tsx` (the
harness strip) and the status bar. The decided behaviour:
- **Harness issues:** polled once, by the App-level poll that already exists. The result goes into a small React
  context, `HarnessIssuesContext`. `ChatTranscript` and `HarnessIssuesPanel` read the context instead of polling.
  The panel's failure toast goes, because a failed source is shown as degraded (Task 7's pattern).
- **Pending approvals:** `useAttentionInbox` is the only poller. `useAgentApprovals` (Dashboard) and `ApprovalsView`
  read it through props or context.
- **Mesh:** one source (`useMeshNodes`), already decided in the surfaces plan.
- **Proof:** a vitest test counts transport calls over one poll interval with all consumers mounted. Each fact is
  fetched once.

---

### Task 11: Toasts are typed everywhere, so every toast carries an honest cause

**Files:** each file below, where `pushToast: (t: any) => void` (or the inline `{ pushToast: (t: any) => void }`)
becomes `(t: Toast) => void` with `import type { Toast } from '<relative>/types/tauri';`:
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

Create `crates/vox-gui/ui/src/lib/typedToasts.test.ts`.

- [ ] **Step 1: Write the failing test** — `typedToasts.test.ts`:

```ts
import { describe, it, expect } from 'vitest';

const sources = import.meta.glob('/src/**/*.{ts,tsx}', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;

describe('typed toasts', () => {
  it('no component takes an untyped pushToast', () => {
    const offenders = Object.entries(sources)
      .filter(([path]) => !path.endsWith('.test.ts') && !path.endsWith('.test.tsx'))
      .filter(([, text]) => /pushToast\s*:\s*\(\s*t\s*:\s*any\s*\)/.test(text))
      .map(([path]) => path);
    expect(offenders).toEqual([]);
  });
});
```

- [ ] **Step 2: Run to verify failure** — `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/typedToasts.test.ts > target/obs-t11-red.txt 2>&1; tail -12 target/obs-t11-red.txt`.
  Expected: 11 offenders.
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

### Task 12: Playwright — notices end to end

**Files:** Create `crates/vox-gui/ui/e2e/notices.spec.ts`. Use `installTauriMock` plus the `installTrustOverrides`
override pattern from `e2e/chat-trust-chips.spec.ts`. Drive events with `window.__TAURI_EMIT__(event, payload)`
(`e2e/lib/tauriMockShared.ts`).

Tests, each saving a screenshot to `review-bundle/latest/` at 1440×900:
1. **An engine error appears in the center, not as a toast or in the chat** → `notices-engine-error.png`.
   - Emit `vox://agent-events` with
     `{ id: 1, timestamp_ms: 1, severity: 'error', kind: { type: 'task_failed', task_id: 7, error: 'boom' } }`.
   - The bell reads "Notifications, 1 need attention".
   - There is no new toast (`role=status` has no new child).
   - The chat transcript text does not contain "boom".
   - Opening the bell shows `data-severity="error"`.
2. **Repeated engine warnings coalesce** → `notices-coalesced.png`. Emit three identical `tool_timed_out` warnings;
   the center shows one item with `×3`.
3. **A toast survives in the center after it expires.** Trigger a failing action that toasts, wait more than 5 s,
   open the center; the item is there.
4. **A failed inbox source shows a degraded Review badge** → `notices-degraded-review.png`. Override
   `vox_pending_approvals` to reject; the Review nav item is named "Review, couldn't load approvals".
5. **The Activity view refreshes only on `activity-appended`.** Count `activity_query` invokes: emitting
   `vox://agent-events` does not add a call; emitting `vox://activity-appended` does.
6. **The bell at narrow width** → `notices-narrow.png`, at 390×844: the drawer fits and the page has no horizontal
   scroll.

- [ ] Run: `timeout 300s pnpm --dir crates/vox-gui/ui exec playwright test e2e/notices.spec.ts --project=chromium --reporter=line`.
  Expected: 6 passed.
- [ ] (Claude) Open each screenshot and check it against the SSOT's rules.
- [ ] Commit the spec: "test(gui): visual verification of notices, severity and degraded states".

---

### Task 13: Verification sweep — Owner: Claude

- [ ] `cargo test -p vox-orchestrator --lib events_severity`
- [ ] `cargo test -p vox-gui --bin vox-gui event_annotate`
- [ ] clippy on both crates
- [ ] the full `vitest run`, `typecheck`, and the notices, models and chat-trust Playwright specs
- [ ] A live check with the real daemon:
  - start the app, submit a task that fails;
  - the failure appears in the center with severity `error`;
  - the Activity view gains a row without re-querying per token (watch `activity_query` calls in the devtools
    network log).
- [ ] `where-things-live.md` rows:
  - Notice model and routing: `lib/notices.ts`
  - Notification center: `components/common/NotificationCenter.tsx`, `hooks/useNoticeCenter.ts`
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
6. **No new dependency.** No OS notifications and no query library.

## Deferred

- **Diagnostics in the GUI:** `vox doctor` checks, including routing health, as an Engine-card popover section.
- **OS notifications** for `needs: decision` while the window is unfocused (`tauri-plugin-notification`).
- **Migrating the ~145 `tone: 'warn', cause: 'backend-error'` toasts to `tone: 'error'`**, per surface, alongside a
  review of which should become center-only.
- **Routing-health violations as engine notices:** they are computed in the orchestrator's refresh, not emitted as
  events. Add an `AgentEventKind::RoutingHealthChanged` event when the routing plan's Task 10 lands.
- **The main toast stack's z-order** (`z-40`) against the onboarding overlay (`z-70`).
- **Severity on durable activity rows** (a schema change).

## Execution Order

- **May run before the chat plans** (they touch none of those plans' files):
  - Task 1 → 2 (Rust; sequential; not concurrent with other Rust plans).
  - Task 3.
  - Task 7 (`useAttentionInbox`, `Sidebar`, one `App.tsx` prop). Also check that no in-flight plan edits `Sidebar.tsx`.
  - Task 8, after 2.
  - Task 9.
- **After the trace, surfaces and visual-language plans, and the routing plan's GUI tasks:**
  - 4 → 5 → 6 (`App.tsx`, the status bar).
  - 10, re-anchored.
  - 11, which edits `ModelsView.tsx` after routing Task 9.
  - 12 → 13.
- **Shared files:**
  - `App.tsx`: Task 7 (early) → Task 4 → Task 6.
  - `notices.ts`: Task 3 → Task 4.
  - `Sidebar.tsx`: Task 7 → Task 11.
  - `ModelsView.tsx`: routing plan Task 9 → Task 11.

  All of these are sequential and settled.
