---
phase: 84-mermaid-emitter-strict-importer
fixed_at: 2026-10-16T00:00:00Z
review_path: .planning/workstreams/voxdiagram/phases/84-mermaid-emitter-strict-importer/84-REVIEW.md
iteration: 3
findings_in_scope: 8
fixed: 8
skipped: 0
status: all_fixed
---

# Phase 84: Code Review Fix Report

**Fixed at:** 2026-10-16
**Source review:** .planning/workstreams/voxdiagram/phases/84-mermaid-emitter-strict-importer/84-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 8 (3 critical, 5 warning; the 3 Info findings are out of `critical_warning` scope)
- Fixed: 8
- Skipped: 0

Verification ran in the main checkout (`workflow.use_worktrees=false`, no temp worktree), so the numbers are reproducible from this tree:
`INSTA_UPDATE=no cargo nextest run -p vox-diagram` 322 passed / 1 skipped, `cargo clippy -p vox-diagram --all-targets -- -D warnings` clean, `cargo fmt -p vox-diagram -- --check` clean, `node --test tests/unit/mermaid-oracle.test.mjs tests/unit/workflow-paths.test.mjs` 79 pass / 0 fail. No `lib.rs` pub item changed (only `pub(crate)` / `pub(super)`), so the generated API page needed no regeneration. No `Cargo.toml`, `Cargo.lock` or `package.json` change. No proptest-regressions file was committed.

## Fixed Issues

### CR-01: Exporter emits labels containing `direction <TB|TD|LR|RL|BT>`

**Files modified:** `crates/vox-diagram/src/mermaid/escape.rs`, `crates/vox-diagram/tests/support/hostile.rs`, `crates/vox-diagram/tests/support/mod.rs`, `crates/vox-diagram/tests/support/direction.rs` (new), `crates/vox-diagram/tests/fixtures/export-hostile/g16-direction-text.{export.mmd,sets.json}` (new)
**Commit:** 7a7bb37d0
**Applied fix:** Shared predicate `direction_traps` (case-insensitive, JS-style whitespace incl. U+FEFF) in `escape.rs`; `encode` numerically encodes the leading `d` of each hit for flowchart text only (sequence text untouched). Test first: new hostile fixture `g16` (node, edge and container labels plus near misses) was blessed pre-fix and the Node oracle failed on it; after the fix the oracle passes (71/71). Independent restatement of the trap added to `payload_problem` (new `support/direction.rs`, kept out of `hostile.rs` to stay under 400 non-blank lines) and two `direction` entries added to `FRAGMENTS` so the proptests see it. Golden diff reviewed: only the new g16 files.

### CR-03: Importer accepts `direction XX` inside flowchart and state text

**Files modified:** `crates/vox-diagram/src/mermaid/escape.rs`, `crates/vox-diagram/src/mermaid/import/text.rs`, `crates/vox-diagram/tests/mermaid_import_review.rs` (new)
**Commit:** 06bc36ed1
**Applied fix:** `decode` refuses text where `has_direction_trap` (same predicate as CR-01, so exporter and importer cannot drift) hits, as `direction-in-text`. Sequence text (`free(.., wrap=true)`) is exempt because Mermaid's sequence lexer has no such statement and the exporter does not encode it there. Entity-encoded `#100;irection LR` still imports as text. Tests failed before the fix. Requires human verification as a logic change (the choice to exempt sequence text is mine; reviewer text said "all callers" but also said sequence is unaffected).

### CR-02: Importer accepts a raw `#` in sequence text

**Files modified:** `crates/vox-diagram/src/mermaid/import/text.rs`, `crates/vox-diagram/tests/mermaid_import_review.rs`
**Commit:** aa577c4b1
**Applied fix:** New `stray_hash` (a `#` that opens no complete entity); `free` refuses it as `hash` when `wrap` (sequence callers). State text still tolerates a bare `#`. Not applied: the optional whole-line check in `seq::scan` ("for symmetry"), because ids cannot contain `#` and `free` already covers every accepted text position. Requires human verification (logic).

### WR-01: `entity()` recognises only all-letter or all-digit `#...;`

**Files modified:** `crates/vox-diagram/src/mermaid/import/text.rs`, `crates/vox-diagram/tests/mermaid_import_review.rs`
**Commit:** bdc8a6563
**Applied fix:** `entity` takes the maximal `[A-Za-z0-9_]+` run before `;`: all digits parse as a code point, a known name decodes, anything else is `Err("entity")` (fail closed). A `#` without such a run plus `;` stays literal. Because `semicolon`/`stray_hash` share `entity`, `;` after a bad entity is now reported as `entity`, not `semicolon`. Requires human verification (logic).

### WR-02: State import with `drop_styling` deletes whole transitions containing `:::`

