---
phase: 327-std-frame-on-polars-frm-01-03
plan: "04"
subsystem: python-bridge
tags: [std.frame, collect, Arrow, decode_frame_columns, Decimal, DateTime, Records, polars]

requires:
  - phase: 327-03
    provides: materialize_collect, canned-reply hook, family frame execute_plan
provides:
  - decode_frame_columns wire-field decoder without polars
  - materialize_collect mapping wire cells to Records with Decimal/Date/DateTime/Duration
affects: [327-05]

actuals:
  tokens: 8690
  tasks: 2
  commits: 2

plan_head_before: ebba4f5fbeb5d3866a3f906ed2b5ca3b7f54fae3
plan_head_after: c5f751d4bd94605f7457f59a8f953621e6c99b03

tech-stack:
  added: []
  patterns: [arrow-array fixtures without polars, bridge wire fields → compiler chrono/rust_decimal]

key-files:
  created:
    - crates/vox-python-bridge/src/frame_columns.rs
  modified:
    - crates/vox-python-bridge/src/lib.rs
    - crates/vox-compiler/src/eval/builtins.rs

key-decisions:
  - "FrameCell carries coefficient+scale and temporal value+unit; compiler builds Decimal/Date/DateTime/Duration."
  - "Null and naive-timestamp errors are py-plan:; uint64 overflow stays the existing py: message."

patterns-established:
  - "Pattern: decode_frame_columns never calls decode_column; collect uses the same fetch cap as arrays."
  - "Pattern: canned reply body skips the worker so typed decode tests need no polars install."

requirements-completed: [FRM-02]

coverage:
  - id: D1
    description: decimal128, zoned timestamp, date32, and duration decode to wire fields; null, naive timestamp, struct, map, and union build no value; uint8–uint32 widen; uint64 overflow is py:.
    requirement: FRM-02
    verification:
      - kind: unit
        ref: cargo test -p vox-python-bridge --lib frame_column_decode
        status: pass
    human_judgment: false
  - id: D2
    description: materialize_collect turns a canned Arrow body into Records with Decimal, UTC DateTime, Date, and Duration; a null slot is py-plan: and the missing worker is not spawned.
    requirement: FRM-02
    verification:
      - kind: unit
        ref: cargo test -p vox-compiler --lib frame_collect_decodes_canned_body
        status: pass
    human_judgment: false

duration: 14min
completed: 2026-11-04
status: complete
---

# Phase 327 Plan 04: Typed collect without polars Summary

**Arrow IPC fixtures decode to non-null Records with Decimal, UTC DateTime, Date, and Duration; null cells and naive timestamps stay `py-plan:` without importing polars.**

## Performance

- **Duration:** ~14 min
- **Started:** 2026-11-04T06:29:13Z
- **Completed:** 2026-11-04T06:42:59Z
- **Tasks:** 2/2
- **Files modified:** 3

## Accomplishments

- `decode_frame_columns` reads an Arrow IPC batch into `FrameCell` wire fields (no chrono / rust_decimal in the bridge).
- Null cells name step+column with `py-plan:`; naive timestamps include `.replace_time_zone("UTC")`; struct/map/union build no value.
- `materialize_collect` maps wire fields through rust_decimal and chrono into `VoxValue::Records` from a canned body without spawning the worker.

## Task Commits

1. **Task 1: Arrow fixtures decode, nulls and naive timestamps do not** - `b74b31b6d` (feat)
2. **Task 2: collect decodes a canned body into Records** - `c5f751d4b` (feat)

## Files Created/Modified

- `crates/vox-python-bridge/src/frame_columns.rs` - decoder, wire types, encode fixtures
- `crates/vox-python-bridge/src/lib.rs` - `mod frame_columns` and pub use
- `crates/vox-compiler/src/eval/builtins.rs` - `records_from_frame_body`, typed mapping, canned-body test

## Decisions Made

- Temporal wire fields keep Arrow unit (`s`/`ms`/`us`/`ns`); the compiler converts with chrono.
- Decimal is coefficient + scale on the wire; `rust_decimal::Decimal::from_i128_with_scale` lives only in the compiler.
- Encode helpers for collect fixtures stay in `frame_columns` so the compiler test never depends on `arrow-array` directly.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Collect encode helpers on the bridge**
- **Found during:** Task 2
- **Issue:** `builtins.rs` cannot encode multi-column Arrow without an `arrow-array` dep.
- **Fix:** Added `encode_typed_frame_fixture` / `encode_nullable_i64_frame` / `encode_i64_frame` in `frame_columns.rs` and exported them.
- **Files modified:** `frame_columns.rs`, `lib.rs`
- **Committed in:** `c5f751d4b`

---

**Total deviations:** 1 auto-fixed (Rule 2)
**Impact on plan:** Required for the canned-body test; no polars install and no Option reopen.

## Issues Encountered

None beyond the encode-helper surface above.

## User Setup Required

None. Polars install remains deferred (D-12).

## Next Phase Readiness

- Plan 05 can assert a single `execute_plan` round-trip once polars is approved.
- Typed decode path is ready for a real worker body with the same fetch-cap check.

## Self-Check: PASSED

- `crates/vox-python-bridge/src/frame_columns.rs` — FOUND
- commits `b74b31b6d`, `c5f751d4b` — FOUND as ancestors of HEAD
- `cargo test -p vox-python-bridge --lib frame_column_decode` — 1 passed
- `cargo test -p vox-compiler --lib frame_collect_decodes_canned_body` — 1 passed
- STATE.md / ROADMAP.md / REQUIREMENTS.md / mid.rs not in task commits

---
*Phase: 327-std-frame-on-polars-frm-01-03*
*Completed: 2026-11-04*
