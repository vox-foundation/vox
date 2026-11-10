---
phase: 339-model-cards-v2
plan: "08"
subsystem: mens-hub-cards
tags: [mens, hub-cards, index, askama, goldens, serving-path, estimates]
requires: ["339-07"]
provides:
  - "hub_card::index_facts (IndexFacts, IndexRow, HubCell, GridCell, Pick, Pending, index_facts, pick_size): per-size state from gate_of and adapter_evidence, no second state computation"
  - "templates/hub/index.md and IndexTemplate: render_index_readme keeps its signature and renders through askama"
  - "v2 index in spec section 5 order: paragraph, size x spoke grid, Which model should I use, What's next, held-out comparison, Links"
  - "index golden over a full six-size ladder fixture; support::ladder_entries, ladder_entries_no_passed, ladder_entries_passed_but_stale, index_row"
  - "legacy_prose.rs, tests/legacy.rs, CardStatus, card_status and recommended_size deleted"
affects: [339-09, 339-10]
key-files:
  created:
    - crates/vox-populi/src/mens/hub_card/index_facts.rs
    - crates/vox-populi/templates/hub/index.md
    - crates/vox-populi/src/mens/hub_card/tests/index_card.rs
    - crates/vox-populi/src/mens/hub_card/tests/records.rs
  modified:
    - crates/vox-populi/src/mens/hub_card/{render,mod,card_facts,config}.rs
    - crates/vox-populi/src/mens/hub_card/tests/{mod,support}.rs
    - crates/vox-populi/tests/golden/hub_cards/index.md
    - crates/vox-populi/src/mens/hub_sync.rs
    - mens/config/hub-cards.yaml
  deleted:
    - crates/vox-populi/src/mens/hub_card/legacy_prose.rs
    - crates/vox-populi/src/mens/hub_card/tests/legacy.rs
key-decisions:
  - "pick_size and the comparison's adapter column use only AdapterEvidence::Current; a size whose gate is passed but whose evidence is stale is a re-evaluation item, never a pick (B1, T-339-37)"
  - "What's next gives a re-gate estimate only for Stale and Missing blockers; a Failed size and a stale-evidence-only size read 'There is no estimate for <size>.'"
  - "The grid shows the gate label as plain text and links only the planned cells; the state legend is linked once under the grid"
  - "default_spoke_source from hub-cards.yaml is printed under the grid, because the vox-lang column is declared, not derived from the run"
requirements-completed: [CARD-09, CARD-11]
status: complete
commits: 3
plan_head_before: 63b659855247689adbcf3436e7a7fe887830ac02
plan_head_after: 0101364d85830486f53f6a7cfc771f2245f1afd8
actuals:
  tokens: 27000
  tasks: 3
  commits: 3
completed: 2026-11-06
---

# Phase 339 Plan 08: v2 index card and removal of the phase-336 prose Summary

The `vox-mens-index` README now renders from typed `IndexFacts` through one askama template: a size x spoke grid, a Which-model rule that names no pick unless a size has both a passed gate and a current-serving score, a What's-next list built from records and labelled yaml estimates, and one held-out comparison, with the phase-336 index prose and `CardStatus` gone.

## Commits

- `dae447646` feat(339-08): render the index through IndexFacts and askama with the size x spoke grid (Task 1, tracer)
- `e70bb59e3` feat(339-08): add Which model, What's next and the held-out comparison to the index (Task 2)
- `0101364d8` refactor(339-08): delete the phase-336 gate label and its tests; keep the record-load test (Task 3)

`commits: 3` is `git rev-list --count 63b659855..HEAD`, read at SUMMARY time from the on-disk ledger (`.git/gsd-plan-head-before-339-08`). Each code commit passed tdd-guard without `--no-verify`; the first Task 1 attempt was refused for six untested pub fns in `index_facts.rs`, so two inline tests were added and the commit was redone. The tracer gate (Task 1 verify re-run end to end before expanding) passed: 107 tests green with `hub_sync::tests`.

## Verification

All cargo runs were `-p vox-populi --features mens-hf-hub`, one at a time through the broker, `nice -n 19`.

| Command | Result |
|---|---|
| `cargo test --lib -- hub_card` (final tree, after mutation restore) | 105 passed, 0 failed |
| `cargo test --lib -- tests::index_card` | 17 passed, 0 failed |
| `cargo test --lib -- hub_card::index_facts` | 2 passed |
| `cargo test --lib -- hub_sync::tests` | 10 passed, 1 ignored (pre-existing live Hub test) |
| `cargo clippy -p vox-populi --features mens-hf-hub --lib -- -D warnings` | clean, exit 0 |
| `cargo check -p vox-ml-cli --features mens-hf-hub` | exit 0; 3 warnings in `dispatch.rs` and `eval_gate/policy.rs`, none from this plan |

