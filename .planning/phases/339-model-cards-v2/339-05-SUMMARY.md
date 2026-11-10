---
phase: 339-model-cards-v2
plan: "05"
subsystem: mens-hub-cards
tags: [mens, hub-cards, askama, card-facts, serving-gate]
requires: ["339-02", "339-03"]
provides:
  - "hub_card::card_facts: CardFacts, SpokeFact, HeldOut, SliceRow, TrainingFacts, card_facts (pure, no IO, no prose)"
  - "templates/hub/model.md, partials/frontmatter.md, partials/status_line.md (v2 model card)"
  - "goldens needs_re_gate.md and stale_serving.md; tests/support.rs Case, case_run_dir, cx_for, render_case"
affects: [339-06, 339-07, 339-08, 339-09]
key-files:
  created:
    - crates/vox-populi/src/mens/hub_card/card_facts.rs
    - crates/vox-populi/src/mens/hub_card/tests/model_card.rs
    - crates/vox-populi/templates/hub/partials/frontmatter.md
    - crates/vox-populi/templates/hub/partials/status_line.md
    - crates/vox-populi/tests/fixtures/hub_cards/cases/1_7b.json
    - crates/vox-populi/tests/golden/hub_cards/needs_re_gate.md
    - crates/vox-populi/tests/golden/hub_cards/stale_serving.md
  modified:
    - crates/vox-populi/src/mens/hub_card/{mod,render,facts,config,legacy_prose}.rs
    - crates/vox-populi/src/mens/hub_card/tests/{mod,support,legacy}.rs
    - crates/vox-populi/templates/hub/model.md
    - mens/config/hub-cards.yaml
key-decisions:
  - "The Training section shows no LoRA rank or alpha (shared settings live on the docs page, which states no value), so the adapter_card.json vs training_manifest.json disagreement does not reach a card. If a later plan shows them, read training_manifest.json (the trainer's record)."
  - "A Stale gate blocker states recorded vs current slice sizes only when they differ; when they are equal (stale for another reason) the status line quotes the refusal instead."
requirements-completed: [CARD-06, CARD-11]
status: complete
commits: 1
plan_head_before: 7e6b4353f011b2716b07b0c51af8a3d16e31900b
plan_head_after: 799b4e5db779a575d848b827e17914b63f642887
actuals:
  tokens: 40000
  tasks: 2
  commits: 1
completed: 2026-11-06
---

# Phase 339 Plan 05: CardFacts and the v2 model card Summary

Every model card now renders from a typed `CardFacts` through askama templates in spec order (Why this size, Use it, Results, Training, Files), states trained, spoke and gate once in one status-line sentence, and shows the adapter's held-out numbers only when its checkpoint records the current serving path.

## Commits

- `799b4e5db` feat(339-05): render model cards from CardFacts and askama templates (Tasks 1 and 2)
- the SUMMARY commit follows `plan_head_after`

`commits: 1` is `git rev-list --count 7e6b4353f..799b4e5db`. The base `7e6b4353f` is 339-04's last commit, which landed in the shared tree while I worked. Tasks 1 and 2 share `model_card.rs`, `support.rs` and the templates (the stale-serving branch and the fact-once tests were written against the same files), so they are one commit instead of two; the tdd-guard hook passed on it without `--no-verify`.

## Verification

Cargo runs were `-p vox-populi --features mens-hf-hub`, one at a time, via the broker.

| Run | Result |
|---|---|
| `--lib -- hub_card hub_sync::tests` | 82 passed, 0 failed, 1 ignored (the ignored one is the pre-existing hub_sync test), 0 warnings |
| `cargo check -p vox-ml-cli --features mens-hf-hub` | Finished; its 3 warnings (`Context` import, `blake3_hex`, `verify`) are in 339-04's files |
| after the mutation restore, same filters | 82 passed, 0 failed, 1 ignored |

Required tests present by name: `card_facts_needs_re_gate_case`, `goldens`, `sections_in_spec_order`, `removed_sections_are_gone`, `gate_state_stated_once`, `trained_and_spoke_stated_once`, `blurbs_state_no_fact`, `stale_serving_card_omits_adapter_score`. Extra tests: `not_trained_status_line_states_only_the_trained_fact`, `frontmatter_is_valid_hub_metadata`, three inline `card_facts` tests (`slices_and_alpha_are_filled_only_for_a_passed_gate`, `untrained_run_has_no_spoke_peak_or_scores`, `adapter_score_needs_current_serving`), and `render::tests::model_card_passes_markup_through_unescaped`. `hub_sync.rs` is untouched (`git diff HEAD` empty), so `stage_bundle_writes_card_and_hashes_and_strips_host_paths` and `sync_classifier_and_index_renderer_handle_all_states` pass unedited.

