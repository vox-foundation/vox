---
title: "Vox/MENS Continuous Language Benchmark — Provenance and Experimental Design (2026-10-10)"
description: "Audit of the current Vox/MENS code-generation eval stack and a design for an automatically triggered, provenance-complete, statistically valid benchmark of MENS versus other models, and of Vox versus other languages, recorded on every syntax change and release but never gating merges."
category: "Architecture SSOTs"
status: "roadmap"
topic: "model-evaluation"
domain: "ml"
authorship: "llm-assisted"
authorship_model: "claude-3-5-sonnet"
training_eligible: false
---

# Vox/MENS Continuous Language Benchmark — Provenance and Experimental Design

<!-- kb:summary -->
Specifies the continuous evaluation and provenance architecture for Vox and MENS code generation benchmarks. Details experimental triggers tied to syntax changes and releases, provenance record metadata, statistical significance testing with McNemar and Holm-Bonferroni corrections, and isolation invariants that prevent held-out fixture contamination while evaluating models across languages without blocking routine CI merges.
<!-- /kb:summary -->

> **Builds on:** [Comparative efficacy research](vox-mens-comparative-efficacy-benchmarking-research-2026-09-01.md),
> [Adversarial audit](vox-efficacy-benchmark-adversarial-audit-2026-09-01.md) (C1–C7, S1–S3),
> [Execution handoff](vox-efficacy-benchmark-execution-handoff-2026-09-04.md).
> Those documents remain authoritative for their findings. This page adds the part they leave
> open: **when** the benchmark runs, **what identity** each result carries, and **how a
> cross-language claim can be made at all**.
>
> **Plan:** phases 12–13 of `.planning/workstreams/research-trial-flywheel/ROADMAP.md`
> (requirements `BENCH-01`…`BENCH-12`).

## 1. Goal

Each time the Vox syntax changes, and each time a release is tagged, record on the shared
results store:

- the success and failure rates of MENS and a fixed panel of other models writing Vox;
- the same models' rates writing Python, TypeScript and Rust on parallel problems;
- a provenance chain that reaches from a result back through the model checkpoint and the
  training data to the exact language version.

Non-goals:

- **This is not a merge gate.** It never runs on pull requests and never blocks one.
- **No results from harnesses driven by hand.** The audit's conflict-of-interest finding
  stands: Claude Code, Cursor and Warp rows stay out of the automated record.

A note on terminology: **WER** (word error rate) is a speech metric. For code the right
primary measure is **functional correctness** (tests pass), reported with a failure breakdown.
§5 defines the metrics.

## 2. Audit of the current state (2026-10-10)

### 2.1 What exists and is sound

| Asset | Location |
|---|---|
| Model eval harness: conditions C0–C3, `--cutoff`, `--max-spend-usd`, `--checkpoint`, cheat canary, no early stop | `crates/vox-cli/src/commands/model/eval_corpus.rs`, `crates/vox-corpus/src/humaneval_runner/` |
| Symbolic verifier (`vox check` + `vox run`); check and run agree on all 164 fixtures (audit) | `humaneval_runner/verify.rs` |
| Unbiased pass@k, cluster bootstrap, Wilson, exact McNemar, Holm, minimum detectable difference | `crates/vox-eval/src/corpus_score.rs`, `corpus_stats.rs` |
| 164-problem corpus with `added_at` dates and a 31-problem held-out split | `contracts/eval/humaneval-vox/` |
| Contamination controls: n-gram leakage precondition, `training_eligible` marker, corpus integrity | `vox-ml-cli/.../eval_gate/leakage`, `vox ci corpus-integrity` |
| Compact grammar prompt (~780 tokens) | `crates/vox-grammar-export/src/compact_prompt.rs` |
| Ed25519 preregistration signing | `crates/vox-orchestrator/src/preregistration/signing.rs` |
| Results table (uncommitted on this branch) | `model_benchmark_results` in `crates/vox-db/src/schema/domains/scientia.rs` |

### 2.2 What is broken or missing

