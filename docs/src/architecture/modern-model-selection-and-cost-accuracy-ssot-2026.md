---
title: "Modern Model Selection, Pareto Routing, and Graduated Evidence SSOT"
description: "Comprehensive 2026 frontier model landscape, cost-accuracy pareto curves, token multiplication economics, and 4-tier graduated evidence grounding."
category: "Architecture SSOTs"
status: "current"
---

# Modern Model Selection, Pareto Routing, and Graduated Evidence SSOT (2026)

**Council-Ratified: 2026-09-15**  
**Rotation ID: `2026-Q3-modern-frontiers`**  
**Review Due: 2026-12-15**  
**SSOT Authority:** Referenced normatively by [`contracts/orchestration/model-pins.v1.yaml`](../../../contracts/orchestration/model-pins.v1.yaml), [`contracts/orchestration/model-routing.v1.yaml`](../../../contracts/orchestration/model-routing.v1.yaml), and [`contracts/orchestration/model-catalog.bootstrap.v1.json`](../../../contracts/orchestration/model-catalog.bootstrap.v1.json).

---

## 1. Executive Summary & Governance Authority

This specification defines the authoritative single source of truth (SSOT) for the Vox model selection and routing architecture, operational Pareto curves, reasoning token economics, multi-lane execution pipelines, and graduated evidence grounding.

As frontier AI ecosystems accelerated through 2026, raw model scale bifurcated into two distinct operational paradigms:
1. **Hybrid & Reasoning Engines** (`Claude 3.7 Sonnet`, `OpenAI o3-mini`, `DeepSeek-R1`): Models that dynamically generate internal reasoning or chain-of-thought (CoT) tokens to solve complex deduction, proof-carrying code generation, and multi-file debugging.
2. **High-Throughput / Large-Context Workhorses** (`Gemini 2.0 Flash`, `DeepSeek-V3`): Models offering millions of tokens of context or hundreds of tokens per second at sub-dollar per million pricing, ideal for real-time tool orchestration, wide-aperture document synthesis, and UI code generation.

Simultaneously, empirical analysis of autonomous research within `vox-search` revealed that historical evidence grounding mechanisms failed under production loads: rigid string matching rejected 35%–50% of genuine evidence snippets, while unwindowed bag-of-words heuristics allowed hallucinations and catastrophic negation-inversion polarity flips.

This SSOT records the ratified decisions of the Model Council rotation `2026-Q3-modern-frontiers`, establishing:
- The modern 2026 frontier roster and catalog representations.
- The multi-dimensional Pareto frontier and scoring formulas, accounting for the empirical $4\times$ reasoning token multiplier.
- The 4-tier graduated evidence grounding engine with bounded sliding-window overlap ($W = N + 4$) and negation parity protection ($N_{\text{snip}} \equiv N_{\text{win}} \pmod 2$).
- The multi-lane execution architecture spanning Sync, Background, and Plan execution lanes.

---

## 2. 2026 Frontier Model Landscape & Catalog

### 2.1 Operational Tiers

Vox categorizes models into four distinct operational tiers based on intelligence, latency, throughput, and hardware locality:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        ELITE / DEEP REASONING                          │
│  • Claude 3.7 Sonnet ($3.00 / $15.00) — Hybrid Thinking leader         │
│  • OpenAI o3-mini ($1.10 / $4.40) — Fast math, types, logic triage    │
│  • DeepSeek R1 ($0.55 / $2.19) — High-throughput open-weights CoT     │
│  • OpenAI o1 ($15.00 / $60.00) — Heavy multi-step deduction            │
├────────────────────────────────────────────────────────────────────────┤
│                     HIGH-SPEED / WORKHORSE CLOUD                       │
│  • Gemini 2.0 Flash ($0.10 / $0.40) — 1M context, 300+ tok/s, vision  │
│  • DeepSeek V3 ($0.14 / $0.28) — High-throughput codegen & lint fixes  │
│  • GPT-4o-mini ($0.15 / $0.60) — Fast structured parsing & triage      │
│  • Claude 3.5 Haiku ($0.80 / $4.00) — Low-latency agent interactions   │
├────────────────────────────────────────────────────────────────────────┤
│                          LOCAL MENS / OFFLINE                          │
│  • Qwen 3 8B ($0.00, <45ms) — Local claim extraction & formatting      │
│  • Qwen 3 32B ($0.00, 16-32GB VRAM) — High-quality offline codegen     │
└────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Frontier Model Specifications

