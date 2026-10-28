---
phase: 01-retrieval-eval-baseline
plan: 11
subsystem: retrieval-eval-gate
tags: [kb, eval, baseline, ratchet, pre-push, canary, mutation-proof]
requires: ["01-07", "01-10", "01-12"]
provides:
  - "contracts/kb/retrieval-baseline.v1.json: epoch 1 retrieval ratchet (dev/all, dev/seed, tolerance 0.01)"
  - "Real-data ratchet tests in kb_eval_golden_contract.rs (shape, real retriever meets baseline, degraded fails)"
  - "Pre-push fast-tier step `vox kb eval --gate` (runs on every PR via ci.yml pre-push)"
affects: []
tech-stack:
  added: []
  patterns: ["single reader of the diff-base override (prepush_base_ref)", "mutation proofs with before/after presence checks and git diff --quiet restore evidence"]
key-files:
  created:
    - contracts/kb/retrieval-baseline.v1.json
  modified:
    - contracts/index.yaml
    - crates/vox-search/tests/kb_eval_golden_contract.rs
    - crates/vox-cli/src/commands/ci/pre_push.rs
    - docs/src/contributors/local-ci-pre-push.md
key-decisions:
  - "Gate fits the fast tier: median 5.06 s wall (debug) against the 10 s budget, so no AP-05/AP-12 fallback was needed"
  - "Mutation (a) was proven on two files: the plan's named research doc is training_eligible:false so the MENS extractor correctly skips it; an eligible doc proves the extractor path"
requirements-completed: [EVAL-01, EVAL-03, EVAL-04]
status: complete
duration: ~45min
completed: 2026-10-04
commits: 3
plan_head_before: f052f58e0c0f0620f91ca41a7b3a172f7d0ecb09
actuals:
  tokens: 9500
  tasks: 2
  commits: 3
---

# Phase 1 Plan 11: Retrieval baseline, ratchet tests, pre-push gate Summary

Epoch 1 retrieval baseline bootstrapped from the real golden set, proven satisfied by the real BM25 retriever and violated by a reversed one, byte-identical across processes, and wired into the pre-push fast tier so every PR (docs-only included) runs `vox kb eval --gate`. Canary, regression, tighten-only and wiring guards each seen to fail when broken and pass when restored.

## Epoch note (recorded in the baseline)

48 agent-seed + 54 llm-expansion queries (dev 76 / test 26), corpus snapshot of 670 docs at commit `161919567` (01-06 SUMMARY); seeds agent-authored under user delegation (01-CONTEXT D-02) and approved by an independent agent reviewer.

## Baseline metrics (epoch 1, tolerance 0.01, budget 1500 tokens, bytes/4)

| key | doc r@5 | doc r@10 | doc mrr@10 | doc ndcg@10 | chunk r@5 | chunk r@10 | chunk mrr@10 | chunk ndcg@10 | answer_in_budget |
|---|---|---|---|---|---|---|---|---|---|
| dev/all (76 q) | 0.469737 | 0.585307 | 0.556835 | 0.459945 | 0.260307 | 0.316667 | 0.339803 | 0.273916 | 0.355263 |
| dev/seed (35 q) | 0.527143 | 0.668571 | 0.676984 | 0.552024 | 0.34381 | 0.399524 | 0.46 | 0.354592 | 0.428571 |

## What was built

