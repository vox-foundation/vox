---
title: "VoxDiagram Authoring Reliability Findings (2026)"
description: "Pre-registered paired comparison of how reliably agents author valid VoxDiagram and Mermaid on 200 tasks, with the decision rule fixed before any model call."
category: "Architecture SSOTs"
topic: "doc-pipeline"
domain: "documentation"
status: "research"
authorship: "llm-assisted"
authorship_model: "claude-opus-5-5"
---

# VoxDiagram Authoring Reliability Findings (2026)

<!-- kb:summary -->
This pre-registered study asks whether agents author VoxDiagram at least as reliably as Mermaid. The same model receives 200 paired natural-language diagram tasks (184 graph, 16 sequence) in both languages, with grammar cards of comparable size and one shared prompt template. Deterministic parsers judge every reply. The primary endpoint is first-pass validity; validity after up to two repair turns is secondary. The decision rule below was committed before any live model call. Results are pending: the live run has not taken place.
<!-- /kb:summary -->

## Question

Do agents author VoxDiagram at least as reliably as Mermaid (requirement EVAL-01)? The study reports a
measured answer. It is not a go/no-go decision on migrating the documentation corpus.

## Method

**Tasks.** 200 hand-authored tasks in `contracts/eval/diagram-authoring/tasks.v1.jsonl`: 184 graph
tasks and 16 sequence tasks. Each prompt is plain prose, identical for both arms, and quotes every
label it expects verbatim. Each task carries a reference VoxDiagram source; the pinned Mermaid parser's
canonical node, edge and container sets of its export are the task's expected sets. A quota gate
measures the hard cases from those canonical sets:

| Hard case | Minimum | Tasks |
|---|---|---|
| punctuation in a label | 60 | 101 |
| nested group depth of 2 or more | 30 | 58 |
| edge labels | 80 | 148 |
| self-loops | 20 | 42 |
| fan-in or fan-out of 3 or more | 30 | 56 |
| non-default shapes | 30 | 75 |
| Mermaid-hostile labels (`end`, a leading o or x) | 15 | 33 |
| sequence notes | 10 | 15 |
| sequence replies | 10 | 15 |
| sequence self-messages | 4 | 9 |

**Grammar cards.** The VoxDiagram card is a byte copy of the shipped grammar card (2189 bytes, 359
words). The Mermaid card mirrors it section for section (2282 bytes, 364 words). Both cards and the
template are pinned by BLAKE3 hashes in `manifest.v1.yaml`.

**Prompt template.** One template serves both arms. The system message is the arm's card. The first
user message asks for one fenced block in that language. A repair turn sends the arm's own parser
output verbatim and asks for a corrected block. The arms differ only in the language name, the fence
tag and the word for the checker.

**Model selection.** No model is pinned. Model A is the highest-ranked model of the model registry's
CodeGen selection, restricted to cloud providers, that answers a one-call probe with a reported cost.
Model B is the highest-ranked answering cloud model of a different vendor organisation, if one
exists. A selection forced through the environment is refused. Every call goes through the
model-agnostic facade (`vox_actor_runtime::llm`) with one locked configuration per model: temperature
0, at most 4096 output tokens, no tools. The provider-reported model id of every call is recorded.

**Oracles.** VoxDiagram validity is `vox_diagram::check` reporting no error. Mermaid validity is
acceptance by the pinned Mermaid 11.17.2 parser (`mermaid.parse`), run in Node by
`docs-astro/tests/lib/mermaid-batch.mjs`. The checked text is the first fenced block of a reply, else
the whole trimmed reply, for both arms. No LLM judges anything.

**Bounded repair.** An invalid reply gets up to 2 repair turns, each fed only that arm's own parser
diagnostics: the `check` diagnostics as JSON for VoxDiagram, the parser's error text for Mermaid. The
harness never applies VoxDiagram's own fixes; the model must repair.

**Spend cap.** The live run stops before any call, probes included, once the cumulative spend reaches
$25. Spend counts every earlier run and every probe in the dataset's probe ledger. A reply without a
cost also stops the run.

**Recording and re-score.** Every model reply is recorded in `runs/<name>/attempts.jsonl`, with the
run header first. The offline re-score reproduces every derived file byte for byte with no network,
split by runtime: Rust re-derives the oracle inputs, `tallies.json` and `tables.md`; Node re-derives
the Mermaid oracle outputs. The Rust half runs in the `tests` job of `ci.yml`, the Node half in the
`docs-quality` job of `docs-quality.yml`.

