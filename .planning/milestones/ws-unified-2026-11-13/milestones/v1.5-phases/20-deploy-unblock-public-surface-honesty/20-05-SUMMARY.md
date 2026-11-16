# Phase 20 Plan 05 Summary: Pre-Fix Dead Relative Link Targets & Source Guard

**Phase:** 20 — Deploy Unblock & Public-Surface Honesty  
**Plan:** 05 — Source-link guard and fix already-dead relative link targets  
**Status:** Completed  
**Completed at:** 2026-10-12  

---

## 1. Executive Summary

- Added unit test guard `docs-astro/tests/unit/source-link-targets.test.mjs` verifying every relative link in non-archive `docs/src` markdown resolves to an existing file on disk.
- Fixed 71 dead relative link targets across 12 documentation pages.
- Initial test run: 659 passed, 12 failed (50 dead links in `vox-as-llm-target-audit-and-plan-2026.md`, 7 in `mesh-dashboard-and-distributed-compute-research-2026.md`, 5 in `v1-llm-target-implementation-plan-2026.md`, 4 in `dependency-consolidation-plan-2026.md`, 2 in `gui-authoring-syntax-2026.md`, 2 in `mesh-and-language-distribution-ssot-2026.md`, 2 in `unified-task-hopper-research-2026.md`, 1 in `036-webir-hir-unification-compare-both.md`, 1 in `webir-hir-split-brain-inventory-2026.md`, 1 in `2026-05-08-llm-misleading-content-cleanup-plan.md`, 1 in `agentic-version-control-automation-research-2026.md`, and 1 in `vox-fullstack-artifacts.md`).
- Final test run: **671 passed, 0 failed** across the entire non-archive corpus.

---

## 2. Changes Made

### Test Guard
- Created `docs-astro/tests/unit/source-link-targets.test.mjs`:
  - Scans `git ls-files docs/src` excluding `docs/src/archive/` and `docs/src/.well-known/`.
  - Exports `relativeLinkTargets(markdown)` which strips fenced and inline code blocks, extracts inline link targets and reference definitions (excluding markdown footnotes `[^...]`), filters URL schemes/anchors/absolute paths, strips `#fragment` and trailing line ranges (`:N`, `:N-M`), decodes URI encoding, and resolves paths relative to the containing document directory.
  - Generates one `test()` per markdown file with explicit error diagnostics.

### Link Fixes by File

1. **`docs/src/adr/036-webir-hir-unification-compare-both.md`**:
   - Replaced dead `tauri_stub.rs` link (`[`tauri_stub.rs`](../../../crates/vox-codegen/src/codegen_rust/emit/tauri_stub.rs)`) with inline code `tauri_stub.rs`.

2. **`docs/src/architecture/webir-hir-split-brain-inventory-2026.md`**:
   - Replaced dead `tauri_stub.rs` link (`[`crates/vox-codegen/src/codegen_rust/emit/tauri_stub.rs`](../../../crates/vox-codegen/src/codegen_rust/emit/tauri_stub.rs)`) with inline code `crates/vox-codegen/src/codegen_rust/emit/tauri_stub.rs`.

3. **`docs/src/architecture/v1-llm-target-implementation-plan-2026.md`**:
   - Adjusted relative link depth from repo-root relative (`crates/...`) to doc relative (`../../../crates/...`) for 5 valid target paths:
     - `crates/vox-code-audit/src/detectors/anonymous_error.rs`
     - `crates/vox-corpus/` (2 occurrences)
     - `crates/vox-cli/src/commands/repair.rs`
     - `crates/vox-deploy-codegen/`

4. **`docs/src/architecture/vox-as-llm-target-audit-and-plan-2026.md`**:
   - Adjusted relative link depth for 50 links written relative to repository root instead of relative to `docs/src/architecture/`:
     - `crates/...` -> `../../../crates/...`
     - `contracts/...` -> `../../../contracts/...`
     - `AGENTS.md` -> `../../../AGENTS.md`
     - `CLAUDE.md` -> `../../../CLAUDE.md`
     - `docs/src/...` -> `../...`
     - `docs/agents/...` -> `../../agents/...`

5. **`docs/src/architecture/2026-05-08-llm-misleading-content-cleanup-plan.md`**:
   - Replaced dead `crates/_frozen.md` link with inline code `crates/_frozen.md`.

6. **`docs/src/architecture/agentic-version-control-automation-research-2026.md`**:
   - Replaced dead `vox-dashboard` link (`[`crates/vox-dashboard/`](../../../crates/vox-dashboard/)`) with inline code `crates/vox-dashboard/`.

7. **`docs/src/architecture/dependency-consolidation-plan-2026.md`**:
   - Replaced dead `vox-install-policy` and `vox-checksum-manifest` links with plain inline code.

8. **`docs/src/architecture/gui-authoring-syntax-2026.md`**:
   - Replaced dead `speak.vox` links pointing into retired `vox-dashboard` with plain inline code.

9. **`docs/src/architecture/mesh-and-language-distribution-ssot-2026.md`**:
   - Replaced dead `vox-dashboard` links (`api/mesh.rs` and `NetworkTab.tsx`) with plain inline code.

10. **`docs/src/architecture/mesh-dashboard-and-distributed-compute-research-2026.md`**:
    - Replaced 7 dead `vox-dashboard` links with plain inline code (`vox-dashboard`, `transport.ts`, `api/mesh.rs:82`, `TaskDispatch.tsx`, `api/runs.rs`, `api/mesh.rs:165`, `NetworkTab.tsx`).

11. **`docs/src/architecture/unified-task-hopper-research-2026.md`**:
    - Replaced 2 dead `vox-dashboard` links with plain inline code.

12. **`docs/src/reference/vox-fullstack-artifacts.md`** (`vox-ssg` Decision):
    - Investigated current home of SSG output: the separate `crates/vox-ssg/` crate was consolidated into `crates/vox-cli/src/utils/ssg/mod.rs` (providing `generate_static_site` for static HTML shell rendering from `routes:` declarations).
    - Linked `[`vox-cli` SSG](../../../crates/vox-cli/src/utils/ssg/mod.rs)` in place of `crates/vox-ssg/src/lib.rs`.

---

## 3. Verification Evidence

- `cd docs-astro && node --test tests/unit/source-link-targets.test.mjs`:
  ```
  ℹ tests 671
  ℹ suites 0
  ℹ pass 671
  ℹ fail 0
  ```
- `cargo run -q -p vox-doc-pipeline -- --lint-only --paths architecture,reference/vox-fullstack-artifacts.md,adr/036-webir-hir-unification-compare-both.md`:
  ```
  vox-doc-pipeline lint complete — no hard errors.
  ```
- No unexpected edits outside designated files.
