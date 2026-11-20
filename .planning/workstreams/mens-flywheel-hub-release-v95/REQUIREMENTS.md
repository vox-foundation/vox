# Requirements: Vox — Milestone v9.5 MENS Flywheel, Truthful Lanes & Trustworthy Hub Releases

**Defined:** 2026-11-20
**Workstream:** `mens-flywheel-hub-release-v95`
**Core Value:** Every claim about a MENS hub or spoke — in a report, a model card, or the Space — is backed by a
reproducible, paired before/after measurement, and nothing reaches the Hugging Face Hub without it.

**Sources:** `docs/src/architecture/mens-eval-flywheel-and-lanes-claude-handoff-2026-11-20.md`, `research/SUMMARY.md`
(which records where the handoff was refuted by the working tree).

**Owner decisions (2026-11-20):** HF namespace stays personal; the weightless 27B repo is kept with a first-line
disclosure; Space scope is the honest Space **plus** merged-ONNX in-browser inference for 0.8B; paid teacher
distillation is capped at 12 USD after a 0-USD local harvest.

**Execution model:** planning, plan-checking and verification agents run on Opus; executor agents run on
Sonnet 5.5. Roadmap phases must name their file-disjoint parallel groups so multiple executors can run at once.
GPU work is one job at a time and never starts while `PID 34157` is alive.

## v1 Requirements

### Baseline hygiene (BASE)

- [ ] **BASE-01**: The pending v9.4 working tree (Phases 547–550, ~157 paths) is reviewed and landed in path-scoped commits on a branch, giving every later comparison a clean base SHA.
- [ ] **BASE-02**: A machine-wide GPU serialization lock prevents two Metal jobs from overlapping, and every eval/train report header records whether another GPU job was running.
- [ ] **BASE-03**: Four JSON contracts are frozen with schema tests before parallel work starts: plugin generate-response fields, `exec_error_kind` enum, run-record keys, failure-funnel schema.
- [ ] **BASE-04**: A frozen proof manifest pins the benchmark files, verifier revision and decode protocol, and all baselines used for before/after claims are re-scored against it.

### Eval timing and speed (TIME)

- [ ] **TIME-01**: The Metal and CUDA inference plugins return `completion_tokens`, `prefill_ms`, `decode_ms`, `total_ms`, token rates and `stop_reason` for every generation, as optional fields that leave request shape and item-checkpoint keys unchanged.
- [ ] **TIME-02**: `eval-local`, `eval-reasoning`, `eval-reasoning-compare` and `eval-spoke` reports carry `model_load_ms`, generation/verify/total wall time, `cached_items` vs `generated_items`, latency p50/p95, mean decode rate and truncation rate.
- [ ] **TIME-03**: A cycle report combines training and eval records into `cycle_wall_secs`, `eval_share_of_cycle` and pass-gain per GPU-hour.
- [ ] **TIME-04**: System-prompt prefix KV reuse (attention KV plus recurrent state, deep-copied snapshot keyed on token ids) is implemented and proven token-identical to uncached greedy decode on all 166 + 64 items; if identity fails it ships only behind a new decode-protocol tag.
- [ ] **TIME-05**: A separate fast sequential gate policy file and a `--policy` option give an early accept/reject inner loop without altering the release ladder policy or invalidating existing gate receipts.
- [ ] **TIME-06**: Existing `.ckpt.jsonl` item caches still resolve after all timing changes, demonstrated by a test using a pre-change checkpoint fixture.

### Failure funnel and training health (FUNL)

- [ ] **FUNL-01**: Every `eval-local` report includes a failure funnel (no code, parse fail, typecheck fail, anti-stub fail, step/string limit, assertion fail, panic, pass) whose buckets sum to n, with truncation reported separately.
- [ ] **FUNL-02**: Runtime failure kinds (`StepLimitExceeded`, `AssertionFailed`, `Panic`) are distinguished in per-item results instead of collapsing to a bare fail.
- [ ] **FUNL-03**: Reports include per-category counts and decode-failure modes (unclosed think, mean completion/prompt tokens) without significance claims on categories too small to support them.
- [ ] **FUNL-04**: Training records carry non-null `truncated_rows`, `prompt_trimmed_rows`, last-epoch validation loss, pre-clip gradient norm and clip fraction, so an unstable recipe is visible within the first steps.
- [ ] **FUNL-05**: `eval-local --repair-turns N` feeds compiler diagnostics back and reports pass@1, pass after 1 and 2 repairs, and tokens and time to first valid Vox.

