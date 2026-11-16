---
phase: "327"
slug: "std-frame-on-polars-frm-01-03"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-03"
---

# Phase 327 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (`vox-compiler` integration and unit tests). The golden runs through `vox run` after the polars checkpoint, same shape as `py_array_stats_run`. |
| **Config file** | none — Cargo tests under `crates/vox-compiler/tests/` |
| **Quick run command** | `cargo test -p vox-compiler --test python_bridge_parity` |
| **Full suite command** | `cargo test -p vox-compiler --test python_bridge_parity --test py_frame_report_run` plus `frame_plan_collect_is_one_round_trip`, `frame_column_decode`, `method_and_or_parse`, and `scan_csv_outside_fs_ro_never_calls_worker` |
| **Estimated runtime** | not measured — each command is one filtered cargo test |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p vox-compiler --test python_bridge_parity` and the new unit test named in that task
- **After every plan wave:** Run the quick command plus `frame_column_decode` and `scan_csv_outside_fs_ro_never_calls_worker`
- **Before `/gsd-verify-work`:** Those tests must be green. The golden `py_frame_report_run` runs only after the polars checkpoint. A skip or `#[ignore]` is not a pass.
- **Max feedback latency:** one scoped cargo test

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 327-w0-parity | — | 0 | FRM-01 | — | Frame allowlist rows have signature, handler, and expected. Array rows stay. | unit | `cargo test -p vox-compiler --test python_bridge_parity` | ❌ W0 | ⬜ pending |
| 327-w0-plan | — | 0 | FRM-01 | — | Plan build records zero worker calls. `collect` records one. | unit | `cargo test -p vox-compiler frame_plan_collect_is_one_round_trip` | ❌ W0 | ⬜ pending |
| 327-w0-decode | — | 0 | FRM-02 | — | Decimal, UTC DateTime, Date, Duration decode. A null cell is `py-plan:`. A naive timestamp names `.replace_time_zone("UTC")` and builds no value. | unit | `cargo test -p vox-python-bridge frame_column_decode` | ❌ W0 | ⬜ pending |
| 327-w0-parse | — | 0 | FRM-02 | — | `.and(` and `.or(` parse as method names. | unit | `cargo test -p vox-compiler method_and_or_parse` | ❌ W0 | ⬜ pending |
| 327-w0-path | — | 0 | FRM-03 | — | A path outside `fs:ro` is `CapabilityDenied` and does not spawn a worker request. `..` never reaches the worker. | unit | `cargo test -p vox-compiler scan_csv_outside_fs_ro_never_calls_worker` | ❌ W0 | ⬜ pending |
| 327-w0-golden | — | 0 | FRM-01 | — | `scan_csv → filter → group_by/agg → sort → collect` passes with one round-trip per `collect`. | integration | `cargo test -p vox-compiler --test py_frame_report_run` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `contracts/python/frame-support.yaml` — the frame allowlist rows
- [ ] `contracts/python/frame-plan-ir.v1.schema.json` — frame names only
- [ ] `crates/vox-compiler/tests/python_bridge_parity_test.rs` — second test; the array test stays
- [ ] `crates/vox-compiler/src/eval/frame_plan.rs` — zero worker calls until `collect`
- [ ] `crates/vox-python-bridge/src/frame_columns.rs` — dtype fixture without polars
- [ ] `crates/vox-compiler/tests/py_frame_report_run.rs` — after the polars checkpoint only
- [ ] Parser test that `.and(` and `.or(` are method calls
- [ ] Existing infrastructure covers the runner. Cargo test is already installed.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Install the named polars pin | FRM-01 | The operator must approve the PyPI project before any install. Candidate is `polars==2.1.0`. It is not approved. | Stop at the checkpoint. Do not install until the operator names that approval. Then run `py_frame_report_run`. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < one scoped cargo test
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