| # | Gap | Evidence | Consequence |
|---|---|---|---|
| G1 | MENS cannot be scored by eval-corpus | `generate_candidate` hardcodes `LlmConfig::openrouter(model_id)`; local MENS routing exists only in `llm/cascade.rs` | The fine-tuned model, the thing the program exists to measure, is unroutable |
| G2 | The stats helpers are not wired in | `bootstrap_ci`, `paired_compare` and `min_detectable_difference` have no callers outside `vox-eval` | Results are point estimates |
| G3 | No seed; `eval_local` pass@k is biased | `eval_local.rs` computes any-sample-passes | Not reproducible; MENS-side numbers are inflated |
| G4 | Underpowered | 31 held-out problems ≈ 9% power for a 10-point gap | No ranking claim is possible |
| G5 | Narrow coverage | 17% of language features, 0 of 56 decorators (audit S1) | Results say little about Vox-specific syntax, so a syntax change is mostly invisible to the benchmark |
| G6 | No cross-language arm | No Python, TS or Rust twins exist | "Vox vs others" cannot be measured |
| G7 | Unbacked claim | `language-benchmark-2026.md` claims "Agentic Success Rate 92%" with no code or data behind it | Credibility risk; must be retracted or labelled |
| G8 | Syntax identity is wrong | `compute_ebnf_hash` hashes **hand-written** EBNF that also embeds `CARGO_PKG_VERSION`; the fingerprint files read by `vox ci grammar-drift` do not exist | A parser change can go undetected; every version bump looks like a syntax change |
| G9 | No triggers | No workflow reacts to parser changes or tags; the MENS lane is dispatch-only and needs a GPU runner that does not exist | Nothing runs automatically |
| G10 | No training lineage | `AdapterCard` has no writer; `TrainingManifest.train_file` is a path, not a hash; `corpus_hash` is used only as an idempotency key | A result cannot be traced to its training data |
| G11 | Incomplete result identity | The eval-corpus report lacks the vox git sha, binary hash, grammar identity and weights hash; `harness_eval_run.config_version` is always `None` | Rows from different language versions are indistinguishable |
| G12 | No durable or signed record | Results live in 30–90-day GitHub artifacts; nothing is signed; the nightly `runs.jsonl` has never been written | No longitudinal record exists |

## 3. Triggers: when it runs

| Trigger | Mechanism | Why |
|---|---|---|
| Release tag `v*` | `push: tags` | Every shipped version gets a datapoint |
| Syntax change on `main` | On every push to `main`, a cheap detector job (a few seconds) computes the behavioral syntax fingerprint (§4) and compares it with the latest fingerprint in the store. If it differs, the detector dispatches the benchmark workflow | Precisely "every time the syntax changes", with no path-filter guesswork |
| MENS checkpoint promoted | `workflow_dispatch` / `repository_dispatch` from the training lane | Every candidate model gets a datapoint |
| Manual | `workflow_dispatch` | Reruns and new panel members |

Rules:

- **Never on `pull_request` or `merge_group`.** The benchmark observes and records; it never
  gates.
- **Separate detector and heavy job.** The detector is push-triggered and tiny. The benchmark
  itself runs as a dispatched workflow under the scheduled-lane time cap, so the 30-minute
  push/PR cap is never hit. Confirm `workflow-policy-guard` classifies a dispatch-only workflow
  under the scheduled cap; if it does not, add a row to `concurrency-exceptions.md` rather than
  raising any cap.
- **Concurrency.** The detector uses `cancel-in-progress: true`. The benchmark uses one
  concurrency group per fingerprint, so superseded syntax versions are cancelled; the newest
  version is the one that matters. This is a known limitation: a fingerprint that lives only
  briefly on `main` may go unrecorded.
- **References are validated first.** Before any model is called, all reference solutions in
  every tier must pass `vox check` and `vox run` under the new syntax. If any fail, the run
  records `reference_migration_required` and stops, failing closed. Otherwise a syntax change
  that breaks the references would show up as model failures.

## 4. Language identity: a behavioral syntax fingerprint

Replace the EBNF-text hash (G8) as the trigger key with a fingerprint of **what the parser
actually does**:

```text
syntax_fingerprint = BLAKE3( canonical_json( for each file in probe corpus, sorted by path:
                         (path, parse_ok, canonical AST dump | sorted diagnostic codes) ) )
```

