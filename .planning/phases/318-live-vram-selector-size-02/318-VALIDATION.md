---
phase: "318"
slug: "live-vram-selector-size-02"
status: draft
nyquist_compliant: true
wave_0_complete: false
created: "2026-10-27"
---

# Phase 318 — Validation Strategy

> Per-phase validation contract for SIZE-02. Every check is hermetic: selector tests pass free memory as a literal `FreeMemory`, read the checked-in `mens/config/gpu-specs.yaml` (or its embedded copy), and never need a GPU, a download, or the network. The one live-reading test accepts either a reading or none. `vox-cli` and `vox-gui` do not compile on this branch, so `chat.rs` and `models_host.rs` are checked by grep and `rustfmt --check` here and compiled by hosted CI.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo nextest` (Rust), `vitest` + `tsc` (GUI UI) |
| **Config file** | none for Rust; `crates/vox-gui/ui/vitest.config.ts`, `crates/vox-gui/ui/tsconfig.json` |
| **Quick run command** | `cargo nextest run -p vox-populi --features mens-train --lib -E 'test(size_select)'` |
| **Full suite command** | the quick command, then `cargo nextest run -p vox-populi --features mens-train --lib -E 'test(spoke_base_resolver) \| test(preset_schema)'`, `cargo nextest run -p vox-populi --features mens-train --test training_presets_yaml_contract`, `cargo nextest run -p vox-ml-cli --features cloud --lib -E 'test(reasoning_compare) \| test(schola::train) \| test(training_selection) \| test(train_arm) \| test(mens::pipeline) \| test(ai::serve)'`, `cargo nextest run -p vox-orchestrator --lib -E 'test(models::) - test(premium_alias)'`, then `pnpm --dir crates/vox-gui/ui typecheck` and `pnpm --dir crates/vox-gui/ui test` (compared with the baseline below, not required green) |
| **Estimated runtime** | ~60 s warm per crate; a cold build on a loaded host takes much longer, so run cargo in the background |

**Baselines (planner, 2026-10-27, before any phase code):**

| Suite | Command | Baseline | Gate after the phase |
|-------|---------|----------|----------------------|
| populi resolver and presets | `cargo nextest run -p vox-populi --features mens-train --lib -E 'test(spoke_base_resolver) \| test(preset_schema)'` | 64 passed | at least 65 after Plan 01; with `test(size_select)` at least 85 after Plan 02 (4 Metal default-base tests deleted) |
| populi preset contract | `cargo nextest run -p vox-populi --features mens-train --test training_presets_yaml_contract` | 5 passed | at least 5 |
| populi selector | `... -E 'test(size_select)'` | 0 (new module) | at least 9 / 19 / 24 after Plan 01 Tasks 1 / 2 / 3 |
| ml-cli reasoning_compare | `cargo nextest run -p vox-ml-cli --features cloud --lib -E 'test(reasoning_compare)'` | 35 passed | exactly 30 (five `resolve_base_pin` tests move to `size_select`) |
| ml-cli schola::train | `... -E 'test(schola::train)'` | 16 passed | at least 21 |
| ml-cli selection, train_arm, pipeline | `... -E 'test(training_selection) \| test(train_arm) \| test(mens::pipeline)'` | 5 + 8 + 11 = 24 passed | at least 25 |
| ml-cli serve | `... -E 'test(ai::serve)'` | 48 passed | at least 52 |
| orchestrator auto_select | `cargo nextest run -p vox-orchestrator --lib -E 'test(auto_select)'` | 5 passed | at least 4 (5 old tests deleted, 4 new) |
| orchestrator registry | `... -E 'test(models::registry)'` | 17 passed | covered by the `models::` row |
| orchestrator models | `... -E 'test(models::) - test(premium_alias)'` | 310 passed (unfiltered `test(models::)`: 317 run, 2 failed) | at least 308 |
| GUI typecheck | `pnpm --dir crates/vox-gui/ui typecheck` | exit 0 | exit 0 |
| GUI vitest | `pnpm --dir crates/vox-gui/ui test` | 16 files and 26 tests failed, 319 files and 2158 tests passed (2184 tests) | no new failing test, at least one more pass, compared with Plan 04's Step 0 capture |
| GUI families mock | `vitest run e2e/lib/tauriMock.families.test.ts` | 3 tests passed | 4 tests passed |

Failures already on this branch, outside this phase and not to be fixed here:
- `vox-orchestrator`: `models::tests::premium_alias_tests::default_premium_alias_targets_exist_in_models_list` and `models::select::tests::select_with_premium_alias_honors_alias_when_intelligence_high`. The `models::` gate excludes `premium_alias`.
- GUI vitest: 26 tests in 16 files, among them `noVersionedModelIds.test.ts`, which already reports two offenders (`src/components/surfaces/Gamify/LeaderboardArenaPanel.tsx: qwen/qwen3.5-27b-base`, `e2e/lib/tauriMock.ts: claude-sonnet-4`). Plan 04's gates require exactly those two and no new failing test.

---

## Sampling Rate

- **After every task commit:** run that task's automated commands
- **After every plan wave:** run every command in the map for the crates the wave touched
- **Before `/gsd-verify-work`:** every command in the map green, plus the 16 mutation checks (M1-M16)
- **Max feedback latency:** 120 seconds warm

A Summary line with fewer passed tests than the gate, or `0 tests run`, is a failed run. Where a gate is exact (`reasoning_compare` after the move), more is also a failure. Do not run workspace-wide tests or clippy; hosted CI does that. No check uses `vox` or `vox ci`.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 318-01-01 | 01 | 1 | SIZE-02 | T-318-01, T-318-02, T-318-03 | Tracer. A literal reading picks the largest fitting size (12000 MB picks 8B, 11999 picks 4B); no reading or nothing fitting refuses; CUDA keeps the 0.88 margin, Metal has none; the six sizes map to their presets, all in `KNOWN_PRESETS`; an unknown size fails closed | unit | quick command (at least 9) | ❌ W0 (new file, created by the task's tests) | ⬜ pending |
| 318-01-01 | 01 | 1 | SIZE-02 | T-318-03 | `qwen3_6g` and `qwen3_12g` are in the contract YAML and resolve to the 16g profile | unit + contract | resolver/preset filter (at least 65); contract (at least 5) | ✅ | ⬜ pending |
| 318-01-02 | 01 | 1 | SIZE-02 | T-318-04, T-318-05 | Explicit size kept when it fits, refused (not shrunk) when it does not unless forced, kept unchecked with no reading; concrete off-ladder ids pass through; unknown bare names error; `resolve_base_pin` moved with its five tests | unit | quick command (at least 19); `cargo nextest run -p vox-ml-cli --features cloud --lib -E 'test(reasoning_compare)'` (exactly 30) | ✅ | ⬜ pending |
| 318-01-03 | 01 | 1 | SIZE-02 | T-318-06, T-318-05 | Re-check keeps a fitting automatic pick (never steps up), steps down with a note naming both sizes and the free MB, refuses with no reading or nothing fitting, never steps an explicit size down; 1.7B/4B get `S1p7`/`S4` | unit + mutation | quick command (at least 24); M1-M8 | ✅ | ⬜ pending |
| 318-02-01 | 02 | 2 | SIZE-02 | T-318-08, T-318-11 | Tracer. CPU runs have no reading; automatic pick on CPU refuses and names `--model`; explicit size on CPU kept unchecked; concrete ids pass through; the trainer's re-check helper refuses an automatic pick with no reading; the Metal default-base path is deleted | unit | `cargo nextest run -p vox-ml-cli --features cloud --lib -E 'test(schola::train)'` (at least 21); populi resolver/preset/selector (at least 85) | ✅ | ⬜ pending |
| 318-02-02 | 02 | 2 | SIZE-02 | T-318-09, T-318-10 | Live mode passes CLI model and preset through and ignores spoke tags for local training; fixed tier kept for cloud; `--domain` local path no longer reads total VRAM | unit | `cargo nextest run -p vox-ml-cli --features cloud --lib -E 'test(training_selection) \| test(train_arm) \| test(mens::pipeline)'` (at least 25) | ✅ | ⬜ pending |
| 318-02-03 | 02 | 2 | SIZE-02 | all of 02 | How-to describes the pick, presets, step-down and refusals; M9-M12 red then green; scoped clippy clean | doc lint + mutation + lint | `cargo run -q -p vox-doc-pipeline -- --lint-only --paths docs/src/how-to/how-to-train-mens-macos-metal.md`; `cargo clippy --color never -p vox-ml-cli --features cloud --lib --tests` | ✅ | ⬜ pending |
| 318-03-01 | 03 | 2 | SIZE-02 | T-318-13, T-318-14 | Tracer. Orchestrator auto-select uses the registered probe's minimum free MB x0.88 first, then the populi reading; returns the 8B pin at 13000 MB; refuses with no reading or nothing fitting; fake tiers gone | unit | `cargo nextest run -p vox-orchestrator --lib -E 'test(auto_select)'` (at least 4) | ✅ | ⬜ pending |
| 318-03-02 | 03 | 2 | SIZE-02 | T-318-17, T-318-14 | Fake `voxlocal` candidates deleted with their test; normalization test still proves both prefixes; `chat.rs` returns `Result` and routes auto-local through the selector | unit + grep | `cargo nextest run -p vox-orchestrator --lib -E 'test(models::) - test(premium_alias)'` (at least 308); `chat.rs` grep gates and `rustfmt --check` (compiled by hosted CI) | ✅ | ⬜ pending |
| 318-03-03 | 03 | 2 | SIZE-02 | T-318-15, T-318-16 | `serve_fit_note` refuses a ladder base that does not fit, notes one that fits or cannot be checked, ignores missing or off-ladder bases; called before the worker spawns; M13-M16 | unit + mutation | `cargo nextest run -p vox-ml-cli --features cloud --lib -E 'test(ai::serve)'` (at least 52) | ✅ | ⬜ pending |
| 318-04-01 | 04 | 3 | SIZE-02 | T-318-18, T-318-20 | Tracer. Mock returns the exact new key set with a pinned id and size `8B`; versioned-id guard reports only its two baseline offenders; typecheck passes; full vitest adds no failing test; GUI command returns the selector's `Result` | unit (vitest) + grep | `pnpm --dir crates/vox-gui/ui exec vitest run e2e/lib/tauriMock.families.test.ts src/__tests__/noVersionedModelIds.test.ts`; `pnpm --dir crates/vox-gui/ui typecheck`; `pnpm --dir crates/vox-gui/ui test` | ✅ | ⬜ pending |
| 318-04-02 | 04 | 3 | SIZE-02 | all | How-to covers serve's pre-load check and the shared auto-local pick; no fake tier, deleted function or crate-manifest change remains | doc lint + grep | doc lint on the how-to; the sweep commands in the task | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Mutation Checks

| ID | Plan | File | Guard | Test that must fail |
|----|------|------|-------|---------------------|
| M1 | 01 | size_select.rs | automatic no-reading refusal | `auto_pick_refuses_without_a_reading` |
| M2 | 01 | size_select.rs | `fits` uses `<=` | `auto_pick_is_the_largest_size_that_fits` |
| M3 | 01 | size_select.rs | CUDA 0.88 margin | `cuda_reading_keeps_the_safety_margin` |
| M4 | 01 | size_select.rs | explicit refusal unless forced | `explicit_size_that_does_not_fit_is_refused_not_shrunk` |
| M5 | 01 | size_select.rs | unknown size fails closed | `ladder_fails_closed_on_a_size_without_a_preset` |
| M6 | 01 | size_select.rs | re-check never steps up | `recheck_keeps_an_auto_pick_that_still_fits` |
| M7 | 01 | size_select.rs | re-check never steps explicit down | `recheck_never_steps_an_explicit_size_down` |
| M8 | 01 | preset_schema.rs | 1.7B size class | `each_real_rung_maps_to_a_size_class` |
| M9 | 02 | run_train.rs | CPU has no reading | `cpu_training_has_no_accelerator_reading` |
| M10 | 02 | run_train.rs | helper calls the re-check | `recheck_of_an_auto_pick_without_a_reading_refuses` |
| M11 | 02 | training_selection.rs | live mode ignores spoke tags | `live_mode_leaves_size_and_preset_to_the_selector` |
| M12 | 02 | train_arm.rs | local path uses live mode | `domain_rust_leaves_size_to_the_live_selector` |
| M13 | 03 | auto_select.rs | probe first, with margin | `registered_probe_free_memory_wins_and_keeps_the_margin` |
| M14 | 03 | auto_select.rs | no reading refuses | `auto_selection_refuses_without_a_reading` |
| M15 | 03 | serve/mod.rs | serve does not force | `serve_fit_note_refuses_a_ladder_base_that_does_not_fit` |
| M16 | 03 | serve/mod.rs | missing base is not checked | `serve_fit_note_passes_unknown_and_missing_bases` |

Protocol for each: back up the file; `grep -cF` the guard (1); mutate (0); run the named test with `--no-fail-fast` (FAIL); restore from backup and `cmp` (1); re-run (PASS). Record all five values in the plan's SUMMARY.

---

## Wave 0 Requirements

- `crates/vox-populi/src/mens/tensor/size_select.rs` is new; Plan 01 Task 1 creates it test-first (its tests fail to compile before the implementation). Every other touched file already has a test module.
- `crates/vox-gui/ui/node_modules` was installed by the planner on 2026-10-27 (gitignored); Plan 04 Task 1 runs `pnpm --dir crates/vox-gui/ui install --frozen-lockfile` only when it is missing, and captures a vitest baseline before editing.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| A real `vox mens train` without `--model` prints the pick, re-checks before download, and steps down when memory falls | SIZE-02 (SC1, SC2) | A real run downloads weights and trains; `vox-cli` does not build on this branch | After merge, on this Mac: `vox mens train --backend qlora --tokenizer hf --device metal --data-dir examples/mens/metal-e2e --output-dir mens/runs/size-select-check --epochs 1 --max-runtime-secs 60 --fast-corpus`; confirm `size: picked <size>` on stderr before the download. Start a memory-heavy process between pick and download to see `stepped down` |
| `vox chat --model auto-local` and the GUI recommendation show the live pick | SIZE-02 (SC4) | `vox-cli` and `vox-gui` compile only in hosted CI here | After merge: `vox chat --model auto-local "hi"` with a local server on `:11435`; the error or the model id names a `Qwen/Qwen3-<size>@<sha>` pin |
| `vox mens serve` refuses a run whose base no longer fits | SIZE-02 (SC2, SC3) | needs a trained run directory | Serve a run whose `training_manifest.json` `base_model` is the 32B pin on a host with less than 60000 MB free; expect the refusal text |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 120s warm
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
