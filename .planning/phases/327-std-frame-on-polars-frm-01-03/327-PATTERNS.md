# Phase 327: `std.frame` on polars (`FRM-01..03`) - Pattern Map

**Mapped:** 2026-11-03
**Files analyzed:** 26 (9 new, 17 modified)
**Analogs found:** 25 / 26

`std.frame` is a second lazy plan on the phase 325/326 bridge. Copy the array host plan, the generated allowlist table, and the single `execute_plan` round-trip. Do not add a `HirType`, a second worker method, or a second `Bridge`. Do not edit `contracts/python/ecosystem-support.yaml` or the `plan-ir.v1.schema.json` name enum.

Polars is not installed in this phase until a human checkpoint names `polars==2.1.0`. Host decoder tests build Arrow IPC with `arrow-array` only.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `contracts/python/frame-support.yaml` | config | file-I/O | `contracts/python/ecosystem-support.yaml` | exact |
| `contracts/python/frame-plan-ir.v1.schema.json` | config | transform | `contracts/python/plan-ir.v1.schema.json` | role-match |
| `contracts/index.yaml` | config | transform | same file, `python-plan-ir` row | exact |
| `crates/vox-compiler/build.rs` | config | transform | `emit_python_array_ops` | exact |
| `crates/vox-compiler/src/python_frame_support.rs` | config | transform | `crates/vox-compiler/src/python_array_support.rs` | exact |
| `crates/vox-compiler/src/lib.rs` | config | transform | `pub mod python_array_support` | exact |
| `crates/vox-compiler/src/eval/frame_plan.rs` | model | transform | `crates/vox-compiler/src/eval/array_plan.rs` | exact |
| `crates/vox-compiler/src/eval/value.rs` | model | transform | `VoxValue::ArrayPlan` | exact |
| `crates/vox-compiler/src/eval/mod.rs` | provider | request-response | `import std.array` bind | exact |
| `crates/vox-compiler/src/eval/builtins.rs` | controller | request-response | `array` namespace + `ArrayPlan` methods + `materialize_to_list` | exact |
| `crates/vox-compiler/src/eval/expr.rs` | controller | request-response | `to_list` / `item` / `shape` intercept | role-match |
| `crates/vox-compiler/src/builtin_registry.rs` | config | request-response | `"array" => StdArrayNs` | exact |
| `crates/vox-compiler/src/typeck/builtins.rs` | service | request-response | `StdArrayNs` + `Ty::Array` lookup | exact |
| `crates/vox-compiler/src/typeck/checker/expr.rs` | middleware | request-response | python cap on `to_list`/`item`/`shape` | role-match |
| `crates/vox-compiler/src/typeck/checker/expr_ops.rs` | middleware | request-response | `check_array_sort_args` | role-match |
| `crates/vox-compiler/src/typeck/checker/expr_field.rs` | middleware | request-response | `StdArrayNs => "array"` | exact |
| `crates/vox-compiler/src/parser/descent/decl/head.rs` | route | request-response | object-key `Token::And`/`Token::Or` in `pratt_match.rs` | role-match |
| `crates/vox-language-surface/src/lib.rs` | test | transform | `lexer_keywords_freeze_std_array_idents` | exact |
| `crates/vox-python-bridge/src/frame_columns.rs` | service | batch | `crates/vox-python-bridge/src/array_numeric.rs` | role-match |
| `crates/vox-python-bridge/src/lib.rs` | config | batch | `pub use array_numeric::decode_column` | exact |
| `crates/vox-python-bridge/worker/vox_bridge/frame_ops.py` | service | batch | `crates/vox-python-bridge/worker/vox_bridge/array_ops.py` | exact |
| `crates/vox-python-bridge/worker/vox_bridge/__main__.py` | controller | request-response | `dispatch` `execute_plan` arm | exact |
| `crates/vox-compiler/tests/python_bridge_parity_test.rs` | test | file-I/O | `on_disk_expected_matches_behavior_cases` | exact |
| `examples/golden/py_frame_report.vox` | test | request-response | `examples/golden/py_array_stats.vox` | exact |
| `crates/vox-compiler/tests/py_frame_report_run.rs` | test | request-response | `crates/vox-compiler/tests/py_array_stats_run.rs` | exact |
| `examples/golden/fixtures/py_frame_report.csv` | fixture | file-I/O | none | no analog |

## Pattern Assignments

### `contracts/python/frame-support.yaml` (config, file-I/O)

**Analog:** `contracts/python/ecosystem-support.yaml`

**Document shape** (lines 1-9, 72-78): `schema_version`, `module`, `engine`, then `ops` with `name`, `kind`, `signature`, `handler`, `expected`. `required_named` is optional and already used by `std` and `sort`.

```yaml
schema_version: 1
module: std.array
engine: numpy
ops:
  - name: from_list
    kind: plan
    signature: "(xs: list[float]) -> Array[f64]"
    handler: from_list
    expected:
      dtype: f64
      values: [1.0, 3.0]
```

```yaml
  - name: sort
    kind: plan
    signature: "sort(nulls: str) -> Array[f64]"
    handler: sort
    required_named: [nulls]
    expected:
      dtype: f64
      input: [3.0, .nan, 1.0]
```

