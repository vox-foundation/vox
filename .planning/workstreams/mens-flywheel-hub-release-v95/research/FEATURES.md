# Feature Research

**Domain:** Open-weights model release on the Hugging Face Hub (versioning, release gating, model cards, before/after eval reporting, demo Space) for the MENS hub-and-spoke fine-tunes
**Milestone:** v9.5 "MENS Flywheel, Truthful Lanes & Trustworthy Hub Releases"
**Researched:** 2026-11-20
**Confidence:** MEDIUM-HIGH (HF mechanics and statistics: HIGH from official docs and the cited paper; "what well-run releases do" and Space-credibility conventions: MEDIUM, partly my synthesis, flagged per row)

Method note: web research used WebSearch/WebFetch directly; the `research-plan`/`research-store` seam was not used (no caching of digests). Items tagged **[inferred]** are my synthesis, not a cited convention.

## What the HF ecosystem actually provides (anchors for everything below)

| Mechanism | What it is | Source |
|---|---|---|
| Git refs | Hub repos are git repos; `create_tag`, `create_branch`, `delete_tag`, `list_repo_refs`; `revision=` accepts branch, tag or full commit SHA | [HF repo guide](https://huggingface.co/docs/huggingface_hub/guides/repository) |
| Tags are mutable | `delete_tag` exists, so a tag is a label, not proof; only the commit SHA is immutable | [HF repo guide](https://huggingface.co/docs/huggingface_hub/guides/repository) (inference from API surface; community sources disagree, treat as LOW) |
| `new_version` | Card metadata on the OLD repo; Hub shows a banner linking to the update and follows chains to the latest | [HF model cards](https://huggingface.co/docs/hub/model-cards) |
| `base_model` + `base_model_relation` | `finetune` / `adapter` / `quantized` / `merge`; feeds the model tree | [HF model cards](https://huggingface.co/docs/hub/model-cards) |
| Release checklist | Separate repo per variant, group in a Collection, safetensors, YAML metadata, copy-and-run usage, limitations, link a Space, `new_version`, then "after release: test all snippets in a clean environment" | [HF release checklist](https://huggingface.co/docs/hub/model-release-checklist) |
| Annotated card | Limitations = technical failure modes (separate from sociotechnical bias/risk); evaluation "ideally disaggregated" by factor; Results with a Summary; Speeds/Sizes/Times; Glossary; links to more info | [Annotated model card](https://huggingface.co/docs/hub/model-card-annotated) |
| `.eval_results/*.yaml` | New structured results format. `dataset.id` MUST be a Hub dataset registered as a Benchmark with an `eval.yaml` task; optional `source`, `notes`, `date`; `verified` badge only if run via HF Jobs + inspect-ai; `community` badge for open PRs. Marked work-in-progress, benchmark registration is beta/allow-listed | [HF eval results](https://huggingface.co/docs/hub/eval-results) |
| `model-index` | Older results metadata in README YAML; free-form task/dataset/metric/source; still documented | [HF model cards](https://huggingface.co/docs/hub/model-cards) |
| Revisions as checkpoints | OLMo ships intermediate checkpoints as revisions plus a revisions list, and points old cards at the newer repo | [OLMo paper](https://arxiv.org/html/2402.00838v4), [OLMo-7B card](https://huggingface.co/allenai/Olmo-7B) |
| Date-suffixed refresh names | Qwen3 "-2507" refresh published as new repos with a year-month suffix; Alibaba publicly said the hybrid-thinking design was dropped for separate Instruct/Thinking models | [Qwen3-30B-A3B-Instruct-2507](https://huggingface.co/Qwen/Qwen3-30B-A3B-Instruct-2507), [The Register](https://www.theregister.com/2025/07/31/alibaba_qwen3_hybrid_thinking/) (MEDIUM) |

There is NO official HF "deprecated" flag for model repos; the only platform-native supersede signal is `new_version` (searched; community notices are ad hoc README banners). Plan on `new_version` + a card banner + the index repo.

## Feature Landscape

Legend: **TS** = table stakes, **DIFF** = differentiator, **ANTI** = anti-feature. Complexity LOW/MED/HIGH. "Builds on" names existing Vox pieces from the required reading (`vox mens hub sync`, `hub-cards.yaml`, `mens-models.md`, `gate_receipt.json`, `collateral_damage_report.json`, `eval-gates-ladder.yaml`, `heldout_bench_v2`, `vox_eval::reasoning_slice` McNemar, the duplicate-sentence lint, `vox-mens-index`, `vox-mens-playground`).

### 1. Versioning, changelogs, supersede/deprecate

| ID | Feature | Class | Why | Cx | Builds on / notes |
|---|---|---|---|---|---|
| V-1 | One repo per rung; `main` always = latest gate-passed revision; version = git tag on that commit (`v9.5.0` or `YYYY.MM` style) created ONLY by the push path after the gate passes | TS | HF checklist says separate repo per variant; users pin `revision=`. Today repos have zero tags, so no version is addressable except by SHA | LOW | `hub sync` push path + `create_tag`. Today: 0 tags measured |
| V-2 | Release record mapping tag -> full commit SHA -> base pin -> eval-bench hash -> date -> headline numbers, published in the index repo as `RELEASES.json` + rendered table | TS | Tags are mutable (`delete_tag` exists); only the SHA is immutable. A published map makes a force-moved tag detectable. Third-party advice is the same: keep your own tag->SHA->metrics mapping [LOW-MEDIUM, community sources] | LOW | `vox-mens-index` repo; run records already carry hashes (`slice_blake3`, `train_data_sha256`) |
| V-3 | Per-repo `CHANGELOG` section on the card (3-5 bullets max per version): what changed (data, recipe, base), why, and the measured delta vs previous version on the SAME items | TS | "What changed and why" is the user expectation; Qwen-style refresh notes are the norm. Must not duplicate the card body; point to the index repo for history | LOW-MED | Needs previous-version run record at render time; extends the existing facts object |
| V-4 | `base_model` + explicit `base_model_relation: adapter` (or `finetune` for merged checkpoints) and `datasets:` in YAML | TS | Free model-tree placement and filtering | LOW | Verify current cards emit it; `library_name: transformers` already set per docs |
| V-5 | When the BASE changes (e.g. 9B moves to a different base), create a new repo and set `new_version` on the old card; do not silently re-point | TS | `new_version` is the one official supersede mechanism and follows chains | LOW | Relevant to handoff plan to swap 9B/4B hub bases |
| V-6 | Status line vocabulary extended with lifecycle states: `current`, `superseded by <tag/repo>`, `evidence-only (no weights)`, `withdrawn (reason)` | TS | No platform deprecated flag exists, so the card must carry it; existing status line already has trained/spoke/gate | LOW | `mens-models.md` state legend; lint must allow the repeated state phrase |
| V-7 | Index repo release log: reverse-chronological list of releases AND non-releases ("run X did not beat base; not published; evidence linked") | DIFF | Keeps interest and trust: a visible cadence including honest misses. Few projects do it; closest precedent is public acknowledgment of design reversals (Qwen) | MED | `vox-mens-index` + gate receipts; a rendered template, not new infra |
| V-8 | Trend chart on index: headline metric per rung per version (v2 benchmark, with CI bars) | DIFF | Makes "improving over time" demonstrable instead of claimed | MED | Needs V-2 + E-2 |
| V-9 | Intermediate checkpoint revisions (OLMo-style step branches) | ANTI (for now) | Adds repo weight and surface area; only valuable for researchers; you have 5 rungs of adapters, not pretraining runs | n/a | Defer; evidence in `evidence/` is enough |
| V-10 | Date-suffixed repo names per release (`-2611`) | ANTI | Multiplies repos and fragments downloads/likes; the per-rung repo + tags already gives version addressing | n/a | Use only if V-5 triggers |
| V-11 | Force-moving or re-pointing tags / rewriting `main` history to "fix" a release | ANTI | Breaks every pinned user and the V-2 map | n/a | If a release is bad: push a new revision, tag a new version, mark the old one `superseded`/`withdrawn` in V-6 and V-7 |
| V-12 | Pre-release "coming soon" page via `release_date` | ANTI (not available) | Enterprise-plan only and for private repos; do not plan around it | n/a | Use the index row `in training` instead (see G-6) |

### 2. Release gating and presenting a non-winning model

| ID | Feature | Class | Why | Cx | Builds on / notes |
|---|---|---|---|---|---|
| G-1 | Hard gate before any public push: (a) current-rule `gate_receipt.json` + `collateral_damage_report.json` valid against pinned slice hashes, (b) paired comparison vs base on `heldout_bench_v2` computed and stored, (c) leakage check clean (`assert_no_text_leakage`), (d) weights present and a clean-room load+generate smoke test passed | TS | HF checklist ends with "test all snippets in a clean environment"; a model card without loadable weights is a broken release | MED | (a) exists; (b)(c) partially exist per handoff; (d) is new and cheap (one script) |
| G-2 | Release classes decided by evidence, not by hope: **Improved** (paired test significant vs base AND no significant regression vs previous version AND collateral gate passed), **Neutral** (no detectable difference vs base or vs previous), **Regressed** (significant loss vs previous or base) | TS | You must say what you can prove. The existing "passed" gate answers collateral damage, not "did it get better"; a separate effect verdict is needed | MED | Reuses McNemar machinery; the gate policy in `eval-gates-ladder.yaml` needs a v2-calibrated "improvement" threshold (handoff open question 4: the 0.25 floor was written for the saturated v1 bench) |
| G-3 | Publication rule per class: **Improved** -> update `main`, tag, announce. **Neutral/Regressed vs previous** -> do NOT move `main`; record in the index release log (V-7) with evidence links; optionally push to a non-default branch (`candidate/<run-id>`) so anyone can inspect or load it | TS | Honest without shipping bad work: the good revision stays default, the miss is visible and reproducible, nothing is hidden | LOW-MED | Branch push is one `create_branch`; the "not published" log row is a template |
| G-4 | First-ever release for a rung that fails (nothing good to keep on `main`): do not publish a public model repo; show the rung on the index as `not yet released` with last measured numbers and the reason | TS | Avoids the 27B situation measured today (card present, no weights). A public model repo with no weights reads as a broken release | LOW | Directly resolves the live 27B finding |
| G-5 | Evidence-only repos, if kept public, must say so in the first line of the card and in metadata tags, and must not appear in the index as a downloadable rung | TS (if G-4 not chosen) | Same trust rule; decision for the requirements author: G-4 (preferred) or G-5 | LOW | Pick one; do not leave the current silent state |
| G-6 | Index status table with explicit rows per rung: version, status (`current` / `in training` / `candidate not released` / `superseded`), delta vs base with CI, delta vs previous with CI, date | DIFF | One page that tells the whole truth and keeps people checking back | MED | Replaces the current 'release-verified list' framing in `mens-models.md` |
| G-7 | Gate by mutation: before trusting a new gate, break it deliberately (feed a known-worse adapter) and confirm it blocks; store that "known-bad control must fail" proof per suite | DIFF | Already house policy for harness suites; extend to the improvement verdict (control = base model or previous adapter with weights shuffled) | MED | `mens-models.md` harness section already requires known-bad controls |
| G-8 | Sequential/early-stop screening for dev runs, fixed-n full evaluation for release promotion | DIFF | Cuts the inner loop cost; release verdict stays fixed-n so alpha is not eroded | MED | Already specified in `eval-gates-ladder.yaml` sequential block; handoff proposes `--sequential` |
| G-9 | Automated publishing of every training run to the Hub "to keep interest up" | ANTI | Every passing-by-chance run becomes a version; users cannot tell signal from noise; interest comes from a legible cadence (V-7/V-8), not volume | n/a | Publish on evidence; log the rest |
| G-10 | Best-of-N-seeds publication (train several seeds, publish the best) without reporting the spread | ANTI | Selection bias: the published number is a max, not an estimate. If seeds are varied, report all and publish the median or pre-registered seed | n/a | Pre-register the seed (the shared seed is already recorded) |
| G-11 | Using the HF `verified` badge or "leaderboard" language for locally computed results | ANTI | `verified` requires an HF Jobs + inspect-ai run with a verifyToken; claiming it otherwise is a trust violation | n/a | Use "source: evidence link" instead |

### 3. Model card content: card vs docs page vs machine-readable metadata

Allocation rule (opinionated): the card answers "should I use this and what changed"; the docs page owns shared definitions; YAML metadata and evidence JSON own anything a tool consumes. If a fact is in two of the three, delete one. The existing duplicate-sentence lint already enforces this across cards; extend it to card vs `mens-models.md`.

| ID | Feature | Class | Why | Cx | Builds on / notes |
|---|---|---|---|---|---|
| C-1 | Exactly one human card per repo: `README.md` (the only file the Hub renders). Delete `MODEL_CARD.md` | TS | HF renders `README.md`; a second card file is dead duplication and a drift risk. Measured: every repo has both | LOW | Change in `hub sync` file set |
| C-2 | One copy of evidence files: keep under `evidence/` only; root copies only if a loader requires them | TS | Measured: duplicated at root and `evidence/`; doubles size, invites mismatch | LOW | `hub sync` upload list |
| C-3 | Card section set (and nothing else): status line (trained / spoke / gate / lifecycle); one-sentence what + **why this fine-tune exists** (purpose of the spoke, what the base gets wrong in Vox); headline result vs base with CI on the current benchmark; where it still fails; usage; version line + 3-bullet changelog; links | TS | Matches HF checklist and annotated card (description, uses, limitations, evaluation, usage) | MED | Re-cut the Askama template; per-size `why` blurbs in `hub-cards.yaml` already exist and already forbid digits (good: numbers come only from run records) |
| C-4 | "Why this fine-tune exists" is stated as a measured gap, not a claim: base pass rate vs adapter pass rate on the same items, plus the one example class the base gets wrong (Rust/TS syntax instead of Vox). One sentence of motivation, then the number | TS | The question every reader has; a motivation without a measured gap is marketing. The existing `default_spoke_source` text is the right seed but states no measurement | LOW | Combine `hub-cards.yaml` spoke text with E-2 numbers |
| C-5 | Limitations section in HF's sense: technical failure modes with numbers (per-category zero cells, funnel stage where completions die, truncation rate), not generic disclaimers | TS | Annotated template separates limitations (technical) from bias/risk; a 32K-card study finds most limitation sections are boilerplate disclaimers [MEDIUM, [arXiv 2402.05160](https://arxiv.org/pdf/2402.05160)] | MED | Needs E-4 funnel and E-3 per-category tables from stored check fields |
| C-6 | Machine-readable block: YAML metadata (`base_model`, `base_model_relation`, `datasets`, `license`, `library_name`, `pipeline_tag`, `new_version` when relevant, `tags` incl. spoke id) + results as `model-index` OR `.eval_results` (see C-8) + `evidence/*.json` with item-level outputs, bench hashes, funnels, timings | TS | Metadata drives filtering/model tree; evidence JSON is the audit trail the card links to | LOW-MED | `.eval_results/vox_mens_eval.yaml` already emitted |
| C-7 | Shared explanations (McNemar, gate states, protocol, hub/spoke, harness) live ONLY on `mens-models.md`, cards link by anchor | TS | Already the design; keep and make the link anchors tested | LOW | Exists; add a link-resolve check to the release gate |
| C-8 | Verify `.eval_results` actually renders for a custom benchmark. Per the docs, `dataset.id` must be a Hub dataset registered as a Benchmark (beta, allow-listed). `heldout_bench_v2` is not one, so the current `.eval_results/vox_mens_eval.yaml` may be silently ignored. Cheap fallback: `model-index` with a named dataset + `source:` linking the evidence file. Strategic option: publish `heldout_bench_v2` as a Hub dataset + `eval.yaml` and request registration | DIFF (registration) / TS (verify) | If it works you get a leaderboard page and a "benchmark" tag; if not you get a false belief that results are surfaced | LOW (verify) / HIGH (register + allow-list + inspect-ai task) | Test on a scratch repo first. Registration also forces a decision about publishing the 166 tasks (leakage vs transparency; see ANTI below) |
| C-9 | "Cost to produce" line: train wall-clock, eval wall-clock, hardware, peak memory (HF template: Speeds/Sizes/Times + optional CO2) | TS-lite | Template section exists; cheap once timings are recorded | LOW (after timing instrumentation) | Handoff section 5 timing schema; `train_wall_secs`, `peak_metal_allocated_bytes` already in run records |
| C-10 | Pass-gain-per-GPU-hour and cycle-time on the INDEX only, not on every card | DIFF | Interesting to the community and to funders; on 5 cards it is repetition | LOW | Handoff metrics `pass_gain_per_gpu_hour`, `eval_share_of_cycle` |
| C-11 | Honest failure display: keep "no broken adapter code on published cards" for raw code (scannability) but REPLACE it with a "Where it still fails" table (funnel stage counts and zero-pass categories) plus a link to raw outputs in `evidence/` | DIFF | Current policy shows only verified wins; combined with Space and card headline, this reads as curated. Counts keep it honest without making cards unscannable | LOW-MED | Policy tension: the existing rule in `mens-models.md` ("Gate-passed model cards render only verified Fixed wins") stays, but cannot be the only failure disclosure |
| C-12 | Per-size blurb sentences that differ only by size name or numbers; boilerplate paragraphs repeated per card | ANTI | The lint exists because of this; keep it | n/a | Existing lint |
| C-13 | Citing the saturated 52-prompt compile-only benchmark as a headline anywhere | ANTI | The project itself measured 94.2% there vs 12-14% for 0.8B on v2; Hugging Face's own leaderboard v2 retired saturated benchmarks for the same reason [HF blog via [DeepLearning.AI](https://www.deeplearning.ai/the-batch/hugging-face-overhauls-open-llm-leaderboard-with-tougher-benchmarks)] [MEDIUM, search snippet] | n/a | Keep v1 only as a labelled smoke test ("compiles", not "works") in `evidence/` |
| C-14 | Publishing the full 166-task `heldout_bench_v2` prompts and tests publicly in model repos | ANTI | Invites contamination of future bases and of your own training mixes; HF's v2 leaderboard cites gated/contamination-resistant selection. Publish hashes + per-category counts + a small public dev slice; keep a rotating blind slice private (handoff proposes a v3 blind slice) | n/a | Decision needed if C-8 registration is pursued |
| C-15 | Environmental-impact numbers estimated without a measurement method | ANTI | CO2 field is optional; an invented figure is worse than none | n/a | Report hours + hardware (C-9) instead |

### 4. Statistically honest before/after reporting on small benchmarks

Key fact for this project: n = 166 items, greedy single sample, base and adapter scored on the same items. A 0.8B at ~12-14% has about 20 passes, so the unpaired SE is ~2.5 pp (my calculation: sqrt(0.12*0.88/166)). A minimum detectable unpaired difference near ~7 pp is plausible at this size, so many real gains will be "not significant" at 0.8B and 2B. Do not report a gain without the paired test.

| ID | Feature | Class | Why | Cx | Builds on / notes |
|---|---|---|---|---|---|
| E-1 | Pair everything: base and adapter (and previous-version adapter) scored on identical items; report discordant counts (adapter-only wins `b`, base-only wins `c`) and the exact McNemar p-value, not just two marginal rates | TS | Two marginal CIs that overlap can still hide a significant paired difference; papers and Anthropic's eval-statistics write-up recommend question-level paired differences [[Miller, arXiv 2411.00640](https://arxiv.org/html/2411.00640v1)], [exact McNemar usage](https://rasbt.github.io/mlxtend/user_guide/evaluate/mcnemar) | LOW | Exact one-sided McNemar already implemented (`PAIRED_RULE`) for collateral; reuse for pass@1. One-sided "did it improve" vs one-sided "did it regress" are different tests; pre-declare which |
| E-2 | Report each pass rate with a small-n-appropriate interval: Wilson (or exact Clopper-Pearson) on each marginal, and a CI on the paired difference (bootstrap over items, or Newcombe paired interval) | TS | Miller's CLT-based SEM is fine near 1000+ questions but the article gives no small-sample threshold and the paper's own examples point to ~1000 items for good signal; at n=166 and p near 0.1 the normal approximation is poor, so use Wilson/bootstrap [inferred from the paper's caution + standard practice] | LOW-MED | New small stats module or reuse a crate; bootstrap seed pinned and recorded |
| E-3 | Per-category breakdown as counts (`passed/n`) with NO per-category significance claims where n is tiny (`endpoint_handler 0/4`, `state_machine 0/4`); flag categories under a minimum n as "too few items to conclude" | TS | HF annotated card asks for evaluation disaggregated by factor; Mitchell et al. likewise ([model cards paper](https://arxiv.org/pdf/1810.03993v1)). With n=4 a test is meaningless | LOW | Bench manifest has categories; stored check fields exist |
| E-4 | Failure funnel per model: `no_code` -> `parse_fail` -> `typecheck_fail` -> `anti_stub_fail` -> `limit/step/string` -> `assertion_fail` -> `pass`, base vs adapter side by side; plus `truncation_rate` (hit max tokens) | DIFF | Explains WHY a fine-tune helped or did not (e.g. gains come from fewer parse failures but not fewer assertion failures). Most open releases report one number | MED | Handoff section 6 lists the fields; mostly derivable from stored `parse_ok`/`typecheck_ok`/`anti_stub_pass`; the string-limit counter is new |
| E-5 | Multiple-comparison control when many slices/categories/rungs are tested at once: pre-declare ONE primary comparison per release (adapter vs base, overall pass@1 on v2); everything else descriptive, or Holm-corrected | TS | Prevents fishing; Holm/BH is the common practice in papers that run many paired tests ([examples via search](https://rasbt.github.io/mlxtend/user_guide/evaluate/mcnemar), MEDIUM) | LOW | Policy change in `eval-gates-ladder.yaml` plus report schema |
| E-6 | Power note on every report: smallest paired effect detectable at the observed discordance; "no significant difference" is printed as "no detectable difference at n=166 (detects >= X pp)" | DIFF | Miller recommends power analysis; stops "not significant" from being read as "no effect". The existing collateral doc already says the test misses a 5% drop | MED | Power table already exists for collateral; extend to pass@1 |
| E-7 | If you ever sample >1 completion per item, use the unbiased pass@k estimator (n samples, c correct, `1 - C(n-c,k)/C(n,k)`), and note that McNemar no longer applies per item (use per-item rates with a paired signed-rank or bootstrap) | TS (if used) | Standard from [Chen et al. 2021](https://arxiv.org/pdf/2107.03374) and [HumanEval harness](https://www.github.com/openai/human-eval); naive `1-(1-p)^k` underestimates | LOW | Handoff plans n=5 pass@k and repair@2 for Tier 2; use then |
| E-8 | Seed variance disclosure: the paired test measures item sampling noise, NOT training-run noise. State this under every headline; for the flagship rungs run >= 2 training seeds when budget allows and report the spread | DIFF | Most honest-looking reports silently omit it; cheap to state, expensive to measure | LOW (state) / HIGH (measure) | Tie to cost reporting; do not gate releases on multi-seed (too expensive at 27B) |
| E-9 | Time-and-cost block per release: `train_wall_secs`, `eval_wall_secs`, `cycle_wall_secs`, `eval_share_of_cycle`, `generated_items` vs `cached_items` so cached resumes do not distort throughput | DIFF | Turns flywheel speed into a reported, comparable fact; informs where to invest | MED | Handoff section 5 schema (additive fields; do not touch checkpoint cache-key inputs) |
| E-10 | Cache the shared system-prompt prefix across eval items | TS (infra) | Removes ~180K redundant prefill passes per run (handoff measurement); required for eval cost to be tractable | MED-HIGH | `inference.rs:945` resets cache per call today |
| E-11 | Calibrated gate floors for the new benchmark (per-size, v2), replacing the v1-calibrated `min_pass_rate_at_1: 0.25` | TS | A floor calibrated on a saturated bench either blocks everything or means nothing | LOW-MED | Handoff open question 4 |
| E-12 | A rotating private blind slice (v3, ~40-50 tasks) used only for release verdicts | DIFF | Protects the verdict from the synthetic-data feedback loop; standard anti-contamination move | MED-HIGH | Needs authoring; handoff already proposes it |
| E-13 | Reporting only overall pass rate; or reporting the best slice; or comparing "before" and "after" runs that used different prompts/decode settings | ANTI | Not comparable; the existing protocol pins `qwen3_non_thinking_codegen_v1` and greedy decode, keep it pinned in the report header and refuse mismatched pairs | n/a | Existing refusal rule for mismatched hashes/grader ids |
| E-14 | Reporting "improved" when only the lm-eval-style marginal moved by less than the interval width | ANTI | The defining honesty failure for sub-200-item benchmarks | n/a | G-2 classes enforce this |

### 5. Making the demo Space credible

Facts: a static Space runs no server, so it cannot call a model by itself (forum thread, LOW); live inference needs a Gradio/Docker Space or a call to an inference endpoint/provider. ZeroGPU is Gradio-only and has compatibility caveats ([HF ZeroGPU docs](https://huggingface.co/docs/hub/spaces-zerogpu)). Free Spaces sleep and wake slowly; community reports of stuck restarts exist (LOW). The HF checklist says: link the Space on the model page, load the model from the Hub repo (not elsewhere), and test snippets in a clean environment.

The Space has two honest modes, and every output on screen must say which one produced it **[inferred]**:

| ID | Feature | Class | Why | Cx | Builds on / notes |
|---|---|---|---|---|---|
| S-1 | Real inference path that has actually been exercised: a "Try it" tab that sends the user's prompt to the pinned release (revision SHA shown) and returns a freshly generated answer, then runs `vox check` on the result and shows the verdict | TS | "Demonstrably works" means a stranger can type a prompt and get a real answer plus a compiler verdict | HIGH | Needs a Gradio/Docker Space (not static) with ZeroGPU or a hosted endpoint; claim in `mens-models.md` that merged checkpoints serve via the HF Inference Router (`:auto`, `:featherless-ai`) must be verified with a real call before the Space depends on it |
| S-2 | Replayed-evidence tab, visibly labelled "Recorded run, not live": shows stored prompt, base output, adapter output, compiler verdict, item id and evidence hash | TS | Cheap, deterministic, works when GPU is unavailable; it is what the static Space has today, just labelled and honest | LOW | Existing static Space + evidence JSON |
| S-3 | Distinct visual treatment and copy for Live vs Replay (badge, colour, timestamp); never present a replay in a box that looks like a chat reply | TS | Core credibility rule; the most common trust failure in demos is blurring the two | LOW | UI only |
| S-4 | Honest degradation: when live inference is queued, out of quota, or asleep, show that state and offer the replay tab; do not silently serve a canned answer | TS | Spaces sleep, ZeroGPU has quotas; silent fallback is the usual deceit | LOW-MED | Space runtime state via the `huggingface_hub` Space runtime API |
| S-5 | Headline benchmark on the Space is `heldout_bench_v2` (paired results with CIs, funnel); v1 52-prompt appears only as a labelled "compile smoke test" | TS | Measured: Space currently headlines the saturated 94.2% bench while v2 shows ~12-14% at 0.8B. This is the single most damaging inconsistency | LOW | Pure content change plus E-1/E-2 numbers |
| S-6 | "Last verified live" stamp: a scheduled job calls the live path with a fixed probe prompt, records pass/fail and time, and the Space displays "last live check: <timestamp>, <model@sha>, ok/failed" | DIFF | Makes "demonstrably works" a continuously checked property, not an assertion at launch | MED | Scheduled workflow + small status JSON committed to the Space; billing note from global CLAUDE.md: hosted CI may be unavailable, so run this from the local scheduler or a Space-internal timer **[inferred]** |
| S-7 | Space pins the model by tag/SHA and shows it; version selector (current + previous) with the same prompt side by side | DIFF | Lets users see the improvement across versions instead of trusting a changelog | MED-HIGH | Needs V-1/V-2; doubles inference cost, make it replay-only for old versions |
| S-8 | Run-it-yourself block: the exact `vox mens serve`/`transformers` snippet, tested in CI against the pushed revision | TS | HF checklist: verify all snippets in a clean environment after release | LOW-MED | Part of G-1(d) |
| S-9 | Cherry-picked "best of" gallery presented as the demo | ANTI | Deterministic picking is already used for card examples, but a Space with only wins looks curated; pair with S-2 failure display (funnel and "show a failing item") | n/a | Add a "show me a failure" button in replay |
| S-10 | A static page that looks like a chat UI but returns canned text for any input | ANTI | Most-cited demo-trust failure; if the page cannot run the model, say it is a viewer | n/a | Rename the current Space tab to "Evidence explorer" until S-1 ships |
| S-11 | Heavy always-on GPU hardware for the Space | ANTI | Cost and sleep behaviour; use ZeroGPU/endpoint + replay fallback | n/a | Review HF ZeroGPU compatibility first |

## Feature Dependencies

```
E-10 prefix cache ─┐
E-9 timings ───────┼─> cheaper/faster evals ──> G-8 sequential screening
                   │
E-1 paired test ──> E-2 CIs ──> E-6 power note ──> G-2 release classes ──> G-3 publication rule ──> V-1 tags ──> V-2 release map ──> V-7/V-8 index log & trend
E-11 v2 floors ────────────────┘                           │
E-12 blind slice ──────────────┘                           └──> G-4/G-5 (27B no-weights decision)

E-3 per-category + E-4 funnel ──> C-5 limitations ──> C-11 "where it still fails"
C-1/C-2 dedupe files ──> C-3 card re-cut ──> C-4 "why it exists" ──> C-7 link-resolve check
V-3 changelog needs previous-version record ──> V-2

G-1(d) clean-room smoke test ──> S-8 snippets ──> S-1 live Space ──> S-6 last-verified stamp
S-5 headline switch ──requires──> E-1/E-2 numbers
S-2 replay tab ──enhances──> S-1 live tab; S-3/S-4 required by both

C-8 (verify eval_results renders) ──> optional bench registration (HIGH) ──conflicts──> C-14 (keep bench private)
```

### Dependency Notes

- **G-2 requires E-1/E-2/E-6:** a release class cannot be assigned without a paired verdict and a power note; otherwise "Neutral" gets mislabelled "Improved".
- **V-1 requires G-1:** tags are minted by the push path only after the gate; never by hand.
- **S-5 is independent and immediate:** it needs only existing v2 numbers (current rescore 0.8B 21.7%, 2B 28.3%, 4B 47.8%, 9B 54.3%, 27B 69.6% on the runnable v1 subset, and the 0.8B v2 canary ~12-14%); fix it first, it is a one-file content change.
- **C-11 conflicts with the existing "verified wins only" display rule if read as the only disclosure:** resolve by keeping the rule for code blocks and adding the counts table.
- **C-8 registration conflicts with C-14:** registering `heldout_bench_v2` as a Hub Benchmark pushes toward publishing its items. Prefer model-index/`evidence/` and keep a blind slice.
- **E-10/E-9 are infra, not user-facing**, but they gate how often a release verdict can be recomputed; they are on the critical path for G-8.

## MVP Definition

### Launch With (this milestone's release path)

- [ ] V-1, V-2, V-6: per-release tags minted at push time, release map in the index, lifecycle status vocabulary
- [ ] G-1, G-2, G-3, G-4: gate with clean-room smoke test, three release classes, publish-or-log rule, resolution for 27B (no public repo until weights)
- [ ] C-1, C-2, C-3, C-4, C-5, C-6, C-7: one card per repo, no duplicate evidence, why-it-exists as a measured gap, limitations with numbers, metadata, shared definitions linked
- [ ] E-1, E-2, E-3, E-5, E-11: paired test, interval, per-category counts, one primary comparison, v2-calibrated floors
- [ ] S-2, S-3, S-5: replay tab labelled honestly, live/replay distinction, v2 headline

### Add After Validation

- [ ] S-1, S-4, S-8: live inference once serving path is verified end to end; trigger = one successful clean-room call through the Hub-served route
- [ ] E-4, E-9, E-10, V-7: funnel, timing block, prefix cache, release log with non-releases; trigger = timing instrumentation lands
- [ ] G-6, G-7, G-8, C-9, C-10, C-11, E-6: index status table, mutation proof of the new verdict, sequential screening, cost lines, failure counts, power note
- [ ] S-6: last-verified stamp; trigger = S-1 live

### Future Consideration

- [ ] V-8 trend chart, S-7 version selector, E-8 multi-seed spread, E-12 blind v3 slice, C-8 benchmark registration: valuable, high effort or authoring-heavy; defer until the first full cycle of tagged releases exists

## Feature Prioritization Matrix

| Feature | User Value | Cost | Priority |
|---|---|---|---|
| S-5 swap Space headline to v2 | HIGH | LOW | P1 |
| C-1/C-2 dedupe README/MODEL_CARD/evidence | MEDIUM | LOW | P1 |
| G-4 27B no-weights resolution | HIGH | LOW | P1 |
| V-1/V-2 tags + release map | HIGH | LOW | P1 |
| E-1/E-2/E-3 paired test, CIs, category counts | HIGH | LOW-MED | P1 |
| G-1/G-2/G-3 gate, release classes, publish rule | HIGH | MED | P1 |
| C-3/C-4/C-5 card re-cut | HIGH | MED | P1 |
| S-2/S-3 labelled replay | HIGH | LOW | P1 |
| E-11 v2 floors | HIGH | LOW-MED | P1 |
| S-1/S-4/S-8 live Space | HIGH | HIGH | P2 |
| E-4 funnel, E-9/E-10 timing and cache | MEDIUM-HIGH | MED-HIGH | P2 |
| V-7 release log with non-releases, G-6 index table | HIGH (trust) | MED | P2 |
| C-8 verify eval_results rendering | MEDIUM | LOW | P2 |
| G-7 mutation proof of verdict | MEDIUM | MED | P2 |
| S-6 last-verified stamp | MEDIUM | MED | P2 |
| V-8, S-7, E-8, E-12, C-8 registration | MEDIUM | HIGH | P3 |

**Priority key:** P1 must ship in this milestone; P2 should ship when its dependency lands; P3 defer.

## Comparison to Typical Open Releases

| Aspect | Typical well-run release | Vox current state (measured) | Vox target |
|---|---|---|---|
| Version addressing | Per-variant repos, collections, tag or date suffix | No git tags | Tags at push + release map (V-1/V-2) |
| Superseding | `new_version` banner on old repo | None | `new_version` when base changes; lifecycle status line |
| Evaluation display | Table of benchmark scores, sometimes structured metadata | Space headlines saturated v1 (94.2%); 0.8B v2 ~12-14% | Paired v2 results with CI, funnel, per-category counts |
| Non-results | Rarely published | Failed runs blocked from push (good), but invisible | Logged in index with evidence (V-7) |
| Card duplication | One README | README.md + MODEL_CARD.md; root + `evidence/` duplicates | One README, one evidence copy |
| Demo | Gradio/ZeroGPU Space or Inference widget | Static Space, replay-like | Live + labelled replay |

## Sources

- HF Hub model cards (metadata, `base_model`, `new_version`, eval results): https://huggingface.co/docs/hub/model-cards (HIGH, official)
- HF annotated model card: https://huggingface.co/docs/hub/model-card-annotated (HIGH, official)
- HF model release checklist: https://huggingface.co/docs/hub/model-release-checklist (HIGH, official)
- HF evaluation results (`.eval_results`, badges, benchmark registration): https://huggingface.co/docs/hub/eval-results (HIGH, official; marked work-in-progress)
- huggingface_hub repository guide (tags, branches, refs): https://huggingface.co/docs/huggingface_hub/guides/repository (HIGH, official)
- HF ZeroGPU: https://huggingface.co/docs/hub/spaces-zerogpu (HIGH, official); Space sleep/static-Space behaviour from forum threads (LOW, anecdotal)
- Miller, "Adding Error Bars to Evals": https://arxiv.org/html/2411.00640v1 (HIGH for recommendations: SEM, clustered SE, paired differences, power; the small-sample caution is my reading of it)
- Chen et al., "Evaluating Large Language Models Trained on Code" (pass@k estimator): https://arxiv.org/pdf/2107.03374 (HIGH)
- Mitchell et al., "Model Cards for Model Reporting": https://arxiv.org/pdf/1810.03993v1 (HIGH)
- "What's documented in AI? Systematic analysis of 32K AI model cards": https://arxiv.org/pdf/2402.05160 (MEDIUM, via search summary)
- McNemar usage in recent code-generation papers and mlxtend docs: https://rasbt.github.io/mlxtend/user_guide/evaluate/mcnemar (MEDIUM)
- OLMo release practice: https://arxiv.org/html/2402.00838v4, https://huggingface.co/allenai/Olmo-7B (MEDIUM)
- Qwen3 -2507 refresh and design reversal: https://huggingface.co/Qwen/Qwen3-30B-A3B-Instruct-2507, https://www.theregister.com/2025/07/31/alibaba_qwen3_hybrid_thinking/ (MEDIUM)
- Open LLM Leaderboard v2 retiring saturated benchmarks: https://www.deeplearning.ai/the-batch/hugging-face-overhauls-open-llm-leaderboard-with-tougher-benchmarks (MEDIUM, search snippet only)
- Project context read: `.planning/PROJECT.md`, `docs/src/architecture/mens-eval-flywheel-and-lanes-claude-handoff-2026-11-20.md`, `docs/src/reference/mens-models.md`, `mens/config/hub-cards.yaml`

## Open items for the requirements author

1. Decide G-4 vs G-5 for 27B (recommendation: G-4, no public model repo until weights pass the gate).
2. Confirm whether `.eval_results/vox_mens_eval.yaml` renders on the Hub for a non-registered dataset (C-8); if not, switch to `model-index` + evidence link.
3. Decide whether `heldout_bench_v2` items are ever public (C-14) before any benchmark registration work.
4. Confirm the HF Inference Router really serves the merged checkpoints (claimed in `mens-models.md`) before S-1 depends on it.
5. Define the pre-declared primary comparison and one-sided direction for G-2 (improvement vs regression) in `eval-gates-ladder.yaml`.

---
*Feature research for: MENS Hugging Face hub releases (v9.5)*
*Researched: 2026-11-20*
