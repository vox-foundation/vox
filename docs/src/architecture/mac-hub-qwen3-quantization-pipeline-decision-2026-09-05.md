---
title: "Mac-as-hub, Qwen3, and the quantization ladder — a decision record (2026-09-05)"
description: "Reassigns the RTX 4080 Super to a mesh spoke and a 128GB Apple Silicon Mac to the hub-and-spoke hub, keeps the already-pinned dense Qwen3 base rather than reinventing it, confirms the quantization pipeline (vox-quantize + ADR-043) already covers Metal, and names the one real gap: a unified-memory safety reserve for training/quantizing while the GUI runs."
category: "Architecture SSOTs"
status: "current"
---

# Mac-as-hub, Qwen3, and the quantization ladder (2026-09-05)

## Revision 2 (same day) — corrections after a deeper code audit

The first draft of this document was written before auditing `mens/config/` and
`crates/vox-populi/src/mens/tensor/`. Four findings materially change it. They
are recorded here rather than silently edited in, because the earlier reasoning
was wrong in ways worth remembering.

1. **The spoke SSOT already exists.** `mens/config/domain-profiles.yaml`
   ("Add new profiles here — no Rust code changes required") declares four
   fine-tuned spokes — `vox-lang`, `rust`, `tool-selection`,
   `argument-generation` — each with `base.{model,method,preset}`,
   `mix_config`, `eval_gate`, and `router.{triggers,priority}`. It is backed by
   `spoke_base_resolver.rs` (capability tag → largest SHA-pinned rung that fits
   VRAM, fail-closed), `spoke_validate.rs`, and a real CI gate
   (`vox ci spoke-check`). §3.1's "hub-and-spoke role assignment" was already
   built; only the hardware sizing is missing.
2. **Training on this Mac is blocked today.** `vram_autodetect.rs` queries only
   `nvidia-smi`. On macOS `get_system_vram_gb()` returns `None`, so
   `resolve_base_model()` fails closed with "no GPU VRAM detected; cannot size
   base tag" — **no spoke can resolve a base model on Apple Silicon.** This is
   the single blocking defect, and it is small.
3. **The GUI reserve is not a new subsystem — it is the return value of that
   missing function.** Because `pick_base` picks the largest rung that fits the
   reported number, reporting `total_unified − reserve` makes the existing
   fail-closed ladder GUI-safe automatically. §4.2's proposed
   `VOX_MENS_UNIFIED_MEM_RESERVE_GIB` belongs in `vram_autodetect.rs`, not in
   `memory_budget.rs`.
4. **The best-benchmarking hub models cannot be loaded by this trainer.**
   `vox-hf-layout` hard-rejects any checkpoint with `vision_config` /
   `image_token_id` / `ForConditionalGeneration`, because the text QLoRA
   trainer has no vision tower. Qwen3.8-27B and Qwen3.6-35B-A3B are both native
   VLMs and are therefore **rejected today**. See §3.2r.

### §3.2r — Hub model, corrected

The open-weights landscape as of 2026-09 (licences verified; benchmark figures
from vendor model cards where cited, aggregator blogs otherwise — treat the
latter as directional):

| Model | Licence | Shape | Why it is / isn't the hub |
|---|---|---|---|
| **Qwen3-32B** | Apache-2.0 | dense, text-only | **Chosen.** Already SHA-pinned in `gpu-specs.yaml` at `floor_mb: 60000`; loads with existing code; auto-resolves on this Mac the moment finding (2) is fixed. |
| Qwen3.8-27B | Apache-2.0 | dense, **VLM** | Best agentic scores of any locally-runnable open model (Terminal-Bench 2.1 73.0, OSWorld-Verified 84.3). Blocked by the VLM gate — the upgrade target, not today's answer. |
| Qwen3.6-35B-A3B | Apache-2.0 | **MoE** + VLM | SWE-bench Verified 73.4 (model card). Rejected for a *heavily fine-tuned* hub: MoE LoRA tooling is immature (expert collapse; routing-restricted LoRA is still an open research area), and it also trips the VLM gate. |
| DeepSeek V4 Pro | MIT | very large MoE | Strongest open coding result reported (80.6% SWE-bench Verified) but far too large for 128 GB, and a non-Qwen architecture. |
| Kimi K3 | conditional | 2.8T-A104B | Top open-weight intelligence index; untrainable locally and not OSI-permissive. |
| GLM-5.1 | MIT | dense | Credible MIT alternative, but a different architecture family. |

**Architecture lock-in is the decisive practical factor.** This workspace has a
hand-written `Qwen35LinearAttention` in Candle (depthwise causal conv, `a_log` /
`dt_bias`, F32 recurrence) because Candle ships no such layer. Staying in the
Qwen family reuses it; leaving the family means writing a new model
implementation. Combined with the VLM gate, that makes **Qwen3-32B the correct
hub base now**, and Qwen3.8-27B the thing to earn later by teaching
`vox-hf-layout` to load a text tower out of a multimodal checkpoint.

### §5r — The flower: one stem, removable petals

A growth model for this system, with each part's honest cost. The rule that
makes it a flower rather than a bush: **the stem is shared and must be
correct; a petal must be removable without touching the stem.**

**Stem** — everything depends on it, so it gets the engineering rigor:

| Stem part | State | Note |
|---|---|---|
| Spoke SSOT (`domain-profiles.yaml`) → `spoke_base_resolver` → `spoke_validate` → `training_selection` | Built, CI-gated | Adding a spoke is a YAML block; proven by four shipped spokes |
| Hardware sizing (VRAM/unified-memory → preset → base rung) | **Broken on macOS** | The one blocker; [plan](../../superpowers/plans/2026-09-05-mens-mac-training-enablement.md) |
| One SHA-pinned, Apache-2.0 hub base | Qwen3-32B | Upgrade path: Qwen3.8-27B, gated on VLM text-tower loading |
| Artifact ladder (`vox-quantize` + ADR-043) | Built, Metal-ready | Q4_K_M…Q8_0 → mesh nodes incl. the 4080 Super |
| Eval (`vox model eval-corpus`, `eval-gates-*.yaml`) | Built, never run against a live model | The measurement that keeps every petal honest |

**Petals** — each independently valuable, each deletable:

| Petal | State | Highest-value next move |
|---|---|---|
| `vox-lang` authoring | Mature | Re-raise `min_coverage_pct`, lowered to pass after the Sept-1 anti-cheat fixes |
| `rust` authoring | Built June | Flip `clippy_clean_rate` to blocking once a baseline exists |
| `tool-selection` / `argument-generation` | Built, corpora thin | All mix sources are `optional: true` placeholders except one |
| **Skill retrieval** | **Bespoke BM25, no semantics** | Best new petal — see §5r.1 |
| **Vision / screenshots** | Absent, and gated | See §5r.2 |
| `research`, `rocks`, `populi-meta` | Retired from training | Curation/retrieval only — correctly not petals |

