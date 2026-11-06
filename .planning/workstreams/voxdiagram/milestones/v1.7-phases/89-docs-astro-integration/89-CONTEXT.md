# Phase 89: Docs-Astro Integration - Context

**Gathered:** 2026-10-17
**Status:** Ready for planning

<domain>
## Phase Boundary

voxlang.org (docs-astro, Astro/Starlight) renders every ```` ```vox-diagram ```` fence as an inline, accessible, themed SVG produced by the Rust pipeline, and `pnpm build` fails rather than shipping a missing render. Includes the docs workflows (deploy + quality) running the pre-render step, Playwright verification in a real browser, and the by-eye review carried from Phase 86. No corpus migration (Phase 90), no measurement harness (Phase 91).

Requirements: DOCS-03, DOCS-04.

</domain>

<decisions>
## Implementation Decisions

### Pre-render pipeline & dev loop
- `docs-astro` `prebuild` / `predev` scripts run `cargo run -p vox-doc-pipeline -- --render-diagrams` (new flag); docs-deploy and docs-quality workflows run the pre-render before `pnpm build` and trigger on `crates/vox-diagram/**` changes.
- Manifest: gitignored `docs-astro/.vox-diagram/manifest.json`, keyed by the normalized fence source text (exact text keys; no hashing).
- `astro dev` after editing a fence: the remark plugin renders a visible placeholder ("diagram not rendered — run `pnpm diagrams`"); `pnpm build` fails hard on any manifest miss, naming the file and the exact command.
- Ids & markup: per-fence id prefix `vd-<page-slug>-<n>` (page order, deterministic, validated `[a-z][a-z0-9-]*`); output `<figure class="vd-figure">` wrapping the inline SVG, inserted as hast nodes (`data.hName` / `hChildren`, MDX-safe — not raw HTML strings). This guarantees the other checkout's uncommitted `pre[data-language="mermaid"]` CDN loader (`PageSidebar.astro`) never processes VoxDiagram output. The fence source is retained for `llms-full.txt`.

### Theming & browser verification
- Site CSS maps `--vd-*` → `--sl-*` (the Phase 85 candidate map including the WR-04 container-stroke contrast fix) for light and dark, plus a `@media (forced-colors: active)` block using system colors.
- Playwright fixtures: the `docs/src/reference/vox-diagram.md` page (graph + sequence, several diagrams on one page) is the primary fixture; add a test-only fixture page excluded from sidebar/search only if more coverage is needed (e.g. long labels, containers, unicode).
- Checks: no duplicate ids per page; every `<text>` bbox inside its box under two font stacks (system-ui and a wide Verdana/DejaVu stack); axe contrast passes in light + dark; forced-colors screenshot legible.
- Carried from Phase 86 + font policy: the executor captures light/dark Playwright screenshots of graph and sequence diagrams (incl. multi-hop message labels) into the review bundle and inspects them (Read the PNGs); confirm the Phase 85 static-width-table font policy or propose a wider slack if any text overflows (proposal only — changing slack re-blesses goldens).

### Post-research decisions (user, 2026-10-17)
- Font fit: add a minimum horizontal padding floor for message labels, edge labels and node labels in the Rust layout, sized so ~20% wider fonts (Verdana/DejaVu class) fit; keep the static width table. Re-bless affected goldens (review diffs). This is a Rust change in `crates/vox-diagram` and must keep determinism + all geometry oracles green.
- No axe dependency: contrast is verified by the researched Playwright WCAG computation (mutation-proven to fail a bad palette). CONTEXT's "axe contrast" check is replaced by this.
- Wide diagrams: scale down to 0.6 minimum, then keep that size and let the figure scroll horizontally (keyboard-focusable, `tabindex="0"`, labelled region); set `--vd-w:<viewBox width>px` so small diagrams don't stretch.
- Stress fixtures (long labels, containers, unicode, multi-hop sequence) are added as documented examples on `docs/src/reference/vox-diagram.md` (no hidden page).
- Research-driven implementation facts (locked by evidence): retype the mdast node to a custom type (not `code`+hName) to avoid the `<pre>` wrapper; `shiki.langAlias {'vox-diagram':'text'}`; hard failure via an inline Astro integration at `astro:build:start` (remark throws don't fail `.md` builds); plugin registered with `strict: command === 'build'` + `addWatchFile`; manifest `digest` passed in plugin options to invalidate Astro's data-store; wrapper script clears the data-store in dev; render once with prefix `vd` and re-prefix ids to `vd-<slug>-<n>` in JS; emit a `<details><pre><code class="language-vox-diagram">` source block and `starlightLlmsTxt({customSelectors:{all:['svg.vd-diagram']}})` so llms-full keeps the fence source; JS normalizer uses `/\p{White_Space}+$/u`.
- CI: `docs-quality.yml` needs `playwright install` for the new spec; path filters add `crates/vox-diagram/**` to deploy push and quality push + pull_request; the pre-render wrapper sets cwd to the repo root.

### Carried forward
- Rust side: deterministic, no new crate edges (vox-doc-pipeline already depends on vox-diagram), files < 400 lines, tests first.
- docs-astro is a JS/MJS package (existing convention `.mjs`); pnpm 11 with `--frozen-lockfile` in CI; any new npm dependency (e.g. a hast helper or axe) requires user approval — prefer already-installed packages.
- Workflow edits must keep the `concurrency:` block and cache rules (main-only cache saves; `package-manager-cache: false` for setup-node caches).

### Claude's Discretion
- Plugin file names, manifest JSON shape, placeholder styling, Playwright spec layout.

</decisions>

<code_context>
## Existing Code Insights

- `docs-astro/src/plugins/remark-vox-include.mjs` (existing remark plugin pattern); `docs-astro/tests/unit/*.test.mjs` (node --test suites); Playwright setup in docs-astro (36 tests per v1.5 notes) — research to confirm paths.
- `vox_diagram::svg::render(&Ir, &SvgOpts)` with `id_prefix`, `title`; `vox_diagram::fence::extract`; `vox_doc_pipeline` lint pass structure (Phase 87).
- `docs-deploy.yml` already builds `vox-doc-pipeline` before `pnpm build`; `docs-quality.yml` runs unit tests after build.

</code_context>

<specifics>
## Specific Ideas

Build test: a fence with no manifest entry fails `pnpm build` with the file name and `pnpm diagrams` in the message.

</specifics>

<deferred>
## Deferred Ideas

- Removing the jsdelivr Mermaid loader (belongs to the other session's uncommitted work; revisit after merge/migration).
- Live re-render in `astro dev` via a Rust watcher.

</deferred>
