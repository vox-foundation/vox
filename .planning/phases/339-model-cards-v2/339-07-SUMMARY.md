---
phase: 339-model-cards-v2
plan: "07"
subsystem: mens-hub-cards
tags: [mens, hub-cards, examples, use-it, goldens, fences, serving-path]
requires: ["339-06"]
provides:
  - "Examples section (D2) on every trained card: size-specific rule-and-totals sentence, two wins, one loss, stored-field verdict lines, fence_for fences"
  - "Use it ends with serve, one eval-user-turn request on a quoted heredoc, and the adapter's recorded output (passed cards with current-serving evidence)"
  - "goldens no_examples.md and missing_output.md; passed/failed/not_evaluated/needs_re_gate/stale_serving regenerated"
  - "where-things-live row for the MENS Hub card generator"
affects: [339-08, 339-09, 339-10]
key-files:
  created:
    - crates/vox-populi/templates/hub/partials/examples.md
    - crates/vox-populi/templates/hub/partials/use_it.md
    - crates/vox-populi/src/mens/hub_card/tests/model_card/examples.rs
    - crates/vox-populi/src/mens/hub_card/tests/model_card/use_it.rs
    - crates/vox-populi/tests/golden/hub_cards/no_examples.md
    - crates/vox-populi/tests/golden/hub_cards/missing_output.md
  modified:
    - crates/vox-populi/src/mens/hub_card/{examples,examples_tests,card_facts,mod}.rs
    - crates/vox-populi/src/mens/hub_card/tests/{support,model_card}.rs
    - crates/vox-populi/templates/hub/model.md
    - crates/vox-populi/tests/golden/hub_cards/{passed,failed,not_evaluated,needs_re_gate,stale_serving}.md
    - docs/src/architecture/where-things-live.md
key-decisions:
  - "The Examples and Use it fallbacks are keyed on `f.heldout`, `f.examples` and `f.adapter_evidence`; `examples` is `Some` only for a trained size with `AdapterEvidence::Current`, so a stale report can never feed a pick (B1)."
  - "Verdict counts print as `(count N)` after the docs-page phrase, and a rejecting check prints as a backticked name. `Verdict::count()` and `Verdict::check()` return those two values; no other verdict wording exists in code or template."
  - "The non-passed Use it sentence now says what the serve code does, read from `dispatch.rs`: `vox mens serve` runs `assert_collateral_gate` (a passing `collateral_damage_report.json` with current reasoning slices) and `assert_serve_preconditions` (a passing `gate_receipt.json` with at least two substantive gates) before loading an adapter run directory."
requirements-completed: [CARD-03, CARD-07, CARD-11]
status: complete
commits: 3
plan_head_before: e7c560160d3970cc6c7f375aeabadc2b62a9f734
plan_head_after: 182f1a53448b1ba9f017c2e64edeb25e333d530b
actuals:
  tokens: 20100
  tasks: 3
  commits: 3
completed: 2026-11-06
---

# Phase 339 Plan 07: Examples section and the recorded Use-it output Summary

A trained card with current-serving evidence now shows two fixed items and one still-failing item from its own held-out run under a size-specific rule-and-totals sentence, and a passed card's Use it ends with a real request and the output the eval recorded for it; stale, missing and absent evidence each get one sentence naming the size.

## Commits

- `9f203427a` feat(339-07): render the D2 Examples section on trained model cards (Task 1, tracer)
- `4879231de` feat(339-07): end Use it with one real request and its recorded output (Task 2; also carries `vox_symbols_are_not_escaped`, see Deviations)
- `182f1a534` docs(339-07): add the MENS Hub cards row to where-things-live (Task 3)

`commits: 3` is `git rev-list --count e7c560160..HEAD`, read from the on-disk ledger. Each code commit passed tdd-guard without `--no-verify`. The tracer gate (re-run of the Task 1 verify end to end before expanding) passed: 86 `hub_card` tests green.

## Verification

Cargo runs were `-p vox-populi --features mens-hf-hub`, one at a time through the broker, `nice -n 19`, in the background.

