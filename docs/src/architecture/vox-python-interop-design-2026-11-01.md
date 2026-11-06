---
title: "Vox ↔ Python Bridge Design — 2026-11-01"
description: "Design for calling numpy, scipy, polars/pandas, matplotlib, scikit-learn and PyTorch from pure .vox code via an out-of-process Python bridge: Vox-native std modules that build lazy plans, a gated escape hatch, and DB/backend interop through existing schema and OpenAPI surfaces — without making Vox Python-shaped."
category: "Architecture SSOTs"
status: "roadmap"
topic: "compiler-frontend"
domain: "language"
authorship: "llm-assisted"
authorship_model: "claude-opus"
training_eligible: false
code_anchors:
  - crates/vox-compiler/src/hir/nodes/stmt_expr.rs#HirType
  - crates/vox-compiler/src/parser/descent/decl/head_import.rs
  - crates/vox-compiler/src/parser/descent/decl/head_fn.rs
  - crates/vox-ast/src/decl/types.rs#ImportPathKind
  - crates/vox-ast/src/decl/effect.rs#EffectAnnotation
  - crates/vox-compiler/src/typeck/effect_check.rs
  - crates/vox-compiler/src/typeck/determinism_lint.rs
  - crates/vox-compiler/src/eval/caps.rs
  - crates/vox-compiler/src/feature_matrix.rs
  - crates/vox-foundation/src/protocol.rs#DispatchRequest
  - crates/vox-skill-runtime/src/runtime.rs
  - crates/vox-codegen/src/syntax_k.rs
  - crates/vox-codegen-ts/src/openapi_emit.rs
  - crates/vox-cli/src/commands/db/introspect.rs
---

# Vox ↔ Python Bridge Design

<!-- kb:summary -->
This design lets `.vox` programs use numpy, scipy, polars, pandas, matplotlib, scikit-learn and
PyTorch without Vox becoming Python-shaped and without a large unverified surface. Python runs in an
out-of-process worker called the Python bridge, and Vox only holds typed opaque handles to objects
inside it. Common work goes through the Vox-native modules `std.array`, `std.frame`, `std.plot` and
`std.tensor`. These build pure lazy plans that cost one round-trip per materialization, executed on
pinned engines. Everything else goes through `import py`, an escape hatch that is gated by an
effect, typed from shipped stubs, and counted. Interop with existing databases and Python backends
reuses `vox db introspect` and the OpenAPI emit, not ORMs.
<!-- /kb:summary -->

> [!IMPORTANT]
> Status: **roadmap**. Nothing here is implemented. Proposed syntax is shown in `text` fences
> because the doc lint compiles `vox` fences. Codebase facts are cited `file:line` as of
> 2026-11-01.

> [!NOTE]
> Naming: **bridge**, not "spoke". In this repo, *spoke* means a MENS model spoke
> ([hub-and-spoke SSOT](voxmens-hub-and-spoke-ssot-research-2026-06-18.md)). A MENS lane trained
> on the bridge's Python code is possible later (§12), but it is not part of this design.

## 1. Decisions

| # | Decision | Rationale |
|---|---|---|
| D1 | **Out-of-process worker.** In-process PyO3 is not planned | Crash and OOM isolation; small boundary; journaled activities; no GIL in the host |
| D2 | **Two tiers.** Tier 1 is Vox-native std modules; Tier 2 is a gated escape hatch. **Never** raw Python strings, `eval`, or dynamic attribute access | Agents write only `.vox`; closed vocabulary for the common path; long-tail reach |
| D3 | **Thin, standards-named Tier 1 APIs on pinned engines:** numpy, polars, matplotlib, torch. No user-visible backend switch | One semantics per family, so conformance tests don't multiply. Polars matches Vox `Option` semantics (real nulls); pandas mixes NaN and None |
| D4 | **One new crate**, `vox-python-bridge`, plus a worker package in `crates/vox-python-bridge/worker/` (the `.py` files there are user-approved, §11) | Ponytail: no separate types crate, no trait with one implementation |
| D5 | **Python→Vox callbacks are deferred.** Tier 2 files are not training-eligible, except a curated set of examples | YAGNI; stops the model learning a Python-in-Vox dialect while still teaching it the escape hatch |
| D6 | **DB and backend interop happen at the schema and wire level** (`vox db introspect`, `vox db verify`, OpenAPI 3.1). No Drizzle or SQLAlchemy through the bridge | ORMs are language-local code generators; the database schema and the HTTP contract are the real interop surfaces, and both already exist |

