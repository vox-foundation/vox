# Phase 92: Community-Collated Intelligence Flywheel, Live Helios Coolify Deployment, Codebase/Git-History Scrub & E2E Verification - Research

**Researched:** 2026-10-19
**Domain:** Ed25519-Signed Community Intelligence Flywheel (`CommunityIntelligenceBundleV1`), Coolify API Deployment on `helios`, and Deterministic `git filter-repo` History Scrubbing
**Confidence:** HIGH (grounded in `centralized-postgres-telemetry-vox-ops-ssot-2026.md` §§7, 8, 10)

## Summary

Phase 92 completes the end-to-end operational loop between `vox` and `vox-ops`:
1. **7-Domain Community Intelligence Bundle (`INTEL-01`, `INTEL-02`, `TYPING-04`):**
   - `vox-ops-intelligence` queries the 8 `SECURITY DEFINER` $k \ge 20$ views in `intelligence_public.*` as `vox_ops_app`, adds Laplace ($\epsilon = 1.0$) differential-privacy noise to bucket counts, signs the canonical payload with `vox_crypto::ed25519_sign`, records provenance in `community.intelligence_bundles`, and serves `/v1/intelligence/manifest.json` + `/v1/intelligence/bundle.v1.json.zst`.
   - `crates/vox-orchestrator/src/models/intelligence_bundle.rs` validates `CommunityIntelligenceBundleV1` (`#[serde(deny_unknown_fields)]`) and verifies its Ed25519 signature against the compiled-in Vox Foundation public key.
2. **Empirical Bayes Blending & Offline Equivalence (`INTEL-03..05`):**
   - `ModelRegistry::apply_community_intelligence_bundle` blends community priors with local `VoxDb` observations using $w_{\text{local}} = \frac{n_{\text{local}}}{n_{\text{local}} + 10}$ and clamps adjustments to $\pm 15\%$ (`MAX_COMMUNITY_WEIGHT_DELTA = 0.15`).
   - `vox telemetry intelligence {status,sync,inspect}` and `SettingsView.tsx` provide transparent inspection and independent opt-out (`VOX_COMMUNITY_INTELLIGENCE=off`).
3. **Live Coolify Deployment on `helios` & Cross-Store Shape Audit (`HELIOS-02..04`, `DBAUDIT-04`):**
   - `vox-ops-server` mounts all 4 operational routers (`/healthz`, `/v1/logs`, `/v1/intelligence/*`, `/v1/ludus/leaderboard*`, `/v1/syndication/handoff`), deploys to `helios` (`178.156.212.19`) under Coolify project `no88080okk0c0gk4ss0cgw0o`, and verifies round-trip strong-type parity between local `VoxDb` and `helios-postgres`.
4. **Moved-Data Working-Tree Guard & `git filter-repo` History Scrub (`SCRUB-01..04`):**
   - `contracts/ci/moved-data-scrub-manifest.v1.json`, `contracts/ci/git-history-scrub.v1.json`, and `scripts/verify-history-scrub.vox` enforce zero working-tree remnants and verify deterministic `git filter-repo` history and commit-message scrubbing on a disposable mirror clone.
