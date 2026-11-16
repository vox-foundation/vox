# Phase 89: Docs-Astro Integration - Research

**Researched:** 2026-10-17
**Domain:** Astro 6.1.9 / Starlight 0.38.3 build pipeline (remark, MDX, Expressive Code, starlight-llms-txt), Rust pre-render hand-off, Playwright verification, docs workflows
**Confidence:** HIGH (every load-bearing claim below was reproduced in a scratch copy of docs-astro against the real dependency tree; the scratch repo, prototype plugin, specs and logs live under `/private/tmp/claude-501/-Users-brbrainerd-dev-vox/1578e102-b2d3-4922-b18e-a965ec6b6147/scratchpad/p89/`)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Pre-render pipeline & dev loop**
- `docs-astro` `prebuild` / `predev` scripts run `cargo run -p vox-doc-pipeline -- --render-diagrams` (new flag); docs-deploy and docs-quality workflows run the pre-render before `pnpm build` and trigger on `crates/vox-diagram/**` changes.
- Manifest: gitignored `docs-astro/.vox-diagram/manifest.json`, keyed by the normalized fence source text (exact text keys; no hashing).
- `astro dev` after editing a fence: the remark plugin renders a visible placeholder ("diagram not rendered — run `pnpm diagrams`"); `pnpm build` fails hard on any manifest miss, naming the file and the exact command.
- Ids & markup: per-fence id prefix `vd-<page-slug>-<n>` (page order, deterministic, validated `[a-z][a-z0-9-]*`); output `<figure class="vd-figure">` wrapping the inline SVG, inserted as hast nodes (`data.hName` / `hChildren`, MDX-safe — not raw HTML strings). This guarantees the other checkout's uncommitted `pre[data-language="mermaid"]` CDN loader (`PageSidebar.astro`) never processes VoxDiagram output. The fence source is retained for `llms-full.txt`.

**Theming & browser verification**
- Site CSS maps `--vd-*` → `--sl-*` (the Phase 85 candidate map including the WR-04 container-stroke contrast fix) for light and dark, plus a `@media (forced-colors: active)` block using system colors.
- Playwright fixtures: the `docs/src/reference/vox-diagram.md` page (graph + sequence, several diagrams on one page) is the primary fixture; add a test-only fixture page excluded from sidebar/search only if more coverage is needed (e.g. long labels, containers, unicode).
- Checks: no duplicate ids per page; every `<text>` bbox inside its box under two font stacks (system-ui and a wide Verdana/DejaVu stack); axe contrast passes in light + dark; forced-colors screenshot legible.
- Carried from Phase 86 + font policy: the executor captures light/dark Playwright screenshots of graph and sequence diagrams (incl. multi-hop message labels) into the review bundle and inspects them (Read the PNGs); confirm the Phase 85 static-width-table font policy or propose a wider slack if any text overflows (proposal only — changing slack re-blesses goldens).

**Carried forward**
- Rust side: deterministic, no new crate edges (vox-doc-pipeline already depends on vox-diagram), files < 400 lines, tests first.
- docs-astro is a JS/MJS package (existing convention `.mjs`); pnpm 11 with `--frozen-lockfile` in CI; any new npm dependency (e.g. a hast helper or axe) requires user approval — prefer already-installed packages.
- Workflow edits must keep the `concurrency:` block and cache rules (main-only cache saves; `package-manager-cache: false` for setup-node caches).

### Claude's Discretion
- Plugin file names, manifest JSON shape, placeholder styling, Playwright spec layout.

