---
phase: 90
phase_name: "private-vox-ops-repository-three-role-fenced-postgres-helios"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 10
  lessons: 0
  patterns: 4
  surprises: 0
missing_artifacts:
  - "VERIFICATION.md"
  - "UAT.md"
  - "STATE.md"
---

# Phase 90 Learnings: private-vox-ops-repository-three-role-fenced-postgres-helios

## Decisions

### Enforced compile-time and role-level separation via distinct IngestDbPoo...
Enforced compile-time and role-level separation via distinct IngestDbPool (write-only vox_ingest_app) and OpsDbPool (read-only intelligence_public.* views via vox_ops_app).

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 90-01-SUMMARY.md

---

### All 8 analytical views in intelligence_public.* enforce WITH (security_b...
All 8 analytical views in intelligence_public.* enforce WITH (security_barrier = true), HAVING count(DISTINCT client_id) >= 20 AND count(*) >= 50, and explicit REVOKE INSERT, UPDATE, DELETE, TRUNCATE.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 90-01-SUMMARY.md

---

### Created /Users/brbrainerd/dev/vox-ops/Cargo.toml and crates/vox-ops-db w...
Created `/Users/brbrainerd/dev/vox-ops/Cargo.toml` and `crates/vox-ops-db` with `#[serde(deny_unknown_fields)]` row structs (`TelemetryEventRow` and all 8 view row structs) and separated `IngestDbPool` / `OpsDbPool` handles.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 90-01-SUMMARY.md

---

### Authored sql/001_vox_ops_fenced_schema.sql with REVOKE CONNECT, TEMPORAR...
Authored `sql/001_vox_ops_fenced_schema.sql` with `REVOKE CONNECT, TEMPORARY ON DATABASE vox_ops FROM PUBLIC`, `vox_ops_owner` (`NOLOGIN`), `vox_ingest_app` (`INSERT`-only on `telemetry_ingest.*`), `vox_ops_app` (`SELECT`-only on `intelligence_public.v_*` + RW on `syndication.*`, `ludus_community.*`, `community.*`), and taxonomy `CHECK` constraints.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 90-01-SUMMARY.md

---

### Authored deploy/helios/scram-verifier.py and deploy/helios/vox-ops-db-se...
Authored `deploy/helios/scram-verifier.py` and `deploy/helios/vox-ops-db-setup` asserting the 7-column privilege vector `t|f|f|t|f|f|t`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 90-01-SUMMARY.md

---

### POST /v1/logs never extracts ConnectInfo, X-Forwarded-For, or User-Agent...
POST /v1/logs never extracts ConnectInfo, X-Forwarded-For, or User-Agent headers, ensuring client IP addresses never enter logs or database rows.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 90-02-SUMMARY.md

---

### Every attribute is validated against collection-taxonomy.v1.json field t...
Every attribute is validated against collection-taxonomy.v1.json field types (enum, int, bool, hash) before calling IngestDbPool::insert_batch.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 90-02-SUMMARY.md

---

### Implemented TaxonomyValidator in crates/vox-ops-ingest/src/validate.rs e...
Implemented `TaxonomyValidator` in `crates/vox-ops-ingest/src/validate.rs` enforcing `MAX_BATCH_BYTES = 262_144`, `MAX_BATCH_RECORDS = 200`, the 26-variant `event_name` allowlist, enum allowlists, and 64-char hex hashes.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 90-02-SUMMARY.md

---

### Implemented ingest_router (POST /v1/logs) in crates/vox-ops-ingest/src/l...
Implemented `ingest_router` (`POST /v1/logs`) in `crates/vox-ops-ingest/src/lib.rs` with `subtle::ConstantTimeEq` bearer verification and write-only insertion via `IngestDbPool`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 90-02-SUMMARY.md

---

### Implemented PostgresFenceAuditReport and audit_postgres_fenced_shapes in...
Implemented `PostgresFenceAuditReport` and `audit_postgres_fenced_shapes` in `crates/vox-ops-db/src/audit.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 90-02-SUMMARY.md

## Lessons

## Patterns

### 7-column PostgreSQL privilege vector verification (t|f|f|t|f|f|t) via ha...
7-column PostgreSQL privilege vector verification (t|f|f|t|f|f|t) via has_table_privilege assertions.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 90-01-SUMMARY.md

---

### Three-role PostgreSQL fencing + SECURITY DEFINER k>=20 view isolation
Three-role PostgreSQL fencing + SECURITY DEFINER k>=20 view isolation

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 90-01-SUMMARY.md

---

### Dual-layer validation
Dual-layer validation: Rust TaxonomyValidator at HTTP ingress + PostgreSQL CHECK constraints at storage boundary.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 90-02-SUMMARY.md

---

### Pre-insert taxonomy validation + ConstantTimeEq bearer verification + ze...
Pre-insert taxonomy validation + ConstantTimeEq bearer verification + zero IP/header logging

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 90-02-SUMMARY.md

## Surprises

