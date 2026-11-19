# Phase 470 Review — Polyglot E2E Proof, Concept-Art Convergence & Workspace Closure

**Scope Reviewed:**
- `crates/vox-cli/tests/v79_polyglot_flywheel_proof.rs`
- `contracts/reports/v79-universal-kb-openwiki-flywheel-proof.v1.json`
- `crates/vox-search/src/promotion.rs`
- `crates/vox-doc-pipeline/src/pipeline/openwiki_heal.rs`
- `crates/vox-gui/ui/public/assets/romanesque/v79-concept-{openwiki-studio,multirepo-atlas}-{carrara,slate}.jpg`
- `crates/vox-gui/ui/e2e/review/{vlmVisionFlywheel.ts,v79CvFlywheelConvergence.test.ts}`
- `docs/src/architecture/{universal-kb-graphify-openwiki-multirepo-research-2026.md,where-things-live.md,docs-system-ssot.md}`

## Review Findings & Resolutions
1. **Cross-Crate Integration Fix — Document-Relative Citations (`../../`) in OpenWiki Docs (`promotion.rs`):**
   - `v79_polyglot_flywheel_proof` revealed that `openwiki_polyglot` writes valid document-relative citations (e.g., `../../src/lib.rs#L4`) inside `docs/wiki/*.md`, whereas `stage_verified_prose_heal_proposal` previously passed `&patched_md` to `verify_citations_mechanically(repo_root, ...)` which rejected every `".."` segment unconditionally.
   - Added `verify_citations_mechanically_for_doc(repo_root, doc_rel_path, markdown)` so `".."` segments inside document-relative links resolve against `repo_root.join(doc_rel).parent()` and undergo strict canonical containment checking (`cand_canon.starts_with(&canon_root) && cand_canon.is_file()`). Escapes outside `repo_root` (`../../../../etc/passwd`) remain fail-closed (`promotion` test `10/10` passing).
2. **`vox-arch-check` Concurrency & `GIT_*` Hook Isolation (`openwiki_heal.rs`):**
   - `vox-arch-check` caught `std::process::Command::new("git")` at `crates/vox-doc-pipeline/src/pipeline/openwiki_heal.rs:91`.
   - Without adding any dependency to `vox-doc-pipeline/Cargo.toml`, updated the read-only `git -C <repo_root> diff --name-only HEAD` probe to strip inherited `GIT_*` environment variables (matching `vox-graph-reader/src/history/git.rs`) and annotated it with `// vox-arch-check: allow git-exec` (matching `claims/prune.rs`). `vox-arch-check` now exits `0`.
3. **SSOT Documentation Parity Verification:**
   - Promoted `universal-kb-graphify-openwiki-multirepo-research-2026.md` to `status: "current"`, replaced hypothetical `vox wiki` examples with real shipped commands, and verified `vox-doc-pipeline --lint-only` (`0` hard errors), `docs_system_parity` (`4/4`), and `vox ci operations-verify` (`1430 catalog rows`).
