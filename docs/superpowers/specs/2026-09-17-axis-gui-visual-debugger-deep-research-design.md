---
title: "Axis GUI Visual Debugger and Deep Research Inspection Lab"
description: "Universal GUI visual debugger with honesty sentries, layout collision auditing, fault injection, and end-to-end stepping and inspection for Deep Research in Axis."
category: "architecture"
status: "current"
date: 2026-09-17
---

# Axis GUI Visual Debugger and Deep Research Inspection Lab

## 1. Problem & Context

### 1.1 Deep Research Failures in Production
During testing of the Vox deep research pipeline, runs claimed completion and produced synthesized reports while actually retrieving **zero evidence from web search sources**.
* **Root Cause 1: DuckDuckGo Instant Answer API Fallback**: In `crates/vox-search/src/duckduckgo.rs:72–75`, the client queries `api.duckduckgo.com`, which is an Instant Answer/disambiguation endpoint, *not* a general web search API. For technical research subqueries, it returns HTTP 200 with empty topic lists. Because it succeeds with HTTP 200, circuit breakers never trip, yet zero hits are produced.
* **Root Cause 2: Error Swallowing in Provider Registry**: In `crates/vox-research-shim/src/research/provider.rs:106–109`, all errors from `WebSearchDispatcher` are mapped to `(Vec::new(), self.primary.clone())`, masking network, transport, or configuration failures.
* **Root Cause 3: Groundless Synthesis Cascade**: In `crates/vox-research-shim/src/research/orchestrator/pipeline.rs:326–339`, when `all_hits` is empty, claim extraction falls back to the user prompt. In `stages.rs:170`, `synthesize_answer_with_llm` outputs *"No external sources were found... Answering from internal knowledge only"*, marks `ResearchStage::Completed`, and returns success.
* **Root Cause 4: Discarded Epistemic Judge Reasoning**: In `stages.rs`, `build_judge_system_prompt()` demands detailed sub-scores and chain-of-thought reasoning (`factual_accuracy_reasoning`, `citation_density_reasoning`, `coverage_reasoning`), but `JudgeResponse` deserializes *only* `total_score: i32`—discarding all reasoning text.

### 1.2 Systemic GUI Defect Classes (413-Cell Review Matrix)
The comprehensive audit of Axis (`docs/superpowers/reviews/2026-07-18-axis-frontend-comprehensive-review.md`) identified four recurring classes of frontend defects:
1. **"Fake Success" & Silent Degradation**: UI transitions to `completed` while primary collections are empty.
2. **Raw Error Leakage**: Caught or uncaught exceptions (`TypeError: s is null`, `can't access property 'invoke'`, `__TAURI_INTERNALS__`) leaking raw JavaScript traces into toasts or visible DOM nodes.
3. **Layout Occlusion & Transparent Bleed-Through**: Overlays and popovers occluding inputs or rendering with transparent backdrops that let background text bleed through in Firefox and compact viewports.
4. **State Desynchronization & Watchdog Stalls**: Ghost "running" spinners where the backend finished or crashed without notifying the frontend.

### 1.3 Goals
1. **Generalized GUI Visual Debugger**: A reusable debugging framework in Axis that can step through, inspect, and audit any multi-step workflow without overfitting.
2. **Deep Research Diagnostic Mode**: Embedded within `ResearchView.tsx` with an isolated **Live Source Prober**, an **Epistemic Judge Inspector**, and a **Zero-Hits Hard Gate** that halts execution before hallucinated synthesis can occur.
3. **Automated Visual Evidence Capture**: Flake-free Playwright snapshot harness writing viewport PNGs and diagnostic metadata into `review-bundle/`.
4. **Codebase-Wide GUI Policy**: A normative policy in `AGENTS.md` and CI ensuring no GUI surface or flow is considered complete or mergeable without automated visual inspection evidence.

---

## 2. Architecture Overview

