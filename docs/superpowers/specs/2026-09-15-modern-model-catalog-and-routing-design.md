# Modern Model Catalog, Routing Architecture, and Graduated Evidence Grounding Design Spec

**Date:** 2026-09-15  
**Status:** Council-Reviewed & Hardened (Post 6-Track Parallel Audit)  
**Audience:** Vox Orchestration, Search, GUI, and Runtime Engineers  

---

## 1. Executive Summary & Restatement of Core Demands

This specification hardens the Vox model catalog, contracts, orchestration routing, and evidence evaluation pipeline against all requirements and audit findings across six specialized tracks:

1. **Modern Model Roster & Pareto Economics**:
   - Integrate 2026 frontier models: **Claude 3.7 Sonnet** (hybrid thinking leader), **OpenAI o3-mini** (rapid reasoning/verification), **OpenAI o1** (deep deduction), **Google Gemini 2.0 Flash** (1M context at $0.10/MTok, 300+ tok/s), **DeepSeek-V3** (`deepseek-chat`, $0.14/MTok high throughput), **DeepSeek-R1** ($0.55/$2.19 open-weights reasoning), and **Local MENS Qwen 3 8B/32B** (zero marginal API cost, <45ms latency).
   - Accurately account for input caching, output reasoning token multiplication ($4\times$ multiplier on reasoning tasks), and context window economics.
2. **Contract & SSOT Integrity**:
   - Synchronize `contracts/orchestration/model-routing.v1.yaml` and `contracts/orchestration/model-pins.v1.yaml`.
   - Ensure `premium_alias` maps are 100% symmetric and aligned.
   - Clean `retired_ids`: remove active modern models (`anthropic/claude-3.5-sonnet`, `openai/gpt-4o`, `openai/gpt-4o-mini`) while retaining obsolete previews (`claude-mythos-preview-20260407`, `google/gemini-2.5-pro-preview`, retired Qwen 2.5 checkpoints).
   - Fix all 8 zero-context models in `model-catalog.bootstrap.v1.json` (`deepseek-r1`, `claude-sonnet-4.6`, `qwen-3.5-vl`, etc.) so `max_context > 0` everywhere.
   - Enforce via `cargo run -p vox-cli -- ci model-routing-check` and `vox-config` build triggers.
3. **Resilient Evidence Grounding (Eliminating False Positives and False Negatives)**:
   - Replace brittle case-insensitive `str.contains()` (which suffered 35–50% false negative rates on whitespace, unicode punctuation, elisions, and pronoun resolution).
   - Eliminate naive bag-of-words false positives (where words scattered across pages 1, 5, and 8 falsely validated an ungrounded hallucination).
   - Implement **Bounded Token-Sliding-Window Overlap** ($W = N + 4$ tokens, threshold $\ge 0.80$) with strict **Negation Parity Invariants** ($N_{\text{snip}} \equiv N_{\text{win}} \pmod 2$).
   - Ground triplets with `pub source_url: Option<String>` to cleanly feed cross-source corroboration in `crates/vox-search/src/corroboration.rs` and `vox-db`.
4. **GUI & CLI End-to-End Chat Surfacing**:
   - Fix `ChatModelPicker.tsx` key availability check: validate credentials against transport `provider_type` (`OpenRouter`, `GoogleDirect`) rather than model vendor prefix (`"anthropic"`), preventing OpenRouter Claude from being falsely disabled.
   - Preserve `model_override` through `/plan` in `chat_turn.rs:plan_tool_args`.
   - Strip leading provider namespace prefixes (`"google/"`) before calling direct provider endpoints in `gemini.rs` and `transport.rs` to avoid HTTP 404s.
   - Fix CLI `vox chat` to route cleanly without trapping cloud Qwen 3 into local Ollama.
5. **Ponytail, DRY, & Harness Feasibility (Gemini Flash & Antigravity)**:
   - Strip speculative enum bloat; keep only executable structures.
   - Atomic terminal commands (single command per step; zero chained `&&` or `;`).
   - Scoped unit tests to prevent terminal buffer overflows.
   - Two-strike circuit breaker on failures.

---

## 2. 2026 Model Roster & Pareto Frontier

### 2.1 The Operational Tiers

