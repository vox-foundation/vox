---
phase: 83-core-crate-parser-ir-diagnostics
fixed_at: 2026-10-16T00:00:00Z
review_path: .planning/workstreams/voxdiagram/phases/83-core-crate-parser-ir-diagnostics/83-REVIEW.md
iteration: 2
findings_in_scope: 12
fixed: 12
skipped: 0
status: all_fixed
---

# Phase 83: Code Review Fix Report

**Fixed at:** 2026-10-16
**Source review:** .planning/workstreams/voxdiagram/phases/83-core-crate-parser-ir-diagnostics/83-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 12 (CR-01..03, WR-01..09)
- Fixed: 12
- Skipped: 0
- Not in scope (`fix_scope: critical_warning`): IN-01, IN-02, IN-03

Verification ran in the main checkout (`workflow.use_worktrees=false`, no worktree created), branch `milestone/v1.7-voxdiagram`, scoped to `vox-diagram`: `cargo nextest run -p vox-diagram` (INSTA_UPDATE=no, 182 passed), `cargo clippy -p vox-diagram --all-targets -- -D warnings` (clean), `cargo fmt -p vox-diagram`. Every finding got a regression test written first and seen to fail (CR-01..03, WR-01..03, WR-05, WR-06, WR-09 against the unfixed code; WR-04 and WR-07 by reverting the fix and re-running). No `proptest-regressions` files were committed. `ALL_CODES.len()` is unchanged (29) and no `lib.rs` pub item changed, so `crate-api-sync` was not re-run (the installed `vox` binary is also stale against HEAD and refuses to run).

## Fixed Issues

### CR-01: Two fixes inserting at the same offset apply in the wrong order

**Files modified:** `crates/vox-diagram/src/fixes.rs`, `crates/vox-diagram/tests/fix_safety.rs`
**Commit:** 5be1973b5
**Status:** fixed: requires human verification (ordering logic)
**Applied fix:** `apply_fixes` sorts by `(start, end, replacement == "\n}")`, so EOF scope closers go after any other insert at the same offset (stable sort keeps the rest in diagnostic order). Regression: `repair("graph {\n    node a: \"abc", 5)` is now exactly `graph {\n    node a: "abc"\n}` and valid; the trailing-backslash variant is valid too.

### CR-02: `quote()` emits a raw newline

**Files modified:** `crates/vox-diagram/src/fixgen.rs`, `crates/vox-diagram/tests/fix_safety.rs`
**Commit:** 3d2d52991
**Applied fix:** `quote` escapes `\n` as `\\n` (matching `print::label` and `mermaid_label`, which is unaffected). Unit test added next to the existing `quote` test; integration regression repairs `a -> b -> c: "x\ny"` to a valid document that keeps the label.

### CR-03: Rule R declares the missing node in the wrong scope

**Files modified:** `crates/vox-diagram/src/fixgen.rs`, `crates/vox-diagram/tests/fix_safety.rs`
**Commit:** 8ff0080a4
**Status:** fixed: requires human verification (scope-placement logic)
**Applied fix:** `insert_decl` uses the end-of-`{`-line form only when the rest of that line is blank or a `//` comment; otherwise it falls back to the insert-right-after-`{` form. Regression: the reviewer's `graph { container c: "C" {` repro now repairs to a valid document with `a` and `ghost` declared once, at root.

### WR-01: A lone CR hides content from `check`

**Files modified:** `crates/vox-diagram/src/lexer.rs`, `crates/vox-diagram/src/span.rs`, `crates/vox-diagram/tests/fix_safety.rs`
**Commit:** 491ef7e4f
**Status:** fixed: requires human verification (chose "lone CR is a line break")
**Applied fix:** One rule: CR, LF and CRLF all end a line. A lone `\r` lexes as `Newline`, `comment()` stops at any `\r`, and `LineIndex` counts a lone CR as a line start. Regression asserts the CR-only form of a document yields the same codes and line/col as the LF form, plus a mixed CR/CRLF case.

