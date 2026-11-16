---
phase: 323-high-value-starlight-mdx-conversions-and-ingest-parity
plan: 02
subsystem: docs-astro-mdx
tags: [mdx, starlight, tutorials, remark-doc-links]
requires: ["323-01"]
provides:
  - ".md <-> .mdx extension fallback in docs-astro/src/plugins/remark-doc-links.mjs"
  - "Starlight <CardGrid> and <Card> integration on docs/src/index.mdx"
  - "Interactive Starlight MDX full-stack walkthrough tutorial (docs/src/tutorials/tut-fullstack-walkthrough.mdx)"
affects:
  - docs-astro
  - docs/src/index.mdx
  - docs/src/tutorials/tut-fullstack-walkthrough.mdx
  - docs/src/tutorials/tut-getting-started.md
  - docs/src/reference/ref-syntax.md
key-files:
  created:
    - docs/src/tutorials/tut-fullstack-walkthrough.mdx
  modified:
    - docs-astro/src/plugins/remark-doc-links.mjs
    - docs/src/index.mdx
    - docs/src/tutorials/tut-getting-started.md
    - docs/src/reference/ref-syntax.md
requirements-completed:
  - MDX-CONV-01
  - MDX-CONV-02
  - MDX-CONV-03
completed: 2026-11-01
---

# Phase 323 Plan 02: Starlight MDX Landing Upgrade, Link Resolution, and Interactive Full-Stack Tutorial Summary

## Accomplishments
- **`MDX-CONV-01` (`docs/src/index.mdx`):** Replaced raw inline-styled HTML feature cards with native Starlight `<CardGrid>` and `<Card>` components and updated beginner walkthrough code snippets to current bare-keyword Vox syntax while preserving `SYNC-FROM-README: why_vox` and E2E-tested onboarding copy.
- **`MDX-CONV-02` (`docs/src/tutorials/tut-fullstack-walkthrough.mdx`):** Authored a full-stack interactive Starlight MDX tutorial using `<Steps>`, `<Tabs>` / `<TabItem>` (comparing `.vox` source with generated Turso SQL, Rust Axum, and React TSX), `<CardGrid>`, `<Aside>`, `<VoxPlayground />`, and `{/* kb:summary */}` with valid KB frontmatter facets and compilable `vox` doctests.
- **`MDX-CONV-03` (`remark-doc-links.mjs` & Cross-Links):** Added transparent `.md` ↔ `.mdx` link resolution fallback in `docs-astro/src/plugins/remark-doc-links.mjs` and cross-linked the interactive MDX walkthrough from `tut-getting-started.md` and `ref-syntax.md`.
