---
phase: "331"
slug: "tolerant-cli-ergonomics-precomputed-semantic-routing"
status: draft
nyquist_compliant: true
wave_0_complete: true
created: "2026-11-01"
---

# Phase 331 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` |
| **Config file** | `crates/vox-cli/Cargo.toml`, `crates/vox-cli-core/Cargo.toml` |
| **Quick run command** | `timeout 900s cargo test -p vox-cli --lib argv_normalize` |
| **Full suite command** | `timeout 900s cargo test -p vox-cli-core && timeout 900s cargo test -p vox-cli --lib` |
| **Estimated runtime** | ~25 seconds |

---

## Sampling Rate

- **After every task commit:** Run `timeout 900s cargo test -p vox-cli --lib argv_normalize`
- **After every plan wave:** Run `timeout 900s cargo test -p vox-cli-core && timeout 900s cargo test -p vox-cli --lib`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 331-01-01 | 01 | 1 | CLI-TOL-01 | T-331-01 | Unambiguous prefix unfolding rejects ambiguous prefixes; global flags hoist safely | unit | `timeout 900s cargo test -p vox-cli --lib prefix_and_global_flags` | ✅ | ⬜ pending |
| 331-02-01 | 02 | 2 | CLI-TOL-02, CLI-TOL-03 | T-331-02 | Mutating commands never auto-execute on non-TTY/agent calls; exit code 2 + `aci.error_recovery` | unit | `timeout 900s cargo test -p vox-cli --lib argv_normalize` | ✅ | ⬜ pending |
| 331-03-01 | 03 | 2 | CLI-TOL-04 | T-331-03 | Antonym banlist and cosine margin gate prevent destructive alias collisions | unit | `timeout 900s cargo test -p vox-cli --lib cli_synonyms` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-11-01
