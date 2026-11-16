---
phase: 339-model-cards-v2
plan: "03"
subsystem: mens-hub-cards
tags: [mens, hub-cards, examples, config, serving-gate]
requires: ["339-01", "339-02"]
provides:
  - "hub_card::facts: checkpoint texts on RunRecords, CURRENT_ADAPTER_SERVING, AdapterEvidence, adapter_evidence"
  - "hub_card::examples: Verdict, Side, Example, ExampleFacts, pick_examples, shown_code"
  - "hub_card::config + mens/config/hub-cards.yaml (validated, embedded)"
  - "hub_card::tests::support (fixture_root, vox_block_ok, evidence_run)"
affects: [339-04, 339-05, 339-06, 339-07, 339-08, 339-09]
key-files:
  created:
    - mens/config/hub-cards.yaml
    - crates/vox-populi/src/mens/hub_card/config.rs
    - crates/vox-populi/src/mens/hub_card/examples.rs
    - crates/vox-populi/src/mens/hub_card/examples_tests.rs
    - crates/vox-populi/src/mens/hub_card/tests/support.rs
    - crates/vox-populi/tests/fixtures/hub_cards/evidence/
  modified:
    - crates/vox-populi/src/mens/hub_card/facts.rs
    - crates/vox-populi/src/mens/hub_card/facts_tests.rs
    - crates/vox-populi/src/mens/hub_card/mod.rs
    - crates/vox-populi/src/mens/hub_card/tests/mod.rs
key-decisions:
  - "Verdicts follow the scorer order in metrics.rs verify_completion: pass, non_empty, parse_ok, typecheck_ok, anti_stub_pass, then the first false of pass_compile / pass_ast / pass_exec"
  - "A header-only checkpoint (no item parses) reads as absent, so adapter_evidence reports NoCheckpoint for it"
  - "No regate_eta for 0.6B: its re-gate is running now, so a projection would be stale on arrival"
requirements-completed: [CARD-03, CARD-05]
status: complete
commits: 2
plan_head_before: 001c33632ebec0de3871b359510a0da80304ff59
plan_head_after: 451ec8289a0d5f60237ab0693cef4f35c728c1a4
actuals:
  tokens: 24800
  tasks: 3
  commits: 2
completed: 2026-11-06
---

# Phase 339 Plan 03: Examples, serving gate and hub-cards.yaml Summary

The two data inputs every card needs besides the gate fact now exist: a validated `mens/config/hub-cards.yaml` loader, and a pure D2 example picker over stored held-out reports with a serving-path gate (`adapter_evidence`) that only counts adapter evidence recorded under `nf4_base_plus_lora_arm v2`.

## Commits

- `6742305da` feat(339-03): load eval checkpoints, gate adapter evidence by serving path, pick D2 examples (Tasks 1 and 2)
- `451ec8289` feat(339-03): add hub-cards.yaml and its validated loader (Task 3)

`commits: 2` is `git rev-list --count 001c33632..451ec8289`. Tasks 1 and 2 share files (`examples.rs`, `examples_tests.rs`), so they are one commit. `mod.rs` was committed without `mod config;` in the first commit and with it in the second, so each commit builds.

## Precondition

`git log -S ADAPTER_SERVING -- crates/vox-ml-cli/src/commands/mens/item_checkpoint.rs` prints `8589be99c` (and earlier). The committed const is `"nf4_base_plus_lora_arm v2"`, written at `inputs.model.adapter_serving` for adapter dirs. `CURRENT_ADAPTER_SERVING` in `facts.rs` carries that value; `item_checkpoint.rs` was not touched.

## Verification

All runs `cargo test -p vox-populi --features mens-hf-hub --lib`, one at a time.

| Run | Result |
|---|---|
| `hub_card::facts` | 14 passed, 0 failed |
| `hub_card` (facts, examples, config, collateral_standin, legacy goldens) | 51 passed, 0 failed |
| `cargo check -p vox-populi --features mens-hf-hub` (non-test) | 0 warnings |

