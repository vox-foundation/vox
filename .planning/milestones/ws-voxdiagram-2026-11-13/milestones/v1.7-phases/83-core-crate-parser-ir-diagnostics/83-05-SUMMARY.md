---
phase: 83-core-crate-parser-ir-diagnostics
plan: 05
subsystem: diagram-core
tags: [rust, formatter, proptest, insta, golden, round-trip]

requires:
  - phase: 83-03
    provides: "Ordered-entry AST with comment/blank trivia, fence scanner, sequence IR types"
  - phase: 83-04
    provides: "Recovery, full diagnostic catalogue, caps (fmt rides on check's validity)"
provides:
  - "vox_diagram::fmt: canonical, idempotent formatter; Some(text) iff check has no Error, never panics"
  - "print::to_source: pure AST -> text printer (4-space indent, declaration order, comments at attachment points, blank-run collapse, LF, one trailing newline)"
  - "tests/support: model-first graph generator (S1), noisy printer (S2), model IR oracle, span stripper, reusable by Plans 06 and 07"
  - "Properties P1 (oracle), P2 (idempotence), P3 (AST+IR round trip), P5 (JSON stability)"
affects: [83-06, 83-07, 84, 88]

key-files:
  created:
    - crates/vox-diagram/src/print.rs
    - crates/vox-diagram/tests/fmt_golden.rs
    - crates/vox-diagram/tests/fmt_props.rs
    - crates/vox-diagram/tests/support/mod.rs
    - crates/vox-diagram/tests/support/noisy.rs
    - crates/vox-diagram/tests/snapshots/fmt_golden__*.snap (4)
  modified:
    - crates/vox-diagram/src/lib.rs

key-decisions:
  - "fmt = check gate + parse + print (two linear passes); check.rs untouched"
  - "Generator ids are unique by a DFS counter suffix (kebab and snake styles), so reserved words and Mermaid statement words can never collide and uniqueness needs no pool"
  - "Edges are generated as indices into the document's declaration-ordered nodes (mod count), so every edge targets a declared node, forward references included; an edge in a node-less document is dropped"
  - "Noise is a shrinkable die stream where die 0 always means the plainest layout, so counterexamples shrink toward clean text"
  - "S2 printer split into tests/support/noisy.rs (400-line cap); mod.rs keeps a thin noisy_source wrapper so the interface is unchanged"

patterns-established:
  - "Golden harness: once(src) helper + explicit fmt(&a) == a idempotence assert per case"
  - "Independent oracle: expected IR built from the model with no parser/resolver call"

requirements-completed: [DGM-05]  # DGM-01 stays Pending: fixes (Plan 06) and Mermaid-isms (Plan 07) still outstanding

duration: about 45m
completed: 2026-10-16
status: complete

actuals:
  tokens: 9500
  tasks: 3
  commits: 3
plan_head_before: 36b0c967d35e6070101bb82378fa930c90a7f575
plan_head_after: c71ef28e8f38275aa96c575f0a2eb84bf6441b12
---

# Phase 83 Plan 05: Canonical formatter and shared generators Summary

**`fmt` prints every valid diagram in one canonical form (the canonical example is a byte-for-byte fixed point), and 128 generated noisy documents per property prove it idempotent, meaning-preserving and in agreement with an independent model oracle.**

## Accomplishments

- Tracer: `fmt(source)` flows check -> parse -> `to_source`; `canonical.vdg` is its own canonical form. Shadowed-id Warning documents format; any Error returns `None`.
- Printer rules, all golden-locked: 4-space indent, one statement per line, order preserved (`direction` never hoisted), ` { shape: x }` only when not rect, multi-line attr blocks collapse, labels re-escaped (`\\`, `\"`, `\n`; tab raw), blank runs collapse to one, none at block edges (A3), `graph { // h`, `node ... // t`, container header comment after `{`, close comment after `}`, comments before `graph` and after the final `}`, `//x` stays `//x`, CRLF in gives LF out, empty container on two lines, comment-only container keeps comments.
- 14 `fmt_golden` tests (4 reviewed insta snapshots) and 3 `print::tests`; each golden asserts `fmt(fmt(x)) == fmt(x)`.
- Generators: containers nested at most 3 deep, six shapes, optional direction in a random root slot, Unicode and escape-heavy labels (A8-forbidden characters and `\r` excluded), noisy layout (tabs/spaces/odd indent, 0-3 blank lines, own-line and trailing comments incl. directly before `}`, explicit `shape: rect`, `{shape:x}`, `, }`, multi-line attr blocks, zero-space `a->b`, trailing spaces, missing final newline, CRLF).
- Properties (128 cases each, `--retries 0`): P1 zero diagnostics and IR equals the model oracle; P2 idempotent plus LF/one-trailing-newline/no-trailing-whitespace; P3 span-stripped AST (comments and `blank_before` included) and IR unchanged by fmt; P5 IR JSON identical across two checks.
- A temporary 6,000-case run of all four properties passed too: no counterexample, so no parser or AST trivia bug surfaced and `ast.rs`, `parser/*` were not touched.
- Full suite 112 tests green under `INSTA_UPDATE=no`; `cargo clippy -p vox-diagram --all-targets -- -D warnings` and `cargo fmt -p vox-diagram --check` clean; all files under 400 non-blank lines (largest: `tests/support/noisy.rs` 239); no `*.snap.new`, no `proptest-regressions` file.

