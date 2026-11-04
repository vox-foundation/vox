---
phase: 84-mermaid-emitter-strict-importer
plan: 03
subsystem: diagram-mermaid
tags: [rust, mermaid, import, fail-closed, subgraph, proptest, mutation-proof]
status: complete

requires:
  - phase: 84-01
    provides: "ImportOpts/ImportReport/Unsupported, tracer import, frozen 33-fence corpus + MANIFEST, escape.rs"
provides:
  - "mermaid::import for graph/flowchart: the documented subset imports to canonical DSL, everything else fails the whole block closed with a by-line `unsupported` list"
  - "Scope rewrite: Mermaid's global subgraph ids become scoped dotted paths (first-closed membership, deepest-common-scope edge placement); closes the 83-07 cross-scope gap"
  - "import/{text,lines,flow,build}.rs split; `flow::Graph` + `flow::to_dsl` are reusable by the state importer (84-04), `text::{valid_id,decode}` by state and sequence"
  - "Catalogue, caps, adversarial and totality suites; corpus import test (29 of 30 flowchart blocks import)"
affects: [84-04, 84-05, 87, 88, 90]

actuals:
  tokens: 17350   # chars/4 over `git diff plan_head_before..plan_head_after -- crates` (69,413 chars)
  tasks: 3
  commits: 3
plan_head_before: 13e2e340946ee4dfd22bd8224c61aac19ce10296
plan_head_after: 0219f643977cbd94b88e5043174e07c2aac669cb

key-files:
  created:
    - crates/vox-diagram/src/mermaid/import/text.rs
    - crates/vox-diagram/src/mermaid/import/lines.rs
    - crates/vox-diagram/src/mermaid/import/flow.rs
    - crates/vox-diagram/src/mermaid/import/build.rs
    - crates/vox-diagram/tests/mermaid_import.rs
    - crates/vox-diagram/tests/mermaid_unsupported.rs
    - crates/vox-diagram/tests/mermaid_import_props.rs
    - crates/vox-diagram/tests/mermaid_import_limits.rs
  modified:
    - crates/vox-diagram/src/mermaid/import/mod.rs
    - crates/vox-diagram/tests/mermaid_corpus.rs

key-decisions:
  - "Semantic pass lives in its own `import/build.rs` (not in the plan's file list) so every file stays under 400 non-blank lines"
  - "A refused `subgraph` header still opens a frame (`Stmt::BadOpen`), so its `end` does not become a spurious `stray-end`; an unclosed refused header also reports `unclosed-subgraph`"
  - "Unquoted edge text has its own special-character set per form: `-->|t|` rejects `( ) [ ] { } | \" @`, `-- t -->` rejects only `\"` (measured with the pinned Mermaid; corpus block 11 has `-- Yes (Iter < Max) -->`)"
  - "`#<digits>;` that is not a valid scalar value (surrogate, > U+10FFFF) is `entity`, not literal text: a literal would render differently from Mermaid's own decoding"
  - "Limit constructs (`limit-nodes`, `limit-edges`, `limit-depth`) are reported once, on the crossing line, not on every later line"
  - "MIG-01 is NOT ticked: it covers flowchart, state and sequence; only flowchart/graph is delivered here"

requirements-completed: []  # MIG-01 stays open until 84-04 (state + sequence)

duration: ~50min
completed: 2026-10-16
---

# Phase 84 Plan 03: Strict Flowchart Importer Summary

**`mermaid::import` now reads exactly the documented flowchart/graph subset, rewrites Mermaid's global subgraph ids into scoped dotted paths (`client -> backend.gateway`), and fails the whole block closed with an exact by-line `unsupported` list for anything else; 29 of the 30 frozen flowchart blocks import and the 30th (markdown-string) is rejected.**

## Accomplishments

