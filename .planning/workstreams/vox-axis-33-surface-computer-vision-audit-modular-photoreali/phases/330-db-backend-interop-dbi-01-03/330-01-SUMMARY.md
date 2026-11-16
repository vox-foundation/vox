---
phase: 330-db-backend-interop-dbi-01-03
plan: 01
subsystem: database
tags: [sqlite, introspect, openapi, schema-owner, vox-sql]

requires: []
provides:
  - SQL-to-Vox type inverse and IntrospectedSchema::to_vox_source
  - vox db introspect --emit json|vox (json default unchanged)
  - Vox.toml [db] schema_owner gate on vox db migrate
  - How-to for generating a Python client from the emitted OpenAPI 3.1 file
affects: [330-db-backend-interop, vox-db-migrate, python-clients]

actuals:
  tokens: 8705
  tasks: 4
  commits: 5

tech-stack:
  added: []
  patterns:
    - "SQL→Vox scalars are derived from sql_for_scalar, the same table vox db verify uses"
    - "External schema ownership refuses migrate apply after the plan is printed"

key-files:
  created:
    - docs/src/how-to/python-client-from-vox-openapi.md
  modified:
    - crates/vox-sql/src/type_map.rs
    - crates/vox-sql/src/schema_model.rs
    - crates/vox-cli/src/commands/db/introspect.rs
    - crates/vox-cli/src/commands/db/verify.rs
    - crates/vox-cli/src/commands/db_cli/core_subcommands.rs
    - crates/vox-cli/src/commands/db_cli/mod.rs
    - crates/vox-cli/src/commands/db/local.rs
    - crates/vox-package-types/src/manifest.rs
    - docs/src/reference/cli.md

key-decisions:
  - "Unmapped SQL types render as str with a // vox: unmapped sql type comment; primary keys are never Option"
  - "Rendered tables are table(extern, source, pk) so verify's live name is the SQL table name"
  - "OpenAPI for Python clients is dist/openapi.json or vox emit openapi, not vox build --target server"
  - "schema_owner is checked after the plan is printed and before either apply path"

patterns-established:
  - "Pattern: sql_type_to_vox is the inverse of vox_type_to_sql in the same file"
  - "Pattern: [db] schema_owner = external blocks migrate apply and still allows --dry-run"

requirements-completed: [DBI-01, DBI-02, DBI-03]

coverage:
  - id: D1
    description: "Introspected sqlite tables render as Vox table decls that parse and pass vox db verify's comparison"
    requirement: DBI-01
    verification:
      - kind: unit
        ref: "crates/vox-cli/src/commands/db/introspect.rs#introspected_sqlite_schema_round_trips_through_verify"
        status: pass
      - kind: unit
        ref: "cargo test -p vox-sql (exit 0; sql_to_vox and to_vox_source included)"
        status: pass
    human_judgment: false
  - id: D2
    description: "vox db introspect --emit defaults to json and can print Vox source; default JSON bytes are unchanged"
    requirement: DBI-01
    verification:
      - kind: unit
        ref: "crates/vox-cli/src/commands/db_cli/mod.rs#introspect_emit_defaults_to_json_and_accepts_vox"
        status: pass
      - kind: unit
        ref: "crates/vox-cli/src/commands/db_cli/mod.rs#introspect_help_shows_emit_flag"
        status: pass
      - kind: unit
        ref: "crates/vox-cli/src/commands/db/introspect.rs#emit_json_output_is_unchanged"
        status: pass
    human_judgment: false
  - id: D3
    description: "[db] schema_owner = external refuses non-dry-run migrate and still allows a dry-run; absent [db] stays vox"
    requirement: DBI-02
    verification:
      - kind: unit
        ref: "crates/vox-package-types/src/manifest.rs#db_section_parses_external_schema_owner"
        status: pass
      - kind: unit
        ref: "crates/vox-cli/src/commands/db/local.rs#external_schema_owner_refuses_non_dry_run_migrate"
        status: pass
      - kind: other
        ref: "cargo test -p vox-package-types (37 passed, exit 0)"
        status: pass
    human_judgment: false
  - id: D4
    description: "How-to for openapi-python-client against the real OpenAPI emit path"
    requirement: DBI-03
    verification:
      - kind: other
        ref: "cargo run -p vox-doc-pipeline -- --lint-only --paths how-to/python-client-from-vox-openapi.md (exit 0)"
        status: pass
    human_judgment: false

duration: 60min
completed: 2026-11-03
status: complete
plan_head_before: cc80864ec7a0ee313116c21cb82821300bb048df
plan_head_after: 8e3d3d41f373a982f571026116f8f16af2692053
---

# Phase 330 Plan 01: DB & Backend Interop Summary

**SQLite schemas render as Vox `table` declarations that pass `vox db verify`, migrate refuses an external schema owner, and a how-to points Python clients at the real OpenAPI file.**

## Performance

- **Duration:** 60 min
- **Started:** 2026-11-03T11:37:35Z
- **Completed:** 2026-11-03T12:37:01Z
- **Tasks:** 4
- **Files modified:** 10

## Accomplishments

- `sql_type_to_vox` is the inverse of `vox_type_to_sql`, and `IntrospectedSchema::to_vox_source` emits `table(extern, source, pk)` declarations.
- A sqlite fixture round-trips through the Vox parser and `compare_tables` (the same comparison `vox db verify` uses) with no findings.
- `vox db introspect --emit json|vox` defaults to JSON. Pretty and compact JSON stay byte-identical to the previous `println!` output.
- `[db] schema_owner = "external"` makes `vox db migrate` print the plan and refuse to apply it. Absent `[db]` stays `vox`.
- `docs/src/how-to/python-client-from-vox-openapi.md` tells Python callers to use `dist/openapi.json` or `vox emit openapi`.

