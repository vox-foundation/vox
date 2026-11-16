---
phase: 89-telemetry-surface-audit-remediation-daemon-gui-sinks-public
plan: 02
subsystem: telemetry
tags: [telemetry, otlp, batching, preview, vox-ops, governance]
requires: ["89-01"]
provides:
  - "Batched OTLP/HTTP ExportLogsServiceRequest merging (<= 200 records, <= 256 KiB) in vox-telemetry-otlp and vox-cli"
  - "Unified offline `vox telemetry preview [--spool] [--limit] [--format json|table]` CLI subcommand"
  - "Clean public/private repository boundary with legacy server/telemetry staged in ~/dev/vox-ops"
affects: [vox-telemetry-otlp, vox-cli, docs]
tech-stack:
  added: []
  patterns: ["Batched OTLP ExportLogsServiceRequest HTTP upload with post-2xx file acknowledgement"]
key-files:
  created: []
  modified:
    - Cargo.toml
    - crates/vox-telemetry-otlp/src/otlp_json.rs
    - crates/vox-telemetry-otlp/src/upload.rs
    - crates/vox-telemetry-otlp/tests/upload_gating.rs
    - crates/vox-cli/src/telemetry_spool.rs
    - crates/vox-cli/src/commands/telemetry.rs
    - docs/src/architecture/telemetry-trust-ssot.md
    - docs/src/adr/023-optional-telemetry-remote-upload.md
key-decisions:
  - "Merged pending spool files into standard OTLP/HTTP ExportLogsServiceRequest batches (up to 200 logRecords or 256 KiB per POST) and delete queued files only after receiving HTTP 2xx."
  - "Staged server/telemetry/ into /Users/brbrainerd/dev/vox-ops/legacy-clickhouse-reference/ and removed it from the public vox workspace."
patterns-established:
  - "Offline-first inspection via `vox telemetry preview --format json|table` using the exact `merge_otlp_log_batch` function used by `upload_pending`."
requirements-completed: [BATCH-01, BATCH-02, PREVIEW-01, REPO-01, REPO-02]
duration: 15min
completed: 2026-10-19
---

# Phase 89 Plan 02: OTLP Batch Upload, Offline Preview & Repository Boundary Summary

**Implemented batched OTLP/HTTP `ExportLogsServiceRequest` uploads (`<= 200` records / `<= 256 KiB`), unified `vox telemetry preview [--format json|table]`, and staged `server/telemetry/` into `~/dev/vox-ops`.**

## Accomplishments
- Added `OTLP_BATCH_MAX_RECORDS` (`200`), `OTLP_BATCH_MAX_BYTES` (`256 * 1024`), and `merge_otlp_log_batch` in `crates/vox-telemetry-otlp/src/otlp_json.rs`.
- Updated `upload_pending_otlp_with_auth` in `crates/vox-telemetry-otlp/src/upload.rs` and `upload_pending` in `crates/vox-cli/src/telemetry_spool.rs` to chunk pending spool files and delete files only after `HTTP 2xx`.
- Unified `vox telemetry preview [--spool <PATH>] [--limit <N>] [--format json|table]` in `crates/vox-cli/src/commands/telemetry.rs`.
- Staged `server/telemetry/` and `contracts/telemetry/collection-taxonomy.v1.json` into `/Users/brbrainerd/dev/vox-ops`, removed `server/telemetry/` from public `vox` and `Cargo.toml`, and updated `telemetry-trust-ssot.md` and `023-optional-telemetry-remote-upload.md`.

## Verification
- `cargo test -p vox-telemetry-otlp --all-features` — passed.
- `cargo test -p vox-cli --lib telemetry_spool` — passed.
- `vox-doc-pipeline --lint-only --paths architecture/telemetry-trust-ssot.md adr/023-optional-telemetry-remote-upload.md` — passed with 0 errors.
