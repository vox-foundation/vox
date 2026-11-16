---
phase: 92
slug: community-intelligence-flywheel-live-helios-deploy-history-scrub
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-10-19
---

# Phase 92 — Validation Strategy

> Per-phase validation contract for Community Intelligence Flywheel (`INTEL-01..05`, `TYPING-04`), Live Helios Coolify Deployment (`HELIOS-02..04`, `DBAUDIT-04`), and Codebase/Git-History Scrub (`SCRUB-01..04`).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (`vox-orchestrator`, `vox-cli`, `vox-ops-*`) + `vox check` (`scripts/verify-history-scrub.vox`) |
| **Config file** | `Cargo.toml`, `/Users/brbrainerd/dev/vox-ops/Cargo.toml` |
| **Quick run command** | `timeout 900s cargo test -p vox-orchestrator --lib intelligence_bundle` |
| **Full suite command** | `timeout 900s cargo test -p vox-orchestrator --lib intelligence_bundle && timeout 900s cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-intelligence -p vox-ops-server && ./target/debug/vox check scripts/verify-history-scrub.vox` |
| **Estimated runtime** | ~60 seconds |

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|--------|
| 92-01-01 | 01 | 1 | `INTEL-01`, `INTEL-02`, `TYPING-04` | T-92-01 | `CommunityIntelligenceBundleV1` (`#[serde(deny_unknown_fields)]`) + Ed25519 signature verification + $k \ge 20$ Laplace publisher | unit | `timeout 900s cargo test -p vox-orchestrator --lib intelligence_bundle && timeout 900s cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-intelligence` | ⬜ pending |
| 92-02-01 | 02 | 2 | `INTEL-03`, `INTEL-04`, `INTEL-05` | T-92-02 | Non-blocking hydration, $\pm 15\%$ Empirical Bayes cap, `vox telemetry intelligence`, and 100% offline equivalence | unit | `timeout 900s cargo test -p vox-orchestrator --lib community_intelligence` | ⬜ pending |
| 92-03-01 | 03 | 3 | `HELIOS-02`, `HELIOS-03`, `HELIOS-04`, `DBAUDIT-04` | T-92-03 | `vox-ops-server` live deployment on `helios` + E2E `/healthz`, `/v1/logs`, `/v1/intelligence/*`, `/v1/ludus/leaderboard` + cross-store shape audit | E2E | `timeout 900s cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-server` | ⬜ pending |
| 92-04-01 | 04 | 4 | `SCRUB-01`, `SCRUB-02`, `SCRUB-03`, `SCRUB-04` | T-92-04 | Working-tree moved-data scrub guard + deterministic `git filter-repo` history/commit-log scrub verification | contract | `timeout 300s ./target/debug/vox check scripts/verify-history-scrub.vox` | ⬜ pending |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify commands
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter
