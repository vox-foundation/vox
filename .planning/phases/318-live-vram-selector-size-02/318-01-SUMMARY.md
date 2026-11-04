---
phase: 318-live-vram-selector-size-02
plan: 01
subsystem: mens-training
status: complete
tags: [size-02, vram, selector, qwen3, presets, vox-populi]
requires:
  - "Phase 317 train_bases.qwen3_code six-size ladder (gpu-specs.yaml)"
provides:
  - "vox_populi::mens::tensor::size_select: FreeMemory, read_free_memory, SizeRung, SizePick, Selection, select_size, recheck_before_load, resolve_base_pin, load_ladder_overlay"
  - "presets qwen3_6g (1.7B) and qwen3_12g (4B); size classes S1p7 and S4"
affects:
  - "318-02 (training wiring), 318-03 (orchestrator auto-select, chat, serve), 318-04 (GUI)"
tech-stack:
  added: []
  patterns:
    - "Live free-memory reading only (Metal working set; nvidia-smi memory.free x0.88); no reading or no fit refuses, no default size"
    - "Defactored 40-hex pin split (split_pin) instead of a vox-eval edge from the mens-gated module"
key-files:
  created:
    - crates/vox-populi/src/mens/tensor/size_select.rs
  modified:
    - crates/vox-populi/src/mens/tensor/mod.rs
    - crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
    - crates/vox-populi/src/mens/tensor/preset_schema.rs
    - contracts/mens/training-presets.v1.yaml
    - crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs
    - docs/src/architecture/where-things-live.md
decisions:
  - "An explicit ladder size with no live reading is kept with note 'fit not checked' (the planner's recorded interpretation of D-01 vs D-04)"
  - "qwen3_6g and qwen3_12g share the qwen3_16g profile values until floors are measured (Phase 320); the per-run auto-sizing sweep fits batch and sequence to the device"
  - "S1p7 and S4 reuse the S8 safe floors in apply_qwen_size_ladder_policy"
metrics:
  duration: "39 min"
  completed: "2026-10-27"
  tests:
    size_select: 24
    spoke_base_resolver_plus_preset_schema: 65
    training_presets_yaml_contract: 5
    reasoning_compare: 30
estimate:
  tokens: 90000
  tasks: 3
actuals:
  tokens: 10774
  tasks: 3
  commits: 4
plan_head_before: 47973e78ffad257d178a769f01ed384b6ed9446a
plan_head_after: 4d3a0b6fc2bf32a3bddd6ddc44c18e07e42c1c3b
---

# Phase 318 Plan 01: Live VRAM Selector Core Summary

`vox-populi` `mens::tensor::size_select` reads live free accelerator memory (the Metal working set with no extra margin, or nvidia-smi `memory.free` x0.88), picks the largest of the six `qwen3_code` QLoRA sizes whose floor fits, and returns that size's preset. Named sizes are kept, refused or passed through. A re-check before load steps an automatic pick down, never up, and never steps a named size down. There is no default-size fallback anywhere.

## Commits

| Task | Commit | Files |
|------|--------|-------|
| 1 Tracer: reading, ladder, automatic pick, two presets | `a99bd9887` | size_select.rs (new), tensor/mod.rs, spoke_base_resolver.rs, preset_schema.rs, training-presets.v1.yaml |
| 2 Explicit sizes, `resolve_base_pin` moved | `efde98553` | size_select.rs, reasoning_compare/detect.rs |
| 3 Re-check before load, S1p7/S4, doc row, mutations | `4d3a0b6fc` | size_select.rs, preset_schema.rs, where-things-live.md |

`git rev-list --count 47973e78f..HEAD` is 4. The fourth commit, `371c198fe fix(build): restore pre-e0770251f history.rs so vox-orchestrator-mcp compiles`, came from another session working in the same worktree and landed between Tasks 1 and 2. It is not part of this plan. Three commits belong to 318-01.

## Verification

| Gate | Command | Result | Required |
|------|---------|--------|----------|
| selector | `cargo nextest run -p vox-populi --features mens-train --lib -E 'test(size_select)'` | 24 passed | >= 24 (9 / 19 / 24 after Tasks 1 / 2 / 3) |
| resolver + presets | `... -E 'test(spoke_base_resolver) \| test(preset_schema)'` | 65 passed | >= 65 |
| preset contract | `cargo nextest run -p vox-populi --features mens-train --test training_presets_yaml_contract` | 5 passed | >= 5 |
| reasoning_compare | `cargo nextest run -p vox-ml-cli --features cloud --lib -E 'test(reasoning_compare)'` | 30 passed | exactly 30 (35 - 5 moved) |
| clippy | `cargo clippy --color never -p vox-populi --features mens-train --lib --tests` | Finished; no finding in a touched file (pre-existing warnings only in vox-eval and vox-corpus) | clean in touched files |
| rustfmt | `rustfmt --edition 2024 --check` on the five Rust files | clean | clean |
| doc lint | `cargo run -q -p vox-doc-pipeline -- --lint-only --paths architecture/where-things-live.md` | no hard errors | no hard errors |
| grep gates | no `get_system_vram_info`/`get_system_vram_gb`/`VOX_VRAM_OVERRIDE`/`probe_gpu` outside comments; `vox_eval` 0; `pub fn resolve_base_pin` 1; detect.rs `fn resolve_base_pin` 0; `reasoning_compare.rs` unchanged since the anchor | all pass | as stated |
| size budgets | size_select.rs non-blank lines before tests: 297 (< 450); preset_schema.rs non-test growth vs anchor: +8 (<= 8) | pass | as stated |
| no edges | `git diff 640f5b685 -- contracts/ci crates/vox-populi/Cargo.toml crates/vox-ml-cli/Cargo.toml` | empty | empty |

