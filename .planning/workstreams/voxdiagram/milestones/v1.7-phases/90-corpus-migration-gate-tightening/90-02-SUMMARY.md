---
phase: 90-corpus-migration-gate-tightening
plan: 02
subsystem: docs
tags: [vox-diagram, mermaid-migration, fidelity, state-diagram, markdown-string, import-case]

requires:
  - phase: 90-01
    provides: "migration_fidelity report, contact-sheet spec, per-batch procedure, wrap fix"
provides:
  - "8 more migrated fences (13 of 32 convertible), baseline at 20 fences / 8 files"
  - "oracle-checked import case 13-rust-crate-imports-stripped (Rust goldens + real-Mermaid three-way check)"
  - "figure id check scoped to ids inside a figure"
affects: [90-03, 90-04, 90-05, 90-06]

actuals:
  tokens: 4800
  tasks: 2
  commits: 4
plan_head_before: 63c2b4c00
plan_head_after: 61c176495f5c2738001b9ce4627626955514c9a4
commits: 4

tech-stack:
  added: []
  patterns:
    - "Hand-converted block = frozen block minus backticks (tr -d) as an import case; HAND table maps the migrated fence to that golden"

key-files:
  created:
    - crates/vox-diagram/tests/fixtures/import-cases/13-rust-crate-imports-stripped.mmd
    - crates/vox-diagram/tests/fixtures/import-cases-roundtrip/13-rust-crate-imports-stripped.vdg
    - crates/vox-diagram/tests/fixtures/import-cases-roundtrip/13-rust-crate-imports-stripped.export.mmd
    - crates/vox-diagram/tests/fixtures/import-cases-roundtrip/13-rust-crate-imports-stripped.sets.json
  modified:
    - crates/vox-diagram/tests/migration_fidelity.rs
    - docs-astro/tests/vox-diagram.spec.ts
    - docs/src/ci/deploy-contract.md
    - docs/src/reference/populi.md
    - docs/src/adr/012-internal-web-ir-strategy.md
    - docs/src/architecture/mcp-vox-language-exposure.md
    - docs/src/how-to/how-to-rust-crate-imports.md
    - docs/src/reference/mens-measurement-gap-analysis.md
    - docs/src/architecture/ai-first-fixtures-research-2026.md
    - docs/src/architecture/tauri-convergence-migration-plan-2026.md
    - contracts/documentation/legacy-mermaid-baseline.v1.yaml

key-decisions:
  - "The figure id uniqueness check only flags a duplicate that involves an id inside a figure; a page's own prose anchors are not the diagram's concern"

requirements-completed: []  # MIG-03 / MIG-04 close in 90-06

duration: n/a
completed: 2026-10-27
status: complete
---

# Phase 90 Plan 02: Batches 3 and 4 Summary

Eight more fences migrated through the 90-01 procedure, including the corpus's only state diagram (populi) and the markdown-string block, which was converted by deleting its backticks and importing it as a new oracle-checked import case; no renderer defect found, one over-broad browser check corrected.

## Commits

| Hash | Message |
|------|---------|
| ffef07d75 | docs(90): migrate batch 3 Mermaid fences to vox-diagram |
| 8031a05a9 | test(90-02): oracle-checked import case for the backtick-stripped how-to block |
| 0b420613a | test(90): scope the figure id check to ids inside a figure |
| 61c176495 | docs(90): migrate batch 4 Mermaid fences to vox-diagram |

## Fidelity

Report lines: after batch 3 `fidelity: migrated 9, pending 23, kept 1, failures 0`; after batch 4 `fidelity: migrated 13, pending 19, kept 1, failures 0`. Tighten lines (generator only): `tightened: 28 -> 24 fences, 16 -> 12 files`, then `tightened: 24 -> 20 fences, 12 -> 8 files`. Reports: `target/vd-migration/b3/fidelity.md`, `b4/fidelity.md`.

| stem | fence | status | kind | nodes | edges | containers |
|---|---|---|---|---|---|---|
| 27-deploy-contract-l14 | docs/src/ci/deploy-contract.md:14 | migrated | graph | 6 | 6 | 0 |
| 32-populi-l211 | docs/src/reference/populi.md:211 | migrated | graph | 5 | 6 | 0 |
| 00-012-internal-web-ir-strategy-l262 | docs/src/adr/012-internal-web-ir-strategy.md:262 | migrated | graph | 8 | 7 | 0 |
| 17-mcp-vox-language-exposure-l21 | docs/src/architecture/mcp-vox-language-exposure.md:21 | migrated | graph | 8 | 7 | 0 |
| 29-how-to-rust-crate-imports-l53 | docs/src/how-to/how-to-rust-crate-imports.md:53 | migrated (HAND golden) | graph | 7 | 6 | 0 |
| 31-mens-measurement-gap-analysis-l284 | docs/src/reference/mens-measurement-gap-analysis.md:284 | migrated | graph | 10 | 9 | 0 |
| 01-ai-first-fixtures-research-2026-l147 | docs/src/architecture/ai-first-fixtures-research-2026.md:147 | migrated | graph | 11 | 10 | 0 |
| 20-tauri-convergence-migration-plan-2026-l117 | docs/src/architecture/tauri-convergence-migration-plan-2026.md:117 | migrated | graph | 11 | 10 | 0 |