### Truthful naming: backend / spoke / data slice (LANE)

- [ ] **LANE-01**: A committed inventory lists every use of "lane" in code, config and docs, classified as rename or keep (serving lane, router triggers and orchestrator lanes are kept).
- [ ] **LANE-02**: The compute-backend field is named `backend` with a read alias for `lane`, and old memory-model and run manifests still load, proven by real old-file fixtures.
- [ ] **LANE-03**: Corpus rows carry `data_slice` (`<domain>.<kind>`) and `verification_tier`; legacy rows map to `legacy.*` with tier `unknown` rather than being guessed.
- [ ] **LANE-04**: Each trained spoke has exactly one canonical mix with required (`optional: false`) inputs; `verified_vox_codegen_v2.jsonl` is wired into the vox-lang mix; duplicate/dead mixes and "Lane G" aliases are removed; non-QLoRA profiles declare `mode: retrieval_only` or `eval_only`.
- [ ] **LANE-05**: Training writes `spoke`, `backend`, `base_preset`, `data_slice_histogram` and `verification_histogram` into `training_manifest.json` and `train-record.json`, including under `--fast-corpus`.
- [ ] **LANE-06**: Hub cards derive the spoke from the run record; the hardcoded `default_spoke` is removed, and a run with no spoke is `unknown` and cannot be released.
- [ ] **LANE-07**: `mens-training-data-contract.md` and related reference docs describe the mixes as they actually are, checked by the doc lint.

### Spoke evaluation (SPOK)

- [ ] **SPOK-01**: `eval-spoke` runs the 72-episode agent suite against a live local model (base and candidate as two sequential passes) and writes a paired report with timing.
- [ ] **SPOK-02**: The spoke report separates tool-selection accuracy, argument-schema validity, tool-call format compliance and recovery after a tool error.
- [ ] **SPOK-03**: Mono `harness-tools` and decomposed `tool-selection` + `argument-generation` are compared on the same suite, and the losing configuration is retired from config.
- [ ] **SPOK-04**: Spoke gate rows are populated and enforced for any spoke marked trained, so a spoke cannot be published on the hub's evidence alone.
- [ ] **SPOK-05**: Spoke base pointers in `domain-profiles.yaml` are updated only for a base whose spoke gate passed; unevaluated candidates stay listed as candidates.

### Before/after proof (PROOF)

- [ ] **PROOF-01**: A blind v3 slice of 40–50 tasks is authored and sealed (its hash committed, its items never used in generation prompts or training) before any new training data is produced.
- [ ] **PROOF-02**: The primary comparison and its statistic are declared in a committed file before the runs: paired exact McNemar on `heldout_bench_v2`, with Wilson or bootstrap intervals and a stated minimum detectable difference.
- [ ] **PROOF-03**: A Tier-1 canary cycle (train + eval) completes in 45 minutes or less on the chosen canary size, and the 0.8B-vs-2B canary choice is settled by a measured comparison.
- [ ] **PROOF-04**: A 30-minute `seq=2048` preflight on `Qwen3.8-27B` records peak memory, finite initial loss and gradient norm before the full run is allowed to start.
- [ ] **PROOF-05**: CAP-27B-04 runs to completion and is scored before/after on all 166 `heldout_bench_v2` tasks and the blind v3 slice, with funnel, timing and collateral results.
- [ ] **PROOF-06**: One generated before/after report per hub rung and per trained spoke states the verdict (Improved / Neutral / Regressed), the paired counts, the interval and the cost in GPU-hours, and never compares a step-capped run with a full one.
- [ ] **PROOF-07**: `heldout_bench_v2` gate floors are calibrated per model size from measured data in their own reviewed change.

