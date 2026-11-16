---
phase: 83-core-crate-parser-ir-diagnostics
plan: 06
subsystem: diagram-core
tags: [rust, fixes, repair, diagnostics, proptest, insta, hostile-input]

requires:
  - phase: 83-04
    provides: "Collector funnel, recovery, full code catalogue"
  - phase: 83-05
    provides: "Canonical fmt (hostile property reuses it)"
provides:
  - "apply_fixes: atomic per-diagnostic fix groups, vetted for range, char boundary and overlap, never panics"
  - "repair: bounded check -> apply_fixes loop returning text, rounds and final report"
  - "fixes[] on 8 native codes: unterminated-string, invalid-escape, forbidden-char, expected-quoted-label, empty-label, invalid-shape, unclosed-brace, undeclared-node (Rule R declare-fix)"
  - "13 seeded repairable fixtures, each repaired to zero errors with a reviewed snapshot"
  - "S3 hostile-input properties (4) over check, apply_fixes, repair and fmt"
affects: [83-07, 84, 88]

key-files:
  created:
    - crates/vox-diagram/src/fixes.rs
    - crates/vox-diagram/src/fixgen.rs
    - crates/vox-diagram/src/lexer/tests.rs
    - crates/vox-diagram/src/resolve/tests.rs
    - crates/vox-diagram/tests/hostile_props.rs
    - crates/vox-diagram/tests/fixtures/repairable/ (13 fixtures)
    - crates/vox-diagram/tests/snapshots/fixes_repair__*.snap (13)
  modified:
    - crates/vox-diagram/src/lib.rs
    - crates/vox-diagram/src/ast.rs
    - crates/vox-diagram/src/lexer.rs
    - crates/vox-diagram/src/resolve.rs
    - crates/vox-diagram/src/parser/mod.rs
    - crates/vox-diagram/src/parser/recover.rs
    - crates/vox-diagram/src/parser/stmt.rs
    - crates/vox-diagram/tests/fixes_repair.rs
    - crates/vox-diagram/tests/graph_constructs.rs
    - crates/vox-diagram/tests/snapshots/diagnostics_all_errors__six_independent_mistakes.snap

key-decisions:
  - "A group is rejected whole if any edit is out of range, off a char boundary, or overlaps an earlier-accepted edit or a mate; equal-offset inserts are legal and keep diagnostic order (never deduplicated)"
  - "Overlap scan is O(k^2) in accepted edits (check caps a report at 100 diagnostics); marked with a ponytail comment naming the interval-map upgrade"
  - "unclosed-brace carries its `\\n}` fix only when the scan is at EOF; an attribute block cut short by the next declaration has no single safe edit, so no fix"
  - "undeclared-node gets a fix only for a single-segment ref (declared in the referencing scope) or the LAST segment of a dotted ref (declared in the resolved container); a scope with no `{` or no `}` yields no fix this round, so an unclosed scope is closed first and the declaration comes next round"
  - "Block records its brace offsets as an unserialised `Option<(u32, u32)>`; two bare offsets (not Spans) keep `Entry` under clippy's large-variant threshold, and `serde(skip)` keeps the AST JSON (and P3) unchanged"
  - "Pure fix builders live in `fixgen.rs` (`quote`, `declare_node`); `quote` drops `\\r` and forbidden chars so a recovered label can never reintroduce a lexer error"

patterns-established:
  - "Fix tests assert (original text at the fix span, replacement) pairs through the public API"
  - "Repairable-set harness: every fixture must be invalid, repair to valid in <= 5 rounds, be stable under a second repair, and match a snapshot"

requirements-completed: []  # DGM-04 stays Pending: Plan 07 adds the Mermaid-ism fixes that close it

duration: about 1h 20m
completed: 2026-10-16
status: complete

actuals:
  tokens: 13000
  tasks: 3
  commits: 3
plan_head_before: c0efdab97838ca15775a28766b6a0cc7e464e086
plan_head_after: 57fa90407eabc0c6e75b497a1b98f9b353c4d268
---

# Phase 83 Plan 06: Machine-applicable fixes, repair loop and hostile-input safety Summary

**Every mechanically fixable native diagnostic now carries a `{span, replacement}` fix, `repair` turns all 13 seeded broken files into documents that pass `check`, and a hostile-input property proves no input can panic the pipeline or yield an unsafe span or fix.**

