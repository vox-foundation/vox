# Roadmap: Vox

**Workstream:** `.planning/workstreams/mens-flywheel-hub-release-v95/`
**Milestone:** v9.5 — MENS Flywheel, Truthful Lanes & Trustworthy Hub Releases
**Requirements:** [`REQUIREMENTS.md`](REQUIREMENTS.md) (`67` v1 requirements, all mapped)
**Research:** [`research/SUMMARY.md`](research/SUMMARY.md), [`research/ARCHITECTURE.md`](research/ARCHITECTURE.md) (file ownership, build order), [`research/PITFALLS.md`](research/PITFALLS.md) (ordering constraints)
**Handoff:** [`mens-eval-flywheel-and-lanes-claude-handoff-2026-11-20.md`](../../../docs/src/architecture/mens-eval-flywheel-and-lanes-claude-handoff-2026-11-20.md)
**Numbering:** Phases `553–564`. Numbers `551–552` are retired (removed from the previous workstream) and are not reused.
**Execution model:** plan, plan-check and verify on Opus; execute on Sonnet 5.5. Each phase names file-disjoint parallel groups, one executor per group.

## Overview

The previous milestone left a 27B recipe waiting on `PID 34157`, a verified corpus that no mix reads, cards that cannot tell
the saturated v1 benchmark from `heldout_bench_v2`, and a Space whose WebGPU path fetches files that do not exist. An
improved release of any size needs the new recipe and corpus, so every size is retrained: smallest first, 27B last.

This milestone lands the pending tree and freezes the shared contracts first. It then instruments the loop (timing, failure
funnel, run stamps), fixes what the public sees today, and seals the proof protocol before any new data is made. Before
any size is retrained, it scores every published adapter eval-only and checks that the benchmark can tell adjacent sizes
apart. Today it sometimes cannot: on the 46 runnable v1 items the 4B and 9B adapters pass 22 vs 25 strict and 33 vs 32
signature-rescued, and the 0.8B and 2B bases both pass none. The ladder then retrains 0.8B → 2B → 4B → 9B in one strictly
serial GPU queue, each rung gated on the one below. The 27B run comes last, and only what paired evidence calls Improved is
released.

## Hard Rules (apply to every phase)

- **One GPU job at a time**, under the Phase 553 lock. No GPU job starts while `PID 34157` is alive. Never rebuild
  `target/release` or reinstall the Metal plugin dylib while it runs; use isolated target dirs.
- **Ladder order:** retraining goes 0.8B → 2B → 4B → 9B → 27B. A size starts only after the size below it is classified
  non-regressed, and the 27B preflight and CAP-27B-04 are the milestone's last training job.
- **No guessed durations:** train and eval times for retrained sizes are measured as each rung completes, never assumed.
- **Nothing cached is invalidated:** no new keys in request JSON, no change to checkpoint header inputs, new metric fields
  are optional (absent means unknown, never zero), and `eval-gates-ladder.yaml` is never edited in place.
- **One owner per file per wave.** Groups below are file-disjoint; a phase's groups may run at once.
- **Lean local verification:** `cargo check -p <crate>` plus the new tests, through the build broker; hosted CI does the rest.
  Security-relevant guards are proven by mutation (break the guard, watch the test fail).

## Execution Waves

| Wave | Phases (run concurrently) | Each phase starts after | GPU in this wave |
|------|---------------------------|-------------------------|------------------|
| W0 | 553 | — | none |
| W1 | 554 · 555 · 556 · 557 | 553 | none, except 556's one Spike B WebGPU smoke run (short, lock-held, after `PID 34157`) |
| W2 | 558 · 559 · 560 · 561 | 558: 555 + 557 · 559: 554 + 557 · 560: 554 + 555 + 557 · 561: 554 + 555 + 556 | 559's parity, "before" and extension evals (short to medium), then 560's live episodes (short); 561's browser checks in gaps |
| W3 | 562, with 564's code groups (and 561 if still open) | 562: 558 + 559 · 564 code: 557 + 559 + 560 + 561 | 562's canary, then rungs 0.8B → 2B → 4B → 9B (each measured as it runs) |
| W4 | 563, with 564 still open | 562 (9B rung non-regressed) | long, 563 holds the lock; the last training job |
| W5 | 564 closes | 563 | short (27B release smoke test) |

| Phase | Depends on | Can run concurrently with | GPU |
|-------|------------|---------------------------|-----|
| 553 | — | nothing (gates all) | none |
| 554 | 553 | 555, 556, 557 | none |
| 555 | 553 | 554, 556, 557 | none |
| 556 | 553 | 554, 555, 557 | short (Spike B only) |
| 557 | 553 | 554, 555, 556 | none |
| 558 | 555, 557 | 559, 560, 561 | none |
| 559 | 554, 557 | 558, 560, 561 | short to medium (parity, "before" evals, extension evals) |
| 560 | 554, 555, 557 | 558, 559, 561 | short (live episodes) |
| 561 | 554, 555, 556 | 558, 559, 560, 562 | short (browser checks, in gaps between training jobs) |
| 562 | 558, 559 | 561, 564 (code) | canary, then four full rungs, strictly serial; durations measured |
| 563 | 562 | 564 (code) | long |
| 564 | 557, 559, 560, 561 (code); 562, 563 (releases) | 562, 563 | short (`hub pull` smoke tests) |

