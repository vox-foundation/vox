---
phase: "90"
slug: "corpus-migration-gate-tightening"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-27"
---

# Phase 90 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust: cargo-nextest. New ignored one-shot integration test `crates/vox-diagram/tests/migration_fidelity.rs` (reads `docs/src`, compares each migrated fence's id-free canonical sets with the oracle-proven `corpus-roundtrip` / `import-cases-roundtrip` goldens, plus an export/import round trip); existing `mermaid_roundtrip`, `svg_corpus`, `fence_scan`; `vox-doc-pipeline` lib tests `pipeline::diagrams::` (incl. `committed_docs_diagram_fences_are_valid`) and `pipeline::mermaid_gate::`. docs-astro: `node:test` (`tests/unit/mermaid-oracle.test.mjs`, pinned mermaid 11.17.2 + jsdom) and `@playwright/test` 1.63 (`tests/vox-diagram.spec.ts`, `tests/build-output/diagrams.spec.ts`, and the new skipped-by-default generator `tests/migration-contact-sheet.spec.ts`). Doc gates: `vox-doc-pipeline --lint-only` (whole-tree legacy-Mermaid pass included) and `--tighten-mermaid-baseline`. No new dependency |
| **Config file** | `.config/nextest.toml`; `docs-astro/playwright.config.ts` (webServer `pnpm preview --port 4321`, reuse locally) |
| **Quick run command** | `FIDELITY_OUT=/Users/brbrainerd/dev/vox-voxdiagram/target/vd-migration/<batch>/fidelity.md timeout 2400 cargo nextest run -p vox-diagram --retries 0 --run-ignored only -E 'binary(migration_fidelity)'` ; `timeout 1200 cargo run -q -p vox-doc-pipeline -- --lint-only --paths <batch docs>` |
| **Full suite command** | `timeout 1800 pnpm --dir docs-astro build && timeout 1800 pnpm --dir docs-astro exec playwright test tests/vox-diagram.spec.ts tests/build-output/diagrams.spec.ts --reporter=line --output=test-results/vd-run` ; `timeout 600 pnpm --dir docs-astro exec node --test tests/unit/mermaid-oracle.test.mjs` ; `INSTA_UPDATE=no timeout 2400 cargo nextest run -p vox-diagram --retries 0 -E 'binary(mermaid_roundtrip) \| binary(svg_corpus) \| binary(fence_scan)'` |
| **Estimated runtime** | fidelity filter seconds once built (first build minutes: background it); doc lint ~30 s; `pnpm build` 1-3 min plus the pre-render; diagram Playwright specs 2-5 min as migrated pages join; contact sheet ~1 min per batch |

Command rules: the machine is heavily loaded, so every cargo, pnpm and Playwright command runs with `timeout` and in the background with its log under `target/vd-migration/`. `cmp` and `node -e` are blocked by the shell allowlist (use `diff -q`); `vox-doc-pipeline` runs through `cargo run -p vox-doc-pipeline`. Stop any `astro preview` on port 4321 before a Playwright run. nextest exits non-zero when a filter selects no test.

---

## Sampling Rate

- **After every batch commit:** the batch's fidelity run (expected summary line), tighten, scoped doc lint, build plus the two diagram specs, and the contact sheet (the Batch procedure P3-P6 in 90-01-PLAN.md).
- **After every renderer fix (P7):** the red-first test, `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0`, `cargo clippy -p vox-diagram --all-targets -- -D warnings`, a mutation proof, and contact sheets of earlier batches re-diffed.
- **After every plan wave:** nothing extra; each wave ends on a batch whose checks already cover all migrated pages (the browser specs discover every figure page from `dist/`).
- **Before `/gsd-verify-work`:** 90-06 (no-op tighten, final 33-row fidelity report, growth mutation proof, archive diff, per-page figure count, full diagram specs, doc lint over 22 docs, oracle, Rust checks, invariants, SSOT drift).
- **Max feedback latency:** about 60 s for the fidelity filter and the scoped lint once built; about 5 min for build plus browser specs.

---

## Requirement → Automated Command Map

