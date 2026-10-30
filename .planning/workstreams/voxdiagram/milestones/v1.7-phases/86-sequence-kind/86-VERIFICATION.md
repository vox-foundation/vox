---
phase: 86-sequence-kind
verified: 2026-10-17T00:00:00Z
status: passed
score: 3/3 must-haves verified
covered_files:
  - ".planning/workstreams/voxdiagram/phases/86-sequence-kind/86-01-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/86-sequence-kind/86-01-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/86-sequence-kind/86-02-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/86-sequence-kind/86-02-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/86-sequence-kind/86-03-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/86-sequence-kind/86-03-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/86-sequence-kind/86-04-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/86-sequence-kind/86-04-SUMMARY.md"
  - "crates/vox-diagram/src/layout/seq.rs"
  - "crates/vox-diagram/src/layout/seq/canvas.rs"
  - "crates/vox-diagram/src/layout/seq/lanes.rs"
  - "crates/vox-diagram/src/mermaid/export_seq.rs"
  - "crates/vox-diagram/src/mermaid/import/seq.rs"
  - "crates/vox-diagram/src/parser/recover.rs"
  - "crates/vox-diagram/src/parser/seq.rs"
  - "crates/vox-diagram/src/parser/seq_mermaid.rs"
  - "crates/vox-diagram/src/print.rs"
  - "crates/vox-diagram/src/resolve/seq.rs"
  - "crates/vox-diagram/src/sequence_ir.rs"
  - "crates/vox-diagram/src/svg/seq.rs"
  - "crates/vox-diagram/tests/sequence_corpus.rs"
  - "crates/vox-diagram/tests/sequence_props.rs"
  - "crates/vox-diagram/tests/svg_sequence_geometry.rs"
covered_digest: "v3:sha256:261dcb4d2ed0e9f1c6f8960757357067be1c2babb3c3a2411564de291d4ea796"
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Open 86-CONTACT-SHEET.html in a real browser (light and dark panes) and inspect the corpus sequence plus the four svg/cases/3x-seq-* diagrams"
    expected: "Actor boxes, dashed lifelines, arrows, 80+ char wrapped labels, self-message loops and notes read cleanly with no overlap or clipping, in both themes"
    why_human: "Headless browser is blocked in this environment, so no by-eye render review was possible. Geometry is covered at SVG level by the independent read-back oracle. Carried to Phase 89 real-browser (Playwright) verification; it is not a blocker for the SVG-level success criteria."
---

# Phase 86: Sequence Kind Verification Report

**Phase Goal:** Authors can write, check, format, and render `sequence` diagrams, and the corpus's sequence blocks round-trip through the DSL.
**Verified:** 2026-10-17
**Status:** human_needed (all automated criteria verified; one visual item carried to Phase 89)
**Re-verification:** No, initial verification

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Author can write a `sequence` with actors, ordered messages, dashed returns, `note over`; `check` and `fmt` behave as for `graph` (all errors at once, codes, spans, fixes; idempotent canonical form) | VERIFIED | `parser/seq.rs` (actor, `reply`, message, `note over`), `resolve/seq.rs`, `parser/seq_mermaid.rs`; `Ir::Sequence` wired in `check.rs:87`. `sequence_dsl`, `sequence_check`, `sequence_mermaid`, `sequence_props` (S1-S5, 128 cases: noisy check = model IR, fmt idempotent/canonical, parse(fmt(x)) = parse(x), repair converges), `fixes_repair` snapshots for 14/15 sequence fixtures all pass in the full run |
| 2 | `sequence` renders to SVG with lane widths driven by the longest label; 80+ char messages neither overlap nor clip; same safe writer; byte-identical across runs | VERIFIED | `layout/seq/lanes.rs::lane_xs` widens each crossed gap to `label.w + 2*8px`; 60-cell soft wrap (`MSG_WRAP_CELLS`); `svg/seq.rs` uses the shared `svg::writer::Writer`; layout is integer half-pixels (no `f32/f64` in seq layout/svg). `tracer_long_message_wraps_and_fits` (87-char label), `generated_sequences_never_overlap_or_clip` (independent oracle, labels to 130 chars, CJK/emoji), `svg_sequence_safety` (hostile text, caps, 20x determinism) pass. Mutation check: weakening the gap rule in `lanes.rs` to a constant made `tracer_long_message_wraps_and_fits`, `explicit_newlines_are_kept_and_long_lines_wrap` and `oracle_rejects_a_forged_overlap` fail; file restored (tree clean) |
| 3 | The corpus sequence block imports, prints as DSL, re-parses to the same IR, renders, and re-exports to Mermaid accepted by the pinned real Mermaid parser | VERIFIED | `sequence_corpus::tracer_corpus_sequence_round_trips` passes: `ImportReport.source` is a `fmt` fixed point, re-checks to the same `SequenceIr` (spans stripped, 8 actors / 9 events), renders checker-clean with geometry oracle, SVG golden matches, `export(ir)` byte-equals committed `corpus-roundtrip/19-scientia-...-l112.export.mmd`. Node oracle (pinned mermaid 11.17.2) `corpus-roundtrip/19-scientia-...-l112` passes; 77/77 |

