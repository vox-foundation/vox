---
phase: 83-core-crate-parser-ir-diagnostics
plan: 02
subsystem: diagram-core
tags: [rust, lexer, parser, resolver, ir, trivia, insta, determinism]

requires:
  - phase: 83-01
    provides: "check()/parse() tracer, Span/Location/Diagnostic/IR types, codes.rs, ALL_CODES"
provides:
  - "Complete graph surface of DGM-01 for valid input: direction, containers (nested), node attr blocks, edge labels, dotted refs, six shapes, self-loops, // comments"
  - "Scoped two-pass resolver (innermost-first, then outward; dotted paths through children only) emitting full dotted ids in DFS declaration order"
  - "Full lexer: kebab ids, whitelisted escapes, CRLF/BOM, // comments, forbidden-char/invalid-escape/unterminated-string/invalid-identifier diagnostics"
  - "Ordered-entry AST with every trivia field the Plan 05 printer needs (Comment, Entry::Comment, Item.trailing, Document/ContainerDecl header/close trailing)"
  - "Golden IR snapshots (canonical example, all-constructs) and a determinism guard over src/"
affects: [83-03, 83-04, 83-05, 83-06, 83-07, 84, 85]

key-files:
  created:
    - crates/vox-diagram/src/parser/stmt.rs
    - crates/vox-diagram/src/parser/tests.rs
    - crates/vox-diagram/tests/determinism_guard.rs
    - crates/vox-diagram/tests/fixtures/valid/canonical.vdg
    - crates/vox-diagram/tests/fixtures/valid/all_constructs.vdg
    - crates/vox-diagram/tests/snapshots/graph_constructs__canonical_example_ir.snap
    - crates/vox-diagram/tests/snapshots/graph_constructs__all_constructs_ir.snap
  modified:
    - crates/vox-diagram/src/ast.rs
    - crates/vox-diagram/src/codes.rs
    - crates/vox-diagram/src/lexer.rs
    - crates/vox-diagram/src/parser/mod.rs
    - crates/vox-diagram/src/resolve.rs
    - crates/vox-diagram/src/check.rs
    - crates/vox-diagram/tests/graph_constructs.rs

key-decisions:
  - "Resolver scopes: Vec<Scope> with BTreeMap names, one namespace per scope; pass B replays pass A's pre-order scope numbering instead of storing a scope id per AST node"
  - "Lexer reports its own problems and still emits a best-effort token (Str{terminated:false}, BadIdent), and the parser accepts those tokens as a label/ident, so one mistake yields exactly one diagnostic"
  - "`direction:` is a statement only directly inside `graph { }`; inside a container it parses as an edge and gives unexpected-token (loud, not silently ignored)"
  - "container-endpoint help names the container's first declared child (tracked in Scope.first_child, since the BTreeMap is alphabetical)"

patterns-established:
  - "Trivia attach rules: closing-line comment of a container goes to ContainerDecl.close_trailing (Item.trailing stays None); first entry of a block / first comment of leading|trailing is never blank_before"
  - "Parser unit tests live in parser/tests.rs (declared #[cfg(test)] mod tests from parser/mod.rs) to stay under the 400-line file budget"

requirements-completed: []  # DGM-01 and DGM-06 stay Pending: invalid-input diagnostics, recovery caps (Plan 04), fmt (05), fixes (06), mermaid (07) are still outstanding

duration: about 1h
completed: 2026-10-16
status: complete

actuals:
  tokens: 14200
  tasks: 3
  commits: 3
plan_head_before: 324b5252965a9ece1a51c3e400aad0f7bc3339c0
plan_head_after: 167836957ba5d7b73174bda1ed58a005a0e705bd
---

# Phase 83 Plan 02: Complete graph surface (lexer, trivia, scoped resolution) Summary

**The CONTEXT canonical example now checks valid end to end into a snapshot-locked IR, and the lexer, parser and resolver cover every DGM-01 construct, with the comment/blank-line trivia the formatter will consume.**

## Accomplishments

- Tracer first: the canonical example (direction, container `backend`, `{ shape: cylinder }` attr, labelled edges, dotted `backend.gateway`) flows source -> AST -> scoped resolution -> `GraphIr`; nodes come out in DFS order `[client, backend.gateway, backend.engine, backend.db]`, edges in source order, and the IR JSON is byte-identical over repeated runs (`ir_json_is_stable`, 3 iterations).
- Resolution: innermost-first then outward, forward references valid, `a -> o.leaf.x` gives `diagram/not-a-container` on `x`, `client -> backend` gives `diagram/container-endpoint` (no fixes, help names `backend.gateway`).
- Lexer: kebab ids (`api-gw`, `a->b`, `a-->b` leaves the dash run), `\"` `\\` `\n` escapes, `//` comments, CRLF as one newline, BOM skipped, and four non-cascading diagnostics (`unterminated-string`, `invalid-escape`, `forbidden-char` per char with exact span, `invalid-identifier`).
- Trivia: comments land in leading / header / own-line / trailing / container header+close / document close / after-`}` positions; `blank_before` follows the A3 rule; multi-line attr blocks; explicit `shape: rect` is stored as the default.
- `all_constructs.vdg` (right direction, six shapes, three escapes, kebab id, 3-level nesting, inner+outer dotted refs, innermost-first, self-loop, every comment position) is valid and snapshot-locked; its CRLF variant yields the same IR once `span` keys are stripped.
- 40 tests pass under `INSTA_UPDATE=no`; `cargo clippy -p vox-diagram --all-targets -- -D warnings` and `cargo fmt -p vox-diagram --check` are clean; no `*.snap.new`; `lib.rs`, `Cargo.toml`, `Cargo.lock` untouched.