Required tests present by name: `real_sample_joins_uniquely`, `ambiguous_join_renders_no_recorded_output`, `load_reads_checkpoints_with_current_protocol`, `adapter_evidence_requires_current_serving`, `load_keeps_legacy_serving_texts`, `verdict_phrases_are_the_docs_list`, `wins_are_the_two_shortest_by_length_then_id`, `loss_example_present_or_stated_none`, `totals_are_counted_from_data`, `fixture_adapter_wins_pass_vox_admission`, `real_file_validates`, `unknown_size_is_an_error`, `missing_size_is_an_error`, `missing_blurb_is_an_error`, `blurb_prose_has_no_digits`. The legacy goldens still pass unchanged; `RunRecords::load` output for the legacy card is unchanged (only new fields added) and no gate is applied inside `load`.

Acceptance greps all as specified: defactor marker 1, `answer_region` called (2 mentions) and not defined (0), no `std::fs`/`read_to_string` in `examples.rs` (0), no `/Users/` in fixtures, report `results` length 8 equals its `total` 8, serving value present in the `evidence/` header and `null` in `legacy_serving/`, item lines identical between the two (diff empty), `nf4_base_plus_lora_arm` appears once in non-test source, `pub use facts::CURRENT_ADAPTER_SERVING` once, all six phrases in both `examples.rs` (quoted) and `docs/src/reference/mens-models.md` (backticked), hardcoded counts grep 0, `examples.rs` 218 lines, `deny_unknown_fields` 4 times, no quoted size literal in `config.rs`, `hub-cards.yaml` not gitignored.

## Mutation proofs (5, 13, 4, 2, 7)

Each: presence grep before, mutate, targeted test FAILED, restore from a byte copy (diff vs copy 0 lines), same grep after, test green again.

| # | File | Grep | Before / mutated / after | Mutation | Test | Mutated | Restored |
|---|---|---|---|---|---|---|---|
| 5 | examples.rs | `fn full_text` | 1 / 1 / 1 | return first match even with a second | `ambiguous_join_renders_no_recorded_output` | FAILED | ok |
| 13 | facts.rs | `CURRENT_ADAPTER_SERVING` | 2 / 2 / 2 | `Some(CURRENT..) \| None => Current` (accept a header with no field) | `adapter_evidence_requires_current_serving` | FAILED | ok |
| 4 | examples.rs | `chars().count()` | 1 / 1 / 1 | `fixes.sort_by(\|a, b\| b.cmp(a))` (longest first) | `wins_are_the_two_shortest_by_length_then_id` | FAILED | ok |
| 2 | examples.rs | `loss` | 3 / 3 / 3 | `loss: None` | `loss_example_present_or_stated_none` | FAILED | ok |
| 7 | config.rs | `unknown size` | 1 / 1 / 1 | unknown-size check disabled (`if false && ...`) | `unknown_size_is_an_error` | FAILED (panic: edit must be rejected) | ok |

## Fixtures: provenance

Source: `mens/runs/qwen3-4b/evidence/heldout_{base,adapter}_nothink.json` and `.ckpt.jsonl` (read only; gitignored run dir). Kept 8 of 52 items, entries byte-for-byte:

- fixes (adapter passes, base fails): `humaneval_vox_069_binary_search` (115 chars), `mutation_update` (127), `matrix_row_sums` (131), `workflow_fetch_save` (138). D2 wins on the full data are the first two.
- break: `string_is_rotation` (base passes, adapter fails)
- both pass: `page_index`
- both fail: `humaneval_vox_042_reverse_list` (the D2 loss on the full 4B data), `test_smoke`

Report totals recomputed for the subset (`total`, `passed_at_1/k`, `pass_rate_at_1/k`, `semantic_task_success`, `anti_stub_task_success`, `construct_richness_mean`, `category_stats`); `model` field set to `mens/runs/qwen3-4b`; `placeholder_event_rate` fields left as recorded (0.0 on both). Each ckpt keeps its header and the 8 item lines found by the preview rule (each matched exactly once on both sides) plus one unrelated item line. The base header's `model.snapshot` had a host path (`/Users/.../snapshots/<hash>`), rewritten to `hf-cache/models--Qwen--Qwen3-4B/snapshots/<hash>`.