Index tests, by name (all in `hub_card::tests::index_card`): `index_golden_matches`, `index_frontmatter_parses_with_required_keys`, `index_grid_has_one_row_per_size_and_one_column_per_spoke`, `index_hub_and_repo_cells_keep_their_wording`, `hub_cell_covers_every_sync_state`, `index_rows_follow_ladder_order_whatever_the_input_order`, `index_without_passed_sizes_still_renders_every_row`, `index_sections_in_spec_order`, `pick_is_highest_passed_adapter_rate_ties_to_smaller`, `pick_ignores_sizes_whose_gate_is_not_passed_however_high_they_score`, `pick_requires_current_serving`, `index_names_no_pick_while_regates_pending`, `whats_next_from_records`, `whats_next_names_a_stale_passed_size_with_only_the_re_evaluation`, `whats_next_is_empty_of_sizes_when_nothing_is_pending`, `comparison_uses_current_protocol_only`, `comparison_with_no_current_report_says_so`. Inline in `index_facts.rs`: `row_and_pick_cells_read_the_facts`, `pending_filters_split_by_reason`. `hub_card_load_reads_records_and_rejects_retired_protocol` now lives in `tests/records.rs` with its assertions unchanged.

Acceptance greps:

- `grep -c 'path = "hub/index.md"' render.rs` 1.
- `grep -cE '"(0\.6B|1\.7B|4B|8B|14B|32B)"|vox-lang|tool-selection' index_facts.rs` 0 (sizes come from `SIZES`, spokes from the yaml, tests index `SIZES`).
- `index_facts.rs` is 310 lines (under 400).
- `grep -c '## Which model should I use'` and `grep -c "## What's next"` in `index.md`: 1 each.
- `grep -c estimate` in the golden 7; `grep -cE '[0-9]+(\.[0-9]+)? ?h\b'` in the template 0.
- `test ! -e legacy_prose.rs && test ! -e tests/legacy.rs` succeeded; `grep -rc 'CardStatus\|recommended_size' crates/vox-populi/src/mens/` printed 0 for every file; `grep -rn 'CardStatus\|card_status\|recommended_size' crates/vox-populi crates/vox-ml-cli --include='*.rs'` (excluding the untracked `crates/vox-ml-cli/mens/`) printed nothing; `mod.rs` still has `pub use` lines.
- Task 3's commit leaves `hub_sync.rs` alone: `git diff --quiet "$C^..$C" -- crates/vox-populi/src/mens/hub_sync.rs` succeeded for `C=0101364d8`.

## hub_sync.rs: one assertion changed

`git log --oneline 52b8e9fe6..HEAD -- crates/vox-populi/src/mens/hub_sync.rs` is not empty: it prints `f5130720a fix(mens): clear three clippy lints in hub_sync and legacy_prose` (5 added, 8 deleted lines, outside the test). The plan said to read the numstat gate against that commit in this case, so the gate is measured against `f5130720a`, not the 339-01 phase base `52b8e9fe6`:

```
git diff --numstat f5130720a HEAD -- crates/vox-populi/src/mens/hub_sync.rs
1	1	crates/vox-populi/src/mens/hub_sync.rs
git diff -U0 f5130720a HEAD -- ... | grep '^-[^-]'
-        assert!(md.contains("| 14B | not trained | no weights |"));
```

The replacement asserts the full new 14B grid row (`| 14B | not trained | [planned](…#hub-and-spoke) | [planned](…) | [planned](…) | no weights | [ns/vox-mens-qwen3-14b]`); `rustfmt --check` accepts it on one line. The other three index assertions (4B repo link, 14B repo link, ``published `v2` ``) are untouched and pass, so the template keeps that wording: the Repo cell is `[<repo_id>](https://huggingface.co/<repo_id>)` and a published Hub cell reads ``published `<tag>` ``.

## Mutation proofs

Each mutant was applied to `crates/vox-populi/src/mens/hub_card/index_facts.rs` after copying it to the scratchpad. After each run the file was restored by `cp` and `git diff --quiet HEAD -- <file>` succeeded.

