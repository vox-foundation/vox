# HITL Interactive Research Debugger & Mathematical Observability Walkthrough Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build, launch, and execute a Human-In-The-Loop (HITL) interactive visual debugging walkthrough of Axis Deep Research via Playwright, driving the application purely through the GUI, capturing high-resolution screenshots at every stage, inspecting raw in/out data payloads, and explaining the mathematical algorithms powering each automated research phase.

**Architecture:** A lightweight Playwright execution driver (`crates/vox-gui/ui/e2e/hitl/research-hitl-driver.ts`) interacts with the Vite/Tauri application running under deterministic mock instrumentation (`tauriMock.ts`). The driver executes one discrete research stage at a time, emits high-resolution screenshots to `crates/vox-gui/ui/review-bundle/hitl/stage-{0..5}.png`, extracts the raw step payloads directly from the DOM and Inspector Drawer state, and yields control in chat for human evaluation and algorithmic verification.

**Tech Stack:** React 19, TypeScript, Vite, Tailwind CSS v4, Playwright, Vitest, Tauri v2 IPC mocks.

**Spec:** `docs/superpowers/specs/2026-09-17-axis-gui-visual-debugger-deep-research-design.md`

## Global Constraints
- GUI interactions must occur exclusively through standard user input mechanisms (clicks, keyboard input, keyboard shortcuts `Mod+Shift+D`), never bypassing UI state via hidden global hacks.
- Screenshots must be captured at high resolution (viewport 1440x900) into `crates/vox-gui/ui/review-bundle/hitl/` with `.gitattributes` `linguist-generated=true -diff` enforcement.
- Every stage must assert both visual presentation and functional DOM state (e.g. data test IDs, active stage badges, latency/hit numbers, claim verdict counts).
- Raw data payloads must be extracted from the Inspector Drawer's JSON viewers and evaluated alongside the mathematical equations governing that phase.

---

### Task 1: `[SEQUENTIAL]` HITL Playwright Execution Driver Setup

**Files:**
- Create: `crates/vox-gui/ui/e2e/hitl/research-hitl-driver.ts`
- Create: `crates/vox-gui/ui/e2e/hitl/hitl.spec.ts`

**Interfaces:**
- Produces: `executeHitlStage(stageNumber: number)`, stage screenshot artifacts in `review-bundle/hitl/`.

- [ ] **Step 1: Create HITL Driver helper**
Create `crates/vox-gui/ui/e2e/hitl/research-hitl-driver.ts` with helper functions:
- `launchResearchSession(page)`: Injects mock, navigates to `/`, opens Research surface, toggles prober and inspector drawer.
- `stepToStage(page, targetStageIndex)`: Uses stepper controls (`getByTestId('stepper-step-next-btn')` or `data-testid="inspector-drawer"`) to advance execution.
- `captureStageArtifact(page, stageName, stageIndex)`: Awaits `window.__VOX_IPC_ACTIVE_COUNT__ === 0` and captures full viewport PNG to `crates/vox-gui/ui/review-bundle/hitl/stage-${stageIndex}-${stageName}.png`.
- `extractCurrentPayloads(page)`: Reads text from `[data-testid="input-payload-view"]` and `[data-testid="output-payload-view"]`.

- [ ] **Step 2: Create HITL Playwright Specification**
Create `crates/vox-gui/ui/e2e/hitl/hitl.spec.ts`:
- Contains test cases for Stage 0 (Initial Mount & Prober), Stage 1 (Planning), Stage 2 (Retrieval), Stage 3 (Claim Extraction), Stage 4 (Epistemic Judge), and Stage 5 (Synthesis & Invariant Sentry).

- [ ] **Step 3: Verify Playwright driver executes**
Run: `pnpm --dir crates/vox-gui/ui test:e2e e2e/hitl/hitl.spec.ts --project=chromium`
Expected: PASS, screenshots generated in `review-bundle/hitl/`.

- [ ] **Step 4: Commit Task 1**
```bash
git add crates/vox-gui/ui/e2e/hitl/
git commit -m "test(hitl): create interactive playwright driver for research visual debugger"
```

---

### Task 2: `[SEQUENTIAL]` Stage 0 & 1: Mount, Diagnostic Prober & Planning Decomposition

**Focus:**
- **GUI Operations:**
  - Launch GUI, verify navigation to Knowledge -> Research.
  - Expand "Diagnostic Prober", submit probe query `"hybrid search architecture"`, verify 200 OK badges across SearXNG, Tavily, DDG, and Wikipedia.
  - Open Pipeline Inspector Drawer via `Mod+Shift+D`.
  - Advance stepper to Stage 1: `planning`.
- **Data Inspected:**
  - Input: Raw user query `query: "hybrid search architecture"`, `max_sources: 5`, `scope: "web"`.
  - Output: Decomposed subqueries, entity keywords, CRAG routing confidence threshold.
- **Mathematical Process Explained:**
  - Query Entropy: $H(Q) = -\sum p(w) \log_2 p(w)$
  - BM25 Term Weighting for Subquery Expansion:
    $$\text{IDF}(q_i) = \ln\left( \frac{N - n(q_i) + 0.5}{n(q_i) + 0.5} + 1 \right)$$
  - CRAG Confidence Gating:
    $$\text{Route} = \begin{cases} \text{DirectSynthesize}, & \text{if } \gamma > 0.85 \\ \text{WebRetrieval}, & \text{if } 0.40 \le \gamma \le 0.85 \\ \text{RefinementDecomposition}, & \text{if } \gamma < 0.40 \end{cases}$$
