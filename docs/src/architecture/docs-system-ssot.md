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
| `architecture/subsystems/*.generated.md` & polyglot `docs/wiki/*.md` | `vox ci wiki-sync` (`--polyglot`, `--heal`) | `docs/src/architecture/layers.toml` + `Cargo.toml` (or polyglot source/manifests) |
| `.well-known/llms.txt` generated block | `vox ci capability-index` | capability contracts |
| `.cursorignore`, `.aiignore`, `.aiexclude` | `vox ci sync-ignore-files` | `.voxignore` |

**Regeneration loop.** `vox ci ssot-drift` (`run_stages` in
`crates/vox-cli/src/commands/ci/run_body_helpers/ssot_drift.rs`, called from `docs.rs`) runs
every verifier. Locally it rewrites a drifted artifact (auto-heal is on when `CI` is unset or
`VOX_AUTO_HEAL` is set); in CI it only fails. On same-repo pull requests the `ssot-autoregen`
job in `.github/workflows/ci.yml` runs `scripts/ssot-regen.vox`, commits `chore(ssot):
auto-regenerate …`, and the gate re-verifies. The `ssot_regen_parity` stage verifies that
`scripts/ssot-regen.vox` matches the stage table. Do not open "regenerate after merge"
commits by hand.

## 4. Gates: what catches which drift

Legend: **fix** rewrites the drift automatically; **report** fails or warns and a human fixes it.

