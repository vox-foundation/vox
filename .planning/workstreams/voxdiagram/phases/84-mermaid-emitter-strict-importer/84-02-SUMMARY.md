---
phase: 84-mermaid-emitter-strict-importer
plan: 02
subsystem: diagram-mermaid
tags: [rust, mermaid, export, subgraph, sequence, proptest, oracle, mutation-proof]
status: complete

requires:
  - phase: 84-01
    provides: "mermaid::export (flat), escape::{label, keeps_id}, roundtrip test support, Node oracle with export-hostile family"
provides:
  - "export: nested subgraphs, one global id namespace, total over hand-built GraphIr (explicit stack, no recursion)"
  - "export_sequence(&SequenceIr) -> String, re-exported from mermaid"
  - "escape::seq_text (`;` and wrap-prefix guards) sharing one encoder with escape::label"
  - "20 hostile fixtures (g00..g15, s00..s03) accepted by the pinned mermaid 11.17.2 parser with equal id-free sets"
  - "Structural proptests over arbitrary GraphIr and SequenceIr; three mutation proofs"
affects: [84-03, 84-04, 84-05, 86]

actuals:
  tokens: 12100   # chars/4 over the diff excluding the export-hostile fixtures
  tasks: 3
  commits: 3
plan_head_before: 13ef6ff41fbc51292d9f22471a8140e8d1afc42a
plan_head_after: a74b9b0d46fb9ea2788fdc051c2f26abf278f74f

key-files:
  created:
    - crates/vox-diagram/src/mermaid/export_seq.rs
    - crates/vox-diagram/tests/mermaid_export.rs
    - crates/vox-diagram/tests/mermaid_export_props.rs
    - crates/vox-diagram/tests/mermaid_export_seq.rs
    - crates/vox-diagram/tests/support/hostile.rs
    - crates/vox-diagram/tests/fixtures/export-hostile/ (20 x .export.mmd + .sets.json)
  modified:
    - crates/vox-diagram/src/mermaid/export.rs
    - crates/vox-diagram/src/mermaid/escape.rs
    - crates/vox-diagram/src/mermaid/mod.rs
    - crates/vox-diagram/tests/support/mod.rs
    - crates/vox-diagram/tests/support/roundtrip.rs

key-decisions:
  - "Ids are assigned nodes first, then containers, then dangling endpoints, all through one `Ids` so Mermaid's single namespace holds across subgraphs"
  - "Blank message text prints a bare `p0->>p1:`; blank actor label and note text print `#32;` (the real parser accepted both, no fallback needed)"
  - "A note with empty `over` exports `Note over : text` (degenerate, documented outside the acceptance guarantee)"
  - "The hostile case table lives in `tests/support/hostile.rs` (shared with the property generators) to keep every file under 400 non-blank lines"

requirements-completed: []  # REND-06 stays open: plan 84-05 also lists it and proves the frozen corpus round trip

duration: ~45min
completed: 2026-10-16
---

# Phase 84 Plan 02: Exporter Correctness and Sequence Export Summary

**`export` now nests containers as Mermaid subgraphs with one global id namespace and is total over arbitrary `GraphIr`; `export_sequence` exports `SequenceIr` with opaque `p<i>` participants; the pinned real parser accepts all 20 hostile fixtures (graph and sequence) with id-free sets equal to the IR's, and three mutation proofs show the encoder guards are load-bearing.**

## Accomplishments

- **Task 1 (tracer), `52f67a214`.** `export.rs` builds a `Tree` (members and children per container position, top-level lists) and emits with an explicit `Step::{Open, Close}` stack, 4 spaces per depth, edges last. A node naming no container is top level; a container nests only under an EARLIER container (cycle-free by construction); a repeated container id resolves to its first occurrence; edges to container ids and dangling endpoints are handled. `g00-nested` (4-deep, empty container, self-loop, sibling containers sharing a local id, depth-1 to depth-4 edges) is accepted by Mermaid with equal sets, empty `subgraph` included.
- **Task 2, `621a74ee2`.** 15 hostile graph cases `g01..g15` (keyword ids, edge-letter ids, `n<k>` collisions, quotes and entities, HTML, backticks, colon and percent guards, punctuation, whitespace, Unicode placeholders, long labels, statement injection, all six shapes, edge labels incl. empty and blank). `mermaid_export_props.rs` (256 cases each): line count `1 + nodes + 2 x containers + edges`, per-line classification with payload safety (no raw `"`, `<`, `>`, `&`, backtick or control character apart from the `<br>` newline encoding, every `#` opens an entity), unique safe ids in one namespace, and the same over `model_graph()` + `noisy_source()` + `check()` with a byte-identical second export. Unit tests cover degenerate IRs and a 500-deep container chain.
- **Task 3, `a74b9b0d4`.** `export_sequence` (opaque `p<i>` ids, dangling ids appended in first-appearance order labelled with the id, notes capped at two ids), `escape::seq_text` on a shared `encode(s, seq)`, `canon_sequence` in the test support, goldens `s00..s03`, and a sequence property.

## TDD evidence

