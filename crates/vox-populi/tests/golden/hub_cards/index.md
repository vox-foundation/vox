---
license: apache-2.0
library_name: vox
pipeline_tag: text-generation
language:
- en
tags:
- vox
- vox-mens
- qwen3
- qlora
- index
---
# Vox MENS — Qwen3 adapter ladder

MENS is the Vox project's own adapter ladder: one LoRA adapter per size of a pinned Qwen3 base model, trained on Vox source and gated before it is published. The docs page explains [what MENS is](https://voxlang.org/reference/mens-models/#what-mens-is), how [hubs and spokes](https://voxlang.org/reference/mens-models/#hub-and-spoke) relate and how the [gates](https://voxlang.org/reference/mens-models/#gating) decide; this index is the one place that lists every size.

![Vox MENS overview](assets/overview.svg)

## Sizes and spokes

| Size | vox-lang | harness: tool-selection | harness: argument-generation | rust | Hub | Repo |
|---|---|---|---|---|---|---|
| 0.6B | failed | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | `v5` on Hub; local run differs | [ns/vox-mens-qwen3-0.6b](https://huggingface.co/ns/vox-mens-qwen3-0.6b) |
| 1.7B | needs re-gate | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | not published | [ns/vox-mens-qwen3-1.7b](https://huggingface.co/ns/vox-mens-qwen3-1.7b) |
| 4B | passed | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | published `v2` | [ns/vox-mens-qwen3-4b](https://huggingface.co/ns/vox-mens-qwen3-4b) |
| 8B | needs re-gate | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | not published | [ns/vox-mens-qwen3-8b](https://huggingface.co/ns/vox-mens-qwen3-8b) |
| 14B | not evaluated | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | not published | [ns/vox-mens-qwen3-14b](https://huggingface.co/ns/vox-mens-qwen3-14b) |
| 32B | not trained | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | no weights | [ns/vox-mens-qwen3-32b](https://huggingface.co/ns/vox-mens-qwen3-32b) |

Each trained cell holds the size's gate, defined in the [state legend](https://voxlang.org/reference/mens-models/#state-legend), and `planned` marks a spoke with no adapter yet. Each trained ladder adapter targets the vox-lang spoke and is evaluated against its active spoke gate in spoke_gates.json and the shared paired McNemar collateral gate.

## Which model should I use

Pick among the sizes whose gate is current by held-out adapter pass@1, and step down to the next smaller current size when the pick does not fit your memory. Each card's Why this size section says what work that size suits, and the training peak is measured while training rather than during serving.

Among the sizes with a current gate and an adapter score from the current serving path, 4B has the highest held-out adapter pass@1, 62.5%, and a tie goes to the smaller size.

| Size | Adapter pass@1 | Training peak |
|---|---|---|
| 4B | 5/8 (62.5%) | 27007 MiB |

```bash
vox mens hub pull --size 4B --namespace ns
```

## What's next

- **0.6B**: review the gate that stopped 0.6B, because the current evaluation measured its adapter and repeating that evaluation would measure the same adapter. There is no estimate for 0.6B.

- **1.7B**: re-run `vox mens eval-collateral-damage` for 1.7B on the current 128-item slices. Re-evaluate the 1.7B adapter on the held-out bench under the current serving path with `vox mens eval-local`. This is an estimate, not a measurement: the 1.7B re-gate is projected at about 1.6 hours, or 2.6 hours if slower, with low confidence. Basis: Paired collateral evaluation for 1.7B runs 576 new generations (3 slices x 96 new items x 2 sides, assuming the 32-item prefix cache) at 10 s/item expected and 16 s/item conservative, giving 5760 s = 1.6 h and 9216 s = 2.6 h.

- **8B**: re-run `vox mens eval-collateral-damage` for 8B on the current 128-item slices. This is an estimate, not a measurement: the 8B re-gate is projected at about 4.3 hours, or 6.2 hours if slower, with low confidence. Basis: Paired collateral evaluation for 8B runs 576 new generations (3 slices x 96 new items x 2 sides, assuming the 32-item prefix cache) at 27 s/item expected and 39 s/item conservative, giving 15552 s = 4.3 h and 22464 s = 6.2 h.

- **14B**: produce the records that are missing for 14B: `collateral_damage_report.json`, `gate_receipt.json`. This is an estimate, not a measurement: the 14B re-gate is projected at about 8.0 hours, or 11.0 hours if slower, with low confidence. Basis: Paired collateral evaluation for 14B runs about 700 new generations across the three reasoning slices at 42 s/item expected and 58 s/item conservative, giving 29400 s = 8.2 h and 40600 s = 11.3 h (rounded to 8.0 h and 11.0 h).

- **harness: tool-selection**: Chooses which harness tool to call next. No adapter has been trained for harness: tool-selection, and there is no estimate for it. The [plan for harness: tool-selection](https://voxlang.org/reference/mens-models/#hub-and-spoke) is on the docs page.

- **harness: argument-generation**: Fills in the arguments for a chosen harness tool call. No adapter has been trained for harness: argument-generation, and there is no estimate for it. The [plan for harness: argument-generation](https://voxlang.org/reference/mens-models/#hub-and-spoke) is on the docs page.

- **rust**: Writes and edits Rust code for the same workflows. No adapter has been trained for rust, and there is no estimate for it. The [plan for rust](https://voxlang.org/reference/mens-models/#hub-and-spoke) is on the docs page.

## Held-out pass@1 by size

| Size | Base pass@1 | Adapter pass@1 |
|---|---|---|
| 0.6B | 2/8 (25.0%) | 5/8 (62.5%) |
| 1.7B | 2/8 (25.0%) | — |
| 4B | 2/8 (25.0%) | 5/8 (62.5%) |
| 8B | 2/8 (25.0%) | 5/8 (62.5%) |
| 14B | 2/8 (25.0%) | 5/8 (62.5%) |

The table lists held-out pass@1 ([defined here](https://voxlang.org/reference/mens-models/#held-out-pass-rate)) only for sizes with a report under the current prompt protocol `qwen3_non_thinking_codegen_v1`, and an adapter score recorded under an earlier serving path is shown as a dash because it does not count.

## Links

- Docs page: [MENS models](https://voxlang.org/reference/mens-models/)
- Training: [MENS training](https://voxlang.org/reference/mens-training/)
- Training data: [training data contract](https://voxlang.org/reference/mens-training-data-contract/)
- Serving: [MENS serving](https://voxlang.org/reference/mens-serving-ssot/)
- Source: [vox-foundation/vox](https://github.com/vox-foundation/vox)
- Machine-readable: `ladder_index.json` in this repository

This index is generated by `vox mens hub sync` from each size's records, and [how the cards are made](https://voxlang.org/reference/mens-models/#how-the-cards-are-made) is on the docs page.