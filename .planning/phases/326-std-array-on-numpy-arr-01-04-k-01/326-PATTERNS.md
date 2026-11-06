# Phase 326: `std.array` on numpy — Pattern Map

**Mapped:** 2026-11-02
**Files analyzed:** 24
**Analogs found:** 20 / 24

Locked: stats-slice ops only (`from_list`, `astype`, `add`, `sub`, `mul`, `div`, `mean`, `std`, `sort`, `to_list`, `item`, `shape`). `to_list` on `Array[f32]` and `Array[f64]` returns host `list[float]` (`VoxValue::Float`, f64). Conformance is the yaml row (`signature` + `handler` + `expected`) plus one golden, `examples/golden/py_array_stats.vox`. Do not edit `ROADMAP.md`, `REQUIREMENTS.md`, or `STATE.md`.

`crates/vox-python-bridge/` is not in the tree. Plan construction, typeck, the yaml, and the parity test can copy in-tree patterns. The worker, Arrow deps, and numeric round-trip test have no tracked analog until Phase 325 lands.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `contracts/python/ecosystem-support.yaml` | config | batch | `contracts/rust/ecosystem-support.yaml` | role-match |
| `contracts/python/ecosystem-support.schema.json` | config | batch | `contracts/rust/ecosystem-support.schema.json` | role-match |
| `contracts/python/plan-ir.v1.schema.json` | config | transform | `contracts/rust/ecosystem-support.schema.json` | role-match |
| `contracts/python/signature-manifest-budget.v1.json` | config | batch | `crates/vox-codegen/src/syntax_k.rs` (`measure_syntax_k_event`) | role-match |
| `crates/vox-compiler/build.rs` | utility | transform | same file, rust policy generator | exact |
| `crates/vox-compiler/src/python_array_support.rs` (new; `include!` of the generated table) | service | transform | `crates/vox-compiler/src/rust_interop_support.rs` | role-match |
| `crates/vox-compiler/tests/python_bridge_parity_test.rs` | test | batch | `crates/vox-compiler/tests/rust_ecosystem_support_parity_test.rs` | exact |
| `crates/vox-cli-ci/src/cmd_enums.rs` | route | request-response | `CiCmd::RustEcosystemPolicy` in the same file | exact |
| `crates/vox-cli/src/commands/ci/run_body.rs` | controller | request-response | `CiCmd::RustEcosystemPolicy` arm | exact |
| `contracts/operations/catalog.v1.yaml` | config | request-response | `ci.rust-ecosystem-policy` row | exact |
| `contracts/index.yaml` | config | batch | `rust-ecosystem-support` entries | exact |
| `docs/src/reference/cli.md` | config | request-response | `rust-ecosystem-policy` row | exact |
| `crates/vox-compiler/src/typeck/ty.rs` | model | transform | `Ty::List` / `to_hir_type` | exact |
| `crates/vox-compiler/src/typeck/builtins.rs` | service | request-response | `json` root binding + `lookup_method` `Ty::Map` arm | role-match |
| `crates/vox-compiler/src/builtin_registry.rs` | service | request-response | `std_root_field_ty` | exact |
| `crates/vox-compiler/src/typeck/checker/expr_field.rs` | service | request-response | `std_namespace_short_name` | exact |
| `crates/vox-compiler/src/typeck/checker/expr_ops.rs` | service | transform | `check_binary_op` / `check_arguments` | exact |
| `crates/vox-compiler/src/typeck/checker/expr.rs` | service | request-response | `MethodCall` + `lookup_method` | exact |
| `crates/vox-compiler/src/eval/value.rs` | model | transform | `VoxValue::Regex` | role-match |
| `crates/vox-compiler/src/eval/builtins.rs` | service | request-response | `call_builtin_method` `VoxValue::Regex` arm | role-match |
| `examples/golden/py_array_stats.vox` | test | request-response | `examples/golden/int_overflow_boundary.vox` + `examples/golden/float_formatting.vox` | role-match |
| `contracts/pipeline/canonical-ladder.v1.yaml` | config | batch | existing fixture rows | exact |
| `contracts/eval/complexity-budget.v1.json` | config | batch | existing fixture keys | exact |
| `contracts/eval/source-token-budget.v1.json` | config | batch | existing fixture keys | exact |
| `crates/vox-codegen/src/canonical_ladder.rs` | test | batch | `ladder_loads_twelve_fixtures` | exact |
| `crates/vox-language-surface/src/lib.rs` | test | batch | `categories_partition_lexer_keywords` | exact |
| `crates/vox-compiler/tests/golden_interp_test.rs` | test | batch | `KNOWN_INTERP_GAPS` | exact |
| ARR-02 snippet tests (`array_dtype_and_required_names`) | test | request-response | `crates/vox-compiler/src/semcov_wave17_tests.rs` | role-match |
| `crates/vox-python-bridge/worker/**` handlers | service | request-response | — | none |
| `crates/vox-python-bridge/Cargo.toml` arrow deps | config | file-I/O | — | none |
| `array_numeric_roundtrip` (i64::MIN, uint64) | test | transform | — | none |
| `vox run` golden in a temp `[python]` project | test | request-response | — | none |

