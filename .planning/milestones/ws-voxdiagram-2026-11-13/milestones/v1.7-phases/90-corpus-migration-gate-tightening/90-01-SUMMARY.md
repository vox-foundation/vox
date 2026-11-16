---
phase: 90-corpus-migration-gate-tightening
plan: 01
subsystem: docs
tags: [vox-diagram, mermaid-migration, fidelity, playwright, contact-sheet]

requires:
  - phase: 89-docs-astro-integration
    provides: "inline figure pipeline, diagram Playwright specs, NATURAL review-bundle CSS"
  - phase: 84-mermaid-import-export
    provides: "strict importer, corpus-roundtrip goldens proven against the pinned Mermaid parse"
provides:
  - "migration_fidelity: ignored per-fence fidelity report over the 33 frozen fences"
  - "migration-contact-sheet.spec.ts: skipped-by-default Mermaid 11.17.2 vs built-site contact sheets"
  - "5 of 32 convertible fences migrated (batches 1 and 2), baseline at 28 fences / 16 files"
  - "renderer fix: words up to 48 cells are never split mid-word"
affects: [90-02, 90-03, 90-04, 90-05, 90-06]

actuals:
  tokens: 15200
  tasks: 2
  commits: 4
plan_head_before: 455c2ffc7
plan_head_after: 5bdbf95131cacad3e1cd1e08bf639b0880e0de7d
commits: 4

tech-stack:
  added: []
  patterns:
    - "Fidelity = id-free canonical sets vs the oracle-proven golden, plus export/import round trip"
    - "Contact sheet per stem: pinned Mermaid render beside the built site's figure (light, dark) plus the as-shipped column PNG"

key-files:
  created:
    - crates/vox-diagram/tests/migration_fidelity.rs
    - docs-astro/tests/migration-contact-sheet.spec.ts
  modified:
    - crates/vox-diagram/tests/freeze_corpus.rs
    - crates/vox-diagram/tests/svg_corpus.rs
    - crates/vox-diagram/src/layout/text.rs
    - crates/vox-diagram/src/layout/text/tests.rs
    - crates/vox-diagram/tests/fixtures/svg/cases/02-wrap.svg
    - crates/vox-diagram/tests/fixtures/svg/corpus/04-deep-research-full-surface-audit-and-roadmap-2026-l46.svg
    - docs-astro/tests/lib/diagrams.ts
    - docs-astro/tests/vox-diagram.spec.ts
    - docs/src/architecture/graphify-python-free-findings-2026.md
    - docs/src/reference/mens-external-tech-options.md
    - docs/src/architecture/as-any-casts-reduction-strategy-research-2026.md
    - docs/src/architecture/gamification-ludus-review-findings-2026.md
    - docs/src/explanation/expl-durable-execution.md
    - contracts/documentation/legacy-mermaid-baseline.v1.yaml

key-decisions:
  - "A word of 29-48 cells keeps a line of its own and widens its box (Mermaid behaviour); only a longer word is hard-broken, at 48 cells"
  - "The 02-wrap golden case now exercises hard breaking through its CJK label only; its ASCII overlong word (40 chars) is whole"

requirements-completed: []  # MIG-03 / MIG-04 close in 90-06

duration: n/a
completed: 2026-10-27
status: complete
---

# Phase 90 Plan 01: Fidelity tooling, batches 1 and 2 Summary

Per-fence fidelity report and Mermaid-vs-VoxDiagram contact-sheet generator, proven on the graphify fence, then four more single-fence docs migrated; the contact-sheet review found one renderer defect (identifiers split mid-word), fixed red-first.

## Commits

| Hash | Message |
|------|---------|
| 89279bf88 | test(90-01): add migration fidelity report and contact-sheet generator |
| 0593f0995 | docs(90): migrate batch 1 Mermaid fences to vox-diagram |
| e45074c5b | fix(90): keep an identifier longer than a line whole instead of splitting it |
| 5bdbf9513 | docs(90): migrate batch 2 Mermaid fences to vox-diagram |

## Fidelity (final report, after the fix)

