---
title: "Coding Agent Instructions"
description: "Instructions and heuristics generated from recent codebase discoveries for coding agents operating on Vox."
category: "contributor"
status: "current"
last_updated: 2026-04-10
training_eligible: true

schema_type: "TechArticle"
---

# Coding Agent Instructions

This guide provides specific heuristics and rules for AI coding agents operating within the Vox ecosystem. It synthesizes recent codebase integrity work into canonical policies to prevent regressions.

## Stale Documentation Risk

1. **Check SSOT Inventories First**: When a user asks you to implement a new feature, verify whether similar features are documented as retired or deprecated. Cross-reference `AGENTS.md` and `docs/src/architecture/legacy-retirement-roadmap.md`.
2. **Beware of Pointers to Deleted Code**: Older documentation may refer to crates or systems that have been renamed or archived (e.g. `vox-dei` being repurposed from orchestrator to a small HITL crate).
3. **Do Not Hallucinate Features**: If a surface is not declared in `architecture-index.md` or `AGENTS.md`, do not assume it exists. Do not write `import`s for non-existent internal crates.
4. **Use Search Proactively**: Always rely on `grep_search` and exact file reads (`view_file`) before modifying large modules.

## God Object Defactor Checklist

1. **Size Limits**: Prevent any module or strut from becoming a "God Object". Files over 500 lines or structs with >12 methods must be broken down into specific domains.
2. **Skeleton Code is Forbidden**: Leaving skeleton implementations (`todo!()`, `unimplemented!()`, or `pass`) will break CI workflows. A file must either be structurally complete or explicitly marked as `stub/todo` via `TOESTUB`.
3. **Component Consolidation**: Respect the split-compiler consolidation. For instance, `vox-lexer`, `vox-parser`, etc., have all been merged into `vox-compiler`. Do not create or request these old architectures.

## Enforcement

Your operations are checked locally by `AGENTS.md` boundaries. When in doubt, prefer decomposition and explicitness over shell cleverness. Ensure that any output avoids the "Retired Surfaces" constraints listed in the core agent prompts.
