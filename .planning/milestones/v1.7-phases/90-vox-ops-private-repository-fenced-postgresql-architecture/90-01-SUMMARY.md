---
phase: 90-vox-ops-private-repository-fenced-postgresql-architecture
plan: 01
subsystem: database
tags: [postgres, scram-sha-256, security-definer, k-anonymity, vox-ops]
provides:
  - "/Users/brbrainerd/dev/vox-ops multi-crate workspace (6 crates)"
  - "sql/001_vox_ops_fenced_schema.sql (5 schemas, 3 fenced roles, 8 SECURITY DEFINER k>=20/n>=50 views)"
  - "deploy/helios/scram-verifier.py + deploy/helios/vox-ops-db-setup"
  - "Live helios-postgres (q40ck888ggkkcsosgcwgc0oo) vox_ops database with verified privilege vector t|f|f|t|f|f|t"
key-files:
  created:
    - /Users/brbrainerd/dev/vox-ops/Cargo.toml
    - /Users/brbrainerd/dev/vox-ops/sql/001_vox_ops_fenced_schema.sql
    - /Users/brbrainerd/dev/vox-ops/deploy/helios/scram-verifier.py
    - /Users/brbrainerd/dev/vox-ops/deploy/helios/vox-ops-db-setup
    - /Users/brbrainerd/dev/vox-ops/crates/vox-ops-db/src/lib.rs
    - /Users/brbrainerd/dev/vox-ops/crates/vox-ops-db/src/audit.rs
completed: 2026-10-19
---

# Phase 90 Plan 01: Private `vox-ops` Workspace & Three-Role Fenced PostgreSQL Schema Summary

## Accomplishments
- Initialized the standalone 6-crate `/Users/brbrainerd/dev/vox-ops` workspace (`vox-ops-db`, `vox-ops-ingest`, `vox-ops-syndication`, `vox-ops-ludus`, `vox-ops-intelligence`, `vox-ops-server`).
- Authored `sql/001_vox_ops_fenced_schema.sql` with 5 schemas (`telemetry_ingest`, `intelligence_public`, `syndication`, `ludus_community`, `community`), 3 fenced roles (`vox_ops_owner`, `vox_ingest_app`, `vox_ops_app`), taxonomy `CHECK` constraints, and 8 `SECURITY DEFINER` views enforcing $k \ge 20$ distinct `client_id`s and $n \ge 50$ minimum events with `REVOKE INSERT, UPDATE, DELETE, TRUNCATE`.
- Provisioned `vox_ops` on live `helios-postgres` (`q40ck888ggkkcsosgcwgc0oo` on `helios`) using RFC 5802 pre-hashed `SCRAM-SHA-256$4096:...` verifiers and verified the live 7-column privilege vector `t|f|f|t|f|f|t`.
