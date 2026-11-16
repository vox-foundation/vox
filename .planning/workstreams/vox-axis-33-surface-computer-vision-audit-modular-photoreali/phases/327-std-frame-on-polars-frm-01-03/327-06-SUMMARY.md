---
phase: 327-std-frame-on-polars-frm-01-03
plan: "06"
subsystem: compiler
tags: [std.frame, VoxValue, Records, golden]

requires:
  - phase: 327-01
    provides: FramePlan and FrameExpr on the host, with collect left off the plan
provides:
  - VoxValue variants FramePlan, FrameExpr, Records, Date, DateTime, and Duration
  - Records num_rows and column without a worker
  - examples/golden/py_frame_report.vox and its CSV fixture, listed as an interpreter gap
affects: [327-02, 327-03, 327-04, 327-05]

actuals:
  tokens: 2371
  tasks: 1
  commits: 2

tech-stack:
  added: []
  patterns: [Records stores concrete columns, a missing name is Result Err]

key-files:
  created:
    - examples/golden/py_frame_report.vox
    - examples/golden/fixtures/py_frame_report.csv
  modified:
    - crates/vox-compiler/src/eval/value.rs
    - crates/vox-compiler/src/eval/builtins.rs
    - crates/vox-compiler/tests/golden_interp_test.rs

key-decisions:
  - "A missing Records column is Result Err carrying a string. It is not a nullable cell, and the methods are not registered on Ty::Record."
  - "Date is chrono::NaiveDate, DateTime is chrono::DateTime<Utc>, and Duration is chrono::Duration. Decimal stays the existing variant."
  - "The golden chain is one statement. sort passes nulls as the string last."

patterns-established:
  - "Pattern: frame runtime values sit next to ArrayPlan. Frame ops stay methods; the binary-op match did not gain arithmetic arms."

requirements-completed: [FRM-01]

coverage:
  - id: D1
    description: Records of two int revenue cells returns num_rows 2 and column revenue as the list 10, 5. A missing name is an error value.
    requirement: FRM-01
    verification:
      - kind: unit
        ref: cargo test -p vox-compiler --lib records_num_rows_and_column
        status: pass
    human_judgment: false
  - id: D2
    description: py_frame_report.vox is listed in KNOWN_INTERP_GAPS with reason needs the uv python worker, beside the CSV fixture, and contains one collect.
    requirement: FRM-01
    verification:
      - kind: unit
        ref: cargo test -p vox-compiler --test golden_interp_test py_frame_report_is_an_interp_gap
        status: pass
    human_judgment: false

duration: 7min
completed: 2026-11-04
status: complete
plan_head_before: 6623f8305db817645d563fead098389f5c2d2999
plan_head_after: 29ff6d34a4e6c6ee3eb9a2a9af7eb81d0961f8e5
---

# Phase 327 Plan 06: Runtime values, golden source, and the CSV fixture Summary

**`Records` answers `num_rows` and `column` without a worker, and `py_frame_report.vox` is an interpreter-sweep gap beside its CSV.**

## Performance

- **Duration:** 7 min
- **Started:** 2026-11-04T05:07:26Z
- **Completed:** 2026-11-04T05:14:44Z
- **Tasks:** 1
- **Files modified:** 5

## Accomplishments

- `VoxValue` gained `FramePlan`, `FrameExpr`, `Records`, `Date`, `DateTime`, and `Duration` next to `ArrayPlan`. `Decimal` was left as it was. No `HirType` variant and no `Ty::Frame`.
- `records_num_rows_and_column` builds two int revenue cells. `num_rows` is 2. `column("revenue")` is the list 10, 5. A missing name is `Result` `Err`, not a nullable cell.
- `examples/golden/py_frame_report.vox` scans `fixtures/py_frame_report.csv`, filters region west through `frame.col`, groups by month, sums amount aliased to revenue, sorts month with nulls last, and collects once. `KNOWN_INTERP_GAPS` lists it so the in-process sweep does not run it.

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: failing Records column test** - `7a1745a2b` (test)
2. **Task 1 GREEN: runtime values, golden, and CSV** - `29ff6d34a` (feat)

## Files Created/Modified

- `crates/vox-compiler/src/eval/value.rs` - `FramePlan`, `FrameExpr`, `Records`, `Date`, `DateTime`, `Duration`
- `crates/vox-compiler/src/eval/builtins.rs` - `num_rows` and `column`, plus type name and display arms
- `crates/vox-compiler/tests/golden_interp_test.rs` - `py_frame_report.vox` gap and `py_frame_report_is_an_interp_gap`
- `examples/golden/py_frame_report.vox` - one `collect` of the west revenue report
- `examples/golden/fixtures/py_frame_report.csv` - header `month,region,amount` and the five data rows

## Decisions Made

- A missing column name returns `VoxValue::Result(Err(...))` with a string message. The lookup does not build a nullable cell, and it is not wired to `Ty::Record`.
- `Date`, `DateTime`, and `Duration` use the chrono types already on `vox-compiler`. Nothing was added to `vox-python-bridge`.
- The golden uses one statement for the plan chain, because a newline ends an expression before a following `.method`. `nulls="last"` is a string, matching `sort(column: str, nulls: str)`.

## Deviations from Plan

None - plan executed exactly as written.

## TDD

RED: `records_num_rows_and_column` was committed while `VoxValue::Records` did not exist. `cargo test -p vox-compiler --lib records_num_rows_and_column` exited 101 with rustc E0599 (`no variant named Records`). The test did not run an assertion. That is the failure the plan asked for ("fail until Records exists"). Semantic assessment: the named test is the compile failure, not an unrelated fixture. Cargo's text report is not a TDD-classifier adapter, and this plan's frontmatter type is `execute`.

GREEN: after the variant, the methods, the golden, and the gap entry, the same lib test exited 0 with 1 passed. `py_frame_report_is_an_interp_gap` exited 0 with 1 passed.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 02 can typecheck `std.frame` against these runtime values. Plan 03 and plan 04 can build `Records` from `collect`. The golden stays an interpreter gap until plan 05, after the polars checkpoint. `execute_plan` was not called. Polars was not installed.

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: crates/vox-compiler/src/eval/value.rs
- FOUND: crates/vox-compiler/src/eval/builtins.rs
- FOUND: examples/golden/py_frame_report.vox
- FOUND: examples/golden/fixtures/py_frame_report.csv
- FOUND: crates/vox-compiler/tests/golden_interp_test.rs
- FOUND: 7a1745a2b
- FOUND: 29ff6d34a

---
*Phase: 327-std-frame-on-polars-frm-01-03*
*Completed: 2026-11-04*