**Copy rule:** New file. `module: std.frame`, `engine: polars`. `handler` equals `name`. `collect` is the only `kind: materializer`. Reuse `contracts/python/ecosystem-support.schema.json` unchanged (`additionalProperties: false`, required `schema_version` / `module` / `engine` / `ops`). Do not rewrite `ecosystem-support.yaml`.

`required_named: [nulls]` on frame `sort` matches the array row. `required_named: [alias]` on `sum`/`count`/`mean`/`min`/`max` is the allowlist flag that the chain must call `.alias("name")`. That is a method, not a named argument on the aggregation. The Vox spelling stays `col("amount").sum().alias("revenue")`.

---

### `contracts/python/frame-plan-ir.v1.schema.json` (config, transform)

**Analog:** `contracts/python/plan-ir.v1.schema.json`

**Envelope** (lines 1-35): same required keys and op object. The array `name.enum` is frozen.

```json
"required": ["schema_version", "module", "engine", "ops"],
"name": {
  "type": "string",
  "enum": ["from_list", "astype", "add", "sub", "mul", "div", "mean", "std", "sort", "to_list", "item", "shape"]
},
"kind": {
  "type": "string",
  "enum": ["plan", "materializer"]
}
```

**Copy rule:** New file. Copy the envelope. Replace the enum with the frame names only (`scan_csv`, `col`, `filter`, `group_by`, `agg`, `sort`, `sum`, `count`, `mean`, `min`, `max`, `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `and`, `or`, `collect`). Set `module` / description to `std.frame` and `engine` expectation to `polars`. Leave `plan-ir.v1.schema.json` alone.

---

### `contracts/index.yaml` (config, transform)

**Analog:** the `python-plan-ir` entry (lines 925-947).

```yaml
  - id: python-ecosystem-support
    path: contracts/python/ecosystem-support.yaml
    owner: platform-ci
    kind: yaml
    description: std.array stats-slice allowlist (signature, handler, and expected value per op).
    enforced_by:
      - vox ci contracts-index

  - id: python-plan-ir
    path: contracts/python/plan-ir.v1.schema.json
    owner: platform-ci
    kind: json-schema
    description: Plan IR schema for the seven std.array stats-slice nodes.
    enforced_by:
      - vox ci contracts-index
```

**Copy rule:** Append `python-frame-support` and `python-frame-plan-ir` after `python-plan-ir`. Same `owner`, `kind`, and `enforced_by: [vox ci contracts-index]`. Do not change the array descriptions.

---

### `crates/vox-compiler/build.rs` + `python_frame_support.rs` + `lib.rs` (config, transform)

**Analog:** `emit_python_array_ops` (lines 180-262) and `python_array_support.rs` (lines 1-27).

```rust
fn emit_python_array_ops(workspace_root: &std::path::Path, out_dir: &std::path::Path) {
    let registry_path = workspace_root.join("contracts/python/ecosystem-support.yaml");
    println!("cargo:rerun-if-changed={}", registry_path.display());
    // ...
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

```rust
//! `std.array` signature table generated from `contracts/python/ecosystem-support.yaml`.

include!(concat!(env!("OUT_DIR"), "/python_array_ops.rs"));

pub fn python_array_ops() -> &'static [PythonArrayOp] {
    PYTHON_ARRAY_OPS
}
```

`lib.rs` line 31: `pub mod python_array_support;`

**Copy rule:** Add `emit_python_frame_ops` beside the array emitter. It reads `contracts/python/frame-support.yaml`, panics unless `module == "std.frame"` and `engine == "polars"`, and writes `OUT_DIR/python_frame_ops.rs`. Keep the array panics. `python_frame_support.rs` is the hand-written wrapper (`PythonFrameOp`, `PythonFrameOpKind`, `include!`, `python_frame_ops()`). Generated rows stay in `OUT_DIR`. Register `pub mod python_frame_support;` next to `python_array_support`.

Empty `signature`, empty `handler`, `handler != name`, and a non-mapping `expected` already panic in the array emitter (lines 221-238). Copy those checks.

---

### `crates/vox-compiler/src/eval/frame_plan.rs` (model, transform)

**Analog:** `crates/vox-compiler/src/eval/array_plan.rs`

**Imports / purity** (lines 1-14, 41-65): no `Bridge`. A process-global counter is copied onto the plan and never incremented by constructors.

```rust
//! Pure `std.array` plans. Building a plan stores host data and op names.
//! It does not call the Python worker.

static PLAN_WORKER_CALLS: AtomicU32 = AtomicU32::new(0);

pub struct ArrayPlan {
    steps: Vec<PlanStep>,
    hosts: Vec<HostColumn>,
    rank: u8,
    worker_calls: u32,
}

pub fn from_list(values: Vec<f64>) -> Self {
    Self {
        steps: vec![PlanStep { op: "from_list", /* ... */ }],
        worker_calls: plan_worker_calls(),
        // ...
    }
}
```

**Append** (lines 115-125, 229-245): `sort` records `first`/`last` and does not call the worker. `push` clones, appends one step, and copies the counter.

```rust
pub fn sort(&self, nulls: &str) -> Self {
    let recorded = match nulls {
        "first" => Some("first"),
        "last" => Some("last"),
        _ => None,
    };
    self.push("sort", None, recorded, self.rank)
}
```

**Test** (lines 287-314): `array_plan_zscore_is_pure` asserts `worker_calls() == 0` before any materializer. The yaml is not filled from the golden (`!raw.contains("py_array_stats")`).

**Copy rule:** `FramePlan` stores the canonical path from `scan_csv` and later steps (`filter`, `group_by`, `agg`, `sort`, expressions). Constructors and appenders call no worker. `worker_calls()` stays 0 until `collect`. Declare `pub mod frame_plan;` in `eval/mod.rs` next to line 4 `pub mod array_plan;`. Same-file `#[cfg(test)]` comes first: a plan built through `sort` records zero worker calls, and `collect` is the only step that is allowed to count as one round-trip (the count itself lives in the materializer, matching `MATERIALIZE_CALLS` below).

Frame `sort` stores both the column string and `nulls`. Do not reuse `args.first()` as the nulls string (see builtins).

---

### `crates/vox-compiler/src/eval/value.rs` (model, transform)

**Analog:** `VoxValue::ArrayPlan` (lines 16-18) and `VoxValue::Decimal` (lines 11-12).

```rust
Decimal(rust_decimal::Decimal),
/// Pure `std.array` plan. Constructors store the host list and append
/// op names; they do not call the Python worker.
ArrayPlan(crate::eval::array_plan::ArrayPlan),
```

**Copy rule:** Add `FramePlan(...)`, `Records { ... }`, `Date`, `DateTime`, and `Duration` next to `ArrayPlan`. `Decimal` already exists. Do not add `Option` for a null cell. The binary-op match in `expr.rs` (lines 351-367) ends in `(op, l, r) => Err(...)`, so new variants do not need arithmetic arms. Frame ops are methods, not `+`.

Exhaustive matches that must gain arms in the same change:

- `vox_value_type_name` (`builtins.rs` lines 3211-3217) — `ArrayPlan(_) => "ArrayPlan"`.
- `vox_value_display` (`builtins.rs` lines 3292-3293) — `ArrayPlan` is in the non-printing group.

---

### `crates/vox-compiler/src/eval/mod.rs` (provider, request-response)

**Analog:** `import std.array` (lines 521-528).

```rust
for imp in &module.imports {
    if imp.module_path == ["std"] && imp.item == "array" {
        let ns = VoxValue::object(vec![(
            "__namespace__".to_string(),
            VoxValue::Str("array".into()),
        )]);
        self.scope.set("array".to_string(), ns.clone());
        self.module_scope.set("array".to_string(), ns);
    }
```

**Copy rule:** Same block for `imp.item == "frame"`, namespace string `"frame"`, bound name `"frame"`.

---

### `crates/vox-compiler/src/builtin_registry.rs` + `expr_field.rs` (config / middleware)

**Analog:** `std_root_field_ty` line 547 and `expr_field.rs` lines 210-227.

```rust
"array" => Ty::Named("StdArrayNs".into()),
```

```rust
"StdArrayNs" => Some("array"),
```

**Copy rule:** `"frame" => Ty::Named("StdFrameNs".into())` and `"StdFrameNs" => Some("frame")`.

---

### `crates/vox-compiler/src/typeck/builtins.rs` (service, request-response)

**Analog:** pre-register (lines 401-411), namespace methods (lines 1875-1881), receiver methods (lines 2040-2048), signature parser (lines 20-32).

```rust
// `import std.array` binds the name `array`.
env.define(
    "array".into(),
    Binding {
        ty: Ty::Named("StdArrayNs".into()),
        mutable: false,
        kind: BindingKind::Import,
        is_deprecated: false,
    },
);
```

```rust
let mut array_ns = std::collections::HashMap::new();
for op in crate::python_array_support::python_array_ops() {
    if op.name == "from_list" {
        array_ns.insert(op.name.to_string(), array_op_fn_ty(op.signature, false));
    }
}
methods.insert("StdArrayNs".into(), array_ns);
```

```rust
if let Ty::Array(_) = obj_ty {
    return crate::python_array_support::python_array_ops()
        .iter()
        .find(|op| op.name == method && !matches!(op.name, "from_list" | "add" | "mul"))
        .map(|op| array_op_fn_ty(op.signature, true));
}
```

**Copy rule:** Pre-register `"frame"` as `Ty::Named("StdFrameNs")`. `StdFrameNs` methods are `scan_csv` and `col` only (the namespace constructors). `Frame` and `Expr` methods come from `python_frame_ops()`, looked up by `Ty::Named("Frame")` / `Ty::Named("Expr")` the way `Ty::Array` looks up `python_array_ops`. Do not add `Ty::Frame`. `Ty::Named` already lowers through `HirType::Named` (`ty.rs` lines 124 and 139).

`Records` is not an allowlist op receiver for the plan. Register `num_rows() -> int` and `column(name) -> list[T]` on `Ty::Named("Records")`. Do not reuse `Ty::Record` `get`, which returns `Option` (`builtins.rs` lines 1989-1996).

`array_op_fn_ty` splits on `->` and the last `(...)`. Frame signatures in the yaml must use that shape or the parser returns `Ty::Error`.

---

### `crates/vox-compiler/src/typeck/checker/expr.rs` (middleware, request-response)

**Analog:** materializer cap (lines 579-590) and the test `item_shape_require_python` / `sort must not require uses python` (lines 1098-1148).

```rust
if matches!(method.as_str(), "to_list" | "item" | "shape")
    && matches!(obj_ty, Ty::Array(_))
    && !self.current_caps.iter().any(|cap| matches!(cap, crate::hir::HirCapability::Python))
{
    self.diags.push(Diagnostic::error(
        format!("{method} on Array requires `uses python`"),
        *span,
        self.source,
    ));
}
```

**Copy rule:**

- `collect` on `Frame` requires `HirCapability::Python` (same test shape as `item`/`shape`).
- `scan_csv` on `StdFrameNs` requires `Python` and `Fs`. The function that calls it carries `@uses(python, fs)`.
- `sort`, `filter`, `group_by`, `agg`, and expression methods do not require the effect by themselves. Copy the `sorted` test: a function without `@uses` that only sorts must typecheck.
- `effect_check.rs` `stdlib_module_capability` (lines 588-603) maps `"python"` and does not map `"array"`. Do not add `"frame"`.

Known CSV schema: if `scan_csv`'s path is a string literal and the file can be read without `..`, split the first line on commas and reject `col("missing")` at typeck. A runtime `str`, or a file that cannot be read, is not a compile error. There is no existing header reader. Diagnostic style is `check_array_sort_args` (`expr_ops.rs` lines 214-218): `self.diags.push(Diagnostic::error(...))`.

---

### `crates/vox-compiler/src/typeck/checker/expr_ops.rs` (middleware, request-response)

**Analog:** `check_array_sort_args` (lines 202-239).

```rust
/// `sort` accepts only the name `nulls` bound to the string `first` or
/// `last`. A missing name, a positional string, and any other string are
/// diagnostics. The name is checked before any positional argument zip.
/// `nulls` is not defaulted.
pub(super) fn check_array_sort_args(&mut self, args: &[HirArg], span: Span) {
    let named = args.len() == 1 && args[0].name.as_deref() == Some("nulls");
    // ...
    if self.uf.unify(&Ty::Str, &value_ty).is_err() || !matches!(literal, Some("first" | "last"))
    {
        self.diags.push(Diagnostic::error(
            "sort argument `nulls` must be the string \"first\" or \"last\"".into(),
            arg_span,
            self.source,
        ));
    }
}
```

**Copy rule:** Frame `sort("month", nulls=last)` is one positional `str` plus that same `nulls` check. Do not default `nulls`. Do not require the only argument to be named (the array function does, because array `sort` has no column argument). Call it from the `method == "sort"` arm in `expr.rs` when the receiver is `Frame`, next to the `Ty::Array` arm at `expr.rs` lines 572-575.

Aggregations (`sum`, `count`, `mean`, `min`, `max`) typecheck as `Expr` and are only legal inside `agg` after `.alias("name")`. Missing `.alias` is a compile error on a literal chain.

---

### `crates/vox-compiler/src/parser/descent/decl/head.rs` (route, request-response)

**Analog for the token map:** `parse_object_lit` in `pratt_match.rs` lines 655-661. **Call site:** `pratt_match.rs` lines 504-514. **Do not grow keywords:** `crates/vox-language-surface/src/lib.rs` lines 412-424.

```rust
Token::Dot => {
    self.advance();
    let field = self.parse_ident_name()?;
    if self.eat(&Token::LParen) {
        let args = self.parse_args()?;
```

```rust
Token::And => {
    self.advance();
    "and".to_string()
}
Token::Or => {
    self.advance();
    "or".to_string()
}
```

```rust
pub(crate) fn parse_ident_name(&mut self) -> Result<String, ()> {
    match self.peek().clone() {
        Token::Ident(n) | Token::TypeIdent(n) => { /* ... */ }
        // keyword-as-name arms exist for TypeKw, On, In, ...
        _ => { /* Expected identifier */ }
    }
}
```

```rust
assert_eq!(LEXER_KEYWORDS.len(), 54);
assert_eq!(LEXER_KEYWORDS.first().copied(), Some("fn"));
assert_eq!(LEXER_KEYWORDS.last().copied(), Some("form"));
for absent in ["array", "astype", "f32", "f64", "correction", "nulls"] {
    assert!(!LEXER_KEYWORDS.contains(&absent), "{absent} must not be a lexer keyword");
}
```

**Copy rule:** Add `Token::And => "and"` and `Token::Or => "or"` to `parse_ident_name`, matching the object-key arms. That makes `.and(` and `.or(` method calls. Do not add keywords. Extend the absent list with `frame`, `scan_csv`, `collect`, `and` is already a keyword so it stays in `LEXER_KEYWORDS`. Add a parse test that `col("a").gt(1).and(col("b").lt(2))` parses. `"and"` and `"or"` are already in the 54.

---

### `crates/vox-compiler/src/eval/builtins.rs` (controller, request-response)

**Analog:** namespace `from_list` (lines 1074-1078), `ArrayPlan` methods (lines 2587-2608), `fs_resolve_allowed` (lines 134-165), `materialize_to_list` (lines 2615-2656), `plan_params` (lines 2798-2826).

```rust
if ns == Some("array") {
    return match method {
        "from_list" => Some(array_from_list(args)),
        _ => None,
    };
}
```

```rust
VoxValue::ArrayPlan(plan) => match method {
    "astype" => Some(VoxValue::ArrayPlan(plan.astype(dtype))),
    "mean" => Some(VoxValue::ArrayPlan(plan.mean())),
    "sort" => {
        let Some(VoxValue::Str(nulls)) = args.first() else {
            return Some(VoxValue::_Panic("sort requires nulls".into()));
        };
        Some(VoxValue::ArrayPlan(plan.sort(nulls)))
    }
    _ => None,
},
```

```rust
fn fs_resolve_allowed(
    caps: &crate::eval::caps::CapabilitySet,
    raw: &str,
    write: bool,
) -> Option<std::path::PathBuf> {
    if raw.is_empty() { return None; }
    if p.components().any(|component| component == std::path::Component::ParentDir) {
        return None;
    }
    // canonicalize, then:
    caps.allows_path(&canon, write).then_some(canon)
}
```

```rust
pub fn materialize_to_list(...) -> VoxValue {
    if !caps.allows_namespace("py") {
        return VoxValue::_Denied("py.to_list".into());
    }
    let params = plan_params(plan);
    let body = match encode_host_column(plan) { /* ... */ };
    match super::python::execute_plan(interp, params, body) { /* Result or err_str */ }
}

fn plan_params(plan: &ArrayPlan) -> serde_json::Value {
    serde_json::json!({ "ops": ops })
}
```

**Copy rule:**

- `ns == Some("frame")`: `scan_csv` calls `fs_resolve_allowed(caps, path, false)`. `None` returns `VoxValue::_Denied("fs.scan_csv".into())` before any `FramePlan` that could be collected is stored. `col` builds an `Expr` and does not touch the worker. This arm must sit before the generic `allows_namespace` denial at lines 1081-1083, the same way `array` does. `frame` is not an `allows_namespace` token.
- `VoxValue::FramePlan` methods (`filter`, `group_by`, `agg`, `sort`) only append steps. `sort` reads the column from the positional string and `nulls` from the named argument's value. Array `sort` uses `args.first()` because it has no column. Do not copy that indexing.
- `collect` does not belong in `call_builtin_method`. It needs `interp`. See `expr.rs`.
- Leave `plan_params` unchanged. Array JSON stays `{"ops":[...]}` with no `family` key. Frame params are a new function: `{"family":"frame","ops":[...]}`. The collect body is empty. The CSV stays on disk. The worker opens the canonical path.

`_Denied` becomes exit 77 in `expr.rs` lines 892-898 (`split_once('.')`) and `crates/vox-cli/src/commands/run.rs` lines 126-130. `py-plan:` is `VoxValue::Result(Err(...))`, not exit 77.

---

### `crates/vox-compiler/src/eval/expr.rs` (controller, request-response)

**Analog:** materializer intercept (lines 718-736).

```rust
if matches!(method.as_str(), "to_list" | "item" | "shape")
    && eval_args.is_empty()
    && let VoxValue::ArrayPlan(plan) = &o
{
    let result = match method.as_str() {
        "item" => super::builtins::materialize_item(interp, &plan, &caps),
        "shape" => super::builtins::materialize_shape(interp, &plan, &caps),
        _ => super::builtins::materialize_to_list(interp, &plan, &caps),
    };
    return match result {
        VoxValue::_Denied(what) => Err(EvalError::CapabilityDenied { /* split_once */ }),
        other => Ok(other),
    };
}
```

**Copy rule:** A sibling intercept for `collect` on `FramePlan`. Missing `py` returns `_Denied("py.collect")`. One call to the existing `python::execute_plan`. Do not add `execute_frame`.

---

### `crates/vox-python-bridge/worker/vox_bridge/__main__.py` (controller, request-response)

**Analog:** `dispatch` lines 53-59.

```python
def dispatch(method: str, params: dict, body: bytes, debug_ops: bool):
    if method == "execute_plan":
        from vox_bridge.array_ops import execute_plan

        return execute_plan(params, body)
```

**Copy rule:** When `params.get("family") == "frame"`, import `vox_bridge.frame_ops` and return its `execute_plan`. Otherwise call `array_ops.execute_plan` unchanged. The `Exception` handler (lines 40-44) already prefixes messages that do not start with `py-plan:` or `py:`.

---

### `crates/vox-python-bridge/worker/vox_bridge/frame_ops.py` (service, batch)

**Analog:** `array_ops.py` lines 1-10, 92-109, 137-190.

```python
class PlanError(Exception):
    """Plan rejected before numpy runs. The message starts with py-plan:."""

def sort(stack, step, host):
    nulls = step.get("nulls")
    if nulls not in ("first", "last"):
        raise PlanError(f"py-plan: sort nulls {nulls!r}")
    # np.sort has no nulls argument. Partition, then join.

HANDLERS = { "from_list": from_list, "sort": sort, /* ... */ }

def execute_plan(params, body: bytes):
    ops = params.get("ops")
    if not isinstance(ops, list):
        raise PlanError("py-plan: plan has no ops")
    for step in ops:
        name = step.get("name")
        handler = HANDLERS.get(name) if isinstance(name, str) else None
        if handler is None:
            raise PlanError(f"py-plan: unknown op {name!r}")
        handler(stack, step, host)
```

**Copy rule:** Own `HANDLERS` dict. Unknown name is `py-plan: unknown op`. Import polars inside `execute_plan`, not at module import, and not from `array_ops`. `scan_csv` is `pl.scan_csv(canonical, glob=False)` with no `storage_options` and no `credential_provider`. Vox `neq`/`gte`/`lte`/`and`/`or` translate to polars `ne`/`ge`/`le`/`and_`/`or_`. Vox `nulls=last` is `nulls_last=True`; `nulls=first` is `nulls_last=False`. Reject any other string before calling polars.

A null cell raises `PlanError` starting with `py-plan:` and naming the step and column (`py-plan: scan_csv must drop or fill nulls in column amount`) before an Arrow body is returned. A naive timestamp's message contains the exact substring `.replace_time_zone("UTC")`. Do not allowlist `replace_time_zone`.

`def` lines stay at column 0 and `HANDLERS` uses `"name": name`. The parity test greps both.

---

### `crates/vox-python-bridge/src/frame_columns.rs` + `lib.rs` (service, batch)

**Analog:** `array_numeric.rs` lines 1-83. **Export analog:** `lib.rs` lines 9-18. **Not the analog:** `frame.rs` (stdio length prefix). Do not put polars plans there.

```rust
pub fn decode_column(bytes: &[u8], max_body: u64) -> Result<DecodedColumn, String> {
    if body_len > max_body {
        return Err("py: fetch exceeds cap".to_string());
    }
    // Float64, Float32, Int64, UInt64 only
    Err(format!("py: arrow decode: unsupported column {}", column.data_type()))
}

fn reject_nulls(null_count: usize) -> Result<(), String> {
    if null_count > 0 {
        Err("py: arrow decode: column contains nulls".to_string())
    } else {
        Ok(())
    }
}
```

```rust
mod array_numeric;
mod frame;

pub use array_numeric::{DecodedColumn, decode_column, encode_f32_column, encode_u64_column};
```

**Copy rule:** New module `frame_columns`. `mod frame_columns;` and a `pub use` next to `array_numeric`. Do not call `decode_column` for a frame batch. Copy `write_batch` / `StreamReader` and the fetch-cap check. Null, naive timestamp, struct, map, and union return an error and build no value. Null and naive-timestamp errors start with `py-plan:` and the naive text includes `.replace_time_zone("UTC")`. Widen `uint8`–`uint32` to `i64`. `uint64` above `i64::MAX` keeps the existing `py:` overflow error. `Decimal` parses with `rust_decimal::Decimal`. `Date` / `DateTime` / `Duration` use `chrono` in the compiler after this decoder reports the wire values. Do not add `chrono` to `vox-python-bridge`. Do not bump `arrow-array` / `arrow-ipc`.

Same-file tests encode fixtures with `arrow-array` (decimal128, timestamp with and without a zone, date32, duration, one null slot). No polars import.

---

### `crates/vox-compiler/tests/python_bridge_parity_test.rs` (test, file-I/O)

**Analog:** `on_disk_expected_matches_behavior_cases` (lines 1-90).

```rust
const D01_OPS: &[&str] = &[
    "from_list", "astype", "add", "sub", "mul", "div",
    "mean", "std", "sort", "to_list", "item", "shape",
];

#[test]
fn on_disk_expected_matches_behavior_cases() {
    let yaml_path = root.join("contracts/python/ecosystem-support.yaml");
    assert_eq!(names, D01_OPS, "yaml ops must be the twelve D-01 names");
    let worker_path = root.join("crates/vox-python-bridge/worker/vox_bridge/array_ops.py");
    let table = vox_compiler::python_array_support::python_array_ops();
    // signature, handler == name, expected mapping, `def {name}(` at column 0,
    // `"name": name` in HANDLERS
}
```

**Copy rule:** Add a second test that reads `frame-support.yaml`, joins `python_frame_ops()`, and greps `frame_ops.py`. Leave `on_disk_expected_matches_behavior_cases` asserting the twelve array names.

---

### `examples/golden/py_frame_report.vox` (test, request-response)

**Analog:** `examples/golden/py_array_stats.vox` (lines 1-21).

```vox
import std.array

pub @uses(python)
fn zscore(xs: list[float]) to Result[list[float]] {
    let a = array.from_list(xs).astype(f64)
    return ((a - a.mean()) / a.std(correction=0)).to_list()
}
```

**Copy rule:** Frontmatter in that shape (`title`, `description`, `syntax_version`, `status: golden`, `category: example`, `constructs`, `training_eligible`). Body:

```vox
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

One `collect`. No `Option`. No `frame.amount`. Assert the sorted `revenue` list, not pre-sort group order. The CSV has no empty fields. This golden does not need `and`/`or`; the parser test still does.

---

### `crates/vox-compiler/tests/py_frame_report_run.rs` (test, request-response)

**Analog:** `py_array_stats_run.rs` lines 1-4 and 52-155.

```rust
//! Fails closed when the worker cannot run. A skip or `#[ignore]` is not a pass.

fs::write(project.join("Vox.toml"), "\
[package]
name = \"py_array_stats\"

[python]
version = \"3.12\"
packages = [\"numpy>=2.1\", \"pyarrow>=17\"]
");
// copy golden, vox py sync with UV_PYTHON=3.12, assert requires-python and venv 3.12
// vox run with VOX_BRIDGE_DEBUG_OPS=1
let execute_plan_lines = log.lines().filter(|line| line.contains("execute_plan")).count();
assert_eq!(execute_plan_lines, 3, "expected three execute_plan lines in:\n{log}");
```

**Copy rule:** Same harness. Copy the golden and `fixtures/py_frame_report.csv` into the temp project (the array test only copies the `.vox`). Packages are the approved pins plus the checkpoint pin: `numpy==2.5.4`, `pyarrow==26.0.1`, `polars==2.1.0`. Expect `execute_plan_lines == 1`. A skip is not a pass. This test waits until the polars checkpoint. Do not run `vox py sync` in an earlier task.

---

### `examples/golden/fixtures/py_frame_report.csv` (fixture, file-I/O)

**No content analog.** Golden examples have no CSV sibling. The copy step is the array golden's `fs::copy` of the `.vox` into the temp project (`py_array_stats_run.rs` lines 70-72). Header row is the known schema for the compile-time missing-column check. No empty fields. Do not add a `schema` argument to `scan_csv`.

---

## Shared Patterns

### One worker call

**Source:** `crates/vox-compiler/src/eval/python.rs` lines 140-174
**Apply to:** `collect` only. Do not edit this function into a second method.

```rust
/// One `execute_plan` on the worker this host already owns.
/// Does not construct a second [`vox_python_bridge::Bridge`].
pub(crate) fn execute_plan(
    interp: &mut Interpreter,
    params: serde_json::Value,
    body: Vec<u8>,
) -> Result<vox_python_bridge::Reply, String> {
    tracing::debug!(payload = %params, "array plan sent to execute_plan");
    // Host::start only when python_host is empty; Msg::ExecutePlan; recv_timeout
    if std::env::var_os("VOX_BRIDGE_DEBUG_OPS").is_some_and(|value| value == "1") {
        eprintln!("execute_plan");
    }
    Ok(reply)
}
```

The debug string says `array plan`. Frame collect still uses this function. The golden counts `execute_plan` lines, so the `eprintln!("execute_plan")` must stay.

### Capability denial vs `py-plan:`

**Source:** `expr.rs` lines 892-898, `run.rs` lines 126-130, `caps.rs` lines 91-107 and 151-153 and 327-335
**Apply to:** `scan_csv` and `collect`

```rust
crate::eval::value::VoxValue::_Denied(what) => {
    let (ns, method) = what.split_once('.').unwrap_or((what.as_str(), ""));
    Err(EvalError::CapabilityDenied { ns: ns.to_string(), method: method.to_string() })
}
```

```rust
Err(vox_compiler::eval::EvalError::CapabilityDenied { ns, method }) => {
    eprintln!("vox: capability denied: {ns}.{method} — grant with --caps <token> (see isolation.md)");
    std::process::exit(77);
}
```

```rust
("fs", r) if r.starts_with("ro=") || r.starts_with("rw=") => {
    let root = std::fs::canonicalize(raw)?;
    out.allowed.insert("fs".into());
}
("py", "allow") => { out.allowed.insert("py".into()); }
```

```rust
pub fn allows_path(&self, path: &Path, write: bool) -> bool {
    match roots {
        None => true, // unscoped fs allows every canonical path
        Some(rs) => { /* prefix */ }
    }
}
```

FRM-03 denial test copies `caps_enforcement_test.rs` lines 6-25 and 48-59: `CapabilitySet::parse`, not `developer_default()`. Grant `py:allow` plus `fs:ro=<tempdir>`. A path outside that root, and a path containing `..`, assert `CapabilityDenied` and zero `execute_plan` log lines. `developer_default()` leaves `fs_ro: None`, which allows every path.

`resolve_local_file_import` (`eval/mod.rs` lines 673-677) is the other path jail. `scan_csv` uses `fs_resolve_allowed` because it also rejects `..` and returns the canonical path to store.

### Types without a new `HirType`

**Source:** `hir/nodes/stmt_expr.rs` lines 111-124, `ty.rs` lines 16-17 and 124 and 139
**Apply to:** `Frame`, `Expr`, `Records`, `Date`, `DateTime`, `Duration`

```rust
pub enum HirType {
    Named(String),
    Generic(String, Vec<HirType>),
    Function(Vec<HirType>, Box<HirType>),
    Tuple(Vec<HirType>),
    Unit,
    Decimal,
}
```

```rust
/// Lazy `std.array` plan. Lowered as `HirType::Generic`, not a new `HirType` variant.
Array(Box<Ty>),
// ...
Ty::Array(inner) => HirType::Generic("Array".into(), vec![inner.to_hir_type()]),
Ty::Named(n) => HirType::Named(n.clone()),
Ty::Decimal => HirType::Decimal,
```

Do not add a variant. `Decimal` stays `Ty::Decimal`. The new names are `Ty::Named`.

### Closed allowlist

**Source:** `array_ops.py` lines 180-182 and `python_bridge_parity_test.rs` lines 84-89
**Apply to:** `frame_ops.py` and the frame parity test

An op that is not in `HANDLERS` does not run. `def` at column 0 and a `"name": name` key are what the parity test checks. Array and frame dicts stay separate so `mean` in one family cannot run the other engine.

### Error prefix

Worker failures that are plan rejections start with `py-plan:`. Protocol failures start with `py:`. `__main__.py` lines 40-44 add `py: ` only when the message has neither prefix. Null cells and naive timestamps use `py-plan:`. Do not reuse `py: arrow decode: column contains nulls`.

### Test-first

New `pub fn` in `frame_plan.rs` and `frame_columns.rs` needs a same-file `#[test]` before the implementation. Integration tests: `python_bridge_parity` (frame test in the existing binary), `scan_csv` denial via `CapabilitySet::parse`, `.and(` / `.or(` parse, then `py_frame_report_run` after the checkpoint.

### Files that stay as they are

| File | Why |
|------|-----|
| `contracts/python/ecosystem-support.yaml` | Twelve-op `std.array` document. A frame row here drops or mixes array ops. |
| `contracts/python/plan-ir.v1.schema.json` | Name enum is the stats slice. |
| `contracts/python/ecosystem-support.schema.json` | Reused by the sibling yaml. Do not bump it. |
| `crates/vox-compiler/src/eval/python.rs` | `execute_plan` is the only compute call. |
| `crates/vox-python-bridge/src/array_numeric.rs` | Array decoder stays f64/i64. |
| `crates/vox-python-bridge/src/frame.rs` | Stdio frames, not dataframes. |
| `crates/vox-compiler/src/typeck/effect_check.rs` | Do not map `"frame"`. |
| `crates/vox-compiler/src/typeck/ty.rs` | Do not add `Ty::Frame`. Use `Named`. |
| `crates/vox-compiler/src/hir/nodes/stmt_expr.rs` | `HirType` stays six variants. |
| `.planning/REQUIREMENTS.md`, `STATE.md`, `ROADMAP.md` | Out of task commits. The non-null override is CONTEXT.md. |

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `examples/golden/fixtures/py_frame_report.csv` | fixture | file-I/O | No CSV under `examples/golden/`. Copy it with the golden the way `py_array_stats_run.rs` copies the `.vox`. Header is the known schema. |
| Known-schema header check | middleware | file-I/O | No typeck path reads a CSV header. Diagnostic push copies `check_array_sort_args`. A missing file is not a compile error. |

## Metadata

**Analog search scope:** `contracts/python/`, `contracts/index.yaml`, `crates/vox-compiler/src/eval/`, `crates/vox-compiler/src/typeck/`, `crates/vox-compiler/src/parser/descent/`, `crates/vox-compiler/src/builtin_registry.rs`, `crates/vox-compiler/build.rs`, `crates/vox-compiler/tests/`, `crates/vox-python-bridge/src/`, `crates/vox-python-bridge/worker/vox_bridge/`, `crates/vox-language-surface/src/lib.rs`, `crates/vox-cli/src/commands/run.rs`, `examples/golden/`
**Files scanned:** 36 tracked sources (`git ls-files` non-empty for every analog path above)
**Pattern extraction date:** 2026-11-03

### What each new frame file copies

| New file | Copy from |
|----------|-----------|
| `contracts/python/frame-support.yaml` | `contracts/python/ecosystem-support.yaml` |
| `contracts/python/frame-plan-ir.v1.schema.json` | `contracts/python/plan-ir.v1.schema.json` (envelope only; new name enum) |
| `crates/vox-compiler/src/python_frame_support.rs` | `crates/vox-compiler/src/python_array_support.rs` |
| `crates/vox-compiler/src/eval/frame_plan.rs` | `crates/vox-compiler/src/eval/array_plan.rs` |
| `crates/vox-python-bridge/src/frame_columns.rs` | `crates/vox-python-bridge/src/array_numeric.rs` (IPC stream; `py-plan:` errors, not `decode_column`) |
| `crates/vox-python-bridge/worker/vox_bridge/frame_ops.py` | `crates/vox-python-bridge/worker/vox_bridge/array_ops.py` |
| `examples/golden/py_frame_report.vox` | `examples/golden/py_array_stats.vox` |
| `crates/vox-compiler/tests/py_frame_report_run.rs` | `crates/vox-compiler/tests/py_array_stats_run.rs` |
| `examples/golden/fixtures/py_frame_report.csv` | no source analog |