| Requirement | Success criterion | Automated command | Fails when |
|-------------|-------------------|-------------------|------------|
| MIG-03 | SC1: each migrated fence keeps the original's node/edge/container sets (import → export → equality) and every not-yet-migrated block is unchanged | `FIDELITY_OUT=<abs>/fidelity.md timeout 2400 cargo nextest run -p vox-diagram --retries 0 --run-ignored only -E 'binary(migration_fidelity)' && grep -c 'fidelity: migrated M, pending P, kept 1, failures 0' <abs>/fidelity.md` | non-zero exit, no tests run, or the expected summary line absent |
| MIG-03 | SC1: the goldens equal the real Mermaid parse of the originals (incl. the backtick-stripped how-to block) | `timeout 600 pnpm --dir docs-astro exec node --test tests/unit/mermaid-oracle.test.mjs` | non-zero exit or `# fail` above 0 |
| MIG-03 | SC1: each batch reviewed on a before/after contact sheet | `CONTACT_SHEET_STEMS=<stems> CONTACT_SHEET_OUT=<abs dir> timeout 900 pnpm --dir docs-astro exec playwright test tests/migration-contact-sheet.spec.ts --reporter=line && ls <dir>/*.png \| wc -l` | non-zero exit, skipped, or PNG count other than 2 x stems (the reading of the PNGs is manual, below) |
| MIG-03 | SC3: every migrated diagram renders on the built site and the Phase 89 suite passes over the migrated corpus | `timeout 1800 pnpm --dir docs-astro build && timeout 1800 pnpm --dir docs-astro exec playwright test tests/vox-diagram.spec.ts tests/build-output/diagrams.spec.ts --reporter=line --output=test-results/vd-run` plus the 90-06 per-page figure count (21 `ok` lines) | non-zero exit, `No tests found`, any failed test, or a count other than 21 |
| MIG-03 | SC3: `git diff` over `docs/src/archive/` is empty | `git diff --quiet 4fc8a42d7 HEAD -- docs/src/archive && git status --porcelain -- docs/src/archive > target/vd-migration/final/archive-status.txt && test ! -s target/vd-migration/final/archive-status.txt` | non-zero exit |
| MIG-04 | SC2: baseline tightened to exactly the allowlist by the generator | `timeout 1200 cargo run -q -p vox-doc-pipeline -- --tighten-mermaid-baseline 2>&1 \| grep -c 'already tight'` ; `grep -c '^  docs/src/' contracts/documentation/legacy-mermaid-baseline.v1.yaml` | grep prints 0; entry count other than 1 |
| MIG-04 | SC2: doc lint fails if a non-allowlisted fence appears | 90-06 growth mutation: scoped lint of a migrated doc with an appended Mermaid fence exits non-zero naming `legacy-mermaid-growth`; restored lint passes | the mutated lint exits 0 |
| MIG-03, MIG-04 | Every touched doc passes the doc lint | `timeout 1200 cargo run -q -p vox-doc-pipeline -- --lint-only --paths <22 docs>` | non-zero exit or any ERROR line |

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 90-01-01 | 01 | 1 | MIG-03, MIG-04 | T-90-01, T-90-02, T-90-03, T-90-04, T-90-SC | Fidelity tool proven able to fail (label mutation, drift mutation); generator-only baseline | ignored integration + e2e build + browser | fidelity filter (expect `migrated 1, pending 31, kept 1, failures 0`) ; scoped lint ; build + diagram specs ; contact sheet (2 PNGs) | ❌ W0 (`migration_fidelity.rs`, `migration-contact-sheet.spec.ts` created test-first in the task) | ⬜ pending |
| 90-01-02 | 01 | 1 | MIG-03, MIG-04 | T-90-01, T-90-02 | Importer output only; set equality | batch | fidelity (`migrated 5, pending 27`) ; lint ; build + specs ; contact sheet (8 PNGs) | ✅ (from 90-01-01) | ⬜ pending |
| 90-02-01 | 02 | 2 | MIG-03, MIG-04 | T-90-05, T-90-06 | as above | batch | fidelity (`migrated 9, pending 23`) ; lint ; build + specs ; contact sheet (8) | ✅ | ⬜ pending |
| 90-02-02 | 02 | 2 | MIG-03, MIG-04 | T-90-05, T-90-06, T-90-07, T-90-08 | Stripped block equals original minus backticks; three-way oracle | fixture + oracle + batch | `cat corpus/29-*.mmd \| tr -d '\140' \| diff -q - import-cases/13-*.mmd` ; `binary(mermaid_roundtrip)` ; oracle ; fidelity (`migrated 13, pending 19`) ; lint ; build + specs ; contact sheet (8) | ❌ W0 (import case 13 and its goldens created in the task) | ⬜ pending |
| 90-03-01 | 03 | 3 | MIG-03, MIG-04 | T-90-09, T-90-10, T-90-11 | as above | batch | fidelity (`migrated 18, pending 14`) ; lint ; build + specs ; contact sheet (10) | ✅ | ⬜ pending |
| 90-03-02 | 03 | 3 | MIG-03, MIG-04 | T-90-09, T-90-10, T-90-11 | Sequence event order kept | batch | fidelity (`migrated 20, pending 12`) ; lint ; build + specs ; contact sheet (4) | ✅ | ⬜ pending |
| 90-04-01 | 04 | 4 | MIG-03, MIG-04 | T-90-12, T-90-13, T-90-14 | as above | batch | fidelity (`migrated 22, pending 10`) ; lint ; build + specs ; contact sheet (4) | ✅ | ⬜ pending |
| 90-04-02 | 04 | 4 | MIG-03, MIG-04 | T-90-12, T-90-13, T-90-14 | as above | batch | fidelity (`migrated 26, pending 6`) ; lint ; build + specs ; contact sheet (8) | ✅ | ⬜ pending |
| 90-05-01 | 05 | 5 | MIG-03, MIG-04 | T-90-15, T-90-16, T-90-17 | Gantt row `kept` (byte-identical) | batch | fidelity (`migrated 29, pending 3`) ; lint ; build + specs ; contact sheet (6) | ✅ | ⬜ pending |
| 90-05-02 | 05 | 5 | MIG-03, MIG-04 | T-90-15, T-90-16, T-90-17 | as above | batch | fidelity (`migrated 32, pending 0`) ; lint ; build + specs ; contact sheet (6) | ✅ | ⬜ pending |
| 90-06-01 | 06 | 6 | MIG-04, MIG-03 | T-90-18, T-90-19, T-90-20 | No-op tighten; growth rejected; archive untouched | gate + mutation | tighten `already tight` ; final fidelity ; scoped lint ; archive diff | ✅ | ⬜ pending |
| 90-06-02 | 06 | 6 | MIG-03, MIG-04 | T-90-21, T-90-SC | No dependency or edge change | closing gate | build + per-page figure count (21) ; diagram specs ; lint over 22 docs ; oracle + Rust filters ; `git diff --quiet 4fc8a42d7 HEAD -- <manifests, lockfiles, edge baselines>` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Test-first inside the tasks that need them (no separate scaffold plan):

