# Phase 91: Social Media Syndication, Multi-User Ludus Extraction, 3-Way Gamification SoC Split & Zero-Cheating Leaderboard Surfaces - Context

**Gathered:** 2026-10-19
**Status:** Ready for planning

<domain>
## Phase Boundary

Deliver the complete Separation of Concerns (SoC) extraction for social syndication (`SOCIAL-01..05`), gamification/Ludus (`LUDUS-SOC-01..05`), and the Zero-Cheating Server-Authoritative Leaderboard with dynamic v0 / Claude Design GUI surfaces on `voxlang.org` and `vox-gui` (`LEADERBOARD-01..05`):

1. **Social Media Syndication Extraction (`SOCIAL-01..05`):**
   - Extract the 9 live social/community broadcast adapters (`twitter.rs`, `reddit.rs`, `youtube.rs`, `discord.rs`, `bluesky.rs`, `mastodon.rs`, `linkedin.rs`, `hacker_news.rs`, `opencollective.rs`, and `canary.rs` probes) from public `crates/vox-publisher/src/adapters/` into `~/dev/vox-ops/crates/vox-ops-syndication/`.
   - Implement the PostgreSQL-backed outbox worker (`syndication.outbox`, `syndication.rate_limit_windows`, `syndication.delivery_receipts`) using `SELECT ... FOR UPDATE SKIP LOCKED`, `idempotency_key` deduplication, and exponential backoff retry (`SOCIAL-01`, `SOCIAL-02`).
   - Keep `crates/vox-publisher` 100% functional offline/locally for scholarly publishing (`CITATION.cff`, Crossref, BibTeX, Zenodo/OpenReview/arXiv local bundle generation, `rss.rs`, `github.rs`, `researchgate.rs`) and add `SyndicationHandoffPayloadV1` (`vox scientia syndication export`) + `Clavis` secret parity (`SOCIAL-03..05`).

2. **3-Way Ludus & Gamification SoC Split (`LUDUS-SOC-01..05`, `D29`):**
   - **Tier 1 (`LUDUS-SOC-01`, `LUDUS-SOC-02`):** Extract core agent session/cost tracking (`AgentEventRecord`, `CostRecord`, `AgentSessionRecord`, `get_events`, `insert_event`, `insert_cost_record`, `list_cost_records`, `get_agent_session`, `get_tenant_monthly_token_usage`, `insert_agent_session`, `update_agent_session`, `end_agent_session`, `list_active_sessions`, `upsert_agent_metric`, `get_agent_metrics`, `get_agent_cost_usd`, `acknowledge_message`) from `crates/vox-gamify/src/db/agent_telemetry.rs` into `crates/vox-db/src/store/ops_agent_telemetry.rs`, update all callers in `vox-orchestrator` and `vox-orchestrator-mcp`, and eliminate unconsented shadow telemetry in `vox-gamify` (`db/hint_telemetry.rs`, `lsp_telemetry.rs`).
   - **Tier 2 (`LUDUS-SOC-03`, `LUDUS-SOC-04`):** Move multi-user competitive/social Ludus (`db/collegium.rs`, `db/arena.rs`, `db/leaderboards.rs`, `db/disputes.rs`, `crates/vox-cli/src/commands/extras/ludus/{collegium,arena,disputes,auth,sync}.rs`) into `~/dev/vox-ops/crates/vox-ops-ludus` backed by `ludus_community.*` in `vox_ops`.
   - **Tier 3 (`LUDUS-SOC-05`):** Retain solo developer progression (`LudusProfile`, `Companion`, `Quest`, `Battle`, `StreakTracker`, `AchievementTracker`, `shop`, `sprite_svg`, and FSRS CLI/GUI discovery `discovery/{fsrs,rank,ledger,scientia_kudos}.rs`) 100% local and offline in `vox-gamify`.

3. **Zero-Cheating Leaderboard & Dynamic v0 / Claude Design GUI Surfaces (`LEADERBOARD-01..05`, `D31`):**
   - Implement `ludus_community.verified_contributions` in `vox-ops-ludus` with zero client score writes, HMAC-SHA256 GitHub webhook verification + signed 5-spoke `(model_id, harness_id, config_digest)` evaluation receipts (with 95% Wilson CIs), `UNIQUE (upstream_provider, upstream_event_id)` replay protection, append-only `BLAKE3` hash chain (`row_chain_hash_hex`), `AntiCheatPolicyV1`, and `Ed25519`-signed `VerifiedLeaderboardSnapshotV1` served at `GET /v1/ludus/leaderboard` and SSE `GET /v1/ludus/leaderboard/stream` (`LEADERBOARD-01`, `LEADERBOARD-02`).
   - Build `docs-astro/src/components/VoxLeaderboardArena.astro` (`<vox-leaderboard-arena>`) mounted at `docs/src/community/leaderboard.mdx` (`/community/leaderboard/`) and `crates/vox-gui/ui/src/components/surfaces/Gamify/LeaderboardArenaPanel.tsx` in `GamifyView.tsx` using v0 / Claude Design tokens, 4 lenses, interactive Cryptographic Proof Drawer, and 100% static/offline fallback (`LEADERBOARD-03`, `LEADERBOARD-04`, `LEADERBOARD-05`).
