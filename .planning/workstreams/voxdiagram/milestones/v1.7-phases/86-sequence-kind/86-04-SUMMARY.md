---
phase: 86-sequence-kind
plan: 04
subsystem: vox-diagram
tags: [sequence-diagram, mermaid-import, dsl-printer, round-trip, corpus, contact-sheet]

requires:
  - phase: 86-sequence-kind
    provides: sequence DSL parse/check/fmt (01, 02), render hardening and geometry oracle (03)
  - phase: 84-mermaid-emitter-strict-importer
    provides: seq importer building SequenceIr with Mermaid-block spans, ImportReport
provides:
  - "print::sequence_source: SequenceIr to canonical sequence DSL (pub(crate))"
  - "mermaid::import fills ImportReport.source for every accepted sequenceDiagram block (shared formatted(), fail closed)"
  - "tests/sequence_corpus.rs: SC3 round trip of the corpus sequence block (import, DSL, re-parse, render, export) and its SVG golden"
  - "three new sequence .vdg goldens (corpus 19, import-cases 08 and 12)"
  - "documented ImportReport invariant: source.is_some() == unsupported.is_empty() for every kind; sequence kept"
affects: [90-migration]

plan_head_before: 4a387fed95ffd08515e025747555809db683ec4c
plan_head_after: eb983c30377342f32d4d3932a956e51114e4c922

actuals:
  tokens: 10000
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "one shared formatted(dsl) refusal path (limit-source / internal) for graph and sequence imports"
    - "every import test strips spans from both the imported IR and the re-checked DSL IR before comparing"

key-files:
  created:
    - crates/vox-diagram/tests/sequence_corpus.rs
    - crates/vox-diagram/tests/fixtures/svg/sequence-corpus/19-scientia-automated-research-historical-extension-research-2026-l112.svg
    - crates/vox-diagram/tests/fixtures/corpus-roundtrip/19-scientia-automated-research-historical-extension-research-2026-l112.vdg
    - crates/vox-diagram/tests/fixtures/import-cases-roundtrip/08-sequence.vdg
    - crates/vox-diagram/tests/fixtures/import-cases-roundtrip/12-seq-words.vdg
    - .planning/workstreams/voxdiagram/phases/86-sequence-kind/86-CONTACT-SHEET.html
  modified:
    - crates/vox-diagram/src/print.rs
    - crates/vox-diagram/src/mermaid/import/mod.rs
    - crates/vox-diagram/src/mermaid/import/seq.rs
    - crates/vox-diagram/src/mermaid/mod.rs
    - crates/vox-diagram/tests/mermaid_import_seq.rs
    - crates/vox-diagram/tests/mermaid_import_props.rs
    - crates/vox-diagram/tests/mermaid_corpus.rs
    - crates/vox-diagram/tests/mermaid_roundtrip.rs
    - crates/vox-diagram/tests/mermaid_fixpoint.rs
    - docs/src/architecture/where-things-live.md
    - docs/src/api/vox-diagram.generated.md

key-decisions:
  - "ImportReport.sequence is kept (planned decision). It carries spans located in the Mermaid block; the IR of `source` has spans into the DSL text. Invariant now: source.is_some() == unsupported.is_empty() for every kind; sequence.is_some() exactly for an imported sequenceDiagram block. Documented on the struct."
  - "An empty Note text (`Note over A:`) is now refused as `empty-label`: the DSL rejects `note over a: \"\"`, so it cannot be represented. Reuses the existing construct name, no catalogue change."
  - "REND-05 ticked: width-aware lanes were delivered and verified in 86-03 (oracle, goldens 30-33); this plan closes SC3 by rendering the corpus block (golden plus geometry oracle). The by-eye browser review is evidence only and is pending (see below)."

patterns-established:
  - "Fail closed on printed DSL: the IR is reported only together with DSL that fmt accepts."

requirements-completed: [DGM-02, REND-05]

status: complete
---

# Phase 86 Plan 04: Sequence import to DSL, SC3 corpus round trip, closing gates Summary

**Every accepted `sequenceDiagram` import now yields canonical `sequence { }` DSL that re-parses to the same IR; the corpus sequence block goes Mermaid, DSL, IR, SVG and back to the committed Mermaid export text, accepted by the pinned real Mermaid parser (77/77 Node oracle tests).**

## Performance

- Tasks: 3 (1 tracer, 2 auto), 3 task commits, then this docs commit.
- Closing gates all green (table below). Crate suite: 551 passed, 3 skipped (floor 464).

## Accomplishments

- `print::sequence_source` prints actors, one blank line, then messages (`reply ` for dashed, `: "text"` only when non-empty) and `note over a[, b]`, all labels via `push_quoted`.
- `import/mod.rs::formatted(dsl)` extracted from `finish` (graph behaviour unchanged); the sequence path uses it, so an in-limit Mermaid block whose DSL outgrows `MAX_SOURCE_BYTES` is refused with exactly one `limit-source` at line 1 (unit test with 312 messages of 200 chars), never a partial report.
- Tracer `tracer_corpus_sequence_round_trips`: source is a `fmt` fixed point, re-checks to the same IR (spans stripped; 8 actors, 8 messages + 1 note), renders checker-clean, passes `assert_sequence_geometry`, equals the new golden, and `export` is byte-equal to the committed `corpus-roundtrip/19-....export.mmd`.
- Every import test, the import property (`unsupported.is_empty() == source.is_some()`), the corpus accounting, `mermaid_roundtrip` (31 corpus `.vdg`, two import-case `.vdg`) and the F2 fixpoint now check the DSL round trip. Exactly three new `.vdg` files appeared; no `.export.mmd` / `.sets.json` golden changed.

## Task Commits

