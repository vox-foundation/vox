---
phase: 318-live-vram-selector-size-02
plan: 02
subsystem: mens-training
status: complete
tags: [size-02, vram, selector, qwen3, training, vox-ml-cli]
requires:
  - "318-01 size_select (select_size, recheck_before_load, load_ladder_overlay, read_free_memory)"
provides:
  - "run_train: pick_train_size, recheck_size_pick, live_free_memory, force_train_env (moved from gpu.rs)"
  - "run_gpu_training: size_pick parameter and one D-05 re-check before the download"
  - "resolve_training_selection live mode (fixed_tier_mb None); Train.preset is Option"
affects:
  - "318-03 (orchestrator auto-select, chat, serve) and 318-04 (GUI) reuse the same selector"
  - "Phase 319 (cloud path still uses the fixed 24 GB tier)"
tech-stack:
  added: []
  patterns:
    - "One live size pick per training invocation in run_train; spokes and the pipeline pass CLI values through"
    - "A step-down refreshes VOX_BASE_MODEL so the profile's size-ladder floors follow the re-checked size"
key-files:
  created: []
  modified:
    - crates/vox-ml-cli/src/commands/schola/train/run_train.rs
    - crates/vox-ml-cli/src/commands/schola/train/gpu.rs
    - crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
    - crates/vox-ml-cli/src/commands/mens/training_selection.rs
    - crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs
    - crates/vox-ml-cli/src/commands/mens/pipeline.rs
    - mens/config/domain-profiles.yaml
    - docs/src/how-to/how-to-train-mens-macos-metal.md
decisions:
  - "The D-05 re-check also re-sets VOX_BASE_MODEL, because resolve_effective_profile reads it for size-ladder floors and run_train set it from the first pick"
  - "run_train.rs's tests module is now cfg(test) on every OS; only the old Metal dead-gate test stays macOS-only"
metrics:
  duration: "17 min"
  completed: "2026-10-27"
  tests:
    schola_train: 21
    populi_resolver_preset_selector: 85
    training_selection_train_arm_pipeline: 25
estimate:
  tokens: 85000
  tasks: 3
actuals:
  tokens: 9199
  tasks: 3
  commits: 3
plan_head_before: 2f97f2bf8672ddb8d2214edcd3c90bf898e04333
plan_head_after: f17f2279a3c78d10b6eed298d8c7037f01873789
---

# Phase 318 Plan 02: Training Uses the Live VRAM Selector Summary

`vox mens train --backend qlora` with no `--model` and no `VOX_MENS_DEFAULT_MODEL` now trains the largest Qwen3 size that fits live free memory, using that size's preset. It prints the pick and why, re-reads memory once right before the HF download, and steps an automatic pick down with a reason. A CPU run, or a run with no reading, refuses. Named sizes are kept or refused, never shrunk, and off-ladder ids pass through. `--domain` and pipeline runs reach `run_train` with only the operator's own model and preset, so that one pick covers the hub and every spoke. Only the cloud path still uses its fixed 24 GB tier. The Metal default-base path, which read the static probe ceiling, is deleted.

## Commits

| Task | Commit | Files |
|------|--------|-------|
| 1 Tracer: one live pick, re-check before download, Metal default deleted | `139ef5fd2` | run_train.rs, gpu.rs, spoke_base_resolver.rs |
| 2 Live mode in `resolve_training_selection` for `--domain` and the pipeline | `c6ec2cb91` | training_selection.rs, train_arm.rs, pipeline.rs, domain-profiles.yaml |
| 3 Operator how-to; M9-M12 | `f17f2279a` | how-to-train-mens-macos-metal.md |

`git rev-list --count 2f97f2bf8..HEAD` is 3. Each commit holds only its task's paths. Commits used `git commit --only`.

## Verification

| Gate | Command | Result | Required |
|------|---------|--------|----------|
| schola::train | `cargo nextest run -p vox-ml-cli --features cloud --lib -E 'test(schola::train)'` | 21 passed | >= 21 |
| populi | `cargo nextest run -p vox-populi --features mens-train --lib -E 'test(spoke_base_resolver) \| test(preset_schema) \| test(size_select)'` | 85 passed | >= 85 |
| selection + train_arm + pipeline | `... -E 'test(training_selection) \| test(train_arm) \| test(mens::pipeline)'` | 25 passed | >= 25 |
| clippy | `cargo clippy --color never -p vox-ml-cli --features cloud --lib --tests` | `Finished`; no finding in a touched file | as stated |
| rustfmt | `rustfmt --edition 2024 --check` on the six Rust files | clean | clean |
| doc lint | `cargo run -q -p vox-doc-pipeline -- --lint-only --paths how-to/how-to-train-mens-macos-metal.md` | no hard errors | no hard errors |
| Task 1 greps | `resolve_metal_default_base` in crates 0; `fn force_train_env` in gpu.rs 0; `pub(super) fn force_train_env` in run_train.rs 1; `recheck_size_pick(` in gpu.rs 1; non-comment `resolve_default_model_id` in run_train.rs 0; `pick_train_size(` 5; `DeviceKind::Cpu => None,` 1 | all pass | as stated |
| Task 2 greps | `get_system_vram_gb` in train_arm.rs 0; live guard line 1; `CLOUD_VRAM_MB` 3 now and 3 at the anchor; no diff line inside `derive_quantization_for_base` | all pass | as stated |
| line budgets (vs anchor `640f5b685`) | gpu.rs non-test non-blank 767 -> 770 (+3); train_arm.rs 1097 -> 1083; pipeline.rs 846 -> 846 | pass | gpu <= +10, train_arm <= +10, pipeline <= +3 |
| how-to scope | text from `## Serve and pin from Drive` onward has the same hash as at the anchor | unchanged | unchanged |
| no edges | `git diff 640f5b685 -- contracts/ci crates/vox-ml-cli/Cargo.toml crates/vox-populi/Cargo.toml` | empty | empty |

