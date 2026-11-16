---
phase: 89-docs-astro-integration
fixed_at: 2026-10-26T00:00:00Z
review_path: .planning/workstreams/voxdiagram/phases/89-docs-astro-integration/89-REVIEW.md
iteration: 1
findings_in_scope: 8
fixed: 8
skipped: 0
status: all_fixed
---

# Phase 89: Code Review Fix Report

**Fixed at:** 2026-10-26
**Source review:** .planning/workstreams/voxdiagram/phases/89-docs-astro-integration/89-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 8 (the blocker and all 7 warnings)
- Fixed: 8
- Skipped: 0
- Info, outside scope: 4 trivial ones fixed (IN-02, IN-04, IN-07, IN-08); 4 skipped (IN-01, IN-03, IN-05, IN-06)

Worktrees were disabled (`workflow.use_worktrees: false`). Every fix and every verification ran in the main checkout `/Users/brbrainerd/dev/vox-voxdiagram` on `milestone/v1.7-voxdiagram`.

## Fixed Issues

### CR-01: Nested containers are drawn several times larger than their contents (D3)
### WR-02: The `emit` edge strikes through a container title (D1)

Both findings share one root cause, so one commit fixes both.

**Files modified:** `crates/vox-diagram/src/layout/{dagre_impl,finish,input,mod,verify}.rs`, `src/layout/tests.rs`, `src/layout/tests/{containers,coords}.rs`, `src/layout/verify/tests.rs`, `src/svg/shapes.rs`, `tests/svg_geometry_props.rs`, goldens in `tests/fixtures/svg/{cases,corpus}` (new case `22-stress-nested.{vdg,svg}`), `docs/src/architecture/{voxdiagram-layout-go-no-go-2026,where-things-live}.md`
**Commit:** 07b66a573
**Applied fix:**
- Each container title is now a dagre node, sized to its wrapped text with the 112% padding floor. Invisible edges rank it before the container's content. They run from the title to each child container's title and to each own node that no edge reaches from inside the container. A cycle that leaves no such node links the title to every own node, and up and left layouts reverse the edges.
  - Top-down layouts put the title above the content.
  - Left-to-right layouts put it in a column left of the content, wrapped at 16 cells. The title then moves up to the content's top when nothing crosses it there.
  - Because the title is a node, dagre routes edges beside it and keeps nodes off it.
- A frame is now the box around what the container holds, padded 8 px on every side. That is the title box, its own nodes, its child frames, labels of edges inside it, and any entering-edge label that would otherwise straddle it. Frames are never dagre's border rectangle, and the strut and `fit_title` are gone.
- `edgesep` now stays below `nodesep` on every rung: 40/60/20, 50/90/25 and 60/140/30.
- The verifier checks the title box (nodes, foreign frames and edge labels must stay off it). An edge segment through a title box is a preference, not a hard rule: it escalates to the next rung, and if every valid rung has one, the first valid rung is drawn instead of failing the diagram.
- Tests (red on the old code first):
  - `nested_frames_hug_their_children`: on 21-nested, the reference stress fence and 20-container, every side gap is at most 24 px and frame area is at most twice the children's box.
  - `no_edge_strikes_through_a_container_title`.
  - Escalation and fallback tests with engine doubles.
  - `title_links` unit tests.
  - A pin that `22-stress-nested.vdg` is the reference page fence verbatim.
  - A property-test oracle that bounds edges through a title at 6 of 128.
- Mutation-proven:
  - Without the title edges, both container tests fail.
  - With padding at 60 instead of 16, the compactness test fails.
  - Ignoring title crossings in the rung choice fails the escalation test.
- Measured:

  | Diagram | Before (px) | After (px) |
  |---|---|---|
  | Reference stress figure | 1696 x 1482 | 1284.5 x 536 |
  | 21-nested | 1147 x 1247 | 1805 x 382.5 |
  | 20-container | 407 x 518 | 311.5 x 488 |
  | Corpus 13 | 1563 x 2550 | 1248.5 x 2070 |
  | Corpus 03 | 1024 x 999.5 | 1157.5 x 1071 |

  - Every corpus diagram with containers now settles on rung 0 with no edge through a title.
  - Generated compound graphs (1,024 cases): 0 fail closed (was 4 to 8%), and 5 keep an edge through a title.

**Status:** fixed: requires human verification. One layout decision is new: in left-to-right and right-to-left layouts the title sits top-left in its own column instead of in a full-width band. This makes those diagrams wider, as the 21-nested and corpus 03 widths show. The before/after renders were inspected by eye.

### WR-04: The padding floor is bypassed for container titles

**Files modified:** `crates/vox-diagram/src/layout/input.rs`
**Commit:** 15ff79ba1
**Applied fix:**
- Commit 07b66a573 already removed both bypassing paths: the 260 px strut cap and the frame re-wrap in `fit_title`.
- This commit adds the regression test the review asked for: a full 28-cell line of `W` and of `m` keeps the floor.
- Mutation: restoring the 520 half-px cap fails the test (a box of 520 for a line of 800).

### WR-01: A miss is fail-open on the next md build; `clearStaleStore` is dead

**Files modified:** `docs-astro/astro.config.mjs`, `docs-astro/src/plugins/vox-diagram-integration.mjs` (new; the integration moved out of the config), `docs-astro/scripts/render-diagrams.mjs`, `docs-astro/tests/unit/render-diagrams.test.mjs`, `docs-astro/tests/unit/vox-diagram-rebuild.test.mjs` (new)
**Commit:** 8591c5d12
**Applied fix:**
- The review's suggested `rmSync('./.astro/data-store.json')` would not have worked. A build reads its content store from Astro's `cacheDir`, which is `node_modules/.astro/data-store.json`, while `.astro/` holds only the dev store.
- Every build now writes a pending marker in `cacheDir` and removes it only after a clean `astro:build:done`. A build that finds the marker drops the build store first, so every page is rendered and checked again.
- Misses now reset at the start of each build.
- New test: two real `astro build` runs on a one-page project with no manifest, and both must fail. Mutation: without the store drop, the second build passes.
- The dead `clearStaleStore`, its false comment and its test are removed.

### WR-05: The pre-render walks only `docs/src`, but mounted repo docs are rendered too

**Files modified:** `crates/vox-doc-pipeline/src/pipeline/render_manifest.rs`, `render_manifest/tests.rs` (tests split out to keep the file under 400 lines), `docs-astro/scripts/render-diagrams.mjs`, `docs-astro/src/plugins/remark-vox-diagram.mjs`, `docs-astro/tests/unit/{render-diagrams,remark-vox-diagram}.test.mjs`, `docs/src/architecture/where-things-live.md`
**Commit:** e03d19732
**Applied fix:**
- `--render-diagrams` takes repeatable `--mount <repo path>`. Only repo-relative `.md`/`.mdx` files are accepted; anything else is a `diagram-mount` error.
- `pnpm diagrams` passes every file `repo-mounts.mjs` discovers, so mount discovery keeps a single implementation. The real run passed 22 mounted files and rendered cleanly.
- A miss on a mounted page now names its `mounted_from` source or its mirror path, never `docs/src/repo/...`.
- Tests:
  - Rust: mounts are rendered and attributed to their repo path, and a bad mount is an error. Both were red before the change. Mutation: removing the path guard lets a real file outside the root through.
  - JS: the wrapper's real argv carries every mount (an injectable spawn). Mutation: dropping the mount args fails the test.
  - Plugin: the message names the right path.

### WR-03: Every figure is a focusable `region` named "Diagram"

