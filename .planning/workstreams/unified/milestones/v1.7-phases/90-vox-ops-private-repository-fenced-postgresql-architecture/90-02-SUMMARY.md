---
phase: 90-vox-ops-private-repository-fenced-postgresql-architecture
plan: 02
subsystem: ingest
tags: [axum, otlp, subtle, taxonomy-validator, vox-ops-ingest, vox-ops-server]
provides:
  - "crates/vox-ops-ingest (POST /v1/logs, subtle::ConstantTimeEq, 256 KiB / 200-record limits, 26-variant taxonomy validator)"
  - "crates/vox-ops-server (/healthz + fenced pool composition + Dockerfile + Coolify compose)"
key-files:
  created:
    - /Users/brbrainerd/dev/vox-ops/crates/vox-ops-ingest/src/lib.rs
    - /Users/brbrainerd/dev/vox-ops/crates/vox-ops-ingest/src/validate.rs
    - /Users/brbrainerd/dev/vox-ops/crates/vox-ops-server/src/lib.rs
    - /Users/brbrainerd/dev/vox-ops/crates/vox-ops-server/src/main.rs
    - /Users/brbrainerd/dev/vox-ops/Dockerfile
    - /Users/brbrainerd/dev/vox-ops/deploy/helios/docker-compose.coolify.yml
completed: 2026-10-19
---

# Phase 90 Plan 02: Fenced OTLP/HTTP Ingest Service & `vox-ops-server` Summary

## Accomplishments
- Implemented `vox-ops-ingest` (`POST /v1/logs`) with constant-time bearer token comparison (`subtle::ConstantTimeEq`), zero IP/header logging, `MAX_BATCH_BYTES = 262_144` (`256 KiB`), `MAX_BATCH_RECORDS = 200`, and strict validation against `contracts/telemetry/collection-taxonomy.v1.json`.
- Implemented `vox-ops-server` (`GET /healthz`, `/v1/logs`, `/v1/syndication/*`, `/v1/ludus/leaderboard`, `/v1/intelligence/*`) and verified all 9 unit/integration tests across the 6-crate `vox-ops` workspace pass (`cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml`).
