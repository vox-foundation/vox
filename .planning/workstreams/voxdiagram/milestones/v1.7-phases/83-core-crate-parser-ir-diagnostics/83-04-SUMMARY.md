---
phase: 83-core-crate-parser-ir-diagnostics
plan: 04
subsystem: diagram-core
tags: [rust, parser, error-recovery, diagnostics, limits, adversarial, insta]

requires:
  - phase: 83-02
    provides: "Full graph lexer/parser/resolver, ordered-entry AST, scoped resolution"
provides:
  - "Panic-mode recovery with declare-then-diagnose for node and container headers; one mistake gives one diagnostic"
  - "Single Collector funnel for lexer, parser and resolver diagnostics: sorted by (span.start, code), exact duplicates dropped, 99 real + one limit-diagnostics Warning"
  - "Parser fuel (4 * tokens + 1000) and consume-or-end scans; iterative depth skip so recursion never exceeds MAX_DEPTH"
  - "18 new stable codes (27 total) with 20 broken fixtures using // expect: headers, a six-mistakes snapshot and a never-renamed code golden"
  - "Per-kind caps (500 nodes, 1,000 edges, depth 4, 200-char labels) rejecting N+1 span-precisely, plus an adversarial corpus under a 10 s hang detector"
affects: [83-05, 83-06, 83-07, 84, 85]

key-files:
  created:
    - crates/vox-diagram/src/parser/recover.rs
    - crates/vox-diagram/tests/diagnostics_all_errors.rs
    - crates/vox-diagram/tests/limits_adversarial.rs
    - crates/vox-diagram/tests/fixtures/broken/ (20 fixtures)
    - crates/vox-diagram/tests/fixtures/codes.golden.txt
    - crates/vox-diagram/tests/snapshots/diagnostics_all_errors__six_independent_mistakes.snap
  modified:
    - crates/vox-diagram/src/diag.rs
    - crates/vox-diagram/src/check.rs
    - crates/vox-diagram/src/codes.rs
    - crates/vox-diagram/src/lexer.rs
    - crates/vox-diagram/src/resolve.rs
    - crates/vox-diagram/src/parser/mod.rs
    - crates/vox-diagram/src/parser/stmt.rs
    - crates/vox-diagram/src/parser/tests.rs

key-decisions:
  - "Collector tracks has_error separately from storage, so a flood of warnings can never push an Error out and make a report look valid"
  - "Items past the node/edge cap stay in the AST (the IR is withheld by the Error anyway) so edges to them do not cascade into undeclared-node; the one error is emitted at the end with the total"
  - "skip_to_stmt_end is brace-aware: braces opened in the skipped region (a multi-line attr block after a broken node) are skipped with it, except it stops at a line-leading node/container"
  - "Unquoted and single-quoted labels are recovered as the label text, so a mistaken label is exactly one expected-quoted-label"
  - "After a non-graph header word or non-word first token the rest of the file is dropped silently, so a wrong diagram kind is one unknown-diagram-kind"

patterns-established:
  - "Fixture harness: first line `// expect: code, code`, sorted multiset compare, spans checked in range and on char boundaries"
  - "Echoed user text goes through diag::clip (40 chars plus an ellipsis)"

requirements-completed: [DGM-06]  # DGM-03 stays Pending: Plan 07 adds the Mermaid-ism codes that DGM-03's "every error" still lacks

duration: about 1h 15m
completed: 2026-10-16
status: complete

actuals:
  tokens: 20000
  tasks: 3
  commits: 3
plan_head_before: a6d59bc945a54468dbf8ddde42c70e4aaf0ca45a
plan_head_after: e7305a26cb987f9b88d8745029954d1aac1d688d
---

# Phase 83 Plan 04: Recovery, diagnostic catalogue and input caps Summary

**One `check` now reports each independent mistake exactly once (stable `diagram/*` code, exact span, sorted and capped), and hostile or oversized input fails span-precisely in bounded time with no panic and no stack overflow.**

## Accomplishments

- Tracer: `node a "A"` (missing colon) plus `b -> c` gives exactly `unexpected-token` and `undeclared-node`; the broken `a` is still declared, so no false undeclared-node for it. Container headers get the same treatment (garbled label with a `{` on the line still parses its body; no `{` on the line declares it empty).
- Collector (`diag.rs`): sort by `(span.start, code, span.end)`, exact-duplicate removal, 99 real diagnostics plus a trailing `limit-diagnostics` Warning; parser halts when it is full or when fuel runs out (sentinel added once).
- Catalogue: `expected-quoted-label`, `unclosed-brace` (one per open `{`, span = that `{`), `trailing-content` (one error over everything after the closing `}`), `unknown-diagram-kind`, `reserved-word`, `invalid-direction` (help names the closest word via `strsim::levenshtein`), `duplicate-direction`, `direction-in-container`, `invalid-shape`, `unknown-attribute`, `duplicate-id` (related = first declaration), `shadowed-id` (Warning, report stays valid), `empty-label`.
- Caps: `limit-nodes` / `limit-edges` (one each, message carries the total), `limit-label` (decoded chars, multibyte-safe), `limit-depth` (container past 4 skips its body with a brace counter). The lexer also stops building diagnostics at the report cap.
- 20 broken fixtures, a snapshot-locked six-mistakes diagnostics JSON, a committed 27-line sorted code golden checked as a subset of `ALL_CODES`; `ALL_CODES.len()` assertion bumped 9 -> 10 -> 23 -> 27.
- 91 tests pass under `INSTA_UPDATE=no`; `cargo clippy -p vox-diagram --all-targets -- -D warnings` and `cargo fmt -p vox-diagram --check` clean; no `*.snap.new`; `lib.rs`, `Cargo.toml`, `Cargo.lock` untouched. All files under 400 non-blank lines.

