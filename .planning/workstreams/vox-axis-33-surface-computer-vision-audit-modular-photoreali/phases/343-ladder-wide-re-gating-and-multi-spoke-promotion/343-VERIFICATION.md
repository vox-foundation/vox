---
phase: 343-ladder-wide-re-gating-and-multi-spoke-promotion
status: passed_with_deferred_gpu_runs
verified: 2026-11-13
---

# Phase 343 Verification: Ladder-Wide Re-Gating & Multi-Spoke Promotion

## Automated Checks Run
- `timeout 900s cargo test -p vox-populi --features mens-hf-hub --lib -- hub_repo hub_sync domain_router domain_profiles`
  - Result: `70 passed; 0 failed; 1 ignored; 0 measured; 360 filtered out`
- `timeout 900s cargo test -p vox-ml-cli --features gpu --lib -- commands::mens::hub`
  - Result: `7 passed; 0 failed; 0 ignored; 0 measured; 485 filtered out`

## Deferred Manual / GPU & Hub Gates
- Live 128-item collateral GPU evaluation, `32B` cloud/quantized run receipts, and remote Hugging Face `--push` promotion require operator GPU runtime and Clavis `HF_TOKEN`.