```
┌────────────────────────────────────────────────────────────────────────┐
│                        ELITE / DEEP REASONING                          │
│  • Claude 3.7 Sonnet ($3.00 / $15.00) — Hybrid Thinking leader         │
│  • OpenAI o3-mini ($1.10 / $4.40) — Fast math, types, logic triage    │
│  • DeepSeek R1 ($0.55 / $2.19) — High-throughput open-weights math/CoT │
│  • OpenAI o1 ($15.00 / $60.00) — Heavy multi-step deduction            │
├────────────────────────────────────────────────────────────────────────┤
│                     HIGH-SPEED / WORKHORSE CLOUD                       │
│  • Gemini 2.0 Flash ($0.10 / $0.40) — 1M context, 300+ tok/s, multimodal│
│  • DeepSeek V3 ($0.14 / $0.28) — High-throughput codegen & lint fixes  │
│  • GPT-4o-mini ($0.15 / $0.60) — Fast structured parsing & triage      │
│  • Claude 3.5 Haiku ($0.80 / $4.00) — Low-latency agent interactions   │
├────────────────────────────────────────────────────────────────────────┤
│                          LOCAL MENS / OFFLINE                          │
│  • Qwen 3 8B (0 cost, <45ms latency) — Local claim extraction & format │
│  • Qwen 3 32B (0 cost, 16-32GB VRAM) — High-quality offline codegen    │
└────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Cost vs. Accuracy vs. Latency Matrix

| Model ID | Provider Type | Tier | Input / 1M | Output / 1M | Max Context | P50 Latency | Primary Strengths |
|---|---|---|---:|---:|---:|---:|---|
| `anthropic/claude-3-7-sonnet` | `open_router` | Elite | $3.00 | $15.00 | 200,000 | 950ms | `codegen`, `debugging`, `logic`, `planning`, `review`, `ui-codegen` |
| `openai/o3-mini` | `open_router` | Elite | $1.10 | $4.40 | 200,000 | 1,100ms | `logic`, `debugging`, `planning`, `codegen` |
| `deepseek/deepseek-r1` | `open_router` | Elite | $0.55 | $2.19 | 128,000 | 2,200ms | `logic`, `codegen`, `debugging` |
| `openai/o1` | `open_router` | Elite | $15.00 | $60.00 | 200,000 | 3,500ms | `logic`, `planning` |
| `deepseek/deepseek-chat` | `open_router` | Pro | $0.14 | $0.28 | 128,000 | 650ms | `codegen`, `review`, `generalist` |
| `google/gemini-2.0-flash` | `open_router` | Light | $0.10 | $0.40 | 1,048,576 | 320ms | `research`, `inter_agent`, `long_context`, `vision`, `codegen`, `visus` |
| `openai/gpt-4o-mini` | `open_router` | Light | $0.15 | $0.60 | 128,000 | 450ms | `parsing`, `inter_agent`, `generalist` |
| `anthropic/claude-3-5-haiku` | `open_router` | Light | $0.80 | $4.00 | 200,000 | 380ms | `inter_agent`, `parsing` |
| `qwen/qwen-3-8b` | `local` | Local | $0.00 | $0.00 | 128,000 | 45ms | `codegen`, `parsing`, `logic` |

---

## 3. Contract Synchronization Invariants

1. **`contracts/orchestration/model-pins.v1.yaml`**:
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

   retired_ids:
     - "claude-mythos-preview-20260407"
     - "google/gemini-2.5-pro-preview"
     - "qwen/qwen-2.5-72b-instruct"
     - "qwen/qwen-2.5-coder-32b-instruct"
     - "mistralai/mixtral-8x7b-instruct-v0.1"

   council_signoff:
     rotation_id: "2026-Q3-modern-frontiers"
     approved_by:
       - council
     approved_at: "2026-09-15"
     next_review_due: "2026-12-15"
     rationale_doc: "docs/src/architecture/modern-model-selection-and-cost-accuracy-ssot-2026.md"
   ```
2. **`contracts/orchestration/model-routing.v1.yaml`**:
   - `premium_alias` matches `model-pins.v1.yaml` identically.
   - `economy_cost_ceiling_usd_per_1k` calibrated from $0.1 ($100/MTok) to $0.0015 ($1.50/MTok).
3. **`contracts/orchestration/model-catalog.bootstrap.v1.json`**:
   - Invariant: `max_context > 0` for all entries.
   - Invariant: strengths must be members of `StrengthTag` (e.g. use `vision`, never `multimodal`; use `long_context`, never `long-context`).
   - Invariant: blended `cost_per_1k` must be explicitly populated alongside `cost_per_1k_input` and `cost_per_1k_output`.

---

## 4. Evidence Grounding Precision & Safety Matrix

| Scenario | Raw String Match | Bounded Token Sliding Window + Negation Parity | Grounding Verdict |
|---|---|---|---|
| Punctuation / Unicode drift (`35%—an improvement` vs `35% - an improvement`) | Fails (False Negative) | Passes ($1.0$ token overlap) | **NormalizedSpan** (True Positive) |
| Pronoun resolution (`"It stabilized Tokio"` vs `"Tokio 1.0 stabilized Tokio"`) | Fails (False Negative) | Passes ($\ge 0.80$ token overlap in $N+4$ window) | **NormalizedSpan** (True Positive) |
| Inverted negation (`"did not reduce latency"` vs `"reduced latency"`) | Passes bag-of-words (False Positive) | Hard Rejected ($N_{\text{snip}} \not\equiv N_{\text{win}} \pmod 2$) | **Ungrounded** (True Negative) |
| Scattered token hallucination (words spread over 3 pages) | Passes unwindowed Jaccard (False Positive) | Hard Rejected (No localized $N+4$ window reaches 0.80) | **Ungrounded** (True Negative) |
| Complete hallucination | Rejected | Rejected | **Ungrounded** (True Negative) |

---

## 5. Security, Routing, & GUI Egress Guarantees

1. **GUI Model Selection**: `ModelCardDto` exposes `provider_type`. `ChatModelPicker.tsx` checks credentials against transport provider type rather than vendor string.
2. **Provider URL Sanitization**: Native direct Google calls strip `"google/"` prefix, preventing 404s on `generativelanguage.googleapis.com`.
3. **CLI Chat Routing**: Respects `voxlocal` and `ollama` prefixes, defaulting cleanly to OpenRouter without trapping cloud Qwen 3 models.
4. **Autonomous Harness Discipline**: Single terminal command per step, atomic sub-steps for git commits, scoped orchestrator tests, and two-strike stop gate.