`fidelity: migrated 5, pending 27, kept 1, failures 0` (baseline before any doc edit: `migrated 0, pending 32, kept 1, failures 0`; after batch 1: `migrated 1, pending 31, ...`).

| stem | fence | status | kind | nodes | edges | containers |
|---|---|---|---|---|---|---|
| 16-graphify-python-free-findings-2026-l57 | docs/src/architecture/graphify-python-free-findings-2026.md:57 | migrated | graph | 4 | 3 | 0 |
| 30-mens-external-tech-options-l280 | docs/src/reference/mens-external-tech-options.md:280 | migrated | graph | 5 | 4 | 0 |
| 02-as-any-casts-reduction-strategy-research-2026-l116 | docs/src/architecture/as-any-casts-reduction-strategy-research-2026.md:116 | migrated | graph | 5 | 4 | 0 |
| 15-gamification-ludus-review-findings-2026-l128 | docs/src/architecture/gamification-ludus-review-findings-2026.md:128 | migrated | graph | 5 | 4 | 0 |
| 28-expl-durable-execution-l35 | docs/src/explanation/expl-durable-execution.md:35 | migrated | graph | 5 | 5 | 0 |

Every `.vdg` was the importer's output (`diff -q` identical to `corpus-roundtrip/<stem>.vdg`, no `unsupported:` and no `dropped:` lines, `vox diagram check` exit 0). No id was renamed.

Baseline: `tightened: 33 -> 32 fences, 21 -> 20 files` (batch 1), `tightened: 32 -> 28 fences, 20 -> 16 files` (batch 2), written by `--tighten-mermaid-baseline` only; 16 `docs/src/` entries now. Scoped doc lint passed for each batch. Built site: `pnpm build` then the two diagram specs, 31 passed (batch 1) and 67 passed (batch 2, and again after the fix); each of the five pages carries its one figure.

## Contact-sheet review (every PNG read)

PNGs under `/Users/brbrainerd/dev/vox-voxdiagram/target/vd-migration/` (`b1/`, `b2/`, `b2-recheck/` after the fix; `<stem>.png` and `<stem>-column.png`).

| stem | doc:line | Mermaid shows | VoxDiagram shows | defects | disposition |
|---|---|---|---|---|---|
| 16-graphify | graphify-python-free-findings-2026.md:57 | 4 purple boxes top-down, uniform width, "leiden-rs" wrapped as "leiden-/rs" | same 4 nodes and 3 edges, white boxes of ragged widths, no wrapped word | none | observations: Mermaid purple fill not carried, box width follows the text |
| 30-mens | mens-external-tech-options.md:280 | 5 boxes, each long identifier on one line | first render (`b2/30-...png`, `b2/30-...-column.png`): every identifier split mid-word: "StrengthenBenchmarksAndMetri/cs", "AddRepoAwareRetrievalForCode/gen", "PrototypeGrammarConstrainedD/ecoding", "PrototypeSemanticBenchmarkSu/bset" | word split inside a label (4 nodes) | fixed in e45074c5b; recheck `b2-recheck/30-...png` and `-column.png` show every identifier whole on one line |
| 02-as-any | as-any-casts-reduction-strategy-research-2026.md:116 | 5 boxes, two-line labels ("... <br/> Removes ~N casts") | 5 nodes, label broken at the `\n` into 2-3 lines, no clipping | none | observation: layout and wrapping differ slightly, still readable |
| 15-gamification | gamification-ludus-review-findings-2026.md:128 | 5 phase boxes in a column | same 5 nodes, 4 edges | none | observation: Mermaid colours not carried |
| 28-expl-durable | expl-durable-execution.md:35 | diamond "Activity Finished?" with No branch left, Yes right, loop back from "Write to Journal" | diamond (flat), Yes branch left to End Workflow, No branch right to Execute Activity, loop edge from Write to Journal back to the diamond; edge labels Yes/No clear of nodes | none | observations: Yes/No sides swapped relative to Mermaid, flatter diamond, loop edge routed on the left |

Rechecks after the fix: stems 16, 02, 15, 28 PNGs are byte-identical to the earlier ones (`diff -q`); only stem 30 changed and was re-read.

## Mutation and negative proofs

- (a) One label character changed in the migrated graphify fence ("Hash Walk" to "Hash Walc"): the report ended `failures 2`, with `MISMATCH 16-graphify-python-free-findings-2026-l57: differs in edges,nodes`. Restored from a saved copy; grep presence checks before (`Hash Walc` 1) and after (0).
- (b) One character changed inside the still-Mermaid block of `mens-external-tech-options.md` ("benchmark[" to "benchmarx["), run together with (a): `DRIFT 30-mens-external-tech-options-l280: docs/src/reference/mens-external-tech-options.md:280 differs from the frozen block`. Restored the same way (grep 1 before, 0 after).
- Contact-sheet negative: `CONTACT_SHEET_STEMS=30-...` before migration failed with `30-mens-external-tech-options-l280: fence not migrated` (exit 1); with neither variable set the spec reports `1 skipped`, exit 0.
- Renderer fix mutation: setting `let word_cap = limit;` (reverting the fix) failed 5 tests (`wrap_keeps_a_zero_width_follower_with_its_base`, `wrap_keeps_a_word_longer_than_a_line_whole`, `wrap_hard_breaks_only_an_overlong_word`, `wrap_counts_cjk_as_two_cells`, `svg_corpus::corpus_words_are_never_split`); restored, `diff -q` against the saved copy identical, grep presence check 1.

## fix(90) commit

`e45074c5b`. Red first: `layout::text::tests::wrap_keeps_a_word_longer_than_a_line_whole` and `svg_corpus::corpus_words_are_never_split` failed (the latter also named corpus fence 04, word `update_research_session_status:`, which a later batch migrates), then passed. Existing wrap tests that pinned the old 28-cell break were updated to the 48-cell cap. Goldens re-blessed and diffs read: `svg/cases/02-wrap.svg` (the 40-char word is now whole, 291 px box; CJK label now breaks at 24 chars) and `svg/corpus/04-...-l46.svg` (long identifiers whole, layout shifts). Full `INSTA_UPDATE=no cargo nextest run -p vox-diagram`: 599 passed; `cargo clippy -p vox-diagram --all-targets -- -D warnings` clean.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Words 29-48 cells split mid-word by the renderer**
- **Found during:** Task 2, contact-sheet review (P6/P7), stem 30
- **Issue:** `wrap_at` hard-broke any word over 28 cells
- **Fix:** keep such a word whole on its own line up to `WORD_CELLS` = 48; break only beyond that
- **Files modified:** crates/vox-diagram/src/layout/text.rs, text/tests.rs, tests/svg_corpus.rs, two SVG goldens
- **Commit:** e45074c5b

### Process notes

- The plan's per-step shell scripts could not be used mid-run: a scratch `.sh` runner was refused by the shell allowlist the second time it was invoked (it worked once earlier), so batch 2's build and specs ran as direct `pnpm` commands instead. Nothing was skipped.
- The commit ledger of protocol 0c was not created; `commits: 4` is `git rev-list --count 455c2ffc7..HEAD` with 455c2ffc7 (HEAD at plan start) as base.
- Mutations (a) and (b) were applied in one run (two independent failures reported) rather than two runs.
- The contact sheet writes the Mermaid original through `innerHTML` of the pinned, strict-mode Mermaid output on a local blank page (threat T-90-03, accepted in the plan).

## Known Stubs

None.

## Threat Flags

None.

## Unverified

- Hosted CI (docs-quality with DejaVu/Liberation metrics) not run; carried to the milestone PR as in Phase 89.
- `docs-astro` `pnpm check` (astro/TypeScript check) was not run on the new spec; it ran under Playwright's TypeScript transpile only.
- `migration-contact-sheet.spec.ts` uses `innerHTML` only for the Mermaid render on a throwaway page.

## Self-Check: PASSED

Commits 89279bf88, 0593f0995, e45074c5b, 5bdbf9513 are ancestors of HEAD; `migration_fidelity.rs`, `migration-contact-sheet.spec.ts` exist; baseline lists 16 files; no change to Cargo.lock, crates/vox-diagram/Cargo.toml, docs-astro/package.json, pnpm-lock.yaml since 4fc8a42d7; `docs/src/archive` diff empty.
