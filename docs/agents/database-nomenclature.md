---
title: "Database Nomenclature Guide"
last_updated: "2026-03-28"
---

# Vox Database Nomenclature — Agent SSOT Guide

> [!IMPORTANT]
> This page is the single source of truth for database naming, access patterns, and
> persistence decisions in the Vox codebase. All agents and contributors **must** follow
> these rules without exception.

## Canonical Names

| Name | Type | Where Defined | What It Is |
|------|------|---------------|------------|
| `VoxDb` | Rust struct | `vox-db/src/lib.rs` | Facade over Turso/libSQL persistence |
| `Codex` | Type alias | `vox-db/src/lib.rs` | `pub type Codex = VoxDb;` — product-facing name |
| **Arca** | concept | `vox-db/src/schema/` | Schema domains + baseline DDL + digest (internal name) |
| **Arca spec** | Rust module | `vox-db/src/schema/spec/mod.rs` | Shared DDL strings + `orchestrator_schema_digest()`; appended in `baseline_sql()` |
| `vox-pm` | crate | `crates/vox-pm` | Package registry / artifacts — **not** the SQL schema SSOT |

**Rule:** In Rust code, use `VoxDb` in type signatures. Use **Codex** in user-facing docs. Do not introduce new aliases for the same type.

## Connection Rules

| Rule | Allowed | Banned |
|------|---------|--------|
| Open Turso via helpers | `VoxDb::connect`, `VoxDb::open`, `VoxDb::open_memory` (`vox-db`, `local` feature) | Ad-hoc `turso::Builder` in product crates without allowlist |
| Run domain SQL | Inside `vox-db/src/store/ops_*.rs` (methods on `VoxDb`) | Raw SQL scattered in consumers for tables owned by Arca |
| Read-only diagnostics | Documented exceptions (allowlist) | `connection().execute` for business writes outside `vox-db` |
| CI: `.connection().query\|execute` | `vox ci sql-surface-guard` (diff-scoped; `--all` for full audit) | Extra path prefixes in [`sql-connection-api-allowlist.txt`](./sql-connection-api-allowlist.txt) while migrating |

> [!CAUTION]
> Prefer adding a method on `VoxDb` in `store/ops_*.rs` instead of embedding SQL in `vox-ludus`, `vox-mcp`, or `vox-orchestrator`.

## Schema Versioning

- **DDL SSOT:** `crates/vox-db/src/schema/domains/*.rs` — one fragment per domain, plus optional append-only DDL from [`schema/spec`](../../crates/vox-db/src/schema/spec/mod.rs) merged in `baseline_sql()`.
- **Ordering / baseline:** `crates/vox-db/src/schema/manifest.rs` — `SCHEMA_FRAGMENTS`, `BASELINE_VERSION`, `baseline_sql()`.
- **Greenfield migrate:** `VoxDb::migrate` (`store/open.rs`) applies baseline when `schema_version < BASELINE_VERSION`.
- **Existing DB fixes:** Column/table alignment was previously handled in `schema_cutover.rs` (now deleted); core alignment is now part of the idempotent baseline fragments.
- **Explicit legacy boundary:** `crates/vox-db/src/legacy/mod.rs` is the namespace for migration-era pathways (`legacy::codex`, `legacy::import_extras`, cutover wrappers). New call sites should use `legacy::*` for transitional operations rather than treating these paths as baseline peers.

### Where DDL/WAL actually runs (audit map)

| Class | Location | Notes |
|-------|-----------|-------|
| Baseline relational | `schema/domains/*.rs`, `schema/domains/sql/*.sql`, `schema/spec` strings appended in `manifest::baseline_sql` | Core SSOT for new DBs via `migrate`. |
| Orchestrator / document collections | `orchestrator_schema_digest` → `sync_schema_from_digest`; `Collection::ensure_table` | `_id`/`_data` layout; not duplicated as flat SQL tables for `provider_usage`. |
| Domain cutover | `ludus_schema_cutover.rs` | Idempotent alignment for DBs already at baseline integer (ALTER/rename/FTS); composite reporting indexes live in domain DDL, not cutover. |
| Meta bootstrap | `store/open.rs`, `facade/migrations.rs` | `schema_version` and local object store tables where applicable. |
| Legacy no-op hooks | `training_run::ensure_training_run_table`, `research::ensure_cap_table` | Baseline now creates tables; hooks remain for call-site stability. |

To add a new table:

