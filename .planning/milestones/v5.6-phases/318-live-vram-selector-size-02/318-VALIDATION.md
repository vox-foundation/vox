---
phase: "318"
slug: "live-vram-selector-size-02"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-29"
---

# Phase 318 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Requirement: SIZE-02. Reviewed committed tests in `07f504dfb..74129dc11` (HEAD `74129dc11`). The dirty working tree was not used.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (lib/bin unit tests) |
| **Config file** | `.config/nextest.toml` (phase verify blocks use `cargo test`, not nextest) |
| **Quick run command** | Feature-qualified filters below. A default filter that prints `0 tests` and exits 0 is not a pass. |
| **Full suite command** | `cargo test -p vox-populi --lib spoke_base_resolver --features mens -- --test-threads=8` and `cargo test -p vox-ml-cli --lib training_selection -- --test-threads=8` and `cargo test -p vox-ml-cli --lib run_train --features gpu -- --test-threads=8` and `cargo test -p vox-ml-cli --lib local_card --features gpu -- --test-threads=8` and `cargo test -p vox-orchestrator --lib auto_select -- --test-threads=8` and `cargo test -p vox-cli --lib chat_routing_ -- --test-threads=8` and `cargo test -p vox-gui --bin vox-gui models_host -- --test-threads=8` |
| **Estimated runtime** | Minutes. The `mens` and `gpu` compiles dominate. |

`vox-populi` default features are empty. `spoke_base_resolver` is `cfg(feature = "mens")`. `vox-ml-cli` default is `mens-base`. `train_arm` and schola `run_train` are `cfg(feature = "gpu")`. `vox-orchestrator` default includes `runtime`, which is what `select_from_free_mb` calls. `vox-gui` has no library target.

---

## Sampling Rate

- **After every task commit:** Run that task's feature-qualified command in the map. Discard a `0 tests` result.
- **After every plan wave:** Run the full suite command.
- **Before `/gsd-verify-work`:** Full suite must be green, including `--features mens` and `--features gpu` where the map requires them.
- **Max feedback latency:** Minutes, bounded by the `mens` or `gpu` compile.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 318-01-01 | 01 | 1 | SIZE-02 | T-318-02 | 70000 MB selects 32B QLoRA. A second reading of 25000 loads 14B and names 32B, 14B, and 25000. 70000 stays on 32B. 50000 selects the 14B QLoRA floor, not the LoRA row. | unit | `cargo test -p vox-populi --lib live_pick_then_preload_reread_names_original_loaded_and_free_mb --features mens -- --test-threads=8` | ✅ | ✅ green |
| 318-01-02 | 01 | 1 | SIZE-02 | T-318-02 | 1.7B and 4B never win. Below 2000 MB errors. Explicit 32B at 25000 MB errors instead of becoming 14B. Explicit 8B below 12000 MB errors. `small_code_default` at 100000 MB stays the 8B pin. | unit | `cargo test -p vox-populi --lib spoke_base_resolver --features mens -- --test-threads=8` | ✅ | ✅ green |
| 318-01-03 | 01 | 1 | SIZE-02 | T-318-01 | CUDA `free_gb` wins over a Metal reading. Metal `memory_free_mb` is used only when CUDA is absent. Both absent is an error. Tests never call the live reader. | unit | `cargo test -p vox-populi --lib free_mb_from_readings --features mens -- --test-threads=8` | ✅ | ✅ green |
| 318-02-01 | 02 | 2 | SIZE-02 | T-318-04 | At 25000 MB every live spoke trains Qwen3-14B on `qwen3_24g`. At 16384 MB every spoke trains Qwen3-8B on `qwen3_16g`. Capability tags stay on `pick_base`. A missing reading is an error. | unit | `cargo test -p vox-ml-cli --lib training_selection -- --test-threads=8` | ✅ | ✅ green |
| 318-02-02 | 02 | 2 | SIZE-02 | T-318-04 | Local spoke resolution and the train stage call `read_accelerator_free_mb` only after `selection_requires_free_reading`. They do not copy total VRAM or substitute `16384`. | unit | `cargo test -p vox-ml-cli --lib resolve_local_spoke_base_does_not_invent_a_vram_reading --features gpu -- --test-threads=8` | ✅ | ✅ green |
| 318-02-03 | 02 | 2 | SIZE-02 | T-318-03 / T-318-04 | An automatic 32B pick at 25000 MB loads as 14B QLoRA and names both sizes and the reading. An explicit 32B at 25000 MB does not load. `run_train` checks that before `select_mens_backend`. | unit | `cargo test -p vox-ml-cli --lib run_train --features gpu -- --test-threads=8` | ✅ | ✅ green |
| 318-03-01 | 03 | 2 | SIZE-02 | T-318-05 | 25000 MB is the 14B QLoRA pin. 1500 MB is an error. 50000 MB stays 14B. 70000 MB is the 32B pin. The test injects the reading. | unit | `cargo test -p vox-orchestrator --lib select_from_free_mb -- --test-threads=8` | ✅ | ✅ green |
| 318-03-02 | 03 | 2 | SIZE-02 | T-318-05 | The directory-tier function, its re-exports, the run-directory ids, and the retired fallback id are absent. The live selector calls `read_accelerator_free_mb` then `select_from_free_mb`. | unit | `cargo test -p vox-orchestrator --lib auto_select -- --test-threads=8` | ✅ | ✅ green |
| 318-03-03 | 03 | 2 | SIZE-02 | T-318-06 | Injected auto-local chat uses the 14B pin. An injected error does not build a config. The GUI mapper returns the selection or the error and the command still calls `select_optimal_local_model_for_host`. | unit | `cargo test -p vox-cli --lib chat_routing_ -- --test-threads=8 && cargo test -p vox-gui --bin vox-gui models_host -- --test-threads=8` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

