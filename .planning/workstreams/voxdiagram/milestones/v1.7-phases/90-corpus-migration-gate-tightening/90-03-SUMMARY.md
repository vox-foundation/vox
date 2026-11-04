---
phase: 90-corpus-migration-gate-tightening
plan: 03
subsystem: docs
tags: [vox-diagram, mermaid-migration, fidelity, sequence-diagram, containers, layout]

requires:
  - phase: 90-02
    provides: "13 of 32 convertible fences migrated, baseline at 20 fences / 8 files, scoped figure id check"
provides:
  - "7 more migrated fences (20 of 32 convertible), baseline at 13 fences / 4 files"
  - "the corpus's only sequence diagram (scientia, 8 actors, 9 events) migrated and reviewed"
  - "renderer fix: a container's title is ranked after the nodes that feed it (no more side-by-side frames)"
affects: [90-04, 90-05, 90-06]

actuals:
  tokens: 3500
  tasks: 2
  commits: 3
plan_head_before: 43a719ae9
plan_head_after: 3f1204e02
commits: 3

tech-stack:
  added: []
  patterns:
    - "title_links also links each outside feeder of a container's entry nodes to its title, so the title (and the frame) lies after the feeder in the flow"

key-files:
  created:
    - crates/vox-diagram/src/layout/tests/feeds.rs
  modified:
    - crates/vox-diagram/src/layout/dagre_impl.rs
    - crates/vox-diagram/src/layout/tests.rs
    - crates/vox-diagram/src/layout/tests/containers.rs
    - crates/vox-diagram/tests/fixtures/svg/cases/20-container.svg
    - crates/vox-diagram/tests/fixtures/svg/corpus/06-deep-research-full-surface-audit-and-roadmap-2026-l238.svg
    - crates/vox-diagram/tests/fixtures/svg/corpus/13-deep-research-waves-and-competitive-analysis-research-2026-l110.svg
    - docs/src/architecture/vox-gui-browser-support-2026.md
    - docs/src/architecture/vox-react-backend-interop-audit-2026.md
    - docs/src/architecture/unified-workstream-execution-plan-2026.md
    - docs/src/architecture/scientia-automated-research-historical-extension-research-2026.md
    - contracts/documentation/legacy-mermaid-baseline.v1.yaml

key-decisions:
  - "A container fed only from outside is laid after its feeder along the flow (top-down and bottom-up); sideways layouts are unchanged (they already give titles no edges)"

requirements-completed: []  # MIG-03 / MIG-04 close in 90-06

duration: n/a
completed: 2026-10-27
status: complete
---

# Phase 90 Plan 03: Batches 5 and 6 Summary

Seven fences migrated through the 90-01 procedure, including the only sequence diagram (reviewed clean: labels in their own lifeline gaps, none crossing a lifeline). The contact-sheet review of the scientia two-container flowchart found a staircase with seven bundled edges; fixed red-first in the layout engine, which also improved three existing goldens.

## Commits

| Hash | Message |
|------|---------|
| addbe2330 | docs(90): migrate batch 5 Mermaid fences to vox-diagram |
| f9ec97c97 | fix(90): rank a container's title after the nodes that feed it |
| 3f1204e02 | docs(90): migrate batch 6 Mermaid fences to vox-diagram |

## Fidelity

Report lines: after batch 5 `fidelity: migrated 18, pending 14, kept 1, failures 0`; after batch 6 `fidelity: migrated 20, pending 12, kept 1, failures 0` (also after the fix). Tighten lines (generator only): `tightened: 20 -> 15 fences, 8 -> 5 files`, then `tightened: 15 -> 13 fences, 5 -> 4 files`. Reports: `target/vd-migration/b5/fidelity.md`, `b6/fidelity.md`.

