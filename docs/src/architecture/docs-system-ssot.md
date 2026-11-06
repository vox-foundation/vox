---
title: "How Vox documentation works"
description: "End-to-end map of the Vox documentation system: sources, site build, generated pages, drift gates, agent-facing surfaces, and the known gaps."
category: "Architecture SSOTs"
status: "current"
topic: "doc-governance"
domain: "documentation"
authorship: "llm-assisted"
authorship_model: "claude-opus-5-5"
---

# How Vox documentation works

<!-- kb:summary -->
This page is the single map of the Vox documentation system. It explains where documentation
sources live, how `docs/src/` becomes voxlang.org, which pages are generated and by what, which
gates detect drift at each stage (pre-commit, pre-push, PR CI, nightly) and whether each gate
fixes drift or only reports it. It also lists the agent-facing instruction surfaces and the
kinds of divergence nothing catches yet. Policy (categories, frontmatter, authority tiers) stays
in Documentation governance; this page describes the machinery that enforces it.
<!-- /kb:summary -->

**Policy vs machinery.** [Documentation governance](../contributors/documentation-governance.md)
owns the rules: authority map, category and status vocabulary, frontmatter, KB facets, and review
cadence. This page owns the mechanism: what runs, when, and what it catches. If you are writing a
page, read governance. If a docs gate failed, or you are adding a generator or gate, read this.

## 1. Sources

| Source | What it is | Notes |
|---|---|---|
| `docs/src/` | All authored and generated site content (Diataxis dirs: `adr`, `architecture`, `reference`, `contributors`, `how-to`, `tutorials`, `explanation`, `api`, …) | `docs/src/archive/` is tombstoned and never built |
| `contracts/**` | Machine-readable SSOTs that generated pages are rendered from | Edit the contract, never the generated page |
| Repo Markdown (`AGENTS.md`, crate READMEs, …) | Mounted into the site under `/repo/<route>/` | `always_mount` + link-driven, see §2 |
| `docs/src/.well-known/llms.txt` | Hand-curated agent index plus a generated capability block | Block written by `vox ci capability-index` |
| `docs/superpowers/` | Specs and plans | Never mounted on the site |

## 2. Site build (`docs-astro/`)

1. **Mirror.** `docs-astro/scripts/setup-content.mjs` (run by `predev`/`prebuild`) builds
   `docs-astro/src/content/docs/` as symlinks into `docs/src/`, skipping `archive`,
   `.well-known`, and `SUMMARY.md`.
2. **Repo mounts.** `docs-astro/src/utils/repo-mounts.mjs` mounts repo Markdown that docs pages
   link to, plus the `always_mount` list in
   `contracts/documentation/site-mounted-repo-docs.v1.json`. `never_mount_prefixes` excludes
   `docs/superpowers/`, `docs/ci/`, `apps/`, and `tree-sitter-vox/`.
3. **Remark chain.** `remark-vox-include` (mdBook-style `{{#include path:anchor}}`, tag
   regions, line ranges) then `remark-doc-links` (rewrites relative `.md` links to routes).
4. **Starlight plugins.** `starlight-github-alerts` (GitHub `> [!NOTE]` alerts become asides),
   `starlight-links-validator` (broken internal links fail the build; exclusions in
   `src/utils/links-validator-exclude.mjs`), `starlight-llms-txt` (site `llms.txt`, version read
   from root `Cargo.toml`).
5. **Sidebar.** `src/utils/sidebar.mjs` builds navigation from frontmatter (`category`,
   `sort_order`, `title`, `status`), ordered by
   `contracts/documentation/docs-sidebar-section-order.v1.json`. `SUMMARY.md` is gitignored and
   never authored.
6. **Freshness banners.** `src/utils/docs-freshness.mjs` reads
   `contracts/reports/docs-freshness/ledger.v1.json`. `lastUpdated` comes from Git history
   (`git-dates.mjs`), so never hand-write `last_updated`.
