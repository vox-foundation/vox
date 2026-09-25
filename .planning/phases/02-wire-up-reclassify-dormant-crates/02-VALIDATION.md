---
phase: "2"
slug: "wire-up-reclassify-dormant-crates"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-09-25"
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo-nextest (`.config/nextest.toml`); `cargo test` as fallback |
| **Config file** | `.config/nextest.toml` |
| **Quick run command** | `cargo test -p vox-orchestrator --lib` |
| **Full suite command** | `vox ci pre-push --full` |
| **Estimated runtime** | minutes on a cold target dir (build-broker queue can add substantial wait) |

---

## Sampling Rate

- **After every task commit:** `cargo test -p vox-orchestrator --lib` (SC#2) and `cargo run -p vox-cli -- ci exec-policy-contract` (SC#1 verification)
- **After every plan wave:** `cargo clippy -p vox-orchestrator --all-targets -- -D warnings` plus `cargo run -q -p vox-cli -- ci crate-edges`. (`vox ci pre-push --complete` cannot go green on this tree: `crate-edges` is already red on HEAD with three unrelated committed violations, and the installed `vox` binary is refused as stale — verified 2026-09-25.)
- **Before `/gsd-verify-work`:** the phase's tests green, and `crate-edges` output contains no violation for `vox-orchestrator -> vox-mcp-registry` (the three pre-existing violations — `vox-gui -> vox-db-types`, `vox-gui -> vox-research-shim`, `vox-research-shim -> vox-compiler` — are out of scope)
- **Max feedback latency:** bounded by the build broker; do not treat a queued build as a pass

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 02-01-T1 | 01 | 1 | REQ-dead-crate-wire-up (SC#2, D-02, D-04) | T-02-01, T-02-03 | Unrecognized MCP tool name is rejected fail-closed; every `TOOL_REGISTRY` entry still accepted | integration + unit | `cargo test -p vox-orchestrator --test tool_receipt_registry_gate` and `cargo test -p vox-orchestrator --lib -- tool_receipt::tests orchestrator::safety::tests` | ❌ W0 (created test-first in this task) | ⬜ pending |
| 02-01-T2 | 01 | 1 | REQ-dead-crate-wire-up (SC#2) | T-02-02, T-02-04 | Guard is load-bearing (fail-open mutant fails the tests); commit holds only this plan's files | mutation + audit | mutant run of the integration test; `git show --name-only` on the task commit; `cargo clippy -p vox-orchestrator --all-targets -- -D warnings` | ✅ after T1 | ⬜ pending |
| 02-02-T1 | 02 | 2 | REQ-dead-crate-wire-up (SC#1, SC#2 vox-mcp-meta clause, D-01) | T-02-07 | Exec-policy rust-fallback path calls `risk::classify` and rejects disallowed commands | integration | `cargo run -q -p vox-cli -- ci exec-policy-contract` | ✅ existing | ⬜ pending |
| 02-02-T2 | 02 | 2 | REQ-dead-crate-wire-up (SC#3, D-03) | T-02-08 | `crate-classification-2026-05-08.md` lists `vox-doc-inventory` as CORE | grep + doc lint | row/line-count/hash checks, plus `cargo run -q -p vox-doc-pipeline -- --lint-only --paths architecture/crate-classification-2026-05-08.md` | ✅ existing file | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/vox-orchestrator/src/tool_receipt.rs` or `orchestrator/safety.rs` — `#[cfg(test)]` coverage for the new fail-closed validation (currently zero in-file tests for `ToolReceiptLedger`)
- [ ] `crates/vox-orchestrator/tests/` — an integration test for `issue_tool_receipt` (none exists; CONTEXT.md D-04 asks for integration-style coverage)
- [ ] Framework install: none

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `vox-doc-inventory` no longer labelled DEAD in the classification doc | REQ-dead-crate-wire-up (SC#3) | The file is a deprecated tombstone outside any automated doc-accuracy gate | Read the row; confirm it lists the unconditional `vox-cli` dependency (`vox-cli/Cargo.toml`) and the label is CORE |
| The crate-edge exception entry matches the user's authorization | REQ-dead-crate-wire-up (SC#2) | Authorization is a human act, not a machine check | Confirm exactly one new exception (`vox-orchestrator` → `vox-mcp-registry`) and no baseline regeneration |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency bounded
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