### WR-02: Messages echo control characters, bidi marks and newlines

**Files modified:** `crates/vox-diagram/src/diag.rs`, `crates/vox-diagram/src/lexer.rs`, `crates/vox-diagram/tests/fix_safety.rs`
**Commit:** a627a254b
**Applied fix:** `diag::clip` (used by all parser/resolver echo sites) renders forbidden characters, CR, LF and tab as `\u{..}` escapes. The lexer's invalid-escape message now goes through it as well. Regression runs hostile text through four echo sites and asserts no message or help line contains a control or bidi character.

### WR-03: Forbidden-character set misses C1 controls and bidi marks

**Files modified:** `crates/vox-diagram/src/lexer.rs`, `crates/vox-diagram/tests/support/mod.rs`, `crates/vox-diagram/tests/fix_safety.rs`
**Commit:** 0706ac192
**Applied fix:** `is_forbidden` now also covers U+0080-009F, U+061C, U+200E/200F and U+2028/2029; the `support::allowed` generator matches. Decision: zero-width characters (U+200B-200D, U+FEFF) stay legal, because ZWJ/ZWNJ are needed for emoji and Persian; documented on the function. Regression checks each new character yields exactly one `forbidden-char` and repairs to valid.

### WR-04: Empty-edge-label fix deletes the target's bracket text

**Files modified:** `crates/vox-diagram/src/parser/edges.rs`, `crates/vox-diagram/src/parser/stmt.rs`, `crates/vox-diagram/tests/fix_safety.rs`
**Commit:** 0acc1147c
**Status:** fixed: requires human verification
**Applied fix:** `from` is the end of the last token before the colon (captured before bumping it), not `Ref.span.end`. `a -> B((x)) : ""` now repairs to `a -> B((x))` and keeps the bracket text (the `((..))` shape has no exact rewrite, so that diagnostic correctly stays fix-less).

### WR-05: `A-.->B` and `A-.-B` mis-lex as identifier `A-`

**Files modified:** `crates/vox-diagram/src/lexer.rs`, `crates/vox-diagram/tests/fix_safety.rs`
**Commit:** 345d19835
**Applied fix:** `word()` does not join a `-` that starts `.-`. Regression: `A-.->B`, `A-.-B` and `A -.-> B` each produce exactly one `mermaid-syntax` diagnostic.

### WR-06: Quadratic work on `a>a>a>...`

**Files modified:** `crates/vox-diagram/src/lexer.rs`, `crates/vox-diagram/src/lexer/mermaid.rs`, `crates/vox-diagram/tests/limits_adversarial.rs`
**Commit:** 48a4fbdae
**Applied fix:** an unclosed `>` opener returns before building the unclosed candidate (no `to_string`) and caches `gt_fail_until` (end of line) so later `>` on the same line skip the scan. `"a>".repeat(30_000)` is added to `adversarial_mermaid_tokens`, plus a coarse scaling test (`unclosed_gt_openers_scale_linearly`: 4x input must cost under 10x; it measured about 15x before the fix). That test is a growth ratio, not an absolute time, but it is timing based, so treat a failure on a heavily loaded machine as a rerun first.

### WR-07: Bare `graph` line then `{` on the next line is rewritten into a worse document

**Files modified:** `crates/vox-diagram/src/parser/mermaid.rs`, `crates/vox-diagram/tests/fix_safety.rs`
**Commit:** 51c43dc86
**Status:** fixed: requires human verification
**Applied fix:** `mermaid_header` returns `None` when, after skipping newlines past the header, the next token is `{`, so the normal `graph` path reports the missing brace. Regression asserts no `mermaid-syntax` diagnostic and that repair does not increase the error count.

### WR-08: Repair/hostile tests do not assert that fixes are safe

