---
phase: 339-model-cards-v2
plan: "06"
subsystem: mens-hub-cards
tags: [mens, hub-cards, goldens, model-card-states, legacy-retirement]
requires: ["339-05"]
provides:
  - "goldens passed.md, failed.md, not_evaluated.md, not_trained.md next to 339-05's needs_re_gate.md and stale_serving.md"
  - "tests/support.rs: passing_collateral_report(req), Case consts PASSED, FAILED, NOT_EVALUATED, NOT_TRAINED"
  - "cases/states.json: sanitized run records for the 0.6B (failed), 14B (not evaluated) and 32B (not trained) shapes"
affects: [339-07, 339-08, 339-09]
key-files:
  created:
    - crates/vox-populi/tests/fixtures/hub_cards/cases/states.json
    - crates/vox-populi/tests/golden/hub_cards/failed.md
  modified:
    - crates/vox-populi/src/mens/hub_card/tests/{support,model_card,legacy}.rs
    - crates/vox-populi/src/mens/hub_card/legacy_prose.rs
    - crates/vox-populi/templates/hub/model.md
    - crates/vox-populi/tests/golden/hub_cards/{passed,not_evaluated,not_trained}.md
  deleted:
    - crates/vox-populi/tests/golden/hub_cards/refused.md
    - crates/vox-populi/tests/golden/hub_cards/invalid_protocol.md
key-decisions:
  - "The 'no current-protocol report' sentence is keyed on `f.heldout.is_none()`, so card_facts.rs needed no change and ends the plan byte-identical to 339-05. It is true whether the held-out reports are absent or under another protocol; it does not name the offending files."
  - "The sliced-stale half of no_stale_numbers_without_current_rule is built from the passing report with `rule` set to `retired` and distinctive numbers, because the real 1.7B retired-rule fixture has no `p_value` or per-slice `n`, so `slice_rows` would produce no rows from it even under the mutation."
requirements-completed: [CARD-06, CARD-11]
status: complete
commits: 3
plan_head_before: f5130720ad678b355320b4cd95036378a157566c
plan_head_after: 67b0d01a8093712adffff2e07f92c2519230be9c
actuals:
  tokens: 6700
  tasks: 3
  commits: 3
completed: 2026-11-06
---

# Phase 339 Plan 06: Remaining model-card states and legacy prose retirement Summary

All six model-card states now have a reviewed golden (passed, needs re-gate, stale serving, failed, not evaluated, not trained), collateral numbers appear only on a `passed` gate, and the phase-336 model-card prose and its two goldens are gone from `legacy_prose.rs`, which now holds only the gate label and the ladder index.

## Commits

- `6258a88ae` feat(339-06): render the passed model card with its collateral slice rows (Task 1, tracer)
- `f734809f7` feat(339-06): add failed, not-evaluated and not-trained model-card cases (Task 2)
- `67b0d01a8` refactor(339-06): retire the phase-336 model-card prose and its goldens (Task 3)

`commits: 3` is `git rev-list --count f5130720a..67b0d01a8`, read from the on-disk ledger. Each task passed tdd-guard without `--no-verify`.

## Verification

Cargo runs were `-p vox-populi --features mens-hf-hub`, one at a time through the broker, `nice -n 19`.

| Command | Result |
|---|---|
| `cargo test --lib hub_card::tests::model_card` (Task 1, after Task 1 changes) | 12 passed |
| same (Task 2) | 14 passed |
| `cargo test --lib hub_card` (Task 2, no golden update) | 76 passed, 0 failed |
| `cargo test --lib hub_card` (Task 3) | 77 passed, 0 failed, 0 warnings |
| `cargo test --lib hub_sync::tests` | 10 passed, 1 ignored (the ignored one is pre-existing) |
| `cargo check -p vox-ml-cli --features mens-hf-hub` | Finished; 3 warnings, all pre-existing in 339-04's files (`Context` import, `blake3_hex`, `verify`) |
| `cargo clippy -p vox-populi --features mens-hf-hub --lib -- -D warnings` | exit 0 |

Required tests present by name: `passed_card_shows_slice_rows`, `no_stale_numbers_without_current_rule`, `failed_card_quotes_the_refusal`, `not_evaluated_card_names_the_missing_records`, `not_trained_card_states_only_the_trained_fact` (the 339-05 test `not_trained_status_line_states_only_the_trained_fact` was renamed and rewritten onto the fixture case), `invalid_protocol_report_shows_no_heldout_numbers`. 339-05's `gate_state_stated_once`, `trained_and_spoke_stated_once`, `sections_in_spec_order` and `frontmatter_is_valid_hub_metadata` now run over a `trained_cases()` list (every golden but `not_trained`, which has no gate, spoke or sections); `removed_sections_are_gone`, `stale_serving_card_omits_adapter_score` and the goldens test run over all six.

