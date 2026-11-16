---
phase: 116-github-ci-red-gate-and-db-digest-remediation
plan: 01
subsystem: ci-and-contracts
tags: [ci, voxup, docs-astro, db-baseline, vox-langtool]
requires: []
provides:
  - Updated `repository_baseline_digest_hex` in `contracts/db/baseline-version-policy.yaml`
  - `[[crates]]` table parsing in `crates/voxup/src/profiles.rs` and Tier 2 plugin/installer entries in `crates/_public.toml`
  - Clean `pnpm --dir docs-astro build` (`855 page(s) built`) and `vox ci check-links` (`5402 internal links PASS`)
  - Fast `vox-langtool check` in `.github/workflows/cr-l8-corpus-feedback.yml`
affects: [voxup, docs-astro, ci]
key-files:
  modified:
    - contracts/db/baseline-version-policy.yaml
    - crates/voxup/src/profiles.rs
    - crates/_public.toml
    - docs/src/community/leaderboard.mdx
    - docs-astro/src/components/VoxLeaderboardArena.astro
    - .planning/workstreams/unified/ROADMAP.md
    - .github/workflows/cr-l8-corpus-feedback.yml
requirements-completed: [CI-GREEN-01, CI-GREEN-02, CI-GREEN-03, CI-GREEN-04]
completed: 2026-10-20
---

# Phase 116 Plan 01: GitHub CI Red Gate & DB Baseline Digest Remediation Summary

## Accomplishments
- **`CI-GREEN-01` (`contracts/db/baseline-version-policy.yaml`):** Updated `repository_baseline_digest_hex` to `"0x2d24ad28ca1cbba8cab84f2fcc5fa0f150d04a361b56d248a7901442fba05071"` matching `vox_db::schema::baseline_sql()` Keccak256 (`0` baseline digest warnings in `vox ci ssot-drift`).
- **`CI-GREEN-02` (`crates/voxup/src/profiles.rs`, `crates/_public.toml`):** Updated `public_toml_crates` to parse both `[[crates]]` table `name` entries and string arrays, and added Tier 2 plugin/installer crates (`vox-plugin-types`, `vox-plugin-api`, `vox-plugin-sdk`, `voxup`) to `crates/_public.toml` (`cargo test -p voxup --test distribution_parity`: `10 passed; 0 failed`).
- **`CI-GREEN-03` (`docs/src/community/leaderboard.mdx`, `docs-astro/src/components/VoxLeaderboardArena.astro`, `.planning/workstreams/unified/ROADMAP.md`):** Replaced HTML comments with MDX `{/* kb:summary */}` in `leaderboard.mdx`, escaped curly braces in `VoxLeaderboardArena.astro`, and fixed relative link resolution in `.planning/workstreams/unified/ROADMAP.md` (`pnpm --dir docs-astro build` built `855` pages in `38.30s`; `vox ci check-links` verified `5402` internal links).
- **`CI-GREEN-04` (`.github/workflows/cr-l8-corpus-feedback.yml`):** Replaced the 14-minute `vox-cli` compilation in `scripts-check` with lightweight `vox-langtool` (`cargo build -p vox-langtool --bin vox-langtool` + `./target/debug/vox-langtool check "$f"`).
