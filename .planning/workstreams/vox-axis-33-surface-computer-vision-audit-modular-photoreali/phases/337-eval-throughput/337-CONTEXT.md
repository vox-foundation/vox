# Phase 337: Eval throughput - Context

**Gathered:** 2026-11-06
**Status:** Ready for planning
**Source:** Orchestrator brief (no interactive discuss-phase). Where discuss-phase would have asked a
question, the recommended default was chosen and is marked `[ASSUMED]`. Items marked `[USER]` come
from the user's own brief.

<domain>
## Phase Boundary

Make gating every ladder size (0.6B, 1.7B, 4B, 8B, 14B) affordable at 128 items/slice. Delivers:

1. The next throughput win on top of the batched decode that already landed (7ccb4adaa), chosen by a
   measured diagnosis of why batch=16 gives only 2.47x on 0.6B Metal.
2. bf16 (and optionally quantized) decode behind an explicit protocol tag, with a fresh base and
   adapter baseline under that tag; tagged results are never compared with f32 results.
3. Base-side generations computed once per snapshot + decode settings + protocol tag and reused
   across adapters — only the reuse gaps that remain after 31c692401.
4. A sequential (early-stop) exact McNemar collateral test with a pre-registered stopping rule in
   `mens/config/eval-gates-ladder.yaml`, keeping the false-fail rate at or below alpha = 0.05.
5. Measured per-item wall-clock for 0.6B-14B before and after, and a 4B 128/slice collateral run
   that fits overnight.

Out of scope: spoke-aware gating, receipts and dataset repos (phase 338), cards (339), local
selector (340), debt sweep (341). 337 must leave its decode/protocol settings in a form 338's
receipts can hash.

</domain>

<decisions>
## Implementation Decisions

### Batched decode
- **D-01 [USER]:** Batched decode is done (7ccb4adaa: per-sequence batch=1 prefill, shared decode
  steps, per-row attention over unpadded caches, byte-identical to batch=1,
  `VOX_REASONING_EVAL_BATCH_SIZE`, default derived from free memory). 337 does not redo it.
- **D-02 [USER]:** Before writing a new kernel, diagnose the sublinear scaling (0.6B Metal, 512 cap:
  26.60 / 16.28 / 14.20 / 10.75 s/item at batch 1/4/8/16) across: per-row attention loops in
  `vox-plugin-mens-candle-metal/src/model.rs` `attend`, per-step host sync, sampler, cache
  concatenation, and anything else measured. The research decides whether a fused/batched attention
  kernel is the next win.
- **D-03 [ASSUMED]:** Any decode change must keep the batch=1 byte-identity guarantee under the f32
  protocol (proved by a test on a fixed slice). A change that cannot be byte-identical belongs under
  a new protocol tag (D-05), not under f32.
- **D-04 [ASSUMED]:** Prefer the cheapest instrumented win first (timing spans, removing per-step host
  syncs, batching the sampler, padded+masked batched attention using existing candle ops) over a
  hand-written Metal kernel. A custom kernel is planned only if research shows the cheaper fixes
  leave attention as the dominant cost, and then as its own plan.

### Precision / protocol tag
- **D-05 [USER]:** bf16/quantized decode changes scores, so it runs under its own protocol tag with
  a fresh base and adapter baseline, and is never compared with f32 results.
- **D-06 [ASSUMED]:** f32 stays the default protocol. bf16 is opt-in via the protocol tag. Quantized
  (int8/q4) decode is deferred unless research shows bf16 alone cannot fit 14B or 4B overnight.
- **D-07 [ASSUMED]:** The protocol tag is part of the decode settings: it is hashed into the per-item
  checkpoint key (31c692401), recorded in the collateral report, and the gate refuses (hard error, not
  a fail verdict) to pair base and adapter generations with different tags. This is the field 338's
  receipts hash as part of `decode_blake3`.

