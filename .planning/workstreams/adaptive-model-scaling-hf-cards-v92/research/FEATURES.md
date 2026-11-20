# Feature Research: Adaptive Model Scaling & HF Cards (v9.2)

**Domain:** 4-signal coding-agent model scaling (`Host Inference VRAM × Mesh Peers × Credentials × Task Complexity`), cross-surface operator routing transparency (`vox model explain`, `ChatExecutionRail`, Axis `ModelsView`), and Hugging Face Hub model cards / Inference Providers (`v9.2`).
**Researched:** 2026-11-19
**Confidence:** HIGH (verified against Vox codebase modules, Askama Hub templates/golden cards, and 2026 coding-agent + HF Hub ecosystem standards)

## Feature Landscape

### Table Stakes (Users Expect These)

Features users and HF Hub ingestion pipelines assume exist by default. Missing these makes routing feel broken or untrustworthy, and hides models from ecosystem tooling.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Unified Credential-Gated Selection Parity (`vox model explain` == Dispatch == Axis)** | Developers expect `vox model explain` and Axis (`ModelsView.tsx`) to reflect the exact selection rules runtime dispatch uses. Right now `vox model explain` (`crates/vox-cli-research/src/model/explain.rs`) calls `ModelRegistry::explain_selection`, which skips `selection_key_available` and `DispatchGate`, so CLI can recommend cloud models with no configured API key while Axis `RoutingExplainer` (`crates/vox-orchestrator-mcp/src/gui_ops/dto/routing_explain.rs`) excludes them (`NoProviderKey`). | MEDIUM | Route `vox model explain` through `best_for_task_under_gate` / shared explanation DTO (or share `DispatchGate` + `key_guard::selection_wire`), keeping `--free-tier` rationale intact. |
| **Inference VRAM vs. Training Memory Separation on HF Cards** | Users checking a 2B or 4B model card expect to know if it fits their 4–16 GB GPU for inference. Currently `crates/vox-populi/templates/hub/index.md` (`Peak memory`) and `model.md` (`Measured QLoRA peak memory ... was 27007 MiB` in `tests/golden/hub_cards/passed.md`) display QLoRA training peak MiB without inference VRAM specs, misleading visitors into thinking a 2B model needs 27 GB to run. | LOW | Add explicit **Inference VRAM (`Q4_K_M`, `Q8_0`, `BF16` + KV context headroom)** fields/columns separate from **Training Peak Memory** across `index.md`, `model.md`, and `CardFacts`. |
| **Dynamic Host Free-VRAM Probing Across CUDA, Metal & CPU** | LM Studio and `llmfit` check GPU/unified memory before recommending local models. In Vox, `crates/vox-orchestrator/src/models/auto_select.rs` reads Metal + `nvidia-smi` (`size_select::read_free_memory`), but `crates/vox-orchestrator/src/models/vram.rs` (`free_vram_mb_from_probe`) is NVML-only — degrading Apple Silicon and AMD hosts to `VramFit::Unknown` in `scoring.rs`. | MEDIUM | Unify `vram.rs` hint refresh with `host_free_memory()` fallback (`NVML -> Metal -> nvidia-smi -> Unknown`) and keep `VramFit::Unknown` a strict no-op when headless/CPU-only. |
| **Quantization- & Context-Aware `VramFit` Classification** | Modern local runners (LM Studio, Ollama) budget `Weights (Q4/Q8/BF16) + KV Cache(ctx)` rather than weights alone. `vram.rs` hardcodes `BYTES_PER_PARAM_DEFAULT_Q4 = 0.5` + flat `0.15` KV overhead, misclassifying `BF16` vs `Q4_K_M` footprints and long-context spikes. | MEDIUM | Parameterize bytes-per-param by quantization tag (`Q4` ~0.55, `Q8` ~1.05, `BF16` ~2.0) and context length, surfacing `Comfortable` / `Tight` / `Exceeds` / `Unknown` badges in CLI, Axis, and cards. |
| **HF Ecosystem Metadata Parity on `index.md` & Merged CausalLM Cards** | Hugging Face search filters, Inference Providers (`router.huggingface.co/v1`), and widget UIs require `library_name: transformers` (merged) or `peft` (adapter-only), valid `tags` (`conversational`, `text-generation-inference`, `vllm`, `qwen3_5`), and root `tokenizer` + `chat_template.jinja`. `crates/vox-populi/templates/hub/index.md` still hardcodes `library_name: vox`. | LOW | Update `index.md` frontmatter and ensure `model.md` / `use_it.md` keep `library_name`, `datasets`, `spaces`, `.eval_results/*.yaml`, and `model-index` compliant (completing `v7.7` gaps G1–G6, G9). |
| **Working HF Inference Router (`:auto` / `:featherless-ai`) & vLLM/Ollama Quickstarts** | HF visitors copy-paste the top OpenAI SDK snippet pointing at `https://router.huggingface.co/v1` with `model="<ns>/repo:auto"` or `":featherless-ai"`. Currently `use_it.md` emits Router/Ollama/vLLM blocks only when `is_complete_model()` is true; adapter-only cards (`passed.md`) lack `vllm serve --enable-lora` and Router guidance. | LOW | Include `vllm serve <base> --enable-lora` and base-model Router fallback snippets on adapter-only cards alongside merged-card `:auto` / `:featherless-ai` / `:fastest` / `:cheapest` examples. |
| **Light/Dark Theme-Adaptive Media on HF Model Cards** | HF Hub supports both light and dark UI themes; static dark-background JPEGs (`assets/banner.jpg`, `assets/overview.jpg` via `svg.rs`) look jarring or clipped in light mode (and white-bg charts blind dark-mode users). | LOW | Emit `<picture><source media="(prefers-color-scheme: dark)" srcset="...">` (or `#hf-dark-mode-only` / `#hf-light-mode-only` Markdown variants, or `@media (prefers-color-scheme)` SVGs linked inside `<picture>`/`<img>`) in `model.md` and `index.md`. |
| **Working Space Deep-Links (`?model=<ns>/<repo>` -> Correct Rung)** | Every `model.md` and `index.md` card links to `.../vox-mens-playground?model={{ f.repo_id }}`, but `crates/vox-populi/templates/hub/space/index.html.tmpl` `resolveSizeParam` strips `^vox-mens-` and `^qwen3...` without stripping `<namespace>/` — causing every card deep-link to silently fall back to `0.8B`. | LOW | Strip leading `[^/]+/` namespace in `resolveSizeParam`, preserve `?provider=` / `?tab=webgpu`, and support `?model=` canonical repo resolution cleanly. |
| **Remove Stale Hardcoded `Qwen3-8B` Spoke Label in `ChatExecutionRail.tsx`** | Operators using `Qwen3.5-{2B,4B,9B}` or `Qwen3.8-27B` or cloud frontier models should never see `vox_lang · Qwen3-8B` fabricated from task-title substrings (`ChatExecutionRail.tsx` lines 867–880). | LOW | Replace title-regex + hardcoded `Qwen3-8B` string with live `RailRouting` / `MensSpoke` telemetry from backend `RoutingSummaryDto` (and hide/fallback cleanly when non-MENS). |

### Differentiators (Competitive Advantage)

Features that set Vox apart from Cursor `Auto`, Cline/Roo, Ollama/LM Studio, and generic fine-tune cards.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **4-Signal Scaling Matrix (`VRAM × Mesh Peers × Credentials × Complexity`)** | Unlike Cursor (cloud-only `Auto`), Cline (static single-model/mode), or LM Studio (local-only VRAM fit), Vox dynamically routes across **local host VRAM**, **trusted Populi P2P GPU peers**, **credential-verified cloud wires** (`Native` vs `OpenRouterFallback` vs `HuggingFaceRouter`), and **task complexity (`1–10`)** in one unified scorer. | HIGH | Extend `ModelSpec` / `SelectionIntent` / `auto_score_parts` so local `VramFit::Comfortable` + low-queue Populi mesh peers compete fairly against cloud models at low/medium complexity while high complexity escalates to `27B`/cloud Elite. |
| **Live Mesh Peer Capacity & Queue Telemetry in Routing** | `catalog_seed.rs` currently registers `mesh/<peer_key>/<kind>` with default `ModelCapabilities` and `ranking.rs` only tie-breaks on `VOX_ROUTING_PREFER_MESH`. Exposing peer free VRAM, loaded model/quant, RTT latency, and `mesh_directory::queue_stats()` depth makes cluster scaling genuine rather than blind. | HIGH | Thread peer capabilities + `queue_stats` into `ModelSpec` / `ScoreParts` so overloaded or undersized mesh peers gracefully yield to local host or cloud routes. |
| **Interactive Complexity Slider & 4-Signal Breakdown in Axis `ModelsView` (`RoutingExplainer`)** | `ModelsView.tsx` currently hardcodes `complexity = 7` (`useRoutingExplanation(mode, task, 7)`) and `RoutingExplainer.tsx` omits `PartsDto.bonuses` (`vram` is hidden!). Giving operators a `1–10` complexity dial + visible `VRAM`, `Mesh`, `Cred Wire`, and `Bonuses` columns makes routing behavior interactively inspectable before running tasks. | MEDIUM | Add complexity slider (`1–10`) in `ModelsView.tsx` `RoutingPanel`; split `PartsDto.bonuses` (or expose `vram_fit`, `wire_mode`, `mesh_peer_status`) in `routing_explain.rs` and render in `RoutingExplainer.tsx`. |
| **Compact 4-Signal Posture Pill in `ChatExecutionRail.tsx`** | Coding tools rarely tell the user *why* a turn scaled down to local 4B vs up to cloud/27B without opening logs. Showing `Host VRAM · Mesh Peers · Auth Wire · Cx <1-10>` directly inside the Chat rail's `Routing` section (with click-through to `ModelsView`) builds instant operator trust. | MEDIUM | Extend `RoutingSummaryDto` / `RailRouting` in `crates/vox-orchestrator-mcp/src/gui_ops/models.rs` and `routingSummary.ts`; keep rail compact using progressive disclosure (`<details>`). |
| **CLI Structured + Visual 4-Signal Explanation (`vox model explain --json` & Signal Matrix)** | Gives power users and CI scripts exact provenance of why a route won or was gated (`NoProviderKey`, `Exceeds` VRAM, `PrivacyLocalOnly`, `FlagshipExcludedByMode`, peer queue saturation) with Wilson 95% CIs (`MIN_CALLS_FOR_CONFIDENT_RANK`) already built in `explain.rs`. | MEDIUM | Add `--json` and 4-signal header table (`Host VRAM`, `Mesh Peers`, `Credentials`, `Complexity + Clutch`) to `crates/vox-cli-research/src/model/explain.rs`. |
| **HF Collections Cross-Linking + Provenance-Verified `.eval_results` & Paired Diffs** | Most HF model cards show cherry-picked prose. Vox already renders deterministic `Fixed`/`Still fails` held-out diffs (`examples.md`), McNemar collateral regression tests (`mens-models.md`), and `hub.rs` `/api/collections` API calls. Surfacing the **Collection badge/link**, **4-signal deployment matrix**, and **Space deep-links with preselected provider** creates top-tier Hub adoption. | LOW | Wire collection URL (`https://huggingface.co/collections/<ns>/<slug>`) and `?size=<S>&tab=webgpu&provider=auto` links into `index.md` and `model.md` while passing the cross-card 4-word sentence uniqueness lint. |

