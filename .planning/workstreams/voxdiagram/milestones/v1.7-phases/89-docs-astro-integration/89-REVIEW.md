---
phase: 89-docs-astro-integration
reviewed: 2026-10-26T00:00:00Z
depth: deep
files_reviewed: 27
files_reviewed_list:
  - .github/workflows/docs-deploy.yml
  - .github/workflows/docs-quality.yml
  - crates/vox-diagram/src/layout/input.rs
  - crates/vox-diagram/src/layout/seq.rs
  - crates/vox-diagram/src/layout/seq/lanes.rs
  - crates/vox-diagram/src/layout/seq/tests.rs
  - crates/vox-diagram/src/layout/text.rs
  - crates/vox-diagram/src/layout/text/tests.rs
  - crates/vox-diagram/tests/fence_scan.rs
  - crates/vox-diagram/tests/fixtures/fence-keys.json
  - crates/vox-diagram/tests/fixtures/svg/cases/20-container.svg
  - crates/vox-diagram/tests/fixtures/svg/cases/21-nested.svg
  - crates/vox-doc-pipeline/src/pipeline/mod.rs
  - crates/vox-doc-pipeline/src/pipeline/render_manifest.rs
  - docs-astro/.gitignore
  - docs-astro/astro.config.mjs
  - docs-astro/package.json
  - docs-astro/scripts/render-diagrams.mjs
  - docs-astro/src/plugins/remark-vox-diagram.mjs
  - docs-astro/src/styles/vox-diagram.css
  - docs-astro/tests/build-output/diagrams.spec.ts
  - docs-astro/tests/lib/diagrams.ts
  - docs-astro/tests/unit/remark-vox-diagram.test.mjs
  - docs-astro/tests/unit/render-diagrams.test.mjs
  - docs-astro/tests/unit/workflow-paths.test.mjs
  - docs-astro/tests/vox-diagram.spec.ts
  - docs/src/reference/vox-diagram.md
findings:
  critical: 1
  warning: 7
  info: 8
  total: 16
status: issues_found
---

# Phase 89: Code Review Report

**Reviewed:** 2026-10-26
**Depth:** deep (cross-file: remark plugin -> Astro content layer -> built HTML; manifest writer -> fence scanner; layout input -> dagre adapter -> finish -> SVG writer)
**Files Reviewed:** 27
**Status:** issues_found

## Summary

Scope is the whole Phase 89 range (`ed1c69bf1..HEAD`, plans 89-01 to 89-04), not only `18c07fc2f~1..HEAD`. The requested range starts after the 89-01 code commits, and those commits hold the manifest writer, plugin and workflow steps this review was asked to check. Re-blessed SVG goldens were spot-checked for geometry, not diffed line by line. Also out of scope: `.planning/`, generated files and lockfiles. Context files read: `docs/src/architecture/voxdiagram-layout-go-no-go-2026.md` and `where-things-live.md` (both changed in this phase), `crates/vox-diagram/src/layout/{dagre_impl,finish,mod,verify}.rs`, `crates/vox-diagram/src/svg/mod.rs`, Astro 6.1.9 `content/loaders/glob.js`, and `@astrojs/mdx` 5.0.4 `plugins.js`.

What holds up under adversarial reading:

- **Hast insertion has no raw-HTML path.** The SVG goes through `svgToHast`, a strict tokenizer that rejects anything it does not recognise. The plugin retypes the mdast node to `vdFigure` and sets `data.hName`/`hProperties`/`hChildren`. Labels, the fence source and the placeholder are all text nodes. No `html` or `raw` node is created anywhere.
- **Id handling is sound.** Re-prefixing maps exactly `vd-{title,desc,arrow}` and throws on any other id. The slug sanitiser makes `vd-<slug>-<n>` always match `[a-z][a-z0-9-]*`.
- **MDX output is correct.** I compiled an `.mdx` fence through `@mdx-js/mdx` with Astro's `elementAttributeNameCase: 'html'`. The output keeps `marker-end`, `aria-labelledby`, `viewBox`, `tabindex` and `data-pagefind-ignore` as real attribute names, and turns `style` into an object Astro serialises correctly.
- **The manifest writer is deterministic.** Files are walked in sorted order, entries live in a `BTreeMap`, serialisation is fixed, the write is atomic, and archive is skipped. The `pad_x` floor is integer-only and saturating.
- **The workflow edits respect the CI rules.** The `concurrency:` blocks, `timeout-minutes: 30`, `package-manager-cache: false` and the main-only rust-cache all survive. No cache was added.
- **The plugin unit tests pass** (14/14, re-run during this review).