**Non-goals:**
- authoring torch training in Vox (Candle stays native, [ADR 003](../adr/003-native-training-over-python.md));
- sharing GPU tensors with the host;
- a Python type-system reimplementation in Rust;
- a backend-swap knob.

**ADR 003 invariant:** when `[python]` is absent from `Vox.toml`, every bridge path is disabled. The
toolchain, default CI and Mens training stay Python-free.

## 2. Prior art

| System | Lesson |
|---|---|
| Mojo (Python superset), Swift `PythonObject` (`@dynamicMemberLookup`) | **Avoid:** syntax convergence and whole-API unverified surface |
| Ray `ObjectRef` | **Adopt:** data stays remote; handles; restart invalidates handles |
| Polars lazy | **Adopt:** pure plan building, one execution per `collect` |
| Array API standard 2024.12 (numpy 2.x implements it in its main namespace) | **Adopt** its names and parameter semantics (e.g. `correction=` rather than `ddof=`) as the `std.array` vocabulary |
| Arrow IPC / PyCapsule (replaces the `__dataframe__` protocol, deprecated in pandas 3 and Polars) | **Adopt** as the only data encoding |
| Vox React interop ([subspec §6](external-frontend-interop-phase5-component-interop-subspec-2026.md)) | **Reuse:** opaque by default; dev-time sidecar (`vox import-types` → `*.vox.extern.json`); the `import react X from "…"` AST shape |

## 3. Architecture

```vox-diagram
graph {
    direction: right
    container host: "Vox host (vox run / compiled app)" {
        node src: ".vox"
        node tc: "typeck: std sigs + py bindings + effects"
        node ev: "interpreter"
        node pb: "plan builder (pure)"
        node br: "vox-python-bridge: spawn, frames, handles, env"
        src -> tc
        tc -> ev
        ev -> pb
        pb -> br: "materialize"
    }
    container worker: "worker (uv venv, optional sandbox via vox-skill-runtime)" {
        node w: "vox_bridge runtime"
        node a: "array → numpy"
        node f: "frame → polars lazy"
        node p: "plot → matplotlib Agg"
        node t: "tensor → torch.export"
        node x: "Tier 2 bound calls"
        node h: "handle table (id, generation)"
        w -> a
        w -> f
        w -> p
        w -> t
        w -> x
        w -> h
    }
    // Bidirectional channel, drawn as request and reply.
    host.br -> worker.w: "stdio: length-prefixed JSON header + Arrow IPC body"
    worker.w -> host.br: "stdio reply (same framing)"
}
```

| Unit | Layer | Owns | Depends on |
|---|---|---|---|
| `vox-python-bridge` (new) | **L2** (domain lib; [layers.toml](layers.toml) header :5-11) | Plan IR, `HandleRef`, framing, worker spawn and lifecycle, `uv` env sync, binding-sidecar reader, allowlist loader | `vox-foundation` (`DispatchRequest`, `protocol.rs:192`), `vox-skill-runtime` (L2, sandbox: `detect_runtime`), tokio, serde, `arrow-ipc` + `arrow-array` (new workspace deps, lock change approved) |
| `vox-compiler` (L3, [layers.toml:200](layers.toml)) | changes | `import py` grammar, effects, std signatures, interpreter builtins calling the bridge | + `vox-python-bridge` (L3→L2 edge) |
| `vox-cli` (L5) | changes | `vox py sync \| bind \| doctor` | `vox-python-bridge` |
| worker `vox_bridge` | asset | Plan interpreter, handle table, op handlers, Tier 2 dispatch, bind generator | numpy ≥ 2, polars ≥ 1, pyarrow; optional matplotlib, torch |

