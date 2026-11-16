# docs-freshness — Workstream Brief (v1.3 Self-Maintaining Public Docs)

## Problem (user's words, paraphrased)

The documentation corpus and the public website (voxlang.org) are bloated and under-maintained; the code has
diverged massively from what the docs say. We want the public-facing docs to be the highest possible quality
and stay relevant, accurate, and current **automatically**, without constant manual updates — at minimum,
drift warnings that can be surfaced to LLMs which then guide/perform the fix. Investigate open-source
innovations and paid tools (cost vs benefit), extend what we already have, and make docs a CI/CD-gated,
front-facing artifact.

## Decisions from scoping (2026-10-08)

| Decision | Choice |
|---|---|
| Placement | New GSD workstream `docs-freshness` |
| Outcomes in scope | (1) automatic doc↔code drift detection surfaced in CI / PR / llms.txt; (2) generate more reference from code/contracts; (3) LLM agents draft doc fixes; (4) shrink/consolidate corpus; (5) reader-visible freshness badges |
| LLM autonomy | **Draft PRs for human review** — never auto-merge prose |
| Budget | Paid platforms (Mintlify, GitBook, ReadMe, Swimm, etc.) are on the table — compare honestly against extending in-house |
| Pruning | **Case by case from an audit report** — no blanket archive |

## Measured baseline (2026-10-08)

- 966 `.md` under `docs/src/`; **668 live** (excluding `archive/`).
- Live by section: architecture 388 (58%), reference 108, adr 49, how-to 38, ci 19, contributors 21, explanation 17, tutorials 7, operations 6.
- `status:` frontmatter: ~383 `current`, ~200 `research`, ~72 `roadmap`, ~26 `deprecated`, 6 `legacy`. Nothing verifies a `current` claim.
- Last 60 days: 1,113 commits touching `crates/` vs 255 touching `docs/src/`.
- Site: Astro Starlight in `docs-astro/` (sidebar from frontmatter, pagefind search), deployed by `.github/workflows/docs-deploy.yml`.

## Existing machinery to extend (do not rebuild)

- `crates/vox-doc-pipeline` — frontmatter lint (`VALID_CATEGORIES`), code-fence lint, anchors, `vox` doctests.
- `crates/vox-doc-inventory` — doc inventory, relevance scoring, hints → `docs/agents/doc-inventory.json`.
- `vox ci` gates: `doctest-md --strict`, `check-links`, `canonical-map-verify`, `command-sync` (generates `reference/cli-command-surface.generated.md`), `ssot-drift` (+ `ssot-autoregen` PR bot), `retired-symbol-check`, `command-compliance`, `sync-ignore-files`.
- `.github/workflows/docs-quality.yml` (lint, doctest, links, Starlight build, biome, markdownlint on contributors only).
- **Docs Reality Audit Program** (`docs/src/contributors/docs-reality-audit-program.md`, `contracts/documentation/docs-reality-audit.program.v1.yaml`, `contracts/reports/docs-reality-audit/{inventory,findings,metrics}.v1.json`, `vox ci docs-reality-audit verify|metrics`) — taxonomy CodeDeficit/DocDeficit/IntentionalHistorical/AmbiguousNeedsDecision + priority score. **Dormant**: zero findings, one commit (2026-05-12).
- `docs/src/architecture/front-facing-honesty-audit-2026.md` — what scrape-first surfaces may claim.
- `docs/src/.well-known/llms.txt` — curated agent index.
- Generated refs: `cli-command-surface`, `plugin-catalog`, `distribution-bundles`, `mens-train-defaults`.
- `vox graph` (graphify) — code-intelligence graph (symbol query, reachability, crate-map) — candidate for doc→code claim resolution.
- `vox audit` umbrella (findings JSONL w/ `schema_version`) — candidate home for a `vox audit docs` subcommand.
- LLM boundary: all LLM calls must go through `vox_actor_runtime::llm` (model-agnostic facade); model selection via `vox-orchestrator::models`.

## Hard constraints (repo policy)

- Automation is `.vox` scripts via `vox run` — no new `.sh`/`.py`/`.ps1`.
- GitHub-hosted runners only; jobs ≤30 min on PR, ≤180 min scheduled; every workflow needs `concurrency:`.
- No vendor LLM hostnames/SDKs in workspace code; secrets via `vox_secrets::resolve_secret`.
- `docs/src/archive/` is tombstoned — never read for planning.
- Generated docs are fixed at the generator, never hand-edited.
- New crate deps need layer compliance (`docs/src/architecture/layers.toml`) and may need user-authorized exceptions.

## Confirmed defects (quick wins)

- `docs-astro/astro.config.mjs` sets `lastUpdated: true`, but every `actions/checkout@v7` in `.github/workflows/docs-deploy.yml` is shallow (no `fetch-depth: 0`), so per-page "Last updated" dates on voxlang.org are wrong/missing. Content is also reached via a symlink (`setup-content.mjs`), which may defeat git-date lookup — verify with a local build.
- **BLOCKER — public site not deploying since 2026-05-12** (77/400 runs green; every run since 2026-09-20 red; bot issue #462 open with 57 comments). Verified 2026-10-08: build succeeds; the `Deploy to Cloudflare Pages` step fails with `Authentication error [code: 10000]` on `/accounts/***/pages/projects/vox-docs` → the Cloudflare API token secret is expired/revoked or lacks `Pages:Edit`. Fix is a human secret rotation, then a liveness alert so this can't silently persist again. All other work is invisible to readers until this is green.
- Overlapping prior designs to merge/retire rather than add a fourth: dormant Docs Reality Audit Program; `docs/superpowers/specs/2026-08-22-docs-corpus-repair*`; `docs/superpowers/specs/2026-09-14-deep-research-documentation-engine*`.