**Not petals — stem hygiene that must stay deterministic code.** Two of the
capabilities described as candidates for this spoke should never be a
fine-tuned model, and both have a cheap fix instead:

- **Agent spawn/spin-down.** `ScalingService::decide_scaling` is already a
  tested, resource-gated elastic controller (scales up on weighted load, retires
  idle dynamic agents, halves retirement under cost alert, refuses to scale up
  past a CPU ceiling / below a memory floor). It should stay rule-based: it makes
  irreversible, safety-relevant decisions where an auditable threshold beats a
  stochastic judgment. **But its resource guard is inert in every shipped
  build** — `local_resources::snapshot()` compiles only under the `system-metrics`
  Cargo feature, which no consuming crate enables, so `local` is always `None`,
  `scale_up_blocked` is always `false`, and the CPU-ceiling/memory-floor knobs
  the GUI exposes configure a guard that never runs. Turning that feature on is
  the highest value-per-line finding in this entire audit, and it is not a
  training change at all. It also matters *specifically* on a 128 GB Mac where
  agents and a 32B training run compete for one memory pool.
- **Model/dispatch routing.** `auto_score_model` already scores the exact axes
  a learned router would need (cost, latency, quality, locality). The real defect
  is that two routers are live — a `FreeTierRouter` in `vox-research-shim`
  intercepts an entire request class before the canonical `decide()` runs. Fix by
  consolidation, not by adding intelligence on top of a scorer that is already
  numeric and auditable.

#### §5r.1 — Skill retrieval: the best new petal

Skill selection today is a from-scratch BM25 index inside
`vox-orchestrator-mcp` (own tokenizer, own IDF/TF), with **zero semantic
component** — which directly contradicts AGENTS.md's rule that richer skill
retrieval "MUST be built on the existing `vox-search` hybrid stack (tantivy +
semantic + RRF), not a bespoke skill index." That stack exists and
`vox-orchestrator-mcp` already depends on it. Separately, the tier-1 catalog
sorts by a `reliability_scores` join whose producer was never written — the
writer function is referenced only by a doc comment, so ordering is effectively
alphabetical in production.

This is the best fine-tuning target of the four surfaces because it is a
natural-language relevance judgment ("does this skill's description fit this
task"), it is advisory rather than safety-critical (a wrong suggestion costs one
turn), and the existing design already assumes an LLM does this reasoning. Two
cheap wins come *before* any model work: route skill search through `vox-search`,
and write the reliability producer.

#### §5r.2 — Vision: what it would actually cost

Three separable layers, and they are very different investments. The mistake to
avoid is treating them as one project.

**Layer 1 — load a VLM checkpoint's text tower (~100-250 LOC, low maintenance).**
`vox-hf-layout` hard-rejects any config carrying `vision_config` /
`image_token_id` / `ForConditionalGeneration`. But the unwrap helper it would
need (`qwen35_text_config`) *already exists and is called on the very next line*
after the rejection — the bail is simply upstream of it. Work: convert the bail
into a text-only load path, and skip `visual.*` / `vision_tower.*` tensors in
`hf_keymap`. **This buys the hub-model upgrade with no vision code at all** —
Qwen3.8-27B (Apache-2.0, best agentic scores of any locally-runnable open model)
becomes trainable as a text model. Highest value-per-line in the vision family
by a wide margin, and it is genuinely a text-stack change.

**Layer 2 — vision inference (~1,500-3,000 LOC in Candle, high ongoing
maintenance).** A real vision tower: patch embedding, ViT blocks, the
merger/projector, dynamic-resolution preprocessing, and mRoPE — the bail comment
names "no vision tower, no mRoPE, no MTP head" precisely because none of it
exists. Candle ships no Qwen-VL implementation, so this is hand-written, must be
validated against reference outputs, and must be re-done per model generation —
the same tax already being paid for the hand-written `Qwen35LinearAttention`.

**The lazy-correct alternative: don't implement it.** Vision inference is a
*consumption* problem, not a training problem. The mesh design already scopes
`LlamaCppRpc` and `OllamaSubprocess` backends, and `external_serving_handoff.rs`
already exists. Serving a VLM through llama.cpp (GGUF + mmproj) or MLX costs
roughly the plumbing to shell out and parse a response, and it **decouples the
petal from the stem entirely**: screenshots get understood by a separate,
un-fine-tuned vision model while the hub stays a text-only fine-tuned Qwen. The
router already supports multiple models; nothing requires the eyes and the brain
to be the same checkpoint.

**Layer 3 — vision fine-tuning (very high, not recommended).** Backward passes
through the vision tower, image-aware corpus/mix formats, VLM eval gates. Even
teams that do this usually freeze the vision tower and LoRA only the text side —
at which point Layer 1 plus a frozen tower gets most of the benefit.

**Highest-value vision surfaces, if pursued:** (1) screenshot verification for
the agent harness — the GUI already ships a browser plugin with CDP live view and
`vox_browser_*` MCP tools, so "did this render correctly / what is on this page"
has an existing capture path; (2) a design-iteration loop (screenshot → critique
→ code change), which is what "designing the future" most concretely means here;
(3) diagram/document ingestion for the corpus. All three are satisfied by Layer 1
+ an external vision runtime. **Recommended sequence: Layer 1 now (it pays for
itself in hub quality alone), external-runtime vision when a real screenshot
consumer exists, Layer 2 only if that consumer proves load-bearing and the
external hop becomes the bottleneck. Layer 3 probably never.**

## Revision 6 (2026-09-06) — The plugin path itself fixed; real serving confirmed; Qwen3.8 text-tower loading unblocked

Revision 5 said the documented `vox mens train`/`serve` path was unusable on
macOS and routed around it via the in-crate `vox-populi::inference` module
instead. That workaround is no longer necessary — the actual plugin path is
now fixed, tested, and verified end-to-end, including through the real
`/generate` HTTP endpoint `VoxLocalAdapter` (the GUI/orchestrator's
local-model client) speaks to. Four real bugs, found only by actually
driving the path to completion rather than reading the code:

1. **`vox-plugin-mens-candle-cuda`'s `Plugin.toml` had no macOS artifact at
   all** (`os = ["windows", "linux"]`, no `macos-aarch64` key) — confirmed
   `vox plugin install --path ...` fails outright. Fixed by adding the
   artifact entry; the crate already compiles CPU-only by default (`cuda` is
   an opt-in Cargo feature), so nothing about this required CUDA.
2. **`vox-cli`'s local `--path` install checksummed every declared
   platform's artifact unconditionally**, erroring when a legitimate
   single-platform local build (exactly what `cargo build --release` on this
   machine produces) didn't have the *other* platforms' files. Fixed to skip
   what isn't present, keeping the existing "current triple's artifact must
   exist" guard as the real requirement.