- **Probe corpus:** `examples/golden/**`, every benchmark reference and test file, every
  negative fixture (§6, tier T2), and a fixed set of deliberately invalid snippets. Diagnostic
  codes (not message text) are included because a change in what is *rejected* is also a
  syntax change.
- **Excluded:** the crate version, spans and line numbers, and message wording. A version bump
  or a pure refactor leaves the hash unchanged.
- **Hashing:** goes through `vox-crypto`, as AGENTS.md requires.
- **Lives in** `vox-grammar-export`, next to the current hash, which stays for its existing
  purpose.
- **Proof:** mutation tests in both directions. A refactor that does not change behavior keeps
  the hash; a one-production grammar change flips it.

Every result row also records the Cargo version, the vox git sha and the binary sha256, so
"language version" is never inferred from a single field.

## 5. Provenance: what every result carries

Each benchmark run is a **campaign** under the Phase 7 identity (`campaign_id`, `run_id`, a
signed immutable preregistration, atomic Tier A persistence). There is no second identity
scheme. Each run record carries:

| Group | Fields |
|---|---|
| Language | `syntax_fingerprint`, `cargo_version`, `vox_git_sha`, `vox_binary_sha256` |
| Benchmark | `tier`, `problem_set_hash` (content hash of every spec, reference and test in the tier), `prompt_template_hash`, `condition` (C0–C3), `context_hash`, `harness_version` |
| Model, MENS | `base_hf_id@base_revision`, `adapter_sha256`, `training_manifest_digest`, `train_data_sha256`, `training_syntax_fingerprint` |
| Model, external | Registry id, provider-reported model/snapshot string, provider, request date |
| Sampling | `temperature`, `top_p`, `n`, `k`, `seed`, `max_tokens` |
| Runner | Runner identity (hosted CI or a named local host), OS/arch, verifier timeouts |
| Integrity | `preregistration_id`, a signature over the canonical record, `run_complete`, infrastructure-error count |

Rules:

- **Missing provenance is a hard fail.** A run missing any required field is recorded with
  `run_complete = false` and is excluded from every comparison. It is never filled in later.
- **Training lineage is written at training time.** The training lane writes `AdapterCard`
  with the sha256 of `train.jsonl` (not its path), the corpus hash and the syntax fingerprint
  of the training corpus. Eval refuses to label a `mens/*` row that has no card.
- **Syntax drift is recorded, not hidden.** `training_syntax_fingerprint ≠ syntax_fingerprint`
  is stored as a flag. The drop in MENS's score after a syntax change is itself a reportable
  quantity: what that change cost a model trained on the old syntax.
- **Signed bundle.** Each run emits a JSON bundle (record, per-sample outcomes, failure
  classes, generated code) signed with `vox-crypto` and stored next to the row, so results can
  be replayed and re-scored offline.

## 6. Benchmark tiers

| Tier | Content | Purpose | Status |
|---|---|---|---|
| **T1 floor** | `humaneval-vox`, 164 problems | Generic algorithmic Vox; continuity with existing work | Exists; held-out split must grow (below) |
| **T2 syntax** | `contracts/eval/vox-syntax/`, keyed to the compiler feature matrix, with **negative fixtures** (an un-annotated version must fail `vox check`) | **The tier that actually moves when syntax changes**: decorators, `actor`/`workflow`, data-layer keywords, `Id[T]`, effects | New (audit S1) |
| **T3 parallel** | One problem set with tests in Vox, Python, TypeScript and Rust | Cross-language comparison | New |
| **Fresh** | A rolling slice of T2 and T3 authored after each panel model's cutoff, gated by `added_at` and `--cutoff` | The only tier eligible for cross-model or cross-language claims | New |

Corpus rules:

- **Held-out size.** MENS is scored on held-out problems only. Grow that set to **≥150**, which
  is about what `min_detectable_difference` requires for a ~10-point paired claim. External
  models may use all problems in a tier, because they were not trained on them; that is the
  point of `--cutoff`.