**Why no trait.** vox-compiler already depends on tokio and reqwest (`Cargo.toml:40-44`) and
already spawns processes (`eval/builtins.rs`, `eval/shell_stdlib.rs`). A trait with one
implementation buys nothing. Tests use a **fake worker**: a Rust test binary that speaks the
framing, selected through the bridge's spawn command.

**Sandboxing** goes through the `vox-skill-runtime` SSOT (L2, [layers.toml:177](layers.toml)),
not the L4 container plugin directly.

Required governance rows:
- a `[crates]` row in [`layers.toml`](layers.toml);
- a [`where-things-live.md`](where-things-live.md) row;
- edges in `contracts/ci/crate-edges.allow.v1.json`;
- the `vox-compiler` fan-in is unaffected (`max_dependents = 21` counts dependents).

## 4. Tier 1 std modules

### 4.1 Semantics (normative)

1. **Plan construction is pure:** no I/O and no effect. Only **materializers** cross the boundary.
   They are `to_list`, `item`, `shape`, `collect`, `save`, `run` and `load_*`, all return
   `Result[T]`, and all require `uses python`.
2. **Plans are immutable values.** They are never cached results, and materializing twice
   re-executes the plan. A `HandleRef` captured by a plan is a **snapshot**: the worker
   copies on capture whenever the object is reachable from a Tier 2 call. Tier 2 can therefore
   never mutate data under a live plan.
3. **Generation check.** `execute_plan` rejects any handle whose generation differs from the current
   worker generation, with `py: stale handle (worker restarted)`.
4. **No implicit cross-kind promotion** (int↔float); this is unspecified by the Array API. It is a
   type error, fixed with an explicit `.astype(f64)`.
5. **Ambiguous parameters are required, not defaulted.** `std(correction=…)` is required because
   the libraries disagree on the default (numpy 0, torch/pandas 1). `sort(nulls=first|last)` is
   required because they also disagree on NaN and null placement.
6. **Naming:** Array API names for arrays and Polars expression names for frames, in snake_case.
   Python-ish vocabulary (`astype`, `axis=`) is the accepted price of free documentation and
   identity lowering. It is not syntax: no new keywords, no comprehensions, no indexing sugar
   beyond existing Vox.

### 4.2 Types (no new `HirType` variant)

These are std-defined `Named`/`Generic` types ([`stmt_expr.rs:111`](../../../crates/vox-compiler/src/hir/nodes/stmt_expr.rs)):

| Type | Meaning |
|---|---|
| `Array[D]`, with `D ∈ {bool, i32, i64, f32, f64}` (v1) | Lazy or worker-resident n-d array. Strings belong in `Frame`; complex, f16 and sparse are deferred |
| `Frame`, `Expr` | Lazy table and column expression (polars `LazyFrame` / `Expr`) |
| `Records` | Fetched, read-only columnar data on the host (Arrow). Not named `Table`, because `table` is the DB declaration keyword (`examples/golden/crud_api.vox:14`) |
| `Plot` | Pure figure spec; rendered in the worker by `save` |
| `Model` | Worker-resident `torch.export` program |
| `PyObject` | Tier 2 opaque handle: **no field or method access** |

Typed shapes (`Array[f32, Shape]`) are reserved syntax only.

### 4.3 Data mapping (normative; JSON is used only for control)

| Arrow / dtype | Vox | Rule |
|---|---|---|
| bool | `bool` | |
| int8–int64, uint8–uint32 | `int` (i64; interpreter `Value::Int(i64)`) | |
| uint64 | `int` | Runtime error if > i64::MAX |
| float32/float64 | `float` (f64) | NaN and ±Inf preserved, because the encoding is binary |
| utf8 / large_utf8 / dictionary<utf8> | `str` | |
| binary | `bytes` | |
| decimal128 | `Decimal` | Same domain as [wire-format v1 §3.1](wire-format-v1-ssot.md) |
| date32 | `Date` | |
| timestamp[tz] | `DateTime` | Normalized to UTC (wire v1 §3.1) |
| timestamp (naive) | **error** | Fix-it: `.replace_time_zone("UTC")` |
| duration | `Duration` | |
| nullable column of T | `Option[T]` | Wire v1 §4 (absent, never null) applies if re-serialized |
| list<T> | `list[T]` | |
| struct, map, union, complex | **error** in v1 | Use Tier 2, or select fields in the plan |

`to_list` is only valid on 1-D arrays; n-d arrays use `shape()` plus `reshape([-1])`. Every
fetch is capped by `[python] max_fetch_mb` (default 256) and fails with `py: fetch exceeds cap`
rather than running out of memory.

### 4.4 Sketches

```text
// PROPOSED, not compilable yet: std.array + std.frame + std.plot.
import std.array
import std.frame
import std.plot

@uses(python)
fn zscore(xs: list[float]) to Result[list[float]] {
    let a = array.from_list(xs).astype(f64)
    return ((a - a.mean()) / a.std(correction=0)).to_list()     // one round-trip
}

@uses(python, fs)
fn revenue(csv_path: str, out_png: str) to Result[int] {
    let monthly = frame.scan_csv(csv_path)                       // path checked host-side (§6)
        .filter(frame.col("region").eq("west"))
        .group_by(["month"])
        .agg([frame.col("amount").sum().alias("revenue")])
        .sort("month", nulls=last)
    plot.line(monthly).x("month").y("revenue").title("West").save(out_png)?   // data never leaves worker
    let rows = monthly.collect()?
    return Ok(rows.num_rows())
}

@uses(python, fs, gpu_compute)
activity classify(pixels: list[float]) to Result[list[float]] {
    let model = tensor.load_exported("models/classifier.pt2")?
    let x = array.from_list(pixels).astype(f32).reshape([1, 3, 224, 224])
    let out = model.run([x])?                                    // Array in/out; DLPack inside worker
    return out[0].softmax(axis=1).reshape([-1]).to_list()
}
```

### 4.5 v1 vocabulary (the allowlist SSOT is `contracts/python/ecosystem-support.yaml`)

| Module | v1 size | Engine |
|---|---|---|
| `std.array` | ≈ 50: creation, elementwise, reductions, `where`, sort, slicing, `linalg.{matmul,solve,inv,norm}`, `fft.{fft,ifft}` | numpy |
| `std.frame` | ≈ 30: `scan_csv/parquet`, `from_records`, `select`, `with_columns`, `filter`, `group_by/agg`, `join`, `sort`, `head`, `unique`, `col`, `lit`, aggregations, `collect` | polars lazy |
| `std.plot` | ≈ 10: `line`, `scatter`, `bar`, `hist`, `x`, `y`, `title`, `legend`, `save` | matplotlib Agg |
| `std.tensor` | ≈ 4: `load_exported`, `Model.run`, `to_device`, `device` | `torch.export.load` only. Pickle-bearing archives and `torch.load` are denied. Native Candle stays the path for Vox-owned models |

**Allowlist parity gate** (`vox ci python-bridge-parity`): every allowlisted op must have:
- a typeck signature;
- a worker handler;
- a conformance test.

Any orphan fails CI. This is the paper-code guard for Tier 1 (§10).

### 4.6 Plan IR

The schema is `contracts/python/plan-ir.v1.schema.json`. A plan is a DAG of
`Source | Op{family, name, args} | Materialize`; `Op.name` must be allowlisted, and the worker
rejects unknown ops. Node sharing comes free from builder handles, so there is no hash-consing
(plans are < 200 nodes). `x-vox-version` follows the contracts convention.