Acceptance greps: `fn passing_collateral_report(req: &CollateralRequirement)` in support.rs 1; `gsm8k_think|math500_think|vox_think` in `needs_re_gate.md` 0 and in `passed.md` 12; `/Users/` and `/home/` in `states.json` and the four new/changed goldens 0; six goldens listed; `refused.md` and `invalid_protocol.md` absent; the five removed `fn` names in legacy_prose.rs 0; `index.md`, `hub_sync.rs` and `card_facts.rs` unchanged against HEAD.

## Mutation proof 11

File: `crates/vox-populi/src/mens/hub_card/card_facts.rs`. Grep `GateState::Passed`: before 3, mutated 2, after restore 3.

Mutation: `(Some(c), GateState::Passed) => slice_rows(c),` became `(Some(c), _) => slice_rows(c),` (a retired-rule report now fills the Results rows). Mutated run of `--lib no_stale_numbers_without_current_rule`:

```
panicked at crates/vox-populi/src/mens/hub_card/tests/model_card.rs:467:13:
stale `gsm8k_think` on a card
test result: FAILED. 0 passed; 1 failed
```

Restored from a byte copy (`cp`); `git diff --quiet HEAD -- card_facts.rs` succeeded, and `--lib hub_card::tests::model_card` then printed `12 passed; 0 failed`, with `no_stale_numbers_without_current_rule` among them.

## Golden review (read line by line before committing)

All four numbers in the held-out row (2/8 and 5/8, 25.0% and 62.5%, `value: 0.6250`, 8 items) come from the shared 339-03 evidence fixture, which every case reuses, so they are identical on every card and are not the real scores of the 0.6B, 1.7B or 14B runs. Slice numbers on `passed.md` come from `passing_collateral_report` (50% on both sides, p 1.0000 on 128 items), also synthetic. Say so if a golden is ever shown as real data.

- `passed.md` (1.7B records, current-pass collateral): frontmatter has `base_model` without `@`, a 40-char revision, tags `spoke-vox-lang` and `gate-passed`, a model-index with the held-out pass@1 and one accuracy metric per slice (`gsm8k_think`, `math500_think`, `vox_think`, each a 128-item slice). Status line: trained, spoke vox-lang, gate passed, once each. Use it has no `--allow-ungated` and no refusal paragraph. Results has base and adapter rows plus three slice rows, each row label naming 1.7B, and the interpreting sentence with alpha 0.05. Gate receipt link, Training and Files present. No `/Users/`.
- `failed.md` (0.6B): tags `spoke-vox-lang` and `gate-failed`, model-index with held-out pass@1 only. Status line quotes the refusal once: the collateral check refused 0.6B: “the collateral report status is `fail` on `gsm8k_think`”. Use it carries `--allow-ungated` and the "serve refuses" paragraph. Results has no slice rows and no model-index accuracy. Training numbers are the 0.6B record's (1108 steps, 2h 49m, 15319 MiB, commit `c56e13a33`).
- `not_evaluated.md` (14B): tags `spoke-vox-lang` and `gate-not-evaluated`. Status line: the records that decide 14B are missing: `collateral_damage_report.json`, `gate_receipt.json`. No gate-receipt link (no receipt). Training numbers are the 14B record's (138 steps, 17h 58m, 63076 MiB, commit `448df0192`).
- `not_trained.md` (32B): tag `not-trained` only; no `spoke-` or `gate-` tag, no model-index. Status line: "32B is not trained: `train-record.json` says “above non-fitting 4B (D-06)”." Sections: Why this size and the footer; no Use it, Results, Training or Files. No gate label and no spoke id anywhere.
- `needs_re_gate.md` and `stale_serving.md` came out byte-identical after the template change (git shows no diff).

## Fixture: `cases/states.json`

Sources are read-only copies from `mens/runs/` with host paths dropped by keeping named fields only (same approach as `1_7b.json`):

