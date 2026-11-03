# Vox Documentation Site (`docs-astro`)

This package contains the Astro + Starlight site generator for [`voxlang.org`](https://voxlang.org). It renders the canonical documentation corpus in `docs/src/` along with mounted repository Markdown files and auto-generated reference catalogs.

## Architecture & Source Layout

- **Content Root**: `../docs/src/` — All authored tutorials, how-to guides, reference pages, architecture SSOTs, and ADRs live in `docs/src/`.
- **Sidebar & Navigation**: `src/utils/sidebar.mjs` — Dynamically builds the Starlight sidebar from frontmatter (`title`, `category`, `sort_order`, `status`) and `contracts/documentation/docs-sidebar-section-order.v1.json`.
- **Trust & Page Status**: `src/utils/page-status.mjs` — Computes reader trust banners, `noindex` rules, and `llms.txt` / sitemap inclusion from page `status` and `contracts/reports/docs-freshness/ledger.v1.json`.
- **Interactive Chrome**: `src/components/` — Custom Starlight overrides (`PageFrame.astro`, `TwoColumnContent.astro`, `VoxPlayground.astro`, `MermaidInit.astro`).

## Commands

Run from `docs-astro/` (or via `pnpm --dir docs-astro <cmd>` from the repository root):

| Command | Action |
| :--- | :--- |
| `pnpm install` | Install dependencies |
| `pnpm dev` | Start local dev server at `http://localhost:4321` |
| `pnpm build` | Build production static site to `./dist/` |
| `pnpm test:unit` | Run unit tests (`tests/unit/*.test.mjs`) |
| `pnpm test:build` | Run build-output verification tests against `./dist/` |

## Documentation Governance

Before adding or editing pages under `docs/src/`, consult `docs/src/contributors/documentation-governance.md` and validate changes with:

```bash
cargo run -p vox-doc-pipeline -- --lint-only --paths <relative-path-under-docs-src>
```
