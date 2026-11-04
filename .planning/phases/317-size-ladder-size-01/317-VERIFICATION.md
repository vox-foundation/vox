---
phase: 317-size-ladder-size-01
verified: 2026-10-27T00:00:00Z
status: passed
score: 2/2 roadmap success criteria verified (plus 5/5 plan-level truths)
covered_files:
  - ".planning/phases/317-size-ladder-size-01/317-01-PLAN.md"
  - ".planning/phases/317-size-ladder-size-01/317-01-SUMMARY.md"
  - "crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs"
  - "crates/vox-ml-cli/src/commands/schola/train/gpu.rs"
  - "crates/vox-populi/src/mens/tensor/preset_schema.rs"
  - "crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs"
  - "mens/config/gpu-specs.yaml"
covered_digest: "v3:sha256:cf754a7c31d08aeb19334a01b19e8fd60fc0cf1c78e4d13df1bf9b77983f8cc9"
behavior_unverified: 0
gaps: []
---

# Phase 317: Size Ladder (SIZE-01) Verification Report

**Goal:** Every Qwen3 dense size is a pinned, licensed rung that the trainer and the harness can name.
**Branch:** gsd/mens-compare-316 (HEAD 7d3377a9e, working tree clean)
**Re-verification:** No, initial.

## Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Ladder has Qwen3-0.6B, 1.7B, 4B, 8B, 14B, 32B, each pinned to a real HF commit and carrying a license (ROADMAP SC1) | VERIFIED | `mens/config/gpu-specs.yaml` `train_bases.qwen3_code` (lines 263-282) holds all six, each `hf_id: Qwen/Qwen3-<size>@<40-hex>`, `license: apache-2.0`, `attribution_required: false`. I re-ran `git ls-remote https://huggingface.co/Qwen/Qwen3-<size> refs/heads/main` for all six just now. Each head equals its pin exactly (0.6B c1899de2.., 1.7B 70d244cc.., 4B 1cfa9a72.., 8B b968826d.., 14B 40c06982.., 32B 9216db57..). Contract test `gpu_specs_qwen3_code_ladder_pins_all_six_dense_sizes` passes. |
| 2 | A rung whose memory floor is not measured says so (ROADMAP SC2) | VERIFIED | Every rung line ends `floor_measured: false`: 23 `false`, 0 `true`, 0 `hf_id` rung lines missing the key. The key has no serde default (a rung without it fails to load), and `parse_overlay` rejects `true` without `measured_on`/`peak_mb`, and `false` with either (WR-04 added the `1..=floor_mb` bound). Smoke run below prints `floor unmeasured` and the record carries `floor_measured: false`. Trainer path: `gpu.rs:430-434` prints each `floor_notes(...)?` entry to stderr right after `resolve_license_class`. |
| 3 | Size names resolve (`--base Qwen3-4B` etc.) | VERIFIED | Smoke run resolved `Qwen3-4B` to `Qwen/Qwen3-4B@1cfa9a72...`. `reasoning_compare` test `base_resolution_resolves_every_ladder_size` (in the 43-test nextest pass) asserts each of the six names resolves to `Qwen/<name>@<ls-remote sha>`. |
| 4 | Unmeasured floors are surfaced by the compare harness (stderr and record) and the trainer (`floor_notes`) | VERIFIED | Smoke stderr: `base Qwen/Qwen3-4B@1cfa...: qlora floor 9800 MB is an estimate (floor unmeasured)`. The written record `mens/runs/reasoning_compare/Qwen3-4B-1cfa9a720891.json` contains `base_rungs[]` with `floor_mb: 9800`, `floor_measured: false`. `floor_notes` is called at `gpu.rs:431`, the only non-test caller. Its parse errors are propagated, not swallowed (WR-01). |
| 5 | Preset size-class fix, including snapshot paths | VERIFIED | `detect_qwen_size_class` strips `@rev`, then classifies the last path segment containing `qwen` (handles `/` and `\`, `models--Org--Name`), so hex like `...8b...` no longer yields S8. `preset_schema` tests (inside the 64-test pass) include `size_class_ignores_the_pinned_revision_text`, with 1.7B/4B snapshot paths mapping to Other, an 8B snapshot with `14b32b` hex mapping to S8, and a Windows-separator 14B path. |

## Test and Smoke Evidence (run by me, this session)

| Command | Result |
|---------|--------|
| `cargo nextest run -p vox-populi --features mens-train --lib -E 'test(spoke_base_resolver) \| test(preset_schema)'` | 64 passed, 0 failed (1 leaky), 294 skipped |
| `cargo nextest run -p vox-populi --features mens-train --test training_presets_yaml_contract` | 5/5 passed |
| `cargo nextest run -p vox-ml-cli --features cloud --lib -E 'test(reasoning_compare) \| test(train_arm)'` | 43/43 passed, 339 skipped |
| `cargo run -q -p vox-ml-cli --features gpu -- mens eval-reasoning-compare --base Qwen3-4B --hub-root /nonexistent-hub` | stderr `... floor 9800 MB is an estimate (floor unmeasured)` then `NOT COMPARED: base_snapshot_absent (...)`. Record JSON has `status: not_compared`, `reason: base_snapshot_absent`, `base_rungs[0].floor_measured: false`. |

The 64/43 counts are higher than the SUMMARY's 63/35 gate numbers because the review-fix round added tests (WR-01..04, IN-01, IN-02) and my `cloud` feature filter also pulls in `train_arm`. No count is below the plan's gate floors.

## Requirements Coverage

| Requirement | Status | Evidence |
|-------------|--------|----------|
| SIZE-01 (six dense sizes, real HF commit + license each; unmeasured floors say so) | SATISFIED | Truths 1 and 2 above. REQUIREMENTS.md still shows `[ ]` / Pending for SIZE-01 and ROADMAP progress shows `0/1 Planned`; the orchestrator should tick these on completion. This is bookkeeping, not a code gap. |

No orphaned requirements: SIZE-01 is the only requirement mapped to Phase 317.

## Anti-Patterns

No `TBD`/`FIXME`/`XXX`, stub returns or hollow data in the touched code paths. Data flows from the YAML overlay to `base_rungs` in the written record and to the stderr notes (confirmed end to end by the smoke run).

## Review Findings

317-REVIEW-FIX.md reports WR-01..WR-04, IN-01, IN-02 and IN-04 fixed, each with its commit in `git log`. IN-03 (no ratio guard on the 1.7B/4B floors), IN-05 (`rungs_for_hf_id` order for inconsistent duplicates) and IN-06 (untyped `serde_json::Value` for `base_rungs`) were deliberately left out of scope. They are informational and do not affect any success criterion.

## Advisory Notes (non-blocking)

- The 1.7B (5000 MB) and 4B (9800 MB) floors are derived estimates, honestly flagged `floor_measured: false`. Real measurement is Phase 320, as designed.
- The train-start floor note is verified by code reading and the `floor_notes` unit tests. I did not run an end-to-end training start, because `vox-cli` does not compile in this worktree and the trainer needs hardware.
- `deferred-items.md` records two out-of-scope items: the 316 WR-03 shard-index issue, which does not affect the dense pins, and the non-real `Qwen3.8-27B` rung in other tags, which is not in `qwen3_code`.
- 1.7B and 4B classify as size class `Other` until Phase 318 (SIZE-02) adds presets, which matches the stated design.

## Verdict

Status: passed. Both roadmap success criteria hold in the code, backed by my own test runs, live `ls-remote` pin checks and the CLI smoke run.

_Verified: 2026-10-27_
_Verifier: Claude (gsd-verifier)_
