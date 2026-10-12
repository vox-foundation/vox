# Research Trial Flywheel — Quality Evaluation Research

Scope: how to rate research-agent answer quality, speed, and efficiency with hard gates plus a Pareto frontier. Every claim below cites file:line on branch `feat/research-trial-flywheel`.

## 1. Existing metrics and evaluators (reuse value)

| Surface | Location | What it does | Reuse verdict |
|---|---|---|---|
| Eval harness | `crates/vox-cli-research/src/eval.rs:26-118` | Runs the live pipeline per query, then scores it in the same function | **Split it.** Scoring has to become a pure fn over a frozen result. |
| Legacy `quality_score` | `eval.rs:93` | `(groundedness + citation_precision + recall.unwrap_or(0.5)) / 3` | **Display-only.** Imputes 0.5 for missing gold and mixes in proxy signals. |
| Token "recall@5" | `crates/vox-search/src/evaluation.rs:43-62` | Word-set overlap (words longer than 3 chars) against the gold answer; empty gold returns 1.0 (`:56-58`) | Misnamed (it is not recall@5). Keep only as a lexical diagnostic. |
| Groundedness | `evaluation.rs:65-100` | A sentence counts as grounded if *any* of its first 5 words (longer than 4 chars) appears anywhere in the evidence; no qualifying sentences returns 1.0 (`:79-81`) | Negation-blind proxy. Must not gate anything. |
| Citation-index precision | `eval.rs:239-265` | `[n]` markers resolvable to `1..=evidence_count`; zero evidence: 1.0 if "abstained" | Index resolvability is a good structural check. The zero-evidence branch is broken (see §6, test R1). |
| Abstention | `eval.rs:267-273` | Returns `true` whenever `no_evidence` is true, whatever the answer text | **Bug:** a hallucinated answer with no evidence is counted as a valid abstention. |
| Citation audit | `crates/vox-research-shim/src/research/orchestrator/pipeline.rs:1211-1252`, `quote_overlaps` `:1295-1308` | A citation is supported when a verifier `Supporting` span shares its source and overlaps (24+ char substring, or 3 shared 4+ char tokens) | **High value.** This is the base for claim-level citation precision. Fix: zero citations gives `precision = 1.0` (`:1245-1246`). |
| Claim verifier | `research/verifier.rs:40-45` (Supported/Contradicted/Contested/Unverified), `:89-103` (`ClaimVerdict` + `evidence_spans` + `resample_stability`), `:132` (majority vote, disagreement becomes Contested) | Claim-level LLM verification with resampling | **High value.** This is the judge substrate. The stability score already measures judge test-retest agreement. |
| Holistic LLM judge | `research/orchestrator/stages.rs:60-74` | One 0-100 score (accuracy 33 / density 33 / coverage 34); sees only the top 5 citations, 200 chars each (`:76-85`) | Opaque single number. Legacy display only. |
| Run metadata | `research/types.rs:284-345` | `quality_score` i32 (`:295`), `citation_audit` (`:300`), `corroboration_counts` (`:307`), `low_grounding_evidence` (`:313`), `judge_error`, where a failed judge also stores score 0 (`:319-321`), `served_from_cache` (`:323`), `claims_extracted_count` (`:333`) | Useful raw inputs. Judge failure (0) and a real 0 are conflated. |
| Retrieval diagnostics | `types.rs:114-135` | `hit_rate` (`:120`), `distinct_domain_count` (`:123`), `citation_diversity_below_threshold` (`:126`) | Direct sources for the diversity and hit-rate metrics. |
| Token/cost/TTFT | `LlmResponse` fields (`stages.rs:642-652`), thrown away by `response_to_content_and_model` (`stages.rs:554-558`) | Usage is never aggregated into `ResearchMetadata` | **Must be instrumented first.** Tokens, cost, TTFT, and tool calls are unobservable today. |
| DB rows | `crates/vox-db-types/src/store_types/research.rs:90-99` (run), `:103-114` (sample) | Metrics stored as untyped JSON; `latency_p50_ms` is actually the *mean* (`eval.rs:200`) | Keep the tables. Store versioned typed blobs in `metrics`/`evidence`. |
| Report contract | `contracts/reports/research-eval/results.v1.schema.json:6-31`, `baseline.v1.json` | `additionalProperties:false`, but `eval.rs:222` emits `multi_hop_completion` | **The harness output fails its own schema.** Supersede with v2. |
| Pareto | `crates/vox-orchestrator/src/models/pareto.rs:30-68` | Dominance where `None` is *incomparable* (transitive, frontier never empty); 3 fixed axes; quality = Wilson lower bound of *reliability* (`:73-76`) | **Reuse the semantics**; generalize to N directed axes. |
| Statistics | `crates/vox-eval/src/corpus_stats.rs:46` (exact McNemar), `:60` paired compare, `:88` Holm, `:114` seeded cluster bootstrap, `:143` Wilson, `:178` min detectable difference, `:208` `evaluate_gate` | Already-tested paired stats and a gate template (infra-error hard fail first) | **Reuse directly.** `vox-eval` is L2 (`layers.toml:185`); `vox-cli-research` is L5 (`:274`) but does not depend on it yet. The new crate edge needs a user-authorized crate-edges ledger entry. |

