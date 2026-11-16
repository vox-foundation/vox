---
phase: 339-model-cards-v2
plan: "01"
subsystem: mens-hub-cards
tags: [mens, hub-cards, askama, gate-state]
requires: []
provides:
  - "hub_card directory module (mod, facts, render, legacy_prose, collateral_standin) with hub_sync.rs untouched"
  - "askama render path for the model card (templates/hub/model.md)"
  - "facts::gate_of, GateState, Blocker, GateFact (three-fact gate state)"
  - "collateral_standin with 338-01's names, fields and refusal order"
affects: [339-03, 339-05, 339-06, 339-08, 338-01, 338-02, 338-06, 338-09]
key-files:
  created:
    - crates/vox-populi/src/mens/hub_card/mod.rs
    - crates/vox-populi/src/mens/hub_card/facts.rs
    - crates/vox-populi/src/mens/hub_card/facts_tests.rs
    - crates/vox-populi/src/mens/hub_card/render.rs
    - crates/vox-populi/src/mens/hub_card/collateral_standin.rs
    - crates/vox-populi/src/mens/hub_card/tests/mod.rs
    - crates/vox-populi/templates/hub/model.md
  modified:
    - crates/vox-populi/Cargo.toml
    - Cargo.lock
    - .planning/phases/338-spoke-aware-gating-and-hub-repos/338-01-PLAN.md
    - .planning/phases/338-spoke-aware-gating-and-hub-repos/338-02-PLAN.md
    - .planning/phases/338-spoke-aware-gating-and-hub-repos/338-06-PLAN.md
    - .planning/phases/338-spoke-aware-gating-and-hub-repos/338-09-PLAN.md
  moved:
    - "hub_card.rs -> hub_card/legacy_prose.rs"
    - "hub_card_tests.rs -> hub_card/tests/legacy.rs"
key-decisions:
  - "Blocker::Missing holds record or gate identifiers only (no prose); a receipt with fewer than two measuring gates is reported as Missing(gate_receipt.json)"
  - "Inline tests were added to facts.rs and legacy_prose.rs in the split commit because the repo's tdd-guard hook flags moved pub fns as untested"
requirements-completed: [CARD-01, CARD-02, CARD-04]
status: complete
commits: 3
plan_head_before: c678a47f54f062cca517a5ad5555c0f7161af4ff
plan_head_after: c8bfb483f952ecde6ab14d51e67f5821fb89cdcb
phase_base: 52b8e9fe659675bfd6e43633a70eb0a4d6fe5a5d
actuals:
  tokens: 21000
  tasks: 3
  commits: 3
completed: 2026-11-06
---

# Phase 339 Plan 01: hub_card split, askama tracer and the gate fact Summary

`hub_card` is now a directory module whose model card renders through an askama template with all six phase-336 goldens byte-identical, and `facts::gate_of` computes the gate state (passed, needs re-gate, failed, not evaluated) through a same-signature stand-in for 338-01's shared collateral check.

## Commits

- `0d4fb7d1a` refactor(339-01): split hub_card into a directory module and render the model card through askama
- `c8bfb483f` feat(339-01): add the gate fact (gate_of) through a same-signature collateral stand-in
- `28bd5f6c9` docs(339-01): retarget 338-01/02/06/09 plans to the hub_card directory module

`commits: 3` is `git rev-list --count c678a47f5..c8bfb483f`, where `c678a47f5` is the last 339-02 commit. 339-02 ran in the same working tree, so its two commits (`f639df32f`, `c678a47f5`) sit between `phase_base` and mine and are not counted. The SUMMARY commit follows `plan_head_after`.
Order deviation: Task 3 (planning docs) was committed before Task 2's code commit, because Task 2's build was queued behind other agents. The files do not overlap.

## phase_base

`git rev-parse HEAD` before Task 1's first edit printed `52b8e9fe659675bfd6e43633a70eb0a4d6fe5a5d`. 339-08 diffs `hub_sync.rs` against it.

## Verification

All cargo runs were `-p vox-populi --features mens-hf-hub`, one crate at a time.

| Run | Result |
|---|---|
| `--lib hub_card` (after Task 1) | 11 passed, 0 failed (9 before the two inline tests were added for tdd-guard) |
| `--lib hub_sync::tests` (after Task 1) | 10 passed, 1 ignored, 0 failed, with `hub_sync.rs` unedited |
| `--lib hub_card` (after Task 2, clean tree) | 29 passed, 0 failed |
| `--lib hub_card` (after mutation restore) | 29 passed, 0 failed |
| `--lib hub_sync::tests` (after Task 2) | 10 passed, 1 ignored, 0 failed |
| `cargo check` (no test cfg) | 0 warnings |