- [ ] `crates/vox-diagram/tests/migration_fidelity.rs` (90-01 Task 1; RED shown by mutations (a) label change and (b) drift)
- [ ] `docs-astro/tests/migration-contact-sheet.spec.ts` + `NATURAL_CSS` in `docs-astro/tests/lib/diagrams.ts` (90-01 Task 1; RED shown by the not-migrated stem)
- [ ] `crates/vox-diagram/tests/fixtures/import-cases/13-rust-crate-imports-stripped.mmd` and its three goldens (90-02 Task 2)
- [ ] A red-first test per renderer defect, if any (P7, in the batch task that finds it)
- [ ] No framework install (Chromium is already cached locally)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| By-eye review of every before/after contact sheet | MIG-03 | Visual judgement of overlaps, clipping, title strikes, split words, staircases, unreadable density | P6 in each batch: Read every `<stem>.png` and `<stem>-column.png` under `target/vd-migration/bN/`; one review row per fence in the plan SUMMARY with concrete defects or "none" |
| Readability of the largest graphs at shipped scale (corpus 13, axis-harness) | MIG-03 | Judgement at the CSS 0.6 floor | 90-04 reviews of the column PNGs |
| Block 06 dropped subgraph directions | MIG-03 | Grammar gap, not a renderer bug | 90-05 Task 1 review; an unreadable result becomes an open item for the user |
| Hosted docs-quality run over the migrated corpus (DejaVu/Liberation font metrics) | MIG-03 | Only runs on hosted CI | Carried to the milestone PR's CI run, as in Phase 89 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s for the fidelity filter and the scoped lint (build plus browser specs are the batch-level gate)
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
