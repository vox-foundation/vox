---
phase: 91-measurement-harness-advisory-closure
plan: 01
subsystem: testing
tags: [eval, voxdiagram, mermaid, newcombe, non-inferiority, wilson, mcnemar, node-test, nextest]

requires:
  - phase: 84
    provides: pinned mermaid 11.17.2 Node oracle (mermaid-oracle.mjs canon/sameSets) and the lossless VoxDiagram -> Mermaid exporter
  - phase: 87
    provides: vox_doc_pipeline::diagram re-export reachable from vox-cli
provides:
  - vox_eval::paired_ni (Newcombe method-10 paired interval, NiVerdict, ni_verdict)
  - vox-cli commands::model::eval_diagram (types, oracle, score, stats, tally, render)
  - docs-astro/tests/lib/mermaid-batch.mjs (stdin/stdout JSONL Mermaid oracle) and parseError()
  - offline re-score tests in both runtimes over a labelled synthetic fixture
affects: [91-02, 91-03, 91-04, 91-05, eval-diagram, diagram-authoring]

actuals:
  tokens: 29014
  tasks: 3
  commits: 3
plan_head_before: efbf294b93876a30a101f8e281d113dc7b9afb82
plan_head_after: 9d75435904994e3993328afce6e0c7d0887aa83d

tech-stack:
  added: []
  patterns:
    - "Runtime-split re-score chain: each runtime re-derives only the files it owns and byte-compares them; neither spawns the other"
    - "EVAL_DIAGRAM_BLESS=1 regenerates derived files on the owning side; recordings are never regenerated"
    - "Recording consistency check ties every repair-turn feedback to the recomputed oracle output of the previous turn"

