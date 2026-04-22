---
title: "Agent Quick Reference"
description: "A tightly condensed reference for essential agent tasks, constraints, and CI rules."
category: "reference"
status: "current"
last_updated: "2026-04-16"
training_eligible: true

schema_type: "TechArticle"
---

# Agent Quick Reference

## Core CI Gates You Must Not Break
1. `vox ci line-endings`: LF only source formats limit. No CRLF allowed except for `*.ps1`.
2. `vox ci command-compliance`: Check CLI compliance updates.
3. `vox stub-check` (`TOESTUB`): Prevent submitting `todo!()`, `unimplemented!()`, `empty-bodies`, or any stubs.
4. `vox ci sync-ignore-files`: Ensures `AGENTS.md` rules and `.voxignore` exclusions correctly sync to `.cursorignore` and `.aiignore`.
5. `vox ci clavis-parity`: Requires secret references securely bind to `resolve_secret(...)`. No static env variables allowed!

## Documentation Rules Fast Track
- Do NOT read or modify files within `docs/src/archive/` or `archive/` for current work streams.
- All new documentation requires comprehensive YAML frontmatter: `title`, `description`, `category`, `status`, `last_updated`, `training_eligible`.
- Inline code blocks across `.md` files should be explicitly imported using `{{#include}}` pointing to `examples/golden/` files, OR manually prepended with `// vox:skip`.

## Secret Management One-Liner
Never read `std::env::var("SECRET")`; exclusively employ `vox_clavis::resolve_secret(...)` and declare it in `crates/vox-clavis/src/spec.rs`.

## Running Dev Environment
If `vox` is explicitly omitted from terminal `$PATH`, use the dev scripts:
- Windows: `scripts\windows\vox-dev.ps1 <commands>`
- Linux/Mac: `./scripts/vox-dev.sh <commands>`

## Retired Surfaces Quick Map

| Retired / Deprecated | Canonical Replacement (Use Instead) |
|---|---|
| `vox-dei` (orchestrating logic) | `vox-orchestrator` |
| `vox-ars` | `vox-skills` |
| `vox-gamify` | `vox-ludus` |
| `vox-lexer`, `vox-parser`, `vox-hir` | `vox-compiler` |
| `@component fn Name()` | `component Name() {}` |
| `TURSO_URL`, `VOX_TURSO_TOKEN` | `VOX_DB_URL` / `VOX_DB_TOKEN` |
| `recall()` | `recall_async()` |
| `persist_fact()` | `sync_to_db()` |

## Entry Points
- Full cross-agent definitions: [`AGENTS.md`](../../AGENTS.md)
- Governance strict rules: [`docs/agents/governance.md`](../../agents/governance.md)
- Contributor entry hub: [`docs/src/contributors/contributor-hub.md`](../contributors/contributor-hub.md)