- **Task 1 (tracer), `cebc0a6ea`.** Four-module split. `lines.rs` scans one trimmed line into a statement or a construct name; `text.rs` owns the DSL id rule and label decoding; `build.rs` does the semantic pass (first-closed membership via a `Vec<Frame>` stack, never recursion); `flow.rs` prints scoped DSL (`to_dsl`), placing each edge in its deepest common scope with endpoints relative to it. `import_cross_scope_edge_uses_dotted_path` matches the plan's exact DSL and the IR edge is `client` to `backend.gateway`.
- **Task 2, `50619bd10`.** All six shapes, `&` fan-out, chains, the four edge-text forms (`-->|t|`, `-->|"t"|`, `-- t -->`, `-- "t" -->`), entity and `<br>` decoding, quoted-text-is-text, subgraph forms, `direction` handling. 15 `import_*` tests. `corpus_blocks_import_or_fail_closed_as_expected`: 29 blocks import (fmt fixed point, `check().valid`); exactly 4 fail closed (gantt `header:gantt`, how-to-rust-crate-imports line 53 only `markdown-string`, populi `header:stateDiagram-v2`, scientia line 112 `header:sequenceDiagram`); block 06 (deep-research line 238) reports two `subgraph-direction` in `dropped`; no other block has any `dropped`.
- **Task 3, `0219f6439`.** Full catalogue (31 named tests in `mermaid_unsupported.rs`), `drop_styling`, caps, 6 adversarial blocks, 2 totality properties. Fan-out is counted before it is materialised (running product, early exit at 100).

## TDD evidence (RED observed before GREEN)

| Task | RED observed | GREEN |
|------|--------------|-------|
| 1 | First run: compile error in my own test helper (partial move). After fixing it: `mermaid_import` 3 tests, 2 FAILED (`import_cross_scope_edge_uses_dotted_path`, `import_node_first_used_outside_then_inside`) against the 84-01 tracer importer | 19 tests across `mermaid_import`, `mermaid_roundtrip`, `mermaid::import::` pass; `import-cases-roundtrip/00-minimal` golden untouched (`git status` empty) |
| 2 | 17 tests run, 7 FAILED (`import_unquoted_labels`, `import_edge_text_forms`, `import_all_six_shapes`, `import_fan_out_and_chains`, `import_text_inside_quotes_is_not_a_construct`, `import_subgraph_direction_equal_is_dropped`, `corpus_blocks_import_or_fail_closed_as_expected`); the other 10 already passed on the Task 1 slice | 33 pass |
| 3 | 31 tests in `mermaid_unsupported`, 10 FAILED (`unsupported_lists_every_line`, `unsupported_shapes`, `unsupported_styling_heads`, `unsupported_subgraph_title`, `unsupported_comments_directives_semicolons`, `drop_styling_*` x2, `expansion_cap_*`, `unsupported_edge_types`, `unsupported_ids`) | 72 pass; `unsupported_subgraph_title` failed once more on a test bug (missing `end` after a refused header), fixed in the test |

Honest gaps: Task 1's `text.rs` (entity/br/html rules) was written together with its inline unit tests in one edit, so those unit tests never had a separate RED. The `mermaid_import_props` and `mermaid_import_limits` suites were green on first run (the structure already satisfied them); they are covered by mutation proof (c) only indirectly. I ran the totality properties at 30,000 cases per property once (exploratory, file restored to `with_cases(256)`): no counterexample, no `proptest-regressions` file created or committed.

## Mutation proof (c): break, confirm named failures, restore with presence checks

Mutation: in `lines.rs::statement`, `if !cx.drop_styling {` became `if !cx.drop_styling && h != "classDef" {` (a `classDef` line is silently dropped and the block imports).

| Check | Result |
|-------|--------|
| `grep -c 'h != "classDef"' lines.rs` before / mutated / restored | 0 / 1 / 0 |
| Failing tests under the mutation | `unsupported_styling_heads`, `unsupported_lists_every_line`, `drop_styling_drops_and_lists` (3 of 31) |
| Restore | copied back from a saved file; `git diff --quiet -- crates/vox-diagram/src` clean against the index; `mermaid_unsupported` 31/31 green again |

## Verification

| Check | Result |
|-------|--------|
| Plan verify (`mermaid_import`, `mermaid_unsupported`, `mermaid_import_props`, `mermaid_corpus`, `mermaid_roundtrip`, `mermaid_import_limits`, `mermaid::import::`) with `--retries 0` | 72 passed |
| `INSTA_UPDATE=no cargo nextest run -p vox-diagram` | 286 passed, 1 skipped (the ignored freezer) |
| `cargo clippy -p vox-diagram --all-targets -- -D warnings` | clean |
| `node --test tests/unit/mermaid-oracle.test.mjs` (from `docs-astro`) | 28 pass, 0 fail |
| `mermaid_import` `import_*` tests | 16 (acceptance needs 14) |
| `grep -c 'drop_styling: true' tests/mermaid_unsupported.rs` | 2 |
| `grep -c 'with_cases(256)' tests/mermaid_import_props.rs` | 1 |
| Construct names (every catalogue name except `internal`) in `mermaid_unsupported.rs` | each at least 1 (lowest counts: `direction`, `directive`, `edge-label`, `edge-to-subgraph`, `entity` = 1; the 12 `edge-type:` tokens and 7 `shape:` openers are asserted individually) |
| Non-blank lines per new file | `build.rs` 181, `flow.rs` 169, `lines.rs` 322, `mod.rs` 140, `text.rs` 175; tests: `mermaid_unsupported.rs` 399, `mermaid_import.rs` 229, `mermaid_corpus.rs` 174 |
| T-84-SC: manifest/lockfile files in `plan_head_before..plan_head_after` | none |
| `git status --porcelain crates/vox-diagram/tests/fixtures/import-cases-roundtrip` | empty (84-01 golden not re-blessed) |

