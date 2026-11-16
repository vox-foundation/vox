# Phase 425 Context: Stage D Pre-Flight Code Seams & Zero-Copy Cache Prefetch CLI

- **Workstream:** `mens-nov2026-v73` (`Milestone v7.3`)
- **Goal:** Close all six pre-flight code seams required before live downloading, calibrating, training, and evaluating `Qwen/Qwen3.5-{2B,4B,9B}` and `Qwen/Qwen3.8-27B` on Apple Silicon Metal.
- **Disjoint File Ownership:**
  - `425-A`: `crates/vox-populi/src/mens/hub.rs`, `crates/vox-populi/src/mens/hub_card/facts.rs`
  - `425-B`: `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs`, `crates/vox-ml-cli/src/commands/mens/size_train.rs`, `crates/vox-ml-cli/src/commands/mens/hub.rs`
  - `425-C`: `crates/vox-populi/src/mens/tensor/size_select.rs`
- **Build Policy:** Subagents do not run concurrent `cargo` builds; coordinator runs `rustfmt --edition 2024`, targeted `cargo test`, and one release build (`vox-ml-cli --features mens-candle-metal`).