| stem | fence | status | kind | nodes / actors | edges / events | containers |
|---|---|---|---|---|---|---|
| 23-vox-gui-browser-support-2026-l33 | docs/src/architecture/vox-gui-browser-support-2026.md:33 | migrated | graph | 4 | 3 | 0 |
| 24-vox-gui-browser-support-2026-l50 | docs/src/architecture/vox-gui-browser-support-2026.md:52 | migrated | graph | 7 | 7 | 0 |
| 25-vox-gui-browser-support-2026-l92 | docs/src/architecture/vox-gui-browser-support-2026.md:99 | migrated | graph | 4 | 3 | 0 |
| 26-vox-react-backend-interop-audit-2026-l51 | docs/src/architecture/vox-react-backend-interop-audit-2026.md:51 | migrated | graph | 11 | 12 | 0 |
| 21-unified-workstream-execution-plan-2026-l45 | docs/src/architecture/unified-workstream-execution-plan-2026.md:45 | migrated | graph | 14 | 17 | 0 |
| 18-scientia-automated-research-historical-extension-research-2026-l50 | docs/src/architecture/scientia-automated-research-historical-extension-research-2026.md:50 | migrated | graph | 10 | 7 | 2 |
| 19-scientia-automated-research-historical-extension-research-2026-l112 | docs/src/architecture/scientia-automated-research-historical-extension-research-2026.md:113 | migrated | sequence | 8 | 9 | - |

(Rows copied from `b5/fidelity.md` and `b6/fidelity.md`.) Every `.vdg` was the importer's output: `diff -q` identical to `corpus-roundtrip/<stem>.vdg`, `vox diagram check` exit 0, no `unsupported:` or `dropped:` line. No id renamed, no allowlist entry added. The sequence row keeps 8 actors and 9 ordered events (8 messages, 1 note), checked by `canon_sequence` equality with the oracle-proven golden plus the export/import round trip.

Doc lint scoped to each batch: `lint complete — no hard errors.` Built site: `pnpm --dir docs-astro build`, then `tests/vox-diagram.spec.ts` + `tests/build-output/diagrams.spec.ts`: batch 5 166 passed; batch 6 175 passed, again 175 passed after the fix. The vox-gui page carries 3 `vd-figure`s, the scientia page 2 with a `vd-lifeline` sequence svg; the baseline lists 5 then 4 `docs/src/` entries.

## Contact-sheet review (every PNG read)

PNGs under `/Users/brbrainerd/dev/vox-voxdiagram/target/vd-migration/` (`b5/`, `b6/` after the fix; `b6-pre/` before it; `<stem>.png` and `<stem>-column.png`). Column PNGs of wide figures are clipped on the right where the real page scrolls (Phase 89 design).

| stem | doc:line | Mermaid shows | VoxDiagram shows | defects | disposition |
|---|---|---|---|---|---|
| 23-vox-gui (preview flow) | vox-gui-browser-support-2026.md:33 | LR chain of 4 purple boxes | same 4 nodes, 3 edges, LR | none | observation: Mermaid fill not carried |
| 24-vox-gui (agent live view) | vox-gui-browser-support-2026.md:52 | LR, Operator input above, back edge from the live view to the daemon curving below | same 7 nodes and 7 edges; back edge drawn straight above the second row, Operator edge into the live view from above; no node crossed | none | observation: routing differs |
| 25-vox-gui (click) | vox-gui-browser-support-2026.md:99 | LR chain of 4, last label on two lines | same, last label on one line | none | observation: wrapping differs |
| 26-vox-react-backend-interop | vox-react-backend-interop-audit-2026.md:51 | LR fan-out from HIR, three edges into External React app | same 11 nodes and 12 edges, tanstack edge runs along the top and down into the React box, no overlap | none | observation: vertical order of the branches differs |
| 21-unified-workstream | unified-workstream-execution-plan-2026.md:45 | TD, 5 wave rows, "Wave 4: Phases 35-36" in its own row below Wave 3 | same 14 nodes and 17 edges, all labels whole; "Wave 4: Phases 35-36" shares the Wave 3 row (its predecessors sit one row higher); Wave 1/Wave 2 rows jitter a few px with box height; edges cross where Mermaid's cross | none | observations: row alignment differs slightly, crossings as in the source |
| 18-scientia flowchart | scientia-...-2026.md:50 | two stacked yellow containers, edges between them | first render (`b6-pre/18-...png`): frames side by side, the fed container ("Vox SCIENTIA layers") starting beside the node row of "Historical eras" and shifted right, its content drifting diagonally; the 7 container-to-container edges squeezed through one shared horizontal channel under the eras frame with V kinks, impossible to tell which goes where | staircase of the two frames; crowded, bundled edges | fixed in f9ec97c97; render after the fix (`b6/18-...png`, `-column.png`): eras frame above, vox frame below, the 7 edges fan out and drop to their targets separately, all readable, titles clear |
| 19-scientia sequence | scientia-...-2026.md:113 | 8 lifeline boxes top and bottom, 8 labelled arrows plus a yellow note over vox_research_shim..vox_scientia | same 8 actors in the same order, 8 solid arrows with arrowheads, same direction (Researcher to vox_scientia points left), note over vox_research_shim..vox_scientia, no dashing needed (the source has no dashed message) | none | see sequence review below |