| Task | RED observed | GREEN |
|------|--------------|-------|
| 1 | 4 of 5 `mermaid_export` tests FAILED against the 84-01 flat exporter (nesting, global ids, degenerate IR, deep chain, missing golden) | 17 tests pass across `mermaid_export`, `mermaid_roundtrip`, `mermaid::export::`; Node `export-hostile/g00-nested` passes |
| 2 | Goldens `g01..g15` did not exist: `export_hostile_goldens` panics on a missing golden before blessing. The totality rules and the encoder already existed from Task 1 and 84-01, so the properties and cases were green on first run; they are covered by the mutation proofs below instead | 30 tests pass; Node 24 pass |
| 3 | `mermaid_export_seq` failed to compile: unresolved import `vox_diagram::mermaid::export_sequence` (exit 101) | 26 tests pass; Node 28 pass (20 `export-hostile`) |

Honest gap: `seq_text` and its unit tests were written in one edit (I implemented the encoder before running the unit tests), so its RED is the compile failure of `export_sequence` above plus mutation (c), not a separate failing unit-test run.

## Mutation proofs (break, confirm named failures, restore with presence checks)

| | Mutation | Presence before / mutated / restored | Rust failures | Node failures after blessing the goldens under the mutation |
|--|----------|------|----------|----------|
| (a) | delete the `'"' => "#quot;"` arm in `escape::label` | 1 / 0 / 1 | `mermaid::escape::tests::each_encoding_row`, `mermaid_export_props` `export_is_structurally_safe_for_arbitrary_irs` and `export_of_checked_documents_is_safe_and_deterministic` | `export-hostile/g04-quotes-entities`, `g07-colon-guard`, `g13-injection`, `g14-shapes`, `g15-edge-labels` |
| (b) | delete the `':' if colon` arm | 1 / 0 / 1 | `mermaid::escape::tests::colon_only_with_style_or_classdef` | `export-hostile/g07-colon-guard` (only) |
| (c) | delete the `wrap_at == Some(at)` guard in `encode` | 1 / 0 / 1 | `mermaid::escape::tests::seq_text_defuses_wrap_prefixes`, `mermaid_export_seq` `export_sequence_hostile_goldens` and `export_sequence_is_structurally_safe` | `export-hostile/s01-text` (only) |

After each: source restored from a saved copy, `git checkout -- crates/vox-diagram/tests/fixtures/export-hostile` (goldens had been staged first), `git diff --quiet -- crates/vox-diagram/src crates/vox-diagram/tests/fixtures` clean, presence count back to 1, the `proptest-regressions` file the mutation wrote deleted (none committed), and the full suite re-run green.

## Verification

| Check | Result |
|-------|--------|
| `INSTA_UPDATE=no cargo nextest run -p vox-diagram` | 226 passed, 1 skipped (the ignored freezer) |
| `cargo clippy -p vox-diagram --all-targets -- -D warnings` | clean |
| `node --test tests/unit/mermaid-oracle.test.mjs` (from `docs-astro`) | 28 pass, 0 fail; 20 `export-hostile` (16 `g`, 4 `s`) |
| `ls export-hostile/g*.export.mmd` / `g*.sets.json` | 16 / 16; `s*.sets.json` 4, all `"kind": "sequence"` |
| `grep -c 'with_cases(256)'` in the two property files | 1 and 1 |
| Largest new/changed file (non-blank lines) | `support/hostile.rs` 367; all under 400 |
| T-84-SC: manifests touched between `plan_head_before` and `plan_head_after` | none (no `package.json`, lockfile or `Cargo.*`) |

## Measured oracle rules

The research encoding held everywhere: no extra numeric encoding was needed. Specifically the real parser accepted an empty `subgraph`, `participant pN as #32;`, `Note over pN: #32;`, and the bare `p0->>p1:` for blank message text; keyword and edge-letter ids never reach Mermaid (they fall back to `n<index>`), and `-->` inside a label survives as `--#gt;`.

## Deviations from Plan

1. **[Judgement] Hostile table in `tests/support/hostile.rs`.** The plan lists `mermaid_export.rs` for the case table; one file would exceed the 400-line rule and the property generators reuse the fragments, so the table and a shared `payload_problem` checker live in the support module (registered in `support/mod.rs`).
2. **[Judgement] 500-deep chain, not deeper.** Output size grows with the square of depth (indentation); 500 proves no recursion at about 1M characters.
3. **Test-side fix, no source change.** A first draft of `export_ids_are_global` used a same-scope node and container named `a`, which the DSL itself rejects as a duplicate; rewritten with the container nested under another container.

Otherwise: plan executed as written.

## Deferred Issues

None new. The 84-01 deferral (`tutorial-record.test.mjs` blob mismatch, unrelated) is unchanged.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. T-84-06 to T-84-10 are mitigated and test-backed (`g13-injection`, `g01`, `g04/g07/g08/g11`, 500-deep chain plus degenerate-IR test, `s01/s02` plus sequence property).

## Self-Check: PASSED

- Files exist: `src/mermaid/export_seq.rs`, `tests/mermaid_export.rs`, `tests/mermaid_export_props.rs`, `tests/mermaid_export_seq.rs`, `tests/support/hostile.rs`, 40 files under `tests/fixtures/export-hostile/`.
- Commits `52f67a214`, `621a74ee2`, `a74b9b0d4` are ancestors of HEAD.
