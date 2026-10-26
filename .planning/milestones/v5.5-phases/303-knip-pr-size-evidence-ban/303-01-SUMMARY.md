# Phase 303 Summary: Knip, PR Size & Evidence Ban

**Status:** Complete  
**Date:** 2026-10-25

## Delivered
- `crates/vox-gui/ui/knip.jsonc` + `pnpm knip` script + `vox ci knip-gui`
- `.github/workflows/pr-size.yml` (size:XS–XXL, non-test LOC)
- `vox ci evidence-asset-ban` + `contracts/ci/evidence-asset-allowlist.v1.yaml`
- AGENTS.md PR hygiene notes; mutation unit tests (3/3)

## Requirements
- [x] T3-CI-HYGIENE-01..04

## Notes
Knip runs via local binary or `pnpm dlx knip@5`; full lockfile pin deferred until UI job installs knip as a regular dep.
