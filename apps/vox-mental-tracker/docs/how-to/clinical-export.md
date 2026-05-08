---
title: "Clinical and Open mHealth–aligned export (Vox Mental Tracker)"
description: "Deterministic CSV/JSON/HTML exports for clinician handoff — owned by this app, not the Vox language repo."
category: "how-to"
status: "current"
---

# Clinical export from Vox Mental Tracker

This document lives in the **app repository** (`apps/vox-mental-tracker`). Platform-generic Vox bootstrap guidance stays in the monorepo under `docs/src/how-to/external-app-bootstrap.md`.

## SSOT

1. **Versioned contracts** under `contracts/export/` — CSV column order (`csv-columns.v1.yaml`), JSON bundle shape (`json-bundle.v1.yaml`), optional Open mHealth field names at mapping time.
2. **Append-only `HealthEventLog`** — exports read the event log (and derived flags such as `is_backdated` at export time).
3. **Deterministic row serialization** — TypeScript helper `src/ts/export_contract.ts` (`buildHealthCsv`, `sortEventsStable`, `sha256Hex`) mirrors the YAML contracts; Vitest locks ordering and escaping.
4. **No cloud** — generation runs locally; share via system sheet / file export.

## Determinism

- Sort keys and rows stably (see `contracts/export/csv-columns.v1.yaml`).
- Include a **content hash** of the exported event range in HTML/PDF headers where applicable so regeneration drift is obvious.
- Map **Open mHealth** / **HL7 FHIR Observation** at export time, not on the hot path for `db` writes.

## Related

- `docs/architecture/failure-modes-research-2026.md`
- Monorepo **`docs/src/how-to/external-app-bootstrap.md`** (generic external-app pattern)
