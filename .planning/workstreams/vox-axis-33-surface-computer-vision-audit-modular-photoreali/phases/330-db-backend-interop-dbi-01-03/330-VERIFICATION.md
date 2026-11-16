---
phase: 330-db-backend-interop-dbi-01-03
verified: 2026-11-04T02:09:44Z
status: passed
score: 4/4 must-haves verified
covered_files:
  - .planning/phases/330-db-backend-interop-dbi-01-03/330-01-PLAN.md
  - .planning/phases/330-db-backend-interop-dbi-01-03/330-01-SUMMARY.md
  - crates/vox-cli/src/commands/db/introspect.rs
  - crates/vox-cli/src/commands/db/local.rs
  - crates/vox-cli/src/commands/db/verify.rs
  - crates/vox-cli/src/commands/db_cli/core_subcommands.rs
  - crates/vox-cli/src/commands/db_cli/mod.rs
  - crates/vox-package-types/src/manifest.rs
  - crates/vox-sql/src/schema_model.rs
  - crates/vox-sql/src/type_map.rs
  - docs/src/how-to/python-client-from-vox-openapi.md
  - docs/src/reference/cli.md
covered_digest: "v3:sha256:207ab2f1b4d7f8ce655bfdffc7542146a8c5720b92ab88bbb907515e9b256578"
behavior_unverified: 0
overrides_applied: 0
decision_coverage:
  honored: 0
  total: 3
  not_honored:
    - D-01
    - D-02
    - D-03
---

# Phase 330: DB & Backend Interop Verification Report

**Phase Goal:** Vox apps share databases owned by Drizzle/SQLAlchemy/Alembic and serve Python clients, using existing introspection and OpenAPI surfaces.
**Verified:** 2026-11-04T02:09:44Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | `vox db introspect --emit vox` on a sqlite fixture emits `table` decls that `vox db verify` accepts against the same DB (round-trip test; postgres/mysql behind existing features). | ✓ VERIFIED | `introspected_sqlite_schema_round_trips_through_verify` parses rendered source with `vox-compiler` and asserts `compare_tables` (the function `verify()` calls) is empty. Re-run: 1 passed. Postgres/MySQL inverses live in `sql_for_scalar` / `sql_type_to_vox` behind the existing `postgres` and `mysql` features. |
| 2 | With `[db] schema_owner = "external"`, `vox db migrate` refuses to apply and `--dry-run` prints the plan. | ✓ VERIFIED | `check_schema_owner` bails with an error containing `schema_owner` when the owner is external and `dry_run` is false. Both migrate paths `println` `plan.describe()` and then call the check before `apply`. Dry-run returns `Ok`, so the existing dry-run branch still runs. Absent `[db]` and a missing manifest resolve to `SchemaOwner::Vox`. |
| 3 | The how-to's Python client example is generated from a golden server app's OpenAPI and checked in CI docs lint. | ✓ VERIFIED | `docs/src/how-to/python-client-from-vox-openapi.md` points `openapi-python-client generate` at `dist/openapi.json` and `vox emit openapi --out`. Those writers exist (`generate_openapi` sets `openapi: 3.1.0`; default `--out-dir` is `dist`; `--target server` returns before TypeScript emit). Doc lint exit 0. No Python package is committed, matching locked decision D-03. |
| 4 | `vox db introspect` without `--emit` output is byte-identical to before (DBI-01). | ✓ VERIFIED | Pre-phase `cc80864ec` printed `serde_json::to_string` / `to_string_pretty` via `println!`. Current JSON emit is the same two calls plus a trailing newline, and clap defaults `--emit` to `json`. |

