---
phase: 61-external-adoption-git-rev-codegen-binary-release
plan: 01
status: complete
completed_at: "2026-10-14T11:05:00Z"
requirements_completed: [DIST-01, DIST-02]
files_modified:
  - crates/vox-codegen/build.rs
  - crates/vox-codegen/src/codegen_rust/emit/mod.rs
  - docs/src/architecture/generated-project-runtime-deps.md
---

# Summary: Plan 61-01 — External Binary Adoption & Git-Rev Codegen Fallback

## Completed Work
- Updated [`crates/vox-codegen/build.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-codegen/build.rs) to emit `VOX_GIT_HASH` (`git rev-parse HEAD` with `// vox-arch-check: allow git-exec`).
- Implemented `format_vox_crate_dep_line` Tier 2.5 fallback (`git = "https://github.com/vox-foundation/vox.git", rev = "<VOX_GIT_HASH>"`) in [`crates/vox-codegen/src/codegen_rust/emit/mod.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-codegen/src/codegen_rust/emit/mod.rs) when `resolve_vox_repo_root()` returns `None`, so prebuilt `vox` binaries work without a local monorepo checkout.
- Updated [`docs/src/architecture/generated-project-runtime-deps.md`](file:///Users/brbrainerd/dev/vox/docs/src/architecture/generated-project-runtime-deps.md) to document Tier 2.5.

## Verification
- `cargo test -p vox-codegen --lib codegen_rust::emit::tests` → `3 passed; 0 failed`.
- `./target/debug/vox-doc-pipeline --lint-only --paths architecture/generated-project-runtime-deps.md` → `0` errors.
- `./target/debug/vox-arch-check` → exit code `0`.