**Files modified:** `docs-astro/src/plugins/remark-vox-diagram.mjs`, `docs-astro/src/plugins/vox-diagram-integration.mjs`, `docs-astro/tests/unit/remark-vox-diagram.test.mjs`, `docs-astro/tests/vox-diagram.spec.ts`
**Commit:** 8b9c75a5e
**Applied fix:**
- No grammar change. The plugin titles each SVG "<nearest preceding heading>, diagram <n>", or "Diagram <n>" before any heading. That title names both the image and the figure, for example "Layout stress examples, diagram 3".
- The figure markup no longer sets `role` or `tabindex`. A page script injected by the integration makes only a figure that scrolls a focusable region, and re-checks on resize.
- The spec now requires:
  - a tab stop and a region exactly when the figure scrolls;
  - a name other than "Diagram";
  - distinct names on each page.
- Mutation: putting the old markup back into the page fails the tab-stop, region and name assertions.

### WR-06: The contrast check measures strokes against the page background

**Files modified:** `docs-astro/tests/vox-diagram.spec.ts`
**Commit:** c0624770e
**Applied fix:**
- Each stroke is now checked against three backgrounds: the page backdrop, the fill of every container its box overlaps, and a shape's own fill.
- The selector is scoped as `svg.vd-diagram :is(...)`.
- A new self-test moves only `--vd-container-fill` to the stroke colour and requires failures. It was red against the old measurement.
- Measured minimum stroke contrast: dark 4.49 (previously reported as 5.27 against the page), light 6.06.

### WR-07: The browser fit check is nearly vacuous for container titles

**Files modified:** `docs-astro/tests/vox-diagram.spec.ts`
**Commit:** 56177529b
**Applied fix:**
- A container title must now sit at least 4 units inside its frame. It must stay clear of nodes, edge labels, other titles and nested frames, and no edge stroke may pass through its text box (checked with `isPointInStroke`).
- New self-test: it moves a title onto an edge and requires a failure.
- Run on the pre-fix reference stress SVG, the check reports "Operations and observability..." as struck through; the fixed SVG is clean.
- Mutations:
  - Disabling the edge scan fails the self-test.
  - The wide-diagram policy test fails at both viewports when the CSS `min-width` is overridden.
- The Rust compactness oracle the review also asked for is part of CR-01.

### Info fixed (trivial)

**Commit:** 3b41164d6
- IN-02: In a build, a `vox-diagram` fence with extra info-string words is now a recorded miss that throws, matching the Rust `diagram-fence-info` rejection. Dev mode still leaves it alone. Mutation-proven.
- IN-04: A comment now says `PREFIX_RE` only guards a future `pageSlug` change.
- IN-07: `customSelectors` is now commented as a backstop.
- IN-08: The reference page and the decision record now say the browser checks run in the docs-quality workflow, not on every build, and that the hosted run is not yet confirmed green.

## Skipped Issues (Info, outside the requested scope)

### IN-01: The pre-render runs twice in both docs workflows

**File:** `.github/workflows/docs-quality.yml:151-157`, `.github/workflows/docs-deploy.yml:96-97`
**Reason:** It changes CI steps that have never run on hosted CI. After WR-05 the explicit `cargo run ... --render-diagrams` step covers `docs/src` only. It is harmless, because `prebuild` re-renders the full manifest with mounts, but it is redundant. Dropping it, or switching it to `pnpm diagrams`, needs a `workflow-paths.test.mjs` update.
**Original issue:** The explicit step is followed by `prebuild`, which runs the same command again.

### IN-03: The SVG parser has a tag allowlist but an attribute denylist

**File:** `docs-astro/src/plugins/remark-vox-diagram.mjs:38-49`
**Reason:** Not trivial. A per-tag attribute allowlist must mirror the writer's grammar and be tested against every golden.
**Original issue:** A future `url(#...)` attribute other than `marker-end` would pass and not be re-prefixed.

### IN-05: No successful-render test for MDX

**File:** `docs-astro/tests/unit/remark-vox-diagram.test.mjs`
**Reason:** Not trivial. It needs `@mdx-js/mdx` resolved through `@astrojs/mdx` and a JSX fence case added to the parity corpus.
**Original issue:** Only the MDX miss path is exercised.