## 5. Tier 2: gated escape hatch

```text
// PROPOSED, not compilable yet.
import py sig from "scipy.signal"                 // same AST shape as `import react X from "…"`

@uses(python_unverified)
fn psd(samples: list[float], fs_hz: float) to Result[Records] {
    let (freq, power) = sig.welch(array.from_list(samples).astype(f64), fs=fs_hz, nperseg=256)?
    return frame.from_arrays(["freq", "power"], [freq, power]).collect()
}
```

- **Grammar.** Generalize `try_parse_react_component_import`
  ([`head_import.rs:89+`](../../../crates/vox-compiler/src/parser/descent/decl/head_import.rs))
  into one foreign-import helper, adding a new
  `ImportPathKind::PyModule { module_specifier, binding }` next to `ReactComponent`
  ([`types.rs:41`](../../../crates/vox-ast/src/decl/types.rs)). No new lexer keyword.
- **Names.** Only names listed in the binding sidecar resolve. Class methods are exposed as
  snake_case free functions (`sig.filter_apply`, not `Filter_apply`).
- **Kwargs** are Vox named args ([`expr.rs:18`](../../../crates/vox-ast/src/expr.rs)), checked by
  name and required-ness. `*args`/`**kwargs` parameters cannot be called.
- **Bindings.** `vox py bind` runs in the locked env and reads the library's **shipped**
  annotations (inline hints, `py.typed`, `pandas-stubs`, torch `.pyi`, typeshed); automatic
  stubgen output for pandas and torch is mostly `Any`, so it is not used. The output is a
  `*.vox.pybind.json` sidecar carrying the `uv.lock` hash. **The compiler rejects a sidecar whose
  hash differs from the current lock.**
- **Type mapping.** §4.3 scalars; `NDArray`/`torch.Tensor` → `Array[D]` (dtype from the
  annotation, else a runtime-checked `f64`); pandas/polars DataFrame → `Frame` (pandas is converted
  through Arrow PyCapsule); everything else → `PyObject`. For overloads, the first fully-mappable one
  wins and the dropped ones are recorded.
- **Curated, verified Tier 2:** sidecars and goldens ship and are tested in the Python CI job.
  - **scipy:** `signal`, `stats`, `optimize` (no callbacks), `interpolate`, `ndimage`.
  - **scikit-learn:** estimators as `PyObject`, `fit`/`predict`/`transform` on `Array`.
  - **pillow:** image load → `Array[i32]` (u8 widened).
  - **pandas:** `read_excel` and legacy I/O → `Frame`.
  
  Other packages work through uncurated `vox py bind`.
- **Denied** in `ecosystem-support.yaml`, the same pattern as
  [`contracts/rust/ecosystem-support.yaml`](../../../contracts/rust/ecosystem-support.yaml):
  - `pickle`, `marshal` and `shelve`;
  - `torch.load`;
  - `os`/`subprocess` process APIs;
  - modules that expose `eval`/`exec`.
- **Counting and promotion:**
  - **Lint:** `vox/python/unverified-call` (warn).
  - **Telemetry:** a `python.escape_call` event, registered in `contracts/telemetry/events.v1.yaml`.
  - **Promotion:** recurring escape calls become Tier 1 candidates when they fit the module budget.

## 6. Effects, capabilities, determinism