## Pattern Assignments

### `contracts/python/ecosystem-support.yaml` (config, batch)

**Analog:** `contracts/rust/ecosystem-support.yaml`

Copy the mechanism (versioned yaml + schema + generated table + test). Do not copy `crate_family`, `decision`, `semantics_state`, `product_lane`, or `supported_targets`. A row is an op.

**Document shape** (lines 1–56): `schema_version: 1`, then a list of entries. The python file's list is `ops`, twelve names, no others.

```yaml
schema_version: 1
generated_from: docs/src/architecture/rust-ecosystem-support-ssot.md

support_entries:
  - crate_family: serde+serde_json
    product_lane: app
    support_tier: tier0
    boundary_owner: builtin_registry
    semantics_state: implemented
    decision: first_class
```

Each python op row needs `name`, `kind` (`plan` or `materializer`), `signature`, `handler`, and a non-null `expected`. `kind: plan` is `from_list`, `astype`, `add`, `sub`, `mul`, `div`, `mean`, `std`, `sort`. `kind: materializer` is `to_list`, `item`, `shape`. `std` carries `required_named: [correction]`. `sort` carries `required_named: [nulls]`.

### `contracts/python/ecosystem-support.schema.json` and `contracts/python/plan-ir.v1.schema.json` (config)

**Analog:** `contracts/rust/ecosystem-support.schema.json`

**Schema envelope** (lines 1–40): draft 2020-12, `$id`, `additionalProperties: false`, `required` lists every field the parity test joins. Plan-IR schema is the same envelope, with stats-slice nodes only (no `linalg`, `fft`, `where`, or slice nodes).

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://vox/contracts/rust/ecosystem-support.schema.json",
  "type": "object",
  "additionalProperties": false,
  "required": ["schema_version", "generated_from", "support_entries"]
}
```

### `crates/vox-compiler/build.rs` (utility, transform)

**Analog:** the rust registry generator in the same file.

**Deserialize + emit** (lines 1–28 and 94–157): `serde::Deserialize` the yaml, panic on an unknown label, write `OUT_DIR` with an `@generated` header. Add a second write for the array signature table. Do not fold array ops into `GENERATED_RUST_INTEROP_POLICY`.

```rust
out.push_str(
    "// @generated by vox-compiler/build.rs from contracts/rust/ecosystem-support.yaml\n",
);
out.push_str("// Do not edit manually.\n\n");
// ...
let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
fs::write(out_dir.join("rust_interop_policy.rs"), out).expect("write rust_interop_policy.rs");
```

Unknown labels panic (`decision_variant`, lines 30–37). A python row missing `signature` should fail the same way at generate time, and again in the parity test.

### `crates/vox-compiler/src/python_array_support.rs` (service, transform)

**Analog:** `crates/vox-compiler/src/rust_interop_support.rs`

**Generated table load** (lines 62–75):

```rust
include!(concat!(env!("OUT_DIR"), "/rust_interop_policy.rs"));

