---
phase: 90-corpus-migration-gate-tightening
verified: 2026-10-27T10:25:00Z
status: human_needed
score: 9/9 must-haves verified
covered_files:
  - ".planning/workstreams/voxdiagram/phases/90-corpus-migration-gate-tightening/90-01-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/90-corpus-migration-gate-tightening/90-01-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/90-corpus-migration-gate-tightening/90-02-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/90-corpus-migration-gate-tightening/90-02-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/90-corpus-migration-gate-tightening/90-03-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/90-corpus-migration-gate-tightening/90-03-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/90-corpus-migration-gate-tightening/90-04-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/90-corpus-migration-gate-tightening/90-04-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/90-corpus-migration-gate-tightening/90-05-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/90-corpus-migration-gate-tightening/90-05-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/90-corpus-migration-gate-tightening/90-06-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/90-corpus-migration-gate-tightening/90-06-SUMMARY.md"
  - "contracts/documentation/legacy-mermaid-baseline.v1.yaml"
  - "crates/vox-diagram/src/layout/dagre_impl.rs"
  - "crates/vox-diagram/src/layout/finish.rs"
  - "crates/vox-diagram/src/layout/mod.rs"
  - "crates/vox-diagram/src/layout/ports.rs"
  - "crates/vox-diagram/src/layout/ports/tests.rs"
  - "crates/vox-diagram/src/layout/tests.rs"
  - "crates/vox-diagram/src/layout/tests/containers.rs"
  - "crates/vox-diagram/src/layout/tests/feeds.rs"
  - "crates/vox-diagram/src/layout/tests/titles.rs"
  - "crates/vox-diagram/src/layout/text.rs"
  - "crates/vox-diagram/src/layout/text/tests.rs"
  - "crates/vox-diagram/src/layout/verify.rs"
  - "crates/vox-diagram/src/layout/verify/tests.rs"
  - "crates/vox-diagram/tests/fixtures/import-cases-roundtrip/13-rust-crate-imports-stripped.export.mmd"
  - "crates/vox-diagram/tests/fixtures/import-cases-roundtrip/13-rust-crate-imports-stripped.sets.json"
  - "crates/vox-diagram/tests/fixtures/import-cases-roundtrip/13-rust-crate-imports-stripped.vdg"
  - "crates/vox-diagram/tests/fixtures/import-cases/13-rust-crate-imports-stripped.mmd"
  - "crates/vox-diagram/tests/freeze_corpus.rs"
  - "crates/vox-diagram/tests/migration_fidelity.rs"
  - "crates/vox-diagram/tests/support/clearance.rs"
  - "crates/vox-diagram/tests/support/mod.rs"
  - "crates/vox-diagram/tests/svg_corpus.rs"
  - "crates/vox-diagram/tests/svg_geometry_props.rs"
  - "docs-astro/tests/lib/diagrams.ts"
  - "docs-astro/tests/migration-contact-sheet.spec.ts"
  - "docs-astro/tests/vox-diagram.spec.ts"
  - "docs/src/adr/012-internal-web-ir-strategy.md"
  - "docs/src/architecture/ai-first-fixtures-research-2026.md"
  - "docs/src/architecture/as-any-casts-reduction-strategy-research-2026.md"
  - "docs/src/architecture/axis-harness-river-design-2026.md"
  - "docs/src/architecture/deep-research-full-surface-audit-and-roadmap-2026.md"
  - "docs/src/architecture/deep-research-waves-and-competitive-analysis-research-2026.md"
  - "docs/src/architecture/gamification-ludus-review-findings-2026.md"
  - "docs/src/architecture/graphify-python-free-findings-2026.md"
  - "docs/src/architecture/mcp-vox-language-exposure.md"
  - "docs/src/architecture/scientia-automated-research-historical-extension-research-2026.md"
  - "docs/src/architecture/tauri-convergence-migration-plan-2026.md"
  - "docs/src/architecture/unified-workstream-execution-plan-2026.md"
  - "docs/src/architecture/vox-compiler-architecture-research-2026.md"
  - "docs/src/architecture/vox-gui-browser-support-2026.md"
  - "docs/src/architecture/vox-react-backend-interop-audit-2026.md"
  - "docs/src/ci/deploy-contract.md"
  - "docs/src/explanation/expl-durable-execution.md"
  - "docs/src/how-to/how-to-rust-crate-imports.md"
  - "docs/src/reference/mens-external-tech-options.md"
  - "docs/src/reference/mens-measurement-gap-analysis.md"
  - "docs/src/reference/populi.md"
  - "docs/src/reference/vox-diagram.md"