**Files modified:** `crates/vox-diagram/tests/hostile_props.rs`, `crates/vox-diagram/tests/repair_props.rs`
**Commit:** c143882a8
**Applied fix:** `fixes_are_safe` now asserts the repaired error count is `<=` the pre-repair count (skipped when the 99-diagnostic cap hides the true count); the stale doc comment is corrected. `repair_props` gains `truncate_and_repair_never_worsens` (cut a valid noisy document at a random char boundary, same no-worse assertion).
**Mutation result:** reverting `fixes.rs` and `fixgen.rs` to the reviewed sources and running `hostile_props` at 20,000 cases fails `hostile_apply_fixes_and_repair_never_panic` (minimal input `flowchart LR%%{\n{node]{"`, a string-plus-brace case). The truncation property did not fail against the old code: the CR-01 repro goes from 2 errors to 1 (the new error replaces two old ones), so an error-count comparison cannot see it. CR-01..03 are therefore guarded by their exact-repro tests, not by the property. A stronger "no new error code" property was tried and dropped: it flags legitimate cases where a fix exposes a previously masked error (for example `not-a-container` appearing once an unclosed scope is closed). Chained-edge `\n` fixtures were not added to `repair_props`; the CR-02 regression covers that input.

### WR-09: Fuel exhaustion halts the parser without recording an error

**Files modified:** `crates/vox-diagram/src/diag.rs`
**Commit:** 34d48a404
**Status:** fixed: requires human verification (defence in depth, not reproducible with valid input)
**Applied fix:** `Collector::trip()` sets `has_error`, so a truncated AST can never produce a valid report, and the sentinel text says "input too complex to check in full" instead of the diagnostics-limit wording. The diagnostic code is unchanged (`limit-diagnostics`); a new code was not added, which keeps `ALL_CODES`, the golden file and the docs stable. Unit test `halted_parser_invalidates_the_report` fails when the `has_error` line is removed.

## Skipped Issues

None. IN-01, IN-02 and IN-03 are Info findings, outside `fix_scope: critical_warning`.

---

_Fixed: 2026-10-16_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

---

# Iteration 2

**Fixed at:** 2026-10-16
**Source review:** 83-REVIEW.md (iteration 2)
**Summary:** findings in scope 1, fixed 1, skipped 0. Verified in the main checkout (`workflow.use_worktrees=false`, no worktree): `INSTA_UPDATE=no cargo nextest run -p vox-diagram` 184/184 pass, `cargo clippy -p vox-diagram --all-targets -- -D warnings` clean, `cargo fmt -p vox-diagram` applied.

## Fixed Issues

### CR-01: EOF closer ordering fix does not cover Mermaid-header documents

**Files modified:** `crates/vox-diagram/src/fixes.rs`, `crates/vox-diagram/tests/fix_safety.rs`, `crates/vox-diagram/tests/repair_props.rs`
**Commit:** 6e862a0ec
**Status:** fixed: requires human verification (ordering logic; guarded by tests below)
**Applied fix:** the `apply_fixes` tie-break now classifies EOF scope closers by `r.trim() == "}"` instead of the literal `"\n}"`, so the Mermaid header closers (`}\n`, `\n}\n`) also sort after a closing quote inserted at the same offset.
**Tests:** `cr01_mermaid_header_truncated_mid_string_repairs_quote_before_brace` covers both reviewer repros (watched failing first, with a stray `""` and `trailing-content`). `repair_props::truncate_and_repair_mermaid_never_worsens` truncates four Mermaid-header documents at every offset and asserts the error count does not rise and that no `trailing-content` diagnostic appears (no fixture contains `}`, so one is always a repair artifact). Mutation: reverting the tie-break to `r == "\n}"` makes the new property fail; restored afterwards. As in iteration 1, the plain error-count comparison alone cannot see this bug (2 errors become 1), hence the `trailing-content` assertion.

---

_Fixed: 2026-10-16_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_