## Task Commits

1. **Task 1 (tracer): recovery + collector** - `09ee01dd5`
2. **Task 2: full diagnostic catalogue + fixtures** - `1ff2decf3`
3. **Task 3: caps, depth skip, adversarial corpus** - `e7305a26c`

## TDD / RED evidence

- Task 1: tests and fixture written first against a stub `Collector` (push stored nothing, finish returned nothing); the run completed with `8 tests run: 2 passed, 6 failed` (four `diag::tests` collector tests, the fixture harness and the declare-then-diagnose test). GREEN: 69 tests passed.
- Task 2: fixtures, six-mistakes snapshot test, registry/golden tests and echo-truncation test written first; run completed with `10 tests run: 2 passed, 8 failed` (`--retries 0`). GREEN: 77 passed after the snapshot was created with `INSTA_UPDATE=always` from reviewed output.
- Task 3: `limits_adversarial.rs` written first; run completed with `14 tests run: 9 passed, 5 failed` (nodes, edges, depth, label, and `four_thousand_nested_headers...` aborted with `fatal runtime error: stack overflow`). The 9 that passed are guard tests over code that already terminated (source cap, container endpoint, adversarial inputs). GREEN: 91 passed.

## Mutation proofs

- **(a) fuel decrement** (`parser/recover.rs`): `self.fuel -= 1;` present (count 1) -> replaced by a marker comment (decrement count 0, marker 1) -> `parser::tests::recovery::exhausted_fuel_halts_with_exactly_one_limit_diagnostics` FAILED -> restored from a pre-mutation copy (decrement count 1, marker 0) -> test PASSED. Honest note: with the decrement removed the `limits_adversarial` tests still passed (14 passed, 1 failed overall in that run), because the parser is linear by construction (consume-or-end) and fuel is a safety net. The guard is therefore proven by the fuel unit test, which calls the internal `run(.., fuel = 3)`, not by the adversarial corpus.
- **(b) depth guard** (`parser/stmt.rs`): `Some(open) if self.depth >= MAX_DEPTH =>` present (count 1) -> changed to `if false && self.depth >= MAX_DEPTH` (original count 0, mutant count 1) -> `depth_4_accepted_5_rejected_on_the_fifth_header` FAILED and `four_thousand_nested_headers_give_one_depth_error_and_no_overflow` died with SIGABRT `thread has overflowed its stack` (12 passed, 2 failed) -> restored from a pre-mutation copy (guard count 1, mutant 0) -> full suite 91 passed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] attr-block recovery consumed the newline it stopped at**
- **Found during:** Task 2 (fixture 20, unclosed attr block)
- **Issue:** stopping at a line-leading `node` after skipping the newline left the statement end unconsumed, producing a spurious `unexpected-token`.
- **Fix:** check `next_line_starts_decl()` before consuming the newline, so the newline stays for the statement end.
- **Files modified:** `crates/vox-diagram/src/parser/stmt.rs`
- **Commit:** `1ff2decf3`

**2. [Rule 1 - Bug] My own test expectations**
- A Task 1 test expected `unexpected-token` for a garbled container label that Task 2 correctly reports as `expected-quoted-label`, and the first version of that test referenced `x` outside its container (`c.x` is the valid reference); an unclosed-brace column was off by one. All were test mistakes; behavior unchanged.

### Plan-versus-reality notes

- **Source cap vs adversarial sizes:** the plan asked for 20,000 repetitions of `node ` and 5,000 nested headers, but both exceed the 64 KiB source cap, so only `limit-source` would be exercised. The tests use the most that fits (13,000 `node `, 21,000 `-> `, 4,000 nested `container a:"a"{`) so the parser really runs.
- **Items past a cap stay in the AST** instead of being dropped (plan: "stop adding items of that kind"): dropping them makes edges to node #501 cascade into `undeclared-node`; the Error already withholds the IR, so nothing downstream sees them.
- **Order of codes in tasks:** `LIMIT_DIAGNOSTICS` landed in Task 1; Task 2 added 13 codes, Task 3 the four `limit-*` codes.
- The `parser/tests.rs` recovery tests (and `exhausted_fuel...`) were written alongside the Task 1 implementation rather than before it; the Task 1 RED above came from the collector and fixture tests.

## Auth Gates

None.

## Known Stubs

None. Deliberate limits owned by later plans (scheduled work, so not added to `.planning/WINDOWS.md`): no fixes on any diagnostic (`fixes` empty, Plan 06); Mermaid-isms stay `unexpected-token` (Plan 07). Collector `ponytail:` note: when more than 99 diagnostics arrive, the first 99 arrivals (lexer, then parser, then resolver) are kept and then sorted, not the 99 earliest by position.

## Threat Flags

None. T-83-11..15 are mitigated as planned: echoed text cut at 40 chars (`echoed_user_text_is_cut_at_40_chars`), recursion <= 4 (mutation (b)), fuel (mutation (a)), 99+1 diagnostic cap, per-kind caps with the IR withheld on Error.

## Self-Check: PASSED

- Files present: `parser/recover.rs`, `tests/diagnostics_all_errors.rs`, `tests/limits_adversarial.rs`, `tests/fixtures/codes.golden.txt`, the six-independent snapshot, 20 broken fixtures.
- Commits `09ee01dd5`, `1ff2decf3`, `e7305a26c` are ancestors of HEAD; `git rev-list --count a6d59bc94..HEAD` was 3 before this docs commit.
