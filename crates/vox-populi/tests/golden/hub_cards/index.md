---
license: apache-2.0
library_name: vox
pipeline_tag: text-generation
language:
- en
spaces:
- ns/vox-mens-playground
tags:
- vox
- vox-mens
- qwen3
- qwen3-5
- qwen3-8
- qlora
- code
- conversational
- index
---
# Vox MENS — AI coding models for the Vox language

MENS is the Vox project's family of open-weights coding models (`0.6B` through `27B`), fine-tuned to write, check, and explain [Vox](https://voxlang.org) — a full-stack programming language where database tables, backend endpoints, reactive web UI components, and AI agents live in one compiler-checked `.vox` file. Pick a size that fits your machine, run locally or through Python and Hugging Face Inference Providers, and try live checkpoints in the [Vox MENS Playground Space](https://huggingface.co/spaces/ns/vox-mens-playground). The docs explain [what MENS is](https://voxlang.org/reference/mens-models/#what-mens-is), how [models are organized](https://voxlang.org/reference/mens-models/#hub-and-spoke), and how automated [quality gates](https://voxlang.org/reference/mens-models/#gating) verify every release; this index lists every size side by side.

![Vox MENS overview](assets/overview.jpg)

## Sizes and spokes

| Size | vox-lang | harness: tool-selection | harness: argument-generation | rust | Hub | Repo |
|---|---|---|---|---|---|---|
| 0.6B | failed | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | `v5` on Hub; local run differs | [ns/vox-mens-qwen3-0.6b](https://huggingface.co/ns/vox-mens-qwen3-0.6b) |
| 2B | needs re-gate | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | not published | [ns/vox-mens-qwen3.5-2b](https://huggingface.co/ns/vox-mens-qwen3.5-2b) |
| 4B | passed | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | published `v2` | [ns/vox-mens-qwen3.5-4b](https://huggingface.co/ns/vox-mens-qwen3.5-4b) |
| 9B | not evaluated | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | not published | [ns/vox-mens-qwen3.5-9b](https://huggingface.co/ns/vox-mens-qwen3.5-9b) |
| 27B | not trained | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | [planned](https://voxlang.org/reference/mens-models/#hub-and-spoke) | no weights | [ns/vox-mens-qwen3.8-27b](https://huggingface.co/ns/vox-mens-qwen3.8-27b) |

Each trained cell holds the size's gate, defined in the [state legend](https://voxlang.org/reference/mens-models/#state-legend), and `planned` marks a spoke with no adapter yet. Each fine-tuned checkpoint specializes in writing, fixing, and explaining Vox programs, and only publishes after automated compiler checks verify it improves Vox code without hurting general coding.

Sizes without a complete merged checkpoint currently distribute the trained add-on while their release review is unfinished, and merged models are held until collateral and gate checks pass.

## Which model should I use

Pick the largest current model that fits comfortably in your memory (choosing a compact laptop checkpoint for quick edits or a larger studio checkpoint for multi-file changes), and step down to a smaller size whenever you want lower latency or lighter RAM use. Each card's Why this size, What it does, and Vox and Python sections explain what tasks it suits and how to wire it up, while the training peak reflects measured training memory rather than serving memory.

Among the sizes with a current gate and an adapter score from the current serving path, 4B has the highest held-out adapter pass@1, 62.5%, and a tie goes to the smaller size.

| Size | Adapter pass@1 | Training peak |
|---|---|---|
| 4B | 5/8 (62.5%) | 27007 MiB |

```bash
vox mens hub pull --size 4B --namespace ns
```

## What's next

- **0.6B**: review the gate that stopped 0.6B, because the current evaluation measured its adapter and repeating that evaluation would measure the same adapter. There is no estimate for 0.6B.

- **2B**: re-run `vox mens eval-collateral-damage` for 2B on the current 128-item slices. Re-evaluate the 2B adapter on the held-out bench under the current serving path with `vox mens eval-local`. This is an estimate, not a measurement: the 2B re-gate is projected at about 1.8 hours, or 2.8 hours if slower, with low confidence. Basis: Paired collateral evaluation for 2B runs 576 new generations (3 slices x 96 new items x 2 sides, assuming the 32-item prefix cache) at 11 s/item expected and 17 s/item conservative, giving 6336 s = 1.8 h and 9792 s = 2.8 h.

- **9B**: produce the records that are missing for 9B: `collateral_damage_report.json`, `gate_receipt.json`. This is an estimate, not a measurement: the 9B re-gate is projected at about 6.2 hours, or 9.0 hours if slower, with low confidence. Basis: Paired collateral evaluation for 9B runs 576 new generations (3 slices x 96 new items x 2 sides, assuming the 32-item prefix cache) at 39 s/item expected and 56 s/item conservative, giving 22464 s = 6.2 h and 32256 s = 9.0 h.

- **harness: tool-selection**: Picks the right external tool or API action next inside automated coding workflows. No adapter has been trained for harness: tool-selection, and there is no estimate for it. The [plan for harness: tool-selection](https://voxlang.org/reference/mens-models/#hub-and-spoke) is on the docs page.

- **harness: argument-generation**: Fills in structured inputs and parameters cleanly when calling coding tools. No adapter has been trained for harness: argument-generation, and there is no estimate for it. The [plan for harness: argument-generation](https://voxlang.org/reference/mens-models/#hub-and-spoke) is on the docs page.

- **rust**: Assists with lower-level Rust crates and runtime bindings alongside Vox apps. No adapter has been trained for rust, and there is no estimate for it. The [plan for rust](https://voxlang.org/reference/mens-models/#hub-and-spoke) is on the docs page.

## Held-out pass@1 by size

| Size | Base pass@1 | Adapter pass@1 |
|---|---|---|
| 0.6B | 2/8 (25.0%) | 5/8 (62.5%) |
| 2B | 2/8 (25.0%) | — |
| 4B | 2/8 (25.0%) | 5/8 (62.5%) |
| 9B | 2/8 (25.0%) | 5/8 (62.5%) |

The table lists held-out pass@1 ([defined here](https://voxlang.org/reference/mens-models/#held-out-pass-rate)) only for sizes with a report under the current prompt protocol `qwen3_non_thinking_codegen_v1`, and an adapter score recorded under an earlier serving path is shown as a dash because it does not count.

## Links

- Docs page: [MENS models](https://voxlang.org/reference/mens-models/)
- Playground: [Vox MENS Playground Space](https://huggingface.co/spaces/ns/vox-mens-playground)
- Training: [MENS training](https://voxlang.org/reference/mens-training/)
- Training data: [training data contract](https://voxlang.org/reference/mens-training-data-contract/)
- Serving: [MENS serving](https://voxlang.org/reference/mens-serving-ssot/)
- Source: [vox-foundation/vox](https://github.com/vox-foundation/vox)
- Machine-readable: `ladder_index.json` in this repository

This index is generated by `vox mens hub sync` from each size's records, and [how the cards are made](https://voxlang.org/reference/mens-models/#how-the-cards-are-made) is on the docs page.