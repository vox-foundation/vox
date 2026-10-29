---
phase: 318-live-vram-selector-size-02
verified: 2026-10-29T22:03:14Z
status: passed
score: 12/12 must-haves verified
behavior_unverified: 0
overrides_applied: 0
covered_files:
  - .planning/phases/318-live-vram-selector-size-02/318-01-PLAN.md
  - .planning/phases/318-live-vram-selector-size-02/318-01-SUMMARY.md
  - .planning/phases/318-live-vram-selector-size-02/318-02-PLAN.md
  - .planning/phases/318-live-vram-selector-size-02/318-02-SUMMARY.md
  - .planning/phases/318-live-vram-selector-size-02/318-03-PLAN.md
  - .planning/phases/318-live-vram-selector-size-02/318-03-SUMMARY.md
  - .planning/phases/318-live-vram-selector-size-02/SUMMARY.md
  - crates/vox-cli/src/commands/chat.rs
  - crates/vox-gui/src/commands/models_host.rs
  - crates/vox-ml-cli/Cargo.toml
  - crates/vox-ml-cli/src/commands/mens/pipeline.rs
  - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
  - crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs
  - crates/vox-ml-cli/src/commands/mens/training_selection.rs
  - crates/vox-ml-cli/src/commands/schola/train/run_train.rs
  - crates/vox-orchestrator/src/models/auto_select.rs
  - crates/vox-orchestrator/src/models/mod.rs
  - crates/vox-orchestrator/src/models/select.rs
  - crates/vox-populi/src/mens/cloud/pipeline_dispatch.rs
  - crates/vox-populi/src/mens/tensor/adapter_card.rs
  - crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
covered_digest: "v3:sha256:e49d3c185ba321ee0309185a22b1fc18dcac3de5c359b52389e9946240040964"
decision_coverage:
  skipped: true
  honored: 0
  total: 0
  not_honored: []
  reason: "no trackable decisions"
---

# Phase 318: Live VRAM Selector Verification Report

**Phase Goal:** One selector reads current free memory, titrates to the largest Qwen3 size that fits, and picks the hub and its spokes together. Training, local serve, and chat all use it.
**Verified:** 2026-10-29T22:03:14Z
**Status:** passed
**Re-verification:** No — initial verification
**Range:** `07f504dfb..74129dc11` (HEAD `74129dc11`). Phase source files had no dirty overlay.

## Goal Achievement

### Observable Truths