| Change | Anchor |
|---|---|
| **Fix first:** `@uses(...)` silently drops unknown names, and `mcp(x)` because `from_keyword` has no `mcp` arm. Make both `@uses` and `uses` go through one helper that **errors on unknown effects** | [`head_fn.rs:668-698`](../../../crates/vox-compiler/src/parser/descent/decl/head_fn.rs), [`head.rs:517-560`](../../../crates/vox-compiler/src/parser/descent/decl/head.rs) |
| Add two flat effects, `python` and `python_unverified`, rather than a parameterized form: less parser surface, nothing to drop | `effect.rs:8,41-59`; `HirEffectKind` (`hir/nodes/effect.rs:9`); `HirCapability` (`hir/nodes/decl.rs:786`) |
| Materializers → `Python`; `py` bindings → `PythonUnverified` | `stdlib_module_capability`, `effect_check.rs:586-600` |
| Interpreter: `py` added to `GATED`; `--caps py` / `py:unverified` | `caps.rs:16`, `parse` at `:65` |
| **Determinism by capability, not by name:** a `workflow` whose inferred capabilities include `Python*` is an error; Python is legal only in `activity`. `NON_DETERMINISTIC_CALLS` is name-based and cannot see user aliases | `determinism_lint.rs:31` |
| **Seeds:** `CapabilitySet.random_seed` (`caps.rs:36-39`) is passed to the worker and seeds `random`, numpy and torch. The activity journal records `{python, numpy, polars, torch}` versions and the lock hash | ADR 019 |
| **Files:** paths given to `scan_*`, `save` and `load_*` are canonicalized and checked **host-side** against `fs_ro`/`fs_rw` before the plan is sent. The worker receives only approved absolute paths | `caps.rs` `allows_path` (`:306`) |
| Capability registry row for `python` / `python_unverified` | `contracts/capability/capability-registry.yaml` |

**Trust boundary.** In-process Python cannot be forced to honour capabilities. The host-side path
checks and the allowlist are the language-level guarantees. Real confinement would come from running
the worker through `vox_skill_runtime::detect_runtime` (container or OS sandbox) in CI and prod.
`SkillRuntime` is a one-shot `run(RunOpts) -> RunOutcome` API, not a long-lived stdio host, so this
phase does not put the worker under it. `vox py doctor` reports sandbox availability as unsandboxed
and prints an explicit warning. A sandboxed long-lived worker is deferred (see §12).

## 7. Runtime

- **Transport.** The worker's stdio carries length-prefixed frames: `u32` header length, then a
  JSON header (`DispatchRequest` from `vox-foundation`), then an optional `u64` length and an
  **Arrow IPC stream body**. Scalars and lists are one-column Arrow too, so there are no JSON
  numerics. Before importing anything, the worker `dup`s fd 1 for RPC and points `sys.stdout` and fd
  1 at stderr, so library `print` and C-extension output cannot corrupt the stream. This works the
  same on Windows. There is no shared memory and no temp files in v1.
  *ponytail: pipe copy costs ~1–2 GB/s; add mmap via the existing `memmap2` workspace dep only if a
  benchmark shows fetch-bound workloads.*
- **Lifecycle.** One worker per Vox process, spawned lazily on the first materialization. Its
  generation is bumped on crash, OOM or timeout. Requests are serialized FIFO, so several Vox tasks
  share one GIL. *ponytail: a per-actor or pooled worker comes later, if contention shows up in
  benchmarks.*
- **Handles.** `HandleRef {generation, id, kind, dtype}`. Drops are batched into the next request.
  When the worker's stdin closes, the worker exits and frees everything.
- **Timeouts and cancellation.** Each materialization has a deadline: the activity timeout, or the
  `[python] timeout_s` default of 300. On expiry the host sends SIGINT (CTRL_BREAK on Windows), then
  kills the worker after 5 s grace and bumps the generation. C extensions are not safely
  interruptible otherwise.
- **Errors.** `Result` errors are strings, so they use stable prefixes: `py:<ExcType>: msg
  [trace:<8 hex>]`, `py-plan:` for validation errors, and `py-env:` for environment errors. The full
  traceback goes to telemetry and the journal.