```mermaid
flowchart TD
    subgraph Axis Frontend Shell ("crates/vox-gui/ui")
        subgraph Universal Debugger Engine ("src/debugger/")
            Stepper[usePipelineStepper Hook\nCanonical 8 Stages]
            Drawer[Global Inspector Drawer\nz-45 Overlay · Cmd+Shift+D]
            Sentries[Diagnostic Invariant Sentries\nHonesty | Error Leak]
            Snap[Playwright Snapshot Harness\nDeterministic IPC Sentinel]
        end
        
        subgraph Surfaces
            Research[Research Surface\n(Diagnostic Prober & Stepper Mode)]
            Chat[Chat Surface\n(Milestone Badges & Summary Cards)]
            Other[Tasks / Models / Settings]
        end
        
        Research --> Stepper
        Chat -.-> Stepper
        Stepper <--> Drawer
        Sentries --> Drawer
        Drawer --> Snap
    end

    subgraph Tauri IPC & Protocol Layer ("crates/vox-gui/src/commands/")
        ProbeCmds[search_probe.rs\n- probe_search_provider\n- probe_all_search_providers]
    end

    subgraph Backend Engine ("crates/vox-research-shim & vox-search")
        Dispatcher[WebSearchDispatcher]
        Prober[Direct Web Search Prober Engine]
        Pipeline[Deep Research Pipeline\nZero-Hits Hard Gate]
    end

    subgraph Verification & CI ("crates/vox-gui/ui/e2e/")
        PW[Playwright Stepper Spec\ne2e/review/stepper.spec.ts]
        ReviewBundle[review-bundle/latest/]
    end

    Research --> ProbeCmds
    ProbeCmds --> Prober
    Pipeline --> Dispatcher
    Snap --> ReviewBundle
    PW --> ReviewBundle
```

---

## 3. Core Universal Debugger (`crates/vox-gui/ui/src/debugger/`)

