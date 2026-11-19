---
phase: 327
phase_name: "std-frame-on-polars-frm-01-03"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 39
  lessons: 3
  patterns: 18
  surprises: 5
missing_artifacts: []
---

# Phase 327 Learnings: std-frame-on-polars-frm-01-03

## Decisions

### std.frame is a sibling allowlist
std.frame is a sibling allowlist. ecosystem-support.yaml and plan-ir.v1.schema.json stay the twelve array ops.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 327-01-SUMMARY.md

---

### FramePlan stores the path it is given and does not store a collect step
FramePlan stores the path it is given and does not store a collect step. Constructors copy the worker counter and do not call the worker.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 327-01-SUMMARY.md

---

### sort stores the column string and the nulls string separately
sort stores the column string and the nulls string separately. nulls is recorded only for first and last.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 327-01-SUMMARY.md

---

### FramePlan::scan_csv, filter, group_by, agg, and sort return another plan...
`FramePlan::scan_csv`, `filter`, `group_by`, `agg`, and `sort` return another plan, copy the process-global worker counter, and do not store a `collect` step.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-01-SUMMARY.md

---

### sort stores the column and nulls separately
`sort` stores the column and `nulls` separately. Only `first` and `last` are recorded.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-01-SUMMARY.md

---

### contracts/python/frame-support.yaml lists the twenty std.frame ops
`contracts/python/frame-support.yaml` lists the twenty `std.frame` ops. `collect` is the only `materializer`. `build.rs` emits `python_frame_ops` from that file.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-01-SUMMARY.md

---

### The array plan-ir name enum was left as the twelve stats-slice ops
The array plan-ir name enum was left as the twelve stats-slice ops. `execute_plan` was not called.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-01-SUMMARY.md

---

### The frame allowlist is a new document
The frame allowlist is a new document. `ecosystem-support.yaml`, its schema, and `plan-ir.v1.schema.json` were not edited.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-01-SUMMARY.md

---

