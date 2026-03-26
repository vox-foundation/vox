---
title: "Contributing — Mens native training"
description: "Onboarding for vox-populi tensor / schola train paths"
category: "how-to"
last_updated: 2026-03-25
---

# Contributing — Mens training (native)

## Read first

- [Mens native training SSOT](../reference/mens-training.md)
- [HF finetune capability matrix](../architecture/hf-finetune-capability-matrix.md)

## Entrypoints

| Surface | Location |
|---------|----------|
| CLI | `vox schola train` → `crates/vox-cli/src/commands/schola/train/` |
| Library | `vox_populi::mens::tensor::run_mens_training` (`lora_train.rs`) |
| Contract | `FineTuneContract`, `ExecutionPlanner`, `preflight_train` |

## Commands

```bash
cargo check -p vox-populi --features mens-train
cargo test -p vox-populi --features mens-train execution_planner
```

## SSOT rule

**Candle QLoRA** is the active `schola train` backend; keep docs and error messages aligned (`lora_train.rs` is authoritative when in doubt).