### 3.1 Types and Contracts (`types.ts`)
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
  severity: 'critical' | 'major' | 'minor' | 'info';
  message: string;
  elementSelector?: string;
  location?: string;
  rawDetails?: unknown;
}
```

### 3.2 The Diagnostic Invariant Sentries
1. **Honesty Sentry (`sentryHonesty.ts`)**:
   - Differentiates between infrastructure failure, legitimate empty search, and groundless synthesis:
     - **Infrastructure Failure**: If all search backends returned HTTP 429, 500, or timeout, a `completed` state is flagged as `critical` `fake_success`.
     - **Honest Empty Search**: If providers were healthy (200 OK) but returned 0 hits, and the UI renders `[data-testid="empty-results-notice"]` or sets `emptyStateAcknowledged: true`, it is classified as honest and passes.
     - **Fake Success**: If 0 sources are retrieved but report synthesis proceeds, it triggers a `critical` `fake_success` violation.
2. **Error Leak Sentry (`sentryErrorLeak.ts`)**:
   - Scans system chrome (`[data-testid="toast-item"]`, `[role="alert"]`, `[role="status"]`, headers, error boundaries).
   - Strictly excludes user/assistant content containers (`.prose`, `.markdown-body`, `pre`, `code`, `[data-testid="chat-transcript"]`, `[data-testid="inspector-drawer"]`).
   - Flags raw JS exceptions (`TypeError:`, `undefined is not a`, `__TAURI_INTERNALS__`, `[object Object]`).

### 3.3 Global Inspector Drawer (`InspectorDrawer.tsx`)
- Mounted at root of `App.tsx` as a fixed overlay at `z-45` (preventing in-flow flex reflows that collapse Dockview sizing).
- Toggled via `keybinds.ts` with `'toggle-inspector': 'Mod+Shift+D'`.
- Hosts:
  - Canonical 8-stage stepper timeline (`queued` $\to$ `planning` $\to$ `retrieving` $\to$ `verifying_claims` $\to$ `synthesizing` $\to$ `auditing_citations` $\to$ `persisting_artifact` $\to$ `completed`).
  - Two-pane JSON viewer (Input vs Output).
  - Sentry warnings counter and alert list.
  - Viewport matrix toggles (`900px`, `1100px`, `1440px`).

---

## 4. Deep Research Integration in `ResearchView.tsx`

### 4.1 Embedded Diagnostic Mode
Rather than creating an un-registered duplicate surface (`ResearchLabView`), `ResearchView.tsx` gains a diagnostic toolbar:
* **"Diagnostic Prober & Stepper" Toggle**: Expands an inline diagnostic workbench:
  - **`LiveSourceProber.tsx`**: Tests SearXNG, Tavily, DuckDuckGo, Wikipedia, and Local DB with a canary query, rendering HTTP status, round-trip latency, hit counts, and error remediation tips.
  - **`JudgeInspector.tsx`**: Displays the full judge prompt, atomic claims mapped against cited snippets, and the complete chain-of-thought rationale from `JudgeEvaluationV1`.
* Reuses existing high-quality components: `ResearchDagCanvas.tsx`, `ResearchClaimAccordion.tsx`, `HeadlineVerdictBanner.tsx`, and `MultiWaveProgressTimeline.tsx`.

### 4.2 Zero-Hits Hard Gate in `pipeline.rs`
```rust
// HARD GATE: Do not allow pipeline to synthesize if zero sources were acquired
if do_web && all_hits.is_empty() {
    let err_msg = "Deep Research halted: Zero evidence sources retrieved across search providers. Halting to prevent hallucinated synthesis.";
    tracing::error!(query = %query.query, "{err_msg}");
    set_session_stage(db, session_id, ResearchStage::Failed).await;
    return Err(anyhow::anyhow!(err_msg));
}
```

---

## 5. Backend Search Prober Command (`search_probe.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProbeResult {
    pub provider: String,
    pub http_status: u16,
    pub latency_ms: u64,
    pub success: bool,
    pub hit_count: usize,
    pub error_message: Option<String>,
}

#[tauri::command]
pub async fn probe_search_provider(
    provider: String,
    query: String,
) -> Result<ProviderProbeResult, String>;

#[tauri::command]
pub async fn probe_all_search_providers(
    query: String,
) -> Result<Vec<ProviderProbeResult>, String>;
```
The command directly instantiates search clients without invoking `SearchProviderCircuitRegistry::global()`, ensuring diagnostic probing never contaminates production circuit-breaker cooldowns.

---

## 6. Codebase-Wide Policy & Visual Verification Harness

### 6.1 Normative Policy in `AGENTS.md`
Add to `AGENTS.md`:
> **GUI Visual Verification Invariant (Normative)**:
> 1. Every new or modified GUI surface, drawer, or complex interactive pipeline flow MUST include Playwright visual inspection coverage in `crates/vox-gui/ui/e2e/review/`.
> 2. The surface MUST have registered states in `states.ts` (including default, empty, and error mock states).
> 3. Automated captures MUST pass the Invariant Sentry checks (zero raw error leaks, zero ungrounded fake success).
> 4. Automated screenshots must be saved to `crates/vox-gui/ui/review-bundle/latest/`.

### 6.2 Flake-Free Playwright Harness
- **`__VOX_IPC_ACTIVE_COUNT__` Sentinel**: In `tauriMockShared.ts`, track active async requests. Playwright tests await `window.__VOX_IPC_ACTIVE_COUNT__ === 0` rather than relying on brittle `waitForTimeout(400)`.
- **Multi-Step Breakpoint States**: Extend `ReviewState` in `states.ts` with `steps?: ReviewStep[]` to execute sequential breakpoints and take screenshots within a single persistent context.
- **Git Diff Hygiene**: Add entries to `.gitattributes` for `bundle-cache.v1.json` and `bundle-digest.md` with `linguist-generated=true -diff`.
