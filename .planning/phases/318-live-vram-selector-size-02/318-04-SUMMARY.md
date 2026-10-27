---
phase: 318-live-vram-selector-size-02
plan: 04
subsystem: gui-inference-selection
status: complete
tags: [size-02, vram, selector, gui, typescript, tauri-mock, docs]
requires:
  - "318-03 AutoModelSelection { selected_model_id, size, free_mb, floor_mb, reason } returned by get_auto_model_recommendation"
provides:
  - "ui/src/transport.ts AutoModelRecommendation in the live-pick shape"
  - "e2e tauriMock get_auto_model_recommendation returns the pinned 8B pick; vitest pins its exact key set"
  - "how-to: serve's pre-load fit check (conservative floor_mb footprint) and the shared auto-local / GUI pick"
affects: []
tech-stack:
  added: []
  patterns:
    - "TS IPC types copy the Rust serde field names verbatim; the mock is shape-checked by vitest because tsconfig does not cover e2e/"
key-files:
  created: []
  modified:
    - crates/vox-gui/ui/src/transport.ts
    - crates/vox-gui/ui/e2e/lib/tauriMock.ts
    - crates/vox-gui/ui/e2e/lib/tauriMock.families.test.ts
    - docs/src/how-to/how-to-train-mens-macos-metal.md
decisions:
  - "transport.ts doc comment says 'Pinned ladder model id', not 'Pinned Qwen3 id': the versioned-id guard flags any bare Qwen3 token in src/"
metrics:
  duration: "~45 min wall (two full vitest runs plus a baseline, ~6 min each)"
  completed: "2026-10-27"
  tests:
    tauriMock_families: 4
    vitest_full_after: "33-35 failed / 2150-2152 passed (2185) vs baseline 37 failed / 2147 passed (2184)"
estimate:
  tokens: 45000
  tasks: 2
actuals:
  tokens: 1100
  tasks: 2
  commits: 2
plan_head_before: 5c527d2e0a0419dc2929f531f26b870915ab3a30
plan_head_after: f709a7e2b5ceed985c26abc4e6b463cfadf764fa
---

# Phase 318 Plan 04: GUI Recommendation Type, Mock and Operator How-To Summary

The GUI's `AutoModelRecommendation` type and the e2e Tauri mock now match the `AutoModelSelection` that `get_auto_model_recommendation` has returned since Plan 03: `{ selected_model_id, size, free_mb, floor_mb, reason }`. The mock names the pinned `Qwen/Qwen3-8B@b968826d…` with size `8B` instead of the fake `mens/runs/qwen3_27b_metal_check` tier. The how-to explains that `vox mens serve` re-checks the run's base against free memory right before load and refuses one that does not fit. It says the check is conservative because the QLoRA floor stands in for the inference footprint, and that `vox chat --model auto-local` and the GUI use the same live pick as training.

## Commits

| Task | Commit | Files |
|------|--------|-------|
| 1 Tracer: TS type and mock carry the live pick | `ac53f0898` | transport.ts, tauriMock.ts, tauriMock.families.test.ts |
| 2 How-to: serve fit check and shared auto-local pick | `f709a7e2b` | how-to-train-mens-macos-metal.md |

`git rev-list --count 5c527d2e0..HEAD` was 2 before this SUMMARY commit. Each commit used `git commit --only` with its task's paths.

## Verification

| Gate | Result |
|------|--------|
| Step 0 vitest baseline (this checkout, before edits) | `Test Files  17 failed \| 318 passed (335)`, `Tests  37 failed \| 2147 passed (2184)`. The planner's baseline was 26 failed / 2158 passed. The extra failures are timeouts in the same load-sensitive files (`App.test.tsx`, perf guards); this host was running other cargo builds at the time. |
| RED | New case `the auto model recommendation mock has the live selector shape` failed against the old mock (`4 tests \| 1 failed`) |
| Task 1 verify 1 | PASS. `tauriMock.families.test.ts (4 tests)` all green. The versioned-id guard reports exactly its two baseline offenders (`e2e/lib/tauriMock.ts: claude-sonnet-4`, `LeaderboardArenaPanel.tsx: qwen/qwen3.5-27b-base`). |
| typecheck | `pnpm --dir crates/vox-gui/ui typecheck` exit 0 |
| Task 1 verify 2 (full vitest) | Counts gate PASS: run 1 `Tests 35 failed \| 2150 passed (2185)`, run 2 `33 failed \| 2152 passed (2185)`. Both have fewer failures than the baseline and at least baseline + 1 passes. The "no new FAIL line" sub-check did **not** pass literally; see Deviation 1. |
| Task 1 verify 3 | Commit file set is exactly the three paths: PASS. The old-name grep needed correcting; see Deviation 2. Corrected check (`git grep -nw`, guard fixture excluded) finds nothing. |
| `noVersionedModelIds.test.ts` unchanged | `git diff --quiet 5c527d2e0 -- …/noVersionedModelIds.test.ts`: PASS |
| Task 2 doc greps | `auto-local`, `training_manifest.json` and `conservative` all present: PASS |
| Doc lint | `cargo run -q -p vox-doc-pipeline -- --lint-only --paths how-to/how-to-train-mens-macos-metal.md` reports `no hard errors` |
| Task 2 commit file set | Only the how-to: PASS |

