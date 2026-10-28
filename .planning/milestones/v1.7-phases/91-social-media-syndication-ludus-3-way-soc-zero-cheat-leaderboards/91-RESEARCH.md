# Phase 91: Social Media Syndication, Multi-User Ludus Extraction, 3-Way Gamification SoC Split & Zero-Cheating Leaderboard Surfaces - Research

**Researched:** 2026-10-19
**Domain:** Social Syndication Outbox (`vox-ops-syndication`), 3-Way Gamification SoC (`vox-db` / `vox-ops-ludus` / `vox-gamify`), Zero-Cheating `BLAKE3` + `Ed25519` Leaderboard, and v0 / Claude Design GUI Surfaces (`voxlang.org` + `vox-gui`)
**Confidence:** HIGH (grounded in direct codebase inspection and `centralized-postgres-telemetry-vox-ops-ssot-2026.md` §§6, 9, 11)

## Summary

Phase 91 executes three tightly coupled Separation of Concerns (SoC) and surface upgrades:
1. **Social Media Syndication Extraction (`SOCIAL-01..05`):** Moves the 9 live social/community adapters (`twitter`, `reddit`, `youtube`, `discord`, `bluesky`, `mastodon`, `linkedin`, `hacker_news`, `opencollective`) and `canary.rs` out of `crates/vox-publisher/src/adapters/` into `~/dev/vox-ops/crates/vox-ops-syndication/` backed by PostgreSQL `syndication.outbox` (`SELECT ... FOR UPDATE SKIP LOCKED`), while keeping `vox-publisher` 100% self-contained for local scholarly artifact preparation (`CITATION.cff`, Crossref, BibTeX, Zenodo/OpenReview/arXiv local bundle generation, `rss.rs`, `github.rs`, `researchgate.rs`) and `SyndicationHandoffPayloadV1` handoff (`vox scientia syndication export`).
2. **3-Way Ludus & Gamification SoC Split (`LUDUS-SOC-01..05`):**
   - **Tier 1 (Core Agent Operational Telemetry → `vox-db`):** Moves `crates/vox-gamify/src/db/agent_telemetry.rs` into `crates/vox-db/src/store/ops_agent_telemetry.rs`, updating the 7 call sites across `vox-orchestrator` and `vox-orchestrator-mcp`, and deletes shadow telemetry modules (`crates/vox-gamify/src/db/hint_telemetry.rs`, `crates/vox-gamify/src/lsp_telemetry.rs`).
   - **Tier 2 (Multi-User Competitive/Social Ludus → `vox-ops-ludus`):** Moves `collegium.rs`, `arena.rs`, `leaderboards.rs`, `disputes.rs`, and CLI `ludus/{collegium,arena,disputes,auth,sync}.rs` into `~/dev/vox-ops/crates/vox-ops-ludus` backed by `ludus_community.*` on `helios-postgres`.
   - **Tier 3 (Solo Developer Progression & FSRS Discovery → Local `vox-gamify`):** Retains `LudusProfile`, `Companion`, `Quest`, `Battle`, `StreakTracker`, `AchievementTracker`, `shop`, `sprite_svg`, and `discovery/{fsrs,rank,ledger,scientia_kudos}.rs` 100% offline in `vox-gamify`.
3. **Zero-Cheating Server-Authoritative Leaderboard & v0 / Claude Design GUI Surfaces (`LEADERBOARD-01..05`):**
   - Replaces the legacy local-SQLite `SELECT user_id, level, xp FROM gamify_profiles ORDER BY xp DESC` leaderboard and client-side GitHub polling with `ludus_community.verified_contributions` (`UNIQUE (upstream_provider, upstream_event_id)`, `BLAKE3` `row_chain_hash_hex`, `AntiCheatPolicyV1`) and `Ed25519`-signed `VerifiedLeaderboardSnapshotV1` (`#[serde(deny_unknown_fields)]`).
   - Renders the dynamic leaderboard on both `voxlang.org` (`docs-astro/src/components/VoxLeaderboardArena.astro` mounted at `docs/src/community/leaderboard.mdx`) and `vox-gui` (`crates/vox-gui/ui/src/components/surfaces/Gamify/LeaderboardArenaPanel.tsx` in `GamifyView.tsx`) using v0 / Claude Design tokens (`#FAF8F5` / `#0C0C0E`, `1px` hairlines, `tabular-nums`, `#D97757` terracotta & `#10B981` emerald accents), 4 lenses (including `(model_id, harness_id, config_digest)` tuples with 95% Wilson CIs for the `5-Spoke Model & Mens Arena` lens), an interactive **Cryptographic Proof Drawer**, and 100% static/offline fallback.

## Codebase Call-Site Audit

