---
phase: 90-corpus-migration-gate-tightening
plan: 05
subsystem: docs
tags: [vox-diagram, mermaid-migration, fidelity, containers, gantt-kept]
requires:
  - phase: 90-04
    provides: "26 of 32 convertible fences migrated, baseline at 7 fences / 1 file"
provides:
  - "32 of 32 convertible fences migrated; the Gantt is the only Mermaid fence left in docs/src (outside archive)"
  - "baseline at 1 fence / 1 file (deep-research-full-surface-audit-and-roadmap-2026.md: 1)"
affects: [90-06]

actuals:
  tokens: 4200
  tasks: 2
  commits: 2
plan_head_before: d25a649ae36dc051b1452efd4c13941d125831c8
plan_head_after: f249adfc5
commits: 2

tech-stack:
  added: []
  patterns: []

key-files:
  created: []
  modified:
    - docs/src/architecture/deep-research-full-surface-audit-and-roadmap-2026.md
    - contracts/documentation/legacy-mermaid-baseline.v1.yaml

key-decisions: []

requirements-completed: []  # MIG-03 / MIG-04 close in 90-06

duration: n/a
completed: 2026-10-27
status: complete
---

# Phase 90 Plan 05: Batches 9 and 10 Summary

The six convertible fences of deep-research-full-surface-audit-and-roadmap (lines 46, 217, 238, 340, 368, 407) are `vox-diagram` fences produced by `vox diagram import`; the Gantt (stem 10) is untouched and reports `kept`. Contact-sheet review found no renderer defect (only the two classes deferred to the Phase 90 code review), so there is no `fix(90)` commit and no golden was re-blessed. Block 06's two dropped subgraph directions are a no-op in this figure (see below) and the converted figure is readable: migrated, not allowlisted.

## Commits

| Hash | Message |
|------|---------|
| 1d8c51f38 | docs(90): migrate batch 9 Mermaid fences to vox-diagram |
| f249adfc5 | docs(90): migrate batch 10 Mermaid fences to vox-diagram |

## Fidelity

Report lines: after batch 9 `fidelity: migrated 29, pending 3, kept 1, failures 0`; after batch 10 `fidelity: migrated 32, pending 0, kept 1, failures 0`. Tighten lines (generator only): `tightened: 7 -> 4 fences, 1 -> 1 files`, then `tightened: 4 -> 1 fences, 1 -> 1 files`. Reports: `target/vd-migration/b9/fidelity.md`, `b10/fidelity.md`. The batch-9 report showed the mixed file paired correctly (04-06 `migrated`, 07-09 `pending`, 10 `kept`).

| stem | fence | status | kind | nodes | edges | containers |
|---|---|---|---|---|---|---|
| 04-deep-research-full-surface-audit-and-roadmap-2026-l46 | docs/src/architecture/deep-research-full-surface-audit-and-roadmap-2026.md:46 | migrated | graph | 17 | 16 | 0 |
| 05-...-l217 | ...-roadmap-2026.md:235 (after batch 9) | migrated | graph | 4 | 3 | 1 |
| 06-...-l238 | ...-roadmap-2026.md:262 (after batch 9) | migrated | graph | 12 | 8 | 2 |
| 07-...-l340 | ...-roadmap-2026.md:363 | migrated | graph | 7 | 8 | 0 |
| 08-...-l368 | ...-roadmap-2026.md:401 | migrated | graph | 6 | 7 | 0 |
| 09-...-l407 | ...-roadmap-2026.md:449 | migrated | graph | 9 | 6 | 3 |
| 10-...-l485 (Gantt) | ...-roadmap-2026.md:529 | kept | - | - | - | - |

Every `.vdg` was the importer's output: `diff -q` identical to `corpus-roundtrip/<stem>.vdg`, no `unsupported:` line, `vox diagram check` exit 0. No id renamed, no allowlist entry added. Only 06 printed `dropped:` lines: `06-...-l238.mmd:3: dropped: subgraph-direction` and `:12: dropped: subgraph-direction`. Fences were replaced 06, 05, 04 (batch 9) and 09, 08, 07 (batch 10), bottom up. The `git diff` of the doc across both batches contains no line mentioning `gantt`.

Doc lint scoped to the file: `lint complete — no hard errors.` (both batches). Built site (`pnpm --dir docs-astro build`, then `tests/vox-diagram.spec.ts` + `tests/build-output/diagrams.spec.ts`): 211 passed both times. The page carries 3 `vd-figure` after batch 9 and 6 after batch 10. Baseline lists exactly 1 `docs/src/` entry, `deep-research-full-surface-audit-and-roadmap-2026.md: 1`.

## Contact-sheet review (every PNG read: 6 in batch 9, 6 in batch 10)

PNGs under `/Users/brbrainerd/dev/vox-voxdiagram/target/vd-migration/b9/` and `b10/` (`<stem>.png` and `<stem>-column.png`). The tall column PNGs show the sticky site header drawn across the figure (element-screenshot artefact, noted in 90-02).

