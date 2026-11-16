---
phase: "317"
slug: "size-ladder-size-01"
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-28"
---

# Phase 317 — Validation Strategy

> Per-phase validation contract for SIZE-01. Tests read `mens/config/gpu-specs.yaml`. They do not call Hugging Face, download weights, train, or touch the mesh.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | built-in `cargo test` (`#[cfg(feature = "mens")]` on the resolver; `mens-train` on the contract binary; `gpu` on `vox-ml-cli`) |
| **Config file** | none |
| **Quick run command** | `cargo test -p vox-populi --features mens --lib -- resolve_qwen3_size unmeasured_pins qwen3_code_fail_closed_below_floor qwen3_code_24g_returns_14b_qlora qwen3_code_48g_prefers_unquantized_14b unmeasured_floor_and_missing_floor` |
| **Full suite command** | the quick command, then `cargo test -p vox-populi --features mens-train --test training_presets_yaml_contract -- gpu_specs_qwen3_train_bases_floor_mb_pins_against_real_weight_size gpu_specs_agentic_default_16g_floor_matches_training_presets_default`, then `cargo test -p vox-ml-cli --features gpu --lib -- resolve_qwen3_size_rejects_unknown_spellings resolve_base_pin_leaves_capability_tag_unpinned size_name_1_7b_records_the_pinned_unmeasured_base size_name_4b_records_the_pinned_unmeasured_base base_resolution_size_name_uses_train_bases base_resolution_full_pin_passes_through base_resolution_unknown_or_ambiguous_is_unverifiable resolve_qwen3_size_errors_when_qwen3_code_has_two_qlora_pins` |
| **Estimated runtime** | ~70 seconds warm |

---

## Sampling Rate

- **After every task commit:** Run that task's automated command
- **After every plan wave:** Run every command in the map
- **Before `/gsd-verify-work`:** Every command in the map must be green, with at least one passing test
- **Max feedback latency:** 70 seconds

`spoke_base_resolver` is behind `--features mens`. The contract binary is behind `--features mens-train`. `vox-ml-cli` filters must be `cargo test -p vox-ml-cli --features gpu --lib -- <filter>`. A line that says `0 passed` is a failed run: the plan's bare `cargo test -p vox-populi --lib` exits 0 with an empty harness.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 317-01-01 | 01 | 1 | SIZE-01 | T-317-01, T-317-02 | `1.7B` resolves to `Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e`, apache-2.0, `floor_measured` false, and `floor_mb` none. `pick_base` at 16384 MB stays on the 8B pin and at 1000 MB errors. A guessed floor on an unmeasured row does not load | unit | `cargo test -p vox-populi --features mens --lib -- resolve_qwen3_size_1_7b_pin_and_16g_pick_unchanged unmeasured_floor_and_missing_floor_do_not_load` | ✅ | ✅ green |
| 317-01-01 | 01 | 1 | SIZE-01 | T-317-01 | Naming `1.7B` with no snapshot writes `Qwen3-1.7B-70d244cc86cc.json` with that revision and does not call the scorer | integration | `cargo test -p vox-ml-cli --features gpu --lib -- size_name_1_7b_records_the_pinned_unmeasured_base` | ✅ | ✅ green |
| 317-01-02 | 01 | 1 | SIZE-01 | T-317-01, T-317-03 | `0.6B`, `4B`, `8B`, `14B`, and `32B`, plus the `Qwen3-` and `Qwen/Qwen3-` spellings of 4B and 8B, return the locked qwen3_code QLoRA pin, a 40-hex revision, and apache-2.0. 4B is unmeasured with no `floor_mb`. A non-40-hex revision or a missing license errors | unit | `cargo test -p vox-populi --features mens --lib -- resolve_qwen3_size_covers_the_dense_ladder resolve_qwen3_size_rejects_bad_revision_and_missing_license` | ✅ | ✅ green |
| 317-01-02 | 01 | 1 | SIZE-01 | T-317-02 | The 1.7B and 4B pins appear only on `qwen3_code`. Measured floors stay 2000, 12000, 20000, 44000, and 60000. `pick_base` still errors below 2000 MB, returns 14B QLoRA at 24000 MB, and 14B LoRA at 49152 MB | unit | `cargo test -p vox-populi --features mens --lib -- unmeasured_pins_are_only_on_qwen3_code qwen3_code_fail_closed_below_floor qwen3_code_24g_returns_14b_qlora qwen3_code_48g_prefers_unquantized_14b` | ✅ | ✅ green |
| 317-01-02 | 01 | 1 | SIZE-01 | T-317-01 | `2B`, `Qwen3-7B`, and `Qwen/8B` error. `qwen3_16g` stays unpinned. A full `hf_id@sha` passes through. `not-a-model` stays unverifiable. Two qwen3_code QLoRA pins for one size error | unit | `cargo test -p vox-ml-cli --features gpu --lib -- resolve_qwen3_size_rejects_unknown_spellings resolve_base_pin_leaves_capability_tag_unpinned base_resolution_full_pin_passes_through base_resolution_unknown_or_ambiguous_is_unverifiable resolve_qwen3_size_errors_when_qwen3_code_has_two_qlora_pins` | ✅ | ✅ green |
| 317-01-02 | 01 | 1 | SIZE-01 | T-317-01 | Naming `4B` with no snapshot writes `Qwen3-4B-1cfa9a720891.json` with revision `1cfa9a7208912126459214e8b04321603b3df60c` and does not call the scorer | integration | `cargo test -p vox-ml-cli --features gpu --lib -- size_name_4b_records_the_pinned_unmeasured_base` | ✅ | ✅ green |
| 317-01-02 | 01 | 1 | SIZE-01 | T-317-02 | The weight-size contract skips `floor_measured` false and still checks measured Qwen3 floors. The agentic 16 GB floor still matches the training-presets default | integration | `cargo test -p vox-populi --features mens-train --test training_presets_yaml_contract -- gpu_specs_qwen3_train_bases_floor_mb_pins_against_real_weight_size gpu_specs_agentic_default_16g_floor_matches_training_presets_default` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

Re-run on 2026-10-28: resolver and `pick_base` filters, 8 passed; contract filters, 2 passed; `vox-ml-cli` filters, 8 passed. No new tests. `resolve_qwen3_size_*`, `unmeasured_pins_are_only_on_qwen3_code_and_measured_floors_stay`, and the existing `pick_base` tests already close SIZE-01. `attribution_required: false` is on the same checked-in 1.7B and 4B rows those tests load (`gpu-specs.yaml` rows 252–253). The assertions cover the pin, license, `floor_measured`, and `floor_mb`. T-317-SC accepts that this phase installs nothing.

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. Tests live in `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs`, `crates/vox-populi/tests/training_presets_yaml_contract.rs`, `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs`, and `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs`.

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 70s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-10-28

## Validation Audit 2026-10-29

| Metric | Count |
|---|---|
| Gaps found | 0 |
| Resolved | 7 |
| Escalated | 0 |