### Base-generation reuse
- **D-08 [USER]:** Base generations are already cached per snapshot (31c692401). 337 only fixes the
  remaining waste research finds (keys that ignored decode settings, `--force-restart` discarding
  the base cache) and does not rebuild the cache.
- **D-09 [ASSUMED]:** `--force-restart` should invalidate adapter-side checkpoints only by default; a
  separate explicit flag is needed to discard base-side generations.

### Sequential McNemar
- **D-10 [USER]:** Sequential testing keeps the false-fail rate at or below alpha (alpha-spending or
  an exact sequential design) and is pre-registered in `mens/config/eval-gates-ladder.yaml`.
- **D-11 [ASSUMED]:** Default design: a small fixed number of pre-registered looks per slice (for
  example at 32/64/96/128 items) with an exact alpha-spending boundary computed from the discordant
  counts, plus deterministic curtailment for the pass direction (stop when the remaining items
  cannot reach the fail boundary). Curtailment does not spend alpha. The exact look schedule and
  spending function are chosen by research and verified by an exact (enumeration) test that the
  overall false-fail rate is at or below 0.05.
- **D-12 [ASSUMED]:** Items are evaluated in the pinned slice order (f51c569c0 pins), so a look at k
  items is always the same k items. The report records the look at which the test stopped.
- **D-13 [ASSUMED]:** The fixed-n 128-item test stays available (and remains the rule for any report
  produced before the sequential rule lands). A report records which rule judged it.

### Constraints on execution
- **D-14 [USER]:** Phase 335 is actively editing `crates/vox-eval`,
  `crates/vox-ml-cli/src/commands/mens/eval_*` and `reasoning_eval.rs`. Plans that touch those files
  carry an `ls` precondition on 335's SUMMARY and re-read the files at execution time.
- **D-15 [USER]:** Phase 338 is planned and depends on 337. 337's outputs (decode settings, protocol
  tag, checkpoint key, collateral report fields) must stay compatible with 338's plans.
- **D-16 [USER]:** AGENTS.md rules: test-first, mutation proofs for guards, no new crate edges without
  a proposed exception, no hardcoded counts (batch sizes and look schedules derive from memory or
  config), per-crate `cargo check`/`cargo test` only.
- **D-17 [USER]:** Every run a plan proposes states its expected eval wall-clock. Any real-model
  benchmark or eval is a non-autonomous checkpoint (`autonomous: false`). No plan starts a model
  process while another eval is running on Metal.
- **D-18 [USER]:** Success evidence: measured per-item wall-clock for 0.6B-14B before and after, and a
  4B collateral run at 128 items/slice that fits overnight.
- **D-19 [ASSUMED]:** "Overnight" means a wall-clock budget of at most 10 hours for the full 4B
  collateral gate (base + adapter, all slices), on the user's Metal machine.
- **D-20 [ASSUMED]:** "Before" numbers come from existing timing in checkpoints/logs where available;
  only sizes with no usable record get a short pre-change benchmark (a few items, not full slices).

### Post-research defaults (2026-11-06, from 337-RESEARCH.md open questions)
- **D-21 [ASSUMED, refines D-04]:** No fused or padded-mask attention kernel in 337. The next win is
  three byte-identical plugin fixes: preallocated KV cache (`slice_set`), GQA attention without
  `repeat_kv` or the K^T copy, row-batched norm/RoPE/gate, and dropping the always-zero LoRA branch
  at inference. Each is proven bitwise against the current path (CPU synthetic, then Metal).
- **D-22 [ASSUMED, refines D-11/D-13]:** The sequential rule ships behind `rule: fixed_n` as the
  default in `eval-gates-ladder.yaml`; the sequential block (looks 32/64/96/128, per-look alpha
  [0.0005, 0.002, 0.005, 0.04795], worst-case size proven <= 0.05 by exhaustive enumeration) is
  pre-registered but not active until the user flips it. Research shows it is not a throughput lever
  (~0 items saved under H0), so the throughput success criteria do not depend on it.
- **D-23 [ASSUMED]:** Futility (early pass) is off and not implemented in 337. Deterministic
  curtailment cannot fire at alpha = 0.05, so it is not wired either.
- **D-24 [ASSUMED, refines D-07]:** Protocol tags are `f32` (default) and `bf16`. The default is
  omitted from the checkpoint-key inputs so every existing cached generation keeps its key (proved by
  a golden-key test); only `bf16` adds the field. The report's `decode` object includes the protocol
  and excludes batch width; that object is what 338's `decode_blake3` should hash.
- **D-25 [ASSUMED]:** The bf16 protocol changes block linears only; LM head, norms, logits and
  attention math stay f32. bf16 priority is set by the existing `decode_dtype_bench` measurement.
- **D-26 [ASSUMED]:** 337's protocol and sequential-rule plans are ordered before its GPU-bound
  throughput plans so 338-02/05/07 can start without waiting for benchmarks.
- **D-27 [ASSUMED]:** Requirement IDs THRU-01..THRU-12 are minted from 337-RESEARCH.md "Phase
  Requirements" and carried in plan frontmatter.

### Claude's Discretion
- Exact timing instrumentation format, plan granularity and wave layout.
- Where the sequential-boundary computation lives, provided it reuses the existing exact McNemar code
  and adds no crate edge.

</decisions>

<specifics>
## Specific Ideas

- Thinking mode decodes at a 2048-token cap. Measure (read-only) how many existing generations in
  `mens/runs/reasoning_compare/checkpoints/*/*.jsonl` hit the cap; long-tail rows dominate batched
  wall-clock because a batch runs until its longest row finishes.
- Weights are dequantized to f32 on Metal; 14B holds about 61 GB (load-path fix 9c8842f9b).
- Power notes for the 128-item McNemar gate: `.planning/phases/335-*/335-RESEARCH.md` §4a.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

- `.planning/ROADMAP.md` §"Phase 337", §"Phase 338"
- `.planning/phases/335-0-6b-collateral-gate-pass-and-hub-sync/335-RESEARCH.md` (§4a power),
  `335-01-PLAN.md`
- `.planning/phases/338-spoke-aware-gating-and-hub-repos/338-CONTEXT.md`, `338-RESEARCH.md`,
  `338-0*-PLAN.md` (consumers of 337's decode settings and checkpoint header)
- `mens/config/eval-gates-ladder.yaml`
- `crates/vox-plugin-mens-candle-metal/src/model.rs` (`attend`)
- Commits 7ccb4adaa (batched decode), 31c692401 (checkpoint keys), f51c569c0 (pins, grader),
  9c8842f9b (14B load path)
- `AGENTS.md`

</canonical_refs>

<deferred>
## Deferred Ideas

- Quantized (int8/q4) decode, unless research shows bf16 alone does not fit (D-06).
- Making bf16 the default ladder protocol. That would orphan every f32 baseline and receipt; raise
  with the user first.
- Flipping the ladder default from `fixed_n` to the sequential rule (one-way once a size is gated
  under it or 338 receipts hash it; user decision).
- Futility early-pass (needs a pooled-level proof first).
- A fused-kernel `kernels` protocol tag (fused softmax/rms-norm/rope): new tag + full re-baseline.
- Thinking-mode cap policy: 39% of 2048-cap generations hit the cap (55% of tokens); a lower cap or
  loop-stop changes scores. Raise with the user separately.
- 338-02 text lists "batch size" in `decode_blake3`; per D-24 it should hash the `decode` report
  object (protocol in, batch width out). Amend 338-02 when 338 is next touched; 337's SUMMARY states it.
- Out-of-scope observation: adapters are evaluated as NF4(W+dW) but trained as NF4(W)+dW; several
  adapter-vs-base scores are identical. Unverified; any fix would be a new protocol tag.

</deferred>