(Rows copied from `b3/fidelity.md` and `b4/fidelity.md`.) Every `.vdg` was the importer's output: `diff -q` identical to the golden (`corpus-roundtrip/<stem>.vdg`; for stem 29, `import-cases-roundtrip/13-rust-crate-imports-stripped.vdg`), exit 0, no `unsupported:` or `dropped:` lines, `vox diagram check` exit 0. No id renamed. No allowlist entry added.

Populi: the state diagram imports as a `graph` with `start`/`end` terminal nodes (shapes `start`, `end`), eight transitions as labelled edges, including the `Leased -> Leased` self-loop.

## Backtick-stripped import case (stem 29)

- `cat corpus/29-how-to-rust-crate-imports-l53.mmd | tr -d '\140' > import-cases/13-rust-crate-imports-stripped.mmd`; re-verified by `diff -q` of the same pipeline against the committed file: identical (the case differs from the frozen block by deleted backticks only). Visible effect: the code-formatting backticks around `.vox`, `ImportPathKind::RustCrate`, `HirRustImport`, `Cargo.toml`, `cargo build` are gone from the labels, as the plan and CONTEXT sanction.
- `UPDATE_GOLDEN=1 ... -E 'binary(mermaid_roundtrip) & test(import_cases_roundtrip)'` then the whole `binary(mermaid_roundtrip)` without it: 8 tests, 8 passed. `git status --porcelain crates/vox-diagram/tests/fixtures` listed exactly the four new `13-` files (the case and its three goldens); no existing golden changed.
- Node oracle: `pnpm --dir docs-astro exec node --test tests/unit/mermaid-oracle.test.mjs`: 78 tests, 78 pass, 0 fail, including `import-cases-roundtrip/13-rust-crate-imports-stripped` (real Mermaid 11.17.2 accepts the stripped block; sets equal three ways).
- `migration_fidelity` `HAND` entry maps stem `29-how-to-rust-crate-imports-l53` to `import-cases-roundtrip/13-rust-crate-imports-stripped.sets.json`; row 29 reads `migrated`.

## Contact-sheet review (every PNG read)

PNGs under `/Users/brbrainerd/dev/vox-voxdiagram/target/vd-migration/b3/` and `b4/` (`<stem>.png` and `<stem>-column.png`, 8 each).

