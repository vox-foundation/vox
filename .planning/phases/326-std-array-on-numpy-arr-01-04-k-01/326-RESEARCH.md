# Phase 326: `std.array` on numpy — Research

**Researched:** 2026-11-02
**Domain:** Vox `std.array` plan IR over the Phase 325 Python worker, numpy stats slice, Arrow IPC numeric mapping, allowlist parity gate
**Confidence:** HIGH for in-repo seams; MEDIUM for numpy/Array API mapping; LOW for Arrow IPC byte-compatibility until the golden runs

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** v1 `contracts/python/ecosystem-support.yaml` lists only the stats slice: `from_list`, `astype`, `add`, `sub`, `mul`, `div`, `mean`, `std`, `sort`, `to_list`, `item`, `shape`. `linalg`, `fft`, `where`, and slicing are absent from that file until a later phase adds their handlers. — **Reversibility:** reversible — adding a row later does not change the type of an existing op.
- **D-02:** `Array[f32]` and `Array[f64]` are plan types. `to_list` on either returns `list[float]` (host f64). The golden asserts those f64 values, including NaN and ±Inf. No host f32 list type. — **Reversibility:** costly — `to_list`'s return type is the caller-facing contract; widening it later changes every caller.
- **D-03:** `ARR-04` is a table-driven check over the yaml. A row counts only when it has a typeck signature, a worker handler, and an expected value for a concrete input. `examples/golden/py_array_stats.vox` is the one end-to-end materialization. A call inside `@test` with no expected value does not satisfy the row. — **Reversibility:** reversible — the table can grow without a new declaration form.
- Plan construction is pure. Materializers are `to_list`, `item`, `shape`. `mean` and `std` return plans.
- No new `HirType` variant. No new lexer keywords.
- This phase calls the Phase 325 worker. It does not spawn a second worker.
- Op bodies follow Array API names already required by the design (`correction=`, `nulls=`, `astype`). Do not rename them.

### Claude's Discretion
- User said the three tradeoffs did not have enough information to choose, and to make the recommended choices. D-01, D-02, and D-03 are those choices.
- Op bodies follow Array API names already required by the design (`correction=`, `nulls=`, `astype`). Do not rename them.

### Deferred Ideas (OUT OF SCOPE)
- `linalg.{matmul,solve,inv,norm}` and `fft.{fft,ifft}` — design §4.4's larger set. A later phase adds the yaml rows and the handlers together.
- `where` and slicing — same rule.
- A host f32 list type — rejected for v1 by D-02.
- One `@test fn` per op — rejected for v1 by D-03. The colocated-test warning proposal in `.planning/research/colocated-test-warnings/` is a separate effort and is not part of Phase 326.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ARR-01 | Every allowlisted `std.array` op typechecks and runs on numpy; the `py_array_stats` golden runs under `vox run` with asserted output. | Twelve yaml rows, `StdArrayNs` signatures, one `execute_plan` round-trip per materializer, golden `fn main` with `assert`. |
| ARR-02 | Mixed int/float arithmetic without `.astype` is a type error; `std` requires `correction=` and `sort` requires `nulls=`. | `check_binary_op` Array arm before host promotion; named-arg check the positional `check_arguments` does not do. |
| ARR-03 | NaN, ±Inf and i64 extremes round-trip through `to_list` unchanged; a uint64 value above i64::MAX is a runtime error. | Arrow IPC body, host `VoxValue::Float` (f64) and `VoxValue::Int` (i64). i64::MIN cannot be a Vox integer literal. uint64 is a worker-produced column, not a Vox literal. |
| ARR-04 | `vox ci python-bridge-parity` fails when any op in the yaml lacks a typeck signature, a worker handler, or a conformance test. | Same fail-closed shape as `CiCmd::RustEcosystemPolicy`, but the row is an op, not a crate family. Negative test feeds a synthetic incomplete row. |
| K-01 | Ladder entry, complexity-budget row, `LEXER_KEYWORDS` unchanged, K2 ceiling. | This phase adds `py_array_stats` only. Also add a `source-token-budget` row or that gate fails. K2 is a new ceiling, not the WebIR budget map. |
</phase_requirements>

## Summary

`std.array` is a closed plan language. The interpreter builds an immutable plan value for `from_list`, `astype`, the four arithmetic ops, `mean`, `std`, and `sort`. Nothing in that set talks to Python. `to_list`, `item`, and `shape` are the only calls that enter the Phase 325 worker, each as one `Bridge::call` whose JSON header is the plan and whose frame body is an Arrow IPC stream. `mean` and `std` stay inside the plan as zero-dimensional arrays so the z-score expression is one materialization.

The allowlist is a new contract, `contracts/python/ecosystem-support.yaml`, with the twelve D-01 names and no others. Copy the rust ecosystem *mechanism* (yaml + schema + a compiler test the `vox ci` command shells out to), not the crate-family fields. A row is closed only when it has a signature, a handler, and an expected value. Scanning the golden for a call does not close it.

No new workspace crate and no new crate-edge. `arrow-ipc` and `arrow-array` are third-party dependencies of `vox-python-bridge`, which Phase 325 creates. That crate is not in the tree today. This phase cannot run the golden until the 325 checklist below is actually landed.

**Primary recommendation:** Extend the existing std-namespace tables and the Phase 325 `Bridge::call` path. Generate the twelve signatures from the yaml the way `vox-compiler/build.rs` generates `rust_interop_policy.rs`. Keep plan values in the interpreter. Cross the worker only from the three materializers.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Dtype and required-kwarg checking | Compiler typeck (`vox-compiler`) | — | Mixed int/float and missing `correction=` / `nulls=` are compile errors (ARR-02). |
| Plan construction | Interpreter (`VoxValue` plan, no I/O) | — | Design §4.1: plans are pure. `mean` / `std` return plans. |
| One worker round-trip | Phase 325 `Bridge::call` | Worker `execute_plan` | This phase does not spawn a process. |
| Numeric payload | Arrow IPC in the frame body | numpy inside the existing worker | Design §4.3: JSON is control only. NaN, Inf, and i64 extremes are not JSON numbers. |
| Allowlist parity | `contracts/python/ecosystem-support.yaml` | `vox ci python-bridge-parity` | ARR-04 fails closed on a missing signature, handler, or expected value. |
| K1 / K2 | Canonical ladder + budget files | Signature-manifest ceiling | K1 is the golden. K2 is the compressed signature text, a different number. |

## Project Constraints (from .cursor/rules/ and AGENTS.md)

