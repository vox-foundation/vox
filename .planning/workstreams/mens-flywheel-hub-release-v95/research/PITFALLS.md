# Pitfalls Research

**Domain:** Adding eval timing + prefix KV caching (hybrid attention / GatedDeltaNet), failure-funnel metrics, a `lane` -> `backend` / `spoke_id` / `data_slice` rename, honest before/after proof, gated teacher distillation, versioned Hugging Face releases and a static in-browser-inference Space to an EXISTING Vox MENS QLoRA + eval + hub pipeline.
**Milestone:** v9.5 "MENS Flywheel, Truthful Lanes & Trustworthy Hub Releases"
**Researched:** 2026-11-20
**Confidence:** MEDIUM-HIGH. Repo-specific facts are HIGH (read from source/handoff). Cross-system practice is MEDIUM (web, mostly blog/RFC level). Items marked `[VERIFY]` are inferences the owning phase must confirm with a test, not assume.

## How to read this

Logical phases used in the mapping (the roadmapper may split/merge, but keep the ORDER constraints):

| Tag | Logical phase | Hard ordering constraint |
|-----|---------------|--------------------------|
| **P0** | Ops / multi-agent hygiene (cross-cutting, before anything else) | First |
| **P1** | Eval instrumentation + failure funnel (timing schema, honesty fields) | Before P2, P4 |
| **P2** | Prefix KV + recurrent-state cache | After P1 (needs timing to prove the win), before P4 "after" runs |
| **P3** | `lane` rename (3-axis schema) | Independent of P1/P2 but must land BEFORE P6 (hub cards read `spoke_id`) |
| **P4** | Before/after proof on 166-task bench + 64 canary | Baseline frozen BEFORE any distillation (P5) |
| **P5** | Gated teacher-distillation data plan | After P4 funnel; needs sealed blind slice |
| **P6** | Versioned gated Hub releases | After P3 + P4 evidence |
| **P7** | Static Space with live in-browser inference | After P6 (needs a public, pinned revision) |

Repo facts these pitfalls lean on (all read from source this session):
- `crates/vox-plugin-mens-candle-metal/src/inference.rs:917-988` `generate()` calls `model.empty_cache()` per item, builds the prompt as flat `"{system}\n\n{prompt}"` text, tokenizes with `encode(prompt, true)`, and returns only `generated_text` + `prompt_tokens` (lines 909-913). There is NO chat template.
- `eval_local.rs:221` sorts prompts lexicographically and prints "prefix-cache friendly" while the plugin resets the cache every call (handoff section 5).
- `item_checkpoint.rs` keys cached items on `max_tokens`, `items_blake3`, model snapshot and adapter weights/manifest BLAKE3 (`reasoning_inputs`, lines 57-76, 202-203). It does NOT visibly key on verifier/interpreter build.
- `crates/vox-ml-cli/src/commands/mens/hub.rs` already has `--push`, `--allow-ungated` ("marks card as `ungated`"), version tags, cards-only push (README.md + MODEL_CARD.md), `pull_adapter_revision_with_spoke`.
- Working tree is already dirty with ~50 modified files of mixed provenance (Phase 547-550 work plus unrelated doc-pipeline edits, incl. `vox-compiler/src/eval/{mod,expr}.rs`, the interpreter that is the benchmark VERIFIER). A long-running `PID 34157` `eval-reasoning-compare` on Qwen3.8-27B owns the GPU.

---

## Critical Pitfalls

### K1: "Same prefix" is not token-identical (flat-text prompt, tokenizer boundary)

**What goes wrong:** The cache is keyed or cut by string length / by independently tokenizing the system prompt. BPE pre-tokenization means `encode(system)` + `encode(rest)` can differ from `encode(system + "\n\n" + prompt)[..n]` at the join (a trailing token merges with, or re-splits against, the first characters of the item prompt). The restored state then corresponds to a different token sequence than the one being "continued"; positions after the join are silently shifted. Output differs only on some items.
**Why it happens:** This codebase builds `"{system}\n\n{prompt}"` with no special-token role boundaries, and `encode(prompt, true)` also adds whatever special tokens the tokenizer config adds. Special tokens are atomic and make clean boundaries; plain whitespace/text joins do not.
**How to avoid:** Key the cache on TOKEN IDS, never strings. Tokenize the FULL prompt once, compute the longest common prefix (LCP) with the snapshot's id vector, and reuse only `ids[..lcp]`, minus a small safety margin; never tokenize the prefix separately. Assert at runtime `full_ids[..snap_len] == snap_ids`, and on mismatch fall back to a cold prefill and increment a `kv_prefix_cache_miss{reason}` counter. Consider ending the shared system prompt with a hard boundary (a real special token or a delimiter that cannot merge) only if that does not change training-time prompt shape; changing prompt shape is a model change, not a cache change.
**Warning signs:** Cache-on vs cache-off greedy outputs differ for a few percent of items; `kv_prefix_cache_hit_tokens` is identical for every item (suggesting a fixed byte-length cut); hit count equals string-prefix length / 4 rather than an exact id count.
**Phase to address:** P2. Test first: property test over items whose first characters are whitespace, digits, quotes, non-ASCII, and a leading space variant.

### K2: Recurrent (GatedDeltaNet) and conv state not snapshotted exactly at the prefix boundary, or aliased and mutated