Plan `must_haves.truths` are the scored rows (12). Each roadmap success criterion and each plan `<success_criteria>` block is covered by those rows; none is dropped.

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | An injected free-memory reading selects the largest measured `qwen3_code` QLoRA `floor_mb` that fits, by calling `pick_base` on that filtered list. | ✓ VERIFIED | `select_qwen3_code_qlora_from_overlay` filters methods to exact `qlora` and calls `pick_base`. `live_pick_then_preload_reread_names_original_loaded_and_free_mb` passed: 70000 → 32B, floor 60000, preset `qwen3_96g`; 50000 → 14B QLoRA floor 20000, preset `qwen3_24g` (not the LoRA row). |
| 2 | 1.7B and 4B stay `floor_measured: false` with no usable floor and cannot win. The 14B LoRA row cannot win. Nothing that fits, including a reading below the 0.6B floor, is an error. | ✓ VERIFIED | `pick_base` keeps only `floor_measured && floor_mb <= reading`. `measured_qlora_ladder_and_unmeasured_sizes_fail_closed` passed: 15000 → 8B floor 12000, 5000 → 0.6B floor 2000, 1500 errors, 100000 is 32B and the id contains neither 1.7B nor 4B. |
| 3 | A second injected reading steps an automatic pick down to the next smaller measured `qwen3_code` QLoRA row that fits, and the result names the original size, the loaded size, and that reading. A second reading that still fits does not step down. If the 0.6B floor no longer fits, the result is an error. | ✓ VERIFIED | `confirm_before_load` keeps the pick when `floor_mb <= free_mb_at_load`; otherwise it calls `select_qwen3_code_qlora_from_overlay` on the same overlay (WR-03). The executed step-down test: 70000 then 25000 yields original 32B, loaded 14B, preset `qwen3_24g`, `free_mb` 25000; 70000 then 70000 stays 32B. The fail-closed test: 70000 then 1500 errors and the error does not name 1.7B or 4B. `run_train` logs `original_size`, `loaded_size`, and `free_mb`. |
| 4 | An explicit `resolve_qwen3_size` spelling is refused when it is unmeasured or its QLoRA floor is above the reading, including on the second reading, and is not replaced by a smaller size. | ✓ VERIFIED | `explicit_qwen3_size_for_free_mb` calls `resolve_qwen3_size`, errors on `floor_measured: false` or `floor_mb > free_mb`, and does not call the largest-fit picker. Explicit `confirm_before_load` errors instead of substituting. `explicit_qwen3_size_refuses_unmeasured_and_over_floor` passed, including 32B at 25000 (error does not contain the 14B pin) and Explicit 8B at 11999 (error does not contain 0.6B). |
| 5 | CUDA free memory is the free field from `query_nvidia_smi_vram`. Metal free memory is `monitor_metal` `memory_free_mb`. Tests pass those values in. They do not probe a GPU. | ✓ VERIFIED | `query_nvidia_smi_vram` parses `memory.free` into `VramInfo.free_gb`. `monitor_metal` sets `memory_free_mb` to `recommendedMaxWorkingSetSize` minus `currentAllocatedSize`. `read_accelerator_free_mb` uses CUDA when present, else Metal, then `free_mb_from_readings`. It does not call `get_system_vram_info`. The three `free_mb_from_readings_*` tests passed (CUDA wins over Metal 100, Metal-only 4096, both absent errors). |
| 6 | With no CLI model and one injected free-memory reading, every live spoke gets the same `qwen3_code` QLoRA hf id and the preset for that size. The spoke yaml preset does not stay `qwen3_16g` when the pick is 14B. | ✓ VERIFIED | `resolve_training_selection` calls `select_qwen3_code_qlora_for_free_mb` when both the CLI model and the env pin are absent, and sets `preset` from that pick unless `cli_preset` is set. `unnamed_size_uses_one_qwen3_code_size_for_every_spoke` asserts vox-lang, rust, tool-selection, and argument-generation at 25000 are the 14B pin on `qwen3_24g`, and at 16384 the 8B pin on `qwen3_16g`. The executed picker test establishes those pins. |
| 7 | A caller that passes `strong_code_default` or `small_code_default` still uses `pick_base` on that tag. Those tags are not retuned. An explicit spelling is refused when it does not fit and is not stepped down. | ✓ VERIFIED | Non-spelling names go to `resolve_base_model`. `capability_tags_stay_on_resolve_base_model` asserts both tags still return the 8B pin. The same training test asserts `small_code_default` at 100000 stays 8B on `qwen3_16g`, `strong_code_default` at 16384 stays 8B, and `32B` / `1.7B` error. No new rows were added to those tags. |
| 8 | Unreadable free memory on the unnamed-size path is an error. The local train resolver does not invent a reading. | ✓ VERIFIED | Absent model and absent reading returns `Err` (`unnamed_size_without_a_reading_is_an_error`). `resolve_local_spoke_base` and pipeline `Train` call `read_accelerator_free_mb` only after `selection_requires_free_reading`, and propagate `Err`. Neither substitutes a constant or `get_system_vram_info`. |
| 9 | Immediately before schola `run_train` loads, an automatic pick is revised with a new free-memory reading. The loaded model and preset follow that revision. An explicit size that no longer fits is an error and is not replaced. | ✓ VERIFIED | `apply_preload_free_mb` is called at line 397; `select_mens_backend` is at line 456. `?` returns before plugin heal and before the backend select. Dogfood passes model `None` and `automatic_qwen3` true. A background spawn re-executes the CLI without `--background`, so the child hits the same path. `apply_preload_free_mb_refuses_an_explicit_32b_pin` asserts the explicit 32B hf id at 25000 does not yield the 14B id. |
| 10 | `select_optimal_local_model` calls `select_qwen3_code_qlora_for_free_mb` behind the existing `runtime` feature. An injected 25000 MB reading returns the 14B QLoRA pin. There is no second ladder in `auto_select.rs`. | ✓ VERIFIED | `select_from_free_mb` calls the populi selector under `cfg(feature = "runtime")` and maps `hf_id`. `select_optimal_local_model` reads free memory, then calls `select_from_free_mb`, and does not call `detect_hardware_vram_gb`. Orchestrator default features include `runtime` (`vox-populi/mens`). The executed step-down path returns the 14B pin at 25000. `select_tier_for_vram` is absent from the commit. |
| 11 | The directory-tier function named in 318-CONTEXT.md, and its re-exports, are gone. Unreadable free memory is an error from `select_optimal_local_model`. | ✓ VERIFIED | `git grep select_tier_for_vram 74129dc11` finds no Rust definition. `auto_select.rs` does not contain `mens/runs/qwen3_27b_metal_check` or `vox-mens-8b-v0.6`. `models/mod.rs` and `models/select.rs` re-export the live functions only. A failed `read_accelerator_free_mb` is returned with `?` and no substitute id. |
| 12 | Chat auto-local and the GUI models host keep calling `select_optimal_local_model`. Tests inject the pick or the free-memory reading. They do not probe a GPU. | ✓ VERIFIED | `resolve_chat_config` passes `None`, so auto-local calls `select_optimal_local_model`. `run` uses `?`. `get_auto_model_recommendation` calls `select_optimal_local_model_for_host` and is registered in `vox-gui` `main.rs`. `chat_routing_auto_and_mens` injects the 14B pin and asserts `Err("unreadable")` does not build a config. The host test injects Ok and Err and checks the command source still names the host selector. |

