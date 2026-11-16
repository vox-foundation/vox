---
phase: 91-measurement-harness-advisory-closure
plan: 05
subsystem: testing
tags: [eval, voxdiagram, mermaid, live-run, pre-registration, non-inferiority]

requires:
  - phase: 91-04
    provides: live runner, $25 spend cap, pre-registration commit f1ef3bb50
provides:
  - live recording contracts/eval/diagram-authoring/runs/2026-10-27 (attempts, oracle inputs/outputs, tallies, tables)
  - probe ledger contracts/eval/diagram-authoring/probe-spend.jsonl
  - EVAL-01 findings report with the measured verdict (non-inferior)
  - test it::eval_diagram_rescore::diagram_authoring_report_matches_run
affects: [eval-diagram, diagram-authoring, milestone v1.7 verification]

actuals:
  tokens: 2700
  tasks: 2
  commits: 2
plan_head_before: 2efcef6b9ef2b2b746f9fa515cee3a03954e98b1
plan_head_after: bfafe4912153dc3e1cfaf467a045078d63686b23

tech-stack:
  added: []
  patterns:
    - "Report results block spliced mechanically from tables.md (awk), then pinned by a test"

key-files:
  created:
    - contracts/eval/diagram-authoring/runs/2026-10-27/attempts.jsonl
    - contracts/eval/diagram-authoring/runs/2026-10-27/oracle-inputs.jsonl
    - contracts/eval/diagram-authoring/runs/2026-10-27/oracle-outputs.jsonl
    - contracts/eval/diagram-authoring/runs/2026-10-27/tallies.json
    - contracts/eval/diagram-authoring/runs/2026-10-27/tables.md
    - contracts/eval/diagram-authoring/probe-spend.jsonl
  modified:
    - crates/vox-cli/tests/it/eval_diagram_rescore.rs
    - docs/src/architecture/voxdiagram-authoring-reliability-findings-2026.md

key-decisions:
  - "EVAL-01 primary verdict (pre-registered rule, run 2026-10-27): non-inferior; model A deepseek/deepseek-v4.1-flash-1017 first-pass delta +2.0 points, Newcombe 95% [-1.7, +5.9]"
  - "Empty model replies are reported as an observed limitation, not re-scored: the rule counts them invalid and the verdict stands as assigned"

requirements-completed: [EVAL-01]

duration: 79min
completed: 2026-10-27
status: complete
---

# Phase 91 Plan 05: Live Measurement and Findings Report Summary

**The single pre-registered live run completed for $0.2849 of the $25 cap. On first-pass validity of model A (deepseek/deepseek-v4.1-flash-1017), VoxDiagram scored 196/200 and Mermaid 192/200: Δ +2.0 points, Newcombe 95% [-1.7, +5.9], so the primary verdict is non-inferior. The replication model B (xiaomi/mimo-v2.6-pro) gave Δ -0.5 [-4.0, +2.9], also non-inferior. Both re-score halves reproduce the recording byte for byte.**

## Performance

- **Duration:** about 79 min (22:32Z to 23:51Z). The live run itself took 3857 s (64 min) for 835 calls.
- **Tasks:** 2/2
- **Files:** 6 data files created (the recording and the probe ledger), 2 files modified (the test and the report).

## The run

- **Name:** `runs/2026-10-27`, header `synthetic: false`, `freeze_commit` `f1ef3bb50d0e934244bedd63608ca0a77f2a9155`. Started 22:33:58Z.
- **Precondition:** the pre-registration commit is an ancestor of HEAD, `runs/` did not exist, and the frozen-file diff was empty.
- **Command:** `env EVAL_DIAGRAM_FREEZE_COMMIT=f1ef… timeout 18000 cargo test -p vox-cli --test it -- --ignored --exact eval_diagram_live::live_run --nocapture`, backgrounded. Log: `target/eval-diagram/live.log`, ending `exit=0`.
- **Models:** A `deepseek/deepseek-v4.1-flash-1017` (deepseek) and B `xiaomi/mimo-v2.6-pro` (xiaomi). That is two vendor organisations, both reached through openrouter. Each answered its first probe, so no probe failed. Every attempt resolved to its requested id: 417 attempts for A and 411 for B, with `model_drift` false for both.
- **Spend:** attempts $0.28490369 plus probes $0.00002317, total $0.28492686 of $25. $24.715 remains.
- **Completeness:** complete, with no stop line. A scored 200/200. B scored 199/200: g056 was excluded after a transport error ("Failed to parse response JSON: error decoding response body") on its Mermaid turn 1.
- **Key-prefix scan:** 0 hits in each of attempts, oracle-inputs, oracle-outputs and probe-spend.

## Results (from tables.md)

| | Model A VoxDiagram | Model A Mermaid | Model B VoxDiagram | Model B Mermaid |
|---|---|---|---|---|
| First pass | 196/200 (98.0%) [95.0, 99.2] | 192/200 (96.0%) [92.3, 98.0] | 194/199 (97.5%) [94.3, 98.9] | 195/199 (98.0%) [94.9, 99.2] |
| After repair | 200/200 (100.0%) [98.1, 100.0] | 198/200 (99.0%) [96.4, 99.7] | 199/199 (100.0%) [98.1, 100.0] | 198/199 (99.5%) [97.2, 99.9] |

Paired difference, VoxDiagram minus Mermaid:

| Endpoint | Model | Δ (points) | Newcombe 95% CI | Verdict | McNemar p |
|---|---|---|---|---|---|
| First pass | A | +2.0 | [-1.7, +5.9] | non-inferior | 0.3877 |
| First pass | B | -0.5 | [-4.0, +2.9] | non-inferior | 1.0000 |
| After repair | A | +1.0 | [-1.0, +3.6] | non-inferior | 0.5000 |
| After repair | B | +0.5 | [-1.4, +2.8] | non-inferior | 1.0000 |

- **Primary verdict: non-inferior.** L = -1.7 is above -10. No descriptive flag applies: L is not above 0 and U is not below 0.
- **Observed:** 17 of the 827 attempts returned an empty reply, 15 of them in the Mermaid arm, and 10 of the 17 used the full 4096 output tokens. For model A at turn 0, 6 of the 8 Mermaid failures and 2 of the 4 VoxDiagram failures were empty replies. These are recorded in the report's Limitations as observed. They were not re-scored.

## Mutation proofs

- **M1, oracle-output corruption:** line 2 of `oracle-outputs.jsonl` was changed from id `g001:A:mermaid:0` to `g001:A:mermaid:7`. That is one byte, and the JSON stays valid.
  - Rust `diagram_authoring_dataset_rescores` FAILED with "no oracle output for g001:A:mermaid:0" (exit=100, `91-05-mut-rust.log`).
  - Node "re-score: Node-derived files of contracts/eval/diagram-authoring" FAILED (fail 1, exit=1, `91-05-mut-node.log`).
  - Restored from the scratchpad copy. `diff -q` was clean and the presence grep returned 0.
- **M2, report results block:** line 133 changed from `| mermaid | 192/200` to `| 193/200`.
  - `diagram_authoring_report_matches_run` FAILED and the other two passed (`91-05-mut-report.log`).
  - Restored from the copy, and `diff -q` was clean.

## Verification

| Check | Log | Result |
|---|---|---|
| Rust re-score (new run, unchanged) | `91-05-rescore.log` | 2 passed, exit=0 |
| Node re-score | `91-05-node1.log` | 8 pass, 0 fail, exit=0 |
| Task 2 RED (pending skeleton) | `91-05-t2-red.log` | report test failed on "Results pending…" vs tables.md, exit=100 |
| Task 2 GREEN, `test(/eval_diagram/)` on `--test it` | `91-05-report.log` | 11 passed, exit=0 |
| Prereg block at f1ef3bb50 vs now | `prereg-then.txt` / `prereg-now.txt` | `prereg-unchanged` |
| Scoped doc lint (report + advisory) | `91-05-lint1.log` | no hard errors, exit=0 |
| `vox ci check-links` | `91-05-links.log` | 5373 links PASS, exit=0 |

Acceptance criteria:
- Header greps: `synthetic:false` 1 and `freeze_commit` 1.
- 1 run directory.
- The frozen-input diff against f1ef3bb50 is clean, and the dependency diff against 5a631bb22 is clean.
- The run commit touches only `runs/` and `probe-spend.jsonl`.
- `Results pending` 0, `Primary verdict` line 1, mermaid fence 0, test name 1.
- The test file has 177 non-blank lines (under 400).
- The kb:summary is 95 words.

**CI homes of the re-score:**
- The Rust half runs in the `ci.yml` `tests` job (nextest, `eval_diagram_rescore`, which now includes the report test).
- The Node half runs in `docs-quality.yml`, job `docs-quality`, step "Docs unit tests".

## Commits

| Task | Commit | Subject |
|---|---|---|
| 1 | `ea0af22a4` | test(91-05): record live diagram-authoring run 2026-10-27 |
| 2 | `bfafe4912` | docs(91-05): report diagram-authoring measurement |

## TDD Gate Compliance

- **Task 2:** RED was valid. The new test failed against the 91-04 skeleton on the results-block assertion. GREEN came after the report was filled.
- No production code changed. The test is the only Rust change.

## Deviations from Plan

1. **[Minor] Run bound and cap variable.** I used the user's command: `timeout 18000` (5 h) and no explicit `EVAL_DIAGRAM_MAX_SPEND_USD`, so the code default of 25 applied. The plan had `timeout 21600` with an explicit cap of 25. The cap was the same either way, and the run took 64 min.
2. **[Minor] Report structure.** The skeleton's "Limitations known in advance" is kept unchanged. A new section "Limitations observed in the run" was added after it, and a "The run" paragraph was added to Method. The plan permits both. Nothing inside the prereg markers changed.
3. **[Minor] rustfmt applied** to the new test before commit, using single-file `rustfmt --config-path .`.

## Issues Encountered

- The shell guard blocked `python3 -c`. Spend sums were computed with awk over `tallies.json` instead.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register.
- T-91-15: the key scan returned 0.
- T-91-16: the results block test, the prereg diff and mutations M1 and M2.
- T-91-17: there is exactly one run, nothing was deleted, and nothing was edited.
- T-91-18: $0.285 spent.
- T-91-19: the report test asserts that no prompt or reference appears in the report.

## Self-Check: PASSED

- The run files, `probe-spend.jsonl`, the test and the report exist.
- `ea0af22a4` and `bfafe4912` are ancestors of HEAD.
- `git rev-list --count 2efcef6b9..bfafe4912` = 2.
