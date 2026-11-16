---
phase: 86-sequence-kind
reviewed: 2026-10-17T00:00:00Z
depth: standard
files_reviewed: 50
files_reviewed_list:
  - crates/vox-diagram/src/ast.rs
  - crates/vox-diagram/src/check.rs
  - crates/vox-diagram/src/codes.rs
  - crates/vox-diagram/src/diag.rs
  - crates/vox-diagram/src/fixgen.rs
  - crates/vox-diagram/src/ir.rs
  - crates/vox-diagram/src/layout/input.rs
  - crates/vox-diagram/src/layout/mod.rs
  - crates/vox-diagram/src/layout/seq.rs
  - crates/vox-diagram/src/layout/seq/canvas.rs
  - crates/vox-diagram/src/layout/seq/tests.rs
  - crates/vox-diagram/src/layout/tests.rs
  - crates/vox-diagram/src/layout/tests/coords.rs
  - crates/vox-diagram/src/lib.rs
  - crates/vox-diagram/src/limits.rs
  - crates/vox-diagram/src/mermaid/export.rs
  - crates/vox-diagram/src/mermaid/export_seq.rs
  - crates/vox-diagram/src/mermaid/import/mod.rs
  - crates/vox-diagram/src/mermaid/import/seq.rs
  - crates/vox-diagram/src/mermaid/mod.rs
  - crates/vox-diagram/src/parser/mermaid.rs
  - crates/vox-diagram/src/parser/mod.rs
  - crates/vox-diagram/src/parser/recover.rs
  - crates/vox-diagram/src/parser/seq.rs
  - crates/vox-diagram/src/parser/seq_mermaid.rs
  - crates/vox-diagram/src/parser/stmt.rs
  - crates/vox-diagram/src/print.rs
  - crates/vox-diagram/src/resolve.rs
  - crates/vox-diagram/src/resolve/seq.rs
  - crates/vox-diagram/src/sequence_ir.rs
  - crates/vox-diagram/src/svg/mod.rs
  - crates/vox-diagram/src/svg/seq.rs
  - crates/vox-diagram/tests/determinism_guard.rs
  - crates/vox-diagram/tests/mermaid_corpus.rs
  - crates/vox-diagram/tests/mermaid_export.rs
  - crates/vox-diagram/tests/mermaid_fixpoint.rs
  - crates/vox-diagram/tests/mermaid_import_props.rs
  - crates/vox-diagram/tests/mermaid_import_seq.rs
  - crates/vox-diagram/tests/mermaid_roundtrip.rs
  - crates/vox-diagram/tests/sequence_check.rs
  - crates/vox-diagram/tests/sequence_corpus.rs
  - crates/vox-diagram/tests/sequence_dsl.rs
  - crates/vox-diagram/tests/sequence_mermaid.rs
  - crates/vox-diagram/tests/sequence_props.rs
  - crates/vox-diagram/tests/support/mod.rs
  - crates/vox-diagram/tests/support/noisy.rs
  - crates/vox-diagram/tests/support/seq.rs
  - crates/vox-diagram/tests/support/svgcheck.rs
  - crates/vox-diagram/tests/support/svgcheck/allow.rs
  - crates/vox-diagram/tests/support/svgcheck/seq.rs
findings:
  critical: 1
  warning: 4
  info: 4
  total: 9
status: issues_found
---

# Phase 86: Code Review Report

**Reviewed:** 2026-10-17
**Depth:** standard
**Files Reviewed:** 50

## Summary

The sequence kind is carefully built. I ran `cargo test -p vox-diagram` (all green, snapshots
unchanged) and `cargo clippy -p vox-diagram --all-targets -D warnings` (clean) against the
worktree with an out-of-tree `CARGO_TARGET_DIR`, then probed with a scratch crate that depends on
the worktree crate by path (no worktree file touched; `git status` is identical to the start).

Probed and found sound:

- **Graph regressions:** none found. `Ir::Sequence` dispatch, `Parser.seq`, `count()` and the
  `Document.kind` default leave the graph path unchanged; `fmt`/`check`/`repair` outputs on the
  graph fixtures are unchanged.
- **Caps before layout:** 501 actors / 1001 events give `limit-nodes` / `limit-edges` with no IR;
  101 actors / 201 events give `RenderError::TooLarge` before any geometry; the canvas is checked
  in `i64` before narrowing to `Half`.
