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
This pre-registered study asks whether agents author VoxDiagram at least as reliably as Mermaid. The same 200 natural-language diagram tasks (184 graph, 16 sequence) went to both languages, with grammar cards of comparable size, and deterministic parsers judged every reply. On the primary endpoint, first-pass validity of model A (deepseek/deepseek-v4.1-flash-1017), VoxDiagram scored 98.0% and Mermaid 96.0%. The paired difference is +2.0 points with a 95% interval of [-1.7, +5.9], so VoxDiagram is non-inferior under the 10-point margin fixed before the run. The replication model B (xiaomi/mimo-v2.6-pro) gave -0.5 points [-4.0, +2.9], also within the margin.
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
cost also stops the run. In this run, calls that failed in transport were charged $0; see
"Limitations observed in the run".

**Recording and re-score.** Every model reply is recorded in `runs/<name>/attempts.jsonl`, with the
run header first. The offline re-score reproduces every derived file byte for byte with no network,
split by runtime: Rust re-derives the oracle inputs, `tallies.json` and `tables.md`; Node re-derives
the Mermaid oracle outputs. The Rust half runs in the `tests` job of `ci.yml`, the Node half in the
`docs-quality` job of `docs-quality.yml`.

**The run.** One live run, `runs/2026-10-27`, started on 2026-10-27 at 22:33 UTC against the
pre-registration commit `f1ef3bb50d0e934244bedd63608ca0a77f2a9155`, and ended complete with no stop
line.

- Model A: `deepseek/deepseek-v4.1-flash-1017` (vendor organisation deepseek), the registry's CodeGen
  selection.
- Model B: `xiaomi/mimo-v2.6-pro` (vendor organisation xiaomi), the highest-ranked answering cloud
  model of another vendor organisation.
- Two model families were measured. Both were reached through the OpenRouter gateway. Each answered
  its single probe, so no probe failed.
- Every recorded call resolved to the requested model id; the tally flags no model drift.
- The run made 835 calls (827 recorded attempts plus 8 transport retries). Total spend was $0.2849:
  $0.2849 for attempts and $0.00002 for the 2 probes, against the $25 cap.
- Exclusions: none for model A. For model B, task g056 is excluded from both arms because one call
  still failed in transport after its retries (the response body could not be decoded).

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
### Model A

Resolved model ids: deepseek/deepseek-v4.1-flash-1017

| Arm | First pass | 95% CI | After repair | 95% CI | Fidelity (of valid) | Fidelity (of scored) | Attempts | Prompt tokens | Completion tokens | Cost (USD) |
|---|---|---|---|---|---|---|---|---|---|---|
| mermaid | 192/200 (96.0%) | [92.3, 98.0] | 198/200 (99.0%) | [96.4, 99.7] | 172/198 (86.9%) | 172/200 (86.0%) | 212 | 168814 | 141002 | 0.0906 |
| voxdiagram | 196/200 (98.0%) | [95.0, 99.2] | 200/200 (100.0%) | [98.1, 100.0] | 193/200 (96.5%) | 193/200 (96.5%) | 204 | 148423 | 89615 | 0.0564 |

### Model B

Resolved model ids: xiaomi/mimo-v2.6-pro

| Arm | First pass | 95% CI | After repair | 95% CI | Fidelity (of valid) | Fidelity (of scored) | Attempts | Prompt tokens | Completion tokens | Cost (USD) |
|---|---|---|---|---|---|---|---|---|---|---|
| mermaid | 195/199 (98.0%) | [94.9, 99.2] | 198/199 (99.5%) | [97.2, 99.9] | 140/198 (70.7%) | 140/199 (70.4%) | 206 | 157055 | 69452 | 0.0730 |
| voxdiagram | 194/199 (97.5%) | [94.3, 98.9] | 199/199 (100.0%) | [98.1, 100.0] | 182/199 (91.5%) | 182/199 (91.5%) | 205 | 149235 | 48422 | 0.0649 |

### Paired difference (VoxDiagram minus Mermaid)

