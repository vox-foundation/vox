---
phase: 90-private-vox-ops-repository-three-role-fenced-postgres-helios
plan: 01
subsystem: database
tags: [postgres, vox-ops, fencing, scram-sha-256, k-anonymity, helios]
requires: ["89-03"]
provides:
  - "Multi-crate ~/dev/vox-ops Rust workspace (vox-ops-db, vox-ops-ingest, vox-ops-syndication, vox-ops-ludus, vox-ops-intelligence, vox-ops-server)"
  - "sql/001_vox_ops_fenced_schema.sql with three-role fencing (vox_ops_owner, vox_ingest_app, vox_ops_app), taxonomy CHECK constraints, and 8 SECURITY DEFINER k>=20 views"
  - "deploy/helios/scram-verifier.py and deploy/helios/vox-ops-db-setup verifying the 7-column privilege vector t|f|f|t|f|f|t"
affects: [vox-ops-db, sql, deploy]
tech-stack:
  added: []
  patterns: ["Three-role PostgreSQL fencing + SECURITY DEFINER k>=20 view isolation"]
key-files:
  created:
    - /Users/brbrainerd/dev/vox-ops/Cargo.toml
    - /Users/brbrainerd/dev/vox-ops/crates/vox-ops-db/Cargo.toml
    - /Users/brbrainerd/dev/vox-ops/crates/vox-ops-db/src/lib.rs
    - /Users/brbrainerd/dev/vox-ops/sql/001_vox_ops_fenced_schema.sql
    - /Users/brbrainerd/dev/vox-ops/deploy/helios/scram-verifier.py
    - /Users/brbrainerd/dev/vox-ops/deploy/helios/vox-ops-db-setup
  modified: []
key-decisions:
  - "Enforced compile-time and role-level separation via distinct IngestDbPool (write-only vox_ingest_app) and OpsDbPool (read-only intelligence_public.* views via vox_ops_app)."
  - "All 8 analytical views in intelligence_public.* enforce WITH (security_barrier = true), HAVING count(DISTINCT client_id) >= 20 AND count(*) >= 50, and explicit REVOKE INSERT, UPDATE, DELETE, TRUNCATE."
patterns-established:
  - "7-column PostgreSQL privilege vector verification (t|f|f|t|f|f|t) via has_table_privilege assertions."
requirements-completed: [PGFENCE-01, PGFENCE-02, PGFENCE-03, PGFENCE-05, TYPING-03, HELIOS-01]
duration: 15min
completed: 2026-10-19
---

# Phase 90 Plan 01: Private `vox-ops` Workspace & Three-Role Fenced PostgreSQL Schema Summary

**Bootstrapped the multi-crate `~/dev/vox-ops` workspace, authored `sql/001_vox_ops_fenced_schema.sql` with 3 fenced roles and 8 `SECURITY DEFINER` $k \ge 20$ views, and provisioned SCRAM-SHA-256 setup scripts for `helios-postgres`.**

## Accomplishments
- Created `/Users/brbrainerd/dev/vox-ops/Cargo.toml` and `crates/vox-ops-db` with `#[serde(deny_unknown_fields)]` row structs (`TelemetryEventRow` and all 8 view row structs) and separated `IngestDbPool` / `OpsDbPool` handles.
- Authored `sql/001_vox_ops_fenced_schema.sql` with `REVOKE CONNECT, TEMPORARY ON DATABASE vox_ops FROM PUBLIC`, `vox_ops_owner` (`NOLOGIN`), `vox_ingest_app` (`INSERT`-only on `telemetry_ingest.*`), `vox_ops_app` (`SELECT`-only on `intelligence_public.v_*` + RW on `syndication.*`, `ludus_community.*`, `community.*`), and taxonomy `CHECK` constraints.
- Authored `deploy/helios/scram-verifier.py` and `deploy/helios/vox-ops-db-setup` asserting the 7-column privilege vector `t|f|f|t|f|f|t`.

## Verification
- `cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-db` — passed.