**Proof 12 (plan), T-339-19.** Grep `GateState::Passed`: before 4, mutated 3, after restore 4. Mutation: in `pick_size`, `if r.gate != GateState::Passed || r.adapter_evidence != AdapterEvidence::Current {` became `if r.adapter_evidence != AdapterEvidence::Current {`. Run of `--lib -- index_names_no_pick_while_regates_pending`:

```
test mens::hub_card::tests::index_card::index_names_no_pick_while_regates_pending ... FAILED
panicked at crates/vox-populi/src/mens/hub_card/tests/index_card.rs:307:5:
no pull command without a current gate
test result: FAILED. 0 passed; 1 failed
```

After restore the full `hub_card` run printed `105 passed`.

**Extra proof (serving-path gate, T-339-37).** Grep `AdapterEvidence::Current`: before 3, mutated 2, after restore 3. Mutation: the same line became `if r.gate != GateState::Passed {`. Run of `--lib -- pick_requires_current_serving`:

```
test mens::hub_card::tests::index_card::pick_requires_current_serving ... FAILED
assertion `left == right` failed
  left: Some(Pick { size: "1.7B", rate: 0.9, peak_mb: None })
test result: FAILED. 0 passed; 1 failed
```

## Golden review

`index.md` was read line by line after each regeneration (`VOX_UPDATE_GOLDEN=1` on `index_golden_matches` only; no other golden was regenerated or changed). It is built from the committed 339-03 evidence fixture rendered under six sizes, with these roles: 0.6B failed (current evidence, Hub `v5` diverged), 1.7B needs re-gate with `legacy_serving/` evidence, 4B passed with current-serving evidence (Hub `v2`), 8B needs re-gate with current evidence, 14B not evaluated, 32B not trained.

- Grid: seven columns (Size, `vox-lang`, `harness: tool-selection`, `harness: argument-generation`, `rust`, Hub, Repo), six rows in ladder order; the three planned cells on every row read `[planned](…#hub-and-spoke)`; gate cells read `failed`, `needs re-gate`, `passed`, `needs re-gate`, `not evaluated`, `not trained`.
- Which model: the rule once, then 4B as the pick (`62.5%`) with a one-row table and `vox mens hub pull --size 4B --namespace ns`.
- What's next: 0.6B (review the stopping gate, `There is no estimate for 0.6B.`), 1.7B (re-run `vox mens eval-collateral-damage` on the 128-item slices plus re-evaluate with `vox mens eval-local`, estimate 1.6 / 2.6 hours with its yaml basis), 8B (re-gate only, 4.3 / 6.2 hours), 14B (missing `collateral_damage_report.json`, `gate_receipt.json`, 8.0 / 11.0 hours), then the three planned spokes with their purpose, link and `no estimate`. No gate label appears in the section.
- Comparison: rows for 0.6B, 1.7B, 4B, 8B, 14B; 1.7B's adapter cell is a dash; 32B (no reports) is absent; one sentence says only current-protocol reports are listed and an earlier-serving-path adapter score is a dash.
- No `/Users/` or `/home/` in the golden (asserted in `index_golden_matches`).

Every number in the golden comes from the shared fixture (real stored 4B held-out reports and the 1.7B case records), so sizes other than 4B and 1.7B show the same 2/8 and 5/8 scores, and the 4B pick shows a `27007 MiB` training peak that belongs to the 1.7B case record. They are fixture values, not those sizes' real results; do not present the golden as real data.

## User choices (defaults implemented; not blocking)

1. **Spoke provenance under the grid.** `default_spoke_source` (the `hub-cards.yaml` text saying the vox-lang value is declared, not derived, until phase 338 plan 06) is printed after the grid legend sentence. Its `#[allow(dead_code)]` waiting for 339-08 is removed. Alternative: omit it and keep the claim only on the docs page.
2. **Grid badges.** Gate labels are plain text (the trained-spoke cell is exactly the label) and only the `planned` cells link; the state legend is linked once under the grid. Alternative: link every badge.
3. **Estimate scope.** An estimate is shown only for items whose blocker a re-run clears (`Stale`, `Missing`). A size that failed a current-rule evaluation, or whose only work is the held-out re-evaluation, reads `There is no estimate for <size>.` because `hub-cards.yaml` holds only a re-gate estimate. The estimate sentence says it covers the re-gate.
4. **Passed size with stale evidence.** It is not a pick and its Which-model table is not shown; it appears in a sentence listing sizes that need a re-evaluation under the current serving path, and as a What's-next item with only that action.
5. **Planned-spoke items** carry the yaml purpose, a link to the docs-page plan and `no estimate`, with no hours.
6. **Wording of actions** per `Blocker` variant lives in `index.md` (`eval-collateral-damage` for a stale rule, the missing record names, a review sentence for a failed gate). Alternative wording is a template-only edit.
7. **Whether the 338-02 per-spoke gate table belongs on the index** (orchestrator decision 8) is left for 339-10: the grid shows one badge per size and spoke.