| Endpoint | Model | a | b | c | d | Δ (points) | Newcombe 95% CI | Verdict | McNemar p |
|---|---|---|---|---|---|---|---|---|---|
| first pass | A | 188 | 8 | 4 | 0 | +2.0 | [-1.7, +5.9] | non-inferior | 0.3877 |
| first pass | B | 190 | 4 | 5 | 0 | -0.5 | [-4.0, +2.9] | non-inferior | 1.0000 |
| after repair | A | 198 | 2 | 0 | 0 | +1.0 | [-1.0, +3.6] | non-inferior | 0.5000 |
| after repair | B | 198 | 1 | 0 | 0 | +0.5 | [-1.4, +2.8] | non-inferior | 1.0000 |

### Per kind

| Model | Kind | Arm | n | First pass | After repair |
|---|---|---|---|---|---|
| A | graph | mermaid | 184 | 176/184 (95.7%) | 182/184 (98.9%) |
| A | graph | voxdiagram | 184 | 180/184 (97.8%) | 184/184 (100.0%) |
| A | sequence | mermaid | 16 | 16/16 (100.0%) | 16/16 (100.0%) |
| A | sequence | voxdiagram | 16 | 16/16 (100.0%) | 16/16 (100.0%) |
| B | graph | mermaid | 183 | 179/183 (97.8%) | 182/183 (99.5%) |
| B | graph | voxdiagram | 183 | 178/183 (97.3%) | 183/183 (100.0%) |
| B | sequence | mermaid | 16 | 16/16 (100.0%) | 16/16 (100.0%) |
| B | sequence | voxdiagram | 16 | 16/16 (100.0%) | 16/16 (100.0%) |

Completeness, model A: complete; 200 scored, 0 excluded for transport errors, 0 missing.
Completeness, model B: complete; 199 scored, 1 excluded for transport errors (g056), 0 missing.

Primary endpoint verdict (model A, first pass): non-inferior
<!-- eval-diagram:tables:end -->

## Verdict

Primary verdict: non-inferior.

On the primary endpoint, first-pass validity of model A (`deepseek/deepseek-v4.1-flash-1017`),
VoxDiagram was valid on 196 of 200 tasks (98.0%, Wilson 95% CI [95.0, 99.2]) and Mermaid on 192 of
200 (96.0%, [92.3, 98.0]).

- Paired, both arms were valid on 188 tasks. Only VoxDiagram was valid on 8, only Mermaid on 4, and
  neither on 0.
