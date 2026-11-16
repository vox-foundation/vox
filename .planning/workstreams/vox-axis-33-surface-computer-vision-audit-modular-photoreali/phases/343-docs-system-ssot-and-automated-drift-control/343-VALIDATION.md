---
phase: "343"
slug: "docs-system-ssot-and-automated-drift-control"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-06"
---

# Phase 343 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `#[test]` via `cargo nextest` (`cargo test -p` fallback); `serde_yaml` workflow probes |
| **Config file** | `.config/nextest.toml` (profiles `ci`, `ci-gate`) |
| **Quick run command** | `cargo nextest run -p <touched crate> <filter>` (backgrounded; never block the foreground on cargo) |
| **Full suite command** | hosted CI on the draft PR (`gate` job); no workspace-wide local runs |
| **Estimated runtime** | ~60–300 seconds per crate-scoped run |

---

## Sampling Rate

- **After every task commit:** `cargo check -p <touched crate>` + the new tests only
- **After every plan wave:** `vox ci pre-push` (fast) from a freshly built binary, plus `vox ci ssot-drift`
- **Before `/gsd-verify-work`:** hosted CI green on the draft PR
- **Max feedback latency:** 300 seconds

---

## Per-Task Verification Map

| Req | Behavior | Test Type | Automated Command | File Exists | Status |
|-----|----------|-----------|-------------------|-------------|--------|
| DOCS-DRIFT-01 | stage table ↔ `scripts/ssot-regen.vox` ↔ `lefthook.yml` parity; aggregate failures | unit | `cargo nextest run -p vox-cli ssot_drift` | ❌ W0 | ⬜ pending |
| DOCS-DRIFT-02 | overlay contradiction fixtures fail; anchors present in AGENTS.md; mutation proof | unit | `cargo nextest run -p vox-cli-ci overlay_parity` | ❌ W0 | ⬜ pending |
| DOCS-DRIFT-03 | unknown subcommand/flag, retired status, "planned" adjacency, feature-gated union | unit | `cargo nextest run -p vox-cli docs_cli_claims` | ❌ W0 | ⬜ pending |
| DOCS-DRIFT-04 | silent-skip sites fail on missing input; orphan graph; baseline rejects growth | unit | `cargo nextest run -p vox-cli-ci check_links orphan` | ❌ W0 | ⬜ pending |
| DOCS-DRIFT-05 | ledger write skipped when unchanged; scheduled workflow passes policy guards | unit + workflow probe | `cargo nextest run -p vox-doc-pipeline ledger` ; `cargo nextest run -p vox-cli --test ci_workflow_contract` | ❌ W0 | ⬜ pending |
| DOCS-DRIFT-06 | every SSOT §4 row has a wired probe; every §6 bullet has a gap id | unit | `cargo nextest run -p vox-cli docs_system_parity` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Re-measure CLI-mention (≈87) and orphan-page (≈48 reader-facing) counts with the in-process catalog before choosing enforce vs. baseline
- [ ] Mutation-proof notes per new guard (delete input / table row / overlay fix, with before/after presence check)
- [ ] No framework install needed

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Nightly ledger/lychee bot opens a PR only on real change | DOCS-DRIFT-05 | Needs a scheduled hosted run | `gh workflow run <workflow>` twice on an unchanged tree; second run opens no PR |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 300s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
