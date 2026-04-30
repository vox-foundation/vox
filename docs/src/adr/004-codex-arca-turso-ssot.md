---
title: "ADR 004: Codex over Arca over Turso"
description: "Official documentation for ADR 004: Codex over Arca over Turso for the Vox language. Detailed technical reference, architecture guides, a"
category: "reference"
last_updated: "2026-03-24"
training_eligible: true

schema_type: "TechArticle"
---

# ADR 004: Codex over Arca over Turso

> [!NOTE]
> Historical note: the `TURSO_*` env var names in this ADR are superseded by `VOX_DB_URL` / `VOX_DB_TOKEN`. ADR text is preserved for context.

## Status

Accepted — greenfield release baseline.

## Context

Vox persisted data through `vox-db` (`VoxDb` / **Codex**), with related crates (`vox-pm`, etc.) and scattered env names (`VOX_DB_*`, legacy `TURSO_*`). Documentation referred to **Arca**, **Codex**, and **VoxDb** interchangeably. The public product name for the database layer must be **Codex** (not “codecs” or other typos). **Schema DDL and store operations** live in **`crates/vox-db`** (`schema/` domains + `store/ops_*.rs`); the only supported SQL engine is **Turso** / libSQL.

## Decision

1. **Codex** — The public, application-facing data API. In Rust, `vox_db::Codex` is a type alias for `VoxDb`; new docs and APIs should say **Codex**.
2. **Arca** — Internal name for schema fragments, baseline migration, CAS tables, and SQL operations **owned by `vox-db`** (`schema/manifest.rs`, `store/`). No second physical store.
3. **Turso** — Sole database engine. No parallel PostgreSQL/SQLite product paths for the same data plane.
4. **Greenfield baseline** — Fresh releases use a forward migration chain from the current schema version; legacy shape is preserved via explicit **importers**, not an unbounded pile of historical migrations in docs.
5. **Convex-like behavior** — Implemented as Codex capabilities (change log, subscriptions, invalidation, SSE/WebSocket), not a second database.
6. **Secrets** — `VOX_DB_TOKEN` (and auth material) are **environment-only**; never committed in TOML. `VOX_DB_URL` may appear in config for convenience; token must not.

## Consequences

- **Repository tenancy** — MCP and orchestration shard filesystem paths; coordination tables use `repository_id` where applicable (e.g. `a2a_messages`). The `agent_events` table does not currently include `repository_id` on the baseline DDL. Session rows carry tenant context in `agent_sessions.task_snapshot` JSON when MCP sets `SessionConfig::repository_id` in `vox-orchestrator`.
- `VoxDb` remains the stable Rust identifier for ABI/compatibility; prefer **Codex** in user-facing text and new modules.
- Compatibility aliases **`VOX_TURSO_URL`** / **`VOX_TURSO_TOKEN`** map to the same remote resolution as `VOX_DB_URL` / `VOX_DB_TOKEN` in `vox_db::DbConfig::resolve_standalone` (after canonical env, before legacy Turso names).
- Legacy env vars `TURSO_URL` / `TURSO_AUTH_TOKEN` are **deprecated**; they remain a last-resort shim in `resolve_standalone` alongside `VOX_TURSO_*`.
- Direct `turso::` usage outside `vox-db` (and documented exceptions) is discouraged; domain code should call **`VoxDb` / `Codex` APIs** (`store/ops_*.rs`). See [direct Turso allowlist](../archive/research-2026-q1/codex-turso-allowlist.md) for the current enforcement story.

## References

- [Environment variables (SSOT)](../reference/env-vars.md) — canonical `VOX_DB_*` / Turso alias precedence
- [Codex / Arca compatibility boundaries](../archive/research-2026-q1/codex-arca-compatibility-boundaries.md) — API, env, and migration contract
- [Codex vNext schema domains](../archive/research-2026-q1/codex-vnext-schema.md)
- [Codex BaaS scaffolding](../archive/research-2026-q1/codex-baas.md)
- [Orphan surface inventory](../archive/research-2026-q1/orphan-surface-inventory.md)
- Crate: `crates/vox-db`, `crates/vox-pm`


