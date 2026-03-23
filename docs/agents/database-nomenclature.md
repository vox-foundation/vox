---
title: "Database Nomenclature Guide"
last_updated: "2026-03-23"
---

# Vox Database Nomenclature — Agent SSOT Guide

> [!IMPORTANT]
> This page is the single source of truth for all database naming, access patterns, and
> persistence decisions in the Vox codebase. All agents and contributors **must** follow
> these rules without exception.

## Canonical Names

| Name | Type | Where Defined | What It Is |
|------|------|---------------|------------|
| `VoxDb` | Rust struct | `vox-db/src/lib.rs` | The facade over all Codex persistence |
| `Codex` | Type alias | `vox-db/src/lib.rs:145` | `pub type Codex = VoxDb;` — product-facing alias |
| `CodeStore` | Rust struct | `vox-pm/src/store/mod.rs` | The Turso connection wrapper; SQL runs here |
| `Arca` | concept | `vox-pm/src/schema/` | The schema migration system (historical name) |
| `VoxDb::store()` | Method | `vox-db/src/lib.rs` | Returns `&CodeStore` for lower-level access |

**Rule:** In Rust code, use `VoxDb` for type signatures. Use `Codex` in documentation and user-facing strings. Never create a new type alias.

## Connection Rules

| Rule | Allowed | Banned |
|------|---------|--------|
| Open a new connection | Inside `vox-pm/src/store/open.rs` | ❌ Any other crate |
| Open standalone SQLite | `vox-runtime/src/store.rs` (fallback only) | ❌ New standalone files |
| Access `db.store().conn` | Inside `vox-pm/src/store/ops_*.rs` | ❌ `vox-ludus`, `vox-orchestrator`, `vox-mcp`, etc. |
| Use `VoxDb::connect()` | Any consumer crate | ✅ |
| Use `CodeStore::open_memory()` | Test code only | ✅ |

> [!CAUTION]
> **Never** do `db.store().conn.execute(...)` from outside a `vox-pm/src/store/ops_*.rs` file.
> Add a `CodeStore` method in `ops_ludus.rs`, `ops_agents.rs`, etc. instead.

## Schema Versioning

All DDL lives in `vox-pm/src/schema/vN.rs` files and is registered in `manifest.rs`.

| Version | What it adds |
|---------|-------------|
| V1–V20  | Core schema (see individual files) |
| **V21** | `actor_state` table (actor KV store via Codex) |

To add a new table:
1. Create `crates/vox-pm/src/schema/vN.rs` with `pub const SCHEMA_VN: &str = "..."`.
2. Add `mod vN;` to `schema/mod.rs`.
3. Import `vN` in `schema/manifest.rs` and add a `SchemaFragment` entry to `SCHEMA_FRAGMENTS`.
4. Update the `assert_eq!(SCHEMA_FRAGMENTS.len(), N)` in `schema/mod.rs` tests.
5. Add CRUD methods in the appropriate `vox-pm/src/store/ops_*.rs` file.
6. *(Optional)* Add a convenience wrapper in `vox-db/src/` if consumers need it.

## Naming Conventions

| Suffix | Use For | Example |
|--------|---------|---------|
| `*Entry` | DB-row DTO returned by `CodeStore` methods (legacy, keep for `vox-pm` public types) | `MemoryEntry`, `SessionTurnEntry` |
| `*Record` | DB-row DTO in consumer crates | `AgentEventRecord`, `CostRecord` |
| `*Row` | Temporary flat struct inside a single function | `CorpusRow`, `SkillExecutionRow` |
| No suffix | Domain/business model | `Battle`, `Companion`, `LudusProfile` |

> [!NOTE]
> The 36 existing `*Entry` types in `vox-pm/src/store/types.rs` are not renamed — they are
> part of the stable `CodeStore` public API. New types in `ops_ludus.rs` and
> `ops_orchestrator.rs` use `*Row` for DB DTOs.

## RAM vs Database Decision Matrix

| Data | Storage | Reason |
|------|---------|--------|
| A2A in-process inbox (`MessageBus`) | **RAM** | Ephemeral; microsecond latency |
| A2A cross-node messages (`a2a_messages`) | **DB** | Cross-node delivery; audit trail |
| OpLog history (`OpLog.entries`, bounded VecDeque) | **RAM** | VecDeque capped at 1000; cleared on restart |
| OpLog audit/replay (`agent_oplog`) | **DB** | Long-term audit, model provenance |
| Agent mailboxes / `broadcast::channel` | **RAM** | Microsecond latency; ephemeral by design |
| Orchestrator task queues | **RAM** | Tasks re-submitted by client on restart |
| Actor KV state (`actor_state`) | **DB via Codex** | Survives restarts; schema V21 |
| Cost aggregator (`CostAggregator`) | **RAM** | Ephemeral hot cache; DB for audit |
| Cost audit trail (`cost_records`) | **DB** | Budget tracking across sessions |
| Gamification profiles/quests/battles | **DB** | User-facing progress must survive restarts |
| Provider usage tracking (`provider_usage`) | **DB** | Rate limit state shared across agents |
| File locks in-process (`FileLockManager`) | **RAM** | Local to node; sub-millisecond |
| Distributed locks (`distributed_locks`) | **DB** | Cross-node mutual exclusion |
| Mesh heartbeats (`mesh_heartbeats`) | **DB** | Cross-node liveness; pruned on expiry |
| Companion/profile state (`gamify_*`) | **DB** | Must survive restarts |

## Adding New Domains

1. **Schema:** Add a `vN.rs` fragment (tables with `IF NOT EXISTS`).
2. **CRUD:** Add methods on `CodeStore` in a new or existing `ops_*.rs` file.
3. **Facade:** Optionally add convenience wrappers to `VoxDb` in `vox-db/src/`.
4. **Tests:** Add a `crates/vox-pm/tests/ops_*_tests.rs` file using `CodeStore::open_memory()`.
5. **Never** redeclare a struct in a consumer crate if an equivalent exists in `vox-pm`.

## Struct Deduplication Policy

Before declaring `pub struct MyFoo { ... }` in any crate, run:

```
grep -rn "pub struct MyFoo" crates/
```

If it exists elsewhere, re-export it with `pub use` instead.

Key resolved duplicates:
- `CostRecord`: canonical in `vox-ludus/src/db.rs`; `vox-ludus/src/cost.rs` re-exports it.
- `CostSummary` in `vox-ludus/cost.rs` vs `CostSummary` in `vox-orchestrator/usage.rs`: kept separate — different domains (gamify vs provider routing).
