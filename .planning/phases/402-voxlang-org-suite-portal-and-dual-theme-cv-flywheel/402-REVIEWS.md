---
phase: 402-voxlang-org-suite-portal-and-dual-theme-cv-flywheel
reviewed_at: 2026-11-10T17:18:00-07:00
status: converged
reviewers:
  - vox-doc-pipeline
  - playwright-cv-flywheel
  - gfm-link-audit
---

# Phase 401 & 402 Adversarial Review Ledger (`402-REVIEWS.md`)

## 1. Cross-Repository README & Link Audit (Phase 401)

| Finding ID | Target | Issue Discovered | Resolution & Verification |
|---|---|---|---|
| `REV-401-01` | All 4 repositories (`vox`, `vox-axis`, `voxlang.org`, `vox-ops`) | 70 broken relative links across root and sub-crate READMEs (missing `LICENSE` files in satellite repos and `crates/vox-gui`, outdated `surfaceRegistry.ts` path vs `generated/surfaceRegistry.generated.ts`, stale archive relative paths). | Copied root `LICENSE` (`Apache-2.0`) to all satellite repos and `crates/vox-gui/LICENSE`; updated `surfaceRegistry.generated.ts` link; fixed all relative paths across sub-crate READMEs; replaced duplicated `vox-axis/crates/vox-gui/README.md` and `voxlang.org/docs-astro/README.md` with canonical pointers. Verified `Total broken links: 0` across all 4 repos. |
| `REV-401-02` | `vox/README.md` & `vox-axis/README.md` | Repeated Pass@1 benchmark prose (`82.0% vs 68.0%`) across multiple sections, duplicated 4-column tradeoff explanations, and duplicated compiler/RAG/QLoRA academic footnotes in `vox-axis/README.md`. | Deduplicated `vox/README.md` (canonical Pass@1 claim in `#the-central-claim`, referenced by anchor elsewhere; 2-column `Design Choice \| Engineering Cost & Operational Constraint` table). Scoped `vox-axis/README.md` footnotes `[^1]`–`[^6]` strictly to GUI/spatial visualization/TOFU TLS/HCI citations while cross-linking `vox/README.md` for compiler/RAG/QLoRA citations. |
| `REV-401-03` | `crates/vox-*` leaf crates & `where-things-live.md` | 5 newly extracted publishable leaf crates (`vox-cli-core`, `vox-mcp-dispatch`, `vox-repository-core`, `vox-search-core`, `vox-scientia-core`) lacked `README.md` files and were missing from `docs/src/architecture/where-things-live.md`. | Authored `README.md` for all 5 crates and added the 4-repository federation boundary + leaf crate rows to `docs/src/architecture/where-things-live.md`. |
| `REV-401-04` | `vox-gui` (`fonts.css`, `index.css`) & `docs-astro` (`reader-layout.css`, `VoxPlayground.astro`) | Typography drift: `fonts.css` mapped `EB Garamond` `font-style: normal` to `EBGaramond-Italic.woff2`; `docs-astro` lacked the unified 4-token font contract (`--font-display`, `--font-serif`, `--font-sans`, `--font-mono`). | Fixed `EB Garamond` normal-style fallback in `crates/vox-gui/ui/src/styles/fonts.css`; added `--font-family-{display,sans,serif,mono}` aliases to `index.css`; wired the unified 4-token contract (`--font-display`, `--font-serif`, `--font-sans`, `--font-mono`) in `docs-astro/src/styles/reader-layout.css` and `VoxPlayground.astro`. |

---

## 2. `voxlang.org` Suite Portal & Dual-Theme CV Flywheel Audit (Phase 402)

| Finding ID | Target | Issue Discovered | Resolution & Verification |
|---|---|---|---|
| `REV-402-01` | `docs/src/index.mdx` & `docs/src/how-to/how-to-axis-workbench.md` | `voxlang.org` frontpage lacked a visual showcase of **Vox Axis** (`VRBS ROMA` 3D/2.5D Simulator, `Imperium Prefecture` Blast-Radius Inspector, Multi-Dock Chat, `AQVAEDUCTUS` 3-Tier Hydraulic Cascade, MENS Studio, and inline Light vs. Dark theme comparison) and explicit 4-repository separation of concerns. | Upgraded `docs/src/index.mdx` and `docs/src/how-to/how-to-axis-workbench.md` with animated `.webp` tours (`vox_language_suite_tour.webp`, `axis_hero_tour.webp`), architectural plates (`axis_dual_mode_architecture.webp`, `axis_five_lanes_diagram.webp`), 4-surface workbench cards, inline Light (`arcane`) vs. Dark (`slate`) theme cards, and the 4-repository federation boundary table. |
| `REV-402-02` | `vox-gui` (`index.css`, `AgentTrajectoryBraid.tsx`, `audits.ts`) | Playwright CV Flywheel (`e2e/review/cv-flywheel.spec.ts`) caught `axe-core` `color-contrast` violations on `activity` (`opacity-75`), `approvals` (`text-amber-600`, `text-rose-600`), `needs-you` (`text-amber-700`, `text-cyan-700`), `harness` (`AgentTrajectoryBraid` inline gold/olive text on marble), and `slate` dark mode (`bg-[#E8DEC9]/75` on `explorer` domain rows), plus `<ellipse>` missing from `auditIconsInPage` drawable selector on `dashboard`. | Normalized `text-amber-600..700`, `text-cyan-500..700`, `text-emerald-600`, `text-rose-500..600`, and `.opacity-70..80` in light mode (`index.css`); normalized `[class*="bg-[#E8D"]` and dark-tinted utility classes in `slate` dark mode (`index.css`); replaced inline colored text in `AgentTrajectoryBraid.tsx` with a color indicator dot + `text-text-primary font-semibold`; added `ellipse` to `auditIconsInPage`. Re-ran `cv-flywheel.spec.ts`: **7/7 passed (`0` `axeViolations` across all 32 surfaces)**. |
| `REV-402-03` | `docs-astro/src/styles/reader-layout.css` & `docs/src/index.mdx` | Playwright `suite-portal-showcase.spec.ts` caught `.vox-hero-btn-primary` in light mode at `4.10:1` (`#0284c7` vs `#ffffff`, below `4.5:1`), and inline screenshot inspection caught raw `$` math notation (`$\ge 4.5:1$`) in `index.mdx`. | Darkened light-mode `.vox-hero-btn-primary` background to `#0369a1` (`5.93:1` contrast) and replaced raw MDX `$` notation with Unicode `(≥ 4.5:1)`. Re-ran `suite-portal-showcase.spec.ts` + `reader-layout.spec.ts`: **15/15 passed**. |
