# Phase 326: `std.array` on numpy (`ARR-01..04`, `K-01`) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-02
**Phase:** 326-std.array on numpy
**Areas discussed:** Allowlist width, Host numeric type, Conformance layout

---

## Allowlist width

| Option | Description | Selected |
|--------|-------------|----------|
| Stats slice | `from_list`, `astype`, `add`, `sub`, `mul`, `div`, `mean`, `std`, `sort`, `to_list`, `item`, `shape` | ✓ |
| Full design set | Include `linalg` and `fft` in this phase | |

**User's choice:** Make the recommended choice.
**Notes:** The yaml is the contract `ARR-04` enforces. Listing an op without a handler fails the gate. Phase 325 is not built yet, so the smaller slice is the one this phase can finish.

## Host numeric type

| Option | Description | Selected |
|--------|-------------|----------|
| Plan dtype, host f64 | `Array[f32]` stays on the plan; `to_list` returns `list[float]` | ✓ |
| Host f32 list | `to_list` on `Array[f32]` returns a distinct f32 list | |

**User's choice:** Make the recommended choice.
**Notes:** The design mapping table already sends float32 and float64 to host `float` (f64). A second host numeric type would break that table.

## Conformance layout

| Option | Description | Selected |
|--------|-------------|----------|
| Table-driven yaml | Signature, handler, and expected value per row; one golden for the end-to-end run | ✓ |
| One `@test` per op | Colocated test beside each signature | |

**User's choice:** Make the recommended choice.
**Notes:** A call with no expected value must not close a row. The golden does not replace the table.

## Claude's Discretion

The user said the tradeoff writeup was not enough to choose, and to make the recommended choices. All three areas were locked that way.

## Deferred Ideas

- `linalg`, `fft`, `where`, slicing
- Host f32 list type
- Per-op `@test` files
- Colocated-test warning proposal (separate roadmap under `.planning/research/colocated-test-warnings/`)
