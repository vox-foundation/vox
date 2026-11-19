# Code Review — Milestone v8.9 & Ship Closure (`Phase 532`)

**Reviewed:** 2026-11-18  
**Scope:** Full workspace pre-merge code review across `v7.9–v8.9` changes (`crates/vox-scientia`, `crates/vox-orchestrator-mcp`, `crates/vox-db`, `crates/vox-doc-pipeline`, `crates/vox-gui/ui`, `contracts/`, `docs/src/`, `docs-astro/`, and `README.md`) prior to `/gsd-complete-milestone` and PR submission.

## Summary of Findings

| ID | Severity | Gate / Location | Root Cause | Fix Reference |
|---|---|---|---|---|
| `CR-01` | High | `vox audit arch` / `crates/vox-scientia/src/manuscript/latex/scientific.rs:1522` | `no-hardcoded-dynlib-ext` pattern triggered on the `.dll` entry inside the arXiv disallowed binary extension list (`[".exe", ".dll", ".dmg", ...]`). | `FIX-CR-01` |
| `CR-02` | High | `vox ci ssot-drift` (`run_sql_surface_guard`) / `crates/vox-scientia/src/review_flow.rs` (3 sites) & `crates/vox-orchestrator-mcp/src/gui_ops/knowledge.rs` (2 test sites) | Direct `db.connection().execute/query(...)` calls outside `crates/vox-db` (`review_flow.rs:2040,2495,3451` and `knowledge.rs:643,660`), violating the `sql-surface-guard` repository invariant. | `FIX-CR-02` |
| `CR-03` | High | `vox ci ssot-drift` (`db-schema-coverage`) / `crates/vox-doc-pipeline/src/pipeline/openwiki_polyglot_tests.rs:57` & `openwiki_polyglot.rs:1242` | Raw SQL `CREATE TABLE items ...` test fixture in non-owner crate `vox-doc-pipeline` tripped `db-schema-coverage`, plus hardcoded `.vox/` joined path string in `openwiki_polyglot.rs:1242`. | `FIX-CR-03` |
| `CR-04` | High | `vox ci ssot-drift` (`docs_cli_claims`) / `docs/src/how-to/how-to-ai-agents.md:70` & `docs/src/reference/gui-navigation.md:194` | Documentation backticks referenced unimplemented/wrong CLI flags (`vox test-mcp` and `vox ci config-gui-codegen --write`). | `FIX-CR-04` |
| `CR-05` | High | `vox ci ssot-drift` (`docs_orphan_pages`) / `docs/src/reference/ref-search.md` | Reader-facing page `docs/src/reference/ref-search.md` lacked inbound cross-links from other reader-facing documentation and navigation pages. | `FIX-CR-05` |
| `CR-06` | High | `surfaceHonesty.guard.test.ts` / `crates/vox-gui/ui/src/components/surfaces/DocReader/DocReader.tsx:266,269,274` | Parameter identifier `placeholder` inside `insertSnippet` tripped the GUI surface honesty placeholder scan. | `FIX-CR-06` |
| `CR-07` | High | Vitest / `crates/vox-gui/ui/src/components/surfaces/Scientia/ArchiveStatusSummary.test.tsx:42` | Test still asserted `'Pending deposit (sample)'` after `ArchiveStatusSummary.tsx` was updated to render `'Pending deposit'`. | `FIX-CR-07` |
| `CR-08` | Medium | Vitest / `PublicationsView.test.tsx:246`, `ResearchView.test.tsx:229`, `ScientiaSurface.test.tsx:35` | Heavy component integration tests exceeded Vitest's default `5000ms` timeout under parallel execution. | `FIX-CR-08` |
| `CR-09` | High | `vox ci ssot-drift` / `contracts/db/baseline-version-policy.yaml:45` | `repository_baseline_digest_hex` was out of sync with live `vox-db` baseline SQL Keccak256 digest (`0x7b31ef919dabf6d03b7e091986ef0e190f51c28979e2c75a362be28a05783e3c`). | `FIX-CR-09` |
| `CR-10` | Medium | `cargo clippy ... -- -D warnings` / `crates/vox-scientia/src/{manuscript/latex/scientific.rs:1667,review_flow.rs:1011}` | Two Clippy lints (`clippy::len_zero` at `scientific.rs:1667` and `clippy::collapsible_if` at `review_flow.rs:1011`) blocked `-D warnings` clean compilation of `vox-scientia`. | `FIX-CR-10` |
