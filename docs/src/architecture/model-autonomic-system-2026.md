---
title: "Vox Model Autonomic System — L1/L2/L3 Design (2026-Q2)"
description: "Continuous-discovery + auto-classification + selection-SSOT architecture replacing hand-curated model bootstrap."
category: "Architecture SSOTs"
status: "current"
---

# Vox Model Autonomic System

**Council-ratified 2026-05-15.** Companion to [`model-selection-2026-q2.md`](./model-selection-2026-q2.md) §8.

## 0. Problem

The current model pipeline mixes three concerns into one hand-edited file:
the bootstrap catalog is simultaneously (a) the cold-start fallback, (b) the
SSOT humans edit when a new model ships, and (c) the pin list for
reproducibility. Result: every new frontier model release blocks on a manual
PR to `model-catalog.bootstrap.v1.json`. Production selection runs through
`models::select::decide` and registry routing; provider-default constants remain
as fallbacks. `resolve_model_with_registry_fallbacks` has no production callers
and is not a maintained routing surface, as verified by
[ADR-046](../adr/046-pareto-frontier-reporting.md).

## 1. The three loops

```
┌───────────────────────────────────────────────────────────────┐
│  L3  Council-review (quarterly + on alert)                     │
│      • approves Provisional → Confirmed tier promotions        │
│      • approves premium_alias rotations                        │
│      • reads council-report.md (auto-generated)                │
└───────────────────────────────────────────────────────────────┘
                  ▲ council-report.md
┌───────────────────────────────────────────────────────────────┐
│  L2  Continuous classification (NEW)                           │
│      • classifier LLM (Haiku/Flash-tier) consumes              │
│        (id, description, param_graph, sample_pricing)          │
│        → emits {tier, strengths[], confidence}                 │
│      • shadow-runs new model on eval panel for N samples       │
│      • DB scoreboard converges on success/cost/latency         │
│      • confidence ≥ threshold → Provisional → Confirmed        │
└───────────────────────────────────────────────────────────────┘
                  ▲ provisional ModelSpec + DB rows
┌───────────────────────────────────────────────────────────────┐
│  L1  Continuous discovery (extension of existing)              │
│      OpenRouter /models • LiteLLM pricing • Anthropic /models  │
│      • runs on a schedule (nightly cron), not just at startup  │
│      • diffs against registry → emits DiscoveryEvent           │
└───────────────────────────────────────────────────────────────┘
```

## 2. Existing pieces we reuse

| Surface | What it does today | How L1/L2/L3 uses it |
|---|---|---|
| `OpenRouterCatalog::refresh()` | One-shot `/models` fetch | L1: scheduled refresh |
| `LiteLLMCatalog::fetch()` | Pricing enrichment | L1: scheduled refresh |
| `AnthropicDirectCatalog::refresh()` | Key-gated Anthropic catalog | L1: scheduled refresh |
| `infer_strengths()` (catalog.rs) | Parameter-graph → strengths | L2: prior for classifier |
| `ModelRegistry::premium_alias_for()` | Task→pin lookup | L3: read from pins.yaml |
| `select(intent, registry)` | Multi-axis SSOT picker | runtime: unchanged |
| `vox-db model_scoreboard` | Per-model success/cost/latency | L2: convergence signal |
| `vox-db model_pricing_catalog` | Telemetry-confirmed pricing | L2: confidence promotion |

## 3. New contracts

### 3.1 `contracts/orchestration/model-pins.v1.yaml`

Council-reviewed, infrequently changed. Pin list separate from catalog:

```yaml
schema: vox.orchestration.pins/v1
premium_alias:
  codegen: anthropic/claude-opus-4.7
  research: google/gemini-3.1-pro
  review: anthropic/claude-sonnet-4.6
  planning: openai/gpt-5.5-pro
  # …
version_pins:
  # When CR-L0 eval-panel reproducibility matters, lock these.
  llm-panel.claude-sonnet: claude-sonnet-4-6
  llm-panel.gpt-frontier: gpt-5.4
council_signoff:
  rotation_id: 2026-Q2-rotation-2
  approved_by: [council]
  approved_at: 2026-05-15
```

### 3.2 `contracts/orchestration/catalog-fallback.v1.json`

What `model-catalog.bootstrap.v1.json` becomes after the rename: a minimal
**emergency-offline** subset (~5–10 models) covering each tier, used only
when L1 discovery has never succeeded. Not the SSOT.

### 3.3 Confidence states

Each `ModelSpec` carries `confidence: Confidence`:

```
Provisional  // discovered, classifier-tagged, no scoreboard data yet
Shadowed     // running on eval panel; not eligible for production routing
Confirmed    // scoreboard data passes thresholds; eligible everywhere
Deprecated   // failing thresholds OR retired by council
```

## 4. New telemetry events