| Command | Result |
|---|---|
| `cargo test --lib hub_card` (final tree) | 92 passed, 0 failed |
| `cargo test --lib hub_sync::tests` | 10 passed, 1 ignored (pre-existing) |
| `cargo clippy -p vox-populi --features mens-hf-hub --lib -- -D warnings` | clean, exit 0 |
| `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/where-things-live.md` | `lint complete — no hard errors` |

New tests, by name (all present in the 92): `fence_for_outlasts_backtick_runs`, `use_it_pick_skips_shown_wins` (`examples_tests.rs`); `examples_section_renders_wins_and_loss`, `rule_and_totals_on_every_trained_card_with_examples`, `totals_sentence_forms`, `stale_serving_card_shows_no_adapter_output`, `verdict_phrases_match_docs_page`, `failing_outputs_are_never_in_vox_fences`, `long_backtick_runs_in_a_prompt_cannot_close_the_fence`, `verdict_lines_name_size_and_item`, `vox_symbols_are_not_escaped` (`tests/model_card/examples.rs`); `use_it_request_uses_eval_user_turn`, `passed_card_ends_use_it_with_recorded_output`, `non_passed_card_shows_no_live_request`, `passed_stale_serving_shows_no_recorded_output` (`tests/model_card/use_it.rs`). `sections_in_spec_order` now expects `Why this size`, `Examples`, `Use it`, `Results`, `Training`, `Files`.

Acceptance greps: `^## Examples$` in `needs_re_gate.md` 1; `For 1.7B, the examples are` 1 in `needs_re_gate.md`, 0 in `stale_serving.md`; `include "partials/examples.md"` and `include "partials/use_it.md"` in `model.md` 1 each; `^## Use it` in `model.md` 0; `fn fence_for` in `examples.rs` 1; the verdict-phrase loop printed nothing (all six phrases in code and on the docs page); `passes|does not parse|type errors|rejected by` in `partials/examples.md` 0; `git diff --quiet HEAD -- docs/src/reference/mens-models.md` succeeded; `examples.rs` is 273 lines; `no_examples.md` has `^## Examples$` 1 and `^### ` 0; `missing_output.md` has `no recorded output` 3 times; `/Users/` and `/home/` in every golden 0; `not_trained.md` and `index.md` unchanged against HEAD.

## Bench-manifest check (blocking-human condition for Task 2: not tripped)

The request on a card is the eval's user turn only when no held-out item carries a primary file or context files.

```
jq -c '[.benchmarks[]|select((.file // "") != "" or ((.context_files // [])|length) > 0)]|length' mens/data/heldout_bench/manifest.json
-> 0        (of 52 items; mens/data/heldout_bench/ holds manifest.json only)
```

`eval_local_prompt.rs` builds `Write a Vox program that: <description>` and adds a primary-file block only when the primary file has text; no item has one. `serve` applies the same `user_for_lane(.., Codegen)` wrapper the eval does, so the request carries no `/no_think` of its own.

## Mutation proofs

Each mutant was applied to a file copied to the scratchpad first, run, and restored by `cp`; `git diff --quiet HEAD` on the file succeeded afterwards.

**Proof 6 (plan).** File `crates/vox-populi/src/mens/hub_card/render.rs`. Grep `escape = "none"`: before 2 (the attribute and a comment), mutated 1, after restore 2. Mutation: `#[template(path = "hub/model.md", escape = "none")]` became `escape = "html"`. Run of `--lib vox_symbols_are_not_escaped`:

```
failures:
    mens::hub_card::tests::model_card::examples::vox_symbols_are_not_escaped
test result: FAILED. 0 passed; 1 failed
```

Restored; the full `hub_card` run then printed `92 passed`.

**Extra proofs (serving-path gate T-339-35, fence T-339-16), one mutated build.**
- `card_facts.rs`: `let examples = (gate.trained && evidence == AdapterEvidence::Current)` became `(gate.trained)`. Grep before 1, mutated 0, after restore 1. Killed: `stale_serving_card_shows_no_adapter_output` and `passed_stale_serving_shows_no_recorded_output` (both FAILED).
- `examples.rs`: `"`".repeat(longest.max(2) + 1)` became `"```".to_string()`. Grep `longest.max(2) + 1` before 1, mutated 0, after restore 1. Killed: `fence_for_outlasts_backtick_runs` and `long_backtick_runs_in_a_prompt_cannot_close_the_fence` (both FAILED).

