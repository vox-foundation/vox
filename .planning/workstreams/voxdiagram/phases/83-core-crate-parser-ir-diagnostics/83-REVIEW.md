---
phase: 83-core-crate-parser-ir-diagnostics
reviewed: 2026-10-16T00:00:00Z
depth: standard
files_reviewed: 46
files_reviewed_list:
  - crates/vox-diagram/Cargo.toml
  - crates/vox-diagram/src/ast.rs
  - crates/vox-diagram/src/check.rs
  - crates/vox-diagram/src/codes.rs
  - crates/vox-diagram/src/diag.rs
  - crates/vox-diagram/src/fence.rs
  - crates/vox-diagram/src/fixes.rs
  - crates/vox-diagram/src/fixgen.rs
  - crates/vox-diagram/src/ir.rs
  - crates/vox-diagram/src/lexer.rs
  - crates/vox-diagram/src/lexer/mermaid.rs
  - crates/vox-diagram/src/lexer/tests.rs
  - crates/vox-diagram/src/lib.rs
  - crates/vox-diagram/src/limits.rs
  - crates/vox-diagram/src/parser/edges.rs
  - crates/vox-diagram/src/parser/frame.rs
  - crates/vox-diagram/src/parser/mermaid.rs
  - crates/vox-diagram/src/parser/mod.rs
  - crates/vox-diagram/src/parser/recover.rs
  - crates/vox-diagram/src/parser/stmt.rs
  - crates/vox-diagram/src/parser/tests.rs
  - crates/vox-diagram/src/print.rs
  - crates/vox-diagram/src/resolve.rs
  - crates/vox-diagram/src/resolve/mermaid.rs
  - crates/vox-diagram/src/resolve/tests.rs
  - crates/vox-diagram/src/sequence_ir.rs
  - crates/vox-diagram/src/span.rs
  - crates/vox-diagram/tests/determinism_guard.rs
  - crates/vox-diagram/tests/diagnostics_all_errors.rs
  - crates/vox-diagram/tests/fence_scan.rs
  - crates/vox-diagram/tests/fixes_repair.rs
  - crates/vox-diagram/tests/fmt_golden.rs
  - crates/vox-diagram/tests/fmt_props.rs
  - crates/vox-diagram/tests/grammar_card.rs
  - crates/vox-diagram/tests/graph_constructs.rs
  - crates/vox-diagram/tests/hostile_props.rs
  - crates/vox-diagram/tests/limits_adversarial.rs
  - crates/vox-diagram/tests/mermaid_catalog.rs
  - crates/vox-diagram/tests/repair_props.rs
  - crates/vox-diagram/tests/sequence_ir.rs
  - crates/vox-diagram/tests/support/mermaid.rs
  - crates/vox-diagram/tests/support/mod.rs
  - crates/vox-diagram/tests/support/noisy.rs
  - crates/workspace-hack/Cargo.toml
  - docs/src/architecture/layers.toml
  - docs/src/architecture/where-things-live.md
findings:
  critical: 3
  warning: 9
  info: 3
  total: 15
status: issues_found
---

# Phase 83: Code Review Report

**Reviewed:** 2026-10-16
**Depth:** standard (plus runtime probing of the lib built standalone from a scratchpad copy; nothing in the worktree was modified)
**Files Reviewed:** 46
**Status:** issues_found

## Summary

The crate is well structured and the panic-freedom, char-boundary and determinism work holds up: a 60k-case token-soup fuzz of `check`/`repair`/`fmt` found no panic, no out-of-range span and no non-determinism. All `get`/`find` based slicing is guarded.

The weak spot is **fix safety**, which the phase brief names as a focus. Three distinct fix paths produce wrong code, and two of them make `repair` actively worsen a document. The test suite did not catch any of them because the repair tests only apply one fault at a time to well-formed generated documents, and the hostile-input property never asserts that fixes do not increase the error count. There are also gaps in untrusted-label handling: control characters and bidi marks are echoed unsanitised into diagnostic messages, the forbidden set is incomplete, and a lone CR hides content from `check` that `fence::normalize` would turn into code.

All reproductions below were run against the reviewed sources (release build, offline).

## Critical Issues

### CR-01: Two fixes inserting at the same offset apply in the wrong order (unterminated string + unclosed brace)

**File:** `crates/vox-diagram/src/fixes.rs:84` (order), `crates/vox-diagram/src/diag.rs:147` (diagnostic sort), `crates/vox-diagram/src/parser/recover.rs:122` (EOF `\n}` fix), `crates/vox-diagram/src/lexer.rs:290` (closing-quote fix)

