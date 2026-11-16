---
phase: 90-corpus-migration-gate-tightening
plan: 04
subsystem: docs
tags: [vox-diagram, mermaid-migration, fidelity, containers, left-to-right]
requires:
  - phase: 90-03
    provides: "20 of 32 convertible fences migrated, baseline at 13 fences / 4 files, container-title feeder ranking fix"
provides:
  - "6 more migrated fences (26 of 32 convertible), baseline at 7 fences / 1 file"
  - "corpus 03 (left-to-right containers) and corpus 13 (largest graph) reviewed at shipped scale: no defect"
affects: [90-05, 90-06]

actuals:
  tokens: 3800
  tasks: 2
  commits: 2
plan_head_before: 1d2a206b8
plan_head_after: 69a42885af8055d59cb12304462f4b99950f0fe9
commits: 2

tech-stack:
  added: []
  patterns: []

key-files:
  created: []
  modified:
    - docs/src/architecture/axis-harness-river-design-2026.md
    - docs/src/architecture/vox-compiler-architecture-research-2026.md
    - docs/src/architecture/deep-research-waves-and-competitive-analysis-research-2026.md
    - contracts/documentation/legacy-mermaid-baseline.v1.yaml

key-decisions: []

requirements-completed: []  # MIG-03 / MIG-04 close in 90-06

duration: n/a
completed: 2026-10-27
status: complete
---

# Phase 90 Plan 04: Batches 7 and 8 Summary

Six fences migrated through the 90-01 procedure: axis-harness (left-to-right containers, the Phase 89 regression case), vox-compiler-architecture (left-to-right, three containers) and the four deep-research-waves fences, including corpus 13, the largest graph. Contact-sheet review found no renderer defect other than the two classes deferred to the Phase 90 code review, so there is no `fix(90)` commit and no golden was re-blessed.

## Commits

| Hash | Message |
|------|---------|
| db2fcbbd8 | docs(90): migrate batch 7 Mermaid fences to vox-diagram |
| 69a42885a | docs(90): migrate batch 8 Mermaid fences to vox-diagram |

## Fidelity

Report lines: after batch 7 `fidelity: migrated 22, pending 10, kept 1, failures 0`; after batch 8 `fidelity: migrated 26, pending 6, kept 1, failures 0`. Tighten lines (generator only): `tightened: 13 -> 11 fences, 4 -> 2 files`, then `tightened: 11 -> 7 fences, 2 -> 1 files`. Reports: `target/vd-migration/b7/fidelity.md`, `b8/fidelity.md`.

| stem | fence | status | kind | nodes | edges | containers |
|---|---|---|---|---|---|---|
| 03-axis-harness-river-design-2026-l83 | docs/src/architecture/axis-harness-river-design-2026.md:83 | migrated | graph | 11 | 11 | 4 |
| 22-vox-compiler-architecture-research-2026-l19 | docs/src/architecture/vox-compiler-architecture-research-2026.md:19 | migrated | graph | 8 | 8 | 3 |
| 11-deep-research-waves-and-competitive-analysis-research-2026-l45 | docs/src/architecture/deep-research-waves-and-competitive-analysis-research-2026.md:45 | migrated | graph | 10 | 11 | 0 |
| 12-deep-research-waves-and-competitive-analysis-research-2026-l76 | ...-research-2026.md:87 | migrated | graph | 13 | 16 | 0 |
| 13-deep-research-waves-and-competitive-analysis-research-2026-l110 | ...-research-2026.md:137 | migrated | graph | 18 | 20 | 1 |
| 14-deep-research-waves-and-competitive-analysis-research-2026-l211 | ...-research-2026.md:254 | migrated | graph | 6 | 5 | 0 |

Every `.vdg` was the importer's output: `diff -q` identical to `corpus-roundtrip/<stem>.vdg`, no `unsupported:` or `dropped:` line, `vox diagram check` exit 0. No id renamed, no allowlist entry added. The waves fences were replaced from line 211 upwards.

Doc lint scoped to each batch: `lint complete — no hard errors.` Built site (`pnpm --dir docs-astro build`, then `tests/vox-diagram.spec.ts` + `tests/build-output/diagrams.spec.ts`): batch 7 193 passed, batch 8 202 passed. The axis-harness page carries 1 `vd-figure`, the waves page 4. Baseline lists 2 then 1 `docs/src/` entries (only the deep-research-full-surface doc remains, 7 fences).

## Contact-sheet review (every PNG read: 4 in batch 7, 8 in batch 8)

PNGs under `/Users/brbrainerd/dev/vox-voxdiagram/target/vd-migration/b7/` and `b8/` (`<stem>.png` and `<stem>-column.png`). Column PNGs of wide figures are clipped on the right where the real page scrolls (Phase 89 design), and the tall column PNGs show the sticky site header drawn across the figure (tooling artefact of the element screenshot, noted in 90-02).