Acceptance greps: `std::fs`/`read_to_string`/`File::open` in `card_facts.rs` 0 (outside tests it has no IO); `include "partials/status_line.md"` in model.md 1; `state-legend` in model.md 0; every `#slug` in the templates is a 339-02 heading (collateral-damage, held-out-pass-rate, how-the-cards-are-made, hub-and-spoke, shared-training-settings, state-legend, why-mcnemar, why-peft-cannot-load-the-adapters); `/Users/` in both goldens and the fixture 0; `adapter_evidence` in `card_facts.rs` 5; `card_facts.rs` 359 lines; `index.md` golden untouched.

## Mutation proof 3 (fact-once part)

File: `crates/vox-populi/templates/hub/model.md`. Grep `include "partials/status_line.md"`: before 1, mutated 2, after restore 1; `diff` of the restored file against a byte copy is 0 lines and `git diff HEAD` on it is empty.

Mutation: a second identical include line after the status-line paragraph. Mutated run:

```
test mens::hub_card::tests::model_card::gate_state_stated_once ... FAILED
test result: FAILED. 0 passed; 1 failed
```

After restore, `--lib -- hub_card hub_sync::tests` printed `test result: ok. 82 passed; 0 failed; 1 ignored`, with `gate_state_stated_once` among them.

## Golden review (read line by line before committing)

- `needs_re_gate.md` (1.7B record set, current-serving adapter checkpoint): frontmatter has `base_model` without `@`, a 40-char revision, tags `spoke-vox-lang` and `gate-needs-re-gate`, and a model-index with the held-out pass@1 only (no accuracy metric, as the gate is not passed). Status line: trained, spoke vox-lang, gate needs re-gate, "it passed the retired 32-item collateral rule and the 128-item paired re-run for 1.7B has not been done". Results shows base 2/8 and adapter 5/8 and no slice rows. No Examples section (339-07). No `/Users/`.
- `stale_serving.md` (same records, adapter checkpoint without a serving field): no model-index at all, adapter cell is a dash, base cell present, one sentence "The adapter score for 1.7B is not shown because its checkpoint has no serving field ..." linking the serving-path rule. The only differences from `needs_re_gate.md` are those three places (verified with `diff`).
- Both end with exactly one newline (asserted in `goldens`).

## Fixture: `cases/1_7b.json`

Key `needs_re_gate_1_7b`, five records copied from `mens/runs/qwen3-1.7b` (read only), host paths dropped by keeping only these fields:

- `train-record.json`: schema, status, reason, size, pin, preset, epochs, seq_len, optimizer_steps, micro_steps, final_loss_ema, avg_train_loss, peak_mb, peak_source, train_wall_secs, truncated_rows, host (dropped: `adapter` path, requested_model, wall totals, recorded_at)
- `gate_receipt.json`: whole (its `run_dir` is the relative `mens/runs/qwen3-1.7b`)
- `collateral_damage_report.json`: whole, the 32-item-era pass (no rule, alpha, slice pins or grader)
- `training_manifest.json`: base_model, provenance_license_class, rank, alpha, seq_len, epochs, git_sha, seed, finetune_contract_digest, execution_kernel, candle_qlora_training_steps_executed, manifest_schema_version (dropped: tokenizer and train-file paths, launch argv)
- `run_summary.json`: schema, base_model, optimizer_step_count, avg_train_loss, avg_val_loss_last_epoch, total_tokens, final_loss_ema, truncated_rows, wall_secs (dropped: artifact paths)

The held-out evidence is 339-03's 4B subset, run under the 1.7B size and pin; it exercises the card, it is not 1.7B data. `case_run_dir` copies it into `evidence/` with the current or the `legacy_serving` adapter checkpoint.

## Rank and alpha

`adapter_card.json` says rank 16 / alpha 32 for every size; `training_manifest.json` (the trainer's record) says 8/16 for 0.6B, 32/64 for 14B and 16/32 for the rest. The v2 Training section does not show them (the plan's section lists steps, tokens, losses, wall time, peak and commit; shared settings are on the docs page, which states no value), so no card reads either file for rank or alpha. The fixture keeps `training_manifest.json` rank/alpha (16/32 for 1.7B). I did not check tensor shapes against the manifests: nothing consumes the value yet.

## Legacy tests deleted or trimmed (tests/legacy.rs)

- Deleted `hub_card_shows_no_score_without_its_record` (rendered phase-336 cards and asserted their wording).
- Trimmed `hub_card_status_quotes_refusal_and_never_claims_a_pass`: the `card_status` assertions stay (the index still uses `card_status`); the half asserting phase-336 card text (`**Gate: refused.**`, "still lists it as missing", serve example) is gone.
- `golden_cases()` now holds only the index; removed the `card()` helper and `invalid_protocol()` builder and the now-unused imports.
- Goldens `refused.md`, `invalid_protocol.md`, `passed.md`, `not_evaluated.md`, `not_trained.md` stay on disk unused for 339-06. `hub_card_frontmatter_parses_with_required_keys` and `hub_card_render_is_deterministic_and_host_free` still pass (they now iterate the index only).

## Deviations from Plan

**1. [Rule 3] One code commit for two tasks** (see Commits).

**2. [Rule 1] Re-added `pub(crate) use facts::{GateState, gate_of}` in `hub_card/mod.rs`.** I removed the 339-01 re-exports along with their `unused_imports` allow, and 339-04's `hub_cards_push.rs` (same working tree) imports exactly those two names, so the test build broke. `AdapterEvidence`, `adapter_evidence`, `Blocker` and `GateFact` stay unexported (nothing outside `hub_card` uses them).

**3. [Rule 3] `facts::load_requirement` is now `pub(super)`**, so `card_facts` tests (and 339-06's `passing_collateral_report`) can build a current-rule report.

**4. [Rule 1] 32B blurb reworded in `mens/config/hub-cards.yaml`:** "not trained yet" and "Until it is trained" tripped `blurbs_state_no_fact`; now "not available yet" and "Until it ships". Config tests still pass.

**5. askama quirks found while rendering:** askama 0.12 strips one trailing newline from every template, so `model.md` ends with a blank line and has an explicit newline after the frontmatter include; the partials end without a newline. `{% match %}` struct patterns (`Blocker::Stale { .. }`) and relative `{% include %}` work as the plan assumed.

**6. Dead-code allows.** Removed the 339-05 allows on `collateral_standin`, `config`, `adapter_evidence`/`AdapterEvidence`, `GateState`, `Blocker`, `GateFact`, `gate_of`, `recorded_slice_n`. Remaining with new reasons: `mod examples` (consumed by 339-07), `legacy_prose::{frontmatter_string, body_string}` (kept live until 339-06 deletes the phase-336 model prose), `HubCardsConfig::default_spoke_source` (339-08, the index).

**7. Added beyond the plan:** `HeldOut` takes its total from the base report (`ponytail:` comment; both sides run the same bench). `CardFacts` grew `protocol`, `adapter_report`, `has_receipt`, `why_is_error` and a few format helpers (`base_cell`, `adapter_cell`, `peak_text`, `is_passed`, `has_model_index`), all number formatting, no prose.

**8. Results table shape.** Column headers link the three anchors (held-out pass@1, collateral accuracy, McNemar p-value) and rows are Base, Adapter and one per slice (passed only), each labelled with the size.

STATE.md, ROADMAP.md and REQUIREMENTS.md were not updated by this executor (339-04 shares the tree and the same planning files). The orchestrator should record CARD-06 and CARD-11 (first two goldens of the matrix) as done.

## Known Stubs

None. The three `dead_code` allows above are real code awaiting consumers.

## Threat Flags

None. T-339-34 (stale adapter score) is covered by `adapter_score_needs_current_serving`, `stale_serving_card_omits_adapter_score` and 339-03's proof 13; T-339-14 by the fact-once tests and proof 3; T-339-15 by the greps above. Record text (refusal, blocker reasons, gate messages) still reaches the public README unescaped, as in phase 336.

## Self-Check: PASSED

- Created files exist: card_facts.rs, model_card.rs, the two partials, `cases/1_7b.json`, both goldens.
- Commit `799b4e5db` is an ancestor of HEAD.