covered_digest: "v3:sha256:03e585813a2a4c37b4b5d121087817cf19fe8f1295f114dd451377590922a4ac"
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Open the PR and let hosted CI run: docs-quality, docs-deploy and the Linux fonts lane"
    expected: "All green. The doc lint, the legacy-Mermaid ratchet, the figure build and the diagram Playwright specs pass on Linux with the shipped Arimo fonts."
    why_human: "Hosted CI is deferred to the PR by decision. Every gate here ran on macOS only."
  - test: "Look over the review-fix contact sheets in target/vd-migration/review-fix/ (32 stems; at least 03, 06, 09, 12, 13, 18 and 22)"
    expected: "Edges leave and enter node faces away from corners and cross no foreign box. Container titles sit just above their content. Long edges read well as straight diagonals. Corpus 12's back edge 'Subagents Distill -> Lead Researcher', which now crosses 'Spawn -> Subagents' near the node, is acceptable."
    why_human: "CR-01 (port pass plus line-of-sight straightening) and WR-01 (title snap) are layout heuristics. The fixer marked both 'requires human verification'. Tests prove there are no through-node, corner or title-gap defects, but they cannot judge legibility."
---

# Phase 90: Corpus Migration & Gate Tightening Verification Report

**Phase Goal:** The live docs corpus uses VoxDiagram, with only an explicit allowlist left as Mermaid and the gate enforcing exactly that.
**Verified:** 2026-10-27
**Status:** human_needed
**Re-verification:** No. This is the initial verification, done after the code review and its fix round (HEAD `884d0681f`).