## Accomplishments

- Tracer: a graph with two unclosed scopes produces two identical `\n}` EOF inserts; both apply (no dedup), the result is valid, and `repair` converges in one round. `repair` on valid input is 0 rounds and unchanged; `max_rounds` 0 returns the input.
- `apply_fixes` unit tests cover: out-of-range group skipped whole, inverted span, multibyte-splitting edit, overlap (later group loses, insert strictly inside a replaced range), intra-group overlap, group atomicity, offset ordering, same-offset insert order, fix-less diagnostics ignored.
- Fix shapes (each asserted as exact `(original text, replacement)`): unterminated-string closes after the trimmed text; invalid-escape doubles just the backslash; forbidden-char deletes the char; expected-quoted-label wraps with `\` and `"` escaped (single quotes stripped); empty-label becomes the owner id, an empty edge label is deleted back to the target; invalid-shape fixes only an exact case-fold match (`hexagon` has none); undeclared-node follows Rule R (end of the `{` line, indent + 4, single-line scope inserts after `{` with a trailing newline, first diagnostic per (scope, id) owns it).
- 13 repairable fixtures (one per fix-bearing code, plus mixed, multi-ghost, single-line scope, unclosed-then-ghost); all reach zero errors in one round except `13` which needs two (close first, declare second), each stable under a second repair. All 13 repaired texts were reviewed by eye before committing.
- `fixes_only_on_errors` (no Warning has fixes, all fix spans in range and on boundaries, at least 10 fix-bearing diagnostics) and `non_mechanical_codes_have_no_fixes` (duplicate-id, container-endpoint, not-a-container, reserved-word, invalid-direction, unknown-attribute, trailing-content, limit-nodes/depth/label all exercised and fix-free).
- Hostile properties (`\PC{0,400}` and token soup, 128 cases each, `--retries 0`): no panic; at most 100 diagnostics sorted by `(span.start, code)`; every diagnostic and fix span in range and on a char boundary; `apply_fixes` and a bounded `repair` never panic; `fmt` idempotent wherever it formats. A temporary 4,000-case run of all four also passed (no counterexample, so no bug surfaced in lexer, parser, resolver or fixes).
- Final state: 138 tests pass under `INSTA_UPDATE=no`; `cargo clippy -p vox-diagram --all-targets -- -D warnings` and `cargo fmt -p vox-diagram --check` clean; no `*.snap.new`, no `proptest-regressions` file; all files under 400 non-blank lines (largest `parser/mod.rs` 370).

## Task Commits

1. **Task 1 (tracer): fix engine end to end** - `640e8bf10`
2. **Task 2: fixes for every fixable native code + seeded repairable set** - `640f17cba`
3. **Task 3: hostile-input properties** - `57fa90407`

## TDD / RED evidence

- Task 1: `fixes::tests` (9) and `fixes_repair_unclosed_brace_end_to_end` written against a stub (`apply_fixes` returned the input, no unclosed-brace fix); run: `10 tests run: 0 passed, 10 failed`. GREEN: 10 passed.
- Task 2: fixtures, the four `fixes_repair` tests and the lexer/resolver unit tests written first; run (`--retries 0`, `INSTA_UPDATE=no`): `25 tests run: 18 passed, 7 failed` (`fixes_repair_seeded_set`, `lexer_and_parser_fixes_have_the_documented_shape`, `fixes_only_on_errors` via its fix-count guard, `lexical_fixes_are_exact`, three resolver fix tests). Snapshots were then created with `INSTA_UPDATE=always` and each reviewed. `non_mechanical_codes_have_no_fixes` passed on first run because it guards behaviour that was already fix-free.
- Task 3: the four properties passed on first run (also at 4,000 cases), so there was no natural RED; sensitivity was proven by mutation (below). Honest note: they guard safety of what Tasks 1-2 built rather than discover new bugs.

## Mutation proofs

