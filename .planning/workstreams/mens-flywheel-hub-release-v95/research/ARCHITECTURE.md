# Architecture Patterns

**Domain:** MENS train/eval flywheel, truthful lane taxonomy, Hub release pipeline (milestone v9.5)
**Researched:** 2026-11-20
**Method:** Codebase-integration study of the working tree (not HEAD). Every claim below was read from disk; handoff claims are tagged CONFIRMED / PARTIAL / REFUTED.
**Handoff:** `docs/src/architecture/mens-eval-flywheel-and-lanes-claude-handoff-2026-11-20.md`
**Live state at research time:** PID 34157 (`eval-reasoning-compare`, 27B) still running, etime 11:35:32. Not touched.

## Recommended Architecture (what the milestone adds to what exists)

```text
 corpus mix (vox-corpus)  ->  train (candle plugin, per-backend)  ->  host post-train (ml-cli train_arm)
   MixRunReport.lane_counts      run_summary.json (plugin)             train-record.json, adapter_card.json
   (already computed)            training_manifest.json (plugin)       (spoke/train_lanes written EMPTY today)
                                                                              |
 eval: eval_local / reasoning_eval / reasoning_compare / eval_collateral      v
   -> plugin run_inference (JSON in/out)  -> metrics.rs verify  -> report JSON -> gate_receipt -> hub_sync stage -> HF repo (+v<N> tag) -> index repo + static Space
```

The five integration seams: (1) plugin `generate()` JSON response, (2) `metrics.rs` verdict `checks`, (3) host-side `train_record.rs`/`train_arm.rs` run stamping, (4) `hub_sync::stage_publish_bundle` + `hub_card`, (5) `eval_spoke::run_agent_suite` policy closure. Most "new" work is additive keys on existing JSON documents plus 3-4 small new modules.

## 1. Eval timing + prefix KV cache

### What exists (verified)
- **No timing anywhere in the plugin response.** `crates/vox-plugin-mens-candle-metal/src/inference.rs:909-913` returns exactly `{"generated_text","prompt_tokens"}` (prompt re-tokenized a second time at :911). CUDA sibling: `crates/vox-plugin-mens-candle-cuda/src/inference.rs:573-580`, `generate` at :584. CONFIRMED.
- **`generate()` builds a fresh cache per call**: `inference.rs:945-947` `model.empty_cache()`; `model.rs:1917-1923` constructs `Vec<Qwen35LayerCache>` of `Full(None)/Linear(None)`. PARTIAL vs handoff: this is not a "reset bug" of a retained cache, there is no retained cache at all. The redundancy (shared system-prompt prefill) is real; the fix is a new feature. Same in CUDA (:608) and in batch path `batch_decode.rs:280` (metal only; CUDA has no `batch_decode.rs`).
- The engine struct that would hold a prefix snapshot is `InferenceEngine` (`inference.rs:28-39`), kept alive across calls behind `CandleModel.engine: Mutex<..>` (`model.rs` ~2310, `backend.rs:107-126` "reuse the in-memory InferenceEngine"). The stale comment at `eval_local.rs:239-241` ("rebuilds the engine from disk per call") is wrong; the engine is loaded once per `with_backend` (`reasoning_eval.rs:256-300`).
- `Qwen35LayerCache` and `FullKvCache` derive `Clone` (`model.rs:2290-2306`) but `Tensor::clone` shares storage and the full-attention path mutates in place via `slice_set` (`model.rs:589-590`). A snapshot must be a deep copy or the snapshot is silently corrupted by the next request. Linear (DeltaNet) state is a recurrent state: it cannot be truncated or rewound, only snapshotted at the exact prefix boundary.
- eval order already sorts prompts lexicographically (`eval_local.rs:221, 458`; `eval_local_prompt.rs:37`) so shared prefixes are adjacent. `prompt` carries the whole assembled prompt (no `system` key, `eval_local.rs:482-487`), so prefix detection must be done engine-side by token LCP/boundary, not by a new request field.

