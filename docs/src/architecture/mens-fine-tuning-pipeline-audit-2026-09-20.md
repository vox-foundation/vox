---
title: "MENS fine-tuning pipeline audit (2026-09-20)"
description: "End-to-end audit of the MENS fine-tuning pipeline — training data read row-by-row, trainer, eval gates, remote tuning — with per-spoke corpus improvement plans."
category: "Architecture SSOTs"
status: "research"
---

# MENS fine-tuning pipeline audit (2026-09-20)

Scope: every file fed to `vox mens train`, the mixer and generators that produce them, the
candle QLoRA trainer, eval gates, and the remote (Vast/RunPod) path. Method: `vox graph`
(refreshed at `877406d84`) for structure, then the data itself was read and measured with
`jq`, n-gram overlap, and `vox check` on sampled responses. Nothing was trained or spent.

**Bottom line.** Model size is not the binding constraint. The trainer currently discards
7/8 of its gradients, the system prompt teaches retired syntax, three of four spokes have no
usable data, and the one corpus that exists is 63% docs prose with mostly-ignored weights.
Fix the trainer and data first (cheap), then scale via a cloud PEFT trainer (~$6–10 per 32B run).

## 1. Critical trainer defects (verified in code)

| # | Defect | Where | Effect |
|---|---|---|---|
| T1 | Non-final grad-accum micro-steps do `let _ = scaled_loss.backward();` — the `GradStore` is dropped; candle does not accumulate into `Var`s | `patches/qlora-rs-1.0.5/src/training.rs:757,776` | With `grad_accum: 8`, only 1 row in 8 trains, and its loss is also ÷8 |
| T2 | Built-in system prompt says "Return type ALWAYS uses `->` arrow. Never use `to`"; a test pins it. `scripts/vox_system_prompt.txt` does not exist; the curated `mens/config/system_prompt.txt` is never loaded | `crates/vox-corpus/src/training/mod.rs:73,108` | Every training row is conditioned on wrong syntax |
| T3 | `vox ai serve` sends `"{system}\n\n{prompt}"` (no ChatML, no assistant opener) and chat completions flatten to `role: text` lines; eval-local *does* build ChatML but adds a directive and an empty `<think>` block training never contains | `inference.rs` `assemble_prompt`, `serve/handlers.rs` `flatten_chat_messages`, `eval_local_prompt.rs` | Served and gated prompts differ from the training distribution |
| T4 | `input` field never deserialized (`instruction`→prompt, `output`→response only) | `crates/vox-tensor/src/data.rs:57-85` | All 1000 research rows train without their evidence/question |
| T5 | `ce_last_k: 64` (rust, tool-selection, argument-generation, CLI default) trains only the last 64 tokens | `forward.rs:38-55` | Spokes learn closing braces, never the start of an answer |
| T6 | Truncation keeps the tail; the ~2.4k-char system prompt goes first; `qwen3_16g` seq_len is 512 | `encoding.rs:47-50` | Long rows lose their prompt entirely → unconditioned completions |
| T7 | Curriculum = stable sort by difficulty, no shuffle, same order every epoch; missing difficulty ⇒ 5; cosine LR planned over rows that curriculum skips | `checkpoint.rs:108-112`, `mod.rs:529` | Source-blocked batches; LR never finishes decaying |
| T8 | Validation split is random after response-agnostic dedup; 738/966 vox-lang rows share a response with another row | `mod.rs:462-475`, `mix/mod.rs:797-808` | `val_loss` is contaminated |
| T9 | `max_grad_norm`, `weight_decay`, `batch_size`, `target_modules`, per-profile `system_prompt`, `reward_hook` ignored | `mod.rs:243-245`, `gpu.rs:452,462`, `train_arm.rs:393,428` | Profile YAML is largely decorative |
| T10 | Metal "QLoRA" dequantizes NF4 to a cached BF16 copy; LoRA dropout not applied | `patches/qlora-rs-1.0.5/src/qlora.rs:297,480` | BF16 memory + NF4 error; no regularization |

Also: no best-checkpoint selection or early stopping; pass@k == pass@1 (greedy, 1 sample);
`max_regression_drop: 0.03` on 52 tasks is 2 tasks (noise); below 12 GB, `small_code_default`
resolves to retired, unpinned Qwen2.5-Coder-3B (`gpu-specs.yaml:266`).

## 2. What the data actually contains