All with before/after presence checks and a restore verified by `diff` against a pre-mutation copy.
- **(d) declare-fix offset** (`fixgen.rs`): `let at = to_u32(at);` (count 1, mutant 0) -> `let at = to_u32(at) + 1;` (count 0, mutant 1) -> `fixes_repair_seeded_set`, `fixgen::tests::crlf_scope_inserts_before_the_carriage_return` and three `resolve::tests` fix tests FAILED (13 passed, 5 failed) -> restored (count 1, mutant 0, `diff` identical) -> `binary(fixes_repair)` 5 passed.
- **Hostile sensitivity** (`lexer.rs`): unterminated-string fix offset `start + kept` -> `start + kept + 1` (mutant count 0 -> 1) -> `hostile_check_is_total_capped_and_sorted` and `hostile_spans_and_fix_spans_are_on_char_boundaries` FAILED with minimal input `"\""` (2 passed, 2 failed) -> restored (mutant count 0, `diff` identical). The `proptest-regressions` file the mutation wrote was deleted, not committed. Note: my literal-string presence check for the original line missed (rustfmt wraps it), so "orig" read 0 throughout; the mutant counter (0 -> 1 -> 0) and the `diff` carry the check.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Files would exceed the 400-line cap**
- **Found during:** Task 2 (`resolve.rs` 366 and `lexer.rs` 359 non-blank lines before any fix code)
- **Fix:** moved the inline test modules to `src/lexer/tests.rs` and `src/resolve/tests.rs` (`#[cfg(test)] mod tests;`; test names `lexer::tests::*`, `resolve::tests::*` unchanged, the `include_str!` path adjusted) and put the pure fix builders in a new `src/fixgen.rs`.
- **Files modified:** `src/lexer.rs`, `src/resolve.rs`, `src/fixgen.rs`, `src/lib.rs`
- **Commit:** `640f17cba`

**2. [Rule 3 - Blocking] clippy `large_enum_variant` on `Entry`**
- **Found during:** Task 2 clippy run
- **Issue:** adding two `Option<Span>` brace fields to `Block` pushed `Item` to 248 bytes.
- **Fix:** store one `Option<(u32, u32)>` of brace offsets with `#[serde(skip)]` (12 bytes), derived into Spans in the resolver.
- **Files modified:** `src/ast.rs`, `src/parser/mod.rs`, `src/resolve.rs`
- **Commit:** `640f17cba`

**3. [Rule 1 - Stale expectation] two earlier tests encoded "no fixes yet"**
- `graph_constructs::tracer_reports_undeclared_node` asserted `fixes.is_empty()` (a Plan 02 placeholder); it now asserts the Rule R fix. The `diagnostics_all_errors__six_independent_mistakes.snap` snapshot gained three `fixes` arrays (`"Backend"`, `"d"`, `node missing: "missing"`), reviewed against the diff before accepting.
- **Commit:** `640f17cba`

### Plan-versus-reality notes

- `lib.rs` also gained a private `mod fixgen;` (plan listed only `mod fixes`); `Cargo.toml` and `Cargo.lock` untouched.
- The plan's `unclosed-brace` fix is attached only at EOF; the second `unclosed` emission point (an attribute block cut short by the next declaration, before EOF) deliberately has no fix.
- The seeded set has 13 fixtures (plan minimum 10), including `13-ghost-in-unclosed-graph` which proves the two-round close-then-declare path.
- Fix text containing a deleted forbidden char (fixture `03`) is invisible in the snapshot by nature.

## Auth Gates

None.

## Known Stubs

None. Deliberately outside this plan (scheduled): Mermaid-ism diagnostics and their rewrite fixes, `chained-edge` (Plan 07); `requirements DGM-04` therefore stays Pending.

## Threat Flags

None. T-83-19 (atomic groups, overlap and boundary vetting, mutation proof d, S3 property), T-83-20 (`repair` bounded by `max_rounds` and a no-change stop), T-83-21 (declare-fix text only for validated ASCII, non-keyword idents; `quote` escapes and strips unrepresentable chars; one owner per (scope, id)) and T-83-22 (S3 properties) are mitigated; T-83-SC accepted (no dependency change).

## Self-Check: PASSED

Files present: `fixes.rs`, `fixgen.rs`, `lexer/tests.rs`, `resolve/tests.rs`, `fixes_repair.rs`, `hostile_props.rs`, 13 repairable fixtures, 13 `fixes_repair__*.snap`. Commits `640e8bf10`, `640f17cba`, `57fa90407` are ancestors of HEAD.