#[must_use]
pub fn rust_interop_policy_entries() -> &'static [RustInteropPolicyEntry] {
    GENERATED_RUST_INTEROP_POLICY
}
```

Typeck reads this table. Do not hand-write a second `match` of the twelve names in the compiler.

### `crates/vox-compiler/tests/python_bridge_parity_test.rs` (test, batch)

**Analog:** `crates/vox-compiler/tests/rust_ecosystem_support_parity_test.rs`

**Workspace root** (lines 12–19):

```rust
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}
```

**Schema gate** (lines 191–213): `jsonschema::validator_for` on the schema, `serde_yaml::from_str` of the yaml, `validator.validate`. Panic with both paths on failure.

The rust tests compare yaml fields to a generated classifier (`registry_decision_matches_classifier_for_each_crate_family_member`, lines 43–67). The python test joins three sets instead: generated signature, a real worker `def` (column 0, or the `HANDLERS` dict), and non-null `expected`. A negative test builds an in-memory row that drops one of those three and asserts `Err`. It must not delete the yaml on disk. A golden `@test` that only calls the op does not fill `expected`.

K2: compress the twelve signature strings with `vox_codegen::syntax_k::measure_syntax_k_event` and compare to `contracts/python/signature-manifest-budget.v1.json`. Do not store that number in `complexity-budget.v1.json`.

### `crates/vox-cli-ci/src/cmd_enums.rs` and `run_body.rs` (route / controller)

**Analog:** `CiCmd::RustEcosystemPolicy`

**Variant** (`cmd_enums.rs` lines 743–745) and catalog id (`cmd_enums.rs` line 1384):

```rust
/// Run rust ecosystem support parity checks (`vox-compiler` contract + classifier test).
#[command(name = "rust-ecosystem-policy")]
RustEcosystemPolicy,
```

```rust
CiCmd::RustEcosystemPolicy => Some("ci-gate/ci.rust-ecosystem-policy"),
```

Add `PythonBridgeParity` the same way, name `python-bridge-parity`, catalog id `ci.python-bridge-parity`.

**Fail-closed shell-out** (`run_body.rs` lines 472–492). A status check that does not return `Err` records a stale Pass.

```rust
CiCmd::RustEcosystemPolicy => {
    let cargo = cargo_bin();
    let st = Command::new(&cargo)
        .current_dir(&root)
        .args(["test", "-p", "vox-compiler", "--test", "rust_ecosystem_support_parity"])
        .status()?;
    if !st.success() {
        Err(anyhow!(
            "rust ecosystem policy parity failed; run `cargo test -p vox-compiler --test rust_ecosystem_support_parity`"
        ))
    } else {
        println!("rust-ecosystem-policy OK");
        Ok(())
    }
}
```

New arm: `cargo test -p vox-compiler --test python_bridge_parity`. `cargo` comes from `cargo_bin()` (PATH / broker), not `~/.cargo/bin/cargo`.

### `contracts/operations/catalog.v1.yaml`, `contracts/index.yaml`, `docs/src/reference/cli.md`

**Catalog row** (`catalog.v1.yaml` lines 8949–8975): copy `ci.rust-ecosystem-policy`. `cli.path` is `ci` / `python-bridge-parity`. `side_effect_class: spawns_process`. `ref_cli_required: true`. `reachability_required: false`. Then `vox ci operations-sync`. Do not hand-edit `contracts/cli/command-registry.yaml` or `*.generated.md`.

**Index** (`contracts/index.yaml` lines 909–923): one entry per new contract file (`kind: yaml` and `kind: json-schema`), `owner: platform-ci`, `enforced_by: [vox ci contracts-index]`.

**CLI doc** (`docs/src/reference/cli.md` line 293): one table row naming the cargo test, next to `rust-ecosystem-policy`.

### `crates/vox-compiler/src/typeck/ty.rs` (model, transform)

**Analog:** `Ty::List` and `to_hir_type`.

Add `Ty::Array(Box<Ty>)`. Do not add `HirType::Array`. `HirType` (`stmt_expr.rs` lines 111–124) already has `Generic(String, Vec<HirType>)`.

**Lowering** (lines 110–129):

```rust
Ty::List(inner) => HirType::Generic("list".into(), vec![inner.to_hir_type()]),
Ty::Option(inner) => HirType::Generic("option".into(), vec![inner.to_hir_type()]),
```

Array lowers as `HirType::Generic("Array".into(), vec![dtype.to_hir_type()])`. Add a `signature()` arm the same way as `Ty::List` (line 57): `Array[{inner}]`.

`from_list(list[int])` → `Array[i64]`. `from_list(list[float])` → `Array[f64]`. `from_list(list[bool])` → `Array[bool]`. `i32` and `f32` appear only after `astype`. `to_list` on `Array[f32]` or `Array[f64]` → `Result[list[float]]`. On integer arrays → `Result[list[int]]`. On `Array[bool]` → `Result[list[bool]]`. `item` matches that host mapping as a scalar `Result`. `shape` → `Result[list[int]]`. `mean` and `std` are legal only on `Array[f32]` and `Array[f64]` and return `Array[D]`.

### `std.array` name resolution (three files, one pattern)

Bare `array.from_list` and `std.array.from_list` are different lookups. Wire both. The json binding is the template, with one correction: `register_all` binds `JsonModule` (lines 413–421), while `std.json` uses `StdJsonNs`. For array, use one name, `StdArrayNs`, on both paths so `import std.array` does not become `Ty::ImportPlaceholder`.

**Root binding** (`builtins.rs` lines 413–421):

```rust
env.define(
    "json".into(),
    Binding {
        ty: Ty::Named("JsonModule".into()),
        mutable: false,
        kind: BindingKind::Import,
        is_deprecated: false,
    },
);
```

Copy that block with `"array"` and `Ty::Named("StdArrayNs")`. `register_hir_module` only inserts a placeholder when the name is unbound (`registration.rs` lines 25–30):

```rust
for imp in &module.imports {
    if env.lookup(&imp.item).is_none() {
        env.define(
            imp.item.clone(),
            Binding::new(Ty::ImportPlaceholder(imp.item.clone()), /* ... */),
```

`import std.array` lowers to `HirImport { module_path: ["std"], item: "array" }` (`hir/lower/mod.rs` lines 131–138). The pre-registered binding wins.

**`std.<field>`** (`builtin_registry.rs` lines 529–546): add `"array" => Ty::Named("StdArrayNs".into())` inside `std_root_field_ty`. `std` itself is `Ty::Named("StdNamespace")`, dispatched at `expr_field.rs` lines 32–38.

**Short name** (`expr_field.rs` lines 40–43 and 209–227): add `"StdArrayNs" => Some("array")` so `std.array.from_list` hits `lookup_std_namespace_field` (lines 140–150) → `std_namespace_method_ty("array", "from_list")`.

**Namespace method vs receiver method.** `from_list` is on `StdArrayNs` (`methods.insert("StdArrayNs", …)` next to the json module map at `builtins.rs` lines 1112–1140, and a `("array", "from_list")` arm in `std_namespace_method_ty`). `astype`, arithmetic, `mean`, `std`, `sort`, `to_list`, `item`, and `shape` are methods on `Ty::Array`. Add them as a dedicated arm in `lookup_method`, the way `Ty::Map` is handled before the string-key fallback (`builtins.rs` lines 1875–1916). The string-key fallback only matches `Named`, `List`, `Str`, `Int`, `Float`, `Bool`, `Result`, `Option`. A new `Ty::Array` falls through to `return None` unless the arm is added first.

`MethodCall` applies that signature (`expr.rs` lines 537–553):

```rust
if let Some(method_ty) = self.builtins.lookup_method(&obj_ty, method) {
    let bindings = match &obj_ty {
        Ty::List(inner) | Ty::Option(inner) | Ty::Stream(inner) | Ty::Set(inner) => {
            vec![inner.as_ref().clone()]
        }
        // ...
        _ => vec![],
    };
    let method_ty = self.uf.instantiate_with(&method_ty, &bindings);
    if let Ty::Fn(params, ret) = method_ty {
        self.check_arguments(&params, args, *span);
        ret.as_ref().clone()
    } else {
        Ty::Error
    }
}
```

Add `Ty::Array(inner) => vec![inner.as_ref().clone()]` so `Array[D]` methods instantiate `D`.

Do not add `"array"` to `NAMESPACE_BUILTINS` unless every method is listed there. `std_namespace_method_ty` returns `None` for an owned namespace whose method is missing (`builtin_registry.rs` lines 558–566).

### `expr_ops.rs` — arithmetic and required names (service, transform)

**Analog:** `check_binary_op` and `check_arguments` in the same file.

**Array arm goes before host promotion** (lines 47–74). `Add` already pushes a diagnostic. `Sub` / `Mul` / `Div` return `Ty::Error` with no diagnostic. Do not copy that silence. An Array arm that runs after the `(Float || Int)` promotion never sees `Array[i64]` vs `Array[f64]`.

```rust
HirBinOp::Add => {
    if l == Ty::Str || r == Ty::Str {
        Ty::Str
    } else if l == Ty::Int && r == Ty::Int {
        Ty::Int
    } else if (l == Ty::Float || l == Ty::Int) && (r == Ty::Float || r == Ty::Int) {
        Ty::Float
    } else {
        self.diags.push(Diagnostic::error(
            format!("Invalid operands for +: {l:?} and {r:?}"),
            span,
            self.source,
        ));
        Ty::Error
    }
}
```

Rule: both operands `Ty::Array` with the same dtype → `Ty::Array` of that dtype. Any other dtype pair (including `i32` vs `i64`, and int-kind vs float-kind) is a diagnostic that tells the user to call `.astype`. Leave host scalar `1 + 1.0` on the existing promotion path.

**`check_arguments` ignores names** (lines 169–171). It zips by position and reads `a.value` only.

```rust
for (e, a) in expected.iter().zip(actual.iter()) {
    let a_ty = self.check_expr(&a.value, Some(e));
    if let Err(msg) = self.uf.unify(e, &a_ty) {
```

`HirArg.name` is `Option<String>` (`stmt_expr.rs` lines 219–226). For `std` and `sort`, reject before this zip: `std` accepts only `correction=<int>`; `sort` accepts only `nulls="first"` or `nulls="last"`. Positional `std(0)` and missing names are errors. Do not default either argument.

There is no existing required-named-argument checker. `url_check.rs` lines 65–80 only rejects duplicate names. Do not reuse it.

### `eval/value.rs` and `eval/builtins.rs` (plan value)

**Analog:** `VoxValue::Regex` (`value.rs` lines 12–15) and its method arm (`builtins.rs` lines 336–369).

A plan is an interpreter value, not a `HirType`. `from_list` stores the host list on that value and does not call the worker. `astype`, arithmetic, `mean`, `std`, and `sort` return another plan. `mean` and `std` stay 0-d plans so the z-score expression is one materialization.

Only `to_list`, `item`, and `shape` call Phase 325 `Bridge::call("execute_plan", plan JSON, Arrow IPC body)`. One call per materializer. Do not construct a second `Bridge`. If `Reply` has no body bytes, add them on the Phase 325 struct. Do not add a parallel client.

**Float equality** (`value.rs` lines 271–275) is `a == b`. NaN is not equal to NaN. The golden uses `isnt` for NaN, not `is`.

```rust
(Self::Int(a), Self::Int(b)) => a == b,
(Self::Float(a), Self::Float(b)) => a == b,
```

Host f32 is widened to f64 (`f64::from(value as f32)`) before it becomes `VoxValue::Float`. Do not `f64::from_bits` on f32 bits.

**Runtime cap** (`caps.rs` lines 296–298): materializers call `caps.allows_namespace("py")`. Denial already exits 77 (`run.rs` lines 116–120):

```rust
Err(vox_compiler::eval::EvalError::CapabilityDenied { ns, method }) => {
    eprintln!(
        "vox: capability denied: {ns}.{method} — grant with --caps <token> (see isolation.md)"
    );
    std::process::exit(77);
}
```

`GATED` does not list `py` today (`caps.rs`). That arm is Phase 324/325. This phase only calls `allows_namespace("py")` once the arm exists.

`assert` is what becomes `EvalError`. `main` returning `Result::Err` still exits 0 (`run.rs` lines 107–151). The golden's `fn main` must `assert`.

### `examples/golden/py_array_stats.vox` (test, request-response)

**Analogs:** header and `fn main` from `int_overflow_boundary.vox` and `float_formatting.vox`. Effect annotation shape from `std_http_wrappers.vox` is the wrong effect (`http` is a keyword and a module capability). `@uses(python)` goes on `zscore` only.

**Header + `fn main`** (`float_formatting.vox` lines 1–16, `int_overflow_boundary.vox` lines 13–16): YAML frontmatter (`title`, `description`, `syntax_version`, `status: golden`, `category: example`, `constructs`, `training_eligible`, `difficulty`), then `fn main`. i64::MAX is the literal `9223372036854775807` (`int_overflow_boundary.vox` line 15). i64::MIN is not a Vox literal: `IntLit` parses with `parse::<i64>()` (`token.rs` lines 386–388) and drops the token on overflow. i64::MIN and uint64 overflow stay in the Rust conformance test.

`list[T]` subscript is `Option[T]` (`expr.rs` lines 866–879). Element asserts go through `.unwrap()`.

One `to_list` for the z-score of `[1.0, 3.0]` with `correction=0` asserts `[-1.0, 1.0]`. A second materializer covers NaN and ±Inf built with `1.0 / 0.0`, `-1.0 / 0.0`, and `0.0 / 0.0`. A third covers i64::MAX. The debug log (`VOX_BRIDGE_DEBUG_OPS=1`) shows three `execute_plan` lines and no `mean` or `std` methods.

### Ladder, budgets, lexer, interp sweep

**Ladder row** (`canonical-ladder.v1.yaml` lines 42–50): append one fixture. `id` matches the budget keys. `path: examples/golden/py_array_stats.vox`. Do not add `py_frame_report`, `py_tensor_infer`, or `py_escape_psd`.

```yaml
  - id: error_propagation
    path: examples/golden/error_propagation.vox
    proves: [error_propagation]
    targets: [rust-script, interp]
  - id: json_as_typed
    path: examples/golden/json_as_typed.vox
    proves: [json_as_typed]
    targets: [rust-script]
```

**Count test** (`canonical_ladder.rs` lines 71–76): `assert_eq!(ladder.fixtures.len(), 12)` becomes 13 in the same change. Rename the test so it does not keep claiming twelve.

**Budgets:** `complexity-budget.v1.json` is a flat `fixtures` map of id → integer (`hello: 120`). `source-token-budget.v1.json` is id → `{tokens, bytes}`. A ladder id with no key fails (`syntax_k.rs` lines 83–88 and 207–211). Insert one key in each file. `--update` replaces the whole map (`syntax_k.rs` lines 94–95). Measure `py_array_stats` and insert that key. Do not commit a rewrite of the other fixtures.

**Lexer freeze** (`lib.rs` lines 186–187 and 399–409): `LEXER_KEYWORDS` stays 54 strings, `"fn"` through `"form"`. `categories_partition_lexer_keywords` only checks that the three category arrays concatenate to that slice. Add `assert_eq!(LEXER_KEYWORDS.len(), 54)` and `assert!(!LEXER_KEYWORDS.contains(&"array"))` (and `astype`, `f32`, `f64`, `correction`, `nulls`) in that test module. Do not add keywords.

**Interp sweep** (`golden_interp_test.rs` lines 23–27): add `("py_array_stats.vox", "needs the uv python worker")` to `KNOWN_INTERP_GAPS`. The sweep runs every golden with `fn main` and has no bridge. The gap entry is not the phase exit.

### ARR-02 tests

**Analog:** `semcov_wave17_tests.rs` lines 42–45 and 373–387.

```rust
fn effect_diags(src: &str) -> Vec<crate::typeck::diagnostics::Diagnostic> {
    let hir = lower(src);
    check_effect_compliance(&hir, src)
}

#[test]
fn effect_check_stdlib_http_call_without_net_is_violation() {
    let src = r#"fn f() uses nothing to str { http.get("https://example.com") }"#;
    let diags = effect_diags(src);
    assert_eq!(diags.len(), 1, "...");
}
```

`array_dtype_and_required_names` uses the same parse → lower → typecheck snippet shape, and asserts a diagnostic (not a silent `Ty::Error`) for `Array[i64] + Array[f64]`, `a.std()`, `a.std(0)`, `a.sort()`, and `a.sort("first")`. Put it next to `check_binary_op` or in a `#[cfg(test)]` module that calls the typechecker. The http test is the assertion style. It is not the capability rule for array (see Shared Patterns).

## Shared Patterns

### Allowlist parity fails closed

**Source:** `rust_ecosystem_support_parity_test.rs`, `build.rs`, `run_body.rs` lines 472–492
**Apply to:** yaml, schema, generated signatures, `python_bridge_parity_test.rs`, `CiCmd::PythonBridgeParity`, catalog, index, `cli.md`

A row counts only with a typeck signature, a worker handler, and an expected value. The CI arm returns `Err` on non-zero status.

### `Array[D]` is `Ty` plus `HirType::Generic`

**Source:** `ty.rs` lines 15 and 110–129, `stmt_expr.rs` lines 111–116
**Apply to:** `ty.rs`, `builtins.rs` `lookup_method`, `expr.rs` `MethodCall`

No new `HirType` variant. `to_list` on either float plan dtype returns `Result[list[float]]` (host f64).

### Two lookups for the name `array`

**Source:** `builtins.rs` lines 413–421, `registration.rs` lines 25–30, `builtin_registry.rs` lines 529–546, `expr_field.rs` lines 32–43 and 209–227
**Apply to:** root binding, `std_root_field_ty`, `std_namespace_short_name`, `std_namespace_method_ty`

`from_list` is a namespace function. Every other v1 op is a method on `Ty::Array`. Pre-register `array` so the import does not become a placeholder.

### Materializers require `python`; plan methods do not

**Source:** `effect_check.rs` lines 286–309 and 584–600 (the pattern not to extend), `expr.rs` lines 435–553 (the arm to extend)
**Apply to:** `expr.rs` `MethodCall` when receiver is `Ty::Array` and method is `to_list`, `item`, or `shape`

`effect_check` only maps a capability when the receiver is `HirExpr::Ident`. `plan.to_list()` is not that shape. Do not add `"array" => HirCapability::Python` to `stdlib_module_capability`. That would mark `array.from_list` as an effect. Plan methods add no capability. Runtime denial uses `allows_namespace("py")` and the existing exit 77.

### Named arguments are invisible to `check_arguments`

**Source:** `expr_ops.rs` lines 151–171, `stmt_expr.rs` lines 219–226
**Apply to:** `std` (`correction=`) and `sort` (`nulls=`)

Check `HirArg.name` before the positional zip. Do not inherit numpy's defaults.

### Arithmetic diagnostic before promotion

**Source:** `expr_ops.rs` lines 43–74
**Apply to:** `Add`, `Sub`, `Mul`, `Div` on `Ty::Array`

Same dtype only. Mixed int/float text tells the user to call `.astype`. Push a diagnostic for all four ops. Do not change host scalar promotion.

### K1 ladder and K2 signatures are different numbers

**Source:** `canonical_ladder.rs` lines 71–76, `syntax_k.rs` lines 83–88 and 207–211 and 277–284
**Apply to:** ladder yaml, both budget json files, the parity test's signature ceiling

K1 is the golden's compressed WebIR size in `complexity-budget.v1.json`. K2 is the compressed signature manifest, checked inside `python-bridge-parity`. Adding a ladder row without both budget keys fails the budget commands.

### Golden assertions and the interp sweep

**Source:** `run.rs` lines 107–151, `golden_interp_test.rs` lines 23–27, `value.rs` lines 271–275, `expr.rs` lines 866–879
**Apply to:** `py_array_stats.vox`

`fn main` calls `assert`. Subscript results are `Option` and need `.unwrap()`. NaN uses `isnt`. The file is listed in `KNOWN_INTERP_GAPS` so the in-process sweep does not become the exit. The exit is `vox run` through the real worker.

### Contract and CLI registration

**Source:** `catalog.v1.yaml` lines 8949–8975, `index.yaml` lines 909–923, `cli.md` line 293, `.cursor/rules/cli-command-registry.mdc`
**Apply to:** the new CI command and the four new files under `contracts/python/`

Catalog row first, then `vox ci operations-sync`. Index every new contract file. One `cli.md` line because `ref_cli_required` is true.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/vox-python-bridge/worker/**` `HANDLERS` dict and lazy numpy/pyarrow imports | service | request-response | The crate is not in the tree. Phase 325 owns the worker. Dispatch is a dict of the twelve names, not `getattr` on numpy. Import numpy inside the handler, not at startup. `nulls=` partitions NaNs and then calls `np.sort`. `std` calls `numpy.std(a, correction=N)`. Unknown `Op.name` returns `py-plan:`. |
| `crates/vox-python-bridge/Cargo.toml` `arrow-ipc` / `arrow-array` `60.1.0` | config | file-I/O | No crate manifest to copy. Default features only (no `zstd`, no `lz4`). Add the deps after Phase 325 creates the crate. |
| `array_numeric_roundtrip` (i64::MIN `Int64Array`, uint64 above `i64::MAX` → `py:`) | test | transform | No Arrow usage in the tree. Do not put `i64::MIN` in the golden. uint64 is a worker-produced column. |
| `vox run` of the golden in a temp project with `[python]` `numpy>=2.1` and `pyarrow>=17` | test | request-response | No tracked test shells out to `vox run` against a synced venv. `golden_interp_test.rs` is the sweep to opt out of, not this harness. Count `execute_plan` frames. A skip is not the phase exit. Use the uv 3.12 worker, not PATH `python3`. |

## Metadata

**Analog search scope:** `contracts/rust/`, `contracts/eval/`, `contracts/pipeline/`, `contracts/operations/catalog.v1.yaml`, `contracts/index.yaml`, `crates/vox-compiler/src/typeck/`, `crates/vox-compiler/src/eval/`, `crates/vox-compiler/src/builtin_registry.rs`, `crates/vox-compiler/build.rs`, `crates/vox-compiler/tests/`, `crates/vox-cli/src/commands/ci/run_body.rs`, `crates/vox-cli-ci/src/cmd_enums.rs`, `crates/vox-codegen/src/canonical_ladder.rs`, `crates/vox-codegen/src/syntax_k.rs`, `crates/vox-language-surface/src/lib.rs`, `examples/golden/`, `docs/src/reference/cli.md`
**Files scanned:** 38 tracked sources (`git ls-files` non-empty for every analog path above)
**Pattern extraction date:** 2026-11-02
