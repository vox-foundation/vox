# Phase 203: Clone T3 Ecosystem Repos into `~/dev` & Build Deterministic Surface/AST Inventory Graph - Context

**Gathered:** 2026-10-24
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure/research-scaffolding phase)

<domain>
## Phase Boundary

Clone `pingdotgg/t3code`, `vibheksoni/t3router`, `vibheksoni/pi-t3chat`, `TGlide/thom-chat`, and `helioschat/app` into `~/dev`, build `scripts/t3-surface-inventory.vox` to deterministically index their file/module/route/symbol trees within strict token budgets, and publish `docs/src/architecture/t3-ecosystem-surface-map-research-2026.md` + `contracts/reports/t3-ecosystem-surface-graph.v1.json`.
</domain>

<decisions>
## Implementation Decisions

### Agent's Discretion
All implementation choices are at the agent's discretion — pure infrastructure/inventory phase. Clone shallow (`--depth 1`) into `~/dev/{t3code,t3router,pi-t3chat,thom-chat,helioschat}`, record exact commit SHAs, author `scripts/t3-surface-inventory.vox` compliant with VoxLang-First Glue Code (`vox check` + `vox run --interp`), emit `contracts/reports/t3-ecosystem-surface-graph.v1.json`, and author `docs/src/architecture/t3-ecosystem-surface-map-research-2026.md` passing `vox-doc-pipeline --lint-only`.
</decisions>

<code_context>
## Existing Code Insights

- `.vox` scripts in `scripts/` use `// vox:caps fs subprocess env` and `import json;`.
- Architecture docs in `docs/src/architecture/` require valid YAML frontmatter (`title`, `description`, `category`, `status`, `topic`, `domain`, `authorship`, `authorship_model`, `training_eligible`) and a 40–120 word `<!-- kb:summary -->` block under the H1.
</code_context>

<specifics>
## Specific Ideas

Respect token and context limits by building a deterministic, subsystem-chunked surface inventory in `contracts/reports/t3-ecosystem-surface-graph.v1.json` before deep-diving individual subsystems in Phases 204–206.
</specifics>

<deferred>
## Deferred Ideas

None.
</deferred>
