---
title: "Telemetry Trust (SSoT)"
description: "Canonical boundaries and trust policies for Vox telemetry."
category: "Architecture SSOTs"
sort_order: 10
status: "current"
---

# Telemetry Trust Policy (SSoT)

> **Note**: Comprehensive research and historical blueprinting for the Vox telemetry architecture were finalized in Q1 2026. The full corpus of design documents is preserved in `docs/src/archive/research-2026-q1/`.

## Core Policy

1. **Zero-Surprise Opt-In & Offline-First**: `vox` works 100% offline; remote upload is disabled by default (`ConsentState::Unset`) and `VOX_TELEMETRY=off` overrides all sinks. Remote sinks must be explicitly authorized via `vox telemetry consent grant`.
2. **Pre-Spool Projection & Redaction**: Only allowlisted `enum|int|bool|hash` scalar fields from `contracts/telemetry/collection-taxonomy.v1.json` enter the local spool. No source code, file paths, secrets, or PII may be included in telemetry payloads.
3. **Transparent Local Inspection (`vox telemetry preview`)**: The primary consumer of `vox` telemetry is the local **`vox-orchestrator`** stack (CLI + `vox-gui` + `vox-orchestrator-d`). Users can inspect the exact OTLP JSON batch or tabular summary queued on disk via `vox telemetry preview [--format json|table]` before any upload.
4. **Batched OTLP/HTTP Egress**: `vox telemetry upload` batches up to `200` records (`<= 256 KiB`) per `POST /v1/logs` request in `ExportLogsServiceRequest` format using Clavis secrets (`SecretId::VoxTelemetryUploadUrl`, `SecretId::VoxTelemetryUploadToken`).
5. **Repository Boundary (`vox` vs. Private `vox-ops`)**: The public `vox` repository contains only client-side telemetry emission, redaction, spooling, and preview/upload; centralized PostgreSQL ingest (`telemetry_ingest.*`) lives in the private `vox-ops` repository (see [`centralized-postgres-telemetry-vox-ops-ssot-2026.md`](centralized-postgres-telemetry-vox-ops-ssot-2026.md)).

For implementation details and configuration of remote sinks, see:
- [`docs/src/architecture/centralized-postgres-telemetry-vox-ops-ssot-2026.md`](centralized-postgres-telemetry-vox-ops-ssot-2026.md)
- [`docs/src/adr/023-optional-telemetry-remote-upload.md`](../adr/023-optional-telemetry-remote-upload.md)
- `vox telemetry --help`