### Sequence review (stem 19)

- Actor order: CodingAgent, vox_research_shim, vox_search, OpenRouter, Tavily, vox_scientia, Researcher, NanoPub_Zenodo_Manuscript, as in the source; lifelines dashed, drawn from the head boxes only (no bottom copy of the boxes).
- Message direction: all 8 arrows point at the receiver; "ApprovalToken" (Researcher to vox_scientia) points left. No dashed replies exist in the source, none drawn.
- Note: a grey box spanning vox_research_shim to vox_scientia, drawn above the three middle lifelines, text centred and whole; sits about 10 px under the "claim_extractor optional paths" arrow and clear of the next one. Mermaid's yellow fill is not carried.
- Multi-hop label placement: each label sits in the first lifeline gap of its arrow ("planner + synthesize", "CRAG multi-hop", "Tier4 search optional", "claim_extractor optional paths", "nanopub / archive / syndicate"), above the line, left-aligned toward the start, and never crosses another lifeline. Mermaid centres these labels on their arrows, so its "planner + synthesize" and "Tier4 search optional" labels sit across the vox_search and OpenRouter lifelines (visible in the Mermaid panel); VoxDiagram is cleaner here.
- Observations (not defects): note tightly under the preceding arrow; different look from Mermaid; the column PNG is clipped on the right by the 0.6 scale floor and scrolls on the real page.

### Remaining observation on stem 18 (not fixed)

After the fix, the long, shallow route of two edges runs close to neighbouring boxes inside the eras frame: the "1960s-80s" to "Retrieval" edge leaves the left side of its box and clips the bottom-right corner of the "2018-2023: Semantic search" box over a few px, and the "1990s-2000s" edge does the same under "2010s: Reproducibility". Every edge can still be traced from source to target and no label is touched, so I recorded it as an observation. It is a property of dagre's edge points; `layout/verify.rs` has no edge-versus-node check (only node, label and title checks), so fixing it means a new verification rule and rung escalation that would re-bless many goldens. See Unverified. Also visible in the same figure: a ~60 px empty band between the eras title and its nodes (existing title-rank spacing, within the Phase 89 `nested_figures_stay_compact` ceilings) and the era order shuffled by crossing minimisation (Mermaid shuffles it too).

Rechecks after the fix: contact sheets for all 18 earlier stems (batches 1 to 5; batches 1 to 4 have no containers, so their layout path is untouched) regenerated into `target/vd-migration/recheck/{1,3,5}/` and `diff -q`ed against the earlier PNGs: 34 identical, and stem 30's two PNGs identical to the post-wrap-fix `b2-recheck/` ones (the original `b2/30-*` predate the 90-01 fix). Stem 19's `-column.png` changed bytes only; re-read, same content. Only stem 18's PNGs changed in substance; re-read.

## fix(90) commit