## Decision rule

<!-- prereg:begin -->
> Decision rule, pre-registered on 2026-10-27 before any live model call.
> 1. Primary endpoint: first-pass validity (turn 0) of model A. Model A is the highest-ranked cloud model of the
>    model registry's CodeGen selection (candidate scope CloudOnly; no model is pinned) that answers a one-call probe
>    with a reported cost. A reply is valid for VoxDiagram when `vox_diagram::check` reports no error, and for Mermaid
>    when the pinned Mermaid 11.17.2 parser (`mermaid.parse`) accepts it. The checked text is the first fenced block of
>    the reply, else the whole trimmed reply, for both arms.
> 2. Effect: Δ = p(VoxDiagram) − p(Mermaid) over the 200 paired tasks (184 graph, 16 sequence), same model, temperature
>    0, the same seed per task and turn, grammar cards of comparable size.
> 3. Interval: the two-sided 95 % Newcombe hybrid-score interval for a difference of paired proportions (method 10,
>    Wilson-based MOVER, no continuity correction), written [L, U].
> 4. Non-inferiority margin: δ = 0.10 (10 percentage points).
> 5. Verdict: non-inferior if L > −0.10; inferior if U < −0.10; inconclusive otherwise. Descriptive flags that never
>    change the verdict: "superior" if L > 0; "worse but within margin" if U < 0 and L > −0.10.
> 6. Missing data: a task whose call still fails in transport after 3 retries of the same model, in any turn of either
>    arm, is excluded from both arms. The run is incomplete, and the primary verdict is "not determined", if exclusions
>    exceed 5 % of the 200 tasks or if the $25 spend cap or any other stop ends the run before model A has turn 0 for
>    all 200 tasks in both arms.
> 7. Secondary results, reported without a decision: validity after up to 2 repair turns, each fed only that arm's own
>    parser diagnostics, with the same interval; per-arm Wilson 95 % intervals; exact McNemar p; structural fidelity
>    against each task's expected sets; per-kind counts; tokens and cost; and model B (the highest-ranked answering
>    cloud model of a different vendor organisation) under the same rule as a replication, never pooled with model A.
>    If no such model answers, the report says that only one model family was measured.
> 8. The provider-reported model id of every call is recorded. Each model runs on one locked configuration; there is
>    no mid-run fallback to another model.
<!-- prereg:end -->

## Results

<!-- eval-diagram:tables:begin -->
Results pending: the live run has not taken place.
<!-- eval-diagram:tables:end -->

## Verdict

Pending. The verdict will be the one the decision rule above assigns to the recorded data.

## Limitations known in advance

- One sample per task and arm at temperature 0, so the study measures one draw, not a distribution.
- Seeds are requests, not guarantees: providers that ignore seeds may answer differently on a rerun.
- Mermaid validity is parse acceptance, which is more lenient than rendering.
- VoxDiagram `check` also rejects undeclared nodes, which Mermaid silently creates; that strictness
  counts against VoxDiagram.
- 50 of the 184 graph references are adapted from documentation diagrams that were public Mermaid
  until Phase 90. Memorisation of public corpora can only favour Mermaid.
- The wording of the two grammar cards may favour one arm despite their comparable size.
- All models are reached through one provider gateway.
- The re-score is tied to the recorded oracle versions: a later vox-diagram or Mermaid change makes it
  fail on purpose.
- `cost_usd`, and so the spend cap, may be an estimate from the model's `cost_per_1k` price when the
  provider omits the cost (`vox_actor_runtime` llm/chat.rs:174).

## Reproduce

Offline re-score, no network:

```text
cargo nextest run -p vox-cli --lib --test it -E 'test(/eval_diagram/)'
pnpm --dir docs-astro exec node --test tests/unit/diagram-authoring-rescore.test.mjs
```

Live run (spends up to the $25 cap; run once, as plan 91-05 describes). The freeze commit is the
commit that added this decision rule, whose subject starts with `docs(91-04): pre-register`:

```text
EVAL_DIAGRAM_FREEZE_COMMIT=<freeze commit> \
  cargo test -p vox-cli --test it -- --ignored --exact eval_diagram_live::live_run --nocapture
```

## Data

Everything lives under `contracts/eval/diagram-authoring/`: `manifest.v1.yaml` (counts, pinned
hashes, the machine form of this rule), `tasks.v1.jsonl`, `cards/`, `prompt-template.txt`, the
reference files, `probe-spend.jsonl` (the probe ledger) and `runs/<name>/` (the recording and its
derived files).
