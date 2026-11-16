---
phase: 83-core-crate-parser-ir-diagnostics
reviewed: 2026-10-16T00:00:00Z
depth: standard
files_reviewed: 14
files_reviewed_list:
  - crates/vox-diagram/src/diag.rs
  - crates/vox-diagram/src/fixes.rs
  - crates/vox-diagram/src/fixgen.rs
  - crates/vox-diagram/src/lexer.rs
  - crates/vox-diagram/src/lexer/mermaid.rs
  - crates/vox-diagram/src/parser/edges.rs
  - crates/vox-diagram/src/parser/mermaid.rs
  - crates/vox-diagram/src/parser/stmt.rs
  - crates/vox-diagram/src/span.rs
  - crates/vox-diagram/tests/fix_safety.rs
  - crates/vox-diagram/tests/hostile_props.rs
  - crates/vox-diagram/tests/limits_adversarial.rs
  - crates/vox-diagram/tests/repair_props.rs
  - crates/vox-diagram/tests/support/mod.rs
findings:
  critical: 1
  warning: 0
  info: 6
  total: 7
status: issues_found
iteration: 2
---

# Phase 83: Code Review Report (iteration 2)

**Reviewed:** 2026-10-16
**Depth:** standard (plus runtime probing of the current sources built standalone in the session scratchpad; nothing in the worktree was modified)
**Files Reviewed:** 14
**Status:** issues_found

## Summary

Re-review of the 12 iteration-1 fixes (CR-01..03, WR-01..09). The current `vox-diagram` sources plus tests were copied to the scratchpad and built there: the full crate suite passes (all unit, integration and property tests), and a 60k-case plus a new 200k-case token-soup fuzz (extra atoms: lone CR, C1 controls, U+2028, U+200E, `container .. {` one-liners, `>`/`]`, `A-.-B`, empty edge labels) found 0 panics and 0 cases where `repair` increased the error count (iteration 1 found 1 in 60k). The 11 non-converging-in-6-rounds cases from the new fuzz converge as soon as more rounds are allowed.

Each prior finding was re-run with the reviewer's original repro input:

| ID | Result |
|----|--------|
| CR-01 | Fixed for native `graph {` documents (`graph {\n    node a: "abc` repairs to valid `... "abc"\n}`; trailing-backslash variant valid). **Not fully resolved**: the same ordering bug survives for Mermaid-header documents, see CR-01 below. |
| CR-02 | Fixed. `a -> b -> c: "x\ny"` repairs to `a -> b` / `b -> c: "x\ny"`, valid. |
| CR-03 | Fixed. `graph { container c: "C" {` repro repairs in one round to a valid document with `a` and `ghost` declared once at root. Also checked `node a: "A" {`, `// c {` trailing comment and `{ ... }` attribute-block layouts: all valid. |
| WR-01 | Fixed. `graph {\r// note\r a -> ghost2\r}\r` now reports `undeclared-node` at 3:2 and 3:7, matching the LF form. Lone-CR text also gets a correct line index. |
| WR-02 | Fixed. `clip` escapes forbidden characters, CR, LF and tab as `\u{..}`; lexer invalid-escape echo routed through it. All other echo sites either use `clip` or print ASCII-validated identifiers. |
| WR-03 | Fixed. C1, U+061C, U+200E/F, U+2028/9 are forbidden; the test generator matches. Zero-width joiners left legal and documented. |
| WR-04 | Fixed. `a -> B((x)) : ""` repairs to `a -> B((x))`; bracket text is kept. |
| WR-05 | Fixed. `A-.->B`, `A-.-B`, `A -.-> B` each give exactly one `mermaid-syntax` error (verified the lexer path; also covered by `wr05_dotted_mermaid_links_after_an_identifier`). |
| WR-06 | Fixed. Gate is logically equivalent to the old behaviour (for `>`, a `]` anywhere later on the line means `find_close` always succeeds, so the removed `open == ">" && !closed` return is unreachable) and `gt_fail_until` resets at each line end. |
| WR-07 | Fixed. `graph\n{\n  node a: "A"\n}` yields no `mermaid-syntax` diagnostic and is left alone (2 errors before and after repair, down from rewriting into a worse document). |
| WR-08 | Fixed as scoped (error-count no-worse property). It cannot see the residual in CR-01 below because no Mermaid document is in either generator. |
| WR-09 | Fixed. `trip()` sets `has_error`; `check` derives `valid` from it. See IN-06 for a presentational leftover. |

## Critical Issues

### CR-01: EOF closer ordering fix does not cover Mermaid-header documents (unterminated string + Mermaid header)

**File:** `crates/vox-diagram/src/fixes.rs:85` (tie-break), `crates/vox-diagram/src/parser/mermaid.rs:83-88` (Mermaid closer text)

**Issue:** The iteration-1 tie-break is `accepted.sort_by_key(|&(s, e, r)| (s, e, r == "\n}"))`. It only recognises the exact native closer `"\n}"` from `recover.rs`. The Mermaid header diagnostic (M5) also inserts a closing brace at EOF, but its replacement is `"}\n"` or `"\n}\n"` (`report_header`), so it is not pushed last. The header diagnostic is at offset 0 and sorts first, so its closer is still applied before the closing quote of an unterminated string at EOF: exactly the CR-01 failure, one input family over. A truncated Mermaid document (the LLM-cut-off case that motivated CR-01) is still turned into an invalid one.

Repros (run against the current sources, `repair(src, 8)`):

```
flowchart TD\nA --> B\nB --> C: "x
  -> graph {\n    node A: "A"\n ... B -> C: "x"\n}\n""      valid=false
     diagnostics: diagram/trailing-content 9:1 (no fix, repair stops)

graph LR\nnode a: "x
  -> graph {\n    direction: right\nnode a: "x"\n}\n""      valid=false
```

The correctly ordered result (quote, then `\n}\n`) would be valid; the string-close fix and the closer are both accepted (`applied` counts them) but land in the wrong order. The stray `""` is not fixable by a later round.

**Fix:** classify closers by content rather than by one literal, so every EOF scope closer sorts after other equal-offset inserts:

```rust
// fixes.rs
accepted.sort_by_key(|&(s, e, r)| (s, e, r.trim() == "}"));
```

(`"\n}"`, `"}\n"` and `"\n}\n"` all trim to `}`; stable sort keeps relative order of closers.) Add a regression next to `cr01_truncated_mid_string_repairs_quote_before_brace`:

```rust
assert_repairs_to("graph LR\nnode a: \"x", /* expected valid text */);
assert!(repair("flowchart TD\nA --> B\nB --> C: \"x", 5).report.valid);
```

and add a Mermaid-header document (or a truncation of `support::mermaid` fixtures) to `repair_props::truncate_and_repair_never_worsens`, which currently only draws from the native `model_graph` and therefore cannot reach this path.

## Info

### IN-01: Dead code and duplicated helper (carried forward, unresolved)

**File:** `crates/vox-diagram/src/span.rs:79-84`, `crates/vox-diagram/src/lexer.rs:94-96`, `crates/vox-diagram/src/diag.rs:90`

**Issue:** `span::slice` is still `#[allow(dead_code)]` with the stale "consumed by plans 02-06" comment. `lexer::clip` (20 chars, no escaping) still duplicates `diag::clip` (40 chars, escaping) under the same name; the lexer now uses both (`lexer.rs:257` and `lexer.rs:332`). The unescaped one is safe today only because it is applied to alphanumeric words.

**Fix:** delete `slice`; have `word()` call `crate::diag::clip` and remove the local helper.

### IN-02: Bracket statement fix is not gated on `declarable` (carried forward, unresolved)

**File:** `crates/vox-diagram/src/resolve/mermaid.rs:41-48`

**Issue:** Still reproduces: `graph LR\nshape[Lbl]` offers the fix `node shape: "Lbl"`, which then fails with `reserved-word`. `bracket_endpoint` already checks `declarable(&id.name)` (line 73). The header comment of `tests/fix_safety.rs` lists IN-02 among the covered cases, but there is no IN-02 test in that file.

**Fix:** require `declarable(id)` before offering the fix, add the test, or drop IN-02 from the header comment.

### IN-03: Inserted fixes always use LF and `indent_of`/`insert_decl` ignore lone CR (carried forward, extended)

**File:** `crates/vox-diagram/src/fixgen.rs:67-74` (`indent_of`), `crates/vox-diagram/src/fixgen.rs:106` (`scope.find('\n')`), `crates/vox-diagram/src/parser/edges.rs:236`, `crates/vox-diagram/src/parser/mermaid.rs:113`

**Issue:** CRLF documents still get LF-only inserts (mixed endings). After the WR-01 decision that a lone CR is a line break, `indent_of` still looks only for `\n` (a nested scope in a CR-only document is indented from the start of the file) and `insert_decl` treats a multi-line CR-only scope as single-line. Output stays valid (verified `graph {\r node a: "A"\r a -> ghost\r` repairs valid), but the result is `\n`-mixed and mis-indented.

**Fix:** treat `\r` like `\n` in `indent_of` (`rfind(['\n', '\r'])`) and in the `find('\n')` of `insert_decl`; reuse the document's line ending for inserted line breaks.

### IN-04: Timing-based scaling test can flake on a loaded runner

**File:** `crates/vox-diagram/tests/limits_adversarial.rs:236-256`

**Issue:** `unclosed_gt_openers_scale_linearly` compares wall-clock best-of-three for 8k vs 32k pairs (`large < small * 10 + 20 ms`). On a loaded shared CI shard where `small` is sub-millisecond and one outlier lands in the large run, this can fail without a regression. The fix report already flags this.

**Fix:** keep it, but count work instead of time if a counter is cheap to add, or mark the assertion with a retry (rerun `large` once before failing).

### IN-05: `fence.rs` still counts only `\n` for the opening line

**File:** `crates/vox-diagram/src/fence.rs:53`

**Issue:** WR-01 made the diagram core use one line-break rule (CR, LF, CRLF), but `open_line` is `md[..block.start].matches('\n').count() + 1`. Markdown with CR-only line endings (which pulldown-cmark treats as line breaks) gets an `open_line` that disagrees with the core's line numbers. Found by inspection; not run.

**Fix:** count line breaks with the same rule as `LineIndex` (reuse it, or count `\n` plus lone `\r`).

### IN-06: A fuel-halted report is `valid: false` with only a Warning-severity diagnostic

**File:** `crates/vox-diagram/src/diag.rs:146-153` and `crates/vox-diagram/src/diag.rs:164-174`

**Issue:** WR-09's `trip()` correctly makes `check().valid == false` and withholds the IR, but the only emitted diagnostic is the `limit-diagnostics` sentinel with `Severity::Warning`. A consumer that renders only errors, or the new `errors()` helpers in the tests, sees an invalid report with zero errors. Defence in depth only (fuel exhaustion is not reachable from valid input).

**Fix:** when `halted`, emit the sentinel at `Severity::Error` (the code can stay `limit-diagnostics`).

---

_Reviewed: 2026-10-16_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard (iteration 2)_