**Files modified:** `crates/vox-diagram/src/mermaid/import/state.rs`, `crates/vox-diagram/tests/mermaid_import_review.rs`
**Commit:** 42f9ec4b5
**Applied fix:** Under `drop_styling` the `:::class` suffix is stripped from each endpoint and the transition kept, with `styling:shorthand` in `dropped`. Default mode still refuses the line first, exactly as before. A malformed suffix and a `:::` inside label text are still refused (probed: the pinned Mermaid rejects `A --> B: see x:::y` itself, so there is nothing to drop). Requires human verification (logic).

### WR-03: Valid in-limit blocks refused as `internal` at line 1

**Files modified:** `crates/vox-diagram/src/mermaid/import/text.rs`, `crates/vox-diagram/src/mermaid/import/mod.rs`, `crates/vox-diagram/tests/mermaid_import_review.rs`
**Commit:** 66d266c24
**Applied fix:** `valid_id` refuses ids over 200 chars as `id-too-long` at the offending line (flowchart, state and sequence); `finish` reports `limit-source` when the generated DSL exceeds `MAX_SOURCE_BYTES`, keeping `internal` only for genuine other failures. Tests (250-char id; 500 nodes / 990 edges in 10 chain lines, under 64 KiB as Mermaid text) failed with `internal` before. The two cases live in `mermaid_import_review.rs` rather than `mermaid_import_limits.rs`.

### WR-04: Importer accepts text that real Mermaid rejects

**Files modified:** `crates/vox-diagram/src/mermaid/import/text.rs`, `.../import/lines.rs`, `.../import/seq.rs`, `.../import/state.rs`, `crates/vox-diagram/tests/mermaid_import_review.rs`, `crates/vox-diagram/tests/mermaid_import.rs`, `crates/vox-diagram/tests/mermaid_import_state.rs`
**Commit:** 6f694feb9
**Applied fix:** Rather than copy the reviewer's lists, I probed the pinned parser for each word and position and encoded exactly what it rejects.
- Sequence: case-insensitive compare, also on the part before a `-` (`loop-x`), and added `note`, `over`, `link`, `links`, `properties`, `details`.
- Flowchart: new `flow_id` rejecting `end`, `style`, `class`, `graph`, `flowchart`, `subgraph`, `interpolate`, `classDef`, `linkStyle` (exact case, alone or before `-`). `click`, `href`, `call` are deliberately left accepted: Mermaid rejects them only as an edge target, so refusing everywhere would refuse valid sources (documented ceiling in a code comment).
- State: new `state_id` rejecting any `-` and `note/style/class/click/default/state/href/classdef/scale` in any case; `A --> B:` (empty label) is refused as `empty-label`.
Two older tests asserted the opposite of the oracle and were updated: `import_styling_head_words_can_still_be_ids` (`class --> b`, `style --> b` are rejected by Mermaid) and `state_header_forms_and_text` (`A --> B:` is a Mermaid parse error). Requires human verification (logic; the `A --> B: ` trailing-space form, which Mermaid accepts, is also refused because lines are trimmed before scanning, so this errs fail-closed).

### WR-05: The Node oracle suite silently skips whole fixture families

**Files modified:** `docs-astro/tests/unit/mermaid-oracle.test.mjs`
**Commit:** ce6c1e831
**Applied fix:** `family()` takes a minimum count and registers an "exists with at least N fixtures" test that fails when the directory is missing or thin; the corpus block gets the same existence test (>= 31 round-tripped, >= 33 frozen blocks). Minimums: export-hostile 21, import-cases-roundtrip 10. Reproduced before: with `export-hostile` removed the suite was green at 50 passes; after: each of the three families removed in turn fails exactly one test, and the intact tree passes 74/74.

## Skipped Issues

None. The Info findings (IN-01, IN-02, IN-03) were out of scope for `fix_scope: critical_warning`.

---

_Fixed: 2026-10-16_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

---

# Iteration 3

**Fixed at:** 2026-10-16
**Source review:** the iteration-2 review (7 warnings, WR-01..WR-07; Info findings out of `critical_warning` scope)
**Iteration:** 3

**Summary:**
- Findings in scope: 7
- Fixed: 7
- Skipped: 0
- Status: all_fixed (every logic change below needs human verification)

Verification ran in the main checkout (`workflow.use_worktrees=false`, no temp worktree), so every number reproduces from this tree:
`INSTA_UPDATE=no cargo nextest run -p vox-diagram` 340 passed / 1 skipped (iteration 1: 322 / 1), `cargo clippy -p vox-diagram --all-targets -- -D warnings` clean, `cargo fmt -p vox-diagram -- --check` clean, `node --test tests/unit/mermaid-oracle.test.mjs tests/unit/workflow-paths.test.mjs` 82 pass / 0 fail (was 79), corpus round-trip still 31 of 33 (`corpus_roundtrip`: only the gantt and markdown-string blocks refuse). No `Cargo.toml`, `Cargo.lock` or `package.json` change. No file over 400 non-blank lines (largest: `tests/support/hostile.rs`, 387). No `proptest-regressions` file committed. Hooks ran on every commit (fmt-fix, tdd-guard).