Runs this audit accepted, from the feature-qualified executions of the committed tests. This audit did not re-execute Cargo.

| Command | Result |
|---------|--------|
| `cargo test -p vox-populi --lib spoke_base_resolver --features mens -- --test-threads=8` | 37 passed. Covers 318-01-01, 318-01-02, and 318-01-03, including `preload_step_down_uses_the_same_overlay_as_the_pick`. |
| `cargo test -p vox-populi --lib spoke_base_resolver` (no `mens`) | 0 tests, exit 0. Not a pass. |
| `cargo test -p vox-ml-cli --lib training_selection -- --test-threads=8` | 9 passed on default `mens-base`. Covers 318-02-01 and the pipeline / local-resolver source order. |
| `cargo test -p vox-ml-cli --lib run_train --features gpu -- --test-threads=8` | 11 passed. Covers 318-02-03 (`apply_preload_free_mb_*` and `run_train_checks_preload_before_select_mens_backend`). |
| `cargo test -p vox-ml-cli --lib local_card --features gpu -- --test-threads=8` | 3 passed. Records the loaded size and preset on the adapter card. |
| `run_train`, `apply_preload_free_mb`, `resolve_local_spoke_base_does_not_invent_a_vram_reading`, `local_card` without `gpu` | 0 tests, exit 0. Not a pass. |
| `cargo test -p vox-orchestrator --lib select_from_free_mb -- --test-threads=8` | 1 passed (`318-03-SUMMARY`). `auto_select.rs` is unchanged from `00d75032f` through `74129dc11`. |
| `cargo test -p vox-orchestrator --lib auto_select -- --test-threads=8` | 3 passed. Same unchanged blob. |
| `cargo test -p vox-cli --lib chat_routing_ -- --test-threads=8` | 2 passed. `chat.rs` unchanged after `00d75032f`. |
| `cargo test -p vox-gui --lib models_host` | Exit 101. `vox-gui` has no library target. Not a pass. |
| `cargo test -p vox-gui --bin vox-gui models_host -- --test-threads=8` | 2 passed. `models_host.rs` unchanged after `00d75032f`. |

318-02-02's negative assertions (no `get_system_vram`, no numeric fallback inside `resolve_local_spoke_base`) live in the `gpu`-gated test. `318-02-SUMMARY` recorded that filter as 1 passed. The post-fix function body still has neither string, and the default `mens-base` test `free_memory_is_read_only_after_the_selection_predicate` (inside the 9) passed against the same committed file.

T-318-SC is accepted. These plans add no package install, so it is not a coverage gap. D-12 forbids unit tests from calling `read_accelerator_free_mb`; the production call is source-sliced and the pick is tested with injected readings.

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

- [x] No new test file. The SIZE-02 tests are already in the committed crates listed above.

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency: minutes, bounded by the `mens` or `gpu` compile
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-10-29

New tests required: no.

## Validation Audit 2026-10-29

| Metric | Count |
|---|---|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |
