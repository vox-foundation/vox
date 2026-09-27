---
title: "Deep Research System Audit, CLI/GUI Parity, and Agent Handoff (2026)"
description: "Empirical audit of vox research run degradation on complex queries, exhaustive CLI vs GUI parity comparison, and architectural remediation handoff for Claude."
category: "Architecture SSOTs"
status: "research"
training_eligible: true
training_rationale: "Documents empirical failure modes in query decomposition, search fallback, free-tier model routing, and CLI/GUI research surfaces."
---

# Deep Research System Audit, CLI/GUI Parity, and Agent Handoff (2026)

**Audited By:** Antigravity (Gemini 3.1 Pro / 3.8 Flash)  
**Target Agent:** Claude (Code / Opus / Sonnet)  
**Date:** September 20, 2026  
**Context:** Empirical evaluation of `vox research run` against a real-world multi-constraint research query (aggregating Tucson local tech/STEM events for an app modeled after `nerdyorkcity.com`), compared against manual agentic subagent decomposition and the Vox GUI surface.

---

## 1. Executive Summary

When tasked with finding high-value, regularly updated Tucson event sources for a custom aggregator application:
1. **Antigravity Manual Subagent Approach:** Deployed three focused subagents (`Tech Event Evaluator`, `University Event Evaluator`, `General Event Evaluator`) using broad queries + targeted URL inspection. Successfully identified 8 viable sources, reverse-engineered their CMS platforms (BLOX, WordPress/The Events Calendar, Ctykit, Granicus, Wix, Trellis), located live REST/iCal endpoints, and evaluated anti-bot protections.
2. **Vox Native Pipeline (`vox research run`):** Completely collapsed. It generated 14 citations consisting of random Wikipedia pages (e.g., *2018 in American television*, *Central Synagogue Manhattan*, *Beatrice Irwin's 1952 Tucson obituary*, *Meanings of minor-planet names*) alongside local Vox Rust repository test files (`repo://crates/...`). Despite producing zero relevant event aggregators, the system assigned itself a `quality_score=80`.

Below is the exhaustive architectural post-mortem, feature parity matrix between the Research GUI and CLI, and targeted instructions for Claude to audit and remediate the pipeline.

---

## 2. CLI vs. GUI Feature Parity Analysis

A side-by-side inspection of the CLI (`crates/vox-cli-research/src/lib.rs`) and GUI (`crates/vox-gui/src/commands/research.rs` & `crates/vox-gui/ui/src/components/surfaces/Research/`) reveals **substantial feature divergence**:

| Feature / Capability | CLI (`vox research`) | GUI (`ResearchView.tsx` / `research.rs`) | Parity Status |
| :--- | :--- | :--- | :--- |
| **Research Lanes** (`⚡ Fast` vs `🔬 Deep`) | ❌ **Missing.** Hardcoded to `ResearchLane::default()` in `run_research_query`. | ✅ **Supported.** Toggleable tab button passing `lane: 'fast' \| 'deep'`. | **CLI Deficit** |
| **Multi-Wave Configuration** | ❌ **Missing.** Hardcoded to `waves: 1` in `run_research_query`. | ✅ **Supported.** Accepts `waves: Option<u32>` and visualizes via `MultiWaveProgressTimeline`. | **CLI Deficit** |
| **Domain Mode** (`general`, `academic`, etc.) | ❌ **Missing.** Hardcoded to `domain_mode: Default::default()`. | ✅ **Supported.** Passed as `domain_mode: Option<String>` to `start_research_async`. | **CLI Deficit** |
| **Async Execution Model** | ⚠️ **Optional via `--async`.** Requires running `vox-orchestrator-d` TCP daemon. Synchronous inline by default. | ✅ **Default.** Always dispatches fire-and-forget via persistent daemon + `listenScientiaQueue` watcher. | **Asymmetric** |
| **Search Engine Drawer & Config** | ❌ **Missing.** No CLI commands to view/save provider settings or timeouts. | ✅ **Supported.** `ResearchEngineDrawer` manages provider toggles, timeouts, and API keys. | **CLI Deficit** |
| **Live Source Probing** | ❌ **Missing.** Only has `vox research status` (static ping). | ✅ **Supported.** `LiveSourceProber` can test arbitrary queries against specific or all engines with latency. | **CLI Deficit** |
| **Quota & Free Tier Discovery** | ❌ **Missing.** | ✅ **Supported.** Exposes `FreeTierOffer` and `QuotaUsageDto` in drawer. | **CLI Deficit** |
| **DAG Visualization** | ❌ **Missing.** | ✅ **Supported.** Interactive node graph via `ResearchDagCanvas`. | **GUI Exclusive** |
| **Claim Corroboration & Inspection** | ⚠️ Plain text or raw JSON only (`--verify-claims`). | ✅ **Supported.** Interactive `ResearchClaimAccordion`, distinct-domain trust chips, `JudgeInspector`. | **CLI Deficit** |
| **Code Sandbox Probing** | ❌ **Missing.** No CLI command to run sandbox probes against claims. | ✅ **Supported.** `SandboxReplModal` / `execute_sandbox_probe` (Python, TS, SQL, Rust, Vox). | **CLI Deficit** |
| **Misguidance / Defect Flagging** | ❌ **Missing.** No `vox research flag` command. | ✅ **Supported.** `MisguidanceFlagModal` / `flag_research_misleading` updates domain penalty in VoxDb. | **CLI Deficit** |
| **Architecture SSOT Publishing** | ⚠️ `vox research result --output` exports raw markdown/JSON. | ✅ **Supported.** `DocPublishModal` scaffolds full Architecture SSOT with frontmatter and indexes to DB. | **CLI Deficit** |
| **Sidecar Management** (`up` / `down`) | ✅ **Supported.** `vox research up` / `down` manages SearXNG Docker container. | ❌ **Missing in GUI.** No UI button to launch Docker sidecar. | **GUI Deficit** |
| **Pre-execution Plan Preview** | ✅ **Supported.** `vox research preview <query>` previews subqueries. | ❌ **Missing in GUI.** Executes directly without interactive subquery editing. | **GUI Deficit** |
| **Golden Eval Harness** | ✅ **Supported.** `vox research eval --queries ... --concurrency N`. | ❌ **Missing in GUI.** Headless benchmark tool. | **CLI Exclusive (By Design)** |
| **Watch Progress Terminal** | ✅ **Supported.** `vox research watch <session_id>`. | ❌ **Replaced by reactive GUI polling.** | **Parity Achieved** |

### Critical Governance Bug Found in GUI
In [`crates/vox-gui/src/commands/research.rs`](../../../crates/vox-gui/src/commands/research.rs) (at the time of the audit, lines 258–269):
`publish_research_doc` still invokes `update_research_index_md(&index_path_clone, ...)` to write to `docs/src/architecture/research-index.md`.
**Direct violation of [`AGENTS.md`](../../../AGENTS.md):**
> *"Do not create or edit `docs/src/architecture/research-index.md` (retired; snapshot under `docs/src/archive/`)."*

---

## 3. Empirical Root-Cause Analysis of Pipeline Failure

Why did `vox research run` produce junk citations for the Tucson events query?

```text
[Query submitted]
   │
   ▼
[Stage 1: Planner Cascade] ──(Fails or returns invalid JSON)──► [Degrades to PassthroughPlan]
                                                                      │ (passes 50-word raw prompt)
   ┌──────────────────────────────────────────────────────────────────┘
   ▼
[Stage 2: Web & Local Gather]
   ├─► Local Scope (`repo://`): Matches tokens "event", "aggregate", "test" in Vox code
   └─► Web Scope (DuckDuckGo): SearXNG OFFLINE, Tavily UNCONFIGURED
         └─► DDG keyword search choked on 50 conversational tokens:
               "Find all high-value sources of data for events in Tucson that are updated..."
               Matched: "events" (NYC Star Trek 2018), "Tucson" (Beatrice Irwin 1952)
   │
   ▼
[Stage 3: Synthesis Cascade]
   ├─► Candidate `microsoft/phi-3-mini-128k-instruct:free`
   │     └─► HTTP 404: "No endpoints found for microsoft/phi-3-mini-128k-instruct:free"
   ▼
[Stage 4: Synthesis Fallback]
   └─► `generate_template_fallback_synthesis()`:
         Dumps raw top 5 hit snippets verbatim under "## Evidence Summary"
   │
   ▼
[Stage 5: Quality Judge]
   └─► Scores garbage as `quality_score=80` (false positive / rubric blindness)
```

### Detailed Breakdown

1. **Planner Cascade Failure (`crates/vox-research-shim/src/research/planner.rs`):**
   `decompose_query_with_config` asks an LLM cascade to decompose the query into 3–6 subqueries. If the model fails or returns non-JSON, line 83 falls back to `passthrough_plan(query, degraded)`. The entire 50-word conversational prompt was passed directly into downstream search engines.
2. **Search Backend Vulnerability & Choke (`crates/vox-search`):**
   - `vox research status` confirmed:
     - `SearXNG: OFFLINE (requires 'vox research up')`
     - `DuckDuckGo: ONLINE`
     - `Tavily: NOT CONFIGURED (Optional)`
   - DuckDuckGo's HTML/lite backend cannot parse complex, conversational natural language prompts. It treats the 50 words as a bag of keywords, returning Wikipedia articles that coincidentally contained isolated words like "events", "Tucson", or "NYC".
3. **Unscoped Hybrid Contamination:**
   `vox research run` defaults to `scope: "both"` (`ResearchScope::Both`). Without a specific code intent, local Tantivy and Qdrant indexed repositories injected local code snippets (`repo://crates/vox-publisher/...`, `repo://crates/vox-plugin-mens-candle-metal/...`) into the evidence bundle.
4. **Dead Free-Tier Model Endpoint (`crates/vox-research-shim/src/selection/`):**
   Synthesis called OpenRouter for `microsoft/phi-3-mini-128k-instruct:free`, which failed:
   `LLM API returned error (404): {"error":{"message":"No endpoints found for microsoft/phi-3-mini-128k-instruct:free."}}`
   The OpenRouter free-tier catalog changed upstream, but the hardcoded candidate list in Vox retained the dead slug.
5. **Silent Template Fallback (`crates/vox-research-shim/src/research/orchestrator/stages.rs`):**
   When synthesis cascade exhausted, line 335 triggered `generate_template_fallback_synthesis`. It did not synthesize anything; it formatted the raw, irrelevant DuckDuckGo hit snippets into markdown headings.
6. **Hallucinatory Judge Score:**
   The output reported `(routing_tier=Light, sources=14, quality_score=80)`. A rubric that awards 80/100 to an article citing 1952 obituaries and 2018 Star Trek Facebook events for a 2026 local event scraping query represents an epistemic hazard for autonomous workflows.

---

## 4. Handoff Directives for Claude (What to Fix & Where to Look)

Claude should systematically audit and implement the following remediations:

### Task 1: CLI Parity Upgrades
*   **File:** [`crates/vox-cli-research/src/lib.rs`](../../../crates/vox-cli-research/src/lib.rs)
*   **Action:**
    1. Add `--lane <fast|deep>` to `ResearchCmd::Run` (wire to `rq.lane`).
    2. Add `--waves <N>` to `ResearchCmd::Run` (wire to `rq.waves`).
    3. Add `--domain-mode <MODE>` to `ResearchCmd::Run` (wire to `rq.domain_mode`).
    4. Expose diagnostic commands in CLI matching GUI:
       - `vox research probe <query> [--provider <name>]` (call `vox-search` prober).
       - `vox research flag <session_id> --url <url> --defect <class>` (call `record_research_misguidance`).
       - `vox research publish <session_id> [--slug <slug>]` (publish clean Architecture SSOT without touching `research-index.md`).

### Task 2: Purge `research-index.md` Call in GUI
*   **File:** [`crates/vox-gui/src/commands/research.rs`](../../../crates/vox-gui/src/commands/research.rs)
*   **Action:** Remove the call to `update_research_index_md`. Starlight's Astro build automatically derives navigation from frontmatter `category` and `title`. Writing to `research-index.md` generates dirty workspace drift.

### Task 3: Robust Query Decomposition & Sanitization
*   **File:** [`crates/vox-research-shim/src/research/planner.rs`](../../../crates/vox-research-shim/src/research/planner.rs)
*   **Action:**
    - In `passthrough_plan`, do NOT send a 50-word conversational prompt directly to web search.
    - If the LLM planner cascade fails, apply an algorithmic query condenser (strip stop words, extract proper nouns, or take the first 6–8 high-entropy tokens) before dispatching to keyword search engines like DuckDuckGo.

### Task 4: OpenRouter Free-Tier Model Pruning
*   **File:** [`crates/vox-research-shim/src/selection/free_tier.rs`](../../../crates/vox-research-shim/src/selection/free_tier.rs) & [`crates/vox-actor-runtime/src/llm/cascade.rs`](../../../crates/vox-actor-runtime/src/llm/cascade.rs)
*   **Action:**
    - Remove `microsoft/phi-3-mini-128k-instruct:free` (endpoint 404s).
    - Update the free tier router allowlist to currently active OpenRouter free models (e.g., `google/gemma-3-27b-it:free`, `meta-llama/llama-3.3-70b-instruct:free`, `qwen/qwen-2.5-72b-instruct:free`).

### Task 5: Epistemic Honesty in Template Fallback & Judge Scoring
*   **File:** [`crates/vox-research-shim/src/research/orchestrator/stages.rs`](../../../crates/vox-research-shim/src/research/orchestrator/stages.rs)
*   **Action:**
    - If synthesis falls back to template due to LLM error, the output must clearly state:  
      `> ⚠️ WARNING: LLM Synthesis cascade failed. Showing unverified raw evidence snippets.`
    - If `evidence_text` is ungrounded or contradictory to the query intent, the judge must cap `quality_score` at `<= 20` rather than defaulting to `80`.

---

## 5. Verification Commands for Claude

Run these to verify the fixes:

```bash
# 1. Verify CLI argument compilation and new flags
cargo test -p vox-cli-research

# 2. Verify research parity test passes
cargo run -p vox-cli -- audit research-parity

# 3. Test research status
cargo run -p vox-cli -- research status

# 4. Run test research with deep lane and web-only scope
cargo run -p vox-cli -- research run "Tucson tech events" --scope web --lane deep
```

---

## 6. Verification Addendum (Claude, 2026-09-20)

Sections 1–5 above are preserved as the original audit. Checking them against the code found the corrections below; where they conflict, this section wins.

### 6.1 Corrections to the root-cause analysis (§3)

| Audit claim | What the code shows |
| :--- | :--- |
| The planner cascade failed and degraded to `PassthroughPlan`. | On `ResearchLane::Fast` (the CLI default) `orchestrator/pipeline.rs` **skips the planner entirely**. The raw query became the single subquery with `planner_degraded: false`. The planner never ran, so `--lane deep` is the primary remedy and the condenser is a backstop. |
| The GUI supports Fast/Deep lanes (§2). | The GUI passed `lane`, but the daemon's `research_run` / `research_start` (`vox-orchestrator-mcp/src/memory_tools/handlers_memory.rs`) hardcoded `ResearchLane::default()`. The toggle was silently ignored end to end until `lane` was added to `ResearchRunParams` / `ResearchStartParams` (`d24242d2b`). |
| The judge "scored garbage 80" because of rubric blindness. | `judge_max_tokens` was 16, too small for the judge's JSON schema, so parsing always failed and `judge_quality` returned `fallback_quality_score` (`DEFAULT_MIN_REVIEW_FINDING_CONFIDENCE` = 80). The 80 was a constant, never a judgment. |
| Only `phi-3-mini` is dead (§4). | **Every** slug in `OPENROUTER_FREE_FALLBACK_MODELS` was absent from the live catalog on 2026-09-20. The audit's suggested replacements (`gemma-3-27b`, `llama-3.3-70b`, `qwen-2.5-72b`) are dead too, and `Qwen 2.5` is a retired family (see `AGENTS.md`). |
| Free-model list lives in `selection/free_tier.rs` / `cascade.rs`. | The single SSOT is `crates/vox-config/src/bootstrap_inference.rs`; `vox-gamify` aliases it and `cascade.rs` only consumes it. |
| `vox audit research-parity` verifies CLI/GUI parity (§5). | It only checks that test files cited in architecture docs exist. It says nothing about CLI/GUI parity. |
| `vox research probe` can call "the `vox-search` prober" (Task 1). | The prober lived inside the Tauri command module in `vox-gui`. It now lives in `vox_search::probe` and is shared by the GUI wrappers and `vox research probe`. |

### 6.2 Remediation status

Each row was re-checked against `main` on 2026-09-27 with `git log -S` / `git grep`.

| Task | Status | Outcome | Commits |
| :--- | :--- | :--- | :--- |
| 1. CLI parity | Fixed | Added `run --lane/--waves/--domain-mode`, `probe`, `flag` and `publish`. `publish` writes the stored report with real frontmatter (`status: "research"`). It does not use the GUI's `generate_research_doc_draft` scaffold, which still emits hardcoded placeholders (`stability_score: 0.90`, `status: "current"`, empty claims). The prober moved into `vox_search::probe`. The daemon now honors `lane` instead of hardcoding `ResearchLane::default()`, and the MCP input schemas advertise it. | `36b82855d` (CLI), `c6aedfd0c` (`vox_search::probe`), `d24242d2b` (daemon lane), `6e9b79286` (MCP schemas) |
| 2. `research-index.md` | Fixed | The GUI no longer writes it; `update_research_index_md` and its tests were deleted. | `d70ea497a` (GUI), `452ab17bc` (vox-db) |
| 3a. Query condensing, planner failure | Fixed | `planner::condense_query` (≤ 8 tokens, proper nouns first) is applied in `planner::passthrough_plan`, which `decompose_query_with_config` returns when the Deep-lane planner cascade fails or returns invalid JSON. | `6fde144d2` |
| 3b. Query condensing, Fast lane | **Open** | Not true on `main`. On `ResearchLane::Fast` (the CLI default), `orchestrator/pipeline.rs` builds its own `ResearchPlan` with `subqueries: vec![query.query.clone()]`. It does not call `passthrough_plan` or `condense_query`, so a long conversational prompt still goes to web search verbatim. Fix: build the Fast-lane plan with `planner::passthrough_plan(&query, false)`. | — |
| 4. Free-tier list | Fixed | `OPENROUTER_FREE_FALLBACK_MODELS` in `crates/vox-config/src/bootstrap_inference.rs` now holds slugs checked against the live catalog on 2026-09-20; re-verify with the `curl` recipe in the constant's doc comment. | `0d509adf5` |
| 5a. Template-fallback warning and cap | Fixed | The template fallback leads with `TEMPLATE_FALLBACK_WARNING`, and its quality score is capped at `TEMPLATE_FALLBACK_QUALITY_CAP` (20). | `6fde144d2` |
| 5b. Judge budget and outage score | Fixed | `judge_max_tokens` is 512; the judge-outage `fallback_quality_score` is 40, below the ≥ 50 persistence gates. | `6fde144d2` |
| 5c. Skip the judge on template fallback | **Open** | Not true on `main`. `orchestrator/pipeline.rs` still calls `judge_quality` on a template-fallback answer and only caps the result afterwards. The score is right, but the judge call is wasted. | — |

### 6.3 Verification commands (corrected)

```bash
cargo test -p vox-research-shim -p vox-cli-research
cargo test -p vox-search --lib probe
cargo run -p vox-cli -- research run "tucson tech events" --scope web --lane deep
cargo run -p vox-cli -- research probe "tucson tech events"
```