**Method.** The principle was "agree with the pinned Mermaid 11.17.2 parser". Instead of guessing word lists, a temporary differential harness (an untracked test binary plus scratch scripts, all deleted) sent generated blocks through the importer and the oracle and classified each as `ok`, `over` (importer refuses, Mermaid accepts), `BAD-accept` (importer accepts, Mermaid rejects) or `BAD-mismatch` (both accept, different meaning). Probes: about 4,500 flowchart, 3,200 state and 3,500 sequence word-by-position blocks (about 100 words, four case variants, 13 to 18 positions), targeted probes per finding, and 54,000 random statement-mix blocks in three seeded rounds (the first round, 12,000 blocks, found the extra cases listed below; the last two rounds, 42,000 blocks, one hostile-word pool and one benign pool, show zero divergence). The final state shows zero `BAD-*` in all of them. Exporter: about 45,500 ids (every 1 to 3 character id plus the keyword list, each as a node and as a subgraph) through export and the oracle, zero failures after the fix.

## Fixed Issues

### WR-01: `drop_styling` regression: a state line that only declares a class

**Files modified:** `crates/vox-diagram/src/mermaid/import/state.rs`, `crates/vox-diagram/tests/mermaid_import_review.rs`
**Commit:** da472c0af
**Applied fix:** New `Item { Edge, State }` and `declaration()`. Under `drop_styling`, `Run:::bad` (and `Solo :::c`) declares the state and records `styling:shorthand` in `dropped`. The oracle lists `Run` as a node, so a node is created. `state Run:::bad` declares nothing the oracle lists, so only the line is dropped (the pre-regression result, which matches). `Run:::bad : desc` is refused as `state:description` (the right name); a malformed class, `[*]:::c`, `end:::c`, `a-b:::c` are refused. Default mode is unchanged. Tests failed before the fix. Requires human verification (logic).

### WR-02: Reserved-word lists refused valid Mermaid by position

**Files modified:** `crates/vox-diagram/src/mermaid/import/ids.rs` (new; the id rules left `text.rs`), `.../import/lines.rs`, `.../import/seq.rs`, `.../import/state.rs`, `.../import/text.rs`, `.../import/mod.rs`, `crates/vox-diagram/tests/mermaid_import_words.rs` (new), `tests/mermaid_import_review.rs`, `tests/mermaid_import.rs`
**Commit:** 2b7fadc13
**Applied fix:** Position-aware rules, every list probed per word and position (see Method).
- Subgraph ids: `style`, `class`, `classDef`, `linkStyle`, `flowchart`, `subgraph` are legal; refused are `interpolate`, `default` (also before a `-`), `_self`, `_blank`, `_parent`, `_top`, and `click`, `href`, `call`. `end` is legal only with a title (a bare `subgraph end` parses in Mermaid but is weird, so it stays refused).
- Sequence: `participant <any word>` is legal (the oracle accepts all of them). `title` is a keyword only as a receiver or when followed by a space (`Title->>B` imports), `accTitle` / `accDescr` only as a receiver or note actor. Block and statement keywords stay refused as sender, receiver and note actor, alone or before a `-` (`loop-x`).
- State: `A --> Note: hi` (and `Style`, `Class`, `State`, `Scale`, `classDef`) imports; the same words are refused as a source or a bare target. `click`, `href`, `default` are refused everywhere; `accTitle` / `accDescr` only before a label.
- The `A --> B: ` (colon, then one space) case stays refused as `empty-label`: lines are trimmed before scanning, so it cannot be told from the genuine error `A --> B:`. Documented in the module doc and pinned by a test as a known limit (the review accepted "document and test it").
Tests: 8 of 9 new tests failed before the fix. Requires human verification (logic).

### WR-03: `click --> b` still fabricated content; `href`, `call`, link targets, `accTitle`, `accDescr` accepted

**Files modified:** same commit and files as WR-02
**Commit:** 2b7fadc13
**Applied fix:** Flowchart node references refuse `end`, `style`, `class`, `graph`, `flowchart`, `subgraph`, `interpolate`, `classDef`, `linkStyle`, `_self`, `_blank`, `_parent`, `_top` (exact case, alone or before a `-`), and `click`, `href`, `call` when bare. `click["x"] --> b`, `click(x) --> b`, `click-x` still import (the oracle agrees). The inverted test `import_styling_head_words_can_still_be_ids` is reverted to what the oracle says (`click --> b` and bare `click` refused, `Class --> b` imports) and renamed. Sequence `accTitle` / `accDescr` as receiver or note actor (any case) are refused. `root_start` / `root_end` (exact case, `IN-05`) are refused in state diagrams. Requires human verification (logic).

