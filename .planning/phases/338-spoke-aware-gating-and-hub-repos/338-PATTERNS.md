# Phase 338: Spoke-aware gating and Hub repos - Pattern Map

**Mapped:** 2026-11-05
**Files analyzed:** 16 (new + modified)
**Analogs found:** 15 / 16
All analog paths are under `/Users/brbrainerd/dev/vox-mens-compare/crates/` and are git-tracked (checked with `git ls-files`). Line numbers come from 338-RESEARCH.md, which read them this session. Re-read each file at execution time (D-15, phase 335 is editing several of them).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `vox-eval/src/collateral_evidence.rs` (new) | utility (pure validator + `gate_state`) | transform | `vox-eval/src/reasoning_slice.rs::verify_reasoning_slices` (477-545) | exact |
| `vox-populi/src/mens/hub_sync.rs` `verify_publish_gates` (mod) | service | request-response | itself, lines 149-205 | exact |
| `vox-populi/src/mens/hub_card.rs` `card_status` (mod, :217) | service | transform | `hub_sync.rs::verify_publish_gates` | role-match |
| `vox-populi/src/mens/dispatch.rs` serve gate (mod, :922) | service | request-response | `check_run.rs:991-1012` | exact |
| `vox-ml-cli/.../eval_gate/check_run.rs` (mod) | service | request-response | itself, lines 961-1039 | exact |
| `vox-ml-cli/.../eval_gate/policy.rs` (mod, canonical hash) | utility | transform | `check_report_slices` (137-158) and `item_checkpoint.rs:139-141` | role-match |
| `vox-ml-cli/.../eval_gate/run_gate.rs` (mod, receipt v2) | service | file-I/O | itself, lines 70-84 (inline `json!` receipt) | exact |
| `vox-ml-cli/.../eval_gate/spoke_route.rs` (new) | utility | transform | `vox-populi/src/tensor/domain_profiles.rs` (`include_lanes`, `eval_gate`) | role-match |
| `vox-eval/src/episode.rs`, `episode_controls.rs` (new) | utility (pure scorer, mock world, scripted policies) | event-driven (multi-turn) | `vox-orchestrator-mcp/.../chat_tools/chat/agent_loop.rs` (loop, salvage 636-742, `DEFAULT_MAX_ITERATIONS` :172) | partial |
| `vox-ml-cli/.../eval_episodes.rs` (new, thin glue) | service | streaming (checkpointed turns) | `reasoning_eval.rs` (199-243, 295-345) + `item_checkpoint.rs` | role-match |
| `vox-cli` `vox harness tool-catalog` + `vox-orchestrator-mcp` drift test (new) | utility + test | batch/file-I/O | `docs/src/reference/mcp-tools.generated.md` generate-and-drift pattern; `registry::tool_registry()` (registry.rs:55-97) | role-match |
| `vox-populi/src/mens/hub_dataset.rs` (new) | service | file-I/O | `hub.rs::upload_model_folder` (407-456) | exact |
| `vox-populi/src/mens/hub_repo.rs` (mod: dataset ids) | config | transform | `index_repo_id` (57-62), `repo_id_for_size` (45-54) | exact |
| `contracts/mens/spoke-gate-state.v1.schema.json` (new) | config (contract) | transform | other `contracts/**/*.schema.json`; validate via `vox-jsonschema-util` | role-match |
| `mens/config/eval-gates*.yaml`, `domain-profiles.yaml` (mod) | config | n/a | `eval-gates-ladder.yaml:95-101` (alpha + slice pins) | exact |
| `mens/data/agent_episodes/*.jsonl`, `mens/data/rust_bench/*` (new fixtures) | fixture | n/a | `contracts/eval/humaneval-vox` (spec.toml + hidden tests) | role-match |
| Rust bench runner (new) | service | batch | `vox-corpus/.../rust_authoring.rs:75-141` tmp-crate compile | role-match |

## Pattern Assignments

### `vox-eval/src/collateral_evidence.rs` (D-18, SPOKE-01)
**Analog:** `reasoning_slice.rs::verify_reasoning_slices` + `policy.rs::check_report_slices`. Compose them, do not re-implement.