- VoxLang-first. No new `.py` / `.sh` / `.ps1` under `scripts/`. Worker Python is only the design §11 exemption: `crates/vox-python-bridge/worker/**`.
- No new workspace-crate edge and no `contracts/ci/crate-edges.allow.v1.json` exceptions entry. The `vox-compiler` → `vox-python-bridge` edge is Phase 325 GOV-01. If it is not landed, stop. Do not propose a second crate.
- No new `HirType` variant. `Array[D]` lowers through the existing `HirType::Generic`.
- No new `LEXER_KEYWORDS` entry. `array`, `astype`, `f32`, `f64`, `correction`, and `nulls` stay identifiers.
- New `pub fn` in `crates/*/src/**` needs a same-file test first (`skeleton/untested-pub-api`).
- Do not open a `turso::Connection`. Do not read `docs/src/archive/`. Do not edit `.planning/ROADMAP.md`, `REQUIREMENTS.md`, `PROJECT.md`, or `STATE.md`.
- Crypto stays in `vox-crypto`. This phase adds none. The uv.lock hash remains the Phase 325 blake3 write.
- Paper-code: the phase exits on `vox run` of the golden through the real worker. A unit test alone does not.
- Route `cargo` through PATH (the build broker shim). Do not call `~/.cargo/bin/cargo`.
- CLI: add a `contracts/operations/catalog.v1.yaml` row, then `vox ci operations-sync`. Do not hand-edit `contracts/cli/command-registry.yaml` or `*.generated.md`.

## Phase 325 Gate (must be true before the golden can run)

`crates/vox-python-bridge/` is **absent** (no files in the tree). The current compiler also does not yet carry the capability the materializers must check. Do not reimplement any of this inside Phase 326. Confirm each row against the landed code, not against the 325 plans.

| Prerequisite | Landed today? | Evidence |
|--------------|---------------|----------|
| Crate `vox-python-bridge`, worker package `vox_bridge`, `Bridge::new` / `call(method, params, body)` / `generation()` | No | No crate directory. Specified in `325-01-PLAN.md` and `325-CONTEXT.md` D-01..D-04. |
| Frame = `u32` header length + JSON header + `u64` body length + body; body opaque in 325 | No | `325-CONTEXT.md` D-02. Phase 326 is the first phase allowed to put Arrow in that body. |
| `[python]` on `VoxManifest`, `vox py sync`, uv.lock blake3 in `vox.lock` | No | `325-02-PLAN.md` Task 1–2. |
| One bridge instance created from `run.rs` for `python.version()` | No | `325-02-PLAN.md` Task 3. `std.array` materializers must use that instance. |
| `HirCapability::Python` and a lower arm for `EffectAnnotation::Python` | No | AST accepts the effect. HIR and the lower match do not. See quotes below. |
| `caps` token `py:allow` and `GATED` entry `"py"` | No | Tests were added; `GATED` and `parse` were not. |
| `stdlib_module_capability("python") => Python` | No | The match has no `python` arm. |
| Error prefixes `py:` / `py-plan:` / `py-env:` and `py: fetch exceeds cap` | No | Specified in 325 D-02 / D-04. Reuse them. Do not invent a fourth prefix. |
| Worker stdout dup so library `print` cannot corrupt the frame | No | `325-CONTEXT.md` D-03. numpy and pyarrow must stay on the dup'd RPC fd. |

### Quotes that show 324/325 are not finished in this tree

AST already accepts the effect (`crates/vox-ast/src/decl/effect.rs:35-38` and `:61-62`):

```rust
    /// Calls the Python bridge through the verified Tier 1 std surface.
    Python,
    /// Calls the Python bridge through the gated Tier 2 escape hatch.
    PythonUnverified,
    // ...
            "python" => Some(Self::Python),
            "python_unverified" => Some(Self::PythonUnverified),
```

`HirEffectKind` has no Python variant (`crates/vox-compiler/src/hir/nodes/effect.rs:9-29`). The lower match stops at `Nothing` and does not mention `Python` (`crates/vox-compiler/src/hir/lower/decl.rs:21-44`). That match is non-exhaustive against the AST enum. Phase 324 owns the new `HirCapability` variant and the lower arm. Phase 326 only *reads* `HirCapability::Python` when a materializer is typechecked.

`GATED` does not list `py` (`crates/vox-compiler/src/eval/caps.rs:16-18`):

```rust
pub const GATED: &[&str] = &[
    "fs", "io", "process", "env", "secrets", "http", "time", "agentos", "crypto",
];
```

`developer_default` copies `GATED` (`caps.rs:285-287`), so a default `vox run` does not allow `"py"` today. The tests at `caps.rs:374-378` already expect the opposite:

```rust
    fn py_allow_grants_py_but_not_unverified() {
        let c = CapabilitySet::parse("py:allow").unwrap();
        assert!(c.allows_namespace("py"));
        assert!(!c.allows_namespace("py.unverified"));
    }
```

`parse` has no `py` arm; unknown namespaces hit `_ => return Err(err("unknown namespace"))` (`caps.rs:141`). Those tests fail until Phase 324 lands. Do not "fix" them in 326 except to call `allows_namespace("py")` from the materializer once the arm exists.