## Task Commits

1. **Task 1 (tracer): canonical example end-to-end** - `e013d7fbe`
2. **Task 2: complete lexer** - `82b42262f`
3. **Task 3: trivia, remaining constructs, all-constructs golden, determinism guard** - `167836957`

## TDD / RED evidence

- Task 1: tests written first; run completed with `26 tests run: 18 passed, 8 failed` (canonical_example_ir, ir_json_is_stable and six resolve::tests failed). GREEN: `26 passed`.
- Task 2: tests written against the new `lex(src, &LineIndex)` signature with a stub body (old logic, empty diagnostics); run completed with `33 tests run: 26 passed, 7 failed` (all seven new lexer behavior tests). GREEN: `33 passed`.
- Task 3: parser tests and integration tests written first; run completed with `40 tests run: 35 passed, 5 failed` (attr block, comment positions, blank lines, all_constructs_ir, crlf_input_yields_same_ir). `determinism_guard` passed before any mutation because it is a guard over existing code; its value is shown by mutation (e). GREEN: `40 passed`.

## Mutation proofs

- **(c) forbidden-char guard** (`crates/vox-diagram/src/lexer.rs`): guard arm `c if is_forbidden(c) => {` present (count 1) -> replaced with `c if false && is_forbidden(c) => {` (original count 0, mutant count 1) -> `lexer::tests::forbidden_char_is_reported_per_char_with_exact_span` FAILED -> restored from a pre-mutation copy (count 1) -> test PASSED.
- **(e) determinism guard**: no hash-collection name in `resolve.rs` (count 0) -> added a `use std::collections::<hash map>` import (count 1; first attempt put it before the `//!` header and failed to compile, so it was redone after the header) -> `determinism_guard_no_hash_collections` FAILED naming `resolve.rs` -> restored (count 0) -> test PASSED.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Wrong column arithmetic in my own first resolve tests**
- **Found during:** Task 1 (after RED, before GREEN)
- **Issue:** expected columns (6,13,14) / (6,13) for not-a-container and container-endpoint were miscounted; correct spans are (6,14,15) and (6,12).
- **Fix:** corrected the test expectations; behavior unchanged.
- **Commit:** `e013d7fbe`

**2. [Process] AST trivia types landed in Task 1, not Task 3**
- The plan lists `ast.rs` under Task 3, but Task 1's `Document` needed the new shape and `Entry::Comment`/`Item.trailing` would otherwise churn twice. All AST types were added in Task 1 (parser attached nothing until Task 3), so Task 3 changed no `ast.rs`. Behavior and the interface block are exactly as specified.

**3. [Process] Task 2 RED used a stub signature, Task 1 RED used a first compile failure**
- My first Task 1 RED run failed to compile (a Python string-escaping mistake in the test module, since fixed); the valid RED run quoted above came after. Task 2's RED used a stub `lex` so the failures were behavioral rather than compile errors.

### Behavior notes (within plan intent)

- A word starting with a digit or any non-ASCII letter is lexed as one `BadIdent`; a word with a non-ASCII letter in the middle (`café`) is also one invalid-identifier word (the plan only named the leading cases).
- The parser accepts `BadIdent` as an identifier and an unterminated `Str` as a label so the lexer's diagnostic is the only one for that mistake; Plan 04 owns fuller recovery.
- Attr-block resync after a bad attribute skips to the closing `}` (across lines) so it cannot end the enclosing block.

## Auth Gates

None.

## Known Stubs

None. Deliberate limits owned by later plans (all scheduled work, so not added to `.planning/WINDOWS.md`): first-declaration-wins on duplicate ids with no diagnostic (`ponytail:` comment in `resolve.rs`, Plan 04); no fuel/depth caps (Plan 04, threat T-83-07 accepted until then); no fixes on any diagnostic (Plan 06); no Mermaid tokens (Plan 07).

## Threat Flags

None new. T-83-04/05 (bidi and control characters, escape whitelist) are implemented and covered by mutation (c); T-83-06 (deterministic IR, Vec/String only) by the determinism guard, `ir_json_is_stable` and mutation (e); T-83-SC held (no dependency change).

## Notes for later plans

- `lex` now returns `(Vec<Token>, Vec<Diagnostic>)` and `TokenKind::Str` is `Str { value, terminated }`; `Comment` and `BadIdent` are new kinds (Plan 07 adds Mermaid tokens beside them).
- The ALL_CODES length assertion in `codes::tests` is now 9; each plan adding codes must bump it.
- `lexer.rs` is at 354 non-blank lines; Plan 07's additions should move the tests to a sibling `lexer_tests.rs`.
- Pre-commit hooks printed `NIGHTLY FAILING: Docs Link Checker (#578)`, which is unrelated to this plan.

## Self-Check: PASSED

- FOUND: parser/stmt.rs, parser/tests.rs, tests/determinism_guard.rs, both fixtures, both snapshots under crates/vox-diagram
- FOUND commits e013d7fbe, 82b42262f, 167836957 (ancestors of HEAD); `commits: 3` measured from the ledger
- No `*.snap.new`; `git log` for lib.rs/Cargo.toml shows only the 83-01 commit
