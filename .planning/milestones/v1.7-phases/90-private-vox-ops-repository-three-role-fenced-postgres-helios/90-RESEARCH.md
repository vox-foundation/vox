# Phase 90: Private `vox-ops` Repository & Three-Role Fenced Postgres on Live `helios-postgres` - Research

**Researched:** 2026-10-19
**Domain:** PostgreSQL 17 Three-Role Security Fencing, SCRAM-SHA-256 Provisioning (`helios-postgres`), and OTLP Taxonomy Validation (`vox-ops-ingest`)
**Confidence:** HIGH (verified against live `helios` infrastructure patterns in `centralized-postgres-telemetry-vox-ops-ssot-2026.md` §§4, 5, 7)

## Summary

Phase 90 establishes the private operational backend repository (`/Users/brbrainerd/dev/vox-ops`) and provisions the fenced `vox_ops` database on the live `helios-postgres` container (`q40ck888ggkkcsosgcwgc0oo` on `178.156.212.19`):
1. **Three-Role PostgreSQL 17 Fencing (`PGFENCE-02`, `PGFENCE-03`, `PGFENCE-05`):**
   - `vox_ops_owner` (`NOLOGIN`): Owns schemas `telemetry_ingest`, `intelligence_public`, `syndication`, `ludus_community`, and `community`, all base tables, and all 8 `SECURITY DEFINER` (`WITH (security_barrier = true)`) views.
   - `vox_ingest_app` (`LOGIN`): Granted `INSERT` only on `telemetry_ingest.*`. Cannot `SELECT`, `UPDATE`, `DELETE`, or `TRUNCATE` raw telemetry rows, and has zero access to `intelligence_public`, `syndication`, `ludus_community`, or `community`.
   - `vox_ops_app` (`LOGIN`): Granted `SELECT` only on `intelligence_public.v_*` (`REVOKE INSERT, UPDATE, DELETE, TRUNCATE ON ALL TABLES IN SCHEMA intelligence_public FROM PUBLIC, vox_ops_app, vox_ingest_app`) and `SELECT, INSERT, UPDATE, DELETE` on `syndication.*`, `ludus_community.*`, and `community.*`. Cannot access `telemetry_ingest.*`.
   - Verified by the 7-column pipe-delimited privilege vector query returning `t|f|f|t|f|f|t`.
2. **Database-Level Strong Typing (`TYPING-03`, `DBAUDIT-03`):**
   - PostgreSQL `CHECK` constraints enforce that `client_id` and `row_chain_hash_hex` match `^[0-9a-f]{64}$` and every bucketed attribute column matches the exact allowed taxonomy values from `contracts/telemetry/collection-taxonomy.v1.json`.
   - `vox-ops-db` defines `#[derive(sqlx::FromRow)]` domain structs with strongly-typed enums/newtypes (`SaltedHashHex`, `DurationBucket`, `TokenBucket`, etc.).
3. **Zero-PII OTLP Collector (`PGFENCE-04`):**
   - `vox-ops-ingest` (`POST /v1/logs`) enforces a 256 KiB payload limit, max 200 records per request, constant-time bearer token verification (`VOX_OPS_INGEST_TOKEN`), and `TaxonomyValidator` validation against `collection-taxonomy.v1.json` before inserting into `telemetry_ingest.*` via `vox_ingest_app`.

## Validation Architecture

- **Plan `90-01`:** `timeout 900s cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-db` + live `deploy/helios/vox-ops-db-setup` privilege vector check (`t|f|f|t|f|f|t`).
- **Plan `90-02`:** `timeout 900s cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-ingest` verifying taxonomy validation, rejection of out-of-taxonomy values/oversized batches, and live PostgreSQL `CHECK` constraint + role fencing audit (`DBAUDIT-03`).