### Checkpoint cache key (MUST stay untouched)
`ItemCheckpoint` (`item_checkpoint.rs:214-372`): header key = BLAKE3 of `inputs`; per-item key = BLAKE3 of the **exact `prompt_json` string** (`item_hash` :375-377); stored value is the whole raw response string.
- eval_local header inputs (`eval_local.rs:396-416`): `scorer` ("eval_local verify_completion v1"), `prompt_protocol`, `model` (`model_identity` :195, includes adapter weights/manifest BLAKE3 + `adapter_serving` = `ADAPTER_SERVING` "nf4_base_plus_lora_arm v2", :188), `base_only`, `bench_manifest_blake3`, `max_tokens`, `temperature`, `samples`, `seed_base`.
- reasoning header inputs (`item_checkpoint.rs:57-84`): `scorer` (`REASONING_SCORER` :31), `model`, `bench`, `max_tokens`, `temperature`, `seed`, `items_blake3`, plus `settings.checkpoint_protocol_fields()` (decode protocol F32/BF16). Pinned by test `the_f32_checkpoint_key_is_unchanged` (:725-757).
- Consequences for the plan:
  1. Never add a key to the request JSON (`eval_local.rs:482`, `reasoning_eval.rs:549 prompt_request`): it changes every `item_hash`, invalidating all `.ckpt.jsonl`.
  2. Never change the `scorer` strings, `ADAPTER_SERVING`, or `checkpoint_inputs` fields. Response fields are safe to add: responses are opaque strings, readers use `generated_text` only (`eval_local.rs:494`, `reasoning_eval.rs:657-664 usable_text`, `hub_card/facts.rs:84-104 read_ckpt`). Resumed items simply lack timing, so timing aggregation must be `cached_items` vs `generated_items` aware.
  3. **Numerics are the hidden trap.** Repo doctrine (`decode_protocol.rs` header): "A later numerics change gets a new tag value, never a redefinition", and `mens/config/decode-identity.yaml` records bitwise-identity evidence for batching. Prefill split at a prefix boundary can change float accumulation vs a single-pass prefill; at temperature 0 that can flip tokens. Prefix reuse is only "zero invalidation" if proven bitwise-identical (CPU tiny-model test is GPU-free; Metal proof needs the GPU). If not identical, the cache must ship behind a new decode-protocol tag, which by design changes the checkpoint key. Decide this before scheduling the 27B run.

### New vs modified
| Item | Kind | File |
|---|---|---|
| timing fields (`completion_tokens`, `prefill_ms`, `decode_ms`, `total_ms`, `*_tok_s`, `stop_reason`, `kv_prefix_cache_hit_tokens`) | MODIFY | metal `inference.rs` (`generate`, `generate_from_json` :843-915), cuda `inference.rs` (mirror) |
| prefix snapshot field + deep-copy helper | MODIFY | `InferenceEngine` in both `inference.rs`; helper in `model.rs` (dirty: cuda `model.rs` already +107 lines) |
| batch path timing | OPTIONAL MODIFY | `batch_decode.rs:280` (metal only) |
| host timing aggregation (`model_load_ms`, `generation_wall_secs`, `verify_wall_secs`, `eval_wall_secs`, `cached_items`, `generated_items`, p50/p95, truncation_rate) | NEW module + MODIFY | NEW `crates/vox-ml-cli/src/commands/mens/eval_timing.rs`; hooks in `eval_local.rs` (:473-545 loop, :724 report), `reasoning_eval.rs` (`score_loaded` :670; `Row` is a 4-tuple, avoid changing it, keep timing in a side struct), `reasoning_compare.rs` (`write_report` :156, `decode` object :205/:258), `eval_spoke.rs` |
| cycle metrics (`cycle_wall_secs`, `eval_share_of_cycle`, `pass_gain_per_gpu_hour`) | NEW | pure fn reading `train-record.json` (`train_wall_secs` :301) + eval report; lives in `eval_timing.rs` |
| Tier-0 / Tier-1 / Tier-2 pyramid, `--sequential` | MODIFY policy plumbing | see Pitfall P-SEQ below |

**Sequential screening: REFUTED as "new".** `eval_collateral.rs:391 run_collateral_damage_sequential` and `eval_gate/policy.rs:121-139 sequential_rule()` already implement sequential McNemar; it is activated by policy `rule: sequential_mcnemar_v1`, not a flag (`eval_collateral.rs:1114-1155`). `eval-gates-ladder.yaml` has the `sequential:` block but `rule: fixed_n` (lines ~31-34), so it is dormant. Flipping `rule` in the ladder file changes `policy_blake3` (`hub_sync.rs:187-191`, `gate_policy.rs:70`) and invalidates every existing `gate_receipt.json`. Correct design: a separate `eval-gates-ladder-fast.yaml` plus a `--policy` option, because `LADDER_POLICY` is a hard-coded const (`eval_collateral.rs:18, 1059, 1114`).

## 2. Failure funnel

### Where a completion is classified today
`metrics.rs:582-703 verify_completion_with_harness_inner`. Stages already distinguishable per sample (they are embedded in the sample as `checks`, `eval_local.rs:528`):
`non_empty` (:602) -> `parse_ok` (:617-619, front-end Ok) -> `typecheck_ok`/`diag_errors` (:620-621) -> `anti_stub_pass` via `placeholder_marker_hits`/`trivial_placeholder_output` (:631-639) -> `pass_exec` (`Some(false)` on harness failure) with `harness_error` string only for compose/typecheck/rebinding failures (:643-656) -> `semantic_match` (:658) -> `pass` (:661-665). `answer_region` strips think (`eval_local.rs:501`); `opened_think` is recorded (:527).

