# Phase 327: `std.frame` on polars (`FRM-01..03`) - Context

**Gathered:** 2026-11-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Vox code builds a lazy polars plan and fetches strongly typed, non-nullable `Records` with one round-trip per `collect`. This phase ships `scan_csv`, `col`, `filter`, `group_by`, `agg`, `sort`, and `collect`, plus the comparisons and aggregations those need. It does not ship the rest of the design's frame vocabulary, `std.plot`, or `std.tensor`.

The published `.vox` surface stays Vox-shaped: no new keywords, no `HirType` variant, no indexing sugar, no Python `None`, and no `Option` in the scripts this phase accepts.

</domain>

<decisions>
## Implementation Decisions

### Plan width
- v1 operations are only `scan_csv`, `col`, `filter`, `group_by`, `agg`, `sort`, and `collect`.
- `collect` returns `Records`: one typed column per field, using the design §4.3 mapping for bool, integers, floats, strings, bytes, `Decimal`, `Date`, UTC `DateTime`, and `Duration`.
- Column names are written `col("amount")`. `frame.amount` is not syntax.
- A missing column is a compile error when the scan schema is known, and a `py-plan:` error when it is not.

### Expressions
- Aggregations are `sum`, `count`, `mean`, `min`, and `max`. The golden uses `sum`.
- `filter` accepts `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `and`, and `or`.
- `sort` requires `nulls=first` or `nulls=last`. That argument chooses placement inside the plan. It is not a null value in the script.
- Every aggregation requires `.alias("name")`.

### What collect hands back
- A null cell at `collect` fails with `py-plan:` and names the plan step that must drop or fill that cell. The script stays non-nullable.
- `num_rows()` returns `int`. `column("revenue")` returns `list[int]`, `list[float]`, `list[str]`, `list[Decimal]`, `list[Date]`, `list[DateTime]`, or `list[Duration]`, matching the column.
- A naive timestamp, struct, map, or union is an error with the existing fix-it, before any Vox value is built. A naive timestamp's fix-it remains `.replace_time_zone("UTC")`.
- Polars is not installed until a plan names one PyPI version and the operator approves it. That checkpoint is the same shape as the numpy and pyarrow approval in phase 326. numpy 2.5.4 and pyarrow 26.0.1 are already approved. No other Python project is approved.

### Vox script surface
- `.vox` scripts stay pristine, strongly typed, and non-nullable. This overrides the `Option[T]` clause in FRM-02 and design §4.3. Do not implement nullable `Records`.
- FRM-02 still requires `Decimal`, UTC `DateTime`, `Date`, `Duration`, and the naive-timestamp error. Meet those without introducing `Option`.
- Do not edit `.planning/REQUIREMENTS.md` inside a task commit. This context is the spec for the Option override.

### Claude's Discretion
- Whether `std.frame` gets a sibling allowlist contract or a schema bump. `contracts/python/ecosystem-support.yaml` is a single `std.array` document today. Do not rewrite it into `std.frame` and do not drop the array ops.
- Expression and aggregation method names that match the design sketch (`col("region").eq("west")`, `col("amount").sum().alias("revenue")`, `sort("month", nulls=last)`) are the intended shape. Exact Rust type names are open.
- How a known CSV schema is recovered for the compile-time missing-column check is open, as long as an unknown schema still fails closed at runtime with `py-plan:`.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/vox-compiler/src/eval/array_plan.rs` `ArrayPlan` stores host columns and plan steps. `collect` for frames should follow that pattern: build the plan on the host, send it once.
- `crates/vox-compiler/src/eval/python.rs` `execute_plan` is the one worker round-trip. Array `to_list`, `item`, and `shape` already go through it.
- `contracts/python/plan-ir.v1.schema.json` is the plan DAG schema (`Source | Op | Materialize`).
- `contracts/python/ecosystem-support.yaml` is the `std.array` allowlist. `vox ci python-bridge-parity` requires a signature, a handler, and an expected case for every allowlisted op.
- The worker lives at `crates/vox-python-bridge/worker/vox_bridge/` and is stdlib-only except the approved numeric projects.

### Established Patterns
- Closed vocabulary. An op that is not allowlisted does not get a handler.
- Errors that cross the bridge start with `py:` or `py-plan:`.
- `@uses(python)` grants the worker. `scan_csv` also needs the host `fs` cap check before any worker request (FRM-03). Phase 324's `py` denial and exit 77 stay as they are.
- No new `HirType`. `Frame`, `Expr`, and `Records` are std `Named` or `Generic` types, same as `Array`.
- `LEXER_KEYWORDS` stays 54. `sort` nulls is a named argument, as on `std.array`.

### Integration Points
- Phase 325's bridge and phase 326's array plans are the base. Do not add a second worker or a `vox-cli` to `vox-python-bridge` crate edge.
- CR-02 (`astype` still materializes f64) and CR-03 (`list[bool]` aborts) stay open. They do not block this phase and are not in scope.
- The golden is `examples/golden/py_frame_report.vox`: `scan_csv`, `filter`, `group_by` / `agg`, `sort`, `collect`, one round-trip per `collect`.

</code_context>

<specifics>
## Specific Ideas

The operator accepted the recommended answers and added one rule: keep Vox scripts pristine, strongly typed, and non-nullable regardless. Null is not a Vox value in these scripts, and the scripts do not mention `Option`.

The design sketch in `docs/src/architecture/vox-python-interop-design-2026-11-01.md` §4.4 is the call shape. `plot.line` in that sketch belongs to phase 328, not this phase.

</specifics>

<deferred>
## Deferred Ideas

- `scan_parquet`, `from_records`, `select`, `with_columns`, `join`, `head`, `unique`, `lit`, and the rest of the design's ~30 frame operations.
- `Option[T]` records for nullable columns. Superseded for this phase by the non-null rule.
- Installing polars before the named version is approved.
- CR-02 and CR-03.

</deferred>