`stdlib_module_capability` (`effect_check.rs:584-600`) maps `http`, `db`, `fs`, `env`, `time`, `process`, scrape/browser, and `repo`. It does not map `python` or `array`. The comment on that function says any method on the module inherits the capability. **Do not add `"array" => Python`.** That would make `array.from_list` an effect. Plan constructors and plan methods stay pure. Only the three materializers require `uses python`.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| numpy | `>=2.1` (PyPI latest inspected: **2.5.4**) | Engine behind the twelve ops | Design §4.5. `np.std(..., correction=)` exists since numpy 2.0. [CITED: https://numpy.org/doc/stable/reference/generated/numpy.std.html] [VERIFIED: PyPI JSON `info.version` = 2.5.4] |
| pyarrow | `>=17` (PyPI latest inspected: **26.0.0**) | Worker-side Arrow IPC stream encode/decode | Design §7 package list. Apache-2.0. [VERIFIED: PyPI JSON `info.version` = 26.0.0, license Apache-2.0] |
| `arrow-ipc` | **60.1.0** | Host IPC reader/writer | Design §11, user-approved. `cargo info`: Apache-2.0, rust-version 1.88, repo `apache/arrow-rs`. Default features are `[]` (no zstd/lz4). [VERIFIED: `cargo info arrow-ipc`] |
| `arrow-array` | **60.1.0** | Host typed arrays (`Float64Array`, `Int64Array`) | Design §11. Direct dep even though `arrow-ipc` already depends on it. [VERIFIED: `cargo info arrow-array`] |

Workspace `rust-version` is `1.96` (`Cargo.toml:31`), which satisfies arrow-rs `1.88`. Do not enable `arrow-ipc`'s `zstd` or `lz4` features.

Interpreter for the golden project is **CPython 3.12**, the design pin and the uv install already on this machine (`cpython-3.12.10` at `~/.local/share/uv/python/...`). `python3` on PATH is **3.15.2**. Do not probe numpy with that interpreter. `uv` is **0.13.6**.

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| Phase 325 `Bridge` | landed by 325 | The only process client | Every materializer |
| `vox_codegen::syntax_k::measure_syntax_k_event` | in tree | K2 compressed size | Signature-manifest ceiling |
| `serde_yaml` + `jsonschema` | already used by the rust parity test | Yaml/schema gate | `python_bridge_parity` test |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `arrow-ipc` 60 + pyarrow | A hand-rolled numeric buffer | Rejected. Design §4.3 and §11 name Arrow. JSON numbers cannot round-trip i64 or NaN. |
| `arrow-ipc` | `picoarrow`, `minarrow`, `pyo3-arrow` | Not the approved deps. Do not add them. |
| numpy `ddof=` as the Vox name | — | Forbidden. Vox name is `correction=`. Map it to numpy's `correction=` (numpy 2.0 alias of `ddof`). |

**Installation (on `vox-python-bridge`, after Phase 325 creates the crate):**

```toml
arrow-ipc = "60.1.0"
arrow-array = "60.1.0"
```

Golden project `[python]` packages, and no others in this phase:

```toml
[python]
version = "3.12"
packages = ["numpy>=2.1", "pyarrow>=17"]
```

Do not add polars, matplotlib, or torch here.

**Version verification:** `cargo info arrow-ipc` / `cargo info arrow-array` (2026-11-02, both 60.1.0). PyPI JSON for numpy 2.5.4 and pyarrow 26.0.0. Package-legitimacy seam: arrow crates `OK`; numpy and pyarrow `SUS` (heuristic: latest release date, missing download counts). They stay. See the audit.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `arrow-ipc` | crates.io | since 2022-11-14 | ~2.1M/wk | github.com/apache/arrow-rs | OK | Approved. Direct dep of `vox-python-bridge`. Default features only. |
| `arrow-array` | crates.io | since 2022-10-03 | ~2.4M/wk | github.com/apache/arrow-rs | OK | Approved. Direct dep. |
| `numpy` | PyPI | latest file 2026-10-10; project is decades old | seam returned null | numpy.org (seam repo field empty) | SUS | Design §4.5 locks it. Seam reasons are `too-new`, `unknown-downloads`, `no-repository` on the *latest release*, not a missing project. Planner adds a checkpoint that `uv lock` resolves the PyPI `numpy` project. Do not pick another array library. |
| `pyarrow` | PyPI | latest file 2026-10-09 | seam returned null | https://arrow.apache.org/ | SUS | Design §7 locks `pyarrow>=17`. Same heuristic. Checkpoint that the lock names `pyarrow` from PyPI. |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** `numpy`, `pyarrow` — seam heuristic only. The engine choice is already locked by the design. The checkpoint confirms the lockfile, it does not reopen D-01.

No npm packages. No postinstall scripts (`cargo info` features do not install host binaries; arrow-ipc default feature list is empty).

## Architecture Patterns

### System Architecture Diagram

```
Vox source
  array.from_list / astype / add / sub / mul / div / mean / std / sort
        │  pure interpreter plan (no Bridge::call)
        ▼
  VoxValue plan DAG
        │  only to_list | item | shape
        ▼
  typeck: materializer requires uses python
  runtime: caps.allows_namespace("py")
        │  one Bridge::call("execute_plan", plan JSON, Arrow IPC body)
        ▼
  Phase 325 worker (same process, same generation)
        │  allowlist check → numpy
        ▼
  Arrow IPC response body
        │  float32/float64 → VoxValue::Float (f64)
        │  int → VoxValue::Int (i64); uint64 > i64::MAX → py: error
        ▼
  Result[list[float]] | Result[float] | Result[list[int]] | Result[int] | Result[list[int]] for shape
```

`from_list` stores the host list in the plan. It does not send a frame. Sending at `from_list` would be a second round-trip and fails success criterion 1.

### Recommended Project Structure

```
contracts/python/ecosystem-support.yaml          # twelve ops, the SSOT
contracts/python/ecosystem-support.schema.json
contracts/python/plan-ir.v1.schema.json          # design §4.6; stats-slice nodes only
contracts/python/signature-manifest-budget.v1.json  # K2 ceiling for std.array
crates/vox-compiler/build.rs                     # generate the signature table (rust_interop_policy.rs pattern)
crates/vox-compiler/src/typeck/builtins.rs       # root binding `array` → StdArrayNs
crates/vox-compiler/src/builtin_registry.rs      # std_root_field_ty("array")
crates/vox-compiler/src/typeck/checker/expr_ops.rs  # Array arithmetic
crates/vox-compiler/src/eval/value.rs            # plan value, not a HirType
crates/vox-compiler/tests/python_bridge_parity_test.rs
crates/vox-python-bridge/                        # EXISTS ONLY AFTER 325
  Cargo.toml                                     # + arrow-ipc, arrow-array
  worker/vox_bridge/                             # lazy-import numpy inside array handlers
examples/golden/py_array_stats.vox
contracts/pipeline/canonical-ladder.v1.yaml      # one new fixture
contracts/eval/complexity-budget.v1.json         # one new key
contracts/eval/source-token-budget.v1.json       # one new key (or source-token-budget fails)
```

### Pattern 1: Yaml SSOT, generated signatures, fail-closed test

**What:** `crates/vox-compiler/build.rs` already reads `contracts/rust/ecosystem-support.yaml` and writes `rust_interop_policy.rs` (`build.rs:95-157`). `rust_ecosystem_support_parity_test.rs` re-reads the yaml and asserts it matches the generated classifier. `CiCmd::RustEcosystemPolicy` (`run_body.rs:472-492`) runs that test and returns `Err` on failure.

**When to use:** The python gate. Do not reuse `crate_family` / `decision` / `semantics_state`. Those fields describe crates. An array row needs `name`, `signature`, `handler`, and `expected`.

**Row shape to generate:**

```yaml
schema_version: 1
module: std.array
engine: numpy
ops:
  - name: std
    kind: plan
    receiver: Array[D]
    dtypes: [f32, f64]
    signature: "std(correction: int) -> Array[D]"
    required_named: [correction]
    handler: std
    input: { dtype: f64, values: [1.0, 3.0], correction: 0 }
    expected: { dtype: f64, values: [1.0] }
  - name: to_list
    kind: materializer
    signature: "to_list() -> Result[list[float]]"
    handler: to_list
    input: { dtype: f64, values: [1.0, 3.0] }
    expected: { host: list_f64, values: [1.0, 3.0] }
```

`kind: plan` ops: `from_list`, `astype`, `add`, `sub`, `mul`, `div`, `mean`, `std`, `sort`.
`kind: materializer` ops: `to_list`, `item`, `shape`.

A missing `signature`, `handler`, or `expected` is a test failure even if the yaml still parses. The negative test calls the checker with an in-memory row that drops one of those three fields and asserts `Err`. It must not delete the real yaml on disk. It must not treat a golden `@test` that merely calls the op as `expected`.

The checker joins three sets:

1. Generated signature table (build.rs output, consumed by typeck).
2. Handler names defined in the worker dispatch (a `HANDLERS` dict whose keys are the op names, functions `def <name>(...). Do not accept a comment that mentions the name. Anchor on a real `def` at column 0, or import the dict from the 3.12 venv).
3. `expected` present and non-null on the yaml row.

`vox ci python-bridge-parity` shells out the same way as rust ecosystem policy:

```text
cargo test -p vox-compiler --test python_bridge_parity
```

Register it as a new `CiCmd` variant next to `RustEcosystemPolicy` (`cmd_enums.rs:744-745`), a `run_body.rs` arm that returns `Err` on non-zero status (the comment at `run_body.rs:485` says a bare status check that doesn't return `Err` records a stale Pass), a catalog row `ci.python-bridge-parity` with `ref_cli_required: true`, and a `docs/src/reference/cli.md` line. Then `vox ci operations-sync`.

Also add `contracts/index.yaml` entries the way `rust-ecosystem-support` is indexed (`contracts/index.yaml:909-923`). `vox ci contracts-index` fails on a contract file that is not listed.

### Pattern 2: `Array[D]` without a new `HirType`

**What:** HIR already has generic application (`stmt_expr.rs:111-124`):

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

The typechecker `Ty` (`ty.rs:5-41`) has `List`, `Option`, `Result`, `Map`, `Set`, and `Named`. It has no `Array` and no general `App`. Add `Ty::Array(Box<Ty>)` and lower it with `HirType::Generic("Array".into(), vec![dtype.to_hir_type()])`. Do not add `HirType::Array`.

Dtype argument of `astype` is an identifier, not a value and not a keyword: `f32`, `f64`, `i32`, `i64`, `bool`. `f64` is not in `LEXER_KEYWORDS`.

`from_list(list[int])` → `Array[i64]`. `from_list(list[float])` → `Array[f64]`. `from_list(list[bool])` → `Array[bool]`. There is no host i32 or host f32 list. `i32` and `f32` appear only after `astype`.

`mean` and `std` are legal only on `Array[f32]` and `Array[f64]`, and they return `Array[D]` (a zero-dimensional plan of the same dtype). Rejecting them on integer arrays avoids the Array API's implementation-defined int→float promotion ([CITED: https://data-apis.org/array-api/latest/API_specification/generated/array_api.mean.html] "if the input array has an integer data type, the returned array must have the default real-valued floating-point data type"). The z-score sketch already `astype`s first.

`to_list` on `Array[f32]` or `Array[f64]` → `Result[list[float]]` (host f64). On `Array[i32]` or `Array[i64]` → `Result[list[int]]`. On `Array[bool]` → `Result[list[bool]]`. Rank is not in the type (typed shapes are reserved). n-d `to_list` is a runtime `py-plan:` error. v1 has no `reshape`, so the golden stays 1-d.

`item` → `Result[float]` or `Result[int]` or `Result[bool]` matching the host mapping. `shape` → `Result[list[int]]`.

### Pattern 3: Arithmetic and required names

`check_binary_op` promotes host int/float to `Ty::Float` (`expr_ops.rs:54-55`) and, for `Sub`/`Mul`/`Div`, returns `Ty::Error` with **no diagnostic** (`expr_ops.rs:65-74`). An Array arm has to run **before** that promotion, and it has to `diags.push` for all four ops. A silent `Ty::Error` does not satisfy ARR-02.

Rule: both operands `Array[D]` with the same `D` → `Array[D]`. `Array[i64]` op `Array[f64]` (and any other int-kind vs float-kind pair) is a type error whose text tells the user to call `.astype`. Same-dtype only, including `i32` vs `i64`. There is no promotion lattice in `Ty`, and §4.1 says the fix is `.astype`. Do not change host scalar `1 + 1.0`.

`check_arguments` zips parameters and arguments by position and reads `a.value` only (`expr_ops.rs:169-171`). It never looks at `HirArg.name` (`stmt_expr.rs:221-226`):

```rust
pub struct HirArg {
    /// Label for named args (`json=`); `None` for positional.
    pub name: Option<String>,
    /// Argument value.
    pub value: HirExpr,
}
```

`std` must reject a positional int and reject a missing name. The only accepted form is `correction=<int>`. `sort` must reject a missing name. The only accepted form is `nulls="first"` or `nulls="last"`. Do not default either one. Numpy's own default (`correction=0`, NaNs at the end) is exactly what the design refuses to inherit.

### Pattern 4: Where the name `array` comes from

`std.<field>` is `std_root_field_ty` (`builtin_registry.rs:529-552`). Add `"array" => Ty::Named("StdArrayNs")`. Mirror it in `std_namespace_short_name` (`expr_field.rs:209-227`) so `std.array.from_list` resolves.

`import std.array` lowers to `HirImport { module_path: ["std"], item: "array" }` (`hir/lower/mod.rs:130-137`). `register_hir_module` then binds that name as `Ty::ImportPlaceholder` **only when the name is not already defined** (`registration.rs:25-35`). Pre-register a root binding `array` → `StdArrayNs` in `BuiltinTypes::register_all`, the same way `json` is bound (`builtins.rs:413-421`). The import then leaves the typed binding alone, and the design sketch `array.from_list` typechecks. A placeholder binding has no methods.

### Pattern 5: Effects on materializers only

`effect_check` applies `stdlib_module_capability` only when the receiver is an `Ident` (`effect_check.rs:286-289` and `:468-472`). `plan.to_list()` has a local or a call as its receiver, so that arm never sees it. Gating the bare method name `to_list` would also be wrong if a user function used the name, but nothing in `*.vox` defines a method `to_list` today (only a HumanEval *function* `str_to_list`). Still key off the receiver type, which only typeck knows.

In the typeck `MethodCall` arm, when the receiver is `Ty::Array` and the method is `to_list`, `item`, or `shape`, require `HirCapability::Python` on the enclosing function. Plan methods add nothing. Runtime: the materializer calls `caps.allows_namespace("py")` and otherwise returns the same capability-denied path `run.rs:116-120` already uses (`exit 77`).

`list[T]` subscript returns `Option[T]` (`expr.rs:866-879`). Element asserts in the golden go through `.unwrap()`. Whole-list `is` does not.

### Pattern 6: Worker handlers

Lazy-import numpy and pyarrow inside the array dispatch. A top-level `import numpy` would break Phase 325's `hello` on a project whose `[python]` packages do not include numpy.

Dispatch is a dict, not a dynamic `getattr` on the numpy module and not `eval`. Unknown `Op.name` → `py-plan:`. The host allowlist and the worker dict are the same twelve names. `np.sort` has no `nulls` parameter. Array API sort leaves NaN order implementation-defined ([CITED: https://data-apis.org/array-api/latest/API_specification/sorting_functions.html]). Numpy 2.5 puts NaNs at the end for both ascending and descending ([CITED: https://numpy.org/doc/stable/reference/generated/numpy.sort.html]). Implement `nulls=first` by partitioning NaNs, `np.sort` on the finite values, then concatenating. `nulls=last` is the numpy 2.5 default, but still implement it explicitly so a future numpy change cannot move NaNs.

`std(correction=N)` calls `numpy.std(a, correction=N)`. Do not pass `ddof` under a Vox name. Empty input: Array API says `M == 0` → mean is NaN, and `M - correction <= 0` → std is NaN ([CITED: mean and std special cases]). Those are conformance `expected` values, not errors.

`add`/`sub`/`mul`/`div` are elementwise and broadcast a 0-d `mean`/`std` plan against the 1-d source. Numpy does that. The type stays `Array[D]`.

One method on the wire: `execute_plan`. Count those frames. `mean` and `std` must not appear as their own `Bridge::call`s. Phase 325's `VOX_BRIDGE_DEBUG_OPS=1` is the existing debug switch. Log one `execute_plan` line per materializer there. The golden test counts lines. Do not add a Vox builtin that returns the count (that would be another materialization).

### Pattern 7: K1 and K2

`CanonicalLadder::load_from_repo_root` reads `contracts/pipeline/canonical-ladder.v1.yaml`. The test `ladder_loads_twelve_fixtures` asserts `fixtures.len() == 12` (`canonical_ladder.rs:71-75`). Adding `py_array_stats` makes that 13. Update the test in the same change.

`run_k_complexity_budget` skips non-ladder goldens and **fails** when a ladder id has no entry (`syntax_k.rs:83-88`):

```rust
            } else if !update {
                failures.push(format!(
                    "Fixture '{}' has no budget defined in {}",
                    fixture_id,
                    budget_path.display()
                ));
            }
```

`run_source_token_budget` does the same (`syntax_k.rs:207-211`) against `contracts/eval/source-token-budget.v1.json`. K-01 names the complexity budget. The source-token gate is the sibling that will go red if the ladder grows alone. Add both keys.

`--update` assigns `budget.fixtures = new_budgets` (`syntax_k.rs:94-95`), replacing every key. Measure `py_array_stats`, insert that one key, and do not commit a full rewrite if other fixtures move.

K2 is not that map. The map stores WebIR gzip/zstd size of the golden (`measure_syntax_k_event`, `syntax_k.rs:277-284`, `k_est` = min compressed length). K2 is the compressed size of the **signature manifest** of `std.array` (the twelve signature strings, canonical order). Store the ceiling in `contracts/python/signature-manifest-budget.v1.json` and check it inside `python-bridge-parity`. Growth fails until the ceiling is intentionally raised. Do not put this number in `complexity-budget.v1.json`.

`LEXER_KEYWORDS` (`vox-language-surface/src/lib.rs:186-241`) is 54 strings, `"fn"` through `"form"`. `categories_partition_lexer_keywords` (`lib.rs:399`) only checks that the three category arrays concatenate to that slice. It does not freeze the set. Add `assert_eq!(LEXER_KEYWORDS.len(), 54)` and `assert!(!LEXER_KEYWORDS.contains(&"array"))` in that test module. K-01 is met when that assertion stays true.

K-01's requirement sentence mentions four `py_*` goldens. The roadmap success criterion for this phase is the one golden. `py_frame_report`, `py_tensor_infer`, and `py_escape_psd` are the exit files of phases 327–329. A budget row for a missing file fails `run_k_complexity_budget`. Do not add those three rows here.

### Anti-Patterns to Avoid

- **`"array" => HirCapability::Python` in `stdlib_module_capability`:** marks plan construction as an effect.
- **A second `Bridge::new` inside the array builtin:** two workers, two generations.
- **JSON numbers in `DispatchRequest.params` for the payload:** `DispatchRequest` (`protocol.rs:192-196`) is `id`, `method`, `params`. Params are control. The list is the frame body. 325's body length stays 0 for `hello`.
- **`assert(true)` or `main` returning `Error(...)`:** `run.rs:107-151` prints a `Result` error value and still returns `Ok(())`. Only `EvalError` (what `assert` raises at `expr.rs:415-417` and `builtins.rs:2729-2739`) becomes a non-zero exit.
- **Positional `std(0)` accepted by `check_arguments`:** ARR-02 requires the name `correction=`.
- **Host f32 stored by reinterpreting f32 bits as f64.**
- **`np.sort` as the implementation of `nulls=`.**
- **Importing numpy at worker startup.**
- **A new `HirType` variant, a new keyword, a new workspace crate, a crate-edges exception.**

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Numeric IPC | Custom float/int framing | `arrow-ipc` + pyarrow IPC stream | NaN, ±Inf, i64::MIN/MAX, and the fetch cap are already specified as Arrow. |
| Op allowlist | Ad hoc `match` copied in three crates with no yaml | Generated table from `contracts/python/ecosystem-support.yaml` | ARR-04 is a join. A hand-written match drifts from typeck. |
| std / sort parameter policy | Numpy defaults | Required `correction=` and `nulls=` | The libraries disagree. The design makes the disagreement a compile error. |
| NaN-aware sort | Trust `np.sort` | Partition, then `np.sort` the finite values | Array API leaves NaN order unspecified. Numpy 2.5 always sorts NaNs last, so `nulls=first` is not the default. |
| Parity CI command | A new framework | `CiCmd::RustEcosystemPolicy` shape | Fail by returning `Err`, or the gate records Pass. |
| K measurement | A new compressor | `measure_syntax_k_event` | Already the min of gzip, bzip2, and zstd. |

**Key insight:** The hard bugs in this phase are silent success: a plan method that calls the worker, a yaml row with no handler, an `f32` decoded as an f64 bit pattern, and an `assert` that never runs because `main` returned `Error`.

## Common Pitfalls

### Pitfall 1: Weak assertions

**What goes wrong:** The golden exits 0 while the numbers are wrong.
**Why it happens:** `vox run` treats `VoxValue::Result(Err)` as a normal return and prints it (`run.rs:145-151`). `assert` is the path that becomes `EvalError::AssertionFailed`.
**How to avoid:** `fn main` calls `assert` on the z-score list and on the NaN / Inf / i64::MAX round-trips. Do not `return Error(...)` from `main` as the failure mode.
**Warning signs:** The test only checks that the process started, or it compares stdout formatting instead of `assert`.

### Pitfall 2: Host f32

**What goes wrong:** `Array[f32]` `to_list` returns a `list[float]` whose bits are not the f64 widening of the f32 value. `0.1` as f32, widened, is not `0.1` as f64, so `is 0.1` fails or, worse, a bit-cast makes a nonsense f64 that still typechecks.
**Why it happens:** D-02 says both plan dtypes materialize as host f64 (`VoxValue::Float`, which is f64; `value.rs:7` and `PartialEq` at `value.rs:275` uses `a == b`).
**How to avoid:** On the host, cast the Arrow column to `Float64Array` before reading. The f32 conformance `expected` is the widened value (`f64::from(value as f32)`), not the f64 literal. The z-score golden uses `astype(f64)` and the exact pair `[1.0, 3.0]` → `[-1.0, 1.0]` (mean 2, population variance 1, std 1, all exact f64). Do not assert a short decimal for a non-dyadic std.
**Warning signs:** A golden `is 0.1` on an `f32` plan, or a decoder that does `f64::from_bits(f32_bits)`.

### Pitfall 3: Allowlist rows without handlers

**What goes wrong:** The yaml lists `sort`, typeck accepts it, and the worker hits a default arm that returns an empty array. Parity stays green if it only checks that the yaml parses.
**Why it happens:** The rust parity test compares yaml decisions to a generated classifier. A python port that only round-trips the yaml to itself never looks at handlers.
**How to avoid:** The negative test builds a row with a signature and an expected value and a handler name that is not a `def` in the worker, and asserts the checker fails. Same for a missing `expected` and a missing signature. A `@test` that calls `array.sort` and does not assert a value does not fill `expected`.
**Warning signs:** `python-bridge-parity` passes after deleting a worker function, or after adding `linalg.matmul` to the yaml with no Rust signature.

### Pitfall 4: `LEXER_KEYWORDS` drift

**What goes wrong:** Someone tokens `array` or `f64` so `astype(f64)` or `array.from_list` stops parsing as identifiers, or the keyword count moves and K-01 is silently broken because the partition test still passes.
**Why it happens:** `categories_partition_lexer_keywords` checks internal consistency, not the set.
**How to avoid:** Freeze `len == 54`. Confirm `"array"`, `"astype"`, `"f32"`, `"f64"`, `"correction"`, and `"nulls"` are absent. `bool` is already an identifier (not in the 54).
**Warning signs:** A lexer change in the same PR as the golden.

### Pitfall 5: i64::MIN is not a Vox literal

**What goes wrong:** The golden contains `-9223372036854775808` and the lexer drops the token. The test never compiles, or it compiles a different program.
**Why it happens:** Integer tokens are `parse::<i64>()` (`token.rs:386-388`):

```rust
    #[regex(r"[0-9]+(_[0-9]+)*", priority = 2, callback = |lex| {
        lex.slice().replace('_', "").parse::<i64>().ok()
    })]
    IntLit(i64),