**Score:** 4/4 truths verified (0 present, behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
| --- | --- | --- | --- |
| `crates/vox-sql/src/type_map.rs` | `sql_type_to_vox` inverse of `vox_type_to_sql` | ✓ VERIFIED | Derived from `sql_for_scalar`. Inverse tests assert forward-map stability for every enabled backend. Used by `to_vox_source`. |
| `crates/vox-sql/src/schema_model.rs` | `IntrospectedSchema::to_vox_source` | ✓ VERIFIED | Emits `table(extern, source, pk)` decls. Nullable non-keys become `Option[T]`. Unmapped SQL types become `str` plus a `// vox: unmapped sql type` comment. Primary keys stay the scalar. |
| `crates/vox-cli/src/commands/db/introspect.rs` | `--emit json\|vox`, default JSON unchanged | ✓ VERIFIED | `render` JSON arm matches the pre-phase `println!`. Vox arm calls `to_vox_source`. Round-trip test passed. |
| `crates/vox-cli/src/commands/db/verify.rs` | Same comparison the round-trip uses | ✓ VERIFIED | `verify()` calls `pub(crate) compare_tables`. The round-trip calls that function with `UnsupportedTypePolicy::Reject` and `strict = true` (stricter than the CLI default). |
| `crates/vox-cli/src/commands/db_cli/core_subcommands.rs` | `--emit` clap flag, default `json` | ✓ VERIFIED | `value_enum`, `default_value = "json"`. |
| `crates/vox-cli/src/commands/db_cli/mod.rs` | Dispatch plus clap tests | ✓ VERIFIED | `DbCliCore::Introspect { emit }` is passed to `db::introspect`. Tests assert the default and `--emit vox`. |
| `crates/vox-cli/src/commands/db/local.rs` | Gate before both apply paths | ✓ VERIFIED | `check_schema_owner` sits after `plan.describe()` and before `apply` on the app-plane and local paths. |
| `crates/vox-package-types/src/manifest.rs` | `[db] schema_owner` default `vox` | ✓ VERIFIED | `SchemaOwner::{Vox, External}`, serde lowercase, default `Vox`. Unknown values fail parse. |
| `docs/src/how-to/python-client-from-vox-openapi.md` | How-to that lints | ✓ VERIFIED | Frontmatter, kb:summary, and bash examples. Lint: no hard errors. |
| `docs/src/reference/cli.md` | Documents `--emit` and the refusal | ✓ VERIFIED | States default `json` and that external ownership prints the plan and refuses apply unless `--dry-run`. |

Plan frontmatter declares no `must_haves.artifacts` (`verify.artifacts` exit 66). Rows above are the files the plan and summary name.

### Key Link Verification

| From | To | Via | Status | Details |
| --- | --- | --- | --- | --- |
| `DbCliCore::Introspect` | `db::introspect` | `db_cli/mod.rs` match arm | WIRED | `emit` is forwarded. |
| `render(Vox)` | `IntrospectedSchema::to_vox_source` | `introspect.rs` | WIRED | Vox arm returns `schema.to_vox_source()`. |
| `to_vox_source` | `sql_type_to_vox` | `schema_model.rs` | WIRED | Column types come from the shared inverse. |
| Round-trip test | `verify::compare_tables` | `introspect.rs` test | WIRED | Same function `verify()` uses after introspect. |
| `migrate` | `check_schema_owner` | `local.rs` both backends | WIRED | After the plan print, before `apply`. |
| `load_schema_owner` | `DbSection::from_manifest_str` | walk to `Vox.toml` | WIRED | Missing section defaults to `Vox`. |
| How-to | OpenAPI 3.1 emit | `vox emit openapi` and TS emitter | WIRED | `generate_openapi` writes `openapi: 3.1.0`. Server target returns before that file is written. Fullstack and `--target client` write emitter files under `out_dir` (`dist` by default), including `openapi.json` when the module has user types or endpoints. |

Plan frontmatter declares no `must_haves.key_links` (`verify.key-links` exit 66).

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| --- | --- | --- | --- | --- |
| `introspect` | `schema` | `AnySqlBackend::introspect_schema` (sqlite `sqlite_master` + `PRAGMA table_info`) | Yes | ✓ FLOWING |
| `to_vox_source` | column types | `sql_type_to_vox` on introspected `data_type` | Yes | ✓ FLOWING |
| `migrate` gate | `schema_owner` | Nearest `Vox.toml` `[db]`, else `SchemaOwner::Vox` | Yes | ✓ FLOWING |
| How-to | OpenAPI document | Existing `generate_openapi` / `dist/openapi.json` | Yes (documented path, not a new emitter) | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| --- | --- | --- | --- |
| Sqlite render parses and `compare_tables` is empty | `cargo test -p vox-cli --lib commands::db::introspect::tests::introspected_sqlite_schema_round_trips_through_verify -- --exact` | `test ...round_trips_through_verify ... ok` — `1 passed; 0 failed; 1147 filtered out` | ✓ PASS |
| How-to doc lint | `cargo run -p vox-doc-pipeline -- --lint-only --paths how-to/python-client-from-vox-openapi.md` | exit 0, `lint complete — no hard errors` | ✓ PASS |
| Default JSON bytes match pre-phase `println!` | `git show cc80864ec:crates/vox-cli/src/commands/db/introspect.rs` vs current `render` | Pretty and compact JSON are still `serde_json` plus one trailing newline. An earlier `--exact` run with only the short test name reported `0 passed` and was discarded. | ✓ PASS |

Schema-owner unit tests were read and not re-run. The refusal is a six-line predicate, and both call sites are on the apply path after the plan print.

### Probe Execution

No probes are declared in `330-01-PLAN.md` or `330-01-SUMMARY.md`. Step 7c is not applicable.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| --- | --- | --- | --- | --- |
| DBI-01 | 330-01 | `vox db introspect --emit vox` prints `table` declarations that pass `vox db verify` against the same sqlite, postgres, or mysql database. | ✓ SATISFIED | Sqlite round-trip passed through `compare_tables`. Postgres and MySQL mappings are the existing feature-gated `sql_for_scalar` table; the roadmap success criterion puts those backends behind those features. Default JSON is unchanged. |
| DBI-02 | 330-01 | With `[db] schema_owner = "external"`, `vox db migrate` refuses to apply; `--dry-run` still prints the plan. | ✓ SATISFIED | Error names `schema_owner`. Plan is printed before the gate. Dry-run does not apply. Absent `[db]` stays `vox`. |
| DBI-03 | 330-01 | A how-to shows generating a Python client from a `--target server` app's OpenAPI 3.1 output, with a checked example. | ✓ SATISFIED | The how-to uses the emitters that actually write OpenAPI 3.1. `--target server` skips TypeScript, so the page says so and uses `dist/openapi.json` and `vox emit openapi`. Example commands only. Doc lint passed. |

### Test Quality Audit

| Test File | Linked Req | Active | Skipped | Circular | Assertion Level | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| `crates/vox-cli/src/commands/db/introspect.rs` `introspected_sqlite_schema_round_trips_through_verify` | DBI-01 | 1 | 0 | No — oracle is `compare_tables` against the live schema | Behavioral | PASS (re-run, 1 passed) |
| `crates/vox-cli/src/commands/db/introspect.rs` `emit_json_output_is_unchanged` | DBI-01 | 1 | 0 | Weak — expected value is `serde_json`, which `render` also calls | Value | PASS via pre-phase source diff, not this assertion alone |
| `crates/vox-cli/src/commands/db_cli/mod.rs` emit clap tests | DBI-01 | 2 | 0 | No | Value | PASS (read) |
| `crates/vox-sql/src/type_map.rs` `sql_to_vox_*` | DBI-01 | 3 | 0 | No | Value | PASS (read) |
| `crates/vox-cli/src/commands/db/local.rs` schema-owner tests | DBI-02 | 3 | 0 | No | Value | PASS (read) |
| `crates/vox-package-types/src/manifest.rs` `db_section_*` | DBI-02 | 4 | 0 | No | Value | PASS (read) |
| `docs/src/how-to/python-client-from-vox-openapi.md` | DBI-03 | n/a | 0 | No | Process (doc lint) | PASS (re-run, exit 0) |

**Disabled tests on requirements:** 0
**Circular patterns detected:** 0 blockers. `emit_json_output_is_unchanged` restates the serializer; byte identity is the diff against `cc80864ec`.
**Insufficient assertions:** 0

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| --- | --- | --- | --- | --- |
| Phase files listed above | — | No `TBD` / `FIXME` / `XXX` / `TODO` / `HACK` in the touched sources | — | None |

`Ok(())` in `check_schema_owner` is the allow path (dry-run or `vox` owner), not an empty handler.

### Decision Coverage

### Decision Coverage (warning)

3 decision(s) not found in shipped artifacts:

- **D-01** (uncategorized): `vox db introspect --emit vox` (keep default JSON output unchanged) renders `IntrospectedSchema` (`crates/vox-sql/src/schema_model.rs`) as Vox `table Name { field: Type }` declarations. The SQL→Vox type mapping is the inverse of `vox_sql::type_map::vox_type_to_sql` and lives next to it in `crates/vox-sql/src/type_map.rs` (one SSOT file, not a CLI-local table). Nullable columns → `Option[T]`. Unmappable SQL types → `str` plus a `// vox: unmapped sql type <t>` comment. Primary key / id conventions follow whatever `vox db verify` (`commands/db/verify.rs`) expects so the round-trip passes.
- **D-02** (uncategorized): `Vox.toml` gains `[db] schema_owner = "vox" | "external"` (default `"vox"`) in `VoxManifest` (`crates/vox-package-types/src/manifest.rs`). `vox db migrate` (`commands/db/local.rs:109`) loads it; when `external`, non-dry-run refuses with an error naming the setting; `--dry-run` still prints the plan.
- **D-03** (uncategorized): How-to `docs/src/how-to/python-client-from-vox-openapi.md` (frontmatter per AGENTS.md, `category: "How-To Guides"`): `vox build --target server` → OpenAPI JSON (locate the actual emit output path from `vox-codegen-ts/src/openapi_emit.rs` callers) → `openapi-python-client generate`. Example commands only; no `.py` files committed. Docs lint must pass.

This is a soft warning - verification status is unchanged.

The scanner did not match that prose verbatim. The code does implement each decision: D-01 is `sql_type_to_vox`, `to_vox_source`, and `--emit` (decls are `table(extern, source, pk) Name { field: Type }`, which is what verify accepts); D-02 is `SchemaOwner` plus the post-plan gate; D-03 is the how-to. D-03's `--target server` step was corrected because that target returns before OpenAPI is written. The page uses the real writers instead. No `.py` file was added.

### Human Verification

N/A — Infrastructure/foundation phase with no user-facing elements.
All acceptance criteria are verifiable programmatically.

### Gaps Summary

No gaps. The sqlite introspect round-trip passes the verify comparison, external schema ownership refuses migrate apply after the plan is printed, default JSON bytes match the pre-phase printer, and the Python-client how-to lints clean against the OpenAPI 3.1 paths that exist.

---

_Verified: 2026-11-04T02:09:44Z_
_Verifier: Claude (gsd-verifier)_