### IN-06: Edge-label backgrounds use the page colour inside containers

**File:** `crates/vox-diagram/src/svg/paths.rs:130`
**Reason:** A cosmetic renderer change that would re-bless the goldens a second time.
**Original issue:** Labels inside containers show as page-coloured patches on the container fill.

## Verification

All gates ran in the main checkout.

- `cargo test -p vox-diagram`: all pass. `limits_adversarial::unclosed_gt_openers_scale_linearly`, a Mermaid-parser timing ratio unrelated to layout, flaked under a loaded parallel run and passed 3 of 3 alone.
- `cargo clippy -p vox-diagram --all-targets -D warnings`: clean.
- `cargo test -p vox-doc-pipeline render_manifest`: 11 pass. `cargo clippy -p vox-doc-pipeline` reports 27 errors, all in files this change does not touch (`src/claims/*`).
- `node --test docs-astro/tests/unit`: 970 of 971 pass. The one failure, `tutorial-record` (blob hash of `docs/src/tutorials/tut-workflow-durability.md`), predates this change and is unrelated.
- After `pnpm diagrams && pnpm build` (844 pages), Playwright `tests/vox-diagram.spec.ts` and `tests/build-output/diagrams.spec.ts`: 21 pass.
- The review bundle was regenerated. `04-light-natural.png` shows the nested figure at 1284.5 x 536 px (was 1696 x 1482), with titles top-left and the `emit` edge clear of the Operations title.

## Follow-up: the left-to-right regression from CR-01/WR-02 (verification gap)

89-VERIFICATION raised a defect that 07b66a573 introduced. A gap-closure fix followed.

**Commit:** 40e95fba9 `fix(89): lay sideways container titles in their content's ranks`
**Files modified:** `crates/vox-diagram/src/layout/{dagre_impl,finish,input}.rs`, `src/layout/tests/containers.rs`, goldens `cases/21-nested.svg`, `cases/22-stress-nested.svg`, `corpus/03-axis-harness-river-design-2026-l83.svg`, `docs/src/architecture/voxdiagram-layout-go-no-go-2026.md`

**Root cause.** 07b66a573 ranked every title before its content with invisible edges. In left-to-right and right-to-left layouts this put each title in a rank of its own. That rank is also the last rank of the container feeding it. Dagre keeps each cluster contiguous within every rank it spans, so two containers that share a rank must be stacked across the flow. Each fed container moved into its own band, offset from the one before it, and corpus 03 became a staircase. Separately, the 16-cell side column (`SIDE_TITLE_CELLS`) hard-broke any longer word, for example `HarnessRiverPane` / `l`.

**Design.**
- A sideways title has no edges, so its container spans only its content's ranks.
- Two dagre behaviours, read from the dagre 0.1.1 source, place it:
  - Ranking leaves a cluster member with no constraints in the cluster's latest rank (longest-path placement, which network simplex never moves at zero cost).
  - Ordering keeps a node with no neighbours at its insertion index. Titles are inserted before every other node, so the title is first, at the top, in that rank.
- A sideways diagram with containers is solved mirrored: the opposite direction, on reversed edges, with the points reversed back. Dagre's latest rank is then the flow's first column, so the title sits on top of the content's first column. Right-to-left mirrors the same way, so its titles sit top-right, over the flow's first column.
- A side effect: dagre aligns long edges with their (mirrored) source, which is the real target. Edges therefore tend to reach a node under a title head-on. Without the mirror, the right-to-left stress figure's `emit` edge struck through the Operations title.
- Titles wrap at the normal node width in every direction.
- Top-down and bottom-up layouts are unchanged: they keep their title links. Their goldens are byte-identical. Diagrams without containers are not mirrored.