```

`9223372036854775808` does not fit in `i64`. The callback returns `None`, and `lex` drops the slice (`cursor.rs:31-36`). Unary minus never runs. `9223372036854775807` (i64::MAX) does fit. Runtime negation of a value that is already `i64::MIN` is `VoxValue::Int(-n)` (`expr.rs:337`), which overflows in debug.

**How to avoid:** The golden asserts i64::MAX with the literal. i64::MIN and uint64 overflow are conformance rows whose inputs are Arrow columns built in the Rust test (`Int64Array` with `i64::MIN`, `UInt64Array` with `i64::MAX as u64 + 1`). The uint64 row's expected outcome is an error string starting with `py:`. Do not negate the round-tripped MIN in Vox.
**Warning signs:** A Vox source line that "contains" i64::MIN and a test that only checks the program parses.

### Pitfall 6: NaN equality

**What goes wrong:** `assert(xs is [nan])` fails on a correct round-trip.
**Why it happens:** `VoxValue` float equality is `a == b` (`value.rs:275`). NaN is not equal to NaN. `is` uses that (`expr.rs:207-208`).
**How to avoid:** Build NaN and Inf on the host with float division (`1.0 / 0.0`, `-1.0 / 0.0`, `0.0 / 0.0`). Float division is not the integer div-by-zero fault (`expr.rs:232-233` vs `:189`). `from_list` those values, `to_list`, `unwrap` the `Option` from subscript, `assert(inf is (1.0 / 0.0))`, `assert(nan isnt nan)`. One `to_list` for that triple.
**Warning signs:** An assert that compares NaN with `is` to a literal, or a printed `NaN` string match that depends on display formatting.

### Pitfall 7: The in-process golden sweep

**What goes wrong:** `all_runnable_goldens_execute_under_interp` (`golden_interp_test.rs:148`) runs every golden that contains `fn main` (`:174-176`) inside an interpreter with no bridge.
**Why it happens:** `vox run` only executes `main` (`run.rs:107-112`), so the golden needs `fn main`. The sweep will then try to run it.
**How to avoid:** Add `py_array_stats.vox` to `KNOWN_INTERP_GAPS` with a reason that it needs the uv worker. The paper-code test is a separate test that runs `vox run` in a temp project with `[python]` and `numpy`/`pyarrow`, and counts `execute_plan` frames. Do not let the sweep's skip be the phase exit.
**Warning signs:** CI green on `golden_interp_test` and no `vox run` invocation in the phase tests.

### Pitfall 8: PATH `python3` is 3.15

**What goes wrong:** Handlers are tested with `/opt/homebrew/bin/python3` (3.15.2). Numpy wheels and the design pin are 3.12.
**Why it happens:** Phase 325's plan probes `python3` on PATH. This machine's PATH `python3` is not the uv 3.12.10 install.
**How to avoid:** The golden runs via `vox py sync` and the worker that sync spawned. Conformance uses that venv's interpreter.
**Warning signs:** `import numpy` failing with a 3.15 wheel error, or a skip that treats "no numpy on system python" as success.

## Code Examples

### Array API signatures to implement, not rename

`std` ([CITED: https://data-apis.org/array-api/latest/API_specification/generated/array_api.std.html]):

```text
std(x, /, *, axis=None, correction=0.0, keepdims=False) -> array
```

Vox requires `correction=` and does not accept `axis` or `keepdims` in v1 (they are not in the D-01 row). The return is a 0-d array when reduced over the whole input, same dtype as `x`. Numpy call: `numpy.std(a, correction=n)` ([CITED: numpy.std, `correction` added in 2.0.0]).

`mean` ([CITED: https://data-apis.org/array-api/latest/API_specification/generated/array_api.mean.html]):

```text
mean(x, /, *, axis=None, keepdims=False) -> array
```

0-d, same dtype, for a full reduction of a float array. No Vox arguments in v1.

`sort` ([CITED: https://data-apis.org/array-api/2024.12/API_specification/generated/array_api.sort.html]):

```text
sort(x, /, *, axis=-1, descending=False, stable=True) -> array
```

No `nulls` parameter. Vox adds required `nulls=first|last` because NaN order is implementation-defined ([CITED: https://data-apis.org/array-api/latest/API_specification/sorting_functions.html]).

### Golden shape

`[1.0, 3.0]` with `correction=0` is exact: mean `2.0`, population std `1.0`, z-score `[-1.0, 1.0]`. One `to_list`.

```vox
import std.array