## Task Commits

1. **Task 1 (tracer): fmt end to end** - `17aa47e93`
2. **Task 2: comment/blank/escape/normalisation goldens** - `ec5c51026`
3. **Task 3: generators and property suite** - `c71ef28e8`

## TDD / RED evidence

- Task 1: `print::tests` and `canonical_example_is_fixed_point` written against a stub (`fmt` returned `None`); run: `4 tests run: 0 passed, 4 failed`. GREEN after implementing the printer: 4 passed.
- Task 2: the printer from Task 1 already satisfied the ten plain-assertion goldens, so they passed on first run. RED was the four snapshot-based goldens: `14 tests run: 10 passed, 4 failed` (`--retries 0`, `INSTA_UPDATE=no`, snapshots missing). Snapshots were then created with `INSTA_UPDATE=always` and each `.snap` was reviewed against the behavior block before GREEN (14 passed). Honest note: these goldens are lock-in tests over behavior implemented in Task 1, not discovery tests.
- Task 3: the four properties passed on the first run (also at 6,000 cases), so there was no natural RED. Sensitivity was proven by mutation instead (below). The tests were written against a complete printer and parser.

## Mutation proofs

Each mutation had a before/after presence check and was restored from a pre-mutation copy (`diff` clean afterwards).
- **(a) blank-line preservation** (`print.rs`): `if item.blank_before {` (count 1) -> `if false && item.blank_before {` (count 1) -> `p3_fmt_preserves_ast_and_ir`, `canonical_example_is_fixed_point`, `blank_line_runs_collapse_to_one`, `crlf_input_formats_to_lf`, `every_construct_formats_stably` FAILED (13 passed, 5 failed).
- **(b) quote re-escaping** (`print.rs`): `'"'` arm changed to push a bare quote -> `p2_fmt_is_idempotent`, `p3_fmt_preserves_ast_and_ir`, `labels_are_re_escaped_and_tabs_stay_raw`, `every_construct_formats_stably` FAILED (14 passed, 4 failed).
- **(c) validity gate** (`print.rs`): `if !check(source).valid {` -> `if false && ...` -> `errors_refuse_to_format` and `print::tests::errors_refuse_and_warnings_format` FAILED (19 passed, 2 failed).
- The mutation runs wrote `tests/fmt_props.proptest-regressions` (seeds that fail only under the mutants); it was deleted, not committed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Test support exceeded the 400-line file cap**
- **Found during:** Task 3 (rustfmt expanded `support/mod.rs` to 449 non-blank lines)
- **Fix:** moved the S2 printer into `tests/support/noisy.rs`; `mod.rs` keeps a `noisy_source` wrapper, so the planned interface is unchanged.
- **Files modified:** `crates/vox-diagram/tests/support/mod.rs`, `crates/vox-diagram/tests/support/noisy.rs`
- **Commit:** `c71ef28e8`

### Plan-versus-reality notes

- Task 2's `snapshots/` dir only holds the four new `fmt_golden__*.snap` (plan asked for at least 3).
- `lib.rs` change: `mod print;` (doc-commented) and `pub use print::fmt;` as planned; the Cargo manifests are untouched.
- A blank line between the last leading comment and `graph` is not stored in the AST (pre-existing Plan 02 design), so fmt drops it; this is idempotent and covered by `comments_around_the_graph_stay_put`.

## Auth Gates

None.

## Known Stubs

None. Deliberately outside this plan (scheduled): `fixes` on diagnostics and `apply_fixes`/`repair` (Plan 06), Mermaid-ism codes (Plan 07); `fmt` returns `None` on those documents until repaired.

## Threat Flags

None. T-83-16 and T-83-17 are mitigated (`fmt` refuses on Errors, P3 proves meaning is preserved, one label-escaping function, goldens per comment position); T-83-18 accepted (printer O(n), only prints documents that already passed Plan 04's caps).

## Self-Check: PASSED

Files present: `print.rs`, `fmt_golden.rs`, `fmt_props.rs`, `support/mod.rs`, `support/noisy.rs`, four `fmt_golden__*.snap`. Commits `17aa47e93`, `ec5c51026`, `c71ef28e8` are ancestors of HEAD.
