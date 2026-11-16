# Phase 326: `std.array` on numpy (`ARR-01..04`, `K-01`) - Context

**Gathered:** 2026-11-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Vox code computes with numpy through `std.array`: a closed, typed, snake_case vocabulary that builds pure lazy plans and crosses into the Python worker only at a materializer. This phase ships the stats slice that `examples/golden/py_array_stats.vox` needs, plus the parity gate that fails when an allowlisted op lacks a signature, a handler, or an expected value.

Depends on Phase 325 (`vox-python-bridge`). Do not spawn a second worker or invent a data plane here.

</domain>

<decisions>
## Implementation Decisions

The user delegated the three open choices. The locks below are the recommended ones. Section 4 of the design spec stays normative for everything else.

### Allowlist width
- **D-01:** v1 `contracts/python/ecosystem-support.yaml` lists only the stats slice: `from_list`, `astype`, `add`, `sub`, `mul`, `div`, `mean`, `std`, `sort`, `to_list`, `item`, `shape`. `linalg`, `fft`, `where`, and slicing are absent from that file until a later phase adds their handlers. — **Reversibility:** reversible — adding a row later does not change the type of an existing op.

### Host numeric type
- **D-02:** `Array[f32]` and `Array[f64]` are plan types. `to_list` on either returns `list[float]` (host f64). The golden asserts those f64 values, including NaN and ±Inf. No host f32 list type. — **Reversibility:** costly — `to_list`'s return type is the caller-facing contract; widening it later changes every caller.

### Conformance layout
- **D-03:** `ARR-04` is a table-driven check over the yaml. A row counts only when it has a typeck signature, a worker handler, and an expected value for a concrete input. `examples/golden/py_array_stats.vox` is the one end-to-end materialization. A call inside `@test` with no expected value does not satisfy the row. — **Reversibility:** reversible — the table can grow without a new declaration form.

### Claude's Discretion
- User said the three tradeoffs did not have enough information to choose, and to make the recommended choices. D-01, D-02, and D-03 are those choices.
- Op bodies follow Array API names already required by the design (`correction=`, `nulls=`, `astype`). Do not rename them.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Design
- `docs/src/architecture/vox-python-interop-design-2026-11-01.md` §4.1–§4.4 — pure plans, materializer list, `Array[D]`, host mapping, zscore sketch
- `.planning/ROADMAP.md` — Phase 326 success criteria
- `.planning/REQUIREMENTS.md` — `ARR-01`, `ARR-02`, `ARR-03`, `ARR-04`, `K-01`

### Pattern to copy
- `contracts/rust/ecosystem-support.yaml` — allowlist shape for `contracts/python/ecosystem-support.yaml`
- `crates/vox-compiler/tests/rust_ecosystem_support_parity_test.rs` — parity test that fails when a registry row is incomplete
- `crates/vox-compiler/src/rust_interop_support.rs` — registry loader

### Phase dependency
- `.planning/phases/325-bridge-runtime-environment-br-01-04-gov-01/325-CONTEXT.md` — worker, frames, caps, timeouts. `std.array` calls that worker; it does not replace it.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `contracts/rust/ecosystem-support.yaml` and `rust_ecosystem_support_parity_test.rs`: the python parity gate should fail the same way, on a missing signature, handler, or expected value.
- `Array[D]` is a std `Named`/`Generic` type. The design forbids a new `HirType` variant.

### Established Patterns
- Materializers return `Result[T]` and require `uses python`. Plan methods do not.
- `to_list` is valid on 1-D arrays only. The stats golden takes a 1-D list, so `reshape` is not in v1.
- `LEXER_KEYWORDS` gains nothing (`K-01`).

### Integration Points
- Phase 325 worker: one round-trip per materialization. `mean` and `std` return plans; `to_list` or `item` executes them.
- `vox ci python-bridge-parity` is the command `ARR-04` names. It does not exist yet. Model it on the rust ecosystem parity test.

</code_context>

<specifics>
## Specific Ideas

The zscore sketch in the design is the shape of a real call, not the golden's full op set:

```vox
let a = array.from_list(xs).astype(f64)
return ((a - a.mean()) / a.std(correction=0)).to_list()
```

`py_array_stats.vox` must assert that output and must show one worker round-trip for that materialization.

</specifics>

<deferred>
## Deferred Ideas

- `linalg.{matmul,solve,inv,norm}` and `fft.{fft,ifft}` — design §4.4's larger set. A later phase adds the yaml rows and the handlers together.
- `where` and slicing — same rule.
- A host f32 list type — rejected for v1 by D-02.
- One `@test fn` per op — rejected for v1 by D-03. The colocated-test warning proposal in `.planning/research/colocated-test-warnings/` is a separate effort and is not part of Phase 326.

</deferred>

---

*Phase: 326-std.array on numpy*
*Context gathered: 2026-11-02*
