# Phase 90: Private `vox-ops` Repository & Three-Role Fenced Postgres on Live `helios-postgres` - Context

**Gathered:** 2026-10-19
**Status:** Ready for execution

<domain>
## Phase Boundary

Bootstrap the private `~/dev/vox-ops` (`brbrainerd/vox-ops`) multi-crate Rust workspace and provision the three-role fenced PostgreSQL 17 database (`vox_ops`) on live `helios-postgres` (`q40ck888ggkkcsosgcwgc0oo` on `178.156.212.19`) (`PGFENCE-01..05`, `TYPING-03`, `HELIOS-01`, `DBAUDIT-03`):

1. **Multi-Crate `vox-ops` Workspace (`PGFENCE-01`):**
   - Create `/Users/brbrainerd/dev/vox-ops` with workspace crates:
     - `vox-ops-db` — Strongly-typed `sqlx::FromRow` models and fenced connection pools (`ingest_pool` vs `ops_pool`)
     - `vox-ops-ingest` — Taxonomy-validated OTLP `/v1/logs` HTTP collector (`vox_ingest_app`)
     - `vox-ops-syndication` — Social media & community broadcast outbox worker (`syndication.*`)
     - `vox-ops-ludus` — Multi-user competitive Ludus & zero-cheating `BLAKE3`/`Ed25519` leaderboard (`ludus_community.*`)
     - `vox-ops-intelligence` — $k \ge 20$ community intelligence aggregator & Ed25519 bundle signer (`intelligence_public.v_*`, `community.intelligence_bundles`)
     - `vox-ops-server` — Unified Axum binary mounting `/healthz`, `/v1/logs`, `/v1/intelligence/*`, and `/v1/ludus/leaderboard*`
   - Sync `contracts/telemetry/collection-taxonomy.v1.json` from `/Users/brbrainerd/dev/vox`.

2. **Three-Role Fenced PostgreSQL 17 Schema (`PGFENCE-02`, `PGFENCE-03`, `TYPING-03`):**
   - Author `sql/001_vox_ops_fenced_schema.sql`:
     - `REVOKE CONNECT, TEMPORARY ON DATABASE vox_ops FROM PUBLIC;`
     - Schemas: `telemetry_ingest`, `intelligence_public`, `syndication`, `ludus_community` (including `ludus_community.verified_contributions` with `UNIQUE (upstream_provider, upstream_event_id)` and 64-hex `row_chain_hash_hex`), and `community`.
     - Fenced roles: `vox_ops_owner` (`NOLOGIN`, schema/table/view owner), `vox_ingest_app` (`LOGIN`, `INSERT`-only on `telemetry_ingest.*`, zero `SELECT`/`UPDATE`/`DELETE`), and `vox_ops_app` (`LOGIN`, `SELECT`-only on `intelligence_public.v_*` + RW on `syndication.*`, `ludus_community.*`, `community.*`, zero access to `telemetry_ingest.*`).
     - PostgreSQL `CHECK` constraints enforcing taxonomy bucket enums (`duration_bucket`, `token_bucket`, `count_bucket`, `size_bucket`, `exit_class`, `op_type`, `file_kind`, `call_kind`, `mode`, `trigger_source`, `surface`, `subsystem`, `error_class`, `gpu_vendor`, `vram_bucket`, `tok_per_sec_bucket`, `domain_category`, `defect_class`) and 64-char lowercase hex digests (`client_id`, `row_chain_hash_hex`).
     - All 8 `SECURITY DEFINER` (`WITH (security_barrier = true)`) views in `intelligence_public.*` enforcing `HAVING count(DISTINCT client_id) >= 20` and `count(*) >= 50` with explicit `REVOKE INSERT, UPDATE, DELETE, TRUNCATE` (`D-36`).

3. **Live `helios-postgres` SCRAM Provisioning & Shape/Privilege Audit (`PGFENCE-04`, `PGFENCE-05`, `HELIOS-01`, `DBAUDIT-03`):**
   - Provision `vox_ops` on live `helios-postgres` (`q40ck888ggkkcsosgcwgc0oo`) using `deploy/helios/scram-verifier.py` and `deploy/helios/vox-ops-db-setup` (`SET log_min_error_statement = panic;`).
   - Implement `vox-ops-ingest` (`POST /v1/logs`) with `TaxonomyValidator`, `<= 256 KiB` / `<= 200` record batch cap, `ConstantTimeEq` bearer auth (`VOX_OPS_INGEST_TOKEN`), and zero IP/header logging.
   - Verify the 7-column pipe-delimited privilege vector (`t|f|f|t|f|f|t`) and PostgreSQL `CHECK` constraint enforcement on live `helios-postgres`.
</domain>

<decisions>
## Implementation Decisions

- **D-90-01 (`D16`, `D24`, `D26`):** `vox_ops` shares the existing `helios-postgres` (`postgres:17-alpine`, Coolify UUID `q40ck888ggkkcsosgcwgc0oo`) instance via strict database/schema/role fencing (`vox_ops_owner`, `vox_ingest_app`, `vox_ops_app`), provisioned with `scram-verifier.py` so plaintext passwords never touch `pg_stat_activity` or server logs.
- **D-90-02 (`D25`, `D31`):** `sql/001_vox_ops_fenced_schema.sql` includes both the taxonomy-checked `telemetry_ingest.*` tables + 8 `intelligence_public.v_*` views AND the `ludus_community.verified_contributions` append-only `BLAKE3` ledger table so Phases 91 and 92 build directly on the provisioned schema.
</decisions>

<canonical_refs>
## Canonical References

- `docs/src/architecture/centralized-postgres-telemetry-vox-ops-ssot-2026.md` — Sections 4, 5, 7, 9, 11 (Complete SQL DDL, SCRAM provisioning, role privilege vector, and `ludus_community.verified_contributions` schema)
- `contracts/telemetry/collection-taxonomy.v1.json` — Telemetry taxonomy SSOT
</canonical_refs>