The problems:

1. **D3, nested-container bloat (BLOCKER).** The root cause is in the dagre adapter: the title strut is laid out as a sibling of the container's children, and container borders are spaced by an `edgesep` larger than `nodesep`. It will reproduce on every migrated diagram with containers (at least 5 Phase 90 pages use `subgraph`).
2. **D1, edge through a title band.** The title band is added after layout, titles are painted before edges, and no rule checks an edge crossing a band.
3. **Fail-closed guard fails open on a rebuild.** On an md rebuild, Astro's content store caches the failed render, so the second build passes and ships an empty page.
4. **Accessibility.** Every figure is a focusable `region` landmark named "Diagram".
5. **Padding floor gap.** The floor is bypassed for container titles.
6. **Weak tests.** Several browser checks would pass against broken code.

## Critical Issues

### CR-01: Nested containers are drawn several times larger than their contents (D3)

**File:** `crates/vox-diagram/src/layout/dagre_impl.rs:38-46`, `crates/vox-diagram/src/layout/mod.rs:40-56`, `crates/vox-diagram/src/layout/finish.rs:44-57`

**Issue:** The stress figure (`04-light-natural.png`) has a 1696x1482 viewBox for six 38 px-tall nodes. "Persistence layer" is 637x407 px around two nodes on one row: 221 px of empty space above them and 114 px below. These causes were measured from the manifest SVG and the committed goldens:

1. **Container borders are spaced by `edgesep`, and `edgesep` exceeds `nodesep` on every rung.** dagre's position step uses `edgeSep` for any dummy node. Compound-border nodes are dummies (`dummy: "border"`), so the gap between a parent border and a child border is a full `edgesep` per nesting level and per side. `RUNGS` set `edgesep` to 70, 120 and 200 against `nodesep` 40, 50 and 60, the opposite of dagre's defaults (nodesep 50, edgesep 20). Evidence:
   - Golden `21-nested.svg` (rung 0): Outer, Middle and Inner frames sit at y 300, 370 and 440, a gap of exactly 70.
   - Stress diagram: Platform, Core and Persistence engine tops sit at 42, 162 and 282, a gap of exactly 120. That is rung 1's `edgesep`, and the rank pitch of 45 px is rung 1's ranksep 90 halved for labels.
2. **Retry rungs inflate everything at once.** `lay_out` re-solves the whole graph on the next rung whenever `verify` rejects any single invariant. One violation anywhere made every container in the stress diagram grow by about 70% (70 to 120 px per level).
3. **The title strut is a disconnected sibling of the children, not a width constraint across them.** `t{i}` is a child of `c{i}` with no edges, so dagre gives it its own slot on the cross axis next to the children:
   - **TB:** the strut sits beside the nodes. Golden `20-container.svg` frames a 64 px column of nodes in a 375 px-wide container. The nodes are pushed to x=258, and the title (centred at 203.5) is off-centre from them (centred at 290).
   - **LR:** the strut sits above the children. Golden `21-nested.svg` "Innermost layer" has 121 px above its only node and 55 px below. Its full `strut_w` also lies along the rank axis.
4. **Padding is added after layout, at every level.** `finish::frame` adds `CONTAINER_PAD` on three sides and the title band on top, outside dagre, so every nesting level adds them again.

Container frames are dagre's border-node rect plus these additions. They are never derived from the children's boxes, so nothing bounds the slack. The geometry oracles check containment and overlap only, never compactness, so all tests stay green.

**Failure scenario:** Phase 90 migrates at least 5 pages with `subgraph` (axis-harness-river, deep-research audit, waves, scientia extension, compiler-architecture). Each renders 3 to 6 times larger than its content. Under the 0.6 floor at 1280 px this gives roughly 8 px text in a horizontally scrolling box. At 375 px most of the diagram is off-screen.

