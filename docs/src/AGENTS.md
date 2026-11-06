---
title: "AGENTS.md"
description: "Documentation for AGENTS.md."
category: "Architecture SSOTs"
status: "current"
training_eligible: true
training_rationale: "Project architecture context."
---
# Documentation Rules (docs/src/ scope)

## Code block policy
- Fenced blocks tagged `vox` are compiled as doctests: write inline blocks that compile, and do not add new `{{#include}}` directives. Tag `// vox:skip` plus a reason only when the snippet is a genuine out-of-file excerpt or intentionally non-compiling; other language fences are not run through the Vox parser
- All code blocks must specify a language identifier

## Frontmatter requirements (all new pages)
- `title`, `description`, `category`, `status` are required, plus the KB facets (`topic`, `domain`, `authorship`, and a `kb:summary` block) in `documentation-governance.md`. Never hand-author `last_updated`: the lint rejects it, and the date comes from Git history
- Use `status: research` for evidence docs; `status: roadmap` for unshipped plans
- Do NOT label a page SSOT unless it is the sole B-canon in `contracts/documentation/canonical-map.v1.yaml`

## Research storage
- Research findings → `docs/src/architecture/`
- Naming: `*-research-2026.md` or `*-findings-2026.md`
- After writing, set frontmatter (`title`, `description`, `category`, `status`).

## Archive prohibition
- Do NOT read or modify files in `docs/src/archive/` for new work
