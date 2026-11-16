---
phase: 333
status: fixed
reviewer: antigravity
reviewed_at: 2026-11-13T08:14:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - crates/vox-populi/src/mens/hub_card/facts.rs
  - crates/vox-populi/src/mens/hub_sync.rs
  - crates/vox-populi/src/mens/serving/vllm_lora.rs
  - contracts/mens/training-presets.v1.yaml
  - templates/hub/model.md
findings:
  critical: 1
  warning: 0
  info: 1
  total: 2
---

# Phase 333 Code Review (Gated Training for Every Model Size)

### CR-333-01: Unguarded `spoke_route` re-export breaks `vox-populi` without `mens-hf-hub` (`vllm_lora.rs:17`)
- **Severity:** Critical
- **Location:** `crates/vox-populi/src/mens/serving/vllm_lora.rs:17`
- **Issue:** `crates/vox-populi/src/mens/serving/mod.rs:6-11` gates `pub mod spoke_route;` and `pub use spoke_route::resolve_spoke_adapter;` with `#[cfg(feature = "mens-hf-hub")]`, while `vllm_lora.rs:17` had an unguarded `pub use super::spoke_route::resolve_spoke_adapter;`. Any downstream crate depending on `vox-populi` without `mens-hf-hub` (such as `vox-cli-research`) failed compilation with `error[E0432]: unresolved import super::spoke_route`.
- **Fix Applied (see `333-REVIEW-FIX.md`):** Added `#[cfg(feature = "mens-hf-hub")]` to `pub use super::spoke_route::resolve_spoke_adapter;` in `vllm_lora.rs:17`.

### IN-333-01: Clean Areas Verified
- `crates/vox-populi/src/mens/hub_sync.rs` resolves Hugging Face credentials strictly via Clavis (`vox_secrets::resolve_secret(SecretId::HuggingFaceToken)`), enforces `ensure_pull_allowed` under `VOX_MENS_NO_DOWNLOAD` / `HF_HUB_OFFLINE`, and gates complete-model staging on verified passing gate status.
- `crates/vox-populi/src/mens/hub_card/facts.rs` validates strict schema headers (`vox_eval_item_checkpoint.v1` + `PROMPT_PROTOCOL`) and downsamples telemetry cleanly while clamping bounds (`n - (64 - i)`).
- Cumulative `total_tokens` / `micro_steps_executed` checkpoint fields and plain-language card templates (`0` occurrences of `"ungated"` / `--allow-ungated`) remain verified in `333-VERIFICATION.md`.
