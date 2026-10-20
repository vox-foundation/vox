---
phase: 12-code-gen-benchmark-plumbing-provenance
plan: 04
subsystem: testing
tags: [vox-eval, provenance, bootstrap-ci, pass-at-k, completeness-gate]
requires: []
provides:
  - "vox_eval::provenance: RunProvenance, RunIdentity, SamplingRecord, MensLineage, missing_required"
  - "vox_eval::report: EvalCorpusReport (schema 2), ReportInputs, ArmStats, SampleRecord, FailureClass, VerifyFacts, classify_failure, arm_stats, per_problem_estimates, assemble, recheck_complete"
affects: [12-05, 12-07, 12-09]
status: complete
actuals:
  tokens: 18800
  tasks: 3
  commits: 3
plan_head_before: f2af31ebdb5b160db6748287b126d7f1266aafdf
tech-stack:
  added: []
  patterns:
    - "one completeness gate (gate_reasons) shared by assemble and recheck_complete"
    - "required provenance fields are Option with no serde default: absent key == None == reported missing"
key-files:
  created:
    - crates/vox-eval/src/provenance.rs
    - crates/vox-eval/src/report.rs
  modified:
    - crates/vox-eval/src/lib.rs
key-decisions:
  - "Completeness lives in a private gate_reasons fn called by assemble and recheck_complete, instead of two copies of the rule"
  - "Samples with n < k or n == 0 are excluded from scoring and listed in unscored_fixture_ids; corpus_score's k > n panic is untouched"
requirements-completed: [BENCH-03, BENCH-04]
---

# Phase 12 Plan 04: Typed eval report and provenance gate Summary

Typed schema-2 eval-corpus report with per-arm bootstrap CIs, reps, analysis seed and MDD, plus a single section-5 provenance completeness gate that is re-derived on read.

## Accomplishments

- `RunProvenance` (+ `RunIdentity`, `SamplingRecord`, `MensLineage`) with `missing_required()`: fixed declaration order, blank/absent counts as missing, `mens/*` ids need every lineage field, external ids need provider, request_date and at least one provider-reported model. `corpus_manifest_hash` is informational. No credential field anywhere (`api_key` grep: 0).
- `EvalCorpusReport::assemble` reuses `bootstrap_ci` / `min_detectable_difference` / `score_corpus` (no new statistics), overwrites `score.n_infra_errors` with the real count, sorts fixtures and samples, builds the failure breakdown and non-infra compile rate.
- `run_complete` false with a named reason for: spend cap, no samples planned, infra rate above ceiling (inclusive at the ceiling), each missing provenance field, verifier mismatch, no scored problems.
- `recheck_complete()` recomputes from stored provenance and counts; a forged `run_complete: true` with identity removed, spend cap, mismatch, missing pass@k or inflated infra count rechecks false.
- Edges: empty set gives `None` stats, single problem gives degenerate CI with MDD 0.25, `n < k` fixtures are listed unscored and never panic.

## Verification

- `cargo nextest run -p vox-eval --lib -E 'test(report::) | test(provenance::)'`: 16 passed (9 report, 7 provenance).
- `cargo clippy -p vox-eval --all-targets -- -D warnings`: clean.
- Acceptance greps: `serde(default)` = 1 (only `provider_reported_models`); `api_key` = 0 across both modules; `fn bootstrap_ci` in report.rs = 0; 3 public fns `classify_failure` / `per_problem_estimates` / `arm_stats`; `corpus_score.rs` and `corpus_stats.rs` untouched in all three commits.

### Mutation proof 1 (before/after presence check)

- Before: `grep -c 'missing_required()' report.rs` = 1 (the single gate call inside `gate_reasons`, which both `assemble` and `recheck_complete` route through).
- Mutated: removed the `missing_required` term from the gate: count 0. `report::tests::gate_rejects_each_cause` FAILED (3 retries, panic at the missing-provenance case).
- Restored (byte-identical to saved copy): count 1, tests pass.

## Deviations from Plan

**1. [Structure] One gate call site instead of two.** The plan expected `missing_required()` twice in report.rs (assemble gate plus recheck). Both now share `gate_reasons`, so the count is 1 -> 0 -> 1 rather than 2 -> 1 -> 2. Same property (deleting the term fails `gate_rejects_each_cause`, and also weakens recheck), less duplicated rule.

**2. [Test-first] Task 1 provenance tests.** The repo's tdd-guard blocked the first commit (a pub fn without a `#[test]` in `provenance.rs`), so two small provenance tests were added to Task 1; Task 2's tests were written red first (7 of 11 failing) before `missing_required` was completed. Task 1's tracer and Task 3's edge tests passed on first run because the implementation they pin already existed from Tasks 1/2.

**3. [Formatting] The lefthook fmt-fix hook reformatted `report.rs` in the Task 3 commit (edition 2024 style, which my first `rustfmt --edition 2021` pass did not match).** Content unchanged; working tree matches HEAD. Later passes used `--edition 2024`.

## Known Stubs

None.

## Threat Flags

None. The report types carry no credential fields (T-12-04-03); `recheck_complete` mitigates the forged-flag case (T-12-04-01); no serde defaults on required fields (T-12-04-02).

## Self-Check: PASSED

- provenance.rs, report.rs present; lib.rs registers both modules.
- Commits present: 5d0ba8b88, 946b8b62e, 147a2ea6c.
