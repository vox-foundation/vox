---
phase: 20-deploy-unblock-public-surface-honesty
plan: 03
subsystem: docs-governance
tags: [docs, governance, frontmatter, status, honesty]
status: complete

requires: []
provides:
  - "One authoritative docs design: two overlapping specs status deprecated + superseded notice naming .planning/workstreams/docs-freshness/ROADMAP.md"
  - "docs-reality-audit-program.md status roadmap with a dormant notice (phase 21 reactivates it)"
  - "documentation-governance.md: '### Internals section' (page-status.mjs single source) and a true 'Last updated' derivation note"
affects: [20-06, 20-07, 21]

actuals:
  tokens: 1000
  tasks: 2
  commits: 2
plan_head_before: 01b076ace23cb077d6e4684b0c5e3451bd1fe8cc

tech-stack:
  added: []
  patterns:
    - "Superseded-design notice: blockquote right after frontmatter, dated, naming the authoritative roadmap and saying 'Do not implement from this document'"

key-files:
  created: []
  modified:
    - docs/superpowers/specs/2026-08-22-docs-corpus-repair-design.md
    - docs/superpowers/specs/2026-09-14-deep-research-documentation-engine-design.md
    - docs/superpowers/plans/2026-09-14-deep-research-documentation-engine.md
    - docs/src/contributors/docs-reality-audit-program.md
    - docs/src/contributors/documentation-governance.md

key-decisions:
  - "docs-reality-audit-program.md gets a training_rationale rather than dropping training_eligible: the doc lint requires a rationale on roadmap pages"
  - "The governance doc names page-status.mjs (shipped by 20-06) as the Internals single source ahead of that plan landing, per the plan's 'describe what this phase ships' intent"

duration: 20min
completed: 2026-10-09
---

# Phase 20 Plan 03: Authoritative Design and Honest Governance Doc Summary

**The docs-freshness roadmap is now the only authoritative docs design; the two overlapping specs and their plan are marked superseded, the dormant reality-audit program is labelled roadmap, and the governance doc describes the Internals section and the real git-derived "Last updated" rule instead of crediting "the AI search engine".**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-10-09
- **Tasks:** 2/2
- **Files modified:** 5

## Accomplishments

Status and notice changes (HONEST-03):

1. `docs/superpowers/specs/2026-08-22-docs-corpus-repair-design.md` — `status: "roadmap"` → `"deprecated"`; superseded blockquote after frontmatter.
2. `docs/superpowers/specs/2026-09-14-deep-research-documentation-engine-design.md` — `status: "current"` → `"deprecated"`; superseded blockquote after frontmatter.
3. `docs/superpowers/plans/2026-09-14-deep-research-documentation-engine.md` — superseded blockquote on line 1 (no frontmatter added).
4. `docs/src/contributors/docs-reality-audit-program.md` — `status: "current"` → `"roadmap"`; "Dormant (2026-10-08)" blockquote after the H1 saying phase 21 of docs-freshness reactivates it; `training_rationale` added.

Governance sections touched (`documentation-governance.md`, HONEST-02 / DEPLOY-04):

- Category table: the `Architecture SSOTs` row now points research and roadmap pages to Internals.
- New `### Internals section` under the status vocabulary: research/roadmap → collapsed Internals sidebar group (shown last), banner, `noindex`, omitted from sitemap and every llms.txt variant, still searchable on-site and labelled Internals; deprecated/legacy keep category with banner + `noindex`; single source `docs-astro/src/utils/page-status.mjs`; no category or frontmatter rewrite needed.
- "Note on temporal metadata" rewritten: committer date of the newest commit touching the source file, computed by `git-dates.mjs`; skips `.git-blame-ignore-revs` SHAs, `chore(ssot): auto-regenerate`, formatting commits and >100-file bulk commits; follows renames; frontmatter `last_updated` ignored; add future sweep SHAs to `.git-blame-ignore-revs`. Wording checked against `DEFAULT_IGNORE_SUBJECTS`, `bulkThreshold = 100` and `%cI` in `git-dates.mjs`.

## Task Commits

1. **Task 1: Mark overlapping designs superseded; audit program roadmap** — `ddf7e43ab`
2. **Task 2: Governance doc describes Internals and true date derivation** — `0ec734092`

## Verification

- Superseded-notice grep: count 1 in each of the three superpowers files.
- Frontmatter status (first `^status:` per file): `deprecated`, `deprecated`, `roadmap`.
- `grep -c 'Phase 21 of the docs-freshness workstream'` on the audit program: 1.
- `cargo run -q -p vox-doc-pipeline -- --lint-only --paths contributors/docs-reality-audit-program.md`: no hard errors (after the rationale fix).
- Governance greps: `### Internals section` 1, `AI search engine` 0, `.git-blame-ignore-revs` 1; `searchable` and `page-status.mjs` present.
- `vox-doc-pipeline` lint and `pnpm dlx markdownlint-cli2` on both `docs/src/contributors` files: 0 errors / 0 issues.
- Task 1 diff was 12 insertions / 3 deletions; Task 2 diff 8 insertions / 2 deletions (no reflow). Staged diffs contained no `lean-ctx` banner.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added `training_rationale` to docs-reality-audit-program.md**
- **Found during:** Task 1 verify
- **Issue:** Changing the page to `status: "roadmap"` made `vox-doc-pipeline` fail with `missing-training-rationale` (`training_eligible: true` on research/roadmap pages requires a rationale).
- **Fix:** Added one `training_rationale:` frontmatter line.
- **Files modified:** `docs/src/contributors/docs-reality-audit-program.md`
- **Commit:** `ddf7e43ab`

**2. [Verify defect] Task 1 status grep matches body content**
- **Found during:** Task 1 verify
- **Issue:** `grep -h '^status:'` also matches an example frontmatter block inside the deep-research spec body (`status: "current"`, describing generated docs), printing four lines instead of three.
- **Resolution:** Left the example unchanged (it is template content, not the file's status) and verified with a first-match-per-file check, which gives the intended three lines. Logged in `deferred-items.md`.

### Tooling incident (no committed impact)

The Cursor edit tool wrote back a lean-ctx-compressed copy of the deep-research plan file (1,101 lines dropped). The file was restored from HEAD with `git checkout -- <file>` before anything was staged, and all edits were then applied by throwaway Node scripts in `/tmp` that do exact replacements and fail unless each anchor matches exactly once.

## Deferred Issues

See `deferred-items.md`: the governance doc's frontmatter starter template (`status: "roadmap"` + `training_eligible: true`, no rationale) fails the same lint it claims to pass.

## Known Stubs

None. `docs-astro/src/utils/page-status.mjs` is referenced by the governance doc but is created by plan 20-06 in this phase.

## Self-Check: PASSED
