---
title: "JARS, FST/FTT Token Reduction, and Local Ollama Harness Architecture (2026)"
description: "Cross-repo research and implementation specification for Joint Adaptive Routing & Scoring (JARS), Just-About-Right (JAR) [-2..+2] calibration, Compiled FST + Fuzzy-Trace Theory (FTT) Gist-Capsule token reduction, and zero-spend local Ollama evaluation across Vox, gigme, greaterFool, and ngi.video SSOT."
category: "Architecture SSOTs"
status: "roadmap"
topic: "model-evaluation"
domain: "ml"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
training_rationale: "Synthesizes cross-repo architecture decisions and quantitative evaluation standards for Vox, gigme, and platform-ssot."

schema_type: "TechArticle"
---

# JARS, FST/FTT Token Reduction, and Local Ollama Harness Architecture (2026)

<!-- kb:summary -->
This architecture specification documents the unified Joint Adaptive Routing and Scoring (JARS), Just-About-Right (JAR) penalty calibration, Compiled Finite-State Transducer (FST) intent classification, and Fuzzy-Trace Theory (FTT) Gist-Capsule token-reduction mechanics shared across Vox, gigme, greaterFool, and platform-ssot, alongside zero-spend local Ollama evaluation protocols and deterministic grammar-constrained sampling gates.
<!-- /kb:summary -->

## 1. Purpose & Cross-Repo Context

This specification documents the unified **JARS / JARVS + FST/FTT Classifier Token-Reduction & Local Ollama Architecture** shared across **`Vox`** (`vox-orchestrator`, `vox-eval`, `vox-mcp-registry`, `vox-sampler`), **`gigme`** (Scout v2.0), **`greaterFool`** (FTT Dual-Trace Orchestrator v1.2), and the polyglot single source of truth in **`ngi.video/packages/platform-ssot`**.