### Anti-Features (Commonly Requested, Often Problematic)

Features that look appealing on the surface but create maintenance traps, silent failures, or UX clutter.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **Hard-Blocking Local Models on `VramFit::Exceeds`** | "Prevent OOMs by hiding models larger than detected VRAM." | As documented in `vram.rs`, NVML/Metal readings can be stale or miss custom quants (`IQ3_S`, partial offload, unified swap). Hard exclusion silently hides working user setups and makes headless/VM machines show zero local models. | Keep `VramFit::Exceeds` as a **soft ranking penalty + explicit warning badge** in CLI/Axis/Chat, never a hard exclusion (`VramFit::Unknown` stays a `0.0` no-op). |
| **Heavy Auto-Switching Mid-Conversation Without Cache Awareness** | "Switch between 0.8B, 4B, 9B, and Claude every single chat turn based on prompt length." | Invalidates KV/prompt caches (raising cloud cost and local TTFT latency, as Cursor's cache-aware router explicitly warns against) and causes jarring style/capability whipsaw inside one task. | Keep turn-level routing sticky per task/session intent (`ClutchProfile` + `SelectionIntent`), escalating only on explicit complexity thresholds, repair loops, or user/mode switch. |
| **Publishing `library_name: vox` or Root `adapter_config.json` Beside Merged Weights** | "Brand the HF YAML metadata as Vox or put every file in root so all loaders see it." | Custom `library_name: vox` locks models out of HF ecosystem filters and Serverless Inference Providers; placing `adapter_config.json` at root alongside `model.safetensors` breaks `transformers` / vLLM / Featherless AI auto-detection. | Emit `library_name: transformers` (merged, PEFT in `peft/`) or `library_name: peft` (adapter-only), keeping `candle_qlora_adapter.safetensors` at root for `vox mens hub pull`. |
| **Reporting QLoRA Training Peak Memory as Model Memory on Cards** | "Auto-print `TrainingManifest` peak MiB in the card summary table." | Training 2B/4B with QLoRA + optimizer/activations spikes to ~27 GB (`27007 MiB` in `tests/golden/hub_cards/passed.md`), scaring away 8–16 GB inference users who only need 2–5 GB VRAM. | Separate **Inference VRAM (`Q4_K_M` / `BF16`)** in the above-the-fold TL;DR table from **Training Peak Memory** inside the `## Training` provenance paragraph. |
| **Duplicating Long Onboarding / Python Boilerplate Across Every Ladder Card** | "Put full Vox tutorials and Python SDK docs on `0.8B`, `2B`, `4B`, `9B`, and `27B` cards." | Triggers Vox's cross-card 4-word sentence repetition lint (`hub_card/lints.rs`), bloats `README.md` (`passed.md` is 389 lines), and buries benchmarks/run snippets below the fold. | Keep per-rung `model.md` focused on size-specific VRAM fit, Inference Router/local snippet, and verified diffs; link deep onboarding/language docs to `voxlang.org/reference/mens-models/` and the Collection index. |
| **Inline Raw `<svg>` With Embedded Scripts or Unescaped HTML in HF Markdown** | "Embed interactive SVG charts directly inside HF `README.md`." | HF's Markdown sanitizer strips inline `<svg>` tags, `<style>` blocks, and scripts, leaving broken/empty blocks on the live Hub page. | Store light/dark SVGs or raster PNG/JPG pairs in `assets/` and reference them via `<picture><source media="(prefers-color-scheme: dark)" srcset="...">` or `#hf-dark-mode-only` / `#hf-light-mode-only`. |

---

## Feature Dependencies

```
[Unified Host Free-VRAM Probe (CUDA + Metal)]
    └──enables──> [Quantization + Context-Aware VramFit]
                      ├──feeds──> [4-Signal Scorer (VRAM × Mesh × Creds × Complexity)]
                      └──feeds──> [HF Card Inference VRAM Table]

[Mesh Peer Capability & Queue Telemetry]
    └──feeds──> [4-Signal Scorer (VRAM × Mesh × Creds × Complexity)]

[Clavis Key Guard + OpenRouter/HF Router Wire Resolution]
    └──feeds──> [4-Signal Scorer (VRAM × Mesh × Creds × Complexity)]
                      ├──powers──> [Parity Explanation DTO (CLI + Axis + MCP)]
                      │                ├──renders──> [vox model explain (CLI)]
                      │                ├──renders──> [ModelsView.tsx + RoutingExplainer.tsx]
                      │                └──renders──> [ChatExecutionRail.tsx Posture Pill]
                      └──unblocks──> [Router Egress Governance]

[index.md & Dual-Layout HF Hub Metadata Parity]
    ├──enables──> [HF Inference Router (:auto / :featherless-ai) Quickstarts]
    ├──enables──> [Collections API Linking (/api/collections)]
    └──enhances──> [Dark/Light <picture> Assets + Space Deep-Link Fix (?model=)]
```

### Dependency Notes

- **Unified Host VRAM Probe -> `VramFit` -> 4-Signal Scorer:** Without Metal/`nvidia-smi` fallback in `vram.rs`, Apple Silicon and non-NVML hosts report `VramFit::Unknown`, disabling local VRAM scaling in `scoring.rs` even though `auto_select.rs` already reads them.
- **4-Signal Scorer -> Parity Explanation DTO -> CLI / Axis / Chat:** Today `vox model explain` (`explain_selection`), Axis `ModelsView` (`routing_explain::explain`), and `ChatExecutionRail` (`routing_summary`) call three different query entry points! Consolidating explanation onto one canonical gate-aware DTO prevents drift across surfaces.
- **Space `resolveSizeParam` Fix enhances HF Card Deep-Links:** Fixing namespace stripping (`ns/vox-mens-qwen3.5-2b` -> `2B`) in `space/index.html.tmpl` must ship alongside updated card CTA links so clicking `Open 4B Playground` lands on 4B instead of silently resetting to `0.8B`.
- **Anti-Feature Guard (Cross-Card Uniqueness Lint):** Any copy added to `model.md` / `index.md` / `use_it.md` must interpolate `{{ f.size }}` or concise unique phrasing so `crates/vox-populi/src/mens/hub_card/lints.rs` does not fail on repeated 4-word sentences.

---

## MVP Definition

### Launch With (v1 — Milestone v9.2 Core)

Minimum cohesive set to ship 4-signal scaling transparency and 2026-grade HF model cards:

- [ ] **Parity Explanation Core (`explain` CLI == Axis == Dispatch):** Wire `vox model explain` to the same credential/policy gate (`DispatchGate` + `key_guard`) and expose a structured 4-signal header (`Host VRAM`, `Mesh Peers`, `Active Auth Wire`, `Complexity / Clutch`) plus `--json`.
- [ ] **Unified Host VRAM & `VramFit` Transparency:** Bridge `auto_select::host_free_memory` fallback (CUDA NVML + Apple Metal + `nvidia-smi`) into `vram.rs`, expose `VramFit` (`Comfortable`/`Tight`/`Exceeds`/`Unknown`) as an explicit field in `CandidateDto` (not buried inside `bonuses`), and render it in `RoutingExplainer.tsx` + `vox model explain`.
- [ ] **Interactive Complexity & Honest Chat Rail:** Add a `1–10` Complexity selector to `ModelsView.tsx` (`RoutingPanel`); remove hardcoded `Qwen3-8B` title-matching in `ChatExecutionRail.tsx` and replace with live backend `RailRouting` + compact 4-signal posture summary.
- [ ] **HF Card Discovery & Inference VRAM Clarity:** Fix `library_name` in `index.md` (`transformers`/`peft`), separate **Inference VRAM (`Q4_K_M`, `BF16`)** from **QLoRA Training Peak Memory** in `index.md` and `model.md`, add `:auto` / `:featherless-ai` + `--enable-lora` quickstarts for both merged and adapter cards, and surface HF Collection URLs.
- [ ] **HF Dark/Light Hero Media & Space Deep-Link Fix:** Add `<picture><source media="(prefers-color-scheme: dark)" ...>` (or `#hf-dark-mode-only` / `#hf-light-mode-only` pairs) to `model.md` and `index.md`, and fix `resolveSizeParam` namespace stripping in `space/index.html.tmpl`.

### Add After Validation (v1.x)

- [ ] **Quantization- & Context-Window-Aware `estimate_vram_fit`:** Factor exact model quant (`Q4_K_M`, `Q8_0`, `BF16`) and prompt/context budget (`num_ctx`) into `vram.rs` KV-cache sizing rather than flat `0.5 B/param + 15%`.
- [ ] **Rich Populi Mesh Peer Telemetry in `catalog_seed.rs` & UI:** Surface per-peer VRAM headroom, loaded adapter/quant, RTT latency, and `mesh_directory::queue_stats()` load inside `ModelsView` and `vox model explain`.
- [ ] **Interactive Cost/Latency/Quality Trade-off Preview in `ChatExecutionRail`:** Show estimated turn cost/tokens per alternative (`local` vs `mesh` vs `cloud` wire) in the `"Why this model"` disclosure.

### Future Consideration (v2+)

- [ ] **Cache-Aware Sticky Escalation:** Mirror Cursor's prompt-cache-miss penalty so small complexity shifts do not evict warm local KV caches or Anthropic/OpenRouter prompt caches mid-thread.
- [ ] **Dynamic Multi-Node Mesh Offload / Speculative Routing:** Split draft/verify across local small MENS rung (`0.8B`/`2B`) and mesh/cloud `27B`+ verifier.

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| `vox model explain` credential/gate parity + 4-signal output | HIGH | MEDIUM | **P1** |
| Remove hardcoded `Qwen3-8B` in `ChatExecutionRail.tsx` + add 4-signal posture | HIGH | LOW | **P1** |
| Separate Inference VRAM (`Q4`/`BF16`) vs Training Peak RAM on HF cards | HIGH | LOW | **P1** |
| Fix `index.md` `library_name`, Collections link & adapter `use_it.md` Router/LoRA snippets | HIGH | LOW | **P1** |
| Fix Space `resolveSizeParam` namespace stripping for `?model=` deep-links | HIGH | LOW | **P1** |
| Expose `VramFit` & `bonuses` breakdown in `routing_explain.rs` + `RoutingExplainer.tsx` | HIGH | MEDIUM | **P1** |
| `ModelsView.tsx` complexity (`1–10`) interactive control | MEDIUM | LOW | **P1** |
| Unified CUDA/Metal host VRAM refresh for `vram.rs` scoring | HIGH | MEDIUM | **P1** |
| Dark/light `<picture>` support for HF `model.md` / `index.md` banners | MEDIUM | LOW | **P1** |
| Mesh peer VRAM + `queue_stats` scoring & UI card display | HIGH | HIGH | **P2** |
| Quantization + context-length dynamic KV-cache `VramFit` math | MEDIUM | MEDIUM | **P2** |
| Cache-aware sticky escalation (prompt-cache miss penalty) | MEDIUM | HIGH | **P3** |

**Priority key:**
- **P1:** Must have for Milestone v9.2 launch
- **P2:** Should have in v9.2.x follow-up once 4-signal baseline is wired
- **P3:** Future consideration after routing telemetry validates usage

---

## Competitor Feature Analysis

| Feature | Cursor (2026) | Cline / Continue (2026) | Ollama / LM Studio (2026) | Vox v9.2 Approach |
|---------|---------------|-------------------------|---------------------------|-------------------|
| **Task Complexity Auto-Scaling** | Native `Auto` mode routes simple vs complex tasks dynamically; prompt-cache-aware. | Manual model per Custom Mode (Architect vs Code) or external proxy router (LiteLLM/Requesty). | None (single loaded model). | Continuous `1–10` complexity + `ClutchProfile` (`Efficiency`/`Balanced`/`Genius`/`Free`) + `SelectionIntent`. |
| **Host VRAM Fit UX** | N/A (cloud-hosted models). | None built-in; relies on user configuring Ollama/LM Studio correctly. | **LM Studio:** Green/Yellow/Red pre-download badges + weight/KV/offload sliders. **Ollama:** `ollama ps`, `OLLAMA_KV_CACHE_TYPE`, implicit hybrid offload. | Soft-ranking `VramFit` (`Comfortable`/`Tight`/`Exceeds`/`Unknown`) across CUDA NVML + Metal, never hard-blocking; surfaced in CLI/Axis/Chat. |
| **Mesh / LAN Peer Scaling** | None. | None (single endpoint URL per profile). | Manual remote `OLLAMA_HOST` URL. | Built-in Populi P2P trusted peer discovery (`mesh/<peer>/<kind>`) with DB Sybil/failure blacklisting and `queue_stats` load signals. |
| **Credential & Fallback Transparency** | Managed subscription abstraction (opaque). | Manual API key entry per provider; errors only on failed request. | N/A (local keys only). | Clavis key guard + `InferenceWire::Native` vs `OpenRouterFallback` + `HuggingFaceRouter`, pre-gated and grouped as `NoProviderKey` in explainer. |
| **Operator Explanation Surface** | Minimal ("Auto" badge in composer). | Shows active mode model name + token cost per step. | `ollama ps` / LM Studio resource monitor HUD. | Synchronized across CLI (`vox model explain`), Chat (`ChatExecutionRail`), and Axis (`ModelsView` `RoutingExplainer` stacked bars + exclusions). |
| **HF Model Cards & Serverless Providers** | N/A | N/A | LM Studio reads GGUF Hub repos; Ollama pulls `hf.co/<org>/<repo>`. | Dual-layout CausalLM (`library_name: transformers`) + `peft/` subfolder, `.eval_results/*.yaml`, Collections API, HF Router `:auto`/`:featherless-ai`, WebGPU Space, light/dark `<picture>`, Inference VRAM vs Training RAM clarity. |

---

## Sources

### Verified Repository Sources
- `crates/vox-orchestrator/src/models/vram.rs` — `VramFit`, `estimate_vram_fit`, NVML `VramProbe`, soft ranking design notes
- `crates/vox-orchestrator/src/models/auto_select.rs` — `host_free_memory` Metal/`nvidia-smi` fallback
- `crates/vox-orchestrator/src/models/scoring.rs` — `VRAM_EXCEEDS_PENALTY`, `ScoreParts`, `vram_score_delta`
- `crates/vox-orchestrator/src/models/key_guard.rs` — `CANDIDATE_PROVIDERS`, `InferenceWire`, `openrouter_can_serve`
- `crates/vox-orchestrator/src/models/mode_select.rs` & `ranking.rs` — `DispatchGate`, `Exclusion`, `VOX_ROUTING_PREFER_MESH`
- `crates/vox-orchestrator/src/models/registry/catalog_seed.rs` & `mesh_directory.rs` — `PopuliMesh` peer registration & `queue_stats`
- `crates/vox-orchestrator/src/models/registry/selection_queries.rs` & `crates/vox-cli-research/src/model/explain.rs` — `explain_selection` vs `DispatchGate` gap
- `crates/vox-orchestrator-mcp/src/gui_ops/dto/routing_explain.rs` & `gui_ops/models.rs` — `RouteExplanationDto`, `PartsDto`, `RoutingSummaryDto`
- `crates/vox-gui/ui/src/components/surfaces/Models/{ModelsView.tsx, RoutingExplainer.tsx, BackendAvailability.tsx, MensServePanel.tsx}`
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatExecutionRail.tsx` & `crates/vox-gui/ui/src/lib/routingSummary.ts`
- `docs/src/architecture/hf-model-cards-and-inference-providers-research-2026.md` & `docs/src/reference/mens-models.md`
- `crates/vox-populi/templates/hub/{index.md, model.md, partials/frontmatter.md, partials/status_line.md, partials/use_it.md, partials/onboarding.md, partials/examples.md, space/index.html.tmpl}`
- `crates/vox-populi/tests/golden/hub_cards/{index.md, passed.md}`, `crates/vox-populi/src/mens/{hub.rs, hub_card/svg.rs}`

### 2026 Ecosystem & Standards Verified via Web Search
- Cursor `Auto` mode complexity & prompt-cache-aware routing; Cline / Continue / Roo Code custom modes & LiteLLM/Requesty router patterns
- LM Studio pre-download Green/Yellow/Red VRAM fit badges, offload/KV-cache sliders; Ollama `ollama ps`, `OLLAMA_KV_CACHE_TYPE`, hybrid PCIe offload penalty, and `llmfit` CLI calculators
- Hugging Face Inference Providers (`https://router.huggingface.co/v1`), `:auto`, `:featherless-ai`, `:fastest`, `:cheapest` model suffixes, `InferenceClient(provider="featherless-ai")`
- Hugging Face Hub README dark/light media standards (`<picture><source media="(prefers-color-scheme: dark)" srcset="...">`, `#hf-light-mode-only` / `#hf-dark-mode-only`, sanitizer rules for inline `<svg>`)
- Hugging Face Model Card VRAM vs. training memory formulas, Collections management, and revision/Space deep-linking standards

---
*Feature research for: Adaptive Model Scaling (4-Signal) & HF Cards (Milestone v9.2)*
*Researched: 2026-11-19*