@uses(python)
fn zscore(xs: list[float]) to Result[list[float]] {
    let a = array.from_list(xs).astype(f64)
    return ((a - a.mean()) / a.std(correction=0)).to_list()
}

fn main() {
    let z = zscore([1.0, 3.0]).unwrap()
    assert(z is [-1.0, 1.0])
}
```

NaN / ±Inf is a second materializer in the same `main`, using host `1.0 / 0.0` and `0.0 / 0.0`, then `is` / `isnt` after `unwrap`. i64::MAX is a third `from_list([9223372036854775807]).to_list()`. The debug log shows three `execute_plan` lines and zero calls whose method is `mean` or `std`. That is "one round-trip per materialization."

Compile-error fixtures (not the golden) are ordinary `vox check` tests: `Array[i64] + Array[f64]`, `a.std()`, `a.std(0)`, `a.sort()`, `a.sort("first")`.

### Parity command shape already in tree

`run_body.rs:472-492` is the template. The new arm runs `cargo test -p vox-compiler --test python_bridge_parity` and returns `Err` with a reproducer string when the status is non-zero. The test file's `workspace_root()` helper is the one in `rust_ecosystem_support_parity_test.rs:12-19` (`CARGO_MANIFEST_DIR` parent parent).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| numpy `ddof` | Array API `correction` | numpy 2.0.0 | Vox spells `correction=`. Numpy accepts that name. |
| Unspecified NaN sort | Vox `nulls=first\|last` | Design 2026-11-01; Array API still unspecified (2025.12) | Worker partitions. Do not forward the name to numpy. |
| JSON numerics on the daemon protocol | Arrow IPC body, JSON header only | Design §4.3 / §7 | `DispatchRequest.params` stays control. |

**Deprecated/outdated:**

- Host f32 lists. Rejected by D-02.
- One `@test` per op with no expected value. Rejected by D-03.
- `linalg`, `fft`, `where`, slicing in this yaml. Deferred.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | arrow-rs 60.1.0 and pyarrow 26 write interoperable IPC streams for float64, int64, and uint64 with default features (no zstd). | Standard Stack | Golden fails at decode. Pin a pyarrow that matches, still `>=17`. Do not hand-roll a format. |
| A2 | Same-dtype arithmetic, including `i32` vs `i64`, is a type error. ARR-02's text only names int/float. | Pattern 3 | Users must `astype` across integer widths. Acceptable for v1; a later phase can add a lattice. |
| A3 | `mean` / `std` on integer arrays are type errors. | Pattern 2 | A caller must `astype(f64)` first, which the z-score sketch already does. |
| A4 | K-01's other three goldens are out of this phase even though the requirement sentence lists four. | Pattern 7 | A strict reading of the requirement text would demand files that phases 327–329 own. Adding budget rows for missing files fails the budget gate. |
| A5 | i64::MIN and uint64 overflow are Rust conformance rows, not Vox literals. | Pitfall 5 | The golden itself will not contain the characters of i64::MIN. ARR-03 is still tested through `to_list` on an Arrow column. |

## Open Questions (RESOLVED)

Both recommendations below are closed. Do not reopen them.

1. **Does the landed Phase 325 `Reply` carry the response body bytes?**
   - What we know: the planned signature is `call(method, params, body) -> Reply`. The crate is not in the tree, so the struct fields are not readable.
   - What's unclear: a header-only `Reply` would drop the Arrow payload.
   - Recommendation: If `Reply` has no body, add the bytes on that struct inside `vox-python-bridge`. Do not create a parallel client.
   - Resolution: Reply body bytes live on the Phase 325 struct if that field is missing. This phase does not create a parallel client.

2. **K-01 wording vs this phase's success criterion.**
   - What we know: REQUIREMENTS.md K-01 names four goldens. Roadmap phase 326 criterion 5 names the one golden. The other files are later phase exits.
   - What's unclear: Nothing technical. It is a scope reading.
   - Recommendation: Land `py_array_stats` plus the K2 mechanism. Leave the other three ladder rows to their phases.
   - Resolution: This phase lands only py_array_stats. The other three ladder rows stay in their phases.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `uv` | `vox py sync` for the golden | ✓ | 0.13.6 | none — 325 owns the "uv missing" doctor path |
| CPython 3.12 | design pin, worker | ✓ via uv | 3.12.10 at `~/.local/share/uv/python/cpython-3.12.10-macos-aarch64-none` | do not use PATH python |
| `python3` on PATH | nothing in this phase | ✓ but wrong | 3.15.2 (Homebrew) | ignore for numpy |
| `cargo` (broker) | parity test, arrow dep | ✓ | 1.98.1 | — |
| `vox-python-bridge` crate | the worker the golden calls | ✗ | — | blocked until Phase 325 |
| numpy / pyarrow in a project venv | golden | not locked yet | floors `>=2.1` and `>=17` | `vox py sync` creates them |
| Context7 MCP | doc lookup | needs auth | — | Official Array API and numpy pages were fetched directly |

**Missing dependencies with no fallback:**

- Phase 325 crate, bridge, `[python]` sync, `HirCapability::Python`, and `py:allow`. The golden cannot run until those exist. Planning can proceed; execution wave 0 is "325 is merged and `py_bridge_hello` runs."

**Missing dependencies with fallback:**

- System numpy on Python 3.15. Not used. The project venv is the runtime.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `#[test]` in `vox-compiler` and `vox-python-bridge`; golden via `vox run` |
| Config file | none — cargo tests. Ladder: `contracts/pipeline/canonical-ladder.v1.yaml` |
| Quick run command | `cargo test -p vox-compiler --test python_bridge_parity` |
| Full suite command | `cargo test -p vox-compiler --test python_bridge_parity --test golden_interp_test` plus the `vox run` golden test and `vox ci k-complexity-budget` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ARR-01 | Twelve ops typecheck; golden `assert`s `[-1.0, 1.0]` under `vox run`; one `execute_plan` per materializer | e2e | `vox run` of `examples/golden/py_array_stats.vox` in a temp `[python]` project, debug log count | ❌ Wave 0 |
| ARR-02 | Mixed int/float, `std` without `correction=`, `sort` without `nulls=` are compile errors | unit | `cargo test -p vox-compiler array_dtype_and_required_names` | ❌ Wave 0 |
| ARR-03 | NaN, ±Inf, i64::MAX in the golden; i64::MIN and uint64 overflow in the conformance rows | e2e + unit | golden `vox run` plus `cargo test -p vox-python-bridge array_numeric_roundtrip` | ❌ Wave 0 |
| ARR-04 | Synthetic row missing signature, handler, or expected fails the checker | unit | `cargo test -p vox-compiler --test python_bridge_parity` | ❌ Wave 0. Model: `crates/vox-compiler/tests/rust_ecosystem_support_parity_test.rs` |
| K-01 | Ladder + both budget files; `LEXER_KEYWORDS.len()==54`; K2 ceiling | unit | `cargo test -p vox-codegen canonical_ladder::tests::ladder_loads_twelve_fixtures` (rename/update the 12) and `cargo test -p vox-language-surface categories_partition_lexer_keywords` and `vox ci k-complexity-budget` and `vox ci source-token-budget` | ❌ rows missing; tests exist and will fail closed |