### .and( and .or( are method names
.and( and .or( are method names. LEXER_KEYWORDS stays length 54. frame, scan_csv, and collect are not keywords.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 327-02-SUMMARY.md

---

### frame is Ty::Named("StdFrameNs")
frame is Ty::Named("StdFrameNs"). No Ty::Frame and no new HirType.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 327-02-SUMMARY.md

---

### scan_csv requires Python and Fs
scan_csv requires Python and Fs. collect requires Python. sort, filter, group_by, agg, and expression methods add no effect by themselves.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 327-02-SUMMARY.md

---

### A literal scan path with no parent-dir component is resolved beside the ...
A literal scan path with no parent-dir component is resolved beside the source file. A readable first line is the known schema for missing col checks.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 327-02-SUMMARY.md

---

### .and( and .or( parse as method calls without adding lexer keywords.
`.and(` and `.or(` parse as method calls without adding lexer keywords.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-02-SUMMARY.md

---

### frame is Ty::Named("StdFrameNs") with scan_csv and col; Frame and Expr m...
`frame` is `Ty::Named("StdFrameNs")` with `scan_csv` and `col`; Frame and Expr methods come from `python_frame_ops`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-02-SUMMARY.md

---

### Records exposes num_rows() and column(name) without reusing Ty::Record g...
`Records` exposes `num_rows()` and `column(name)` without reusing `Ty::Record` get.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-02-SUMMARY.md

---

### Effect, sort-nulls, aggregation-alias, and known-header checks land in t...
Effect, sort-nulls, aggregation-alias, and known-header checks land in typeck.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-02-SUMMARY.md

---

### scan_csv denies with fs.scan_csv via fs_resolve_allowed before any Frame...
scan_csv denies with fs.scan_csv via fs_resolve_allowed before any FramePlan is stored.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 327-03-SUMMARY.md

---

### collect is the only execute_plan call; params carry family frame
collect is the only execute_plan call; params carry family frame. No execute_frame.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 327-03-SUMMARY.md

---

### Python keywords and/or use def and_/or_ with HANDLERS keys still named a...
Python keywords and/or use def and_/or_ with HANDLERS keys still named and/or.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 327-03-SUMMARY.md

---

### scan_csv uses fs_resolve_allowed; outside grant and .
`scan_csv` uses `fs_resolve_allowed`; outside grant and `..` paths return `_Denied("fs.scan_csv")` without storing a plan.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-03-SUMMARY.md

---

### import std.frame binds the namespace; plan methods only append steps; fr...
`import std.frame` binds the namespace; plan methods only append steps; `frame_plan_params` emits `family: frame`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-03-SUMMARY.md

---

### materialize_collect is the only frame worker entry; missing py is _Denie...
`materialize_collect` is the only frame worker entry; missing `py` is `_Denied("py.collect")`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-03-SUMMARY.md

---

### frame_ops.py owns HANDLERS (polars imported inside execute_plan); __main...
`frame_ops.py` owns HANDLERS (polars imported inside `execute_plan`); `__main__.py` routes `family == "frame"`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-03-SUMMARY.md

---

### Worker stub for the denial test is an empty .venv/bin/python so venv_pyt...
Worker stub for the denial test is an empty `.venv/bin/python` so `venv_python` selects that path instead of `python3` on PATH; a spawn would not look like `CapabilityDenied`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-03-SUMMARY.md

---

### decode_frame_columns reads an Arrow IPC batch into FrameCell wire fields...
`decode_frame_columns` reads an Arrow IPC batch into `FrameCell` wire fields (no chrono / rust_decimal in the bridge).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-04-SUMMARY.md

---

### Null cells name step+column with py-plan:; naive timestamps include .rep...
Null cells name step+column with `py-plan:`; naive timestamps include `.replace_time_zone("UTC")`; struct/map/union build no value.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-04-SUMMARY.md

---

### materialize_collect maps wire fields through rust_decimal and chrono int...
`materialize_collect` maps wire fields through rust_decimal and chrono into `VoxValue::Records` from a canned body without spawning the worker.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-04-SUMMARY.md

---

### Temporal wire fields keep Arrow unit (s/ms/us/ns); the compiler converts...
Temporal wire fields keep Arrow unit (`s`/`ms`/`us`/`ns`); the compiler converts with chrono.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-04-SUMMARY.md

---

### Decimal is coefficient + scale on the wire; rust_decimal::Decimal::from_...
Decimal is coefficient + scale on the wire; `rust_decimal::Decimal::from_i128_with_scale` lives only in the compiler.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-04-SUMMARY.md

---

### User approved polars==2.1.0 on PyPI (resume signal
User approved polars==2.1.0 on PyPI (resume signal: approved for polars).

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 327-05-SUMMARY.md

---

### Real worker execution against frame_ops.py succeeded without changes to ...
Real worker execution against frame_ops.py succeeded without changes to frame_ops.py.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 327-05-SUMMARY.md

---

### A missing Records column is Result Err carrying a string
A missing Records column is Result Err carrying a string. It is not a nullable cell, and the methods are not registered on Ty::Record.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 327-06-SUMMARY.md

---

### Date is chrono::NaiveDate, DateTime is chrono::DateTime<Utc>, and Durati...
Date is chrono::NaiveDate, DateTime is chrono::DateTime<Utc>, and Duration is chrono::Duration. Decimal stays the existing variant.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 327-06-SUMMARY.md

---

### The golden chain is one statement
The golden chain is one statement. sort passes nulls as the string last.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 327-06-SUMMARY.md

---

### VoxValue gained FramePlan, FrameExpr, Records, Date, DateTime, and Durat...
`VoxValue` gained `FramePlan`, `FrameExpr`, `Records`, `Date`, `DateTime`, and `Duration` next to `ArrayPlan`. `Decimal` was left as it was. No `HirType` variant and no `Ty::Frame`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-06-SUMMARY.md

---

### records_num_rows_and_column builds two int revenue cells
`records_num_rows_and_column` builds two int revenue cells. `num_rows` is 2. `column("revenue")` is the list 10, 5. A missing name is `Result` `Err`, not a nullable cell.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-06-SUMMARY.md

---

### examples/golden/py_frame_report.vox scans fixtures/py_frame_report.csv, ...
`examples/golden/py_frame_report.vox` scans `fixtures/py_frame_report.csv`, filters region west through `frame.col`, groups by month, sums amount aliased to revenue, sorts month with nulls last, and collects once. `KNOWN_INTERP_GAPS` lists it so the in-process sweep does not run it.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-06-SUMMARY.md

---

### A missing column name returns VoxValue::Result(Err(...)) with a string m...
A missing column name returns `VoxValue::Result(Err(...))` with a string message. The lookup does not build a nullable cell, and it is not wired to `Ty::Record`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-06-SUMMARY.md

---

### Date, DateTime, and Duration use the chrono types already on vox-compile...
`Date`, `DateTime`, and `Duration` use the chrono types already on `vox-compiler`. Nothing was added to `vox-python-bridge`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 327-06-SUMMARY.md

## Lessons

### The first cargo test -p vox-compiler --lib frame_plan_collect_is_one_rou...
The first `cargo test -p vox-compiler --lib frame_plan_collect_is_one_round_trip` was killed with exit 137 while `vox-compiler` was still compiling. The same command was retried once. That run exited 101 because `FramePlan` and `python_frame_support` were not defined. After the implementation, the test exited 0 with 1 passed. `frame_support_collect_is_only_materializer` and `array_plan_zscore_is_pure` also exited 0.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 327-01-SUMMARY.md

---

### None beyond the keyword HANDLERS shape above.
None beyond the keyword HANDLERS shape above.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 327-03-SUMMARY.md

---

### None beyond the encode-helper surface above.
None beyond the encode-helper surface above.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 327-04-SUMMARY.md

## Patterns

### Pattern: emit_python_frame_ops mirrors the array emitter and panics on a...
Pattern: emit_python_frame_ops mirrors the array emitter and panics on an empty signature, an empty handler, a handler that differs from name, a non-mapping expected, or an unknown kind.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-01-SUMMARY.md

---

### Pattern: collect is the only PythonFrameOpKind::Materializer.
Pattern: collect is the only PythonFrameOpKind::Materializer.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-01-SUMMARY.md

---

### sibling allowlist emitted beside the array table
sibling allowlist emitted beside the array table

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-01-SUMMARY.md

---

### host plan copies a worker counter and never increments it
host plan copies a worker counter and never increments it

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-01-SUMMARY.md

---

### Pattern: check_frame_sort_args mirrors check_array_sort_args for nulls=f...
Pattern: check_frame_sort_args mirrors check_array_sort_args for nulls=first|last.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-02-SUMMARY.md

---

### Pattern: typecheck_hir receives source_path so frame header checks can o...
Pattern: typecheck_hir receives source_path so frame header checks can open the CSV.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-02-SUMMARY.md

---

### keyword tokens as method names via parse_ident_name
keyword tokens as method names via parse_ident_name

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-02-SUMMARY.md

---

### Ty::Named carriers for frame types
Ty::Named carriers for frame types

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-02-SUMMARY.md

---

### literal scan path header check
literal scan path header check

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-02-SUMMARY.md

---

### Pattern: materialize_collect mirrors materialize_to_list (py denial, can...
Pattern: materialize_collect mirrors materialize_to_list (py denial, canned-body hook, one execute_plan).

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-03-SUMMARY.md

---

### Pattern: __main__ routes family frame to frame_ops and leaves array_ops ...
Pattern: __main__ routes family frame to frame_ops and leaves array_ops on every other plan.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-03-SUMMARY.md

---

### fs_resolve_allowed before FramePlan store
fs_resolve_allowed before FramePlan store

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-03-SUMMARY.md

---

### family frame on execute_plan
family frame on execute_plan

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-03-SUMMARY.md

---

### polars import inside execute_plan
polars import inside execute_plan

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-03-SUMMARY.md

---

### fail-closed uv worker integration tests
fail-closed uv worker integration tests

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-05-SUMMARY.md

---

### Pattern: frame runtime values sit next to ArrayPlan. Frame ops stay meth...
Pattern: frame runtime values sit next to ArrayPlan. Frame ops stay methods; the binary-op match did not gain arithmetic arms.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-06-SUMMARY.md

---

### Records stores concrete columns
Records stores concrete columns

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-06-SUMMARY.md

---

### a missing name is Result Err
a missing name is Result Err

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 327-06-SUMMARY.md

## Surprises

### Estimate variance on 327-01
Estimate variance on 327-01: planned 32000 tokens vs 6016 actual (0.19x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 327-01-PLAN.md, 327-01-SUMMARY.md

---

### Estimate variance on 327-02
Estimate variance on 327-02: planned 50000 tokens vs 14000 actual (0.28x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 327-02-PLAN.md, 327-02-SUMMARY.md

---

### Estimate variance on 327-03
Estimate variance on 327-03: planned 52000 tokens vs 8184 actual (0.16x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 327-03-PLAN.md, 327-03-SUMMARY.md

---

### Estimate variance on 327-04
Estimate variance on 327-04: planned 40000 tokens vs 8690 actual (0.22x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 327-04-PLAN.md, 327-04-SUMMARY.md

---

### Estimate variance on 327-06
Estimate variance on 327-06: planned 28000 tokens vs 2371 actual (0.08x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 327-06-PLAN.md, 327-06-SUMMARY.md

