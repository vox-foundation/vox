---
phase: 89-telemetry-surface-audit-remediation-daemon-gui-sinks-public
reviewed: 2026-10-19T17:48:00-07:00
depth: standard
status: passed
findings_critical: 0
findings_warning: 1
findings_fixed: 1
---

# Phase 89 Code Review Report (`--fix`)

## Scope Reviewed
- [`crates/vox-telemetry/src/types.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-telemetry/src/types.rs)
- [`crates/vox-telemetry/src/lib.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-telemetry/src/lib.rs)
- [`crates/vox-telemetry-otlp/src/project.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-telemetry-otlp/src/project.rs)
- [`crates/vox-telemetry-otlp/src/otlp_json.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-telemetry-otlp/src/otlp_json.rs)
- [`crates/vox-telemetry-otlp/src/upload.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-telemetry-otlp/src/upload.rs)
- [`crates/vox-db/src/telemetry_sink.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-db/src/telemetry_sink.rs)
- [`crates/vox-cli/src/commands/telemetry.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-cli/src/commands/telemetry.rs)

## Findings & Auto-Fixes (`--fix`)

1. **[Warning -> FIXED] Enforce `MAX_OTLP_BATCH_BYTES` (`256 KiB`) on client OTLP batch uploader (`TELEM-03`):**
   - **Location:** [`crates/vox-telemetry-otlp/src/upload.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-telemetry-otlp/src/upload.rs)
   - **Issue:** `upload_pending_otlp_with_auth` batched up to `MAX_OTLP_BATCH_RECORDS = 100` files, but did not guard the serialized JSON batch against the `256 KiB` ceiling enforced by `vox-ops-ingest` (`POST /v1/logs`).
   - **Fix Applied:** Added `pub const MAX_OTLP_BATCH_BYTES: usize = 256 * 1024;` and explicit pre-flight validation before sending the HTTP request.