| Gate | Detects | Where it runs | Mode |
|---|---|---|---|
| <!-- gate: doc-lint --> `vox-doc-pipeline --lint-only` | Missing/invalid frontmatter, category vocabulary, forbidden `last_updated`, KB facets and `kb:summary`, README↔index drift, Mermaid structure | pre-push fast (changed docs), pre-push complete (all), docs-quality.yml | report |
| <!-- gate: doctest-md --> Doctests (`vox-doc-pipeline`, `vox ci doctest-md --strict`) | ` ```vox ` fences that do not compile (`// vox:skip` opts out) | pre-push fast, pre-push complete, docs-quality.yml | report |
| <!-- gate: facet-backlog --> KB facet backlog ratchet | Backlog growth in `kb-facet-backlog.v1.yaml` | pre-push fast, pre-push complete | report (shrink-only) |
| <!-- gate: mermaid-ratchet --> Legacy-Mermaid ratchet (`vox-doc-pipeline --mermaid-lint`) | Legacy Mermaid baseline growth across `docs/src` | pre-push fast, pre-push complete, docs-quality.yml | report (shrink-only) |
| <!-- gate: ssot-drift --> `vox ci ssot-drift` | Any generated artifact out of date (~40 stages, `ssot_drift_stages()`); reports every failing stage in one run; `ssot_regen_parity` fails when `scripts/ssot-regen.vox` or the lefthook retry diverge from the table | pre-push fast, pre-push complete, ci.yml | fix locally / report in CI |
| <!-- gate: ssot-autoregen --> `ssot-autoregen` job | Same drift on a PR | ci.yml (same-repo) | **fix** (bot commit) |
| <!-- gate: check-links --> `vox ci check-links` | Broken internal Markdown links; a missing `docs/src` or target is an error; expired, unneeded or unused allowlist entries fail | pre-push fast, pre-push complete, docs-quality.yml | report |
| <!-- gate: starlight-links --> `starlight-links-validator` | Broken internal links and hashes in the built site | docs-quality.yml, docs-deploy.yml | report (fatal) |
| <!-- gate: external-links --> Lychee link checker | Broken links, including external; offline on built site in PRs, external weekly triage into `external-links.v1.json` + `.lycheeignore` proposals | docs-quality.yml (offline), docs-drift-refresh.yml (weekly) | report (non-blocking draft PR) |
| <!-- gate: check-drift --> `vox audit docs --check-drift` | New freshness deficits on reader-facing `status: current` pages | docs-quality.yml | report |
| <!-- gate: retired-symbol-check --> Retired-symbol check | Prose using symbols in `contracts/documentation/retired-symbols.v1.yaml`; `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `copilot-instructions.md` and `.cursor/rules` are mandatory inputs | pre-push complete, nightly.yml | report |
| <!-- gate: canonical-map-verify --> `canonical-map-verify` | A second canonical doc for a registered domain | pre-push complete, docs-quality.yml | report |
| <!-- gate: sync-ignore-files --> `sync-ignore-files --verify` | Derived ignore files drifted from `.voxignore` | ssot-drift, ci.yml | report (pre-push fallback regenerates) |
| <!-- gate: playwright-docs --> Build-output Playwright specs (`docs-astro/tests/build-output/`) | Alerts, archive exclusion, dates, honesty, links, `llms.txt`, repo mounts | docs-quality.yml | report |
| <!-- gate: frontmatter-governance-parity --> `frontmatter_governance_parity.rs` | Governance page and lint rules disagree | cargo test -p vox-doc-pipeline | report |
| <!-- gate: overlay-parity --> Overlay parity (`overlay_parity` ssot-drift stage) | An overlay line contradicting a claim in `contracts/documentation/overlay-claims.v1.yaml`; a claim whose `AGENTS.md` anchor vanished; a missing overlay file | ssot-drift, pre-push fast, ci.yml | report |
| <!-- gate: docs-cli-claims --> CLI-claims check (`docs_cli_claims` ssot-drift stage) | A prescriptive `vox` command or flag in agent files or reader-facing `status: current` pages that the clap tree (union the registry for feature-gated groups) does not define; a shipped command described as planned | ssot-drift, pre-push fast, ci.yml | report |
| <!-- gate: docs-orphan-pages --> Orphan-page check (`docs_orphan_pages` ssot-drift stage) | A docs page that no other docs page or root guide links to (index, generated, archive pages exempt; reader-facing gated at zero, whole-tree reported) | ssot-drift, pre-push fast, ci.yml | report |
| <!-- gate: docs-drift-refresh --> Weekly docs drift refresh & link triage (`docs-drift-refresh.yml` + `scripts/lychee-triage.vox`) | Idempotent refresh of freshness ledger (`ledger.v1.json`), reality findings/metrics (`findings.v1.json`, `metrics.v1.json`), and sorted external-link triage (`external-links.v1.json` + 403/429/5xx `.lycheeignore` proposals) on `bot/docs-drift-refresh` | docs-drift-refresh.yml (weekly) | non-blocking draft PR (never auto-merged) |
| <!-- gate: docs-system-parity --> Docs-system SSOT parity (`docs_system_parity` ssot-drift stage) | §4 gate rows, `Where it runs` targets/workflows, `GATE_PROBES` code anchors, and closed-gap status in `docs-system-ssot.md` | ssot-drift, pre-push fast, ci.yml | report |

The pre-push hook (`lefthook.yml` → `vox ci pre-push`) retries once after regenerating fmt,
line endings, ignore files, GUI/capability/plugin syncs, and crate API docs (checked against
`ssot_drift_stages()` by `ssot_regen_parity`). Git hooks never build and never run generators
that need a fresh `vox`.

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

These track Phase 343 (*Docs System SSOT and Automated Drift Control*) status.

### Open gaps

- <!-- gap: prose-behavior-drift --> **Prose behavior drift and manual governance cadence.** Semantic claims in narrative prose now receive Tier-2 four-gate verified proposal assistance via `kb_heal_proposals` (`openwiki_heal` + `vox_search::promotion`), while final application/demotion decisions and governance review cadences remain operator-reviewed.

### Closed in Phase 343

- <!-- gap: overlay-contradiction --> **Closed (Phase 343):** Cross-tool overlay contradictions against `AGENTS.md` are checked by `overlay_parity` (`ssot-drift`).
- <!-- gap: aspirational-cli-claim --> **Closed (Phase 343):** Nonexistent or mislabelled CLI commands/flags in docs and overlays are checked by `docs_cli_claims` (`ssot-drift`).
- <!-- gap: silent-noop-guard --> **Closed (Phase 343):** Missing-input no-ops in docs/link guards were removed and mutation-tested.
- <!-- gap: orphan-page --> **Closed (Phase 343):** Unreachable reader-facing docs pages are blocked by `docs_orphan_pages` (`ssot-drift`).
- <!-- gap: dormant-freshness-ledger --> **Closed (Phase 343):** Weekly freshness ledger, reality findings/metrics, and docs self-heal reporting now run idempotently in `.github/workflows/docs-drift-refresh.yml` (draft PR on `bot/docs-drift-refresh`); `docs-self-heal.yml` is strictly read-only.
- <!-- gap: blocking-external-links --> **Closed (Phase 343):** Blocking `link_checker.yml` is retired; external links run non-blockingly in `docs-drift-refresh.yml` with `scripts/lychee-triage.vox`.

## 7. Changing the system

- **New generated page:** add the generator as a `vox ci` subcommand, register it as a stage in
  `ssot_drift_stages()` (`ssot_drift.rs`) and in `scripts/ssot-regen.vox`, name the output
  `*.generated.md`, and add it to `.gitattributes` `linguist-generated`.
- **New gate:** wire it into the pre-push tier and workflow, add a row to §4 with a
  `<!-- gate: <id> -->` marker and canonical `Where it runs` target, register its `Probe` in
  `crates/vox-cli/src/commands/ci/docs_system_parity.rs` (`GATE_PROBES`), and add a row in
  [`detector-coverage-ledger.md`](../contributors/detector-coverage-ledger.md).
- **New docs-about-docs page:** don't. Extend this page (mechanism) or governance (policy).
