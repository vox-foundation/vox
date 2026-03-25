---
title: "Populi LoRA / adapter ownership (vox-tensor vs vox-populi)"
description: "Official documentation for Populi LoRA / adapter ownership (vox-tensor vs vox-populi) for the Vox language."
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# Populi LoRA / adapter ownership (vox-tensor vs vox-populi)

## Split

| Crate / tree | Owns | Do **not** duplicate here |
|--------------|------|-------------------------|
| **`vox-tensor`** `crates/vox-tensor/src/lora.rs` | Low-level **LoRA linear** math, parameter layout, and shared tensor utilities consumed by graph code. | HF-specific key maps, QLoRA export, merge-CLI, or `training_manifest` fields. |
| **`vox-populi`** `crates/vox-populi/src/tensor/lora.rs` + `lora_vox_transformer.rs` | **Transformer-shaped** LoRA modules, **Burn** training graph, **checkpoint** (`*.bin`), **merge** for Burn, and integration with **`FineTuneContract`** / planner. | Re-implementing generic rank decomposition — call into `vox-tensor` where appropriate. |
| **`vox-populi`** `candle_qlora_*`, `qlora_preflight`, `adapter_schema_v3` | **Candle + qlora-rs** QLoRA train/export, **v2/v3** adapter manifests, **`merge-qlora`**, HF shard/key inventory. | Burn `*.bin` merge path (`merge-weights`). |

## Drift guard

- Any change to **LoRA scaling** (`alpha/rank`), **merge equation**, or **adapter tensor naming** must either touch **one** canonical implementation and call sites, or be documented as an intentional fork with a test linking both behaviors.
- PRs touching both trees: use [`populi-llm-pr-checklist.md`](../architecture/populi-llm-pr-checklist.md) and add/adjust a **regression test** in the kernel that actually runs the changed path (`vox-populi` train or merge tests; `vox-tensor` unit tests for primitives).

## Related

- [`populi-training.md`](populi-training.md) — CLI, kernels, manifests, CI commands.
- [`hf-finetune-capability-matrix.md`](../architecture/hf-finetune-capability-matrix.md) — supported combos.
