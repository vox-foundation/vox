---
phase: 317-size-ladder-size-01
reviewed: 2026-10-29T03:58:33Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - mens/config/gpu-specs.yaml
  - crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
  - crates/vox-populi/tests/training_presets_yaml_contract.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
status: clean
---

# Phase 317: Code Review Report

**Reviewed:** 2026-10-29T03:58:33Z
**Depth:** standard
**Files Reviewed:** 5
**Status:** clean

## Summary

Reviewed the size-ladder commits (`2d6aea459`, `3592ec1e6`) against the locked contract. Planning docs in the later commits were not treated as product source.

The two new `train_bases.qwen3_code` rows are `Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e` and `Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c`. Each is QLoRA only, `apache-2.0`, `attribution_required: false`, omits `floor_mb`, and sets `floor_measured: false`. Those hf ids appear on no other tag, including `small_code_default` and `strong_code_default`. Existing measured floors stay 2000, 12000, 20000, 44000, and 60000.

`TrainBase.floor_mb` is `Option<u32>` and `floor_measured` defaults true. `parse_overlay` rejects a measured row with no floor and an unmeasured row that carries one. `pick_base` only considers rows that are measured and whose floor fits, then still takes the largest floor, so 16384 MB stays on the 8B pin, 1000 MB still errors, and the 24 GB / 48 GB winners are unchanged. `resolve_qwen3_size` accepts only the six exact size spellings (bare, `Qwen3-`, and `Qwen/Qwen3-`), requires one distinct `qwen3_code` QLoRA pin, a 40-character lowercase hex revision, and a recorded license. Phase 316's `resolve_base_pin` calls it for those spellings and `run_eval_reasoning_compare_with` propagates the error before detection. Capability tags such as `qwen3_16g` stay on the old scan.

All reviewed files meet quality standards. No issues found.

---

_Reviewed: 2026-10-29T03:58:33Z_
_Reviewer: gsd-code-reviewer_
_Depth: standard_
