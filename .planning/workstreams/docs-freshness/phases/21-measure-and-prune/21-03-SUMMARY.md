---
phase: "21-measure-and-prune"
plan: "03"
status: completed
date: "2026-10-13"
requirements_met:
  - MEASURE-05
---

# Plan 21-03 Summary: Documentation Prune Report Engine & Artifacts

## Execution Summary
Implemented the documentation prune report engine in `crates/vox-doc-pipeline::claims::prune` and generated the complete corpus prune report across all 971 documentation pages.

## Delivered Components
1. **Prune Engine (`crates/vox-doc-pipeline/src/claims/prune.rs`)**:
   - Inbound link graph builder tracking internal links and finding orphaned pages.
   - Substantive git author date fetcher with mechanical commit filtering (`chore(docs): regenerate`, `ssot-autoregen`, `style: fmt`, `ci: sync`).
   - Duplicate-topic clusterer utilizing Jaccard token similarity ($\ge 0.6$).
   - Rule-based disposition recommender evaluating the 6 dimensions into `keep`, `merge`, `move-to-internals`, or `archive`.
2. **CLI Integration (`crates/vox-cli/src/commands/audit.rs`)**:
   - Wired `--prune-report` and `--prune-out` flags into `vox audit docs`.
   - Supports both machine-readable JSON format and readable Markdown report formats.
3. **Artifacts Generated**:
   - `contracts/reports/docs-freshness/prune-report.v1.json`: Machine-readable prune database covering 971 pages.
   - `docs/src/architecture/docs-prune-report.md`: Human-readable Markdown summary with canonical frontmatter.
   - Corpus statistics:
     - Total pages: 971
     - Archive candidates: 414
     - Move to Internals candidates: 183
     - Keep: 345
     - Merge candidates: 29
     - Topic clusters detected: 43

## Verification
- Engine tests: `timeout 60s cargo test -p vox-doc-pipeline claims::prune` passed.
- All claims tests: `timeout 60s cargo test -p vox-doc-pipeline claims` passed (5/5 ok).
- Prune report CLI: `cargo run -p vox-cli -- audit docs --prune-report --prune-out ...` executed cleanly and emitted valid JSON and Markdown.
- Doc pipeline lint: `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/docs-prune-report.md` passed with 0 errors.
