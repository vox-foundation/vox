---
title: "Vox Memory System"
description: "Persistent, searchable long-term storage for agent knowledge with tiered primaries per concern."
category: "architecture"
status: "current"
last_updated: "2026-04-11"
training_eligible: false

schema_type: "TechArticle"
archived_date: 2026-04-18
---

# Vox Memory System

The memory system combines **Codex (VoxDB)** for structured, queryable data with **workspace files** for human-edited logs and optional exports. There is no single on-disk file for “all memory”; use the table below to pick the right tier.

## Tiered persistence (SSOT by concern)

| Concern | Primary store | Notes |
|---------|---------------|--------|
| Structured memory facts (`vox_memory_save_db`, `agent_memory` / related tables) | **Codex** ([`VoxDb`](../../../crates/vox-db/src/lib.rs)) — user-global or workspace journey per [how-to-voxdb-canonical-store](../how-to/how-to-voxdb-canonical-store.md) | Resolved like other Codex data (`VOX_DB_*`, `.vox/store.db` default for repo MCP). |
| Tool-facing flat store (`vox_memory_store` → `memory/MEMORY.md`) | **Markdown under workspace `memory/`** | Human-readable; not a substitute for relational queries. |
| Daily narrative logs (`vox_memory_log`) | **`memory/logs/YYYY-MM-DD.md`** | Append-only prose; retention is operator-managed. |
| Orchestrator MCP sessions (replay) | **Codex** when a DB handle is attached | See [database-nomenclature](../../agents/database-nomenclature.md) RAM vs DB matrix. |

For RAM vs database vs JSONL tradeoffs across the whole stack (A2A, sessions, training corpora), use **[Database nomenclature — agent SSOT](../../agents/database-nomenclature.md)**.

## Architecture (high level)

```
┌─────────────────────────────────────────────────────────────┐
│  Codex (VoxDB): structured memory, knowledge, sessions      │
│  (tier: canonical vox.db vs repo .vox/store.db — see how-to)│
└────────────────────────────┬────────────────────────────────┘
                             │
              ┌──────────────┴──────────────┐
              ▼                             ▼
    ┌──────────────────┐         ┌─────────────────┐
    │ MemoryManager    │         │ SessionManager  │
    │ (markdown logs)  │         │ (Codex events)  │
    └────────┬─────────┘         └─────────────────┘
             ▼
   memory/MEMORY.md, memory/logs/*.md
```

## MCP Tools

| Tool | Description |
|------|-------------|
| `vox_memory_store` | Persist a typed memory fact to workspace markdown (`MEMORY.md` path) |
| `vox_memory_recall` | Retrieve a fact from long-term memory by key |
| `vox_memory_search` | Unified retrieval pipeline: hybrid (BM25+vector) when available, with deterministic fallback to BM25-only and lexical substring scan |
| `vox_memory_log` | Append an entry to today's daily memory log |
| `vox_memory_list_keys` | List all section keys from MEMORY.md |
| `vox_knowledge_query` | Query the knowledge graph for related concepts |
| `vox_memory_save_db` | Persist a typed memory fact to Codex (`agent_memory` and related tables) |
| `vox_memory_recall_db` | Recall typed memory facts from Codex |

## Usage

```rust
// From Rust
use vox_db::VoxDb;

let db = VoxDb::open("path/to/db.sqlite").await?;

// Store a memory
db.store_memory("user_preference", "Use tabs for indentation").await?;

// Recall it
let val = db.recall_memory("user_preference").await?;

// Search
let results = db.search_memories("indentation").await?;
```

## Compaction

When context gets large, use `vox_compaction_status` to check token budget.
The `CompactionEngine` supports three strategies:

- **Summarize** — condense history into a summary block
- **Drop Oldest** — drop oldest entries until under budget
- **Hybrid** — summarize, then drop if still over

## Persistence (summary)

- **`vox_memory_store`** → flat text in `memory/MEMORY.md` (workspace).
- **`vox_memory_log`** → `memory/logs/YYYY-MM-DD.md`.
- **`vox_memory_save_db` / DB-backed tools** → Codex relational tables for structured queries and search.

## Storage and domain persistence

Prefer **Arca-governed** `VoxDb` operations in `crates/vox-db` for gamification (`vox-ludus`), schedules, and telemetry rather than duplicating state in unstructured logs. Markdown remains appropriate for human-curated narratives alongside Codex.


