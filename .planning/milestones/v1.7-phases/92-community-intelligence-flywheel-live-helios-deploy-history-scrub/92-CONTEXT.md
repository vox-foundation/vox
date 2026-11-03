# Phase 92: Community-Collated Intelligence Flywheel, Live Helios Coolify Deployment, Codebase/Git-History Scrub & E2E Verification - Context

**Gathered:** 2026-10-19
**Status:** Ready for execution

<domain>
## Phase Boundary

Deliver the complete Community-Collated Intelligence Flywheel (`INTEL-01..05`, `TYPING-04`), deploy `vox-ops` live on `helios` via the Coolify API (`HELIOS-02..04`), verify cross-store database shape parity (`DBAUDIT-04`), and execute the codebase & git-history scrub (`SCRUB-01..04`):

1. **Signed Community Intelligence Bundle & $k \ge 20$ Publisher (`INTEL-01`, `INTEL-02`, `TYPING-04`):**
   - `contracts/intelligence/community-intelligence-bundle.v1.schema.json` and `crates/vox-orchestrator/src/models/intelligence_bundle.rs` define `CommunityIntelligenceBundleV1` with `#[serde(deny_unknown_fields)]` across all 7 domains (`routing_priors`, `openrouter_free_tier_liveness`, `default_decision_priors`, `prompt_profile_priors`, `mens_spoke_priors`, `training_throughput_priors`, `domain_misguidance_hotspots`) and verify Ed25519 signatures via `vox_crypto::ed25519_verify`.
   - `~/dev/vox-ops/crates/vox-ops-intelligence` queries `intelligence_public.v_*` as `vox_ops_app`, adds Laplace ($\epsilon = 1.0$) noise to bucket counts, signs with Ed25519, logs provenance in `community.intelligence_bundles`, and serves `GET /v1/intelligence/manifest.json` + `GET /v1/intelligence/bundle.v1.json.zst`.

2. **Non-Blocking Client Hydration, CLI/GUI Controls & Offline Equivalence (`INTEL-03`, `INTEL-04`, `INTEL-05`):**
   - `vox-orchestrator` (`models::registry`, `models::prompt_profile`) hydrates `~/.vox/cache/community-intelligence.v1.json` asynchronously (`3s` timeout, `If-None-Match` ETag) and applies Empirical Bayes priors ($w_{\text{local}} = \frac{n_{\text{local}}}{n_{\text{local}} + 10}$, clamped to $\pm 15\%$ max delta).
   - `vox telemetry intelligence {status,sync,inspect}` and `SettingsView.tsx` expose bundle status and `VOX_COMMUNITY_INTELLIGENCE=off` opt-out.
   - Integration tests prove 100% offline/air-gapped equivalence when `VOX_TELEMETRY=off`, `VOX_COMMUNITY_INTELLIGENCE=off`, or `vox-ops` is unreachable/503/tampered.

3. **Live Coolify Deployment on `helios` & Cross-Store Shape Audit (`HELIOS-02..04`, `DBAUDIT-04`):**
   - `vox-ops-server` combines `/healthz`, `/v1/logs`, `/v1/intelligence/*`, `/v1/ludus/leaderboard*`, and `/v1/syndication/handoff`, deployed to `helios` (`178.156.212.19`) under Coolify project **"Vox Ecosystem"** (`no88080okk0c0gk4ss0cgw0o`), verified end-to-end with cross-store `VoxDb` <-> `helios-postgres` shape audits.

4. **Working-Tree & `git filter-repo` History/Commit-Log Scrub (`SCRUB-01..04`, `D30`):**
   - `contracts/ci/moved-data-scrub-manifest.v1.json`, `contracts/ci/git-history-scrub.v1.json`, and `scripts/verify-history-scrub.vox` enforce zero working-tree remnants and verify deterministic `git filter-repo` history and commit-log scrubbing of moved `server/telemetry`, moved social adapters, moved multi-user Ludus modules, and shadow gamify telemetry while preserving transparent opt-in `vox-telemetry`.
</domain>

<decisions>
## Implementation Decisions

- **D-92-01 (`D18`, `D21`, `D28`):** `CommunityIntelligenceBundleV1` is independently opt-out (`VOX_COMMUNITY_INTELLIGENCE=off`), requires zero telemetry opt-in to consume, is verified via `vox_crypto::ed25519_verify` before touching `~/.vox/cache/community-intelligence.v1.json`, and is bounded to $\pm 15\%$ max weight adjustment via Empirical Bayes blending.
- **D-92-02 (`D27`, `D30`):** Live `vox-ops` deployment on `helios` is driven through the Coolify API (`no88080okk0c0gk4ss0cgw0o`) and verified end-to-end before sealing the moved-data working-tree and `git filter-repo` history scrub contracts.
</decisions>

<canonical_refs>
## Canonical References

- `docs/src/architecture/centralized-postgres-telemetry-vox-ops-ssot-2026.md` — Sections 7, 8, 10, 11 (Intelligence bundle schema, Empirical Bayes formula, Coolify deployment, and `SCRUB-01..04` contract)
</canonical_refs>