## 2. Versioned metric schema proposal (`research-quality-metrics.v1`)

Every scorecard carries these version fields:

- `metric_schema_version`
- `evaluator_version` (semver plus scorer git SHA)
- `judge_id` (model id, prompt BLAKE3 hash via `vox-crypto`, temperature, resample k)
- `corpus_snapshot_id` (BLAKE3 hash of the canonical fixture JSON)
- `price_table_version`
- `scoring_policy_id`

Raw values are stored at all times. Normalized values exist only for weighted *views*.

**Missing data.** A metric value is one of `Value(x)`, `NotApplicable` (e.g. no gold answer; excluded from the denominator), `Missing(reason)` (instrument/judge failure; never imputed, never 0), or `Abstained`. Rule: if a metric is `Missing` on more than 10% of items, that metric is invalid for the arm. That arm cannot pass a gate on it and is marked incomparable on that Pareto axis.

**Normalization** (weighted views only). Normalize against **fixed anchors from the policy file**, never min-max over the arms in the current race; min-max makes one arm's score depend on which other arms happened to run.

- Higher-is-better (↑): `clamp((x - lo) / (hi - lo))`
- Lower-is-better (↓): `clamp((hi - x) / (hi - lo))`
- Latency and cost: log scale.
- Weights must sum to 1.00 (±1e-9), are validated at policy load, and never feed promotion.

In the table, ↑ = higher is better, ↓ = lower is better.