### Ladder retrain order (LADR)

Added 2026-11-20 on owner feedback: improved releases of every size need retraining with the new recipe and
corpus, so the ladder goes smallest first and the 27B last.

- [ ] **LADR-01**: Every existing published adapter (0.8B, 2B, 4B, 9B) and its base are scored on `heldout_bench_v2` under the frozen proof manifest, eval-only, giving each size a "before" that is comparable with its "after".
- [ ] **LADR-02**: The ladder is retrained with the new recipe and canonical mix in ascending size order (0.8B, 2B, 4B, 9B, then 27B); a size starts only after the size below it is classified non-regressed, and the 27B run is last.
- [ ] **LADR-03**: Measured train and eval wall time per size is recorded as each rung completes, and the remaining-queue estimate is updated from those measurements rather than from guesses.
- [ ] **LADR-04**: A recipe failure on a small rung (collapse, regression, memory spill) stops the queue and is recorded with its funnel and training-health evidence before any larger rung is attempted.

### Discriminating, affordable evals (DISC)

Added 2026-11-20 on owner feedback: metrics must tell the models apart, at a cost that keeps the loop fast, and
each kind of model needs the kind of eval that fits it.

- [ ] **DISC-01**: An item-discrimination report over the existing ladder completions identifies which `heldout_bench_v2` items separate adjacent sizes and which carry no signal (all pass or all fail), and states for each adjacent pair whether the benchmark can distinguish them at the declared power.
- [ ] **DISC-02**: Where an adjacent pair cannot be distinguished, the benchmark is extended with verified tasks at that difficulty band (never by reusing blind-slice or training items) until the pair separates or the report says plainly that it does not.
- [ ] **DISC-03**: A fixed anchor subset chosen for discrimination gives a fast ladder check whose ranking agrees with the full benchmark, and its agreement is measured on runs not used to choose it.
- [ ] **DISC-04**: An eval matrix names, for each hub size and each spoke or auxiliary model, the eval type that fits it, its dataset, its n and its detectable difference: execution-verified codegen for vox-lang; live episodes with selection, argument and recovery scores for harness tools; skill-trigger precision/recall for harness skills; compile-and-test for rust-authoring; classification precision/recall for the guard/judge model; retrieval recall and rank quality for the embedder.
- [ ] **DISC-05**: Each eval in the matrix has a wall-time budget per model size, met through caching, sequential early-stop and the anchor subset, and the report flags any eval that exceeds its budget.
- [ ] **DISC-06**: Ladder reports show all sizes on one table with paired intervals, so a visitor or a card can see the measured difference between sizes and where it is not significant.

### Training data (DATA)

- [ ] **DATA-01**: A code-level firewall rejects any training row that overlaps the v1, v2 or blind v3 benchmarks (exact, n-gram and near-duplicate), and the generation prompts cannot read benchmark items.
- [ ] **DATA-02**: A 0-USD local harvest adds verified rows from fixed workspace `.vox` files, multi-construct files and compiler repair traces.
- [ ] **DATA-03**: Paid teacher distillation is hard-capped at 12 USD, runs only through the LLM facade with MIT/Apache-licensed teachers, records the resolved teacher model id, cost and slice per row, and produces a spend ledger report.
- [ ] **DATA-04**: Every added row passes execution verification and anti-stub checks; failed-then-fixed attempts are kept as repair-trace rows; a synthetic-share ceiling with human-written anchors is enforced by the mix.

### Hugging Face releases (HUB)