### Phase-wide SIZE-02 sweep

1. `git grep -n 'qwen3_27b_metal_check' -- crates/vox-orchestrator crates/vox-gui/src crates/vox-gui/ui/src crates/vox-gui/ui/e2e/lib/tauriMock.ts` has one hit: `crates/vox-gui/ui/src/__tests__/noVersionedModelIds.test.ts:68`. It is the guard's own fixture string, which tests that `mens/` names are exempt. The plan requires that file to stay unchanged. No GUI code and no mock uses the fake tier (see Deviation 2).
2. `git grep -n 'resolve_metal_default_base\|select_tier_for_vram\|register_mens_local_candidates' -- crates` has no hits.
3. `git grep -n 'size_select::' -- crates` has callers in `serve/mod.rs` (8), `run_train.rs` (14), `gpu.rs` (1), `auto_select.rs` (2) and `reasoning_compare/detect.rs` (1). That is all five expected files.
4. `git diff 640f5b685 -- contracts/ci 'crates/*/Cargo.toml'` (anchor `docs(318): plan live vram selector`) is empty. No new crate edge and no manifest change.

## Deviations from Plan

**1. [Verification] The full-suite "no new FAIL line" check flaked in `App.test.tsx`.**
- In both post-change runs, two `App.test.tsx` tests appeared that had not failed in the baseline. Run 1: `binds /plan and ignores a stale latest_plan_session_for_chat` and `harness issue polling > suppresses toasts…` (a 5000 ms timeout). Run 2: `binds /plan…` and `a toast is kept by the status bar bell as an unread problem`. The baseline itself had 13 different `App.test.tsx` failures.
- These failures are not from this plan:
  - Running `vitest run src/App.test.tsx` on its own after the change failed only 3 tests, and all 3 were in the baseline. Neither of the "new" tests failed.
  - `App.test.tsx` does not import `e2e/lib/tauriMock`.
  - The `transport.ts` change is type-only (interface fields and comments), so it emits no runtime code.
- The count gate passed on both runs. No test was quarantined or edited.

**2. [Plan bug] Two of the plan's grep gates cannot reach zero as written.**
- `tier_reason` is an unbounded substring, so it matches `frontier_reasoning` in `src/lib/platformSsot.generated.ts` (2 hits).
- `qwen3_27b_metal_check` matches a fixture string in `noVersionedModelIds.test.ts:68`, a file the plan says must stay unchanged.
- Corrected check: `git grep -nw 'qwen3_27b_metal_check\|detected_vram_gb\|tier_reason' -- crates/vox-gui/src crates/vox-gui/ui/src crates/vox-gui/ui/e2e/lib/tauriMock.ts ':!crates/vox-gui/ui/src/__tests__/noVersionedModelIds.test.ts'` finds nothing.

**3. [Rule 1] Doc comment wording.** The plan's suggested comment ("pinned Qwen3 id") made the versioned-id guard report `src/transport.ts: Qwen3` as a third offender. It now reads "Pinned ladder model id".

**4. [Accuracy] How-to adds the no-reading case.** `serve_fit_note` keeps a ladder base unchecked when there is no Metal or NVIDIA reading (`serve_fit_note_without_a_reading_keeps_an_explicit_base`). The paragraph lists that case among the ones served as before, so operators are not told serve will refuse there.

**5. Paths.** Logs went to the session scratchpad instead of `/tmp`, and the doc-lint `--paths` is relative to `docs/src`, as the coordinator noted.

No Rule 4 issues, package installs, Rust changes, crate edges or contract changes. STATE.md and ROADMAP.md were not touched.

## Left to hosted CI

- The `ui` job (required, since this touches `crates/vox-gui/**`) runs typecheck, vitest and the **existing** Playwright suite. No new GUI surface is added (P-318-07): nothing in `ui/src` reads or renders the recommendation, so there is no new Playwright spec.
- `vox-cli` and `vox-gui` compile and their tests: Plan 03's compile probes took the compile branch locally (both compiled; `chat_routing` 2 and `auto_model_recommendation` 2 passed). Hosted CI re-runs them as usual.
- Manual checks in 318-VALIDATION (real train, `vox chat --model auto-local`, serve refusal on a 32B run) remain for after merge.

## Known Stubs

None.

## Threat Flags

None. T-318-18 is mitigated: the field names are copied from the Rust struct, and vitest asserts the mock's exact key set. T-318-19 and T-318-SC are accepted as planned; no install ran (`node_modules` was already present).

## Self-Check: PASSED

- FOUND: `floor_mb: number` in transport.ts; `floor_mb: 12000` in tauriMock.ts; `get_auto_model_recommendation` in tauriMock.families.test.ts; `auto-local` in the how-to
- FOUND commits on HEAD: ac53f0898, f709a7e2b