| stem | doc:line | Mermaid shows | VoxDiagram shows | defects | disposition |
|---|---|---|---|---|---|
| 03-axis-harness | axis-harness-river-design-2026.md:83 | LR, three stacked-looking columns: Controls above Stream on the left, Store in the middle, UI on the right; `<br/>` labels wrapped in boxes | same four containers: Controls above Stream in column 1, Store tall in column 2, UI tall in column 3, side by side with no staircase (frame tops differ by 20 px steps, as in Mermaid). All titles whole ("Pure Correlation Layer (riverMath.ts)" and "HarnessRiverPanel (Dockable + Full Surface)" wrap at a space, no split word, no edge through any title). All `<br/>` labels wrapped inside their boxes. The 11 edges are separately traceable: deriveTierCascade fan-out to Zone A/B, deriveRiverSnapshot to A, B, C, D, no bundles | none | verified for corpus 03 after 40e95fba9 and f9ec97c97; observations: Mermaid yellow/purple fills not carried, the TC to Zone A edge arcs over the Store frame top and passes about 12 px under the UI frame's second title line without touching it |
| 22-vox-compiler-architecture | vox-compiler-architecture-research-2026.md:19 | LR: Vox_source, LexerAndParser (lex, parse), Analysis (AST_module, HIR_lower, typecheck / validate), Emit | same three containers left to right in one row, same nodes and edges; the typecheck/validate pair is swapped (validate above), edges into codegen arrive at the top of the box clear of the Emit title | none | observation: branch order swapped; column PNG scrolls |
| 11-deep-research | waves-...-2026.md:45 | TD chain with a plan/approve loop and a refine loop | same 10 nodes, 11 edges; the "No" back edge and the forward edge between Plan and Approve are two separate curves with the label between; the refine loop runs up the right side into Google Search; all edge labels on their edges and clear of nodes | none | observations: diamonds flatter, bottom row order swapped relative to Mermaid (Synthesize left), different sizes |
| 12-deep-research | waves-...-2026.md:76 | TD, lead researcher, decomposition, three workers, three note boxes, distill, back edge on the right, reflection diamond with Yes/No | same 13 nodes and 16 edges; Lead above, reflection diamond on the left, workers (N, 2, 1) on the right, back edge from Distill runs under the row and up the left side into Lead | none | observations: worker order reversed, Yes/No sides swapped; the Distill-to-Lead back edge leaves the same side of the Distill box where the Spawn Specialist edge arrives (two lines 10 px apart over ~100 px) but both stay traceable by their arrowheads |
| 13-deep-research (largest, natural 1216 x 2168 px) | waves-...-2026.md:110 | TD, triage diamond with four tier edges, one container with the three-wave engine, tail chain of five | all 18 nodes, 20 edges and the container present; Tier 0 and Tier 1 nodes to the right of the frame, Tier 2/3 edges drop into the frame to Wave 1, tail chain below; all 8 edge labels sit on their edges, none overlaps a node or the container title; column PNG (0.6 scale, scrolls horizontally) reads cleanly: node text and labels legible, no crowding; the long "Sufficient Evidence" and "Sufficient" edges run clear of Wave 2/3 and enter Consolidated Knowledge Subgraph separately | none (deferred class only) | judged readable at shipped scale; sheer size an observation. Deferred (b): "Consolidated Knowledge Subgraph" sits about 4 px from the frame's bottom edge. Deferred (a): the Sandbox-run edge ends at the top-right corner of Consolidated Knowledge Subgraph |
| 14-deep-research | waves-...-2026.md:211 | LR chain of 6, last label on three lines | same chain, last label on three lines, all whole | none | observation: column PNG scrolls |

### Deferred classes seen (recorded, not fixed)

- (a) Edge grazing a node corner: stem 13, the "Run Sandboxed Compiler" to "Consolidated Knowledge Subgraph" edge ends at the node's top-right corner (`target/vd-migration/b8/13-...-l110.png`, `-column.png`).
- (b) Nodes sitting on a container frame's bottom edge: stem 13, "Consolidated Knowledge Subgraph" (same PNGs). There is no tall empty title band in this figure.

## Mutation and negative proofs

None needed: no renderer or tool change in this plan; the fidelity tool and contact-sheet generator were proven in 90-01.

## Deviations from Plan

None - plan executed exactly as written. No `fix(90)` commit; no golden re-blessed; earlier stems' contact sheets did not need rechecking (no renderer change).

### Process notes

- Each batch's verification (fidelity, tighten, lint, build, Playwright, contact sheet) ran as one backgrounded `&&`-free chain writing a status file under `target/vd-migration/bN/`, then a foreground `until grep -q ...` wait. No scratch runner script.
- The commit ledger of protocol 0c was not created; `commits: 2` is `git rev-list --count 1d2a206b8..HEAD` with 1d2a206b8 (HEAD at plan start) as base, measured before the SUMMARY commit. `actuals.tokens` is a rough chars/4 over the non-planning diff.

## Known Stubs

None.

## Threat Flags

None.

## Unverified

- Hosted CI (docs-quality with DejaVu/Liberation metrics) not run; carried to the milestone PR.
- `docs-astro` `pnpm check` and the full `cargo nextest run -p vox-diagram` were not run (no source change in this plan).
- The two deferred classes above are recorded for the Phase 90 code review.

## Self-Check: PASSED

Commits db2fcbbd8 and 69a42885a are ancestors of HEAD; baseline lists 1 file (7 fences); no change to Cargo.lock, crates/vox-diagram/Cargo.toml, docs-astro/package.json, pnpm-lock.yaml since 4fc8a42d7; `docs/src/archive` diff empty.
