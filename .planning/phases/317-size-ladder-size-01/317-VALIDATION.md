---
phase: "317"
slug: "size-ladder-size-01"
status: draft
nyquist_compliant: true
wave_0_complete: false
created: "2026-10-27"
---

# Phase 317 — Validation Strategy

> Per-phase validation contract for SIZE-01. Every check is hermetic: tests read the checked-in `mens/config/gpu-specs.yaml` and never the network. No model is loaded and nothing is downloaded or trained. Pins were confirmed with `git ls-remote` by the planner on 2026-10-27; the contract test asserts those exact values.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo nextest`, `tempfile` for hub and runs fixtures |
| **Config file** | none |
| **Quick run command** | `cargo nextest run -p vox-populi --features mens-train --lib -E 'test(spoke_base_resolver) \| test(preset_schema)'` |
| **Full suite command** | the quick command, then `cargo nextest run -p vox-populi --features mens-train --test training_presets_yaml_contract`, then `cargo nextest run -p vox-ml-cli --features gpu --lib reasoning_compare` |
| **Estimated runtime** | ~60 seconds warm per crate (the vox-populi baseline ran in 7 s once built) |

---

## Sampling Rate

- **After every task commit:** Run that task's automated commands
- **After every plan wave:** Run every command in the map
- **Before `/gsd-verify-work`:** Every command in the map must be green, plus the six-size smoke run and the seven mutation checks
- **Max feedback latency:** 120 seconds warm

A Summary line with fewer passed tests than the gate, or `0 tests run`, is a failed run. Do not run workspace-wide tests; hosted CI does that. `vox-cli` does not compile on this branch, so no check uses `vox` or `vox ci`.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 317-01-01 | 01 | 1 | SIZE-01 | T-317-02 | Tracer. A rung without `floor_measured` fails to load. `floor_label` says `floor unmeasured` for an estimate and names host and peak for a measured rung. `rungs_for_hf_id` matches the exact pin and dedupes only identical rungs. `pick_base(qwen3_code, 8000)` is the pinned 4B rung with floor 7400 and Apache-2.0 | unit | `cargo nextest run -p vox-populi --features mens-train --lib -E 'test(spoke_base_resolver) \| test(preset_schema)'` (at least 59) | ✅ | ⬜ pending |
| 317-01-01 | 01 | 1 | SIZE-01 | T-317-01 | The floor-vs-weight contract still holds with the 4B rung (`KNOWN_PARAMS_B` row present) | contract | `cargo nextest run -p vox-populi --features mens-train --test training_presets_yaml_contract` (at least 4) | ✅ | ⬜ pending |
| 317-01-01 | 01 | 1 | SIZE-01 | T-317-01, T-317-02 | `--base Qwen3-4B` resolves to the 4B pin, and the record's `base_rungs` carries `floor_mb` 7400, `floor_measured` false and `apache-2.0`. A pin not on the ladder writes an empty array | unit | `cargo nextest run -p vox-ml-cli --features gpu --lib reasoning_compare` (at least 34; baseline 33) | ✅ | ⬜ pending |
| 317-01-01 | 01 | 1 | SIZE-01 | T-317-02 | Real CLI run with an empty `--hub-root` prints `NOT COMPARED: base_snapshot_absent` and `floor unmeasured`, and the 4B record has `"floor_measured": false` and no score key | smoke | `cargo run -q -p vox-ml-cli --features gpu -- mens eval-reasoning-compare --base Qwen3-4B --hub-root <empty dir>`, then grep the record | ✅ | ⬜ pending |
| 317-01-02 | 01 | 1 | SIZE-01 | T-317-02 | `floor_measured: true` without non-blank `measured_on` and `peak_mb`, or `false` with either, fails to load. A complete measured rung loads, and its label names the host | unit | quick command (at least 61) | ✅ | ⬜ pending |
| 317-01-02 | 01 | 1 | SIZE-01 | T-317-03 | All six pins resolve `apache-2.0` with attribution false through `resolve_license_class` and no override, which is the trainer's base-license path | unit | quick command | ✅ | ⬜ pending |
| 317-01-02 | 01 | 1 | SIZE-01 | T-317-01, T-317-06 | The six sizes are in `qwen3_code` at the ls-remote pins, with one pin per size across all tags. Every Qwen3 revision is 40 lowercase hex, every dense rung is Apache-2.0, QLoRA floors strictly increase, Qwen3.8-27B is absent from `qwen3_code`, and repeated rungs agree | contract | `cargo nextest run -p vox-populi --features mens-train --test training_presets_yaml_contract` (at least 5) | ✅ | ⬜ pending |
| 317-01-02 | 01 | 1 | SIZE-01 | T-317-01 | `Qwen3-0.6B` through `Qwen3-32B`, bare or `Qwen/`-prefixed, each resolve through `resolve_base_pin` to the matching `qwen3_code` pin | unit | `cargo nextest run -p vox-ml-cli --features gpu --lib reasoning_compare` (at least 35) | ✅ | ⬜ pending |
| 317-01-03 | 01 | 1 | SIZE-01 | T-317-04 | The 1.7B and 4B pins classify as `Other`, not `S8`, and a synthetic `32b` revision does not make 0.6B `S32`. `floor_notes` lists every rung of a pin (14B: two), works from the embedded copy, and is empty off the ladder | unit | quick command (at least 63) | ✅ | ⬜ pending |
| 317-01-03 | 01 | 1 | SIZE-01 | T-317-01, T-317-02 | All six size names through the real CLI each write a per-size record naming the size with `"floor_measured": false` and print `floor unmeasured` | smoke | six-size loop over `cargo run -q -p vox-ml-cli --features gpu -- mens eval-reasoning-compare --base <size> --hub-root <empty dir>` | ✅ | ⬜ pending |
| 317-01-03 | 01 | 1 | SIZE-01 | all | Each guard fails its named test when broken on purpose (M1-M7), then passes when restored. Clippy finds nothing in touched files. New-file size limits hold, and growth of the two pre-existing large files is capped | mutation + lint | `cargo clippy --color never -p vox-populi --features mens-train --lib --tests` and `cargo clippy --color never -p vox-ml-cli --features gpu --lib --tests`, both without `-D warnings`, each log ending in `Finished` with no finding in a touched file; `rustfmt --edition 2024 --check` on the six touched Rust files | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

Rows are tagged to the three tasks in `317-01-PLAN.md`. Every test file already exists, so there is no Wave 0 scaffold. New tests are added inside existing test modules.

---

## Wave 0 Requirements

- None. `spoke_base_resolver.rs`, `preset_schema.rs`, `training_presets_yaml_contract.rs`, `reasoning_compare.rs` and `reasoning_compare/detect.rs` all have test modules; each task writes its tests first.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| The train-start line `base <pin>: ... floor unmeasured` appears in a real `vox mens train` run | SIZE-01 (SC2) | A real run needs a corpus and downloads weights, which this phase must not do. `floor_notes` is unit-tested and the `gpu.rs` call is compiled by the vox-ml-cli test build | In Phase 320, start `vox mens train --model Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e ...` and confirm stderr shows the floor line before the download |
| Pins still match the Hub | SIZE-01 (SC1) | Tests are hermetic by decision (D-05). A pin is an immutable commit, so a later move of `main` does not invalidate it | `git ls-remote https://huggingface.co/Qwen/<repo> refs/heads/main` for the six repos, compared with the plan's evidence table |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 120s warm
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
