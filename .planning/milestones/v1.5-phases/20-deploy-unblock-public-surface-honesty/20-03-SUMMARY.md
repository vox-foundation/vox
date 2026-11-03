# Plan 20-03 Summary: Authoritative Documentation Governance & Superseded Specs

## Completed Tasks
- **Task 1: Mark overlapping design files superseded and audit program roadmap**
  - Updated `docs/superpowers/specs/2026-08-22-docs-corpus-repair-design.md`: frontmatter `status: "deprecated"`; added notice stating it is superseded by the `docs-freshness` workstream.
  - Updated `docs/superpowers/specs/2026-09-14-deep-research-documentation-engine-design.md`: frontmatter `status: "deprecated"`; added notice stating it is superseded by the `docs-freshness` workstream.
  - Updated `docs/superpowers/plans/2026-09-14-deep-research-documentation-engine.md`: added superseded blockquote at the top of the file.
  - Updated `docs/src/contributors/docs-reality-audit-program.md`: frontmatter `status: "roadmap"`, added `training_rationale: "Roadmap for documentation reality audit program and claim taxonomy."`, and inserted dormant notice indicating Phase 21 reactivates it.
  - Verified with `vox-doc-pipeline --lint-only`.
- **Task 2: Governance doc describes Internals and true date derivation**
  - Updated `docs/src/contributors/documentation-governance.md`:
    - Updated `Architecture SSOTs` row in the category table to note research and roadmap pages appear under Internals.
    - Added `### Internals section` describing the routing of `status: research` and `status: roadmap` pages under the collapsed **Internals** sidebar, status banner, `noindex`, exclusion from llms variants and sitemap, single source `page-status.mjs`.
    - Rewrote `Note on temporal metadata` to truthfully state that dates are committer dates computed by `git-dates.mjs` skipping `.git-blame-ignore-revs` and bulk/format commits, and removed the obsolete reference to the AI search engine.
  - Verified with `vox-doc-pipeline` and `markdownlint-cli2` (0 errors).

## Verification Evidence
- Superseded notices: 1 occurrence in each of the 3 superpowers files.
- Frontmatter statuses: `deprecated`, `deprecated`, `roadmap`.
- `### Internals section`: 1 occurrence.
- `AI search engine`: 0 occurrences.
- `.git-blame-ignore-revs`: 1 occurrence.
- Markdownlint and doc-pipeline lint: clean.
