---
phase: "83"
slug: "core-crate-parser-ir-diagnostics"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-16"
---

# Phase 83 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo-nextest 0.9.143 + insta 1.48 (json) + proptest 1.11 |
| **Config file** | `.config/nextest.toml` (no per-crate config) |
| **Quick run command** | `cargo nextest run -p vox-diagram <filter>` |
| **Full suite command** | `cargo nextest run -p vox-diagram` |
| **Estimated runtime** | ~60 seconds (after first build) |

---

## Sampling Rate

- **After every task commit:** `cargo check -p vox-diagram --all-targets` + the task's new test filter
- **After every plan wave:** `cargo nextest run -p vox-diagram`
- **Before `/gsd-verify-work`:** Full crate suite green; hosted CI is authoritative for workspace gates (`vox-arch-check`, `crate-edges`, `crate-api-sync`, `ssot-drift`, hakari)
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

| Requirement | Behavior | Test Type | Automated Command | File Exists | Status |
|-------------|----------|-----------|-------------------|-------------|--------|
| DGM-01 | Every construct parses; IR JSON stable across runs | integration + insta | `cargo nextest run -p vox-diagram graph_constructs` | ❌ W0 | ⬜ pending |
| DGM-01 | No HashMap/HashSet in src; IR byte-identical | guard | `cargo nextest run -p vox-diagram determinism_guard ir_json_is_stable` | ❌ W0 | ⬜ pending |
| DGM-03 | k independent mistakes → k diagnostics with code/severity/span | integration | `cargo nextest run -p vox-diagram diagnostics_all_errors` | ❌ W0 | ⬜ pending |
| DGM-03 | Stable code registry | unit | `cargo nextest run -p vox-diagram codes_` | ❌ W0 | ⬜ pending |
| DGM-04 | Seeded broken set repairs to zero errors; Mermaid-ism P1 rows fixed | integration + insta | `cargo nextest run -p vox-diagram fixes_repair mermaid_` | ❌ W0 | ⬜ pending |
| DGM-04 | mutate-and-repair restores IR; hostile fix spans valid | proptest | `cargo nextest run -p vox-diagram mutate_and_repair hostile_` | ❌ W0 | ⬜ pending |
| DGM-05 | fmt idempotent + parse round-trip; canonical example fixed point | proptest + golden | `cargo nextest run -p vox-diagram fmt_props canonical_example_is_fixed_point` | ❌ W0 | ⬜ pending |
| DGM-06 | Caps at N ok / N+1 one `diagram/limit-*`; container endpoint; adversarial corpus terminates | integration | `cargo nextest run -p vox-diagram limits_adversarial` | ❌ W0 | ⬜ pending |
| support | Fence scanner, sequence IR serde, grammar card dogfood | unit | `cargo nextest run -p vox-diagram fence_scan sequence_ir grammar_card` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/vox-diagram/` scaffold (Cargo.toml, lib.rs, span/codes/diag/limits modules)
- [ ] `tests/fixtures/{valid,broken,mermaid}/` incl. the CONTEXT canonical example verbatim
- [ ] shared proptest generators in `tests/support/mod.rs`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Mutation proofs (a)–(e) from RESEARCH.md | DGM-03/04/06 | Deliberate break-then-restore of guards | Break guard, confirm named test fails, restore; record in SUMMARY |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
