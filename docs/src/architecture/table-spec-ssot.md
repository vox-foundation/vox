# Table metadata SSOT (Arca ↔ `@table` convergence)

This document sketches the **shared table-spec pathway** called for in the DB parity program.
It is **not** the live SSOT yet; today definitions live in two places:

| Source | Role |
|--------|------|
| **Arca** (`crates/vox-db/src/schema/domains/*.rs`) | Canonical SQL DDL, migrations, Turso runtime |
| **Vox `@table`** → HIR → `emit_table_ddl` (`crates/vox-compiler/src/codegen_rust/emit/tables.rs`) | Generated app-local DDL + typed accessors in emitted crates |

## Near-term (current)

- Pin **explicit** parity fixtures: see `crates/vox-db/tests/arca_compiler_table_parity.rs` (column signatures + `_id`/`id` mapping).
- Expand that file with additional `(HIR mirror ↔ Arca fragment)` rows whenever a surface must stay aligned.
- Normalize comparisons: strip benign `DEFAULT` clauses, compare logical nullability + SQLite affinity, not raw formatting.

## Target architecture

1. **Single logical spec** (YAML/JSON or Rust `const` module) describing:
   - logical table name (Arca snake_case + Vox PascalCase),
   - columns: logical name, storage SQL type, `NOT NULL`, primary key / auto-increment, optional FK.
2. **Generators** (or shared readers):
   - emit Arca domain SQL fragments,
   - emit compiler `HirTable` fixtures or drive `emit_table_ddl` tests,
   - optional: generate `.vox` `@table` stubs for greenfield apps.
3. **CI**: `arca_compiler_table_parity` (and cousins) iterate the spec instead of hand-duplicating DDL strings.

## Related

- `docs/agents/sql-connection-api-allowlist.txt` — consumer crates must not embed ad-hoc SQL; use `VoxDb` ops.
- `docs/src/explanation/expl-architecture.md` — compiler pipeline overview.
