---
phase: "338"
slug: "spoke-aware-gating-and-hub-repos"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-05"
---

# Phase 338 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source: 338-RESEARCH.md §"Validation Architecture". Project rule: executors run `cargo check -p <crate>`
> plus the new tests only; hosted CI is the gate. Route cargo through the build broker and background long builds.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` / nextest; `tempfile` fixtures; `wiremock` for HTTP stubs |
| **Config file** | workspace `Cargo.toml`, `.config/nextest.toml` (profile `ci-gate`) |
| **Quick run command** | `cargo test -p vox-eval --lib -- collateral_evidence episode` |
| **Full suite command** | `cargo test -p vox-eval --lib`; `cargo test -p vox-populi --features mens-hf-hub --lib hub_`; `cargo test -p vox-ml-cli --features gpu --lib -- eval_gate eval_collateral item_checkpoint episodes` |
| **Estimated runtime** | ~120-600 seconds per crate after a warm build (unit tests only; no GPU) |

---

## Sampling Rate

- **After every task commit:** `cargo check -p <crate>` + that task's new tests
- **After every plan wave:** the per-crate full commands above for crates touched
- **Before `/gsd-verify-work`:** hosted CI green; locally `vox ci crate-edges`, `cargo run -p vox-arch-check` if an edge/feature moved
- **Max feedback latency:** 600 seconds

---

## Per-Task Verification Map

| Req | Behavior | Test Type | Automated Command | File Exists | Status |
|-----|----------|-----------|-------------------|-------------|--------|
| SPOKE-01a | publish refuses report without current slice pins | unit + mutation | `cargo test -p vox-populi --features mens-hf-hub --lib hub_sync::tests::stale_pins` | ❌ W0 | ⬜ pending |
| SPOKE-01b | publish refuses wrong rule / alpha | unit + mutation | `cargo test -p vox-populi --features mens-hf-hub --lib hub_sync::tests::stale_rule_alpha` | ❌ W0 | ⬜ pending |
| SPOKE-01c | publish refuses wrong/absent grader | unit + mutation | `cargo test -p vox-populi --features mens-hf-hub --lib hub_sync::tests::stale_grader` | ❌ W0 | ⬜ pending |
| SPOKE-01d | card + serve + eval-gate share one check | unit | `cargo test -p vox-populi --features mens-hf-hub --lib hub_card` | partial | ⬜ pending |
| SPOKE-02 | every gate key wired with producer test, or deleted | unit | `cargo test -p vox-ml-cli --features gpu --lib eval_gate` | partial | ⬜ pending |
| SPOKE-03 | spoke → policy routing; `include_lanes` inference | unit | `cargo test -p vox-ml-cli --lib spoke_route` | ❌ W0 | ⬜ pending |
| SPOKE-04 | canonical policy hash stable under key order; `gate_state` truth table | unit + mutation | `cargo test -p vox-eval --lib -- gate_state policy_hash` | ❌ W0 | ⬜ pending |
| SPOKE-05 | spoke in card/receipt/manifest; repo id unchanged | unit + golden | `cargo test -p vox-populi --features mens-hf-hub --lib -- hub_card hub_repo` | partial | ⬜ pending |
| SPOKE-06 | episode scorer: oracle 100%, each control fails its behavior | unit + mutation | `cargo test -p vox-eval --lib episode` | ❌ W0 | ⬜ pending |
| SPOKE-06b | live tool-catalog drift | unit | `cargo test -p vox-orchestrator-mcp --lib tool_catalog_lock` | ❌ W0 | ⬜ pending |
| SPOKE-07 | vox-lang gate on hidden-test compile-and-test; rust bench | unit | `cargo test -p vox-ml-cli --features gpu --lib eval_gate::tests::humaneval` | ❌ W0 | ⬜ pending |
| SPOKE-08 | dataset id fn, hash manifest, licence policy | unit | `cargo test -p vox-populi --features mens-hf-hub --lib hub_dataset` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Shared test helper building a valid collateral report from `reasoning_slices_json` + pins (replaces the `{"status":"pass"}` fixture at `hub_sync.rs:1383`)
- [ ] `contracts/mens/spoke-gate-state.v1.schema.json` + schema-validating test
- [x] ~~Probe: HF `validate-yaml` with a `vox:` block~~ (dropped: plan 06 records the spoke only via standard HF tags)
- [ ] Test counting held-out HumanEval-Vox fixtures
- [ ] Episode fixture authoring guide + load-time leakage check

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| 0.6B base control fails the episode suite; scripted oracle passes | SPOKE-06 | GPU run, ~20-55 min at 0.6B | Run the episode eval on Qwen3-0.6B base after human approval of wall-clock; record result in the receipt |
| Dataset repo push | SPOKE-08 | One-way Hub action; ids and visibility need user sign-off | Human checkpoint before any push |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 600s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
