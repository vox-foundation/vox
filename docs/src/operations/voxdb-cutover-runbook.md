---
title: "VoxDB data cutover and telemetry sidecar runbook"
description: "Operator runbook for legacy schema_version migration via export/import, deprecating the training telemetry sidecar, aligning telemetry consumers with Populi envelopes, publication/news tables, and rollback guidance."
category: "operations"
---

# VoxDB data cutover & telemetry sidecar runbook

Operator-facing sequence for converging on **canonical `vox.db`**, telemetry contracts, and deprecating the **training telemetry sidecar**.

## Stage 0 — Preconditions

- Read `docs/src/architecture/voxdb-connect-policy.md` (strict vs degraded vs sidecar).
- Ensure `vox ci ssot-drift` and `vox ci data-ssot-guards` pass on main.

## Stage 1 — Legacy `schema_version` chain (blocking)

**Symptom:** `StoreError::LegacySchemaChain` on normal `VoxDb::connect`.

1. `vox codex export-legacy backup.jsonl` (opens source without baseline migrate).
2. Point `VOX_DB_PATH` at a **new file** or delete the old DB.
3. Run any command that connects normally (e.g. `vox codex verify`) -> apply baseline.
4. `vox codex import-legacy backup.jsonl` (**replace** semantics — tables cleared then loaded).

## Stage 2 — Training telemetry sidecar (`vox_training_telemetry.db`)

**When:** Primary DB is legacy; `connect_default_with_training_fallback` attaches the sidecar.

**Deprecation criteria (check off before dropping sidecar usage):**

- Primary DB opens with baseline `schema_version` (no `LegacySchemaChain`).
- Mens runs persist training rows to **canonical** DB in smoke tests.
- No open bugs referencing missing runs in the sidecar file.

**Cutover:** After primary migration, remove reliance on sidecar paths in operator scripts; delete sidecar file only after backup.

## Stage 3 — Telemetry consumers

- Align JSONL viewers with Populi envelope (`docs/src/reference/telemetry-metric-contract.md`).
- When changing `telemetry_schema`, update `vox mens watch-telemetry` and re-run `vox ci data-ssot-guards`.

## Stage 4 — Publication / news

- `published_news.content_sha3_256` gates syndication per content revision; see `docs/architecture/news_syndication_security.md`.
- **`publication_attempts`** is canonical for attempt history; `news_publish_attempts` is legacy.

## Rollback

- Keep `export-legacy` JSONL artifacts until Stage 1 verification passes on a clone.
- Do not delete primary DB until export verified.
