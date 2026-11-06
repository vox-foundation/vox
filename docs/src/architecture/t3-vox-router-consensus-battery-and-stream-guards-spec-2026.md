---
title: "T3-to-Vox Domain 2 Implementation Spec: RepetitionGuard, ToolRefusalDetector, Battery Ledger & Multi-Model Consensus (2026)"
description: "Detailed Rust and TypeScript implementation specification for Phases 218-220: O(1) 1024-slot ring-buffer RepetitionGuard, ToolRefusalDetector auto-retry, Claude ENABLE_TOOL_SEARCH cache-write fix, Battery Unit pre-flight gate & 3-tier cost ledger, Multi-Model Consensus SSE, and ResponseQualityAnalyzer."
category: "Architecture SSOTs"
status: "current"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-7"
---

# T3-to-Vox Domain 2 Implementation Spec: `RepetitionGuard`, `ToolRefusalDetector`, Battery Ledger & Multi-Model Consensus (2026)

<!-- kb:summary -->
This document specifies the exact Rust and TypeScript implementation for `Phases 218–220` (`T3-GUARD-REFUSAL-01..04`, `T3-BATTERY-LEDGER-01..04`, and `T3-CONSENSUS-QA-01..04`) of Milestone `v4.1`. It details the 1024-slot power-of-2 ring-buffer `RepetitionGuard` (`NotT3Chat`), `ToolRefusalDetector` auto-retry (`pi-t3chat`), MCP tool-search prompt-cache bloat mitigation (`t3code` `ClaudeAdapterV2`), the Battery Unit (`1 BU = $0.001`) pre-flight balance gate and 4-step transactional usage ledger (`omnichat`), `<think>`/`<WebSearch>` context sanitization, parallel Multi-Model Consensus SSE fan-out (`lulkebit/t3-cloneathon`), and the Flesch-Kincaid `ResponseQualityAnalyzer`.
<!-- /kb:summary -->

## 1. Phase 218: `RepetitionGuard`, `ToolRefusalDetector` & Cache-Write Guard (`T3-GUARD-REFUSAL-01..04`)

### 1.1 O(1) 1024-Slot Ring-Buffer `RepetitionGuard` (`crates/vox-llm-egress/src/repetition_guard.rs`)
- **Source Inspiration:** `shaltielshmid/NotT3Chat` `backend/Program.cs:1080-1145` (`RepetitionGuard`).
- **Legacy Code Replaced:** Unbounded token accumulation in `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/salvage_and_stream.rs:70-134` that streams degenerate model loops until `max_tokens` is exhausted.
- **Implementation Design:**
  ```rust
  pub const REPETITION_BUFFER_SIZE: usize = 1024;
  pub const REPETITION_MODULO_MASK: usize = REPETITION_BUFFER_SIZE - 1;
  pub const MAX_SINGLE_TOKEN_REP: usize = 32;
  pub const MIN_NGRAM_PERIOD: usize = 3;
  pub const MAX_NGRAM_PERIOD: usize = 12;
  pub const MIN_NGRAM_REPETITIONS: usize = 5;

  pub struct RepetitionGuard {
      ring: [u64; REPETITION_BUFFER_SIZE],
      count: usize,
      consecutive_same: usize,
      last_hash: u64,
  }
  ```
  - On each non-whitespace token/chunk (`push_token(token: &str) -> Result<(), RepetitionLoopError>`):
    1. Computes `hash = vox_crypto::fast_hash(token.as_bytes())` (`XXH3-64` from `crates/vox-crypto/src/facades.rs:25`).
    2. Increments `consecutive_same` if `hash == self.last_hash`; if `consecutive_same >= MAX_SINGLE_TOKEN_REP` (`32`), returns `Err(RepetitionLoopError::SingleTokenRun { count: 32 })`.
    3. Writes `self.ring[self.count & REPETITION_MODULO_MASK] = hash; self.count += 1;`.
    4. For each n-gram period `p` in `MIN_NGRAM_PERIOD..=MAX_NGRAM_PERIOD` (`3..=12`), if `self.count >= p * MIN_NGRAM_REPETITIONS`, checks whether the last `p * MIN_NGRAM_REPETITIONS` slots repeat the final `p` hashes identically. If so, returns `Err(RepetitionLoopError::NGramCycle { period: p, repetitions: MIN_NGRAM_REPETITIONS })`, aborting the stream immediately.