The five required tests appear by name in the output: `stale_pass_is_needs_re_gate`, `retired_rule_fail_reads_failed_and_quotes_refusal`, `current_pass_is_passed`, `missing_requirement_fails_closed`, `load_requirement_embedded_matches_disk`. The six goldens were not regenerated (`VOX_UPDATE_GOLDEN` never set) and `git diff HEAD` on `hub_sync.rs`, `mens/mod.rs` and `tests/golden/hub_cards/` is empty. The askama template needed no whitespace tuning: it is `{{ frontmatter }}{{ body }}` with no trailing newline.

Acceptance greps: `hub_card.rs` and `hub_card_tests.rs` gone; `dep:askama` and `dep:vox-eval` each appear once under `mens-hf-hub`; `default-features = false, features = ["config"]` appears once; `path = "hub/model.md"` appears once; `check_collateral_report` is defined once and the three guards exist; `/Users/` and `/home/` appear 0 times in `facts_tests.rs`; `crates/vox-eval/src/collateral_evidence.rs` does not exist; mod.rs 36, facts.rs 355 and render.rs 65 lines.

### Cargo.lock, crate edges, hakari

- `Cargo.lock` diff is exactly one added line, `"askama"` in the `vox-populi` dependency list. `grep -c 'name = "askama"' Cargo.lock` prints 1 (version 0.12.1, already locked for uniffi_bindgen). No new version.
- `vox ci crate-edges` (run with `VOX_SKIP_FRESHNESS_CHECK=1`, because the installed `vox` binary is stale against the tree) exits non-zero with one violation: `NEW EDGE not in baseline: vox-plugin-mens-candle-core -> vox-eval`. That edge comes from `crates/vox-plugin-mens-candle-core/Cargo.toml` (commit `5e0dccbb9`, phase 313), which this plan did not touch. This plan adds no edge: `vox-populi -> vox-eval` is already in the baseline and the optional dependency already existed. The binary also reports a stale baseline edge `vox-gui -> vox-db-types`. Both are pre-existing. Needs the user, or whoever owns phase 313's baseline.
- `cargo hakari generate --diff` reports drift (tower-http, moka, serde_bytes, reqwest features) with no askama line and no vox-populi line. It is pre-existing, so `crates/workspace-hack/Cargo.toml` was left untouched.

## Mutation proof 1 (spec section 7)

File: `crates/vox-populi/src/mens/hub_card/collateral_standin.rs`.

| | `grep -c 'fn require_slice_pins'` | `grep -c 'fn require_rule_alpha'` |
|---|---|---|
| before | 1 | 1 |
| after restore | 1 | 1 |

Mutation: `check_collateral_report` returned `Ok(())` straight after the status check (the three guards and `verify_reasoning_slices` removed from the path). Restored by copying the pre-mutation file back; `git diff HEAD` on the file was empty and `return Ok(());` count was 0 afterwards.

Mutated run, `cargo test -p vox-populi --features mens-hf-hub --lib stale_pass_is_needs_re_gate`:

```
test mens::hub_card::facts::tests::stale_pass_is_needs_re_gate ... FAILED
assertion `left == right` failed
  left: Passed
 right: NeedsReGate
test result: FAILED. 0 passed; 1 failed
```

After restore, `--lib hub_card` printed `test result: ok. 29 passed`.

## Run dirs the inline test records mirror

Read-only, copied no host paths, nothing committed from `mens/runs`:

- 1.7B stale pass: `mens/runs/qwen3-1.7b` (status pass, `slice_size` 32, no `rule`, `slice_blake3` or `grader`)
- 14B no collateral: `mens/runs/qwen3-14b` (trained, receipt with missing gates, no collateral report)
- 32B not trained: `mens/runs/qwen3-32b` (`train-record.json` with status `not_trained`, reason `above non-fitting 4B (D-06)`)
- 0.6B retired-rule fail: `mens/runs/qwen3-0.6b` (status `fail`, `failed_on` `gsm8k_think`, receipt `overall_passed` false)

Beyond the five shapes, `facts_tests.rs` also covers a trained record without weights (`NotPublished`), a refused receipt gate with a current collateral report, and the label/tag pairing.

## 338 retarget table (applied in `28bd5f6c9`)