| id | dir | unit | definition / normalization | missing rule | hard gate (initial, calibrate on baseline) |
|---|---|---|---|---|---|
| `quality.gold_fact_recall` | ↑ | ratio | Gold atomic facts that are entailed by the answer / gold facts | `NotApplicable` without gold | — (Pareto axis) |
| `ground.claim_support_rate` | ↑ | ratio | Supported claims / `claims_extracted_count` (`types.rs:333`); capped-out claims and `Unverified` count as unsupported | `Missing` if the verifier failed | ≥ 0.70 |
| `ground.contradicted_rate` | ↓ | ratio | Contradicted claims / extracted claims | as above | ≤ 0.02 |
| `ground.no_evidence_assertions` | ↓ | count | Items with zero evidence whose answer is not an abstention (hallucination detector) | never missing | **== 0** |
| `cite.precision` | ↑ | ratio | Citations backed by a verifier span quote / citations (`pipeline.rs:1211`) | 0 citations: `NotApplicable` if abstained, else counts as 0 | ≥ 0.80 |
| `cite.recall` | ↑ | ratio | Claims with ≥1 supporting citation / claims (ALCE-style) | as above | ≥ 0.60 |
| `cite.domain_diversity` | ↑ | ratio | `min(distinct supporting domains / min_distinct_domains, 1)` (`types.rs:123`) | `NotApplicable` if abstained | — |
| `retrieval.hit_rate` | ↑ | ratio | `expected_sources` retrieved / expected (`eval.rs:17`) | `NotApplicable` without expected sources | — |
| `abstain.unanswerable_recall` | ↑ | ratio | Abstained / unanswerable fixtures (`is_adversarial`, `eval.rs:19`) | — | ≥ 0.90 |
| `abstain.over_rate` | ↓ | ratio | Abstained / answerable fixtures | — | ≤ 0.15 |
| `speed.latency_ms_p50` / `_p95` | ↓ | ms | True quantiles; cache-served items stratified out (`types.rs:323`) | `Missing` makes the axis incomparable | — (Pareto axis) |
| `speed.ttft_ms_p50` | ↓ | ms | First token of the synthesis stage | `Missing` until instrumented | — |
| `eff.tokens_total` | ↓ | tokens | Prompt + completion across all stages | as above | budget cap per policy |
| `eff.cost_usd_est` | ↓ | USD | Tokens × versioned price table; an unknown price stays `Missing`, never 0 | — | budget cap |
| `eff.tool_calls` / `eff.steps` | ↓ | count | Provider/search calls; pipeline stages and waves | — | — |
| `rel.completion_rate` | ↑ | ratio | Wilson lower bound of non-error completions | infra errors make the *run* invalid | ≥ 0.95 |
| `rel.reproducibility` | ↑ | ratio | Replicate agreement: claim-verdict Jaccard plus spread of `gold_fact_recall` | needs ≥ 2 replicates | ≥ 0.80 |
| `know.reuse_rate` | diag | ratio | Answers using cached/frozen verified findings / items | — | diagnostic only (never an objective) |

## 3. Computing groundedness and citation metrics

**Deterministic offline tiers** (default; hermetic):

- **T0 structural.** Citation index resolves (`eval.rs:249-258`). The cited span is an exact substring of the frozen source text, which is stronger than the 3-token `quote_overlaps`. Every number, date, and named entity in a claim appears in its cited span. A claim with no citation is unsupported.
- **T1 lexical proxy.** Token overlap plus negation and number guards. Labeled `proxy`; it is allowed to *fail* a gate, never to *pass* one alone.
- **T2 recorded verdicts.** Claim-level verdicts are replayed from a `RecordedJudge` keyed by `blake3(claim, span, judge_prompt_hash)`. A missing key is a test failure, never a live call.

**LLM judge** (campaign runs only):

- Use claim-level verdicts through the existing verifier taxonomy (`verifier.rs:40-45`), not the holistic 0-100 score.
- Atomic claims are extracted once per answer and frozen, so judge re-runs do not shift the denominator.

**Judge bias controls:**

1. The judge's model family differs from the synthesis family (self-preference bias).
2. The judge sees only `(claim, span)`: blind to arm id, model name, and answer length (length/verbosity bias).
3. Any pairwise comparison runs in both orders (position bias); count a win only if it is consistent.
4. Temperature 0 with k=3 resamples. Majority vote; disagreement becomes Contested (`verifier.rs:132`). Report `resample_stability` (`:99`) per arm.
5. Keep a frozen calibration set of ≥100 human-labeled `(claim, span, label)` items, including negation flips, number swaps, and topic-adjacent-but-unsupportive spans. Each `judge_id` reports accuracy and Cohen's κ. A judge change ships only if κ ≥ 0.6 and accuracy does not regress (McNemar, `corpus_stats.rs:46`).
6. Judge failure becomes `Missing(judge_error)`, not 0 (fixes `types.rs:319-321`).
7. Any judge or prompt change bumps `evaluator_version` and re-scores the frozen corpus.

## 4. Scorecard design: hard gates plus Pareto frontier

The scorecard runs in four stages:

1. **Validity.** A run is invalid if any of these hold: infra errors > 0 (mirrors `evaluate_gate`, `corpus_stats.rs:220`), a schema/evaluator/corpus snapshot mismatch against the policy, or a `Missing` rate over threshold. Invalid means "not measured", never "lost".
2. **Hard gates per arm.** Thresholds as in §2. Output `GateFailed(Vec<reason>)` with each metric value next to its threshold.
3. **Pareto frontier** among gate-passing arms. Default axes: `quality.gold_fact_recall` (or `ground.claim_support_rate` when there is no gold) ↑, `eff.cost_usd_est` ↓, `speed.latency_ms_p50` ↓, `eff.tokens_total` ↓.
   - Generalize `ParetoPoint` (`pareto.rs:21-25`) to `Vec<Axis { id, dir, value: Option<f64> }>` and keep "`None` is incomparable" (`pareto.rs:34-40`).
   - **Conservative dominance.** Compare the bootstrap CI lower bound on ↑ axes against the upper bound on ↓ axes (`bootstrap_ci`, `corpus_stats.rs:114`), so noise cannot create domination.
   - Arms that were never observed are excluded, matching `is_observed` (`pareto.rs:97`).
4. **Confirmatory comparison** vs the champion, preregistered per campaign.
   - Binary per-item outcomes use paired McNemar; continuous ones use a paired bootstrap.
   - Holm correction across the confirmatory family (`corpus_stats.rs:88`).
   - Report `min_detectable_difference(n)` (`:178`). The default corpus of 30 queries (`eval.rs:386-419`) only resolves about 25 points.
   - Exploratory metrics carry `role: exploratory` and can only *nominate* later trials.

Per-arm status: `Invalid | GateFailed | Dominated{by} | Frontier`.

- Weighted views are labeled `view:<policy_id>` and are never sorted into a "winner".
- Every scoring-policy change records `corpus_snapshot_id` and the before/after scorecards on that snapshot.

## 5. Hermetic fixture strategy

- **Corpus.** Proposed at `contracts/eval/research-quality/v1/items/*.json`. Each item holds: `query`, `answerable`, `gold_facts[]`, `expected_sources[]`, frozen `sources[] {id, url, domain, text}`, and `candidate_outputs[]` (answer, citations, claims, recorded verdicts, usage, timings).
- **Archetypes per item:** good, hallucinated-no-evidence, cited-but-unsupported, uncited-but-correct, negation-flip, fabricated-index `[9]`, number-swap, correct abstention, over-abstention, citation-spam (many citations, one supports).
- **Scoring is pure.** `score_item(&FrozenOutput, &Item, &Policy, &dyn JudgeVerdicts) -> ItemScore`, with no `VoxDb`, network, or clock. The live path (`eval.rs:58-60`) only *produces* a `FrozenOutput`. The scorer never calls the pipeline.
- **Judge.** Default tests use `RecordedJudge`. The live judge sits behind `#[ignore]` plus the `runtime` feature, like the probe at `stages.rs:839-840`.
- **DB tests** use `DbConfig::Memory` with all providers disabled (pattern: `tests/research_zero_hits_gate_test.rs:5-20`).
- **Snapshot identity.** Canonical JSON (sorted keys) hashed with BLAKE3 via `vox-crypto`. A golden test pins the hash, so any fixture edit forces a deliberate snapshot bump.
- **Determinism.** Bootstrap uses a fixed seed (`corpus_stats.rs:114` takes `seed`). Latency and cost in fixtures are recorded values, not measured ones.

## 6. Discriminating red tests (write first; R1–R9 should fail on current code)