**Fix:** In order of payoff:
```rust
// 1. Border gap = nodesep-sized, not edgesep: keep edgesep <= nodesep on every rung,
//    and escalate only what the failing rule needs.
pub(crate) const RUNGS: [Separations; 3] = [
    Separations { nodesep: 40, ranksep: 60, edgesep: 20 },
    Separations { nodesep: 50, ranksep: 90, edgesep: 25 },
    Separations { nodesep: 60, ranksep: 140, edgesep: 30 },
];
```
2. Delete the `t{i}` strut. `finish::fit_title` already re-wraps a title to the final frame width, so a wide title grows the band instead of the frame. If a minimum frame width is still wanted, widen the frame after layout (centred) and let `verify`'s `SiblingOverlap`/`ForeignOverlap` reject collisions.
3. Shrink-wrap each frame in `finish` to the union of its children's placed boxes (nodes, child frames, and labels of edges internal to the container) plus `CONTAINER_PAD` and the band, instead of using dagre's border rect.
4. Add a compactness oracle so this cannot regress silently, for example: for every container, `frame area <= k * (children bbox area + band)`; and pin the stress fence's viewBox in a golden.

Re-bless goldens after the change; the existing determinism and geometry oracles must stay green.

## Warnings

### WR-01: A miss is fail-open on the next md build (the content store caches the failed render); `clearStaleStore` is dead and its comment is false

**File:** `docs-astro/astro.config.mjs:59-62`, `docs-astro/src/plugins/remark-vox-diagram.mjs:229-233`, `docs-astro/scripts/render-diagrams.mjs:18-29`

**Issue:** For `.md`, a strict miss throws inside the content-layer render. Astro 6.1.9 `glob.js:120-139` catches the throw, logs `Error rendering`, and still calls `store.set({ ..., digest, rendered: undefined })`. On the next build, if the config digest has not changed (same manifest bytes, same plugin options), `glob.js:90-98` sees `existingEntry.digest === digest` and returns early. The remark plugin never runs, no miss is recorded, and `assertNoMisses()` passes. The page then renders `entry.rendered?.html`, which is `undefined`: the page ships with **no body at all**. 89-03-SUMMARY recorded exactly this ("A first mutation run without clearing the store also exited 0 but logged no miss (cached store)") but treated it as a test-procedure artefact rather than a product bug.

`clearStaleStore` was written for this cache, but nothing calls it. Its doc comment says "`astro.config.mjs` calls this at every (re)start", which is false. Its unit test guards a function nothing uses.

**Failure scenario:** A developer edits a fence and runs `npx astro build` (or a divergence makes remark see a fence the Rust walk did not; see WR-05). Build 1 fails as intended. They re-run the same command and build 2 exits 0 with that page empty. CI is not affected, because hosted runners start without `.astro/`. The guarantee "a build never ships a missing render" is only true on a clean tree.

**Fix:** When the guard fails, invalidate the store so the next build re-renders. Either reuse the dead helper or remove the file directly:
```js
'astro:build:start': () => {
  try { assertNoMisses(); }
  catch (e) { rmSync(new URL('./.astro/data-store.json', import.meta.url), { force: true }); throw e; }
},
```
(Apply the same to `astro:build:done`.) Then either delete `clearStaleStore` and its test, or wire it in as above and fix the comment. Add a regression test: miss, build, rebuild with no change, and assert the rebuild still fails.

### WR-02: The `emit` edge strikes through the four-line title of "Operations and observability tooling..." (D1)

**File:** `crates/vox-diagram/src/layout/finish.rs:44-57`, `crates/vox-diagram/src/svg/mod.rs:191-192`, `crates/vox-diagram/src/layout/verify.rs:1-4`

**Issue:** There are three compounding causes:
1. `finish::frame` grows each container upward by `title_band(lines)` after dagre has routed the edges. dagre never sees the band, so it routes edges into the container straight through what becomes the band. In the stress SVG the last segment of the `emit` path runs `L962.5 782.5 -> L1108.5 1335`, a single 570 px diagonal that crosses the bottom borders of Core and Platform and the band of Ops (y 1125 to 1199).
2. `render_graph` paints containers, titles included (`write_containers` -> `shapes::write_container`), **before** `paths::write_edges`, so the edge stroke is drawn over the title glyphs.
3. `verify` treats edge-path crossings as soft by design. `LabelInTitleBand` covers edge-label boxes only, not edge segments, so no rung retry or rejection happens.

