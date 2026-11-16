---
phase: 327-std-frame-on-polars-frm-01-03
plan: "02"
subsystem: compiler
tags: [std.frame, typeck, parser, and-or, known-schema]

requires:
  - phase: 327-01
    provides: FramePlan and the sibling frame allowlist
  - phase: 327-06
    provides: VoxValue::Records, the golden source, and the CSV fixture
provides:
  - Token::And and Token::Or parse as method names on .and( and .or(
  - StdFrameNs, Frame, Expr, and Records method types
  - scan_csv / collect effect checks, sort nulls, aggregation alias, and known-header missing-column diagnostics
affects: [327-03, 327-04, 327-05]

actuals:
  tokens: 14000
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns: [keyword tokens as method names via parse_ident_name, Ty::Named carriers for frame types, literal scan path header check]

key-files:
  created: []
  modified:
    - crates/vox-compiler/src/parser/descent/decl/head.rs
    - crates/vox-compiler/src/parser/descent/expr/pratt_match.rs
    - crates/vox-language-surface/src/lib.rs
    - crates/vox-compiler/src/builtin_registry.rs
    - crates/vox-compiler/src/typeck/builtins.rs
    - crates/vox-compiler/src/typeck/checker/expr_field.rs
    - crates/vox-compiler/src/typeck/checker/expr.rs
    - crates/vox-compiler/src/typeck/checker/expr_ops.rs
    - crates/vox-compiler/src/typeck/checker/mod.rs
    - crates/vox-compiler/src/typeck/mod.rs

key-decisions:
  - ".and( and .or( are method names. LEXER_KEYWORDS stays length 54. frame, scan_csv, and collect are not keywords."
  - "frame is Ty::Named(\"StdFrameNs\"). No Ty::Frame and no new HirType."
  - "scan_csv requires Python and Fs. collect requires Python. sort, filter, group_by, agg, and expression methods add no effect by themselves."
  - "A literal scan path with no parent-dir component is resolved beside the source file. A readable first line is the known schema for missing col checks."

patterns-established:
  - "Pattern: check_frame_sort_args mirrors check_array_sort_args for nulls=first|last."
  - "Pattern: typecheck_hir receives source_path so frame header checks can open the CSV."

requirements-completed: [FRM-01, FRM-02]

coverage:
  - id: D1
    description: col("a").gt(1).and(col("b").lt(2)) parses as a method chain, and LEXER_KEYWORDS length stays 54.
    requirement: FRM-01
    verification:
      - kind: unit
        ref: cargo test -p vox-compiler --lib method_and_or_parse
        status: pass
      - kind: unit
        ref: cargo test -p vox-language-surface --lib lexer_keywords_freeze_std_array_idents
        status: pass
    human_judgment: false
  - id: D2
    description: import std.frame binds StdFrameNs; scan_csv and col are namespace methods; Records exposes num_rows and column.
    requirement: FRM-01
    verification:
      - kind: unit
        ref: cargo test -p vox-compiler --lib frame_typeck_namespace_methods
        status: pass
      - kind: unit
        ref: cargo test -p vox-compiler --lib frame_typeck_records_column
        status: pass
    human_judgment: false
  - id: D3
    description: scan_csv and collect require effects; sort requires nulls; aggregations require alias; a known missing column is a compile error.
    requirement: FRM-02
    verification:
      - kind: unit
        ref: cargo test -p vox-compiler --lib frame_typeck_
        status: pass
    human_judgment: false

duration: 50min
completed: 2026-11-03
status: complete
---

# Phase 327 Plan 02 Summary

**The golden pipeline parses and typechecks: `.and(` / `.or(` are methods, `StdFrameNs` is registered, and a literal CSV header rejects a missing column.**

## Performance

- **Duration:** ~50 min (executor hit usage limits mid task 3; task 3 finished on the orchestrator)
- **Tasks:** 3/3
- **Commits:** `701aed711`, `feb0c2794`, `2d7ab2f3f`

## Accomplishments

- `.and(` and `.or(` parse as method calls without adding lexer keywords.
- `frame` is `Ty::Named("StdFrameNs")` with `scan_csv` and `col`; Frame and Expr methods come from `python_frame_ops`.
- `Records` exposes `num_rows()` and `column(name)` without reusing `Ty::Record` get.
- Effect, sort-nulls, aggregation-alias, and known-header checks land in typeck.

## Deviations

- Task 3 was finished after the original executor stopped on a usage limit. The six `frame_typeck_` tests were already green in the working tree.
- An unrelated `mid.rs` activity/`uses` edit was left unstaged.
- Added `typecheck_hir_module_with_path_accepts_source_path` so touching `typeck/mod.rs` clears the pre-commit TDD guard.

## Verification

- `cargo test -p vox-compiler --lib method_and_or_parse` — pass
- `cargo test -p vox-language-surface --lib lexer_keywords_freeze_std_array_idents` — pass
- `cargo test -p vox-compiler --lib frame_typeck_` — 6 passed
