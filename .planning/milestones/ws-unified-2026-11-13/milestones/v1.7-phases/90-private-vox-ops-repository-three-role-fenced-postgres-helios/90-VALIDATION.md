---
phase: 90
slug: private-vox-ops-repository-three-role-fenced-postgres-helios
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-10-19
---

# Phase 90 — Validation Strategy

> Per-phase validation contract for Private `vox-ops` Repository & Three-Role Fenced Postgres on Live `helios-postgres` (`PGFENCE-01..05`, `TYPING-03`, `HELIOS-01`, `DBAUDIT-03`).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (`~/dev/vox-ops`) + `deploy/helios/vox-ops-db-setup` |
| **Config file** | `/Users/brbrainerd/dev/vox-ops/Cargo.toml` |
| **Quick run command** | `timeout 900s cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-db -p vox-ops-ingest` |
| **Full suite command** | `timeout 900s cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-db -p vox-ops-ingest` |
| **Estimated runtime** | ~30 seconds |

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|--------|
| 90-01-01 | 01 | 1 | `PGFENCE-01`, `TYPING-03` | T-90-01 | Multi-crate `vox-ops` workspace + `sqlx::FromRow` typed row models | unit | `timeout 900s cargo check --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-db` | ⬜ pending |
| 90-01-02 | 01 | 1 | `PGFENCE-02`, `PGFENCE-03`, `PGFENCE-05`, `HELIOS-01` | T-90-02 | Three-role fenced PostgreSQL 17 DDL + SCRAM-SHA-256 setup + `t\|f\|f\|t\|f\|f\|t` vector | unit / DB | `timeout 900s cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-db` | ⬜ pending |
| 90-02-01 | 02 | 2 | `PGFENCE-04`, `TYPING-03` | T-90-03 | `TaxonomyValidator` + `POST /v1/logs` zero-IP collector | unit | `timeout 900s cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-ingest` | ⬜ pending |
| 90-02-02 | 02 | 2 | `DBAUDIT-03`, `TYPING-03` | T-90-04 | PostgreSQL `CHECK` constraint & privilege vector audit | unit | `timeout 900s cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-db` | ⬜ pending |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify commands
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter
