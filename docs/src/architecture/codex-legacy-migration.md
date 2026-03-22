---
title: "Codex legacy migration (importers)"
category: architecture
last_updated: 2026-03-20
---

# Codex legacy migration

Greenfield **Codex** releases do not rely on an unbounded chain of old SQL migrations as the primary story. Instead:

1. **Baseline schema** — Arca applies one manifest-defined DDL snapshot on Turso; `schema_version` stores **1** only. Legacy multi-row chains require export → fresh DB → import.
2. **Importers** — Rust modules read legacy exports or attached old DBs and write normalized rows into the new baseline.

## API surface (crate)

- `vox_db::codex_legacy` in `crates/vox-db/src/codex_legacy.rs` — `verify_legacy_store`, `LegacyImportSource`, JSONL export/import helpers.

## Shipped CLI (minimal `vox` binary)

- `vox codex verify` — connection + `schema_version` + manifest-derived reactivity tables + legacy-chain flag
- `vox codex export-legacy` — dump portable JSONL artifact (`LEGACY_EXPORT_TABLES`)
- `vox codex import-legacy` — apply importers from that JSONL where possible

See [ref-cli.md](../ref-cli.md).

## Import sources

| Source | Notes |
|--------|--------|
| Turso file / remote `CodeStore` | Full relational + CAS |
| Orchestrator `memory/` files | Map into `memories` / `session_turns` with provenance |
| Skill bundles | `publish_skill` + object store |

See [Codex vNext schema](codex-vnext-schema.md) and [ADR 004](../adr/004-codex-arca-turso-ssot.md).
