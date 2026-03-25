---
title: "Database Nomenclature Guide"
last_updated: "2026-03-25"
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
| `vox-pm` | crate | `crates/vox-pm` | Package registry / artifacts — **not** the SQL schema SSOT |

**Rule:** In Rust code, use `VoxDb` in type signatures. Use **Codex** in user-facing docs. Do not introduce new aliases for the same type.

## Connection Rules

| Rule | Allowed | Banned |
|------|---------|--------|
| Open Turso via helpers | `VoxDb::connect`, `VoxDb::open`, `VoxDb::open_memory` (`vox-db`, `local` feature) | Ad-hoc `turso::Builder` in product crates without allowlist |
| Run domain SQL | Inside `vox-db/src/store/ops_*.rs` (methods on `VoxDb`) | Raw SQL scattered in consumers for tables owned by Arca |
| Read-only diagnostics | Documented exceptions (allowlist) | `connection().execute` for business writes outside `vox-db` |

> [!CAUTION]
> Prefer adding a method on `VoxDb` in `store/ops_*.rs` instead of embedding SQL in `vox-ludus`, `vox-mcp`, or `vox-orchestrator`.

## Schema Versioning

- **DDL SSOT:** `crates/vox-db/src/schema/domains/*.rs` — one fragment per domain.
- **Ordering / baseline:** `crates/vox-db/src/schema/manifest.rs` — `SCHEMA_FRAGMENTS`, `BASELINE_VERSION`, `baseline_sql()`.
- **Greenfield migrate:** `VoxDb::migrate` (`store/open.rs`) applies baseline when `schema_version < BASELINE_VERSION`.
- **Existing DB fixes:** `crates/vox-db/src/schema_cutover.rs` runs idempotent column/table alignment after migrate (e.g. `agent_events.payload_json`, `published_news.news_id`).

To add a new table:

1. Add `CREATE TABLE IF NOT EXISTS` (and indexes) to the appropriate **domain** module under `schema/domains/`.
2. If needed, extend `schema_cutover.rs` for migrations baseline `IF NOT EXISTS` cannot perform.
3. Add `VoxDb` methods in `store/ops_<domain>.rs`.
4. Add tests under `crates/vox-db/tests/`.

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
| OpLog audit/replay (`agent_oplog`) | **DB** | Long-term audit, model provenance |
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