**GPU queue (strictly serial, one lock, nothing while `PID 34157` is alive):** `PID 34157` exits → 556 Spike B WebGPU
smoke run → 559 Metal prefix-cache parity → 559 "before" evals (published 0.8B, 2B, 4B, 9B adapters and their bases;
stored completions reused) → 559 extension-slice evals for any pair the benchmark cannot separate → 560 live agent
episodes → 562 Tier-1 canary (0.8B, 2B) → 562 rung 0.8B (train, anchor check, full eval) → rung 2B → rung 4B → rung 9B →
563 27B "before" evals if not already stored → 563 27B preflight (30 min) → 563 CAP-27B-04 + evals → 564 27B `hub pull`
smoke test.

No duration is assumed for the 0.8B, 2B, 4B or 9B rungs under the new recipe. Each rung's measured train and eval time is
written to `contracts/reports/mens-v95-ladder-queue.json` as it completes, and the remaining-queue estimate is recomputed
from those measurements. That includes the 27B run, whose handoff estimate (about 30 h of training) is replaced as data
arrives. Short lock-held jobs (561's browser WebGPU and local-serve checks, 564's `hub pull` smoke tests for 0.8B–9B
releases) fit between rungs or before 563's preflight, never during a training run.

**Critical path:** 553 → 554 or 557 → 559 → 562 → 563 → 564, or through 555 → 558 when the final mix lands after 559.

## Phases

- [ ] **Phase 553: Clean Base, GPU Lock, Frozen Contracts & Proof Manifest** (`BASE-01..04`) — land the pending tree for a clean base SHA, serialize the GPU, freeze four contracts and the proof manifest
- [ ] **Phase 554: Eval Timing, Failure Funnel & Prefix-Cache CPU Spike** (`TIME-01..03`, `TIME-05..06`, `FUNL-01..03`, `FUNL-05`) — every eval report shows where time went and where completions fail; cache design settled on CPU
- [ ] **Phase 555: Truthful Lanes & Run Stamping** (`LANE-01..05`, `LANE-07`, `FUNL-04`) — "lane" means one thing per place, one canonical vox-lang mix, every run records spoke/backend/slices/health
- [ ] **Phase 556: Honest Hub & Space Hygiene** (`HUB-04`, `HUB-06`, `HUB-07`, `CARD-06`, `SPACE-01`, `SPACE-02`, `SPACE-05`) — public Space, cards and repos stop misleading now, with no new weights; Space feasibility spike
- [ ] **Phase 557: Sealed Blind Slice, Pre-Registered Statistic & Contamination Firewall** (`PROOF-01`, `PROOF-02`, `DATA-01`) — the proof cannot be gamed before any new data exists
- [ ] **Phase 558: Verified Data Harvest & Capped Distillation** (`DATA-02..04`) — 0-USD harvest, then at most 12 USD of licensed distillation, every row verified and attributed
- [ ] **Phase 559: Cached Decode, Discriminating Evals & Ladder Baselines** (`TIME-04`, `LADR-01`, `DISC-01..05`) — before any retrain: every published size has a comparable "before", the benchmark is shown to separate adjacent sizes, every model has a budgeted eval that fits it
- [ ] **Phase 560: Live Spoke Gates** (`SPOK-01..05`) — every trained spoke is gated on live agent episodes, not hub evidence
- [ ] **Phase 561: Evidence-Bound Cards & In-Browser 0.8B Space** (`LANE-06`, `CARD-01..05`, `SPACE-03`, `SPACE-04`, `SPACE-06`) — cards and Space say only what evidence supports; 0.8B runs live in the browser
- [ ] **Phase 562: Tier-1 Canary & Ladder Retrain 0.8B → 9B** (`PROOF-03`, `LADR-02..04`) — recipe validated in a 45-minute canary, then the small and mid sizes retrained in ascending order, each gated on the one below
- [ ] **Phase 563: Qwen3.8-27B Preflight & CAP-27B-04 Full Run** (`PROOF-04`, `PROOF-05`) — the last and longest training job, alone on the GPU, while GPU-free work continues
- [ ] **Phase 564: Before/After Ladder Proof, Calibrated Floors & Gated Releases** (`PROOF-06`, `PROOF-07`, `DISC-06`, `HUB-01`, `HUB-02`, `HUB-03`, `HUB-05`) — only Improved adapters reach the Hub, hash-bound and pullable by SHA, with all sizes on one table

## Phase Details

### Phase 553: Clean Base, GPU Lock, Frozen Contracts & Proof Manifest
**Goal**: Every later comparison starts from one reviewed base SHA, only one GPU job can run at a time, and the JSON shapes and proof inputs that parallel agents share can no longer move.
**Depends on**: Nothing (first phase)
**Requirements**: BASE-01, BASE-02, BASE-03, BASE-04
**Success Criteria** (what must be TRUE):
  1. The pending Phase 547–550 tree (~157 paths) is landed on a branch in path-scoped commits (no `git add -A`); afterwards `git status` shows none of those paths, and the resulting base SHA is the one recorded in the proof manifest.
  2. While the GPU lock is held, a second GPU job waits or refuses instead of starting (shown with stand-in jobs, no Metal use), a test fails when the lock check is removed, and every eval and train report header records whether another GPU job was running.
  3. Schema tests for the four frozen contracts (plugin generate-response fields, `exec_error_kind` enum, run-record keys, failure-funnel schema) pass, and fail when any field is renamed or dropped.
  4. A frozen proof manifest pins the benchmark files, verifier revision and decode protocol, and `contracts/reports/mens-v95-baselines-rescored.json` re-scores on CPU every stored baseline used for a later claim against it, listing each verdict that changed from its original report.
