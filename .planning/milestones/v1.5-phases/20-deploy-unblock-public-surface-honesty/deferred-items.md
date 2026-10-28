# Phase 20 — Deferred Items

- **Status:** resolved

All out-of-scope discoveries logged during Phase 20 have been resolved and verified.

| Found in | File | Issue | Status |
|---|---|---|---|
| 20-03 | `docs/src/contributors/documentation-governance.md` ("Frontmatter starter template") | Updated starter template to `status: "current"` so docs lint passes on first run. | resolved |
| 20-03 | Plan 20-03 Task 1 verify | Frontmatter-only check (`rg -m1 '^status:'`) documented. | resolved |
| 20-04 | `crates/vox-codegen/src/codegen_rust/emit/workflow.rs` (`emit_fn_with_actor_handlers`) | Removed `if !func.actor_state_fields.is_empty()` guard so actor handlers always route into actor dispatch match. | resolved |
| 20-04 | `crates/vox-compiler/src/eval/expr.rs` (`HirExpr::For`) | `VoxValue::_Break` inside a `for` loop now breaks out of the loop and returns `VoxValue::Null` instead of leaking `_Break`. | resolved |
| 20-04 | `crates/vox-compiler/src/parser/descent/expr/pratt_ops.rs` | Parser now treats `is not` as the compound inequality operator `BinOp::Isnt`. | resolved |
| 20-04 | `contracts/index.yaml` | Registered `contracts/documentation/tutorial-verification.v1.json` and `contracts/documentation/docs-sidebar-section-order.v1.json` in `contracts/index.yaml`. | resolved |
| 20-08 | `docs-astro/astro.config.mjs` (`editLink.baseUrl`) | Resolved in 20-09 (`src/routeData.ts` sets real repo file URL). | resolved |
| 20-08 | `docs/src/explanation/expl-architecture.md` | Resolved in 20-10 (`docs-astro/public/media/durable_essentialist_loop.webp`). | resolved |
| 20-09 | `contracts/index.yaml` | Registered `contracts/documentation/site-mounted-repo-docs.v1.json` in `contracts/index.yaml`. | resolved |
| 20-09 | `docs-astro/README.md` | Rewrote `docs-astro/README.md` to document the Vox Starlight documentation site (`voxlang.org`). | resolved |
| 20-09 | Mounted repo Markdown (`/repo/*`) | Dead links fixed in 20-09 (`tree-sitter-vox`, `vox-vscode`, `build-timings` READMEs). | resolved |
| 20-08 | `docs-astro/node_modules/.astro/data-store.json` | `docs-astro/scripts/setup-content.mjs` (`prebuild`/`predev`) now removes `data-store.json` so local builds run fresh. | resolved |
| 20-REVIEW WR-03 | `docs-astro/src/plugins/remark-doc-links.mjs` (`docLinksGate`) | Resolved by clearing `data-store.json` in `setup-content.mjs`. | resolved |
| 20-REVIEW WR-05 | `.github/workflows/docs-deploy.yml`, `.github/workflows/docs-quality.yml` | Resolved (`workflow-paths.test.mjs` + `contracts/documentation/**`). | resolved |
| 20-REVIEW WR-06 | `docs/src/architecture/*-research-*.md`, `*-findings-*.md` | Resolved in Phase 58 (`58-01..58-03`: 810 verified, 0 drifting, 0 broken, 308 historical, 0 unanchored). | resolved |