**Issue:** `apply_fixes` keeps equal-offset inserts in *diagnostic order*, and diagnostics are sorted by `span.start`. The `unclosed-brace` diagnostic is positioned at the `{` (early), but its fix inserts at EOF; the `unterminated-string` fix also inserts at EOF but its diagnostic sorts later. So the brace is inserted before the quote.

Repro, source `graph {\n    node a: "abc` (no trailing newline):

```
repair -> graph {\n    node a: "abc"\n}""      (valid=false, trailing-content error)
```

Same with a trailing backslash (`"abc\`). The repair loop turns one fixable file into a worse one. Any file truncated mid-string (the exact case LLM-truncation produces) hits this.

**Fix:** make EOF closers sort after other inserts at the same offset. Smallest change: give `apply_fixes` an explicit tie-break so closers go last, for example

```rust
// fixes.rs: carry a priority with each edit
type Edit<'a> = (usize, usize, &'a str);
accepted.sort_by_key(|&(s, e, r)| (s, e, r == "\n}"));   // false < true: closers last
```

(a stable sort already keeps identical `"\n}"` inserts in order). Add a regression test: `repair("graph {\n    node a: \"abc", 5)` must be valid with text `graph {\n    node a: "abc"\n}`.

### CR-02: `quote()` emits a raw newline, so the chained-edge fix generates broken code

**File:** `crates/vox-diagram/src/fixgen.rs:21` (`quote`), `crates/vox-diagram/src/parser/edges.rs:219` (call site)

**Issue:** `quote` escapes `\` and `"` and drops `\r`, but passes `\n` through. A decoded label can contain `\n` (from the `\n` escape). In `expansion`, a trailing edge label is re-quoted with `quote(&l.value)`, so the fix writes a literal line break inside the string.

Repro:

```
graph {
    node a: "A"
    node b: "B"
    node c: "C"
    a -> b -> c: "x\ny"
}
repair -> "    a -> b\n    b -> c: \"x\"\ny\"\"\n}"      (valid=false)
```

The label is split across lines, the tail `y"` becomes a statement, and the next repair round adds another stray quote. Labels with `\n` are legitimate and used by `mermaid_label` (which correctly splits on `\n`), so this is an inconsistency inside `fixgen`.

**Fix:** make `quote` escape the newline, matching `print::label` and `mermaid_label`:

```rust
'\n' => out.push_str("\\n"),
```

(`mermaid_label` already splits on `\n` before calling `quote`, so it is unaffected.) Add a test next to `quote_escapes_and_drops_unrepresentable_chars`.

### CR-03: Rule R declares the missing node in the wrong scope when the `{` line is not otherwise empty; repair then diverges

**File:** `crates/vox-diagram/src/fixgen.rs:98-112` (`insert_decl`)

**Issue:** For a multi-line scope the declaration is inserted at the end of the line containing the scope's `{`, assuming `{` is the last thing on that line. When something follows the `{` on the same line, and that something opens a nested block, the insert lands inside the nested block.

Repro (`graph { container c: "C" {` on one line, undeclared refs at root):

```
graph { container c: "C" {
  node x: "X"
}
  a -> ghost
}
```

Eight repair rounds produce eight copies of `node a: "a"` / `node ghost: "ghost"` inside `c`, 14 `duplicate-id` errors, and the root refs are still undeclared. The same flaw puts the declaration inside an attribute block for `graph { node a: "A" {` style layouts. It is reported as a safe, machine-applicable fix and converges nowhere.

**Fix:** only use the "end of the `{` line" form when the rest of that line is blank or a `//` comment; otherwise fall back to the existing single-line form (insert directly after the `{`).

```rust
let first_line = &scope_text[..scope_text.find('\n').unwrap_or(scope_text.len())];
let rest = first_line.trim();
let multi = scope_text.contains('\n') && (rest.is_empty() || rest.starts_with("//"));
```

Add a regression test with the repro above asserting `repair` ends valid with the nodes declared at root.

## Warnings

### WR-01: A lone CR hides content from `check` that `fence::normalize` turns into statements

**File:** `crates/vox-diagram/src/lexer.rs:214-227` (`comment`), `crates/vox-diagram/src/lexer.rs:264` (`string`), `crates/vox-diagram/src/lexer/mermaid.rs:86` (`rest_of_line`), `crates/vox-diagram/src/fence.rs:76`

**Issue:** The crate has three different definitions of a line end. `comment()` runs through a lone `\r` (only `\r\n` ends it); `string()` and `rest_of_line()` end at any `\r`; `LineIndex` counts only `\n`; `fence::normalize` rewrites a lone `\r` to `\n`. Repro: `// note\r a -> ghost2` inside `graph { }` checks `valid=true` (the `a -> ghost2` is comment text), while the LF form of the same bytes reports `undeclared-node`. A document body that passes `check` can therefore render or hash as a different, unchecked document once normalised. Diagnostic line numbers also disagree with editors for CR-only text.

