---
title: "Front-facing honesty audit (2026)"
description: "Calibrated audit of scrape-first marketing versus shipped code: what research agents see, which claims hold, and the Track A remediations."
category: "Architecture SSOTs"
status: "research"
training_eligible: false
---

# Front-facing honesty audit (2026)

Measured 2026-09-07 against `main`, live `voxlang.org`, and `gh` metadata. This page is the persistence of the six-track critique. It is **research**, not a product claim.

## Phase 0 decisions (locked for Track A)

| Decision | Choice |
|---|---|
| Install SSOT | **Source-first**: `cargo install --locked --path crates/vox-cli` with Rust from `rust-toolchain.toml` (1.98.1). Demote `voxup` until a *published* (non-draft) GitHub Release exists. |
| `llms.txt` authority | **Both**: hand file `docs/src/.well-known/llms.txt` (MCP `resource://vox/llms.txt`) and Starlight plugin `/llms.txt` (site). Update together. Serve `/.well-known/*` from `docs-astro/public/.well-known/`. |
| Community CTA | [Contributor Hub](../contributors/contributor-hub.md) + [GitHub Issues](https://github.com/vox-foundation/vox/issues). Do **not** link Discussions until the repo setting is enabled (USER-AUTHORIZED). |

## What a no-JS researcher sees

- GitHub: 2 stars, 0 forks, empty `homepageUrl`, no topics, no published Releases, Discussions disabled.
- Live homepage: hero is current; playground **static-demo** still taught retired `@endpoint` / `@table type` / `@mcp.tool` (hard parse errors since 2026-06-30).
- Live `/reference/stability/` 404 (undeployed). Site-root `/llms.txt` 200; `/.well-known/llms.txt` 404.
- Live `https://voxlang.org/voxup` 404 — `curl … \| sh` pipes HTML into a shell.

## Claim calibration

| Claim | Verdict |
|---|---|
| One `.vox` → schema + server + browser | **Defensible.** Drop the deploy limb from the hero. |
| “100+ MCP tools” | **Underclaim** (331 in `contracts/mcp/tool-registry.canonical.yaml`). Overclaim is **Mature**, not the count. |
| “Marching toward a production-hardened v1.0” | **Defensible** as destination language next to a pre-1.0 install line. |
| Python-free QLoRA | **Defensible** with Emergent/Preview tags. |
| Compile `.vox` into native Tauri apps | **True overclaim.** `vox-gui` is the operator console. |
| Mesh “automatically routes” | **True overclaim.** Opt-in; federation routing experimental and default-off. |
| “zero hallucination compiler” | **True overclaim.** Compiler checks syntax/types; Socrates is Preview. |
| LSP “production-grade… full cross-reference” | **True overclaim.** Completions/hover partial; no `definition_provider`. Compiler-core Mature is fine. |
| `voxup` one-liner works | **False today.** No published releases; CI drafts; `voxup` ignores drafts; live script 404s. |
| “Funded via Open Collective” | **Overstate.** Collective exists; use “community-backed.” |

## Track A remediations (landed in-repo)

Source-first install, playground current syntax, `why_vox` lockstep, stability grades, agent surfaces (`llms.txt`, `vox-docs.json`, `CITATION.cff`, system prompt), Discussions unlinked, smoke assertions added. Live `voxlang.org` stays stale until `docs-deploy.yml` runs after these files reach `main`.

## USER-AUTHORIZED GitHub settings (not done in this change)

Do not flip these from a docs PR. Operator checklist (`gh` as `brbrainerd`):

1. Set repository homepage to `https://voxlang.org` (`gh repo edit vox-foundation/vox --homepage https://voxlang.org`).
2. Add topics mirroring [CITATION.cff](../../../CITATION.cff) keywords (`programming-language`, `compiler`, `rust`, `ai-native`, `model-context-protocol`).
3. Enable Discussions **or** keep the Issues/Contributor Hub links this change uses.
4. Align [`.github/FUNDING.yml`](../../../.github/FUNDING.yml) with Open Collective vs GitHub Sponsors.
5. Cloudflare zone: confirm **Managed robots.txt** is off (or only prepends Content-Signals). Origin `docs/src/robots.txt` is copied to the site root at build; an edge override can hide the `voxlang.org` sitemap lines.

## Track B / C backlog (later PRs)

- Public `@endpoint` sweep: `expl-runtime.md`, `vox-web-stack.md`, VS Code README, design-system pillar tables.
- Retarget `frontend-surface-ownership.md` / dashboard docs to `vox-gui` (ADR-045).
- Optional noindex for `vox-marquee-explainer-2026.md` (do not robots-block `research-index`).
- Fix `crates/vox-forge` User-Agent (`github.com/vox-lang/vox` → `vox-foundation/vox`).
- Default `status: roadmap` on `docs/superpowers/plans/**` and architecture `*-plan-*.md`.
- Bump remaining 0.4 metadata if any surface was missed.

## Success criterion

A no-JS scrape of GitHub + `voxlang.org/` should say: pre-1.0 (0.6.0), build from source, compiler/CLI most mature, GUI is an operator console (Preview), current bare-keyword syntax. It should not say `@endpoint`, “compile `.vox` into native Tauri apps,” “curl voxup works,” “Funded via,” or “zero hallucination compiler.”