- **Determinism:** no `HashMap` / `HashSet` anywhere in `src/`; `BTreeMap` / `BTreeSet` throughout.
- **Importer to DSL:** 40+ crafted blocks (keyword-like ids, `reply` / `style` / `class` / `click`
  as actors, quotes, backslashes, tabs, `<br/>`, `//`, empty labels) either fail closed or print
  DSL whose re-checked IR equals the imported IR (spans aside) and whose `fmt` is a fixed point.
- **`fmt` idempotence:** comments, blank lines, `reply`, contextual-word actors (`note`, `reply`,
  `actor`, `over`) round-trip with identical AST and IR.
- **declare-actor insertion:** correct for LF, trailing comment, `;`-separated statements, Mermaid
  `participant` rewrites in the same round (touching edits are accepted), and missing braces.

Defects are concentrated in `repair`, where one Mermaid-ism rewrite changes meaning silently, and in
two layout cases where a drawn box sits over lifelines it does not belong to.

## Critical Issues

### CR-01: `repair` of a Mermaid message with `;` silently merges or corrupts statements and reports `valid`

**File:** `crates/vox-diagram/src/parser/recover.rs:167-199` (`unquoted_label`), reached from
`crates/vox-diagram/src/parser/seq.rs:85-100` (`message`) and `parser/seq.rs:37-52` (`note_stmt`)
**Issue:** `unquoted_label` collects tokens up to a newline, `}` or comment but not `Semi`. In
Mermaid `;` ends a sequence statement and message text stops at it. So the rewrite of an unquoted
message or note label swallows the `;` and everything after it into one label. Verified with
`repair`:

```text
sequenceDiagram
  participant a
  participant b
  a->>b: x; b->>a: y
  a->>b: z;
```

repairs to a document that `check` accepts (`valid=true`):

```text
sequence {
  actor a: "a"
  actor b: "b"
  a->b: "x; b->>a: y"
  a->b: "z;"
}
```

The second message is gone (merged into the first label) and the trailing `;` leaks into the
text. The importer refuses the same input (`semicolon`), so the two Mermaid entry points disagree
and `repair` is the unsafe one: the user gets a valid-looking diagram with a lost message. Graph
documents share `unquoted_label`, but the graph Mermaid form has no `: text` label, so this class
is new with the sequence kind.
**Fix:** In sequence mode, stop the unquoted label at `Semi` and let the statement terminator
handling (M8, `semicolon()`) own it:

```rust
// recover.rs, unquoted_label
let stop = match t.kind {
    TokenKind::Newline | TokenKind::Eof | TokenKind::RBrace | TokenKind::Comment(_) => true,
    TokenKind::Semi => self.seq,
    TokenKind::LBrace => stop_lbrace,
    _ => false,
};
```

Then add an S4-style case to `tests/sequence_props.rs` (a corruption kind that appends `;` and
joins two statements on one line) and an exact `repair` test for `a->>b: x; b->>a: y` expecting
two messages. Today none of kinds 0-5 exercise `;`, `<br>` or an undeclared actor.

## Warnings

### WR-01: unterminated Mermaid alias is repaired into garbage (`"\"unterminated"""`)

**File:** `crates/vox-diagram/src/parser/seq_mermaid.rs:46-82` (`mermaid_actor`)
**Issue:** For `participant a as "unterminated` the lexer's unterminated-string fix (a closing `"`
at end of line) and the Mermaid-participant rewrite (which replaces the whole statement span) are
accepted in the same round because the lexer insert only touches the end of the replaced range.
`mermaid_label` does not strip the lone leading quote, so the result is
`actor a: "\"unterminated"""`, which stays invalid (`unexpected-token`). The input is mangled and
`repair` does not converge. Messages and notes with an unterminated string repair correctly, so
this is specific to the alias path.
**Fix:** If the last scanned tail token is a `Str` with `terminated: false`, skip the rewrite fix
(report the Mermaid diagnostic with no fix) and let the lexer fix close the quote first; the next
round then rewrites a well-formed `participant a as "unterminated"`. Add a `repair` test for it.

### WR-02: Mermaid `<br>` is decoded for aliases but kept literal for message and note text

**File:** `crates/vox-diagram/src/parser/recover.rs:188-193` vs `crates/vox-diagram/src/fixgen.rs:41-64`
**Issue:** `participant a as Foo<br/>Bar` repairs to `actor a: "Foo\nBar"` (via `mermaid_label`),
but `a->>a: line1<br/>line2` and `Note over a: n1<br>n2` repair to labels containing the literal
`<br/>` / `<br>`. The importer turns all three into line breaks. The same Mermaid block therefore
renders differently depending on whether the user ran `repair` or the importer, and the SVG shows
raw `<br/>` text.
**Fix:** Route the unquoted label of a sequence message or note through `mermaid_label` (decode
`<br>`, strip matching outer quotes) instead of `quote(text)` alone.