Everything that can be checked in code or by a command is verified. The two items left are hosted CI, which is deferred to the PR by decision, and a by-eye check of the two layout heuristics from the review fixes.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1 / MIG-03: every convertible ```` ```mermaid ```` fence under `docs/src` (outside `archive/`) is migrated to ```` ```vox-diagram ```` | VERIFIED | A fence grep over `docs/src` (any number of backticks or tildes, `{mermaid}` too, `archive/` excluded) finds only `deep-research-full-surface-audit-and-roadmap-2026.md:530`, the Gantt. There are 38 `vox-diagram` fences: the 32 migrated ones plus the 6 on the reference page. |
| 2 | SC1: each migration is faithful (node, edge and container sets equal the oracle golden) | VERIFIED | I ran `cargo nextest run -p vox-diagram --run-ignored all -E 'binary(migration_fidelity)' --no-capture`. It printed `fidelity: migrated 32, pending 0, kept 1, failures 0`, with one row for each of the 33 frozen fences, and the sequence stem 19 shows 8 actors and 9 events. The review also compared all 32 fences byte for byte with the importer goldens. |
| 3 | SC1: batches were reviewed on before/after contact sheets | VERIFIED | `target/vd-migration/` has b1 to b10, final and review-fix (64 PNGs for 32 stems). SUMMARYs 01 to 05 hold the fidelity and by-eye rows. I read the review-fix sheet for stem 18: Mermaid is beside VoxDiagram in light and dark, and no edge crosses a foreign box. |
| 4 | SC2 / MIG-04: the baseline lists exactly the allowlist (the Gantt) | VERIFIED | `legacy-mermaid-baseline.v1.yaml` has one entry, `docs/src/architecture/deep-research-full-surface-audit-and-roadmap-2026.md: 1`. `--tighten-mermaid-baseline` reported "already tight" and left the YAML unchanged. |
| 5 | SC2: the allowlist reason is recorded | VERIFIED | There is an HTML comment directly above the Gantt (line 529): no Gantt kind, the only allowlisted entry, user decision 2026-10-16. The "Legacy Mermaid fences" section of `docs/src/reference/vox-diagram.md` (line 257) names the Gantt and the reason. |
| 6 | SC2: the gate enforces exactly the allowlist | VERIFIED | Mutation: I appended a Mermaid fence to `how-to-rust-crate-imports.md` and ran `vox-doc-pipeline --lint-only --mermaid-only`. It exited 1 with `legacy-mermaid-growth` at `how-to/how-to-rust-crate-imports.md:185`. The file was restored (hash checked) and the clean run exits 0. |
| 7 | SC3: `git diff` over `docs/src/archive/` is empty | VERIFIED | `git diff --stat $(git merge-base main HEAD) -- docs/src/archive` and `git diff 455c2ffc7..HEAD -- docs/src/archive` are both empty, and `git status` shows no change there. |
| 8 | SC3: every migrated diagram renders on the built site, and the Phase 89 Playwright suite passes over the full corpus | VERIFIED | I re-ran at HEAD: `pnpm diagrams` gave 38 diagrams from 842 files; `pnpm build` built 844 pages; then `playwright test tests/vox-diagram.spec.ts tests/build-output/diagrams.spec.ts` passed 211 of 211. For each corpus page, the number of `vd-figure` elements in `dist` equals its number of `vox-diagram` fences, 38 in total, with no mismatch. |
| 9 | The review fixes are real: CR-01 and WR-01 to WR-06 each have a commit and a guarding test | VERIFIED | See the table below. Mutation for CR-01: with the `ports::route` call disabled, `corpus_edges_clear_foreign_nodes` failed ("41 graphs: 20 with a defect"). The file was restored (hash checked). |

**Score:** 9/9 truths verified (0 present but behavior-unverified)

### Review findings: commit and guard

| Finding | Commit | Guarding test (exists, runs in the default nextest set) |
|---|---|---|
| CR-01 edge through or at the corner of a node | `e247c112e` | `svg_corpus::corpus_edges_clear_foreign_nodes` (an SVG-level oracle in `tests/support/clearance.rs`), the `geometry_holds` ceilings, `layout::ports::tests`, `verify` `node_crossing`. I re-proved it by mutation. |
| WR-01 hole in the title band | `1b17a49c5` | `layout::tests::titles::a_title_hugs_its_content`, plus the `snap` unit test |
| WR-02 nested feeder staircase | `f12ec3bf8` | `feeds::a_container_fed_through_a_nested_child_lies_after_its_source`; the `dagre_impl` unit test now expects `(n(3), t(0))` |
| WR-03 CJK not wrapping | `d663d2663` | `text/tests.rs` CJK (15 x U+65E5 to 14 + 1) and the mixed Latin/CJK case; the `word_cap` applies only to all-narrow words (`text.rs:133`) |
| WR-04 loose property bound | `cfabd9bc0` | `text/tests.rs:224` `cells(l) <= WRAP_CELLS \|\| (one_narrow_word && cells(l) <= WORD_CELLS)` |
| WR-05 split-word check matched all SVG text | `58a7bcbad` | `corpus_words_are_never_split` now filters `class == "vd-label" && owner == k` and uses cell widths |
| WR-06 destructive freezer | `5e27d7c3e` | `the_freezer_refuses_to_shrink_the_corpus_and_touches_nothing`, `the_freezer_freezes_a_corpus_that_does_not_shrink` |

IN-04 was resolved as a side effect of WR-01. IN-01, IN-02 and IN-03 are info-level and were deliberately left alone. They are not must-haves.