The trained set is `target/dogfood/train_mixed.jsonl` == `mens/data/train.jsonl` ==
`train_full_backup.jsonl` (byte-identical, 9,268 rows).

| Source | Rows emitted | Share | Weight | Notes |
|---|---|---|---|---|
| `mix_sources/docs.jsonl` | 5,856 | 63% | 1.0 | — |
| `validated_mixed.jsonl` | 1,733 | 19% | 6.0 | "primary" |
| `research-lane-sft.jsonl` | 1,000 | 11% | 1.0 | — |
| `synthetic.jsonl` | 679 | 7% | 0.5 | sample_rate 0.1 |
| `rust_source.jsonl` | 0 | 0% | 2.0 | lane-filtered out |

**Weights are no-ops.** Without `physical_repeats`, weight is written as `mix_weight`
(`mix/mod.rs:511-528`), which `TrainingPairRaw` does not have — serde drops it.

### 2.1 docs.jsonl (63% of the set)

- Prompt template: `Explain the Vox concept: <heading> (precise prose; any code snippets must use \`->\` returns.)` —
  5,256/5,856 rows. Hard-coded at `extract_docs.rs:352-354`. Golden `.vox` uses `to` 189:1.
- Response = the raw doc section: many are link lists (219 start with `- [`), mid-thought
  fragments ("Task M3 … asks"), regex tables. Prompts carry no context, so this is
  memorization of doc text dressed as QA. Length p50 581 chars, max 28,825.
- All rated 3, difficulty 5 — the rating/difficulty fields carry no information.

### 2.2 validated_mixed.jsonl (the Vox code signal)

- 1,733 rows but **807 unique responses**; 1,198 prompts use the placeholder name `example`
  (`instruction.rs:109` fallback); prompts still teach retired `@table`.
- 1,353 responses include the file's YAML frontmatter comment block (`// training_eligible: true`,
  `// syntax_version`, …); 710 include `// ANCHOR:` markers; 244 start with a fabricated
  `// STABLE: \`example\` reactive_component is production-ready.` (`multiturn.rs:136`).
- 502 rows are **copy tasks**: the response is already in the prompt (turn-2 rows embed the
  previous full file and "answer" with the same file).
- 308 error_correction rows use 4 textual mutations (missing `}`, `fun`, `lett`, dropped
  return type). Every diff is 1–2 lines, yet the response regenerates the whole file.
  Broken code is never compiled to confirm it fails (`negative.rs:5-50`).

### 2.3 synthetic.jsonl

- 6,952 rows, **1,181 unique responses**. `vox_gen_cot` (1,000 rows): **one** `<think>` template
  repeated 1,000×, 277 unique code bodies.
- Typo augmentation mangles backticked identifiers while the response keeps the correct name
  ("What does thos training veent mean?"). Largest category `cli_command` (1,345) includes
  hidden/retired commands with fake `vox {cmd} [options]` usage.
- Tool rows: `{tool, description: "<name> action", arguments: {}}` — the description is
  the tool name, and the arguments are placeholders.

### 2.4 research-lane-sft.jsonl

- One instruction for all 1,000 rows; fictional entity chains. The output's relation labels
  come from a bucketing of `fact_idx` that does not match the fact template
  ("constraint prevents" → "temporal resonance"), evidence is unnumbered and never cited
  despite "You must cite evidence". Combined with T4, the model sees neither the evidence
  nor a faithful answer — this is hallucination training.

### 2.5 rust_source.jsonl (unused)

19,664 rows, lane `rust_context` → filtered out of `mix.yaml`, and absent from `mix-rust.yaml`.
Prompts are context-free ("Implement the `try_from` function in Rust"); 779 rows are
`*.generated.rs` (the same `try_from` 180×).

### 2.6 Compile check

200 unique sampled Vox codegen responses through `vox check`: **158 pass, 42 fail (21%)**.
Failures are dominated by the fake `<think>` prefix, retired `@`-decorators, undefined names.
By contrast, 58/60 golden files and ~83% (123/148) of non-golden repo `.vox` files compile —
roughly 600 compile-clean files beyond the 146 marked `training_eligible: true`.

### 2.7 Held-out bench

52 tasks, vox-lang only. Only `action_delete` shows >30% 8-gram overlap with training responses —
the bench is clean. There is no bench for rust, tool-selection, or argument-generation.

## 3. Spoke readiness

| Spoke | Sources present | Trainable today? |
|---|---|---|
| vox-lang | only `docs.jsonl` (5 of 6 missing) | trains on docs prose only; 966 rows pass its filter from the main set |
| rust | none (`rust_authoring.validated.jsonl` is `optional:false` but the mix runs non-strict) | empty |
| tool-selection | none; generator is library-only, `ToolSelectionRow` lacks `Serialize`, and default `include_lanes={vox_codegen}` would drop its rows anyway | empty |
| argument-generation | same as tool-selection | empty |

`mens/B8-TRAINING-GATES.md` marks vox-lang/rust **PROCEED** based on
`data-sufficiency-spike-b2_5.json`, which states "Corpus data not yet generated … generator-capacity
estimates". Those decisions do not describe real data. The readiness gate
(`corpus_readiness.rs:36`) is not called by any training path.

## 4. Runs and the 27B artifact

All three runs are local Metal candle QLoRA. `qwen3_8b_metal_v2` shows pre == post on every
collateral metric (the adapter likely wasn't applied in eval). `vox-mens-27b` was merged from
`checkpoint_step_500`, which is ~5.7% of one epoch, with loss ~57–75 (vs 0.85–2.9 on 8B, suggesting summed rather than
averaged loss or divergence). Its README's PPL 5.12 / 100% compile / 92% exec have no eval
artifact behind them. The base-model provenance is inconsistent (Qwen3.5-27B vs "Qwen3.8-27B",
multimodal config). Treat it as untrained.

## 5. Remote tuning / larger models

The cloud path has never completed a job:

- `infra/containers/Dockerfile.populi` copies a nonexistent `populi/`, builds features on the
  wrong crate, has no nvcc, and no workflow publishes the image (the name also mismatches `cloud/mod.rs:119`).
- `sync_checkpoint_down` is a stub (`pipeline_dispatch.rs:185-204`), so the post-run eval gate
  runs on an empty dir. The HF token is never passed, so the upload fails; entrypoint errors are unchecked.
- Single-GPU only (`gpuCount: 1`; multi-GPU offers rejected in `offer_filter.rs:14-18`);
  no FSDP/DeepSpeed; no MoE support. Runtime cap 3600 s, $10 budget, `num_samples` hardcoded to 5000, 80 GB disk.
- Security: full-account Vast/RunPod API keys are injected into marketplace containers
  (`vast.rs:114`, `runpod_provider.rs:141`); RunPod `/stop` keeps billing the volume.

**Recommendation.** Do not extend candle to multi-GPU/MoE. Add a pinned-digest PEFT trainer
(Unsloth or axolotl) to the cloud image for 14B / 32B / Qwen3-Coder-30B-A3B, keeping candle for
local Metal. This crosses the VoxScript-first rule and needs an ADR. Estimated cost at the current
~7.2M tokens/epoch × 3 epochs: 14B on A100-80G ≈ $1.5–3; 32B on H100 ≈ $6–10; 30B-A3B ≈ $4–6.
A side benefit: the same data trained with both trainers is a differential test for the candle
trainer (it would have exposed T1).

Required plumbing: fix the Dockerfile + publish workflow; real artifact pull-down; scoped
write-only HF token; fail-loud entrypoint; row-count-derived `num_samples`; per-model runtime/disk/budget;
pin image digest, base revision, dataset revision, seed; record `git_sha` and actual cost;
checkpoint resume for spot instances; host-side termination instead of in-container API keys.

## 6. Per-spoke training-set plan

Cross-cutting (do once):

1. **Clean extraction.** Strip frontmatter/`ANCHOR`/`STABLE` lines; emit at declaration
   granularity with real names (fix `extract_name_from_source` for `component`/`table`/`query`/
   `mutation`/`server`/`tool`/`routes`/`state_machine`/`pub fn`); delete turn-2 echo rows.
2. **Compiler as oracle.** Every Vox response must pass `vox check` (and its `@test`s where
   present) at mix time; reject otherwise. Record the check result in the row.
3. **Dedup and split by normalized response hash** (not prompt+response); make `sample_rate` seeded.
4. **Make weights real**: either enable physical repeats or have the loader read `mix_weight`
   as a sampling weight. Run mixes strictly; fail when a spoke's output is below its readiness
   threshold; call `corpus_readiness` from `vox mens train`.
5. **Measure, don't estimate**: regenerate `data-sufficiency-spike` from actual files
   (rows, unique-response ratio, compile-pass rate, token-length histogram vs seq_len).

### vox-lang

- **Primary data: compile-verified code with generated instructions.** From ~750 compile-clean repo
  files (not just 146 eligible) extract each declaration; generate the instruction by
  back-translation through `vox_actor_runtime::llm` (code → natural-language task), then keep only
  pairs where a base-model regeneration from that instruction also compiles, or where the
  instruction is judged faithful. This is the OSS-Instruct / back-translation pattern.
- **Rejection-sampling fine-tuning (STaR/RFT).** Sample N completions from the base model for
  task descriptions (bench-style, *not* bench items), keep those that pass `vox check` + tests.
  This is the highest-leverage lever for a low-resource language because the compiler is a free
  verifier. It is also the natural on-ramp to the planned GRPO `r_test` reward.
- **Real error correction.** Mutate at AST level, keep only mutants the compiler rejects, put
  the actual diagnostic text in the prompt, and make the response the fixed declaration (or a
  unified diff), not the whole file. Mine the `vox repair` / healing loop for real pairs.
- **Docs.** Drop the `->` suffix. Either convert to grounded QA (doc section in context →
  question → answer) or move raw sections to a low-weight continued-pretraining lane. Drop link-list
  and <100-char sections.
- **Remove** the templated `vox_gen_cot` think blocks (or replace with real rationales), and
  typo augmentation that touches identifiers.
- Raise `seq_len` to ≥2048 once T6 is fixed (17% of codegen rows exceed ~1024 tokens).

### rust

- Wire `rust_source.jsonl` into `mix-rust.yaml` with lane `rust_context`, **excluding
  `*.generated.rs`** and test-only helpers.
- Give prompts context: signature + doc comment + enclosing module skeleton → body (FIM-style),
  so the task is well-posed. Verify with `cargo check` on the reinserted body for a sampled subset.
- **Commit mining**: `git log -p` over workspace crates yields (commit message + pre-image → diff)
  authoring pairs that reflect how this codebase actually changes. Filter to commits whose CI passed.
- Set `ce_last_k: 0`. Build a held-out rust bench (e.g. 50 functions from post-cutoff commits).

### tool-selection / argument-generation

- Expose the generators via `vox mens corpus` + a pipeline stage; derive `Serialize`; emit
  prompt/response or `messages`; set `include_lanes` in their mix files; add converters for
  `record_format: tool_selection|argument_generation` (or drop the field).
- Source descriptions and JSON Schemas from the real MCP registry (~866 `vox_*` names found in
  `vox-orchestrator-mcp`), not `name + " action"`. Validate generated arguments against the
  schema; keep hard negatives from the same category; match the phase-3 10-candidate config
  (code caps at 6).
- **Real traces.** Capture orchestrator/MCP sessions (already modeled as `tool_trace`,
  `a2a_trace`) with outcome labels; weight successful trajectories. Until then, given ~450
  achievable synthetic rows each, train the union `harness` adapter only and defer the split.
- Build the BFCL-style bench referenced by `eval-gates-bfcl.yaml` from held-out tools and capture
  the base baseline first (the beat-base gate silently skips otherwise).

### research-expert (retired from fine-tuning)

Keep it out of the main mix until: `input` is read (T4); labels are derived from the chosen
fact template; evidence is numbered and the answer cites `[n]`; generation is seeded. Better:
harvest real research-pipeline outputs whose citations were verified (`vox-research-shim`).

## 7. Remediation status (branch `fix/mens-training-quick-fixes`, based on `fix/base-breakages`)

### Fixed and verified

| Issue | Fix | Verification |
|---|---|---|
| T1 grad accumulation | `accumulate_and_maybe_step` sums per-`Var` grads across the window for all four step entry points; paged path writes params back | 2 new tests fail on the old code (`w stayed 1`) and pass on the new; a mutant that disables the buffer turns both red |
| T2 system prompt | Built-in prompt rewritten to current syntax (`to` returns, bare keywords, braces), kept under 2.6k chars | `builtin_system_prompt_uses_current_vox_syntax` |
| T3 template | `assemble_prompt` (Metal + CUDA) emits training ChatML, keeps pre-wrapped prompts, adds a missing system turn; chat completions render ChatML; eval-local prefix matches training byte-for-byte | 3 tests per backend; handler tests updated |
| T4 `input` | `instruction` + `input` joined into the prompt | `alpaca_input_is_joined_into_prompt` |
| T5 `ce_last_k` | Default 0 (whole assistant response) in CLI, pipeline, dispatch, both configs and every profile | populi config test; logits are full-sequence either way, so no memory change |
| T6 truncation | `fit_to_seq_len` guarantees the answer up to half the window, trims an over-long prompt from its middle, then cuts the answer tail (pure head-truncation left no answer when the ~700-token system prompt exceeded `--seq-len 512`); the masked-CE preflight gives up after 256 unsupervised probes | 4 `fit_to_seq_len` tests; real Qwen3-0.6B run passes preflight at seq 512 |
| **Loss → ~250 after the first optimizer step (all Metal runs, incl. the 27B)** | Root cause in `candle-metal-kernels` 0.10.2: the rank>4 strided reduce fallback indexed outer instead of inner dims, so `repeat_kv`'s (GQA) backward produced garbage K/V LoRA grads that overflowed to NaN; AdamW wrote NaN into every LoRA var even at `lr=0`, and Metal's NaN-skipping `max` hid it as a finite ~250 loss. Crate vendored under `patches/` with a one-line fix | `repeat_kv_backward_on_metal_matches_cpu` fails on 0.10.2 (max diff 7.81), passes patched; real runs keep sane loss after step 1 (val_loss 1.29) |
| Reported loss included the mix weight | `loss_scalar` is the plain masked NLL; only the backward loss carries the weight | backend suites; real-run logs |
| `--seq-len` / `--grad-accum` silently overridden | Explicit flags beat `apply_qwen_size_ladder_policy` | preset test failed before, passes after |
| Stale `vox-ml-cli` sidecar | `VOX_PARENT_BUILD_ID` handshake warns on skew (`VOX_REQUIRE_MATCHING_ML_CLI=1` fails); `vox doctor` row | handshake + doctor tests; binary smoke |
| Train overwrote `--data-dir`, mix fed back into itself, env opt-out cleared, `--fast-corpus` ignored `--data-dir` | Pairs file and mix output separated; non-canonical `--data-dir` trained as-is; self-referencing mix rejected; user env never cleared | data-dir-untouched and no-feedback tests; real run logs "corpus mix not run" |
| Non-reproducible mix; docs-dominated main mix | Seeded hash sampling; docs `sample_rate 0.25` + `max_lines 1500`, research capped | byte-identical mix test; effective weight code 62% / docs 31% |
| Silent training | Progress line every 10 opt steps or 30 s, "Training started", preflight heartbeat, `val_loss=n/a` | cadence/format tests; real Metal output |
| Only ~116–259 unique Vox answers | Per-declaration pairs (prompt carries referenced signatures; answer verified in context); compiler-verified error correction with real diagnostics; extraction widened to a configured source pool (`mens/config/vox-source-pool.yaml`, inventory in `contracts/reports/mens-vox-source-inventory.v1.json`) with a bench-containment leakage guard | regenerated corpus: 1,510 pairs, 839 unique answers, 0 metadata leaks |
| No instruction synthesis | `vox mens corpus back-translate` / `rft` stages via `vox_actor_runtime::llm`, dry-run by default, spend-gated, cached | 21 mock-backend tests; dry-run cost estimates |
| T7 curriculum order | Shuffle, then stable-sort by difficulty | `curriculum_order_is_sorted_by_difficulty_but_shuffled_within_levels` |
| T8 val leakage | Validation split grouped by response hash, seed-deterministic | `rows_sharing_a_response_stay_on_one_side` |
| LR off-by-one | Schedule applied before step 0; counter advanced before computing the next LR | compile-checked on both backends (inside the loop; no unit seam) |
| Messages rows lacked system prompt | `with_system_turn` in training and validation | `messages_rows_gain_system_turn_once` |
| Mix weights ignored | `mix_weight` read by the loader, applied as per-row loss weight (clamped at 8) | `mix_weight_scales_loss_even_without_trajectory_weighting` |
| Mix dedup collapsed messages-only rows | Key covers prompt/instruction, input, response/output, messages | `dedup_key_distinguishes_messages_only_rows` |
| Spoke mixes dropped every row | Explicit `include_lanes` on vox-lang, tool-selection, argument-generation, harness | config |
| Docs `->` suffix, link lists, archive | Suffix removed; link-list sections and `archive/` skipped | 2 new extractor tests |
| Golden pairs | Frontmatter / `ANCHOR` / `@training_prompt` stripped from answers; `@training_prompt` used as the instruction; name extraction handles every bare keyword and decorators (no `"example"` fallback); templates no longer say `@table` / "JSX"; whole-file multi-turn echo generator removed | 2 new tests |
| Typos / shuffle | Identifiers, code spans, paths, CamelCase protected from typos and lowercasing; word shuffle off | `typos_never_touch_identifiers_or_code_spans` |
| Rust corpus | `*.generated.rs`, `patches/`, `archive/` excluded | `generated_and_vendored_rust_is_excluded` |
| Cloud sizing | `num_samples` from the real `train.jsonl` row count | `jsonl_row_count_counts_non_empty_lines` |

The build breaks this audit hit on `877406d84` (`vox-populi` hub fields, CUDA duplicate Q/K-norm initializers, `merge.rs` test import, stale CUDA RoPE test, misplaced `ANTI_STUB_MIN_CONSTRUCT_RICHNESS`) were fixed independently on `fix/base-breakages`, which this branch builds on.

### Proposed (not quick; each needs a design decision or larger change)

1. ~~Per-declaration extraction~~ and ~~real error correction~~ — done (above). Open: error correction is still 43% of pairs and over-represents retired-decorator mutants; rebalance.
2. **Run the synthesis stages** — needs a pinned provider/model and budget (dry-run: ~$1.3 back-translation of 749 rows; RFT up to ~$95).
3. **Checkpoint/data mismatch guard.** `vox mens train` auto-resumes a checkpoint in `--output-dir` even when it was trained on different data; refuse or warn on a data-fingerprint mismatch.
4. **Edit tasks** to replace the removed multi-turn generator: real before/after pairs mined from `git log -p` on `.vox` files, answered as diffs.
4. **Rejection-sampling fine-tuning** for vox-lang: sample the base model on task descriptions, keep completions that pass `vox check` and `@test`.
5. **Docs lane.** Grounded QA (section in context → question → answer) or a low-weight continued-pretraining lane.
6. **Tool spokes.** Expose `tool_selection_synth` / `argument_generation_synth` as CLI + pipeline stages (add `Serialize`, prompt/response); descriptions and JSON Schemas from the real MCP registry; validate arguments against schema; capture real orchestrator traces; BFCL-style held-out bench with a base baseline.
7. **Rust spoke.** Wire `rust_source.jsonl` with FIM-style context (signature + docs + module skeleton → body), `cargo check` on a sample, commit-diff mining; held-out bench from recent commits.
8. **Research lane.** Derive relation labels from the chosen fact template, number evidence and cite `[n]`, seed generation; or harvest verified research-pipeline outputs.
9. **Profile plumbing.** Carry `max_grad_norm`, `weight_decay`, per-profile `system_prompt`, and `target_modules` through `LoraTrainingConfig` (both copies) and the plugin wire format.
10. **Strict mixes + readiness gate** in the train path: fail when a spoke's output is below its readiness threshold; seeded `sample_rate`; regenerate `data-sufficiency-spike` from real files.
11. **Eval.** Best-checkpoint selection / early stopping on the de-leaked val loss; pass@k with real sampling; a regression gate sized for noise (bootstrap CI instead of a 0.03 drop on 52 tasks).
12. **Remote training** (needs an ADR — crosses VoxScript-first): pinned PEFT trainer image (Unsloth/axolotl) for 14B/32B/30B-A3B; fix `Dockerfile.populi`; real artifact pull-down; scoped HF token; fail-loud entrypoint; host-side termination instead of in-container account keys; per-model runtime/disk/budget; pin image digest + base/dataset revisions + seed; spot resume. First run: the same data on candle and PEFT at 8B as a differential test.
13. **Retire** the `vox-mens-27b` staging artifact and its unsupported README metrics; replace `small_code_default`'s retired Qwen2.5 rungs with pinned Qwen3 revisions.

After merging, regenerate the corpora (`vox mens corpus` extract → pairs → mix): the files under `target/dogfood/` and `mens/data/` still contain the old defects until rebuilt.

## 8. Priority order

1. T1 grad accumulation; T2 system prompt (load `mens/config/system_prompt.txt`, fix the pinned test); T3 eval template; T4 `input`.
2. `ce_last_k: 0` everywhere; shuffle within curriculum buckets; response-hash val split; system prompt short enough for seq_len.
3. Clean extraction + compile oracle + real weights + strict mix + readiness gate in the train path.
4. vox-lang: back-translated compile-verified pairs, then rejection sampling.
5. Cloud PEFT trainer (ADR), 8B differential run vs candle, then 14B/32B.
6. Wire rust + tool spokes with benches; retire the 27B staging artifact and its README numbers.
