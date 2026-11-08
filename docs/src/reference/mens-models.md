---
title: "Reference: MENS models, spokes and gating"
description: "What the MENS adapter ladder is, how hub and spoke work, what the gate states mean, and how the model cards on Hugging Face are produced."
category: "Language Reference"
status: "current"
topic: "mens-training"
domain: "ml"
authorship: "llm-assisted"
authorship_model: "claude-sonnet-5-5"
---

# Reference: MENS models, spokes and gating

<!-- kb:summary -->
This page holds every explanation that the MENS model cards on Hugging Face share, so each card can link here instead of repeating it. It defines the adapter ladder, hub and spoke, the harness, the three independent facts that describe a model (trained, spoke, gate), what each gate check measures, how the before and after examples on a card are chosen, the prompt protocol, and the rule that stops two cards from repeating a sentence. Cards link to its headings by anchor.
<!-- /kb:summary -->

## What MENS is

MENS is the Vox project's own adapter ladder. Each rung is one size of a pinned Qwen3 base model plus a small LoRA adapter trained on Vox source.

The base weights are never changed. An adapter is a separate file that is applied on top of the pinned base at load time.

The ladder exists to answer one question per size: does the adapter write better Vox than the base model it sits on? Every adapter is therefore scored against its own base on the same held-out items, with the same prompt and the same decode settings.

Each size has its own repository on the Hugging Face Hub. A separate index repository, `vox-mens-index`, lists the sizes and is the only place that shows all of them together. The Hub namespace is set by the `VOX_MENS_HF_NAMESPACE` environment variable.

## Hub and spoke

The hub is the pinned base model of one size. A spoke is an adapter trained for one job on top of that hub. A size can have several spokes, and each spoke is gated on the job it exists to do.

```vox-diagram
graph {
    direction: right

    node base: "Pinned Qwen3 base model, one repo per size"
    node harness: "Harness: gates, eval and prompt protocol"

    container spokes: "Spoke adapters" {
        node vox: "vox-lang (trained)" { shape: rounded }
        node tool: "harness: tool-selection (planned)" { shape: rounded }
        node args: "harness: argument-generation (planned)" { shape: rounded }
        node rust: "rust (planned)" { shape: rounded }
    }

    base -> spokes.vox: "QLoRA adapter"
    base -> spokes.tool: "adapter"
    base -> spokes.args: "adapter"
    base -> spokes.rust: "adapter"
    harness -> spokes.vox: "gate"
}
```

The spokes and their status today:

- **vox-lang** is trained. It writes Vox source from a description. Its training file, `train_mixed.jsonl`, holds two lanes: `vox_codegen` and `vox_docs_qa`.
- **harness: tool-selection** is planned. No adapter has been trained for it. Phase 338 adds a behavioural suite that gates it.
- **harness: argument-generation** is planned. No adapter has been trained for it. Phase 338 adds a behavioural suite that gates it.
- **rust** is planned. No adapter has been trained for it. It is gated on compile-and-test pass rates.

Every adapter on the ladder today is a vox-lang adapter. The cards show that value because the run records do not carry training lanes yet. It is declared in `mens/config/hub-cards.yaml` as `default_spoke`. Phase 338 plan 06 replaces the declaration with a value derived from the training-lane histogram of each run.

## The harness

The harness is the Vox agent loop. The model is offered a catalog of tools with schemas, picks a tool, writes its arguments, and receives the tool's result. The harness spokes are adapters for two steps of that loop. The tool-selection spoke picks the right tool. The argument-generation spoke writes arguments that satisfy the tool's schema. Their training lanes are `vox_tool_selection` and `vox_argument_generation` in `mens/config/domain-profiles.yaml`.

A harness spoke plugs into the same hub as the vox-lang spoke: it is an adapter on the pinned base of one size, and the harness runs its evaluation.

A harness spoke is not gated on math or on text matching a reference. It is gated on multi-turn episodes run against the real harness with its live tool schemas. The episode outcome and schema validation decide the score. Each suite ships with a known-bad control, such as the base model, that must fail it. A suite that the control passes proves nothing, so it is not used as a gate.

## Gating

A gate decides whether an adapter's evidence is good enough to publish it as passing. The gate policy lives in `mens/config/eval-gates-ladder.yaml`, and the result of running it is recorded in `gate_receipt.json` in the run directory.

Two checks decide a vox-lang adapter. The first asks whether the adapter writes working Vox. The second asks whether training damaged what the base model could already do.

### Held-out pass rate

Held-out pass@1 is the share of held-out items for which the single greedy completion passes. Held-out means the items are not in the training data. The bench is the manifest under `mens/data/heldout_bench`.

An output passes when it parses, type-checks and is not a stub. The check records `parse_ok`, `typecheck_ok`, `diag_errors` and `anti_stub_pass` for every output.