- [ ] **HUB-01**: Each adapter is classified Improved / Neutral / Regressed from its paired evidence; only Improved moves `main` and receives a `v<N>` tag, and the release path has no ungated bypass.
- [ ] **HUB-02**: A release is refused unless the evidence files hash-match the weights being uploaded and the adapter is non-regressed against the previous tag.
- [ ] **HUB-03**: `vox-mens-index` publishes a release map (tag → full commit SHA → benchmark hash → headline metrics → verdict), including Neutral and Regressed attempts.
- [ ] **HUB-04**: Each repo has one card file and one copy of each evidence file; the cleanup lands as one atomic commit with a file-list contract test, after confirming nothing in `hub pull` or the Space reads the removed paths.
- [ ] **HUB-05**: Weights, card and evidence for a release land in one commit, and a real `hub pull` by commit SHA succeeds in a clean directory with a load-and-generate smoke test.
- [ ] **HUB-06**: Known sync defects are fixed: the wrong train-record filename, dead Space templates, Space upload failure reported as success, and missing attribution/NOTICE for MIT-with-attribution bases and distilled data.
- [ ] **HUB-07**: The 27B repo and any other weightless repo state on the first line of the card that no weights are released yet and why.

### Model cards (CARD)

- [ ] **CARD-01**: Every number on a card is rendered from an uploaded evidence file and names the benchmark and version that produced it; a test fails if a card states a number with no evidence source.
- [ ] **CARD-02**: Cards lead with why the model exists as a measured gap against its base, then what changed since the previous release and why, in plain language.
- [ ] **CARD-03**: Cards include a counts-based "where it still fails" table from the failure funnel alongside the existing verified-wins examples.
- [ ] **CARD-04**: Cards include a generated release history and a "how to pin this exact version" section using the commit SHA.
- [ ] **CARD-05**: Shared explanation lives once in `mens-models.md` and is linked, not repeated; a lint fails when card text duplicates the reference page or another card beyond a set threshold.
- [ ] **CARD-06**: Superseded results (the saturated v1 compile-only benchmark) are labelled as superseded wherever they still appear, and `mens-models.md` no longer claims the HF router serves MENS checkpoints.

### Space (SPACE)

- [ ] **SPACE-01**: The Space headlines `heldout_bench_v2` evidence loaded from a pinned commit SHA rather than `main`, and labels the v1 benchmark as superseded.
- [ ] **SPACE-02**: Every output shown carries a visible source label (recorded, live in-browser, live local serve, live base model), and recorded output is never presented as live.
- [ ] **SPACE-03**: A merged-ONNX export of the released 0.8B adapter runs in the browser on WebGPU, preceded by a feasibility spike; the download is click-to-start with its size shown, and unsupported devices get an explicit message.
- [ ] **SPACE-04**: The base-model demo and local-serve mode work from the real `*.hf.space` origin, with no token stored in or required by the static page.
- [ ] **SPACE-05**: A contract test checks that every URL and filename the Space fetches exists, and a Playwright spec against the live Space proves a real generation from a nonce prompt, reporting SKIP (not pass) when WebGPU is unavailable.
- [ ] **SPACE-06**: After each sync, a live check confirms the Space is `RUNNING` at the pushed commit and writes a proof receipt that the release step requires.

## v2 Requirements

Deferred; tracked but not in this roadmap.

### Later

- **ARCH-01**: Granite-4.2 aLoRA architecture port for shared-base multi-spoke switching.
- **ARCH-02**: MiMo-V2.6-Distill-9B, FrogNano-4B and Qwen3Guard spokes trained and released (v9.5 only evaluates them as candidates).
- **ARCH-03**: `Qwen3-Embedding-0.6B` replaces the hash-embedding fallback in `vox-search`.
- **DATA-05**: Preference/DPO pairs and harness-episode imputation (handoff stages 3–5, remaining budget).
- **SPACE-07**: In-browser inference for 2B and 4B.
- **HUB-08**: Move releases to a `vox-foundation` Hugging Face organization.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Sparse MoE support (Qwen3.6-35B-A3B, 80B-class) | No measured quality gain over Qwen3.8-27B; removed in v9.4 |
| Non-commercial or restricted-licence bases (xLAM-2, Hammer2.1, Nemotron-Orchestrator, Plano, Devstral-2) | Cannot be redistributed |
| Publishing `heldout_bench_v2` or blind v3 task items | Contamination of the benchmark |
| Paid always-on inference Space | Ongoing cost; owner chose in-browser 0.8B instead |
| Editing the release ladder gate policy in place | Would invalidate every existing gate receipt |
| Renaming serving, router-trigger or orchestrator "lane" | Different concepts that are named correctly |
| Paid teacher spend above 12 USD | Owner cap for this milestone |

