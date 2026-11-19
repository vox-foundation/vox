---
phase: 12
phase_name: "code-gen-benchmark-plumbing-provenance"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 46
  lessons: 25
  patterns: 16
  surprises: 14
missing_artifacts:
  - "VERIFICATION.md"
  - "UAT.md"
---

# Phase 12 Learnings: code-gen-benchmark-plumbing-provenance

## Decisions

### No serde attribute on LlmConfig.seed
No serde attribute on LlmConfig.seed: serde already maps an absent Option key to None, proven by llm_config_without_seed_key_still_deserializes

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-01-SUMMARY.md

---

### An unknown mens/* id is a hard error before any generation; no other men...
An unknown mens/* id is a hard error before any generation; no other mens/ special case exists

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-02-SUMMARY.md

---

### from-dir samples carry seed 0 (they are not sampled); live samples carry...
from-dir samples carry seed 0 (they are not sampled); live samples carry sample_seed(base, id, i)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-02-SUMMARY.md

---

### classify_generation_error also matches the facade's 'LLM API returned er...
classify_generation_error also matches the facade's 'LLM API returned error (5xx)' and 'HTTP request failed:' strings, not only the plan's 'provider status' forms

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-02-SUMMARY.md

---

### Tracer (Task 1, 561cf8245)
**Tracer (Task 1, 561cf8245):** `with_mens_runs`, `resolve_eval_model` (reuses `bench::resolve_target` and `llm_config_for_spec(.., TaskCategory::CodeGen)`), `Generated`, and `generate_candidate(&LlmConfig, prompt)` returning the provider-reported model string. The unknown-`mens/*` guard is the only mens special case. Tracer gate: a wiremock test shows one POST to `/v1/chat/completions` with `model`, `seed`, `temperature`, `top_p`, `max_tokens` in the body. Tracer verify re-run end to end before expansion.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-02-SUMMARY.md

---

### Seed, validation, classification (Task 2, d3afbe9ff)
**Seed, validation, classification (Task 2, d3afbe9ff):** `--seed` (default 0) and `--top-p` (default 1.0); `sample_seed` is a defactored copy of 12-03's function (pinned vector 780188561 asserted); `validate_sampling` rejects n=0, k=0, k>n, top_p outside (0,1], non-finite or negative temperature; `classify_generation_error` replaces `is_infra_error`; warning for voxlocal without `VOX_LOCAL_ENDPOINT` (11434 Ollama vs 11435 `vox mens serve`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-02-SUMMARY.md

---

### draw_samples (Task 3, 1bec70013)
**draw_samples (Task 3, 1bec70013):** the sample loop is extracted with its semantics intact (spend cap before each sample, infra `continue`d without touching attempts, model failures as all-false attempts, no early break). 409 "Loaded model is" aborts the run. Each sample yields a `SampleRecord` with seed, code, `provider_model` and a `classify_failure` class (`parse_ok` from an in-process lex+parse, `timed_out` from "timed out" in the detail). Fixtures are sorted by id in `select_fixtures`; records are sorted by `(fixture_id, sample_index)`. `VerifyOutcome`/`RunOutcome` carry `assertion_failed`, set from `assertion failed: ` in the run stage's stderr. `run()` still writes today's report (12-07 swaps in the typed one).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-02-SUMMARY.md

---

### CUDA plugin gets the identical seed/top_p/tie-break change as metal, so ...
CUDA plugin gets the identical seed/top_p/tie-break change as metal, so no D-12 deviation is recorded

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-03-SUMMARY.md

---

### sample_seed(7, "001", 0) is pinned to 780188561 for 12-02's defactored c...
sample_seed(7, "001", 0) is pinned to 780188561 for 12-02's defactored copy

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-03-SUMMARY.md

---

### An inference error stays in the sample list as pass:false so every item ...
An inference error stays in the sample list as pass:false so every item has exactly n samples

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-03-SUMMARY.md

---

### Tracer (Task 1, a5c8ebc96)
**Tracer (Task 1, a5c8ebc96):** `seed: Option<u64>` / `top_p: Option<f32>` on `ChatCompletionRequest` and `GenerateRequest` (`serde(default)`), copied onto `InferenceRequest`, emitted as `"seed"` / `"top_p"` in `inference_payload`, parsed by the metal `PromptRequest`, and consumed by `generation_rng(seed)` replacing `thread_rng()`. Tracer feedback gate: both `<verify>` commands re-run end to end and pass, so expansion proceeded.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-03-SUMMARY.md

---

### Samplers (Task 2, 18c1662f3)
**Samplers (Task 2, 18c1662f3):** `ranked_logits` ranks by logit desc then token index asc; nucleus keeps the shortest best-first prefix whose mass reaches `top_p` (at least one token) and renormalizes; non-finite or non-positive `top_p` is treated as 1.0. The CUDA plugin got the Task 1 plumbing plus the Task 2 sampler in one commit.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-03-SUMMARY.md

---

### eval-local (Task 3, c3c75fc69)
**eval-local (Task 3, c3c75fc69):** `per_problem_pass_at_k` delegates to `pass_at_k(n, c, k)`; `sample_seed` is BLAKE3 over length-prefixed framing, masked to 31 bits; `run_eval_local` takes `k` after `samples` and rejects `k < 1` or `k > samples` before any model work. The single-sample block is replaced by an n-sample loop; report keys `pass_rate_at_1` / `pass_rate_at_k` are kept (now means of per-problem estimates), and `n`, `k`, per-item `n`/`c`, and per-sample `seed` are recorded. `passed_at_1` / `passed_at_k` report fields are now f64 sums of estimates (no other reader in the tree).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-03-SUMMARY.md

---

### Pinned constant for 12-02
Pinned constant for 12-02: `sample_seed(7, "001", 0) == 780188561`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-03-SUMMARY.md

---

### Completeness lives in a private gate_reasons fn called by assemble and r...
Completeness lives in a private gate_reasons fn called by assemble and recheck_complete, instead of two copies of the rule

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-04-SUMMARY.md

---

### Samples with n < k or n == 0 are excluded from scoring and listed in uns...
Samples with n < k or n == 0 are excluded from scoring and listed in unscored_fixture_ids; corpus_score's k > n panic is untouched

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-04-SUMMARY.md

---

### RunProvenance (+ RunIdentity, SamplingRecord, MensLineage) with missing_...
`RunProvenance` (+ `RunIdentity`, `SamplingRecord`, `MensLineage`) with `missing_required()`: fixed declaration order, blank/absent counts as missing, `mens/*` ids need every lineage field, external ids need provider, request_date and at least one provider-reported model. `corpus_manifest_hash` is informational. No credential field anywhere (`api_key` grep: 0).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-04-SUMMARY.md

---

### EvalCorpusReport::assemble reuses bootstrap_ci / min_detectable_differen...
`EvalCorpusReport::assemble` reuses `bootstrap_ci` / `min_detectable_difference` / `score_corpus` (no new statistics), overwrites `score.n_infra_errors` with the real count, sorts fixtures and samples, builds the failure breakdown and non-infra compile rate.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-04-SUMMARY.md

---

### run_complete false with a named reason for
`run_complete` false with a named reason for: spend cap, no samples planned, infra rate above ceiling (inclusive at the ceiling), each missing provenance field, verifier mismatch, no scored problems.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-04-SUMMARY.md

---

### recheck_complete() recomputes from stored provenance and counts; a forge...
`recheck_complete()` recomputes from stored provenance and counts; a forged `run_complete: true` with identity removed, spend cap, mismatch, missing pass@k or inflated infra count rechecks false.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-04-SUMMARY.md

---

### Edges: empty set gives None stats, single problem gives degenerate CI wi...
Edges: empty set gives `None` stats, single problem gives degenerate CI with MDD 0.25, `n < k` fixtures are listed unscored and never panic.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-04-SUMMARY.md

---

### Floor check uses first_attempt_difference computed as (arm-only minus ba...
Floor check uses first_attempt_difference computed as (arm-only minus baseline-only passes) / n_aligned, one division of an exact integer

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-05-SUMMARY.md

---

### Duplicate arm labels are refused (baseline lookup is by label)
Duplicate arm labels are refused (baseline lookup is by label)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-05-SUMMARY.md

---

### compare::compare partitions arms by recheck_complete(), errors on a miss...
`compare::compare` partitions arms by `recheck_complete()`, errors on a missing/excluded baseline, no challenger, differing `problem_set_hash` or `k`, the same `run_id` twice, or no fixtures scored by both arms. Fixtures align by id in a `BTreeMap`; claims and Holm run in input order (`holm_reject` sorts stably, so equal p-values keep it).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-05-SUMMARY.md

---

### Verdict: Unresolved when |Δ first attempt| < min_detectable_difference(n...
Verdict: `Unresolved` when `|Δ first attempt| < min_detectable_difference(n_aligned)` (strict `<`, so the floor itself is resolved), else `Significant` if Holm rejects, else `NotSignificant`. Conditions stay separate (`condition_arm` / `condition_baseline`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-05-SUMMARY.md

---

### CLI wrapper loads files through read_utf8_path_capped, refuses a wrong s...
CLI wrapper loads files through `read_utf8_path_capped`, refuses a wrong `schema_version` by file name, requires `--baseline` to be one of the given paths, prints arms/exclusions/claims, and `--output` writes the `Comparison` (with alpha, reps, seed) as JSON.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-05-SUMMARY.md

---

### model.eval-compare added to catalog.v1.yaml; registry, capability regist...
`model.eval-compare` added to `catalog.v1.yaml`; registry, capability registry, CLI surface doc, operations inventory and the catalog-paths baseline regenerated by their generators.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-05-SUMMARY.md

---

### read_training_lineage falls back to run_dir/training_manifest.json when ...
read_training_lineage falls back to run_dir/training_manifest.json when the adapter sits in final/ and has no manifest beside it

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-06-SUMMARY.md

---

### Cloud cards keep empty lineage (recorded deviation in the plan); eval re...
Cloud cards keep empty lineage (recorded deviation in the plan); eval refuses to label cloud-trained rows until the pod can report the consumed file hash

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-06-SUMMARY.md

---

### AdapterCard gained five #[serde(default)] lineage fields
`AdapterCard` gained five `#[serde(default)]` lineage fields. `validate()` is unchanged (serve depends on it); `validate_lineage()` rejects empty fields, non-lowercase-hex or non-64-char digests, and a `train_data_sha256` equal to the SHA-256 of empty input. Legacy cards parse and then fail lineage.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-06-SUMMARY.md

---

### read_training_lineage hashes the adapter and the file named by train_fil...
`read_training_lineage` hashes the adapter and the file named by `train_file` in the run's `training_manifest.json` (raw bytes, no normalization); the manifest digest is BLAKE3 over its raw bytes.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-06-SUMMARY.md

---

### train_arm::run_train writes the card in the post-success block; failure ...
`train_arm::run_train` writes the card in the post-success block; failure only prints "eval will refuse to label this run" and never fails training. An unpinned base model (no `@revision`) produces no card.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-06-SUMMARY.md

---

### grammar_fingerprint() returns (compute_ebnf_hash(), "ebnf-sha256") throu...
`grammar_fingerprint()` returns `(compute_ebnf_hash(), "ebnf-sha256")` through the existing vox-ml-cli to vox-compiler edge; no new crate edge.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-06-SUMMARY.md

---

### check_for_eval refuses missing adapter, missing/unreadable/invalid card,...
`check_for_eval` refuses missing adapter, missing/unreadable/invalid card, invalid lineage and a re-hashed adapter that differs from the card; syntax drift is reported as `Some(true)` without refusing.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-06-SUMMARY.md

---

### verifier_identity is all-or-nothing
verifier_identity is all-or-nothing: if `--version` or the binary read fails, vox_git_sha, vox_binary_sha256, verifier_version and verifier_matches_harness are all reported missing

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-07-SUMMARY.md

---

### config_digest takes a ConfigKnobs struct instead of eleven positional ar...
config_digest takes a ConfigKnobs struct instead of eleven positional arguments (transposition-prone strings)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-07-SUMMARY.md

---

### from-dir rows get sampling n=1, k=1, seed 0 and no provider, so they are...
from-dir rows get sampling n=1, k=1, seed 0 and no provider, so they are honestly incomplete and never write the scoreboard

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-07-SUMMARY.md

---

### Tracer (d8876169e)
**Tracer (d8876169e):** `run()` builds `RunProvenance` with `collect_provenance`, hands it and the `DrawOutcome` to `finalize_report` (the only caller of `EvalCorpusReport::assemble`), prints pass@1/pass@k with CIs, `seed N (enforced)` or `(requested; provider may ignore)`, and each incomplete reason. `seed_enforced` is true only for `voxlocal`. `provider_reported_models` is the sorted, de-duplicated set of per-sample `provider_model` (12-02 handoff). New `--runner` (defaults to `github-hosted` under `GITHUB_ACTIONS=true`). `identity` and `preregistration_id` stay `None`, so every report is honestly incomplete and names both until 12-09.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-07-SUMMARY.md

---

### Identity collectors (5c150e8c6)
**Identity collectors (5c150e8c6):** `problem_set_hash` is SHA-256 via `interop_sha256` over id-sorted fixtures, each field a u64-LE length plus raw bytes of id, spec.toml, reference.vox, tests.vox (order-independent, duplicate id is an error, `("ab","c")` vs `("a","bc")` cannot collide). `config_digest` is BLAKE3 over model id, harness, n, k, condition, context hash, temperature, top_p, max_tokens, seed and problem set hash (12-02 handoff: seed and top_p now included); `sha2` is no longer used in eval_corpus.rs. Verifier identity comes from `<vox_bin> --version` plus SHA-256 of the binary; `verifier_matches_harness = version == VOX_VERSION`. Syntax fingerprint and kind come from `grammar_fingerprint()`. `Fixture` gained `spec_path` and `reference_path`; `files.reference` is now required.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-07-SUMMARY.md

---

### Card check and write-back guard (15a70e49a)
**Card check and write-back guard (15a70e49a):** `mens_lineage` wraps `check_for_eval`; problems print as a red "mens label refused:" list and leave `MensLineage` unset, so the gate marks the run incomplete (no mens special case in scoring). Syntax drift is printed. `should_write_back(run_complete, measured, no_write_back)` wraps the scoreboard upsert; skipped runs print `model_scoreboard not updated: run incomplete` (or the ingested reason).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-07-SUMMARY.md

---

### repair turns reuse the single llm_chat path (generate_chat), sending [us...
repair turns reuse the single llm_chat path (generate_chat), sending [user prompt, assistant raw completion, user feedback]; no vendor code

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-08-SUMMARY.md

---

### a repair candidate is classified by the same classify_outcome as the ori...
a repair candidate is classified by the same classify_outcome as the original; only ParseError/CheckError/RuntimeError/WrongAnswer are repaired, and a pass, cheat or timeout ends that sample's repairs

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-08-SUMMARY.md

---

### feedback_turns is always part of config_digest (so K=0 digests differ fr...
feedback_turns is always part of config_digest (so K=0 digests differ from pre-12-08 digests; schema-2 reports are new in phase 12)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-08-SUMMARY.md

---

### from-dir runs record feedback_turns 0 (pre-generated solutions cannot be...
--from-dir runs record feedback_turns 0 (pre-generated solutions cannot be re-prompted)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 12-08-SUMMARY.md

---

### Tracer (871cee721)
**Tracer (871cee721):** `--feedback-turns K` (validated 0..=3). In `draw_samples`, after the original attempt is verified, a failed live sample (parse, check, runtime or wrong-answer) loops over turns 1..=K: messages are the original prompt, the previous raw completion as the assistant, and `feedback_prompt(previous detail)`; the config is cloned with `seed = repair_seed(base, fixture, turn, sample_index)`; the new candidate is verified with the same closure and classified with the same `classify_outcome`. Repair tokens and cost go on the sample and into the spend counter (cap hit ends the loop and marks the run incomplete). Pass, cheat or timeout ends a sample's repairs; an infra error increments `n_infra_errors` and ends them; the remaining `passed_by_turn` entries carry the last value forward. K = 0 sends no repair request. The `AttemptOutcome` is never replaced.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-08-SUMMARY.md

---

### Per-turn statistics (a5dc95191)
**Per-turn statistics (a5dc95191):** `assemble` reads K from `provenance.sampling.feedback_turns`; for each turn and each scorable fixture the value is (verified samples true at index t) / (verified samples), then `arm_stats`. Unscored fixtures are excluded; K = 0 yields an empty vec. `print_summary` prints a line per turn.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 12-08-SUMMARY.md

## Lessons

### 1. [Rule 2 - Test-first policy] Added a test to vox-search embedding_env...
**1. [Rule 2 - Test-first policy] Added a test to vox-search embedding_env.rs**

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-01-SUMMARY.md

---

### Found during
**Found during:** Task 2 commit (lefthook `tdd-guard`)

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-01-SUMMARY.md

---

### Issue: the mechanical seed: None edit touched a file whose pre-existing ...
**Issue:** the mechanical `seed: None` edit touched a file whose pre-existing `pub fn embedding_config_from_env` had no `#[test]`, and the hook blocked the commit.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-01-SUMMARY.md

---

### Fix: added embedding_config_never_carries_a_seed (passes). No hook bypas...
**Fix:** added `embedding_config_never_carries_a_seed` (passes). No hook bypass.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-01-SUMMARY.md

---

### 1. [Rule 1 - Bug] Infra classification matches the facade's real strings...
**1. [Rule 1 - Bug] Infra classification matches the facade's real strings.** The plan's marker list only had `provider status 5xx`, but `llm_chat` formats provider failures as `LLM API returned error (<code>): ...` and transport failures as `HTTP request failed: ...` (`vox-actor-runtime/src/llm/chat.rs` `map_egress_error`). Added `returned error (500|502|503|504)` and `http request failed:` markers; tests cover both forms. The 409 check uses the "Loaded model is" substring, which holds in either wrapping.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-02-SUMMARY.md

---

### 2. [Structure] pub assertion_failed: bool count is 2, not 1. The plan as...
**2. [Structure] `pub assertion_failed: bool` count is 2, not 1.** The plan asked for the field on both `VerifyOutcome` and `RunOutcome` but its acceptance grep expects a count of 1. Both structs are public with public fields, so the field is `pub` on each; the literal grep reads 2.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-02-SUMMARY.md

---

### 3. [Process] Tasks 2 and 3 were first built and tested together. The hos...
**3. [Process] Tasks 2 and 3 were first built and tested together.** The host's `syspolicyd` stalled every freshly linked binary (build scripts and test binaries sat at 0% CPU for about 13 minutes, see `~/dev/memory/macos-syspolicyd-stalls-fresh-test-binaries.md`), so the first successful build compiled Tasks 1-3 as one file state. After the stall cleared, each task was re-verified in isolation (Task 1: 5 tests; Task 2: 4 tests) before its commit; Task 3 was committed from the file state that passed all 19 tests, and `git show HEAD:...eval_corpus.rs` is byte-identical to that tested state. The tests were written alongside the code in each task rather than observed red first (red was a compile error until the code existed); the two mutation proofs supply the failing-test evidence.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-02-SUMMARY.md

---

### 4. [Scope] config_digest not extended. It does not include --seed or --t...
**4. [Scope] `config_digest` not extended.** It does not include `--seed` or `--top-p`. The plan did not ask for it; 12-07 owns row identity and provenance. Flagged for that plan.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-02-SUMMARY.md

---

### 1. [Rule 3 - Blocking] eval-local tests need --features gpu. The plan's ...
**1. [Rule 3 - Blocking] eval-local tests need `--features gpu`.** The plan's Task 3 `<verify>` runs `cargo nextest run -p vox-ml-cli --lib -E 'test(eval_local::...)'` without the feature, but `mod eval_local` is `#[cfg(feature = "gpu")]` in `mens/mod.rs`, so that command runs 0 tests. All Task 3 test runs and the mutation proof used `--features gpu`. No code change.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-03-SUMMARY.md

---

### 2. [Structure] Tracer commit split reconstructed. Task 1 and Task 2 both...
**2. [Structure] Tracer commit split reconstructed.** Task 1 and Task 2 both edit the metal `inference.rs`. Task 2's metal edits were written before the Task 1 commit, so the Task 1 file state was reconstructed (Task 2 hunks removed), committed, and Task 2 re-applied from a saved copy. Both commits are self-consistent; Task 1's metal build carried an unused `top_p` parameter warning, by design per the plan.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-03-SUMMARY.md

---

### 3. [Rule 3 - Blocking] Existing test constructions updated. Three ChatCo...
**3. [Rule 3 - Blocking] Existing test constructions updated.** Three `ChatCompletionRequest { .. }` literals in `handlers.rs` tests and two `.generate(..)` calls in the metal live tests needed the new fields/arguments (`seed: None, top_p: None`, `None, None`).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-03-SUMMARY.md

---

### 1. [Structure] One gate call site instead of two. The plan expected miss...
**1. [Structure] One gate call site instead of two.** The plan expected `missing_required()` twice in report.rs (assemble gate plus recheck). Both now share `gate_reasons`, so the count is 1 -> 0 -> 1 rather than 2 -> 1 -> 2. Same property (deleting the term fails `gate_rejects_each_cause`, and also weakens recheck), less duplicated rule.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-04-SUMMARY.md

---

### 2. [Test-first] Task 1 provenance tests. The repo's tdd-guard blocked th...
**2. [Test-first] Task 1 provenance tests.** The repo's tdd-guard blocked the first commit (a pub fn without a `#[test]` in `provenance.rs`), so two small provenance tests were added to Task 1; Task 2's tests were written red first (7 of 11 failing) before `missing_required` was completed. Task 1's tracer and Task 3's edge tests passed on first run because the implementation they pin already existed from Tasks 1/2.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-04-SUMMARY.md

---

### 3. [Formatting] The lefthook fmt-fix hook reformatted report.rs in the T...
**3. [Formatting] The lefthook fmt-fix hook reformatted `report.rs` in the Task 3 commit (edition 2024 style, which my first `rustfmt --edition 2021` pass did not match).** Content unchanged; working tree matches HEAD. Later passes used `--edition 2024`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-04-SUMMARY.md

---

### 1. [Rule 1 - Bug] Floor comparison could land one ulp under the floor. p...
**1. [Rule 1 - Bug] Floor comparison could land one ulp under the floor.** `paired_compare().difference` is `rate(a) - rate(b)`; 120/200 - 100/200 is 0.09999999999999998 in f64, which would have labelled a difference exactly at the 0.10 floor `Unresolved`. `first_attempt_difference` is therefore computed as `(arm_only - baseline_only) / n_aligned` (one correctly rounded division of an exact integer); `paired_compare` still supplies the McNemar p-value. Covered by `compare_floor_boundary_is_inclusive` (both the 20/0 and the 120-vs-100 construction).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-05-SUMMARY.md

---

### 2. [Test-first] Task 1 tests were not red first. The tracer test and the...
**2. [Test-first] Task 1 tests were not red first.** The tracer test and the implementation were written together and passed on first run. Task 2 was red-first for the one missing behavior: `compare_refuses_duplicate_run` failed (returned Ok) before the run_id check was added. `compare_empty_and_single_fixture_edges` initially failed on my own wrong expectation (one fixture with a 1.0 difference is resolved, p = 1.0, so `NotSignificant`, not `Unresolved`); the test was corrected, not the code.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-05-SUMMARY.md

---

### 3. [Scope] Two extra files in the Task 3 commit. contracts/reports/opera...
**3. [Scope] Two extra files in the Task 3 commit.** `contracts/reports/operations-catalog-inventory.v1.json` (regenerated by `ci operations-verify`: 1299 to 1300 operations) is not in the plan's file list. The paths baseline gained `model/bench` as well as `model/eval-compare`: `bench` was already registered in the committed `mod.rs` but absent from the fixture (pre-existing drift), and the fixture is regenerated wholesale by its update switch.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-05-SUMMARY.md

---

### 4. [Addition] Duplicate arm labels are refused (two lines in compare), s...
**4. [Addition] Duplicate arm labels are refused** (two lines in `compare`), since the baseline is resolved by label.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-05-SUMMARY.md

---

### Task split detail
**Task split detail:** `check_for_eval`/`EvalCardCheck` were held back from the Task 1 commit and added in Task 2, so each commit contains only its task's behavior. No change in final content.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-06-SUMMARY.md

---

### Manifest lookup fallback (Rule 2)
**Manifest lookup fallback (Rule 2):** the plan says read the manifest "next to the adapter"; the adapter may be in `<run>/final/`, where no manifest is written, so the lookup also tries `run_dir`. No other deviation.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-06-SUMMARY.md

---

### 1. [Test-first] Tests were written together with the code and observed g...
**1. [Test-first] Tests were written together with the code and observed green, not red.** The new tests need the new types to compile, so red was a compile error. The write-back mutation proof supplies the failing-test evidence for the guard; the other guards (duplicate id, boundary collision, byte sensitivity, drift) assert both directions in one test.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-07-SUMMARY.md

---

### 2. [Structure] config_digest(&ConfigKnobs) instead of positional argumen...
**2. [Structure] `config_digest(&ConfigKnobs)` instead of positional arguments.** Same inputs as the plan lists; a struct avoids eleven transposable arguments. The old `config_digest_is_stable_and_covers_every_knob` test was replaced by `config_digest_covers_every_knob`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-07-SUMMARY.md

---

### 3. [Structure] cargo_version is env!("CARGO_PKG_VERSION") (the vox-cli p...
**3. [Structure] `cargo_version` is `env!("CARGO_PKG_VERSION")` (the vox-cli package version), as the plan specifies**, not the output of `cargo --version`. The design doc's wording for the Language group says "Cargo version"; whether that means the toolchain is unresolved and left for the reviewer.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-07-SUMMARY.md

---

### 4. [Scope] --from-dir model ids starting with mens/ also go through the ...
**4. [Scope] `--from-dir` model ids starting with `mens/` also go through the card check** (the lineage lookup is keyed on the id, not on measured mode). A from-dir row is incomplete regardless (no provider), so this only adds the refusal message.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-07-SUMMARY.md

---

### A pre-existing uncommitted change to crates/vox-cli/src/commands/model/c...
A pre-existing uncommitted change to `crates/vox-cli/src/commands/model/classify.rs` (another session) was left untouched and unstaged.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 12-08-SUMMARY.md

## Patterns

### top_p-shaped optional field
top_p-shaped optional field: Option + skip_serializing_if, so unset callers keep a byte-identical wire body

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 12-01-SUMMARY.md

---

### one resolve_target + llm_config_for_spec pair shared by bench and eval-c...
one resolve_target + llm_config_for_spec pair shared by bench and eval-corpus

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 12-02-SUMMARY.md

---

### sample loop as a pure-ish unit taking a verify closure, so tests need no...
sample loop as a pure-ish unit taking a verify closure, so tests need no vox binary

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 12-02-SUMMARY.md

---

### seed present => StdRng::seed_from_u64; absent => from_entropy (no behavi...
seed present => StdRng::seed_from_u64; absent => from_entropy (no behavior change for existing callers)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 12-03-SUMMARY.md

---

### tied logits ranked by (logit desc, token index asc)
tied logits ranked by (logit desc, token index asc): a total order, so a seeded draw never depends on sort_unstable internals

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 12-03-SUMMARY.md

---

### pass@1/pass@k are means of per-problem unbiased estimates from vox_eval:...
pass@1/pass@k are means of per-problem unbiased estimates from vox_eval::corpus_score::pass_at_k

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 12-03-SUMMARY.md

---

### one completeness gate (gate_reasons) shared by assemble and recheck_comp...
one completeness gate (gate_reasons) shared by assemble and recheck_complete

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 12-04-SUMMARY.md

---

### required provenance fields are Option with no serde default
required provenance fields are Option with no serde default: absent key == None == reported missing

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 12-04-SUMMARY.md

---

### eligibility re-derived from provenance via recheck_complete(), never fro...
eligibility re-derived from provenance via recheck_complete(), never from the stored flag

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 12-05-SUMMARY.md

---

### verdict = floor first (strict <), then Holm; no interval-overlap input a...
verdict = floor first (strict <), then Holm; no interval-overlap input anywhere

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 12-05-SUMMARY.md

---

### hashing only through vox_crypto facades (interop_sha256 for files, secur...
hashing only through vox_crypto facades (interop_sha256 for files, secure_hash/BLAKE3 for the manifest digest); no direct sha2/blake3

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 12-06-SUMMARY.md

---

### card is validated (validate + validate_lineage) before anything is writt...
card is validated (validate + validate_lineage) before anything is written; an untraceable run leaves no card

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 12-06-SUMMARY.md

---

### provenance fields that cannot be observed stay None and are named by the...
provenance fields that cannot be observed stay None and are named by the one gate, never defaulted

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 12-07-SUMMARY.md

---

### hashing only through vox_crypto::facades
hashing only through vox_crypto::facades: interop_sha256 for externally checkable values, secure_hash (BLAKE3) for internal digests

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 12-07-SUMMARY.md

---

### repair outcomes live in a separate field; the AttemptOutcome that feeds ...
repair outcomes live in a separate field; the AttemptOutcome that feeds pass@1/pass@k is always the original attempt

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 12-08-SUMMARY.md

---

### trajectory padded with resize(K+1, last)
trajectory padded with resize(K+1, last): carries a pass forward (cumulative) and carries the last value after an early stop

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 12-08-SUMMARY.md

## Surprises

### Estimate variance on 12-01
Estimate variance on 12-01: planned 70000 tokens vs 1700 actual (0.02x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 12-01-PLAN.md, 12-01-SUMMARY.md

---

### 1. [Rule 2 - Test-first policy] Added a test to vox-search embedding_env...
**1. [Rule 2 - Test-first policy] Added a test to vox-search embedding_env.rs**

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 12-01-SUMMARY.md

---

### Estimate variance on 12-02
Estimate variance on 12-02: planned 110000 tokens vs 9300 actual (0.08x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 12-02-PLAN.md, 12-02-SUMMARY.md

---

### 1. [Rule 1 - Bug] Infra classification matches the facade's real strings...
**1. [Rule 1 - Bug] Infra classification matches the facade's real strings.** The plan's marker list only had `provider status 5xx`, but `llm_chat` formats provider failures as `LLM API returned error (<code>): ...` and transport failures as `HTTP request failed: ...` (`vox-actor-runtime/src/llm/chat.rs` `map_egress_error`). Added `returned error (500|502|503|504)` and `http request failed:` markers; tests cover both forms. The 409 check uses the "Loaded model is" substring, which holds in either wrapping.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 12-02-SUMMARY.md

---

### Estimate variance on 12-03
Estimate variance on 12-03: planned 100000 tokens vs 7800 actual (0.08x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 12-03-PLAN.md, 12-03-SUMMARY.md

---

### 1. [Rule 3 - Blocking] eval-local tests need --features gpu. The plan's ...
**1. [Rule 3 - Blocking] eval-local tests need `--features gpu`.** The plan's Task 3 `<verify>` runs `cargo nextest run -p vox-ml-cli --lib -E 'test(eval_local::...)'` without the feature, but `mod eval_local` is `#[cfg(feature = "gpu")]` in `mens/mod.rs`, so that command runs 0 tests. All Task 3 test runs and the mutation proof used `--features gpu`. No code change.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 12-03-SUMMARY.md

---

### 3. [Rule 3 - Blocking] Existing test constructions updated. Three ChatCo...
**3. [Rule 3 - Blocking] Existing test constructions updated.** Three `ChatCompletionRequest { .. }` literals in `handlers.rs` tests and two `.generate(..)` calls in the metal live tests needed the new fields/arguments (`seed: None, top_p: None`, `None, None`).

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 12-03-SUMMARY.md

---

### Estimate variance on 12-04
Estimate variance on 12-04: planned 90000 tokens vs 18800 actual (0.21x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 12-04-PLAN.md, 12-04-SUMMARY.md

---

### Estimate variance on 12-05
Estimate variance on 12-05: planned 90000 tokens vs 7500 actual (0.08x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 12-05-PLAN.md, 12-05-SUMMARY.md

---

### 1. [Rule 1 - Bug] Floor comparison could land one ulp under the floor. p...
**1. [Rule 1 - Bug] Floor comparison could land one ulp under the floor.** `paired_compare().difference` is `rate(a) - rate(b)`; 120/200 - 100/200 is 0.09999999999999998 in f64, which would have labelled a difference exactly at the 0.10 floor `Unresolved`. `first_attempt_difference` is therefore computed as `(arm_only - baseline_only) / n_aligned` (one correctly rounded division of an exact integer); `paired_compare` still supplies the McNemar p-value. Covered by `compare_floor_boundary_is_inclusive` (both the 20/0 and the 120-vs-100 construction).

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 12-05-SUMMARY.md

---

### Estimate variance on 12-06
Estimate variance on 12-06: planned 90000 tokens vs 14000 actual (0.16x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 12-06-PLAN.md, 12-06-SUMMARY.md

---

### Manifest lookup fallback (Rule 2)
**Manifest lookup fallback (Rule 2):** the plan says read the manifest "next to the adapter"; the adapter may be in `<run>/final/`, where no manifest is written, so the lookup also tries `run_dir`. No other deviation.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 12-06-SUMMARY.md

---

### Estimate variance on 12-07
Estimate variance on 12-07: planned 110000 tokens vs 9100 actual (0.08x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 12-07-PLAN.md, 12-07-SUMMARY.md

---

### Estimate variance on 12-08
Estimate variance on 12-08: planned 80000 tokens vs 7700 actual (0.10x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 12-08-PLAN.md, 12-08-SUMMARY.md