**Failure scenario:** Every migrated diagram where an edge enters a titled container from the title side. In LR that is any edge arriving from above, which dagre produces whenever the target container is on another row. The result is struck-through, partly illegible titles. This is a legibility defect, not wrong data, hence WARNING. It should still be fixed before Phase 90.

**Fix:** The cheapest durable fix is paint order plus a halo. Emit the container titles in their own `<g class="vd-container-labels">` after the edges (and before nodes), with `paint-order:stroke;stroke:var(--vd-container-fill,#f6f8fa);stroke-width:4px;stroke-linejoin:round`, so edges pass visibly behind the title. In addition, either reserve the band inside dagre (a cross-axis spacer on the title side of the container), or add a verify rule "edge segment intersects a title text box" that escalates the rung and, failing that, is reported as a soft metric in the go/no-go record.

### WR-03: Every figure is a focusable `region` landmark named "Diagram"; no per-diagram title is reachable

**File:** `docs-astro/src/plugins/remark-vox-diagram.mjs:257-265`, `crates/vox-diagram/src/svg/mod.rs:154-158`, `crates/vox-diagram/src/validate.rs:87`

**Issue:** The figure's `aria-labelledby` is `<prefix>-title`, and that title is always the literal "Diagram":
- `SvgOpts.title` exists, but `validate()` renders with `SvgOpts::default()`, and the manifest writer only calls `validate`.
- More fundamentally, neither `GraphIr` nor `SequenceIr` has a title field, and the grammar has no `title:` statement, so no per-diagram title exists to pass.

Separately, the plugin sets `role="region"` and `tabindex="0"` on **every** figure. CONTEXT asked for this only on wide figures that scroll. Small diagrams that never scroll still become tab stops. A page with N diagrams gets N landmarks all called "Diagram" (axe `landmark-unique`) and loses native `figure` semantics. The spec checks only that the name is non-empty (`vox-diagram.spec.ts:165`), so it passes.

**Failure scenario:** After Phase 90, a screen-reader user's landmark list on an architecture page reads "Diagram region, Diagram region, Diagram region...". Keyboard users also tab through every diagram, including ones with nothing to scroll.

**Fix:**
1. Add an optional `title: "..."` statement to `graph`/`sequence` (it fits beside `direction:`), carry it into the IR, and have `render_manifest` call `svg::render(&ir, &SvgOpts { title: ir.title.clone(), ..Default::default() })`. Until then, the remark plugin can name the figure after the nearest preceding heading (`aria-label`), with "Diagram n" as the fallback, so names are unique.
2. Emit `role="region"` and `tabindex="0"` only where scrolling can occur. For example, keep it on the figure but add it with a tiny inline script or `:has()` check after layout. Or accept the tab stop, but then require unique names and test uniqueness in the spec.

### WR-04: The padding floor is bypassed for container titles (strut cap and `fit_title`)

**File:** `crates/vox-diagram/src/layout/input.rs:86`, `crates/vox-diagram/src/layout/finish.rs:27-40,46`

**Issue:** The 89-02 claim is "every padded box goes through `pad_x`" (`text.rs:68`). Two paths break it for container titles:
- `strut_w = (widest + 2*pad_x(widest, TITLE_PAD)).min(MAX_STRUT)`. For any title line wider than about 232 px (260/1.12), the cap cuts off the floor.
- `finish::fit_title` keeps or re-wraps the title against `inner = frame_w - 2*TITLE_INSET` using plain `measure`, with no 12% floor.

So a frame whose width is set by a capped strut has `inner = 256 px` and accepts a 256 px-measured line. Under Verdana (about 1.098x `measure`) that line is about 281 px against a 276 px frame: about 2.5 px past each border. The unit test `a_long_title_strut_uses_the_padding_floor` deliberately asserts `w < MAX_STRUT`, so it never exercises the capped case. The browser fit check cannot catch it either (WR-06).

**Fix:** In `fit_title`, compare with the floor: `widest(lines) + 2 * pad_x(widest(lines), 0) <= inner`, or `widest * 112 <= inner * 100`. With that in place the cap stays harmless, because the title re-wraps. Add a test with a 28-cell `W`/`m` title line under the cap.

### WR-05: The pre-render walks only `docs/src`, but the plugin also processes mounted repo Markdown, and the error tells you to run a command that cannot fix it

**File:** `crates/vox-doc-pipeline/src/pipeline/render_manifest.rs:45`, `docs-astro/src/plugins/remark-vox-diagram.mjs:98-111`