## Traceability

Filled by the roadmapper on 2026-11-20 and revised the same day after owner feedback on ladder order and eval
discrimination (see `ROADMAP.md`).

| Requirement | Phase | Status |
|-------------|-------|--------|
| BASE-01 | Phase 553 | Pending |
| BASE-02 | Phase 553 | Pending |
| BASE-03 | Phase 553 | Pending |
| BASE-04 | Phase 553 | Pending |
| TIME-01 | Phase 554 | Pending |
| TIME-02 | Phase 554 | Pending |
| TIME-03 | Phase 554 | Pending |
| TIME-04 | Phase 559 | Pending |
| TIME-05 | Phase 554 | Pending |
| TIME-06 | Phase 554 | Pending |
| FUNL-01 | Phase 554 | Pending |
| FUNL-02 | Phase 554 | Pending |
| FUNL-03 | Phase 554 | Pending |
| FUNL-04 | Phase 555 | Pending |
| FUNL-05 | Phase 554 | Pending |
| LANE-01 | Phase 555 | Pending |
| LANE-02 | Phase 555 | Pending |
| LANE-03 | Phase 555 | Pending |
| LANE-04 | Phase 555 | Pending |
| LANE-05 | Phase 555 | Pending |
| LANE-06 | Phase 561 | Pending |
| LANE-07 | Phase 555 | Pending |
| SPOK-01 | Phase 560 | Pending |
| SPOK-02 | Phase 560 | Pending |
| SPOK-03 | Phase 560 | Pending |
| SPOK-04 | Phase 560 | Pending |
| SPOK-05 | Phase 560 | Pending |
| PROOF-01 | Phase 557 | Pending |
| PROOF-02 | Phase 557 | Pending |
| PROOF-03 | Phase 562 | Pending |
| PROOF-04 | Phase 563 | Pending |
| PROOF-05 | Phase 563 | Pending |
| PROOF-06 | Phase 564 | Pending |
| PROOF-07 | Phase 564 | Pending |
| LADR-01 | Phase 559 | Pending |
| LADR-02 | Phase 562 | Pending |
| LADR-03 | Phase 562 | Pending |
| LADR-04 | Phase 562 | Pending |
| DISC-01 | Phase 559 | Pending |
| DISC-02 | Phase 559 | Pending |
| DISC-03 | Phase 559 | Pending |
| DISC-04 | Phase 559 | Pending |
| DISC-05 | Phase 559 | Pending |
| DISC-06 | Phase 564 | Pending |
| DATA-01 | Phase 557 | Pending |
| DATA-02 | Phase 558 | Pending |
| DATA-03 | Phase 558 | Pending |
| DATA-04 | Phase 558 | Pending |
| HUB-01 | Phase 564 | Pending |
| HUB-02 | Phase 564 | Pending |
| HUB-03 | Phase 564 | Pending |
| HUB-04 | Phase 556 | Pending |
| HUB-05 | Phase 564 | Pending |
| HUB-06 | Phase 556 | Pending |
| HUB-07 | Phase 556 | Pending |
| CARD-01 | Phase 561 | Pending |
| CARD-02 | Phase 561 | Pending |
| CARD-03 | Phase 561 | Pending |
| CARD-04 | Phase 561 | Pending |
| CARD-05 | Phase 561 | Pending |
| CARD-06 | Phase 556 | Pending |
| SPACE-01 | Phase 556 | Pending |
| SPACE-02 | Phase 556 | Pending |
| SPACE-03 | Phase 561 | Pending |
| SPACE-04 | Phase 561 | Pending |
| SPACE-05 | Phase 556 | Pending |
| SPACE-06 | Phase 561 | Pending |

**Coverage:**
- v1 requirements: 67 total
- Mapped to phases: 67
- Unmapped: 0

---
*Requirements defined: 2026-11-20*
*Last updated: 2026-11-20 after roadmap revision (LADR and DISC mapped, phases 553–564)*