- **Task 1 (`64e8af6fd`)**: three real-data tests added first and run RED (3 of 8 failed: baseline file absent), then GREEN after `vox kb eval --write-baseline`: 8 of 8 pass. `contracts/index.yaml` gained `kb-retrieval-baseline` and `kb-eval-golden-set`; `vox ci contracts-index` OK (193 entries).
- **Task 2 (`6a56512d1`)**: `base_ref_or_default` (pure) + `prepush_base_ref()` (the only reader of the override; `VOX_PREPUSH_BASE` appears once in the file) used by `git_diff_name_only_for_prepush` and the new `step_kb_eval_gate`, appended to the fast list after canonical-map-verify. Test `prepush_base_ref_defaults_to_origin_main` passes.
- **Docs (`72e69fe06`)**: fast-tier row in `docs/src/contributors/local-ci-pre-push.md` now lists the gate (not in the plan's file list; keeps the tier SSOT truthful).

## Determinism (two processes)

`vox kb eval` twice and `vox kb eval --json` twice: `diff` exit 0 for both pairs; `shasum` identical per pair (text a90014a3..., json 762b8c94...). `grep -c dev/all` on the text output = 1.

## Gate timing (debug build, finished golden set, 10841 tracked files)

Gate self-reported 4.89 s, 4.97 s, 4.97 s; wall (python perf_counter, includes process start) 8.40 s (cold first run), 5.06 s, 4.99 s; median wall 5.06 s. Within the 10 s budget, so the AP-05 and AP-12 fallbacks are not triggered. 01-07's 0.98 s was a partial run; this replaces it.

## Mutation proofs (real repo, nothing staged while mutated)

| id | mutation | presence check before to after | gate exit | category tag(s) | restore | `git diff --quiet` after restore | gate after |
|---|---|---|---|---|---|---|---|
| a | append `## Canary probe` (prose paragraph, 300+ chars) + README canary line via grep pipeline to `knowledge-base-practices-research-2026.md` | GUID count 0 to 1 | 1 | `[canary] repo-scan` names the file; MENS extractor does NOT (see deviation) | `git checkout -- file` | 0 | 0 |
| a2 | same append to eligible `docs/src/architecture/where-things-live.md` | GUID count 0 to 1 | 1 | `[canary] mens-extractor` and `[canary] repo-scan`, both name the file; GUID absent from output (count 0) | `git checkout -- file` | 0 | 0 |
| b | baseline `dev/all` doc recall@5 raised by 0.05 (jq temp + mv) | 0.469737 to 0.519737 | 1 | `[regression]` | `git checkout -- file` | 0 | 0 |
| c | same value lowered by 0.05, gate `--base HEAD` | 0.469737 to 0.419737 | 1 | `[tighten-only]` | `git checkout -- file` | 0 | 0 |
| d | OwnedStep removed from `pre_push.rs`, rebuilt | `Box::new(step_kb_eval_gate)` count 1 to 0; dry-run lists step 1 to 0 | n/a | dry-run no longer lists `vox kb eval --gate` | `git checkout -- file`, rebuilt | 0 | dry-run lists 1 |

Final state: `git diff --quiet` on the research doc and the baseline = 0; `git status --porcelain` shows only the untracked `milestone.lock` (not staged); gate passes after the final commit (10841 tracked files scanned, so no planted canary reached a commit).

## Deviations from Plan

**1. [Rule 1 - Plan defect] Mutation (a) target is excluded from MENS by design**
- **Found during:** Task 2, mutation (a).
- **Issue:** `knowledge-base-practices-research-2026.md` carries `training_eligible: false` (LLM-synthesized research), so `walk_and_extract_docs` correctly skips it; the plan's expectation that the MENS extractor names this file cannot hold. The repo scan did name it. Nothing in the gate is wrong.
- **Fix:** ran the same mutation (a2) on a tracked, eligible doc (`where-things-live.md`); both repo-scan and MENS extractor named it. Restored and verified.
- **Files modified:** none (mutations never committed).

**2. [Rule 2] Documented the new fast-tier step** in `local-ci-pre-push.md` (commit `72e69fe06`); doc-pipeline lint and `ci check-links` pass.

**3. Test shape:** `prepush_base_ref_defaults_to_origin_main` tests the pure helper `base_ref_or_default` (env access in tests is unsafe in edition 2024, and the plan caps `VOX_PREPUSH_BASE` mentions at one). The test was written alongside the helper, not observed RED.

**4. Observation, unexplained:** the first mutation-(d) pass recorded `dry_run_lists=0` on the unmutated tree (likely a stale binary at that moment); the proof was re-run from scratch and recorded above with 1 to 0 to 1 and consistent greps. Four further dry-runs on the clean tree all listed the step.

## Verification

- `cargo nextest run -p vox-search --test kb_eval_golden_contract`: 8 passed. `cargo nextest run -p vox-cli --lib pre_push::tests::prepush_base_ref`: 1 passed.
- `cargo clippy -p vox-cli -p vox-search --all-targets -- -D warnings`: clean. `rustfmt --edition 2024 --config-path .` on touched Rust files: clean.
- Not run, per instruction: full `vox ci pre-push`. `pre_push.rs` is far over the 500-line toestub limit; if `arch/god_object` fires in CI, follow the existing "Legacy: already N non-blank lines" suppression precedent (not applied here). `cli-command-surface.generated.md` drift: none expected (no CLI surface change).

## Known Stubs

None.

## Threat Flags

None. T-01-12, T-01-21, T-01-02, T-01-13 mitigated as planned (mutations c, d, restore evidence, timing); T-01-22 accepted.

## Self-Check: PASSED

- Files exist: baseline, contract test, pre_push.rs change, SUMMARY. Commits `64e8af6fd`, `6a56512d1`, `72e69fe06` exist; `git rev-list --count` from plan_head_before = 3.
- No GUID in crates, commit messages or this summary.
