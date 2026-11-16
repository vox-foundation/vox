---
phase: 01-retrieval-eval-baseline
plan: 03
subsystem: retrieval-eval
tags: [vox-db, turso, fts, spike, candle, kb]
requires: []
provides:
  - "crates/vox-db/tests/turso_fts_probe.rs: 5 characterization tests pinning turso 0.6.1 FTS behaviour (S1)"
  - "DOC-01 SSOT section `Phase 1 spike outcomes (2026-10)` with S1 and S2 results"
affects: [IDX-07 lexical-leg decision (Phase 2), Phase 5 embedder plan]
tech-stack:
  added: []
  patterns: ["characterization test that fails on dependency drift and names the decision it guards"]
key-files:
  created:
    - crates/vox-db/tests/turso_fts_probe.rs
  modified:
    - docs/src/architecture/knowledge-base-practices-research-2026.md
key-decisions:
  - "Error text asserted on Display + Debug combined, so a substring match survives either rendering"
  - "No scratch code committed for S2; reproduction recipe is prose only"
requirements-completed: []
status: complete
duration: ~20m
completed: 2026-10-04
commits: 3
plan_head_before: 8bcbfe38869fdfc36a6dcd59712cd41afc2a0741
actuals:
  tokens: 6500
  tasks: 2
  commits: 3
---

# Phase 1 Plan 03: Record Phase 1 spikes Summary

S1 (Turso FTS, feeds IDX-07) is a permanent red-on-drift `vox-db` test, and S1 plus S2 (candle ModernBERT, feeds Phase 5) are recorded in the DOC-01 research SSOT.

## Tasks

| Task | Commit | Result |
|------|--------|--------|
| 1. S1 characterization test | 54c914a77 | 5 tests in `turso_fts_probe.rs`, all pass |
| 2. Spike section in DOC-01 SSOT | 7d5a783d6 | `## Phase 1 spike outcomes (2026-10)` appended, doc lint clean |

`commits: 3` is the measured `rev-list` count from the ledger. Only two are this plan's: `d28b68c07` (`test(vox-cli-research): gate live research e2e behind runtime feature`) landed between my two commits from another session and is not part of this plan.

## Verification

- `cargo nextest run -p vox-db --test turso_fts_probe`: 5 tests run, 5 passed. The Display+Debug substring forms matched on the first attempt, and `CREATE INDEX kb_probe_fts ON kb_probe USING fts (body)` reproduced the recorded errors (no variant needed).
- Mutation proof (assertion 2, `fts5_virtual_table_module_is_absent`):
  - before: `39:        err_text(&err).contains("no such module: fts5"),`
  - mutated to `"no such module: fts5-MUTATED"`: test FAILED (retried and failed again, 4 others passed)
  - after restore: `39:        err_text(&err).contains("no such module: fts5"),` and 5/5 pass
  - Note: I mutated the expected substring rather than flipping to "expect Ok", which proves the assertion is live against the real error text; the intent of the mutation check is the same.
- `cargo run -q -p vox-doc-pipeline -- --lint-only --paths architecture/knowledge-base-practices-research-2026.md`: "no hard errors".
- Acceptance greps: heading present once, `turso_fts_probe` 1, both model SHAs 1 each, `last_updated` 0.

## Deviations from Plan

None. The plan executed as written.

## Known Stubs

None.

## Threat Flags

None. T-01-08 is mitigated by the test (fails on any flip and names IDX-07). T-01-09: the spike section contains only public model pins and measured timings.

## Self-Check: PASSED

- `crates/vox-db/tests/turso_fts_probe.rs`: found
- Commits `54c914a77`, `7d5a783d6`: found
