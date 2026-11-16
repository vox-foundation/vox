# Phase 92 Plan 04: Moved-Data Working-Tree Guard & Deterministic `git filter-repo` History Scrub Verification Summary

## Objective
Authored `contracts/ci/moved-data-scrub-manifest.v1.json`, `contracts/ci/git-history-scrub.v1.json`, and `scripts/verify-history-scrub.vox` enforcing that all 17 moved server/social/multi-user-Ludus/shadow-telemetry paths remain absent from the public `vox` repository while all 10 transparent opt-in `vox-telemetry`, `vox-db::telemetry_sink`, `vox-gamify::verified_leaderboard`, and `vox-orchestrator::intelligence_bundle` surfaces remain intact (`SCRUB-01..04`).

## Verification Evidence
- `vox-doc-pipeline --lint-only`: **0 hard errors**
- Verified all 17 forbidden moved paths are absent from working tree and all 10 required preserved surfaces are present.