### Deferred Ideas (OUT OF SCOPE)
- Removing the jsdelivr Mermaid loader (belongs to the other session's uncommitted work; revisit after merge/migration).
- Live re-render in `astro dev` via a Rust watcher.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DOCS-03 | The docs build pre-renders every ```` ```vox-diagram ```` fence to SVG (Rust pre-render → gitignored manifest) and inlines it via a remark plugin (MDX-safe), failing the build on a missing render | Findings 1, 2, 4, 5: unknown-node `vdFigure` technique (md and mdx proven), manifest keyed by normalized source with Rust/remark key parity (26 of 28 adversarial inputs identical, the 2 diffs fail closed), neutral-prefix render + JS re-prefix, an Astro-integration guard because a throw in a remark plugin does NOT fail `.md` builds, llms-full retention via `<details>` + `customSelectors`, workflow wiring |
| DOCS-04 | Rendered diagrams pass Playwright checks: no duplicate ids, labels fit boxes under two font stacks, contrast passes in both themes | Finding 3 and Validation Architecture: executed Playwright spec (unique ids + reference resolution, text-in-box under two stacks, custom WCAG contrast, forced-colors), measured font ratios, evidence that axe cannot test SVG text contrast, CI gaps (no Chromium install, reader-layout never ran green) |
</phase_requirements>

## Summary

The pre-render, manifest and inline-figure design in CONTEXT is sound and was prototyped end to end: Rust (`vox diagram render`, standing in for the future `--render-diagrams`) produces SVGs, a gitignored manifest keyed by normalized fence text feeds a ~150-line pure-Node remark plugin, and a real `astro build` of 846 pages inlines themed figures from both `.md` and `.mdx` (including a `index.mdx`-shaped page with imports and a JSX `<div>`). Build time stayed 23 s baseline to 24-42 s with the plugin. No new npm package is needed.

Seven findings change or sharpen the locked decisions, and the plan must account for each (details in the numbered findings): (1) the obvious `data.hName` on the `code` node leaves a `<pre>` wrapper, so the plugin must retype the node to an unknown mdast type; (2) a `throw` inside a remark plugin does **not** fail an `astro build` for `.md` pages (Starlight's docs loader logs `[ERROR] ... Error rendering` and exits 0), so "build fails hard" needs an Astro-integration guard; (3) `llms-full.txt` is built from **rendered HTML**, not source markdown, so retaining the fence source takes a `<details><pre><code class="language-vox-diagram">` block plus `customSelectors` to drop the SVG text; (4) the Rust id-prefix validator caps prefixes at 32 chars but real page slugs reach 62, so render once with the neutral prefix `vd` and re-prefix in JS; (5) axe-core's `color-contrast` rule returns *incomplete* for SVG text even for a deliberately unreadable palette, so "axe contrast passes" would be vacuous and the contrast check must be computed in Playwright; (6) the static width table fits Arial-class and system-ui fonts but **overflows under Verdana** (measured 1.05 to 1.19 x model vs 1.08 slack) on sequence-message and edge-label boxes that have 4 to 8 px padding; (7) `docs-quality.yml` has no `playwright install` step and its existing reader-layout step has never run green (every recent run skipped it), so DOCS-04 needs a Chromium + font install step.

**Primary recommendation:** Build it as: Rust `pipeline/render_manifest.rs` (new file) behind `--render-diagrams`; a wrapper `docs-astro/scripts/render-diagrams.mjs` (sets cwd to repo root, prints the exact command); `remark-vox-diagram.mjs` + a small inline Astro integration in `astro.config.mjs` that registers the plugin with `strict: command === 'build'`, passes a manifest digest option, and throws in `astro:build:start` if any miss was recorded; `src/styles/vox-diagram.css`; a `<details>` source block; and a top-level `tests/vox-diagram.spec.ts` that discovers figure pages from `dist/`. Extend `docs/src/reference/vox-diagram.md` with a stress-example section instead of adding a hidden fixture page.

## Project Constraints (from CLAUDE.md)

- `./CLAUDE.md` (vox) imports `AGENTS.md`; Claude-specific: treat `.vox` as Vox, honor `// vox:skip`, and do not store project research in agent memory (write architecture research to `docs/src/architecture/` with frontmatter). This RESEARCH.md is a GSD planning artifact at the path the orchestrator specified; if durable findings (font slack, llms-full behavior, md-build-error behavior) should live in docs, put them in a `docs/src/architecture/*-research-2026.md` page in a later plan.
- Test-first: every new `pub fn` in `crates/*/src/**` (files over 30 non-blank lines) needs a `#[test]`/`#[cfg(test)] mod tests` in the same file (`skeleton/untested-pub-api`).
- Rust files < 400 lines (`diagrams.rs` is 332, `mod.rs` is 813 and legacy: add new code in a NEW file, not `mod.rs`).
- VoxScript-first glue: no new `.sh`/`.py`/`.ps1`. The docs-astro wrapper is `.mjs` (existing docs-astro convention, set in CONTEXT); the TS-only rule applies to TS repos, docs-astro is the documented JS/MJS exception.
- No new crate edges (none needed: `vox-doc-pipeline -> vox-diagram` exists). Never regenerate baselines.
- Hosted CI is the gate: locally `cargo check -p vox-doc-pipeline` and `cargo nextest run -p vox-doc-pipeline` (background them; never `clippy --workspace`). Never `cargo fmt --all`; use `vox run scripts/fmt.vox`.
- Workflow edits keep `concurrency:` with `cancel-in-progress: true`; no cache saves from PR runs; `package-manager-cache: false` on setup-node.
- Fenced `vox-diagram` blocks in docs are validated by the doc lint (Phase 87); new docs need frontmatter, `topic`/`domain`, `authorship`, and a 40-120 word `<!-- kb:summary -->` (the reference page already has them).
- GUI visual-verification invariant (`crates/vox-gui/ui/e2e`) is for the Tauri GUI, not docs-astro; docs-astro's equivalent is its own Playwright suite (Finding 3).
- Do not read `docs/src/archive/`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Fence discovery, validation, SVG render, manifest write | Build tooling (Rust `vox-doc-pipeline`) | - | Reuses the Phase 87 whole-tree walk and `vox_diagram::fence::extract`; one process for all fences; pure Rust, no node-side parser drift |
| Manifest transport | Build artifact (gitignored `docs-astro/.vox-diagram/manifest.json`) | - | Decoupled hand-off; plugin stays unit-testable with a fixture manifest |
| Fence-node replacement, id re-prefix, placeholder | Build tooling (docs-astro remark plugin, Node) | Astro integration hook | mdast to hast is where md and mdx converge |
| "Build fails on a miss" enforcement | Build tooling (Astro integration `astro:build:start` + plugin throw for mdx) | Build-output Playwright spec (belt and braces) | A remark throw only fails mdx; md needs the guard (Finding 2) |
| Light/dark/forced-colors theming | Browser / CSS (static `vox-diagram.css`) | - | The SVG paints only `var(--vd-*, fallback)`; Starlight tokens flip per `data-theme` |
| `llms-full.txt` source retention | Build tooling (starlight-llms-txt config + figure markup) | - | That plugin converts rendered HTML back to Markdown |
| Visual / a11y verification | Browser (Playwright against `astro preview` of `dist/`) | CI workflow | Needs real layout (`getBBox`, computed styles) |
| Workflow wiring | CI (docs-deploy, docs-quality) | - | Path filters, pre-render step, Chromium + fonts |

## Standard Stack

### Core (all already installed; no new dependency)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| astro | 6.1.9 installed (`^6.0.1` in package.json) | Static build, markdown pipeline, integrations | `[VERIFIED: docs-astro/node_modules/astro/package.json]` |
| @astrojs/starlight | 0.38.3 | Docs theme; adds MDX and Expressive Code | `[VERIFIED: node_modules/@astrojs/starlight/package.json]` |
| starlight-llms-txt | 0.10.0 (patched: `patches/starlight-llms-txt@0.10.0.patch`) | `llms*.txt` | `[VERIFIED: package.json]` |
| @playwright/test | 1.63.0 (Chromium revision 1243, present in `~/Library/Caches/ms-playwright/chromium-1243`) | Browser checks | `[VERIFIED: browsers.json in playwright-core@1.63.0]` |
| node:test | Node 24 in CI (v26.11.1 locally) | Unit tests (`tests/unit/*.test.mjs`) | existing convention |
| mdast-util-to-hast | 13.2.1 (transitive) | The hName/hChildren contract | behavior verified at `lib/handlers/code.js:17-49` and `lib/state.js:352-391,407-423` |
| expressive-code | 0.41.7 (transitive) | Code blocks; runs as a rehype plugin AFTER remark | observed: it re-processed our inner `pre` |
| pulldown-cmark | 0.13 (workspace) | Rust fence scanner (inside `vox_diagram::fence`) | `[VERIFIED: Cargo.toml:415]` |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `@astrojs/markdown-remark` via `createRequire(require.resolve('astro/package.json'))` | with astro | Whole-pipeline unit test of the plugin without a new dependency | Node unit test (prototype passed: figure emitted, no `<pre>` wrapper, ids `vd-a-b-1-title`) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Strict 45-line SVG-subset parser in the plugin | `hast-util-from-html` 2.0.3 (already in the lockfile transitively, `[VERIFIED: ls node_modules/.pnpm]`) | Needs a direct `package.json` + `pnpm-lock.yaml` edit = user approval. Not recommended: the writer's grammar is closed (5 entities, double-quoted attrs, a small fixed tag set: svg, title, desc, defs, marker, path, g, rect, text, tspan and the polygon/shape elements) so a fail-closed mini parser is smaller and safer |
| Custom Playwright WCAG contrast calculation | `@axe-core/playwright` 4.11.3 + axe-core 4.14.0 | Needs approval, and axe cannot evaluate SVG text (Finding 3) so it adds no coverage for this requirement. Still valid for non-contrast rules if the user wants it |
| Extend `reference/vox-diagram.md` with stress examples | A hidden `status: research` fixture page | Hidden page still pays doc-lint/facet/frontmatter cost; the reference page is the right home for example diagrams |

**Installation:** none. (`pnpm install --frozen-lockfile` stays untouched; no lockfile edit.)

**Version verification:** `[VERIFIED: file reads this session]` astro 6.1.9, starlight 0.38.3, starlight-llms-txt 0.10.0, @playwright/test 1.63.0. Axe versions `[VERIFIED: npm install in /scratchpad/p89/axe]` 4.11.3 / 4.14.0.

## Package Legitimacy Audit

No package is recommended for installation. The optional axe packages were checked because CONTEXT names axe:

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| @axe-core/playwright | npm | latest release 2026-10-15 | 14.3M/wk | github.com/dequelabs/axe-core-npm | SUS (`too-new`: latest version just published) | NOT recommended (no value for SVG text); if the user wants it for other rules, planner adds `checkpoint:human-verify` before install |
| axe-core | npm | latest release 2026-10-05 | 79.7M/wk | github.com/dequelabs/axe-core | SUS (`too-new`) | same |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** `@axe-core/playwright`, `axe-core` (both only because the newest version is days old; both are the mainstream dequelabs packages with `postinstall: null`). `[VERIFIED: gsd-tools package-legitimacy check, this session]`

## Architecture Patterns

### System Architecture Diagram

```text
 docs/src/**/*.md(x)  (archive/ skipped)            contracts/ (unchanged)
          |
          |  pnpm dev / pnpm build  ->  predev | prebuild
          v
 docs-astro/scripts/render-diagrams.mjs   (cwd = repo root; prints exact command on failure)
          |  spawns:  cargo run -p vox-doc-pipeline -- --render-diagrams
          v
 vox-doc-pipeline  pipeline/render_manifest.rs
   gather_md_files(docs/src) -> skip archive/ -> read (capped)
   fence::extract(text, "vox-diagram")  -> fence::normalize(body)  -> dedupe (BTreeMap by source)
   per distinct source: check/validate -> svg::render(ir, SvgOpts{id_prefix:"vd"})
   ANY invalid fence or render error -> print file:line:col diagnostics (existing DiagramIssue::render) -> exit 1
          |
          v   atomic write (tmp + rename)
 docs-astro/.vox-diagram/manifest.json   { schema_version:1, diagrams:[{source, svg}] sorted by source }   (gitignored)
          |
          |  astro build | astro dev
          v
 astro.config.mjs  inline integration "vox-diagram"
   astro:config:setup -> updateConfig(markdown.remarkPlugins += [remarkVoxDiagram,{strict: command==='build', digest: sha256(manifest)}])
                       -> addWatchFile(manifest)   (dev: server restarts on manifest change)
   astro:build:start  -> throw if the plugin recorded any miss (md-page misses never throw out of the loader)
          |
          v
 remark pass (md via Starlight docs loader; mdx via @astrojs/mdx/vite) ---------------------------+
   code[lang==="vox-diagram" && !meta]                                                              |
     key = normalizeSource(node.value)  (CRLF->LF, trim trailing White_Space)                       |
     hit : retype node -> "vdFigure"; data.hName="figure"; hChildren = svg hast + <details>source   |
           ids re-prefixed  vd-<page-slug>-<n>; figure style --vd-w:<viewBox width>px               |
     miss: dev -> placeholder figure.vd-figure.vd-missing ; build -> record + throw                 |
          v                                                                                         |
 remark-rehype (unknown node + hName => <figure>, NO <pre> wrapper)                                 |
          v                                                                                         |
 rehype-expressive-code  (re-processes the inner <pre><code class=language-vox-diagram>;            |
                          langAlias {'vox-diagram':'text'} silences the unknown-language WARN)      |
          v                                                                                         |
 dist/**/index.html  <figure class="vd-figure" style="--vd-w:..."><svg .../><details>...</details></figure>
          |                       |
          |                       +--> starlight-llms-txt: HTML -> Markdown; customSelectors drops svg.vd-diagram,
          |                            leaving "Diagram source" + a ```vox-diagram fence in llms-full.txt
          v
 astro preview (Playwright webServer) -> tests/vox-diagram.spec.ts (ids, fit x2 stacks, contrast x2 themes, forced-colors)
```

### Recommended Project Structure
```text
crates/vox-doc-pipeline/src/pipeline/
  render_manifest.rs        # NEW (< 400 lines): walk, extract, dedupe, render, atomic write, error report
docs-astro/
  .gitignore                # + .vox-diagram/
  package.json              # scripts: diagrams, predev, prebuild
  scripts/render-diagrams.mjs        # NEW: spawn cargo in repo root, print exact command
  src/plugins/remark-vox-diagram.mjs # NEW: normalizeSource, svgToHast, reprefix, pageSlug, plugin, assertNoMisses
  src/styles/vox-diagram.css         # NEW: --vd-* -> --sl-* map, figure sizing, forced-colors
  astro.config.mjs                   # + integration, + customCss entry, + shiki.langAlias, + llms customSelectors
  tests/unit/remark-vox-diagram.test.mjs
  tests/build-output/diagrams.spec.ts   # fs-only: figure count, no vd-missing, llms-full block, no <pre><figure
  tests/vox-diagram.spec.ts             # browser: ids, fit, contrast, forced-colors, screenshots
  tests/lib/diagrams.ts                 # page discovery from dist/, font probes, WCAG math
docs/src/reference/vox-diagram.md        # + "Layout stress examples" fences (long labels, shapes, container, multi-hop sequence)
.github/workflows/docs-deploy.yml, docs-quality.yml
```

### Pattern 1: Retype the code node (no `<pre>` wrapper) `[VERIFIED: prototype + mdast-util-to-hast source]`
**What:** mdast-util-to-hast's `code` handler applies `data.hName/hProperties/hChildren` to the inner `<code>` element and THEN wraps the result in `<pre>` (`lib/handlers/code.js:42-48`: `result = state.applyData(node, result)` then `result = {type:'element', tagName:'pre', ...children:[result]}`). A `figure` produced that way renders as `<pre><figure>` (observed in built HTML and in the llms HTML). An **unknown** mdast type goes through `defaultUnknownHandler` (`lib/state.js:407-423`) which builds a `div` and calls `applyData`, so `hName: 'figure'` replaces the `div` with no wrapper.
**When to use:** always, for md and mdx.
```js
node.type = 'vdFigure'; delete node.value; delete node.lang; delete node.meta;
node.data = { hName: 'figure', hProperties: { className: ['vd-figure'], style: '--vd-w:248px' }, hChildren: [...svgHast, detailsSource] };
```
Proven in: `.md` (reference page, fixture), `.mdx` plain, `.mdx` inside `<div>` with blank lines, and a copy of the real `docs/src/index.mdx` (imports + JSX) with a fence appended (`<figure class="vd-figure"><svg` directly after the `<h2>`; ids `vd-index-1-title|desc|arrow`).

### Pattern 2: Neutral-prefix render, per-fence re-prefix in JS
`SvgOpts::id_prefix` must match `[a-z][a-z0-9-]*` and be at most 32 chars (`crates/vox-diagram/src/svg/mod.rs:19-25,119-129`), but page slugs reach 62 chars (e.g. `architecture/diagramming-and-mermaid-evaluation-advisory-2026`). Render each distinct source once with the default prefix `vd`, then in the plugin rewrite the three id families it emits. Ids occur only in `id=`, `aria-labelledby=` and `marker-end="url(#...)"` (verified from rendered SVG: `id="vd-title"`, `id="vd-desc"`, `id="vd-arrow"`, `aria-labelledby="vd-title vd-desc"`, `url(#vd-arrow)`), none of them derived from labels, so a hast-level property rewrite is exact. Two identical fences on one page get different prefixes (`vd-p89-fixture-md-1`, `-2`); the duplicate-id test passed on a page with identical sources.

### Pattern 3: Strict closed-grammar SVG to hast (no dependency)
`writer.rs` emits only `<name k="v"...>`, `</name>`, `<name .../>`, text, and escapes exactly `&amp; &lt; &gt; &quot; &#39;` (`crates/vox-diagram/src/svg/writer.rs:11-38,67-78`). A ~45-line parser that throws on anything else (unknown entity, unbalanced tag, unparsable offset) is fail-closed; attribute names are kept in attribute form (`aria-labelledby`, `marker-end`, `viewBox`, `text-anchor`) and `class` becomes `className: string[]`. Verified output survives both pipelines: the built `.mdx` and `.md` HTML contain `aria-labelledby`, `marker-end`, `viewBox`, `style` intact.

### Anti-Patterns to Avoid
- **`data.hName` on the `code` node:** leaves `<pre><figure>` (invalid nesting, `pre` styling, EC and the Mermaid loader's `pre` selectors still see a `pre`).
- **Relying on a `throw` in the remark plugin for md pages:** exit code stays 0 (Finding 2).
- **Raw `html` mdast nodes / `set:html`:** not MDX-safe and defeats the closed-grammar guarantee (CONTEXT already forbids it).
- **Registering the plugin statically with `strict: true`:** `astro dev` then dies on the first edited fence instead of showing the placeholder.
- **`system-ui` as the only "narrow" font in tests:** resolves differently per OS (Finding 3).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Fence scanning in Rust | a regex fence finder | `vox_diagram::fence::{extract, normalize}` (pulldown-cmark, `crates/vox-diagram/src/fence.rs:25,76`) | Already shared by lint, ratchet, CLI; handles lists, quotes, tilde, longer outer fences |
| Rust to JS key agreement | ad-hoc trimming | `normalize` mirror: CRLF/CR to LF then `trimEnd` with `/\p{White_Space}+$/u` | `\p{White_Space}` is exactly Rust's `char::is_whitespace` (JS `trimEnd` differs on U+FEFF and U+0085) |
| id-prefix validation | a second validator | `[a-z][a-z0-9-]*` regex on `vd-<slug>-<n>` after slug sanitizing | Same grammar as `valid_prefix`; length cap only matters inside Rust |
| Pre-render error formatting | new messages | `DiagramIssue::render` / `fence_issues` in `pipeline/diagrams.rs` | file:line:col with container-prefix mapping already done and tested |
| WCAG contrast | a dependency | ~15 lines (sRGB to linear to luminance) in the spec | proven in the prototype; axe cannot do SVG text |
| Page slug for ids | github-slugger inside the plugin | lowercase + non-alnum runs to `-` over the path after `/src/content/docs/` or `/docs/src/` | md gets the mirror path, mdx gets the realpath (Finding 1); slug only needs to be stable and unique per page |
| Whole-pipeline plugin test | adding remark-parse/unified | `createMarkdownProcessor` resolved through `astro` | zero lockfile change |

**Key insight:** the only genuinely new logic is the key normalization, the closed SVG parser, and the build guard; everything else composes parts that already exist and are tested.

## Runtime State Inventory

Not a rename/refactor/migration phase. Omitted. (One runtime-state item that behaves like cached state: the Astro content-layer store, see Pitfall 3.)

## Common Pitfalls

### Pitfall 1: `<pre><figure>` from the `code` handler
**What goes wrong:** invalid nesting; llms conversion turned the whole figure into one untagged fenced block mixing SVG text and "Diagram source".
**Why:** `code.js` wraps after `applyData`. **How to avoid:** retype to `vdFigure` (Pattern 1). **Warning signs:** `grep -c '<pre[^>]*><figure class="vd-figure'` over `dist/` is non-zero; add that as a build-output assertion.

### Pitfall 2: a throw in the remark plugin does not fail `.md` builds
**What goes wrong:** with a fence missing from the manifest the log shows `[ERROR] [starlight-docs-loader] Error rendering p89-fixture-md.md: Failed to parse Markdown file ... remark-vox-diagram: ...`, the page is emitted with no figures, and `astro build` **exits 0** (`exit=0 seconds=45` in `build9.log`). The same fence in an `.mdx` page fails the build with exit 1 (`[@mdx-js/rollup] remark-vox-diagram: ...`, `build10.log`). The existing `remark-vox-include.mjs` comment "Build fails loudly" is equally untrue for `.md` pages today (pre-existing; not Phase 89's to fix, but worth a note).
**How to avoid:** plugin pushes misses to `globalThis[Symbol.for('vox-diagram.misses')]` and an inline integration throws from `astro:build:start`; prototype result: `[ERROR] [vox-diagram-guard] An unhandled error occurred while running the "astro:build:start" hook ... Run: pnpm diagrams`, `exit=1`. Also keep the throw (it is what fails mdx early with the file name).
**Warning signs:** a negative-control build (fence deleted from manifest) must exit non-zero for BOTH a `.md` and an `.mdx` page.

### Pitfall 3: Astro content-layer store serves stale md renders
**What goes wrong:** md entries are cached in `docs-astro/node_modules/.astro/data-store.json` (Astro `cacheDir` default). Changing the plugin or the manifest does not change an entry's digest, so md pages keep old HTML: observed three times (plugin change left `<pre dir="ltr"><figure>`; manifest edit `go` to `GO` did not appear; a later plugin edit left figures without the new `style`). Worse, a dev-mode (non-strict) build can cache placeholder HTML that a later `pnpm build` reuses.
**Evidence of the fix:** passing `digest: sha256(manifest)` and `strict` in the plugin options changes the config digest and invalidates the store: a manifest edit then re-rendered md (`>Go3<` count 2). In `astro dev`, after the manifest watcher restarts the server, md pages still showed the placeholder (mdx updated), so the dev loop needs the `diagrams` script to also delete `node_modules/.astro/data-store.json` (or the user re-saves the page).
**How to avoid:** options digest + `rm` of the data-store from `render-diagrams.mjs` when the manifest content changed. CI is clean-checkout, so no cache there.

### Pitfall 4: prefix length cap vs slug length
See Pattern 2. A Rust-side prefix derived from the page slug would be rejected (`InvalidIdPrefix`) for long slugs.

### Pitfall 5: `llms-full.txt` comes from rendered HTML
**What goes wrong:** with just `<figure><svg>`, the SVG's `<desc>` and every `<text>` label leak into llms-full as unstructured lines; the DSL source is not present at all. `entryToSimpleMarkdown.ts` renders the entry with the Astro container then converts HTML to Markdown with rehype-remark (`node_modules/starlight-llms-txt/entryToSimpleMarkdown.ts:212-222`); `rawContent` is off.
**How to avoid:** emit `<details class="vd-source"><summary>Diagram source</summary><pre><code class="language-vox-diagram">SOURCE</code></pre></details>` inside the figure (Expressive Code re-processes it into a framed, copyable, highlighted-as-text block with `data-language="vox-diagram"`, which `improveExpressiveCodeHandling` turns into a `language-vox-diagram` class), and set `customSelectors: { all: ['svg.vd-diagram'] }` on `starlightLlmsTxt(...)`. Result in `dist/llms-full.txt`: `Diagram source` followed by a ```` ```vox-diagram ```` fence containing the normalized source; no SVG text. `llms-small.txt` drops the figure entirely (its minify removes `details`), which is acceptable for the small variant.
**Warning signs:** build-output assertion that `llms-full.txt` contains a ```` ```vox-diagram ```` fence for a known fixture and does not contain the diagram's `<desc>` sentence.

### Pitfall 6: Expressive Code sees the inner `pre`
EC is a rehype plugin (runs after remark-rehype), so it never "grabs" the fence before the remark plugin, but it does process the `<pre><code class="language-vox-diagram">` inside the `<details>`. Without a language alias it logs `[WARN] [astro-expressive-code] Error while highlighting code block using language "vox-diagram" ... Using "txt" instead` (4 warnings in the dev log). `expressiveCode.shiki.langAlias: { 'vox-diagram': 'text' }` removes the warning and keeps `data-language="vox-diagram"`. (The existing build already logs the same WARN for `rust,ignore`, `ignore`, `no_run`, `gitattributes`: noise, not failures.) If the plugin did not emit a `pre`, no alias would be needed. EC also injects a stylesheet link and module script into each such block (once-per-page cost, same as any code block).

### Pitfall 7: reference page diagrams are too easy
All 12 `<text>` elements of the two current fences pass every check under every font stack; only long-label/wide fixtures expose the Verdana overflow. Extend the reference page with stress examples or the font-fit check proves nothing.

### Pitfall 8: Playwright output dir is wiped per run; CI has no browser
`playwright test` empties `test-results/` at the start of each invocation (observed: the `review-bundle/vox-diagram` PNGs vanished after the next run). `docs-quality.yml` runs build-output (30 tests, no browser) then `tests/reader-layout.spec.ts` (12 browser tests) and uploads `docs-astro/test-results/review-bundle/` with `if: always()`; there is no `playwright install` step in that workflow (only `docs-deploy.yml:224` has `npx playwright install chromium --with-deps`, for the live-site smoke job), and in every docs-quality run since the reader-layout step was added (`gh run view` of 7 recent runs) both Playwright steps were `skipped` because an earlier step failed. So reader-layout has never been proven green in this workflow. Add `npx playwright install chromium --with-deps` and write the diagram bundle outside the wiped dir (or run with `--output=test-results/vd-run`).

### Pitfall 9: font stacks can be identical on a runner
`system-ui` is SF on macOS and typically DejaVu or Liberation on Linux; `Verdana, "DejaVu Sans"` falls back to Liberation if DejaVu is absent. Playwright's Ubuntu 24.04 dependency list includes `fonts-liberation` but no DejaVu (`playwright-core@1.63.0/lib/coreBundle.js:31213-31226`, the `"ubuntu24.04-x64"` block: fonts-noto-color-emoji, fonts-unifont, xfonts-cyrillic, xfonts-scalable, fonts-liberation, fonts-ipafont-gothic, fonts-wqy-zenhei, fonts-tlwg-loma-otf, fonts-freefont-ttf). Pin explicit stacks (narrow: `"Liberation Sans", Arimo, Arial, Helvetica, sans-serif`; wide: `Verdana, "DejaVu Sans", sans-serif`), install `fonts-dejavu-core` in the job, and assert the probe widths differ by at least 5 percent so a collapsed pair fails loudly instead of passing vacuously.

### Pitfall 10: cwd of `vox-doc-pipeline`
`pipeline::run()` calls `lint_in(Path::new("."), ...)` (`crates/vox-doc-pipeline/src/pipeline/mod.rs:632-637`): the binary assumes cwd = repo root. `pnpm build` runs `prebuild` in `docs-astro/`, so `cargo run -p vox-doc-pipeline -- --render-diagrams` from there would report `docs/src/ not found`. The wrapper script must spawn with `cwd = repo root`.

### Pitfall 11: tight JSX fences in MDX
`fence::extract` (CommonMark) treats a fence directly inside an HTML/JSX block with no blank lines as HTML, remark-mdx sees a code node. Case `21-mdx-jsx-tight.mdx`: Rust 0 fences, remark 1. Result: manifest miss, build fails closed naming the file (acceptable); optionally also lint it (raw-opener scan like `mermaid_gate::counts` does for `.mdx`).

### Pitfall 12: file.path differs between md and mdx
md: `<repo>/docs-astro/src/content/docs/reference/vox-diagram.md` (mirror path); mdx: `<repo>/docs/src/p89-fixture.mdx` (realpath of the symlink). Derive the slug from either marker; fall back to the last two path segments (mounted `repo/*` pages, wrapped or symlinked, land here).

### Pitfall 13: figure sizing
The SVG has only a `viewBox`. With `max-width: 100%; height: auto` a 248 px diagram is stretched to the 676 px column (~2.7x). Setting `--vd-w:<viewBox width>px` on the figure and `svg { width:100%; max-width: var(--vd-w) }` gives natural size for small diagrams. Wide diagrams still shrink: measured at 676 px column: 945 px diagram to 0.71, 1355 px multi-hop sequence to 0.50 (about 7 px text). Legibility policy (scroll with a minimum scale vs shrink) is a Phase 89 decision (Open Question 3).

### Pitfall 14: `astro preview` after a rebuild
The preview server kept serving the pre-rebuild file list (pages missing the new `style`). Playwright's own `webServer` starts fresh in CI (`reuseExistingServer: !process.env.CI`); locally, kill any running preview before re-running specs.

### Pitfall 15: mounted repo docs
`setup-content.mjs` mounts ~22 out-of-tree repo markdown files under `repo/` (`Mounted 22 repo docs`). The Rust walk covers `docs/src` only. A `vox-diagram` fence in a mounted README would miss and fail the build (fail-closed, with file name). No such fence exists today; note it in the reference page or extend the walk later.

## Code Examples

### Integration (astro.config.mjs) `[VERIFIED: prototype build and dev]`
```js
import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { remarkVoxDiagram } from './src/plugins/remark-vox-diagram.mjs';

const manifestFile = new URL('./.vox-diagram/manifest.json', import.meta.url);
const manifestDigest = existsSync(manifestFile)
  ? createHash('sha256').update(readFileSync(manifestFile)).digest('hex') : 'none';

// integrations: [ { name: 'vox-diagram', hooks: {
//   'astro:config:setup': ({ command, updateConfig, addWatchFile }) => {
//     addWatchFile(manifestFile);   // dev: manifest change restarts the server (log: "Configuration file updated. Restarting...")
//     updateConfig({ markdown: { remarkPlugins: [[remarkVoxDiagram, { strict: command === 'build', digest: manifestDigest }]] } });
//   },
//   'astro:build:start': () => assertNoMisses(),   // throws with every file and the exact command
// } }, ...existing ]
// starlight({ expressiveCode: { shiki: { langs: [voxGrammar], langAlias: { 'vox-diagram': 'text' } } },
//             customCss: ['./src/styles/reader-layout.css', './src/styles/vox-diagram.css'],
//             plugins: [starlightLlmsTxt({ ..., customSelectors: { all: ['svg.vd-diagram'] } })] })
```
`command` is `'build'` for `astro build` (log line `[vox-diagram-guard] command=build`) and `'dev'` for `astro dev`. `updateConfig` merges after the static `markdown.remarkPlugins`, before Starlight's own hook, and the MDX integration picked it up (both md and mdx rendered figures).

### Plugin core (prototype, abridged) `[VERIFIED: ran in build]`
```js
const WS_END = /\p{White_Space}+$/u;                      // == Rust char::is_whitespace
export const normalizeSource = (b) => b.replace(/\r\n/g, '\n').replace(/\r/g, '\n').replace(WS_END, '');
// svgToHast(svg): sticky regex over <tag attrs> | </tag> | text; throws on unknown entity / unbalanced tag
// reprefix(nodes, prefix): id, aria-labelledby, marker-end:  /(^|[\s#"(])vd-(title|desc|arrow)\b/g -> `$1${prefix}-$2`
// per fence (node.type==='code' && node.lang==='vox-diagram' && !node.meta):  n += 1; key = normalizeSource(node.value)
//   hit  -> retype 'vdFigure'; data = { hName:'figure', hProperties:{className:['vd-figure'], style:`--vd-w:${vbw}px`}, hChildren:[svg..., details] }
//   miss -> data.hProperties.className = ['vd-figure','vd-missing'], role:'note', text "diagram not rendered — run `pnpm diagrams`"
//           strict ? record + throw : record
// slug: path after /src/content/docs/ or /docs/src/, strip .md/.mdx and /index, lowercase, [^a-z0-9]+ -> '-', empty -> 'index'
```
Use `!node.meta` and `node.lang === 'vox-diagram'`: Rust matches only an exact trimmed info string, so ```` ```vox-diagram title="x" ```` (meta set) and ```` ```vox-diagram,x ```` (lang `vox-diagram,x`) must be skipped by the plugin too (the lint already hard-errors on those near-misses).

### Rust side sketch (`pipeline/render_manifest.rs`, tests first)
```rust
// fn render_diagrams(root:&Path, out:&Path) -> Result<ManifestStats, Vec<String>>
// walk: super::lint::gather_md_files(&root.join("docs/src")); skip rel.starts_with("docs/src/archive/")   (same as mermaid_gate::docs)
// per file: fence_issues(&text) hard errors -> collect DiagramIssue::render(rel) lines (reuse, do not reformat)
// per fence: key = fence::normalize(&f.body);  BTreeMap<String,String> source -> svg, rendered once:
//            vox_diagram::validate(&key, ValidateOpts{ from_mermaid:false, emit:Some(Emit::Svg) })  -> .emitted | .emit_error
// output: serde_json { "schema_version":1, "diagrams":[{"source":..., "svg":...}, ...] } sorted by source; write tmp + rename
// exit: 1 with all collected errors (still write the manifest of the valid diagrams so dev keeps working); 0 otherwise
```
Argument parsing follows the existing `args.contains(...)` style in `lint_in` (`mod.rs:243-284`): handle `--render-diagrams` (and `--out <path>`, default `docs-astro/.vox-diagram/manifest.json` under `root`) before `FacetCtx::load` like the ratchet tools. Dispatch lives in `mod.rs` (one line); the logic stays in the new file.

### CSS (`src/styles/vox-diagram.css`) `[VERIFIED: rendered light/dark/forced]`
```css
.vd-figure {
  --vd-text: var(--sl-color-white);       --vd-muted: var(--sl-color-gray-2);
  --vd-node-fill: var(--sl-color-black);  --vd-node-stroke: var(--sl-color-gray-3);
  --vd-edge: var(--sl-color-gray-3);      --vd-container-fill: var(--sl-color-gray-6);
  --vd-container-stroke: var(--sl-color-gray-3);   /* WR-04: was gray-5 */
  --vd-bg: var(--sl-color-black);         --vd-font: var(--sl-font-system);
  margin: 1.5rem 0; overflow-x: auto;
}
.vd-figure > svg { display: block; width: 100%; max-width: var(--vd-w, 100%); height: auto; margin-inline: auto; }
@media (forced-colors: active) {
  .vd-figure { --vd-text: CanvasText; --vd-muted: CanvasText; --vd-node-fill: Canvas; --vd-node-stroke: CanvasText;
               --vd-edge: CanvasText; --vd-container-fill: Canvas; --vd-container-stroke: CanvasText; --vd-bg: Canvas; }
}
```
The variable set equals the crate's `THEME_VARS` (`crates/vox-diagram/src/svg/mod.rs:77-87`): `--vd-text, --vd-muted, --vd-node-fill, --vd-node-stroke, --vd-edge, --vd-container-fill, --vd-container-stroke, --vd-bg, --vd-font`. Starlight sets `data-theme` on `<html>`; the map flips with it. The fallbacks in the crate (`#1f2328`, `#57606a`, `#fff`, `#848d97` ...) are the standalone palette; the site never reaches them.

### Playwright checks (executed; abridged) `[VERIFIED: ran against astro preview of the scratch dist]`
```ts
// ids: [...document.querySelectorAll('[id]')] duplicates == []; every aria-labelledby token and url(#x) in marker-end resolves via getElementById
// fit:  for each svg text: box = nearest previous sibling matching '.vd-node-shape, .vd-container, .vd-edge-label-bg, .vd-note, .vd-actor'
//       text.getBBox() inside box.getBBox() (eps 0.5); polygon (diamond): all 4 text corners satisfy |dx|/(w/2)+|dy|/(h/2) <= 1
//       stack applied with page.addStyleTag('.vd-figure { --vd-font: <stack> !important }') then await document.fonts.ready
// contrast: getComputedStyle(text).fill vs getComputedStyle(box).fill >= 4.5 ; stroke of .vd-node-shape/.vd-edge/.vd-container vs page bg >= 3
//           theme via document.documentElement.dataset.theme = 'light'|'dark' (same as reader-layout.spec.ts)
// forced-colors: page.emulateMedia({ forcedColors: 'active' }) then computed fill/stroke of text and box
```
Executed results (this machine, Chromium 153): ids and reference resolution pass on all pages (incl. a page with two identical fences); contrast light: min text 11.71, min non-text 7.02; dark: min text 10.06, min non-text 5.27 (page bg `rgb(23, 24, 28)` dark, `rgb(255, 255, 255)` light); forced-colors computed `text rgb(0,0,0)` on `boxFill rgb(255,255,255)`, stroke black. Mutation check: forcing a pale palette made the edge-stroke ratio 1.16 (threshold 3), so the custom check does fail on a bad palette.

### Workflow edits
```yaml
# docs-deploy.yml  on.push.paths: add - 'crates/vox-diagram/**'      (today: crates/vox-doc-pipeline/** only)
# docs-quality.yml on.pull_request.paths and on.push.paths: replace the narrow 'crates/vox-diagram/tests/fixtures/**'
#                  with  - 'crates/vox-diagram/**'   (keep the Phase 84 comment; workflow-paths.test.mjs asserts a fixtures path stays covered)
# both: before "Build Starlight":
#   - name: Pre-render VoxDiagram fences
#     run: cargo run -p vox-doc-pipeline -- --render-diagrams        # cwd = repo root (no working-directory)
# docs-quality only, after "Reader layout visual-stepper checks":
#   - run: sudo apt-get install -y fonts-dejavu-core fonts-liberation      # working-directory not needed
#   - run: npx playwright install chromium --with-deps     (working-directory: docs-astro; before BOTH browser specs)
#   - run: npx playwright test tests/vox-diagram.spec.ts --reporter=line --output=test-results/vd-run
#   - upload-artifact the diagram review bundle (if: always())
```
Cargo availability: both jobs already install Rust (`./.github/actions/setup-rust`) and build `vox-doc-pipeline` (deploy: `cargo build -p vox-doc-pipeline --profile ci` then `cargo run -p vox-doc-pipeline` in the default profile; quality: `cargo build -p vox-cli -p vox-doc-pipeline`). `prebuild` will also run the wrapper, so the explicit step is redundant but gives a named, early failure; it is milliseconds when the binary exists. No other workflow builds docs-astro (`grep docs-astro .github/workflows`: only these two). Concurrency blocks and `package-manager-cache: false` are untouched.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Client-side Mermaid via CDN (the other checkout's `PageSidebar.astro` loader) | Build-time inline SVG from a Rust renderer | this milestone | no network at read time, deterministic, themed |
| `data.hName` on a `code` node | unknown mdast node + `hName`/`hChildren` | (mdast-util-to-hast 13.x behavior) | no `<pre>` wrapper |
| Throw to fail builds | integration hook that fails the build | Starlight docs loader swallows md render errors | the only reliable fail-closed path for md |

**Deprecated/outdated assumptions:** `ARCHITECTURE.md` suggested `docs-astro/src/generated/diagrams.manifest.json` and a `hast-util-from-html` dependency; CONTEXT (newer, locked) uses `docs-astro/.vox-diagram/manifest.json`, and this research shows the dependency is avoidable.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The hosted `ubuntu-latest` image may lack DejaVu Sans, so `fonts-dejavu-core` should be installed explicitly | Pitfall 9, Workflow edits | If DejaVu is present the extra apt step is harmless; if absent and not installed the wide-stack test would be vacuous (guarded by the distinctness probe) |
| A2 | `npx playwright install chromium --with-deps` works inside docs-quality exactly as in docs-deploy's smoke job | Pitfall 8 | Step fails in CI; fallback is installing without `--with-deps` plus explicit apt packages |
| A3 | docs-quality's reader-layout step would fail today for lack of a browser (never observed running; always `skipped`) | Pitfall 8 | If a browser is somehow available the extra install is redundant, not harmful |
| A4 | Dev-mode md staleness after a manifest-triggered restart has the same cause as the build-mode store caching (not isolated further) | Pitfall 3 | The mitigation (delete `data-store.json` in the wrapper) may not be sufficient in dev; verify during the plan |
| A5 | The measured Verdana ratio (1.05-1.19 x the Arial-metric model) approximates DejaVu Sans on Linux | Open Question 1 | DejaVu not installed here (`"DejaVu Sans"` fell back to the Arial-class default on this Mac); CI numbers may differ. Treat the slack proposal as provisional until measured on the runner |
| A6 | Phase 90's migrated corpus will be picked up automatically if the spec discovers figure pages from `dist/` rather than a hardcoded list | Validation Architecture | If discovery is hardcoded, Phase 90 must re-edit the spec |

## Open Questions

1. **Font policy (decision needed, proposal only).** Measured actual/model ratios by stack on the generated fixture (`resources: /scratchpad/p89/repo/docs-astro/tests/font-ratio.spec.ts`): `system-ui` (SF) min 1.013 / median 1.039 / max 1.067 (headroom to the 1.08 slack is only 1.3%); Arial and Helvetica 0.994-1.000; Verdana min 1.047 / median 1.145 / max 1.186; Trebuchet 0.90-1.08. Under Verdana the following boxes overflow horizontally by up to about 7 px per side: sequence message labels (`LABEL_PAD` 4 px, `layout/seq.rs:39`), graph edge labels (`EDGE_LABEL_PAD` 8 px, `layout/input.rs:13`); node labels (14 px padding each side) passed. Model: `SLACK_PCT = 8`, `FONT_PX = 14`, table = Arimo Regular advances (`layout/text.rs:10-18,20`). Proposal for the user: either raise `SLACK_PCT` toward ~20 or add a padding floor for message/edge labels; either re-blesses goldens, so it is **not** part of this phase's plan unless the user approves. Minimum for this phase: keep the wide-stack test, but assert it against the policy the user picks (if the policy stays 8%, the wide-stack fit test must be scoped to node labels or marked as a tracked known failure; do not weaken it silently).
2. **Hidden fixture page vs extending the reference page.** Recommendation: extend `docs/src/reference/vox-diagram.md` with a "Layout stress examples" section (long labels, diamond/cylinder/rounded, a container with a long title, multi-hop sequence). Needs the user only if they prefer a hidden page (then `status: research` gives noindex + Internals + llms exclusion per `page-status.mjs`, but still pays doc-lint cost).
3. **Legibility policy for wide diagrams.** Today wide diagrams shrink to the column (0.50 for a 4-actor sequence). Options: accept; or `min-width` + `overflow-x:auto` scroll below a scale (suggest 0.7); decide before the Playwright review bundle is captured.
4. **axe.** CONTEXT says "axe contrast passes". Axe cannot evaluate SVG text (Finding 3), so a custom computation is required regardless. If the user still wants axe for other rules (`svg-img-alt`, ARIA validity), it is a new devDependency needing approval (and a `checkpoint:human-verify` because of the SUS recency flag). Recommendation: do not add.
5. **Visible source block.** The collapsed `<details>` also makes the DSL searchable (Pagefind indexes it) and keyboard-accessible; add `data-pagefind-ignore` to the `svg` (its `<desc>` is machine text) if search noise matters. Default recommendation: ignore the svg, index the source.
6. **Container title vs incoming edge (by-eye finding).** In the stress graph the edge `d -> grp.x` runs straight through the wrapped container title text (light and dark). This is the Phase 85 "soft metric" (edge crosses a title band), not a Phase 89 defect; record in the review bundle and leave to a layout follow-up.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| node | docs-astro, unit tests | yes | v26.11.1 local (CI uses 24) | - |
| pnpm | docs-astro scripts | yes | 12.11.2 local (CI pins 11) | - |
| cargo (via build broker shim) | pre-render | yes | `~/.vox/build-broker/bin/cargo` | pre-built `target/debug/vox-doc-pipeline` exists (also `target/debug/vox`) |
| docs-astro node_modules | build, Playwright | yes | in the worktree (`docs-astro/node_modules`, 700 MB) | `pnpm install --frozen-lockfile` |
| Playwright Chromium | browser specs | yes locally | chromium-1243 matches @playwright/test 1.63.0 | CI needs install (Pitfall 8) |
| Fonts: Verdana, Arial | wide / narrow stacks (local) | Verdana and Arial yes; DejaVu and Liberation no (fell back) | - | install `fonts-dejavu-core fonts-liberation` in CI |
| network (npm registry, gh) | version checks | yes | - | - |
| `fc-list`, `rsync`, `python -c`, `node -e` | n/a | blocked by the local shell allowlist | - | use script files / `cp` |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** DejaVu/Liberation fonts locally (not needed: the CI job installs them); Chromium in docs-quality CI (add install step).

## Validation Architecture

Config: `workflow.nyquist_validation` not read as false (absent treated as enabled).

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo nextest (Rust), `node:test` (unit), `@playwright/test` 1.63.0 (build-output fs specs + browser specs) |
| Config file | `docs-astro/playwright.config.ts` (testDir `./tests`, `testIgnore: ['unit/**','lib/**','fixtures/**']`, chromium project, webServer `pnpm preview --port 4321` unless `BASE_URL` set) |
| Quick run command | `cargo nextest run -p vox-doc-pipeline render_manifest` (background it) ; `cd docs-astro && node --test tests/unit/remark-vox-diagram.test.mjs tests/unit/workflow-paths.test.mjs` |
| Full suite command | `cargo nextest run -p vox-doc-pipeline` ; `cd docs-astro && node --test "tests/unit/*.test.mjs" && pnpm build && npx playwright test tests/build-output/ tests/vox-diagram.spec.ts` |

### Phase Requirements to Test Map
| Req / SC | Behavior | Test Type | Automated Command | File Exists? |
|----------|----------|-----------|-------------------|-------------|
| DOCS-03 / SC1 | Manifest is deterministic, deduped, sorted; invalid fence or render error exits 1 with `file:line:col`; valid diagrams still written; archive skipped; `--out` honored; uses cwd-relative root | Rust unit (test-first) | `cargo nextest run -p vox-doc-pipeline render_manifest` | Wave 0 (`pipeline/render_manifest.rs`) |
| DOCS-03 / SC1 | Key parity: normalized Rust key equals remark node value on the adversarial corpus (list, ordered list, nested list, blockquote, CRLF, tilde, trailing/leading blank, fence indent, tabs, trailing spaces, empty, unicode, aside, frontmatter-with-fence, HTML block) | Rust golden + Node unit | commit `crates/vox-diagram/tests/fixtures/fence-keys/*.md` + expected keys JSON; Rust test asserts `normalize(extract(..))`, Node test reads the same fixtures and parses with remark (resolved via `createRequire` through `astro`) | Wave 0 |
| DOCS-03 / SC1 | Plugin: normalizeSource vectors (incl. U+FEFF vs U+0085 whitespace), strict `svgToHast` accept/reject, `reprefix`, `pageSlug` (md mirror path, mdx realpath, mounted `repo/` page, empty -> `index`), `meta`/near-miss skipped, strict miss message names file and `pnpm diagrams`, non-strict placeholder, `assertNoMisses` | node:test | `node --test tests/unit/remark-vox-diagram.test.mjs` | Wave 0 |
| DOCS-03 / SC1 | Whole-pipeline md output: `<figure class="vd-figure"` present, no `<pre><figure`, ids `vd-<slug>-<n>-*` | node:test via `createMarkdownProcessor` | same file | Wave 0 |
| DOCS-03 / SC1 | Built site: every fence rendered, mdx and md; no `vd-missing`; no `<pre><figure`; figure count equals manifest fence count; `llms-full.txt` has a ```` ```vox-diagram ```` fence and no `<desc>` sentence | Playwright fs-only | `npx playwright test tests/build-output/diagrams.spec.ts` | Wave 0 |
| DOCS-03 / SC1 | Negative control: with a fence absent from the manifest, `astro build` exits non-zero for a `.md` page AND an `.mdx` page, message has the file and `pnpm diagrams` | manual once (executor pastes output) or one CI control step | temporary fence + `VOX_DIAGRAM_MANIFEST`-style override or edit; mutation: delete the `astro:build:start` guard and confirm the md control then exits 0 | manual |
| DOCS-04 / SC2 | No duplicate ids, references resolve, per page with several diagrams | Playwright browser | `npx playwright test tests/vox-diagram.spec.ts -g "ids"` | Wave 0 |
| DOCS-04 / SC2 | Every `<text>` in its box under two pinned stacks, with a font-distinctness probe | Playwright browser | `-g "fits"` | Wave 0 |
| DOCS-04 / SC2 | Contrast (text >= 4.5 against its box fill; node/edge/container strokes >= 3 against page bg) in light and dark; forced-colors computed styles + screenshot | Playwright browser | `-g "contrast"`, `-g "forced"` | Wave 0 |
| DOCS-04 / SC3 | Workflow path filters cover `crates/vox-diagram/**` source paths for deploy push, quality push and quality pull_request; pre-render step precedes `Build Starlight` | node:test (extend `workflow-paths.test.mjs`) | `node --test tests/unit/workflow-paths.test.mjs` | exists; extend |
| Carried (Phase 86 review) | Light/dark screenshots of graph and sequence (incl. multi-hop labels) inspected by eye | manual | executor reads PNGs from the review bundle and records findings | manual |

### Sampling Rate
- **Per task commit:** `cargo check -p vox-doc-pipeline` + the new Rust tests (background); `node --test` on the touched unit test files.
- **Per wave merge:** `node --test "tests/unit/*.test.mjs"` + `pnpm build` + `npx playwright test tests/build-output/ tests/vox-diagram.spec.ts` (needs Chromium).
- **Phase gate:** hosted CI green (docs-quality incl. the new browser step, docs-deploy build), reviewed screenshots, negative-control evidence pasted into the summary.

### Wave 0 Gaps
- [ ] `crates/vox-doc-pipeline/src/pipeline/render_manifest.rs` with tests first (determinism, dedupe, error exit, archive skip, atomic write).
- [ ] `crates/vox-diagram/tests/fixtures/fence-keys/` shared parity corpus (28 cases drafted in `/scratchpad/p89/cases/`, 26 identical across parsers; the two known differences, tight-JSX mdx and lone CR, are expected-fail-closed cases).
- [ ] `docs-astro/tests/unit/remark-vox-diagram.test.mjs`, `tests/build-output/diagrams.spec.ts`, `tests/vox-diagram.spec.ts`, `tests/lib/diagrams.ts`.
- [ ] Extend `workflow-paths.test.mjs` (assert `crates/vox-diagram/src/svg/mod.rs` is covered by deploy push and quality push/pull_request).
- [ ] No framework install needed.

## Security Domain

`security_enforcement` not set to false in config reads (absent = enabled).

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication / V3 Session | no | static site |
| V4 Access Control | no | static site |
| V5 Input Validation | yes | Strict closed-grammar SVG parser that throws on any unknown entity/tag/attribute shape; hast element nodes only (never raw HTML / `set:html`); `id`/slug sanitized to `[a-z0-9-]` before use; fence key is exact-match lookup, never evaluated |
| V6 Cryptography | no (manifest digest is a cache key only; `node:crypto` sha256 in the config, not a security boundary) | n/a |
| V14 Config / supply chain | yes | No new npm dependency; `pnpm install --frozen-lockfile` unchanged; workflow `permissions: contents: read` unchanged |

### Known Threat Patterns for this stack
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Hostile label reaching markup (XSS via diagram text) | Tampering | Crate writer is the single escaper and no attribute value derives from a label (`writer.rs` doc); plugin re-parses with a fail-closed grammar and builds hast (text nodes, not HTML strings) |
| Manifest tampering / stale manifest shipped | Tampering | Gitignored, regenerated by `prebuild` in CI from the same tree; key mismatch fails the build (guard + throw) |
| Placeholder cached then shipped | Integrity | `strict` and manifest digest in plugin options + build-output assertion that no `vd-missing` exists in `dist/` |
| Path traversal via fence path | - | no file paths come from fence content |
| Third-party script injection via Mermaid CDN loader | Tampering | out of scope (deferred); figure markup contains no `pre[data-language="mermaid"]` / `code.language-mermaid` (Finding 6) |

## Findings by research question (evidence index)

**1. Astro 6 / Starlight pipeline, plugin order, EC, hast replacement.**
- Registration site: `markdown.remarkPlugins: [remarkVoxInclude, [remarkDocLinks, {repoUrl}]]` in `docs-astro/astro.config.mjs`; Starlight adds `@astrojs/mdx` and Expressive Code (`expressiveCode: { shiki: { langs: [voxGrammar] } }`). `markdown.remarkPlugins` apply to both md and mdx; an integration's `updateConfig` additions did too.
- `remark-vox-include.mjs` is a plain `function remarkVoxInclude() { return function transformer(tree, file) {...} }` that walks `code` nodes with a hand-rolled `visit`, resolves `{{#include path:anchor}}` against `dirname(file.path)` and throws on failures. Ours follows the same shape; remarkVoxInclude and ours do not interact (ours matches `lang === 'vox-diagram'`).
- EC runs as a **rehype** plugin, after remark-rehype: it cannot take the fence before remark. It does re-process any `pre > code` we emit (Pitfall 6); a Shiki/EC alias matters only for that inner source block.
- md vs mdx difference: Starlight loads md through its docs loader (content store, errors swallowed, mirror `file.path`), mdx through `@mdx-js/rollup` (Vite transform, errors fail the build, realpath `file.path`).
- `data.hName`/`hProperties`/`hChildren` works in both when applied to an unknown node type (Pattern 1); style strings with custom properties (`--vd-w:142.5px`) survive the mdx JSX conversion.

**2. `llms-full.txt`.** Built from rendered HTML (Pitfall 5); fence source is NOT retained by default; retained via the `<details>` block; SVG text suppressed via `customSelectors`. File: `dist/llms-full.txt` (existing `tests/build-output/llms.spec.ts` already reads llms output; extend there).

**3. Playwright.** Config `docs-astro/playwright.config.ts`; `playwright test --list`: **82 tests in 10 files** (not 36): baseline 1, build-output/{archive 5, dates 3, honesty 9, links 4, llms 3, repo-docs 6}, reader-layout 12, smoke 15, trust-signals 24. CI runs only build-output and reader-layout (docs-quality) and smoke against the live site (docs-deploy). Specs run against `astro preview` of `docs-astro/dist` (built by `pnpm build`); build-output specs read files via `tests/lib/dist.ts` (`readDist`, `listDistHtml`). Theme switching convention: `document.documentElement.dataset.theme = 'light'|'dark'` (`reader-layout.spec.ts:27-30`). axe: not in docs-astro (`grep axe-core docs-astro/pnpm-lock.yaml`: no match; only `crates/vox-gui/ui` has it). Axe experiment (axe-core 4.14.0 via `@axe-core/playwright` 4.11.3 installed in a scratch dir): on the diagram figures axe reports `color-contrast` as **incomplete (36 nodes, "background color could not be determined because element contains an image node")** for both a correct and a deliberately unreadable palette (`#ccc` text on `#fff`), with no violation either way. Therefore axe cannot gate SVG text contrast.

**4. Pre-render / key parity.** Walk: reuse `gather_md_files` + archive skip exactly as `mermaid_gate::docs` (`crates/vox-doc-pipeline/src/pipeline/mermaid_gate.rs:70-94`). Prefix: Pattern 2. Plugin locates the manifest relative to its own file (`join(HERE,'../../.vox-diagram/manifest.json')`) with an options override for tests. Parity experiment: Rust `fence::extract` + `fence::normalize` (the real `fence.rs` compiled via `#[path]` into a scratch crate with pulldown-cmark 0.13) vs `remark-parse` + `remark-gfm` + `remark-directive` (+ `remark-mdx` for mdx) on 28 inputs: 26 identical, 2 differ: tight-JSX mdx (Pitfall 11) and a lone-CR file (git normalizes line endings, not a real case); both fail closed.

**5. Workflows.** Quoted in Code Examples. Structure facts: docs-deploy `build-docs` job: checkout fetch-depth 0, setup-rust, apt deps, pnpm/node (package-manager-cache false), `cargo build -p vox-doc-pipeline --profile ci`, `cargo run -p vox-doc-pipeline`, `pnpm install --frozen-lockfile`, `pnpm build`, upload artifacts; deploy jobs do not rebuild. docs-quality: build docs tooling, lint, doctests, drift gate, checks, `pnpm install --frozen-lockfile`, `pnpm build`, lychee, unit tests, build-output, reader-layout, biome (`docs-astro/src/**/*.{ts,tsx,js,jsx,astro}` only: `.mjs`, `.css` and `tests/` are not linted), markdown lint. Existing unit test `tests/unit/workflow-paths.test.mjs` already parses these path lists: extend it (TDD for SC3). Both workflows run under `timeout-minutes: 30`; the Astro build is 23-42 s here.

**6. The jsdelivr Mermaid loader.** Read-only inspection of `/Users/brbrainerd/dev/vox/docs-astro/src/components/PageSidebar.astro`: `initMermaidDiagrams()` uses `document.querySelectorAll('pre[data-language="mermaid"], pre > code.language-mermaid')`, imports `https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs`, marks `pre.dataset.mermaidProcessed`, inserts a `div.vox-mermaid-diagram` before the nearest `.expressive-code` wrapper and hides the wrapper. Our output has no `pre` outside the `<details>` source block, whose `pre` has `data-language="vox-diagram"` and `code` class `language-vox-diagram`; neither selector can match. Status update: in that checkout the loader is no longer uncommitted: `git log` shows it in local commit `ceb1b5d1a` ("feat(milestone): complete v1.5 ... (Phases 01-88)", 2026-10-16) on that checkout's `main`; `origin/main` and this worktree's `PageSidebar.astro` (last change `d2d943c48`) do not contain it (0 mermaid mentions). Expect a merge-time interaction only if a later phase changes `PageSidebar.astro`; Phase 89 does not.

**7. Validation Architecture.** Section above.

## Sources

### Primary (HIGH confidence)
- Scratch prototype and logs (`/private/tmp/claude-501/.../scratchpad/p89/`): `build0..21.log`, `repo/docs-astro/src/plugins/remark-vox-diagram.mjs`, `tests/vox-diagram.spec.ts`, `font-ratio.spec.ts`, `axe.spec.ts`, `shots/*.png`, `cases/`, `rust-fences.json`
- `docs-astro/node_modules/.pnpm/mdast-util-to-hast@13.2.1/.../lib/handlers/code.js:17-49`, `lib/state.js:352-391,407-423`
- `docs-astro/node_modules/starlight-llms-txt/entryToSimpleMarkdown.ts`, `generator.ts`, `patches/starlight-llms-txt@0.10.0.patch`
- `crates/vox-diagram/src/fence.rs`, `src/svg/mod.rs`, `src/svg/writer.rs`, `src/validate.rs`, `src/layout/text.rs`; `crates/vox-doc-pipeline/src/pipeline/{diagrams.rs,mermaid_gate.rs,mod.rs}`
- `.github/workflows/docs-deploy.yml`, `docs-quality.yml`; `docs-astro/{astro.config.mjs,package.json,playwright.config.ts,scripts/setup-content.mjs,src/utils/*,tests/**}`
- `.planning/workstreams/voxdiagram/phases/85-graph-layout-deterministic-svg/85-RESEARCH.md` (theme map, contrast table), `85-REVIEW-FIX.md` (WR-04)
- `/Users/brbrainerd/dev/vox/docs-astro/src/components/PageSidebar.astro` (read-only)

### Secondary (MEDIUM confidence)
- `gh run view` step conclusions for recent docs-quality runs; `gsd-tools package-legitimacy check`

### Tertiary (LOW confidence)
- Hosted-runner font inventory (A1) not inspected.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH, no new dependency, versions read from disk.
- Architecture: HIGH, each pattern executed in a real `astro build` and (for dev) `astro dev`; md/mdx both proven.
- Pitfalls: HIGH for 1-7, 10-14 (reproduced); MEDIUM for 8-9 and 15 (CI behavior inferred from workflow files and run history).
- Font slack numbers: MEDIUM (one machine, Verdana only; Linux DejaVu unmeasured).

**Research date:** 2026-10-17
**Valid until:** 2026-11-16 (Astro/Starlight patch releases can change loader error handling; re-run the md negative control after any Astro or Starlight bump)