- Δ = +2.0 points with a Newcombe 95% interval [L, U] = [-1.7, +5.9].
- L = -1.7 is above the margin of -10 points, so the pre-registered rule assigns "non-inferior".
- No descriptive flag applies: L is not above 0 ("superior"), and U is not below 0 ("worse but within
  margin").

This is a measured result, not a decision to migrate the documentation corpus.

**Secondary results (reported without a decision).**

- **After repair, model A:** VoxDiagram 200/200 (100.0%, [98.1, 100.0]) and Mermaid 198/200 (99.0%,
  [96.4, 99.7]). Δ = +1.0 points, interval [-1.0, +3.6].
- **Exact McNemar p, model A:** 0.3877 for first pass and 0.5000 after repair. Neither difference is
  significant.
- **Structural fidelity, model A:** among valid replies, VoxDiagram matched the task's expected sets
  on 193/200 (96.5%) and Mermaid on 172/198 (86.9%). Over all scored tasks the figures are 96.5% and
  86.0%.
- **Per kind:** every first-pass failure of either model was on a graph task. Both arms were valid on
  all 16 sequence tasks at both endpoints. For model A's graph tasks, VoxDiagram went from 180/184 to
  184/184 after repair and Mermaid from 176/184 to 182/184.
- **Cost and tokens, model A:** VoxDiagram used 204 attempts, 89,615 completion tokens and $0.0564.
  Mermaid used 212 attempts, 141,002 completion tokens and $0.0906.

**Model B as a replication** (`xiaomi/mimo-v2.6-pro`, never pooled with model A). Model B was scored
on 199 tasks, with g056 excluded.

- **First pass:** VoxDiagram 194/199 (97.5%, [94.3, 98.9]) and Mermaid 195/199 (98.0%, [94.9,
  99.2]). Δ = -0.5 points, interval [-4.0, +2.9], McNemar p 1.0000. Under the same rule this is
  non-inferior.
- **After repair:** VoxDiagram 199/199 (100.0%) and Mermaid 198/199 (99.5%). Δ = +0.5 points,
  interval [-1.4, +2.8].
- **Fidelity among valid replies:** VoxDiagram 182/199 (91.5%) and Mermaid 140/198 (70.7%).
- **Cost:** VoxDiagram $0.0649 and Mermaid $0.0730.

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

## Limitations observed in the run

- **Empty replies count as invalid.** 17 of the 827 recorded attempts returned an empty reply: 15 in
  the Mermaid arm and 2 in the VoxDiagram arm. Ten of them used the full 4096-token output limit.
  The rule scores the whole trimmed reply when there is no fenced block, so an empty reply is
  invalid.
  - For model A at turn 0, 6 of the 8 Mermaid failures and 2 of the 4 VoxDiagram failures were empty
    replies rather than parse errors.
  - The first-pass endpoint therefore measures failures to produce any output as well as syntax
    errors. The verdict stands as the rule assigns it.
  - The recording cannot say why a reply was empty. At the time of the run the facade's response
    (`LlmResponse`) carried no finish reason, refusal or reasoning field, so none was recorded.
    A runaway reasoning budget, a content filter and a gateway quirk all look the same. The 10
    empty replies that used the full limit fit truncation. The other 7 do not: g128 used 225
    completion tokens.
- **Both arms sit near the ceiling.** First-pass validity was 96 to 98%, so model A's paired
  comparison rests on 12 discordant tasks. The narrow interval reflects how rarely the arms disagreed.
- **Two inexpensive models.** The registry selected two low-cost models, and the whole run cost under
  $0.30. Larger or different models were not measured.
- **One exclusion, and it favoured Mermaid.** Model B lost task g056 to a transport error, which is
  within the 5% allowance. Model A had none.
  - The failure was in a repair turn (Mermaid, turn 1), which cannot change first-pass validity.
    Rule 6 still excludes the task from both endpoints.
  - At first pass, g056 was valid in VoxDiagram and invalid in Mermaid. The exclusion therefore
    removed a discordant pair that favoured VoxDiagram.
  - With g056 kept, model B's first pass is b = 5, c = 5: Δ = 0.0 points, interval [-3.6, +3.6],
    McNemar p 1.0000. The verdict, non-inferior, is the same.
  - A future pre-registration should exclude per endpoint, dropping a task from the first-pass
    endpoint only on a turn-0 transport failure.
- **The fidelity comparison favours VoxDiagram by construction.**
  - The expected sets come from the VoxDiagram reference's own Mermaid export, and a valid
    VoxDiagram reply is scored through the same exporter.
  - So whatever the exporter canonicalises matches automatically in the VoxDiagram arm: the shape
    mapping, the default direction and the container titles. The Mermaid arm has to guess these
    wherever the prompt is loose, for example `(x)` or `([x])` for a rounded node.
  - Part of the fidelity gap (96.5% vs 86.9% for model A, 91.5% vs 70.7% for model B) is therefore
    built into the scorer. It is not evidence that models author VoxDiagram more faithfully.
  - No exporter-independent comparison, such as labels and edge endpoints only, was run.
- **Mermaid validity accepts any diagram type.** A reply that parses as a `pie` or `classDiagram`
  counts as valid, while VoxDiagram is held to its graph and sequence kinds. This follows rule 1
  ("the parser accepts it"); fidelity catches the wrong kind.
- **Failed calls were charged $0 against the cap.** The run made 9 calls that failed in transport:
  8 retries and the final failure of g056. A failure after the request was sent can still be billed,
  for example a body that does not decode after HTTP 200, and these calls were not counted.
  - The cap was also checked without reserving the next call's cost.
  - The recorded $0.2849 is therefore a lower bound on what was billed. Charged at the run's
    largest per-call cost ($0.0037), as the harness now charges them, the 9 failed calls would add
    about $0.034. The cap was never at risk.
  - The harness has since changed, for later runs only:
    - A failed call that may have been billed is charged the largest per-call cost seen. A run
      stops if one fails before any call has been priced.
    - The largest per-call cost is reserved before each call.
    - A probe that may have been billed without a usable cost stops probing.
    - Only the provider error's class and HTTP status are recorded.

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