| stem | doc:line | Mermaid shows | VoxDiagram shows | defects | disposition |
|---|---|---|---|---|---|
| 27-deploy-contract | deploy-contract.md:14 | top-down chain, fork after deploy-coolify (Poll & verify left, Failure right), merge into notify | same nodes and edges; fork sides swapped (Failure left); edge labels sit in gaps on their edges; no overlap | none | observations: swapped branch order, Mermaid fill not carried |
| 32-populi | populi.md:211 | state diagram: dot start, rounded states, Pending<->Leased loop with two labels, Leased self-loop, bullseye end | Start and End pills, Pending/Leased/Done boxes, deliver/ack/lease-renew labels, self-loop right of Leased, lease expiry (swept) edge back up into Pending, all labels readable | none | observations: pill terminals instead of dot/bullseye, states drawn as boxes, ack edge straight (Mermaid's curves around lease-renew); judged on states, transitions, start, end: all present |
| 00-012-internal-web-ir-strategy | adr/012-internal-web-ir-strategy.md:262 | LR chain of 5 then 3 emitters fanned out (React top) | same chain, fan-out order reversed (Future top, React bottom) | none | observation: emitter order reversed |
| 17-mcp-vox-language-exposure | mcp-vox-language-exposure.md:21 | LR, three rows, resources/read above, installed skills below | same arrangement; the Surface -> tools/list edge passes just under WorkspaceMcpSurface and clear of resources/read | none | observation: edge routes close to the Surface box but touch nothing |
| 29-how-to-rust-crate-imports | how-to/how-to-rust-crate-imports.md:53 | before panel shows Mermaid's rejection ("Lexical error on line 2") and the source, as expected | 7 nodes, LR, "AST ImportPathKind::RustCrate" wrapped on two lines, fan-out from HIR, long last label on two lines; no clipping | none | reviewed on its own merits (the original never rendered on the site); column PNG scrolls at the right edge by design (see below) |
| 31-mens-measurement-gap-analysis | mens-measurement-gap-analysis.md:284 | 6 sources in a row, 3 contracts, DecisionGate | same shape; source order reversed per group (Cli/Mcp, Mens/HeldOut, Corpus/Training); edges cleanly separated | none | observation: order reversed; column PNG scrolls |
| 01-ai-first-fixtures-research-2026 | architecture/ai-first-fixtures-research-2026.md:147 | 11-box column | same 11 nodes in a column, two-line labels where Mermaid wraps, no clipping | none in the diagram | see tooling note: the sticky site header overlaps the figure in the element screenshot of the column PNG |
| 20-tauri-convergence-migration-plan | architecture/tauri-convergence-migration-plan-2026.md:117 | LR with two branches from vox compile | same 11 nodes; branch order differs (native-binary below), edges fan without crossing nodes or labels | none | observation: layout differs, readable; column PNG scrolls |

Tooling notes (not diagram defects): (a) the 0.6 scale floor plus in-figure horizontal scroll is the designed Phase 89 behaviour (spec `wide diagrams`), so the column PNGs of 29, 31, 20 are clipped on the right where the real page scrolls; (b) `01-...-column.png` shows the page's sticky header ("Search" bar) drawn across the `parser` box because Playwright's element screenshot of a tall figure scrolls under the fixed header; the whole-figure panels in `01-....png` show the box intact.

## Mutation and negative proofs

- Figure id check (deviation below): with the pre-fix spec the tauri page failed with 9 `duplicate id phase-N--...` entries (explicit `<a id>` plus heading in the doc prose). After the fix 14/14 `ids:` tests pass. Mutation: in the built `dist/architecture/mcp-vox-language-exposure/index.html` the page `<h1 id="_top">` was given the figure's title id (grep count of the id 1 before, 2 after); the `ids:` test for that route then failed with `duplicate id vd-architecture-mcp-vox-language-exposure-1-title`; the saved copy was restored (count 1) and no source file was touched.
- Fidelity mutation proofs (label change, drift) were done in 90-01 and the tool is unchanged apart from the `HAND` entry.

## Verification

- Doc lint scoped to each batch's four docs: `lint complete — no hard errors.` (both batches).
- `pnpm --dir docs-astro build` then `tests/vox-diagram.spec.ts` + `tests/build-output/diagrams.spec.ts`: batch 3 103 passed; batch 4 final run 139 passed (first batch-4 run: 138 passed, 1 failed, the id check below).
- Contact sheets generated with `CONTACT_SHEET_STEMS`/`CONTACT_SHEET_OUT`: 8 PNGs per batch.
- No change to Cargo.lock, crates/vox-diagram/Cargo.toml, docs-astro/package.json, pnpm-lock.yaml since 4fc8a42d7; `git diff 4fc8a42d7 -- docs/src/archive` empty.
- Baseline: 8 `docs/src/` entries remain, written only by `--tighten-mermaid-baseline`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Page-wide id uniqueness check flagged the doc's own prose**
- **Found during:** Task 2, batch 4 browser specs (`ids:` for `/architecture/tauri-convergence-migration-plan-2026/`)
- **Issue:** the doc writes `<a id="phase-N--..."></a>` before each phase heading, so those ids appear twice in the page. The check was global, so the page failed as soon as it carried a figure. Not a renderer defect: the figure's own ids (`vd-...`) are unique.
- **Fix:** `docs-astro/tests/vox-diagram.spec.ts` reports a duplicate only when one of the elements holding the id is inside `.vd-figure`. The doc prose was not edited (execution rule 5).
- **Files modified:** docs-astro/tests/vox-diagram.spec.ts
- **Commit:** 0b420613a

### Process notes

- Plan 90-02's per-step commands were run as `&&`/sequential commands in the background with status files; no scratch runner script was used.
- `lean-ctx` rewrites `cat`/`tail` on absolute paths and denies paths outside the other project root: log files were read with the Read tool.
- Protocol 0c ledger was again not created; `commits: 4` is `git rev-list --count 63c2b4c00..HEAD` with 63c2b4c00 (HEAD at plan start) as base. `actuals.tokens` is a rough chars/4 over the 19 KB non-planning diff.
- No renderer defect, so no `fix(90)` commit and no re-bless beyond the four new `13-` files.

## Known Stubs

None.

## Threat Flags

None.

## Unverified

- Hosted CI (docs-quality with DejaVu/Liberation metrics) not run; carried to the milestone PR.
- `docs-astro` `pnpm check` not run on the spec edit (ran under Playwright's TypeScript transpile).
- `limits_adversarial::unclosed_gt_openers_scale_linearly` and the full `vox-diagram` suite were not run (no Rust source changed; only test fixtures, `migration_fidelity.rs` and the new import case, covered by `mermaid_roundtrip` 8/8 and the fidelity run).

## Self-Check: PASSED

Commits ffef07d75, 8031a05a9, 0b420613a, 61c176495 are ancestors of HEAD; the four `13-rust-crate-imports-stripped.*` fixtures exist; baseline lists 8 files.