### 1. `vox_gamify::db::agent_telemetry` Call Sites to Rewire to `vox_db::ops_agent_telemetry` (`LUDUS-SOC-01`)
| Crate & File | Line(s) | Current Symbol | Target Symbol |
|---|---|---|---|
| `crates/vox-orchestrator/src/orchestrator/task_dispatch/submit/task_submit.rs` | 280 | `vox_gamify::db::get_tenant_monthly_token_usage(&db, tenant_id)` | `vox_db::ops_agent_telemetry::get_tenant_monthly_token_usage(&db, tenant_id)` |
| `crates/vox-orchestrator-mcp/src/a2a_tools.rs` | 572 | `vox_gamify::db::insert_event(...)` | `vox_db::ops_agent_telemetry::insert_event(...)` |
| `crates/vox-orchestrator-mcp/src/dei_tools/control.rs` | 136, 197 | `vox_gamify::db::get_events(...)`, `vox_gamify::db::list_cost_records(...)` | `vox_db::ops_agent_telemetry::{get_events, list_cost_records}` |
| `crates/vox-orchestrator-mcp/src/dei_tools/vcs_runtime.rs` | 162, 264, 270 | `vox_gamify::db::{get_events, get_agent_session, insert_cost_record}` | `vox_db::ops_agent_telemetry::{get_events, get_agent_session, insert_cost_record}` |
| `crates/vox-orchestrator-mcp/src/dispatch.rs` | 635 | `vox_gamify::db::insert_event(...)` | `vox_db::ops_agent_telemetry::insert_event(...)` |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs` | 1478 | `vox_gamify::db::insert_event(...)` | `vox_db::ops_agent_telemetry::insert_event(...)` |

### 2. Shadow Telemetry Call Sites to Eliminate (`LUDUS-SOC-02`)
| Crate & File | Line(s) | Current Symbol | Remediation |
|---|---|---|---|
| `crates/vox-lsp/src/server.rs` | 337 | `vox_gamify::lsp_telemetry::after_diagnostic_publish(...)` | Remove call; LSP diagnostic state transitions must not write unconsented rows into `.vox/store.db`. |
| `crates/vox-cli/src/commands/check.rs` | 153 | `vox_gamify::lsp_telemetry::after_cli_check_clean(&db, &key).await` | Remove shadow telemetry call; `vox check` telemetry goes through consented `vox-telemetry`. |
| `crates/vox-cli/src/commands/extras/ludus/quests_notifications.rs` | 260, 270 | `vox_gamify::db::log_hint_event(...)` | Remove `gamify_hint_telemetry` shadow DB write; keep deterministic local hint display. |

### 3. Multi-User Ludus & Leaderboard Surfaces (`LUDUS-SOC-03..05`, `LEADERBOARD-01..05`)
- `crates/vox-gamify/src/db/process_rewards.rs` lines 8, 179–182: Remove `get_user_collegium` and `update_collegium_lumens` side-effect from local solo reward processing.
- `crates/vox-orchestrator-mcp/src/gui_ops/gamify.rs` lines 97–118: Update `gamify_leaderboard` (`list_gamify_leaderboard` in Tauri) to load and verify the Ed25519-signed `VerifiedLeaderboardSnapshotV1` (from `~/.vox/cache/verified-leaderboard.v1.json` or embedded fallback `contracts/ludus/leaderboard-snapshot.v1.json`) via `vox_gamify::verified_leaderboard::verify_snapshot`, never querying local SQLite `gamify_profiles` for global rankings.

## Validation Architecture

1. **Plan `91-01` (`SOCIAL-01`, `SOCIAL-02`):** `cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-syndication` verifies all 9 extracted social adapters, `SELECT ... FOR UPDATE SKIP LOCKED` outbox claiming, `idempotency_key` deduplication, rate-limit backoff, and `syndication.delivery_receipts`.
2. **Plan `91-02` (`SOCIAL-03..05`):** `timeout 900s cargo test -p vox-publisher --all-features && timeout 900s cargo test -p vox-secrets` verifies public `vox-publisher` compiles with zero live social adapters, exports `SyndicationHandoffPayloadV1` (`#[serde(deny_unknown_fields)]`), and passes secret parity checks.
3. **Plan `91-03` (`LUDUS-SOC-01..05`):** `timeout 900s cargo test -p vox-db -p vox-gamify -p vox-orchestrator -p vox-orchestrator-mcp && cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-ludus` verifies `vox_db::ops_agent_telemetry`, removal of shadow telemetry (`hint_telemetry`, `lsp_telemetry`), extraction of multi-user Ludus into `vox-ops-ludus`, and 100% offline solo Ludus/FSRS tests.
4. **Plan `91-04` (`LEADERBOARD-01..05`):** `cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-ludus && timeout 900s cargo test -p vox-gamify -p vox-orchestrator-mcp && pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Gamify/LeaderboardArenaPanel.test.tsx && ./target/debug/vox-doc-pipeline --lint-only --paths community/leaderboard.mdx` verifies the `BLAKE3` hash chain, `AntiCheatPolicyV1`, `Ed25519`-signed `VerifiedLeaderboardSnapshotV1`, `VoxLeaderboardArena.astro`, and `LeaderboardArenaPanel.tsx`.
