---
phase: "22"
slug: "deterministic-drift-gate-and-pr-impact"
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-13"
---

# Phase 22 — Validation Strategy: Deterministic Drift Gate & PR Impact

> Per-phase validation contract for feedback sampling during execution. Source: `22-RESEARCH.md` mapped to requirements DRIFT-01 through DRIFT-07 and TUT-02.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (`vox-doc-pipeline`, `vox-cli`, `vox-cli-ci`) + `.vox` script runner |
| **Config file** | `Cargo.toml`, `contracts/reports/docs-freshness/*.json`, `.github/workflows/docs-quality.yml` |
| **Quick run command** | `timeout 120s cargo test -p vox-doc-pipeline` |
| **Full suite command** | `timeout 180s cargo test -p vox-doc-pipeline -p vox-cli -p vox-cli-ci && timeout 120s cargo run -p vox-cli -- audit docs --check-drift` |
| **Estimated runtime** | unit tests ~10s; drift audit ~8s; tutorial test harness ~40s |

---

## Sampling Rate

- **After every task commit:** `timeout 120s cargo test -p vox-doc-pipeline` + `rustfmt --edition 2024 <touched .rs files>`.
- **After every plan wave:** `timeout 180s cargo test -p vox-doc-pipeline -p vox-cli -p vox-cli-ci` + `cargo run -p vox-cli -- audit docs --check-drift`.
- **Before completion:** Full CI checks (`vox ci ssot-drift`, tutorial runner, docs-quality workflow validation).
- **Max feedback latency:** ~30 s.

---

## Requirement → Verification Map

| Requirement | Behavior | Test type | Automated command | Exists |
|---|---|---|---|---|
| **DRIFT-01** | Every PR receives one PR comment edited in-place listing reader-facing docs referencing changed code anchors | CLI integration | `cargo test -p vox-cli pr_impact && cargo run -p vox-cli -- audit docs pr-impact --dry-run` | Plan 22-03 |
| **DRIFT-02** | Generated freshness ledger `contracts/reports/docs-freshness/ledger.v1.json` tracks symbol-span hashes, `last_verified_commit`, and doc states | Unit + Integration | `cargo test -p vox-doc-pipeline claims::ledger && cargo test -p vox-doc-pipeline claims::hasher` | Plan 22-01 |
| **DRIFT-03** | Authored frontmatter gains `owner`, `review_cadence`, `code_anchors`; hand-written `last_verified` is rejected with hard error; governance parity tested | Unit + Parity test | `cargo test -p vox-doc-pipeline frontmatter_governance_parity` | Plan 22-01 |
| **DRIFT-04** | Gate blocks merges introducing new broken mentions on reader-facing `status: current` docs; grandfathered baseline can only tighten | CLI + CI gate | `cargo test -p vox-cli docs_drift_gate && cargo run -p vox-cli -- audit docs --check-drift` | Plan 22-02 |
| **DRIFT-05** | Suppressions carry owner, reason, and expiry date; expired suppressions fail gate | Unit test | `cargo test -p vox-doc-pipeline claims::suppressions` | Plan 22-02 |
| **DRIFT-06** | Contributor CLI `vox audit docs --attest <path>` re-stamps verified state and commits against HEAD | CLI test | `cargo test -p vox-cli docs_attest` | Plan 22-02 |
| **DRIFT-07** | Docs Definition-of-Done emits deterministic warning if PR alters public CLI/env/contract without doc updates | Integration test | `cargo test -p vox-cli docs_definition_of_done` | Plan 22-03 |
| **TUT-02** | Every tutorial in `docs/src/tutorials/` executes hermetically end-to-end as docs-as-tests | Script test | `vox run scripts/docs/tutorial-test-runner.vox` | Plan 22-04 |