#### `anthropic/claude-3-7-sonnet` (Elite)
- **Role:** Primary leader for architecture, code generation, refactoring, and security audits.
- **Capabilities:** Hybrid thinking with elastic chain-of-thought allocation; native tool calling; vision; structured JSON.
- **Economics:** \$3.00 / 1M input tokens, \$15.00 / 1M output tokens (blended \$0.009 / 1k tokens).
- **Context Window:** 200,000 tokens (`max_tokens: 200000`, `max_context: 200000`).
- **P50 Latency:** ~950ms.
- **Pinned Aliases:** `codegen`, `debugging`, `security`, `review`.

#### `openai/o3-mini` (Elite)
- **Role:** Primary reasoning, planning, and deductive logic engine.
- **Capabilities:** Deep step-by-step reasoning; competitive programming math; type checking and invariant proof verification; native tool calling.
- **Economics:** \$1.10 / 1M input tokens, \$4.40 / 1M output tokens (blended \$0.00275 / 1k tokens).
- **Context Window:** 200,000 tokens.
- **P50 Latency:** ~1,100ms.
- **Pinned Aliases:** `planning`.

#### `deepseek/deepseek-r1` (Elite)
- **Role:** High-throughput open-weights reasoning and verifiable deduction.
- **Capabilities:** Fully visible reasoning chain; mathematical proofs; code generation; `supports_reasoning: true`.
- **Economics:** \$0.55 / 1M input tokens, \$2.19 / 1M output tokens (blended \$0.00137 / 1k tokens).
- **Context Window:** 128,000 tokens.
- **P50 Latency:** ~2,200ms.
- **Pinned Aliases:** `logic`.

#### `openai/o1` (Elite)
- **Role:** Exhaustive multi-step architectural exploration and formal system design.
- **Capabilities:** Extended reasoning; complex algorithmic synthesis; native tool calling.
- **Economics:** \$15.00 / 1M input tokens, \$60.00 / 1M output tokens.
- **Context Window:** 200,000 tokens.
- **P50 Latency:** ~3,500ms.

#### `deepseek/deepseek-chat` (DeepSeek-V3, Pro)
- **Role:** Bulk code generation, multi-file lint sweeps, and high-throughput pull request review.
- **Capabilities:** Competitive coding benchmark performance at near-commodity pricing; native tools; structured JSON.
- **Economics:** \$0.14 / 1M input tokens, \$0.28 / 1M output tokens (blended \$0.00021 / 1k tokens).
- **Context Window:** 128,000 tokens.
- **P50 Latency:** ~650ms.

#### `google/gemini-2.0-flash` (Light)
- **Role:** Workhorse for deep research, web corpus ingestion, multimodal visual verification, and high-speed inter-agent communication.
- **Capabilities:** 1,048,576 token native context window; 300+ tok/s output generation; native tool calling; vision; audio; direct provider URL support with prefix sanitization.
- **Economics:** \$0.10 / 1M input tokens, \$0.40 / 1M output tokens (blended \$0.00025 / 1k tokens).
- **Context Window:** 1,048,576 tokens.
- **P50 Latency:** ~320ms.
- **Pinned Aliases:** `research`, `visus`.

#### `openai/gpt-4o-mini` (Light)
- **Role:** Fast structured extraction, JSON classification, and triage.
- **Capabilities:** High availability; vision; native tools; structured JSON.
- **Economics:** \$0.15 / 1M input tokens, \$0.60 / 1M output tokens (blended \$0.000375 / 1k tokens).
- **Context Window:** 128,000 tokens.
- **P50 Latency:** ~450ms.

#### `anthropic/claude-3-5-haiku` (Light)
- **Role:** Low-latency agent interactions, classifier passes, and prompt-cached classifications.
- **Economics:** \$0.80 / 1M input tokens, \$4.00 / 1M output tokens.
- **Context Window:** 200,000 tokens.
- **P50 Latency:** ~380ms.