## Deviations from Plan

1. **[Judgement] Extra module `import/build.rs`.** The plan names `mod/text/lines/flow`; the semantic pass (node bookkeeping, membership, caps, clashes) would have pushed `flow.rs` past 400 lines, and the plan states the model/printer reuse boundary for 84-04 as `Graph` + `to_dsl`, which `flow.rs` still owns.
2. **[Judgement] New test file `mermaid_import_limits.rs`** for the six adversarial blocks, because `mermaid_unsupported.rs` already sits at 399 non-blank lines. The cap tests (`limit_*`, `expansion_cap_*`) stay in `mermaid_unsupported.rs` so the catalogue greps hold.
3. **[Rule 1 - plan text vs corpus] Unquoted `-- text -->` edge text allows `( ) [ ] { } | @`.** The plan applies one unquoted-text rule (`( ) [ ] { } | " @`) to node labels, titles and edge text, but corpus block 11 contains `-- Yes (Iter < Max) -->` (and the plan requires every non-markdown corpus block to import). Probed with the pinned Mermaid: the dash form rejects only `"`, the pipe form rejects the full set. Implemented per form (`NODE_SPECIAL`, `DASH_SPECIAL`); test `import_edge_text_forms` covers the parenthesised case.
4. **[Judgement] Invalid scalar numeric entities (`#55296;`) are `entity`**, where the plan only says other `#` text stays literal.
5. **[Test updates] 84-01 inline tests adjusted**: `reserved_words_and_chained_edges_are_refused` no longer lists `a --> b --> c` and `a -->|x| b` (they import now; renamed `reserved_words_and_dangling_hyphens_are_refused`), and `every_bad_line_is_listed_with_its_line_number` uses `no pe` instead of `nope` (a bare node is valid). The 84-01 tracer integration tests and golden are unchanged.
6. **Not done by me, by design:** `mod.rs` still has no `stateDiagram-v2` / `sequenceDiagram` arm (plan 84-04); those headers report `header:<word>`.

## Known limitations (documented, not stubs)

- A refused `subgraph` header that is also never closed reports both its construct and `unclosed-subgraph` on the same line.
- `end` and Mermaid-only keywords other than the DSL `RESERVED` list are not rejected as ids (an `end --> b` line imports as node `end`; the exporter never emits it).
- Only the first problem on a line is reported for scan errors (one entry per line); semantic entries (`redeclared-node`, `id-clash`, `edge-to-subgraph`, limits) can add to it, deduplicated by (line, construct).

## Deferred Issues

None new. The 84-01 deferral (`tutorial-record.test.mjs` blob mismatch) is unrelated and unchanged.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. T-84-11 (DoS): size gate first, single pass per line, fan-out product counted with saturating arithmetic before materialising, `Vec` stack (4,000 nested headers: one `limit-depth`, 4,000 `unclosed-subgraph`, no overflow), six bounded adversarial tests. T-84-12 (silent loss): every non-subset line is an entry, redeclaration is refused, `drop_styling` is opt-in and lists every dropped line, mutation proof (c). T-84-13: `html-label`, `html-entity`, `markdown-string`, `entity` on RAW text, single-pass decoding. T-84-14: membership tests (siblings, parent/child, root-first), corpus import. T-84-SC: no manifest change.

## Self-Check: PASSED

- Files exist: `src/mermaid/import/{mod,text,lines,flow,build}.rs`, `tests/{mermaid_import,mermaid_unsupported,mermaid_import_props,mermaid_import_limits}.rs`; `tests/mermaid_corpus.rs` extended, not rewritten.
- Commits `cebc0a6ea`, `50619bd10`, `0219f6439` are ancestors of HEAD (measured: `git rev-list --count plan_head_before..HEAD` = 3).