- `failed_0_6b` (from `qwen3-0.6b`): `train-record.json` (schema, status, reason, size, pin, preset, epochs, seq_len, optimizer_steps, micro_steps, final_loss_ema, avg_train_loss, peak_mb, peak_source, train_wall_secs, truncated_rows, host); `run_summary.json` (schema, base_model, optimizer_step_count, avg_train_loss, avg_val_loss_last_epoch, total_tokens, final_loss_ema, truncated_rows, wall_secs); `training_manifest.json` (base_model, provenance_license_class, rank, alpha, seq_len, epochs, git_sha, seed, finetune_contract_digest, execution_kernel, candle_qlora_training_steps_executed, manifest_schema_version); `gate_receipt.json` whole (its `run_dir` and message paths are repo-relative); `collateral_damage_report.json` reduced to status, failed_on, threshold, reports.
- `not_evaluated_14b` (from `qwen3-14b`): the same three trimmed records. The 14B run does have a `gate_receipt.json` (anti_stub, pass_at_k and collateral gates "missing"); it is left out so the case gives `NotEvaluated` with the plan's `Missing` blocker naming the two files.
- `not_trained_32b` (from `qwen3-32b`): `train-record.json` reduced to schema, status, reason, size, pin, preset, epochs, seq_len.

The 0.6B receipt predates its collateral report, so its collateral gate message still says the report is "missing". `gate_of` ignores that gate and judges collateral from the report, so no card quotes it.

## Legacy removed (Task 3)

`legacy_prose.rs` shrank from 986 to 315 lines: `s`, `field`, `or_nr`, `cell`, `hours`, `role`, `frontmatter`, `CollateralRow`, `collateral_rows`, `opt_f`, `results_section`, `category_table`, `training_section`, `usage_section`, `links_section`, `frontmatter_string`, `body_string` (with their two `dead_code` allows) and the `SIZES`, `CardContext` and `repo_name` imports. Kept: `CardStatus`, `card_status`, `collateral_failure`, `pct`, `pct2`, `num`, `hub_cell`, `recommended_size`, `render_index_readme` and the doc consts the index uses; the module header was rewritten. `mod.rs` needed no edit (every `pub use` is still used). Deleted goldens: `refused.md`, `invalid_protocol.md`.

Legacy tests (`tests/legacy.rs`): no test was deleted outright (339-05 already deleted `hub_card_shows_no_score_without_its_record` and trimmed the status test). The one trim: `hub_card_frontmatter_parses_with_required_keys` lost its dead branch (everything after `if name == "index" { continue; }`, which asserted `base_model`, `model-index` and the `invalid_protocol` pass@1 rule for model-card names that `golden_cases()` no longer yields). `hub_card_goldens_per_state` (index), `hub_card_status_quotes_refusal_and_never_claims_a_pass`, `hub_card_render_is_deterministic_and_host_free`, `hub_card_index_lists_every_size_and_picks_by_rule` and `hub_card_load_reads_records_and_rejects_retired_protocol` still pass unedited.

## Deviations from Plan

**1. [Rule 3] `model.md` edited in Task 3.** The plan's Task 3 file list omits `templates/hub/model.md`, but `invalid_protocol_report_shows_no_heldout_numbers` needs the sentence, so the template gained one conditional sentence in Results. Tasks 1 and 2 needed no template change: 339-05 had already built the Passed-only slice rows, model-index metrics, interpreting sentence, every `Blocker` arm and the not-trained branches.

**2. [Rule 1] No `card_facts.rs` field for invalid evidence.** The sentence reads `f.heldout.is_none()`, not `rec.invalid_evidence`, which keeps `card_facts.rs` byte-identical to 339-05 as the plan requires. Consequence: the card does not name which files were under another protocol.

**3. `Case` gained a `passing: bool` field** (default false) so `case_run_dir` can swap the collateral report for `passing_collateral_report`. `support::passing_collateral_report` is a thin wrapper over the 339-01 builder in `collateral_standin::test_support` (shared, not copied), so the plan's grep signature holds and 338's retarget name resolves.

**4. Mutation proof 11 needed a different stale fixture** (see key-decisions): the 1.7B retired-rule fixture alone would have survived the mutation. The test checks both that fixture and the sliced variant.

**5. `trained_cases()`** was added to model_card.rs so the 339-05 tests that assume a gate and spoke skip the not-trained card; `frontmatter_is_valid_hub_metadata` now compares the gate tag with the case's own gate state instead of the literal `gate-needs-re-gate`.

## Known Stubs

None.

## Threat Flags

None. T-339-13 is covered by `no_stale_numbers_without_current_rule` and mutation proof 11; T-339-38 by the sanitized fixture and the `/Users/`/`/home/` greps (0 on the fixture and every new golden, plus the goldens test's own assertion); T-339-33 by the deleted prose and goldens. Record text (refusal, missing record names, the train-record reason) still reaches the public README unescaped, as in 339-05.

## Self-Check: PASSED

- Created files exist: `states.json`, `failed.md`, `339-06-SUMMARY.md`.
- Commits `6258a88ae`, `f734809f7` and `67b0d01a8` are ancestors of HEAD.