**Issue:** `setup-content.mjs` mounts AGENTS.md, CLAUDE.md, GEMINI.md, CHANGELOG.md, `crates/**/README.md`, `docs/agents/**` and others into `src/content/docs/repo/`, and the same remark plugin runs on them. `render_diagrams` only gathers `root/docs/src`. A `vox-diagram` fence in any mounted file is therefore a guaranteed manifest miss. The miss is reported as `docs/src/repo/agents-md.md fence 1 ... Run: pnpm diagrams`, which names a path that does not exist and recommends a command that cannot fix it. `where-things-live.md` documents the gap, but the failure itself is misleading. There are no such fences today; AGENTS.md tells authors to use `vox-diagram` fences for new diagrams.

**Fix:** Have `render_diagrams` also walk the mounted set. Reuse `contracts/documentation/site-mounted-repo-docs.v1.json` and the same discovery rules, or have `setup-content` run first and walk `docs-astro/src/content/docs`. At minimum, make `docsPath` map `/src/content/docs/repo/` back to the source path from the `mounted_from` frontmatter and say "mounted repo docs are not pre-rendered".

### WR-06: The contrast check measures strokes against the page background, not the fill they sit on; the stroke selector is unscoped

**File:** `docs-astro/tests/vox-diagram.spec.ts:228-229`

**Issue:** `backdrop(el.closest('figure'))` walks up from the **figure**, so every node, edge and nested-container stroke is compared with the page colour (`--sl-color-black`). Many of these strokes actually sit on a container fill (`--sl-color-gray-6`). Today both themes pass anyway:
- dark: gray-3 on gray-6 is about 4.4:1, against the measured 5.27:1 on the page colour;
- light: about 6.1:1, against the measured 7.02:1.

But a change that darkens `--vd-container-fill` in dark mode toward the stroke colour would pass this test. The selector also scopes only its first part: `'svg.vd-diagram .vd-node-shape, .vd-edge, .vd-container, .vd-lifeline'` matches `.vd-edge`, `.vd-container` and `.vd-lifeline` anywhere on the page.

**Fix:** For each stroke element, use the fill of the innermost `.vd-container` whose rect contains it (or `--vd-bg` when there is none) as the background. Scope every selector part: `svg.vd-diagram :is(.vd-node-shape, .vd-edge, .vd-container, .vd-lifeline)`. Extend the pale-palette self-test with a case where only `--vd-container-fill` is pushed toward the stroke colour, and require it to fail.

### WR-07: The browser fit check is nearly vacuous for container titles, and nothing would catch D1 or D3

**File:** `docs-astro/tests/vox-diagram.spec.ts:93-97,110-111`

**Issue:** Container title text is mapped to `previousElementSibling` `.vd-container`, which is the **whole frame**. A title "fits" whenever it lies anywhere inside a 637x407 box. Reverting the strut floor (WR-04), overflowing the band vertically into the children, or the D1 strike-through would all pass. No test bounds container slack or figure size (CR-01), and no test checks edge segments against title bands (WR-02), so both defects shipped with every test green. The summary also says the wide-diagram policy test was never mutation-proven.

**Fix:**
- For `.vd-container-label`, check against the band rect (`x + TITLE_INSET` to `x + w - TITLE_INSET`, `y` to `y + band`) instead of the frame.
- Add a browser assertion that no `.vd-edge` path intersects a `.vd-container-label` bbox; `isPointInStroke` sampled along the title bbox is enough.
- Add a Rust compactness oracle (see CR-01).
- Mutation-prove the wide-policy test, for example by removing `min-width` from `vox-diagram.css` and confirming the test fails.

## Info

### IN-01: The pre-render runs twice in both docs workflows

**File:** `.github/workflows/docs-quality.yml:151-157`, `.github/workflows/docs-deploy.yml:96-97,118`

**Issue:** The explicit `Pre-render VoxDiagram fences` step is followed by `pnpm build`, whose `prebuild` runs `render-diagrams.mjs`, which runs `cargo run ... --render-diagrams` again. The second run is cheap, since it is already compiled, but it is redundant. It also means the explicit step adds nothing except a separately named log section.

**Fix:** Drop the explicit step and rely on `prebuild`, or keep the step and call `astro build` after `setup-content`. Update `workflow-paths.test.mjs` to match.

