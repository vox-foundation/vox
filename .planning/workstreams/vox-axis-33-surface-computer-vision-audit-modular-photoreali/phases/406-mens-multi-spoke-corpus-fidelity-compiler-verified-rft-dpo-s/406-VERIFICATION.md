---
phase: 406-mens-multi-spoke-corpus-fidelity-compiler-verified-rft-dpo-s
verified: 2026-11-13T06:56:49Z
status: passed
score: 6/6 must-haves verified
---

# Phase 406 Verification Report

**Phase:** `406-mens-multi-spoke-corpus-fidelity-compiler-verified-rft-dpo-s`
**Requirement Set:** `MENS-CORPUS-406-01..06`
**Status:** `passed` (`Phase 406 Code Reviewer` verdict: `APPROVE`)

## Verified Truths

| # | Requirement | Status | Evidence |
|---|---|---|---|
| 1 | `MENS-CORPUS-406-01` | passed | `cargo test -p vox-corpus --lib extract_docs` (`28/28` passed in `1.24s`, incl. `qa_prompt_tracks_frontmatter_title_breadcrumbs_and_mixed_mode`, `code_blocks_skip_retired_syntax_and_annotate_compiler_check`, `code_blocks_ignore_hash_comments_inside_other_fences`, `qa_sections_drop_retired_inline_vox_fence_and_reject_traversal_link`) |
| 2 | `MENS-CORPUS-406-02` | passed | `cargo test -p vox-corpus --lib dpo` (`12/12` passed in `0.01s`, incl. `generate_dpo_skips_unparseable_and_retired_chosen`) & `extract_vox` (`10/10` pre-remediation; `part_walk.rs` untouched afterward) |
| 3 | `MENS-CORPUS-406-03` | passed | `cargo test -p vox-populi --features mens --lib spoke_validate` (`10/10` passed in `0.01s`, incl. `every_shipped_mix_declares_explicit_include_lanes`, `flags_mix_config_missing_include_lanes`, `flags_invalid_yaml_and_whitespace_only_include_lanes`) |
| 4 | `MENS-CORPUS-406-04` | passed | `cargo test -p vox-plugin-mens-candle-metal --lib` (`token_norm_window_scale` `1/1`, `packed_mask` `2/2`, `flush_pending_grads` `1/1`, `packed_forward` `2/2`, `checkpointed_packed` `1/1`) |
| 5 | `MENS-CORPUS-406-05` | passed | `cargo test -p vox-plugin-mens-candle-core --lib effective_alpha` (`1/1`) & `cargo test -p vox-plugin-mens-candle-metal --lib` (`effective_lora_alpha` `1/1`, `from_weight_frozen` `1/1`) |
| 6 | `MENS-CORPUS-406-06` | passed | `cargo test -p vox-ml-cli --features gpu --lib corpus_threshold` (`5/5` passed in `0.01s`, incl. `direct_relative_jsonl_file_path_passes`, `relative_training_contract_override_passes`) |