### What is NOT distinguishable (the real gaps)
- Runtime failure kinds are collapsed: `run_harness_assertions_inner` does `interp.run_module(..).is_err() -> Ok(false)` and each `interp.call(..).is_err() -> Ok(false)` (`metrics.rs:232-258`), same in `run_test_assertions` (:122-139). The compiler **has** the discriminator: `EvalError::{StepLimitExceeded, RecursionLimitExceeded, AssertionFailed, Panic, ...}` (`vox-compiler/src/eval/mod.rs:118-142`). The new string cap also returns `StepLimitExceeded` (`eval/expr.rs:245-258`), so `step_or_string_limit_rate` is one variant, but `assertion_fail_rate` vs `step_or_string_limit_rate` vs `panic` needs `metrics.rs` to keep the error. **Modification of `metrics.rs`** (already dirty: +323 lines).
- `truncation_rate` / `unclosed_think_rate` need `stop_reason` from the plugin (section 1); `no_code_rate` needs `extract_code` result emptiness separate from `non_empty` (`metrics.rs:276-300`).
- Doc comment at `metrics.rs:~245` says `250_000` steps but the code uses `Interpreter::new(100_000)` (:129, :232). Stale comment only.

### Design (lowest-conflict)
Add `checks.exec_error_kind` (+ `checks.no_code`) in `metrics.rs`; they flow into samples automatically with no `eval_local.rs` edit. The funnel aggregate is a **pure function over report JSON** (`results[].samples[].checks`), so it also works on historical reports and on checkpoint-resumed runs with no model. NEW `eval_funnel.rs`; one-line hook where the report JSON is assembled (`eval_local.rs:724-755`). `aggregate_gate_producer_keys` (:932) must not emit funnel keys into `eval_results.json` (existing tests at :1249-1290 pin its key set).

## 3. Lane taxonomy (every use of "lane")

The handoff says four meanings; there are at least **seven**:

| # | Meaning | Where (verified) |
|---|---|---|
| 1 | compute backend | `contracts/mens/memory-model.v1.yaml:3,36-41` (`lanes:` / `lane:`); `vox-populi/src/mens/tensor/memory_model.rs:49-66,709-742` (`enum Lane`, `RawLaneRow.lane`); `calibration.rs:32,49,102,418`; `accel_budget.rs:208-209` (inline YAML fixture); `probe.rs:79-83` (ml-cli). Both Rust files are dirty from Phase 549. |
| 2 | corpus row slice | stamped by `vox-corpus/src/corpus/mix/mod.rs` `enrich_lane_metadata` :531-565 / `default_lane_from_row` :459-478 (emits only `vox_tool_selection`, `vox_argument_generation`, `vox_tooling`, `vox_speech`, `vox_docs_qa`, `vox_codegen`); gated by `include_lanes/exclude_lanes` :108-113; counted in `MixRunReport.lane_counts` :205-211, written to `{output}.mix_report.json` (:1028-1040). The other names (`vox_lang_tier_b`, `vox_rust_*`, `vox_research_expert`, `vox_harness`) are only ever in YAML or upstream stamping. PARTIAL vs handoff ("mod.rs:459-478 lists ten lanes"). |
| 3 | letter alias | "Lane G" at `domain-profiles.yaml:184` AND `mens/config/mix-research-expert.yaml:1` (handoff names only the first) |
| 4 | orchestrator Sync/Background/Plan | outside this tree; not re-verified |
| 5 | **serving lane** `codegen|hub|agent` | `inference.rs:839-841,876-891 resolve_lane`; `hub_card/card_facts.rs:314-319` (`"lane":"codegen"` in example request). NOT in handoff. Must not be renamed by the taxonomy cleanup. |
| 6 | **router signal** `lane:<row-lane>` | `domain_router.rs:22-60 LANE_DOMAIN_TABLE`, `domain_profiles.rs:55-61,130-143 spokes_for_lanes`, `domain-profiles.yaml:64,84,111,138,154`. NOT in handoff. This is the lane -> spoke map and is the natural home of "data_slice -> spoke". |
| 7 | **`AdapterCard.train_lanes`** | `adapter_card.rs:46-48`; read at `card_facts.rs:418-429` and `eval_gate/mod.rs:70-79`; **always written empty**: `train_arm.rs:849-850` and `pipeline_dispatch.rs:553` (`train_lanes: Vec::new(), spokes: Vec::new()`). NOT in handoff. |