**Score:** 12/12 truths verified (0 present, behavior-unverified)

### Roadmap success criteria

| # | Criterion | Status | Covered by |
| --- | --- | --- | --- |
| 1 | The selector reads current free memory (Metal live budget, CUDA free VRAM), not total memory, and picks the largest ladder size whose predicted footprint fits. | ✓ VERIFIED | Truths 1 and 5 |
| 2 | If free memory drops between the pick and the load, it steps down a size and says why, instead of loading and running out of memory. | ✓ VERIFIED | Truths 3 and 9. The log line is `pre-load free-memory revision` with original size, loaded size, and `free_mb`. Chat does not load weights; the load path is schola `run_train`. |
| 3 | Every spoke's base and preset follow the picked size. An explicit size overrides the pick, and a size that does not fit is refused rather than silently shrunk. | ✓ VERIFIED | Truths 4, 6, and 7 |
| 4 | The inference auto-selector and the training base pick both use this selector. The old fixed tiers that point at run directories that do not exist are gone. | ✓ VERIFIED | Truths 6, 10, and 11 |

### Plan success criteria

| Plan | Criterion | Status |
| --- | --- | --- |
| 318-01 | Injected readings select and step down only measured `qwen3_code` QLoRA rows through `pick_base`. Explicit spellings error when they do not fit. The free-memory adapter prefers the CUDA free field, then Metal `memory_free_mb`. | ✓ VERIFIED |
| 318-02 | Unnamed training picks one measured `qwen3_code` QLoRA size and its preset for every spoke. The load path steps that pick down or refuses an explicit size, and does not load when the smallest measured row no longer fits. | ✓ VERIFIED |
| 318-03 | Chat and the GUI host recommend the measured `qwen3_code` QLoRA size that fits free memory, or they return an error. The old directory tiers are not reachable. | ✓ VERIFIED |

### Required Artifacts

`verify.artifacts` returned `all_passed: true` for all three plans (exit 0). The listed paths match `74129dc11`.

| Artifact | Expected | Status | Details |
| --- | --- | --- | --- |
| `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs` | `Qwen3LivePick`, `select_qwen3_code_qlora_for_free_mb`, `confirm_before_load`, `explicit_qwen3_size_for_free_mb`, `free_mb_from_readings` | ✓ VERIFIED | All five exist and are called. |
| `crates/vox-ml-cli/src/commands/mens/training_selection.rs` | Unnamed size uses the live picker and that pick's preset | ✓ VERIFIED | Call at the absent-name branch. |
| `crates/vox-ml-cli/src/commands/schola/train/run_train.rs` | `confirm_before_load` before the train backend is selected | ✓ VERIFIED | Via `apply_preload_free_mb` at line 397, backend select at line 456. |
| `crates/vox-orchestrator/src/models/auto_select.rs` | `select_from_free_mb` and a `Result`-returning `select_optimal_local_model` | ✓ VERIFIED | Runtime-gated populi call. |
| `crates/vox-cli/src/commands/chat.rs` | auto-local uses the selector result and surfaces an error | ✓ VERIFIED | Production path calls the live function; tests inject. |
| `crates/vox-gui/src/commands/models_host.rs` | `get_auto_model_recommendation` still calls `select_optimal_local_model_for_host` | ✓ VERIFIED | Command body and `main.rs` registration. |

### Key Link Verification

`verify.key-links` exited 1 for every plan because each `from` is a symbol, not a relative file path (`Source file not found`). That result is unevaluable, not a missing call. Manual wiring against `74129dc11`:

