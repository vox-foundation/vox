---
phase: 90-corpus-migration-gate-tightening
plan: 06
subsystem: docs
tags: [vox-diagram, mermaid-migration, allowlist, closing-gate, gantt-kept]
requires:
  - phase: 90-05
    provides: "32 of 32 convertible fences migrated, baseline at 1 fence / 1 file"
provides:
  - "legacy-Mermaid baseline equals the allowlist: the Gantt only, reason recorded beside the fence and on the reference page"
  - "growth of the allowlist shown to fail the doc lint; archive untouched; closing gate green over the full migrated corpus"
affects: []

actuals:
  tokens: 400
  tasks: 2
  commits: 1
plan_head_before: 354ae63d0
plan_head_after: 4a02aa69b
commits: 1

tech-stack:
  added: []
  patterns: []

key-files:
  created: []
  modified:
    - docs/src/architecture/deep-research-full-surface-audit-and-roadmap-2026.md
    - docs/src/reference/vox-diagram.md

key-decisions:
  - "Gantt reason lives in an HTML comment above the fence and one sentence in the reference page; the generated baseline YAML has no reason field and was not touched (orchestrator decision)"

requirements-completed: [MIG-03, MIG-04]

duration: n/a
completed: 2026-10-27
status: complete
---

# Phase 90 Plan 06: Allowlist and Closing Gate Summary

The baseline holds exactly one entry, `docs/src/architecture/deep-research-full-surface-audit-and-roadmap-2026.md: 1` (the Gantt). `--tighten-mermaid-baseline` reported it already tight. The reason is written above the fence and in `## Legacy Mermaid fences`. The closing gate is green over the full migrated corpus.

## Commits

| Hash | Message |
|------|---------|
| 4a02aa69b | docs(90): record the Gantt as the only legacy-Mermaid allowlist entry |

`commits: 1` is `git rev-list --count 354ae63d0..4a02aa69b` (the HEAD at plan start; the protocol-0c ledger was not created, as in 90-05). `actuals.tokens` is chars/4 over the two doc edits.

## Task 1: allowlist and gate