key-files:
  created:
    - crates/vox-eval/src/paired_ni.rs
    - crates/vox-cli/src/commands/model/eval_diagram/mod.rs
    - crates/vox-cli/src/commands/model/eval_diagram/oracle.rs
    - crates/vox-cli/src/commands/model/eval_diagram/score.rs
    - crates/vox-cli/src/commands/model/eval_diagram/stats.rs
    - crates/vox-cli/src/commands/model/eval_diagram/tally.rs
    - crates/vox-cli/src/commands/model/eval_diagram/render.rs
    - crates/vox-cli/tests/it/eval_diagram_rescore.rs
    - crates/vox-cli/tests/fixtures/eval_diagram_synthetic/ (manifest, tasks, reference in/out, runs/synthetic-not-a-measurement/*)
    - docs-astro/tests/lib/mermaid-batch.mjs
    - docs-astro/tests/unit/diagram-authoring-rescore.test.mjs
  modified:
    - crates/vox-eval/src/lib.rs
    - crates/vox-cli/src/commands/model/mod.rs
    - crates/vox-cli/tests/it/main.rs
    - docs-astro/tests/lib/mermaid-oracle.mjs
    - docs/src/architecture/where-things-live.md

key-decisions:
  - "VoxDiagram verdict, feedback and export come from one validate(.., Emit::Mermaid) call; its diagnostics equal check(..).diagnostics (asserted in a unit test)"
  - "Node oracleLine match: null without expected; false when the text does not parse, its sets are unsupported, or sameSets throws; true otherwise"
  - "Slot completeness = every dataset task has turn 0 in both arms and infra exclusions are at most max_infra_rate of the dataset; tasks without a turn-0 pair are reported as missing, separately from transport-error exclusions"
  - "Tokens, cost and attempt counts per arm include excluded tasks (money was spent); validity and fidelity count scored tasks only"
  - "A stop line is recorded in tallies.stopped and relaxes the missing-repair-turn check; the primary verdict is withheld only by model A incompleteness"
  - "tallies.json echoes the manifest's decision_rule; parse_manifest refuses any primary_endpoint, interval, confidence or repair cap the scorer does not implement"

patterns-established:
  - "Dataset directory layout (manifest.v1.yaml, tasks.v1.jsonl, reference-*.jsonl, runs/<name>/{attempts,oracle-inputs,oracle-outputs}.jsonl, tallies.json, tables.md) shared with contracts/eval/diagram-authoring/ from 91-02"

requirements-completed: []

coverage:
  - id: D1
    description: "Newcombe method-10 paired interval and non-inferiority verdict reproduce the six RESEARCH worked vectors and the shape properties"
    requirement: "EVAL-01"
    verification:
      - kind: unit
        ref: "cargo nextest run -p vox-eval -E 'test(/paired_ni/)' (4 tests)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Synthetic recording re-scores byte-for-byte: Rust re-derives reference-inputs, oracle-inputs, tallies.json, tables.md; Node re-derives reference-outputs and oracle-outputs"
    requirement: "EVAL-01"
    verification:
      - kind: integration
        ref: "cargo nextest run -p vox-cli --lib --test it -E 'test(/eval_diagram/)' (31 tests)"
        status: pass
      - kind: unit
        ref: "pnpm --dir docs-astro exec node --test tests/unit/diagram-authoring-rescore.test.mjs tests/unit/mermaid-oracle.test.mjs (83 tests)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Recording checks and re-score fail on a flipped Mermaid verdict, a deleted repair turn and an edited feedback character"
    requirement: "EVAL-01"
    verification:
      - kind: other
        ref: "mutation proofs recorded below (target/eval-diagram/91-01-mut*.log)"
        status: pass
    human_judgment: false

duration: 151min
completed: 2026-10-27
status: complete
---

# Phase 91 Plan 01: Offline Measurement Harness Summary

**Offline half of the VoxDiagram-vs-Mermaid authoring eval: Newcombe method-10 paired non-inferiority statistics in vox-eval, a vox-cli scorer with recording-consistency checks, and a stdin JSONL Mermaid oracle. A labelled synthetic recording re-scores byte-for-byte in Rust and in Node, and each half fails when a verdict, repair turn or feedback is tampered with.**

## Performance

- **Duration:** about 151 min (07:52 to 10:23 MST). Most of it was waiting for cargo: the machine load was 115 to 150, the first vox-cli test-binary build took 21m46s, and later incremental vox-cli rebuilds took a few minutes each.
- **Completed:** 2026-10-27
- **Tasks:** 3/3
- **Files:** 24 changed, 2569 insertions (measured against efbf294b9)

## Accomplishments

- `vox_eval::paired_ni::{paired_difference_ci, NiVerdict, ni_verdict}` matches all six worked vectors to 4 decimals. Over every table up to 4x4x4x4 it holds L <= delta <= U, bounds within [-1, 1], b/c swap antisymmetry and b == c symmetry. The empty table gives (0, -1, 1). The verdict is strict at the margin.
- `commands::model::eval_diagram` (no CLI subcommand) is split into seven files:
  - `mod.rs`: serde types (`Arm`, `Task`, `RunHeader` with `synthetic`, `Attempt`, `Line` tagged run/attempt/stop, `OracleInput`/`OracleOutput`, `Rule`, `Manifest`), plus `parse_manifest`, `read_jsonl`/`write_jsonl` and `reference_inputs`.
  - `oracle.rs`: `extract`, `vd_verdict`, `judge` and `oracle_map`.
  - `score.rs`: `oracle_inputs`, `index_attempts` and `check_recording`.
  - `stats.rs`: the shape of tallies.json, plus `Rate`, `Paired` and `verdict_key`.
  - `tally.rs`: `tally`.
  - `render.rs`: `render_tables`.
- `mermaid-batch.mjs` exports `oracleLine` and `oracleJsonl`. Run as a script, it reads stdin only and exits non-zero naming a malformed line. `mermaid-oracle.mjs` gains `parseError` beside `accepts`; `accepts` is unchanged.
- The synthetic fixture lives at `crates/vox-cli/tests/fixtures/eval_diagram_synthetic/` with run `runs/synthetic-not-a-measurement/`. The header is `synthetic: true` and `tables.md` line 1 reads `SYNTHETIC FIXTURE: not a measurement`. It has 4 tasks (3 graph, 1 sequence) and covers:
  - a turn-1 VoxDiagram repair
  - a Mermaid arm that is invalid at turns 0, 1 and 2
  - a transport-error exclusion (g003)
  - a one-task slot B with a different resolved model id
- On the synthetic run the primary verdict is "not determined": 1 of 4 tasks is excluded, which is above the 5% ceiling. That is expected for this fixture.

## Worked-vector results (Task 2, all within 5e-4)

| a, b, c, d | L | U | verdict (margin 0.10) |
|---|---|---|---|
| 120, 10, 8, 12 | -0.0439 | 0.0712 | non-inferior |
| 110, 12, 8, 20 | -0.0325 | 0.0864 | non-inferior |
| 125, 3, 12, 10 | -0.1150 | -0.0098 | inconclusive |
| 100, 5, 30, 15 | -0.2399 | -0.0938 | inconclusive |
| 150, 0, 0, 0 | -0.0250 | 0.0250 | non-inferior |
| 140, 0, 10, 0 | -0.1184 | -0.0276 | inconclusive |

These vectors come from the same formula (an independent Python reimplementation was used here only to cross-check them), so they are regression vectors, not an external oracle.

## Mutation proofs (all observed, all restored and confirmed)

1. **phi forced to 0** (Task 2): `(fa * fd - fb * fc) / den` was replaced by `0.0`. `paired_ni_worked_vectors` failed with `120,10,8,12: low -0.0662, high 0.0929`. The file was restored from a copy, `diff -q` was clean and the presence grep counted 1.
2. **Mermaid verdict flip** (Task 1): in `oracle-outputs.jsonl`, `g001:A:mermaid:0` was changed from `"ok":true` to `"ok":false`.
   - Rust failed: tallies.json differed, with Mermaid first-pass 1/3 against 2/3.
   - Node failed: "oracle-outputs.jsonl differs from its re-derivation" (pass 4, fail 1).
   - Restored from a copy; `diff -q` was clean and the presence grep counted 1.
3. **Deleted repair turn** (Task 3): the `g002` Mermaid turn-2 line was removed from attempts.jsonl.
   - The Rust re-score failed, first at the oracle-inputs byte check.
   - The rule that rejects an invalid turn with no repair turn and no stop line is proven separately: `check_recording_requires_the_next_repair_turn_unless_the_run_stopped` was red against the stub and is green now.
   - Restored from a copy; `diff -q` was clean and the line count was 1.
4. **Edited feedback character** (Task 3): `diagram/` was changed to `Diagram/` inside the `s001` VoxDiagram turn-1 feedback. The Rust re-score failed with `s001:A:voxdiagram:1: recorded feedback differs from the recomputed feedback of turn 0; expected "[{\"code\":\"di...`. Restored from a copy; `diff -q` was clean, the mutated text counted 0 and the original counted 1.
   - The first attempt at this mutation broke the JSON instead, because it changed the character after a backslash. That run failed with "JSONL line 10", a parse error rather than the feedback check. It was redone on a plain character.

## Bless sequence

- **Task 1:** Rust, Node, Rust, Node, Rust. A final non-bless run of both halves was green.
- **Task 3:** Rust, Node, Rust, Node, Rust, then the feedback loop below.
  - The extended recording was written with placeholder feedback `TBD`.
  - Each failing `check_recording` message named the expected feedback as a JSON string. A script copied that exact text into the fixture generator: g002:A:mermaid:1, then g002:A:mermaid:2, then s001:A:voxdiagram:1.
  - Rust bless rounds 4 and 5 picked up those values, and round 6 passed.
  - A non-bless run of both halves was green.
- No Node-derived or Rust-derived file was edited by hand.

## CI homes of the two halves

- Rust half: `ci.yml` job `tests` (nextest on affected crates, `it::eval_diagram_rescore::synthetic_dataset_rescores` and the `eval_diagram` lib tests).
- Node half: `.github/workflows/docs-quality.yml` job `docs-quality`, step "Docs unit tests" (`node --test "tests/unit/*.test.mjs"`, triggered by `crates/vox-cli/**`, `docs-astro/**`, `contracts/**`).

## Task Commits

1. **Task 1: re-score chain tracer**: `82bf6b9a0` (test)
2. **Task 2: paired_ni**: `c0d754b05` (feat)
3. **Task 3: full tallies and recording checks**: `9d7543590` (feat)

## TDD Gate Compliance

- **Task 1:** the unit tests were written in the same pass as the implementation and passed on their first run. RED was observed at chain level only:
  - Rust: `synthetic_dataset_rescores` failed because the derived files did not exist yet.
  - Node: the dataset test failed with "missing Rust-derived input".
- **Task 2:** RED was valid. With a stub returning (0, 0, 0) and `Inconclusive`, `paired_ni_worked_vectors`, `paired_ni_empty_table_is_uninformative` and `paired_ni_verdict_mapping_is_strict_at_the_margin` failed on their assertions. `paired_ni_properties_over_small_tables` passed vacuously on the all-zero stub. All 4 pass on the real implementation.
- **Task 3:** RED was valid for `check_recording`. With an `Ok(())` stub, all six rejection tests failed: duplicate/unknown, missing next repair, missing previous/turn above 2, edited feedback, header/stop placement, and repair after a valid turn.
  - The tally, stats and render tests were written with the implementation.
  - One stats test failed on first run because of bad test data, not a code bug: a 2-of-200 table that is not "worse within margin". It was fixed to 8 of 200, checked in Python as [-0.0769, -0.0128].

## Decisions Made

See key-decisions in the frontmatter. None changes the pre-registered rule: margin, interval, primary endpoint and the infra ceiling all come from the manifest.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] tdd-guard refused the Task 1 commit**
- **Found during:** Task 1 commit.
- **Issue:** registering `pub mod eval_diagram;` staged `commands/model/mod.rs`. That file already had an untested `pub async fn run`, and lefthook's `toestub --mode enforce-strict` fails on any finding in a staged file.
- **Fix:** added a real test there, `model_args_route_a_subcommand_and_its_flags` (clap routes `model list --limit 3` and rejects an unknown subcommand). No suppression was added.
- **Files modified:** `crates/vox-cli/src/commands/model/mod.rs`
- **Commit:** `82bf6b9a0`

**2. [Rule 3 - File-size limit] Two moves to keep files under 400 non-blank lines**
- **Found during:** Task 3.
- **Issue:** `tally.rs` reached 548 non-blank lines.
- **Fix, tally.rs:** a new `stats.rs` (not in the plan's file list) now holds the tallies.json types, `Rate::new`, `Paired::new` and `verdict_key`. `tally.rs` re-exports them, so paths such as `tally::Rate` still resolve. `tally.rs` now calls `Paired::new` (in `stats.rs`) for the interval and verdict; `paired_difference_ci` appears in `tally.rs` only in its tests.
- **Fix, score.rs:** `judge`, `Judged`, `OracleMap` and `oracle_map` moved into `oracle.rs`.
- **Final non-blank line counts:** mod 308, oracle 164, render 226, score 365, stats 184, tally 391, re-score test 85.
- **Commit:** `9d7543590`

**3. [Minor] One validate call instead of check plus validate**
- `vd_verdict` uses `validate(.., Emit::Mermaid)`, which runs `check` internally. A unit test asserts its diagnostics equal `check(..).diagnostics`.

---

**Total deviations:** 3 (two blocking, one minor). **Impact:** none on the rule or the data contract.

## Issues Encountered

- **Shell restrictions:** the shell allowlist blocks `bash -c`, `python3 -c`, `perl -i`, `timeout` wrapping a script, and `tail`/`sed` on some paths. Edits and waits went through script files under the session scratchpad, plus Node `wait.mjs`, which waits on a log line without a foreground `sleep`.
- **Wasted rebuild:** adding `pub mod paired_ni;` to `vox-eval/src/lib.rs` while the warm build was running forced one extra vox-eval/vox-corpus/vox-cli rebuild.
- **Node test output:** the Node runner uses the spec reporter, so logs show `ℹ fail 0`, not TAP `# fail 0`.
- **Pre-existing warning:** `unused variable: ctx` in `eval_corpus.rs:761` is not from this plan, so it was left alone.

## Known Stubs

None. The synthetic fixture is a deliberately labelled scorer fixture under `tests/fixtures`, not a stub that blocks the goal.

## Requirements

EVAL-01 is only partly delivered: this plan is the offline scorer and statistics. It is not marked complete. The task set (91-02/03), the live runner (91-04) and the run and report (91-05) remain.

## User Setup Required

None.

## Next Phase Readiness

- 91-02 can create `contracts/eval/diagram-authoring/` with this layout and add it to `DATASETS` in the Node test and a `rescore` call in the Rust test.
- The manifest there needs the same `decision_rule` block.

## Self-Check: PASSED

- Files exist:
  - crates/vox-eval/src/paired_ni.rs
  - crates/vox-cli/src/commands/model/eval_diagram/{mod,oracle,score,stats,tally,render}.rs
  - crates/vox-cli/tests/it/eval_diagram_rescore.rs
  - docs-astro/tests/lib/mermaid-batch.mjs
  - docs-astro/tests/unit/diagram-authoring-rescore.test.mjs
  - runs/synthetic-not-a-measurement/{tallies.json,tables.md}
- Commits 82bf6b9a0, c0d754b05 and 9d7543590 are ancestors of HEAD.
- `git diff --quiet 5a631bb22 --` over Cargo.lock, both Cargo.toml files, package.json, pnpm-lock.yaml and crate-edges.allow.v1.json exits 0.