Each task was RED before it was GREEN. The tests were written first, and each first build failed to compile: 23 errors in Task 1, 4 in Task 2 (`resolve_base_pin`, `split_pin`) and 7 in Task 3 (`recheck_before_load`, `S1p7`, `S4`).

## Mutation Results (M1-M8)

Protocol: back up the file, `grep -cF` the guard, apply the mutation, count the guard again, run the named test with `--no-fail-fast`, restore from the backup and `cmp` it, count the guard, and re-run the test. After all eight, `shasum -c` matched both files' pre-mutation hashes.

| ID | File | Guard | Count before / mutated / restored | Red run (named test) | Restore | Green re-run |
|----|------|-------|-----------------------------------|----------------------|---------|--------------|
| M1 | size_select.rs | `let Some(free) = free else` | 1 / 0 / 1 | FAIL `auto_pick_refuses_without_a_reading` | cmp identical | PASS |
| M2 | size_select.rs | `u64::from(rung.floor_mb) <= free.mb` | 1 / 0 / 1 | FAIL `auto_pick_is_the_largest_size_that_fits` | cmp identical | PASS |
| M3 | size_select.rs | `SAFETY_FRACTION) as u64` | 1 / 0 / 1 | FAIL `cuda_reading_keeps_the_safety_margin` | cmp identical | PASS |
| M4 | size_select.rs | `if !force {` | 1 / 0 / 1 | FAIL `explicit_size_that_does_not_fit_is_refused_not_shrunk` | cmp identical | PASS |
| M5 | size_select.rs | `has no preset for size` (bail replaced by `continue`) | 1 / 0 / 1 | FAIL `ladder_fails_closed_on_a_size_without_a_preset` | cmp identical | PASS |
| M6 | size_select.rs | `if fits(&pick.rung, free) {` (3-line block deleted) | 1 / 0 / 1 | FAIL `recheck_keeps_an_auto_pick_that_still_fits` | cmp identical | PASS |
| M7 | size_select.rs | `if pick.explicit {` -> `if false {` | 1 / 0 / 1 | FAIL `recheck_never_steps_an_explicit_size_down` | cmp identical | PASS |
| M8 | preset_schema.rs | `near(1.7)` -> `near(1.75)` | 1 / 0 / 1 | FAIL `each_real_rung_maps_to_a_size_class` | cmp identical | PASS |

Every red run exited 100 (nextest's test-failure code, not a compile error), and its log names the expected test in a `FAIL` line.

## Deviations from Plan

1. **Guard text for M1 and M6 after rustfmt.** rustfmt splits `let Some(free) = free else { bail!(..) };` and `if fits(&pick.rung, free) { return Ok(pick); }` over three lines each. M1's fixed-string guard is still a unique substring. For M6 the guard became `if fits(&pick.rung, free) {` and the mutation deleted the three-line block. The guarded behavior did not change.
2. **Doc-lint path.** `vox-doc-pipeline --paths` resolves relative to `docs/src`, so the plan's `--paths docs/src/architecture/where-things-live.md` failed with "did not match any file" (it looked for `docs/src/docs/src/...`). I ran it with `--paths architecture/where-things-live.md`, and it reported no hard errors.
3. **Test constant.** In `size_select` tests, `PIN_8B` is `crate::mens::DEFAULT_MODEL_ID`, as it was in detect.rs, so `base_resolution_default_is_the_pinned_8b` keeps its meaning. The moved test asserts `split_pin(&pin).1.is_some()` where it used to call `BaseLabel::from_pin`, as the plan specified.
4. **Interim explicit branch (Task 1 only).** Task 1's placeholder `bail!` interpolates `force` so the parameter is not unused. Task 2 replaced it, as planned.
5. **Commit hygiene incident, fixed.** Task 1's first commit (`ee5f58b93`) also swept in `crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs`, which another session had already staged in this worktree's shared index. I ran `git reset --soft HEAD~1` and re-committed with `git commit --only -- <five paths>` as `a99bd9887`, which left their staged change untouched. That session committed it later as `371c198fe`. Tasks 2 and 3 also used `--only`. `ee5f58b93` is not on the branch.

No Rule 4 (architectural) issues came up. No package installs, crate edges, Cargo.toml, contracts/ci, or exceptions-ledger changes.

## Known Stubs

None. `select_size` and `recheck_before_load` are complete. Wiring training, serve, chat and the GUI to them is Plans 02 to 04 by design.

## Threat Flags

None. The selector adds no network, auth or file-write surface. The new reads are `nvidia-smi` through the existing `query_nvidia_smi_vram` and the Metal budget through the existing `query_accel_budget`. Every T-318-01..06 mitigation is in place and covered by M1-M7.

## Self-Check: PASSED

- FOUND: crates/vox-populi/src/mens/tensor/size_select.rs (contains `pub fn recheck_before_load`)
- FOUND: `"qwen3_12g"` in spoke_base_resolver.rs; `id: qwen3_12g` in training-presets.v1.yaml; `S1p7` in preset_schema.rs; `size_select::resolve_base_pin` in detect.rs
- FOUND commits on HEAD: a99bd9887, efde98553, 4d3a0b6fc
