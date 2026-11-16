---
phase: 84-mermaid-emitter-strict-importer
plan: 04
subsystem: diagram-mermaid
tags: [rust, mermaid, import, stateDiagram, sequenceDiagram, fail-closed, proptest, mutation-proof]
status: complete

requires:
  - phase: 84-03
    provides: "import dispatch, flow::Graph + to_dsl, text::{valid_id,decode}, lines::{id_run,styling_head}, catalogue/props/limits suites"
provides:
  - "mermaid::import for stateDiagram-v2: [*] as source/target becomes the start/end terminals, transitions become labelled edges, printed through flow::to_dsl"
  - "mermaid::import for sequenceDiagram into ImportReport.sequence (source stays None, Ir untouched), spans from span::LineIndex"
  - "text::free / text::semicolon: free-text decoding without the unquoted-label ban"
  - "Corpus: 31 of 33 blocks import; only gantt and markdown-string are refused (ROADMAP SC3)"
affects: [84-05, 86, 90]

actuals:
  tokens: 11871   # chars/4 over `git diff plan_head_before..plan_head_after -- crates` (47,483 chars)
  tasks: 2
  commits: 2
plan_head_before: 77b4ce8d00df00f5822ddbaecbe9f94507c2bd36
plan_head_after: 3b674a5b27ca06a22bdfa9228925914a4c748555

key-files:
  created:
    - crates/vox-diagram/src/mermaid/import/state.rs
    - crates/vox-diagram/src/mermaid/import/seq.rs
    - crates/vox-diagram/tests/mermaid_import_state.rs
    - crates/vox-diagram/tests/mermaid_import_seq.rs
  modified:
    - crates/vox-diagram/src/mermaid/import/mod.rs
    - crates/vox-diagram/src/mermaid/import/text.rs
    - crates/vox-diagram/src/mermaid/import/lines.rs
    - crates/vox-diagram/tests/mermaid_corpus.rs
    - crates/vox-diagram/tests/mermaid_import_props.rs
    - crates/vox-diagram/tests/mermaid_import_limits.rs

key-decisions:
  - "A raw `;` check that is entity-aware (`#59;` is text, any other `;` is `semicolon`) lives in text::semicolon and is shared by state and sequence scanners"
  - "Because every HTML entity carries a raw `;`, `R&amp;D` in state/sequence text reports `semicolon` (flowchart labels still report `html-entity`); both fail closed"
  - "Participant and node maps use BTreeMap: determinism_guard forbids hash collections in src/"
  - "Sequence ids that are Mermaid keywords (loop, end, actor, title, ...) are `reserved-id`, since Mermaid cannot parse them as participants"
  - "Empty state transition text (`A --> B:`) imports as no label (an empty label equals no label)"
  - "MIG-01 is NOT ticked: 84-05 also lists it and owns the three-way oracle verification of the imported blocks"

requirements-completed: []  # MIG-01 functionally delivered for all four kinds; tick left to 84-05

duration: ~45min
completed: 2026-10-16
---

# Phase 84 Plan 04: stateDiagram-v2 and sequenceDiagram Importers Summary

**`mermaid::import` now reads all four block kinds: stateDiagram-v2 folds into the graph DSL (`[*]` becomes `start`/`end` terminals, self-loops kept) and sequenceDiagram imports into `SequenceIr` with located spans; everything outside the accepted subset fails the block closed with a by-line list, and 31 of the 33 frozen corpus blocks import.**

## Accomplishments