3. **`QLoraConfig::default()`'s BF16 compute dtype doesn't work on CPU at
   all** — Candle's CPU backend has no BF16 matmul kernel. Real symptom:
   `candle error: unsupported dtype BF16 for op matmul` on every `/generate`
   call. Fixed with a device-aware override (F32 on CPU, BF16 elsewhere,
   unit-tested).
4. **Dense Qwen3's per-head Q/K RMSNorm was missing from both the training
   and inference code paths** in `vox-plugin-mens-candle-cuda` itself — a
   second, independent occurrence of the exact bug class found and fixed in
   `vox-populi::inference` in Revision 5, in a *different* implementation of
   the same architecture. Symptom after fixing (3): the server ran with no
   error and produced fluent-looking garbage, not a crash — the same
   deceptive failure mode as before. Fixed the same way: `Option<RmsNorm>`
   fields, applied after projection and before RoPE, on both the training
   (`candle_qlora_train/mod.rs`) and inference (`inference.rs`) construction
   sites, proven with a same-underlying-weights A/B comparison test.

**Verified end-to-end, for real, over HTTP:** retrained a fresh adapter (the
Revision-5 one was trained without Q/K norm and would have been numerically
mismatched against a norm-applying forward), built `vox-ml-cli` with
`--features gpu,execution-api`, ran `vox mens serve --model
mens/runs/e2e-smoke`, and curled it:

```
POST /generate {"prompt": "What is the capital of France?", "temperature": 0.0}
→ "The capital of France is Paris. The capital of France is Paris. ..."
```

This is the actual documented golden path (`vox mens train` → `vox mens
serve`) working correctly on this machine, through the exact protocol the
orchestrator's `VoxLocalAdapter` consumes
(`crates/vox-orchestrator-mcp/src/llm_bridge/provider_adapter.rs` — its
`GenerateResponse` schema comment names `VoxLocalAdapter` as the consumer it
was built for) — not a bypass of it.

**`--device metal` remains unfixed and is a separate, larger project.**
Nothing in this revision touches it. Per Revision 4: `vox-plugin-mens-candle-metal`'s
`Plugin.toml` is already correct (declares `macos-aarch64` properly), but its
`run_train_step`/`run_eval_step` still return `unimplemented!()` pending "the
SP3-D training host protocol" — a real implementation gap, not a
manifest/config problem like the four bugs above. CPU training is real and
now confirmed correct; Metal acceleration on this Mac is not.

**Qwen3.8 "Minimal tier" (§3.2r) landed.** `vox-hf-layout`'s blanket
rejection of any `ForConditionalGeneration`/`vision_config` checkpoint is now
narrowed to only reject when there is no `text_config` block to extract a
text tower from — verified against Qwen3.8-27B's real `config.json` (fetched
this session), which has a `text_config` in the same hybrid-attention shape
`qwen35_text_config` already parses for Qwen3.5. Qwen3.8-27B is now loadable
as a text-only causal LM, no vision code involved. **Not yet addressed:** the
weight loader (`inference.rs`'s `get_tensor` closure) reads entire safetensors
shards into memory via `std::fs::read` before filtering by key — for a real
27B+vision checkpoint this is the genuinely slow, RAM-hungry path the
original bail's comment warned about ("~10 minutes force-loading a huge
multimodal embedding"). That is a memory-efficiency project (stream/mmap and
filter by key before materializing), separate from and larger than this
config-parsing fix, and was not attempted here. **Also not attempted:**
downloading the real Qwen3.8-27B checkpoint to prove this end-to-end the way
Qwen3-0.6B was proven above — it is tens of GB, and per policy that needs
explicit go-ahead before starting; the fix is verified today only against
Qwen3.8's real config shape in a unit test, not against its real weights.

**A concurrency note for whoever picks this up next.** Both follow-up chips
from this session's earlier revisions (`task_46c6b1f1` — the plugin/list.rs
clippy fix — and `task_1ca9e5f0` — bridging local MENS models to the GUI)
were independently started by the user as separate sessions while this one
was still running, which is almost certainly what caused a real mid-session
collision: this session's shared (non-worktree) checkout got reset to
`origin/main` by another process, silently discarding an uncommitted edit
(recovered by redoing the work in an isolated `git worktree`). **Both of
those follow-ups are now independently resolved in this session's own
worktrees** (`fix/items-after-test-module-lint` and the fixes described
above on `mens/mac-hub-enablement`). Check whether the separately-started
sessions produced overlapping or conflicting work before merging either
branch.

## Revision 5 (same day) — Real end-to-end text confirmed; three bugs fixed to get there; GUI/orchestrator path traced and blocked at a known point

**The confirmation the whole session was building toward, done for real.** With
the user's explicit permission, downloaded `Qwen/Qwen3-0.6B` (1.5 GB, real HF
weights, cached at `~/.cache/huggingface/hub/models--Qwen--Qwen3-0.6B/`),
quantized it with the now-delegated `vox quantize` CLI, and loaded it through
`vox-populi::inference::CandleCpuBackend` — the in-crate Mn-T2 inference path
(no plugin loading required, unlike `vox mens train`/`serve`, both of which
are blocked — see below). Result, verified on this machine:

```
Prompt:  "What is the capital of France?"
Output:  "The capital of France is Paris. The capital of France is Paris. ..."
```

Correct and coherent (the repetition is expected — greedy decoding,
temperature 0.0, no repetition penalty, on a 0.6B model). This is real: real
downloaded weights, real tokenizer, real quantization, real forward pass, on
this machine's CPU. The test is committed as an `#[ignore]`d manual
integration test
(`crates/vox-populi/src/inference/backends/candle_cpu.rs::real_qwen3_checkpoint_produces_real_text`),
run via `VOX_REAL_MODEL_DIR=<dir> cargo test ... -- --ignored --nocapture`.

**Getting here required finding and fixing three real, previously-latent
bugs** — the `vox-populi::inference` module's own header comment already
admitted "No numerical-parity claim is made here" and "tested for shapes +
finiteness only"; it had never been run against a real checkpoint before this
session:

1. **Wrong weight-key namespace for dense Qwen3** (`vox-hf-layout`). The
   prefix decision matched on the model name (`model_l.contains("qwen3")`),
   which is true for both "qwen3" and "qwen3_5" — but only `text_config`-
   wrapped checkpoints (Qwen3.5's hybrid stack) use the
   `model.language_model.layers` prefix. Dense Qwen3 uses the flat
   `model.layers` prefix, same as Qwen2/Llama/Mistral. Fixed by checking
   whether `text_config` is actually present, the same signal already used
   one line above for `cfg_source`. Commit `dd8dd2e91`.
2. **`q_norm`/`k_norm` silently quantized instead of kept F32**
   (`vox-quantize`). The tensor-role classifier's `.ends_with(".norm.weight")`
   check (dot) doesn't match dense Qwen3's `q_norm.weight`/`k_norm.weight`
   keys (underscore before `norm`), so these small, precision-sensitive
   per-head normalization vectors fell through to the default `Matrix` role
   and got quantized to Q8_0. Worst-case tensor MSE went from **1.44e-2 to
   1.06e-5** after the fix — three orders of magnitude — and this alone was
   the difference between fluent-looking garbage and a correct answer.
   Commit `ccb44c09e`.
3. **Q/K RMSNorm never applied at all** (`vox-populi`). Even with the
   weights correctly preserved, `FullAttention` had no `q_norm`/`k_norm`
   fields — the normalization step dense Qwen3 applies to Q and K right
   after projection, before RoPE, was simply missing from the forward pass.
   Added as `Option<RmsNorm>` fields (absent on Qwen2.5-style checkpoints),
   proven with a new test that confirms providing the weights changes the
   output (a same-random-base-weights A/B comparison, not just "doesn't
   crash"). Commit `7556b990c`.

**Why the documented `vox mens train`/`vox mens serve` path was not used
instead:** both route through a dynamically-loaded `MlBackend` plugin
(`mens-candle-cuda`), and that plugin's `Plugin.toml` declares
`os = ["windows", "linux"]` with a hard `native-libs` requirement on
`cudart`/`cublas` — it cannot be installed on macOS at all, CPU-fallback or
not, independent of the Metal-specific gap in Revision 4. Confirmed directly:
`vox plugin install --path .../vox-plugin-mens-candle-cuda --yes` fails with
"plugin 'mens-candle-cuda' declares no artifact for 'macos-aarch64'." (The
`vox-plugin-mens-candle-metal` plugin, by contrast, *does* declare a correct
`macos-aarch64` artifact — but its training methods still return
`unimplemented!()` pending the SP3-D protocol per Revision 4, so installing it
would not have helped either.) The in-crate `vox-populi::inference` path used
above needs no plugin at all, which is exactly why it was the way through.

**GUI/orchestrator path, traced and honestly reported, not built further this
session.** The orchestrator side is real and correctly wired:
`MensCatalog::refresh` (`crates/vox-orchestrator/src/catalog.rs:583-646`)
already scans `mens/runs/<name>/` for a `final`/`checkpoint-*` subdirectory
and registers it as a routable `mens/<name>` model with
`provider_type: VoxLocal`; `VoxLocalAdapter`
(`crates/vox-orchestrator-mcp/src/llm_bridge/provider_adapter.rs:230-336`) is
a real HTTP client that probes health and POSTs to a `/generate` endpoint.
**Both are correct and would work today** — but they expect a `vox mens
serve` process already running, and that process loads its model through the
same blocked `mens-candle-cuda` plugin path described above. Confirming the
GUI chatbot against a real local Mac-trained model therefore needs one of:
(a) fixing the plugin manifest/native-lib declaration so a CPU-only build can
install and run on macOS, (b) finishing the Metal plugin's training methods
(Revision 4's tracked gap), or (c) building a small HTTP server that speaks
`VoxLocalAdapter`'s `/generate` protocol on top of the now-proven-working
`vox-populi::inference::CandleCpuBackend` path — the cheapest of the three,
since everything under it is now verified correct. None of these were
attempted this session; each is a real, separately-scoped project, not a
config change.

## Revision 4 (same day) — Metal training is not actually wired to `vox mens train`

**Critical correction to Revisions 2 and 3.** Both said "Apple Silicon training
already exists" based on `vox-plugin-mens-candle-metal` containing a full,
tested QLoRA training port. That plugin's code and unit tests are real, but
**they are not reachable from the actual `vox mens train` command a user
would run.** Confirmed directly this session:

```
$ vox mens train --device metal ...
error: `--device metal` for Candle QLoRA is not supported yet: there is no
Metal-enabled Candle training backend in this build.
```

The rejection is deliberate and self-documenting —
[`crates/vox-ml-cli/src/commands/schola/train/run_train.rs:150-179`](../../../crates/vox-ml-cli/src/commands/schola/train/run_train.rs):
`vox-populi`'s own in-crate trainer (feature `mens-candle-qlora-cuda`) has "no
Metal twin" (no `candle-core/metal` wiring in `vox-populi`'s Cargo.toml at
all), and while `vox-plugin-mens-candle-metal` does exist, it needs "the SP3-D
training host protocol" — the abi_stable plugin-loading bridge — which is not
wired into this CLI command. This is the same "built but not consumed by
anything live" pattern found repeatedly this session (`DomainRouter`,
`ScalingService`'s resource guard) — the difference here is that this one
directly blocks the user's stated goal ("training on Mac OS for the training
and its GPU, taking full advantage of 128 gigabytes").

**Consequence for this session's end-to-end confirmation:** the real
training-and-serving smoke test below runs on **CPU**, via the
`qwen3_dev_cpu` preset (rank 8, seq_len 128, batch 1, 1 epoch — literally
built for this purpose, per its own comment: "smoke only — no quality gate").
This proves the pipe end-to-end with real weights and real text, but it does
**not** use the Mac's GPU. Wiring Metal into `vox mens train` is now the
single highest-value follow-up in this entire audit relative to the
originally stated goal — flagged as a follow-up task (see spawn_task in this
session), scoped as its own project rather than folded into this one, because
bridging the abi_stable plugin host into the CLI's training dispatch is
real, non-trivial integration work, not a config change.

**What this changes about §3.1's hub/spoke sizing.** The base-model ladder
(Task 4, this session) correctly resolves `Qwen3-32B` for a 128 GB Mac — that
part is real and tested. But *training* it today would still route through
whatever Candle backend is actually wired, which for CPU-tier presets is CPU,
not Metal — meaning the Mac cannot yet exercise the 32B rung's un-quantized
LoRA method in practice at anything resembling reasonable speed. The ladder is
correctly *sized* for the hardware; the *execution backend* has not caught up
to it yet.

## Revision 3 (same day) — Qwen3.8, vision as three petals, quantization laning, and a corrected quantization finding

Three corrections to Revision 2, then new ground: Qwen3.8 adoption tiers, vision
split precisely into the three capabilities actually asked for, a single-SSOT
statement for cross-machine training, a future cloud-dispatch note, and a local
execution ceiling.

**Correction to §5r's quantization finding.** Revision 2 said no CLI wraps
`vox-quantize`. That was wrong — a full search only checked struct-variant
syntax (`Quantize { ... }`) and missed the actual tuple-variant registration.
`vox-ml-cli quantize --input <dir> --output <dir> --to q4_k_m|q5_k_m|q6_k|q8_0
--device auto|cuda|metal|cpu` ([`crates/vox-ml-cli/src/commands/quantize.rs`](../../../crates/vox-ml-cli/src/commands/quantize.rs))
is complete: mixture selection, device auto-detect (tries CUDA, then Metal,
then CPU), round-trip verification, JSON or table output. It already operates
on **any** local SafeTensors directory — hub output, spoke output, a merged
QLoRA checkpoint, or a raw downloaded HF checkpoint — which is exactly
"quantize freely and dynamically, not hardcoded." **The actual gap was one
line**: `vox quantize ...` (the user-facing `vox` binary) did not delegate to
`vox-ml-cli`, so this fully-built command was unreachable except by invoking
the `vox-ml-cli` binary directly. Fixed in this session
(`crates/vox-cli/src/main.rs`'s `is_ml` delegation match).

### Qwen3.8 adoption: three tiers, honestly costed

Charter reminder (§3.2r): the text QLoRA trainer hard-rejects any checkpoint
carrying `vision_config` / `image_token_id` / `ForConditionalGeneration`.
Qwen3.8-27B is a native VLM, so every tier below is gated on at least
partially defeating that gate.

| Tier | What it buys | What it costs | Recommendation |
|---|---|---|---|
| **Minimal** | Load Qwen3.8-27B's **text tower only** for inference and QLoRA fine-tuning — no vision capability, just a better/newer text model | ~100-250 LOC: convert the VLM bail in `vox-hf-layout` into a text-only load path (the unwrap helper, `qwen35_text_config`, already exists and runs on the very next line); skip `visual.*`/`vision_tower.*` keys in `hf_keymap` | **Do this first, regardless of vision.** Pays for itself in raw hub quality — best agentic scores of any locally-runnable open model — with no vision code at all. |
| **Moderate** | Above, plus: a new capability tag (e.g. `agentic_flagship`) in `gpu-specs.yaml`'s `train_bases`, sized for the Mac's usable memory; quantized rungs produced via the now-delegating `vox quantize`; a real `vox model eval-corpus` run comparing it against the pinned dense Qwen3-32B before any base-model change is made | Small-medium: config + one eval run, still zero vision code | **Do this once Minimal lands**, to actually answer "is 3.8 worth switching to" with data instead of a benchmark table, per the hub-and-spoke doc's own rule not to lock a base model without live re-verification |
| **Full** | Above, plus a real vision tower: patch embedding, ViT blocks, the merger/projector, dynamic-resolution preprocessing, mRoPE — native in-model screenshot/image understanding | ~1,500-3,000 LOC in Candle, hand-written (no Qwen-VL in Candle upstream), re-paid per model generation, validated against reference outputs | **Not recommended now.** See below — the three vision goals are satisfied by Minimal/Moderate plus an external vision runtime, at a fraction of the cost. Revisit only if the external-hop latency or quality genuinely becomes the bottleneck for a real, in-use consumer. |

### Vision: three genuinely different petals, not one

The three capabilities named — analyze screenshots, design websites like Claude
Design, generate images — are architecturally unrelated. Treating them as one
"vision" project is the mistake to avoid; each has its own cost and its own
honest priority.

**Petal 1 — screenshot/website understanding (image → text).** This is a VLM
*inference* problem, not a training problem. Two paths, in cost order:
- **External vision runtime (recommended, ~zero new model code).** The mesh
  design already scopes `LlamaCppRpcBackend` / `OllamaSubprocessBackend`
  ([`crates/vox-populi/src/inference/backends/`](../../../crates/vox-populi/src/inference/backends/) —
  confirmed built this session, with tests, though not yet consumed by a live
  server) and `external_serving_handoff.rs` already exists in both Candle
  plugins. Point either at an existing open vision-capable GGUF/MLX model
  (Qwen3.8-27B's own GGUF+mmproj once published, or a smaller dedicated
  vision model for latency) and get screenshot description today. The hub
  stays text-only and fine-tuned; the eyes are a separate, un-fine-tuned
  model. Nothing requires them to be the same checkpoint.
- **Native (Qwen3.8 Full tier, above).** Only worth it if the external hop's
  latency or an extra network/subprocess round-trip genuinely blocks a real
  workflow. Start with the external hop; it is strictly cheaper to build and
  already has a capture path — the GUI ships a browser plugin with CDP live
  view and `vox_browser_*` MCP tools.

**Petal 2 — "design websites like Claude Design."** This is **not a vision
generation task** — it needs zero new model capability beyond Petal 1. It's an
agentic loop wiring three things this codebase already has: (a) code generation
(the `vox-lang`/`rust` spokes, or the base hub model, writing HTML/CSS/Vox-GUI
components), (b) a screenshot (the existing browser plugin), (c) critique (feed
the screenshot to Petal 1's vision model, get feedback, iterate). The build
here is a harness/orchestration feature — a generate → screenshot → critique →
revise loop — not a training feature. It should be scoped as its own small
plan once Petal 1's external hop exists, and it has nothing to do with the
`domain-profiles.yaml` spoke SSOT at all.

**Petal 3 — image generation (text → image).** A genuinely separate model
family: diffusion (Stable Diffusion / FLUX-class), not an autoregressive
transformer, sharing no architecture, weights, or training infrastructure with
Qwen or Candle's existing stack. This is not a Qwen hub capability at any
tier — it would be its own external-tool integration (a local diffusion
runtime as a subprocess, mirroring the already-scoped Ollama-subprocess
pattern, or a hosted API call). Lowest priority of the three relative to the
stated goal of building a development harness: it is a creative-output feature
with no code-quality or agentic-capability payoff. Worth having eventually as
an independent petal; it should not gate or entangle with anything else here.

### One SSOT, regardless of which machine trains

This was already true structurally and is now true in practice. `mens/config/
domain-profiles.yaml`'s `hub:` key pins one model id
(`Qwen/Qwen3-8B@<sha>` today); every spoke's `base.model` is a capability tag,
never a machine name. `spoke_base_resolver::pick_base` resolves that tag
against **whatever memory figure the host reports** — `nvidia-smi` on the 4080
Super, unified memory minus the GUI reserve on the Mac (Revision 2's fix,
implemented and tested this session). The same `domain-profiles.yaml` file,
unedited, trains `rust` at Qwen3-8B on the 4080 Super and at Qwen3-32B-LoRA on
the Mac, because the ladder — not the file — encodes the hardware difference.
This is what "single source of truth whether training on the 4080 Super or
this new system" already means in this codebase; nothing about it is
machine-specific by construction.

### Cloud dispatch: a future petal, and a premise worth checking

`mens/cloud/` (RunPod, Vast, local provider catalog with cloud-burst budgeting)
already exists as a training-only overflow path — this is not new territory.
But the premise "we're limited by training more than execution" is worth
re-measuring now, not assumed: before this session, the *only* trainer was the
4080 Super at a 32B ceiling (QLoRA) with no Metal path at all. As of this
session, Metal training is real (`vox-plugin-mens-candle-metal`, a full QLoRA
port) and the Mac's ladder reaches an un-quantized 32B LoRA rung. **Local
training capacity just grew by roughly an order of magnitude in resident
budget** (16 GB → ~116 GB usable), which may have already resolved the
constraint the premise describes. Recommend deferring cloud-dispatch work
until Task 4 Step 5's measured calibration run shows where the *actual*
local ceiling sits — cloud burst is the right tool for "bigger than 128 GB can
hold," not for anything this machine can already do.

### The biggest model executable locally, and what it implies

Training and *inference* have different ceilings, because inference needs no
optimizer state, no gradients, and far less activation memory — only resident
weights plus a KV cache. Using the quantization ladder's own arithmetic
(`Q4_K_M` ≈ 0.55 GiB/B parameter, per `vox-quantize`'s k-quant mixture): 116 GiB
usable ÷ 0.55 GiB/B ≈ **~210B dense parameters**, or considerably more for an
MoE architecture where only active experts need to be hot in practice-tuned
serving engines (llama.cpp, MLX) even though all experts must be resident for
a naive loader. That ceiling is far above anything Candle in this workspace
loads today (`Qwen35LinearAttention` is hand-written for the Qwen3.5-family
architecture only) — reaching it means **inference through an external
runtime** (llama.cpp GGUF or MLX), not the in-tree Candle path, and very likely
a **different hub entirely for serving than for training**: e.g. a heavily
quantized 70B-class dense model, or a larger open MoE (DeepSeek-V4-class,
MIT-licensed) served for high-quality *chat/research* use, while the
fine-tuned fast-iterating hub (Qwen3-32B, Candle-native) stays the target for
spoke training. This is exactly the shape the mesh design already anticipated
(`LlamaCppRpc`/`OllamaSubprocess` backends, §Petal-1 above) — worth keeping as
a named future direction, not a decision to make now, since it depends on
Task 4 Step 5's real measured numbers rather than the arithmetic ceiling here.

### §6r — Corrected action list

Supersedes §6 below. Items 2-4 of §6 were written without knowing the SSOT
existed; the real sequence is in
[`docs/superpowers/plans/2026-09-05-mens-mac-training-enablement.md`](../../superpowers/plans/2026-09-05-mens-mac-training-enablement.md):

1. macOS unified-memory detection with a GUI reserve (unblocks everything).
2. Metal-aware preset auto-selection onto the existing `qwen3_*` ladder.
3. Fail-closed `base.preset` validation against `KNOWN_PRESETS` — today a typo
   in a spoke's preset silently falls through `base_for_name`'s catch-all to a
   plausible-looking `rank: 16` default. This is what makes "easy to add or
   remove spokes" safe rather than merely easy.
4. A Mac-tier rung on the `train_bases` ladder, calibrated from a measured run.

The quantization ladder (§3.3) and the `vox-quantize`/ADR-043 findings (§1) are
unchanged and remain correct.

---

## 0. What was asked, and the short answer

Audit everything in this repo about GPU/ML/Qwen/Candle/Burn, and — now that the
operator's daily driver is a 128GB-unified-memory Mac rather than solely the RTX
4080 Super — pick a base-model hub and a quantization pipeline sized for both,
grounded in real Sept-2026 hardware-ownership data.

**Short answer: almost none of this needs building.** The pipeline the user is
asking for already exists (`vox-quantize` + [ADR-043](adr-043-quantized-safetensors-ondisk-format.md)),
the Metal training/inference backend already exists
(`vox-plugin-mens-candle-metal`, a full QLoRA port), and the base-model choice
was already made three times over the last four months, most recently to plain
dense **Qwen3** (not 2.5, not 3.5). The actual decision this doc makes is a
**topology** one: the Mac becomes the hub-and-spoke **hub**, the 4080 Super
becomes a **spoke**, and one concrete gap gets named — a unified-memory safety
reserve for training/quantizing while the GUI is running, since Apple Silicon
has no equivalent of "VRAM that isn't the desktop's memory."

## 1. What already exists (do not rebuild)

| Ask | Already built | Where |
|---|---|---|
| "A pipeline capable of quantizing it at various sizes" | `vox-quantize`: Q4_K_M / Q5_K_M / Q6_K / Q8_0 k-quant mixtures, tensor-role-aware (embeddings/output kept at Q6_K even under Q4_K_M; norms/biases kept F32), GPU-first with CPU fallback, **Metal already wired** (`Device::new_metal(0)` is a first-class `DevicePref`, tried automatically after CUDA in `Auto` mode) | [`crates/vox-quantize/src/{policy,engine,device}.rs`](../../../crates/vox-quantize/) |
| "Quantized at various sizes, SafeTensors-native (charter §0.2.3)" | Already decided: quantized tensors are 1-D `u8` SafeTensors carrying raw GGML block bytes + a `quant-metadata.json` sidecar. No GGUF on disk. | [ADR-043](adr-043-quantized-safetensors-ondisk-format.md) |
| "Runs the GUI + does ML on the Mac" | `vox-plugin-mens-candle-metal` is not a stub — it is a byte-for-byte-structured **full QLoRA training port** from the CUDA plugin (`candle_qlora_train/`, ported "SP3 sub-batch C"), plus inference, checkpointing, merge, and external-serving-handoff, gated behind an opt-in `metal` Cargo feature | [`crates/vox-plugin-mens-candle-metal/`](../../../crates/vox-plugin-mens-candle-metal/) |
| "Choose a hub[-and-spoke topology]" | VoxMens is already "a nascent hub-and-spoke" (per-domain mix configs, `lane`-tagged records, a proposed `spokes.yaml` SSOT) — the vocabulary and half the plumbing exist | [voxmens-hub-and-spoke-ssot-research-2026-06-18.md](voxmens-hub-and-spoke-ssot-research-2026-06-18.md), [voxmens-serving-topology-decision-2026-06-19.md](voxmens-serving-topology-decision-2026-06-19.md) |
| "Choose a [base] hub[/model]" | Already pivoted twice since the 4080 incident (Qwen2.5-Coder → Qwen3.5 → present-day dense **Qwen3** ladder, `DEFAULT_MODEL_ID = "Qwen/Qwen3-8B@<pinned sha>"`, commits described in history as "Qwen3-everywhere" and "agentic_default = user decision") | `crates/vox-populi/src/mens/mod.rs:46`, `memory_budget.rs:56-64,275-289` |
| A way to actually measure "is this base/quant choice good" | Just landed, never run: `vox model eval-corpus` against the 164-fixture `humaneval-vox` corpus (31 held-out), identical harness for MENS and any frontier model | [vox-efficacy-benchmark-execution-handoff-2026-09-04.md](vox-efficacy-benchmark-execution-handoff-2026-09-04.md) |

The one thing genuinely **not** built: nothing consumes any of this from the
mesh's `InferenceBackend` dispatcher yet. `Mn-T13` (Apple Silicon path via
Candle-Metal) in the [distributed-training plan](mesh-mens-distributed-training-and-execution-plan-2026.md)
is still listed as a stub in that doc's own tracking table, even though the
plugin it would wrap has since been fully built. That doc is from 2026-05-09 —
four months stale on this specific point — and its blanket charter line
("Apple Silicon training explicitly out of scope for v0.6/v0.7/v1.0") is
**superseded by code**: the Metal QLoRA trainer exists today. Treat that
charter line as historical, not current.

## 2. Real hardware data (Sept 2026), not vibes

**The 4080 Super, unchanged:** 16 GB GDDR6X, 736 GB/s bandwidth, 10,240 CUDA
cores — the exact card `memory_budget.rs`'s constants were calibrated against.

**Steam Hardware Survey, Jul–Aug 2026** (largest available real-world consumer
GPU sample): NVIDIA holds 72.8% overall GPU share. VRAM tiers: **16 GB is now
the single largest tier at ~25.9%**, just ahead of 8 GB at ~25.3% (the July
2026 survey was the first time 16 GB overtook 8 GB); 12 GB sits at ~12.9%; 24
GB at ~5.4%; 51.4% of surveyed gaming PCs now carry ≥10 GB. **The 4080 Super's
own tier (16 GB) is the modal consumer tier, not a niche one** — quantizing for
it is quantizing for the largest single segment that exists.

**Apple Silicon, Sept 2026 refresh (ships 2026-09-22):** Mac mini M6, 32 GB
max, $899; Mac Studio M5 Pro, 64 GB max, $1,699; **Mac Studio M5 Max, 128 GB
max, $2,499** — the operator's class of machine; Mac Studio M5 Ultra, up to
512 GB (the 512 GB configuration ships October). Because CPU/GPU/Neural Engine
share one memory pool, a 128 GB Mac can hold a model that would need a
discrete 128 GB-VRAM card (i.e., nothing consumer-grade) to match — this is
the entire reason "buy the memory, not the machine" is the 2026 local-LLM
buying advice, and it is why the Mac, not the 4080 Super, is now the larger
compute surface in this two-machine mesh.

**The Qwen family, Sept 2026, so the model choice isn't guessed:**

| Line | Status | Notable sizes |
|---|---|---|
| Qwen3 (dense, pre-3.5) | Open, Apache-2.0 | 0.6B/8B/14B/32B/30B-A3B/235B-A22B — **this is what the repo currently pins** |
| Qwen3.5 (Feb–Mar 2026) | Open | 0.8B/2B/4B/9B, 27B, 35B-A3B, 122B-A10B, 397B-A17B |
| Qwen3.6 (Apr 2026) | Open | 27B, 35B-A3B |
| **Qwen3.7 (May 2026)** | **Closed, API-only, no weights of any kind** | Max-Preview / Plus-Preview — confirmed by this repo's own June-2026 research; irrelevant as a local target |
| Qwen3.8 (Aug 2026) | Open, most capable to date | 27B dense (262K context, described as running on "a laptop-class GPU"), 2.4T-A95B MoE (~95B active) |

`unsloth` publishes Dynamic-quant GGUF and MLX conversions for the whole 3.5/3.6/3.8
line same-day-ish (e.g. `unsloth/Qwen3.6-35B-A3B-UD-MLX-4bit`); nothing "3.7"
exists there because there is nothing to quantize. On the Rust-native inference
side, `candle-core`'s GGUF/quantized module already speaks the same k-quant
scheme Vox's own `vox-quantize` writes, and `mistral.rs` (not adopted here,
but worth knowing) proves ISQ-style quantization is a solved problem across
CPU/CUDA/Metal in Rust generally.

## 3. Decision

### 3.1 Topology: the Mac is the hub, the 4080 Super is a spoke

This is not a new invention — it's naming what the hardware already forces.
The Mac now has ~8× the 4080 Super's usable memory and can run the GUI,
orchestrator, and dashboard *and* hold a much bigger model resident at the
same time (see §4 for the safety margin that makes that true rather than
aspirational). Concretely:

- **Hub (Mac, 128 GB unified memory):** produces the canonical BF16
  SafeTensors checkpoint (via `vox-plugin-mens-candle-metal`, `metal`
  feature), runs the full `vox-quantize` ladder locally, hosts the GUI/
  dashboard, and is the natural place to run `vox model eval-corpus` sweeps
  before anything ships to a spoke.
- **Spoke (RTX 4080 Super, 16 GB):** stays exactly what it already is —
  the `qwen_4080_16g` canonical training/inference preset, consuming a
  quantized rung pulled from the hub rather than re-deriving it, per the
  existing `WorkerDonationPolicy` / mesh-inventory design in the
  distributed-training plan.

This is a direct, low-risk application of the hub-and-spoke SSOT research's
own recommended shape (§1.4 there: `base.model_id` / `method` / `preset` per
spoke, validated against registries) — it does not require writing
`spokes.yaml` today to be true; it just needs the role assignment stated,
which this document does.

### 3.2 Base model: keep dense Qwen3, don't re-litigate it today

The repo pivoted to dense Qwen3 (`DEFAULT_MODEL_ID = Qwen/Qwen3-8B`, ladder
0.6B/8B/14B/32B) very recently and — per the commit history — deliberately
("agentic_default = user decision"). This document does **not** override
that. Two things are worth surfacing rather than silently dropping, because
they came from this repo's own prior research and are now more affordable
than when they were written:

1. **An unresolved tension exists in the repo's own history.** The
   2026-05-15 model-selection refresh recommended migrating MENS's base from
   Llama-4 Scout to **Qwen3.6-27B** specifically (+5 pts SWE-bench,
   Apache-2.0, 128K context) — a different conclusion than the current
   dense-Qwen3 pin. Nothing in what this session read explains which
   consideration won out, or whether Qwen3.6-27B was tried and rejected.
2. **The Mac's headroom makes the top rung of the *existing* ladder (32B)
   trivial to hold resident** (`RESIDENT_GIB_PER_B_PARAMS ≈ 3.5` puts 32B at
   ~113 GiB, technically over a naive 128 GiB budget once macOS + GUI
   overhead is subtracted — see §4.1's arithmetic, which lands 32B just
   inside a realistic budget, and safely inside once quantized). Before this
   week, 32B only ever ran on 24GB+ cloud burst.

**Recommendation, not a mandate:** now that `vox model eval-corpus` actually
exists and has never been run against a real model (per the 2026-09-04
handoff), the highest-leverage next step is to run it — dense Qwen3-32B vs.
Qwen3.6-27B vs. Qwen3.6-35B-A3B (MoE, only ~3B active params, cheap compute
for its size, already has official `unsloth` GGUF+MLX quants if a quick
apples-to-apples run against pre-made quants is wanted) — on the Mac, where
holding all three resident in turn is affordable for the first time. That is
a measurement task, not a doc-review task, and this document deliberately
does not guess its outcome. Until it's run, dense Qwen3 stays canonical
because it's the most recent explicit decision on record.

### 3.3 Quantization ladder: map the existing pipeline to real hardware tiers

No new quantization code — just a stated target for each `vox-quantize`
mixture, so "producing rungs" has a concrete purpose instead of being quantize-
for-its-own-sake:

| `vox-quantize` mixture | Approx. bits/weight | Target hardware tier | Real-world share |
|---|---|---|---|
| `Q4_K_M` | ~4.5 | 8 GB VRAM tier; entry Apple Silicon (16-24 GB unified) | 8 GB: ~25.3% of Steam GPUs (largest legacy tier) |
| `Q5_K_M` | ~5.5 | 12 GB VRAM tier | ~12.9% |
| `Q6_K` | ~6.5 | **16 GB VRAM tier — the 4080 Super's own tier** | ~25.9%, now the single largest tier |
| `Q8_0` | ~8.5 | 24 GB+ VRAM tier (4090/3090/5080); 32-64 GB Macs | ~5.4% (24 GB) |
| BF16 canonical (unquantized) | 16 | 128 GB+ Macs (the hub); cloud-burst H100/A100 | the hub's own tier |

The 4080 Super's rung (`Q6_K`) and the largest single real-world consumer tier
(16 GB, `Q6_K`/`Q5_K_M` boundary) are the same rung — quantizing for "your own
card" and "the modal Steam GPU" is the same work, not a tradeoff.

## 4. The one real gap: unified-memory safety while the GUI runs

Every VRAM-safety mechanism in this codebase (`VOX_MENS_VRAM_SAFETY`,
`RESIDENT_GIB_PER_B_PARAMS`, the whole `memory_budget.rs` calibration) was
built against **discrete VRAM that CUDA's desktop compositor never touches**.
On the 4080 Super, the GUI (Windows desktop, or nothing at all during a
headless training run) lives in system RAM; the 16 GB of GDDR6X is the
model's alone. That assumption is false on Apple Silicon: the GUI (Tauri
window, WindowServer, Metal compositor, the rest of macOS) and the model
share **the same physical memory pool**. This is a pitfall class the 4080
Super history never had to teach us, because it structurally couldn't occur
there.

### 4.1 Concrete numbers

128 GB Mac Studio M5 Max, budgeting for the GUI running concurrently:

- macOS + WindowServer + background daemons: realistically 8-12 GiB resident
  even idle, more with the Tauri GUI, a browser, and other apps open.
- `vox-plugin-mens-candle-metal` resident footprint for a candidate base,
  using the same `RESIDENT_GIB_PER_B_PARAMS ≈ 3.5` heuristic the CUDA plugin
  was calibrated with (untested on Metal — see caveat below): 8B ≈ 28 GiB,
  14B ≈ 49 GiB, 32B ≈ 113 GiB.
- **32B dense at BF16/QLoRA does not comfortably fit** a 128 GB machine once
  10-15 GiB is reserved for the OS/GUI and some allocator slack is added —
  it is right at the edge, the same "OOMed even at seq 128" failure mode the
  4080 Super hit at 4B, just at a different scale. **32B is the ceiling to
  quantize *to*, not necessarily the rung to *train* on the Mac.** 8B and 14B
  have real headroom; 32B training should be treated as borderline until
  measured, exactly the caution the 2026-06-07 audit applied to 4B-on-16GB.

### 4.2 What's missing, concretely

Neither `memory_budget.rs` nor the Metal plugin has a macOS-specific reserve
today (confirmed: no `UNIFIED_MEM`/Metal-specific safety constant exists
anywhere in either crate). The CUDA `VOX_MENS_VRAM_SAFETY` pattern is a
*fraction* (default 0.88) — appropriate when the only consumer of the
fraction lost is CUDA context/allocator overhead. On unified memory the right
model is a *fixed floor reserve* (the OS/GUI's footprint doesn't scale with
model size) layered under the existing fractional headroom, e.g. a
`VOX_MENS_UNIFIED_MEM_RESERVE_GIB` (proposed default ~12 GiB) subtracted from
total unified memory *before* `memory_budget.rs`'s existing math runs. This
is a small, additive change — a new constant and one subtraction, not new
architecture — and it should ship with a measured calibration run (train/
quantize on the Mac with the GUI open, watch actual peak resident vs. the
`Activity Monitor`/`vm_stat` "wired" figure) the same way the 4080's
constants were calibrated from an actual OOM, not a guess.

### 4.3 Also worth closing, lower priority

- **Wire `Mn-T13`.** The Metal plugin is built; the mesh's `InferenceBackend`
  dispatcher (Mn-T2) doesn't consume it yet per the distributed-training
  plan's own tracking table. Closing this is what makes the Mac a first-class
  mesh hub node instead of a manually-invoked CLI host.
- **MLX stays opt-in, not default, and not urgent.** The distributed-training
  plan already scoped this correctly ("Apple Silicon path via Candle-Metal
  default + mlx opt-in") — Vox's own mesh nodes don't need MLX, since
  Candle-Metal already reads the same ADR-043 quantized SafeTensors artifacts
  the CUDA spoke does. Only build an MLX export if a real requirement shows
  up for interop with non-Vox tooling (LM Studio, Ollama's MLX backend) —
  YAGNI until then.

## 5. What this document is not deciding

- **Not** re-opening Burn vs. Candle. Burn was audited and fully removed
  2026-05-08 ([burn-necessity-audit-2026-05-08.md](burn-necessity-audit-2026-05-08.md));
  nothing here revisits that, and the charter's "no new ML frameworks" rule
  still holds.
- **Not** picking Qwen3.6/3.8 over the pinned dense Qwen3 — see §3.2. That's
  an eval-run decision, not a doc-review one, and the tooling to run it
  (`vox model eval-corpus`) only became usable this week.
- **Not** building GGUF export or an MLX converter today — see §4.3.

## 6. Immediate action items, ranked

1. **(Free, do now)** Fixed the stale `where-things-live.md` line claiming
   `vox-populi` still does "Burn / Candle QLoRA" — it's Candle-only since
   2026-05-08.
2. **(Small)** Add `VOX_MENS_UNIFIED_MEM_RESERVE_GIB` to the Metal-side
   memory budgeting, calibrated from one real measured run on this machine
   (mirrors how every existing CUDA constant was calibrated).
3. **(Small-medium)** Run `vox model eval-corpus` on the Mac against Qwen3-8B/
   14B/32B (dense, already pinned) to get a real, current baseline before any
   base-model debate — this is the single highest-leverage thing named in the
   2026-09-04 handoff, and it directly informs §3.2.
4. **(Medium)** Wire `Mn-T13` (CandleMetal into the `InferenceBackend`
   dispatcher) so the Mac participates in mesh inventory/routing as a hub,
   not just a local CLI target.
