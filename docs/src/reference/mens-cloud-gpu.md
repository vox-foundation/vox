---
title: "Mens Cloud GPU Training Strategy"
description: "Official documentation for Mens Cloud GPU Training Strategy for the Vox language. Detailed technical reference, architecture guides, an"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# Mens Cloud GPU Training Strategy

This page documents what is implemented now in cloud-profile selection and what remains experimental.

## Implemented behavior (code-aligned)

- Local 4080-class training remains the baseline: `vox mens train --backend qlora --preset 4080`.
- `DEFAULT_PRESET` is `4080` in `preset_schema`.
- `4080` is an alias of `qwen_4080_16g` in in-code preset shaping.
- `--preset auto` resolves from `mens/config/gpu-specs.yaml` (`presets` table) by VRAM fit.
- CUDA VRAM hinting may also select QLoRA presets through `vram_autodetect` helper output.

## Canonical preset sources

- Runtime preset defaults and aliases: `crates/vox-populi/src/mens/tensor/preset_schema.rs`.
- Runtime VRAM autodetect helper: `crates/vox-populi/src/mens/tensor/vram_autodetect.rs`.
- SSOT GPU/preset data for local + cloud estimators: `mens/config/gpu-specs.yaml`.

## Profile compatibility matrix (practical)

| Surface | Supported now | Notes |
|---|---|---|
| Local workstation (4080 class) | Yes | Primary baseline; recommended default path. |
| Local higher VRAM (24G/48G/80G) | Yes | Use explicit preset or `--preset auto`. |
| `vox mens train --cloud ...` dispatch | Feature-gated | Requires `vox-cli` built with `cloud`; provider dispatch path exists but should be treated as additive. |
| Remote execution via Populi routing hints | Read-only scheduling signal | Hints enrich placement choices; execution remains local-safe unless explicitly extended. |

## Non-goals (current wave)

- No promise of full provider-native lifecycle automation parity across all clouds.
- No replacement of local-first runbook with cloud-only assumptions.
- No second preset stack: cloud path reuses the same preset machinery as local.

## Operational guidance

- Keep `4080` as first-pass default for regression and acceptance gating.
- Use cloud dispatch when you need faster iteration or larger VRAM, not as a dependency for baseline dev flow.
- For interruptible cloud hosts, persist `--output-dir` to durable storage and avoid `--force-restart` unless intentionally resetting.