Existing commands and tests to extend, not replace:

- `cargo test -p vox-compiler --test rust_ecosystem_support_parity` — pattern only. Do not weaken it.
- `vox ci rust-ecosystem-policy` — pattern for the new command (`run_body.rs:472-492`).
- `vox ci k-complexity-budget` (`cmd_enums.rs:615-622`, impl `syntax_k.rs:18`).
- `vox ci source-token-budget` (`cmd_enums.rs:625`, impl `syntax_k.rs:146`).
- `all_runnable_goldens_execute_under_interp` (`golden_interp_test.rs:148`) — add the gap entry or this goes red.
- `ladder_loads_twelve_fixtures` (`canonical_ladder.rs:71`) — update the count.

### Sampling Rate

- **Per task commit:** `cargo test -p vox-compiler --test python_bridge_parity` and the new typeck tests.
- **Per wave merge:** those, plus `cargo test -p vox-language-surface categories_partition_lexer_keywords`, plus the `vox run` golden when 325 is present.
- **Phase gate:** `vox run` golden green through the real worker, `vox ci python-bridge-parity` green, negative test red when a field is removed, `vox ci k-complexity-budget` and `vox ci source-token-budget` green.

### Wave 0 Gaps

- [ ] `crates/vox-compiler/tests/python_bridge_parity_test.rs` — ARR-04, including the negative synthetic row.
- [ ] Typeck tests for mixed dtype and required names — ARR-02.
- [ ] `examples/golden/py_array_stats.vox` plus a `vox run` test — ARR-01, ARR-03 (NaN/Inf/i64::MAX).
- [ ] `array_numeric_roundtrip` on the bridge crate — i64::MIN and uint64 — ARR-03.
- [ ] `KNOWN_INTERP_GAPS` entry so the existing interp sweep does not become the false exit.
- [ ] Ladder, complexity-budget, and source-token-budget rows — K-01.
- [ ] `LEXER_KEYWORDS` length assertion — K-01.
- [ ] Phase 325 landed (crate, bridge, caps, capability). Without it the golden test cannot be more than a skip, and a skip is not the phase exit.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Local worker. No user session. |
| V3 Session Management | no | One worker, generation from Phase 325. Stale handle stays `py: stale handle (worker restarted)`. |
| V4 Access Control | yes | `uses python` on materializers only. `caps.allows_namespace("py")`. Worker rejects op names that are not the twelve. |
| V5 Input Validation | yes | Same-dtype arithmetic, required `correction=` / `nulls=`, dtype ident allowlist, plan schema, uint64 > i64::MAX rejected, n-d `to_list` rejected, fetch cap from 325 before the body is read. |
| V6 Cryptography | no | No new crypto. Lockfile hash stays `vox-crypto` in Phase 325. Do not import `sha2` or hash the IPC body ad hoc. |

