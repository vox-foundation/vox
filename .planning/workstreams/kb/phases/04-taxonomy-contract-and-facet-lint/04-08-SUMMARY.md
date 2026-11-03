---
phase: 04-taxonomy-contract-and-facet-lint
plan: 08
subsystem: doc-pipeline
tags: [kb, facets, backlog, contracts, docs]
requires: ["04-03", "04-04", "04-06", "04-07"]
provides:
  - "contracts/documentation/kb-facet-backlog.v1.yaml (epoch 1, seeded by --seed-facet-backlog)"
  - "contracts/index.yaml ids documentation-kb-taxonomy, documentation-kb-facets-schema, documentation-kb-facet-backlog"
  - "documentation-governance.md section 'KB facets (taxonomy contract)' and facet starter template"
affects: []
key-files:
  created:
    - contracts/documentation/kb-facet-backlog.v1.yaml
  modified:
    - contracts/index.yaml
    - docs/src/contributors/documentation-governance.md
    - docs/src/architecture/where-things-live.md
    - docs/src/contributors/local-ci-pre-push.md
    - AGENTS.md
    - CHANGELOG.md
key-decisions:
  - "Backlog produced only by the seed command; no facet written into any existing doc"
  - "Summary-block example sits in a column-0 fence in the governance doc (the summary scanner only recognises column-0 fences)"
requirements-completed: [TAX-03, TAX-04, TAX-05]
completed: 2026-10-11
status: complete
commits: 2
plan_head_before: b3c3dedaded567f94058f9e750c3db0ba6080870
actuals:
  tokens: 9000
  tasks: 2
  commits: 2
---

# Phase 4 Plan 08: Activate the facet lint on the real tree Summary

The facet lint is now live on the real tree: the seeded shrink-only backlog excuses existing docs, a new unfaceted doc fails, the three KB contracts are indexed, and the facet rules are documented in AGENTS.md and the governance SSOT.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: seed backlog, index contracts, prove activation | e48e22ed9 |
| 2 | Document facet rules (governance, where-things-live, pre-push, AGENTS.md, CHANGELOG) | 0d95bd324 |

## Backlog seed (for the PR description)

`seeded 667 docs, 2668 rule ids (epoch 1)` (667 x 4 rule ids; archive and `*.generated.md` excluded, verified by grep):

| Rule id | Docs |
|---------|------|
| facet-missing-topic | 667 |
| facet-missing-domain | 667 |
| facet-missing-summary | 667 |
| facet-missing-authorship | 667 |

The file was not edited by hand; `--tighten-facet-backlog` afterwards reports "already tight" and leaves it unchanged.

## Verification

- `vox ci contracts-index`: OK (196 entries). `vox ci check-links`: PASS (4650 links). (installed `vox` is stale; run with `VOX_SKIP_FRESHNESS_CHECK=1`.)
- Full real-tree `cargo run -p vox-doc-pipeline -- --lint-only`: "no hard errors", exit 0, run both after the seed and after the doc edits. No facet-* output.
- Probe: a throwaway `docs/src/kb-facet-probe.md` (valid title/description/category, no facets) failed the scoped lint with facet-missing-topic, -domain, -summary, -authorship (4 hard errors); deleted before commit (`test ! -e` confirmed).
- A backlog doc (`contributors/documentation-governance.md`) scoped lint: exit 0.
- Scoped lint of the three edited docs: no hard errors; `--tighten-facet-backlog` after the edits left the backlog byte-identical (the edits neither fixed nor added a violation).
- TAX-05 prohibition: `git diff b3c3dedad..HEAD -- docs/src` touches only governance, where-things-live and local-ci-pre-push; no facet written into any existing doc.

## Mutation proof

Deleted the `docs/src/architecture/where-things-live.md` row from the backlog (1 occurrence before, 0 after): the scoped lint then failed with the four facet-missing errors; `git checkout` restored it (1 occurrence), lint clean again.

## Deviations from Plan

**1. [Rule 1 - Bug in my own doc] Summary example inside an indented fence tripped `facet-summary-placement`.** The first draft of the governance section put the `kb:summary` example in a fence indented under a list item; the summary scanner (`vox-kb-facets/src/summary.rs`, `is_fence` uses `starts_with` with no indentation allowance) does not treat indented fences as fences, so it saw a misplaced real block. Fixed by moving the example to a column-0 fence. Not changing the scanner: out of scope. Noted as a minor scanner limitation (CommonMark allows up to 3 spaces of fence indent); a doc that indents a fence containing the marker lines will get a false `facet-summary-placement`.

**2. [Process] Task edits to text files done with one-off scripted exact-match replacement** (no committed glue).

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

Backlog file, SUMMARY and both commits (e48e22ed9, 0d95bd324) exist.
