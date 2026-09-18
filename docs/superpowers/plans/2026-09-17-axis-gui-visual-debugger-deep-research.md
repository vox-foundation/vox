# Axis GUI Visual Debugger and Deep Research Inspection Lab Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a generalized GUI visual debugger and diagnostic invariant sentry system in Axis, wire it into a dedicated Deep Research Lab surface with in-flight breakpoints, a live search provider prober, epistemic judge breakdown, and zero-hits halting, backed by an automated Playwright multi-step screenshot harness and codebase-wide visual verification policy.

**Architecture:** A reusable React 19 + TypeScript debugger engine in `crates/vox-gui/ui/src/debugger/` (FSM stepper, global inspector drawer, DOM invariant sentries) communicates via Tauri IPC (`pipeline_step_*`, `gui_capture_snapshot`, `probe_search_provider`) with the Rust backend. The Deep Research Lab registers its 5 pipeline stages into the stepper and enforces a zero-hits hard gate against hallucination. Automated Playwright specs capture full-resolution screenshots at every breakpoint to verify end-to-end evidence.

**Tech Stack:** TypeScript, React 19, Tailwind CSS, Playwright, Vitest, Rust (Tauri 2, tokio, serde), `vox-search`, `vox-research-shim`.

**Spec:** [`docs/superpowers/specs/2026-09-17-axis-gui-visual-debugger-deep-research-design.md`](file:///Users/brbrainerd/dev/vox/docs/superpowers/specs/2026-09-17-axis-gui-visual-debugger-deep-research-design.md)

## Global Constraints

- Never use `cargo fmt --all` on this workspace (Windows argument limit overflow); format dirty files with `cargo fmt -p vox-gui` / `cargo fmt -p vox-research-shim`.
- All automation scripts must be `.vox` or native commands, never new `.ps1`, `.sh`, or `.py`.
- No raw `__TAURI_INTERNALS__` or unhandled promise rejections may leak to DOM nodes or toasts.
- All errors displayed to the user must be sanitized with actionable remediation advice.
- No pipeline stage may claim `completed` if primary output collections are empty without explicit user warning.

---

### Task 1: Diagnostic Invariant Sentries (`Honesty`, `ErrorLeak`, `Occlusion`, `Watchdog`)

**Files:**
- Create: `crates/vox-gui/ui/src/debugger/types.ts`
- Create: `crates/vox-gui/ui/src/debugger/sentries/sentryHonesty.ts`
- Create: `crates/vox-gui/ui/src/debugger/sentries/sentryHonesty.test.ts`
- Create: `crates/vox-gui/ui/src/debugger/sentries/sentryErrorLeak.ts`
- Create: `crates/vox-gui/ui/src/debugger/sentries/sentryErrorLeak.test.ts`
- Create: `crates/vox-gui/ui/src/debugger/sentries/sentryOcclusion.ts`
- Create: `crates/vox-gui/ui/src/debugger/sentries/sentryOcclusion.test.ts`
- Create: `crates/vox-gui/ui/src/debugger/sentries/sentryWatchdog.ts`
- Create: `crates/vox-gui/ui/src/debugger/sentries/sentryWatchdog.test.ts`

**Interfaces:**
- Produces: `InvariantViolation`, `checkHonestyInvariant()`, `checkErrorLeakInvariant()`, `checkOcclusionInvariant()`, `createWatchdogTracker()`.

- [ ] **Step 1: Define Debugger Types**
Create `crates/vox-gui/ui/src/debugger/types.ts`:
```typescript
export type StepStatus = 'pending' | 'active' | 'paused' | 'completed' | 'failed' | 'skipped';

export interface DebugStep<TInput = unknown, TOutput = unknown> {
  id: string;
  label: string;
  description?: string;
  status: StepStatus;
  inputPayload?: TInput;
  outputPayload?: TOutput;
  error?: string;
  timingMs?: number;
}

export interface BreakpointConfig {
  pauseBeforeStepIds: Set<string>;
  pauseAfterStepIds: Set<string>;
  pauseOnError: boolean;
  pauseOnZeroData: boolean;
}

export interface InvariantViolation {
  kind: 'fake_success' | 'raw_error_leak' | 'occlusion' | 'watchdog_stalled' | 'a11y_contrast';
  severity: 'critical' | 'major' | 'minor';
  message: string;
  elementSelector?: string;
  location?: string;
  rawDetails?: unknown;
}
```

- [ ] **Step 2: Write failing test for Honesty Sentry**
Create `crates/vox-gui/ui/src/debugger/sentries/sentryHonesty.test.ts`:
```typescript
import { describe, it, expect } from 'vitest';
import { checkHonestyInvariant } from './sentryHonesty';

describe('sentryHonesty', () => {
  it('flags fake success when status is completed but required collections are empty', () => {
    const violation = checkHonestyInvariant({
      status: 'completed',
      payload: { sources: [], claims: [] },
      requiredFields: ['sources'],
      stageName: 'MultiSourceRetrieval',
    });
    expect(violation).not.toBeNull();
    expect(violation?.kind).toBe('fake_success');
    expect(violation?.severity).toBe('critical');
    expect(violation?.message).toContain('MultiSourceRetrieval completed with zero sources');
  });

  it('passes when required collections have items', () => {
    const violation = checkHonestyInvariant({
      status: 'completed',
      payload: { sources: [{ url: 'https://example.com' }] },
      requiredFields: ['sources'],
      stageName: 'MultiSourceRetrieval',
    });
    expect(violation).toBeNull();
  });
});
```

- [ ] **Step 3: Run Honesty Sentry test to verify failure**
Run: `pnpm --dir crates/vox-gui/ui test sentryHonesty.test.ts`
Expected: FAIL with "checkHonestyInvariant is not defined"

- [ ] **Step 4: Implement Honesty Sentry**
Create `crates/vox-gui/ui/src/debugger/sentries/sentryHonesty.ts`:
```typescript
import type { InvariantViolation, StepStatus } from '../types';

export interface HonestyCheckParams {
  status: StepStatus;
  payload: Record<string, unknown> | null | undefined;
  requiredFields?: string[];
  stageName: string;
}

export function checkHonestyInvariant({
  status,
  payload,
  requiredFields = [],
  stageName,
}: HonestyCheckParams): InvariantViolation | null {
  if (status !== 'completed') return null;
  if (!payload) {
    return {
      kind: 'fake_success',
      severity: 'critical',
      message: `${stageName} completed with null/empty payload`,
      location: stageName,
    };
  }
  for (const field of requiredFields) {
    const val = payload[field];
    if (Array.isArray(val) && val.length === 0) {
      return {
        kind: 'fake_success',
        severity: 'critical',
        message: `${stageName} completed with zero ${field}`,
        location: `${stageName}.${field}`,
      };
    }
  }
  return null;
}
```

- [ ] **Step 5: Write failing test for Error Leak Sentry**
Create `crates/vox-gui/ui/src/debugger/sentries/sentryErrorLeak.test.ts`:
```typescript
import { describe, it, expect } from 'vitest';
import { checkErrorLeakInvariant } from './sentryErrorLeak';

describe('sentryErrorLeak', () => {
  it('detects raw TypeError and __TAURI_INTERNALS__ leak', () => {
    const el = document.createElement('div');
    el.innerHTML = '<span>TypeError: cannot read property of null at window.__TAURI_INTERNALS__</span>';
    const violations = checkErrorLeakInvariant(el);
    expect(violations.length).toBeGreaterThan(0);
    expect(violations[0].kind).toBe('raw_error_leak');
    expect(violations[0].severity).toBe('critical');
  });

  it('passes on clean DOM tree', () => {
    const el = document.createElement('div');
    el.innerHTML = '<p>Operation completed successfully. No results found.</p>';
    const violations = checkErrorLeakInvariant(el);
    expect(violations.length).toBe(0);
  });
});
```

- [ ] **Step 6: Implement Error Leak Sentry**
Create `crates/vox-gui/ui/src/debugger/sentries/sentryErrorLeak.ts`:
```typescript
import type { InvariantViolation } from '../types';

const FORBIDDEN_PATTERNS = [
  /TypeError:/i,
  /__TAURI_INTERNALS__/i,
  /undefined is not/i,
  /null is not/i,
  /\[object Object\]/i,
  /uncaught (in promise)/i,
];

export function checkErrorLeakInvariant(root: Element = document.body): InvariantViolation[] {
  const violations: InvariantViolation[] = [];
  const textNodes: Node[] = [];
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let node: Node | null;
  while ((node = walker.nextNode())) {
    textNodes.push(node);
  }

  for (const textNode of textNodes) {
    const content = textNode.textContent ?? '';
    for (const pattern of FORBIDDEN_PATTERNS) {
      if (pattern.test(content)) {
        violations.push({
          kind: 'raw_error_leak',
          severity: 'critical',
          message: `Raw error string leaked to DOM: "${content.trim().slice(0, 100)}"`,
          elementSelector: textNode.parentElement?.tagName.toLowerCase(),
          rawDetails: content,
        });
        break;
      }
    }
  }
  return violations;
}
```

- [ ] **Step 7: Implement Occlusion and Watchdog Sentries & Tests**
Create `crates/vox-gui/ui/src/debugger/sentries/sentryOcclusion.ts` and `crates/vox-gui/ui/src/debugger/sentries/sentryWatchdog.ts` with matching test files.
Run: `pnpm --dir crates/vox-gui/ui test sentry`
Expected: All sentry tests PASS.

- [ ] **Step 8: Commit Task 1**
```bash
git add crates/vox-gui/ui/src/debugger/
git commit -m "feat(gui): implement diagnostic invariant sentries for honesty, leaks, and occlusion"
```

---

### Task 2: Universal Stepper Finite State Machine & Hook (`usePipelineStepper`)

**Files:**
- Create: `crates/vox-gui/ui/src/debugger/usePipelineStepper.ts`
- Create: `crates/vox-gui/ui/src/debugger/usePipelineStepper.test.ts`
- Create: `crates/vox-gui/ui/src/debugger/snapshotService.ts`
- Create: `crates/vox-gui/ui/src/debugger/snapshotService.test.ts`

**Interfaces:**
- Consumes: `DebugStep`, `BreakpointConfig`, `InvariantViolation` from Task 1.
- Produces: `usePipelineStepper({ steps, onStepExecute })` returning `{ currentStepIndex, isPaused, activeStep, stepNext, pause, resume, overridePayload }`.

- [ ] **Step 1: Write failing test for usePipelineStepper**
Create `crates/vox-gui/ui/src/debugger/usePipelineStepper.test.ts`:
```typescript
import { describe, it, expect, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { usePipelineStepper } from './usePipelineStepper';
import type { DebugStep } from './types';

describe('usePipelineStepper', () => {
  const initialSteps: DebugStep[] = [
    { id: 's1', label: 'Step 1', status: 'pending' },
    { id: 's2', label: 'Step 2', status: 'pending' },
  ];

  it('pauses at step when breakpoint is set', async () => {
    const onExecute = vi.fn().mockResolvedValue({ result: 'data1' });
    const { result } = renderHook(() =>
      usePipelineStepper({
        steps: initialSteps,
        initialBreakpoints: {
          pauseBeforeStepIds: new Set(['s2']),
          pauseAfterStepIds: new Set(),
          pauseOnError: true,
          pauseOnZeroData: true,
        },
        onExecuteStep: onExecute,
      })
    );

    await act(async () => {
      await result.current.start();
    });

    // s1 executed, stopped before s2
    expect(result.current.currentStepId).toBe('s2');
    expect(result.current.isPaused).toBe(true);
    expect(onExecute).toHaveBeenCalledTimes(1);
  });
});
```

- [ ] **Step 2: Run hook test to verify failure**
Run: `pnpm --dir crates/vox-gui/ui test usePipelineStepper.test.ts`
Expected: FAIL with "usePipelineStepper is not defined"

- [ ] **Step 3: Implement usePipelineStepper**
Create `crates/vox-gui/ui/src/debugger/usePipelineStepper.ts`:
Implement the state machine supporting `start()`, `pause()`, `stepNext()`, `resume()`, `overridePayload(stepId, data)`, and registering active invariant violations.

- [ ] **Step 4: Implement snapshotService**
Create `crates/vox-gui/ui/src/debugger/snapshotService.ts` to capture DOM bounding client rects, active sentry violations, and invoke Tauri `gui_capture_snapshot` when available, falling back to local canvas serialization.

- [ ] **Step 5: Run tests and verify passing**
Run: `pnpm --dir crates/vox-gui/ui test usePipelineStepper.test.ts`
Run: `pnpm --dir crates/vox-gui/ui test snapshotService.test.ts`
Expected: PASS

- [ ] **Step 6: Commit Task 2**
```bash
git add crates/vox-gui/ui/src/debugger/usePipelineStepper.* crates/vox-gui/ui/src/debugger/snapshotService.*
git commit -m "feat(gui): implement universal pipeline stepper FSM and snapshot service"
```

---

### Task 3: Backend Search Prober & Stepper Tauri Commands

**Files:**
- Create: `crates/vox-gui/src/commands/search_probe.rs`
- Create: `crates/vox-gui/src/commands/debugger.rs`
- Modify: `crates/vox-gui/src/commands/mod.rs`
- Modify: `crates/vox-gui/src/main.rs`
- Modify: `crates/vox-research-shim/src/research/orchestrator/pipeline.rs` (Zero-hits hard gate)
- Test: `crates/vox-gui/tests/search_probe_test.rs`

**Interfaces:**
- Produces: `probe_search_provider`, `probe_all_search_providers`, `gui_capture_snapshot`, `pipeline_step_pause`, `pipeline_step_resume`.

- [ ] **Step 1: Implement Zero-Hits Hard Gate in Research Pipeline**
In `crates/vox-research-shim/src/research/orchestrator/pipeline.rs`:
Add explicit check right after web/local gather:
```rust
if all_hits.is_empty() {
    let msg = "Deep Research failed: zero evidence sources retrieved across providers. Synthesis halted.";
    tracing::error!(query = %query.query, "{msg}");
    set_session_stage(db, session_id, ResearchStage::Failed).await;
    return Err(anyhow::anyhow!("{msg}"));
}
```

- [ ] **Step 2: Implement search_probe.rs in vox-gui**
Create `crates/vox-gui/src/commands/search_probe.rs` with `probe_search_provider(provider_name: String, query: String)` calling `WebSearchDispatcher` and measuring status code, latency, and hit count.

- [ ] **Step 3: Implement debugger.rs in vox-gui**
Create `crates/vox-gui/src/commands/debugger.rs` handling `gui_capture_snapshot` writing PNG bytes + JSON metadata to `review-bundle/latest/` or `target/gui-snapshots/`.

- [ ] **Step 4: Wire commands into main.rs**
Register in `tauri::generate_handler![..., commands::search_probe::probe_search_provider, commands::search_probe::probe_all_search_providers, commands::debugger::gui_capture_snapshot]`.

- [ ] **Step 5: Run Rust tests**
Run: `cargo test -p vox-gui search_probe`
Expected: PASS

- [ ] **Step 6: Commit Task 3**
```bash
git add crates/vox-gui/src/commands/search_probe.rs crates/vox-gui/src/commands/debugger.rs crates/vox-gui/src/main.rs crates/vox-gui/src/commands/mod.rs crates/vox-research-shim/src/research/orchestrator/pipeline.rs
git commit -m "feat(backend): add direct search provider prober, snapshot command, and zero-hits hard gate"
```

---

### Task 4: Global Inspector Drawer Component (`InspectorDrawer.tsx`)

**Files:**
- Create: `crates/vox-gui/ui/src/debugger/InspectorDrawer.tsx`
- Create: `crates/vox-gui/ui/src/debugger/InspectorDrawer.test.tsx`
- Modify: `crates/vox-gui/ui/src/App.tsx` (Mount global drawer + keybinding)

**Interfaces:**
- Consumes: `usePipelineStepper`, `snapshotService`, `sentry*`.
- Produces: Collapsible bottom/side dock with tabs: *Timeline/DAG*, *Payload Inspector/Editor*, *Sentry Violations*, *Viewport Matrix*.

- [ ] **Step 1: Write failing test for InspectorDrawer**
Create `crates/vox-gui/ui/src/debugger/InspectorDrawer.test.tsx`:
Verify that pressing `Cmd+Shift+D` opens the drawer and renders the Step timeline, JSON editor, and Sentry Violations badge.

- [ ] **Step 2: Implement InspectorDrawer**
Create `crates/vox-gui/ui/src/debugger/InspectorDrawer.tsx`:
Include:
- Two-pane JSON editor for step inputs/outputs.
- Live Sentry warnings counter with alert popups.
- Viewport size switcher (`Compact 900px`, `Laptop 1100px`, `Wide 1440px`).
- "Capture Visual Evidence" snapshot button.

- [ ] **Step 3: Mount InspectorDrawer in App.tsx**
Add keyboard listener for `(e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 'D'` to toggle drawer.

- [ ] **Step 4: Run tests**
Run: `pnpm --dir crates/vox-gui/ui test InspectorDrawer.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit Task 4**
```bash
git add crates/vox-gui/ui/src/debugger/InspectorDrawer.* crates/vox-gui/ui/src/App.tsx
git commit -m "feat(gui): mount global Inspector Drawer with payload editor, sentry alerts, and viewport switcher"
```

---

### Task 5: Deep Research Lab Surface (`ResearchLabView`, `LiveSourceProber`, `JudgeInspector`)

**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/ResearchLab/LiveSourceProber.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/ResearchLab/LiveSourceProber.test.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/ResearchLab/JudgeInspector.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/ResearchLab/JudgeInspector.test.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/ResearchLab/ResearchLabView.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/ResearchLab/ResearchLabView.test.tsx`
- Modify: `crates/vox-gui/ui/src/generated/surfaceRegistry.generated.ts` (Register `research-lab`)

**Interfaces:**
- Consumes: `usePipelineStepper`, `probe_search_provider`, `JudgeParams`.
- Produces: Complete step-by-step Research Lab workbench surface.

- [ ] **Step 1: Write test and implement LiveSourceProber**
Create `LiveSourceProber.tsx`: A diagnostic card where users can type a query and click "Probe Sources". Renders a table with provider name (SearXNG, Tavily, DDG, Wikipedia, Local), HTTP status badge, round-trip latency, returned snippet count, and error remediation tips.

- [ ] **Step 2: Write test and implement JudgeInspector**
Create `JudgeInspector.tsx`: An epistemic breakdown component displaying the judge LLM system prompt, atomic claims mapped against evidence citations, verifiability confidence score, and the judge's full written chain-of-thought rationale.

- [ ] **Step 3: Implement ResearchLabView**
Create `ResearchLabView.tsx`: Connects the 5 research stages (Decomposition, Multi-Source Retrieval, Evidence Extraction, Claim Extraction & Epistemic Judge, Synthesis) to `usePipelineStepper`.
Provides per-stage breakpoint checkboxes (`Pause Before Stage`). When paused at MultiSourceRetrieval, renders the `LiveSourceProber`. When paused at EpistemicJudge, renders the `JudgeInspector`.

- [ ] **Step 4: Register Surface in surfaceRegistry**
Add `'research-lab'` to `SURFACE_REGISTRY` with title `"Research Lab"`.

- [ ] **Step 5: Run tests**
Run: `pnpm --dir crates/vox-gui/ui test ResearchLab`
Expected: PASS

- [ ] **Step 6: Commit Task 5**
```bash
git add crates/vox-gui/ui/src/components/surfaces/ResearchLab/
git commit -m "feat(gui): create Deep Research Lab surface with live prober and epistemic judge inspector"
```

---

### Task 6: Playwright Automated Stepper & Screenshot Verification Suite

**Files:**
- Create: `crates/vox-gui/ui/e2e/review/stepper.spec.ts`
- Modify: `crates/vox-gui/ui/e2e/review/states.ts` (Add `research-lab` state entries)
- Modify: `AGENTS.md` (Add normative GUI Visual Verification Invariant policy)

**Interfaces:**
- Consumes: Research Lab surface, Playwright test runner.
- Produces: Captured full-resolution viewport PNGs at each pipeline breakpoint saved to `crates/vox-gui/ui/review-bundle/latest/`.

- [ ] **Step 1: Update AGENTS.md with Normative Policy**
Add the `GUI Visual Verification Invariant (Normative)` section to `AGENTS.md`.

- [ ] **Step 2: Add States in states.ts**
Add `research-lab` to `SURFACE_STATES` with default, paused-at-retrieval, and zero-hits error states.

- [ ] **Step 3: Create stepper.spec.ts**
Create `crates/vox-gui/ui/e2e/review/stepper.spec.ts`:
- Loads `research-lab`.
- Inputs a query with breakpoints set.
- Advances step-by-step, taking screenshots at Decomposition, Retrieval, Claim Extraction, Judge, and Synthesis.
- Injects a zero-hits scenario and asserts the Honesty Sentry halts with `ZeroSourcesError`.

- [ ] **Step 4: Run Playwright test**
Run: `pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/stepper.spec.ts --project=chromium`
Expected: PASS, screenshots generated in `review-bundle/latest/`.

- [ ] **Step 5: Commit Task 6**
```bash
git add AGENTS.md crates/vox-gui/ui/e2e/review/
git commit -m "test(e2e): add automated Playwright stepper test suite and normative visual verification policy"
```

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-09-17-axis-gui-visual-debugger-deep-research.md`.

Two execution options:

1. **Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration
2. **Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