1. Task 1 (tracer): `ff1334643` feat(86-04): sequence import prints canonical DSL; corpus sequence round trip
2. Task 2: `329687b64` test(86-04): every sequence import path checks the DSL round trip
3. Task 3: `eb983c303` docs(86-04): document ImportReport source/sequence, sequence rows, contact sheet

## TDD record

- RED (Task 1), observed with `seq.rs` temporarily reverted to the old `source: None` behaviour: 10 failures of 32 (`import_fails_closed_when_the_dsl_is_refused`, `mermaid_import_seq` x5, `mermaid_corpus`, `mermaid_import_props::import_is_total_on_mermaid_fragment_soup`, both `sequence_corpus` tests). `print::tests::sequence_source_prints_every_event_kind` was already GREEN because the printer existed.
- RED (Task 2): `corpus_roundtrip` and `import_cases_roundtrip` panicked on the missing `.vdg` goldens; blessed with the mermaid_roundtrip command only, `git status` then showed exactly three new `.vdg` files.
- GREEN: all targeted binaries, then the full crate.

## Mutation proof (T-86-11)

Copied `print.rs`, changed the dashed branch to print `""` instead of `"reply "` (diff present: 1 insertion, 1 deletion; the grep count for `reply ` stays 3 because `let reply` and the test text contain it, so the git diff was the presence check). Failed as required: `mermaid_import_seq::seq_basic`, `seq_arrow_spacing_note_case_and_single_actor_note`, `mermaid_roundtrip::import_cases_roundtrip` (lost dashed flag via `08-sequence`) and `print::tests::sequence_source_prints_every_event_kind`. (`corpus_roundtrip` stayed green: the corpus block has no dashed message, which `import_cases_roundtrip` covers.) Restored from the copy; `git diff --quiet -- print.rs` exit 0; rerun green.

## Deviations from Plan

**1. [Rule 1 - Bug] Empty `Note over A:` imported but could not print as DSL**
- **Found during:** Task 1 (`seq_text_rules` failed with `internal`, the fail-closed fallback, once `source` was filled).
- **Issue:** the DSL checker reports `diagram/empty-label` for `note over a: ""`, so an accepted block with an empty note produced DSL that `fmt` refused.
- **Fix:** `import/seq.rs::note` refuses an empty note text with the existing construct `empty-label`; the assertion in `seq_text_rules` changed from "imports with empty text" to "refused as empty-label". Fail-closed behaviour stays as the second line of defence.
- **Files modified:** `src/mermaid/import/seq.rs`, `tests/mermaid_import_seq.rs`
- **Commit:** `ff1334643`

**2. [Rule 3 - Blocking] Contact sheet generator**
- Added an `#[ignore]` `contact_sheet` test to `tests/sequence_corpus.rs` (mirrors 85's `svg_corpus::contact_sheet`) because headless Chromium cannot be run from this shell. Output committed as `86-CONTACT-SHEET.html`.

`mermaid/export_seq.rs` and the `export_sequence` re-export docs already used present tense (no change needed); `docs/src/api/index.generated.md` did not change.

## Closing gates

| Gate | Result |
|------|--------|
| `cargo fmt -p vox-diagram --check` | pass |
| `cargo clippy -p vox-diagram --all-targets -- -D warnings` | pass |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p vox-diagram --no-deps` | pass |
| `INSTA_UPDATE=no cargo nextest run -p vox-diagram` | 551 passed, 3 skipped (floor 464) |
| `vox ci crate-api-sync --write`, then check | OK (130 crates); only `docs/src/api/vox-diagram.generated.md` changed |
| `vox ci crate-edges` | OK (700 edges within baseline; a pre-existing stale-edge warning for vox-gui, untouched) |
| `cargo run -p vox-arch-check` | pass (pre-existing stale-evidence WARN lines only) |
| `cargo hakari generate --diff` | clean (empty output, exit 0) |
| `vox-doc-pipeline --lint-only --paths architecture/where-things-live.md` | no hard errors |
| Node oracle `node --test tests/unit/mermaid-oracle.test.mjs` | 77 pass, 0 fail |
| `git diff --quiet 883a3e4b1 -- Cargo.toml Cargo.lock crates/vox-diagram/Cargo.toml crates/workspace-hack contracts/ci docs/src/architecture/layers.toml` | exit 0 |
| 400 non-blank-line check over every `.rs` under `crates/vox-diagram/src` and `tests` | none over |

The `vox` binary was stale; rebuilt with `cargo build -p vox-cli` (background), freshness guard never bypassed.

## Visual review

Pending for the orchestrator. Headless Chromium is outside the shell allowlist, so the by-eye browser review was not performed and not worked around. `86-CONTACT-SHEET.html` holds the corpus sequence plus `30-seq-long-labels`, `31-seq-self-and-notes`, `32-seq-replies-and-wide-actors` and `33-seq-unicode`, each in a light and a dark pane. Text-level check only (ImageMagick flatten, no strokes or markers drawn): the corpus golden shows eight lane headers, labels centred between lifelines, and the note spanning lanes 2 to 6 (RS to SC, x 219 to 1150) with its text readable and nothing clipped. Arrows, lifelines and CJK/emoji glyphs are covered by the geometry oracle and checker only.

## Known Stubs

None.

## Threat Flags

None. No network, auth or schema surface added; no manifest, lockfile, baseline or exception change.

## Self-Check: PASSED

- Files exist: `crates/vox-diagram/tests/sequence_corpus.rs`, the sequence-corpus SVG golden, the three `.vdg` goldens, `86-CONTACT-SHEET.html`.
- Commits `ff1334643`, `329687b64`, `eb983c303` are ancestors of HEAD; `git rev-list --count 4a387fed9..HEAD` was 3 before this docs commit.
