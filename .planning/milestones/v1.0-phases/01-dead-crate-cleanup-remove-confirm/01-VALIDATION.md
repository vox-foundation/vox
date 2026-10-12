---
phase: "1"
slug: "dead-crate-cleanup-remove-confirm"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-09-22"
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` / `cargo nextest` (workspace standard per AGENTS.md §Local CI Gate Tiers) |
| **Config file** | root `Cargo.toml` + `contracts/budgets/test-tier-budgets.v1.yaml` |
| **Quick run command** | `cargo check --workspace` |
| **Full suite command** | `vox ci pre-push --full` |
| **Estimated runtime** | ~seconds for the compile-gate; full tier per local-ci-pre-push.md |

---

## Sampling Rate

- **After every task commit:** Run `cargo check --workspace`
- **After every plan wave:** Run `vox ci pre-push --complete` (default fast tier skips clippy — see AGENTS.md's toolchain/clippy-gap pitfall)
- **Before `/gsd-verify-work`:** `cargo tree -p vox-cli --offline` + `cargo metadata` grep-clean, `vox graph coverage` clean (D-03), `cargo build -p vox-plugin-catalog` green, `git diff` on the three frozen crates empty
- **Max feedback latency:** seconds (compile-gate only; no new pub fn surface in this phase)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-* | 01 | 1 | REQ-dead-crate-delete | — / N/A | No reference to any of the 10 already-deleted crates in cargo tree/metadata | smoke/compile-gate | `cargo tree -p vox-cli --offline` (expect zero matches for the 10 crate names) | ✅ ad hoc command | ⬜ pending |
| 01-01-* | 01 | 1 | REQ-dead-crate-delete (ci.yml fix) | — / N/A | `all-features check` matrix no longer references a non-existent package | CI smoke | job run via `vox ci pre-push --act`, or next `git push` | ✅ existing CI job | ⬜ pending |
| 01-02-* | 01 | 1 | REQ-dead-crate-catalog-cleanup | — / N/A | catalog.toml parses and `vox-plugin-catalog` build succeeds after D-02's comment addition | compile-gate | `cargo build -p vox-plugin-catalog` | ✅ | ⬜ pending |
| 01-02-* | 01 | 1 | REQ-dead-crate-keep-frozen | — / N/A | Three frozen crates remain present, unmodified, documented | manual/doc-diff review | `git diff --stat -- crates/vox-workflow-runtime crates/vox-integration-tests crates/vox-test-harness` (expect empty) | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements.* This phase produces no new `pub fn` surface (RESEARCH.md confirms: only a `.github/workflows/ci.yml` matrix-line edit, a `catalog.toml` comment addition, and D-03 verification command output), so AGENTS.md's Test-First Policy trigger does not fire and no new test file/fixture is required.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Three KEEP-FROZEN crates remain present, unmodified, and documented as intentionally inactive | REQ-dead-crate-keep-frozen | No automated "is this crate frozen and annotated" check exists; confirmed by doc review of the D-02 catalog.toml comment plus a no-op diff | Read the new catalog.toml comment block; run `git diff --stat -- crates/vox-workflow-runtime crates/vox-integration-tests crates/vox-test-harness` and confirm empty |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s (compile-gate only)
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