```rust
// shape from RESEARCH Pattern 1
pub struct CollateralRequirement<'a> {
    pub alpha: f64,
    pub slice_pins: &'a [(&'a str, &'a str)], // (slice, blake3) from ladder yaml
    pub grader: &'a str,                      // qwen_reasoning::GRADER_ID
}
pub fn check_collateral_report(report: &serde_json::Value, req: &CollateralRequirement) -> Result<(), String> {
    crate::reasoning_slice::verify_reasoning_slices(report, req.alpha)?; // rule + alpha + counts
    // pins: report["slice_blake3"][name] == pin  (same compare as policy.rs:137-158)
    // grader: report["grader"].as_str() == Some(req.grader)  -- NEW, nobody checks it today
    Ok(())
}
```
Same file hosts `gate_state(receipt, current) -> Passed | Failed | NeedsReGate(reason) | NotRun`, driven by the truth table in RESEARCH "Gate-state truth table".
Constants: `PAIRED_RULE` (reasoning_slice.rs:22), `GRADER_ID = "grade_final_answer v2"` (qwen_reasoning.rs:153), `SLICE_NAMES` (reasoning_bench.rs:18).
Tests: one fixture per refusal, valid except for one field, built from the real writers (`reasoning_slices_json`), not a hand-typed blob (Pitfall 2). Mutation-prove each guard.

### `hub_sync.rs::verify_publish_gates` (modify)
**Analog:** itself (149-205). The hole is line 160:
```rust
if val.get("status").and_then(|s| s.as_str()) != Some("pass") {
    missing_or_failed.push("collateral_damage_report.json (status != pass)".to_string());
}
```
Replace with a call to `check_collateral_report`, pushing its `Err` text into `missing_or_failed`. Add a `&CollateralRequirement` parameter. Callers: `hub_sync.rs:382`, `:856`, `train_arm.rs:677`. Keep the error shape (`bail!("refusing to publish adapter in {}: ...")`, `Ungated{reason}` when `force_ungated`). Add `dep:vox-eval` to the `mens-hf-hub` feature in `vox-populi/Cargo.toml` (edge already allowed).
**Fixture to fix:** `hub_sync.rs:1383-1386` writes `{"status":"pass"}` and test `stage_bundle_writes_card_and_hashes_and_strips_host_paths` (1464-1480) asserts `passed`. Same pattern in `hub_card_tests.rs`.

### `hub_card.rs::card_status` (:217) and `dispatch.rs` (:922)
Replace `Some(c) if c["status"].as_str() == Some("pass") => {}` with the same shared function so the card never says `passed` where publish refuses. Serve (`assert_collateral_gate`, dispatch.rs:922-962) already calls `verify_reasoning_slices`; add pins and grader through the shared fn. Add a structural test that the literal `Some("pass")` appears in none of the four files outside caller glue.

### `eval_gate/check_run.rs` (modify)
**Analog:** lines 991-1012:
```rust
("pass", Some(alpha)) => verify_reasoning_slices(&v, alpha).and_then(|_| cd.check_report_slices(...))
```
Add the grader check via the shared function. Wired-vs-paper work (SPOKE-02): each PAPER key in RESEARCH "Wired vs paper inventory" is either given a real adapter-measuring producer or deleted from the YAML. The nested-telemetry keys (`supervised_ratio`, `throughput`) fail because the reader looks at top-level keys while lines are `{"event","payload":{...}}`.

### `eval_gate/run_gate.rs` and `policy.rs` (receipt v2, canonical hash; SPOKE-04)
**Analog:** `run_gate.rs:70-84` inline receipt (`schema: "vox_mens_gate_receipt_v1"`, `source_commit: env!("VOX_GIT_HASH")`, `policy` path string, `gates[{name,passed,block,message}]`). Additive only: keep v1 fields and the `gates[]` row shape (consumers match `message` substrings "not applicable"/"skipped"/"missing" in `hub_sync.rs:174-184`, `hub_card.rs:241-249`, `dispatch.rs:991-999`, and the GUI). Add `policy_blake3` and `inputs{adapter_sha256, base_pin, slice_blake3, grader, decode_blake3, episode_set_blake3, catalog_blake3, datasets}`.
**Hash helper analog:** `item_checkpoint.rs:139-141` (`vox_crypto::secure_hash` + `hex_encode`). Do not reuse `hash_json`: `v.to_string()` is order-sensitive under `serde_json` `preserve_order` (workspace-hack Cargo.toml:153). Sort keys recursively, include code-side constants (`PAIRED_RULE`, `GRADER_ID`), keep `timestamp_unix`/`source_commit` out of the hash. Test: reordered YAML hashes equal; any value change differs; comment-only edit unchanged.
No direct `blake3`/`sha2` imports (cryptography policy).