## Deviations from Plan

**1. [Rule 3 - blocking] `card_facts.rs` and `config.rs` edited.** Neither is in the plan's file list. The index reuses `held_out` and the doc URL from `card_facts.rs`, so `held_out` and `DOCS` became `pub(super)`, and `HeldOut` gained `base_cell` / `adapter_cell` (the two `CardFacts` methods now delegate to them; behaviour and card goldens unchanged). `config.rs` lost the `#[allow(dead_code)]` on `default_spoke_source`, now read by the index.

**2. [Rule 1 - bug found while planning 339-09] `hub-cards.yaml` basis text edited.** The 4B, 8B and 14B `regate_eta.basis` strings ended with the same sentence, `Per-item rates are projections, not measurements.` The index prints each basis, so 339-09's duplicate-sentence lint would have failed on the ladder set (8B and 14B are both pending there). Each now reads `Per-item rates for <size> are projections, not measurements.` No other yaml text changed, and `config` validation and tests still pass. The file is outside this plan's list; 339-09 should not need to touch it.

**3. [Rule 3] Task 1 trimmed `legacy_prose.rs` and `tests/legacy.rs`, then Task 3 deleted them.** The old and new `render_index_readme` could not coexist, so Task 1 removed the old index renderer, `hub_cell` and the index tests (moved to `index_card.rs`) while `card_status`, `CardStatus` and `recommended_size` stayed until Task 3, as the plan sequences it.

**4. [Rule 3] Task 3 moved one test only.** Of the tests left in `legacy.rs` after the index tests moved, only `hub_card_load_reads_records_and_rejects_retired_protocol` covers live code (`RunRecords::load`, the protocol check, the compare lookup); it went to `tests/records.rs` with its `heldout` helper, unchanged. The status-label test went with `CardStatus`. The plan's mention of other surviving `compare_json_path` tests did not match the file: the lookup is exercised by that load test and by `facts.rs`'s inline `empty_run_dir_loads_as_all_absent`.

**5. [Rule 3] hub_sync numstat gate read against `f5130720a`** (see above), as the plan prescribes for an intervening commit.

**6. Tests added beyond the plan's names:** grid-cell, Hub-cell, ladder-order, section-order, stale-passed What's-next, empty What's-next and empty-comparison tests, plus `pick_ignores_sizes_whose_gate_is_not_passed_however_high_they_score`. They cover branches of `index_facts` that the named tests do not.

## Known Stubs

None. Every empty case renders a sentence (no pending work, nothing to compare, no recommendation); no placeholder text reaches the index.

## Threat Flags

None new. T-339-19 and T-339-37 are covered by `index_names_no_pick_while_regates_pending`, `pick_requires_current_serving` and the two mutations; T-339-20 by the `This is an estimate, not a measurement` wording with conservative figure and basis from yaml, `no estimate` otherwise, and the hours-free template grep; T-339-21 by `gate_of` being the only state computation and What's next naming actions only (asserted in `whats_next_from_records`).

## Notes for 339-09 and 339-10

- `support::ladder_entries(tmp)` is the ladder 339-09's lints should render (one entry per `SIZES`, same six roles as above); `ladder_entries_no_passed` and `ladder_entries_passed_but_stale` are the variants. The golden index is the one the duplicate-sentence lint sees for it; sentences repeated per item all name their size or spoke label.
- The index footer is the last line (`This index is generated by …`), the only sentence meant to repeat across cards.
- With real records today no size has current-serving evidence on a passed gate, so the real index will name no pick: 339-10 should expect `No size has a current gate, so this index recommends none and gives no pull command.` unless 0.6B's re-gate produces a pass and a current-serving re-evaluation.
- The index states the vox-lang column is declared (text from `default_spoke_source`); phase 338 plan 06 changes that sentence at its source.

## Self-Check: PASSED

- Created files exist: `index_facts.rs`, `templates/hub/index.md`, `tests/index_card.rs`, `tests/records.rs`; `legacy_prose.rs` and `tests/legacy.rs` are absent.
- Commits `dae447646`, `e70bb59e3` and `0101364d8` are ancestors of HEAD.
