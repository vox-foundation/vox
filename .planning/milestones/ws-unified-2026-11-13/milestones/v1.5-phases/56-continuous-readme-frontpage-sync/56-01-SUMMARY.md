---
phase: 56-continuous-readme-frontpage-sync
plan: 01
type: execute
completed_at: 2026-10-14T11:55:00Z
files_modified:
  - crates/vox-doc-pipeline/src/pipeline/lint.rs
  - crates/vox-doc-pipeline/src/pipeline/mod.rs
  - crates/vox-cli/src/commands/ci/command_sync.rs
  - README.md
  - docs/src/index.mdx
requirements-completed:
  - FRONTPAGE-01
  - FRONTPAGE-02
  - FRONTPAGE-03
---

# Phase 56 Plan 01–03 Summary: Continuous `README.md` & `voxlang.org` Frontpage Codebase Sync

## Overview

Implemented automatic, deterministic synchronization between `README.md` (`<!-- ANCHOR: id --> ... <!-- ANCHOR_END: id -->`) and `docs/src/index.mdx` (`{/* SYNC-FROM-README: id */} ... {/* SYNC-END: id */}`), wired into both `vox-doc-pipeline` (`--fix` and `--sync-readme-frontpage [--write]`) and `vox ci command-sync [--write]` (invoked by `scripts/ssot-regen.vox` in CI's `ssot-autoregen` workflow and verified by `vox ci ssot-drift`).

## Key Changes

1. **`transform_readme_block_for_mdx` & `sync_readme_to_frontpage` (`FRONTPAGE-01`)**:
   - Added `transform_readme_block_for_mdx` and `sync_readme_to_frontpage(root, write)` in [`crates/vox-doc-pipeline/src/pipeline/lint.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/src/pipeline/lint.rs).
   - Automatically transforms `docs/src/assets/...` image paths to `./assets/...` for Starlight MDX compatibility while preserving byte-exact normalized parity with `check_readme_sync`.
2. **CLI & CI Autoregen Wiring (`FRONTPAGE-02`)**:
   - Wired `--sync-readme-frontpage [--write]` and `--fix` into [`crates/vox-doc-pipeline/src/pipeline/mod.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/src/pipeline/mod.rs).
   - Wired `vox_doc_pipeline::pipeline::sync_readme_to_frontpage(repo_root, write)` into [`crates/vox-cli/src/commands/ci/command_sync.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-cli/src/commands/ci/command_sync.rs) so `vox ci command-sync --write` and `scripts/ssot-regen.vox` keep `docs/src/index.mdx` synchronized with `README.md` automatically.

## Verification

- `./target/debug/vox-doc-pipeline --sync-readme-frontpage`: `readme-frontpage-sync OK (updated=false)`.
- `cargo test -p vox-doc-pipeline`: all `readme_sync_*` unit tests pass.