Mutated run: `test result: FAILED. 0 passed; 4 failed`. All three files were restored from byte copies and showed no diff against HEAD.

## Golden review (each golden read line by line before committing)

Every number and every output on the trained goldens comes from the shared 339-03 evidence fixture (real stored 4B held-out reports and checkpoints from `mens/runs/qwen3-4b`, copied read-only, host paths absent). The fixture is rendered under the case's own size, so the 0.6B, 1.7B and 14B goldens show the same eight items and the same 2/8 and 5/8 scores. They are not those sizes' real results; say so if a golden is ever shown as real data.

- `needs_re_gate.md` (1.7B, current-serving checkpoint). `## Examples` between Why this size and Use it. Totals sentence: `For 1.7B, the examples are [the 2 shortest items the adapter fixes and the shortest it still fails](…#examples-on-the-cards); of 8 items it fixes 4 and breaks 1.` Two fixed blocks (`humaneval_vox_069_binary_search`: base `type errors (count 7)`, adapter `passes`; `mutation_update`: base `does not parse (count 3)`, adapter `passes`) and one `Still fails` block (`humaneval_vox_042_reverse_list`: base `type errors (count 2)`, adapter `type errors (count 1)`). Passing adapter code is in `vox` fences, failing code in `text` fences; prompts in `text` fences. Use it has the pull with `--allow-ungated` and the sentence about what `vox mens serve` checks, no serve or request block.
- `passed.md` (1.7B, current-pass collateral). Same Examples content. Use it: pull, `vox mens serve --model mens/runs/qwen3-1.7b --port 11435`, a `curl` on a quoted `<<'JSON'` heredoc whose body is the pretty-printed request (lane `codegen`, user turn `Write a Vox program that: define a page named `IndexPage` …`, `max_tokens` 1024, `temperature` 0.0 from the report), one label sentence naming 1.7B, the item `page_index` and `evidence/heldout_adapter_nothink.ckpt.jsonl`, then the recorded component in a `vox` fence. `page_index` is the shortest adapter-passing item that is not a shown win (both sides pass it). Exactly one blank line before `## Results`.
- `failed.md` (0.6B) and `not_evaluated.md` (14B): the same Examples under `For 0.6B,` and `For 14B,`; non-passed Use it as above, with the size in the serve sentence.
- `stale_serving.md` (1.7B, legacy checkpoint with no serving field): Examples is one sentence (`No 1.7B example is shown because the adapter checkpoint has no serving field, so its outputs do not count under the serving-path rule.`) and `no recorded output` in a `text` fence. No totals, no `###`, none of the legacy adapter code (asserted from the legacy fixture).
- `missing_output.md` (needs-re-gate records, base checkpoint removed): same Examples, but every base side reads `no recorded output` while its stored verdict line (`type errors (count 7)`) stays; adapter sides show code.
- `no_examples.md` (passed records, no held-out evidence at all): Examples is `No held-out report is on record for 1.7B, so no example is shown for it.`, no `###`; Use it keeps pull and serve and ends with `The 1.7B adapter output for a request is not shown because no adapter checkpoint records its serving path.` plus `no recorded output`.