### Known Threat Patterns for this slice

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Unknown op name in the plan (`eval`, `linalg.solve`) | Tampering | Worker dict of twelve names. `py-plan:` otherwise. Parity gate keeps the dict equal to the yaml. |
| JSON number for the payload | Tampering | Arrow IPC body. `params` is the plan structure only. |
| uint64 that does not fit i64 | Tampering | Runtime error, prefix `py:`. Do not saturate or truncate. |
| numpy/pyarrow `print` on stdout | Tampering (frame corruption) | Phase 325 fd dup. Do not undo it. Import libraries after the dup. |
| Pickle / `torch.load` | Tampering | Not in this slice. Do not add a loader. |
| Over-cap body | Denial of service | Phase 325 `max_fetch_mb` check on the declared length. Do not read past it. |

## Sources

### Primary (HIGH confidence)

- `crates/vox-compiler/src/hir/nodes/stmt_expr.rs:111-124` — `HirType` variants, including `Generic`.
- `crates/vox-compiler/src/hir/nodes/stmt_expr.rs:221-226` — `HirArg.name`.
- `crates/vox-compiler/src/typeck/ty.rs:5-41` — `Ty` has no `Array`.
- `crates/vox-compiler/src/typeck/checker/expr_ops.rs:43-74` and `:151-171` — binary ops and positional `check_arguments`.
- `crates/vox-compiler/src/typeck/effect_check.rs:286-289`, `:468-472`, `:584-600` — module capability only, no `array` / `python`.
- `crates/vox-compiler/src/hir/lower/decl.rs:21-44` — lower match has no `Python` arm.
- `crates/vox-ast/src/decl/effect.rs:35-38`, `:61-62` — AST does accept `python`.
- `crates/vox-compiler/src/eval/caps.rs:16-18`, `:141`, `:285-287`, `:374-378` — `py` tests without an implementation.
- `crates/vox-compiler/src/eval/value.rs:7`, `:275` — host float is f64, equality is `==`.
- `crates/vox-compiler/src/lexer/token.rs:386-388`, `cursor.rs:31-36` — overflowing integer tokens are dropped.
- `crates/vox-compiler/src/builtin_registry.rs:529-552` — `std_root_field_ty`.
- `crates/vox-compiler/src/typeck/registration.rs:25-35` — import placeholder only if unbound.
- `crates/vox-compiler/build.rs:95-157` — yaml → generated policy.
- `crates/vox-cli/src/commands/ci/run_body.rs:472-492` — fail-closed CI shell-out.
- `crates/vox-cli/src/commands/run.rs:107-151` — `main` only; `Result` errors still exit 0.
- `crates/vox-compiler/tests/golden_interp_test.rs:174-176` — sweep runs every `fn main`.
- `crates/vox-codegen/src/canonical_ladder.rs:71-75` — hard-coded 12 fixtures.
- `crates/vox-cli/src/commands/ci/run_body_helpers/syntax_k.rs:83-88` and `:207-211` — missing budget row fails.
- `crates/vox-language-surface/src/lib.rs:186-241` — `LEXER_KEYWORDS`, 54 entries, no `array`.
- `docs/src/architecture/vox-python-interop-design-2026-11-01.md` §4.1–§4.4, §4.6, §9, §11.
- `.planning/phases/325-bridge-runtime-environment-br-01-04-gov-01/325-CONTEXT.md` and `325-02-PLAN.md`.

