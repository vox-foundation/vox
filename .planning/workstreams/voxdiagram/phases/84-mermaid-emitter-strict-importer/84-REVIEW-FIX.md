---
phase: 84-mermaid-emitter-strict-importer
fixed_at: 2026-10-16T00:00:00Z
review_path: .planning/workstreams/voxdiagram/phases/84-mermaid-emitter-strict-importer/84-REVIEW.md
iteration: 1
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
