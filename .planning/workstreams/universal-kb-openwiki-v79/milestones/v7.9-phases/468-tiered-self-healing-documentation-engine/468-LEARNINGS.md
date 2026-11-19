# Phase 468 Learnings

1. **Doc-Relative Citations vs Repo-Root Validation:** Multi-layer wiki engines place pages in nested directories (`docs/wiki/*.md`), so emitted Markdown links naturally start with `../../`. Path-escape validators checking Markdown citations must normalize against `repo_root.join(doc_rel).parent()` and assert `canonicalize().starts_with(canon_root)` rather than banning `".."` lexicographically.
2. **Schema Baseline Preservation via Extension Tables:** Keeping `kb_heal_proposals` in `schema_extensions.rs` rather than mutating `SCHEMA_FRAGMENTS` adds full proposal triage durability without destabilizing `BASELINE_VERSION` contracts.
3. **Zero-Dep `GIT_*` Sanitization:** Read-only `git diff` probes in low-layer crates must strip inherited `GIT_*` hook env vars and document why `GitExec` is out-of-layer so `vox-arch-check` remains green.
