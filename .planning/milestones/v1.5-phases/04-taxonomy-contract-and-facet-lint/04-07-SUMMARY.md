---
phase: 04-taxonomy-contract-and-facet-lint
plan: 07
subsystem: doc-pipeline
tags: [kb, facets, backlog, ratchet, pre-push]
requires: ["04-05"]
provides:
  - "vox_kb_facets::Backlog::{from_violations, to_yaml_string, tightened, tighten_only_violations}"
  - "vox-doc-pipeline --seed-facet-backlog --epoch-note <why> and --tighten-facet-backlog"
  - "vox_doc_pipeline::pipeline::{facet_backlog_gate_failures, FACET_BACKLOG_REL}"
  - "vox_cli::commands::kb::facet_backlog::{run_facet_backlog_gate, file_at}"
  - "fast pre-push step: vox-doc-pipeline facet backlog (tighten-only vs base)"
affects: [04-08]
key-files:
  created:
    - crates/vox-doc-pipeline/src/pipeline/facet_backlog.rs
    - crates/vox-cli/src/commands/kb/facet_backlog.rs
  modified:
    - crates/vox-kb-facets/src/backlog.rs
    - crates/vox-doc-pipeline/src/pipeline/mod.rs
    - crates/vox-cli/src/commands/kb.rs
    - crates/vox-cli/src/commands/ci/pre_push.rs
key-decisions:
  - "Seed/tighten handlers return before FacetCtx::load so a missing backlog never blocks seeding"
  - "An addition is accepted only with epoch+1 and a changed non-empty epoch_note; same-epoch additions, a lower epoch, and a deleted file fail"
  - "Base read is fail-closed: unresolvable base or any git error fails; only an ls-tree that lists nothing bootstraps"
requirements-completed: [TAX-04]
completed: 2026-10-11
status: complete
commits: 2
plan_head_before: f85193759d9988fa4c1c37f6e98f0c3025d12fdc
actuals:
  tokens: 11000
  tasks: 2
  commits: 2
---

# Phase 4 Plan 07: Facet backlog ratchet tools Summary

The facet backlog now has a seeding command, a removal-only `--tighten-facet-backlog`, and a base-ref gate in the fast pre-push tier (so hosted CI too) that rejects any added (path, rule) without a user-authorized re-epoch.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: seed fixture, fix a doc, tighten removes, addition refused | f0976ce7b |
| 2 | Tighten-only gate against the PR base in fast pre-push | c27447fa2 |

Tracer verified end-to-end (kb-facets and doc-pipeline tests green) before expanding.

## Tests (all green; clippy `-D warnings` clean on vox-kb-facets, vox-doc-pipeline, vox-cli lib+tests)

- vox-kb-facets: `from_violations_is_sorted_roundtrips_and_deterministic`, `tighten_is_removal_only`, `gate_rejects_same_epoch_addition`
- vox-doc-pipeline: `facet_backlog_seed_then_tighten_then_refuse_addition` (seed via `lint_in` with no backlog file present, rule removal, fully-clean row removal, deleted-file row removal, refused addition naming `docs/src/c.md` + `USER-AUTHORIZED-ONLY`, file untouched on refusal, missing `--epoch-note` returns 1, re-seed writes epoch 2, canonical bytes), `facet_backlog_seed_summary_line_and_blank_note` (`seeded 2 docs, 8 rule ids (epoch 1)`), `facet_backlog_gate_text_cases` (bootstrap, shrink, addition, re-epoch, unchanged note, deletion, parse errors)
- vox-cli: `file_at_bootstraps_only_on_proven_absence` (fake git), `gate_rejects_same_epoch_addition` (real temp git repo: absent/absent, unresolvable base, bootstrap, identical, same-epoch addition, re-epoch, deletion)
- Regression: `cargo nextest run -p vox-cli facet_backlog pre_push`: 27 passed
- `vox ci crate-edges`: OK (the one stale-edge warning `vox-gui -> vox-db-types` is pre-existing, untouched). No `vox_kb_facets` reference in vox-cli.

## Mutation proofs (presence-checked before and after each edit)

1. `Backlog::tightened`: forced the additions branch off (`if false && !additions.is_empty()`, 1 occurrence confirmed), `tighten_is_removal_only` went red; restored (0 occurrences), green.
2. `Backlog::tighten_only_violations`: inserted `if head.epoch == base.epoch { return Vec::new(); }`, `gate_rejects_same_epoch_addition` (kb-facets) red; restored, green.
3. Same mutation 2 rerun against the CLI gate: `vox-cli ... facet_backlog::tests::gate_rejects_same_epoch_addition` red; restored (`git diff` of vox-kb-facets empty after restore at the time), green.

## Deviations from Plan

None - plan executed as written. TDD note: tests and implementation were written together in each task and the red state was demonstrated through the mutation proofs rather than a separate failing commit.

## Known Stubs

None. The real backlog file is intentionally not created here (seeding it is plan 04-08); the gate passes while it is absent at both base and head.

## Self-Check: PASSED