### IN-02: Fences with a near-miss info string are skipped silently by the plugin

**File:** `docs-astro/src/plugins/remark-vox-diagram.mjs:224`; test `remark-vox-diagram.test.mjs:171-183`

**Issue:** ```` ```vox-diagram title="x" ```` (`node.meta` set) is left as a plain `text` code block and records no miss. `pnpm build` still fails, because the Rust `diagram-fence-info` hard error stops `prebuild`. A bare `astro build` ships it as a code block, so the Astro-level guard does not cover this case.

**Fix:** In strict mode, record a miss (or throw) for `lang === 'vox-diagram' && node.meta`, mirroring Rust.

### IN-03: The SVG parser has an allowlist for tags but a denylist for attributes; `url(#...)` outside `marker-end` is not re-prefixed

**File:** `docs-astro/src/plugins/remark-vox-diagram.mjs:38-49,131-156`

**Issue:** Only `on*` and `*href` attributes are rejected. A future writer attribute such as `fill="url(#vd-grad)"`, `clip-path` or `marker-start` would pass `svgToHast`. Its reference would not be re-prefixed, and the dangling `#vd-...` would silently drop the paint or marker; the browser ids test inspects only `marker-end`. The manifest is generated from trusted repo content by the Rust writer, so this is not exploitable today. It does contradict the "closed grammar" claim.

**Fix:** Allowlist the writer's attribute names per tag, and reject any attribute value containing `url(` except `marker-end`.

### IN-04: `PREFIX_RE` can never fail

**File:** `docs-astro/src/plugins/remark-vox-diagram.mjs:116,245-246`

**Issue:** `pageSlug` already reduces the slug to `[a-z0-9-]` (or `index`), so `vd-${slug}-${n}` always matches `^[a-z][a-z0-9-]*$`. The check is harmless defence in depth, but it is not the validation it appears to be.

**Fix:** Keep the check and say what it guards against (a future change to `pageSlug`), or validate the slug itself.

### IN-05: No successful-render test for MDX; the key-parity corpus is CommonMark (md) only

**File:** `docs-astro/tests/unit/remark-vox-diagram.test.mjs:228-262`, `crates/vox-diagram/tests/fixtures/fence-keys.json`

**Issue:** Only the MDX miss path was exercised, as a manual negative control. The MDX success path was verified during this review by a manual compile, but no test protects it. MDX differs from CommonMark in ways that affect fences: no indented code, JSX flow elements. A divergence fails closed (a manifest miss), so the risk is a noisy build, not a silent one.

**Fix:** Add a `@mdx-js/mdx` compile test (resolved through `@astrojs/mdx`, as the md test does through astro) that asserts `marker-end`, `aria-labelledby` and `data-pagefind-ignore` in the output. Add a fence-inside-JSX case to the corpus.

### IN-06: Edge-label backgrounds use the page colour inside containers (D2)

**File:** `crates/vox-diagram/src/svg/paths.rs:130`

**Issue:** Labels on edges inside a container show as white patches in light mode and dark patches in dark mode on the grey container fill. This is cosmetic.

**Fix:** Paint the label background with the fill of the innermost container holding the label centre (the layout already knows the frames).

### IN-07: `customSelectors: { all: ['svg.vd-diagram'] }` is documented as having no effect

**File:** `docs-astro/astro.config.mjs:100-101`

**Issue:** 89-03-SUMMARY mutation-proved that removing the selector changes nothing. It is config kept "for intent".

**Fix:** Keep it, but add a comment that it is a backstop, or remove it.

### IN-08: The reference page claims browser checks "on every build", and the CI steps have never run

**File:** `docs/src/reference/vox-diagram.md:110`, `.github/workflows/docs-quality.yml:197-210`

**Issue:** The browser spec runs only in `docs-quality`, only on its path filters, and never in `docs-deploy`. Per 89-04-SUMMARY, the new apt-font and `playwright install --with-deps` steps have never executed on hosted CI: 7 of the last 8 runs were `action_required`. As a result, Linux DejaVu/Liberation metrics and the step `if:` conditions are unverified, and so is the 30-minute margin.

**Fix:** Change the wording to "in the docs-quality workflow", and treat the first green hosted run as a phase exit criterion.

---

_Reviewed: 2026-10-26_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