Drift claims:
- **docs vs mix.yaml: CONFIRMED.** `docs/src/reference/mens-training-data-contract.md:11,40,57,67` say code-only `include_lanes: [vox_codegen]`; `mens/config/mix.yaml:83-88` includes `vox_codegen, vox_tooling, vox_docs_qa, vox_lang_tier_b, vox_research_expert`. `target/dogfood/train.jsonl` is the sole `optional: false` (`mix.yaml:16-18`). Doc also lists `vox_trajectory_repair`/`vox_retrieval_grounded` (:36), which no config or detector uses.
- **Two vox-lang mixes (not in handoff):** `domain-profiles.yaml:46` points `vox-lang` at `mix-vox-lang.yaml` (output `mens/data/train_mixed_vox_lang.jsonl`, every source `optional: true`, CONFIRMED), while `mix.yaml` (output `target/dogfood/train_mixed.jsonl`) is the no-`--domain` default. Unifying them is a real decision, not a rename.
- **verified_vox_codegen_v2.jsonl unwired: CONFIRMED.** Referenced only by its producer `crates/vox-ml-cli/src/commands/mens/v94_eval_corpus.rs` and docs; no `mens/config/*.yaml` names it. It is `.gitignore:192 *.jsonl` (untracked), 3,338 rows, every row `lane: vox_codegen`, with an `origin` field (`v94_synth_exec_verified` 2109, `dogfood_v4_verified` 675, `repo_source_pool` 204, `humaneval_vox_training_eligible` 133, `train_full_backup_verified` 105, `mit_teacher_rft_v94` 98, `validated_clean_verified` 14). No verification-tier or `data_slice` field exists, so `vox.exec_verified` vs `vox.compile_verified` needs either a stamp at the producer (`v94_eval_corpus.rs`/`enrich_lane_metadata`) or an `origin -> data_slice` table. Because it is gitignored, the mix source must be `optional: true` plus a producer step, or the clean-clone/CI path breaks.
- **default_spoke hardcoded: CONFIRMED**, `hub-cards.yaml:13` (comment :10-12 says the same). Handoff's fix is PARTIAL: the **reader already exists**: `hub_card/facts.rs:657-685 active_spoke_id` (manifest `spoke`/`domain_profile` -> train_record -> `spoke_gates` -> default) and `eval_gate/mod.rs:55-95 SpokeDerivation` (card `spokes` -> card `train_lanes` via `spokes_for_lanes` -> manifest `spoke`/`domain_profile` -> fallback). The key those readers look for is **`spoke` or `domain_profile`, not `spoke_id`**. No writer emits any of them (`rg` finds no `domain_profile`/`spoke` write in `candle-core/manifest.rs`, `vox-populi manifest/mod.rs`, `train_record.rs`). Fix is writer-side; use the existing key (or add `spoke_id` as a reader alias in two places).
- **stale spoke bases: CONFIRMED** `domain-profiles.yaml:57/61`, `:105/108`, `:132/135` (`small_code_default`/`agentic_default`/`qwen3_16g`). Also `mix-rust.yaml:16` still includes `vox_rust_review` while `domain-profiles.yaml:83` and `domain_profiles.rs:429-446` (test) say review routes to base: CONFIRMED.
- **mix-agents.yaml "duplicate/dead": PARTIAL.** Not routed by any `domain-profiles.yaml` profile, but still referenced by `vox-populi/.../spoke_validate.rs:262-266` (test) and `vox-corpus/src/training/mix_prepare.rs:685-689`. Deleting it requires editing both.
- No `mode:` key exists in `domain-profiles.yaml` / `DomainProfile` (`domain_profiles.rs:74`); non-QLoRA profiles are today identified only by the absence of `base:`. Adding `mode: retrieval_only|eval_only` is a schema change in `domain_profiles.rs` + `validate_all_profiles` (:159).

## 4. Run artifacts: where written, what spoke_id + data_slice_histogram touch

| Artifact | Writer | Notes |
|---|---|---|
| `training_manifest.json` | plugin side: `vox-plugin-mens-candle-core/src/manifest.rs:46-130 TrainingManifest`, `initial_training_manifest` :238, `finalize_candle_qlora_training_manifest` :351; mirrored struct in `vox-populi/src/mens/tensor/manifest/mod.rs:81` and a third, `cloud/pipeline_dispatch.rs:426` | three struct copies; plugin ABI carries `LoraTrainingConfig`. Adding fields here means 3 structs + config plumbing. |
| `run_summary.json` | `candle_qlora_train/finalize.rs:313-337` (metal) and the cuda copy | contains `avg_val_loss_last_epoch` but **no `truncated_rows`/`prompt_trimmed_rows`** |
| `train-record.json` | host, `ml-cli/.../populi/train_record.rs:243-309 build_train_record`, written from `train_arm.rs:594-596` | reads `s["truncated_rows"]`/`s["prompt_trimmed_rows"]` (:303-304) that `run_summary` never writes. **That is the root cause of the handoff's "null in all v94 runs": CONFIRMED + explained.** `RecordInput` (:30-48) has no `spoke`/`domain`. |
| `adapter_card.json` | host, `train_arm.rs:810-856 write_local_adapter_card`, called at :647 after the record | `train_lanes`/`spokes` hard-empty (:849-850). Card embeds `training_manifest_digest` = BLAKE3 of manifest bytes (`adapter_card.rs:37`): any post-hoc manifest patch must happen before the card is written. |
| mix report | `vox-corpus` `{output}.mix_report.json` | `lane_counts` is **already** the data_slice histogram for mixed runs. `--fast-corpus` (`train_arm.rs:333-342`, `VOX_TRAIN_SKIP_CORPUS_MIX`) skips mixing entirely, so histogram must be computable by scanning the consumed train file (path is `training_manifest.train_file`). |