</domain>

<decisions>
## Implementation Decisions

### Social Syndication Split (`D17`, `D20`)
- **D-91-01:** Remove `bluesky.rs`, `discord.rs`, `hacker_news.rs`, `linkedin.rs`, `mastodon.rs`, `opencollective.rs`, `reddit.rs`, `twitter.rs`, `youtube.rs`, and `canary.rs` from `crates/vox-publisher/src/adapters/` after extracting them into `~/dev/vox-ops/crates/vox-ops-syndication/src/adapters/`. Retain `rss.rs`, `github.rs` (`forge`), and `researchgate.rs` in `vox-publisher`.
- **D-91-02:** Introduce `contracts/scientia/syndication-handoff.v1.schema.json` and `SyndicationHandoffPayloadV1` (`#[serde(deny_unknown_fields)]`) in `crates/vox-publisher/src/handoff.rs` so `Publisher::publish_all` and `vox scientia syndication export` emit deterministic handoff bundles for `vox-ops-syndication` without requiring direct social API credentials in public `vox` builds.

### 3-Way Ludus & Gamification SoC (`D29`)
- **D-91-03:** Move `crates/vox-gamify/src/db/agent_telemetry.rs` functions and types to `crates/vox-db/src/store/ops_agent_telemetry.rs` (re-exported at `vox_db::ops_agent_telemetry::*`), and update `vox-orchestrator` (`task_submit.rs`) and `vox-orchestrator-mcp` (`a2a_tools.rs`, `dei_tools/control.rs`, `dei_tools/vcs_runtime.rs`, `dispatch.rs`, `chat_tools/chat/message.rs`) to call `vox_db::ops_agent_telemetry::*` directly instead of `vox_gamify::db::*`.
- **D-91-04:** Delete `crates/vox-gamify/src/db/hint_telemetry.rs` (`log_hint_event`) and `crates/vox-gamify/src/lsp_telemetry.rs` (`after_diagnostic_publish`, `after_cli_check_clean`), removing unconsented shadow telemetry writes in `crates/vox-lsp/src/server.rs`, `crates/vox-cli/src/commands/check.rs`, and `crates/vox-cli/src/commands/extras/ludus/quests_notifications.rs`.
- **D-91-05:** Move `collegium.rs`, `arena.rs`, `leaderboards.rs` (keeping `get_profile_stats` in `profile.rs`), and `disputes.rs` out of `crates/vox-gamify/src/db/` into `~/dev/vox-ops/crates/vox-ops-ludus/`, and remove `update_collegium_lumens` from local `process_rewards.rs`.

### Zero-Cheating Leaderboard & v0 / Claude Design Surfaces (`D31`)
- **D-91-06:** Define `VerifiedLeaderboardSnapshotV1` (`#[serde(deny_unknown_fields)]`) in `contracts/ludus/verified-leaderboard-snapshot.v1.schema.json` and `crates/vox-gamify/src/verified_leaderboard.rs` (shared read-only verifier using `vox_crypto::ed25519_verify`), including `(model_id, harness_id, config_digest)` tuples and 95% Wilson confidence intervals (`pass_at_1_ci95_low`, `pass_at_1_ci95_high`) for the `5-Spoke Model & Mens Arena` lens (closing Gap K of `vox-mens-comparative-efficacy-benchmarking-research-2026-09-01.md`).
- **D-91-07:** Mount `<VoxLeaderboardArena />` (`docs-astro/src/components/VoxLeaderboardArena.astro`) in `docs/src/community/leaderboard.mdx` with a build-time signed snapshot at `docs-astro/src/data/leaderboard-snapshot.v1.json`, and mount `LeaderboardArenaPanel.tsx` inside `crates/vox-gui/ui/src/components/surfaces/Gamify/GamifyView.tsx` backed by `gamify_leaderboard` verifying `VerifiedLeaderboardSnapshotV1`.
</decisions>

<canonical_refs>
## Canonical References

- `docs/src/architecture/centralized-postgres-telemetry-vox-ops-ssot-2026.md` — Sections 4, 6, 9, 10, 11 (Social Syndication, 3-Way Ludus SoC, Zero-Cheating Leaderboard & v0/Claude Design GUI specification)
- `docs/src/architecture/vox-mens-comparative-efficacy-benchmarking-research-2026-09-01.md` — §F (Gap K: `voxlang.org` live benchmark leaderboard with `(model, harness, config)` tuples and 95% Wilson CIs)
- `crates/vox-publisher/src/publisher/mod.rs` — Current `Publisher::publish_all` social channel dispatch
- `crates/vox-gamify/src/db/mod.rs` — Current `vox-gamify` database module exports
- `crates/vox-gui/ui/src/components/surfaces/Gamify/GamifyView.tsx` — Current Axis GUI Gamify surface
- `docs-astro/src/components/VoxPlayground.astro` — Reference Astro custom element (`<vox-playground>`) pattern for `voxlang.org`
</canonical_refs>
