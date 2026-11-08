---
phase: 343-docs-system-ssot-and-automated-drift-control
plan: 06
status: complete
wave: 6
selected_options:
  task-2: option-a
---

# Phase 343 Plan 06 Summary

Unifed the prune link graph to serve both pruning reports and orphan-page gating, linked all 29 reader-facing orphan pages at their sources, implemented the gated `docs_orphan_pages` ssot-drift stage (zero reader-facing orphans, whole-tree reported), and closed the orphan-page gap in the docs system SSOT.

## 1. Key Accomplishments

1. **Unified Link Graph (`prune.rs`):**
   - Refactored `vox_doc_pipeline::claims::prune::build_inbound_link_graph` to take `(repo_root: &Path, docs_dir: &Path, extra_sources: &[PathBuf])`.
   - Strips fenced code blocks (` ``` `) and inline code spans (`` `...` ``) before scanning Markdown links so code examples do not generate false inbound links.
   - Ignores archive paths (`/archive/`) and ignores self-links (`target_rel == source_rel`).
   - Normalizes `repo_root` and `docs_dir` paths to avoid prefix stripping errors with relative root paths.
   - Adds root guides (`AGENTS.md`, `README.md`, `CONTRIBUTING.md`, `CLAUDE.md`, `GEMINI.md`) into `extra_sources` so inbound links from top-level repository guides count toward page reachability.

2. **Orphan-Page Detection & ssot-drift Integration (`docs_orphan_pages.rs`):**
   - Implemented `find_orphans(root: &Path, pages: &[PathBuf]) -> Result<Vec<String>>` with exemptions for index files (`index.md`, `index.mdx`), generated pages (`*.generated.md`), `AGENTS.md`, and paths containing `.well-known/` or `archive/`.
   - Implemented `run(root: &Path) -> Result<()>` gating `Scope::ReaderFacing` (plus tutorials) at zero orphans, printing actionable heal hints for any orphan detected, and reporting whole-tree orphan counts (primarily architecture research documents).
   - Hooked `docs_orphan_pages` into `ssot_drift_stages()` right before `ssot_regen_parity`.

3. **Task 2 Checkpoint Decision (`option-a`):**
   - Selected `option-a`: linked every reader-facing `status: current` orphan from an existing related page; gated reader-facing pages at zero with no baseline ledger; reported whole-tree count without gating.
   - Updated `.planning/ROADMAP.md` line 193 to say "zero reader-facing orphans (no baseline)" in place of "shrink-only baseline".

4. **Linked All 29 Reader-Facing Orphans:**
   - Linked all 29 previously orphaned pages from existing section indexes or related sibling pages without creating new host pages.
   - Verified that `find_orphans` on `Scope::ReaderFacing` returns 0 orphans.
   - Whole-tree orphans dropped from 244 to 215.

5. **SSOT & Coverage Ledger Updated:**
   - In `docs/src/architecture/docs-system-ssot.md`:
     - Added §4 table row for `Orphan-page check (docs_orphan_pages ssot-drift stage)`.
     - Deleted the orphan gap bullet from §6 ("Orphan pages. Nothing flags pages that no other page links to...").
   - In `docs/src/contributors/detector-coverage-ledger.md`:
     - Added row for `docs_orphan_pages` (Bug class: "Docs page unreachable by links", Severity: blocking, Enforcement: pre-push fast + PR CI linux via ssot-drift).
   - Validated docs with `cargo run -p vox-doc-pipeline -- --lint-only`.

## 2. Reader-Facing Orphans & Host Links (29 Pages Resolved)

| # | Orphan Page | Host Page | Link Description |
|---|---|---|---|
| 1 | `docs/src/explanation/expl-ai-orchestration.md` | `docs/src/explanation/why-vox-for-ai.md` | Added to Next Steps: AI Agent Orchestration |
| 2 | `docs/src/explanation/glossary.md` | `docs/src/explanation/faq.md` | Added to intro: Glossary definition link |
| 3 | `docs/src/how-to/clinical-export-from-vox-apps.md` | `docs/src/how-to/index.md` | Added under Language & Development Recipes |
| 4 | `docs/src/how-to/how-to-axis-drive-openrouter-e2e.md` | `docs/src/how-to/index.md` | Added under Language & Development Recipes |
| 5 | `docs/src/how-to/how-to-custom-types.md` | `docs/src/how-to/index.md` | Added under Language & Development Recipes |
| 6 | `docs/src/how-to/how-to-error-handling.md` | `docs/src/how-to/index.md` | Added under Language & Development Recipes |
| 7 | `docs/src/how-to/how-to-scale-actors.md` | `docs/src/how-to/index.md` | Added under Language & Development Recipes |
| 8 | `docs/src/how-to/how-to-share.md` | `docs/src/how-to/index.md` | Added under Language & Development Recipes |
| 9 | `docs/src/how-to/how-to-train-mens-macos-metal.md` | `docs/src/how-to/index.md` & `how-to-train-mens-4080.md` | Added under Language & Development Recipes and cross-linked in RTX 4080 guide |
| 10 | `docs/src/how-to/how-to-unit-test.md` | `docs/src/how-to/index.md` & `how-to-testing-integration.md` | Added under Language & Development Recipes and cross-linked in Testing Integration |
| 11 | `docs/src/how-to/python-client-from-vox-openapi.md` | `docs/src/how-to/index.md` | Added under Language & Development Recipes |
| 12 | `docs/src/reference/agent-quick-reference.md` | `docs/src/contributors/contributor-hub.md` | Added under Start Here list |
| 13 | `docs/src/reference/ai-ide-feature-matrix.md` | `docs/src/how-to/editor-integrations.md` | Added under intro capability inventory |
| 14 | `docs/src/reference/build-profiles.md` | `docs/src/reference/feature-builds.md` | Added under build profile intro |
| 15 | `docs/src/reference/cli-json-output.md` | `docs/src/reference/cli.md` | Added to `--json` flag description |
| 16 | `docs/src/reference/cli-vox-deploy.md` | `docs/src/how-to/how-to-deploy.md` | Added under deploy intro |
| 17 | `docs/src/reference/mens-quality-scoring.md` | `docs/src/reference/mens-training.md` | Added under tokenization and defaults |
| 18 | `docs/src/reference/ref-doc-comments.md` | `docs/src/reference/ref-syntax.md` | Added under syntax reference intro |
| 19 | `docs/src/reference/ref-stdlib.md` | `docs/src/reference/ref-stdlib-index.md` | Added under See also |
| 20 | `docs/src/reference/ref-web-model.md` | `docs/src/reference/vox-web-stack.md` | Added under reactive UI note |
| 21 | `docs/src/reference/repo-root-files.md` | `docs/src/contributors/contributor-hub.md` | Added under Start Here list |
| 22 | `docs/src/reference/review-antipattern-catalog-contract.md` | `docs/src/reference/mens-training-data-contract.md` | Added under Related contracts |
| 23 | `docs/src/reference/review-fix-pairs-contract.md` | `docs/src/reference/mens-training-data-contract.md` | Added under Related contracts |
| 24 | `docs/src/reference/review-regression-challenges-contract.md` | `docs/src/reference/mens-training-data-contract.md` | Added under Related contracts |
| 25 | `docs/src/reference/shipped-v0.4.md` | `docs/src/reference/stability.md` | Added under stability matrix intro |
| 26 | `docs/src/reference/std-surfaces.md` | `docs/src/reference/ref-stdlib-index.md` | Added under See also |
| 27 | `docs/src/reference/vox-ir-specification.md` | `docs/src/reference/vox-fullstack-artifacts.md` | Added under fullstack artifacts intro |
| 28 | `docs/src/reference/vox-system-prompt.md` | `docs/src/reference/ref-syntax.md` | Added under syntax reference intro |
| 29 | `docs/src/reference/webhook_integration.md` | `docs/src/reference/communication-protocols.md` | Added under Related protocols |

## 3. Mutation Proofs

1. **Tracer Scope Mutation Proof (Task 1):**
   - Temporarily removed the link to `use-a-react-component-from-vox.md` from `docs/src/tutorials/tut-getting-started.md`.
   - Executed `cargo test -p vox-cli --lib docs_orphan_pages::tests::test_run_tutorials_tracer_real_tree`.
   - Verified test failed, identifying `docs/src/tutorials/use-a-react-component-from-vox.md (HEAL: link it from a related existing page)`.
   - Restored `tut-getting-started.md` and verified byte-identical match with `cmp -s`.

2. **Reader-Facing Real-Tree Mutation Proof (Task 3):**
   - Backed up `docs/src/explanation/faq.md` to `faq.md.bak`.
   - Removed the link to `glossary.md` from `docs/src/explanation/faq.md`.
   - Executed `cargo test -p vox-cli --lib docs_orphan_pages::tests::test_run_reader_facing_real_tree`.
   - Verified failure output:
     ```text
     docs_orphan_pages: found 1 orphaned reader-facing page(s):
       docs/src/explanation/glossary.md (HEAL: link it from a related existing page)
     ```
   - Restored `faq.md` from backup and verified byte-identical match with `cmp -s`.
   - Re-ran `test_run_reader_facing_real_tree`, confirming clean pass with 0 reader-facing orphans.

## 4. Verification Results

- `cargo test -p vox-doc-pipeline --lib claims::prune`: passed.
- `cargo test -p vox-cli --lib docs_orphan_pages::tests`: 4 passed, 0 failed.
- `cargo run -p vox-doc-pipeline -- --lint-only --paths ...`: passed with no errors across all touched documentation pages.
- `rustfmt --edition 2024`: all modified Rust source files formatted cleanly.
- `cargo build -p vox-cli`: built cleanly.
- `CI=1 ./target/debug/vox ci ssot-drift`: all stages passed, including:
  - `docs_cli_claims: 215ms`
  - `docs_orphan_pages: 760ms` (`docs_orphan_pages OK (zero reader-facing orphans)`)
  - `ssot_regen_parity: 2ms`
  - `ssot-drift: nested SSOT guards OK`