Recommended host-only path (no plugin ABI change): NEW `populi/run_stamp.rs` computes `spoke` (from `--domain`, `train_arm.rs:423`), `data_slice_histogram`/`verification_histogram` (scan consumed train file), patches `training_manifest.json` before the adapter card, and `RecordInput` gains the fields so `train-record.json` carries them. `adapter_card.spokes/train_lanes` are filled at `train_arm.rs:849-850`. This makes `active_spoke_id` and `SpokeDerivation` work with zero reader changes. Touches: `train_arm.rs`, `train_record.rs`, NEW `run_stamp.rs`; optional plugin follow-up for `truncated_rows` in both `finalize.rs` files.

## 5. Hub sync + cards + Space

**Uploaded per size repo** (`hub_sync.rs:774-1118 stage_publish_bundle`): trained `candle_qlora_adapter.safetensors`; merged checkpoint only when the gate passed (:787, :861); PEFT export (:886-913); `chat_template.jinja`; sanitized `training_manifest.json`; `OPTIONAL_BUNDLE_FILES` (:32-60+); per-slice collateral reports; freshly derived `spoke_gates.json` (:985-1021); `evidence/reasoning_compare.json`; `.eval_results/vox_mens_eval.yaml`; `external_serving_handoff_v1.json`; README.md + MODEL_CARD.md; raster assets; `hf_sync_manifest.json` (`HfSyncManifest` :280-289: size, base_pin, corpus_hash, final_loss, peak, `gate_status` string, files with BLAKE3).

**README.md vs MODEL_CARD.md duplication: CONFIRMED, by construction.** One `card` string written to both: `hub_sync.rs:1072-1073` (stage), `:737-738` (reserve), `hub_cards_push.rs:25 CARD_FILES`. A third, divergent card is written at training time by the plugin (`candle-core/src/model_card.rs:69`, referenced in `finalize.rs` artifacts) and is overwritten at stage time. A fourth path, `scripts/hf_model_pipeline.vox:445` + `mens/config/hf_readme_template.md` (27B-only, hard-coded), is unrelated to the Rust renderer and can drift. Fix surface: stop writing MODEL_CARD.md in 3 Rust sites + `CARD_FILES`, and decide remote deletion of the existing duplicate (manifest diff will show it removed).

**Version/tag concept: EXISTS** (HFVER-01/02). `parse_version_tag` :1191, `latest_version_tag` :1200, `next_version_tag` :1208, `list_repo_tags` :1218, `create_repo_tag` :1232; publish tags `v<N>` at the uploaded commit with a message that carries corpus hash and loss (`:1344-1371`); legacy repos baselined as `v1` (:1329-1337); pull defaults to latest tag; index rows carry `latest_version` (`hub_card/index_facts.rs:153`, `index.md:39`). Gaps: tag message has no gate/bench/spoke info; tag is created even for `Ungated` uploads (`force_ungated`, :261-267 -> label `ungated (...)`); `HfSyncManifest` has no `spoke`, bench id, or timing; **the static Space fetches `resolve/main/...` (`templates/hub/space/index.html.tmpl:589`), not a version tag**, so it can show an unreleased head.

**Gate checked before upload** (`hub_sync.rs:159-268 verify_publish_gates_with`): `collateral_damage_report.json` via `check_collateral_report` against the ladder requirement; `gate_receipt.json` state vs current `eval-gates-ladder.yaml` BLAKE3 + `adapter_sha256` from `adapter_card.json`; per-claimed-spoke rows from `gate_policy::spoke_gate_rows` (`gate_policy.rs:49`). A claimed-spoke row only applies if `adapter_card.spokes` names it, and `spokes` is written empty today, so spoke gates currently never apply to locally trained adapters. Not checked: which bench produced the held-out numbers. `hub_card/facts.rs:77-79,305-315` accepts `evidence/heldout_*_nothink.json` on `prompt_protocol` alone and never reads `bench`/`bench_manifest_blake3`; `eval_local.rs:343-373` mirrors any report with that protocol into `evidence/`. So v1 and v2 results are indistinguishable on a card. This is the main "truthful card" gap.

**Bugs found (not in handoff):**
- `hub_sync.rs:1141` stages `populi_train_record.json` into `spokes/<spoke>/`; no such file exists anywhere. Real name is `train-record.json`. Spoke bundles silently omit it.
- `templates/hub/space/app.py.tmpl` (446 lines) and `requirements.txt.tmpl` have **no references** in any `.rs`/`.toml`; `write_local_cards` even deletes stale `app.py`/`requirements.txt` (:1918-1920). Dead templates.
- Space upload failure is warn-only (`hub_sync.rs:2159-2168`); the index upload before it is `?`-fatal. A run can report success with a stale Space.
- `hub-cards.yaml` ETA basis text says "32-item prefix cache" meaning checkpoint prefix reuse, not KV cache; a third meaning of "prefix cache" next to `eval_local.rs:221`.