### `eval_gate/spoke_route.rs` (new; SPOKE-03, SPOKE-05)
**Analog:** `vox-populi/src/tensor/domain_profiles.rs` (`eval_gate` field :81/:207, only used by `let _ = &eff.eval_gate;` at :274, existence-checked in `spoke_validate.rs:55-59`). Rule: `spokes_of(adapter) = { profile : include_lanes ⊇ observed training lanes }`; precedence explicit `--domain` (train_arm.rs:424-432) > lane inference > `unclassified`. Current ladder adapters resolve to `vox-lang` (D-19). Applicable gates for a vox-lang adapter are `[ladder, vox-lang]`; do not emit `not_run` for non-applicable spokes. Cloud path hard-codes the policy at `train_arm.rs:941`; route it too. Record the spoke in `AdapterCard` (new optional field, `validate()` tolerates old cards; adapter_card.rs:11-45), receipt v2, `HfSyncManifest` (additive `#[serde(default)]`, schema `vox_mens_hf_sync_v1` hub_sync.rs:553-562; `manifests_match` :229-234 compares only size/base_pin/gate_status/files), card tag `spoke-<name>`, and a shipped `spoke_gates.json` (add to `OPTIONAL_BUNDLE_FILES`, hub_sync.rs:27-50).

### `vox-eval/src/episode.rs` + `episode_controls.rs` (SPOKE-06)
**Analog:** `agent_loop.rs` for presentation, call-parsing (salvage `{"name","arguments"}` :636-742, `bare_json_candidate` :679-694) and loop bound (`DEFAULT_MAX_ITERATIONS = 8`, :172). Do not reuse its dispatch (`handle_tool_call_with_receipt` runs real handlers, :981-989). Pure code, zero deps; schema validator injected as `&dyn Fn(&Value,&Value)->Result<(),String>`. Policy type `FnMut(&[ChatMessage], &[ToolDef]) -> ModelTurn`. Add a test that the vox-eval parser and the harness salvage agree on a fixture list.
Controls matrix test (model-free, seconds): `oracle` 100%; `never_call`, `refuse_everything`, `shuffled_tool`, `first_offered_tool`, `random_seeded`, `ignore_results`, `no_recovery`, `call_forever`, `extra_call_after_goal` each below floor; mutation-prove (e.g. make `redundant` always false, `extra_call_after_goal` must fail). Significance vs base: `vox_eval::reasoning_slice::mcnemar_one_sided_p(losses, gains)` (reasoning_slice.rs:80), direction reversed.
Requests must not contain tool name or description n-grams (Pitfall 7; leakage analog `check_leakage_precondition`, check_run.rs:49-88).

### `eval_episodes.rs` (new, thin glue in vox-ml-cli)
**Analogs:** `reasoning_eval.rs:199-243` (`with_backend` -> `vox_plugin_host::cached_code_plugin(...).as_ml_backend()` -> `load_model`/`run_inference(prompt_json)`; batch `{"batch":[...]}`) and `ItemCheckpoint::infer(prompt_json, infer)` (item_checkpoint.rs:1-14) for per-turn resume. Put catalog hash, episode-set hash, system prompt and decode settings in checkpoint header inputs (item_checkpoint.rs:48-56). Prompt = serve prompt (`flatten_chat_messages`, serve/handlers.rs:254-305) plus compact parameter schemas (D-23); `max_retries` 0 and recorded (handlers.rs:364). JSON-Schema via `vox_jsonschema_util::{compile_validator, validate}`. Keep it thin (ml-cli is over its `max_loc`). One parity test through `vox_actor_runtime::llm` against `vox mens serve` (D-20).

### `vox harness tool-catalog` (new, vox-cli) + drift test (D-21, Option A)
**Analog:** `registry::tool_registry() -> Vec<rmcp::model::Tool>` (registry.rs:55-97), already linked by `vox-cli` (Cargo.toml:217,270). Writes `{tools:[{name,description,input_schema,lane,side_effect_class}], selection}`; a `vox-orchestrator-mcp` test regenerates and compares to a committed lock file (same generate-and-drift idea as `mcp-tools.generated.md`). No `vox-ml-cli -> vox-orchestrator-mcp` edge.

