# Axis GUI Visual Debugger and Deep Research Inspection Lab Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a generalized GUI visual debugger and diagnostic invariant sentry system in Axis, wire it into Deep Research with in-flight breakpoints, a live search provider prober, epistemic judge breakdown, and zero-hits halting, backed by an automated Playwright multi-step screenshot harness and codebase-wide visual verification policy.

**Architecture:** A reusable React 19 + TypeScript debugger engine in `crates/vox-gui/ui/src/debugger/` (canonical 8-stage FSM stepper, global inspector drawer, DOM invariant sentries) communicates via Tauri IPC (`probe_search_provider`) with the Rust backend. Deep Research embeds the prober and judge breakdown into `ResearchView.tsx` and enforces a zero-hits hard gate against hallucination. Automated Playwright specs with deterministic IPC sentinels capture full-resolution screenshots at every breakpoint to verify end-to-end evidence.

**Tech Stack:** TypeScript, React 19, Tailwind CSS, Playwright, Vitest, Rust (Tauri 2, tokio, serde), `vox-search`, `vox-research-shim`.

**Spec:** [`docs/superpowers/specs/2026-09-17-axis-gui-visual-debugger-deep-research-design.md`](file:///Users/brbrainerd/dev/vox/docs/superpowers/specs/2026-09-17-axis-gui-visual-debugger-deep-research-design.md)

## Global Constraints

- Never run `cargo fmt --all` on this workspace (Windows command-line length overflow); format specific crates with `cargo fmt -p vox-gui` / `cargo fmt -p vox-research-shim`.
- All automation scripts must be `.vox` or native commands, never new `.ps1`, `.sh`, or `.py`.
- No raw `__TAURI_INTERNALS__` or unhandled promise rejections may leak to DOM nodes or toasts.
- Never edit `surfaceRegistry.generated.ts` manually; it is derived automatically.
- Every task MUST be atomic, end green, and be committed before proceeding.
- Verify-before-use: execute inlined `rg` commands before editing any file to verify existing symbol presence.
- Two-strike circuit breaker: if any test or compilation step fails twice consecutively, STOP and produce a detailed handoff note.

---

### Task 1: `[SEQUENTIAL]` Zero-Hits Hard Gate & Search Integrity in `vox-research-shim`

**Files:**
- Modify: `crates/vox-research-shim/src/research/orchestrator/pipeline.rs:320-335`
- Modify: `crates/vox-research-shim/src/research/orchestrator/web_gather.rs:140-155`
- Test: `crates/vox-research-shim/tests/research_zero_hits_gate_test.rs`

**Interfaces:**
- Produces: `ResearchPipelineError::ZeroRetrievalHits`, site scope pushdown in `search_one_subquery`.

- [ ] **Step 1: Verify existing code structure via Preflight Grep**
```bash
rg -n "let mut all_hits" crates/vox-research-shim/src/research/orchestrator/pipeline.rs
rg -n "extract_claims_with_model" crates/vox-research-shim/src/research/orchestrator/pipeline.rs
rg -n "fn search_one_subquery" crates/vox-research-shim/src/research/orchestrator/web_gather.rs
```
Expected: `all_hits` at line 212, `extract_claims_with_model` at line 341, `search_one_subquery` at line 142.

- [ ] **Step 2: Write failing unit test for Zero-Hits Hard Gate**
Create `crates/vox-research-shim/tests/research_zero_hits_gate_test.rs`:
```rust
use vox_research_shim::research::{ResearchConfig, ResearchQuery, ResearchScope, run_research};

#[tokio::test]
async fn test_empty_web_retrieval_halts_without_synthesis() {
    let mut config = ResearchConfig::default();
    config.claim_detection_enabled = true;
    
    // An obscure query with web scope that yields zero hits
    let query = ResearchQuery {
        query: "x89q_gibberish_term_guaranteed_zero_hits_2026".to_string(),
        scope: ResearchScope::Web,
        max_sources: 5,
        verify_claims: true,
        site_scope: None,
        waves: 1,
        domain_mode: vox_research_shim::research::ResearchDomainMode::General,
    };

    let result = run_research(query, &config).await;
    assert!(result.is_err(), "Pipeline must halt with Err on zero retrieval hits");
    let err_str = result.err().unwrap().to_string();
    assert!(err_str.contains("Zero evidence sources retrieved"), "Error must clearly cite zero retrieval hits: {err_str}");
}
```

- [ ] **Step 3: Run test to verify it fails**
Run: `cargo test -p vox-research-shim --test research_zero_hits_gate_test`
Expected: FAIL (currently succeeds and synthesizes empty placeholder).

- [ ] **Step 4: Implement Zero-Hits Hard Gate and Site Scope Pushdown**
1. In `crates/vox-research-shim/src/research/orchestrator/pipeline.rs`, right before step `(e) Confidence gate` (around line 324):
```rust
    if do_web && all_hits.is_empty() {
        let err_msg = "Deep Research halted: Zero evidence sources retrieved across search providers. Halting to prevent hallucinated synthesis.";
        tracing::error!(query = %query.query, "{err_msg}");
        set_session_stage(db, session_id, ResearchStage::Failed).await;
        return Err(anyhow::anyhow!(err_msg));
    }
```
2. In `crates/vox-research-shim/src/research/orchestrator/web_gather.rs` (lines 142–155), push `site:<domain>` into the search query:
```rust
async fn search_one_subquery(
    subquery: &str,
    policy: &SearchPolicy,
    registry: &ProviderRegistry,
    site_scope: Option<&str>,
    seen_urls: &mut HashSet<String>,
    all_hits: &mut Vec<ResearchHit>,
    novelty_scorer: &mut vox_search::novelty::NoveltyScorer,
) -> (usize, usize) {
    let query_string = match site_scope {
        Some(scope) if !scope.trim().is_empty() => format!("{subquery} site:{scope}"),
        _ => subquery.to_string(),
    };
    let (mut hits, _) = registry.search(&query_string, policy).await;
```

- [ ] **Step 5: Run test to verify it passes**
Run: `cargo test -p vox-research-shim --test research_zero_hits_gate_test`
Expected: PASS
Run: `cargo check -p vox-research-shim`
Run: `cargo fmt -p vox-research-shim`

- [ ] **Step 6: Commit Task 1**
```bash
git add crates/vox-research-shim/src/research/orchestrator/pipeline.rs crates/vox-research-shim/src/research/orchestrator/web_gather.rs crates/vox-research-shim/tests/research_zero_hits_gate_test.rs
git commit -m "feat(research): enforce zero-hits hard gate and pushdown site scope in retrieval"
```

---

### Task 2: `[SEQUENTIAL]` Direct Search Provider Prober Tauri Command

**Files:**
- Create: `crates/vox-gui/src/commands/search_probe.rs`
- Modify: `crates/vox-gui/src/commands/mod.rs`
- Modify: `crates/vox-gui/src/main.rs`
- Test: `crates/vox-gui/tests/search_probe_test.rs`

**Interfaces:**
- Produces: `probe_search_provider`, `probe_all_search_providers`.

- [ ] **Step 1: Verify existing command registration via Preflight Grep**
```bash
rg -n "pub mod research;" crates/vox-gui/src/commands/mod.rs
rg -n "start_research_async" crates/vox-gui/src/main.rs
```

- [ ] **Step 2: Implement search_probe.rs command**
Create `crates/vox-gui/src/commands/search_probe.rs`:
```rust
use serde::{Deserialize, Serialize};
use vox_search::policy::SearchPolicy;
use vox_search::searxng::SearxngSearchClient;
use vox_search::tavily::TavilySearchClient;
use vox_search::duckduckgo::DuckDuckGoClient;
use vox_search::wikipedia::WikipediaClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProbeResult {
    pub provider: String,
    pub http_status: u16,
    pub latency_ms: u64,
    pub success: bool,
    pub hit_count: usize,
    pub sample_titles: Vec<String>,
    pub error_message: Option<String>,
    pub remediation_tip: Option<String>,
}

#[tauri::command]
pub async fn probe_search_provider(
    provider: String,
    query: String,
) -> Result<ProviderProbeResult, String> {
    let start = std::time::Instant::now();
    let q = query.trim();
    if q.is_empty() {
        return Err("Query cannot be empty".into());
    }

    match provider.to_lowercase().as_str() {
        "searxng" => {
            let policy = SearchPolicy::default();
            let base_url = match &policy.searxng_url {
                Some(url) if !url.trim().is_empty() => url.clone(),
                _ => return Ok(ProviderProbeResult {
                    provider,
                    http_status: 0,
                    latency_ms: 0,
                    success: false,
                    hit_count: 0,
                    sample_titles: vec![],
                    error_message: Some("SearXNG URL is not configured".into()),
                    remediation_tip: Some("Set VOX_SEARCH_SEARXNG_URL in environment or settings".into()),
                }),
            };
            let client = SearxngSearchClient::new(base_url);
            match client.search(q, 3, None, None).await {
                Ok(hits) => Ok(ProviderProbeResult {
                    provider,
                    http_status: 200,
                    latency_ms: start.elapsed().as_millis() as u64,
                    success: true,
                    hit_count: hits.len(),
                    sample_titles: hits.iter().map(|h| h.title.clone()).collect(),
                    error_message: None,
                    remediation_tip: None,
                }),
                Err(e) => Ok(ProviderProbeResult {
                    provider,
                    http_status: 502,
                    latency_ms: start.elapsed().as_millis() as u64,
                    success: false,
                    hit_count: 0,
                    sample_titles: vec![],
                    error_message: Some(e.to_string()),
                    remediation_tip: Some("Check if the SearXNG instance is running and reachable".into()),
                }),
            }
        },
        "tavily" => {
            let client = match TavilySearchClient::from_env() {
                Some(c) => c,
                None => return Ok(ProviderProbeResult {
                    provider,
                    http_status: 0,
                    latency_ms: 0,
                    success: false,
                    hit_count: 0,
                    sample_titles: vec![],
                    error_message: Some("TAVILY_API_KEY is unset".into()),
                    remediation_tip: Some("Configure Tavily API key in settings or secrets".into()),
                }),
            };
            match client.search(q, 3, "basic").await {
                Ok(hits) => Ok(ProviderProbeResult {
                    provider,
                    http_status: 200,
                    latency_ms: start.elapsed().as_millis() as u64,
                    success: true,
                    hit_count: hits.len(),
                    sample_titles: hits.iter().map(|h| h.title.clone()).collect(),
                    error_message: None,
                    remediation_tip: None,
                }),
                Err(e) => Ok(ProviderProbeResult {
                    provider,
                    http_status: 500,
                    latency_ms: start.elapsed().as_millis() as u64,
                    success: false,
                    hit_count: 0,
                    sample_titles: vec![],
                    error_message: Some(e),
                    remediation_tip: Some("Verify your Tavily API quota and key validity".into()),
                }),
            }
        },
        "duckduckgo" => {
            match DuckDuckGoClient::search(q, 3).await {
                Ok(hits) => Ok(ProviderProbeResult {
                    provider,
                    http_status: 200,
                    latency_ms: start.elapsed().as_millis() as u64,
                    success: true,
                    hit_count: hits.len(),
                    sample_titles: hits.iter().map(|h| h.title.clone()).collect(),
                    error_message: if hits.is_empty() { Some("Instant Answer returned 0 topics".into()) } else { None },
                    remediation_tip: if hits.is_empty() { Some("DDG Instant Answer only matches entity terms; general web requires SearXNG or Tavily".into()) } else { None },
                }),
                Err(e) => Ok(ProviderProbeResult {
                    provider,
                    http_status: 500,
                    latency_ms: start.elapsed().as_millis() as u64,
                    success: false,
                    hit_count: 0,
                    sample_titles: vec![],
                    error_message: Some(e.to_string()),
                    remediation_tip: None,
                }),
            }
        },
        "wikipedia" => {
            match WikipediaClient::search(q, 3).await {
                Ok(hits) => Ok(ProviderProbeResult {
                    provider,
                    http_status: 200,
                    latency_ms: start.elapsed().as_millis() as u64,
                    success: true,
                    hit_count: hits.len(),
                    sample_titles: hits.iter().map(|h| h.title.clone()).collect(),
                    error_message: None,
                    remediation_tip: None,
                }),
                Err(e) => Ok(ProviderProbeResult {
                    provider,
                    http_status: 500,
                    latency_ms: start.elapsed().as_millis() as u64,
                    success: false,
                    hit_count: 0,
                    sample_titles: vec![],
                    error_message: Some(e.to_string()),
                    remediation_tip: None,
                }),
            }
        },
        other => Err(format!("Unknown provider: {other}")),
    }
}
```

- [ ] **Step 3: Register command in mod.rs and main.rs**
In `crates/vox-gui/src/commands/mod.rs`, add `pub mod search_probe;`.
In `crates/vox-gui/src/main.rs`, add `commands::search_probe::probe_search_provider` to `tauri::generate_handler![...]`.

- [ ] **Step 4: Verify compilation and formatting**
Run: `cargo check -p vox-gui`
Run: `cargo fmt -p vox-gui`
Expected: Clean compile.

- [ ] **Step 5: Commit Task 2**
```bash
git add crates/vox-gui/src/commands/search_probe.rs crates/vox-gui/src/commands/mod.rs crates/vox-gui/src/main.rs
git commit -m "feat(gui): implement isolated search provider prober command"
```

---

### Task 3: `[SEQUENTIAL]` Core Invariant Sentries (`Honesty` & `ErrorLeak`) Reusing `backendGuard`

**Files:**
- Create: `crates/vox-gui/ui/src/debugger/types.ts`
- Create: `crates/vox-gui/ui/src/debugger/sentries/sentryHonesty.ts`
- Create: `crates/vox-gui/ui/src/debugger/sentries/sentryHonesty.test.ts`
- Create: `crates/vox-gui/ui/src/debugger/sentries/sentryErrorLeak.ts`
- Create: `crates/vox-gui/ui/src/debugger/sentries/sentryErrorLeak.test.ts`

**Interfaces:**
- Consumes: `LEAK_PATTERN` from `crates/vox-gui/ui/src/lib/backendGuard.ts`.
- Produces: `checkHonestyInvariant`, `checkErrorLeakInvariant`.

- [ ] **Step 1: Verify existing error patterns via Preflight Grep**
```bash
rg -n "LEAK_PATTERN" crates/vox-gui/ui/src/lib/backendGuard.ts
```
Expected: line 52.

- [ ] **Step 2: Create types.ts**
Create `crates/vox-gui/ui/src/debugger/types.ts`:
```typescript
export type StepStatus = 'pending' | 'active' | 'paused' | 'completed' | 'failed' | 'skipped';

export interface DebugStep<TInput = unknown, TOutput = unknown> {
  id: string;
  label: string;
  status: StepStatus;
  inputPayload?: TInput;
  outputPayload?: TOutput;
  error?: string;
  timingMs?: number;
}

export type InvariantSeverity = 'critical' | 'major' | 'minor' | 'info';

export interface InvariantViolation {
  kind: 'fake_success' | 'raw_error_leak' | 'occlusion' | 'watchdog_stalled';
  severity: InvariantSeverity;
  message: string;
  elementSelector?: string;
  location?: string;
  rawDetails?: unknown;
}
```

- [ ] **Step 3: Write test and implement sentryHonesty.ts**
Create `crates/vox-gui/ui/src/debugger/sentries/sentryHonesty.ts`:
```typescript
import type { InvariantViolation, StepStatus } from '../types';

export interface HonestyAuditContext {
  status: StepStatus;
  payload: Record<string, unknown> | null | undefined;
  requiredFields: string[];
  stageName: string;
  providerProbeStatuses?: Array<{ provider: string; ok: boolean; httpStatus: number; hitCount: number }>;
  domContainer?: HTMLElement | null;
}

export function checkHonestyInvariant(ctx: HonestyAuditContext): InvariantViolation | null {
  if (ctx.status !== 'completed') return null;

  const emptyField = ctx.requiredFields.find((f) => {
    const val = ctx.payload?.[f];
    return !val || (Array.isArray(val) && val.length === 0);
  });

  if (!emptyField) return null;

  // 1. If providers all failed, this is an infrastructure failure
  const allProvidersFailed =
    ctx.providerProbeStatuses &&
    ctx.providerProbeStatuses.length > 0 &&
    ctx.providerProbeStatuses.every((p) => !p.ok || p.httpStatus >= 400);

  if (allProvidersFailed) {
    return {
      kind: 'fake_success',
      severity: 'critical',
      message: `${ctx.stageName} claimed completion, but all search providers failed. Expected error state.`,
      location: ctx.stageName,
    };
  }

  // 2. Check if the UI honestly rendered an acknowledged empty state
  const hasEmptyNotice = ctx.domContainer?.querySelector('[data-testid="empty-results-notice"]') !== null;
  const isAcknowledged = ctx.payload?.emptyStateAcknowledged === true;

  if (hasEmptyNotice || isAcknowledged) {
    return null; // Valid honest empty search
  }

  // 3. Groundless fake success
  return {
    kind: 'fake_success',
    severity: 'critical',
    message: `${ctx.stageName} completed with zero ${emptyField} without rendering an honest empty-state notice.`,
    location: `${ctx.stageName}.${emptyField}`,
  };
}
```

- [ ] **Step 4: Write test and implement sentryErrorLeak.ts**
Create `crates/vox-gui/ui/src/debugger/sentries/sentryErrorLeak.ts`:
```typescript
import { LEAK_PATTERN } from '../../lib/backendGuard';
import type { InvariantViolation } from '../types';

const SYSTEM_CHROME_SELECTOR = '[data-testid="toast-item"], [role="alert"], [role="status"], header, [data-testid="error-boundary"]';
const EXCLUDE_CONTENT_SELECTOR = '.prose, .markdown-body, pre, code, [data-testid="chat-transcript"], [data-testid="terminal-stream"], [data-testid="inspector-drawer"]';

export function checkErrorLeakInvariant(root: Element = document.body): InvariantViolation[] {
  const violations: InvariantViolation[] = [];
  const systemElements = Array.from(root.querySelectorAll<HTMLElement>(SYSTEM_CHROME_SELECTOR));

  for (const el of systemElements) {
    if (el.closest(EXCLUDE_CONTENT_SELECTOR)) continue;
    const text = el.textContent ?? '';

    if (LEAK_PATTERN.test(text) || /\[object Object\]/.test(text) || /undefined is not a/.test(text) || /TypeError:/.test(text)) {
      violations.push({
        kind: 'raw_error_leak',
        severity: 'critical',
        message: `Raw runtime exception leaked into system chrome: "${text.trim().slice(0, 80)}"`,
        elementSelector: el.tagName.toLowerCase(),
      });
    }
  }
  return violations;
}
```

- [ ] **Step 5: Run tests and verify passing**
Run: `pnpm --dir crates/vox-gui/ui test sentryHonesty.test.ts sentryErrorLeak.test.ts`
Expected: PASS

- [ ] **Step 6: Commit Task 3**
```bash
git add crates/vox-gui/ui/src/debugger/
git commit -m "feat(debugger): implement honesty and scoped error leak sentries"
```

---

### Task 4: `[SEQUENTIAL]` Canonical Pipeline Stepper Hook & Global Inspector Drawer

**Files:**
- Create: `crates/vox-gui/ui/src/debugger/usePipelineStepper.ts`
- Create: `crates/vox-gui/ui/src/debugger/usePipelineStepper.test.ts`
- Create: `crates/vox-gui/ui/src/debugger/InspectorDrawer.tsx`
- Create: `crates/vox-gui/ui/src/debugger/InspectorDrawer.test.tsx`
- Modify: `crates/vox-gui/ui/src/lib/keybinds.ts`
- Modify: `crates/vox-gui/ui/src/App.tsx`

**Interfaces:**
- Consumes: `RESEARCH_STAGES` from `crates/vox-gui/ui/src/lib/pipeline.ts`.
- Produces: `usePipelineStepper`, `InspectorDrawer`, keybind `'toggle-inspector'`.

- [ ] **Step 1: Verify RESEARCH_STAGES via Preflight Grep**
```bash
rg -n "RESEARCH_STAGES" crates/vox-gui/ui/src/lib/pipeline.ts
```
Expected: line 4: `export const RESEARCH_STAGES = ['queued', 'planning', 'retrieving', 'verifying_claims', 'synthesizing', 'auditing_citations', 'persisting_artifact', 'completed'] as const;`

- [ ] **Step 2: Implement usePipelineStepper.ts matching 8 stages**
Create `crates/vox-gui/ui/src/debugger/usePipelineStepper.ts` providing step transitions, active index tracking, `stepNext()`, `pause()`, `resume()`, and `overridePayload()`.

- [ ] **Step 3: Implement InspectorDrawer.tsx as fixed overlay at z-45**
Create `crates/vox-gui/ui/src/debugger/InspectorDrawer.tsx`:
Slide-out drawer mounted fixed on the right (`fixed inset-y-0 right-0 z-45 w-[420px] bg-surface-primary border-l border-border-subtle shadow-2xl`), rendering the 8-stage timeline, two-column JSON payload viewer, and sentry warnings badge.

- [ ] **Step 4: Register keybind in keybinds.ts and mount in App.tsx**
1. In `keybinds.ts`, add `'toggle-inspector': 'Mod+Shift+D'` to `DEFAULT_BINDINGS`.
2. In `App.tsx`, wire `actionHandlers['toggle-inspector'] = () => setIsInspectorOpen(p => !p)`.
3. Render `<InspectorDrawer open={isInspectorOpen} onClose={() => setIsInspectorOpen(false)} />` alongside `DocViewerDrawer` at line 2042.

- [ ] **Step 5: Run tests**
Run: `pnpm --dir crates/vox-gui/ui test usePipelineStepper.test.ts InspectorDrawer.test.tsx`
Expected: PASS

- [ ] **Step 6: Commit Task 4**
```bash
git add crates/vox-gui/ui/src/debugger/usePipelineStepper.* crates/vox-gui/ui/src/debugger/InspectorDrawer.* crates/vox-gui/ui/src/lib/keybinds.ts crates/vox-gui/ui/src/App.tsx
git commit -m "feat(gui): implement 8-stage pipeline stepper hook and mount Inspector Drawer overlay"
```

---

### Task 5: `[SEQUENTIAL]` Embed Live Source Prober and Diagnostic Mode in `ResearchView.tsx`

**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/Research/LiveSourceProber.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/Research/LiveSourceProber.test.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/Research/JudgeInspector.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/Research/JudgeInspector.test.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx`

**Interfaces:**
- Consumes: `probe_search_provider` Tauri command.
- Produces: Inline diagnostic toolbar and prober inside `ResearchView.tsx`.

- [ ] **Step 1: Preflight Grep on ResearchView.tsx**
```bash
rg -n "export function ResearchView" crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx
```
Expected: line 76.

- [ ] **Step 2: Implement LiveSourceProber.tsx**
Create `LiveSourceProber.tsx`: Input query text, selector for SearXNG, Tavily, DDG, Wikipedia, and "Probe" button invoking `probe_search_provider`. Renders latency badge, HTTP code, hit count, and remediation tips.

- [ ] **Step 3: Implement JudgeInspector.tsx**
Create `JudgeInspector.tsx`: Displays the judge rubric sub-scores (Factual Accuracy, Citation Density, Coverage) and the full chain-of-thought rationale.

- [ ] **Step 4: Embed Diagnostic Controls in ResearchView.tsx**
In `ResearchView.tsx`: Add a `"Diagnostic Prober"` toggle button in the header (adjacent to `Publish Architecture SSOT` and `Sandbox REPL`) that expands `LiveSourceProber` and `JudgeInspector`.

- [ ] **Step 5: Run tests**
Run: `pnpm --dir crates/vox-gui/ui test LiveSourceProber.test.tsx JudgeInspector.test.tsx ResearchView.test.tsx`
Expected: PASS

- [ ] **Step 6: Commit Task 5**
```bash
git add crates/vox-gui/ui/src/components/surfaces/Research/LiveSourceProber.* crates/vox-gui/ui/src/components/surfaces/Research/JudgeInspector.* crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx
git commit -m "feat(gui): embed live source prober and judge inspector into ResearchView"
```

---

### Task 6: `[SEQUENTIAL]` Deterministic Playwright Stepper Suite & Policy Enforcement

**Files:**
- Modify: `crates/vox-gui/ui/e2e/lib/tauriMockRich.ts`
- Modify: `crates/vox-gui/ui/e2e/lib/tauriMockShared.ts`
- Modify: `crates/vox-gui/ui/e2e/review/states.ts`
- Create: `crates/vox-gui/ui/e2e/review/stepper.spec.ts`
- Modify: `.gitattributes`
- Modify: `AGENTS.md`

**Interfaces:**
- Produces: Deterministic IPC sentinel, multi-step Playwright test, git diff hygiene, normative policy.

- [ ] **Step 1: Add mock in tauriMockRich.ts and IPC counter in tauriMockShared.ts**
1. In `tauriMockRich.ts`, add:
```typescript
case 'probe_search_provider':
  return {
    provider: (args as any)?.provider ?? 'searxng',
    http_status: 200,
    latency_ms: 85,
    success: true,
    hit_count: 3,
    sample_titles: ['Title 1', 'Title 2', 'Title 3'],
    error_message: null,
    remediation_tip: null,
  };
```
2. In `tauriMockShared.ts`, track `window.__VOX_IPC_ACTIVE_COUNT__` on each invoke start/end.

- [ ] **Step 2: Add prober state to states.ts**
In `SURFACE_STATES['research']`, add:
```typescript
{
  name: 'prober-open',
  setup: async (p) => {
    const btn = p.getByRole('button', { name: /diagnostic prober/i });
    if (await btn.isVisible()) await btn.click();
  },
}
```

- [ ] **Step 3: Create stepper.spec.ts**
Create `crates/vox-gui/ui/e2e/review/stepper.spec.ts`:
Loads `/`, navigates to Research view, toggles Diagnostic Prober, executes a probe, asserts 200 OK badge and hit counts, and takes a full-resolution viewport capture saved into `review-bundle/latest/`.

- [ ] **Step 4: Update .gitattributes and AGENTS.md**
1. In `.gitattributes`, add:
```gitattributes
contracts/reports/gui-visual-review/bundle-cache.v1.json linguist-generated=true -diff
contracts/reports/gui-visual-review/bundle-digest.md     linguist-generated=true -diff
```
2. In `AGENTS.md`, add the normative `GUI Visual Verification Invariant` policy.

- [ ] **Step 5: Run Playwright test**
Run: `pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/stepper.spec.ts --project=chromium`
Expected: PASS, screenshot saved in `review-bundle/latest/`.

- [ ] **Step 6: Commit Task 6**
```bash
git add crates/vox-gui/ui/e2e/ .gitattributes AGENTS.md
git commit -m "test(e2e): implement deterministic playwright stepper spec and register normative visual policy"
```
