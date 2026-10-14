---
phase: 04-taxonomy-contract-and-facet-lint
plan: 02
subsystem: kb-facets
tags: [kb, taxonomy, skos, query-expansion, contract]
requires: ["04-01"]
provides:
  - "vox_kb_facets::taxonomy: TAXONOMY_REL, Concept, Taxonomy, LabelMatch, from_yaml_str, resolve_topic, resolve_domain, topic_labels, domain_labels, expand_query, facet_card_lines"
  - "contracts/documentation/kb-taxonomy.v1.yaml: 11 domains, 55 topics (D-07)"
affects: [04-05, 04-06, 04-08]
tech-stack:
  added: []
  patterns: ["deny_unknown_fields contract loader that reports every curation error at once", "whole-token altLabel match emitting only validated prefLabel phrases"]
key-files:
  created:
    - crates/vox-kb-facets/src/taxonomy.rs
    - contracts/documentation/kb-taxonomy.v1.yaml
  modified:
    - crates/vox-kb-facets/src/lib.rs
key-decisions:
  - "Topic prefLabels must differ from domain prefLabels (one label namespace), so the domain `orchestration` has the topic `agent-orchestrator`"
  - "expand_query matches only altLabels and deprecated aliases, never the prefLabel itself (a doc already tagged with it needs no expansion)"
  - "Every scopeNote is double-quoted in the YAML because they contain `: `"
requirements-completed: [TAX-01]
duration: ~25min
completed: 2026-10-11
status: complete
commits: 2
plan_head_before: ffcf127b65f56b89390d1a7844185b2c66932376
actuals:
  tokens: 11000
  tasks: 2
  commits: 2
---

# Phase 4 Plan 02: SKOS-lite taxonomy and altLabel query expansion Summary

A validated controlled vocabulary (11 domains, 55 topics) with a loader that resolves prefLabels, altLabels and deprecated aliases, and an `expand_query` that turns an altLabel or retired name in a query into a quoted `"topic <prefLabel>"` facet phrase.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: taxonomy YAML -> validated Taxonomy -> label resolution and query expansion | a8d2782fa |
| 2 | Curate kb-taxonomy.v1.yaml from where-things-live.md | 25d4519f6 |

Tracer verified end-to-end (15 taxonomy tests green) before the curation task.

## Tests (40 in crate, all passing, `cargo nextest run -p vox-kb-facets`)

Behavior bullet -> test:
- fixture load, Pref/Alt/Deprecated/Unknown and case rules -> `resolve_topic_pref_alt_deprecated_unknown`
- altLabel appends phrase -> `expand_query_alt_label_appends_phrase`
- deprecated alias expands to replacement -> `expand_query_deprecated_alias_expands_to_replacement`
- multi-word label appends once -> `expand_query_multiword_label_appends_once`
- whole-token boundaries -> `expand_query_requires_whole_token`
- no match / odd quotes unchanged -> `expand_query_no_match_and_odd_quotes_are_unchanged`
- only prefLabels emitted (T-04-05) -> `expand_query_emits_only_pref_labels`
- loader rejections -> `from_yaml_rejects_unknown_key`, `_non_kebab_pref_label`, `_duplicate_pref_label`, `_bad_or_missing_broader`, `_unknown_related`, `_ambiguous_label`, `_schema_version_2`
- card lines -> `facet_card_lines_skips_blank_and_absent`
- committed file -> `real_taxonomy_loads_and_meets_curation_bounds`

`cargo clippy -p vox-kb-facets --all-targets -- -D warnings` clean; no `regex` dependency; two `deny_unknown_fields` (Concept, file struct).

## Mutation proofs

- Odd-quote guard (`% 2 == 1` -> `% 2 == 2`): `expand_query_no_match_and_odd_quotes_are_unchanged` failed; restored, presence verified.
- Whole-token left boundary (`!hay[..i]...is_some_and(word)` -> `true`): `expand_query_requires_whole_token` failed; restored by hand (the file was untracked at that point so `git checkout` was a no-op), presence verified, 40/40 green after.

## Curation

- 11 domains: language, runtime, orchestration, knowledge, ml, gui, data, platform, security, mesh, documentation (the plan's target list, unchanged).
- 55 topics, each with `broader`, a scopeNote and altLabels (crates plus specific synonyms; generic single words avoided to keep expansion precise).
- Deprecated aliases: vox-lexer/-parser/-hir/-typeck (compiler-frontend), vox-ars (agent-skills), vox-dei-shim (scientia-research), vox-oratio (speech-stt), vox-dashboard (desktop-app), vox-ludus (gamification).
- Crate-coverage check (every where-things-live layer-table crate is an altLabel of exactly one topic; uniqueness enforced by the loader), output empty:

```
sed -n '/^## Quick reference/,/^## Common tasks/p' docs/src/architecture/where-things-live.md | grep -oE '^\| \[.[a-z0-9-]+.\]' | grep -oE '[a-z0-9][a-z0-9-]*' | sort -u | while read c; do grep -qE "^ *- \"?$c\"? *$" contracts/documentation/kb-taxonomy.v1.yaml || echo "MISSING $c"; done
(no output)
```

## Deviations from Plan

**1. [Rule 1 - Bug] scopeNote values unquoted broke YAML parse.** Notes containing `: ` failed (`mapping values are not allowed`); all scopeNote values are now double-quoted. Found by the real_taxonomy test; fixed in the Task 2 commit.

**2. [Process] Task 1 had no separate RED commit.** Tests and implementation were committed together (same as 04-01); the guards are covered by the mutation proofs above.

**3. [Process] Pre-commit allow-list check for agent-* branches skipped.** This is a sequential run on the claude/ feature branch, per the dispatch instructions; protected-branch and root-pin checks were applied.

## Known Stubs

None.

## Threat Flags

None. T-04-05 and T-04-13 mitigations are implemented and covered by tests and mutation proofs; T-04-14 accepted.

## Self-Check: PASSED

taxonomy.rs and kb-taxonomy.v1.yaml exist; commits a8d2782fa and 25d4519f6 present; `commits: 2` measured from `plan_head_before`.