**Space render + push:** `render_playground_space_files` (:1935-1948) substitutes `{{MODELS_YAML}}`/`{{NAMESPACE}}` into `README.md.tmpl` and `{{NAMESPACE}}` into `index.html.tmpl` (1,899-line single static file, `sdk: static`); `write_local_cards` (:1856) writes locally to `mens/runs/hub_cards/vox-mens-playground`; `upload_ladder_index` (:2126-2187) uploads index repo, then Space via `hub::upload_space_folder` (`hub.rs:523`), then the HF collection.

**Constraint:** cards must be pure functions of records (no clock/Hub state), `hub_card/mod.rs:3-8`; `manifests_match` (:292) relies on it, and `render_or_marker` makes errors non-matching. Golden tests live under `hub_card/tests/`; every card-visible change updates goldens.

## 6. eval_spoke::run_agent_suite

**Only stubs: CONFIRMED.** `eval_spoke.rs:16-94 run_agent_suite` takes policy closures; its only callers are the unit test (:160-203: `never_call` vs `oracle`, `n == 72`) and the test-name inventory string in `eval_gate/tests.rs:690` that lists that unit test as the "producer" of the `suites` gate. It is `#[allow(dead_code)]`, no CLI/command calls it. The consumer exists: `eval_gate/check_run.rs:521-565` reads `suite_report.<suite>.json` and `.base.json`; policy `eval-gates-agents.yaml:6-13` has `agent_episodes` with `block: false, require_report: false`, so the gate is vacuous.

Integration facts: `vox_eval::episode_controls::BoxedPolicy<'a> = Box<dyn FnMut(&[ChatMessage], &[ToolDef]) -> Result<String,String> + 'a>` (`episode_controls.rs:6`), but `run_agent_suite` demands `BoxedPolicy<'static>` (:28, :32), which cannot borrow the `infer` closure of `with_backend`: signature change needed. `with_backend` (`reasoning_eval.rs:256`) loads one model per call; with one GPU, base and candidate must be run as **two sequential passes** (refactor to a per-side runner), not the current per-episode interleave. XML tool calls: **REFUTED as a gap** - `vox_eval::episode::parse_tool_call` already handles `<tool_call>` wrappers and `<function=..><parameter=..>` (`episode.rs:457-460`, test :1230); `eval_local.rs:524-526,763-900` JSON-shape checks are codegen-bench telemetry and `aggregate_gate_producer_keys` intentionally never emits them.

## Component Boundaries (new/changed)

| Component | Responsibility | Talks to |
|---|---|---|
| plugin `generate()` (metal, cuda) | timing fields, stop_reason, prefix snapshot | `run_inference` JSON <-> host |
| `eval_timing.rs` (NEW) | parse response timing, aggregate, cycle metrics | eval_local, reasoning_*, eval_spoke |
| `metrics.rs` | `exec_error_kind`, `no_code` in `checks` | eval_local samples |
| `eval_funnel.rs` (NEW) | pure funnel aggregation over report JSON | eval_local report, scorecards |
| `run_stamp.rs` (NEW) | spoke + histograms into manifest/record/card | train_arm, train_record, hub_card readers |
| `eval_spoke_live.rs` (NEW) | model-backed policy, per-side passes, writes `suite_report.*.json` | with_backend, eval_gate check_run |
| hub_sync / hub_card / templates | gated, bench-labelled, versioned publish; Space pinned | HF Hub |

## Patterns to Follow
- **Additive JSON keys only** on responses, reports, manifests (`serde(default)`; readers use `Value` lookups). Never alter request JSON or checkpoint header inputs.
- **Pure-function derivation over persisted JSON** (funnel, cycle metrics, histograms) so old runs backfill and no GPU is needed.
- **Mutation-verify guards** (AGENTS.md): any gate/stamp test must fail when the guard is removed (e.g. spoke gate applies only when `spokes` is populated).

## Anti-Patterns to Avoid
- Editing `eval-gates-ladder.yaml` for experiments (policy hash invalidates all receipts; four hard-coded path sites: `eval_collateral.rs:18`, `hub_sync.rs:189`, `gate_policy.rs:70`, `ladder_requirement`).
- Renaming serving `lane` (`codegen|hub|agent`) or router `lane:` triggers as part of the "lane" cleanup.
- Shallow-cloning `Qwen35LayerCache` for the prefix snapshot.
- Taking both sides of a conflict in `mens/mod.rs`-style registries is safe only for append-only `mod` lines; do not union-resolve `eval_local.rs`.

## Build Order and Parallel Groups

**Gate 0 (blocking, no GPU): commit the v94 working tree** (~157 dirty paths; dirty files this milestone must edit: `eval_local.rs`, `metrics.rs`, `mens/mod.rs`, `memory_model.rs`, `calibration.rs`, cuda `model.rs`). Committing does not touch PID 34157 or `target/release`. Pre-register the `mod eval_timing; mod eval_funnel; mod eval_spoke_live;` lines in `mens/mod.rs` here to avoid three agents colliding on it.