- **R1** `hallucinated_answer_with_zero_evidence_is_not_abstention`: "Paris has 9M people [1]." with no evidence must give `abstained=false` and `no_evidence_assertions=1`. Today `eval.rs:269` returns true and `eval.rs:241-242` returns precision 1.0.
- **R2** `uncited_answer_does_not_get_perfect_citation_precision`: zero citations on a non-abstaining answer must not score 1.0 (`pipeline.rs:1245-1246`).
- **R3** `negated_claim_is_not_grounded`: answer "X does not support Y" against evidence "X supports Y" must be unsupported (`evaluation.rs:91-95` passes it).
- **R4** `short_answer_is_not_auto_grounded`: "Yes." must not score 1.0 (`evaluation.rs:79-81`).
- **R5** `missing_gold_is_not_imputed`: the item is excluded from `gold_fact_recall`, not set to 0.5 (`eval.rs:93`) or 1.0 (`evaluation.rs:56-58`).
- **R6** `judge_error_is_missing_not_zero` (`types.rs:319-321`); `averages_skip_missing` (`eval.rs:176,319` use `unwrap_or(0.0)`).
- **R7** `eval_report_validates_against_schema` (`eval.rs:217-224` vs `results.v1.schema.json:22`).
- **R8** `p50_is_median_not_mean` (`eval.rs:200`). `multi_hop_not_triggered_by_substring`: "authentication" contains "then" (`eval.rs:278`).
- **R9** `fabricated_citation_index_fails_cite_precision`: `[9]` with 2 sources.
- **New-code guards:**
  - `weights_must_sum_to_one`
  - `adding_dominated_arm_does_not_change_other_scores` (no min-max normalization)
  - `missing_axis_is_incomparable_not_worst` (port the `pareto.rs` tests)
  - `always_abstain_arm_fails_over_abstention_gate`
  - `citation_spam_lowers_precision`
  - `policy_change_without_snapshot_id_is_rejected`
  - `recorded_judge_missing_key_fails_not_calls_network`
  - `legacy_quality_score_never_read_by_promotion` (grep/type guard)
- **Mutation-verify** R1/R2 per AGENTS.md: break the guard, confirm the test goes red, then restore.

## 7. Pitfalls

- **Abstention gaming.** "Always abstain" maxes precision. Gates on over-abstention and gold-fact recall close this.
- **Denominator gaming.** Fewer extracted claims means a higher support rate. Freeze claim extraction and pair it with gold-fact recall.
- **Verification cap.** Capped-out claims (`types.rs:334-344`) must count as unverified, not drop out of the denominator.
- **Cache contamination.** Cache hits (`types.rs:323`) make latency look fast. Stratify them, or freeze per arm.
- **Judge drift.** The same name can hide different weights (OpenRouter fallbacks; `stages.rs:633-640` shows the winning model can differ). Record the actual model per call.
- **Opaque composites creeping back in.** The holistic judge (`stages.rs:60-74`) and the `avg_quality` printout (`eval.rs:174-178`, `:448-456`) must be visibly labeled legacy.
- **Live web non-determinism.** Confirmatory comparisons replay from frozen source snapshots only.
- **Underpowered comparisons.** At n=30, compare only differences of ≥25 points, or grow the corpus first.
- **Missing instrumentation.** Tokens, cost, TTFT, and tool calls are discarded today (`stages.rs:554-558`). Plumb usage through `ResearchMetadata` before declaring efficiency axes.
- **Cost math.** An unknown price must stay `Missing`; `0.0` would become an unbeatable Pareto minimum (cf. `pareto.rs:78`).
- **Layering.** Pareto lives in `vox-orchestrator` (L3) and the stats in `vox-eval` (L2). Place the scorer where both are downward deps, and propose (do not self-author) any new crate-edge exception.

## Summary

1. Reusable now: the claim verifier and citation audit (`verifier.rs`, `pipeline.rs:1211`), `pareto.rs` dominance semantics, and `vox-eval` McNemar/Holm/bootstrap/gate.
2. Broken today: hallucinated answers with no evidence score as abstentions with precision 1.0; uncited answers get precision 1.0; groundedness is negation-blind; the harness output violates its own schema.
3. Proposed `research-quality-metrics.v1`: 18 directed metrics, explicit NotApplicable/Missing/Abstained values, fixed-anchor normalization, every version id recorded.
4. Scorecard: validity, then hard gates, then a conservative CI-based N-axis Pareto frontier, then Holm-corrected confirmatory tests. Weighted views are display-only.
5. Default tests stay hermetic by scoring frozen archetype outputs with a recorded judge. Tokens, cost, and TTFT have to be plumbed through before the efficiency axes can be measured.
