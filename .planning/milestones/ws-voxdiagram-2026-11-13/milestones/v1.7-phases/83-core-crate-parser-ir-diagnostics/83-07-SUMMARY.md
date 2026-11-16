---
phase: 83-core-crate-parser-ir-diagnostics
plan: 07
subsystem: diagram-core
tags: [rust, mermaid, lenient-parse, fixes, grammar-card, proptest, insta, governance]

requires:
  - phase: 83-04
    provides: "Collector funnel, recovery, fuel, code catalogue"
  - phase: 83-05
    provides: "Canonical fmt (grammar card example is a fixed point)"
  - phase: 83-06
    provides: "apply_fixes, repair, Rule R declare-fix, hostile-input properties"
provides:
  - "Mermaid-isms M1-M10 parse leniently into the canonical AST; each is an Error (IR withheld) with the exact rewrite as fixes[] where one exists"
  - "diagram/mermaid-syntax and diagram/chained-edge codes (ALL_CODES = 29)"
  - "GRAMMAR_CARD: 1,747-byte draft card, dogfooded by tests"
  - "S4 mutate-and-repair property over five exact-inverse corruptions"
  - "6 corpus-shaped Mermaid fixtures that repair to valid documents (reviewed snapshots)"
  - "Phase governance state: API page regenerated, hakari/crate-edges/arch-check/clippy/rustdoc clean"
affects: [84, 85, 87, 88]

key-files:
  created:
    - crates/vox-diagram/src/lexer/mermaid.rs
    - crates/vox-diagram/src/parser/mermaid.rs
    - crates/vox-diagram/src/parser/edges.rs
    - crates/vox-diagram/src/parser/frame.rs
    - crates/vox-diagram/src/resolve/mermaid.rs
    - crates/vox-diagram/grammar-card.txt
    - crates/vox-diagram/tests/mermaid_catalog.rs
    - crates/vox-diagram/tests/grammar_card.rs
    - crates/vox-diagram/tests/repair_props.rs
    - crates/vox-diagram/tests/support/mermaid.rs
    - crates/vox-diagram/tests/fixtures/mermaid/ (6 fixtures)
    - crates/vox-diagram/tests/snapshots/mermaid_catalog__*.snap (6)
  modified:
    - crates/vox-diagram/src/lexer.rs
    - crates/vox-diagram/src/parser/mod.rs
    - crates/vox-diagram/src/parser/stmt.rs
    - crates/vox-diagram/src/parser/recover.rs
    - crates/vox-diagram/src/resolve.rs
    - crates/vox-diagram/src/ast.rs
    - crates/vox-diagram/src/fixgen.rs
    - crates/vox-diagram/src/print.rs
    - crates/vox-diagram/src/codes.rs
    - crates/vox-diagram/src/lib.rs
    - crates/vox-diagram/tests/hostile_props.rs
    - crates/vox-diagram/tests/limits_adversarial.rs
    - crates/vox-diagram/tests/fixtures/codes.golden.txt
    - docs/src/api/vox-diagram.generated.md

key-decisions:
  - "Free-text Mermaid forms are single lexer tokens (links with their |text| / -- text --> label, id[Label] bracket forms, whole styling lines, %% comments, :::name), so label text never reaches the identifier rules and cannot raise lexical noise such as invalid-identifier on `2px`"
  - "M3 (id[Label]) is reported by the resolver, not the parser: the in-edge fix declares the node via Rule R, which needs the scope's braces. The parser only annotates the AST (`Ref.bracket`, `NodeDecl.bracket`, `#[serde(skip)]` and boxed so Entry stays small)"
  - "A chain or `&` fan-out gets ONE diagnostic whose fix replaces the whole statement by the expanded edge lines (per-arrow diagnostics are suppressed then, so no overlapping groups); with no exact expansion it gets a fix-less diagnostic plus the per-arrow fixes"
  - "The link label is inserted after the target's bracket text (`C(Process): \"yes\"`, not between id and bracket), so a pending C(Process) rewrite stays whole and the two groups touch instead of overlap"
  - "The lexer caches the end of the current line (`rest_of_line`): a line full of links or brackets is scanned once, not once per token"
  - "M5 closes the block with `}\\n` when the source already ends in a newline (`\\n}\\n` otherwise), so repair leaves no blank line before the brace"

patterns-established:
  - "Mermaid catalogue tests assert (original text at fix span, replacement) pairs through the public API"
  - "Corpus-shaped fixtures: invalid at start, valid within 5 rounds, stable under a second repair, snapshot of the repaired text"
  - "S4 sites are located with the AST (never by text search), so a `->` inside a label or comment is never corrupted"

requirements-completed: [DGM-01, DGM-03, DGM-04]

duration: about 3h
completed: 2026-10-16
status: complete

actuals:
  tokens: 19500
  tasks: 3
  commits: 3
plan_head_before: 30082f0f2d0cf56b1171092da746ee72dc468ec1
plan_head_after: 9b34403108e379f96970281a89bc182ac2ee7816
---

# Phase 83 Plan 07: Mermaid-isms, grammar card, S4 property and governance closure Summary

**Mermaid flowchart text (`-->`, `-->|t|`, `-- t -->`, `A[Label]` in four shapes, `subgraph ... end`, `graph TD` / `flowchart LR`, `%%`, `;`, `&`, chains, styling) parses leniently, is reported as `diagram/mermaid-syntax` or `diagram/chained-edge` Errors with exact rewrites, and `repair` turns corpus-shaped Mermaid into valid VoxDiagram in at most 3 rounds.**

## Accomplishments

- Tracer: `a --> b` flows lexer (one `MArrow` token) -> lenient Edge in the AST -> one Error on the `-->` span with fix `-->` => `->` -> `repair`, with the IR withheld until repaired (A2). `A --> B` with both ends undeclared converges in 1 round (the arrow and both declarations are independent fixes), better than the planned 2. `==>`, `-.->`, `---`, `--x`, `--o` report without a fix.
- Catalogue (one named test per row, 24 tests in `mermaid_catalog`, each asserting exact codes, fix spans and replacement text): M2 pipe and dash labels (quotes stripped, `<br>` -> `\n`, `\` and `"` escaped, label inserted after the target); M3 statement and in-edge forms for rect/rounded/diamond/cylinder, quoted and punctuated labels (`A[API: gw (beta) / x.y]` is never split), unfixable shapes (`((x))`, `([x])`, `[[x]]`, `{{x}}`, `>x]`, unclosed), repeats (same label: bare id only; different: `duplicate-id`, no silent text loss); M4 three subgraph forms with one diagnostic and two edits, title-only no fix; M5 all five direction tokens, none, trailing `;`, with and without a trailing newline; M6 `direction LR` / `direction: LR`; M7 `%%` and `%%{` directive; M8 trailing and mid-line `;`; M9 eight styling heads plus `:::name`, and `class -> style` stays an edge; M10 `&` fan-out and chains, with Mermaid arrows, labels and trailing labels.
- 6 corpus-shaped fixtures (header, subgraphs, `A[Label] --> B`, three label spellings, `%%`, `;`, `&`, chain, nested subgraphs) are invalid, repair to valid within 5 rounds, are stable under a second repair, and match reviewed snapshots.
- Grammar card: 1,747 bytes (budget 2,600), no triple-backtick lines. Four dogfood tests: the example checks with zero diagnostics and is a `fmt` fixed point; each of the 10 "Do NOT write" examples yields `diagram/mermaid-syntax`; the six reserved words on the card are each rejected as ids; size budget.
- S4 `mutate_and_repair` (128 cases; a temporary 5,000-case run also passed): arrow `->` => `-->`, unquoted edge label ending its line, deleted final `}`, dropped closing `"`, `node A: "L"` => `A[L]`. Sites come from the AST; a companion test proves every kind has a real site and repairs back, so the property cannot pass by always falling back to the same corruption.
- Safety: Mermaid fragments added to the S3 hostile token soup (temporary 4,000-case run passed) and a new `adversarial_mermaid_tokens` test (12 hostile 60 KiB inputs). It caught a real quadratic scan on its first run (see Deviations).
- Final state: 170 tests pass under `INSTA_UPDATE=no`; clippy, rustdoc and `cargo fmt -p vox-diagram --check` clean; no `*.snap.new`, no `proptest-regressions` file; all files under 400 non-blank lines (largest `parser/mod.rs` 344, `mermaid_catalog.rs` 391).

## Task Commits

1. **Task 1 (tracer): Mermaid arrow end to end** - `e29441480`
2. **Task 2: M2-M10 and corpus fixtures** - `b3639b5dc`
3. **Task 3: grammar card, S4, API page** - `9b3440310`

## TDD / RED evidence

- Task 1: `mermaid_m1_*` (3) written first; run: `3 tests run: 0 passed, 3 failed`. GREEN: 3 passed.
- Task 2: honest note. I drafted the implementation before writing the Task 2 tests, so RED was observed after the fact, not before: with `src` reverted to the Task 1 commit (`git checkout HEAD -- crates/vox-diagram/src`, later restored from a copy and verified with `diff -r`) the 24-test binary gave `23 tests run: 5 passed, 18 failed` (the 5 passes are the three M1 tests, the class-as-id guard and the "every diagnostic is an Error" guard, which hold trivially without the feature). Then GREEN after two real bugs the tests found (see Deviations 3 to 5).
- Task 3: `grammar_card` written first; run: compile error `unresolved import vox_diagram::GRAMMAR_CARD` (RED). `repair_props` passed on first run (no natural RED: it guards fixes built in Tasks 1-2); sensitivity proven by mutation below.

## Mutation proofs

All with before/after presence checks and a restore verified by `diff` against a pre-mutation copy.
- **Label insertion point** (`parser/edges.rs`): `map_or(to.span.end, |b| b.form.end)` (count 1, mutant 0) -> `let at = to.span.end;` (count 0, mutant 1) -> `mermaid_m2_label_goes_after_a_bracketed_target` and `mermaid_corpus_fixtures_repair` FAILED -> restored (count 1, `diff` identical). The first attempt showed only the corpus test caught it, so the targeted test was added.
- **M1 replacement text** (`parser/edges.rs`): `replace(hop.span, "->")` -> `"-->"` (count 1 -> 0, mutant 0 -> 1) -> `mutate_and_repair` and `every_corruption_kind_applies_and_repairs` FAILED (0 passed, 2 failed) -> restored (`diff` identical). The `repair_props.proptest-regressions` file the mutation wrote was deleted, not committed.

## Closing gates (all run from `/Users/brbrainerd/dev/vox-voxdiagram`)

| Gate | Result |
|------|--------|
| `vox ci crate-api-sync --write` then `vox ci crate-api-sync` | OK (130 crates); page regenerated after the last `lib.rs` change; `GRAMMAR_CARD` present |
| `cargo hakari generate --diff` | exit 0, empty output |
| `vox ci crate-edges` | exit 0 (700 live edges within baseline; one pre-existing `stale baseline edge vox-gui -> vox-db-types` warning, not touched) |
| `cargo run -p vox-arch-check` | exit 0 (only pre-existing evidence-staleness warnings; nothing about vox-diagram) |
| `cargo clippy -p vox-diagram --all-targets -- -D warnings` | exit 0 |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p vox-diagram --no-deps` | exit 0 after the ast.rs link fix |
| `INSTA_UPDATE=no cargo nextest run -p vox-diagram` | 170 passed |
| baseline files vs merge-base | `crate-edges.allow.v1.json` and `fan-in-snapshot.v1.json` unchanged; no `exceptions` written; Cargo.toml / Cargo.lock untouched |

The installed `vox` binary was stale at first (the `vox ci` freshness guard refused); I rebuilt it with `cargo build -p vox-cli` (24 s) instead of overriding the guard.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Pre-existing rustdoc error blocked the closing gate**
- **Found during:** Task 3 closure
- **Issue:** `src/ast.rs` module doc linked `[`Span`]` with no `Span` in scope; `RUSTDOCFLAGS='-D warnings' cargo doc` failed.
- **Fix:** `[`Span`](crate::span::Span)`.
- **Commit:** `9b3440310`

**2. [Rule 2 - Missing critical functionality] Quadratic lexer scan found by the new adversarial test**
- **Found during:** Task 2 (`adversarial_mermaid_tokens`, first run: 10 s timeout)
- **Issue:** every link and bracket re-split the whole rest of the line, so `-- ` x 21,000 on one line took 19 s (`-->| ` 5 s, `a --> b ` x 8,000 7 s).
- **Fix:** `rest_of_line` caches the end of the current line, and failed `-->` / `|` scans are memoised per line (T-83-24). All twelve inputs now lex in under 10 ms.
- **Files modified:** `src/lexer/mermaid.rs`, `src/lexer.rs`
- **Commit:** `b3639b5dc`

**3. [Rule 1 - Bug] Link label landed inside a pending bracket rewrite**
- **Found during:** Task 2, fixture `02-flowchart-labels` (`B -- yes --> C(Process)` repaired to `C: "yes"(Process)`)
- **Issue:** the `: "label"` insert sat between `C` and `(Process)` while the M3 fix was not yet available (root had no braces until the header was fixed).
- **Fix:** insert after the target's bracket text; covered by `mermaid_m2_label_goes_after_a_bracketed_target`.
- **Commit:** `b3639b5dc`

**4. [Rule 1 - Bug] Mid-line `;` left a 5-space indent**
- **Found during:** Task 2, fixture `05` snapshot review (`ingest -> parse; parse -> emit` repaired to ` parse -> emit`)
- **Fix:** the mid-line fix replaces `;` plus the blanks after it with the line break and indent (`"; "` => `"\n    "`).
- **Commit:** `b3639b5dc`

**5. [Rule 1 - Bug] Re-declaring a clashing label offered a second declaration**
- **Found during:** Task 2 (`mermaid_m3_repeat_same_and_different`)
- **Issue:** `X[Two]` after `X[One]` got the replace-and-declare fix, which would only repeat the `duplicate-id`.
- **Fix:** no fix when the id is already declared in the scope.
- **Commit:** `b3639b5dc`

**6. [Rule 3 - Blocking] Files would exceed the 400-line cap**
- **Fix:** `parser/mod.rs` shed `describe`, `count`, `report_caps` and `trailing_content` to `recover.rs`; the Mermaid parser is split into `mermaid.rs` (header, direction, `;`, styling), `edges.rs` (chains, fan-out, labels, endpoints) and `frame.rs` (subgraph); the lexer gets `lexer/mermaid.rs`; test helpers live in `tests/support/mermaid.rs`; `Ref.bracket` and `NodeDecl.bracket` are boxed after clippy flagged `Entry` at 280 bytes.

### Plan-versus-reality notes

- **M3 lives in the resolver, M7 and M9 in the lexer.** The plan puts every recognizer in `parser/mermaid.rs`. Free-text forms must be single tokens or their text raises lexical noise (`linkStyle 0 stroke:red`, `stroke-width:2px` give `invalid-identifier`), and the in-edge M3 fix needs the scope's braces for Rule R, which only the resolver has. The recognizers still obey the fuel and consume-or-end rule.
- **M1 converges in one round, not two**, because the arrow fix and the declare fixes do not overlap.
- **M5 closing insert** is `}\n` when the source ends with a newline (the plan says newline + `}` + newline). Same text for a source with no trailing newline.
- **Grammar card:** the plan's "Do NOT write" list had no chain line; I left `a -> b -> c` off the card because it is `diagram/chained-edge`, not `mermaid-syntax`, and the dogfood test requires `mermaid-syntax`. `&` fan-out is on the card instead. `accTitle` / `accDescr` are styling heads even when followed by `:` (they always are in Mermaid); for the other heads a following `:` keeps the word an id.
- **Mermaid wording kept out of scope:** I did not read or depend on any validator outside this crate.

## Known limitations (not stubs)

- Mermaid subgraph nodes are global; VoxDiagram container nodes are scoped. An edge from outside a subgraph to a node declared inside it becomes `undeclared-node` whose existing Rule R fix declares a ghost node in the referencing scope (valid output, plus a `shadowed-id` warning, but not the author's intent). Resolving it needs the importer (Phase 84), not a fix.
- `direction LR` inside a subgraph is rewritten to `direction: right` and then reported as `direction-in-container` (no fix); `graph TD` plus a body `direction` ends as `duplicate-direction`. Both are honest error states, not silent loss.
- Fan-out and chains expand to at most 100 edges per statement (`ponytail:` comment in `parser/edges.rs`); past that the diagnostic has no fix.

## Known Stubs

None.

## Threat Flags

None. The new scanning surface (link labels, brackets, styling lines) is the surface T-83-23 to T-83-25 describe and is covered by the hostile and adversarial suites; no network, auth or file access was added.

## Self-Check: PASSED

- Created files exist: `src/lexer/mermaid.rs`, `src/parser/{mermaid,edges,frame}.rs`, `src/resolve/mermaid.rs`, `grammar-card.txt`, `tests/{mermaid_catalog,grammar_card,repair_props}.rs`, `tests/support/mermaid.rs`, 6 fixtures, 6 snapshots (verified with `ls`).
- Commits `e29441480`, `b3639b5dc`, `9b3440310` are ancestors of HEAD (`git merge-base --is-ancestor`).
- `commits: 3` was measured with `git rev-list --count 30082f0f2..HEAD` before this summary commit.