| 338 plan / task | Edit as planned in 338 | Where it goes after 339 |
|---|---|---|
| 338-01 Task 1 | add `dep:vox-eval` to `mens-hf-hub` | already done by 339-01; skip that line |
| 338-01 Task 1 | create `gate_policy.rs` with `ladder_requirement` | unchanged |
| 338-01 Task 2 | `RunRecords.collateral_req`, set in `load` | field exists in `hub_card/facts.rs`; replace the body of private `load_requirement` with `gate_policy::ladder_requirement(root)` and delete its embedded `include_str!` copy |
| 338-01 Task 2 | `card_status` maps Stale to `CardStatus::NeedsReGate` | already done by `facts::gate_of` (`GateState::NeedsReGate`); delete `collateral_standin.rs`; change the one `use` in facts.rs to `vox_eval::collateral_evidence::{...}`; do not add `CardStatus::NeedsReGate` |
| 338-01 Task 2 | `hub_card_tests.rs` fixtures | `hub_card/tests/support.rs` `passing_collateral_report(req)` calls `gate_policy::test_support::passing_collateral_report` (339-06) |
| 338-01 Task 2 | goldens `needs_re_gate.md`, `passed.md` | exist after 339-05/06; regenerate only if the refusal text changed |
| 338-02 Task 3 | `RunRecords.spoke_gates`, `card_status` from ladder row | in `facts.rs`, read inside `gate_of` |
| 338-02 Task 3 | "Gate state by spoke" table on the model card | not rendered (D5); open question for the user, see below |
| 338-06 Task 3 | frontmatter `spoke-<name>` tag and `Spoke:` line | `SpokeFact` source in `hub_card/card_facts.rs` becomes `gate_policy::spokes_of_run`; remove `default_spoke` fields; provenance goes inside the status-line sentence |
| 338-09 | `Dataset` subcommand | next to 339-04's `cards_only` on `HubAction::Sync`; reuses `print_sync_rows` |
| hub_sync tests | index/card text assertions | 339-08 may have changed one; rebase onto it |

`! grep -lE 'src/mens/hub_card\.rs|hub_card_tests\.rs' .planning/phases/338-spoke-aware-gating-and-hub-repos/338-0[1269]-PLAN.md` prints nothing (exit 0). The commit touched only the four plan files.

## Deviations from Plan

**1. [Rule 3 - Blocking] tdd-guard hook rejected the first split commit.** The pre-commit hook `tdd-guard` (`skeleton/untested-pub-api`) treats the moved `facts.rs` and `legacy_prose.rs` as new files with untested pub fns. I added one small `#[cfg(test)]` test in each (`record_tests::empty_run_dir_loads_as_all_absent`, `legacy_prose::tests::status_label_and_card_status_without_adapter`) rather than a suppression or `--no-verify`. The plan said no toestub suppression in `legacy_prose.rs`, and none was added. The tests live in `legacy_prose.rs`, which 339-08 deletes.

**2. [Rule 1 - Bug] Dead-code warnings in the non-test build.** The `collateral_standin` items and the `GateState`, `Blocker`, `GateFact` and `gate_of` items have no non-test consumer until 339-05, which gave 13 `dead_code` warnings under `cargo check`. I marked them `#[allow(dead_code, reason = "...")]` (and the `pub(crate)` re-export `unused_imports`), each naming 339-05. The next plan should delete those attributes when it consumes the items. Clippy `-D warnings` was not run (workspace policy: CI is the gate).

**3. Fixed a missing import in the moved tests.** `tests/legacy.rs` needed `repo_name` after replacing `use super::*;` with explicit imports.

**4. `Blocker::Missing` granularity.** The plan lists `Missing(Vec<String>)` without saying what the strings are; I used record file names and gate names, and report a receipt with fewer than two measuring gates as `Missing(["gate_receipt.json"])`. This loses the "N of its gates measured anything" count that the legacy prose showed; 339-05/06 may want a typed field if the card must state it.

**5. Task 3 committed before Task 2's code commit** (see Commits). No content effect.

**6. `crate-edges` is not clean at the repo baseline** (see Verification). The plan's automated verify includes `vox ci crate-edges`; it fails on an edge outside this plan. I judged it not a blocking-human trigger for this plan because this plan introduces no edge and no new lock version.

STATE.md, ROADMAP.md and REQUIREMENTS.md were not updated by this executor, because another executor shares the working tree and edits the same planning state at the same time. The orchestrator should record CARD-01, CARD-02 and CARD-04 as done.

## Known Stubs

None. `legacy_prose.rs` is intentional carry-over of phase-336 wording, deleted by 339-08. `collateral_standin.rs` is an intentional stand-in, deleted by 338-01.

## Threat Flags

None. The only new third-party surface is askama, already in the lock graph and checked as above.

## Open question for the user

338-02 Task 3 planned a "Gate state by spoke" table on the model card. Per D5 (gate stated once) the retarget drops it from the model card. Decide whether per-spoke detail belongs on the index grid or in a collapsed block.

## Self-Check: PASSED

- Created files exist (mod.rs, facts.rs, facts_tests.rs, render.rs, collateral_standin.rs, tests/mod.rs, templates/hub/model.md) and `hub_card.rs`, `hub_card_tests.rs` are gone.
- Commits `0d4fb7d1a`, `28bd5f6c9`, `c8bfb483f` are ancestors of HEAD.