```rust
// fired by select() on every selection
SelectionDecisionEvent {
    intent_caller: Option<&'static str>,   // "repair-loop", "research", …
    task: TaskCategory,
    axes: (u8, u8, u8),                    // (cost, responsiveness, intelligence)
    chosen_model: String,
    reason: SelectionReason,
    timestamp_ms: u64,
}

// fired by L1 when a model id appears that wasn't in the prior catalog
DiscoveryEvent {
    source: DiscoverySource,               // OpenRouter | LiteLLM | Anthropic | Mesh
    model_id: String,
    seen_at_ms: u64,
}

// fired by L2 when classifier completes
ClassificationEvent {
    model_id: String,
    classifier_model: String,              // which LLM classified it
    tier: ModelTier,
    strengths: Vec<StrengthTag>,
    confidence: f32,                       // 0.0–1.0
    timestamp_ms: u64,
}

// fired when confidence crosses a state boundary
ConfidencePromotionEvent {
    model_id: String,
    from: Confidence,
    to: Confidence,
    evidence: PromotionEvidence,           // ScoreboardThreshold | CouncilApproval
    timestamp_ms: u64,
}
```

These feed the L3 council report and CR-L8 corpus-feedback flywheel.

## 5. New CLI surfaces

```text
vox model discover [--bench-new]      # L1 refresh; --bench-new benches new models and moved -latest targets
vox model bench --max-usd <cap> ...   # graded benchmark; feeds routing (see §5.1)
vox model classify <ID> [--measure]   # L2 classifier; --measure derives tier/strengths from a real bench
vox model shadow <ID>                 # single-model bench under a small cap, prints the confidence verdict
vox model council-report              # generate the L3 quarterly markdown
```

### 5.1 Benchmarks feed routing

`vox model bench` (`crates/vox-cli/src/commands/model/bench/`) is the measurement
source for routing. Targets come from `--model` (catalog ids, or `provider:model`
for a direct wire: `groq:`, `cerebras:`, `mistral:`, `deepseek:`, `sambanova:`,
`anthropic:`, `gemini:`, `ollama:`, `voxlocal:`, `openrouter:`), `--provisional N`
(models routing does not yet trust), and `--latest N` (what each `~vendor/…-latest`
alias currently points at).

| Dimension | Measured as |
|---|---|
| Intelligence | Graded suites mapped to task categories: reasoning→Planning, codegen and vox-gen (compiled by `vox-compiler`)→CodeGen, json→Parsing, tool-call→ToolOrchestration, needle (8k/32k)→Research, chat→Chat. Pass rate with a Wilson 95% interval. |
| Responsiveness | Streamed time to first token, end-to-end latency p50/p95, tokens/sec. |
| Efficiency | Provider-reported cost (`usage.cost`), cost per solved case, tokens per solved case. |
| Reliability | Error rate by class (rate limit, auth, timeout, transport) and per-case score spread across `--repeats`. |
| Effective context | Largest needle the model retrieved. |

Spend is capped: the run refuses to start when the worst case (every case using
its full `max_tokens`, unpriced models assumed at $0.01/$0.03 per 1k) exceeds
`--max-usd`, and reserves each call's worst case against the cap before sending it.

Each case is persisted to `model_benchmark_results` (`vox-db`). On load,
`ModelRegistry::inject_evidence_from_db` turns a model's last 30 days into
`BenchEvidence`:

- Its quality prior becomes `QualitySource::Measured`, shrunk toward the prior index
  with weight 8 cases.
- Its routing confidence follows the Wilson verdict before any other signal. A model
  with at least 8 cases and a lower bound of at least 0.3 is Confirmed. One whose
  upper bound is below 0.3 is Shadowed, and background dispatch excludes it
  (`Exclusion::FailedBenchmark`).

Every bench run ends by printing the mode × category routing table
(`models::leaderboard::mode_routing_table`) that the same registry now produces.
Axis shows that table, plus the bench results, on the Models surface.

The served model id reported by the stream is recorded too. Bench an alias
(`openrouter:~vendor/x-latest`) and the report names the release that answered.
`models::adaptation` diffs catalog snapshots. When a `-latest` alias moves, its new
target is queued ahead of new unbenched models and is re-benched even if it was
benched before.

A category's strength tags gate its candidates, but they are catalog heuristics. When the
mode's pick with the tags ignored has a quality prior at least 0.15 higher than the tagged
pick, the untagged pick wins. Vision tags always gate. A model left out only for a
missing tag still supersedes older members of its family. An undated hosted entry (a
bootstrap model the live catalog no longer lists) yields to a dated family member with a
higher version. Undated local models are never superseded.

A live Ollama listing (`/api/tags`) decides which Ollama entries exist
(`ModelRegistry::reconcile_ollama`). The daemon's catalog refresh, `vox model bench`,
`vox chat --model auto` and the Axis Models surface all go through
`catalog::sync_installed_ollama`, which drops bootstrap Ollama entries that are not
installed. If Ollama is unreachable, it drops every Ollama entry.