**Score:** 3/3 truths verified (0 present, behavior-unverified). Behavior-dependent invariants (width fit, no overlap, repair convergence) each have an executing test.

### Plan-level must-haves
Plan truths for 86-01..04 (caps `TooLarge`, `InvalidIr` for unknown actors, undeclared-actor declare fix, Mermaid-ism rewrites, `.vdg` per round-tripped block, `ImportReport.source`) are exercised by `svg_sequence_safety`, `sequence_check`, `sequence_mermaid`, `mermaid_roundtrip` and `mermaid_import_seq`; all pass. No plan truth reduces the roadmap contract.

### Required Artifacts / Key Links

| Artifact | Status | Details |
|----------|--------|---------|
| `src/ir.rs` `Ir::Sequence`, `src/sequence_ir.rs` | VERIFIED | Present; consumed by check, fmt (`print.rs::sequence_source`), export, render |
| `src/parser/seq.rs`, `seq_mermaid.rs`, `src/resolve/seq.rs` | VERIFIED | Substantive (143/249/206 lines), dispatched from the statement parser |
| `src/layout/seq.rs` (+ `lanes.rs`, `canvas.rs`), `src/svg/seq.rs` | VERIFIED | Wired: `svg/seq.rs` imports `layout::seq::place` and the shared `Writer` |
| `src/mermaid/import/seq.rs` -> `print.rs::sequence_source` | WIRED | Import prints DSL and keeps it only when `fmt` accepts it |
| `tests/sequence_corpus.rs`, corpus `.vdg`/`.export.mmd`/SVG golden | VERIFIED | Exist; asserted byte-for-byte |
| `docs/src/architecture/where-things-live.md` | VERIFIED | Rows name sequence parser/resolve/layout/svg modules |

### Data-Flow
Corpus `.mmd` -> `import` -> DSL `source` -> `check` -> `Ir::Sequence` -> `render` / `export`: each hop asserted in one test against committed goldens; real data, no static fallback.

### Behavioral Spot-Checks and Gates

| Check | Command | Result | Status |
|-------|---------|--------|--------|
| Crate suite | `INSTA_UPDATE=no cargo nextest run -p vox-diagram` | 563 passed, 3 skipped | PASS |
| Lint | `cargo clippy -p vox-diagram --all-targets -- -D warnings` | exit 0 | PASS |
| Real Mermaid parser oracle | `node --test tests/unit/mermaid-oracle.test.mjs` (docs-astro) | 77/77 pass | PASS |
| Mutation (width rule) | weaken `lane_xs` gap to constant | 3+ tests fail; restored | PASS |

Hosted CI not run (deferred to the milestone PR per user). Workspace-wide checks not run, by instruction.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| DGM-02 (write a `sequence` with actors, ordered messages, dashed returns, `note over`) | 86-01, 86-02, 86-04 | SATISFIED | Truth 1 and 3; REQUIREMENTS.md marks Complete |
| REND-05 (sequence SVG with width-aware lanes) | 86-01, 86-03, 86-04 | SATISFIED | Truth 2 and 3; REQUIREMENTS.md marks Complete |

No orphaned requirements: REQUIREMENTS.md maps only DGM-02 and REND-05 to Phase 86, and both are claimed by plans.

### Anti-Patterns
No `TBD`/`FIXME`/`XXX`/`todo!`/`unimplemented!` in `crates/vox-diagram/src` or `tests`. No stubs found in the sequence modules. Code review (86-REVIEW.md: 1 critical, 4 warnings) is fully addressed by 86-REVIEW-FIX.md (5/5 fixed); fixes are visible in `recover.rs`, `seq_mermaid.rs`, `layout/seq.rs` and covered by `sequence_repair_review.rs` and property S5, all green.

### Human Verification Required

1. **By-eye render review of the sequence contact sheet**
   - **Test:** Open `86-CONTACT-SHEET.html` in a real browser, light and dark panes.
   - **Expected:** Corpus sequence and the `3x-seq-*` cases show no overlap or clipping; labels legible; self-loops and notes sit correctly.
   - **Why human:** Headless browser unavailable here. SVG geometry is independently covered by the read-back oracle. Carried to Phase 89 real-browser verification; not a blocker for the SVG-level criteria.

Also for human awareness: WR-03 changed multi-hop message labels to sit in the middle gap instead of the arrow midpoint (a layout choice recorded in 86-REVIEW-FIX.md).

### Gaps Summary
No gaps. All three ROADMAP success criteria are met in the codebase and backed by passing tests, a mutation check and the pinned Mermaid oracle. Status is `human_needed` only because the by-eye visual review is outstanding; it is explicitly carried to Phase 89.

---

_Verified: 2026-10-17_
_Verifier: Claude (gsd-verifier)_

## Human Validation

2026-10-17: user accepted SVG-level evidence; by-eye light/dark review of 86-CONTACT-SHEET.html is carried to Phase 89 real-browser verification.