The base model and the adapter are run on the same items under the prompt protocol `qwen3_non_thinking_codegen_v1`, which is defined [below](#prompt-protocol-and-training-method). The card shows both scores so the difference is the adapter's effect.

Adapter evidence counts only when it was produced by the current serving path. Adapters are served as the NF4-quantized base plus the full-precision LoRA arm, exactly as they were trained. The adapter checkpoint header records this as `inputs.model.adapter_serving`, and the current value is the `ADAPTER_SERVING` constant in `crates/vox-ml-cli/src/commands/mens/item_checkpoint.rs`. A checkpoint whose header lacks the field, or names another path, is not current. Earlier generations folded the adapter delta into the weights before quantizing and lost most of it, so their scores are not evidence. Base-model results are unaffected.

### Collateral damage

Collateral damage is the loss of ability that training causes in something it did not target. The check asks whether the adapter still reasons as well as its base.

It uses three fixed reasoning slices: `gsm8k_think`, `math500_think` and `vox_think`. Each slice is 128 items, not a full GSM8K or MATH-500 run. For each slice the report holds accuracy and think-initiation, base against adapter, on the same items. The three slices' accuracy is also pooled.

A pass means no detectable regression. It does not mean the adapter reasons better.

### Why McNemar

Each item is scored for the base and for the adapter, so the two scores are paired. The test looks only at the items where the two disagree: items the adapter loses and items it gains. If losses outnumber gains by more than chance explains, the adapter has regressed.

The rule is the exact one-sided McNemar test on paired items, written in the report as `exact one-sided McNemar on paired items, fail when p < alpha`. The constant is `PAIRED_RULE` in `crates/vox-eval/src/reasoning_slice.rs`. A metric fails when its p-value is below alpha, and alpha is 0.05 in `collateral_damage.alpha` of the policy file.

The test is exact, so it does not rely on a large-sample approximation. It is one-sided because only a drop matters.

The test can miss small drops at these sizes. The power table in the policy file shows that a 20 percent relative drop is caught reliably when the three slices are pooled, and a 5 percent drop is not.

### Pinned slices

A report is evidence only for the items it was computed on. The policy pins each slice by the BLAKE3 hash of its items, under `collateral_damage.slices.<name>.blake3`, and the report records the hash it used as `slice_blake3`.

A report whose hash differs from the pin is refused, not reinterpreted. The same holds when the report's paired rule or alpha differs from the policy, or when its `grader` id differs from the grader in the running code. The current grader id is `GRADER_ID` in `crates/vox-eval/src/qwen_reasoning.rs`.

An earlier rule used 32-item slices and a 5 percent relative drop. A report from that rule is not evidence under the current one.

### State legend

A model is described by three independent facts. Each card states each fact once, in the status line under its title.

**Trained.** `trained` means the adapter weights exist and `train-record.json` says the run finished. `not trained` means they do not.

**Spoke.** The spoke is the job the adapter was trained for: `vox-lang` today. `harness: tool-selection`, `harness: argument-generation` and `rust` are planned and have no adapter.

**Gate.** The gate has four values, computed from `gate_receipt.json` and `collateral_damage_report.json` against the current policy:

- `passed`: the adapter was measured under the current rule and met it.
- `needs re-gate`: the report passed under a rule that is no longer current. A report counts as current only when its slice pins, its paired rule, its alpha and its `grader` id all match the policy and the running grader.
- `failed`: a current-rule evaluation measured the adapter and it did not pass.
- `not evaluated`: the model is trained but a required current-rule evaluation is missing.

The gate part of the status line names what blocks it, for example the rule the old report passed under.

## Examples on the cards

A model card shows three real examples taken from the stored held-out evaluation of its own size. The rule that picks them is fixed, so no one chooses flattering items.

- **Two wins.** Among items the adapter passes and the base fails, the two with the shortest prompt. A tie in prompt length goes to the lower item id.
- **One loss.** Among items the adapter still fails, the one with the shortest prompt, with the same tie-break. If the adapter fails no item, the card says so.

The card then states the totals. An item the adapter fixes is one the adapter passes and the base fails. An item the adapter breaks is one the base passes and the adapter fails. Items both pass or both fail are in neither count.

Each example shows the prompt, the base output and the adapter output. Each output has one compiler-verdict line. The verdict comes from the stored check fields of that output and is never recomputed by the card renderer. There are six phrases:

| Phrase on a card | Meaning (from stored check fields only) |
|---|---|
| `passes` | `pass` is true: it parses, type-checks and is not a stub |
| `no code` | the output held no code to check |
| `does not parse` | `parse_ok` is false; the stored error count follows when recorded |
| `type errors` | it parses but `typecheck_ok` is false; the `diag_errors` count follows when recorded |
| `stub` | it compiles but the anti-stub check failed |
| `rejected by` | it compiles, is not a stub, and the named stored check is false |

When an output is missing from the checkpoint, the card says `no recorded output`. An example is never dropped without saying so.

The output under "Use it" on a card is the recorded output of that request's item from the held-out run. It is not a transcript of `vox mens serve`.

The serving-path rule from [Held-out pass rate](#held-out-pass-rate) applies here. An adapter output appears on a card only when the adapter checkpoint header's `inputs.model.adapter_serving` names the current serving path. Otherwise the card shows no adapter output and says why in one sentence. Base outputs are shown either way.

## Prompt protocol and training method

The held-out evaluation uses the prompt protocol `qwen3_non_thinking_codegen_v1`. It is the Qwen3 non-thinking prefix that training supervises: a `/no_think` marker on the user turn and a prefilled empty think block. The checkpoint header and the report both record the protocol name.

The user turn is `Write a Vox program that: <description>`, where the description comes from the bench item. Decoding is greedy: temperature 0, one sample per item.

### Shared training settings

All trained sizes use the same method: QLoRA through the `candle_qlora` kernel. They share the training data file, whose hash is recorded as `train_data_sha256` in `adapter_card.json`. They share the seed, the sequence length and the epoch count, recorded in `adapter_card.json` (`seed`), `train-record.json` (`seq_len`, `epochs`) and `training_manifest.json`.

The LoRA rank and alpha are recorded per run in `adapter_card.json` (`lora_rank`, `lora_alpha`) and in `training_manifest.json` (`rank`, `alpha`). The preset is recorded in `train-record.json` as `preset` and differs by size, as the preset follows the memory tier the size trains on. Fields that differ between sizes are stated on that size's card, not here.

The data contract is in [Mens training data contract](mens-training-data-contract.md). The training pipeline is in [Mens native training SSOT](mens-training.md).

### Why PEFT cannot load the adapters

The adapter file `candle_qlora_adapter.safetensors` uses Candle key names. For each of the `q`, `k`, `v`, `o`, `gate`, `up` and `down` projections in every layer, it holds a pair of tensors named like `l{n}.q.lora_a.weight` and `l{n}.q.lora_b.weight`, plus one `lm_head` pair.

The repository ships no `adapter_config.json`. `PeftModel.from_pretrained` needs that file and PEFT's own key names, so it cannot load these adapters. The `transformers` library cannot load them either. Use the Vox CLI, which serves them as described in [Mens local serving SSOT](mens-serving-ssot.md).

## How the cards are made

Cards are generated from the run records by `vox mens hub sync`. The renderer reads the records, builds one facts object per size, and fills one template. All wording is in the templates and in `mens/config/hub-cards.yaml`. The 32B card is the exception: it is hand-written, and the generator does not touch it.

Each fact is stated once. A model card states its own state and never another size's. The index grid is the only place that lists every size's state.

The index shows an estimated evaluation time for work that is still to do. These figures are estimates, not measurements. They come from `hub-cards.yaml`, and the index states the basis of each.

Nothing is pushed before review. The rendered cards are written to `mens/runs/hub_cards/<repo>/README.md`, read side by side, and only then synced.

A lint enforces that no sentence is repeated across cards. This is the rule it implements.

**The set.** The set is every card rendered for one ladder: one model card per size, and the index. From each card, drop the YAML frontmatter, every fenced code block, and the footer line. The footer is the one exception the rule allows.

**Units.** Each heading, list item and table row is its own unit. Consecutive plain lines of one paragraph join into one unit.

**Splitting.** Split each unit after `.`, `?` or `!` when a space or the end of the unit follows.

**Normalization.** Normalize each sentence in this order: keep link text and drop the URL, remove inline-code backticks and emphasis markers, turn table pipes into spaces, lowercase, collapse whitespace, drop trailing punctuation.

**Words.** A word is a whitespace-separated token holding at least one letter or digit.

**The failure.** Any sentence of four or more words that occurs twice in the set fails the build. It fails whether the two occurrences are on two cards or twice on one card.

Two consequences follow. A sentence that a template repeats for each size must name that size, because numbers alone can coincide. The per-size blurbs in `hub-cards.yaml` must differ sentence by sentence. Section headings on model cards are at most three words, so they never count as sentences, and the title names the size.

## Links

- [Mens native training SSOT](mens-training.md): the training pipeline, at `/reference/mens-training/`.
- [Mens training data contract](mens-training-data-contract.md): the JSONL contract, at `/reference/mens-training-data-contract/`.
- [Mens local serving SSOT](mens-serving-ssot.md): serving an adapter, at `/reference/mens-serving-ssot/`.
- The `vox-mens-index` repository on the Hugging Face Hub: the one place that lists every size.

This page links to those pages and does not copy their content.
