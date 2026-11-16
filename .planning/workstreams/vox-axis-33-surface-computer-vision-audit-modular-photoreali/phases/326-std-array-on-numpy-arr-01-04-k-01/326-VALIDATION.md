---
phase: "326"
slug: "std-array-on-numpy-arr-01-04-k-01"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-02"
---

# Phase 326 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `#[test]` in `vox-compiler` and `vox-python-bridge`; golden via `vox run` |
| **Config file** | none — cargo tests. Ladder: `contracts/pipeline/canonical-ladder.v1.yaml` |
| **Quick run command** | `cargo test -p vox-compiler --test python_bridge_parity` |
| **Full suite command** | `cargo test -p vox-compiler --test python_bridge_parity` plus the typeck tests, `cargo test -p vox-language-surface categories_partition_lexer_keywords`, and `vox run` of `examples/golden/py_array_stats.vox` once Phase 325 has landed |
| **Estimated runtime** | ~60 seconds for the compiler tests; the golden waits on the worker |

---

## Sampling Rate

- **After every task commit:** `cargo test -p vox-compiler --test python_bridge_parity` and the typeck tests named in the plan
- **After every plan wave:** those, plus `cargo test -p vox-language-surface categories_partition_lexer_keywords`
- **Before `/gsd-verify-work`:** parity gate green, negative row red when a field is removed, `vox run` golden green through the real worker, `vox ci k-complexity-budget` and `vox ci source-token-budget` green
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 326-01 | 01 | 1 | ARR-04 | T-326-01 | Unknown op name rejected; row without signature, handler, or expected value fails | unit | `cargo test -p vox-compiler --test python_bridge_parity` | ❌ W0 | ⬜ pending |
| 326-02 | 01 | 1 | ARR-02 | T-326-02 | Mixed dtype and missing `correction=` / `nulls=` are compile errors | unit | `cargo test -p vox-compiler array_dtype_and_required_names` | ❌ W0 | ⬜ pending |
| 326-03 | 01 | 1 | ARR-01, ARR-03 | T-326-03 | Golden asserts `[-1.0, 1.0]` plus NaN/Inf/i64::MAX; one `execute_plan` per materializer | e2e | `vox run` of `examples/golden/py_array_stats.vox` | ❌ W0 | ⬜ pending |
| 326-04 | 01 | 1 | ARR-03 | T-326-04 | i64::MIN round-trips; uint64 above i64::MAX errors with `py:` | unit | `cargo test -p vox-python-bridge array_numeric_roundtrip` | ❌ W0 | ⬜ pending |
| 326-05 | 01 | 1 | K-01 | — | Ladder and budget rows exist; `LEXER_KEYWORDS` stays 54 | unit | `cargo test -p vox-language-surface categories_partition_lexer_keywords` and `vox ci k-complexity-budget` | ❌ rows | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/vox-compiler/tests/python_bridge_parity_test.rs` — ARR-04, including the negative synthetic row
- [ ] Typeck tests for mixed dtype and required names — ARR-02
- [ ] `examples/golden/py_array_stats.vox` plus a `vox run` test — ARR-01 and ARR-03
- [ ] `array_numeric_roundtrip` on the bridge crate — i64::MIN and uint64 — ARR-03
- [ ] Ladder, complexity-budget, and source-token-budget rows — K-01
- [ ] Phase 325 landed. Without the worker the golden is not a phase exit.

---

## Manual-Only Verifications

All phase behaviors have automated verification. The golden stays skipped, not passed, until Phase 325's worker runs `py_bridge_hello`.

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