**Gate 1: freeze four contracts before fan-out:** (a) plugin response field names/types (handoff section 5 list); (b) `checks.exec_error_kind` enum values (`step_or_string_limit|recursion_limit|assertion|panic|harness_compose|none`); (c) run-record keys: reuse `spoke`, add `data_slice_histogram`, `verification_histogram`; (d) funnel schema.

```text
Gate0 commit v94 -> Gate1 contracts
   |-> P1 plugin timing+prefix KV (metal,cuda) ----------\
   |-> P2 host timing (eval_timing + eval_local/reasoning_*) --\
   |-> P3 failure funnel (metrics.rs + eval_funnel) -> hook via P2 owner
   |-> P4 lane/config/docs cleanup --------------------> P4b wire verified_v2 into canonical vox-lang mix
   |-> P5 run stamping (run_stamp, train_arm, train_record) -> P6b hub-cards default_spoke removal + card spoke/slice display
   |-> P6a hub hardening (README dedupe, bench label, tag msg, Space pin, dead templates, spokes bug)
   |-> P7 live eval_spoke (eval_spoke(.live).rs, CLI wiring)
   '-> P8 memory-model lane->backend (serde alias only) [low value, after Gate0]
P1+P2+P3 merged -> [GPU] G1 metal parity+canary -> [GPU] G2 Tier-1 2B/0.8B baseline w/ timing
P7 merged -> [GPU] G3 live agent_episodes (n=72)
G2 -> [GPU] G4 27B 30m preflight -> [GPU] G5 CAP-27B-04 (~30h) + v2 eval -> P9 v2 gate-floor calibration -> release
```

### Parallel groups (file-disjoint; separate agents can run each simultaneously)
| Group | Owner files | GPU | Depends on |
|---|---|---|---|
| **P1** plugin timing + prefix KV | `vox-plugin-mens-candle-metal/src/{inference,model,batch_decode}.rs`, `vox-plugin-mens-candle-cuda/src/{inference,model}.rs` | No for code + CPU tiny-model parity test (tests use `Device::Cpu`, `inference.rs:271+`); **Yes** for Metal parity proof; CUDA verified by `cargo check` only (no CUDA behaviour test possible on this Mac) | Gate1(a) |
| **P2** host timing | NEW `eval_timing.rs`; `eval_local.rs`, `reasoning_eval.rs`, `reasoning_compare.rs`(+`reasoning_compare/`), `eval_collateral.rs` only if `--policy` added | No (fake `infer` tests) | Gate1(a), P3 function signature |
| **P3** funnel | `metrics.rs`, NEW `eval_funnel.rs` (no `eval_local.rs` edit; P2 owner adds the one-line hook) | No | Gate1(b,d) |
| **P4** lane/config/docs | `mens/config/mix*.yaml`, `domain-profiles.yaml`, `docs/src/reference/mens-training-data-contract.md`, `domain_profiles.rs` (`mode`), `spoke_validate.rs`+`mix_prepare.rs` if `mix-agents.yaml` is removed | No | none (P4b needs a producer run of `v94_eval_corpus` on CPU) |
| **P5** run stamping | `populi/train_arm.rs`, `populi/train_record.rs`, NEW `populi/run_stamp.rs` (+ optional both `finalize.rs` for `truncated_rows`) | No | Gate1(c) |
| **P6a** hub hardening | `hub_sync.rs`, `hub_cards_push.rs`, `hub_card/*.rs`, `templates/hub/**`, goldens under `hub_card/tests/` | No (network only for real push) | none; bench label reads `bench` already in reports |
| **P6b** | `mens/config/hub-cards.yaml` (+`hub_card/config.rs` validation :152) | No | P5 landed |
| **P7** live eval_spoke | `eval_spoke.rs`, NEW `eval_spoke_live.rs`, `populi/action_populi_enum.rs`+`dispatch.rs` (CLI), regenerated CLI docs via `vox ci command-sync` | tests No; **live run Yes** | P2 (shares `with_backend` usage only; can use plain `generated_text`) |
| **P8** backend rename | YAML + `memory_model.rs`, `calibration.rs` (`#[serde(alias="lane")]`) | No | Gate0 (files dirty) |
| **P9** v2 gate floors | new `eval-gates-ladder-v2*.yaml` or ladder edit + re-gate | No (needs G5 data) | G5 |

P1/P2/P3/P4/P5/P6a/P7 are mutually file-disjoint (only the one-line funnel hook in `eval_local.rs` and the three `mod` lines in `mens/mod.rs` cross groups, both pre-assigned above). Build contention, not files, is the practical limit: all cargo runs go through the build broker; run only `cargo check -p`/`cargo test -p <crate> <filter>` per group.