| From | To | Via | Status | Details |
| --- | --- | --- | --- | --- |
| `select_qwen3_code_qlora_for_free_mb` | `pick_base` | filtered `qwen3_code` QLoRA rows | WIRED | `select_qwen3_code_qlora_from_overlay` builds that map and calls `pick_base`. |
| `confirm_before_load` | same picker | second reading, same overlay | WIRED | Step-down calls `select_qwen3_code_qlora_from_overlay`, not a second max-by-floor loop. WR-03 stopped the reload through `select_qwen3_code_qlora_for_free_mb` so the step-down uses the overlay the pick used. |
| `resolve_training_selection` | `select_qwen3_code_qlora_for_free_mb` | CLI model and env pin both absent | WIRED | Present. A non-empty `VOX_MENS_DEFAULT_MODEL` is an explicit name (CR-01) and does not step down. |
| schola `run_train` | `confirm_before_load` | `automatic_qwen3`, before `select_mens_backend` | WIRED | `apply_preload_free_mb` calls `confirm_before_load` before line 456. |
| `select_optimal_local_model` | `select_qwen3_code_qlora_for_free_mb` | `read_accelerator_free_mb` then `select_from_free_mb`, `runtime` | WIRED | One hop through `select_from_free_mb`. |
| `resolve_chat_config` auto-local | `select_optimal_local_model` | production passes no injected pick | WIRED | `resolve_chat_config` passes `None`. |
| `get_auto_model_recommendation` | `select_optimal_local_model_for_host` | command `Result` | WIRED | Direct call. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| --- | --- | --- | --- | --- |
| `spoke_base_resolver.rs` | `Qwen3LivePick` | `pick_base` on measured `qwen3_code` QLoRA rows from the workspace overlay | Yes | ✓ FLOWING |
| `free_mb_from_readings` | `u32` | CUDA `memory.free`, else Metal `memory_free_mb` | Yes | ✓ FLOWING |
| `training_selection.rs` | spoke `model` / `preset` | that pick when no name is given | Yes | ✓ FLOWING |
| `run_train` | loaded id, preset, `PreloadRevision` | second `read_accelerator_free_mb` then `confirm_before_load` | Yes | ✓ FLOWING |
| `auto_select.rs` | `selected_model_id` | the same pick's `hf_id` | Yes | ✓ FLOWING |
| `chat.rs` / `models_host.rs` | recommended model | `select_optimal_local_model` on the production path | Yes | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| --- | --- | --- | --- |
| 70000 picks 32B; 25000 steps to 14B and names both sizes and the reading; 70000 keeps 32B; 50000 is the 14B QLoRA row | `cargo test -p vox-populi --features mens --lib mens::tensor::spoke_base_resolver::tests::live_pick_then_preload_reread_names_original_loaded_and_free_mb -- --exact` | 1 passed, exit 0 | ✓ PASS |
| 1.7B and 4B cannot win; 1500 errors; 0.6B / 8B floors hold | `cargo test -p vox-populi --features mens --lib mens::tensor::spoke_base_resolver::tests::measured_qlora_ladder_and_unmeasured_sizes_fail_closed -- --exact` | 1 passed, exit 0 | ✓ PASS |
| Explicit 1.7B, 4B, and over-floor 32B error and are not replaced | `cargo test -p vox-populi --features mens --lib mens::tensor::spoke_base_resolver::tests::explicit_qwen3_size_refuses_unmeasured_and_over_floor -- --exact` | 1 passed, exit 0 | ✓ PASS |
| CUDA free wins; Metal is used when CUDA is absent; both absent errors | `cargo test -p vox-populi --features mens --lib mens::tensor::spoke_base_resolver::tests::free_mb_from_readings --` | 3 passed, exit 0 | ✓ PASS |

### Probe Execution

No probe script is declared in the phase plans or success criteria. Step 7c is not applicable.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| --- | --- | --- | --- | --- |
| SIZE-02 | 318-01, 318-02, 318-03 | One selector reads current free memory and titrates to the largest ladder size that fits, steps down before load, picks hub and spokes together, refuses an explicit size that does not fit, and replaces the fixed tiers in `auto_select.rs`. | ✓ SATISFIED | Truths 1–12. `REQUIREMENTS.md` still shows the checkbox pending; this report does not edit it. |

No requirement mapped to phase 318 is missing from the plans. MESH-01 is phase 319. SIZE-03 is phase 320.

### Decision Coverage

`check.decision-coverage-verify` returned: "No trackable decisions in CONTEXT.md." (`skipped: true`, `total: 0`). The CONTEXT decisions are prose bullets, so the checker had no ids to match. Status impact: none.

Manual check of the locked decisions against `74129dc11` (non-blocking):

