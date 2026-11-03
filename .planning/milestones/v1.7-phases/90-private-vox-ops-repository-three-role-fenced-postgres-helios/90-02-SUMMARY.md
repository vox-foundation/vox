---
phase: 90-private-vox-ops-repository-three-role-fenced-postgres-helios
plan: 02
subsystem: ingest
tags: [otlp, ingest, taxonomy, postgres, audit, vox-ops]
requires: ["90-01"]
provides:
  - "TaxonomyValidator in vox-ops-ingest enforcing collection-taxonomy.v1.json, 256 KiB cap, 200 record cap, and 64-hex SaltedHashHex digests"
  - "Axum POST /v1/logs collector with subtle::ConstantTimeEq bearer auth and zero IP/header logging"
  - "PostgreSQL shape, CHECK constraint, and 7-column privilege vector auditor in vox-ops-db::audit"
affects: [vox-ops-ingest, vox-ops-db]
tech-stack:
  added: []
  patterns: ["Pre-insert taxonomy validation + ConstantTimeEq bearer verification + zero IP/header logging"]
key-files:
  created:
    - /Users/brbrainerd/dev/vox-ops/crates/vox-ops-ingest/Cargo.toml
    - /Users/brbrainerd/dev/vox-ops/crates/vox-ops-ingest/src/validate.rs
    - /Users/brbrainerd/dev/vox-ops/crates/vox-ops-ingest/src/lib.rs
    - /Users/brbrainerd/dev/vox-ops/crates/vox-ops-db/src/audit.rs
  modified: []
key-decisions:
  - "POST /v1/logs never extracts ConnectInfo, X-Forwarded-For, or User-Agent headers, ensuring client IP addresses never enter logs or database rows."
  - "Every attribute is validated against collection-taxonomy.v1.json field types (enum, int, bool, hash) before calling IngestDbPool::insert_batch."
patterns-established:
  - "Dual-layer validation: Rust TaxonomyValidator at HTTP ingress + PostgreSQL CHECK constraints at storage boundary."
requirements-completed: [PGFENCE-04, TYPING-03, DBAUDIT-03]
duration: 15min
completed: 2026-10-19
---

# Phase 90 Plan 02: Fenced OTLP Collector (`vox-ops-ingest`) & PostgreSQL Shape Auditor Summary

**Implemented `vox-ops-ingest` (`POST /v1/logs`) with strict `collection-taxonomy.v1.json` validation, constant-time bearer auth, and zero IP/header logging, plus `vox-ops-db::audit` verifying PostgreSQL `CHECK` constraints and the `t|f|f|t|f|f|t` privilege vector.**

## Accomplishments
- Implemented `TaxonomyValidator` in `crates/vox-ops-ingest/src/validate.rs` enforcing `MAX_BATCH_BYTES = 262_144`, `MAX_BATCH_RECORDS = 200`, the 26-variant `event_name` allowlist, enum allowlists, and 64-char hex hashes.
- Implemented `ingest_router` (`POST /v1/logs`) in `crates/vox-ops-ingest/src/lib.rs` with `subtle::ConstantTimeEq` bearer verification and write-only insertion via `IngestDbPool`.
- Implemented `PostgresFenceAuditReport` and `audit_postgres_fenced_shapes` in `crates/vox-ops-db/src/audit.rs`.

## Verification
- `cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-db -p vox-ops-ingest` — passed.
