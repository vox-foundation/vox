# Phase 330: DB & Backend Interop - Context

**Gathered:** 2026-11-01
**Status:** Ready for planning
**Mode:** Smart discuss (decisions locked by design SSOT §8, D6)

<domain>
## Phase Boundary
Vox apps can adopt a database whose schema is owned by Drizzle/SQLAlchemy/Alembic, refuse to migrate it, and serve Python clients via the existing OpenAPI 3.1 emit. No ORM calls, no bridge dependency.
</domain>

<decisions>
## Implementation Decisions
- **D-01 (DBI-01):** `vox db introspect --emit vox` (keep default JSON output unchanged) renders `IntrospectedSchema` (`crates/vox-sql/src/schema_model.rs`) as Vox `table Name { field: Type }` declarations. The SQL→Vox type mapping is the inverse of `vox_sql::type_map::vox_type_to_sql` and lives next to it in `crates/vox-sql/src/type_map.rs` (one SSOT file, not a CLI-local table). Nullable columns → `Option[T]`. Unmappable SQL types → `str` plus a `// vox: unmapped sql type <t>` comment. Primary key / id conventions follow whatever `vox db verify` (`commands/db/verify.rs`) expects so the round-trip passes.
- **D-02 (DBI-02):** `Vox.toml` gains `[db] schema_owner = "vox" | "external"` (default `"vox"`) in `VoxManifest` (`crates/vox-package-types/src/manifest.rs`). `vox db migrate` (`commands/db/local.rs:109`) loads it; when `external`, non-dry-run refuses with an error naming the setting; `--dry-run` still prints the plan.
- **D-03 (DBI-03):** How-to `docs/src/how-to/python-client-from-vox-openapi.md` (frontmatter per AGENTS.md, `category: "How-To Guides"`): `vox build --target server` → OpenAPI JSON (locate the actual emit output path from `vox-codegen-ts/src/openapi_emit.rs` callers) → `openapi-python-client generate`. Example commands only; no `.py` files committed. Docs lint must pass.
### the agent's Discretion
Renderer function name/location (prefer `crates/vox-sql`), test placement.
</decisions>

<code_context>
- `vox db introspect`: `crates/vox-cli/src/commands/db/introspect.rs:6-19` (prints JSON).
- `vox db verify`: `commands/db/verify.rs` (declared `@table` vs live).
- Migration: `commands/db/local.rs:109-180` (`AppAutoMigrator`).
- Introspection: `crates/vox-sql/src/introspect.rs` (sqlite/postgres/mysql).
</code_context>

<specifics>
Round-trip test on in-memory/temp sqlite: create tables → introspect → render Vox → parse with vox-compiler → `verify` logic reports ok. Reuse `crates/vox-sql/tests/p3_introspect_smoke.rs` setup.
</specifics>

<deferred>
OpenAPI-import typed Vox client; Drizzle schema emit.
</deferred>