It extends two foundational Vox architecture documents:
- [`context-management-mechanics-research-2026-07-31.md`](./context-management-mechanics-research-2026-07-31.md) (prompt-cache `tools -> system -> messages` prefix ordering, `clear_tool_uses_20250919`, `compact_20260112`)
- [`vox-harness-implementation-spec-2026-07-31.md`](./vox-harness-implementation-spec-2026-07-31.md) (the 30–50 tool degradation wall across Vox's 330 MCP tools, `select_tools_for_turn`, and `vox_tool_search`)

---

## 2. The Four-Pillar JARS + FST/FTT Architecture

### 2.1 Pillar 1: Compiled FST + Grammar-Constrained Classifier (`<0.1 ms` to `<40 µs/token`)
Across our chat and agent harnesses, replacing ad-hoc surface regexes with a two-stage deterministic + grammar-constrained classifier eliminates both regex brittleness and cloud router latency:

1. **Tier 0 — Compiled FST & Clause-Scoped Concept-Frame Scorer (`<0.1 ms`, `$0`)**:
   - Uses `tantivy-fst` in Rust (`crates/vox-mcp-registry`, `crates/vox-orchestrator`) and `semanticIntentScorer.ts` in TypeScript (`gigme`, `ngi.video/packages/platform-ssot`) to map morphological stems to weighted semantic frames (`constraint_correction`, `search_recalibration`, `memory_audit`, `deep_synthesis`, `tool_discovery`).
   - Enforces the **Clause-Scoped Negation Parity Invariant** (`N_snip === N_win (mod 2)`): negation operators (`not`, `never`, `without`, `refuse`, `cannot`) invert concept polarity only within their clause boundary (`but`, `however`, `;`, `.`), preventing false-positive intent triggers on negated statements.
2. **Tier 1 — Pushdown / Earley Grammar-Constrained Local Classification (`crates/vox-sampler/src/earley.rs`)**:
   - When Tier 0 confidence falls in the borderline ambiguity band $[0.35, 0.72)$, a local model (`huihui_ai/qwen3-abliterated:8b` via Ollama or `vox-populi` local inference) is invoked with `EarleySampler` / JSON-schema logit masking (`<40 µs/token` overhead), guaranteeing valid structured output in a single pass.

### 2.2 Pillar 2: Fuzzy-Trace Theory (FTT) Gist-Capsule Multi-Turn Token Reduction
Instead of running a heavy secondary neural token pruner (`LLMLingua-2`, which adds `50–180 ms` and risks stripping negation words or identifiers), multi-turn history is compressed using **Fuzzy-Trace Theory (Reyna & Brainerd, 1995)** dual-trace memory (`compressTurnToGistCapsule` in `ngi.video/packages/platform-ssot/src/harness.ts`):

- **Verbatim Trace Window (Last $W = 4$ messages)**: Retained verbatim to preserve immediate local co-reference, code blocks, and exact numeric values.
- **Gist Capsule Ledger (Older turns $> W$)**: Distilled deterministically in `<0.05 ms` into structured single-line capsules (`T{i}[frame=...,tools=...,choice=...]: anchor`), preserving 100% of quantitative anchors, entity IDs, and negation polarity while cutting 8-turn conversational history tokens by **54.1–60.9%**.
- **Synergy with Vox `ContextLifecycleManager`**: Complements `clear_tool_uses_20250919` (which strips stale tool-result payloads) by compressing conversational prose turns before `compact_20260112` is ever needed.

### 2.3 Pillar 3: Just-About-Right (JAR) $[-2..+2]$ Calibration for Tool & Context Sizing
Borrowing the psychometric **Just-About-Right (JAR) $[-2..+2]$ bipolar scale** (Rothman & Parker, 2009), every turn is scored on three sizing axes before dispatch:
- **`toolCountJar` $\in [-2, -1, 0, +1, +2]$**:
  - `-2` / `-1` (`too_little`): Complex multi-step or cross-domain query given too few tools $\rightarrow$ widens dynamic top-$K$ selection or triggers `vox_tool_search` / `discover_tools`.
  - `0` (`just_about_right`): Pinned core prefix (`8` tools, preserving L1 prompt cache) + `4–6` semantically matched domain tools (`12–14` total, well below the 30–50 tool degradation wall).
  - `+1` / `+2` (`too_much`): Simple turn given $> 25$ tools $\rightarrow$ prunes unreferenced domain schemas, saving `45–85%` of tool-schema tokens.
- **`contextBudgetJar` $\in [-2..+2]$**: Calibrates the verbatim window $W$ and RAG chunk count against query complexity.
- **`verbosityJar` $\in [-2..+2]$**: Calibrates target response length so simple factual lookups do not emit 600-token essays.

### 2.4 Pillar 4: APA JARS-Quant/Mixed & Local Ollama Trial Flywheel (`crates/vox-eval/src/trial_flywheel.rs`)
1. **Local Ollama Integration (`http://127.0.0.1:11434`)**:
   - Uses `huihui_ai/qwen3-abliterated:8b` for fast `$0` routing/tool-selection trials (`pass@k` and $\text{pass}^k = (c/n)^k$) and `huihui_ai/Qwen3.8-abliterated:27b` as the local LLM-as-Judge for rubric scoring in `vox-eval`.
2. **JARS-Quant/Mixed Provenance Ledger**:
   - Every benchmark artifact records git SHA, corpus fingerprint, model tier, $\text{pass}^k$ reliability across $k=3$ trials, JAR penalty drop, token-reduction percentage, and a ranked weakness table generated by `surfaceHarnessWeaknesses`.

---

## 3. Concrete Vox Integration Map

| Vox Crate / Module | Current Mechanism | JARS + FST/FTT + Ollama Enhancement |
|---|---|---|
| `crates/vox-mcp-registry` & `vox-capability-registry` | 330 MCP tools (~42k tokens if dumped flat); `select_tools_for_turn` | Enforce prefix-stable `pinned_core` tool ordering (`tools -> system -> messages` L1 cache invariant) + JAR $[-2..+2]$ dynamic top-$K$ sizing + `vox_tool_search` active discovery |
| `crates/vox-orchestrator/src/models/mod.rs` | Heuristic model cascade | Add Tier 0 Compiled FST + Clause Negation Parity scorer (`<0.1 ms`) and Tier 1 Local Ollama (`qwen3-abliterated:8b` via `EarleySampler` JSON schema mask) before cloud escalation |
| `crates/vox-orchestrator/src/context_lifecycle.rs` | `clear_tool_uses_20250919` + `compact_20260112` | Add deterministic FTT Gist-Capsule compression (`compress_turn_to_gist_capsule`) for turns older than $W = 4$ before triggering paid LLM compaction |
| `crates/vox-eval/src/trial_flywheel.rs` | `pass_at_k`, `pass_pow_k`, `EvalTrialTask` | Add `JarScaleEvaluation` ($[-2..+2]$ penalty analysis), local Ollama trial runner (`http://127.0.0.1:11434`), and automated `surface_harness_weaknesses` failure-mode miner |
