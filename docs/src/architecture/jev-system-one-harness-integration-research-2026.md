---
title: "Jev / System One for Vox harness & orchestrator — research 2026-10-28"
description: "Jev/System One for Vox: redundancy vs select/free_floor/hub-spoke, better harness surfaces, Oct 2026 alternative-model performance, bakeoff guidance."
category: "Architecture SSOTs"
status: "research"
topic: "model-routing"
domain: "orchestration"
authorship: "llm-assisted"
authorship_model: "composer"
code_anchors:
  - crates/vox-orchestrator/src/models/select.rs
  - crates/vox-orchestrator/src/confidence_fusion.rs
  - crates/vox-orchestrator/src/models/autonomic.rs
---

# Jev / System One for Vox harness & orchestrator

<!-- kb:summary -->
Integrate System One / RLCD ideas by feeding calibrated answers into `build_selection_request` and existing repair/stop hooks — not by invoking PreferFree or rewriting `FREE_FLOOR`. `allow_free_in_performance_mode` only lets `:free` models compete in the scorer. ChatARC composes skill, MCP, and command catalogs. Do not override clutch, emit model ids, or distill into MENS chat. See §17.
<!-- /kb:summary -->
> **Revision (2026-10-28, pass 4).** Pass 3 = redundancy + alternative performance (§11–§15). Pass 4 drops facade framing: map **RLCD / System One research** onto a fully plumbed harness turn (when/how `select` runs, PreferFree via intent fields, mid-loop escalate/stop)—worked example in §16. **Do not treat hypothesis tables as a committed roadmap.**
Companions: [`autonomous-orchestration-policy-research-2026.md`](autonomous-orchestration-policy-research-2026.md), [`multi-provider-local-cloud-routing-research-2026-07-30.md`](multi-provider-local-cloud-routing-research-2026-07-30.md), [`model-catalog-ssot-2026.md`](model-catalog-ssot-2026.md), [`agentos-ssot-2026.md`](agentos-ssot-2026.md), [`search-retrieval-ssot-2026.md`](search-retrieval-ssot-2026.md), [`voxmens-hub-and-spoke-ssot-research-2026-06-18.md`](voxmens-hub-and-spoke-ssot-research-2026-06-18.md), [`voxmens-finetuning-boundaries-research-2026-06-21.md`](voxmens-finetuning-boundaries-research-2026-06-21.md), [`vox-harness-graph-audit-2026-07-30.md`](vox-harness-graph-audit-2026-07-30.md). Normative generative boundary: `AGENTS.md` §Model-Agnostic LLM Boundary.
... [lean-ctx: omitted 1 lines]
## 0. Direct answers (operator FAQ)
| Question | Answer |
... [lean-ctx: omitted 2 lines]
| **Can we download Jev weights / self-host?** | **No public weights or self-host binary.** Hosted API only (`POST /v1/systemone`). Enterprise page lists VPC/on-prem as a sales question, not a documented product. Community “Jev-like” weights are **not** Jev. |
| **May we distill / imitate Jev into MENS?** | **Likely prohibited.** MCA language reported by secondary digests bans distillation / imitating Service outputs / competing products. AutoResearch-style use of Jev probs as **features for your own labels** is what TypeSafe’s cookbook shows — still get legal review before any training on API outputs. |
| **Should we put Jev on the hot path for faster responses?** | **Only when cloud egress is already allowed and the judgment is closed-set.** Vendor E2E is ~70–500 ms *plus* RTT/gateway. For local-only / privacy-strict / sub-50 ms floors, use **rules → local classifier / MENS constrained pick → optional cloud Jev → frontier LLM**. Jev is often *cheaper* than a frontier call, not always *faster* than a local SLM. |
... [lean-ctx: omitted 2 lines]
## 1. Critique of the pass-1 draft (what was wrong or weak)
| Issue | Why it was bad | Correction |
... [lean-ctx: omitted 1 lines]
| Implied “new crate `vox-system-one`” | Crate-edges / layer policy; premature | Prefer a thin client in existing egress (`vox-http-client` / orchestrator module) **if** adopted; no new workspace crate without USER-AUTHORIZED edges |
... [lean-ctx: omitted 1 lines]
| “Replace cross-encoder with Jev” | `vox-search` already has term-density / hybrid RRF; CLERC cookbook ≠ SOTA claim | Optional **stage after shortlist**, A/B against current reranker; never rip out lexical/hybrid first |
... [lean-ctx: omitted 1 lines]
| Banking “wire transfer” examples | Not Vox’s MCP surface; confuses readers | Use real irreversible classes: destructive VCS, secrets write, external effect, policy grant (`oversight/floor.rs`) |
... [lean-ctx: omitted 3 lines]
| Conflated registry “confidence” with Jev confidence | `autonomic::ModelConfidence` ≠ Choice confidence | Keep namespaces separate in telemetry |
**What pass 1 got right:** non-generative contract; parallel questions; Kahneman System-1/2 split; do not stuff through `llm_chat`; escalate on low confidence; jaggedness (math/dates/counts in code).
... [lean-ctx: omitted 1 lines]
## 2. What Jev is (primary)
| Axis | System One / Jev | Frontier / MENS chat |
... [lean-ctx: omitted 4 lines]
| Hosting | TypeSafe API and **OpenRouter** (`typesafe/jev-*`, Decisions API / System One base URL) | Multi-provider |
... [lean-ctx: omitted 3 lines]
- Aliases move (`jev-latest` → currently `jev-1.14.0`); **pin versioned IDs** when thresholds are tuned.
... [lean-ctx: omitted 1 lines]
- Coding agents: Jev is **not** a drop-in for Cursor/Claude Code models ([coding-agents](https://docs.typesafe.ai/introduction/coding-agents.md)).
... [lean-ctx: omitted 1 lines]
- [Evaluating and Benchmarking the System One Model Jev](https://arxiv.org/abs/2609.37647) (`jev-1.13.0`, 37 datasets, ~346k requests): strong on several classification/NLI sets vs Qwen3.8-27B / Gemma-4-E4B on identical option-prob protocols; choice probs well calibrated for selective prediction; **binary Nouls need threshold tuning** (UNFAIR-ToS micro-F1 0.50→0.75); option rotation does not change accuracy.
- [Jev-ToM](https://arxiv.org/abs/2610.18610): typed ToM selection; Jev competitive; confidence calibration **task-dependent**; candidate design matters.
Treat vendor 40–400× speed/cost claims as **workflow-self-eval** (blog discloses bias), not universal.
... [lean-ctx: omitted 1 lines]
## 3. Fine-tune, download, and “should we build our own?”
### 3.1 Fine-tune Jev — no
Official: not fine-tuned / LoRA’d per customer; same weights; customize via request + code ([Models — Customizing Jev](https://docs.typesafe.ai/models.md)).
### 3.2 Download / self-host Jev — no (public)
No HF weight drop, no self-host binary in docs. OpenRouter and Vercel AI Gateway are **hosted proxies** to TypeSafe, not local runtimes ([OpenRouter Jev](https://openrouter.ai/docs/guides/community/jev)).
### 3.3 Train something *like* System One for Vox — maybe, carefully
| Path | Fit for Vox | Risk |
... [lean-ctx: omitted 3 lines]
| **C. Jev probs as features → classical head** (AutoResearch pattern) | Good when cloud Jev allowed offline for dataset build | MCA: features-for-your-labels OK in cookbook spirit; **imitation/distillation banned** — legal review |
| **D. Community Jev-like** (Jevlike, decider/Qwen3.5, JevForge, ModernBERT “open jev”, etc.) | Possible air-gap System-1 API shape | Quality/calibration unproven vs Jev; supply-chain; **not** TypeSafe; do not claim parity |
... [lean-ctx: omitted 1 lines]
| **F. Llama-Guard / ShieldGemma / NeMo-style rails** | Local guardrails | Different task than general Choice routing |
**Recommendation:** Do **not** fine-tune Jev (impossible). Do **not** distill Jev into MENS. For faster *local* responses: invest in **B/E** (labeled routers + constrained gen) and keep **hosted Jev (via OpenRouter)** as an optional cloud tier when D8 allows. Community Jev-likes are a spike candidate for air-gap only after an eval harness, not a default dependency.
### 3.4 Latency reality
```text
Local rule / embedding kNN     ~0.1–5 ms
Local SLM / MENS constrained   ~10–200+ ms (device-bound)
Hosted Jev (vendor claim)      ~70–500 ms compute + WAN/gateway RTT
Frontier LLM                   seconds (often)
```
... [lean-ctx: omitted 2 lines]
## 4. Fit / anti-fit (revised)
### 4.1 Strong fits (independent or hybrid)
- Closed-set **intent / stakes / complexity** when cloud OK → feed `SelectionIntent` / skip Elite.
- **I/O guardrail** Noul battery (jailbreak, exfil, self-harm, …) with code thresholds ([guardrails cookbook](https://docs.typesafe.ai/cookbooks/llm_guardrails.md)).
... [lean-ctx: omitted 1 lines]
- **Skill shortlist** re-check after lexical `vox skill search` ([skill suggestion cookbook](https://docs.typesafe.ai/cookbooks/skill_suggestion.md)).
... [lean-ctx: omitted 1 lines]
### 4.2 Anti-fits
- Open-ended generation / “discover new themes.”
- Physical / hard-realtime control.
... [lean-ctx: omitted 2 lines]
- Loops that need a **textual rationale** from the discriminator for the next LLM step — pass **structured fields**, not fake prose.
... [lean-ctx: omitted 2 lines]
## 5. Integration boundaries (revised)
| Concern | Home |
... [lean-ctx: omitted 2 lines]
| System One judgments | Separate HTTP client → TypeSafe **or** OpenRouter System One / Decisions API (reuse OpenRouter secret if already resolved) |
| Secrets | `vox_secrets::resolve_secret` — TypeSafe key **or** existing OpenRouter key; never raw `env::var` in consumers |
... [lean-ctx: omitted 1 lines]
| Detector | `llm_provider_call` must not treat System One as a banned chat SDK — document egress allowlist in the adopting PR |
... [lean-ctx: omitted 2 lines]
**Fail policy:** guardrail path → fail **closed** on System One outage (or fall back to local Guard-class model). Cascade prefilter outage → fail **open** to existing `select()` (do not block the harness on Jev).
... [lean-ctx: omitted 1 lines]
## 6. Hypothesis backlog (not a roadmap)
| ID | Experiment | Exit |
... [lean-ctx: omitted 1 lines]
| H0 | OpenRouter System One spike + golden fixtures | Measured p50/p95 from CI region; cost/call |
| H1 | Optional guardrail battery behind flag | FP/FN budget on labeled prompts |
| H2 | Optional intent prefilter when cloud eligible | Token \$ drop without quality regress on chat golden set |
... [lean-ctx: omitted 3 lines]
## 7. Claim ledger
| Claim | Status |
... [lean-ctx: omitted 3 lines]
| No public self-host weights | Primary docs silence + secondary; treat as **hosted-only** until TypeSafe publishes otherwise |
| OpenRouter `typesafe/jev-*` | Primary OpenRouter docs |
| MCA bans distillation / imitation | Secondary digests quoting MCA — **verify PDF before training** |
... [lean-ctx: omitted 1 lines]
| “1k entries &lt;6s / &lt;\$0.10” | Unverified tertiary |
... [lean-ctx: omitted 1 lines]
| Vendor 193×/444× workflow speedups | Vendor self-eval |
... [lean-ctx: omitted 1 lines]
## 8. References
### Primary
- [System One](https://docs.typesafe.ai/concepts/system-one.md), [Models](https://docs.typesafe.ai/models.md), [Confidence](https://docs.typesafe.ai/confidence.md), [How to build](https://docs.typesafe.ai/concepts/how-to-build-with-system-one.md)
... [lean-ctx: omitted 2 lines]
- Cookbooks: [guardrails](https://docs.typesafe.ai/cookbooks/llm_guardrails.md), [function calling](https://docs.typesafe.ai/cookbooks/function_calling.md), [skill suggestion](https://docs.typesafe.ai/cookbooks/skill_suggestion.md), [rerank](https://docs.typesafe.ai/cookbooks/rerank_typesafe.md), [AutoResearch features](https://docs.typesafe.ai/cookbooks/autoresearch_feature_discovery.md)
... [lean-ctx: omitted 3 lines]
- [Wikipedia: Jev](https://en.wikipedia.org/wiki/Jev_(AI_model)) (version lag possible)
### Independent / secondary
- arXiv [2609.37647](https://arxiv.org/abs/2609.37647), [2610.18610](https://arxiv.org/abs/2610.18610)
- Digests (verify against MCA PDF): [Can you fine-tune Jev?](https://calibrateddecisions.com/fine-tune-jev), [Can I self-host Jev?](https://tessary.ai/answers/jev/can-i-self-host-jev)
... [lean-ctx: omitted 1 lines]
## 9. Before / after — `vox research` enrichment (inline)
### 9.1 What we ran
```text
vox research up   # SearXNG was offline; Tavily plan was over quota (432)

# Pass A — broad deep research (hypothesis generation)
vox research run --json --scope web --lane deep --waves 2 --max-sources 12 \
  "TypeSafe Jev System One: can weights be downloaded or self-hosted; is fine-tuning
   allowed; OpenRouter hosting; alternatives for local calibrated decision models;
   latency and privacy tradeoffs vs cloud API for agent harness routing and guardrails"

# Pass B — focused + claim verification
VOX_DATA_DIR=/tmp/vox-research-scratch \
  vox research run --json --scope web --lane fast --waves 1 --max-sources 8 \
  --verify-claims \
  "TypeSafe official docs: is Jev fine-tunable and are weights downloadable"
```
... [lean-ctx: omitted 13 lines]
### 9.2 Pass-1 doc gaps (before)
Snapshot: `/tmp/jev-research-BEFORE.md` (237 lines, sha256 `2da17353…`). Missing or weak relative to pass 2:
... [lean-ctx: omitted 3 lines]
- Explicit “do not put Jev on every `select()`”
... [lean-ctx: omitted 2 lines]
- Honest critique of proposing a new crate / replacing rerankers
### 9.3 What deep research added (after) — distilled
| Lead | Source pass | Disposition |
... [lean-ctx: omitted 1 lines]
| OpenRouter lists `typesafe/jev-1.13` / `1.14` at \$0.042/M in | A | **Confirmed** via OpenRouter docs |
... [lean-ctx: omitted 1 lines]
| Fine-tuning not offered / same weights all accounts | B (improved) | **Confirmed** against TypeSafe Models; Pass A wrongly said “not addressed” |
| Local alternatives: cascades, NeMo/Llama-Guard, community System One-compatible servers | A | **Adopted as path D/F** with “not Jev” warning |
... [lean-ctx: omitted 1 lines]
### 9.4 Excerpts — research executive summaries (raw)
**Pass A (deep, unverified citations):**
> Weights and self-hosting: evidence says Jev's weights are not available… Fine-tuning: the evidence does not address whether fine-tuning is allowed… OpenRouter: Jev is listed… Local alternatives: community local server… NeMo Guardrails or Llama Guard-style classifiers… Privacy: hosted-only means prompts go to a third party, and you need a fallback if it is unavailable.
... [lean-ctx: omitted 1 lines]
> Fine-tuning: The evidence says no. Jev is not offered as a customer-fine-tunable model. TypeSafe is quoted as saying Jev is "not fine-tuned or LoRA-adapted with customer data"… Weights: The evidence says no. TypeSafe has not published Jev's weights… Caveat: TypeSafe-owned pages in the retrieval set were snippet-thin; claims still need primary-doc confirmation.
... [lean-ctx: omitted 1 lines]
### 9.5 Net effect on this document
Pass 2 **replaced** soft roadmap language, **answered** the fine-tune/download/speed questions, **wired** OpenRouter as preferred cloud egress when D8 allows, **added** MCA distillation caution, **added** arXiv evidence, **fixed** research tooling blockers (SearXNG, `VOX_DATA_DIR`, dead `free_floor` slug), and showed that **`--verify-claims` is required** for non-zero citation precision.
## 10. Decision summary
| Question | Answer |
... [lean-ctx: omitted 5 lines]
[… truncated at ~4055 of 7464 tokens — use ctx_read with lines= parameter to see specific sections]
