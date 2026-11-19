# Phase 468 Review Fixes

**Phase:** `468` — Tiered Self-Updating & Self-Fixing Documentation Engine

## Fixes Applied During Review
1. **`vox-arch-check` Raw `git` Concurrency Policy (`crates/vox-doc-pipeline/src/pipeline/openwiki_heal.rs`):**
   - Replaced bare `std::process::Command::new("git")` at line `91` (`resolve_blast_radius`) with an isolated `Command::new("git")` that strips inherited `GIT_*` hook variables (matching `crates/vox-graph-reader/src/history/git.rs`) and carries `// vox-arch-check: allow git-exec` explaining the read-only `git diff --name-only HEAD` query without adding a `vox-git` dependency edge.
2. **Document-Relative Citation Containment for OpenWiki Proposals (`crates/vox-search/src/promotion.rs`):**
   - Added `verify_citations_mechanically_for_doc` and updated `stage_verified_prose_heal_proposal` so `../../` relative Markdown links originating in `docs/wiki/*.md` resolve relative to the doc's parent directory and validate canonical containment inside `repo_root`, while external escapes still fail closed.
3. **Post-Fix Verification:**
   - `cargo test -p vox-search --lib promotion` (`10 passed; 0 failed`), `cargo test -p vox-doc-pipeline openwiki` (`13 passed; 0 failed`), `cargo run -p vox-arch-check` (exit `0`), `cargo clippy ... -- -D warnings` (exit `0`), and `toestub` (exit `0`).