#### Local MENS `qwen/qwen-3-8b` and `Qwen/Qwen3-32B-Instruct` (Local)
- **Role:** Completely offline, zero-network, zero-marginal-cost claim extraction, AST analysis, and local test repair.
- **Capabilities:** Local HuggingFace/Candle/Ollama deployment; native JSON output; sub-45ms latency for 8B model.
- **Economics:** \$0.00 input, \$0.00 output.
- **Context Window:** 128,000 tokens (`qwen-3-8b`), 32,000 tokens (`qwen3-coder:free`).
- **P50 Latency:** ~45ms (8B) to ~350ms (32B on Apple Silicon / CUDA).

---

### 2.3 Cost, Accuracy, and Latency Matrix

| Model ID | Provider Type | Tier | Input / 1M | Output / 1M | Blended / 1k | Max Context | P50 Latency | Primary Strengths |
|---|---|---|---:|---:|---:|---:|---:|---|
| `anthropic/claude-3-7-sonnet` | `open_router` | Elite | \$3.00 | \$15.00 | \$0.00900 | 200,000 | 950ms | `codegen`, `debugging`, `logic`, `planning`, `review`, `ui_codegen` |
| `openai/o3-mini` | `open_router` | Elite | \$1.10 | \$4.40 | \$0.00275 | 200,000 | 1,100ms | `logic`, `debugging`, `planning`, `codegen` |
| `deepseek/deepseek-r1` | `open_router` | Elite | \$0.55 | \$2.19 | \$0.00137 | 128,000 | 2,200ms | `logic`, `codegen`, `debugging` |
| `openai/o1` | `open_router` | Elite | \$15.00 | \$60.00 | \$0.03750 | 200,000 | 3,500ms | `logic`, `planning` |
| `deepseek/deepseek-chat` | `open_router` | Pro | \$0.14 | \$0.28 | \$0.00021 | 128,000 | 650ms | `codegen`, `review`, `generalist` |
| `google/gemini-2.0-flash` | `open_router` | Light | \$0.10 | \$0.40 | \$0.00025 | 1,048,576 | 320ms | `research`, `inter_agent`, `long_context`, `vision`, `codegen`, `visus` |
| `openai/gpt-4o-mini` | `open_router` | Light | \$0.15 | \$0.60 | \$0.000375 | 128,000 | 450ms | `parsing`, `inter_agent`, `generalist` |
| `anthropic/claude-3-5-haiku` | `open_router` | Light | \$0.80 | \$4.00 | \$0.00240 | 200,000 | 380ms | `inter_agent`, `parsing` |
| `qwen/qwen-3-8b` | `local` | Local | \$0.00 | \$0.00 | \$0.00000 | 128,000 | 45ms | `codegen`, `parsing`, `logic` |

---

## 3. Pareto Frontier & Routing Mechanics

### 3.1 Multi-Objective Optimization

The Vox autonomic model router selects models by projecting available candidates onto an empirical 3-dimensional Pareto surface:
- **Quality Score ($Q \in [0.0, 1.0]$):** Measured benchmark performance across code synthesis, type consistency, and Socrates factuality calibration.
- **Latency ($L$):** Normalized round-trip completion time, reflecting p50 latency in milliseconds.
- **Cost ($C$):** Blended cost per 1,000 tokens under expected task token ratios.

A model $M_1$ dominates $M_2$ if and only if:
$$\forall x \in \{Q, -L, -C\}, \quad x(M_1) \ge x(M_2) \quad \land \quad \exists x \in \{Q, -L, -C\}, \quad x(M_1) > x(M_2)$$

The Pareto frontier comprises the non-dominated subset of models across these dimensions.

### 3.2 Composite Scoring Equation

When selecting models without an explicit hard pin, `auto_score_model` evaluates eligible candidates according to task-specific weights and caller intent:

$$\text{Score}(M, T) = w_q \cdot Q(M, T) + w_l \cdot \left(1 - \frac{L(M)}{L_{\max}}\right) + w_c \cdot \left(1 - \frac{C(M)}{C_{\max}}\right) + \Delta_{\text{affinity}}(M, T)$$

Where:
- $w_q, w_l, w_c$ represent normalized axis weights derived from `SelectionIntent.axes` (`(cost, responsiveness, intelligence)`).
- $Q(M, T)$ incorporates the base benchmark score, historical success rate from `vox-db` (`model_scoreboard`), and task-specific strength modifiers (e.g. `StrengthTag::CodeGen`, `StrengthTag::Vision`).
- $\Delta_{\text{affinity}}$ rewards provider locality (e.g. local MENS models receive an affinity bonus for low-latency claim extraction) or verified prompt cache hit capability.

#### Elimination of the "Ghost Scorer"
Prior implementations of `ModelRegistry::best_for_internal` suffered from a defect termed the *Ghost Scorer*: when multiple candidate models had identical tier levels, the engine discarded the multi-axis heuristic score and sorted candidates strictly by lowest raw price. This led to cheap, low-capability models displacing superior candidates during complex tasks. The updated engine ensures candidate scoring strictly evaluates `auto_score_model` across intelligence, latency, and cost before tie-breaking.

### 3.3 Reasoning Token Multiplication Economics

A critical insight of the 2026 model landscape is that **reasoning models cannot be budgeted using traditional input/output ratios**.

Models such as `OpenAI o3-mini`, `DeepSeek-R1`, and `Claude 3.7 Sonnet` generate internal chain-of-thought tokens that are billed at output token prices, even if hidden or collapsed in user-facing viewports.

#### The $4\times$ Reasoning Token Multiplier
Empirical traces across formal verification, algorithmic deduction, and rustc borrow-checker repair demonstrate that reasoning models emit an average of **4 output tokens for every 1 visible token produced**:

$$T_{\text{billed\_out}} \approx M_{\text{reasoning}} \cdot T_{\text{visible\_out}}, \quad \text{where } M_{\text{reasoning}} \approx 4.0$$

#### Budget Estimation Formula
To prevent autonomic agent loops from encountering unexpected context truncation or budget-cap exhaustion, the budget estimator applies the reasoning multiplier:

$$\text{Cost}_{\text{task}} = \left(T_{\text{in}} \cdot P_{\text{in}}\right) + \left(T_{\text{visible\_out}} \cdot M_{\text{reasoning}} \cdot P_{\text{out}}\right)$$

Where:
- $M_{\text{reasoning}} = 4.0$ if `capabilities.supports_reasoning == true` and task category is in `[logic, planning, debugging, codegen]`.
- $M_{\text{reasoning}} = 1.0$ for non-reasoning models or purely descriptive/formatting tasks.

#### Economy Cost Ceiling
The economy cost ceiling in `model-routing.v1.yaml` was historically set to \$0.10 / 1k tokens (\$100 / MTok), which rendered the economy filter inert since every frontier model qualified. The ceiling has been recalibrated to:
```yaml
economy_cost_ceiling_usd_per_1k: 0.0015
```
At \$0.0015 / 1k tokens (\$1.50 / MTok), only Light and Pro tier models (`Gemini 2.0 Flash`, `DeepSeek-V3`, `GPT-4o-mini`, `DeepSeek-R1`, and local MENS) qualify for economy routing, guaranteeing real cost reduction when economy mode is requested.

---

## 4. Graduated Evidence Grounding

### 4.1 The Grounding Crisis in Deep Research

In autonomous research pipelines (`vox-search`, `mens_research_subagent.rs`, SCIENTIA), LLM extractors distill unstructured web and repository data into structured factual triplets:
$$\langle \text{subject}, \, \text{predicate}, \, \text{object} \rangle \quad \text{with an associated } \text{evidence\_snippet}$$

These triplets form the bedrock of the Vox epistemic knowledge graph in `vox-db`. If false claims enter the database, downstream agents synthesize erroneous conclusions.