`f9ec97c97`. Red first: `layout::tests::feeds::a_fed_container_lies_wholly_after_its_source_container` failed on the unfixed code (`down: Rect { x: 32, y: 32, w: 2356, h: 360 } feeds Rect { x: 2516, y: 286, w: 2472, h: 398 }`: the fed frame starts at y 286, inside the source frame's 32..392, and sits wholly to its right), then passed. Fix: in `dagre_impl::title_links`, link each outside feeder of a container's entry nodes to the container's title (top-down feeder to title; bottom-up title to feeder), so the title is never in the feeder's rank. Unit test `titles_link_to_child_titles_and_to_nodes_entered_only_from_outside` extended (top-down feeder link, plus a bottom-up case). Goldens re-blessed with `UPDATE_GOLDEN=1` on `svg_render` and `svg_corpus`; the three changed files were each rendered before and after in Chromium and read:

- `svg/cases/20-container.svg` (311.5x488 to 275x578): Client above the Backend frame instead of beside it, "request" edge dropping in; no overlap.
- `svg/corpus/06-...-l238.svg` (1262.5x1108 to 892x1320): the OpenRouter frame sits below Local Models rather than beside it, one clean edge between them, narrower figure.
- `svg/corpus/13-...-l110.svg` (1248.5x2070 to 1216.5x2168): the engine frame now below the triage node instead of beside it; the Tier 2 and Tier 3 labels sit in the frame's top row next to the title text without touching it.

Mutation proof: with the feeder loop turned into `.take(0)` (presence check: pattern count 1 before, `take(0)` count 1 after the edit), the new test, the extended unit test, `svg_render::svg_cases` and `svg_corpus::corpus_top10_goldens` all failed; the saved good copy was restored (`diff -q` identical, `take(0)` count 0, fix count 1). Full `INSTA_UPDATE=no cargo nextest run -p vox-diagram`: 600 passed, 4 skipped (before the fix the same run failed only the two golden tests); `cargo clippy -p vox-diagram --all-targets -- -D warnings` clean; `rustfmt --check` clean on the touched files.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] A container fed only from outside was laid beside its feeder (staircase, bundled edges)**
- **Found during:** Task 2, contact-sheet review (P6/P7), stem 18
- **Issue:** `title_links` ranked the title before the content but not after the outside feeder, so the title took the feeder's rank and dagre set the frames side by side
- **Fix:** link outside feeders to the title (see fix(90) commit)
- **Files modified:** crates/vox-diagram/src/layout/dagre_impl.rs, layout/tests.rs, layout/tests/containers.rs (`laid` made `pub(super)`), layout/tests/feeds.rs (new), three SVG goldens
- **Commit:** f9ec97c97

### Process notes

- Long commands ran as backgrounded chains writing a status file, then a foreground `until grep -q ... ; do sleep 3; done` wait; no scratch runner script. `lean-ctx` rewrites `cat`/`tail` on paths under `target/`, so logs were read with the Read tool (and only tails of the 480-line Playwright log were needed afterwards).
- The commit ledger of protocol 0c was not created; `commits: 3` is `git rev-list --count 43a719ae9..HEAD` with 43a719ae9 (HEAD at plan start). `actuals.tokens` is a rough chars/4 over the non-golden, non-planning diff.
- The before/after Chromium renders of the three re-blessed goldens used a throwaway script in the session scratchpad (not committed).
- The plan expected no Rust change; the fix is the only one, and it is the red-first path the plan's P7 prescribes.

## Known Stubs

None.

## Threat Flags

None.

## Unverified

- Hosted CI (docs-quality with DejaVu/Liberation metrics) not run; carried to the milestone PR.
- `docs-astro` `pnpm check` not run (no spec change in this plan).
- The corner graze on stem 18 (above) is recorded, not fixed; it is a pre-existing class (no edge-versus-node verification exists). If the reviewer counts it as a defect, a follow-up would add the check to `verify.rs` with a rung escalation and re-bless goldens.
- Pending batch fences 04, 06, 11, 12, 13 (containers, top-down) will now render with the fixed ordering; their goldens in `svg/corpus/` were already re-blessed (06, 13) and will be reviewed as their batches run.
- `limits_adversarial::unclosed_gt_openers_scale_linearly` passed in this plan's full run (known load flake not triggered).

## Self-Check: PASSED

Commits addbe2330, f9ec97c97, 3f1204e02 are ancestors of HEAD; `crates/vox-diagram/src/layout/tests/feeds.rs` exists; baseline lists 4 files; no change to Cargo.lock, crates/vox-diagram/Cargo.toml, docs-astro/package.json, pnpm-lock.yaml since 4fc8a42d7; `docs/src/archive` diff empty.