1. Add `CREATE TABLE IF NOT EXISTS` (and indexes) to the appropriate **domain** module under `schema/domains/`.
2. Use `ludus_schema_cutover.rs` or domain-specific idempotent SQL for migrations baseline `IF NOT EXISTS` cannot perform (e.g. renames).
3. Add `VoxDb` methods in `store/ops_<domain>.rs`.
4. Add tests under `crates/vox-db/tests/`.

When cutover logic grows “`CREATE … IF NOT EXISTS` + `INSERT` seed” blocks that are **stable** for new databases, prefer promoting them into the matching domain fragment under `schema/domains/` so **greenfield `migrate`** and **digest** stay authoritative. Use cutover logic only for **ordering-sensitive** fixes on databases that already passed an older baseline (column type widens, backfills, index renames). Steps before moving DDL:

1. Prove idempotence on an empty DB (`VoxDb::migrate` only) and on a snapshot from the prior `BASELINE_VERSION`.
2. Extend `crates/vox-db/tests/migration_tests.rs` (or a domain smoke test) so both paths stay covered.
3. Bump `BASELINE_VERSION` only when the baseline digest must change; keep cutover steps until no shipped DB is expected to need them (then delete redundant cutover branches in a later release).

Legacy deletion criteria:

1. Release policy no longer supports opening pre-baseline `schema_version` chain databases.
2. `codex_legacy` JSONL import/export has no active operator dependency.
3. Ludus/gamify cutover DDL is fully represented in baseline fragments and validated on upgrade snapshots.

**Why some cutover DDL mirrors baseline:** [`VoxDb::migrate`](../../crates/vox-db/src/store/open.rs) runs the full [`baseline_sql`](../../crates/vox-db/src/schema/manifest.rs) only when `schema_version < BASELINE_VERSION`. Once a file is pinned at the baseline integer, **opening it again skips the baseline batch** and runs cutover logic only. Tables first introduced in a domain fragment during an era when many DBs were already at baseline therefore need an idempotent `CREATE TABLE IF NOT EXISTS` (or additive `ALTER`) in cutover until every replica can be assumed to have the table — not merely because greenfield installs already see the DDL in `schema/domains/`.

## Naming Conventions

| Suffix | Use For | Example |
|--------|---------|---------|
| `*Entry` | Row DTO returned by older list APIs | `MemoryEntry`, `SessionTurnEntry` |
| `*Record` | Row DTO in consumer crates | `AgentEventRecord`, `CostRecord` |
| `*Row` | Flat row mapping a specific `SELECT` | `AgentEventRow`, `A2AMessageRow`, `SessionRow` |
| No suffix | Domain model | `Battle`, `Companion`, `LudusProfile` |

## RAM vs Database Decision Matrix

| Data | Storage | Reason |
|------|---------|--------|
| A2A in-process inbox (`MessageBus`) | **RAM** | Ephemeral; microsecond latency |
| A2A cross-node messages (`a2a_messages`) | **DB** | Cross-node delivery; audit trail |
| OpLog history (`OpLog.entries`, bounded VecDeque) | **RAM** | VecDeque capped at 1000; cleared on restart |
| OpLog audit/replay (`agent_oplog`) | **DB** | Long-term audit, model provenance (main `record_operation` path + undo/redo flag sync) |
| Orchestrator MCP sessions (when DB attached) | **DB** (`agent_sessions`, `agent_session_events`) | Durable replay SSOT |
| MCP session JSONL (`SessionConfig::persist`) | **Files** | Optional non-authoritative export |
| Training / eval JSONL corpora | **Files** | Large immutable artifacts; not operational SSOT |
| Actor KV state (`actor_state`) | **DB** | Survives restarts |
| Cost audit trail (`cost_records`) | **DB** | Budget tracking across sessions |
| Gamification (`gamify_*`) | **DB** | User-facing progress must survive restarts |

## JSONL vs Codex

- **Codex:** operational state, approvals, session replay, coordination, gamification, cost records.
- **JSONL:** training exports, run telemetry, optional session file export — **not** authoritative when a DB is attached.

## Struct Deduplication Policy

Before declaring `pub struct MyFoo { ... }` in any crate, search the repo for an existing definition. Prefer `pub use` / shared crates for identical wire shapes.

Key examples:

- `CostRecord`: canonical in `vox-ludus/src/db.rs`; `vox-ludus/src/cost.rs` re-exports it.
- `CostSummary` in `vox-ludus/cost.rs` vs `vox-orchestrator/usage.rs`: different domains — keep separate.