Historically, verification relied on two polar extremes, both of which failed in production:
1. **Brittle Verbatim Substring Matching (`source.contains(snippet)`):**
   Produced a **35% to 50% false negative rate**. Minor typographic differences caused valid evidence to be discarded:
   - Unicode punctuation drift (em-dash `—` vs en-dash `–` vs hyphen `-`).
   - Quotation marks (curly `“...”` vs straight `\"...\"`).
   - Whitespace formatting (newlines, tabs, multiple spaces).
   - Coreference resolution: the source text says *"SQLite added JSONB in version 3.45. It provides 3x faster reads."*, while the extractor produces the clearer snippet *"SQLite JSONB provides 3x faster reads."*
2. **Naive Bag-of-Words / Global Jaccard Overlap:**
   Produced catastrophic **false positives**. If an extractor hallucinated a claim combining keywords scattered across pages 1, 5, and 12 of a document, global set intersection scored high overlap despite the claim having zero factual grounding in any single sentence.
3. **Inversion & Negation Blindness:**
   Bag-of-words heuristics cannot distinguish between:
   - Source: *"The microbenchmark did not reduce latency across threads."*
   - Snippet: *"The microbenchmark did reduce latency across threads."*
   Naive token overlap scores 88%+, falsely validating an inverted assertion.

### 4.2 The 4-Tier Graduated Grounding Engine

Vox replaces naive matching with a robust 4-tier graduated grounding engine implemented in `crates/vox-search/src/mens_research_subagent.rs`:

```
┌─────────────────────────────────────────────────────────────┐
│                       EVIDENCE SNIPPET                      │
└──────────────────────────────┬──────────────────────────────┘
                               │
            ┌──────────────────▼──────────────────┐
            │  Tier 1: Verbatim Exact Substring   │ ──(Found)──► VerbatimExact (1.0)
            └──────────────────┬──────────────────┘
                               │ (Not Found)
            ┌──────────────────▼──────────────────┐
            │  Tier 2: Normalized Exact Substring │ ──(Found)──► NormalizedSpan (1.0)
            │  (Unicode dashes, quotes, spaces)   │
            └──────────────────┬──────────────────┘
                               │ (Not Found)
            ┌──────────────────▼──────────────────┐
            │  Tier 3: Bounded Sliding Window     │
            │  Window Size: W = N + 4 tokens      │
            └──────────────────┬──────────────────┘
                               │
            ┌──────────────────▼──────────────────┐
            │  Tier 4: Negation Parity Invariant  │
            │  N_snip ≡ N_win (mod 2)             │
            └──────────────────┬──────────────────┘
                               │
            ┌──────────────────▼──────────────────┐
            │      Overlap Ratio ≥ 0.80 ?         │
            └──────────┬──────────────────┬───────┘
                       │ Yes              │ No
                       ▼                  ▼
               NormalizedSpan(ratio)    REJECT
```

#### Tier 1: `GroundingQuality::VerbatimExact`
Direct, case-insensitive substring search in the raw source text. If found, the snippet is grounded with absolute fidelity.

#### Tier 2: `NormalizedExact`
Both source text and evidence snippet are processed via `normalize_for_matching`:
- Em-dashes (`—`) and en-dashes (`–`) normalize to `-`.
- Curved quotes (`“`, `”`, `‘`, `’`) normalize to straight quotes (`"`, `'`).
- All ASCII and unicode whitespace sequences collapse to single spaces.
- Text is converted to lowercase.
If the normalized snippet is a substring of the normalized source, it is accepted as `GroundingQuality::NormalizedSpan { overlap_ratio: 1.0 }`.

#### Tier 3: Bounded Sliding-Window Overlap ($W = N + 4$)
To prevent scattered-word hallucinations while tolerating pronoun expansion and minor elisions:
1. Let the snippet consist of $N$ tokens.
2. The source document is segmented into overlapping sliding windows of bounded size $W = N + 4$ tokens.
3. Overlap ratio is computed as:
   $$\text{Ratio} = \frac{|\text{Tokens}_{\text{snip}} \cap \text{Tokens}_{\text{win}}|}{|\text{Tokens}_{\text{snip}}|}$$
4. The candidate window must satisfy $\text{Ratio} \ge 0.80$.

#### Tier 4: The Negation Parity Invariant Gate
To eliminate polarity inversions, any candidate window that satisfies the 0.80 overlap threshold must pass the **Negation Parity Invariant**:

$$N_{\text{snip}} \equiv N_{\text{win}} \pmod 2$$

Where $N_{\text{text}}$ counts tokens in the recognized negation vocabulary:
`{"not", "no", "never", "none", "neither", "nor", "cannot", "can't", "won't", "didn't", "doesn't", "isn't", "aren't", "without"}`.

- If a source sentence contains `"not"` but the snippet omits it, $N_{\text{snip}} = 0$ and $N_{\text{win}} = 1$. The parity mismatch ($0 \not\equiv 1 \pmod 2$) triggers an immediate hard rejection.
- **Parity-Aware Window Selection:** The engine filters candidate windows through the negation parity check *before* scoring overlap, ensuring that an inverted candidate window cannot greedily shadow another valid window in the document that meets both parity and overlap criteria.

### 4.3 Provenance Propagation & Graph Corroboration in `vox-db`

Grounded triplets preserve their origin URL and grounding quality:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GroundingQuality {
    VerbatimExact,
    NormalizedSpan { overlap_ratio: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimTriplet {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f64,
    pub evidence_snippet: String,
    pub source_url: Option<String>,
    pub grounding: GroundingQuality,
}
```

When triplets enter `vox-db`:
1. Triplets with `source_url: Some(...)` are indexed by normalized claim hash.
2. If the same $\langle \text{subject}, \, \text{predicate}, \, \text{object} \rangle$ is corroborated across multiple independent domain origins (e.g. `docs.rs` and `github.com/rust-lang`), the claim's epistemic confidence escalates to *Corroborated Fact*.
3. Single-source triplets remain marked as *Provisional Evidence*, preventing single-origin misinformation from dominating agent decisions.

---

## 5. Multi-Lane Execution Architecture

Vox operates across three execution lanes, each tailored to distinct latency, reliability, and security profiles:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        VOX MULTI-LANE ROUTING                          │
├────────────────────────────────────────────────────────────────────────┤
│  1. SYNC LANE (Agent Loop & Interactive Tools)                         │
│     • Preserves native tool-calling agent loop                         │
│     • Direct Google/Anthropic specs map to LlmConfig                   │
├────────────────────────────────────────────────────────────────────────┤
│  2. BACKGROUND LANE (Autonomous Agents & Gamify)                       │
│     • Client-side safety guards (is_claude_model)                      │
│     • URL sanitization (strips "google/" from API endpoints)           │
├────────────────────────────────────────────────────────────────────────┤
│  3. PLAN LANE (/plan Command & Execution)                              │
│     • Preserves model_override from ChatTurnInput to PlanParams        │
│     • Decoupled GUI transport credential checks                        │
└────────────────────────────────────────────────────────────────────────┘
```

### 5.1 Sync Lane: Interactive Chat & Native Tool Preservation

The Sync Lane powers interactive conversations in `vox-orchestrator-mcp` (`agent_loop.rs`):
- Executes the multi-turn agent loop with tool dispatching (`mcp_run_turn`).
- **Tool Preservation Invariant:** Historical code returned `None` from `model_spec_to_llm_config` for models whose `provider_type` was `ProviderType::GoogleDirect` or `ProviderType::Anthropic`. This caused the orchestrator to fall back to toolless single-shot completion. The updated engine constructs `LlmConfig::openrouter(model)` when appropriate, preserving full native tool-calling capabilities across all providers.

### 5.2 Background Lane: Autonomous AI & Client Guardrails

The Background Lane powers background tasks, gamification NPCs, and autonomous maintenance loops (`vox-gamify`):
- **Model Override Safety Guard:** `UserModelOverride` in `crates/vox-gamify/src/ai/client/ctor.rs` incorporates `is_claude_model(name)` guards to prevent Claude models from being accidentally forwarded to Google Gemini direct streaming endpoints.
- **Provider URL Sanitization:** Upstream Google Gemini endpoints at `generativelanguage.googleapis.com` reject requests containing provider prefixes (`google/gemini-2.0-flash`). Both `vox-orchestrator-mcp` and `vox-gamify` apply `sanitize_google_model_id` / `sanitize_gemini_model_id` to strip leading `"google/"` namespaces prior to constructing HTTP request envelopes.

### 5.3 Plan Lane: `/plan` Model Threading & GUI Key Decoupling

The Plan Lane governs high-level planning sessions triggered via the `/plan` command or GUI plan modal:
- **`model_override` Threading:** When a user selects a specific model in the chat interface or CLI, the override must persist through the entire planning workflow:
  1. Captured in `ChatTurnInput.model_override`.
  2. Passed via `plan_tool_args` into the `vox_plan` JSON tool invocation.
  3. Declared in `vox_plan` input schema (`"model_override": {"type": "string", "maxLength": 256}`).
  4. Deserialized into `PlanParams.model_override`.
  5. Resolved in `plan.rs` and `plan_loop.rs` using `effective_model_pref`.
- **GUI Transport Credential Decoupling:** In `ChatModelPicker.tsx`, models were previously marked unavailable if the user lacked an API key matching the model's vendor name (e.g. demanding an `ANTHROPIC_API_KEY` for `anthropic/claude-3-7-sonnet`). The picker now checks `model.provider_type ?? model.provider`, validating transport credentials (`OPENROUTER_API_KEY`) rather than the upstream model vendor.
- **CLI Local Routing Guard:** In `vox chat`, user input like `qwen3:8b` is routed to local Ollama, while fully-qualified catalog slugs like `qwen/qwen-3-32b-instruct` cleanly route to OpenRouter cloud endpoints without collision.

---

## 6. Contract Synchronization & CI Invariants

The model routing subsystem enforces strict contract integrity across configuration, build scripts, and CI:

### 6.1 Pinned Aliases Symmetry
The `premium_alias` mappings in `contracts/orchestration/model-pins.v1.yaml` and `contracts/orchestration/model-routing.v1.yaml` must remain 100% identical:

```yaml
premium_alias:
  codegen: "anthropic/claude-3-7-sonnet"
  debugging: "anthropic/claude-3-7-sonnet"
  security: "anthropic/claude-3-7-sonnet"
  research: "google/gemini-2.0-flash"
  planning: "openai/o3-mini"
  review: "anthropic/claude-3-7-sonnet"
  logic: "deepseek/deepseek-r1"
  visus: "google/gemini-2.0-flash"
```

### 6.2 Bootstrap Catalog Validation
Every entry in `contracts/orchestration/model-catalog.bootstrap.v1.json` must satisfy:
1. `capabilities.max_context > 0` (zero-context entries are hard errors).
2. `cost_per_1k >= 0.0`.
3. `strengths` must contain only valid `StrengthTag` identifiers (e.g. `vision`, `long_context`, `ui_codegen`).
4. All `premium_alias` targets must exist in the catalog.

### 6.3 Build & CI Gates
- **`crates/vox-config/build.rs`:** Registers `cargo:rerun-if-changed` for both YAML contracts, preventing stale in-memory contract definitions across workspace compilations.
- **Routing Gate:** `cargo run -p vox-cli -- ci model-routing-check` runs in CI and pre-push hooks to verify alias symmetry, catalog validity, and council signoff currency.

---

## 7. Council Signoff History

| Rotation ID | Approved Date | Review Due | Approved By | Rationale & Key Decisions |
|---|---|---|---|---|
| `2026-Q2-rotation-2` | 2026-05-15 | 2026-08-15 | Council | Initial split of `model-pins.v1.yaml` from bootstrap catalog. Pinned Claude Mythos preview and Gemini 2.5 Pro preview. |
| `2026-Q3-modern-frontiers` | 2026-09-15 | 2026-12-15 | Council | Ratified Claude 3.7 Sonnet as codegen/debugging leader, o3-mini for planning, Gemini 2.0 Flash for research/visus, DeepSeek-R1 for logic. Retired Mythos and Gemini 2.5 previews. Adopted 4-tier graduated evidence grounding ($W=N+4$, negation parity). |

---

*Document authoritative as of 2026-09-15. Review due: 2026-12-15.*