### `hub_dataset.rs` and `hub_repo.rs` (SPOKE-08)
**Analog:** `hub.rs::upload_model_folder` (407-456, hard-codes `RepoTypeModel`) and `hub_repo.rs`:
```rust
pub fn index_repo_id(namespace: &str) -> Result<String> {
    Ok(format!("{}/vox-mens-index", resolve_namespace(Some(namespace))?))
}
```
Add `eval_repo_id`/`data_repo_id` -> `<ns>/vox-mens-eval`, `<ns>/vox-mens-data`, with a unit test mirroring `an_illegal_namespace_or_size_is_refused`. Use `hf-hub` 1.0.0 `client.dataset(owner,name)` / `RepoTypeDataset` (repository/mod.rs:43,644; upload.rs:91). Tag revisions `v<N>` via `parse_version_tag`/`latest_version_tag` (hub_sync.rs:589-600). Default private; build bundles and manifests locally, then stop at a human checkpoint before any push (D-25). MATH-500 publishes ids + blake3 + pointer.

### vox-lang suite (D-09)
**Analog:** `vox_corpus::humaneval_runner::verify_program` (`vox check` then `vox run --mode interp`, oracle-neutralisation canary), driven by `vox model eval-corpus` (generation via `vox_actor_runtime::llm`, adapter-card fingerprint check, served-model-mismatch abort, eval_corpus.rs:272-350). Add gate key (e.g. `humaneval_vox_pass_rate`) to `eval-gates.yaml`; control = base model. Count held-out fixtures in a test via `held_out(&load_corpus(..))`, not prose (30 vs 31).

### Rust bench runner (D-09, D-22)
**Analog:** `rust_authoring.rs:75-141` (`compile_batch_in_workspace`, tmp crate, file-redirected output, timeout). Extend to `cargo test` with hidden `tests.rs`; replace paper keys `rust_compile_rate`/`clippy_clean_rate` with `rust_test_pass_rate`. No rust adapter exists, so proof is oracle + scripted controls + base model.

## Shared Patterns

### Absent evidence is not evidence
**Source:** `dispatch.rs:966-999` (`assert_serve_preconditions`). Missing receipt, hash, control result or report yields `not_run` or `needs_re_gate`, never `passed` (Pitfall 10).
**Apply to:** `gate_state`, every consumer of `spoke_gates.json`.

### Mutation-proven guards
**Apply to:** each D-18 refusal, policy-hash sensitivity, each scripted control. Repo rule: break the guard, watch exactly its test fail, restore; guard the mutation with a before/after presence check.

### Hashing and crypto
**Source:** `item_checkpoint.rs:139-141` using `vox_crypto::{secure_hash, hex_encode, hash_fast_hex}`. **Apply to:** policy, episode-set, catalog, decode hashes.

### Crate-edge and LoC discipline
New pure logic in `vox-eval` (deps only serde/serde_json/regex/tempfile/log/xxhash). `vox-ml-cli` gets only thin glue. Run `vox ci crate-edges` and `cargo run -p vox-arch-check` in verify steps. No `exceptions` entries.

### Phase 335/337 ordering
Re-read `reasoning_slice.rs`, `reasoning_bench.rs`, `qwen_reasoning.rs`, `eval_collateral.rs`, `eval_gate/{policy,check_run,run_gate}.rs`, `item_checkpoint.rs`, `reasoning_eval.rs`, `eval_local.rs`, `dispatch.rs`, `hub_sync.rs` at execution time.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| Mock tool world + episode scorer (`episode.rs`) | utility | event-driven | Closest is `agent_loop.rs` but it dispatches to real handlers; `vox harness eval --live` (harness/live_eval.rs) is an outcome-based precedent for hosted models only. Use RESEARCH "Design: six behaviors" |

## Metadata

**Analog search scope:** taken from 338-RESEARCH.md (vox-eval, vox-ml-cli eval_gate, vox-populi mens, vox-orchestrator-mcp, vox-corpus); spot-verified `hub_sync.rs`, `hub_repo.rs`, `policy.rs`.
**Files scanned this session:** 5 (plus the research and context files).
**Pattern extraction date:** 2026-11-05