**Fix:** pick one rule. Either end `comment()` at any `\r` (consistent with `string`/`rest_of_line`) and treat a lone `\r` as `Newline` in `plain_token`, or reject a lone CR with a diagnostic. Then add `\r` to `LineIndex`.

### WR-02: Diagnostic messages echo untrusted text containing control characters, bidi marks and newlines

**File:** `crates/vox-diagram/src/parser/recover.rs:17` (`describe` for `Str`), `crates/vox-diagram/src/parser/recover.rs:192` (`unquoted_label`), `crates/vox-diagram/src/lexer.rs:329` (invalid-escape `shown`)

**Issue:** `clip()` limits length but not content. A label such as `"x<ESC>[2Jy"` yields the message `expected end of line, found string "x\u{1b}[2Jy"` with the raw ESC byte (verified), and an unquoted label echoes raw text the same way. Decoded `\n` escapes put a real newline into a message, which forges log lines. Messages are consumed by the CLI, MCP and agents, which is the untrusted-label threat the phase calls out (T-83-11 only addresses length).

**Fix:** sanitise at the echo point, e.g. in `diag::clip`: map `is_forbidden(c)`, `\n`, `\t` and the bidi set to `char::escape_default`/`\u{..}` form. One helper used by both `clip` functions covers every site.

### WR-03: The forbidden-character set misses C1 controls and bidi marks

**File:** `crates/vox-diagram/src/lexer.rs:83-87`

**Issue:** `is_forbidden` covers C0, DEL and U+202A-202E / U+2066-2069 only. Not covered: C1 controls U+0080-009F (U+009B is a CSI introducer in 8-bit terminals), U+200E/200F and U+061C (implicit bidi marks), U+2028/2029, and U+200B-200D / U+FEFF inside labels. Labels reach the IR unchanged, so they pass through to renderers and terminals.

**Fix:** extend the `matches!` with `'\u{80}'..='\u{9f}' | '\u{200e}' | '\u{200f}' | '\u{061c}' | '\u{2028}' | '\u{2029}'` (and decide on zero-width characters explicitly). `fixgen::quote` and `mermaid_label` already route through `is_forbidden`, so the fixes stay consistent. Update the `support::allowed` generator in `tests/support/mod.rs:65` to match.

### WR-04: Empty-edge-label fix deletes the target's bracket text

**File:** `crates/vox-diagram/src/parser/edges.rs:65-71`, `crates/vox-diagram/src/parser/stmt.rs:86-92`

**Issue:** `Owner::Edge { from }` is the end of the last endpoint's *segments* (`r.span.end`), which excludes a following `[...]` bracket. The fix deletes `from..label_end`, so any bracket text between them goes too. Repro: `a -> B((x)) : ""` repairs to `node B: "B"` / `a -> B`; the `((x))` label is lost silently and the node gets a different label. This contradicts the Mermaid fix principle that a rewrite must never drop text.

**Fix:** compute `from` from the last consumed token before the colon (`self.toks[self.pos - 1].span.end` captured before `self.bump()` of the colon), not from `Ref.span`.

### WR-05: `A-.->B` and `A-.-B` mis-lex as an identifier `A-`

**File:** `crates/vox-diagram/src/lexer.rs:230-240` (`word`)

**Issue:** A `-` joins a word unless followed by `>` or `-`. `-.->` begins `-.`, so `A-.->B` lexes as `Ident("A-")`, `Dot`, `Arrow` and reports `expected an identifier after '.'` instead of the intended "no VoxDiagram equivalent" Mermaid diagnostic. `A==>B` and `A---B` are fine, so this is an inconsistent hole in the Mermaid catalogue.

**Fix:** add `r.starts_with(".-")` to the non-join condition.

### WR-06: Quadratic work in the lexer on `a>a>a>...`

**File:** `crates/vox-diagram/src/lexer/mermaid.rs:107-130` (`bracket`)

**Issue:** For the `>` opener an unclosed form returns `None` without consuming anything, but before returning it runs `body.find("]")` and `body.trim_end().to_string()` over the rest of the line. Every `>` directly after an identifier repeats that. 30,000 `a>` pairs (60 KB, under the cap) take 237 ms against 18 ms for 8,000 pairs, which is quadratic scaling. `lexer.rs:1-4` and `parser/mod.rs:1-11` document the lexer as one-pass O(n), and `limits_adversarial.rs` does not include this shape (its 10 s timeout cannot detect it).

**Fix:** bail out for `>` before building the unclosed variant (`if open == ">" { return None }` when `find_close` fails), and avoid the `to_string()` for the discarded unclosed candidate. Cache a per-line "no `]` after this offset" like `dash_fail_until`. Add `"a>".repeat(30_000)` to `adversarial_mermaid_tokens`.

### WR-07: A bare `graph` line followed by `{` on the next line is rewritten into a worse document

**File:** `crates/vox-diagram/src/parser/mermaid.rs:37-66` (`mermaid_header`), `crates/vox-diagram/src/parser/mermaid.rs:70-93` (`report_header`)

**Issue:** `graph` followed by a newline is accepted as a Mermaid header. For `graph\n{\n  node a: "A"\n}` the fix gives `graph {\n{\n  node a: "A"\n}\n}`, which has two errors (`unexpected-token`, `trailing-content`) where the original was one repairable-looking diagnostic.

**Fix:** in `mermaid_header`, when the header ends at a `Newline`, peek past newlines and return `None` if the next token is `LBrace` (so the normal `graph` path reports the missing `{`).

### WR-08: The repair/hostile tests do not assert that fixes are safe, which is why CR-01 to CR-03 shipped

**File:** `crates/vox-diagram/tests/hostile_props.rs:101-115` (`fixes_are_safe`), `crates/vox-diagram/tests/repair_props.rs:47` (`sites`)

**Issue:** `fixes_are_safe` computes `check(&out.text)` and discards it (`let _ = ...`) and asserts only that `repair` ran at most 3 rounds, so it cannot fail on wrong output; the doc comment "the fixed text still checks" overstates it. `mutate_and_repair` applies exactly one inverse-exact corruption to a well-formed, multi-line generated document, so it never exercises combined faults (truncation, string + brace), one-line scopes, `\n` in labels, or brackets plus empty labels. The three critical bugs above are all outside that space, and the fuzz that found them is a 15-line loop.

**Fix:** add a property that checks, for any soup input, that `error_count(repair(src).report) <= error_count(check(src))` whenever `check(src)` has fixes (my fuzz found 1 violation in 60,000 tiny inputs and the three deterministic cases above). Add truncation corruption (cut at a random char boundary) to `repair_props` and include `"\\n"`-bearing labels in `chained` fixtures.

### WR-09: Fuel exhaustion halts the parser without recording an error

**File:** `crates/vox-diagram/src/diag.rs:136` (`trip`), `crates/vox-diagram/src/parser/recover.rs:42` (`spend_fuel`)

**Issue:** `trip()` only sets `overflow`; it does not set `has_error`. If fuel ever runs out before an error was reported, the AST is truncated but `check` returns `valid: true` with an IR missing statements, plus a misleading "more than 99 diagnostics" warning. I could not drive valid input to exhaustion (valid syntax spends about 2 units per statement against a `4 * tokens + 1000` budget), so this is defence in depth rather than a demonstrated bug, but the failure mode is silent acceptance of partial input.

**Fix:** in `trip()` set `self.has_error = true`, and use a dedicated message/code for "input too complex" instead of the diagnostics-limit sentinel.

## Info

### IN-01: Dead code and duplicated helper

**File:** `crates/vox-diagram/src/span.rs:72-76`, `crates/vox-diagram/src/lexer.rs:~60` and `crates/vox-diagram/src/diag.rs:269`

**Issue:** `span::slice` is `#[allow(dead_code)]`, used only by its own test, and its comment ("consumed by plans 02-06") is stale. `lexer::clip` (20 chars) duplicates `diag::clip` (40 chars) under the same name with a different cap.

**Fix:** delete `slice` (or use it in `fixes.rs`/`fixgen.rs`), and have the lexer call `diag::clip`.

### IN-02: Bracket statement fix is not gated on `declarable`

**File:** `crates/vox-diagram/src/resolve/mermaid.rs:41-48`

**Issue:** `bracket_stmt` emits `node <id>: "..."` without the `declarable(&id)` check that `bracket_endpoint` uses. `shape[Lbl]` gets a "fix" `node shape: "Lbl"` that immediately fails with `reserved-word`.

**Fix:** require `declarable(id)` before offering the fix.

### IN-03: Inserted fixes always use LF, producing mixed line endings in CRLF files

**File:** `crates/vox-diagram/src/fixgen.rs:98-112`, `crates/vox-diagram/src/parser/edges.rs:236`, `crates/vox-diagram/src/parser/mermaid.rs:113`

**Issue:** `insert_decl`, `expansion` and `semicolon` insert `\n` regardless of the file's line endings. Valid output, but a CRLF document ends up with mixed endings; `fmt` normalises to LF so this only matters for `repair` without `fmt`.

**Fix:** detect `\r\n` at the insertion point (as `insert_decl` already does for the position) and reuse it.

---

_Reviewed: 2026-10-16_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
