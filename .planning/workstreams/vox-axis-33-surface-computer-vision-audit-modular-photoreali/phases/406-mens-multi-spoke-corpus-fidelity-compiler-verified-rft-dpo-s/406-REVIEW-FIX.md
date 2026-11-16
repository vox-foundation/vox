---
phase: 406
status: fixed
fixed_at: 2026-11-13T06:50:00Z
---

# Phase 406 Code Review Fixes

## Fixed Items

1. **`WR-01` (`crates/vox-corpus/src/corpus/extract_docs.rs`, `crates/vox-corpus/src/corpus/corpus_readiness.rs`):**
   - Guarded relational `.vox` chunk injection in `flush_qa_section` with `crate::corpus::corpus_readiness::check_vox_retired_and_enforcement_rules(&vox_code).is_empty()` and wired `resolve_doc_confine_root(&config.root)` (`confine_root.as_ref().is_none_or(|root| can.starts_with(root))`), eliminating the dead-code compiler warning and preventing out-of-root traversal.
   - Added `fn ... ) -> ` retired return-type detection to `check_vox_retired_and_enforcement_rules` in `corpus_readiness.rs` so both QA inline fences (`qa_sections_drop_retired_inline_vox_fence_and_reject_traversal_link`) and relational linked `.vox` files reject retired C/Rust arrow return syntax.
   - Added unit test `relational_chunking_skips_retired_linked_vox_file`.
   - Formatted with `rustfmt --edition 2024 crates/vox-corpus/src/corpus/extract_docs.rs` and `rustfmt --edition 2024 crates/vox-corpus/src/corpus/corpus_readiness.rs`.
2. **`WR-02` (`crates/vox-corpus/src/corpus/dpo.rs`):**
   - Added `unwrap_optional_vox_fence` so markdown-fenced ` ```vox\n...\n``` ` (and ` ```\n...\n``` `) responses compile and mutate cleanly inside `build_verified_vox_rejected`, preserving fence wrapping on `rejected`.
   - Added `fails_vox_frontend` and verified both AST-mutated and keyword-degraded `rejected` candidates fail Vox frontend checks.
   - Added unit test `dpo_generator_handles_markdown_fenced_vox_responses`.
   - Formatted with `rustfmt --edition 2024 crates/vox-corpus/src/corpus/dpo.rs`.
3. **`IN-01` (`crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/finalize.rs`):**
   - Removed `#[cfg_attr(not(test), allow(dead_code))]` from `effective_lora_alpha` and logged `effective_alpha` via `train_log::info` during QLoRA adapter manifest v3 finalization.
   - Formatted with `rustfmt --edition 2024 crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/finalize.rs`.

## Verification

- `timeout 900s cargo test -p vox-corpus --lib extract_docs`: **28 passed; 0 failed** (`1.04s`, 0 warnings).
- `timeout 900s cargo test -p vox-corpus --lib dpo`: **12 passed; 0 failed** (`0.08s`).
- `timeout 900s cargo test -p vox-plugin-mens-candle-metal --lib effective_lora_alpha`: **1 passed; 0 failed** (`0.00s`).