| Decision | Honored |
| --- | --- |
| D-01 1.7B and 4B cannot win; no invented `floor_mb` | Yes. Truth 2. |
| D-02 CUDA `memory.free`, else Metal `memory_free_mb`; not the total-copy helper, wired-limit sysctl, or `vm_stat` | Yes. Truth 5. `detect_hardware_vram_gb` still contains `hw.memsize`. `select_optimal_local_model` does not call it. |
| D-03 Largest measured `qwen3_code` QLoRA floor wins | Yes. Truth 1. |
| D-04 Nothing fits, or an unreadable reading, fails closed | Yes. Truths 2, 8, and 11. |
| D-05 Extend `pick_base`. No second ladder in `auto_select.rs` | Yes. Truths 1 and 10. |
| D-06 One dense size; every spoke base and preset follow it | Yes. Truth 6. |
| D-07 Capability tags stay on `pick_base`. Call sites are unnamed training, chat auto-select, and the GUI host. Mesh stays phase 319. | Yes. Truths 7 and 12. |
| D-08 Remove `select_tier_for_vram` and the retired directory ids from that map | Yes. Truth 11. The model registry still lists those ids for a caller-typed string. Plan 03 keeps that chat branch. |
| D-09 Call populi through the existing `runtime` feature. No new orchestrator crate edge. | Yes. `crates/vox-orchestrator/Cargo.toml` is unchanged in the range. |
| D-10 Re-read before load; name original, loaded, and the reading; error if the smallest measured row does not fit | Yes. Truths 3 and 9. |
| D-11 Explicit spellings are not stepped down | Yes. Truth 4. |
| D-12 Tests inject readings. No GPU probe in unit tests. | Yes. Truths 5 and 12. |

### Test Quality Audit

| Test File | Linked Req | Active | Skipped | Circular | Assertion Level | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| `spoke_base_resolver.rs` tests | SIZE-02 | live pick, fail-closed, explicit refuse, free-memory adapter (all executed) | 0 | No. Expected pins are the checked-in hf ids, not values the test writes. | Value | PASS |
| `training_selection.rs` tests | SIZE-02 | unnamed size, capability tags, missing reading | 0 | No | Value | PASS |
| `run_train.rs` tests | SIZE-02 | step-down, explicit refuse, nothing-fits, source order before `select_mens_backend` | 0 | The step-down test also compares to a fresh pick at 25000 and asserts the 14B token. | Value | PASS |
| `auto_select.rs` tests | SIZE-02 | `select_from_free_mb(25000)` is the 14B pin; directory-tier name absent | 0 | No | Value | PASS |
| `chat.rs` / `models_host.rs` tests | SIZE-02 | injected 14B pin and injected `Err` | 0 | No | Value for the injected result; the live probe is not called (D-12) | PASS |

**Disabled tests on requirements:** 0. Two pre-existing `train_arm` tests are `cfg(feature = "hw-probe-live-test")` so the default suite does not spawn `nvidia-smi` or a Metal device. They are not the only proof of SIZE-02.
**Circular patterns detected:** 0.
**Insufficient assertions:** 0.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| --- | --- | --- | --- | --- |
| `spoke_base_resolver.rs` | 324, 766 | `PLACEHOLDER` | ℹ️ Info | Existing guard rejects an unpinned revision. Not a `TBD` / `FIXME` / `XXX` and not a stub return. |
| `auto_select.rs` | 49 | `hw.memsize` | ℹ️ Info | Left inside `detect_hardware_vram_gb`, which the live selector does not call. Plan 03 told the executor to leave that probe. |
| `models/registry.rs` | 779 | old run-directory ids | ℹ️ Info | Catalog entries for a caller-typed id. `select_tier_for_vram` is gone, and `auto_select.rs` does not emit those ids. |
| `vox-ml-cli/Cargo.toml` | 40 | empty `hw-probe-live-test` | ℹ️ Info | Declared so the two live `train_arm` tests compile only when opted in. No new crate dependency. |

No `TBD`, `FIXME`, or `XXX` markers were added in the phase files.

### Gaps Summary

No gap. The committed range implements one `pick_base` selector over measured `qwen3_code` QLoRA rows, reads CUDA free memory then Metal `memory_free_mb`, uses that pick for every unnamed spoke, re-reads before schola selects a train backend, refuses an explicit size that does not fit, and routes chat auto-local and the GUI host through the same selector. The directory-tier function is gone.

---

_Verified: 2026-10-29T22:03:14Z_
_Verifier: Claude (gsd-verifier)_