### WR-04: Cross-line `direction` trap

**Files modified:** `crates/vox-diagram/src/mermaid/escape.rs`, `.../import/mod.rs`, `.../import/state.rs`, `crates/vox-diagram/tests/mermaid_import_traps.rs` (new)
**Commit:** 40f3d59d9
**Applied fix:** `direction_spans_lines(line, next)` (shares the word list and whitespace rule with the existing predicate): a line ending in `direction` and a next non-blank line starting with `tb|td|lr|rl|bt` (any case) is refused as `direction-in-text` at the first line. Applied to state (the reviewer's case) and to flowcharts, where the oracle showed the same mismatch for an id such as `xdirection` before `LR --> B`. A real `direction LR` inside a subgraph is untouched. Requires human verification (logic).

### WR-05: Flowchart `&` without surrounding spaces

**Files modified:** `crates/vox-diagram/src/mermaid/import/lines.rs`, `crates/vox-diagram/tests/mermaid_import_traps.rs`
**Commit:** 40f3d59d9
**Applied fix:** `group()` requires whitespace before and after `&`; anything else is refused as `statement`. All 11 reviewer and probe variants (`a-->b&c`, `a&b-->c`, `b[x]&c`, ...) are refused; `a & b --> c` still imports. Requires human verification (logic).

### WR-06: Dash edge text containing `--`

**Files modified:** `crates/vox-diagram/src/mermaid/import/lines.rs`, `crates/vox-diagram/tests/mermaid_import_traps.rs`
**Commit:** 40f3d59d9
**Applied fix:** Unquoted `-- text -->` text that contains `--`, ends in `-` (`a--->b`), or is blank is refused as `edge-label`. Probing found the same class: zero-length quoted dash text, `|""|` and `||` are Mermaid parse errors too, so they are refused. Single dashes, quoted text and `|" "|` still import. Requires human verification (logic).

### WR-07: Exporter emitted ids Mermaid rejects

**Files modified:** `crates/vox-diagram/src/mermaid/escape.rs`, `crates/vox-diagram/tests/support/hostile.rs`, `crates/vox-diagram/tests/mermaid_export_props.rs`, `crates/vox-diagram/tests/fixtures/export-hostile/g01-keyword-ids.{export.mmd,sets.json}`
**Commit:** c5e4324e6
**Applied fix:** `KEYWORDS` gains `_self`, `_blank`, `_parent`, `_top`. The sweep (about 45,500 ids) found exactly those four, plus one new exporter bug: an id ending in `direction` (`adirection`) at the end of an edge line followed by a line starting with an uppercase direction word (`LR1 --> ...`) is one `direction LR` statement to the flowchart lexer and silently dropped an edge. `keeps_id` now also renames ids that end in `direction`. Test first: `g01-keyword-ids` extended (the four link targets as nodes and as a subgraph, plus `adirection` and `LR1` placed on adjacent edge lines) and blessed before the fix; the Node oracle failed on it (73 of 74), then passed after the fix. The proptest keyword list and pool were extended and failed before the fix (`keyword id "_self"`).

## Beyond the seven findings (found while verifying; same class, same principle)

**Commit:** 3231baf0a (fixture and Node minimum: 3c8a0ee6f)
- Flowchart: more than one whitespace character after a closing pipe label (`a -->|t|  b`) is a Mermaid error; a lone `o` / `x` id fused to an arrow (`o-->b`) starts a circle/cross link. Both are refused (`edge-label`, `invalid-id`). The `o`/`x` rule over-refuses `o-->c` as a later statement, which Mermaid accepts (fail closed).
- Sequence: `Title ->> A` (title, a space, anything) is a title statement in any case; refused as `title`. Sequence ids containing `-x` (the cross arrow) are refused when used (a `participant a-x` declaration is fine).
- The test canon (`canon_sequence`) now collapses `Note over A,A` to one actor, as the oracle does; the importer still produces both ids. Pinned by the oracle fixture.
- Three new `import-cases` (flowchart, state, sequence words) judged three-way by the Node oracle; the family minimum is 13. Module doc lists the deliberate over-refusals (also covers `IN-04`, the case-insensitive `direction` superset).

## Skipped Issues

None. No finding needed the fail-closed fallback; each was fixed to the oracle's exact behaviour, with the one documented limit above (`A --> B: ` with a trailing space).

## Known remaining differences (all fail closed)

DSL-reserved words (`graph`, `node`, `direction`, ...) and `start` / `end` as state ids still refuse where Mermaid accepts them; `state --> A` stays refused (Mermaid parses it as something else); `subgraph click["x"]` (no space before the bracket) is refused though Mermaid accepts it; `o-->c` as a non-first statement is refused.

---

_Fixed: 2026-10-16_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 3_
