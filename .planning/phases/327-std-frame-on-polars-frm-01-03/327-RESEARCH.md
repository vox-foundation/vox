# Phase 327: `std.frame` on polars (`FRM-01..03`) - Research

**Researched:** 2026-11-03
**Domain:** Lazy polars plans behind the existing Python bridge, fetched as non-null Vox columns
**Confidence:** MEDIUM

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

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

### Deferred Ideas (OUT OF SCOPE)
- `scan_parquet`, `from_records`, `select`, `with_columns`, `join`, `head`, `unique`, `lit`, and the rest of the design's ~30 frame operations.
- `Option[T]` records for nullable columns. Superseded for this phase by the non-null rule.
- Installing polars before the named version is approved.
- CR-02 and CR-03.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| FRM-01 | A `scan_csv → filter → group_by/agg → sort → collect` pipeline runs on polars with one round-trip per `collect`; the `py_frame_report` golden passes. | Sibling allowlist plus `frame_ops.py` dispatched from the existing `execute_plan` method. Plan builders stay on the host. `collect` is the only materializer. Golden run counts `execute_plan` lines the way `py_array_stats_run` does, and expects 1. |
| FRM-02 | Arrow columns map per design §4.3 (Decimal, UTC DateTime, Date, Duration, and the naive-timestamp fix-it). CONTEXT overrides the `Option[T]` clause: a null cell is `py-plan:`, not `Option`. | New host decoder beside `decode_column`. Worker raises `py-plan:` with the step name before encoding a null. Naive `datetime[μs]` (no zone) errors with the exact fix-it `.replace_time_zone("UTC")` before a `VoxValue` is built. Decimal uses existing `rust_decimal`. Date and DateTime use existing `chrono` inside `vox-compiler`. |
| FRM-03 | Paths passed to `scan_*`, `save`, and `load_*` outside the `--caps fs` grants are refused host-side before any request reaches the worker. | Call `fs_resolve_allowed` at `scan_csv` eval time. Denial is `VoxValue::_Denied("fs.scan_csv")`, which becomes `EvalError::CapabilityDenied` and exit 77. The worker receives only the canonical path, with `glob=False`. This phase implements the check for `scan_csv` only; `save` and `load_*` stay deferred with plot and tensor. |
</phase_requirements>

## Project Constraints (from .cursor/rules/)

These bind the plan. Research does not reopen them.

- Worker Python belongs only in `crates/vox-python-bridge/worker/**`. That tree is the approved bridge-worker exemption. No new `.py` anywhere else, and no new `.ps1` / `.sh` automation.
- A new `pub fn` in `crates/*/src/**` needs a same-file test first (`skeleton/untested-pub-api`).
- No new `HirType` variant. `LEXER_KEYWORDS` stays 54. Accepting `Token::And` / `Token::Or` as method names after a dot does not add a keyword.
- Do not add a `vox-cli` → `vox-python-bridge` crate edge. `vox-compiler` already depends on `vox-python-bridge`. `exceptions` ledger entries are user-authorized only.
- Secrets go through `vox_secrets::resolve_secret`. This phase adds no secret and no `VOX_*` env var.
- Do not open `turso::Connection`. Do not `std::fs::write` event or log data. A golden CSV fixture is test input, not a spool.
- Application crypto stays in `vox-crypto`. `vox py sync` already hashes `uv.lock` with blake3. Do not add a hash crate.
- Do not edit `contracts/python/ecosystem-support.yaml`, `.planning/STATE.md`, `.planning/ROADMAP.md`, or `.planning/REQUIREMENTS.md`.
- Local verification is `cargo test -p vox-compiler` (and the one golden test), not a workspace clippy or full suite.
- Plain `cargo` on `PATH` so the build broker shim stays in front. Do not call `~/.cargo/bin/cargo` by path.

## Summary

`std.frame` is a second lazy plan on the bridge phase 325 already ships. `ArrayPlan` stores host columns and op names and never calls the worker. `execute_plan` is the one round-trip, and `materialize_to_list` is the only host entry that calls it for arrays. Frame `scan_csv`, `filter`, `group_by`, `agg`, and `sort` append steps the same way. `collect` is the only step that calls `execute_plan`.

The allowlist file is a single `std.array` / `numpy` document, and `build.rs` panics on any other module or engine. A schema bump that turns that file into a multi-module document would rewrite the array contract. The frame allowlist is a sibling YAML file that reuses the existing schema unchanged. The array plan-IR name enum stays the twelve stats-slice ops. Frame nodes get their own schema file.

Polars is not installed in this worktree. The candidate pin is **`polars==2.1.0`**, from the PyPI JSON `info.version` fetched this session (`upload_time` `2026-10-25T17:58:26`, `requires_python` `>=3.11`). The legitimacy seam marks it `SUS` (`too-new`, `unknown-downloads`). The install stays behind the same human checkpoint phase 326 used for numpy and pyarrow. numpy 2.5.4 and pyarrow 26.0.1 stay approved and are not re-pinned here.

`HirType` has no `Frame`, `Date`, `DateTime`, or `Duration` variant, and it must not grow one. `Decimal` already exists as `HirType::Decimal` and `VoxValue::Decimal`. `Date`, `DateTime`, and `Duration` are wire names only. They become runtime values in the compiler, lowered through `HirType::Named`. A null cell is a `py-plan:` error that names the step. It is not `Option`.

