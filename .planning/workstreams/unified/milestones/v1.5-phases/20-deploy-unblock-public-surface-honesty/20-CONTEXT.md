# Phase 20 — Decisions (user, 2026-10-08)

Phase: Deploy Unblock & Public-Surface Honesty. Requirements: DEPLOY-01..04, HONEST-01..03, LINKS-01..03, TUT-01.
Workstream decisions D1–D11 live in `../../REQUIREMENTS.md`; research in `20-RESEARCH.md`.

| # | Decision |
|---|---|
| P20-D1 | **Internals rule:** every page with `status: research` or `status: roadmap` (131 pages) moves to the Internals sidebar section, regardless of category. No frontmatter or category rewrites; driven by one shared status module. |
| P20-D2 | **Site search:** Internals pages stay searchable in the on-site search (pagefind), visibly labelled Internals. They are `noindex` for search engines and excluded from every llms.txt variant. |
| P20-D3 | **Liveness:** the docs site raises a `ci-liveness` alert when there has been no successful `docs-deploy.yml` run for **14 days**. |
| P20-D4 | **Failure issue:** one issue edited in place (no repeat comments), closed automatically on the next green deploy, labelled `nightly-failure`, assigned to **@brbrainerd**. |
| P20-D5 | **llms.txt:** drop the links to `front-facing-honesty-audit-2026` (research) and `mcp-tool-reference` (legacy). Fix the three 404 links (`/AGENTS.md`, `/CLAUDE.md`, `/GEMINI.md`) by rendering those files as pages (LINKS-02). |
| P20-D6 | **Docs Reality Audit page:** set `status: roadmap` with a note that phase 21 of `docs-freshness` reactivates it. The two overlapping specs and the plan get `status: deprecated` plus a "Superseded by docs-freshness" note. |
| P20-D7 | **DEPLOY-01 evidence:** a manual dispatch is not enough; criterion 1 needs a green run triggered by a push to `main` that touches `docs/src/`. Close #462 only after that, recording both root causes (build breaks May–Sep; expired Cloudflare token from 2026-09-20). |
| P20-D8 | **Archive:** stop publishing `docs/src/archive/` on the site. Old archive URLs redirect to the closest current page, or to a single "retired" notice. *(Recommended default applied — question interrupted; user may override.)* |
| P20-D9 | **Repo Markdown URL prefix:** `/repo/` (e.g. `/repo/agents-md/`), so it can't collide with the hand-written `/agents/` page. llms.txt links are updated to match. *(Recommended default applied; user may override.)* |
| P20-D10 | **Tutorial verification record:** written by the tutorial check into a generated file, which phase 22's freshness ledger later absorbs. No hand-edited `verified_against` frontmatter. *(Recommended default applied; user may override.)* |

## Defaults chosen without asking (planner may adjust)

- Pin `wrangler` as a `docs-astro` devDependency at the current latest stable (no unpinned `npx wrangler` with the Cloudflare token in scope).
- Scope the Pages-write / OIDC permissions to the GitHub Pages deploy job only.
- Fix `robots.txt` (sitemap 404), the misleading `noindex` comment in `routeData.ts`, and remove the no-op `llmsFullTxt: true` key.
- Patch `starlight-llms-txt` 0.10.0 with `pnpm patch` for llms-full exclusion (no upgrade: 0.12 needs Astro 7).
- Git-date map ignores commits in a new `.git-blame-ignore-revs` plus `ssot-autoregen` bot commits, follows renames, and is keyed by file path.