### 1.2 `ToolRefusalDetector` & ```` ```tool:<name> ```` Salvage (`crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/salvage_and_stream.rs`)
- **Source Inspiration:** `vibheksoni/pi-t3chat` `tools.ts:45-305` (`ToolRefusalDetector` & `ToolCallTranslator`).
- **Legacy Code Replaced:**
  - Delete stale `// ponytail: this ceiling (fallback-on-empty...)` block at `salvage_and_stream.rs:1-34`.
  - Update `looks_like_streamed_tool_call_envelope` (`salvage_and_stream.rs:219-248`) to also recognize ```` ```tool: ```` fenced blocks (`trimmed.starts_with("```tool:")`).
- **Refusal Detection & 1-Retry Recovery (`detect_false_tool_refusal`):**
  - When `available_tools_count > 0`, `assembled_tool_calls.is_empty()`, and `content.len() < 700`, inspects the first `350` chars against both regexes:
    - `REFUSAL_RE`: `(?i)\b(can't|cannot|can not|don't have|do not have|unable to|not able to|no access|without access|can't actually|cannot actually)\b`
    - `CAPABILITY_RE`: `(?i)\b(tool|terminal|shell|command|run|execute|file|filesystem|browse|browser|web search|search|calendar|schedule|environment)\b`
  - If both match and `!retried_refusal`, discards the refusal turn and automatically re-invokes once with a corrective system reminder listing the active tool names.

### 1.3 MCP Tool-Search Prompt-Cache Write Guard (`crates/vox-orchestrator-mcp/src/external_mcp.rs`)
- **Source Inspiration:** `t3code` `apps/server/src/orchestration-v2/Adapters/ClaudeAdapterV2.ts:66-105, 1295-1356` (`claudeToolSearchOverride`, commit `24c80f34`).
- **Mechanics:** When delegating to Claude Code or building MCP tool catalogs where `ENABLE_TOOL_SEARCH` is unset or `auto` / `auto:N` (`0 < N < 100`), automatically enforces `ENABLE_TOOL_SEARCH = "true"` (and activates `LazyMcpGroupRegistry` 3-tool mode when MCP tool count `> 12`), preventing the 10x prompt-cache write cost explosion caused by inlining full MCP schemas into 1M-context sessions.

---

## 2. Phase 219: Battery Unit Pre-Flight Gate, 3-Tier Cost Ledger & Context Sanitization (`T3-BATTERY-LEDGER-01..04`)

### 2.1 `BatteryAndCostLedger` (`crates/vox-orchestrator/src/budget/tracker.rs`)
- **Source Inspiration:** `guitaripod/omnichat` `src/lib/battery-pricing.ts` & `src/lib/usage-tracking.ts:130-198`, `TGlide/thom-chat` `/api/v1/generation`, and `vibheksoni/t3router` `poll_credit_delta`.
- **Legacy Code Replaced:** Bare `UsageTracker` (`crates/vox-orchestrator/src/budget/tracker.rs:1-27`) and `cost_usd: None` in `salvage_and_stream.rs:187`.
- **Implementation Design:**
  - **Unit Definition:** `1 Battery Unit (BU) = $0.001 USD` (`1_000` micro-USD). Local models (`ProviderType::Ollama` / `VoxLocal` / `Mens`) cost `0 BU` and bypass pre-flight balance checks (`is_zero_cost_local`).
  - **Pre-Flight Balance Check (`check_preflight_balance`):** Before dispatching any paid provider turn, estimates `estimated_bu` from prompt token count + expected completion tokens; if `remaining_balance_bu < estimated_bu`, returns `BudgetGateError::InsufficientBattery { required_bu, remaining_bu }` (HTTP `402` / RPC equivalent).
  - **4-Step Post-Stream Ledger Write (`record_turn_usage_and_reconcile`):**
    1. Records `TurnCostDeltaReceipt` (`EstimatedTokenCount`, `ProviderGenerationLookup`, or `SubscriptionCreditDelta`).
    2. Decrements `remaining_balance_bu` atomically.
    3. Appends a signed `BatteryTransaction` (`Purchase | Subscription | Bonus | Refund | Usage`, `amount_bu`, `balance_after_bu`, `model_id`).
    4. Updates `DailyUsageSummary` (`total_bu_used`, `total_messages`, `models_used` map) and calls `ModelRegistry::reconcile_turn_cost_delta` to update `observed_cost_per_1k_usd`.

### 2.2 Subsequent-Turn Context Tag Sanitization (`crates/vox-orchestrator-mcp/src/chat_tools/chat/context_sanitizer.rs`)
- **Source Inspiration:** `shaltielshmid/NotT3Chat` `backend/Program.cs:554-586, 629-670` (`CleanMessageFromSpecialEntities`) & `helioschat` `openai-compatible.ts:452-498`.
- **Mechanics:**
  - Parses all four reasoning tag pairs (`<think>...</think>`, `<thinking>...</thinking>`, `<reason>...</reason>`, `<reasoning>...</reasoning>`) plus `<WebSearch>...</WebSearch>` during streaming.
  - When assembling prior assistant turns for subsequent LLM calls, strips closed `<think|thinking|reason|reasoning>` blocks and replaces `<WebSearch>[\s\S]*?</WebSearch>` with `"[web_search was called, results removed to save context.]"`, while escaping prompt-injection tokens (`<|im_start|>`, `<|im_end|>`, `<tool_call>`) in raw user inputs.

### 2.3 External Multi-Benchmark Priors (`SnitchBench`, `SkateBench`, `SweBench`, `LiveBench`), `/1e6` Cost Evaluator & LiteLLM 24h Rate Cache (`crates/vox-orchestrator/src/models/bench_evidence.rs` & `discovery_pipeline.rs`)
- **Source Inspiration:** `vibheksoni/t3router` `src/t3/models.rs:1-421` (`scrape_benchmarks`, `eval_js_number`) & `t3code` `apps/server/src/usage/UsageService.ts` (`LiteLLM 24h rate cache`).
- **Legacy Code Replaced:** Single-index `intelligence_index` check in `crates/vox-orchestrator/src/models/discovery_pipeline.rs:91` and static pricing fallback in `crates/vox-orchestrator/src/models/cost_tier.rs`.
- **Mechanics:**
  1. **`ExternalBenchmarkPrior` (`crates/vox-orchestrator/src/models/bench_evidence.rs`):** Extends `ModelCapabilities` and `BenchEvidence` with normalized `0.0..=1.0` external benchmark scores:
     - `artificial_analysis_index: Option<f32>`
     - `swe_bench_verified: Option<f32>`
     - `live_bench_coding: Option<f32>`
     - `skate_bench_score: Option<f32>`
     - `snitch_bench_safety_penalty: Option<f32>`
  2. **Cold-Start Bayesian Prior Blending:** When a model has `< 5` local benchmark runs in `BenchEvidence`, `blended_quality()` seeds its Beta prior (`alpha_0, beta_0`) from the weighted mean of available external benchmarks (`0.40 * swe_bench + 0.30 * live_bench + 0.20 * artificial_analysis + 0.10 * skate_bench`), rather than flat `0.5`.
  3. **`eval_cost_expression` & 24h LiteLLM Rate Table Cache:** Supports `t3router`'s `/1e6` fractional cost expressions (`eval_js_number("3.0/1e6") -> 0.000003`) and caches upstream LiteLLM/OpenRouter per-token rate tables (`input_cost_per_token`, `output_cost_per_token`, `cache_read_input_token_cost`, `cache_creation_input_token_cost`) with a 24-hour TTL on disk so `TurnCostDeltaReceipt` always computes exact cache-read and cache-write USD costs even for newly discovered models.

### 2.4 Partial History Compaction (`up_to`) & Gemini `thoughtSignature` Preservation (`crates/vox-orchestrator/src/compaction.rs`)
- **Source Inspiration:** `t3code` `apps/server/src/orchestration-v2/CompactionService.ts` (`up_to`, `compact_message_index`) & `helioschat` `src/lib/server/ai/openai-compatible.ts:127-205` (`thoughtSignature` preservation).
- **Legacy Code Replaced:** Whole-buffer-only compaction in `crates/vox-orchestrator/src/compaction.rs:210-340`.
- **Mechanics:**
  - Adds `CompactionDirection { Full, UpToMessageId(String), FromMessageId(String) }` and persists `compact_message_index: Option<usize>` on the thread record so the UI renders an inline `"Context compacted up to here"` divider while keeping full raw messages inspectable in SQLite.
  - Preserves provider-specific `thought_signature: Option<String>` on tool-call parts across compaction and turn replay so Gemini 2.5/3 thinking models never reject multi-step tool-calling turns.

---

## 3. Phase 220: Multi-Model Consensus SSE Fan-Out & `ResponseQualityAnalyzer` (`T3-CONSENSUS-QA-01..04`)

### 3.1 `ConsensusFanOutEngine` (`crates/vox-orchestrator/src/models/consensus.rs`)
- **Source Inspiration:** `lulkebit/t3-cloneathon` `src/app/api/chat/consensus/route.ts` & `src/components/ConsensusMessage.tsx`.
- **Mechanics:**
  - Accepts `models: Vec<String>` (`2..=4` models) and dispatches them concurrently via `futures_util::future::join_all`, multiplexing all streams over a single channel emitting:
    - `ConsensusEvent::Start { models }`
    - `ConsensusEvent::Update { model_index, model, delta, is_streaming: true }`
    - `ConsensusEvent::Complete { model_index, model, response_time_ms, quality_metrics: ResponseQualityMetrics }`
    - `ConsensusEvent::Error { model_index, model, error }`
    - `ConsensusEvent::Final { responses: Vec<ConsensusResponse>, average_response_time_ms }`

### 3.2 Deterministic `ResponseQualityAnalyzer` (`crates/vox-orchestrator/src/models/quality_analyzer.rs`)
- **Source Inspiration:** `lulkebit/t3-cloneathon` `src/lib/response-quality-analyzer.ts`.
- **Formula (`0.0..=100.0`):**
  \[
  \text{quality\_score} = 0.30 \cdot \text{relevance} + 0.25 \cdot \text{clarity} + 0.20 \cdot \text{coherence} + 0.20 \cdot \text{completeness} + 0.05 \cdot \text{readability} - \text{penalties}
  \]
  - `readability`: Flesch Reading Ease (`206.835 - 1.015 * (words / sentences) - 84.6 * (syllables / words)` clamped to `0..=100`).
  - `coherence`: Density of 19 logical connectors (`however`, `therefore`, `furthermore`, `consequently`, `specifically`, etc.) + paragraph transition smoothness.
  - `clarity`: Average sentence length (`15..=25` words optimal) + structural markdown formatting (headers, lists, code blocks) minus lexical repetition penalty.
  - `relevance`: Non-stopword prompt keyword overlap ratio.
  - Bucketed into 5 tiers (`Excellent >= 85`, `Good >= 70`, `Fair >= 55`, `Poor >= 40`, `VeryPoor < 40`) for router telemetry and `/api/quality-analytics` reporting.

---

## 4. Verification Gates (`Phases 218–220`)
- `timeout 900s cargo test -p vox-llm-egress --lib repetition_guard::tests` (verifying 32-token single-token run halt and 3..12 n-gram period x5 cycle halt using `vox_crypto::fast_hash`).
- `timeout 900s cargo test -p vox-orchestrator-mcp --lib chat_tools::chat::agent_loop::salvage_and_stream::tests` (verifying `detect_false_tool_refusal` 1-retry recovery and ```` ```tool:<name> ```` salvage).
- `timeout 900s cargo test -p vox-orchestrator --lib budget::tracker::tests` (verifying Battery Unit `1 BU = $0.001` pre-flight `402` balance gate, `is_zero_cost_local` bypass, and 4-step ledger reconciliation).
- `timeout 900s cargo test -p vox-orchestrator --lib models::consensus::tests` (verifying parallel 2..4 model fan-out and Flesch-Kincaid `ResponseQualityAnalyzer`).