### Secondary (MEDIUM confidence)

- [Array API `std`](https://data-apis.org/array-api/latest/API_specification/generated/array_api.std.html) — `correction`, 0-d return, NaN propagation, `M - correction <= 0` → NaN.
- [Array API `mean`](https://data-apis.org/array-api/latest/API_specification/generated/array_api.mean.html) — 0-d return, same dtype for floats, empty → NaN, integer promotion is implementation-defined.
- [Array API `sort`](https://data-apis.org/array-api/2024.12/API_specification/generated/array_api.sort.html) — no `nulls` parameter.
- [Array API sorting note](https://data-apis.org/array-api/latest/API_specification/sorting_functions.html) — NaN order is implementation-dependent.
- [numpy.std](https://numpy.org/doc/stable/reference/generated/numpy.std.html) — `correction` since 2.0.0.
- [numpy.sort](https://numpy.org/doc/stable/reference/generated/numpy.sort.html) — NaNs sort to the end (2.5).
- `cargo info arrow-ipc` / `arrow-array` — 60.1.0, Apache-2.0, rust-version 1.88, default features empty.
- PyPI JSON — numpy 2.5.4, pyarrow 26.0.0.

### Tertiary (LOW confidence)

- Arrow IPC bytes from arrow-rs 60 and pyarrow 26 interoperate for the dtypes in this slice. The golden is the falsification. Tagged A1.

## Metadata

**Confidence breakdown:**

- Standard stack: MEDIUM — crate and PyPI versions were read this session; IPC interoperability is untested here.
- Architecture: HIGH — the compiler seams were read. The worker crate was confirmed absent.
- Pitfalls: HIGH — each one cites a function that already behaves that way.

**Research date:** 2026-11-02
**Valid until:** 2026-12-02 (re-check arrow-rs / numpy floors if the golden is still unwritten)