### WR-03: a multi-hop message label is centred on the lifeline(s) it crosses, and the oracle cannot see it

**File:** `crates/vox-diagram/src/layout/seq.rs:218-229` (`between`), `layout/seq.rs:178-211`
(`lane_xs`); oracle `tests/support/svgcheck/seq.rs:78-82`
**Issue:** The label box is centred at `(a + b) / 2` and only each adjacent gap is widened. For
`a -> c` over three equally spaced lanes the midpoint is exactly lane `b`: probed output has the
label rect `x=331..510` over the lifeline at `x=420.5`. The `--vd-bg` rect (drawn after the
lifelines) erases that stretch of the middle lifeline, and the text sits on it. The geometry
oracle checks clearance from the two endpoint lifelines only, so the "labels clear of lifelines"
property that the property tests claim is not enforced for intermediate lanes.
**Fix:** Either place the label in the widest gap it crosses (or beside the arrowhead end) so that
no intermediate lifeline is inside the box, or accept the overlap deliberately and document it;
in both cases extend `message()` in `svgcheck/seq.rs` to assert no lifeline `x` with
`min(ax,bx) < x < max(ax,bx)` falls strictly inside `[l.x, l.x + l.w]` (or state the exception).

### WR-04: a one-actor note wider than its lane gap covers neighbouring lifelines

**File:** `crates/vox-diagram/src/layout/seq.rs:334-356`
**Issue:** `lane_xs` ignores notes. A note over one actor is as wide as its text (up to 60 cells,
about 420 px). In the probe, `note over a` with a long text is 419 px wide and covers lane `b`'s
lifeline (the opaque note rect hides it), so the note reads as spanning `a` and `b`. The oracle
only requires a one-actor note to be centred on its lifeline (`svgcheck/seq.rs:116-119`), so this
passes every test.
**Fix:** Widen the gaps next to a note (like the self-message label rule) or wrap one-actor notes
to the neighbouring gap width; add the "no foreign lifeline inside a note" check to the oracle.

## Info

### IN-01: single-line fallback for the declare fix reverses lane order

**File:** `crates/vox-diagram/src/resolve/seq.rs:152-175`
**Issue:** When the last `actor` statement shares the line with `}` (or with other statements),
`insert_decl` puts the new actor first in the block. `sequence { actor a: "A"; a -> b }` repairs to
`actor b` before `actor a`, so lanes become `b, a` although the module doc promises first-mention
order. Rare input; the multi-line path is correct.
**Fix:** In the fallback, insert before the first message (or after the `{` line) only when no
actor exists; otherwise insert after the last actor statement's end, not its line.

### IN-02: `insert_after_line` yields mixed line endings in CRLF files

**File:** `crates/vox-diagram/src/fixgen.rs:124-139` (test at `fixgen.rs:203-208` asserts it)
**Issue:** The inserted text starts with a bare `\n` placed before the existing `\r`, so
`actor a: "A"\r\n` becomes `actor a: "A"\n\tactor b: "b"\r\n`. `fmt` normalises LF, but `repair`
output is mixed.
**Fix:** Detect `\r\n` at the break and use it as the inserted separator.

### IN-03: diagnostics still say "graph block" in sequence documents

**File:** `crates/vox-diagram/src/parser/recover.rs:256`
**Issue:** `trailing-content` reports "after the closing `}` of the graph block" for a `sequence`
document.
**Fix:** Use the same `kind` word that `report_caps` already derives from `self.seq`.

### IN-04: `-->` family rewrites change arrow semantics

**File:** `crates/vox-diagram/src/parser/seq_mermaid.rs:117-135`
**Issue:** Mermaid `-->` (dotted, no arrowhead) and the long forms `--->`, `--->>` (not sequence
arrows at all) are rewritten to `reply a -> b` (dashed with an arrowhead), and `-->>>` is
rewritten to `reply a -> > b`, which stays invalid. Only `-->>` is an exact Mermaid dashed arrow.
**Fix:** Offer the rewrite only for `-->>`; report the others without a fix like `==>` and `---`.

---

_Reviewed: 2026-10-17_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