- **Environment.** `[python]` in `Vox.toml`
  ([`manifest.rs:10`](../../../crates/vox-package-types/src/manifest.rs)):

  ```toml
  [python]
  version = "3.12"
  packages = ["numpy>=2.1", "polars>=1.9", "pyarrow>=17"]
  extras = ["matplotlib>=3.9", "torch>=2.5", "scipy>=1.14", "scikit-learn>=1.5"]
  max_fetch_mb = 256
  timeout_s = 300
  ```

  - `vox py sync` writes `.vox/python/pyproject.toml` and runs `uv lock` and `uv sync --frozen`.
    `uv.lock` is universal across platforms, so one lock covers macOS-arm64, Linux and Windows
    wheels. Its hash is recorded in `vox.lock` (struct at
    [`lockfile.rs:12`](../../../crates/vox-package-types/src/lockfile.rs)).
  - `vox py sync --offline` uses the uv cache.
  - `uv` is a discovered external tool; `vox py doctor` reports when it is missing.

**Cost model.** With *m* materializations, *c* Tier 2 calls and *b* bytes fetched, Tier 1 costs
O(m) round-trips at about 50–200 µs each, and transfer is O(b) only on fetch. Tier 2 costs O(c)
round-trips. Compute runs at native library speed. Cold start is about 0.1 s for numpy and 1–3 s
with torch; matplotlib's first font-cache build (several seconds) is paid once, by `vox py sync`.
**Acceptance:** ≤ 1.25× the wall-clock time of the equivalent Python script on compute-bound
goldens after warm start.

## 8. Database and backend interop (D6)

| Scenario | Mechanism | New work |
|---|---|---|
| A Vox server consumed by a Python service (FastAPI, Django, scripts) | Existing **OpenAPI 3.1** emit (`vox-codegen-ts/src/openapi_emit.rs`; `--target server`, `cli_args.rs:53`), consumed by `openapi-python-client` and similar tools | A how-to doc only |
| A Vox app sharing a DB whose schema is owned by **Drizzle, SQLAlchemy or Alembic** | Existing `vox db introspect` (`commands/db/introspect.rs:12`, sqlite/postgres/mysql via `vox-sql/src/introspect.rs`) and `vox db verify` drift check (`commands/db/verify.rs:63`) | `vox db introspect --emit vox` (generate `table` decls), plus `[db] schema_owner = "external"`, which makes `vox db migrate` (`commands/db/local.rs:144`) refuse to apply |
| A Vox app calling an existing Python HTTP API | Import that API's OpenAPI document as a typed Vox client | **Deferred:** a separate design (reverse of the OpenAPI emit) |
| Calling SQLAlchemy or Drizzle *as libraries* | **Not supported.** It duplicates Vox `db`, adds an ORM-shaped unverified surface, and a Drizzle call would need Node | — |

## 9. K complexity and LLM-target impact

The existing machinery is `K_est(x) = min_z |z(x)|` and NCD in
[`crates/vox-codegen/src/syntax_k.rs`](../../../crates/vox-codegen/src/syntax_k.rs) (`:277`),
gated by `run_k_complexity_budget` (`contracts/eval/complexity-budget.v1.json`) and
`run_source_token_budget`.

| Metric | Gate |
|---|---|
| **K1 program K** | Four goldens, `examples/golden/py_{array_stats,frame_report,tensor_infer,escape_psd}.vox`, each with a ladder entry (`contracts/pipeline/canonical-ladder.v1.yaml`) and a budget row. The ratio against a Python twin (`crates/vox-python-bridge/worker/twins/`) is **reported, not gated**, so the metric doesn't reward imitating Python |
| **K2 surface K** | Compressed size of each module's signature manifest, plus the counts in §4.5, as hard ceilings. Growth only through promotion review |
| **K3 grammar delta** | 0 new `LEXER_KEYWORDS` ([`vox-language-surface/src/lib.rs:186`](../../../crates/vox-language-surface/src/lib.rs)); `py` is a contextual import identifier; two effect names |
| **K4 interop density** | A **new HIR-level** `py_tier1_calls` / `py_tier2_calls` count in `support_metrics.llm_surface`. The existing `interop_nodes` counts WebIR React nodes (`tests/it/golden_vox_examples_test.rs:84`) and never sees Python. Observe first, then enforce a Tier 2 share |

