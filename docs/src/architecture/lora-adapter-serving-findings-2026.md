---
title: "LoRA adapter application at serve time"
description: "Why a QLoRA-fine-tuned model served through vox mens serve returned base-model output, the real adapter key and scaling conventions, and how delta application is now wired and enforced."
category: "Architecture SSOTs"
status: "current"
---

# LoRA adapter application at serve time

## What was broken

`InferenceEngine::load` (both `vox-plugin-mens-candle-cuda` and
`vox-plugin-mens-candle-metal`) built every `QuantizedLinear` from
`get_tensor(base_key)`, which only ever searched **base-model key names**. The
adapter file was in the search path but contributes no base keys, so a model
fine-tuned via QLoRA and served through `vox mens serve` produced bit-identical
output to the unmodified base model. The "adapter loaded" claim was false end to
end.

Two further defects fell out of reading the training path:

- **The adapter key spelling in `merge.rs` was wrong.**
  `merge_qlora_into_base_subset` looked up `<logical>.lora_a` /
  `<logical>.lora_b`. `QLoraTrainer::save_adapter` dumps the varmap verbatim, and
  peft-rs registers each factor as a `candle_nn::Linear` weight under the layer's
  `VarBuilder` prefix (`linear_no_bias(.., vb.pp("lora_a"))`), so what is on disk
  is **`<logical>.lora_a.weight`**. The offline merge would have failed with
  `adapter missing lm_head.lora_a` on any real adapter. Confirmed against
  `crates/*/src/model.rs`, which asserts on the varmap name
  `layer0.qkv.lora_a.weight`.
- `top_k` / `output_mode` were sent by `worker.rs::inference_payload` but absent
  from `PromptRequest`, so serde dropped them silently. Fixed in a **separate
  commit** — it is an unrelated bug (see below).

## Conventions verified, not assumed

| Thing | Real value in this codebase | Source |
|---|---|---|
| Adapter tensor keys | `<logical>.lora_a.weight`, `<logical>.lora_b.weight` | `QLoraTrainer::save_adapter` (varmap dump) + peft-rs `LoraLayer::new` |
| Logical names | `layer{i}.{q,k,v,o,qkv,z,b,a,gate,up,down}`, `lm_head` | `candle_qlora_train/mod.rs` `adapter_layer_order` |
| Logical → base key | `meta.base_key_map` in `adapter_manifest.json` (v3) | `finalize.rs::build_adapter_manifest_v3` |
| Shapes | A `[rank, in]`, B `[out, rank]` | peft-rs `LoraLayer` docs + qlora-rs `lora_weights()` |
| Scaling | **alpha / rank** (PEFT convention), not alpha alone | `merge::lora_delta_f32` and qlora-rs `QLoraConfig::scale()` — both agree |
| Delta | `W' = W + (B @ A) * (alpha / rank)` | `merge::lora_delta_f32` |

## Approach: reuse, not a third implementation

`qlora_rs::QuantizedLinear::forward` **already** computes
`x @ W_q^T + (x @ A^T) @ B^T * scaling`. But the inference constructor
`from_weight` builds its LoRA via `LoraLayer::new_with_zeros`, so the residual is
identically zero, and there is no way to inject trained factors afterwards:
`LoraLayer`'s `lora_a`/`lora_b` are `candle_nn::Linear` with private weights, and
peft-rs 1.0.3's `SaveLoad::load_state_dict` for `LoraLayer` is an explicit
**no-op placeholder** that only validates shapes.

So the delta is folded into the base weight **before** quantization, reusing the
existing `merge::lora_delta_f32` primitive — the same function, same scaling, that
the offline `merged.safetensors` path uses. No new LoRA math anywhere.

### Memory: store factors, multiply lazily

The first version of this fix materialized every layer's dense delta up front and
held them all in a `HashMap` for the whole of `InferenceEngine::load`. Since LoRA
targets nearly every linear layer, that resident set is approximately a full F32
copy of every linear layer in the model at once — the LM head alone is ~4.7 GiB at
27B. It would have OOM'd before serving a token.

`LoraFactors` therefore stores `a` `[rank, in]` and `b` `[out, rank]` (orders of
magnitude smaller) plus the scaling, and `fold_lora_delta` forms the `[out, in]`
product transiently across a single `broadcast_add`, then drops it. Peak extra
memory is one weight, not one per layer. The offline merge never had this problem
because it processes one tensor at a time and writes it out immediately; the
inference path had copied its loop shape but not its streaming discipline.

## Wiring is enforced, not assumed

A review demonstrated that the first version's tests were worthless for the actual
fix: reverting **one** of the 13 `linear_weight` call sites back to `get_tensor` —
reintroducing the original defect at that site — left all 57 tests passing. The
headline test hand-built the delta application in its own body, so it tested
candle/qlora arithmetic rather than any wiring.

Three guards now exist. **The safety property is the first one; the other two are
early warnings.** Do not read the static text-matching test as the thing that makes
an unapplied adapter unshippable — it is not.

1. **`assert_adapter_fully_applied`** — the actual guarantee. It runs at the end of
   every `InferenceEngine::load` in **both** crates, in production, not just under
   `cargo test`. `linear_weight` records every key it fetches; the check then fails
   the load unless every trained delta key was requested by some call site (catches
   an adapter keyed to a tensor nothing loads — the tied-head spelling mismatch)
   **and** every projection `linear_projection_keys(layout)` declares was requested
   (catches a call site that stopped going through the adapter path, however it was
   spelled). A model whose adapter is not fully applied therefore refuses to serve
   rather than quietly serving base weights. Both halves are pure functions and are
   unit-tested directly.