- **Test strength.** Every reference test suite must kill at least a preregistered fraction of
  mutants of its reference solution, in the style of EvalPlus (mean is 5.9 assertions per
  fixture today, versus HumanEval's 7.7). Weak tests inflate pass rates.
- **Parallel-language sourcing.** The Python, TS and Rust arms of T3 can start from the
  MultiPL-E translations of HumanEval. Check the license before vendoring, and record
  provenance in a `SOURCES.toml`. Those problems are contaminated for every frontier model, so
  they sit in a labelled **contaminated tier**. Cross-language claims come only from fresh
  problems authored in all four languages at once.
- **Non-Vox verification.** Python, TS and Rust candidates run in a sandboxed verifier with
  pinned toolchains, the same timeouts and no network. A toolchain failure is an
  infrastructure error, never a model failure.

## 7. Experimental design: what is being estimated

The audit is right that "MENS beats Claude at Vox" is a tautology, and that "Vox is a better
language for LLMs than Python" cannot be defended directly: the difference in training-data
volume is a confound that cannot be controlled. The design therefore names **estimands**,
each with its own validity statement, instead of producing one leaderboard.

Design: a full factorial of models × languages × conditions on paired problems.

- **Models:** MENS, its base Qwen3-8B (same revision), and two or three frontier models from
  the live panel. `contracts/eval/llm-panel.v1.yaml` resolves them; nothing is pinned by hand.
- **Languages:** Vox, Python, TS, Rust (T3 and fresh tiers).
- **Conditions:** C0, C1, C3 for Vox. Other languages get the matched empty context (C0) and a
  matched documentation context, so the context budget is equal.

| Estimand | Definition | Claim it supports |
|---|---|---|
| E1 Fine-tuning effect | MENS − base, Vox, held-out, same condition | "Fine-tuning improved Vox generation by X [CI]" |
| E2 In-context acquisition | Per model, Vox C1/C3 − Vox C0 | "With the 780-token grammar, model M goes from a to b": Vox's learnability by an unseen model |
| E3 Language gap | Per model, Vox (C3) − Python/TS/Rust (matched context), fresh tier | "On fresh problems, M's success writing Vox is Δ below/above writing Python": **descriptive, confounded by pretraining exposure, labelled as such** |
| E4 Practical parity | MENS-Vox − base-Qwen3-8B-Python, fresh tier | "An 8B model fine-tuned on Vox reaches Δ of what the same base achieves in Python": the user-facing question, stated as a practical comparison, not a causal claim about the language |
| E5 Syntax-change effect | For a fixed model and fixed T2 problems, score at fingerprint *t* − score at *t−1*, frontier models under C1/C3 | "This syntax change made Vox easier/harder for an unseen model to write": **the language-design signal** |
| E6 Drift cost | MENS score when `training_syntax_fingerprint ≠ syntax_fingerprint`, minus after retraining | What a syntax change costs a model trained on the old syntax |

E5 is what the trigger exists for. It is valid only if the panel model itself did not change,
which the anchor control below checks.

## 8. Metrics

Primary:

- **pass@1** from the unbiased estimator with n ≥ 5 samples at a preregistered temperature,
  plus one greedy (T=0) run reported separately. All n samples are always drawn; there is no
  early stop (audit C3).

Secondary, always reported:

- **Compile rate**: the share of samples that pass `vox check` (or the language's
  type-check/compile step).
- **Failure breakdown**: parse error, type/check error, runtime error, wrong answer, timeout,
  cheated (canary), infrastructure error. This is the useful replacement for "WER": it says
  *which* syntax hurts.
- **pass@1 after k compiler-feedback turns** (k = 1 and 3). This measures whether Vox's
  diagnostics help a model repair its code, which is a central claim of the language.
- **Tokens per passing solution** and **cost per success at matched quality**. Because all n
  samples are drawn, these are not conditioned on the outcome.
- **Calibration** (optional): the model states a confidence and Brier score is reported, per
  the HLE lesson.

## 9. Statistics

- **Unit of analysis is the problem.** Samples within a problem are not independent.
- **Pairing.** Every arm sees the same problems.
  - Pairwise single-sample pass@1 comparisons use exact McNemar.
  - Pass@k means and differences use a **paired cluster bootstrap over problems**: resample
    problems, compute the arm difference.
  - E3 and E4 use a mixed-effects logistic model, `pass ~ model × language × condition +
    (1 | problem)`, as the confirmatory analysis, with the bootstrap as a robustness check.
- **Multiplicity.** Holm across the claims preregistered for a campaign. Across the sequence of
  runs triggered by syntax changes, E5 uses a preregistered alpha-spending rule, so the series
  of runs cannot manufacture a finding by repetition.
- **No CI-overlap tests** (audit C4).
- **Resolution floor.** Every report prints `min_detectable_difference(n)` next to each
  comparison, and any difference below it is labelled *unresolved*, not "no difference".
- **Preregistration.** Hypotheses, estimands, tiers, panel resolution rule, sampling settings
  and the analysis plan are signed before a campaign starts. Syntax-change runs reuse a
  standing preregistration, re-signed only when the analysis plan changes.

## 10. Flaws and their corrections

| Threat | Control |
|---|---|
| Memorized benchmark (contamination) | `--cutoff`/`added_at`; fresh tier; contaminated tier reported separately; leakage precondition for MENS |
| MENS trained on test problems | Held-out-only scoring; n-gram leakage hard gate; `training_eligible: false` on held-out material |
| Silent API model updates | Record the provider snapshot string; a fixed **anchor subset** of 20 T1 problems rerun every campaign. If the anchor score moves beyond its CI while the fingerprint is unchanged, the model drifted and E5 is not computed for that pair |
| Weak tests inflating pass rates | Mutant-kill threshold per fixture |
| Candidate disables the tests | Existing canary and differential control (audit C1); cheated counts as a failure |
| Optional stopping | Fixed n, no early break |
| Infrastructure noise | Infrastructure errors excluded and counted; a run is incomplete above a preregistered rate |
| References broken by a syntax change | Validate references first, fail closed (§3) |
| Author conflict of interest | COI statement on every report; held-out authoring by someone other than the MENS trainer where possible; automated lane only |
| Narrow coverage | T2 tier keyed to the feature matrix; coverage percentage printed in each report |
| Unequal context across languages | Matched context budgets per condition |
| Pretraining-volume confound (Vox vs others) | Cannot be removed. E3 is labelled descriptive; causal language claims come only from E5 (within-language, across versions) |

## 11. Storage and publication

- **Store.** Phase 7 Tier A campaign, run and sample tables in the shared vox-db
  (`VOX_DB_URL`), extended with the §5 fields. `model_benchmark_results` on this branch becomes
  a view or projection over them, not a parallel writer. Signed bundles are Tier C evidence
  (Phase 9 STORE-03), with the GitHub artifact as a transport only.
- **Report.** Per campaign: a JSON report validated against a versioned schema, and a CLI
  summary. Conditions appear as separate columns, never averaged, with CIs, resolution floor
  and COI statement. A public leaderboard (voxlang.org, Hugging Face) is out of scope until E1
  and E2 have been reproduced across two campaigns.
- **Retraction.** `language-benchmark-2026.md`'s 92% claim gets `status: research` and a
  banner saying it is unbacked, until this pipeline produces a number.

## 12. Open decisions

1. **Results server.** The default here is the shared vox-db (Turso, `VOX_DB_URL`). Confirm, or
   name another store.
2. **Where MENS inference runs.** Hosted CI has no GPU. Options:
   - **(a)** The MENS arm runs on a named local host (Mac via MLX or Ollama, already in the
     cascade) and submits a signed bundle. Cheapest, available now.
   - **(b)** A rented GPU endpoint serving the LoRA adapter through vLLM
     (`serving/vllm_lora.rs`), registered as a model-registry entry so the hosted workflow can
     call it.
   - The recommendation is (a) first and (b) when cadence demands it. Either way, the routing
     goes through the `llm` facade (G1).
3. **MultiPL-E license and vendoring** for the contaminated T3 seed.
4. **Authoring budget** for ≥150 held-out problems and the T2 and fresh tiers. This is the
   largest cost in the program and the actual prerequisite for any ranking claim.