## Task Commits

Each task was committed atomically:

1. **Task 1: SQL→Vox type map + renderer** - `1de9e9536` (feat) and `8d33657c8` (feat)
2. **Task 2: `--emit vox` flag** - `24dcc2d13` (feat)
3. **Task 3: schema_owner** - `803275ee3` (feat)
4. **Task 4: how-to** - `8e3d3d41f` (docs)

**Plan metadata:** pending this summary commit

## Files Created/Modified

- `crates/vox-sql/src/type_map.rs` - `sql_type_to_vox` inverse of the verify mapping
- `crates/vox-sql/src/schema_model.rs` - `IntrospectedSchema::to_vox_source`
- `crates/vox-cli/src/commands/db/verify.rs` - `compare_tables` is `pub(crate)` so the round-trip uses it
- `crates/vox-cli/src/commands/db/introspect.rs` - `--emit` rendering and the sqlite round-trip
- `crates/vox-cli/src/commands/db_cli/core_subcommands.rs` - `--emit json|vox` (default `json`)
- `crates/vox-cli/src/commands/db_cli/mod.rs` - dispatch plus clap tests for the flag
- `crates/vox-cli/src/commands/db/local.rs` - `check_schema_owner` before both migrate apply paths
- `crates/vox-package-types/src/manifest.rs` - `[db] schema_owner`
- `docs/src/reference/cli.md` - documents `--emit` and the external-owner refusal
- `docs/src/how-to/python-client-from-vox-openapi.md` - Python client from the OpenAPI emit

## Decisions Made

- Nullable non-key columns become `Option[T]`. Primary keys stay the scalar type even when SQLite reports them nullable. Unmapped SQL types become `str` plus a `// vox: unmapped sql type` comment.
- The round-trip creates `dbi01_users` and `dbi01_blog_posts` and compares only those tables. `DbConfig::Memory` already contains the migrated Vox schema, including `users`.
- `vox build --target server` emits Rust only and does not write `openapi.json`. Fullstack and `--target client` write `dist/openapi.json` when the module has user types or endpoints. `vox emit openapi` writes `openapi.json` or `--out`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Round-trip cannot use a bare `users` table on `DbConfig::Memory`**
- **Found during:** Task 1
- **Issue:** In-memory libSQL already has the migrated app schema, so `CREATE TABLE users` fails and a full introspect includes tables the renderer is not claiming to verify.
- **Fix:** Create `dbi01_*` tables (same shape as the plan fixture) and retain only those rows before render and `compare_tables`.
- **Files modified:** `crates/vox-cli/src/commands/db/introspect.rs`
- **Verification:** `introspected_sqlite_schema_round_trips_through_verify` passed
- **Committed in:** `8d33657c8`

**2. [Rule 2 - Missing Critical] Expose the verify comparison to the round-trip test**
- **Found during:** Task 1
- **Issue:** `compare_tables` was private, so the CLI test could not run the same declared-vs-live check without copying it.
- **Fix:** `pub(crate)` on `compare_tables` and `VerifyFinding`.
- **Files modified:** `crates/vox-cli/src/commands/db/verify.rs`
- **Verification:** the round-trip asserts `findings.is_empty()`
- **Committed in:** `1de9e9536`

**3. [Rule 3 - Blocking] How-to command in the context does not emit OpenAPI**
- **Found during:** Task 4
- **Issue:** `vox build --target server` skips TypeScript. OpenAPI is written by the TS emitter and by `vox emit openapi`.
- **Fix:** The how-to documents `dist/openapi.json` and `vox emit openapi --out`.
- **Files modified:** `docs/src/how-to/python-client-from-vox-openapi.md`
- **Verification:** doc lint exit 0
- **Committed in:** `8e3d3d41f`

---

**Total deviations:** 3 auto-fixed (2 blocking, 1 missing critical)
**Impact on plan:** The round-trip still uses the real verify comparison. The how-to matches the emit path that exists. No crate-edge exception.

A parallel session on this checkout also committed the renderer (`1de9e9536`) and the schema-owner gate (`803275ee3`). Those commits are the task 1 implementation and task 3 implementation; this executor verified them and did not recommit the same diff.

`docs/src/reference/cli-command-surface.generated.md` was not hand-edited. If `--emit` drifts that generated page, `ssot-autoregen` regenerates it. The hand page `docs/src/reference/cli.md` was updated.

## Issues Encountered

- `cargo test -p vox-cli` without `--lib` launches every integration binary. One of those runs was killed by the output cap (exit 137) after the library round-trip had already passed. Later runs used `--lib`.
- `DbCli` is a clap subcommand, not a `Parser`. The emit-flag tests parse `VoxCliRoot`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `vox db introspect --emit vox` and the schema-owner gate are in the CLI.
- Python clients follow the how-to. No Python package was added.
- STATE.md, ROADMAP.md, and REQUIREMENTS.md were left for the orchestrator.

## Self-Check: PASSED

- Files listed under key-files exist on disk.
- `1de9e9536`, `8d33657c8`, `24dcc2d13`, `803275ee3`, and `8e3d3d41f` are ancestors of HEAD.

---
*Phase: 330-db-backend-interop-dbi-01-03*
*Completed: 2026-11-03*