- Tighten: `legacy-Mermaid baseline already tight`, exit 0, YAML untouched.
- Baseline: `grep -c '^  docs/src/'` is 1 (the full-surface doc, count 1). `git log 4fc8a42d7..HEAD` on the YAML lists exactly ten commits, all `docs(90): migrate batch N Mermaid fences to vox-diagram`.
- `grep -rl '^```mermaid$' docs/src` outside `archive/` prints only the full-surface doc.
- Reason: HTML comment directly above the Gantt fence (contains `no Gantt kind`, names the baseline's only allowlisted entry, user decision 2026-10-16, Phase 90); one added sentence in `docs/src/reference/vox-diagram.md` linking to the doc. Fence and frontmatter untouched.
- Plan-checker warning applied: the fidelity report was re-run after the comment was added. The Gantt row is still `kept` (its fence line moved 529 -> 530, body unchanged).
- Growth mutation proof: a two-node Mermaid fence appended to `graphify-python-free-findings-2026.md` made its scoped lint exit 1 with
  `ERROR architecture/graphify-python-free-findings-2026.md:98 — 1 ```mermaid fence(s), the legacy-Mermaid baseline allows 0; author new diagrams as ```vox-diagram (legacy-mermaid-growth)`.
  After `git checkout -- <file>`, `git diff --quiet` on it exited 0 and the scoped lint passed (exit 0).
- Archive: `git diff --quiet 4fc8a42d7 HEAD -- docs/src/archive` exit 0; `git status --porcelain -- docs/src/archive` empty.
- Scoped lint over the two edited docs: `lint complete — no hard errors`.

### Final fidelity report (33 rows)

`fidelity: migrated 32, pending 0, kept 1, failures 0` (`target/vd-migration/final/fidelity.md`)

| stem | fence | status | kind | nodes/actors | edges/events | containers |
|---|---|---|---|---|---|---|
| 00-012-internal-web-ir-strategy-l262 | docs/src/adr/012-internal-web-ir-strategy.md:262 | migrated | graph | 8 | 7 | 0 |
| 01-ai-first-fixtures-research-2026-l147 | docs/src/architecture/ai-first-fixtures-research-2026.md:147 | migrated | graph | 11 | 10 | 0 |
| 02-as-any-casts-reduction-strategy-research-2026-l116 | docs/src/architecture/as-any-casts-reduction-strategy-research-2026.md:116 | migrated | graph | 5 | 4 | 0 |
| 03-axis-harness-river-design-2026-l83 | docs/src/architecture/axis-harness-river-design-2026.md:83 | migrated | graph | 11 | 11 | 4 |
| 04-deep-research-full-surface-audit-and-roadmap-2026-l46 | ...deep-research-full-surface-audit-and-roadmap-2026.md:46 | migrated | graph | 17 | 16 | 0 |
| 05-...-l217 | ...-roadmap-2026.md:235 | migrated | graph | 4 | 3 | 1 |
| 06-...-l238 | ...-roadmap-2026.md:262 | migrated | graph | 12 | 8 | 2 |
| 07-...-l340 | ...-roadmap-2026.md:363 | migrated | graph | 7 | 8 | 0 |
| 08-...-l368 | ...-roadmap-2026.md:401 | migrated | graph | 6 | 7 | 0 |
| 09-...-l407 | ...-roadmap-2026.md:449 | migrated | graph | 9 | 6 | 3 |
| 10-...-l485 (Gantt) | ...-roadmap-2026.md:530 | kept | - | - | - | - |
| 11-deep-research-waves-and-competitive-analysis-research-2026-l45 | docs/src/architecture/deep-research-waves-and-competitive-analysis-research-2026.md:45 | migrated | graph | 10 | 11 | 0 |
| 12-...-l76 | ...-research-2026.md:87 | migrated | graph | 13 | 16 | 0 |
| 13-...-l110 | ...-research-2026.md:137 | migrated | graph | 18 | 20 | 1 |
| 14-...-l211 | ...-research-2026.md:254 | migrated | graph | 6 | 5 | 0 |
| 15-gamification-ludus-review-findings-2026-l128 | docs/src/architecture/gamification-ludus-review-findings-2026.md:128 | migrated | graph | 5 | 4 | 0 |
| 16-graphify-python-free-findings-2026-l57 | docs/src/architecture/graphify-python-free-findings-2026.md:57 | migrated | graph | 4 | 3 | 0 |
| 17-mcp-vox-language-exposure-l21 | docs/src/architecture/mcp-vox-language-exposure.md:21 | migrated | graph | 8 | 7 | 0 |
| 18-scientia-automated-research-historical-extension-research-2026-l50 | docs/src/architecture/scientia-automated-research-historical-extension-research-2026.md:50 | migrated | graph | 10 | 7 | 2 |
| 19-...-l112 | ...-research-2026.md:113 | migrated | sequence | 8 | 9 | - |
| 20-tauri-convergence-migration-plan-2026-l117 | docs/src/architecture/tauri-convergence-migration-plan-2026.md:117 | migrated | graph | 11 | 10 | 0 |
| 21-unified-workstream-execution-plan-2026-l45 | docs/src/architecture/unified-workstream-execution-plan-2026.md:45 | migrated | graph | 14 | 17 | 0 |
| 22-vox-compiler-architecture-research-2026-l19 | docs/src/architecture/vox-compiler-architecture-research-2026.md:19 | migrated | graph | 8 | 8 | 3 |
| 23-vox-gui-browser-support-2026-l33 | docs/src/architecture/vox-gui-browser-support-2026.md:33 | migrated | graph | 4 | 3 | 0 |
| 24-...-l50 | ...-support-2026.md:52 | migrated | graph | 7 | 7 | 0 |
| 25-...-l92 | ...-support-2026.md:99 | migrated | graph | 4 | 3 | 0 |
| 26-vox-react-backend-interop-audit-2026-l51 | docs/src/architecture/vox-react-backend-interop-audit-2026.md:51 | migrated | graph | 11 | 12 | 0 |
| 27-deploy-contract-l14 | docs/src/ci/deploy-contract.md:14 | migrated | graph | 6 | 6 | 0 |
| 28-expl-durable-execution-l35 | docs/src/explanation/expl-durable-execution.md:35 | migrated | graph | 5 | 5 | 0 |
| 29-how-to-rust-crate-imports-l53 | docs/src/how-to/how-to-rust-crate-imports.md:53 | migrated | graph | 7 | 6 | 0 |
| 30-mens-external-tech-options-l280 | docs/src/reference/mens-external-tech-options.md:280 | migrated | graph | 5 | 4 | 0 |
| 31-mens-measurement-gap-analysis-l284 | docs/src/reference/mens-measurement-gap-analysis.md:284 | migrated | graph | 10 | 9 | 0 |
| 32-populi-l211 | docs/src/reference/populi.md:211 | migrated | graph | 5 | 6 | 0 |

(`...` abbreviates the same file path as the nearest full row above.)

## Task 2: closing gate

All logs under `target/vd-migration/final/`.

| Gate | Result |
|------|--------|
| `pnpm --dir docs-astro diagrams` then `build` | exit 0; 844 pages built |
| Per-page `class="vd-figure"` count vs convertible fences | 21 `ok`, 0 `MISMATCH`, 32 figures total (`figcount.txt`) |
| Playwright `tests/vox-diagram.spec.ts` + `tests/build-output/diagrams.spec.ts` | 211 passed (11.1 s); all 21 corpus routes appear in the run output |
| Doc lint, 22 touched docs, one scoped run | `lint complete — no hard errors`, exit 0 |
| `tests/unit/mermaid-oracle.test.mjs` | 78 pass, 0 fail |
| `cargo nextest -p vox-doc-pipeline --lib` (`pipeline::diagrams::` + `pipeline::mermaid_gate::`) | 25 passed (includes `committed_docs_diagram_fences_are_valid`) |
| `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0` (full; phase touched `crates/vox-diagram/src`) | 600 passed, 4 skipped; the `unclosed_gt_openers_scale_linearly` flake did not fire |
| `cargo clippy -p vox-diagram --all-targets -- -D warnings` | exit 0 |
| `cargo fmt -p vox-diagram --check` | exit 0 |
| `pnpm --dir docs-astro check` | exit 1, 70 files with errors, none in this milestone (see Pre-existing failures) |
| No dependency / lockfile / crate-edge change since `4fc8a42d7` (Cargo.toml/lock, both crate manifests, docs-astro package.json + lockfile, crate-edges and fan-in baselines) | `git diff --quiet` exit 0 |
| `target/debug/vox ci ssot-drift` (after `cargo build -p vox-cli`, 1m26s) | exit 0, no drift; no regeneration needed (`test-inventory.v1.json` not changed) |
| `target/debug/vox ci check-links` | PASS, 5369 internal links |

## Pre-existing failures (noted, not fixed)

- `pnpm --dir docs-astro check` errors are confined to `astro.config.mjs`, `src/components/Banner.astro`, `src/components/VoxPlayground.astro` (25), `src/content/docs/architecture/fableforge-impl/*.ts` (4 files), `src/examples/golden/ts_source_ffi_helper.ts` and `tests/smoke.spec.ts` (3). The phase diff under `docs-astro/` touches only `tests/lib/diagrams.ts`, `tests/migration-contact-sheet.spec.ts`, `tests/vox-diagram.spec.ts`, and none of those has an error. All are TypeScript strictness errors in files this milestone never edited.
- Not run, per the orchestrator's list of known failures outside this milestone: playwright `llms.spec.ts` TRUST-02, unit `tutorial-record`, the 27 clippy errors in `vox-doc-pipeline` `src/claims/*`. Only the diagram specs and `vox-diagram` clippy were run.

## Deviations from Plan

None - plan executed as written. Orchestrator-directed additions (the fidelity re-run after the comment, `pnpm check`, full `vox-diagram` nextest/clippy/fmt) are plan step 5 conditions or explicit closing-gate additions, not deviations.

### Process notes

- The plan's figure-count pipeline was run inline as written; the `vox-diagram` full nextest ran once and passed, so no flake re-run was needed.
- `git commit` ran in the background with a timeout; the lefthook pre-commit passed (ci-status only).

## Known Stubs

None.

## Threat Flags

None.

## Unverified

- Hosted CI (docs-quality font metrics, `docs-deploy`, the full playwright suite incl. the known TRUST-02 failure) not run; carried to the milestone PR.
- The Phase 87 base-ref gate (fast pre-push tier) was not run locally; the baseline only shrank over the phase.

## Self-Check: PASSED

- Commit `4a02aa69b` is an ancestor of HEAD; both edited docs exist and contain `no Gantt kind` / `Phase 90`.
- `target/vd-migration/final/fidelity.md` exists and ends with the expected summary line.
