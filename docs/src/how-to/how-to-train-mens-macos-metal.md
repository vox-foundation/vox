---
title: "Train Mens on macOS with Metal"
description: "Record the blocking Apple Silicon model and Metal training spike ahead of the complete operator guide."
category: "How-To Guides"
status: "current"
training_eligible: true
---

# Train Mens on macOS with Metal

## Spike results

`SPIKE_MODEL_ID=Qwen/Qwen2.5-Coder-0.5B-Instruct`

On an Apple Silicon (`arm64`) host:

- `mens-candle-metal` v0.6.0 installed and passed `vox plugin doctor`.
- `Qwen/Qwen3.5-0.8B` was rejected during config parsing with `vision-language / multimodal model`; its checkpoint declares `Qwen3_5ForConditionalGeneration`, image/video token IDs, and `vision_config`.
- The text-only Mac-tier fallback `Qwen/Qwen2.5-Coder-0.5B-Instruct` completed a direct `run_full_training` Metal micro-run: one step, 39 tokens, and a training summary returned in 0.655 seconds.

The CLI Metal dead gate remains in place. The complete operator procedure will be added after host-aware Metal dispatch is wired.
