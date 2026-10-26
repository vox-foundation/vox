# Phase 303: Knip Dual Gate, PR Size Labels & Evidence-Asset Ban - Context

## Phase Boundary
Catch unused TS exports in `vox-gui/ui`, label PRs by non-test LOC, and ban committing evidence/screenshot spam outside allowlisted paths.

## Implementation Decisions
- Knip config at `crates/vox-gui/ui/knip.jsonc` (exported types off); `vox ci knip-gui` runs `pnpm exec knip` when available.
- PR size: adapted t3code `pr-size.yml` with Vox test pathspecs.
- Evidence ban: `vox ci evidence-asset-ban` + `contracts/ci/evidence-asset-allowlist.v1.yaml`.

## Deferred
- Full knip CI gate in `ui` job until knip is installed in the pnpm lockfile on runners that already have node_modules.