2. **`no_linear_layer_is_built_from_an_unadapted_weight`** — a CI-time early warning,
   not a safety guarantee. It asserts on `include_str!("inference.rs")` because no
   unit test can execute those call sites without a real multi-shard model, so the
   source text is the only thing that can see them at test time. Its value is
   catching the mistake at review time instead of at first load. Its limits are
   real: it is text matching, so it sees call-site *deletions* in the current
   formatting and nothing more. Two review mutations shaped it — reverting a call
   site to `get_tensor` in place (caught by the substring check) and hoisting the
   fetch into a local before constructing the layer (caught **only** by asserting
   the exact occurrence count; an earlier `>= 13` lower bound tolerated losing one
   site and let that refactor through).
3. **`every_key_training_can_map_to_is_one_inference_requests`** — asserts every
   base key `candle_qlora_train` can put in `base_key_map` is a key inference
   actually fetches, so a spelling drift on either side is caught statically.

## Adversarial review

- **Can a base-only request get a delta?** No. `fold_lora_delta` returns the weight
  unchanged for any key absent from the map, and the map is empty when there is
  nothing to fold. `InferenceEngine::load` also hard-bails when
  `candle_qlora_adapter.safetensors` is missing, so there is no base-only path
  through this function at all.
- **Can a delta be applied twice?** `merged.safetensors` holds `W + BA·α/r` under
  *base* key names and `weight_sources` puts it ahead of the base shards, so that
  file is the only double-apply hazard. `adapter_deltas_must_be_folded` returns
  false exactly when that file is present at the path `weight_sources` reads, so
  the two paths are mutually exclusive by construction.
- **Does `weight_sources`' adapter-first ordering interact badly?** No. The adapter
  file's tensors are named `<logical>.lora_*.weight` and can never collide with a
  base key, so it cannot shadow anything.
- **Tied `lm_head` / `embed_tokens`.** When weights are tied, `base_key_map` maps
  `lm_head` onto the embedding key. `lm_head` is read through `linear_weight`
  (gets the delta) and `embed_tokens` through `get_tensor` (does not) — matching
  training, where only the LM head carried a `QuantizedLinear`/LoRA.
- **Device / dtype / shape.** Factors are `.to_device`d at load; `get_tensor`
  normalizes every base weight to F32 and the delta is F32; the delta is
  `[out, in]`, identical to `W`, so `broadcast_add` is a plain element-wise add.

## Sampling parameters — a separate bug, a separate commit

`GenerateRequest` has always defaulted to `temperature: 0.7`, and `handlers.rs` has
always sent `top_k: 40`. The plugin's `PromptRequest` declared neither, so serde
dropped both and served greedy decoding regardless of what the caller asked for.
That is a real bug, but it has no causal link to LoRA application, so it ships as
its own commit.

An earlier draft of this document justified the change by claiming the
structured-output repair loop needs sampling because a greedy model returns the
same bad output on retry. **That was wrong.** `handlers.rs` embeds the previous
error and output in a *different* prompt on each retry attempt, so retries vary
regardless of temperature. The honest rationale is simply that the plugin was
ignoring caller-requested sampling parameters.

Consequence to be aware of: `vox mens serve` now honors those defaults, so the
default HTTP path is stochastic rather than deterministic. `temperature <= 0` or
`top_k == 1` still gives exactly the old argmax.

## `output_mode`

This plugin has **no constrained decoder** — grammar-constrained generation is
deliberately not linked in. It is not the enforcement point either: `vox-ml-cli`'s
serve handlers shape the prompt (`prompt_for_output_mode`) and validate with
repair retries (`validate_structured_output_with_reason`). So `check_output_mode`
accepts the three labels that layer enforces (`strict_json`, `jsonl_records`,
`tool_args_json`) as advisory, and **errors** on any other value, which has nobody
enforcing it and would otherwise be served as free-form text the caller believes
is structured. Erroring on `strict_json` itself would have broken the shipped,
working repair loop.

## Known limitations

- **No end-to-end test through `InferenceEngine::load`.** It needs a real
  multi-shard base model; nothing in the repo provides a small enough fixture. The
  runtime guard in `load` plus the source-level test cover the wiring instead.
- **The delta is quantized along with the base** (folded pre-NF4), whereas training
  adds it in F32 after dequantization. This is the standard merged-adapter
  deployment tradeoff and is exactly what the existing `merged.safetensors` path
  already does, so serving is now consistent between the merged and un-merged
  routes. Bit-exact parity with training would require injecting the factors into
  `LoraLayer`, which peft-rs 1.0.3 does not support.
- **The two plugin crates have diverged** outside this change: the CUDA crate
  carries device-resolution and RoPE-synthesis fixes the Metal one lacks. That
  pre-existing divergence is untouched and remains a hazard of its own.
- **Neither `cuda` nor `metal` kernel feature was exercised** — no such hardware on
  the machine this was developed on. The changed code is device-agnostic
  (`to_device` + F32 `broadcast_add`), but the GPU paths are unverified.
