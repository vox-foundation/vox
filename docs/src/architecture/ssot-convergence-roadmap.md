# SSOT / DRY convergence roadmap

This document tracks the **Rev C** convergence program: contracts, VoxDb persistence ownership, MCP/CLI parity, and CI gates (`vox ci ssot-drift`).

## Authoritative artifacts (current)

- CLI surface — `contracts/cli/command-registry.yaml` + `vox ci command-compliance`
- Contracts index — `contracts/index.yaml` + `vox ci contracts-index`
- Codex HTTP + schema — `contracts/codex-api.openapi.yaml`, `crates/vox-db/src/schema/manifest.rs`, `vox ci check-codex-ssot`
- Baseline / digest policy — `contracts/db/baseline-version-policy.yaml`
- MCP tool names — `contracts/mcp/tool-registry.canonical.yaml` → `vox-mcp-registry` (Rust `TOOL_REGISTRY`)
- DeI wire types — `vox-protocol` (`DispatchRequest` / `DispatchResponse`), schema `contracts/dei/rpc-methods.schema.json`

## Evidence snapshot

Machine-readable drift notes: `contracts/reports/evidence-snapshot-rev-c.json`. SQL ownership audit (incremental): `contracts/reports/sql-write-ownership-rev-c.json`.

## Next waves

Remaining work follows the internal 292-operation checklist (persistence CRUD normalization, env registry YAML, workflow gate matrix). Prefer **extending** existing guards over parallel checkers.