| stem | doc:line | Mermaid shows | VoxDiagram shows | defects | disposition |
|---|---|---|---|---|---|
| 04 | :46 | TD, 17-step pipeline, cache diamond with Hit left / Miss right | same 17 nodes and 16 edges; Miss on the left, Hit on the right (sides swapped), Hit/Miss labels on their edges, the whole 15-step spine a straight vertical with no overlaps; all labels whole | none | observations: sides of the diamond swapped, diamond flatter, Mermaid fills not carried |
| 05 | :217 | LR written, but Mermaid draws the single container top-to-bottom (decide diamond in the middle, two outcomes below) | LR as written: Stage Request, diamond, two outcome boxes at the right, Success / None-Fail labels on their edges, inside one frame | none | observation: layout differs (follows the written LR direction); the container title wraps onto two lines although the frame is wide (readable, no split word) |
| 06 | :238 | TD, two stacked containers (Local Models above, OpenRouter below), UserQuery above, Web/Local Search to the right, orphan boxes at the container sides | all 12 nodes and 8 edges, both containers, titles whole (wrapped), all labels whole; chain L2 to L3 to L4, then one edge down into C2 and the C2 to C4 to C5 chain; UserQuery and Web edges read cleanly; orphans (L5, C1, C3) sit beside the chains | none | see "Block 06" below |
| 07 | :340 | TD fan-out to three gatherers, fan-in to the synthesizer, verdict below | same 7 nodes and 8 edges, fan-out and fan-in symmetric, no bundles, labels whole | none | observation: left/right order of the three gatherers reversed |
| 08 | :368 | TD, query fans to three, two merge into the sandbox node, Local Codebase Scan joins at the end | same 6 nodes and 7 edges; the far-right edge to Local Codebase Scan runs clear of the middle nodes; all labels whole | none | observation: order of the two upper boxes swapped |
| 09 | :407 | LR, three containers (Future Retrieval top, Active Research Run and Storage bottom), cross-container edges | same three containers, 9 nodes, 6 edges; the retrieval chain in its own frame above, run frame left and storage frame right of it; the Synthesize edges cross the run frame's edge and reach their targets; titles whole | none (deferred class only) | observations: containers placed side by side; storage title wraps; the two Synthesize edges leave the node at its top-right and bottom-right corners (deferred class a) |

### Block 06: dropped subgraph directions

The importer dropped two `direction TB` lines (one per subgraph, `.mmd` lines 3 and 12). The root direction is `flowchart TD`, so the dropped `direction TB` equals what the containers get from the root anyway: the Mermaid render is also top-to-bottom inside both subgraphs. The drop is a no-op for this figure. Judgement: clearly readable, migrated, no allowlist entry, no hand restructuring. PNG: `/Users/brbrainerd/dev/vox-voxdiagram/target/vd-migration/b9/06-deep-research-full-surface-audit-and-roadmap-2026-l238.png` (column view: `...-l238-column.png`). Observation recorded for the grammar gap itself: VoxDiagram has no per-container direction, so a diagram that *relies* on a differing subgraph direction would lose it; nothing in the current corpus does.

### Deferred classes seen (recorded, not fixed)

- (a) Edge grazing a node corner: stem 06, the NLI Filter to Multi-Source Synthesis edge leaves the bottom-right corner of the NLI Filter box and Query Normalization to Web/Local Search arrives at the top-left corner of the Web box; stem 09, the two Synthesize Research Report edges leave at the node's top-right and bottom-right corners (`b9/06-...png`, `b10/09-...png`).
- (b) Nodes on a container frame's bottom edge / tall empty title band in top-to-bottom containers: stem 06, the Local Models frame has a band of about 30 px below its title before the first node row; no node touches a bottom edge here.

## Mutation and negative proofs

None needed: no renderer or tool change in this plan; the fidelity tool and contact-sheet generator were proven in 90-01. The batch-9 report doubled as the mixed-fence pairing check, and the Gantt row stayed `kept` (body equal to the frozen block byte for byte) in both reports.

## Deviations from Plan

None - plan executed exactly as written. No `fix(90)` commit; no golden re-blessed; earlier stems' contact sheets not rechecked (no renderer change).

### Process notes

- Each batch's verification ran as one backgrounded chain writing a status file under `target/vd-migration/bN/`, then a foreground `until grep -q done` wait. No scratch runner script.
- The commit ledger of protocol 0c was not created; `commits: 2` is `git rev-list --count d25a649ae..HEAD` measured before the SUMMARY commit. `actuals.tokens` is a rough chars/4 over the non-planning diff.

## Plan-level checks

- `git diff --quiet 4fc8a42d7 -- Cargo.lock crates/vox-diagram/Cargo.toml docs-astro/package.json docs-astro/pnpm-lock.yaml` exits 0; `git diff --quiet 4fc8a42d7 -- docs/src/archive` exits 0.
- `grep -c '^```vox-diagram$'` on the doc prints 6; `grep -c '^```mermaid'` prints 1 (the Gantt).

## Known Stubs

None.

## Threat Flags

None.

## Unverified

- Hosted CI (docs-quality with DejaVu/Liberation metrics) not run; carried to the milestone PR.
- `docs-astro` `pnpm check` and the full `cargo nextest run -p vox-diagram` were not run (no source change in this plan).
- The two deferred classes above are recorded for the Phase 90 code review.

## Self-Check: PASSED

Commits 1d8c51f38 and f249adfc5 are ancestors of HEAD; baseline lists 1 file (1 fence); fidelity ends `migrated 32, pending 0, kept 1, failures 0`; no change to Cargo.lock, crates/vox-diagram/Cargo.toml, docs-astro/package.json, pnpm-lock.yaml since 4fc8a42d7; `docs/src/archive` diff empty.