If the database cannot be opened (a running daemon holds the user store's lock), the bench
warns, routes the rest of the run on its own results, and reports `persisted: false`. Set
`VOX_DATA_DIR` to bench into a separate store.

### 5.2 Live proof (2026-10-09)

`vox run scripts/model-routing-live-proof.vox` passed, spending at most $1.83 of its
$10 cap. All legs share an isolated `VOX_DATA_DIR`, so the results each leg saves are
what the next leg routes on.

- **Discover:** refreshed the catalog (364 OpenRouter, 6 Ollama, 3 Hugging Face models)
  and benched new models and moved aliases. A failing local model went from Confirmed
  to Shadowed.
- **OpenRouter:** 11 models, 22 cases each, for $0.33. The suite includes harder cases
  (clock-hand overlaps, inclusion–exclusion counting, Python mutable-default and
  aliasing traps, and JSON arithmetic), so scores spread from 0.65 to 0.98:
  - `deepseek/deepseek-v4.1-flash` 0.98 (1.0 s TTFT, 217 tok/s, $0.005 for the run);
    `openai/gpt-6-luna` and `x-ai/grok-4.7` 0.97 (grok cost 30× more);
    `anthropic/claude-haiku-5.5` 0.94; `google/gemini-3.8-flash` 0.65.
  - The alias `~google/gemini-flash-latest` was served by `google/gemini-3.8-flash`.
  - All 24 mode × category routes ran on measured quality. Balanced and genius mostly
    route to `deepseek-v4.1-flash`, and genius Research to `claude-haiku-5.5`.
- **Direct vendor wires:** skipped; no direct key resolves on the proof host.
- **Local:** `ollama:qwen3:8b` passed 25% of the suite, with a 53 s TTFT and $0 spent.
  Free and efficiency modes route to the best installed local model on measured
  evidence (`qwen2.5-coder:7b`, 0.58 over 17 cases). `auto-local` refused a dead
  endpoint instead of falling back to the cloud.
- **`vox chat --model auto`:** routed to `deepseek/deepseek-v4.1-flash` and answered.

The previous run had sent free, efficiency and balanced modes to `qwen/qwen-3.6-27b`.
That was a bootstrap Ollama entry that was never installed. Genius Planning had gone to
an undated bootstrap `claude-3-7-sonnet` that inherited its family's newest benchmark.
The Ollama reconciliation and supersession rules in §5.1 fix both.

One gap remains. Frontier models were not in the $10 proof sweep, so genius mode only
sees them through their public index, and a measured 0.98 flash model outranks them.
Bench them at a higher `--max-usd` before trusting genius routes to prefer them.

One gap remains. Frontier models were not in the $10 proof sweep, so genius mode only
sees them through their public index, and a measured 0.98 flash model outranks them.
Bench them at a higher `--max-usd` before trusting genius routes to prefer them.

## 6. Roll-out phases

| Phase | What | Status |
|---|---|---|
| **A** | `SelectionDecisionEvent` emit from `select()` | landed 2026-05-15 |
| **B** | Migrate `registry_model_resolve` to wrap `select()` | landed 2026-05-15 |
| **C** | Migrate `vox-code-audit::default_*_model()` to `select()` | landed 2026-05-15 |
| **D** | Split pinning from cataloging: `model-pins.v1.yaml` + rename bootstrap | landed 2026-05-15 |
| **E** | `vox models classify` scaffold + classifier prompt schema | landed 2026-05-15 |
| **F** | Nightly catalog-diff infrastructure + `DiscoveryEvent` | landed 2026-05-15 |
| **G** | Shadow-eval hook into llm-panel for Provisional models | landed 2026-05-15 |

| **H** | Graded `vox model bench` feeding routing (§5.1); shadow/classify run real benches; discover-bench-promote loop | landed 2026-10-08 |

Phases E/F/G landed as scaffolds. Phase H backs shadow and `classify --measure`
with real benchmark runs. The discover → bench → promote loop runs from
`vox model discover --bench-new`. The daemon has no periodic catalog refresh, so
schedule that command if you need it to recur. The live end-to-end proof is
`vox run scripts/model-routing-live-proof.vox`.

## 7. Backwards compatibility

- `bootstrap_inference::*` constants kept as last-resort fallbacks; not in
  the hot path.
- `model-catalog.bootstrap.v1.json` kept at its filename for one release
  with a deprecation pointer to `catalog-fallback.v1.json`.
- Production routing remains in `models::select::decide` and registry routing.
  `resolve_model_with_registry_fallbacks` has no production callers and is not
  a maintained routing surface; see
  [ADR-046](../adr/046-pareto-frontier-reporting.md). Pareto-frontier behavior
  is reporting-only and does not change model eligibility, promotion, or routing.

## 8. Open questions

- **Classifier model choice.** Haiku 4.5 vs Gemini 3.1 Flash-Lite. Both
  support structured-output JSON schema. Lean Haiku for the prompt-cache
  TTL during shadow runs.
- **Promotion threshold.** Graded benches promote by Wilson verdict (§5.1).
  Production success counts still promote via `should_promote`, but they no longer
  write quality: success is not correctness.
- **Mesh peer classification.** PopuliMesh entries don't have an OpenRouter
  description. Treat them as `Confirmed`+`Local` at registration time and
  let scoreboard drive demotion.

---

*Document dated 2026-05-15. SSOT for the model-autonomic system; supersedes
the bootstrap-as-SSOT model.*