- **Presentation:** Capture screenshot `stage-1-planning.png`, display in chat, report in/out JSON, await user observability ruling.

---

### Task 3: `[SEQUENTIAL]` Stage 2: Multi-Provider Retrieval & Reciprocal Rank Fusion

**Focus:**
- **GUI Operations:**
  - In Stepper, click `Step Next` to transition from `planning` to `retrieving`.
  - Verify active badge updates on Stage 3 of 8 (`retrieving`).
  - Inspect retrieved hits in `LiveSourceProber` and Payloads view.
- **Data Inspected:**
  - Input: 3 decomposed subqueries from Stage 1.
  - Output: 8 retrieved hits with URLs, snippets, provider provenance, and deduplication map.
- **Mathematical Process Explained:**
  - Reciprocal Rank Fusion (RRF) Ranking:
    $$RRF(d) = \sum_{m \in M} \frac{1}{k + r_m(d)}, \quad k = 60$$
  - Jaccard & MinHash Novelty Filter:
    $$J(S_i, S_j) = \frac{|S_i \cap S_j|}{|S_i \cup S_j|} < \theta_{\text{novelty}}$$
- **Presentation:** Capture screenshot `stage-2-retrieving.png`, display in chat, report raw hits JSON, await user observability ruling.

---

### Task 4: `[SEQUENTIAL]` Stage 3: Atomic Claim Extraction & Entailment Grounding

**Focus:**
- **GUI Operations:**
  - In Stepper, click `Step Next` to transition to `verifying_claims`.
  - Toggle claim accordion in ResearchView to view extracted assertions.
  - Inspect claim verification statuses (Supported, Contested, Refuted).
- **Data Inspected:**
  - Input: Retrieved document texts and provenance URLs.
  - Output: Array of extracted claims with `claim_id`, `text`, `verdict`, `confidence`, `citation_urls`.
- **Mathematical Process Explained:**
  - Natural Language Inference (NLI) Cross-Entropy:
    $$P(\text{entailment} \mid D_{\text{premise}}, C_{\text{hypothesis}}) = \frac{e^{z_{\text{entail}}}}{\sum_{j} e^{z_j}}$$
  - Citation Precision:
    $$\text{Precision}_{\text{cite}} = \frac{|\{c \in C \mid \text{verdict}(c) = \text{Supported} \land \text{citations}(c) \neq \emptyset\}|}{|C|}$$
- **Presentation:** Capture screenshot `stage-3-verifying-claims.png`, display in chat, report extracted claim table, await user observability ruling.

---

### Task 5: `[SEQUENTIAL]` Stage 4: Epistemic Judge & Rubric Transparency

**Focus:**
- **GUI Operations:**
  - Advance stepper to `synthesizing` and `auditing_citations`.
  - Toggle "Judge Inspector" (`data-testid="toggle-judge-btn"`) in ResearchView.
  - Inspect Confidence Tier badge (`DeepResearch`), Source Count (`3`), Citation Precision (`1.0`), verdict breakdown pills, and full chain-of-thought rationale.
- **Data Inspected:**
  - Input: Full claim corpus and citation graph.
  - Output: Epistemic rubric sub-scores (Factual Accuracy: 0.95, Citation Density: 1.0, Grounding Coverage: 0.92) and synthesized rationale.
- **Mathematical Process Explained:**
  - Bayesian Confidence Rating with Dirichlet Conjugate Prior:
    $$P(\theta \mid \mathbf{\alpha}) = \frac{1}{\text{B}(\mathbf{\alpha})} \prod_{i=1}^K \theta_i^{\alpha_i - 1}$$
  - Contradiction Entropy:
    $$H(\text{Claims}) = - \sum_{v \in \{\text{Supp}, \text{Cont}, \text{Ref}\}} p(v) \log p(v)$$
- **Presentation:** Capture screenshot `stage-4-judge-inspector.png`, display in chat, report rubric breakdowns, await user observability ruling.

---

### Task 6: `[SEQUENTIAL]` Stage 5: Artifact Persistence & Zero-Tolerance Sentry Audit

**Focus:**
- **GUI Operations:**
  - Advance stepper to `completed`.
  - In Inspector Drawer, click `Audit` button (`data-testid="stepper-audit-btn"`) to trigger `checkHonestyInvariant` and `checkErrorLeakInvariant`.
  - Switch to "Violations" tab in Inspector Drawer.
  - Assert zero invariant violations ("Pipeline sentries report clean state").
- **Data Inspected:**
  - Input: Entire completed session payload and DOM container.
  - Output: Sentry audit report: `fake_success` = 0, `raw_error_leak` = 0, `occlusion` = 0, `watchdog_stalled` = 0.
- **Mathematical Process Explained:**
  - Zero-Tolerance Honesty Invariant Criterion:
    $$\forall s \in \text{Stages}, \quad \left( \text{status}(s) = \text{Completed} \implies \left( |\text{hits}(s)| > 0 \lor \text{AckEmpty}(s) \right) \right)$$
  - Hallucination Rate Lower Bounding:
    $$\text{Error}_{\text{hallucination}} \le \epsilon \iff \text{AllClaimsGrounding}(C, S) \land \neg \text{ProviderOutage}(P)$$
- **Presentation:** Capture screenshot `stage-5-sentry-audit.png`, display in chat, report audit verdict, await user final sign-off.