- **Task 1 (tracer), `53e4c71c3`.** `import/state.rs` scans one line to a transition or a construct name, creates nodes at first mention (terminals included, so `end` is declared where `Leased --> [*]` first appears), builds a `flow::Graph` and prints through `flow::to_dsl` + `fmt`. `import/mod.rs` was refactored so `flowchart()` and `state()` share `finish()` for report assembly. `text::free` added. The populi.md line 211 block imports, is a `fmt` fixed point and `check()`s valid.
- **Task 2, `3b674a5b2`.** `import/seq.rs`: `participant`, `participant X as Label`, `->>`, `-->>`, `Note over X[,Y]` (any case of `note`), implicit participants in first-mention order, `participant-after-use`, `redeclared-participant`, caps, spans as `Location` over the trimmed statement byte range. Full catalogues for both kinds, `drop_styling` in both, property extension (80 fragments, `stateDiagram-v2` and `sequenceDiagram` headers), adversarial bounds. The scientia line 112 block imports with 8 actors, 8 messages and 1 note (the plan's "9 messages" counted the note; asserted exactly as `(8, 8, 1)`).

## TDD evidence (RED observed before GREEN)

| Task | RED observed | GREEN |
|------|--------------|-------|
| 1 | `mermaid_import_state` 2 tests and `corpus_blocks_import_or_fail_closed_as_expected` FAILED (`header:stateDiagram-v2` instead of an import / `id-clash`) before any implementation | 67 tests across state, corpus, import, roundtrip, unsupported, `mermaid::import::` pass; the inline `free_text_rules` test failed once on a real bug (entity terminator `;` was treated as a raw `;`) and was fixed with the entity-aware `text::semicolon` |
| 2 | All 9 `mermaid_import_seq` tests and the corpus sequence assertion FAILED against Task 1 code (sequence header unsupported); `state_both_terminals_and_caps` failed on a wrong expected line number in my test (1003 vs 1002), fixed in the test | 88 targeted tests, then the full crate: 303 pass |

Honest gaps: the state catalogue test (`state_unsupported_catalogue`) and `state_header_forms_and_text` passed on their first run because Task 1's scanner already implemented the whole state catalogue; their RED is only the pre-implementation state of Task 1. `mermaid_import_props` / `mermaid_import_limits` additions were green on first run (structure already satisfied them).

## Mutation proof (d): break, confirm named failures, restore with presence checks

Mutation: in `seq.rs::keyword`, `w if BLOCKS.contains(&w) =>` became `w if BLOCKS.contains(&w) && w != "loop" =>` (a `loop` line stops being refused as `block:loop`).

| Check | Result |
|-------|--------|
| `grep -c 'w != "loop"' seq.rs` before / mutated / restored | 0 / 1 / 0 |
| Failing tests under the mutation | `seq_unsupported_catalogue`, `seq_reports_every_bad_line_and_dedups` (2 of 9) |
| Restore | copied back from a saved file, `diff -q` identical, `mermaid_import_seq` 9/9 green again |

Also run once, exploratory: both totality properties at 20,000 cases (restored to `with_cases(256)`, count verified): no counterexample, no `proptest-regressions` file created.

## Verification

| Check | Result |
|-------|--------|
| Plan verify set (`mermaid_import_seq`, `_state`, `_props`, `mermaid_corpus`, `mermaid_unsupported`, `mermaid_import`, `mermaid_roundtrip`, `mermaid::import::`) with `--retries 0` | 88 passed |
| `INSTA_UPDATE=no cargo nextest run -p vox-diagram` | 303 passed, 1 skipped (ignored freezer) |
| `cargo clippy -p vox-diagram --all-targets -- -D warnings` | clean |
| `cargo fmt -p vox-diagram -- --check` | clean |
| `node --test tests/unit/mermaid-oracle.test.mjs` (docs-astro) | 28 pass, 0 fail |
| Corpus | 31 of 33 import; exactly 2 expected-unsupported (gantt `header:gantt`, markdown-string); populi state block and scientia sequence block import |
| Construct-name coverage (`grep -c` of the quoted name) | sequence: actor 2, lifecycle 2, activation 4, `arrow:->` 2 (all 8 other-arrow tokens asserted individually), autonumber 2, title 1, box 1, links 4, note-placement 2, note-actors 1, block:loop 1 (all 11 block keywords asserted via a generated list), styling:accTitle 2, styling:accDescr 2, comment 2, directive 1, statement 3, message-syntax 2, participant-after-use 2, redeclared-participant 1, wrap 4, semicolon 3, limit-nodes 1, limit-edges 1, empty-label 1. State: state:alias 1, state:description 1, state:composite 2, state:pseudo 3, state:note 2, state:concurrency 1, direction 1, styling:classDef 2, styling:shorthand 1, id-clash 1 |
| Non-blank lines per file | `seq.rs` 308, `state.rs` 185, `text.rs` 225, `mod.rs` 163, `lines.rs` 322; tests: `mermaid_import_seq.rs` 275, `mermaid_import_state.rs` 160, `mermaid_import_props.rs` 174, `mermaid_corpus.rs` 190; `mermaid_unsupported.rs` untouched |
| T-84-SC: Cargo.toml / Cargo.lock in `plan_head_before..plan_head_after` | none |

## Deviations from Plan

1. **[Judgement] `R&amp;D` in state/sequence text reports `semicolon`, not `html-entity`.** An HTML entity always contains a raw `;`, which ends a Mermaid statement, so `semicolon` is reported first. Both are fail-closed; flowchart labels (where `;` is legal inside brackets) still report `html-entity`. The `html-entity` branch of `decode` is simply unreachable from free text.
2. **[Rule 1 - Bug] Entity-aware semicolon check.** The plan's "a raw `;` is `semicolon`" would have rejected the exporter's own `#59;` encoding. `text::semicolon` skips `;` that terminate `#name;` / `#digits;` entities, so the exporter's encoded forms import back as text (test `seq_text_rules`: `a #59; b #119;rap: #96;x#96;` decodes to ``a ; b wrap: `x` ``).
3. **[Judgement] Keyword ids are `reserved-id` in sequences** (`loop`, `end`, `actor`, `title`, ...), an addition beyond the plan's list, because Mermaid cannot parse them as participants and the exporter never emits them.
4. **[Judgement] Unspaced `-x` is not an arrow.** `A-xB: hi` is lexed as the hyphenated id `A-xB` (the DSL id rule allows inner hyphens), then `statement`. Spaced `A -x B` reports `arrow:-x`. Fail-closed either way.
5. **[Test count] Scientia block: 8 messages plus 1 note, not 9 messages**; asserted as `(8, 8, 1)` (actors, messages, notes).
6. **[Judgement] `determinism_guard` caught `HashMap`**; both new modules use `BTreeMap`.

## Known limitations (documented, not stubs)

- A lone CR (not part of CRLF) is a line break for `span::LineIndex` but not for `str::lines`; a block containing one only inside whitespace-trimmed regions would get sequence `Location::line` values one off from the report's line numbers. Any lone CR inside a statement is already refused (`forbidden-char` / invalid id).
- Only the first problem on a line is reported (one entry per line), plus the semantic entries (`participant-after-use`, `redeclared-participant`, limits).
- State `A --> B --> C`, `A --- B` and similar are `invalid-id` / `statement`, not `edge-type:*` (the state subset has no edge-type catalogue in the plan).

## Deferred Issues

None new. The 84-01 deferral (`tutorial-record.test.mjs` blob mismatch) is unrelated and unchanged.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. T-84-15 (silent loss): closed subset, every other keyword/arrow a by-line entry, `participant-after-use` / `redeclared-participant`, mutation proof (d), property extended to both headers. T-84-16 (DoS): single pass per line, `limit-nodes` / `limit-edges` reported once, new `adversarial_state_and_sequence_blocks` (4,000 to 11,000 line blocks plus an entity/bracket soup message of about 60 KiB, all within 10 s). T-84-17 (spoofing): free text keeps `html-label`, `html-entity` (via the flowchart path), `entity`, `forbidden-char`, `label-too-long`, `markdown-string`, plus sequence `wrap`. T-84-SC: no manifest or lockfile change.

## Self-Check: PASSED

- Files exist: `src/mermaid/import/{state,seq}.rs`, `tests/{mermaid_import_state,mermaid_import_seq}.rs`; `mermaid_corpus.rs`, `mermaid_import_props.rs`, `mermaid_import_limits.rs` extended, not rewritten.
- Commits `53e4c71c3` and `3b674a5b2` are ancestors of HEAD (measured: `git rev-list --count plan_head_before..HEAD` = 2 at SUMMARY write).