**Primary recommendation:** Add `contracts/python/frame-support.yaml` (module `std.frame`, engine `polars`) beside the array file, route `family: frame` plans through `vox_bridge/frame_ops.py` from the existing `execute_plan` method, refuse `scan_csv` paths with `fs_resolve_allowed` before that call, and decode Arrow columns in a new host function that rejects nulls and naive timestamps before building a `VoxValue`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Plan construction (`scan_csv`, `col`, `filter`, `group_by`, `agg`, `sort`) | Interpreter (host) | — | Same rule as `ArrayPlan`: store steps, do not call the worker. |
| `scan_csv` path grant | Interpreter caps | — | `fs_resolve_allowed` runs before any `execute_plan`. The worker never sees a rejected path. |
| Lazy compute | Python worker (`polars`) | — | One `collect` sends the plan. Polars stays inside the worker. |
| `collect` → `Records` | Interpreter (host) | Arrow IPC in `vox-python-bridge` | The worker returns an Arrow body. The compiler maps columns to `VoxValue`. `decode_column` stays the array decoder. |
| Missing column, known schema | Type checker | — | A string-literal path whose CSV header can be read. No worker. |
| Missing column, unknown schema | Python worker | — | `py-plan:` after the path grant has already succeeded. |
| Null cell, naive timestamp, struct/map/union | Python worker, then host decoder | — | Worker errors first, with the step name. The host decoder repeats the refusal so a bad body cannot become `Option` or a `VoxValue`. |
| Allowlist parity | `contracts/python/frame-support.yaml` | `vox ci python-bridge-parity` | That command already runs the whole `python_bridge_parity` test binary. A second test in that file covers frame rows without a new CI arm. |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| polars | `2.1.0` (candidate, not installed) | Lazy CSV scan, filter, group_by, agg, sort, collect | Locked engine. Design floor was `polars>=1.9`. The pin to approve is the current PyPI release. [VERIFIED: https://pypi.org/pypi/polars/json `info.version` = `2.1.0`] |
| pyarrow | `26.0.1` (already approved) | Arrow IPC body on the worker | Already the array body format. Frame collect writes one record batch. Do not approve another Python project. |
| numpy | `2.5.4` (already approved) | Unchanged array plans | Frame dispatch must not import numpy for a frame plan. Array plans keep importing it inside `array_ops.py`. |
| arrow-array / arrow-ipc | `=60.1.0` | Host decode of the collect body | Already pinned in `crates/vox-python-bridge/Cargo.toml`. Do not bump them for this phase. [VERIFIED: crates/vox-python-bridge/Cargo.toml:14-15] |
| rust_decimal | workspace `1.36` | `VoxValue::Decimal` | Already the interpreter decimal. [VERIFIED: Cargo.toml:328] [VERIFIED: crates/vox-compiler/src/eval/value.rs:11] |
| chrono | workspace `0.4` | UTC `DateTime` and `Date` in the compiler | Already a `vox-compiler` dependency. Do not add it to `vox-python-bridge`. [VERIFIED: Cargo.toml:242] [VERIFIED: crates/vox-compiler/Cargo.toml:30] |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| Existing `vox_python_bridge::Bridge` | in tree | The one worker | `execute_plan` already sends params JSON plus an Arrow body. Frame plans use that method. Do not construct a second `Bridge`. |
| `contracts/python/ecosystem-support.schema.json` | schema_version 1 | Validates the sibling YAML | The schema does not fix `module` to `std.array`. `build.rs` does. Reuse the schema. Do not bump it. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Sibling `frame-support.yaml` | Schema bump to a list of modules inside `ecosystem-support.yaml` | A bump rewrites the array document. `build.rs` and `python_bridge_parity_test.rs` both require that file to be `std.array` / the twelve D-01 names. The sibling leaves those checks untouched. |
| `family: frame` inside `execute_plan` | A new RPC method `execute_frame` | A second method is a second worker entry. The design and the array golden both treat `execute_plan` as the only compute method. |
| New `HirType` variants | `HirType::Named("Frame")` / `Named("Records")` / `Named("Date")` / `Named("DateTime")` / `Named("Duration")` | The locked rule is no new `HirType`. `Ty::Array` already lowers to `HirType::Generic` without a new variant. Named types use the existing `Ty::Named` arm. |

**Installation:**

Do not install polars in a task that runs before the checkpoint. After the operator approves `polars==2.1.0`, the golden project's `[python] packages` list is the place it is named. `vox py sync` writes `.vox/python/pyproject.toml` and runs `uv lock` / `uv sync --frozen`. This research did not run that command.

```toml
[python]
version = "3.12"
packages = ["numpy==2.5.4", "pyarrow==26.0.1", "polars==2.1.0"]
```

`render_pyproject` turns `version = "3.12"` into `requires-python = ">=3.12"`. The array golden sets `UV_PYTHON=3.12` so the venv is CPython 3.12 even when the machine Python is newer. Copy that. [VERIFIED: crates/vox-cli/src/commands/py/env.rs:133-145] [VERIFIED: crates/vox-compiler/tests/py_array_stats_run.rs:63-91]

**Version verification:** PyPI JSON fetched 2026-11-03 from `https://pypi.org/pypi/polars/json`. `info.version` is `2.1.0`. `info.requires_python` is `>=3.11`. First upload in `urls` is `polars-2.1.0-py3-none-any.whl` at `2026-10-25T17:58:26`. Classifiers list `Programming Language :: Python :: 3.11` through `3.14`. A Tavily snippet that said `polars 1.43.2` contradicts that JSON. Use `2.1.0`.

The classifiers stop at 3.14. That list does not prove 3.15 is unsupported. `requires_python = ">=3.11"` is only a lower bound. The worker stays 3.12 because the manifest and `UV_PYTHON` say so, not because 3.15 was falsified.

## Package Legitimacy Audit

`gsd-tools query package-legitimacy check --ecosystem pypi polars` on 2026-11-03:

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| polars | PyPI | latest file `2026-10-25` (`too-new` vs 2026-11-03) | unknown (`weeklyDownloads: null`) | `https://www.pola.rs/` | SUS | Flagged. The planner inserts `checkpoint:human-verify` before any `vox py sync` that names it. Pin the approved text to `polars==2.1.0`. |

**Packages removed due to SLOP verdict:** none

**Packages flagged as suspicious [SUS]:** `polars` [WARNING: flagged as suspicious — verify before using.] The seam reasons are `too-new` and `unknown-downloads`. The project URL is the official Polars site, and the version was read from the registry JSON. The checkpoint is still required. Do not swap in another dataframe library.

numpy 2.5.4 and pyarrow 26.0.1 are already approved. This phase does not re-audit them and does not add a third Python project.

## Architecture Patterns

### System Architecture Diagram

```text
.vox source
    │
    ▼
typeck (StdFrameNs / Frame / Expr / Records)
    │  literal CSV path? ──yes──► read header on the host
    │                              missing col → compile error
    │                         no ─► schema unknown (runtime py-plan later)
    ▼
eval scan_csv(path)
    │
    ├─ fs_resolve_allowed(caps, path, write=false) == None
    │       └─► _Denied("fs.scan_csv") → CapabilityDenied → exit 77
    │           zero worker requests
    │
    └─ store canonical path + later steps on FramePlan
           filter / group_by / agg / sort append steps only
    │
    ▼
eval collect
    │
    ├─ caps lack py → _Denied("py.collect") → exit 77
    │
    ▼
execute_plan  (one call per collect; same Bridge, same method name)
    params: {"family":"frame","ops":[...]}
    body: empty (the CSV stays on disk; the worker opens the canonical path)
    │
    ▼
worker dispatch
    family == "frame" → frame_ops.execute_plan   (import polars here)
    else              → array_ops.execute_plan   (unchanged)
    │
    ├─ unknown op / missing column / null cell / naive timestamp
    │       └─► PlanError "py-plan: ..."
    │
    └─ Arrow IPC record batch
    │
    ▼
host frame decoder (not decode_column)
    null, naive timestamp, struct/map/union → error, no VoxValue
    else Records { columns }
         num_rows() → int
         column(name) → list of one concrete type
```

### Recommended Project Structure

```text
contracts/python/frame-support.yaml              # new sibling allowlist
contracts/python/frame-plan-ir.v1.schema.json    # new; array plan-ir enum stays
contracts/index.yaml                             # add two ids; do not rewrite the array ids
crates/vox-compiler/build.rs                     # second emitter; array emitter unchanged
crates/vox-compiler/src/python_frame_support.rs  # generated table, mirror python_array_support.rs
crates/vox-compiler/src/eval/frame_plan.rs       # host plan; no worker calls
crates/vox-compiler/src/eval/value.rs            # FramePlan + Records + Date/DateTime/Duration values
crates/vox-python-bridge/src/frame_columns.rs    # Arrow decode for collect; leave frame.rs alone
crates/vox-python-bridge/worker/vox_bridge/frame_ops.py
examples/golden/py_frame_report.vox
examples/golden/fixtures/py_frame_report.csv     # header is the known schema
```

`crates/vox-python-bridge/src/frame.rs` is the stdio length-prefix, not a dataframe. Its first lines are `//! Length-prefixed frames: u32 header length, JSON header, u64 body length, body.` [VERIFIED: crates/vox-python-bridge/src/frame.rs:1-2]. Do not put polars plans in that file.

### Pattern 1: Sibling allowlist, same schema

**What:** A second YAML document with the same keys the array file uses.
**When to use:** This phase. The array file stays the twelve-op numpy document.
**Example:**

```yaml
schema_version: 1
module: std.frame
engine: polars
ops:
  - name: scan_csv
    kind: plan
    signature: "(path: str) -> Frame"
    handler: scan_csv
    expected:
      path: "fixtures/py_frame_report.csv"
  - name: sort
    kind: plan
    signature: "sort(column: str, nulls: str) -> Frame"
    handler: sort
    required_named: [nulls]
    expected:
      nulls: last
  - name: sum
    kind: plan
    signature: "sum() -> Expr"
    handler: sum
    required_named: [alias]
    expected:
      alias: revenue
  - name: collect
    kind: materializer
    signature: "collect() -> Result[Records]"
    handler: collect
    expected:
      rows: 2
```

The schema this file must satisfy, unchanged [VERIFIED: contracts/python/ecosystem-support.schema.json:5-27]:

```json
"additionalProperties": false,
"required": ["schema_version", "module", "engine", "ops"],
```

Each op requires `name`, `kind`, `signature`, `handler`, `expected`. `kind` is `"plan"` or `"materializer"`. `required_named` is optional. [VERIFIED: contracts/python/ecosystem-support.schema.json:27-51]

`build.rs` will not read the sibling until a second function is added. The array emitter must keep these panics [VERIFIED: crates/vox-compiler/build.rs:201-211]:

```rust
if parsed.module != "std.array" {
    panic!(
        "python array registry module must be std.array, got {}",
        parsed.module
    );
}
if parsed.engine != "numpy" {
    panic!(
        "python array registry engine must be numpy, got {}",
        parsed.engine
    );
}
```

`vox ci python-bridge-parity` runs `cargo test -p vox-compiler --test python_bridge_parity` with no test filter [VERIFIED: crates/vox-cli/src/commands/ci/run_body.rs:494-504]. Add a frame test in that binary. Leave `on_disk_expected_matches_behavior_cases` asserting the twelve array names.

Rows to allowlist, in this order. `collect` is the only `materializer`. Every other row is `kind: plan`. `handler` equals `name`.

| name | required_named | Worker call (polars 2.1 stable docs) |
|------|----------------|--------------------------------------|
| scan_csv | — | `pl.scan_csv(path, glob=False)` |
| col | — | `pl.col(name)` |
| filter | — | `LazyFrame.filter(expr)` |
| group_by | — | `LazyFrame.group_by(keys)` |
| agg | — | `LazyGroupBy.agg(exprs)` |
| sort | `[nulls]` | `LazyFrame.sort(column, nulls_last=...)` |
| sum | `[alias]` | `Expr.sum().alias(name)` |
| count | `[alias]` | `Expr.count().alias(name)` → Arrow `UInt32`, host `int` |
| mean | `[alias]` | `Expr.mean().alias(name)` → `f64` |
| min | `[alias]` | `Expr.min().alias(name)` [ASSUMED] |
| max | `[alias]` | `Expr.max().alias(name)` [ASSUMED] |
| eq | — | `Expr.eq` |
| neq | — | `Expr.ne` |
| gt | — | `Expr.gt` |
| gte | — | `Expr.ge` |
| lt | — | `Expr.lt` |
| lte | — | `Expr.le` |
| and | — | `Expr.and_` |
| or | — | `Expr.or_` |
| collect | — | `LazyFrame.collect()` then one Arrow IPC batch |

Vox method names stay `neq`, `gte`, `lte`, `and`, and `or`. The worker translates. Polars spells those `ne`, `ge`, `le`, `and_`, and `or_` because `and` / `or` are Python keywords. [CITED: https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.ne.html] [CITED: https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.ge.html] [CITED: https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.le.html] [CITED: https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.and_.html] [CITED: https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.or_.html]

`contracts/index.yaml` already indexes the array file as `python-ecosystem-support` with description `std.array stats-slice allowlist (signature, handler, and expected value per op).` [VERIFIED: contracts/index.yaml:925-929]. Add new ids (`python-frame-support`, `python-frame-plan-ir`). Do not change that description.

`plan-ir.v1.schema.json` `name.enum` is exactly `from_list`, `astype`, `add`, `sub`, `mul`, `div`, `mean`, `std`, `sort`, `to_list`, `item`, `shape`. [VERIFIED: contracts/python/plan-ir.v1.schema.json:31]. Leave that enum. Put frame names in `frame-plan-ir.v1.schema.json`.

### Pattern 2: One worker call, at collect

**What:** `execute_plan` is already the single host→worker compute call.
**When to use:** `collect`, and nowhere earlier.
**Example:** The function's own contract [VERIFIED: crates/vox-compiler/src/eval/python.rs:140-147]:

```rust
/// One `execute_plan` on the worker this host already owns.
///
/// Does not construct a second [`vox_python_bridge::Bridge`].
pub(crate) fn execute_plan(
    interp: &mut Interpreter,
    params: serde_json::Value,
    body: Vec<u8>,
) -> Result<vox_python_bridge::Reply, String> {
```

Array materialize refuses a missing `py` grant before that call [VERIFIED: crates/vox-compiler/src/eval/builtins.rs:2628-2637]:

```rust
pub fn materialize_to_list(
    interp: &mut super::Interpreter,
    plan: &super::array_plan::ArrayPlan,
    caps: &crate::eval::caps::CapabilitySet,
) -> VoxValue {
    if !caps.allows_namespace("py") {
        return VoxValue::_Denied("py.to_list".into());
    }
```

`_Denied` is not catchable. The method-call boundary splits on the first dot and returns `CapabilityDenied` [VERIFIED: crates/vox-compiler/src/eval/expr.rs:892-898]. `vox run` exits 77 [VERIFIED: crates/vox-cli/src/commands/run.rs:126-130]:

```rust
Err(vox_compiler::eval::EvalError::CapabilityDenied { ns, method }) => {
    eprintln!(
        "vox: capability denied: {ns}.{method} — grant with --caps <token> (see isolation.md)"
    );
    std::process::exit(77);
}
```

Frame `collect` uses `_Denied("py.collect")`. Frame `scan_csv` uses `_Denied("fs.scan_csv")` when the path is outside the grant. Both exit 77. A `py-plan:` string is a `Result` error, not exit 77. Keep those apart.

The worker method switch loads array ops for every `execute_plan` today [VERIFIED: crates/vox-python-bridge/worker/vox_bridge/__main__.py:53-59]:

```python
def dispatch(method: str, params: dict, body: bytes, debug_ops: bool):
    if method == "execute_plan":
        from vox_bridge.array_ops import execute_plan

        return execute_plan(params, body)
```

Change this to: when `params.get("family") == "frame"`, import `frame_ops` and return. Otherwise call `array_ops.execute_plan` unchanged. Unknown frame names fail in `frame_ops` with `py-plan: unknown op`. Array unknown names stay in `array_ops` [VERIFIED: crates/vox-python-bridge/worker/vox_bridge/array_ops.py:180-182]:

```python
handler = HANDLERS.get(name) if isinstance(name, str) else None
if handler is None:
    raise PlanError(f"py-plan: unknown op {name!r}")
```

Array plans have no `family` key. `plan_params` emits `{"ops":[...]}` only [VERIFIED: crates/vox-compiler/src/eval/builtins.rs:2798-2826]. Leaving that function alone keeps array plans on the numpy path.

### Pattern 3: Host path check before the worker

**What:** Canonicalize, reject `..`, then `allows_path`.
**When to use:** `scan_csv`, at eval time, before a `FramePlan` that could be collected is stored.
**Example:** [VERIFIED: crates/vox-compiler/src/eval/builtins.rs:134-164]

```rust
fn fs_resolve_allowed(
    caps: &crate::eval::caps::CapabilitySet,
    raw: &str,
    write: bool,
) -> Option<std::path::PathBuf> {
    if raw.is_empty() {
        return None;
    }
    let p = std::path::Path::new(raw);
    if p.components()
        .any(|component| component == std::path::Component::ParentDir)
    {
        return None;
    }
    // ...
    caps.allows_path(&canon, write).then_some(canon)
}
```

`allows_path` is the grant check. An unscoped `fs` (`fs_ro: None`) allows every path [VERIFIED: crates/vox-compiler/src/eval/caps.rs:327-342]:

```rust
/// `path` MUST be canonical (the fs builtins canonicalize before calling this).
/// rw roots satisfy reads too — a writable directory is always readable.
pub fn allows_path(&self, path: &Path, write: bool) -> bool {
    if !self.allows_namespace("fs") {
        return false;
    }
    let roots = if write { &self.fs_rw } else { &self.fs_ro };
    match roots {
        None => true,
        Some(rs) => {
```

Phase 324 added `"py"` and `"py.unverified"` to `GATED` and did not add a scan-path check. Phase 325's `execute_plan` has no path argument. The import gate is the sibling pattern: `resolve_local_file_import` canonicalizes, then `allows_path(&canonical, false)`, and returns `CapabilityDenied { ns: "fs", method: "import" }` [VERIFIED: crates/vox-compiler/src/eval/mod.rs:673-677]. `scan_csv` uses `fs_resolve_allowed` rather than that import function, because `fs_resolve_allowed` also rejects `..` and returns the canonical path to store.

The FRM-03 test must parse a scoped grant, `py:allow` plus `fs:ro=<tempdir>`. `CapabilitySet::developer_default()` and `fs_unscoped` allow `/`. A test on the developer default cannot show refusal. `fs:ro=` roots are canonicalized at parse time [VERIFIED: crates/vox-compiler/src/eval/caps.rs:91-107].

`@uses(python)` is `HirCapability::Python`. `stdlib_module_capability` maps `"python"` and does not map `"array"` or `"frame"` [VERIFIED: crates/vox-compiler/src/typeck/effect_check.rs:588-603]. Array `to_list` / `item` / `shape` check `current_caps` for `HirCapability::Python` in the type checker [VERIFIED: crates/vox-compiler/src/typeck/checker/expr.rs:579-590]. Do the same for `collect` (`Python`) and `scan_csv` (`Python` and `Fs`). `sort` on an array does not require `uses python` [VERIFIED: crates/vox-compiler/src/typeck/checker/expr.rs:1147 area, test name `sort must not require uses python`]. Frame `sort` is also a plan step, so it does not require the effect by itself. The function that calls `scan_csv` or `collect` does.

### Pattern 4: Types without a new HirType

**What:** Runtime values and `Ty` names. `HirType` stays the six variants it has.
**When to use:** `Frame`, `Expr`, `Records`, `Date`, `DateTime`, `Duration`.
**Example:** [VERIFIED: crates/vox-compiler/src/hir/nodes/stmt_expr.rs:111-124]

```rust
pub enum HirType {
    /// Named scalar or path type (`int`, `User`).
    Named(String),
    /// Generic application (`list[T]`).
    Generic(String, Vec<HirType>),
    /// Function type (`fn(a, b) -> c`).
    Function(Vec<HirType>, Box<HirType>),
    /// Tuple type.
    Tuple(Vec<HirType>),
    /// Unit / void.
    Unit,
    /// Fixed-point decimal.
    Decimal,
}
```

`Ty::Array` is the precedent for a checker type that is not a new `HirType` [VERIFIED: crates/vox-compiler/src/typeck/ty.rs:16-17 and :124]:

```rust
/// Lazy `std.array` plan. Lowered as `HirType::Generic`, not a new `HirType` variant.
Array(Box<Ty>),
```

```rust
Ty::Array(inner) => HirType::Generic("Array".into(), vec![inner.to_hir_type()]),
```

```rust
Ty::Named(n) => HirType::Named(n.clone()),
```

Use `Ty::Named("Frame")`, `Ty::Named("Expr")`, and `Ty::Named("Records")`. Use `Ty::Named("Date")`, `Ty::Named("DateTime")`, and `Ty::Named("Duration")` for the fetched scalars. `Decimal` stays `Ty::Decimal` / `HirType::Decimal`.

Do not reuse `Ty::Record`. Its `get` returns `Option` [VERIFIED: crates/vox-compiler/src/typeck/builtins.rs:1989-1996]. `Records` is a column store: `num_rows() -> int`, `column(name) -> list[T]` with no `Option` in the signature.

`VoxValue` may grow variants. `ArrayPlan` already did [VERIFIED: crates/vox-compiler/src/eval/value.rs:16-18]. Recommended names, open under discretion: `FramePlan`, `Records`, `Date`, `DateTime`, `Duration`. There is no `VoxValue` for Date, DateTime, or Duration today (search of `crates/vox-compiler/src/eval` found none).

`LEXER_KEYWORDS` length is frozen at 54, first `"fn"`, last `"form"` [VERIFIED: crates/vox-language-surface/src/lib.rs:416-418]:

```rust
assert_eq!(LEXER_KEYWORDS.len(), 54);
assert_eq!(LEXER_KEYWORDS.first().copied(), Some("fn"));
assert_eq!(LEXER_KEYWORDS.last().copied(), Some("form"));
```

`"and"` and `"or"` are already in that list [VERIFIED: crates/vox-language-surface/src/lib.rs:223-224]. `parse_ident_name` accepts `Token::Ident` and does not match `Token::And` or `Token::Or` anywhere in `head.rs` (search this session found no `Token::And` in that file). Method calls take the name from `parse_ident_name` [VERIFIED: crates/vox-compiler/src/parser/descent/expr/pratt_match.rs:504-514]. `.and(` and `.or(` will not parse until `parse_ident_name` maps `Token::And` → `"and"` and `Token::Or` → `"or"`, the same way object keys already do [VERIFIED: crates/vox-compiler/src/parser/descent/expr/pratt_match.rs:655-658]. That change does not add a keyword.

`sort`'s `nulls` argument is already a named-argument check on arrays [VERIFIED: crates/vox-compiler/src/typeck/checker/expr_ops.rs:202-213]:

```rust
/// `sort` accepts only the name `nulls` bound to the string `first` or
/// `last`. A missing name, a positional string, and any other string are
/// diagnostics. The name is checked before any positional argument zip.
/// `nulls` is not defaulted.
pub(super) fn check_array_sort_args(&mut self, args: &[HirArg], span: Span) {
```

Frame `sort("month", nulls=last)` needs a column string plus that same `nulls` check. Do not default `nulls`.

### Pattern 5: Known CSV schema is the header of a string literal

**What:** If `scan_csv`'s path is a string literal, and the file can be read without `..`, the type checker splits the first line on commas and treats those names as the scan schema. `col("missing")` is then a compile error. If the path is a runtime `str`, or the file cannot be read, the schema is unknown and the worker returns `py-plan:` when the column is absent. A missing file is not a compile error.

**When to use:** The golden. Put the CSV at a literal path next to the script so the header is known. Do not add a `schema` argument to `scan_csv`. That would be a new surface. Default `scan_csv` inference will not produce `Decimal`, a time zone, `Date`, or `Duration`. The FRM-02 fixture is an Arrow batch through the collect decoder (and, after the checkpoint, a polars `DataFrame` of those dtypes inside the worker test). It is not a CSV dialect.

### Anti-Patterns to Avoid

- **Rewriting `ecosystem-support.yaml` into a frame document.** The array parity test compares its `ops` names to `D01_OPS` [VERIFIED: crates/vox-compiler/tests/python_bridge_parity_test.rs:19-47].
- **Putting frame op names in `plan-ir.v1.schema.json`.** That enum is the stats slice.
- **Calling `decode_column` for a frame batch.** It only returns `Floats` or `Ints`, and a null is `py: arrow decode: column contains nulls` [VERIFIED: crates/vox-python-bridge/src/array_numeric.rs:18-20 and :77-79]. The wrong prefix, and it does not name a plan step. Leave that function for arrays.
- **Loading polars inside `array_ops.execute_plan`.** Array plans must keep their current imports.
- **Passing the raw user path to `scan_csv`.** Polars `glob` defaults to `True`, and `credential_provider` defaults to `'auto'` [CITED: https://docs.pola.rs/api/python/stable/reference/api/polars.scan_csv.html]. The worker calls `pl.scan_csv(canonical, glob=False)` and passes no `storage_options`.
- **Using `developer_default()` to prove FRM-03.** Unscoped `fs` allows every canonical path.
- **Implementing `.replace_time_zone` as a Vox method.** It is the fix-it text, not a v1 op. Polars spells the real method `Expr.dt.replace_time_zone(time_zone)` [CITED: https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.dt.replace_time_zone.html]. The error string stays `.replace_time_zone("UTC")`.
- **Mapping a null cell to `Option`.** CONTEXT overrides FRM-02 and design §4.3 on this point.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Lazy CSV engine | A Vox dataframe | polars `LazyFrame` inside the worker, after the checkpoint | The locked engine. Filter pushdown and the dtype table are polars/Arrow behavior. |
| Arrow IPC | A JSON numeric body | Existing framed body (`arrow-ipc` 60.1.0) | Design §7: scalars and columns are Arrow, not JSON numbers. Array plans already do this. |
| Decimal parse | A custom decimal | `rust_decimal::Decimal`, already `VoxValue::Decimal` | Wire v1 encodes `Decimal` as a string (`"19.99"`) [VERIFIED: docs/src/architecture/wire-format-v1-ssot.md:109]. Parse with the crate. |
| UTC timestamps | Epoch integers in the script | `chrono` in the compiler, after the bridge reports microseconds plus a zone | Wire v1: `Date` is `2026-05-01`, `DateTime` is `2026-05-01T14:30:00Z`, `Duration` is `PT1H30M` [VERIFIED: docs/src/architecture/wire-format-v1-ssot.md:111-113]. The script never sees a naive timestamp. |
| Path jail | A new sandbox | `fs_resolve_allowed` + `allows_path` | Symlinks die at `canonicalize`. `..` dies before the unscoped short-circuit. |
| Second worker | `execute_frame` or another `Bridge` | `family: frame` on `execute_plan` | One process, one method, FIFO. Phase 325 already serializes calls. |
| Nullable rows | `Option[T]` columns | `py-plan:` naming the step | Locked override. |

**Key insight:** The host owns caps, types, and the plan JSON. Polars owns the scan. The Arrow body is the only collect payload, and it is decoded into non-null Vox columns or rejected.

## Common Pitfalls

### Pitfall 1: `sort(nulls=last)` is not a polars keyword

**What goes wrong:** The worker passes `nulls="last"` into `LazyFrame.sort` and polars raises `TypeError`.
**Why it happens:** Polars takes `nulls_last: bool`, default `False` (nulls first). [CITED: https://docs.pola.rs/api/python/stable/reference/lazyframe/api/polars.LazyFrame.sort.html `nulls_last: bool | Sequence[bool] = False`]
**How to avoid:** `nulls=first` → `nulls_last=False`. `nulls=last` → `nulls_last=True`. Reject any other string with `py-plan:` before calling polars. The array worker already refuses to pass `nulls` through to `np.sort` [VERIFIED: crates/vox-python-bridge/worker/vox_bridge/array_ops.py:97-101].
**Warning signs:** A sort test that only checks the happy path with no nulls still passes while the argument is ignored.

### Pitfall 2: An aggregation without `.alias` still runs in polars

**What goes wrong:** `col("amount").sum()` keeps the name `amount`. The golden's `column("revenue")` then misses.
**Why it happens:** `group_by().agg(pl.col("b").sum())` leaves the column named `b`. [CITED: https://docs.pola.rs/api/python/stable/reference/lazyframe/api/polars.LazyFrame.group_by.html]. `Expr.alias(name)` is a separate call [CITED: https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.alias.html].
**How to avoid:** Typeck and the worker both require `alias`. The allowlist row sets `required_named: [alias]`. A missing alias is a compile error on a literal chain, and `py-plan:` if it still reaches the worker.
**Warning signs:** `collect` succeeds and `num_rows()` is right, but the column name is the source name.

### Pitfall 3: `count` is `UInt32`, and the array decoder will reject it

**What goes wrong:** `collect` fails with `py: arrow decode: unsupported column` or wraps a count as a float.
**Why it happens:** `Expr.count()` returns `UInt32`. [CITED: https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.count.html `Expression of data type UInt32.`] `decode_column` accepts `Int64` and `UInt64` only.
**How to avoid:** The frame decoder widens `uint8`–`uint32` to `i64` per design §4.3. `uint64` above `i64::MAX` is the existing `py:` error. `mean` is `f64` [CITED: https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.mean.html].
**Warning signs:** The golden uses `sum`, so a count-only bug stays hidden until the allowlist expected case runs.

### Pitfall 4: Nulls and naive timestamps become values

**What goes wrong:** An empty CSV field or a timezone-naive datetime comes back as `Option`, `Null`, or a `DateTime` with no zone.
**Why it happens:** `scan_csv` defaults `empty_string_is_null=True` [CITED: https://docs.pola.rs/api/python/stable/reference/api/polars.scan_csv.html]. Polars prints a naive datetime as `datetime[μs]` with no zone in the `replace_time_zone` example [CITED: https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.dt.replace_time_zone.html]. The array decoder's null error is `py: arrow decode: column contains nulls`, which is the wrong prefix and does not name a step.
**How to avoid:** After `collect`, if any column has a null, raise `PlanError` whose message starts with `py-plan:` and includes the step name (`scan_csv` when the null was in the file) plus the column name. The host decoder, given a null or a timestamp with no timezone, returns an error and builds no `VoxValue`. The naive-timestamp text includes `.replace_time_zone("UTC")`. Struct, map, and union take the same early error. The golden CSV has no empty fields.
**Warning signs:** A fixture assertion that checks `is_some()` or mentions `Option`.

### Pitfall 5: `group_by` order is not the sort

**What goes wrong:** The golden asserts row order before `sort`, and polars returns groups in hash order.
**Why it happens:** `maintain_order` defaults to `False` [CITED: https://docs.pola.rs/api/python/stable/reference/lazyframe/api/polars.LazyFrame.group_by.html].
**How to avoid:** The golden sorts on `month` with `nulls=last` and asserts the sorted column, not the pre-sort group order. `num_rows()` alone can pass with a wrong filter. Assert the row count and the sorted `revenue` list.
**Warning signs:** A test that passes twice locally and fails when the group hash seed changes.

### Pitfall 6: `scan_csv` glob and cloud credentials

**What goes wrong:** A path containing `*` reads more than the granted file, or polars tries a cloud credential.
**Why it happens:** `glob=True` and `credential_provider='auto'` are the defaults on the page cited above.
**How to avoid:** One canonical file path, `glob=False`, no storage options. The path check uses `write=false`.
**Warning signs:** The worker log shows more than one file, or a network call during `scan_csv`.

### Pitfall 7: Proving the path grant with the developer cap set

**What goes wrong:** FRM-03 looks green because `allows_path` returns true for every path.
**Why it happens:** `None => true` when `fs_ro` is unset. `fs_unscoped` is true when `/` is readable and writable.
**How to avoid:** The denial test builds `CapabilitySet::parse` with `py:allow` and `fs:ro` set to a real temp directory, scans a path outside it, and asserts `CapabilityDenied` with zero `execute_plan` log lines. Also assert `..` never reaches the worker.
**Warning signs:** The test never calls `CapabilitySet::parse`.

### Pitfall 8: `.and(` / `.or(` fail in the parser

**What goes wrong:** `col("a").gt(1).and(col("b").lt(2))` is a parse error even though the allowlist has `and`.
**Why it happens:** `and` and `or` lex as keywords. `parse_ident_name` does not map those tokens.
**How to avoid:** Map `Token::And` and `Token::Or` in `parse_ident_name`. Add a parse test. Do not add them to `LEXER_KEYWORDS`.
**Warning signs:** The golden avoids `and`/`or` (it only needs `eq`) and the parser bug ships anyway.

### Pitfall 9: Installing polars to make the decoder test compile

**What goes wrong:** A task runs `uv add` or `vox py sync` before the checkpoint, or the host decoder test imports polars.
**Why it happens:** FRM-02 needs Arrow types that CSV inference will not produce.
**How to avoid:** Host decoder tests build Arrow IPC with `arrow-array` 60.1.0 (decimal128, timestamp with and without a zone, date32, duration, a null slot). The polars-backed worker test and the golden `vox py sync` wait for the checkpoint.
**Warning signs:** A diff that creates `.vox/python/` or edits a lockfile before the checkpoint task.

## Code Examples

### Golden shape

```vox
// vox:skip — proposed golden, not compilable until this phase lands
import std.frame

@uses(python, fs)
fn main() {
    let monthly = frame.scan_csv("fixtures/py_frame_report.csv")
        .filter(frame.col("region").eq("west"))
        .group_by(["month"])
        .agg([frame.col("amount").sum().alias("revenue")])
        .sort("month", nulls=last)
    let rows = monthly.collect()?
    assert(rows.num_rows() == 2)
    assert(rows.column("revenue") == [10, 5])
}
```

One `collect`, so one `execute_plan` line in the debug log. The array golden expects three because that script materializes three times [VERIFIED: crates/vox-compiler/tests/py_array_stats_run.rs:144-150].

### Worker translation of sort and comparisons

```python
# Source: docs.pola.rs LazyFrame.sort, Expr.eq, Expr.ge, Expr.and_
# Vox nulls=last  → nulls_last=True
# Vox nulls=first → nulls_last=False
# Vox gte         → expr.ge(other)
# Vox lte         → expr.le(other)
# Vox neq         → expr.ne(other)
# Vox and / or    → expr.and_(other) / expr.or_(other)
frame.sort("month", nulls_last=True)
```

### Naive timestamp error

The worker and the host decoder both refuse a timestamp whose Arrow type has no timezone, before any `VoxValue::DateTime` is constructed. The message contains this exact substring:

```text
py-plan: .replace_time_zone("UTC")
```

Polars' own method is `Expr.dt.replace_time_zone(time_zone)`. Do not change the fix-it to that spelling. Do not allowlist `replace_time_zone`.

### Null cell error

```text
py-plan: scan_csv must drop or fill nulls in column amount
```

The step name is the plan step that produced the null (`scan_csv` for a null in the file). The script does not mention `Option`, `drop_nulls`, or `fill_null`. Those ops are deferred.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Design §4.5 one file, `ecosystem-support.yaml`, for every std module | Phase 326 made that file a twelve-op `std.array` document and froze the plan-IR enum | Phase 326 | Frame ops cannot be appended to that file without dropping or mixing array ops. Use a sibling. |
| Design §4.3 nullable column → `Option[T]` | CONTEXT: null at `collect` is `py-plan:` | 2026-11-03 context | Do not implement nullable `Records`. Do not edit REQUIREMENTS.md to match. |
| Design package floor `polars>=1.9` | PyPI latest inspected this session is `2.1.0` | Registry JSON `2026-10-25` | The checkpoint names `polars==2.1.0`. The `>=1.9` floor is not the pin. |
| `decode_column` for every fetch | Array decoder stays f64/i64. Frame collect gets its own decoder | This phase | Sharing the decoder mis-labels nulls as `py:` and drops Decimal/Date/DateTime/Duration. |

**Deprecated/outdated:**

- Tavily's `polars 1.43.2` snippet. The fetched registry JSON says `2.1.0`.
- Treating `crates/vox-python-bridge/src/frame.rs` as the dataframe module. It is the stdio frame codec.
- Calling `Expr.gte` / `Expr.lte` / `Expr.and` in Python. Stable docs name those `ge`, `le`, and `and_`.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `Expr.min()` and `Expr.max()` exist as methods next to `Expr.sum()` and `Expr.mean()`. The group_by page shows `pl.max("c")` as a function and `pl.col("b").sum()` as a method. The min/max method pages were not opened. | Standard Stack, allowlist table | The worker handler names are wrong. Confirm against the 2.1 docs before writing `frame_ops.py`. If the method is absent, call the documented function form inside the handler. The Vox spelling stays `.min()` / `.max()`. |
| A2 | Polars' default datetime unit remains microseconds (`datetime[μs]`), as printed in the stable `replace_time_zone` example. The host should accept `timestamp[us]`, `timestamp[ns]`, `timestamp[ms]`, and `timestamp[s]` and normalize to UTC microseconds. | Code Examples, FRM-02 | A single-unit match drops a legal UTC column. Accept the Arrow time unit and convert. |
| A3 | `arrow-array` 60.1.0 can encode `Decimal128`, `Timestamp`, `Date32`, and `Duration` arrays. The crate is pinned. Those type names were not opened in the crate source this session. | Don't Hand-Roll, Wave 0 | The decoder test cannot build the fixture with the names above. Use the types that `arrow-array` 60.1.0 actually exports. Do not bump the crate. |

**If this table is empty:** It is not. A1–A3 need a look during implementation. They do not need a user decision before planning. The polars pin does: the checkpoint approves `2.1.0`.

## Open Questions

1. **Exact Rust type names for `FramePlan` and `Records`**
   - What we know: They are runtime values, not `HirType` variants. `ArrayPlan` is the pattern.
   - What's unclear: The identifier spelling.
   - Recommendation: `VoxValue::FramePlan` and `VoxValue::Records`. Planner may rename if a clash shows up. Do not add `HirType` variants either way.

2. **Which plan step a null produced by `sum` of an empty group should name**
   - What we know: A null in the scanned file is `scan_csv`. CONTEXT says the message names the step that must drop or fill the cell.
   - What's unclear: Polars' null-from-aggregation step name, because `drop_nulls` / `fill_null` are deferred and were not probed.
   - Recommendation: Name the op that wrote the column (`sum`, `count`, `mean`, `min`, `max`, or `scan_csv`). The golden has no nulls, so this is the fixture's assertion, not the golden's.

3. **`min` / `max` Python spelling**
   - What we know: See A1.
   - What's unclear: Method versus `pl.min` / `pl.max`.
   - Recommendation: Resolve while writing the handler, after the checkpoint, against the installed 2.1 docs. Do not block the allowlist row names. Those stay `min` and `max`.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo | Host tests | ✓ | 1.98.1 | — |
| uv | `vox py sync` after the checkpoint | ✓ | 0.13.6 | — |
| Python on `PATH` | Not the worker | ✓ | 3.15.2 | Do not use it as the worker. The array golden forces `UV_PYTHON=3.12`. |
| `.vox/python` venv | Golden run | ✗ | — | Created by `vox py sync` after the checkpoint. Absent in this worktree (`VOX_PYTHON_DIR=no`). |
| polars | Worker compute | ✗ | candidate `2.1.0` | Do not install during research or before the checkpoint. Host decoder tests use `arrow-array` only. |
| numpy / pyarrow | Array plans; frame IPC | approved, not installed in this worktree | 2.5.4 / 26.0.1 | The golden sync installs the approved pins together with polars. |
| Context7 | Preferred docs provider | ✗ needs auth | — | Official `docs.pola.rs` pages were fetched instead. |

**Missing dependencies with no fallback:**

- None for planning. The polars install is a gated checkpoint, not a silent skip.

**Missing dependencies with fallback:**

- Polars, until the checkpoint. Decoder tests and the path-denial test do not need it.
- Context7. Docs were read from `docs.pola.rs` stable.

Graphify status was `disabled`. No graph context was used.

## Validation Architecture

`workflow.nyquist_validation` is absent in `.planning/config.json`, so this section is in force.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` (`vox-compiler` integration tests + unit tests). Worker behavior is covered after the checkpoint by `vox run` of the golden, same as `py_array_stats_run`. |
| Config file | none — Cargo integration tests under `crates/vox-compiler/tests/` |
| Quick run command | `cargo test -p vox-compiler --test python_bridge_parity` |
| Full suite command | `cargo test -p vox-compiler --test python_bridge_parity --test py_frame_report_run` plus the frame plan unit tests. The golden binary does not exist yet. |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| FRM-01 | Golden pipeline, one `execute_plan` per `collect` | integration | `cargo test -p vox-compiler --test py_frame_report_run` | ❌ Wave 0, and it waits on the polars checkpoint |
| FRM-01 | Allowlist rows have signature, handler, expected; array rows unchanged | unit | `cargo test -p vox-compiler --test python_bridge_parity` | ✅ array test exists. ❌ frame test is Wave 0 inside that binary |
| FRM-01 | Plan build records zero worker calls; `collect` records one | unit | `cargo test -p vox-compiler frame_plan_collect_is_one_round_trip` | ❌ Wave 0 |
| FRM-02 | Decimal, UTC DateTime, Date, Duration decode; null is `py-plan:`; naive timestamp includes `.replace_time_zone("UTC")` and builds no value | unit | `cargo test -p vox-python-bridge frame_column_decode` | ❌ Wave 0. No polars import |
| FRM-02 | `.and(` / `.or(` parse as method names | unit | `cargo test -p vox-compiler method_and_or_parse` | ❌ Wave 0 |
| FRM-03 | Path outside `fs:ro` is `CapabilityDenied` and does not spawn a worker request | unit | `cargo test -p vox-compiler scan_csv_outside_fs_ro_never_calls_worker` | ❌ Wave 0. Uses `CapabilitySet::parse`, not `developer_default` |
| FRM-03 | `..` in the path never reaches the worker | unit | same test module | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p vox-compiler --test python_bridge_parity` and the new unit test named in that task.
- **Per wave merge:** the quick command plus `frame_column_decode` and the path-denial test.
- **Phase gate:** those tests green, then the golden `py_frame_report_run` after the checkpoint. `/gsd-verify-work` does not treat a skipped golden as a pass. The array golden says a skip or `#[ignore]` is not a pass [VERIFIED: crates/vox-compiler/tests/py_array_stats_run.rs:3-4].

### Wave 0 Gaps

- [ ] `contracts/python/frame-support.yaml` — the twenty rows above
- [ ] `contracts/python/frame-plan-ir.v1.schema.json` — frame names only
- [ ] `crates/vox-compiler/tests/python_bridge_parity_test.rs` — second test, array test unchanged
- [ ] `crates/vox-compiler/src/eval/frame_plan.rs` — zero worker calls until `collect`
- [ ] `crates/vox-python-bridge/src/frame_columns.rs` — dtype fixture without polars
- [ ] `crates/vox-compiler/tests/py_frame_report_run.rs` — after the checkpoint only
- [ ] Parser test that `.and(` and `.or(` are method calls
- [ ] No new test framework. Cargo test is already the runner.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No user login on this path. |
| V3 Session Management | no | One local worker process. No session cookie. |
| V4 Access Control | yes | `fs_resolve_allowed` then `CapabilitySet::allows_path` before `execute_plan`. Missing `py` is `_Denied`, exit 77. Phase 324's `py` denial stays. |
| V5 Input Validation | yes | Allowlist + plan schema. Unknown op is `py-plan:`. `sort` accepts only `nulls=first\|last`. Aggregations require `alias`. Null, naive timestamp, struct, map, and union never become a `VoxValue`. `scan_csv(..., glob=False)` only. |
| V6 Cryptography | no | No new crypto. The existing `uv.lock` blake3 hash in `vox py sync` stays where it is. |

### Known Threat Patterns for this bridge

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| CSV path outside `fs:ro`, or a `..` segment | Elevation of privilege | `fs_resolve_allowed` returns `None`. Caller emits `_Denied("fs.scan_csv")`. No worker request. |
| Symlink out of the root | Elevation of privilege | `canonicalize` then `allows_path` on the canonical path. Roots are canonicalized at cap parse. |
| `scan_csv` glob or cloud credentials | Information disclosure | Worker passes the canonical path and `glob=False`. No `storage_options`. No `credential_provider`. |
| Null or naive timestamp turned into a Vox value | Tampering | Worker `py-plan:` first. Host decoder repeats the refusal and does not construct `Option` or `DateTime`. |
| Op name outside the allowlist | Tampering | `HANDLERS` lookup. Unknown name is `py-plan: unknown op`. Array and frame dicts stay separate so `mean` in one family cannot run the other engine. |
| Plan JSON built from user text | Injection | The host builds the JSON from the closed op set. The worker does not `eval` a user string. |
| Fetch larger than `[python] max_fetch_mb` | Denial of service | Existing cap. Default 256 from `default_max_fetch_mb` [VERIFIED: crates/vox-package-types/src/manifest.rs:74-94]. Frame collect uses the same `fetch_cap_bytes` path as arrays. |

## Sources

### Primary (HIGH confidence)

- `crates/vox-compiler/src/eval/array_plan.rs` — `ArrayPlan` stores steps and does not call the worker. Read this session.
- `crates/vox-compiler/src/eval/python.rs:140-174` — `execute_plan` is the one worker call.
- `crates/vox-compiler/src/eval/builtins.rs:134-165` and `:2628-2827` — `fs_resolve_allowed`, `materialize_to_list`, `plan_params`.
- `crates/vox-compiler/src/eval/caps.rs:16-28`, `:91-107`, `:327-342` — `GATED`, `fs:ro=` parse, `allows_path`.
- `crates/vox-compiler/src/hir/nodes/stmt_expr.rs:111-124` — `HirType` variants.
- `crates/vox-language-surface/src/lib.rs:186-241` and `:412-424` — `LEXER_KEYWORDS` length 54.
- `contracts/python/ecosystem-support.yaml`, `ecosystem-support.schema.json`, `plan-ir.v1.schema.json` — array allowlist and the frozen name enum.
- `crates/vox-compiler/build.rs:180-211` — array emitter panics unless module is `std.array` and engine is `numpy`.
- `crates/vox-python-bridge/worker/vox_bridge/__main__.py` and `array_ops.py` — dispatch and `py-plan:` unknown op.
- `crates/vox-python-bridge/src/frame.rs:1-2` — stdio frames, not dataframes.
- `crates/vox-python-bridge/src/array_numeric.rs:18-79` — array decoder limits.
- `docs/src/architecture/vox-python-interop-design-2026-11-01.md` §4.2–§4.6 and §6 — type table, sketch, allowlist gate, host path check.
- `docs/src/architecture/wire-format-v1-ssot.md:109-113` — Decimal, Date, DateTime, Duration encodings.
- `https://pypi.org/pypi/polars/json` — `info.version` `2.1.0`, fetched 2026-11-03.

### Secondary (MEDIUM confidence)

- `https://docs.pola.rs/api/python/stable/reference/api/polars.scan_csv.html` — `glob=True` default, `empty_string_is_null=True`, `credential_provider='auto'`.
- `https://docs.pola.rs/api/python/stable/reference/lazyframe/api/polars.LazyFrame.sort.html` — `nulls_last: bool = False`.
- `https://docs.pola.rs/api/python/stable/reference/lazyframe/api/polars.LazyFrame.group_by.html` — `maintain_order=False`, `agg(pl.col("b").sum())`.
- `https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.alias.html`
- `https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.eq.html`
- `https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.ne.html`
- `https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.gt.html`
- `https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.ge.html`
- `https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.lt.html`
- `https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.le.html`
- `https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.and_.html`
- `https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.or_.html`
- `https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.dt.replace_time_zone.html`
- `https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.count.html` — return type `UInt32`.
- `https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.mean.html`

Context7 was `needsAuth` and was not used. The research-plan seam had selected it for the API question. The pages above are the fallback.

### Tertiary (LOW confidence)

- Tavily result claiming `polars 1.43.2`. Contradicted by the PyPI JSON. Do not pin 1.43.2.
- Package-legitimacy seam: `SUS`, reasons `too-new` and `unknown-downloads`, `publishedAt` `2026-10-25T17:58:26.863145Z` (matches the wheel upload time).
- WebFetch provider tier from `classify-confidence` is `LOW`. The Polars pages are still the official API docs. The pin itself is the registry JSON, not the WebFetch confidence tier.

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH for the in-repo bridge, MEDIUM for the polars pin (registry JSON read; install not performed; seam says SUS).
- Architecture: HIGH. The plan, the single `execute_plan`, the path helper, and the array allowlist were read this session.
- Pitfalls: MEDIUM. Sort, alias, count, glob, and the parser gap are sourced. `min`/`max` method spelling and the exact Arrow Rust type names are assumed.

**Research date:** 2026-11-03
**Valid until:** 2026-11-10 for the `polars==2.1.0` pin. Re-check PyPI if the checkpoint is later than that. The in-repo architecture holds until `execute_plan` or the array allowlist changes.