**Corpus.** Tier 1 goldens are training-eligible. Tier 2 is excluded except the curated
`py_escape_*` examples, which teach the escape-hatch form itself.

## 10. Testing and paper-code gates

| Layer | Python? |
|---|---|
| Plan IR serde and schema; framing and handle generation against the fake worker | No |
| Compiler goldens: `import py`, unknown-effect error, a `workflow` using Python rejected, `PyObject.field` rejected, mixed-kind dtype error, stale-sidecar hash error | No |
| Worker conformance per allowlisted op against reference values, including NaN, null, empty-input and ordering cases | Yes, in an opt-in CI job via uv on PRs touching the bridge or `contracts/python/**`, and nightly |
| End-to-end goldens through the real worker under `vox run`, with asserted output | Yes, same job |

Paper-code gates (per [PAPER-GUARD](anti-laziness-hallucination-and-paper-code-sota-2026.md) §2.1):
- every phase exits on an end-to-end `vox run` golden through the **real** worker, never only on a
  fake or unit test;
- the allowlist parity gate (§4.5);
- `feature_matrix.rs` cells flip to `Implemented` only together with a running ladder entry. Note
  that `decorator_row` has a `_ => unverified()` catch-all (`feature_matrix.rs:597`), so any new
  decorator-shaped feature must be listed explicitly. TypeScript cells are
  `Unsupported(E_PY_TS)`. RustAxum/Tauri are out of scope for this milestone.

## 11. Approved exemptions and dependency changes

- **`.py` exemption (user-approved 2026-11-01)** for `crates/vox-python-bridge/worker/**` only
  (runtime, conformance tests, twins). Record it in AGENTS.md §VoxLang-First as a scoped
  exception.
- **New deps (user-approved):** `arrow-ipc` and `arrow-array` (minimal features). This is a lock
  change; both are pure Rust, so the build-toolchain invariant is unaffected.

## 12. Deferred

- Sandboxed long-lived worker. `vox_skill_runtime::SkillRuntime` cannot host the stdio worker
  without a new long-lived API, so phase 325 runs the worker unsandboxed and `vox py doctor`
  warns. Confinement stays deferred until that runtime can own a process for the worker's lifetime.
- Python→Vox callbacks (e.g. `scipy.optimize` objectives). The candidate approach is pure Vox
  functions compiled to Plan IR expressions.
- Typed shapes.
- Complex, f16 and sparse dtypes.
- Worker pool.
- mmap data plane.
- Compiled targets.
- Importing an OpenAPI document as a client.
- `transformers`/HF pipelines as curated Tier 2.
- A MENS lane for bridge maintenance.

## 13. Phasing (milestone v5.7, phases 324–330)

| Phase | Scope | Exit (end-to-end) |
|---|---|---|
| 324 | Effect parser fix (unknown → error, `mcp` preserved), `python*` effects and caps, determinism-by-capability | Compiler goldens green; `@uses(python_unverified)` survives into HIR |
| 325 | `vox-python-bridge` + worker skeleton, framing, handles, timeouts, `[python]`, `vox py sync/doctor`, sandbox via skill-runtime | `vox run` on a one-op golden through the real worker |
| 326 | `std.array` v1, conformance, allowlist parity gate | `py_array_stats` golden, K1/K2/K3 rows |
| 327 | `std.frame` (polars), `Records`, §4.3 mapping, host-side path caps | `py_frame_report` golden |
| 328 | `std.plot`, `std.tensor` (`torch.export`) | Plot and tensor goldens on CPU (MPS in a nightly) |
| 329 | Tier 2: `import py`, `vox py bind`, sidecar hash check, curated scipy/sklearn/pillow/pandas, lint, telemetry, K4 | `py_escape_psd` golden |
| 330 | DB/backend interop: `vox db introspect --emit vox`, `schema_owner = "external"`, OpenAPI→Python how-to | Introspect→verify round-trip test on sqlite |