**Tests (red on 56592855d first):**
- `a_title_never_breaks_a_word_that_fits_a_line`: a title holding HarnessRiverPanel, EnforcementLayers and Inbound_discovery_planned keeps every word whole in all directions. On HEAD it failed with `["HarnessRiverPane", "l", ...]`.
- `sideways_containers_follow_the_flow`: on corpus 03, drawn left-to-right and right-to-left, for each of the 11 edges between top-level containers, the feeding container's frame lies wholly before the fed one along the flow axis. The test first asserts that no pair feeds each other. On HEAD it failed: Controls x 16..450 px, while Store, which Controls feeds, spans 280..823. This property encodes the root cause directly: a title rank shared with the feeding container forces dagre to stack the two across the flow.
- Also added:
  - `sideways_river_frames_hug_and_titles_top_them`: compact frames, no edge through a title, and each title box at the top of its frame.
  - Right-to-left variants of 21-nested and the stress figure in the compactness and title-crossing tests.
  - `nested_figures_stay_compact`: a viewBox-area ceiling 10% above the new measurements, in both sideways directions.
- Mutations (each restored afterwards; the diff was checked):
  - Re-enabling title links sideways fails the flow test, the title-crossing test and the size ceiling.
  - Dropping the mirror fails the title-crossing test (the right-to-left stress figure) and the size ceiling.
  - Inserting titles after the clusters fails the compactness, crossing and size tests.
  - Restoring the 16-cell wrap fails the word test.
  - Padding 60 in place of 16 fails the compactness, title-on-top and size tests.

**Measured (px):**

| Diagram | Pre-CR-01 | 07b66a573 | Now |
|---|---|---|---|
| Corpus 03 | 1024 x 999.5 | 1157.5 x 1071 | 964 x 761 |
| Reference stress figure | 1696 x 1482 | 1284.5 x 536 | 1392.5 x 446 (area -10%) |
| 21-nested | 1147 x 1247 | 1805 x 382.5 | 1016.5 x 802 (area +18%, 57% of pre-CR-01) |

- The total viewBox area of every corpus and case diagram with containers, left-to-right and right-to-left, fell 16% (13.56M to 11.44M px2).
- Edge segments through a node box, counted over all 30 corpus graphs, three container cases and their right-to-left variants: 33 before, 25 now. 21-nested gains one: its `up` edge runs through `Source`. Corpus 03 loses one, its right-to-left variant loses eight, and 09 loses one.
- Generated compound graphs, 1,024 cases: 0 fail closed (was 1), and 6 keep an edge through a title (was 6 to 7). Only one of those is sideways. The 128-case oracle varies from run to run between 0 and 4, under its bound of 6.

**By eye (Chromium renders):**
- Corpus 03 is three columns again, each title a band on top of its frame. `HarnessRiverPanel (Dockable` / `+ Full Surface)` is whole, and the edges flow left to right. The right-to-left variant mirrors it cleanly.
- The stress figure: Platform, Core and Persistence titles sit top-left and stack as nested bands. The `emit` edge reaches Metrics horizontally, under the four-line Operations title.
- Reference figure 01: Desktop Client now sits left of the Orchestrator container, not below it.
- 21-nested is more compact than before CR-01 and narrower than 07b66a573, but taller. One long edge passes behind `Source`. That is a soft defect, which `verify` does not check by design.

**Status:** fixed: requires human verification. This is a layout heuristic built on dagre's internal ranking and ordering rules. The tests pin its effects (flow order, titles on top, compactness, no crossing), not the mechanism.

**Verification** (all in the main checkout, worktrees disabled):
- `cargo nextest run -p vox-diagram`: 597 of 597 pass.
- `cargo clippy -p vox-diagram --all-targets -D warnings`: clean.
- `cargo nextest run -p vox-doc-pipeline -E 'test(render_manifest)'`: 11 of 11 pass.
- The doc lint on the decision record is clean.
- `pnpm diagrams && pnpm build`, then Playwright `tests/vox-diagram.spec.ts` and `tests/build-output/diagrams.spec.ts`: 22 of 22 pass. Fits: narrow stack widest fill 82.9%, wide stack 95.7%. The review bundle was regenerated and 04 and 01 were inspected.

---

_Fixed: 2026-10-26_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
