---
phase: 91
slug: social-media-syndication-multi-user-ludus-extraction-3-way-g
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-10-19
---

# Phase 91 — Validation Strategy

> Per-phase validation contract for Social Media Syndication Extraction (`SOCIAL-01..05`), 3-Way Gamification/Ludus SoC Split (`LUDUS-SOC-01..05`), and Zero-Cheating Leaderboard Surfaces (`LEADERBOARD-01..05`).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` + Vitest (`crates/vox-gui/ui`) + `vox-doc-pipeline --lint-only` |
| **Config file** | `Cargo.toml`, `/Users/brbrainerd/dev/vox-ops/Cargo.toml`, `crates/vox-gui/ui/vite.config.ts` |
| **Quick run command** | `timeout 900s cargo test -p vox-publisher -p vox-gamify -p vox-db` |
| **Full suite command** | `timeout 900s cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-syndication -p vox-ops-ludus && timeout 900s cargo test -p vox-publisher --all-features && timeout 900s cargo test -p vox-db -p vox-gamify -p vox-orchestrator -p vox-orchestrator-mcp && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Gamify/LeaderboardArenaPanel.test.tsx && ./target/debug/vox-doc-pipeline --lint-only --paths community/leaderboard.mdx` |
| **Estimated runtime** | ~90 seconds |

---

## Sampling Rate

- **After every task commit:** Run the task's `<automated>` verification command.
- **After every plan wave:** Run the full suite command above.
- **Max feedback latency:** 90 seconds.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|--------|
| 91-01-01 | 01 | 1 | `SOCIAL-01` | T-91-01 | 9 live social adapters extracted to `vox-ops-syndication` | unit | `timeout 900s cargo check --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-syndication` | ⬜ pending |
| 91-01-02 | 01 | 1 | `SOCIAL-02` | T-91-02 | `SELECT ... FOR UPDATE SKIP LOCKED` outbox + `idempotency_key` deduplication | unit | `timeout 900s cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-syndication` | ⬜ pending |
| 91-02-01 | 02 | 2 | `SOCIAL-03`, `SOCIAL-04` | T-91-03 | `SyndicationHandoffPayloadV1` (`#[serde(deny_unknown_fields)]`) + 9 adapters pruned from `vox-publisher` | unit | `timeout 900s cargo test -p vox-publisher --all-features` | ⬜ pending |
| 91-02-02 | 02 | 2 | `SOCIAL-04`, `SOCIAL-05` | T-91-04 | `vox scientia syndication export` + `Clavis` secret parity | unit | `timeout 900s cargo test -p vox-secrets && timeout 900s cargo check -p vox-cli` | ⬜ pending |
| 91-03-01 | 03 | 2 | `LUDUS-SOC-01`, `LUDUS-SOC-02` | T-91-05 | `vox_db::ops_agent_telemetry` extraction + shadow telemetry (`hint_telemetry`, `lsp_telemetry`) deletion | unit | `timeout 900s cargo test -p vox-db -p vox-orchestrator` | ⬜ pending |
| 91-03-02 | 03 | 2 | `LUDUS-SOC-03`, `LUDUS-SOC-04`, `LUDUS-SOC-05` | T-91-06 | Multi-user Ludus extracted to `vox-ops-ludus`; solo offline Ludus + FSRS verified | unit | `timeout 900s cargo test -p vox-gamify && timeout 900s cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-ludus` | ⬜ pending |
| 91-04-01 | 04 | 3 | `LEADERBOARD-01`, `LEADERBOARD-02` | T-91-07 | Zero client score writes, `BLAKE3` hash chain, `AntiCheatPolicyV1`, `Ed25519`-signed `VerifiedLeaderboardSnapshotV1` | unit | `timeout 900s cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-ludus && timeout 900s cargo test -p vox-gamify -p vox-orchestrator-mcp --lib gamify` | ⬜ pending |
| 91-04-02 | 04 | 3 | `LEADERBOARD-03`, `LEADERBOARD-04`, `LEADERBOARD-05` | T-91-08 | `VoxLeaderboardArena.astro` (`voxlang.org`) + `LeaderboardArenaPanel.tsx` (`vox-gui`) + Cryptographic Proof Drawer + offline fallback | unit / UI | `./target/debug/vox-doc-pipeline --lint-only --paths community/leaderboard.mdx && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Gamify/LeaderboardArenaPanel.test.tsx` | ⬜ pending |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify commands
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] No watch-mode flags
- [x] Feedback latency < 90s
- [x] `nyquist_compliant: true` set in frontmatter