**The current-serving adapter header is derived.** `evidence/heldout_adapter_nothink.ckpt.jsonl` is the real 4B header plus `"adapter_serving":"nf4_base_plus_lora_arm v2"` inside `inputs.model`; item lines are byte-identical to `legacy_serving/`. The header `key` hash was not recomputed. These outputs came from the older serving path and are NOT real current-path generations; they exercise the gate and the join only. `evidence/legacy_serving/` holds the real header (no field).

## A4 probe

`fixture_adapter_wins_pass_vox_admission` passes with the strict `support::vox_block_ok` (`verify_vox_admission`, including the interpreter smoke step) on both adapter wins. No fallback to a stage-only check was needed; `vox_block_ok` is the strict form.

## regate_eta arithmetic (source: `.planning/phases/337-eval-throughput/337-RESEARCH.md`, section Projections)

New generations for the 4B gate = 3 slices x 96 new items x 2 sides = 576. Per-item rates (AFTER f32, B=32): expected / conservative s/item: 1.7B 10/16, 4B 24/36, 8B 27/39, 14B 42/58 (all LOW confidence, unmeasured at batch).

| Size | Generations | Expected | Conservative | Basis |
|---|---|---|---|---|
| 4B | 576 | 576 x 24 = 13824 s = 3.8 h | 576 x 36 = 20736 s = 5.8 h | stated in the research text |
| 1.7B | 576 (assumed) | 576 x 10 = 5760 s = 1.6 h | 576 x 16 = 9216 s = 2.6 h | same count, cached prefix not verified for this size |
| 8B | 576 (assumed) | 576 x 27 = 15552 s = 4.3 h | 576 x 39 = 22464 s = 6.2 h | same |
| 14B | about 700 | 700 x 42 = 29400 s = 8.2 h, written 8.0 per the research text "about 8 h" | 700 x 58 = 40600 s = 11.3 h, written 11.0 per "about 11 h" | stated in the research text |

0.6B has no `regate_eta`: its re-gate is running now. 32B is not trained. Each entry has `confidence: low` and a basis string; the loader rejects anything else.

## Deviations from Plan

**1. [Rule 3] Unit tests for `config.rs` are inline (`#[cfg(test)] mod tests`)**, because the plan's `files_modified` lists no `config_tests.rs`. `facts` and `examples` tests use `_tests.rs` files via `#[path]` as planned.

**2. [Rule 2] Extra test coverage** beyond the plan's list: `unknown_keys_are_an_error` (deny_unknown_fields at four levels) and `spokes_are_well_formed` mutation cases (bad status, missing link, duplicate id, non-trained default). `tests/support.rs` also has `evidence_run`, `CURRENT_CKPT` and `LEGACY_CKPT` helpers, which the plan did not list.

**3. `#[allow(dead_code, reason = "consumed by card_facts in 339-05")]`** on `mod config;`, `mod examples;` and the `adapter_evidence` items and re-exports, as 339-01 did, because nothing consumes them until 339-05. 339-05 should delete these attributes.

**4. Design note:** `pick_examples` returns `None` when either report has no `results` array (the "absent" case, since the signature takes `&Value`). An id seen in only one report counts in `unmatched_ids` and in no other total.

**5. `regate_eta` for 1.7B and 8B assumes the 4B generation count**, labelled as assumed in `basis`.

**6. Existing finding from 339-02 stands:** `adapter_card.json` and `training_manifest.json` disagree on rank/alpha; this plan does not touch it.

## Pre-existing, not mine

`vox ci crate-edges` was not re-run: this plan changes no `Cargo.toml` or `Cargo.lock` (`git diff 001c33632..HEAD -- Cargo.lock '*.toml'` is empty), so no edge or version moved. The known violation `vox-plugin-mens-candle-core -> vox-eval` (phase 313, reported in 339-01) is unchanged and not addressed.

## Known Stubs

None. The `#[allow(dead_code)]` items are real code awaiting their consumer in 339-05.

## Threat Flags

None. All new input (reports, checkpoints) is parsed with typed fallbacks; no new network, auth or file-write surface.

## Self-Check: PASSED

- Created files exist: `hub-cards.yaml`, `config.rs`, `examples.rs`, `examples_tests.rs`, `tests/support.rs`, and the five fixture files.
- Commits `6742305da` and `451ec8289` are ancestors of HEAD.
