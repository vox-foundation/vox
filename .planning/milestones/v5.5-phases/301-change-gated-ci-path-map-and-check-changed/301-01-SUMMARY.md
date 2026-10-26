# Phase 301 Summary: Change-Gated CI Path Map & `vox ci check-changed`

**Status:** Complete  
**Date:** 2026-10-25

## Delivered
- `contracts/ci/changed-path-owners.v1.yaml` — inert prefixes + ordered owner globs
- `crates/vox-cli-ci/src/check_changed.rs` — plan + inert short-circuit (no cargo on inert)
- `vox ci check-changed` (`CiCmd::CheckChanged`) wired through `run_body`
- Catalog baseline + docs in `affected-crate-selective-ci.md` (F3 closed)
- Unit tests: 6/6 `check_changed` passed (`cargo test -p vox-cli-ci --lib check_changed`)
- Collateral: fixed `history.rs` checkpoint sync API + `SecretId::OpenaiApiKey` so workspace compiles

## Requirements
- [x] T3-CI-CHANGED-01..04 (owner map, CLI, inert CI output via `--github-output`, docs+tests)

## Notes
Non-inert execution still delegates to pre-push/affected-crates (plan-only by design for Phase 301). PR job wiring can consume `inert=true` from `--github-output` in a follow-up workflow edit if needed.
