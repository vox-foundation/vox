# Agent Support Directory (docs/agents/)

This directory contains automation- and agent-oriented support files. Most
JSON and YAML files here are generated or machine-maintained.

## Key files
- `ai-ide-feature-matrix-2026.json` — machine-readable IDE feature comparison
- `doc-inventory.json` — comprehensive inventory of all doc files
- `governance.md` — TOESTUB, sprawl, god-object, and quality policy (authoritative)
- `orchestrator.md` — orchestrator behavior reference (authoritative)

## Do not edit by hand
- `doc-inventory.json` is generated; run `vox ci check-docs-ssot` to regenerate
- `ai-ide-feature-matrix-2026.json` is manually curated but not agent-auto-maintained

## Cross-tool ignore rule
See root `.voxignore` for AI context exclusions. `docs/agents/` is NOT excluded
from agent context — it is intentionally surfaced.