Both TDD tasks went RED before GREEN. Task 1's first build failed with E0432 (the four helpers were unresolved). Task 2's failed with 6 type errors (`preset` was still `String`).

## Mutation Results (M9-M12)

Protocol: back up the file, `grep -cF` the guard, mutate with the Edit tool, count again, run the named test with `--no-fail-fast`, restore from the backup and confirm the restored hash matches the backup (`cmp` is blocked by this host's shell allowlist, so `shasum` was compared instead), count again, and re-run the test. After all four, `shasum -c` matched the pre-mutation hashes of all three files.

| ID | File | Guard | Count before / mutated / restored | Red run (named test) | Restore | Green re-run |
|----|------|-------|-----------------------------------|----------------------|---------|--------------|
| M9 | run_train.rs | `DeviceKind::Cpu => None,` | 1 / 0 / 1 | exit 100, FAIL `cpu_training_has_no_accelerator_reading` | hash identical | PASS |
| M10 | run_train.rs | `size_select::recheck_before_load(` (call replaced by `Ok::<_, anyhow::Error>(pick)`) | 1 / 0 / 1 | exit 100, FAIL `recheck_of_an_auto_pick_without_a_reading_refuses` | hash identical | PASS |
| M11 | training_selection.rs | `let Some(tier_mb) = fixed_tier_mb else {` -> `.or(Some(16_384))` | 1 / 0 / 1 | exit 100, FAIL `live_mode_leaves_size_and_preset_to_the_selector` | hash identical | PASS |
| M12 | train_arm.rs | `resolve_training_selection(&root, Some(domain), cli_model, cli_preset, None)` -> `Some(16_384)` | 1 / 0 / 1 | exit 100, FAIL `domain_rust_leaves_size_to_the_live_selector` | hash identical | PASS |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] A step-down left `VOX_BASE_MODEL` naming the old size**
- **Found during:** Task 1
- **Issue:** `run_train` sets `VOX_BASE_MODEL` from the first pick, and `resolve_effective_profile` reads it for the size-ladder floors. Without a fix, a D-05 step-down would download the smaller size but size the profile for the larger one.
- **Fix:** the re-check block in `gpu.rs` sets `VOX_BASE_MODEL` to the re-checked pin before `resolve_effective_profile`. It adds three lines and stays inside the +10 budget.
- **Files modified:** crates/vox-ml-cli/src/commands/schola/train/gpu.rs
- **Commit:** `139ef5fd2`

### Other adjustments

2. **Tests module cfg.** `run_train.rs`'s `tests` module was `cfg(all(test, target_os = "macos"))`. The new tests and the moved force test would have run only on macOS. The module is now `cfg(test)`, and the old dead-gate test has its own `#[cfg(target_os = "macos")]`.
3. **M9 mutation text.** `live_free_memory` uses the full path, not a `size_select` import, so the mutation was `Some(vox_populi::mens::tensor::size_select::FreeMemory::cuda(1 << 40))`. The behavior under test is the same.
4. **Doc-lint path.** `--paths how-to/how-to-train-mens-macos-metal.md`, relative to `docs/src`, as noted after Plan 01.
5. **train_arm comment.** The comment above `effective_preset` said the preset was resolved from the spoke. It now says the CLI preset stays and `run_train` supplies the size's preset. The line count did not change.

No Rule 4 issues came up. There were no package installs, crate edges, Cargo.toml, contracts/ci or exceptions-ledger changes. The cloud path (`resolve_cloud_spoke_base`, `derive_quantization_for_base`, `CLOUD_VRAM_MB`) is unchanged apart from unwrapping the now-`Option` preset.

## Known Stubs

None.

## Threat Flags

None. T-318-08 through T-318-12 are mitigated as planned: re-check before download (M10), no total VRAM or 16 GB fallback in `train_arm` (grep gate, M12), spoke tags ignored for local training (M11), no reading on CPU (M9), and the pick and step-down notes printed to stderr. No new network, auth or file-write surface.

## Self-Check: PASSED

- FOUND: `fn recheck_size_pick` in run_train.rs; `recheck_size_pick(` in gpu.rs; `fixed_tier_mb` in training_selection.rs; `largest size` in the how-to
- FOUND commits on HEAD: 139ef5fd2, c6ec2cb91, f17f2279a