7. **Deploy.** `.github/workflows/docs-deploy.yml` on push to `main`: `cargo run -p
   vox-doc-pipeline`, `pnpm build`, publish to GitHub Pages and Cloudflare Pages, then a
   Playwright smoke test against the live site. Failures open a `docs-deploy-broken` issue.

## 3. Generated pages

Never hand-edit a `*.generated.md`. Fix the generator or its input contract.

| Output | Generator | Input |
|---|---|---|
| `reference/cli-command-surface.generated.md` | `vox ci command-sync` | `contracts/cli/command-registry.yaml` |
| `reference/env-vars.generated.md` | `vox ci command-sync` | `contracts/config/env-vars.v1.yaml` |
| `reference/config-keys.generated.md` | `vox ci command-sync` | `contracts/config/registry.v1.yaml` |
| `reference/mcp-tools.generated.md` | `vox ci command-sync` | `contracts/mcp/tool-registry.canonical.yaml` |
| `reference/mens-train-defaults.generated.md` | `vox ci command-sync` | MENS training presets |
| `docs/src/index.mdx` front page text | `vox ci command-sync` | `README.md` |
| `reference/plugin-catalog.generated.md`, `distribution-bundles.generated.md` | `vox ci generate-plugin-catalog-docs` | plugin catalog |
| `api/<crate>.generated.md`, `api/index.generated.md` | `vox ci crate-api-sync` | crate sources |
| `architecture/subsystems/*.generated.md` | `vox ci wiki-sync` (`vox-doc-pipeline` openwiki) | `docs/src/architecture/layers.toml` + `Cargo.toml` |
| `.well-known/llms.txt` generated block | `vox ci capability-index` | capability contracts |
| `.cursorignore`, `.aiignore`, `.aiexclude` | `vox ci sync-ignore-files` | `.voxignore` |

**Regeneration loop.** `vox ci ssot-drift` (`run_ssot_drift` in
`crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs`) runs every verifier. Locally it
rewrites a drifted artifact (auto-heal is on when `CI` is unset or `VOX_AUTO_HEAL` is set); in CI it
only fails. On same-repo pull requests the `ssot-autoregen` job in `.github/workflows/ci.yml`
runs `scripts/ssot-regen.vox`, commits `chore(ssot): auto-regenerate …`, and the gate
re-verifies. Do not open "regenerate after merge" commits by hand.

## 4. Gates: what catches which drift

Legend: **fix** rewrites the drift automatically; **report** fails or warns and a human fixes it.

| Gate | Detects | Where it runs | Mode |
|---|---|---|---|
| `vox-doc-pipeline --lint-only` | Missing/invalid frontmatter, category vocabulary, forbidden `last_updated`, KB facets and `kb:summary`, README↔index drift, Mermaid structure | pre-push fast (changed docs), `--complete` (all), PR `docs-quality.yml` | report |
| Doctests (`vox-doc-pipeline`, `vox ci doctest-md --strict`) | ` ```vox ` fences that do not compile (`// vox:skip` opts out) | pre-push, PR | report |
| KB facet backlog ratchet | Backlog growth in `kb-facet-backlog.v1.yaml` | pre-push fast | report (shrink-only) |
| `vox ci ssot-drift` | Any generated artifact out of date (≈35 stages) | pre-push fast, PR CI | fix locally / report in CI |
| `ssot-autoregen` job | Same drift on a PR | PR CI (same-repo) | **fix** (bot commit) |
| `vox ci check-links` | Broken internal Markdown links (allowlist with expiry) | pre-push fast, PR | report |
| `starlight-links-validator` | Broken internal links and hashes in the built site | every `pnpm build` | report (fatal) |
| Lychee on `dist` / nightly `link_checker.yml` | Broken links, including external | PR (offline), nightly (external) | report (nightly opens an issue) |
| `vox audit docs --check-drift` | New freshness deficits on reader-facing `status: current` pages | PR and push `docs-quality.yml` | report |
| Retired-symbol check | Prose using symbols in `contracts/documentation/retired-symbols.v1.yaml` | pre-push fast | report |
| `canonical-map-verify` | A second canonical doc for a registered domain | pre-push fast, PR | report |
| `sync-ignore-files --verify` | Derived ignore files drifted from `.voxignore` | PR, pre-push fallback | report (pre-push fallback regenerates) |
| Build-output Playwright specs (`docs-astro/tests/build-output/`) | Alerts, archive exclusion, dates, honesty, links, `llms.txt`, repo mounts | PR `docs-quality.yml` | report |
| `frontmatter_governance_parity.rs` | Governance page and lint rules disagree | `cargo test -p vox-doc-pipeline` | report |