### Required Artifacts

| Artifact | Status | Details |
|---|---|---|
| `crates/vox-diagram/tests/migration_fidelity.rs` | VERIFIED | Ignored test, run explicitly, reports 33 rows |
| `docs-astro/tests/migration-contact-sheet.spec.ts` | VERIFIED | Skipped by default; it produced the review-fix sheets |
| `contracts/documentation/legacy-mermaid-baseline.v1.yaml` | VERIFIED | Gantt only; generated header intact |
| `crates/vox-diagram/src/layout/ports.rs` (CR-01) | VERIFIED, WIRED | Called from `layout/mod.rs:260` in `place` |
| `crates/vox-diagram/tests/support/clearance.rs` | VERIFIED, WIRED | Used by `svg_corpus.rs` and `svg_geometry_props.rs` |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Crate tests | `cargo nextest run -p vox-diagram` | 619 passed, 4 skipped (the known flake did not fire) | PASS |
| Lints | `cargo clippy -p vox-diagram --all-targets -- -D warnings` | clean | PASS |
| Fidelity | `nextest --run-ignored all -E 'binary(migration_fidelity)'` | migrated 32, pending 0, kept 1, failures 0 | PASS |
| Baseline tight | `cargo run -p vox-doc-pipeline -- --tighten-mermaid-baseline` | "already tight", no diff | PASS |
| Ratchet clean | `cargo run -p vox-doc-pipeline -- --lint-only --mermaid-only` | exit 0 | PASS |
| Ratchet growth (mutation) | the same, with an extra fence | exit 1, `legacy-mermaid-growth` | PASS |
| Site and figures | `pnpm diagrams && pnpm build` + diagram Playwright specs | 38 diagrams, 844 pages, 211/211 | PASS |

### Probe Execution

Not applicable: the phase declares no `probe-*.sh`.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|---|---|---|---|
| MIG-03 | 90-01 to 90-05 | SATISFIED | Truths 1, 2, 3, 7 and 8 |
| MIG-04 | 90-06 | SATISFIED | Truths 4, 5 and 6 |

No requirement mapped to Phase 90 is orphaned.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| (all phase files) | - | TBD/FIXME/XXX | none found | - |
| `crates/vox-diagram/tests/svg_geometry_props.rs` | 400 lines | CONTEXT says "files < 400 lines", and this file is at exactly 400 (it was 397) | Info | At the boundary; not a goal blocker |
| `90-VALIDATION.md` | frontmatter | `status: draft`, `nyquist_compliant: false` | Info | validate-phase has not run; optional |

No new crate edges or dependency changes: `Cargo.toml` and `contracts/ci` are unchanged since the phase base.

### Human Verification Required

#### 1. Hosted CI on the PR

**Test:** Open the PR and let docs-quality, docs-deploy and the Linux fonts lane run.
**Expected:** All green.
**Why human:** Deferred to the PR by decision; every local gate ran on macOS.

#### 2. By-eye check of the CR-01 and WR-01 layout heuristics

**Test:** Look over `target/vd-migration/review-fix/*.png`, especially 03, 06, 09, 12, 13, 18 and 22.
**Expected:** Edges leave and enter node faces away from corners, titles hug their content, and the straight-diagonal long edges read well. Corpus 12's back edge crossing near "Subagents" is acceptable.
**Why human:** Routing and title placement are heuristics. The tests prove there are no defects, but they cannot judge legibility.

### Gaps Summary

There are no gaps. The corpus is migrated faithfully: 32 fences pass the fidelity oracle and the importer goldens. The Gantt is the only Mermaid fence left; its reason is recorded in two places, and the ratchet provably rejects growth. The archive is untouched. The built site renders every figure, and the diagram Playwright suite passes over the full corpus. Every review finding has a commit and a guarding test, and the blocker's guard was re-proved by mutation.

---

_Verified: 2026-10-27_
_Verifier: Claude (gsd-verifier)_