### Single-GPU schedule (serialized; nothing starts until PID 34157 exits)
1. G1 (minutes): Metal bitwise parity of prefix-cached vs uncached greedy decode on a 0.8B model, plus timing sanity; requires P1+P2+P3 merged; build into isolated target dirs (`target/v94_canary_*` precedent), never `target/release`.
2. G2 (<=45 min): Tier-1 canary on `heldout_bench_v2_canary` (64) with timing + funnel; first comparable flywheel datapoint. Handoff open question (2B vs 0.8B canary) is answered here.
3. G3 (<2 min): live `agent_episodes` (72) on the canary model after P7.
4. G4 (30 min): 27B `seq=2048` preflight (untied `lm_head` LoRA, 15-20 optimizer steps).
5. G5 (~30 h train + 10-40 h eval): CAP-27B-04 command from handoff section 1; eval uses sequential screening only through the fast policy; the release gate stays `fixed_n`.
Everything else is GPU-free. Order short GPU jobs (G1-G4) before the 30 h job.

## Handoff claim ledger

| Claim | Verdict | Evidence |
|---|---|---|
| response only `{generated_text,prompt_tokens}` (`inference.rs:909-913`) | CONFIRMED | `inference.rs:909-914` |
| `empty_cache()` per `generate()` at `inference.rs:945` | PARTIAL | `:945-947`; fresh cache construction, no retained cache; "bug" framing wrong |
| `eval_local.rs:221` prints prefix-cache-friendly | CONFIRMED | `:220-223` |
| no eval timing in eval_local/reasoning_*/eval_spoke | CONFIRMED | grep: no `Instant`/`elapsed` in those files |
| ~1.1k-token shared system prompt | UNVERIFIED | prompt is `generate_training_system_prompt()` (`eval_local_prompt.rs:144`); `mens/config/system_prompt.txt` is 9,496 B, so likely larger |
| ladder yaml has sequential block (`:33-35`) so expose `--sequential` | REFUTED (as new work) | sequential already implemented (`eval_collateral.rs:391`, `policy.rs:121`); activation is `rule:` not a flag; block is at ~`:32-34` |
| `eval-gates-ladder.yaml:8` = 0.25 floor | CONFIRMED | `:8` |
| XML `<tool_call>` not graded by vox-eval episode | REFUTED | `episode.rs:457-460`, test `:1230` |
| `eval_local.rs:524-526` tool-call checks | CONFIRMED location; JSON-only and never emitted to gates | `:524-526,763-900,932+` |
| `truncated_rows`/`prompt_trimmed_rows` null | CONFIRMED + root cause | `train_record.rs:303-304` reads keys `run_summary` never writes (`finalize.rs:313-337`) |
| `eval_spoke` has only stubs, no live inference | CONFIRMED | `eval_spoke.rs:160-203`, no non-test caller |
| four meanings of "lane" | REFUTED as complete | seven found (table in section 3) |
| memory-model `lane:` at `:36-41` | CONFIRMED | `:3,36,37,41` |
| docs claim code-only mix at `:40,67` | CONFIRMED | `mens-training-data-contract.md:11,40,57,67` vs `mix.yaml:83-88` |
| `verified_vox_codegen_v2.jsonl` unwired | CONFIRMED | only `v94_eval_corpus.rs` + docs; gitignored; all rows `lane: vox_codegen` |
| `mix-vox-lang.yaml` all sources optional | CONFIRMED | `mix-vox-lang.yaml:6-45` |
| `Lane G` at `domain-profiles.yaml:184` | CONFIRMED (+ `mix-research-expert.yaml:1`) | |
| mix-agents.yaml dead/duplicate | PARTIAL | referenced by `spoke_validate.rs:262`, `mix_prepare.rs:685` |
| `mix-rust.yaml:16` includes `vox_rust_review` | CONFIRMED | |
| stale bases `domain-profiles.yaml:57-61,105-108,132-135` | CONFIRMED | |
| `default_spoke` hardcoded `hub-cards.yaml:13`, facts.rs should read spoke_id | PARTIAL | reader exists (`facts.rs:657-685`, `eval_gate/mod.rs:55-95`), key is `spoke`/`domain_profile`; writers absent; `train_lanes`/`spokes` written empty |
| `mix/mod.rs:459-478` defines the ten lane names | PARTIAL | only six names there; rest come from YAML/upstream |
| gpu-specs pins at `:347-349` | CONFIRMED | Guard 347, FrogNano 348, MiMo 349 |

## Scalability / Risk notes
| Concern | Risk | Mitigation |
|---|---|---|
| Prefix cache numerics | token flips at temp 0 invalidate cached generations or force a new protocol tag | CPU bitwise test first, Metal proof second; decide tag policy before G5 |
| Two backend copies | metal/cuda drift | mirror edits in one PR; CUDA only `cargo check`-able here |
| Dirty tree | conflicts with 157 uncommitted paths | Gate 0 commit first |
| 30 h run | one run per cycle | land P1-P3 first so the run is timed, funnelled, prefix-cached |

## Sources
- All paths above were read in `/Users/brbrainerd/dev/vox` working tree on 2026-11-20 (HIGH confidence for file:line facts; MEDIUM for performance/numerics expectations, which are untested).