The pre-push hook (`lefthook.yml` → `vox ci pre-push`) retries once after regenerating fmt,
line endings, ignore files, GUI/capability/plugin syncs, and crate API docs. Git hooks never
build and never run generators that need a fresh `vox`.

## 5. Agent-facing surfaces

| Surface | Role |
|---|---|
| `AGENTS.md` | Cross-tool policy, always loaded |
| `CLAUDE.md`, `GEMINI.md`, `.github/copilot-instructions.md`, `.cursor/rules/*.mdc` | Tool overlays on top of `AGENTS.md` |
| `docs/src/AGENTS.md` | Rules scoped to the docs tree |
| `docs/src/architecture/where-things-live.md` | Concept → crate lookup; add a row in the same PR as new code |
| `docs/src/architecture/layers.toml` | Crate layer roster; input to the subsystem hubs and `vox-arch-check` |
| `docs/src/.well-known/llms.txt` and the site `llms.txt` | Machine-readable discovery index |

The instruction layering model is in
[Agent instruction architecture](../contributors/agent-instruction-architecture.md).

## 6. Known gaps (no automated detection or correction today)

These are verified as of 2026-11-06 and are the scope of roadmap Phase 343
(*Docs System SSOT and Automated Drift Control*).

- **Overlay contradictions.** No check compares `AGENTS.md` against the tool overlays.
  Example: `.cursor/rules/documentation-policy.mdc` asks for `last_updated`, which the lint
  rejects.
- **Stale "planned" claims.** `AGENTS.md` calls `vox audit retirement` and `vox audit effort`
  planned. Both already exist in `crates/vox-cli/src/commands/audit.rs`. Nothing compares
  "planned per …" prose with the code.
- **Silently dead check.** `check_docs_ssot` enforces the orphan-crate inventory only if
  `docs/src/architecture/orphan-surface-inventory.md` exists. It does not, so the guard is a no-op.
- **Orphan pages.** Nothing flags pages that no other page links to; the sidebar lists everything
  by frontmatter.
- **Prose vs CLI surface.** Prose mentioning `vox …` commands or flags is checked only against
  the retired-symbol regex list, not against `contracts/cli/command-registry.yaml`.
- **Hand-kept sync lists.** `scripts/ssot-regen.vox` must mirror `run_ssot_drift` ("keep in
  sync"), and nothing checks that it does.
- **Manual loops.** Freshness ledger refresh/attest, `.lycheeignore` triage for the nightly link
  checker, README status-table review, and governance review cadences are all manual. The
  docs-reality-audit program is dormant, and the self-heal workflow only runs its unit tests.

## 7. Changing the system

- **New generated page:** add the generator as a `vox ci` subcommand, register it as a stage in
  `run_ssot_drift` and in `scripts/ssot-regen.vox`, name the output `*.generated.md`, and add it
  to `.gitattributes` `linguist-generated`.
- **New gate:** wire it into the pre-push tier and `docs-quality.yml`, add a row to §4, and add
  a detector-coverage row in [`detector-coverage-ledger.md`](../contributors/detector-coverage-ledger.md).
- **New docs-about-docs page:** don't. Extend this page (mechanism) or governance (policy).
