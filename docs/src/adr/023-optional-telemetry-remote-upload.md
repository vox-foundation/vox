---
title: "ADR 023: Optional telemetry remote upload"
description: "Opt-in local spool and explicit upload; no default transmission; Clavis-backed secrets."
category: "Architecture Decisions (ADRs)"
training_eligible: true

schema_type: "TechArticle"
---

# ADR 023: Optional telemetry remote upload

## Status

Accepted — implementation ships as **`vox telemetry`** with a local file spool and explicit **`upload`** (see [`telemetry-remote-sink-spec`](../archive/research-2026-q1/telemetry-remote-sink-spec.md)).

## Context

Vox records many **operator-controlled** diagnostics and research metrics locally (Codex / `research_metrics`, completion audits, benchmark hooks). Some deployments may want a **separate**, **explicit** path to copy aggregated JSON to an operator-run HTTPS ingest. That path must never be default-on, must not bypass Clavis for credentials, and must respect data residency and legal review outside this ADR.

## Decision

1. **No default remote upload.** The product does not phone home. Transmission requires an explicit CLI invocation (`vox telemetry upload`) and configured ingest URL.
2. **Local spool first.** Pending payloads live as one JSON file per event under a configurable directory (default under the current working tree’s `.vox/telemetry-upload-queue/pending/`, overridable via `VOX_TELEMETRY_SPOOL_DIR`). Operators inspect queued redacted OTLP envelopes locally via **`vox telemetry preview [--format json|table]`** and enqueue with **`vox telemetry enqueue`** or out-of-band file drops consistent with the spool layout.
3. **Secrets via vox-secrets only.** Ingest URL and bearer token are [`SecretId::VoxTelemetryUploadUrl`](../../../crates/vox-secrets/src/lib.rs) and [`SecretId::VoxTelemetryUploadToken`](../../../crates/vox-secrets/src/lib.rs) (`VOX_TELEMETRY_UPLOAD_URL`, `VOX_TELEMETRY_UPLOAD_TOKEN`). CLI code uses `vox_secrets::resolve_secret`; do not add parallel `std::env::var` reads for those values.
4. **Normative wire behavior** (rate limits, signing roadmap, headers) lives in [telemetry-remote-sink-spec](../archive/research-2026-q1/telemetry-remote-sink-spec.md), not in this ADR.
5. **Legal / security sign-off** for any *organization-wide* or *end-user* upload policy is recorded in that organization’s process; this ADR defines the **technical** guardrails (opt-in, explicit command, vox-secrets, delete-after-ack on success).

## Consequences

- CLI surface: `vox telemetry doctor|consent|status|export|preview|enqueue|upload` (catalog + command-registry generated from `contracts/operations/catalog.v1.yaml`).
- `vox telemetry preview` renders the exact redacted OTLP/HTTP `ExportLogsServiceRequest` (`resourceLogs`) batch offline without network calls or spool deletion, and `vox telemetry upload` batches up to `200` pending records (`<= 256 KiB`) per `POST /v1/logs` request to the centralized ingest service residing in the private `vox-ops` repository backed by PostgreSQL (`telemetry_ingest.*`) rather than `server/telemetry/`.
- Documentation: [Centralized Postgres Telemetry & Vox-Ops SSOT](../architecture/centralized-postgres-telemetry-vox-ops-ssot-2026.md) + env-var rows in [env-vars](../reference/env-vars.md).
- Future HMAC or mTLS layers extend the sink spec and vox-secrets `SecretId` list without changing the “explicit upload” invariant.

## See also

- [Telemetry trust SSOT](../architecture/telemetry-trust-ssot.md)
- [Telemetry implementation backlog 2026](../archive/research-2026-q1/telemetry-implementation-backlog-2026.md) — Phase 7
- [Environment variables (SSOT)](../reference/env-vars.md)