Three lines of model output in `passed.md` carry trailing spaces (the model's own blank-indented lines). They are kept byte-exact on purpose; the fmt-fix and tdd-guard hooks accepted them.

## User choices (defaults implemented; not blocking)

1. **Reduced Use it on non-passed cards (checker W8, 339-10 choice 11).** They show the pull with `--allow-ungated` plus one sentence, and no serve or request block. The sentence reads: `vox mens serve` loads the {size} adapter only from a run directory that holds a passing collateral report and a passing `gate_receipt.json`, so the pull needs `--allow-ungated` and no serving example is given. This is verified against `dispatch.rs` (`assert_collateral_gate`, `assert_serve_preconditions`); the phase-336 claim was true in substance but vaguer. Alternative: a full serve-and-request block even when ungated (it would fail at serve).
2. **Verdict count format.** Counts print as `type errors (count 7)` rather than `7 errors`, so the phrase list on the docs page stays the only verdict wording. Say if a more natural form is wanted.
3. **Zero-win and no-loss wording.** `none that the adapter fixes` and `no item it still fails` are the template's forms for those edge cases (tested in `totals_sentence_forms`); the docs page does not specify them.

## Deviations from Plan

**1. [Rule 3 - structure] New test files instead of growing `model_card.rs`.** The plan puts every new test in `tests/model_card.rs`, already 576 lines. The new tests are in `tests/model_card/examples.rs` and `tests/model_card/use_it.rs`, declared as child modules of `model_card`, so their paths still start with `hub_card::tests::model_card` and the plan's verify commands match them. `vox_symbols_are_not_escaped` lives in `examples.rs`.

**2. [Rule 3 - scope] `vox_symbols_are_not_escaped` landed in the Task 2 commit.** It shares helpers with the Examples tests and was written in the same cycle, so Task 3's commit holds only the where-things-live row. Mutation proof 6 was still run after both commits.

**3. [Rule 3 - golden set] The cross-card sentence check compares different sizes.** `rule_and_totals_on_every_trained_card_with_examples` asserts the clause differs between cards of different size. `needs_re_gate`, `passed` and `missing_output` are all 1.7B cases over one evidence set, so their clause is identical by construction; the real ladder holds one card per size and 339-09's lint covers that set.

**4. [Rule 3] `mod.rs` edited.** The plan's file list omits `hub_card/mod.rs`, but the remaining `#[allow(dead_code, reason = "consumed by the Examples section in 339-07")]` on `mod examples` had to come off now that the module is used. `render.rs` was not changed (an import I first added was reverted; `Verdict::count()` and `Verdict::check()` let the template avoid naming the type).

**5. [Rule 1] Template whitespace and wording fixes found while reading goldens.** Two missing or doubled blank lines before `## Use it` / `## Results` in fallback branches, and a request-intro sentence that did not name the size (a repeat across cards if two sizes pick the same item id, which 339-09's lint would flag). All fixed before the commits and covered by the goldens.

**6. Task 1 regenerated five goldens, not two.** The plan's Task 1 names `needs_re_gate.md` and `stale_serving.md`, but `passed`, `failed` and `not_evaluated` run in the same golden test and also gain Examples, so they were regenerated in Task 1 and again in Task 2.

## Known Stubs

None. No placeholder text or empty value reaches a card; every fallback is a sentence naming the size.

## Threat Flags

None new. T-339-16 (output closing its fence) is covered by `fence_for_outlasts_backtick_runs`, `long_backtick_runs_in_a_prompt_cannot_close_the_fence` and the fence mutation; shown code can hold at most two consecutive backticks because `shown_code` stops at the first closing fence, so the longer-fence rule matters mainly for item descriptions. T-339-17 by the quoted heredoc and the JSON-parse test. T-339-18 by `use_it_request_uses_eval_user_turn` (source-tied to `eval_local_prompt.rs`), the recorded decode settings and the bench-manifest check. T-339-35 by the two stale-serving tests and the gate mutation. T-339-36 by the rule clause on every card with examples. Record text and model output still reach a public README unescaped by design (mutation 6 keeps it byte-exact).

## Notes for 339-09 and 339-10

- Sentences new on a card, all naming the size: the Examples totals sentence, the unmatched-ids sentence, three Examples fallbacks (no reports, no checkpoint, stale serving, no per-item results), the request intro, the label sentence, three Use-it fallbacks, and the serve sentence on non-passed cards. Verdict lines are list-free paragraphs of the form `Qwen3-<size> base on `<id>`: <phrase>.` and `<size> adapter on `<id>`: <phrase>.`, each unique per size and item.
- The unmatched-ids sentence is exercised only by the synthetic test; no committed fixture has unmatched ids.
- Real cards will show examples only for sizes re-evaluated after 335's serving change; 339-10 records which.

## Self-Check: PASSED

- Created files exist: `partials/examples.md`, `partials/use_it.md`, `tests/model_card/examples.rs`, `tests/model_card/use_it.rs`, `no_examples.md`, `missing_output.md`.
- Commits `9f203427a`, `4879231de` and `182f1a534` are ancestors of HEAD.
