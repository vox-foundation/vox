# Phase 342: Docs Authoring Parity and Starlight 0.42 Upgrade - Context

**Gathered:** 2026-11-05
**Status:** Ready for planning
**Source:** External "AsciiDoc-Parity Modernization Blueprint" critiqued against the live tree; SSOT `docs/src/architecture/docs-site-asciidoc-parity-findings-2026.md`.

<domain>
## Phase Boundary

Finish the parts of the blueprint that survived verification: framework upgrade that unblocks ecosystem plugins, GitHub-alert rendering, and build-time link validation. Everything else in the blueprint is either already shipped or built on artifacts that do not exist.
</domain>

<decisions>
## Implementation Decisions

- **DOCS-PARITY-01 (done, uncommitted):** Extend the existing dependency-free `remark-vox-include.mjs` instead of replacing it with the blueprint's `unist-util-visit`/`remark-directive` rewrite. Exported `extractRegion(content, selector, srcPath)`; selectors `name`, `a,b`, `N..M`/`N-M`; markers `ANCHOR:`/`ANCHOR_END:` and `tag::x[]`/`end::x[]` behind `//` or `#`; exact-name match (blueprint's `\b` matched `foo` inside `foo-2`). `llms.txt` description reads version from root `Cargo.toml` (AGENTS.md version SSOT, not `CITATION.cff`).
- **DOCS-PARITY-02 first:** `starlight-links-validator@0.27.0` peers `astro >=7.2.10`, `@astrojs/starlight >=0.42.0`; `starlight-github-alerts@0.5.0` peers starlight `>=0.42.0`. Repo pins starlight `^0.38.3`, astro `^6.0.1`. No plugin lands before the upgrade.
- **Banners stay ledger-driven:** `Banner.astro` already renders drifting/broken/historical from `contracts/reports/docs-freshness/ledger.v1.json`; `page-status.mjs` covers research/roadmap/deprecated/legacy. `docs-reality-audit/inventory.v1.json` has `claims` with no `verification_status`, `findings.v1.json` is empty. Banner accuracy is `docs-freshness` Phase 32's job — do not add a second status source.
- **No new contracts:** `contracts/documentation/stability-matrix.v1.json` does not exist; tables that should be data use the `*.generated.md` + `ssot-drift` pattern.
- **`.mdx` is already first-class** (Phase 323); this phase neither forces `.md` purity nor converts more pages.
- **YAGNI cuts:** `remark-vox-attributes` `{token}` substitution, `::include{}` partials, `// !hide`, auto Expressive Code titles, `examples/golden/anti/` (directory absent), `starlight-markdown-blocks` custom asides.

### Claude's Discretion
- Upgrade sequencing (Astro then Starlight vs together) and whether Vite alias shims for `@astrojs/starlight/components` are still needed after 0.42.
</decisions>

<code_context>
## Existing Code Insights

- `docs-astro/astro.config.mjs` remark chain: `remarkVoxInclude`, `remarkDocLinks` (no `remarkStripTitleH1`/`voxDiagram`, contrary to the blueprint).
- Overridden components: `SiteTitle`, `Sidebar`, `TableOfContents`, `PageSidebar`, `Banner` — each must be re-checked against 0.42 component props.
- Baseline build (2026-11-05): 906 pages, 245 s, exit 0; logs two `remark-doc-links` errors that do not fail the build (DOCS-PARITY-04 targets).
- Unit tests: `node --test docs-astro/tests/unit/`; Playwright specs `reader-layout`, `trust-signals`, `smoke`, `baseline`.
</code_context>

<deferred>
## Deferred Ideas

- xref empty-label title resolution in `remark-doc-links.mjs` — revisit once the links validator lands and there is author demand.
</deferred>
