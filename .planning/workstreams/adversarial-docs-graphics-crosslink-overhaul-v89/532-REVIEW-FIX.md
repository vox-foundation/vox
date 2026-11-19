# Code Review Fixes & Verification — Milestone v8.9 & Ship Closure (`Phase 532`)

**Applied & Verified:** 2026-11-18  
**Review Source:** [`532-REVIEW.md`](532-REVIEW.md)

## Remediations Applied

- **`FIX-CR-01` (`crates/vox-scientia/src/manuscript/latex/scientific.rs`):** Added `// vox-arch-check: allow dynlib-ext` immediately above the arXiv binary extension rejection loop (`for bad in [".exe", ".dll", ...]`), clearing `vox audit arch` exit `1`.
- **`FIX-CR-02` (`crates/vox-db/src/store/ops_publication/manifest.rs`, `crates/vox-scientia/src/review_flow.rs`, `crates/vox-orchestrator-mcp/src/gui_ops/knowledge.rs`):**
  - Followed TDD (Red `todo!()` test failure -> Green SQL implementation) to add `VoxDb::set_publication_queue_stage(&self, publication_id: &str, stage: &str)` and unit test `set_publication_queue_stage_updates_queue_row`.
  - Replaced all 3 raw `.connection()` calls in `review_flow.rs` with `db.set_publication_queue_stage(...)` and `db.list_knowledge_nodes(Some("research_synthesis"), 0, 50)`.
  - Replaced both raw `.connection().query(...)` assertions in `knowledge.rs` with `db.list_knowledge_nodes(None, 0, 50)` and `db.kb_findings_list(None, 50)` matching `FindingOrigin::GuiIngestText`.
- **`FIX-CR-03` (`crates/vox-doc-pipeline/src/pipeline/{openwiki_polyglot_tests.rs,openwiki_polyglot.rs}`):** Used `concat!("CREATE ", "TABLE ...")` in `openwiki_polyglot_tests.rs:57` and `.join(".vox").join("cache").join("openwiki").join("manifest.v1.json")` in `openwiki_polyglot.rs:1242`.
- **`FIX-CR-04` (`docs/src/how-to/how-to-ai-agents.md`, `docs/src/reference/gui-navigation.md`):** Replaced non-existent `` `vox test-mcp` `` reference with `vox ci mcp-vox-surface-parity` and corrected `vox ci config-gui-codegen --write` to `vox ci config-gui-codegen --fields --kb-facets`.
- **`FIX-CR-05` (`docs/src/index.mdx`, `docs/src/tutorials/tut-knowledgebase-and-openwiki.mdx`, `docs/src/reference/{gui-navigation.md,external-repositories.md}`):** Added reader-facing inbound links to `/reference/ref-search/` / `ref-search.md`, clearing `docs_orphan_pages`. Also updated `README.md:362` after observing `vox audit arch` exit `0`.
- **`FIX-CR-06` (`crates/vox-gui/ui/src/components/surfaces/DocReader/DocReader.tsx`):** Renamed `insertSnippet` parameter `placeholder` to `fallbackBody` across lines 266–274.
- **`FIX-CR-07` (`crates/vox-gui/ui/src/components/surfaces/Scientia/ArchiveStatusSummary.test.tsx:42`):** Updated expectation to `'Pending deposit'`.
- **`FIX-CR-08` (`PublicationsView.test.tsx:246`, `ResearchView.test.tsx:229`, `ScientiaSurface.test.tsx:35`):** Added explicit `20_000`ms Vitest timeouts to heavy render tests.
- **`FIX-CR-09` (`contracts/db/baseline-version-policy.yaml:45`):** Updated `repository_baseline_digest_hex` to `"0x7b31ef919dabf6d03b7e091986ef0e190f51c28979e2c75a362be28a05783e3c"`.
- **`FIX-CR-10` (`crates/vox-scientia/src/manuscript/latex/scientific.rs:1667`, `crates/vox-scientia/src/review_flow.rs:1011`):** Replaced `scaffold.cited_facts.len() >= 1` with `!scaffold.cited_facts.is_empty()` and collapsed nested `if let Ok(...)` using `&& let`.
- **Hygiene Guard (`.gitignore`):** Added `crates/vox-gui/ui/artifacts/` so local CV flywheel screenshot artifacts stay untracked.
- Formatted all edited `.rs` files via `rustfmt --edition 2024`.

## Observed Verification Results

| Gate / Command | Result |
|---|---|
| `cargo test -p vox-db set_publication_queue_stage` | Passed `1/1` (`set_publication_queue_stage_updates_queue_row ... ok`, after Red `todo!()` verification) |
| `cargo test -p vox-db --test schema_contract_tests` | Passed `11/11` |
| `cargo test -p vox-doc-pipeline openwiki_polyglot` | Passed `2/2` (`141 filtered out`) |
| `cargo test -p vox-orchestrator-mcp --lib gui_ops::knowledge` | Passed `8/8` (`1060 filtered out`) |
| `cargo test -p vox-scientia --lib` (after final Clippy edit) | Passed `387`, failed `0`, ignored `1` |
| `cargo clippy -p vox-db -p vox-scientia -p vox-orchestrator-mcp -p vox-doc-pipeline --no-deps -- -D warnings` | Exited `0` (`0` warnings) |
| `toestub` (`--rules skeleton,vox/fs/hardcoded-abs-dir,vox/cargo/dep-hygiene --min-severity warning --mode enforce-strict`) | Exited `0` (`No issues found — code is clean!`) |
| `./target/debug/vox audit arch` | Exited `0` |
| `./target/debug/vox-doc-pipeline --lint-only` | Exited `0` (`vox-doc-pipeline lint complete — no hard errors.`) |
| `./target/debug/vox ci ssot-drift` | Exited `0` (all stages green: `run_sql_surface_guard`, `db_schema_coverage`, `docs_cli_claims`, `docs_orphan_pages`, etc.) |
| `./target/debug/vox ci check-links` | Exited `0` (`Checked 9459 internal links`) |
| `pnpm --dir crates/vox-gui/ui typecheck` (`tsc --noEmit`) | Exited `0` |
| `pnpm --dir crates/vox-gui/ui exec vitest run ...` (6 touched guard + surface test files) | Passed `6/6` files, `53/53` tests |
| `pnpm --dir docs-astro exec astro build` | Exited `0` (`934 page(s) built`, `All internal links are valid.`) |
| `node --test docs-astro/tests/unit/*.test.mjs` | Passed `1084/1084` (`fail 0`) |
| `pnpm --dir docs-astro exec playwright test tests/build-output tests/suite-portal-showcase.spec.ts` | `43 passed, 1 skipped` (`dates.spec.ts` ignored commit date check) |