**What goes wrong:** Attention KV can be truncated to any prefix length; recurrent state cannot. State updated in place after the full prompt has consumed the suffix cannot be rolled back to represent only the prefix. Three distinct states exist per hybrid layer family: full-attention KV, short-conv state (last kernel-1 tokens' inputs), and the delta-rule recurrent matrix. Snapshotting only KV, or snapshotting at end-of-prompt, or restoring by reference so the next item's decode mutates the snapshot, produces "plausible but slightly wrong" completions (no crash).
**Why it happens:** The existing code comment says it carries "KV (full attention) and short-conv + recurrent state (linear attention)", so the cache struct (`Qwen35LayerCache`) already mixes all three; a naive `clone()` of a struct holding `Arc`-shared candle tensors is a shallow copy. Serving systems solved this explicitly: vLLM keeps a full recurrent snapshot once per block boundary and, for Qwen3.5 hybrids, forces a 528-token attention block size to align with the Mamba page, and a 2026 RFC proposes application-declared checkpoints because "1 or 2" snapshots per sequence is all that is affordable; SGLang uses a hybrid radix tree with tombstones. Alignment and boundary placement are real design constraints, not details.
**How to avoid:** (1) Prefill in TWO explicit segments: forward exactly the prefix ids, snapshot (deep copy: new buffers for KV, conv, recurrent state), then forward the suffix. (2) Never restore by reference: restore = deep copy of snapshot into a fresh cache per item. (3) Add a test that hashes every layer's state before and after `generate()` on the snapshot object and asserts it is unchanged. (4) Test a prefix length that is NOT a multiple of the delta-rule chunk size (e.g. 64) and one that is. (5) Keep one snapshot per (model, adapter) and drop it on model/adapter unload.
**Warning signs:** Outputs of item B differ depending on whether item A ran immediately before it (order dependence); first item after a cold start passes while later ones degrade; results change when prompts are shuffled.
**Phase to address:** P2. The `generate_handles_hybrid_linear_attention_beyond_one_token` test (inference.rs:~2302) and the existing candle-core `sameness_gate` are the pattern to extend.

### K3: Warm and cold paths use different numerics, so "equivalent" is not bitwise and greedy argmax flips on near-ties

**What goes wrong:** Cold run prefill is one long pass; warm run is prefix pass + suffix pass with initial state. Different matmul shapes, chunking and (for GatedDeltaNet) chunked-vs-recurrent algorithm choice change reduction order. Logits differ by float noise; at temperature 0 a near-tie flips and the whole completion diverges, which then shows up as a spurious pass/fail change in the proof.
**Why it happens:** Floating-point non-associativity plus shape-dependent kernels. Thinking Machines' analysis attributes most real-world run-to-run variation to missing batch invariance (results depend on how work is batched/shaped), not to random races; making kernels invariant costs speed. We are not batching, but the warm path changes the shapes the kernels see, which is the same mechanism.
**How to avoid:** Define "equivalent" before writing code, in terms the proof can enforce:
1. Measure the numerical noise floor first: cold prefill vs cold prefill with a different chunk size/sequence split. The cache must stay within that floor, not within an arbitrary 1e-6.
2. Token-ID-identical greedy decode (first 64 tokens minimum, then full completion) for ALL 166 + 64 items, cache-on vs cache-off. Zero divergences is the target; any divergence must be explained as a measured top-1/top-2 margin below the noise floor.
3. Last-prompt-position logits: max abs diff, top-1 equality, and KL(cold || warm) per item, reported in a committed JSON.
4. Run in the model's real eval dtype and on real Metal (CPU equivalence proves nothing about Metal kernels). CUDA crate gets `cargo check` only unless hardware is available; say so explicitly.
**Warning signs:** Divergence concentrated on items with long completions; divergence rate changes when prefix length changes; cache-on pass rate differs from cache-off by 1-2 items on a 64-task canary (looks like "improvement" or "regression" but is noise).
**Phase to address:** P2 (equivalence suite), consumed by P4 (the "after" run must reference the equivalence report).

### K4: Cache keyed too loosely across models, adapters, merged-LoRA, tokenizers, dtype

**What goes wrong:** `reasoning_compare` runs base then adapter (and `eval_local` may merge LoRA deltas at load). A snapshot built under the base weights is reused under the adapter (or a different adapter, rank, or dtype), producing completions that reflect base-model state with adapter-model decode. Pass rates for the two arms silently converge, hiding the very effect being measured.
**Why it happens:** Per-item checkpoints are already well keyed (adapter weights+manifest BLAKE3, model snapshot, `max_tokens`, `items_blake3`), which gives a false sense that "the cache" is keyed. The new in-process KV cache is a separate cache with its own key.
**How to avoid:** Key = (model snapshot hash, adapter weights BLAKE3, adapter manifest BLAKE3, lora-merge flag, tokenizer hash, dtype, device, `empty` prefix token ids hash). Invalidate on any `load_adapter` / model swap, structurally (cache lives inside the loaded-model object and dies with it). Test: base -> adapter -> base ordering gives base-identical outputs on the second base pass.
**Warning signs:** Adapter arm == base arm on every item; `kv_prefix_cache_hit_tokens > 0` on the first item after a model swap.
**Phase to address:** P2.

### K5: Cache toggle changes outputs but not the item-checkpoint key, so cached and fresh items mix

**What goes wrong:** Existing `.ckpt.jsonl` items (cold, pre-v9.5) are reused by runs that use the new cache. If the cache is not exactly equivalent, a "run" is now part cold-numerics, part warm-numerics, and any timing/pass comparison is contaminated. Handoff correctly says "do NOT touch `item_checkpoint.rs` cache-key inputs" so old checkpoints stay valid, which is right ONLY if K3 proves equivalence.
**How to avoid:** Keep the key unchanged iff the equivalence report passes; record per-item provenance as additive fields (`from_checkpoint: bool`, `prefix_cache: on|off`, `kv_prefix_cache_hit_tokens`, `gen_wall_ms`). All timing aggregates (tok/s, latency p50/p95, eval_wall) computed over generated-this-run items only; report `cached_items` / `generated_items` in every header (handoff section 5 already lists these; make them mandatory fields, not optional). If equivalence cannot be shown, put `prefix_cache` into the key and accept re-generation.
**Warning signs:** `eval_wall_secs` of a resumed run is wildly lower than a fresh one; mean tok/s is NaN or infinite (zero generated items); two runs with same inputs disagree on pass items only for resumed items.
**Phase to address:** P1 (fields), P2 (key decision), P4 (comparator refuses to compare runs whose `generated_items` mixes cache modes).

### K6: Missing new fields read as zero, and plugin/CLI version skew

**What goes wrong:** New timing fields (`completion_tokens`, `prefill_ms`, `decode_ms`, `stop_reason`, ...) are added to the plugin JSON. The CLI and the plugin dylib are built and installed SEPARATELY (the handoff uses isolated `target/v94_canary_plugins` and `~/Library/Application Support/vox/plugins/`). An old plugin + new CLI yields absent fields; deserialized with `#[serde(default)]` into `0`, the report then shows `decode_tok_s = 0` and `truncation_rate = 0%` ("no truncation!") when the truth is "unknown".
**How to avoid:** Use `Option<T>` for every new field; aggregate only over `Some`; report `n_with_timing / n_total`; fail the eval header with an explicit `timing: unavailable (plugin vX)` instead of zeros. Put a plugin ABI/schema version in the response and refuse to claim metrics when it is lower than the CLI expects. Never default `stop_reason` to `eos`.
**Warning signs:** `truncation_rate` exactly 0.0 on a model known to ramble; `mean_decode_tok_s` equals 0.
**Phase to address:** P1.

### K7: Memory growth / unified-memory contention from snapshots

**What goes wrong:** Per-item snapshot clones are never dropped (Metal buffers kept alive by Arc), or a snapshot per (model, adapter) accumulates across `reasoning_compare` arms. On Apple Silicon unified memory a 27B eval already sits near the predictor's training figures (43-78 GB range in `memory-model.v1.yaml` for training; eval budget is unmodeled), so even tens of MB of leak per item can matter, and a second process makes it worse.
**How to avoid:** Exactly ONE snapshot per loaded model, explicit `drop` on model unload; deep-copy on restore and free at item end. Assert peak RSS / `peak_metal_allocated_bytes` is flat (within 1%) from item 10 to item 166 in a regression test on the 0.8B canary model. Record snapshot bytes in the report. Do not extrapolate memory numbers from the 0.8B to 27B; `[VERIFY]` on 27B only after PID 34157 finishes.
**Warning signs:** RSS ramps linearly with items processed; `peak_metal_allocated_bytes` higher with cache on than off by more than the snapshot size.
**Phase to address:** P2.

### K8: Claiming a speedup that includes prefix work that was never really skipped (or is dominated by something else)

**What goes wrong:** The headline "skips ~180K prefill tokens" is true arithmetic (166 x ~1.1k) but wall-clock gain depends on prefill:decode ratio; with long completions (`max_tokens` 512-1024) decode dominates, so measured eval speedup may be small. Reporting the token-count saving as time saving, or measuring the warm run against a cold run executed under a different machine load (PID 34157 on the same GPU), misleads the flywheel metrics (`cycle_wall_secs`, `pass_gain_per_gpu_hour`).
**How to avoid:** Report prefill_ms and decode_ms separately, cold vs warm, same machine state, interleaved A/B (ABAB) over >=3 repetitions, with `load_avg`/concurrent GPU job count recorded in the header. State the speedup as measured eval wall time, never as tokens skipped.
**Warning signs:** `prefill_ms` drops but `eval_wall` barely moves; large run-to-run variance.
**Phase to address:** P1 (fields) + P2 (measurement protocol).

---

### B1: Synthetic training data overfits the held-out set by construction (leakage through targeting)

**What goes wrong:** The handoff's own plan targets teacher spend at categories with zero pass on `heldout_bench_v2` (`endpoint_handler 0/4`, `state_machine 0/4`, `matrix_grid 0/6`, `data_structures 0/7`, `string_parsing 0/6`). Generating data "for the failing categories" of a benchmark, then re-scoring on that benchmark, makes the benchmark a training signal even with zero verbatim overlap: the generator prompt carries the category (and possibly task text), producing paraphrases/rewrites of the test items.
**Why it happens:** String/n-gram decontamination (`assert_no_text_leakage`) catches verbatim and near-verbatim only. Research on rephrased benchmarks showed paraphrased/translated test items evade n-gram overlap entirely, and a 13B model trained on rephrased MMLU reached 85.9 while passing standard decontamination; they also found synthetic GPT-generated sets carrying benchmark contamination and 8-18% HumanEval overlap in common pretraining corpora.
**How to avoid:**
1. Teacher prompts take only family-level names and independently authored spec seeds, never an eval prompt, test, or reference solution (code-level firewall: the distillation module must not import the eval task loader).
2. Layered decontamination: normalized n-gram (8-gram sliding) + embedding max-similarity against ALL eval items (v1, v2, canary, v3) with a threshold and an LLM-judge pass on the top-k neighbours; log the nearest eval neighbour and score per accepted row.
3. Seal a rotating blind slice (the handoff's "v3", 40-50 tasks) BEFORE data generation: commit only its hash; never use it for category selection or hyperparameter choice; evaluate it exactly once per release candidate. The headline claim in the card is the blind slice number, with v2 as secondary.
4. Report v2 improvement and blind-slice improvement side by side; a v2 gain with no blind-slice gain is overfitting, not progress.
**Warning signs:** Gains concentrated in exactly the categories targeted; blind-slice gain much smaller than v2 gain; accepted rows whose nearest eval neighbour similarity is high but below the n-gram threshold.
**Phase to address:** P5 (firewall + decontam), P4 (seal blind slice and baseline before P5 starts).

### B2: Canary is a subset of the benchmark, so it is not independent evidence

**What goes wrong:** `heldout_bench_v2_canary` (64) is a slice of `heldout_bench_v2` (166). A model "improving on both" has one piece of evidence counted twice; early "wins" on the canary are then re-reported as confirmed on the full set when 64/166 of the full-set items are the same items.
**How to avoid:** Report canary and full-set results as nested, never as corroborating. For confirmation use the disjoint remainder (166 minus canary = 102 items) and the blind slice. State in every proof report which items are shared.
**Warning signs:** Proof summary says "confirmed on 166" with the canary delta carrying most of it.
**Phase to address:** P4.

### B3: Changing the benchmark/verifier and the model in the same comparison

**What goes wrong:** The v9.4 "before" numbers (e.g. 8.0/64 for the r64 canary run, 9.0/64 legacy) were produced with a specific interpreter build; the dirty tree currently modifies `vox-compiler/src/eval/{mod,expr}.rs` (the `max_string_bytes` guard, step caps) and `vox-ml-cli` metrics. If "after" is scored with a different verifier than "before", a pass-rate change can come from the verifier (string/step limits, anti-stub rules, assertion semantics), not the model. Same for tasks edited or added to v2 mid-milestone.
**How to avoid:** Freeze a "proof manifest" before any new training: git SHA + dirty flag, `heldout_bench_v2` manifest hash, verifier/interpreter source hash, `max_tokens`, `max_string_bytes`, step cap, seed, prompt template hash, tokenizer hash. Every report embeds it. The comparator refuses (or loudly flags) any pair of reports whose frozen fields differ except the one declared "independent variable". Re-score the BASELINE completions with the final verifier (cheap, CPU) rather than trusting old verdicts: generation is expensive and cacheable, verification is cheap and should always be redone. `[VERIFY]` whether item checkpoints store verdicts or only generations; if verdicts, re-verify on read.
**Warning signs:** The baseline report's timestamp predates a commit touching `vox-compiler/src/eval/` or the bench manifests; two reports with different `items_blake3` compared.
**Phase to address:** P4 (freeze + comparator), P0 (commit the Phase 547-550 working tree in clean, reviewable commits first so the "before" has a SHA).

### B4: max_tokens truncation and step/string-limit hits read as model failure

**What goes wrong:** `matrix_row_sums` died on a truncated `if` at 512 tokens; `to_hex` exposed an exponential string-doubling hang, now capped at 1 MiB, which turns some executions into limit failures. A 512- vs 1024-token comparison, or a model that rambles, shows as "worse model". Field practice varies: some evaluations score truncation as wrong, others exclude it from the denominator; that single choice moves pass rates, and an example paper found a ~12-13% truncation rate for one model and 0% for others under identical settings.
**How to avoid:** Funnel buckets must be mutually exclusive, ordered and sum to n (assert `sum == n` in code): `no_code | truncated(max_tokens) | parse_fail | typecheck_fail | anti_stub_fail | limit_hit(step|string) | assertion_fail | pass`. Always report `truncation_rate` beside pass@1, and a sensitivity row: pass@1 on non-truncated items and pass@1 with `max_tokens` doubled on truncated items only. Compare arms only at identical `max_tokens` (already in the checkpoint key; make the comparator enforce it). Record `stop_reason` per item; unrecognised stop reasons are a warning, never defaulted.
**Warning signs:** `truncation_rate` differs between arms by more than a few points; a "regression" localised to long-solution categories.
**Phase to address:** P1 (funnel + stop_reason), P4 (sensitivity table).

### B5: Step-capped / partial runs compared to full runs

**What goes wrong:** The 0.8B r64 canary run is explicitly "partial / step-capped" (205 optimizer steps, 25.7% of epoch 1, 8.0/64) and is compared in prose with the full 437-step legacy r16 run (9.0/64). Different compute, different data exposure; a one-item difference on n=64 is read as "approaching".
**How to avoid:** Add `completeness: full | step_capped`, `optimizer_steps`, `epoch_fraction`, `tokens_seen` to `training_manifest.json` / `train-record.json` and to the proof header. The comparator only declares a winner between runs of equal `completeness` and equal (steps or tokens) budget, otherwise prints "NOT COMPARABLE: <reason>". Use the handoff's `pass_gain_per_gpu_hour` only with both numerator and denominator from equal-completeness runs.
**Warning signs:** A table that lists a step-capped row next to a full row with no flag column.
**Phase to address:** P3 (manifest fields land with the rename) and P4 (comparator).

### B6: Small-n noise read as gain (n=64, per-category n=4..7)

**What goes wrong:** At n=64 and p~0.125 the binomial 95% interval is about +/-8 percentage points (my calculation: SE = sqrt(.125*.875/64) = 4.1pp); 3.0/64 -> 8.0/64 is five items. Per-category 0/4 is compatible with a true rate near 40% (probability of 0/4 at p=0.2 is ~41%), so "zero-pass category" is mostly noise at those sizes. CLT-based error bars (the usual "add error bars to evals" recipe) are reported to under-cover on benchmarks with only a few hundred items; exact or Bayesian intervals are recommended. Paired analysis (same items, only discordant pairs, exact McNemar) tightens the comparison substantially but does not rescue n=4.
**How to avoid:** Report counts and exact (Clopper-Pearson or Wilson) intervals for single-arm rates; for A vs B report b/c discordant counts and an exact McNemar p on the SAME items; never claim a per-category win below ~15 paired discordant items; show seeds: at least 2 training seeds on the canary arm or state "single seed, treat as indicative". Pre-register the primary metric (blind-slice or full-166 pass@1, paired) and the minimum effect before running. Sequential screening (`--sequential`, alpha-spending in `eval-gates-ladder.yaml`) is for early reject only; a release claim uses the fixed-n gate on fresh data. Re-using the same looks' data for a fixed-n p-value is optional stopping.
**Warning signs:** Card or docs say "improved" with no interval; spend decisions keyed on single-digit category counts.
**Phase to address:** P4 (stats module + report schema), P5 (spend gates use pooled evidence across models/seeds, not a 0/4).

### B7: Gate thresholds calibrated on the old benchmark

**What goes wrong:** `eval-gates-ladder.yaml` `min_pass_rate_at_1: 0.25` was set for compile-only v1 (0.8B scored 73.1%); on execution-based v2 canary it scores 14.1%. Either every candidate fails (gate bypassed with `--allow-ungated`) or the threshold is lowered ad hoc to make a candidate pass.
**How to avoid:** Re-derive v2 floors per size from measured baselines with intervals, commit them in one reviewed change BEFORE any candidate is evaluated against them, never edit thresholds in the same PR as the candidate result. Gate on non-regression vs the currently published release (paired), not only an absolute floor.
**Warning signs:** A gate-config diff in the same PR as a release result.
**Phase to address:** P4 (calibrate), P6 (enforce).

---

### R1: `serde(alias)` is a one-way read bridge (new reads old, old cannot read new)

**What goes wrong:** `#[serde(alias = "lane")]` only affects deserialization. A manifest written by new code uses `backend`/`spoke_id`/`data_slice`; an older installed `vox` / plugin / external tool / the `vox-ops` sibling repo / a hub-pull on another machine does not know those keys and sees nothing (or errors under `deny_unknown_fields`). Meanwhile the aliasing is invisible in error messages (they list canonical names only), so failures look like "missing field".
**How to avoid:** Decide direction per surface: for persisted run manifests that older readers may still open, WRITE canonical new keys AND (for one release) the legacy `lane` key via `rename(serialize=..., deserialize=...)` or an explicit dual-write; state the sunset release in a `// vox-deprecated-since=... retire-by=...` marker (required by AGENTS.md). Provide a `schema_version` field in every manifest so readers branch on version, not on key presence. Add a round-trip test matrix: {old fixture, new fixture} x {new reader}, plus {new writer} x {frozen old-reader struct copied into the test}.
**Warning signs:** `missing field` on a manifest written seconds ago by the new binary; hub cards for old runs showing the wrong spoke.
**Phase to address:** P3.

### R2: Key migration is easy, VALUE migration is lossy and non-invertible

**What goes wrong:** Alias handles keys, not values. `lane: vox_codegen` must become a `data_slice` of `vox.exec_verified` OR `vox.compile_verified`, and `vox_lang_tier_b` splits into `vox.repair_trace` / `vox.exec_verified`. That is one-to-many; old manifests do not contain the information needed to choose. A silent default (e.g. mapping everything to `vox.exec_verified`) stamps a verification tier that was never true, which then feeds the hub card ("trained on execution-verified data") and the failure-funnel slices.
**How to avoid:** Add explicit `legacy.<old_value>` slices and `verification_tier: unknown` for anything migrated from old manifests; only NEW runs may carry `exec_verified`. Value mapping lives in one table in code with a test for every old value in `mens/config/mix*.yaml`, `crates/vox-corpus/src/corpus/mix/mod.rs:459-478`, and historical `training_manifest.json` fixtures. Hub card generation must treat `unknown` as a blocker for any "verified" wording.
**Warning signs:** `data_slice_histogram` of an old run showing `vox.exec_verified` for a run that predates the verifier.
**Phase to address:** P3 (mapping), P6 (card generation consumes it).

### R3: Old run manifests become unreadable or silently mis-attributed

**What goes wrong:** Old `training_manifest.json` / `train-record.json` lack `spoke_id`. Today `hub-cards.yaml:13` hardcodes `default_spoke: vox-lang`. After the change, `facts.rs` reads `spoke_id` directly; if the fallback to the hardcoded default is kept "for compatibility", every legacy run is labelled `vox-lang` (possibly wrongly) and now flows into signed/versioned release cards.
**How to avoid:** Missing `spoke_id` -> `spoke_id: unknown` + a warning counter; the Hub publisher refuses to release an `unknown`-spoke run. Remove `default_spoke` from `hub-cards.yaml` in the same change (don't keep two sources). Keep a fixtures dir of REAL old manifests (copy one from `mens/runs/` for each shape: v9.3, v9.4 canary, 27B legacy) and test they still parse.
**Warning signs:** A card whose `spoke_id` equals the config default for a run that predates spokes.
**Phase to address:** P3.

### R4: Docs / config / contracts drift and dual sources of truth

**What goes wrong:** Verified drift already exists: `mens-training-data-contract.md:40,67` claims `include_lanes: [vox_codegen]` while `mix.yaml:83-88` includes four lanes; `domain-profiles.yaml` bases are stale vs `hub-cards.yaml`; `mix-agents.yaml` duplicates `mix-harness.yaml`. A rename that edits Rust + YAML but leaves docs, generated reference pages, CLI flags (`--lane`?), env keys and telemetry event names creates a third vocabulary. `contracts/mens/memory-model.v1.yaml` is under `contracts/`, which AGENTS.md says may require `vox ci cross-repo-parity --sync` with sibling repos (BLAKE3 parity) `[VERIFY whether this file is in the shared set]`. Renamed CLI flags change `docs/src/reference/*.generated.md` and `command_catalog_paths_baseline.txt`.
**How to avoid:** One grep-driven inventory of every occurrence of `lane`/`lanes`/`include_lanes`/`Lane G` (code, yaml, md, json reports, telemetry) as the phase's first task, each with a disposition (rename / keep as orchestrator-lane / delete). KEEP "lane" where it genuinely means the orchestrator `Sync/Background/Plan` execution lane (concept 4 in the handoff); renaming that would be a false unification. Add a CI-checkable invariant: a test that walks `mens/config/*.yaml` and fails if any key named `lane` remains outside the alias-allowlist. Regenerate docs via the repo's generators (`vox ci command-sync`), never by hand; run `vox ci cross-repo-parity` if a contract changed. Delete `mix-agents.yaml` (or archive) in the same PR as the canonical-mix decision, not later.
**Warning signs:** `rg -w lane` hits outside the allowlist; two mixes that both claim to be canonical for `vox-lang`; docs describing a field the schema no longer has.
**Phase to address:** P3.

### R5: The canonical mix is not actually wired, and optional sources fail silently

**What goes wrong:** Phase 550 trained with `--fast-corpus --data-dir target/v94_canary_data` to bypass `mix.yaml`; `mix.yaml`/`mix-vox-lang.yaml` do not point at `verified_vox_codegen_v2.jsonl`, and every source in `mix-vox-lang.yaml` is `optional: true`. Rename work that "unifies the mix" while keeping `optional: true` produces a run that trains on nothing (or on a tiny remainder) and reports success; with the rename, the data_slice histogram would be empty/all-legacy and nobody notices.
**How to avoid:** Verified corpus inputs are `optional: false`; after mix resolution assert `rows > N_min` and that the histogram contains each required slice with minimum counts; fail the run before the first optimizer step. Test: build the canonical mix from the repo checkout alone (no `target/` data dir) and assert expected slice counts.
**Phase to address:** P3 (mix wiring), P4 (proof run uses the canonical mix, not `--fast-corpus`, or records that it did not).

---

### P1: Pushing unverified or regressed weights ("ungated" is one flag away)

**What goes wrong:** `hub sync --push --allow-ungated` already exists and "marks card as ungated". A milestone titled "trustworthy releases" fails if a release can be produced by habit-passing that flag, or if the gate compares only against the base model (a regression vs the PREVIOUS release goes unnoticed).
**How to avoid:** Make the default path non-bypassable in CI: a release is created only by a command that (a) loads a `release-evidence.json` generated by the eval pipeline, (b) re-verifies that the evidence's `adapter_weights_blake3` equals the BLAKE3 of the files about to be uploaded, (c) requires non-regression vs the last published tag (paired, exact McNemar, p-threshold fixed ahead of time) and absolute floors from B7, (d) refuses `unknown` spoke/slice (R2/R3). `--allow-ungated` remains only for private/dev repos and stamps `ungated` into card AND repo metadata AND blocks the "latest"/stable tag. Mutation-test the gate (break the comparison, confirm the test fails), per repo policy.
**Warning signs:** Any public repo whose card says `ungated`; evidence hash != uploaded hash.
**Phase to address:** P6.

### P2: Mutable `main`, mutable tags, no pinned revision

**What goes wrong:** Consumers (Space, docs, `pull_adapter_revision_with_spoke`, users) fetch `main` or a tag. Tags/branches are mutable pointers; only the commit SHA is immutable. Hugging Face's own security stance and the Bandit rule treat an unpinned download as the risk; a re-push, a card-only update, or a late fix changes what `main` serves while published eval numbers describe older bytes. A multi-file load can also straddle two commits if `main` moves mid-download (the hub library recently added `resolve_revision` to pin once per load).
**How to avoid:** Every release has an immutable record: `{repo, commit_sha, version tag, weights_blake3, evidence_blake3}` committed in-repo (`releases.json`) and printed in the card as the SHA, not just the tag. All first-party consumers (Space, docs snippets, `hub pull`, tests) download by full SHA. Never force-move a version tag; a fix is a new version. Keep `main` = last gated release only, never a staging area; stage on a branch/revision and fast-forward after verification. Upload weights + card + evidence in ONE commit (single `create_commit`) so no commit exists with weights but a stale card, or vice versa; `--cards-only` push must be refused when the card references numbers whose evidence is not already in that revision.
**Warning signs:** Docs say `revision="main"`; two files of one release with different commit SHAs; a card-only commit after a weights commit changing numbers.
**Phase to address:** P6; consumed by P7.

### P3: Card numbers not derivable from uploaded evidence; redundant card text drifts

**What goes wrong:** `README.md` and `MODEL_CARD.md` are separate files pushed together by `sync_cards_only`; handwritten numbers (pass@1, dataset sizes, hours, USD) drift, or describe a run other than the uploaded one. A `default_spoke`-style hardcode (R3) can make the card state the wrong adapter target.
**How to avoid:** Generate the card from one template + `release-evidence.json`; the two files become one source (or one a pure copy with a hash assertion). Every numeric token in the card must come from a template variable bound to an evidence field; add a test that parses the rendered card, extracts all numbers, and asserts each is either in the evidence JSON or on an explicit allowlist (e.g. version numbers). Include in the card: bench manifest hash, verifier hash, n, interval, paired baseline comparison, truncation rate, `completeness`, SHA of the training run, and the blind-slice number with a statement of how many times it was evaluated. Include the repo's own limits: "single seed", "n=166", etc., derived not typed.
**Warning signs:** `rg '[0-9]+(\.[0-9]+)?%' README.md MODEL_CARD.md` yields numbers not in evidence; README and MODEL_CARD disagree.
**Phase to address:** P6.

### P4: Licence / attribution errors (MIT-with-attribution bases, teacher-distilled data, base-model lineage)

**What goes wrong:** (a) MiMo-V2.6-Distill-Qwen-9B is "MIT (+ attr.)" per the handoff; MIT requires keeping the copyright notice and licence text with substantial copies, and the base lineage (Qwen) carries its own licence; a LoRA adapter card with only `license: apache-2.0` omits it. (b) The teacher is "MIT" `deepseek/deepseek-chat`: that OpenRouter alias resolves to whatever DeepSeek version is current; DeepSeek V3 originally shipped under a custom model licence with use restrictions that defined derivatives to include distillation/synthetic-data methods, and only V3-0324 and later moved to MIT. A row generated by the wrong version (or by a provider route with its own terms) is mislabelled. (c) `humaneval-vox` rows derive from HumanEval (MIT) and need attribution. (d) Model-agnostic facade rule: licence is per model version, not per vendor.
**How to avoid:** Per row, store the response's actual `model` id/route from the provider, not the requested alias; a licence allowlist keyed by exact resolved id (`ModelRegistryEntry.license` + licence URL + date checked) and reject rows whose resolved id is not on it. Release repos include `LICENSE` (the release licence), `NOTICE`/`ATTRIBUTION.md` listing base model(s) with copyright lines and licence texts (MiMo MIT text, Qwen Apache-2.0 + modification statement, HumanEval MIT), the `base_model:` + `base_model_relation` metadata, `license:` metadata consistent with the files, and a dataset card listing teacher ids with licence. A test asserts the card's licence block equals the generator's `licenses.toml`. Do not rely on the spec's optional frontmatter `license` field as evidence; use repo-level licence files (matches AGENTS.md skills rule). This is engineering hygiene, not legal advice; flag anything commercial for human review. Never ship the excluded bases (cc-by-nc xLAM/Hammer, Gemma, Llama, Devstral modified-MIT).
**Warning signs:** Teacher id in the ledger is an alias with no version; card licence field differs from `LICENSE` file; no attribution file in the release folder.
**Phase to address:** P5 (row metadata + allowlist), P6 (card/NOTICE generation + test).

### P5: Renaming or deleting files that existing downloaders depend on

**What goes wrong:** Existing consumers (`hub pull`, `pull_adapter_revision_with_spoke`, user scripts, the cache's `snapshot_download`) expect file names/layout (adapter weights, manifest, config). Moving to a new layout (subfolders per spoke, renamed manifest, deleting old `MODEL_CARD.md`) breaks them; `snapshot_download` with old files removed also leaves stale files in local caches, so breakage appears only on fresh machines.
**How to avoid:** Additive changes only to a published repo layout: keep every existing filename; add new files (`release-evidence.json`, `ATTRIBUTION.md`) beside them; a layout change is a new repo or a documented major version with the old SHA still downloadable. Contract test: download the PREVIOUS release's file list by SHA and the new one; assert old names present or the change is declared. Test the real consumer (`pull_adapter_revision_with_spoke`) against a staging repo in CI before tagging.
**Warning signs:** A `delete` op in the commit plan of an existing repo; a downloader test that only uses fresh cache.
**Phase to address:** P6.

### P6: Gated-access and token handling in the release path

**What goes wrong:** "Gated" can mean (a) the Hub access-request gate for the repo or (b) the eval gate before upload. Conflating them leaves a repo that is gated for users but unverified, or a Space that needs an authenticated fetch of a gated repo (impossible without leaking a token to the browser; see S5).
**How to avoid:** Name them differently in code/docs: `eval_gate` (internal) vs `hub_access_gate`. The repo the Space loads from is public and un-gated, with the weights it serves explicitly licensed for that. Tokens for upload come through Clavis (`vox_secrets::resolve_secret`) and never appear in logs, cards, or evidence JSON; run `vox ci secret-env-guard` / `secrets-parity` if a secret surface changes.
**Phase to address:** P6.

---

### D1: Teacher spend decided by noise, over-spend by retries, and cost reported only on successes

**What goes wrong:** Stage gates keyed to `<20%` on categories with n=4-7 (B6); spend cap enforced after the call; `usd_per_verified_row` computed from successes only (0.1001 USD / 98 rows hides the 82 failed attempts' cost unless included; the handoff's 54.4% yield suggests it was included, but the metric definition must say so). Repair turns multiply calls invisibly.
**How to avoid:** Spend gate is code, not prose: the distillation command requires a funnel report path whose hash is recorded, and refuses to run if the report predates the current verifier hash (B3) or the category n is below the pooled-evidence minimum. Hard cap enforced BEFORE each request using `max_tokens x price` reservation; stop on cap, write the ledger incrementally (append-only) so a crash does not lose spend history. Report `usd_total / verified_rows_after_dedup_and_decontam` (all attempts, all repair turns). Stage 5 reserve stays unspent by default; a one-line ledger test asserts `sum(ledger) <= cap`.
**Warning signs:** Ledger total != provider dashboard total; rows accepted after decontam << rows paid for.
**Phase to address:** P5.

### D2: Synthetic collapse / mix ceiling not enforced

**What goes wrong:** Distilled and self-play synthetic rows crowd out human `.vox` golden anchors; the model learns teacher/Python-like idioms and degrades `@table`/`query`/`mutation`/`workflow` idioms while `humaneval-vox` rises (cross-slice collateral).
**How to avoid:** Enforce a mix ceiling per `origin` in the mix resolver with a hard error (not a warning); per-category deltas on the collateral matrix are a release blocker (non-regression on idiom categories), per-slice `supervised_ratio` recorded. Keep the human golden anchors as `optional: false`.
**Phase to address:** P5 (ceiling), P4 (collateral matrix baseline).

---

### S1: WebGPU is not available on most visitors' devices or configurations

**What goes wrong:** WebGPU is now in Chrome/Edge, Firefox (141+ on Windows; macOS from 145 on ARM64 only; Linux/Android in progress per web.dev) and Safari 26, but Android needs Android 12+ with supported GPUs, and enterprise browsers, older iOS, Linux, Intel Macs on Firefox, and many headless/VM environments have none. A demo built assuming `navigator.gpu` shows a blank or crashes for a large share of visitors. I found no authoritative current usage percentage in this research; do not state one in the card.
**How to avoid:** Feature-detect with `await navigator.gpu?.requestAdapter()` (not just `navigator.gpu`), record `adapter.info`/limits, and branch to ONE of three explicit states: `live-webgpu`, `live-wasm` (only if a CPU model is genuinely small enough), or `unsupported` (shows "your device cannot run the live demo" + a clearly-labelled recorded example). Never a silent switch (S4). Document the minimum browser matrix in the Space README, generated from the same test matrix CI runs.
**Warning signs:** Support requests of "nothing happens"; no telemetry/log of which state a visitor got (privacy-respecting local display only, no tracking by default: repo is offline-by-default).
**Phase to address:** P7.

### S2: Multi-GB downloads, GPU buffer limits and memory ceilings

**What goes wrong:** Auto-downloading the model on page load costs the visitor GBs and crashes mobile (iOS jetsam at ~1 GB buffers, per WebLLM/WebKit thread). WebGPU defaults are small (`maxBufferSize` 256 MiB, `maxStorageBufferBindingSize` 128 MiB) and must be requested at device creation; hardware reports vary; weights not sharded to fit fail at allocation time with opaque errors; JS/WASM heap and file-read layers add further memory (MediaPipe write-up). A WebGPU in-browser model can only be ONE the runtime supports.
**How to avoid:** (a) Explicit "Download N MB and run" button showing the exact size, never on load. (b) Shard weights well under the binding limit; request `requiredLimits` from `adapter.limits`; check memory estimate vs `navigator.deviceMemory` where available and refuse politely. (c) Persist in Cache API/OPFS with a visible "clear cache" control; handle quota eviction and resumable/partial downloads; verify SHA of each shard against the pinned manifest. (d) Pick the smallest model that makes the demo meaningful.
**Warning signs:** Page load network > a few MB before click; allocation errors at `createBuffer`; tab crash on mobile.
**Phase to address:** P7.

### S3: The browser model is not the released model (silent model substitution)

**What goes wrong:** The released/evaluated spokes are 9B-27B hybrid `qwen3_5_text` (GatedDeltaNet) models; the handoff itself says to "keep Qwen3-0.6B only for CPU/WebGPU ONNX". A browser demo of a 0.6B (or other) model next to a card showing the 27B's numbers implies capabilities the demo does not have. Hybrid linear-attention may also lack ONNX/WebGPU runtime support `[VERIFY]`.
**How to avoid:** The Space states, in the UI next to every output, the exact model, size, quantization and revision SHA that is running, and links its own evidence (its own eval on the same blind slice if any). Do not show the headline numbers of a different model on the Space. If the browser model is un-fine-tuned or generic, say so.
**Phase to address:** P7 (UI + copy), P6 (card wording).

### S4: Demo silently replays cached evidence while labelled "live"

**What goes wrong:** Fallback-on-error design: when WebGPU fails or the model is slow, the page falls back to canned completions (shipped for offline use or for tests) and still displays "Live". Because outputs look right, nobody notices, and visitors/reviewers believe inference happened.
**How to avoid:** Provenance is a typed, always-visible field on every rendered output: `source: live-webgpu | live-wasm | replay`, set at the single place that produces text, with replay requiring an explicit user action ("Show recorded example"). Replays are visually distinct. No code path may emit text without setting `source`; unit-test that the renderer throws if `source` is missing. Display measured tokens/sec and time-to-first-token for live outputs (cannot be faked by a replay without it being obviously constant).
**Warning signs:** Identical latency across prompts; output independent of input; no model download observed in the network panel.
**Phase to address:** P7.

### S5: Token handling in the browser

**What goes wrong:** Static Spaces expose both variables AND secrets to client JS through `window.huggingface.variables` (hub-docs PR), so any "secret" put in Space settings is public. Shipping an HF token (even a read token) to fetch a gated/private repo, or embedding it in JS, leaks it.
**How to avoid:** The Space loads only from public, un-gated repos pinned by SHA; no token of any kind in the bundle, settings, or repo; if a feature truly needs auth, use a server-side proxy or the Hub OAuth flow, not a static secret. CI check: grep the built Space for `hf_` token patterns and for the Clavis secret names (extend `vox ci secret-env-guard` scope or add a bundle scan).
**Phase to address:** P7.

### S6: CORS and redirect targets

**What goes wrong:** Browser `fetch()` of model files cross-origin works only if every hop (the `resolve` URL and its redirect to the CDN/LFS/Xet host) sends CORS headers and the request is simple enough to avoid a failing preflight. Community reports conflict and some are old: an old issue says model files lacked `Access-Control-Allow-Origin`; many in-browser libraries load from the Hub in practice; a forum thread reports a Spaces proxy stripping a credentials header on preflight. Sending an `Authorization` header (gated/private) forces a preflight and is the typical break. Failures are generic network errors even when the network panel shows 200.
**How to avoid:** Test in a real browser from the real `*.hf.space` origin (not localhost) against the exact pinned `resolve/<sha>/...` URLs, public repo, no auth header; add a deploy-time probe (`fetch` HEAD + a small range request) that fails the Space build when CORS is broken. Have a documented fallback: mirror shards in the Space repo itself (same-origin) if the Hub CDN CORS regresses, within the Space's size limits `[VERIFY limits]`. Range requests and cross-origin-isolation headers (needed for threaded WASM) can conflict with CORS: set COOP/COEP in README front matter only if needed and re-test cross-origin fetches after (COEP blocks no-CORS cross-origin loads).
**Phase to address:** P7.

### S7: Headless tests pass without ever running inference

**What goes wrong:** Headless Chromium has no WebGPU by default (`requestAdapter()` returns null without Vulkan/SwiftShader flags and system Vulkan packages). An e2e test that "loads the page, clicks Run, sees an answer" passes because the page fell back to replay (S4), or the assertion is only "an element with text appears". The repo's own GUI policy demands Playwright coverage, which makes this tempting to satisfy superficially.
**How to avoid:** Tests must assert the live path: (1) `navigator.gpu` + `requestAdapter()` non-null (enable the unsafe-WebGPU/SwiftShader flags and Vulkan libs in the CI image, documented by Chrome and Emscripten's CI config; verify with `chrome://gpu`), else the job reports SKIPPED/NOT-RUN, never green; (2) run a randomized-nonce prompt (`"repeat this token: <uuid>"`) so a canned replay cannot satisfy it, assert the nonce appears and `source == live-webgpu`; (3) assert two different prompts give different outputs and `tokens/sec > 0` and that weight shard requests actually happened (network log); (4) a tiny test-only model so CI is fast, plus a human UAT checklist item for a real GPU device; automated CI cannot prove real-hardware inference. Mutation check: force the live path to throw and confirm the test fails.
**Phase to address:** P7.

---

### M1: Parallel agents editing the same hot files in a dirty, mixed-provenance tree

**What goes wrong:** Planned work concentrates on a few files: `eval_local.rs`, `reasoning_eval.rs`, `reasoning_compare.rs`, `eval_spoke.rs`, BOTH `vox-plugin-mens-candle-{metal,cuda}/src/inference.rs` (duplicated code; the CUDA one cannot be tested locally), `mix` code, `domain-profiles.yaml`, `hub.rs`, plus `REQUIREMENTS.md`/`ROADMAP.md`. The tree is already ~50 files dirty with unrelated work (doc-pipeline, gui, Cargo.lock). Per repo notes, "move + reformat" merges produce duplicate definitions with no conflict markers; parallel `rustfmt` drift lands in the wrong commit; union-resolving is unsafe except append-only registries.
**How to avoid:** P0 first: review and commit (or stash to a branch) the Phase 547-550 working tree in clean, path-scoped commits so v9.5 starts from a SHA; never `git add -A`. One owner per hot file per wave; partition by crate/file in the roadmap (plan frontmatter `files_modified` must not overlap within a wave). Use worktrees for parallel agents, each with its own `target/` (never symlinked) and the vox-gui sidecar/`ui/dist` caveat. After merging parallel work: `grep -c 'fn <name>'` for each moved item, `cargo check -p <crate>`, `vox run scripts/fmt.vox -- --all`. Duplicate Metal/CUDA changes in the same plan with a shared test vector so they cannot diverge. Verify shipped state with `git show <sha>:<path>`, not the working tree.
**Warning signs:** Two plans touching one file; a conflict-free merge that fails `cargo check`; a commit containing files the agent did not intend.
**Phase to address:** P0 and every wave's plan check.

### M2: Concurrent cargo builds and the isolated-artifact contract

**What goes wrong:** Parallel agents invoke `cargo` against one workspace/`target/`, serialize on the build-dir lock for hours, or (worse) rebuild `target/release/vox-ml-cli` or the installed plugin dylib while PID 34157 is using them. The handoff built canary artifacts in `target/v94_canary_rel`, `target/v94_canary_plugins` precisely to avoid this.
**How to avoid:** All cargo through the build broker (plain `cargo` on PATH, never `~/.cargo/bin/cargo`); background long builds, never foreground-wait (repo memory: "never block on cargo"); executors run `cargo check -p <crate>` + the new tests only, CI does the rest (repo policy: hosted CI is the gate; never `cargo clippy --workspace` locally). Never write to `target/release/` or `~/Library/Application Support/vox/plugins/` while the live job runs; use `CARGO_TARGET_DIR` per task plus `VOX_PLUGINS_DIR` override. Never pattern-kill (`pkill -f`); check with `ps -p 34157 -o pid,etime,rss,command` and wait.
**Warning signs:** Build "hangs" at "Blocking waiting for file lock"; `vox doctor` broker warnings; plugin version mismatch errors at runtime.
**Phase to address:** P0.

### M3: A single (unified-memory) GPU shared with a long-running job corrupts both the job and the measurements

**What goes wrong:** On Apple Silicon the 27B eval/training and any other model load share one memory pool and one GPU. A second GPU process can push the long run into swap/jetsam, killing 10+ hours of eval; and any timing taken during overlap (tok/s, `eval_wall_secs`, the P2 speed-up claim) is contaminated. The memory predictor (`memory-model.v1.yaml`) was fit for training; eval and a concurrent canary are outside its fit.
**How to avoid:** A serialization lock for GPU work (reuse the repo's `lockf`-style convention for local CI, e.g. `lockf -k ~/.cache/mens-gpu.lock <cmd>`); all GPU tasks declare estimated peak memory first (predictor + margin) and refuse to start if free memory < estimate. Record `concurrent_gpu_jobs`, load average, and free memory in every timing report header; timing comparisons use only runs with `concurrent_gpu_jobs == 0`. Plan so that P1/P2 development uses the 0.8B/2B canary and CPU tests; any 27B GPU step is queued strictly after PID 34157 finishes and its report is saved.
**Warning signs:** Memory pressure warnings, swap growth, one job's tok/s halving when another starts.
**Phase to address:** P0 (lock + header fields), P1 (record), P4 (only compare clean-load runs).

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Keep `default_spoke: vox-lang` as fallback for old manifests | No migration code | Wrong spoke on public cards; two sources of truth | Never for releases; ok for local dev display with `unknown` flag |
| `#[serde(default)]` instead of `Option` for new metrics | Compiles fast | Zeros presented as measurements | Never for metrics |
| Add `--allow-ungated` to the CI release path "just this once" | Ships a release | Normalises regressed releases | Private/dev repos only, never tagged stable |
| Compare to old verdicts instead of re-verifying baseline completions | Skip re-score time | Verifier drift in the diff | Never (verification is cheap) |
| Handwritten card numbers | Fast | Drift, unverifiable | Never; template + evidence only |
| Replay fallback for the Space "so it always works" | Demo never blank | Dishonest "live" label | Only as an explicitly user-triggered, labelled mode |
| Hardcode CPU/job counts in new scripts | Quick | Violates repo rule | Never (derive from machine) |
| Port prefix cache to Metal only, "CUDA later" | Less work | Divergent behaviour, untested CUDA | Acceptable if CUDA path stays cache-OFF behind a flag and tests assert it |
| Single seed per arm | Cheaper | Seed noise > effect | Only if reported as "indicative", with intervals |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Plugin JSON <-> CLI | Assume both are rebuilt together | Schema version in response; `Option` fields; refuse claims on old plugin |
| Hugging Face Hub | Tag or `main` as the identifier | Full commit SHA in every consumer and in the card |
| HF Hub upload | Weights and card in separate commits | One commit; evidence file in same revision |
| OpenRouter teacher | Trust requested model alias for licence | Record resolved `model` per row; allowlist by exact id+date |
| `vox_actor_runtime::llm` facade | Add vendor hostname/SDK in distillation script | Facade only; `llm_provider_call` audit must stay clean |
| `item_checkpoint` | Add new inputs to key to "be safe" | Leave key; add additive provenance fields (invalidating caches costs GPU hours) |
| Generated docs (`*.generated.md`) | Hand-edit after renaming CLI flag | `vox ci command-sync`; autoregen bot owns drift |
| Cross-repo contracts | Rename key in `contracts/` only locally | `vox ci cross-repo-parity` (+ `--sync` if shared) |
| Static Space + HF CDN | Test CORS from localhost | Test from `*.hf.space` against pinned URLs, public repo, no auth header |
| Playwright WebGPU | Assume Chromium headless has a GPU | Explicit flags/Vulkan libs, SKIPPED not green when absent |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Decode-dominated eval, prefix cache wins little | Eval wall barely changes | Measure prefill/decode split first (P1) | `max_tokens` 1024 with long completions |
| Snapshot clone per item | RSS grows linearly | One snapshot, free on item end | 27B on unified memory, 166+ items |
| Re-verifying with interpreter per item serially | CPU idle GPU idle | Verify while next generation runs (CPU/GPU overlap) | Full 166 x n=5 pass@k |
| Warm-up counted in p50 | First item 20x slower skews tail | Report first-item and steady-state separately | Small canary (64) |
| Sorting prompts lexicographically for cache friendliness | Order dependence in sampling | Seed per item (not a shared RNG stream); greedy for proofs | Any temperature > 0 eval |
| Large Space asset load | Slow first paint, mobile crash | Click-to-download, shard, cache | Any model > ~500 MB on mobile |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| HF token (or "secret") in static Space | Public token leak (static Spaces expose variables and secrets to JS) | Public pinned repo only, no tokens in bundle; bundle scan in CI |
| Loading weights by `main`/tag | Silent weight substitution (supply chain) | SHA pin + per-shard hash check in browser |
| Teacher output executed by interpreter without limits | Hang/OOM (the `to_hex` doubling was found this way) | Keep step/string/time caps and run verifier in sandbox tier; caps are part of frozen proof manifest |
| Pickle/unsafe formats in release | Arbitrary code on load | safetensors only |
| Uploading from a worktree that has `.env`-derived values in evidence/logs | Secret leak | Evidence JSON built from an allowlist of fields; scan before upload |
| Publishing the eval items | Future models/data contaminate | Publish hashes + aggregate results; keep blind slice private until retired |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| "Live" badge that is always on | Misled about what ran | Per-output provenance chip (S4) |
| No size warning before download | Data cost / crash | Exact MB, explicit confirm, cache control |
| Card leads with the best number | Overclaim | Lead with paired delta + interval + n + blind-slice; list limits |
| Unsupported device sees a blank page | Looks broken | Explicit "unsupported" state with browser matrix |
| Failure funnel only as JSON | Maintainers cannot see where completions die | One-screen text/table summary at end of every eval |

## "Looks Done But Isn't" Checklist

- [ ] **Prefix KV cache:** often missing cold-vs-warm token-identical proof on ALL items, snapshot immutability test, base->adapter->base test; verify the committed equivalence report is referenced by the proof.
- [ ] **Timing:** often missing `cached_items`/`generated_items` and `concurrent_gpu_jobs`; verify no tok/s is computed over cached items.
- [ ] **Failure funnel:** often missing `sum == n` assertion and `stop_reason`; verify buckets are exclusive.
- [ ] **Rename:** often missing value migration (`vox_codegen` -> slice) and writer-side compatibility; verify with real old manifests as fixtures and `rg -w lane` allowlist test.
- [ ] **Before/after:** often missing frozen verifier hash and re-scored baseline; verify both reports carry identical frozen manifest.
- [ ] **Proof statistics:** often missing paired exact test and intervals; verify card shows b/c counts.
- [ ] **Blind slice:** often missing seal (committed hash before data generation) and single-use accounting.
- [ ] **Distillation:** often missing resolved-model-id licence check and pre-call cap; verify ledger sum <= cap and provider-dashboard match.
- [ ] **Release:** often missing weights-hash == evidence-hash check, SHA in card, NOTICE/attribution; verify by downloading the release by SHA on a clean cache with the real `hub pull`.
- [ ] **Space:** often missing real-device live inference proof; verify nonce-prompt test passes with `source == live-webgpu` and a human UAT on a real GPU is recorded.
- [ ] **Multi-agent:** often missing clean starting SHA; verify `git status` has no foreign files before each wave.

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Cache not equivalent (K3) after results published | HIGH | Disable cache by flag, re-generate affected items (key-bumped), re-run proof, publish corrected evidence and new version tag |
| Contaminated benchmark (B1) | HIGH | Retire the slice, build and seal a new blind slice, re-score all published releases on it, annotate old cards |
| Regressed weights released (P1) | MEDIUM | Do NOT delete; publish a new version, move `main` to last good SHA via a new commit, mark bad version deprecated in card (old SHA stays downloadable) |
| Card states wrong number (P3) | LOW | New card commit regenerated from evidence; note correction in changelog section |
| Wrong licence/attribution (P4) | MEDIUM | Add NOTICE and fix metadata in a new commit; drop affected teacher rows, retrain only if licence forbids the rows |
| Verifier drift discovered (B3) | MEDIUM | Re-score all stored completions with pinned verifier (CPU), regenerate reports |
| Long GPU job killed by contention (M3) | HIGH | Resume from item checkpoint (`item_checkpoint` is append-only; resume works), then enforce GPU lock |
| Space labelled live while replaying (S4) | LOW | Hotfix label + provenance chip; add failing test first |
| Alias one-way break (R1) | MEDIUM | Ship dual-write patch; fixtures for old readers; bump `schema_version` |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| K1 token-identical prefix | P2 | Property test on boundary-sensitive prompts; runtime id-prefix assert; miss counters |
| K2 recurrent/conv state snapshot, aliasing | P2 | State-hash-unchanged test; non-chunk-aligned prefix test |
| K3 numerical equivalence | P2 -> P4 | Equivalence report: token-identical greedy on all 166+64, logit diff vs measured noise floor |
| K4 cache key scope | P2 | base->adapter->base test |
| K5 cached vs fresh mixing | P1, P2, P4 | Report headers `cached_items`/`generated_items`; comparator refuses mixed cache modes |
| K6 absent-as-zero, plugin skew | P1 | Old-plugin fixture test yields `timing: unavailable` |
| K7 memory growth | P2 | Flat peak-memory test on canary model |
| K8 speedup claim honesty | P1, P2 | Interleaved ABAB timing, clean-load only |
| B1 targeting leakage | P4 seal, P5 firewall | Import-boundary test, layered decontam log, blind-slice delta reported |
| B2 canary nested in bench | P4 | Report lists shared items; confirmation on disjoint remainder |
| B3 verifier/benchmark drift | P0, P4 | Frozen proof manifest equality check; baseline re-scored |
| B4 truncation/limits as failure | P1, P4 | Funnel sums to n; sensitivity row |
| B5 step-capped vs full | P3, P4 | `completeness` field; NOT COMPARABLE output |
| B6 small-n noise | P4, P5 | Exact intervals, paired McNemar, pre-registered primary metric |
| B7 stale gate thresholds | P4, P6 | Thresholds change in own reviewed commit; non-regression vs last release |
| R1 alias one-way | P3 | Round-trip matrix incl. frozen old-reader struct |
| R2 value migration lossy | P3, P6 | Value-mapping table test; `unknown` blocks "verified" wording |
| R3 old manifests mis-attributed | P3 | Real old manifest fixtures; `default_spoke` removed |
| R4 docs/config drift | P3 | `lane` inventory + allowlist test; command-sync, cross-repo-parity |
| R5 mix not wired / optional silent | P3, P4 | Mix builds from checkout alone; min-row/slice assertions |
| P1 regressed/unverified release | P6 | Evidence-hash==upload-hash; mutation-tested gate |
| P2 mutable revision | P6, P7 | SHA-only consumers; single-commit upload; `releases.json` |
| P3 card drift | P6 | Card-number-in-evidence test; single template |
| P4 licence/attribution | P5, P6 | Resolved-id allowlist; card licence == `LICENSE`; NOTICE present |
| P5 downloader breakage | P6 | Old-vs-new file list contract test; real `hub pull` on clean cache |
| P6 two meanings of "gated" | P6 | Naming split in code/docs |
| D1 spend by noise, cap after call | P5 | Pre-call reservation; ledger sum test; funnel-report hash required |
| D2 mix ceiling | P5 | Hard error on ceiling; idiom-category non-regression |
| S1 WebGPU unavailable | P7 | Three explicit states; browser matrix doc |
| S2 size/limits | P7 | Click-to-download; shard-under-limit; clear-cache |
| S3 model substitution | P7, P6 | UI shows model/size/SHA; no foreign headline numbers |
| S4 replay labelled live | P7 | `source` field mandatory; renderer throws without it |
| S5 token in browser | P7 | Bundle scan for tokens |
| S6 CORS | P7 | Deploy-time probe from `*.hf.space` |
| S7 headless passes without inference | P7 | Nonce prompt + `source==live-webgpu` + SKIP-not-green + human UAT |
| M1 same-file edits, dirty tree | P0 + every plan | No overlapping `files_modified` per wave; clean base SHA; `grep -c` post-merge |
| M2 concurrent cargo | P0 | Broker; per-task target dir; no writes to live artifacts |
| M3 shared GPU | P0, P1, P4 | GPU lock; header `concurrent_gpu_jobs`; clean-load-only timings |

## Phase-Level Research Flags (for the roadmapper)

- **P2 (prefix KV cache): needs a design spike first.** The delta-rule chunk alignment and the exact `Qwen35LayerCache` mutation semantics are implementation-level unknowns; serving systems needed non-trivial alignment (vLLM 528-token blocks for Qwen3.5) and application-declared boundaries. Plan: spike on CPU with the 0.8B config, produce the equivalence harness BEFORE the optimization.
- **P7 (Space): needs a feasibility spike** on which model/runtime can run in-browser at all (hybrid arch support in ONNX/WebGPU runtimes is unverified), and on Space size limits and CORS from the real origin.
- **P4/P5:** standard but statistics-sensitive; decide the primary metric and blind-slice protocol in the plan, not during analysis.
- **P3, P1, P6:** standard patterns; no extra research, but P3 needs the occurrence inventory first.

## Sources

- vLLM RFC, application-directed prefix checkpoints for hybrid GDN/Mamba (1-2 snapshots per sequence; per-layer state size; checkpoint placement): https://discuss.vllm.ai/t/rfc-pr-stack-application-directed-prefix-checkpoints-for-hybrid-gdn-mamba-up-to-7-6x-speedup-on-l40s/2949.md (MEDIUM; RFC reports latency only, no accuracy evidence)
- vLLM issue on hybrid prefix caching / Qwen3.5 528-token block alignment: https://github.com/vllm-project/vllm/issues/40696 (MEDIUM, via search summary, not opened)
- SGLang hybrid radix/Mamba cache: https://pytorch.org/blog/hybrid-models-meet-sglang-more-than-full-attention/ (MEDIUM)
- Marconi, prefix caching for hybrid LLMs: https://arxiv.org/pdf/2411.19379 (MEDIUM)
- Thinking Machines, Defeating Nondeterminism in LLM Inference (batch invariance): https://thinkingmachines.ai/blog/defeating-nondeterminism-in-llm-inference/ and Simon Willison's summary https://simonwillison.net/2025/Sep/11/defeating-nondeterminism (MEDIUM; primary page not opened)
- Tokenization boundaries and prefix-cache keys (BPE pre-tokenizer/merge effects at joins; special tokens as atomic boundaries): https://arxiv.org/pdf/2607.29678 , https://discuss.huggingface.co/t/how-to-cache-common-instruction-prompt/101419/7 (MEDIUM; token healing not shown to fix cache reuse)
- Yang et al., Rethinking Benchmark and Contamination with Rephrased Samples: https://arxiv.org/abs/2311.04850 ; LMSYS decontaminator blog https://www.lmsys.org/blog/2023-11-14-llm-decontaminator (MEDIUM-HIGH)
- Miller, Adding Error Bars to Evals: https://arxiv.org/abs/2411.00640 ; Bowyer, Aitchison, Ivanova on CLT failure for small benchmarks: https://arxiv.org/abs/2503.01747 ; errorbars package (paired SE, exact McNemar) https://pypi.org/project/errorbars/ (MEDIUM; McNemar attribution is from secondary write-up)
- Truncation scoring conventions in evals: https://arxiv.org/pdf/2609.06993 , https://arxiv.org/pdf/2609.27678 , https://arxiv.org/pdf/2609.19113 , https://community.openai.com/t/o4-mini-returns-empty-response-because-reasoning-token-used-all-the-completion-token/1359002 (LOW-MEDIUM; recent preprints and forum)
- serde alias semantics (deserialization only; separate serialize/deserialize rename): https://serde.rs/field-attrs.html (HIGH for semantics; migration pattern is my inference, test it)
- Hugging Face revision pinning: https://bandit.readthedocs.io/en/1.8.6/plugins/b615_huggingface_unsafe_download.html , https://raw.githubusercontent.com/huggingface/huggingface_hub/97c5f5f2030c2df01b60548f7d357970a106cd7d/SECURITY.md , https://baseten.co/blog/pinning-ml-model-revisions-for-compatibility-and-security , https://discuss.huggingface.co/t/purpose-of-commit-hash-in-pretrainedmodel-from-pretrained/174304 (MEDIUM-HIGH)
- DeepSeek-V3-0324 moved to MIT (earlier V3 custom licence): https://api-docs.deepseek.com/news/news250325 , https://simonwillison.net/2025/Mar/24/deepseek/ (HIGH for the switch; original licence text not retrieved); DeepSeek custom licence derivative definition: https://raw.githubusercontent.com/deepseek-ai/DeepSeek-V3/main/LICENSE-MODEL (MEDIUM); MIT attribution duties for R1-distills (secondary): https://www.blackduck.com/blog/deepseek-license.html (LOW-MEDIUM; not legal advice)
- WebGPU browser status: https://web.dev/blog/webgpu-supported-major-browsers (HIGH for the vendor status; no usage percentage found)
- WebGPU limits (default `maxBufferSize` 256 MiB, `maxStorageBufferBindingSize` 128 MiB; must be requested): https://developer.mozilla.org/en-US/docs/Web/API/GPUSupportedLimits ; WebLLM/iOS jetsam thread https://bugs.webkit.org/show_bug.cgi?id=275958 ; memory layers: https://research.google/blog/unlocking-7b-language-models-in-your-browser-a-deep-dive-with-google-ai-edges-mediapipe/ (MEDIUM)
- Headless WebGPU in CI: https://developer.chrome.com/docs/web-platform/webgpu/troubleshooting-tips ; https://agent-browser.dev/webgpu (third party) (MEDIUM; exact Playwright args must be tested)
- Static Space variables/secrets are client-visible: https://huggingface.co/docs/hub/spaces-sdks-static , https://github.com/huggingface/hub-docs/pull/1284 (MEDIUM-HIGH)
- HF CORS reports (mixed, partly old): https://lightrun.com/answers/huggingface-huggingface_hub-cors-headers-for-model-files , https://discuss.huggingface.co/t/hugging-face-spaces-proxy-suddenly-stripping-access-control-allow-credentials-header-on-options-preflight/177064 (LOW; must be tested from the real origin)
- Repo sources: `/Users/brbrainerd/dev/vox/docs/src/architecture/mens-eval-flywheel-and-lanes-claude-handoff-2026-11-20.md`; `/Users/brbrainerd/dev/vox/AGENTS.md` (Perennial Bug Patterns, PR & Review Discipline, Dependency Discipline, Build Broker); `crates/vox-plugin-mens-candle-metal/src/inference.rs`; `crates/vox-ml-cli/src/commands/mens/{item_checkpoint,hub}.rs` (HIGH)
- Method note: provider seam (`research-plan`/Context7) was not used; Context7 requires OAuth in this session. Findings came from built-in WebSearch/WebFetch; two WebFetch attempts (arXiv PDF 2609.15021 on shared KV caching correctness failures, vLLM RFC body) returned little usable text, so that paper's findings are NOT relied upon here and are worth reading directly in P2 spike.

---
*Pitfalls research for: MENS flywheel, truthful lanes, trustworthy Hub releases (v9.5)*
*Researched: 2026-11-20*