**Parallel groups:** Serial first: BASE-01 (one executor owns the whole pending tree, and pre-registers the `mod` lines for this milestone's new modules — `eval_timing`, `eval_funnel`, `eval_spoke_live`, `proof_stats`, `eval_discrimination`, `ladder_queue`, `proof_report` — in `crates/vox-ml-cli/src/commands/mens/mod.rs`). Then at once: (a) GPU lock — new lock helper plus the header field in the eval/train report writers; (b) contracts — new schema files and schema tests under `contracts/mens/`; (c) proof manifest — manifest file under `mens/config/` plus a CPU re-score driver. The phase runs alone: it gates every other phase.
**GPU:** none (`PID 34157` is not touched; the lock is tested with stand-in jobs).
**Agents:** plan + verify on Opus; execute on Sonnet 5.5.
**Plans**: TBD

### Phase 554: Eval Timing, Failure Funnel & Prefix-Cache CPU Spike
**Goal**: Every eval report shows where the time went and where completions fail, without invalidating any cached item or gate receipt, and the prefix-cache design is settled on CPU before any Metal work.
**Depends on**: Phase 553
**Requirements**: TIME-01, TIME-02, TIME-03, TIME-05, TIME-06, FUNL-01, FUNL-02, FUNL-03, FUNL-05
**Success Criteria** (what must be TRUE):
  1. The Metal and CUDA plugins return `completion_tokens`, `prefill_ms`, `decode_ms`, `total_ms`, token rates and `stop_reason` as optional fields, and `eval-local`, `eval-reasoning`, `eval-reasoning-compare` and `eval-spoke` reports carry `model_load_ms`, generation/verify/total wall time, `cached_items` vs `generated_items`, latency p50/p95, mean decode rate and truncation rate; a response from an old plugin yields "timing unavailable", never zeros.
  2. Nothing cached is invalidated: a pre-change `.ckpt.jsonl` fixture still resolves every item, `eval-gates-ladder.yaml` and its `policy_blake3` are byte-identical, and `--policy` with the new fast sequential policy file gives an early accept/reject while every existing `gate_receipt.json` still validates.
  3. Every `eval-local` report includes a failure funnel whose buckets sum to n (asserted in code), with truncation reported separately, `StepLimitExceeded` / `AssertionFailed` / `Panic` distinguished per item, and per-category counts with no significance claim on small categories; `contracts/reports/mens-v95-funnel-backfill.json` applies it to the stored canary completions from the previous milestone (re-verified on CPU) and shows where they fail.
  4. `eval-local --repair-turns 2` reports pass@1, pass after 1 and 2 repairs, and tokens and time to first valid Vox; the cycle report computes `cycle_wall_secs`, `eval_share_of_cycle` and pass gain per GPU-hour from a train record plus an eval report (fixture-tested here; the first real one comes from Phase 562's canary).
  5. Spike A writes `contracts/reports/mens-v95-prefix-cache-cpu-spike.json`: on the 0.8B config on CPU, the cold-vs-cold numeric noise floor, whether split prefill from a deep-copied snapshot (attention KV plus recurrent state, keyed on token ids) is token-identical to one-pass prefill, and the resulting decision (same decode protocol, or a new protocol tag).
**Parallel groups:** (P1) plugin timing — `crates/vox-plugin-mens-candle-metal/src/{inference,batch_decode}.rs`, `crates/vox-plugin-mens-candle-cuda/src/inference.rs` (one owner; CUDA verified by `cargo check` only); (P2) host timing + repair turns — new `eval_timing.rs`, `eval_local.rs`, `reasoning_eval.rs`, `reasoning_compare.rs` and `reasoning_compare/`, `item_checkpoint.rs` tests, plus the one-line funnel hook in `eval_local.rs`; (P3) funnel — `metrics.rs`, new `eval_funnel.rs`; (P4) fast policy — `eval_collateral.rs`, new `mens/config/eval-gates-ladder-fast.yaml`; (Spike A) a new test module and the spike report only, no product code. Paths are under `crates/vox-ml-cli/src/commands/mens/` unless noted. Runs in parallel with 555, 556 and 557.
**GPU:** none (fake-`infer` and CPU tiny-model tests only).
**Agents:** plan + verify on Opus; execute on Sonnet 5.5.
**Plans**: TBD

### Phase 555: Truthful Lanes & Run Stamping
**Goal**: "Lane" names one thing wherever it appears, the vox-lang mix is the one the docs describe, and every training run records the spoke, backend, data slices and training health it actually had.
**Depends on**: Phase 553
**Requirements**: LANE-01, LANE-02, LANE-03, LANE-04, LANE-05, LANE-07, FUNL-04
**Success Criteria** (what must be TRUE):
  1. A committed inventory classifies every use of "lane" in code, config and docs as rename or keep; the serving lane, router `lane:` triggers and orchestrator lanes are unchanged, and a test fails if a renamed-category `lane` key reappears in `mens/config/*.yaml`.
  2. The compute-backend field is `backend` with a `lane` read alias, real old memory-model and run-manifest fixtures (copied from existing runs) still load, and corpus rows carry `data_slice` and `verification_tier`, with legacy rows mapped to `legacy.*` and tier `unknown` rather than guessed.
  3. vox-lang has exactly one canonical mix with required (`optional: false`) inputs, including `verified_vox_codegen_v2.jsonl`; duplicate/dead mixes and "Lane G" aliases are gone; non-QLoRA profiles declare `mode: retrieval_only` or `eval_only`; a missing required input fails the run before the first optimizer step; `mens-training-data-contract.md` matches the mix and passes the doc lint; `contracts/reports/mens-v95-vox-lang-mix-before-after.json` gives rows per `data_slice` for the old and the canonical mix.
  4. A training run (CPU tiny-model test, with and without `--fast-corpus`) writes `spoke`, `backend`, `base_preset`, `data_slice_histogram` and `verification_histogram` into `training_manifest.json` and `train-record.json`, fills the adapter card's spokes, and records non-null `truncated_rows`, `prompt_trimmed_rows`, last-epoch validation loss, pre-clip gradient norm and clip fraction.
**Parallel groups:** Serial first: LANE-01 inventory (read-only sweep, one inventory file). Then at once: (G-backend) `contracts/mens/memory-model.v1.yaml`, `crates/vox-populi/src/mens/tensor/{memory_model,calibration,accel_budget}.rs`; (G-slice) `crates/vox-corpus/src/corpus/mix/mod.rs`; (G-mix) `mens/config/mix*.yaml`, `mens/config/domain-profiles.yaml`, `crates/vox-populi/src/mens/tensor/{domain_profiles,spoke_validate}.rs`, `crates/vox-corpus/src/training/mix_prepare.rs`, `docs/src/reference/mens-training-data-contract.md`; (G-stamp) `crates/vox-ml-cli/src/commands/mens/populi/{train_arm,train_record}.rs`, new `populi/run_stamp.rs`, both plugins' `src/candle_qlora_train/finalize.rs` and the training loop's gradient-norm reporting. Runs in parallel with 554, 556 and 557.
**GPU:** none.
**Agents:** plan + verify on Opus; execute on Sonnet 5.5.
**Plans**: TBD

### Phase 556: Honest Hub & Space Hygiene
**Goal**: The public Space, cards and hub repos stop misleading anyone now, before any new weights exist: one card and one evidence copy per repo, known sync bugs fixed, superseded results and weightless repos labelled, and every output on the Space page labelled by where it came from.
**Depends on**: Phase 553
**Requirements**: HUB-04, HUB-06, HUB-07, CARD-06, SPACE-01, SPACE-02, SPACE-05
**Success Criteria** (what must be TRUE):
  1. Each repo has one card file and one copy of each evidence file, landed as one atomic Hub commit and pinned by a file-list contract test, after the Space's readers were migrated and `hub pull` was shown to read none of the removed paths.
  2. Each known sync defect has a failing-first test and is fixed: spoke bundles carry `train-record.json`, the dead `app.py.tmpl` / `requirements.txt.tmpl` are gone, a failed Space upload fails the sync, and NOTICE/attribution files ship for MIT-with-attribution bases and distilled data.
  3. The 27B card, and any other weightless repo's card, states on its first line that no weights are released yet and why; every remaining v1 compile-only result is labelled superseded; `mens-models.md` no longer claims the HF router serves MENS checkpoints.
  4. The Space page headlines `heldout_bench_v2` evidence loaded from a pinned commit SHA (not `main`), labels v1 as superseded, and shows a visible source label (recorded, live in-browser, live local serve, live base model) on every output; recorded output is never shown as live.
  5. `contracts/reports/mens-v95-space-contract-before-after.json` lists every URL and filename the Space fetches that is missing today and none after; the nonce-prompt Playwright spec reports SKIP, not pass, when WebGPU or a live path is unavailable; Spike B writes `contracts/reports/mens-v95-space-onnx-spike.json` with a go/no-go for SPACE-03 (export path, runtime support for the hybrid architecture, download size, CORS from `*.hf.space`).
**Parallel groups:** (G-sync) `crates/vox-populi/src/mens/{hub_sync,hub_cards_push}.rs`, deletion of `crates/vox-populi/templates/hub/space/{app.py.tmpl,requirements.txt.tmpl}`, NOTICE/attribution templates; (G-card-text) `crates/vox-populi/src/mens/hub_card/**` and its golden tests, `docs/src/reference/mens-models.md`; (G-space) `crates/vox-populi/templates/hub/space/{index.html.tmpl,README.md.tmpl}`, the Space contract test, a Playwright spec under `docs-astro/tests/`; (Spike B) throwaway code in `~/dev/_scratch/`, only the report is committed. Inside the phase, G-space migrates the Space's evidence readers before G-sync pushes the atomic cleanup commit. Runs in parallel with 554, 555 and 557.
**GPU:** short, Spike B only: its single in-browser WebGPU run takes the GPU lock and waits for `PID 34157` to exit; everything else is GPU-free.
**Agents:** plan + verify on Opus; execute on Sonnet 5.5.
**Plans**: TBD
**UI hint**: yes

### Phase 557: Sealed Blind Slice, Pre-Registered Statistic & Contamination Firewall
**Goal**: Before any new training data exists, the proof cannot be gamed: a blind slice is sealed, the comparison, its statistic and the verdict rule are fixed in writing, and code refuses training rows that overlap any benchmark.
**Depends on**: Phase 553
**Requirements**: PROOF-01, PROOF-02, DATA-01
**Success Criteria** (what must be TRUE):
  1. A 40–50-task blind v3 slice passes reference self-verification and fails wrong-return stubs; only its hash is committed, in a commit that precedes every data-generation commit; its items appear in no generation prompt, training file or public repo.
  2. A committed pre-registration file names the primary comparison (paired exact McNemar on `heldout_bench_v2`), the interval method (Wilson or bootstrap), the minimum detectable difference and power, and the Improved / Neutral / Regressed rule that every later rung and release verdict uses; the statistics code reproduces hand-checked McNemar and Wilson examples in unit tests.
  3. The firewall rejects a planted exact copy, an n-gram overlap and a near-duplicate of a v1, v2 and v3 item; each check's test fails when that check is removed; an import-boundary test fails if generation or distillation code can read benchmark items.
  4. `contracts/reports/mens-v95-firewall-audit.json` runs the firewall over the existing `verified_vox_codegen_v2.jsonl` (3,338 rows) and lists every row it now rejects.
**Parallel groups:** (G-seal) blind v3 authoring and its committed hash (items kept out of public repos); (G-prereg) pre-registration file under `mens/config/` and new `crates/vox-ml-cli/src/commands/mens/proof_stats.rs` (statistics plus the verdict rule); (G-firewall) `crates/vox-ml-cli/src/commands/mens/{eval_gate/leakage,v94_teacher_distill,v94_eval_corpus}.rs`, `crates/vox-ml-cli/src/commands/corpus/source_pool.rs` (its v3 tests run after G-seal). Runs in parallel with 554, 555 and 556.
**GPU:** none.
**Agents:** plan + verify on Opus; execute on Sonnet 5.5.
**Plans**: TBD

### Phase 558: Verified Data Harvest & Capped Distillation
**Goal**: The canonical vox-lang mix gains verified rows at near-zero cost — local harvest first, then at most 12 USD of licensed teacher distillation — with every row executed, attributed and counted.
**Depends on**: Phase 555 (slice stamps, canonical mix), Phase 557 (sealed slice, firewall)
**Requirements**: DATA-02, DATA-03, DATA-04
**Success Criteria** (what must be TRUE):
  1. The 0-USD local harvest adds verified rows from fixed workspace `.vox` files, multi-construct files and compiler repair traces; `contracts/reports/mens-v95-data-harvest.json` gives rows per `data_slice` before and after, and the firewall result for every added row.
  2. Paid distillation runs only through the LLM facade with MIT/Apache-licensed teachers, checks the 12 USD cap before each request and stops at it, and stores the resolved teacher model id, cost and slice on every row; `contracts/reports/mens-v95-distill-spend-ledger.json` totals at most 12 USD, counting failed attempts and repair turns.
  3. Every added row passes execution verification and anti-stub checks, failed-then-fixed attempts are kept as repair-trace rows, and mix resolution stops with an error when the synthetic share exceeds the ceiling or a human-written anchor is missing.
**Parallel groups:** (G-harvest) the workspace `.vox` files being fixed, new multi-construct files, a harvest `.vox` driver under `scripts/`; (G-distill) `crates/vox-ml-cli/src/commands/mens/v94_teacher_distill.rs` (or a successor module) and the spend ledger, LLM facade only; (G-ceiling) `crates/vox-corpus/src/corpus/mix/mod.rs` resolver and `mens/config/mix-vox-lang.yaml`. Runs in parallel with 559, 560 and 561. Its mix is the one every rung in 562 and 563 trains on.
**GPU:** none (network spend only, capped at 12 USD).
**Agents:** plan + verify on Opus; execute on Sonnet 5.5.
**Plans**: TBD

### Phase 559: Cached Decode, Discriminating Evals & Ladder Baselines
**Goal**: Before any size is retrained, every published adapter and its base have a "before" that is comparable with its "after", the benchmark is shown to tell adjacent sizes apart (or is extended, or says plainly that it cannot), and every hub size, spoke and auxiliary model has an eval that fits it within a wall-time budget.
**Depends on**: Phase 554 (timing, funnel, fast policy, Spike A), Phase 557 (declared power and verdict rule, sealed slice, firewall); GPU jobs also wait for `PID 34157` to exit
**Requirements**: TIME-04, LADR-01, DISC-01, DISC-02, DISC-03, DISC-04, DISC-05
**Success Criteria** (what must be TRUE):
  1. `contracts/reports/mens-v95-prefix-cache-parity-metal.json` shows prefix-cached and uncached greedy decode token-identical on all 166 + 64 items (otherwise the cache ships only behind a new decode-protocol tag), and gives cold vs warm eval wall time from interleaved clean-load runs as the measured speed-up.
  2. `contracts/reports/mens-v95-ladder-before.json` scores the published 0.8B, 2B, 4B and 9B adapters and their bases on all 166 `heldout_bench_v2` tasks, eval-only, under the frozen proof manifest and the same decode protocol the after-evals will use, with funnel and timing; stored completions that match the manifest are reused and only missing items are generated.
  3. `contracts/reports/mens-v95-item-discrimination.json` is computed on CPU, first from the existing completions (the 5-rung, 46-runnable-item v1 re-score in `contracts/reports/mens-execution-eval-rescore-v2.json`), then from the v2 "before" evals. It lists the items that separate each adjacent pair and the items that carry no signal (all pass or all fail), and states for each pair whether v2 distinguishes it at the pre-registered power. The 9B/27B pair uses the stored 27B completions until Phase 563 adds v2 ones.
  4. For every pair v2 cannot separate, `contracts/reports/mens-v95-bench-extension.json` records verified tasks added at that difficulty band in a separately versioned, private slice. No task comes from blind v3 or any training row, and each passes the firewall in both directions. The slice is pinned by a proof-manifest revision committed before the first ladder training run, and the report states for each pair whether it now separates, or plainly that it does not; `heldout_bench_v2` and its pre-registered comparison are unchanged.
  5. A committed anchor subset ranks the models the same as the full benchmark on runs not used to choose it (`contracts/reports/mens-v95-anchor-subset.json`), and a committed eval matrix names, for every hub size and every spoke or auxiliary model in `domain-profiles.yaml`, its eval type, dataset, n, detectable difference and a wall-time budget per size (a test fails when a model has no row; a row without a dataset says so instead of inventing an n). `contracts/reports/mens-v95-eval-budgets.json` compares measured wall time with each budget, met through item caching, prefix-cached decode, the fast sequential policy and the anchor subset, and flags every overrun.
**Parallel groups:** (G-prefix) `crates/vox-plugin-mens-candle-metal/src/{inference,model,batch_decode}.rs` and `crates/vox-plugin-mens-candle-cuda/src/{inference,model}.rs` (one owner for both backends; CUDA by `cargo check` only); (G-disc) new `crates/vox-ml-cli/src/commands/mens/eval_discrimination.rs` (item discrimination, adjacent-pair power through 557's `proof_stats.rs`, anchor selection and held-out agreement) and the anchor subset file under `mens/config/`; (G-matrix) new `mens/config/eval-matrix.yaml`, its coverage test and the budget check in `crates/vox-ml-cli/src/commands/mens/eval_timing.rs`; (G-extend) extension-slice authoring (items kept out of public repos), its hash and the proof-manifest revision under `mens/config/`, and its registration in `eval_gate/leakage.rs`; (Operator) one agent runs the GPU jobs in order from a `.vox` driver: Metal parity → "before" evals → extension-slice evals. G-disc's first pass on the existing completions starts at once on CPU; anchor selection, G-extend and the budgets report wait for the "before" evals. CLI variants for this phase's reports are added by 560's G-spoke-live, the single W2 owner of `populi/action_populi_enum.rs` and `dispatch.rs`. Runs in parallel with 558, 560 and 561.
**GPU:** short to medium — serial lock-held jobs (parity, the eight "before" evals with 9B the longest, then any extension-slice evals), none before `PID 34157` exits; each eval's wall time is recorded, not assumed.
**Agents:** plan + verify on Opus; execute on Sonnet 5.5.
**Plans**: TBD

### Phase 560: Live Spoke Gates
**Goal**: Every trained spoke is gated on live agent episodes run against a local model instead of on hub evidence, and spoke base pointers move only for bases that passed.
**Depends on**: Phase 554 (timing), Phase 555 (run stamping, spokes on cards), Phase 557 (statistic declared before the runs); GPU jobs also wait for `PID 34157` and for 559's jobs ahead of them in the queue
**Requirements**: SPOK-01, SPOK-02, SPOK-03, SPOK-04, SPOK-05
**Success Criteria** (what must be TRUE):
  1. `eval-spoke` runs the 72-episode suite against a live local model as two sequential passes (base, then candidate) and writes `contracts/reports/mens-v95-spoke-<spoke>-before-after.json` with tool-selection accuracy, argument-schema validity, tool-call format compliance, recovery after a tool error, and timing.
  2. Mono `harness-tools` and decomposed `tool-selection` + `argument-generation` are scored on the same suite in `contracts/reports/mens-v95-spoke-mono-vs-decomposed.json`, and the losing configuration is removed from config.
  3. Spoke gate rows are populated and enforced for every spoke marked trained (publishing one without its live suite report fails, and the test fails when the gate is removed), and `domain-profiles.yaml` base pointers change only for bases whose spoke gate passed; unevaluated candidates stay listed as candidates.
**Parallel groups:** (G-spoke-live) `crates/vox-ml-cli/src/commands/mens/eval_spoke.rs`, new `eval_spoke_live.rs`, `populi/action_populi_enum.rs` and its dispatch (also adding 559's report variants), CLI docs regenerated with `vox ci command-sync`; (G-spoke-gate) `crates/vox-populi/src/mens/gate_policy.rs`, `crates/vox-ml-cli/src/commands/mens/eval_gate/check_run.rs`, `mens/config/eval-gates-agents.yaml`; (G-spoke-config, last, after live results) `mens/config/domain-profiles.yaml` and the harness/tool mix files. One operator agent runs the live episodes when the queue reaches them. Runs in parallel with 558, 559 and 561.
**GPU:** short — lock-held episode runs, each under an hour, after 559's evals and before 562's canary.
**Agents:** plan + verify on Opus; execute on Sonnet 5.5.
**Plans**: TBD

### Phase 561: Evidence-Bound Cards & In-Browser 0.8B Space
**Goal**: Cards and the Space can only say what uploaded evidence supports: cards explain why the model exists, what changed and where it still fails, and the Space page runs the released 0.8B adapter live in the browser.
**Depends on**: Phase 554 (funnel), Phase 555 (run stamping), Phase 556 (hub hygiene, Spike B go/no-go)
**Requirements**: LANE-06, CARD-01, CARD-02, CARD-03, CARD-04, CARD-05, SPACE-03, SPACE-04, SPACE-06
**Success Criteria** (what must be TRUE):
  1. Cards derive the spoke from the run record, `default_spoke` is removed, and a run with no spoke renders as `unknown` and is refused for release (the test fails when the refusal is removed).
  2. A test parses every rendered card and fails on any number without an evidence source, and every number names the benchmark and version behind it; `contracts/reports/mens-v95-card-provenance-before-after.json` counts the unsourced numbers on today's cards and on the regenerated ones (zero).
  3. Cards lead with the measured gap against the base, then what changed since the previous release and why; they include a counts-based "where it still fails" table from the funnel, a generated release history, and a "pin this exact version" section with the commit SHA; a lint fails when card text duplicates `mens-models.md` or another card beyond the set threshold.
  4. A merged-ONNX export of the released 0.8B adapter, pinned by commit SHA, runs on WebGPU from the real `*.hf.space` origin behind a click-to-start button that shows the download size, and unsupported devices get an explicit message; `contracts/reports/mens-v95-space-webgpu-0.8b.json` records size, time to first token and tokens per second, and the nonce spec passes with source = live in-browser on a WebGPU device. The base-model demo and local-serve mode work from the same origin with no token stored in or required by the page.
  5. After each sync, a live check confirms the Space is `RUNNING` at the pushed commit and writes a proof receipt; a sync whose Space is not running at that commit writes none.
**Parallel groups:** (G-cards) `mens/config/hub-cards.yaml`, `crates/vox-populi/src/mens/hub_card/**` except `index_facts.rs`, card goldens, `docs/src/reference/mens-models.md`; (G-onnx) merged-ONNX export under `crates/vox-python-bridge/worker/**` plus a `.vox` driver; (G-space) `crates/vox-populi/templates/hub/space/**` and the Space specs under `docs-astro/tests/`; (G-sync) `crates/vox-populi/src/mens/{hub_sync,hub}.rs` (unknown-spoke refusal, Space status check and proof receipt). SPACE-03 work starts only on a "go" from Spike B. Runs in parallel with 558, 559 and 560, and with 562 if still open; 564's code groups start after it, since they share `hub_sync.rs`.
**GPU:** short — the in-browser WebGPU run and the local-serve check take the GPU lock in a gap between training jobs (between rungs or before 563's preflight), never during a training run (or on a separate WebGPU device).
**Agents:** plan + verify on Opus; execute on Sonnet 5.5.
**Plans**: TBD
**UI hint**: yes

### Phase 562: Tier-1 Canary & Ladder Retrain 0.8B → 9B
**Goal**: The new recipe is validated in a fast canary cycle, then the 0.8B, 2B, 4B and 9B sizes are retrained on the canonical mix in ascending order, each started only once the size below it is shown non-regressed, with every rung's real cost measured.
**Depends on**: Phase 558 (final mix), Phase 559 ("before" baselines, discrimination, anchor subset, extension slice); GPU jobs follow 559's and 560's in the queue
**Requirements**: PROOF-03, LADR-02, LADR-03, LADR-04
**Success Criteria** (what must be TRUE):
  1. `contracts/reports/mens-v95-canary-cycle.json` shows a Tier-1 canary cycle (train + eval, with timing and funnel) finishing in 45 minutes or less on the chosen size, contains the measured 0.8B-vs-2B comparison that chose it, and evaluates on 559's anchor subset where its agreement report supports that (otherwise the 64-item canary, with the reason recorded); the canary run is never paired with a full rung for a verdict.
  2. The queue starts only after the firewall, re-run over the final canonical mix against v1, v2, 559's extension slice and blind v3, rejects no row. Rungs then train in the order 0.8B → 2B → 4B → 9B with the canary-validated recipe, each to completion (not step-capped). The driver refuses to start a rung, including Phase 563's 27B, unless the rung below is classified non-regressed, and a test fails when that check is removed.
  3. Each completed rung has `contracts/reports/mens-v95-ladder-rung-<size>.json` with its paired verdict against its 559 "before" under the pre-registered rule (full v2 plus any extension slice, frozen manifest), funnel, timing and training-health fields. The anchor check may stop a clearly regressed rung early, but every verdict comes from the full benchmark.
  4. `contracts/reports/mens-v95-ladder-queue.json` records the measured train and eval wall time of each completed rung and a remaining-queue estimate, including the 27B run, recomputed from those measurements; no rung carries a duration written before it ran.
  5. A planted collapse, regression and memory spill each stop the queue in tests. A real recipe failure writes `contracts/reports/mens-v95-ladder-stop-<size>.json` with its funnel and training-health evidence, and no larger rung starts after it.
**Parallel groups:** (G-queue) new `crates/vox-ml-cli/src/commands/mens/ladder_queue.rs` (rung order, the non-regressed start check, stop records, the measured queue estimate; verdicts through 557's `proof_stats.rs`) and a `.vox` ladder driver under `scripts/`; in W3 it is the single owner of `populi/action_populi_enum.rs` and `dispatch.rs` and also adds 564's proof-report variant; (G-canary) the canary config and its anchor-subset eval wiring under `mens/config/`; (Operator) one agent runs the jobs in order: canary (0.8B, 2B) → firewall re-run → rung 0.8B → 2B → 4B → 9B, each as train → anchor check → full eval. Runs in parallel with 564's code groups (and 561 if still open).
**GPU:** canary jobs, then four full rungs, strictly serial under the lock; no rung duration is assumed in advance, each is measured as it completes.
**Agents:** plan + verify on Opus; execute on Sonnet 5.5.
**Plans**: TBD

### Phase 563: Qwen3.8-27B Preflight & CAP-27B-04 Full Run
**Goal**: The largest size is retrained last: the 27B high-capacity recipe trains to completion on the canonical mix, only after the 9B rung held, and is scored on the full benchmark and the blind slice.
**Depends on**: Phase 562 (9B rung non-regressed, recipe and queue estimate from the measured rungs)
**Requirements**: PROOF-04, PROOF-05
**Success Criteria** (what must be TRUE):
  1. A 30-minute `seq=2048` preflight on `Qwen3.8-27B`, on the same canonical mix the full run uses, writes `contracts/reports/mens-v95-27b-preflight.json` with peak memory, a finite initial loss and the gradient norm, and the full run refuses to start without a passing preflight.
  2. CAP-27B-04 trains to completion (not step-capped) on the canonical vox-lang mix rather than `--fast-corpus`, and its training manifest records spoke, backend and `data_slice_histogram`.
  3. `contracts/reports/mens-v95-27b-before-after.json` scores before (the base model, plus the existing r16 adapter) and after on all 166 `heldout_bench_v2` tasks, any extension slice 559 added for the 9B/27B pair, and the blind v3 slice, with paired counts, funnel, timing and collateral results under the frozen proof manifest.
  4. The preflight and full-run report headers show that no other GPU job ran at the same time, and `contracts/reports/mens-v95-ladder-queue.json` shows CAP-27B-04 as the milestone's last training job, started after the 9B rung was classified non-regressed, with its measured wall time in place of the estimate.
**Parallel groups:** one operator executor (27B "before" evals if not already stored, then the preflight, the full run and its evals, through 562's `.vox` ladder driver); the only code is the preflight-receipt check in `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs`. Runs in parallel with 564's code groups; nothing else may take the GPU lock until it ends.
**GPU:** long — CAP-27B-04 (about 30 h of training by the handoff's estimate, re-estimated from the measured rungs) plus the v2 and v3 evals, holding the lock throughout.
**Agents:** plan + verify on Opus; execute on Sonnet 5.5.
**Plans**: TBD

### Phase 564: Before/After Ladder Proof, Calibrated Floors & Gated Releases
**Goal**: Only adapters proven better reach the Hub: every rung and trained spoke has a generated verdict, all sizes sit on one table with paired intervals, `heldout_bench_v2` floors are calibrated in their own change, and each Improved release is one hash-bound commit that a clean `hub pull` by SHA can load and run.
**Depends on**: Phase 557 (pre-registered statistic and verdict rule), Phase 559 (measured baselines), Phase 560 (spoke evidence), Phase 561 (cards, Space proof receipt) for its code and floors; each rung's release also needs that rung's evidence from Phase 562 (0.8B–9B) or Phase 563 (27B)
**Requirements**: PROOF-06, PROOF-07, DISC-06, HUB-01, HUB-02, HUB-03, HUB-05
**Success Criteria** (what must be TRUE):
  1. `heldout_bench_v2` gate floors per model size are calibrated from 559's measured "before" baselines and land in their own reviewed commit, in a new policy file, before any candidate is gated against them; `eval-gates-ladder.yaml` and existing gate receipts are untouched.
  2. One generated report per hub rung and per trained spoke (`contracts/reports/mens-v95-release-proof-<rung-or-spoke>.json`) states the verdict (Improved / Neutral / Regressed, by the pre-registered rule), the paired counts, the interval and the cost in GPU-hours; pairing a step-capped run with a full one yields "not comparable", never a verdict.
  3. `contracts/reports/mens-v95-ladder-table.json` puts all five sizes, before and after, on one table with paired intervals between adjacent sizes and marks every difference that is not significant; the `vox-mens-index` card renders that table from that file.
  4. Only Improved adapters move `main` and receive the next `v<N>` tag; a release is refused when the evidence hashes do not match the weights being uploaded, when the adapter regresses against the previous tag, or when the Space proof receipt is missing; the release path has no ungated bypass, and each refusal's test fails when its guard is removed.
  5. `vox-mens-index` publishes a release map (tag → full commit SHA → benchmark hash → headline metrics → verdict) that includes Neutral and Regressed attempts, each release's weights, card and evidence land in one Hub commit, and a real `hub pull` by that commit SHA into a clean directory loads the adapter and generates text in a smoke test.
**Parallel groups:** Serial first: (G-floors) the new floors policy file under `mens/config/`, in its own commit. Then at once: (G-proof) new `crates/vox-ml-cli/src/commands/mens/proof_report.rs`, the per-rung/per-spoke reports and the ladder table; (G-release) `crates/vox-populi/src/mens/hub_sync.rs` gate/stage/tag path and `crates/vox-ml-cli/src/commands/mens/hub.rs` (`--allow-ungated` off the release path); (G-index) `crates/vox-populi/src/mens/hub_card/index_facts.rs` and the index templates under `crates/vox-populi/templates/hub/` (release map and ladder table). The code groups start once 557, 559, 560 and 561 are done and run alongside 562 and 563. Each rung is released after its proof report exists: 0.8B–9B may go out between 562's last rung and 563's preflight, and the 27B after 563; the phase closes with the 27B verdict.
**GPU:** short — the `hub pull` smoke tests, taken in a gap before 563's preflight for 0.8B–9B and after 563 for the 27B.
**Agents:** plan + verify on Opus; execute on Sonnet 5.5.
**Plans**: TBD

## Coverage

`67/67` v1 requirements mapped, each to exactly one phase (see the Traceability table in [`REQUIREMENTS.md`](REQUIREMENTS.md)).
Deferred v2 items (`ARCH-01..03`, `DATA-05`, `SPACE-07`, `HUB-08`) are not scheduled.

## Progress

**Execution order:** by wave (W0 → W5); phases within a wave run concurrently, GPU jobs follow the single queue above.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 553. Clean Base, GPU Lock, Frozen Contracts & Proof Manifest | 0/TBD | Not started | - |
| 554. Eval Timing, Failure Funnel & Prefix-Cache CPU Spike | 0/TBD | Not started | - |
| 555. Truthful Lanes & Run Stamping | 0/TBD | Not started | - |
| 556. Honest Hub & Space Hygiene | 0/TBD | Not started | - |
| 557. Sealed Blind Slice, Pre-Registered Statistic & Contamination Firewall | 0/TBD | Not started | - |
| 558. Verified Data Harvest & Capped Distillation | 0/TBD | Not started | - |
| 559. Cached Decode, Discriminating Evals & Ladder Baselines | 0/TBD | Not started | - |
| 560. Live Spoke Gates | 0/TBD | Not started | - |
| 561. Evidence-Bound Cards & In-Browser 0.8B Space | 0/TBD | Not started | - |
| 562. Tier-1 Canary & Ladder Retrain 0.8B → 9B | 0/TBD | Not started | - |
| 563. Qwen3.8-27B Preflight & CAP-27B-04 Full Run | 0/TBD | Not started | - |
| 564. Before/After Ladder Proof, Calibrated Floors & Gated Releases | 0/TBD | Not started | - |
