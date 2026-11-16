# Phase 338: Spoke-aware gating and Hub repos - Research

**Researched:** 2026-11-05
**Domain:** MENS promotion gates (`vox-ml-cli` eval-gate, `vox-populi` hub-sync/cards), behavioral spoke suites (agent episodes, compile-and-test), Hub dataset repos
**Confidence:** HIGH for the code inventory and D-18 (every file read this session); MEDIUM for the episode-suite design (new code, pieces verified); LOW for per-size wall-clock (derived from file mtimes and one measured 0.6B row)

Tag key: `[VERIFIED: path:lines]` = file opened this session, discrete values quoted verbatim; `[CITED: url]` = official page fetched this session; `[ASSUMED]` = not verified, needs user confirmation or a Wave-0 probe.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
(Copied from 338-CONTEXT.md `## Implementation Decisions`. `[USER]` = the user's own brief, `[ASSUMED]` = recommended default chosen by the orchestrator, still open to the user.)

**Spoke identity on the Hub**
- **D-01 [ASSUMED]:** The spoke is recorded in card metadata (HF YAML front matter / `vox` metadata block) and in the receipt, NOT in the repo id. Existing ids `<ns>/vox-mens-qwen3-<size>` (phase 324) are kept unchanged. Reason: renaming published repos is one-way, and the current ladder adapters are trained on mixed data (`target/dogfood/train_mixed.jsonl`), so their honest spoke value is a "mixed"/multi-spoke value, not one spoke. A spoke-specific repo id is only needed when a single-spoke adapter is first published; that naming is deferred to that moment (see Deferred) and must be raised with the user before any such repo is created.
- **D-02 [ASSUMED]:** A mixed-data adapter is gated against every spoke suite whose gate file applies to it, and its card shows one gate state per spoke. It is never shown as "passed" for a spoke whose suite it did not run.

**Gate policy and promotion**
- **D-03 [USER]:** Per-spoke gate files (`mens/config/eval-gates.yaml`, `eval-gates-rust.yaml`, `eval-gates-agents.yaml`, `eval-gates-ladder.yaml`) are the policy source of truth, and promotion consults the gate(s) mapped to the adapter's spoke via `mens/config/domain-profiles.yaml`. Plans start from a wired-vs-paper inventory of each gate file and wire or delete each paper item.
- **D-04 [USER]:** GSM8K and MATH remain collateral (no-regression) checks only. They are never the evidence that a spoke works.

**Receipts**
- **D-05 [USER]:** Receipts hash their inputs (eval item files, adapter weights/revision, base snapshot, decode settings) and the gate policy (a canonical hash of the effective gate rule), in addition to the existing `source_commit` (commit 2e9f45caf). A receipt whose policy hash differs from the current policy renders as "needs re-gate".
- **D-06 [ASSUMED]:** The 1.7B/4B/8B receipts that passed under the retired 5%-relative/n=32 rule are rendered "needs re-gate" by this phase; re-running those sizes is scheduled but optional and gated on the stated wall-clock budget, not required to close the phase. Proof of the behavioral suites and their known-bad controls is done on the smallest size (0.6B) first.
- **D-18 [USER, coordinator 2026-11-05]:** First wave of 338, ahead of everything else (but still after phase 335 lands): close the stale-pass publish hole in hub-sync. Today `verify_publish_gates` accepts any collateral report whose status is `"pass"`, including the 32-item-era reports for 1.7B/4B/8B that carry no `slice_blake3`; the receipt gate checks slice pins only when eval-gate is re-run, so a sync of those sizes could publish weights on a stale pass. The publish path must refuse when (a) the collateral report lacks the current slice pins, (b) its rule or alpha differs from `mens/config/eval-gates-ladder.yaml`, or (c) its `grader` field (added in f51c569c0, currently "grade_final_answer v2") differs from the current grader. Each of the three refusals is mutation-proven (break the guard, watch its test fail, restore).

**Behavioral suites**
- **D-07 [USER]:** Harness/agent spokes (tool-selection, argument-generation) are gated on multi-turn episodes against the real Vox harness with its live tool schemas, checking that the model: picks the right tool; emits schema-valid arguments; consumes tool results; recovers from a tool error; stops when done; refuses out-of-scope calls.
- **D-08 [USER]:** Episode scoring comes from episode outcome and schema validation, never from text matching a reference.
- **D-09 [USER]:** vox-lang and rust spokes are gated on compile-and-test pass rates.
- **D-10 [USER]:** Every suite ships a known-bad control (e.g. the base model, or a shuffled-tool adapter) that must fail it. A suite whose control passes is treated as non-discriminating and cannot gate.
- **D-11 [ASSUMED]:** Tool schemas are read from the live registry at eval time (the same source the harness serves), not from a hand-copied snapshot; the receipt records the hash of the schema set used, so a schema change also renders "needs re-gate".

**Dataset repos**
- **D-12 [ASSUMED]:** Eval and data items are published as Hub dataset repos, versioned by revision, and receipts reference the dataset revision they used. New dataset repo ids are new names, not renames; the planner picks ids consistent with the phase-324 naming config and lists them explicitly. Every actual push to the Hub is a human-approved checkpoint (no autonomous push).
- **D-13 [ASSUMED]:** Third-party collateral items (GSM8K, MATH) are only republished if their licences allow redistribution; otherwise the dataset repo publishes item ids plus hashes and a pointer to the upstream source.

**Constraints on execution**
- **D-14 [USER]:** Reuse existing eval code (`crates/vox-eval`, `crates/vox-ml-cli` `mens eval-*`, the harness in vox-orchestrator / vox-actor-runtime) rather than a new framework. Test-first; no new crate edges without a proposed exception in the PR description; all LLM calls through `vox_actor_runtime::llm`.
- **D-15 [USER]:** Phase 335 is actively editing `crates/vox-eval` and `crates/vox-ml-cli/src/commands/mens/eval_*`. Plans that touch those files are ordered after phase 335 lands and must re-read those files at execution time rather than assume their current shape. Phase 337 (eval throughput) also precedes 338.
- **D-16 [USER]:** Every plan that runs an eval states the expected wall-clock per size and avoids redundant runs (reuse base-side generations and existing records where the protocol matches).
- **D-17 [USER]:** 338's output contract for phases 339 (cards v2) and 340 (local selector) is explicit: a versioned, machine-readable per-spoke gate state and receipt schema that those phases read, with "passed", "failed", "needs re-gate" and "not run" as distinct states.

### Claude's Discretion
- Exact file layout of new suites, episode fixture format, and which existing eval module hosts the episode runner, provided D-14 holds.
- The canonical-hash algorithm and where the hash helper lives, provided it goes through `vox-crypto` (repo cryptography policy).
- Plan granularity and wave layout.

### Deferred Ideas (OUT OF SCOPE)
- Repo-id scheme for a future single-spoke adapter (e.g. a spoke suffix). Raise with the user before the first such repo is created; renaming is one-way.
- Re-gating 4B/8B under the new rule if the measured wall-clock does not fit the budget; they stay "needs re-gate" until run.

Out of scope for this phase (CONTEXT `<domain>`): card redesign (339), local selector/pipeline (340), eval throughput work (337), debt sweep (341).
</user_constraints>

<phase_requirements>
## Phase Requirements

No requirement IDs are mapped (ROADMAP: `**Requirements**: TBD`). The planner should mint IDs from the table below (minted as `SPOKE-` to avoid colliding with phase 315 GATE-01/02 in REQUIREMENTS.md), one per testable outcome.

| Suggested ID | Description | Research Support |
|----|-------------|------------------|
| SPOKE-01 | Publish/card/serve refuse a stale collateral pass (pins, rule, alpha, grader), mutation-proven | "D-18" section |
| SPOKE-02 | Every key in each gate file is wired to a producer and reader or deleted | "Wired vs paper inventory" |
| SPOKE-03 | Gate policy chosen from the adapter's spoke via `domain-profiles.yaml` | "Spoke routing" |
| SPOKE-04 | Receipt v2 hashes inputs + canonical policy; stale receipts render "needs re-gate" | "Output contract" |
| SPOKE-05 | Spoke recorded in adapter card, receipt and Hub card metadata; repo ids unchanged | "Spoke identity and repo ids" |
| SPOKE-06 | Agent episode suite (six behaviors) with scripted controls that discriminate | "Behavioral suites" |
| SPOKE-07 | vox-lang gate on hidden-test compile-and-test (HumanEval-Vox held-out); rust suite built | "Behavioral suites" |
| SPOKE-08 | Eval/data dataset repos published (human checkpoint), receipts cite dataset revision | "Dataset repos" |
</phase_requirements>

## Summary

The gate machinery is more wired than the CONTEXT assumed in one place and far less in another. **More:** the serve boundary and `eval-gate` already re-derive a collateral pass from paired counts at the policy alpha, and they already pin the item sets; the exact function to reuse exists (`verify_reasoning_slices`). **Less:** (1) the two consumers that decide what gets *published* and what a *card* says (`verify_publish_gates` at `hub_sync.rs:160`, `card_status` at `hub_card.rs:217`) still trust the bare `status == "pass"`, and **nobody on any path checks the `grader` field**; (2) the per-spoke `eval_gate:` field in `domain-profiles.yaml` is parsed and existence-checked but never used to pick a policy; (3) the `rust_compile_rate`, `clippy_clean_rate`, `tool_call_valid_json_rate` and `tool_name_exists_rate` gate keys are produced by `vox corpus eval --input <train jsonl>`, i.e. they score the **training corpus's reference responses**, not the adapter; (4) several ladder/full-policy keys never fire on current artifacts (telemetry is `{"event","payload":{...}}` but the gate reads top-level keys; `truncation_fraction` and `mcp_tool_schema_kpi.json` and `review_metrics.json` and `bfcl_results.json` have no writer). On disk, the 1.7B, 4B and 8B collateral reports are `status: pass`, 32-item, with no `slice_blake3`, no `grader`, no `rule`/`alpha`, and the locally rendered cards already say `passed` / `gate-passed` for them; Hub index lists v5/v5/v4 for those sizes.

**CONTEXT correction (D-01/D-02 premise):** the ladder adapters are not mixed-spoke. `target/dogfood/train_mixed.jsonl` has 1166 rows, lanes `vox_codegen` 1021 and `vox_docs_qa` 145, which is exactly `mix-vox-lang.yaml`'s `include_lanes: [vox_codegen, vox_docs_qa, vox_lang_tier_b]`. Every published adapter is a `vox-lang` adapter. No tool-selection, argument-generation or rust adapter exists, and no rust/agent training data is on disk (`mens/data/mix_sources/` is absent). So the agent and rust suites cannot gate any existing adapter; their proof is the suite itself plus controls on the base model at 0.6B.

Nothing in the code forces a model-repo rename: `repo_id_for_size` takes only a namespace and a size (`hub_repo.rs:45-54`). D-01 (spoke in metadata, ids unchanged) is safe. The only genuinely one-way choices are the new dataset repo ids and their visibility (see `## ONE-WAY DECISION`).

**Primary recommendation:** Wave 1 = one shared pure function `check_collateral_report(report, requirement)` in `vox-eval` (composes `verify_reasoning_slices` + pin compare + grader compare), called from `verify_publish_gates`, `card_status`, `assert_collateral_gate` and `check_run`, each guard mutation-proven. Then receipt v2 + canonical policy hash + a versioned per-spoke gate-state JSON contract. Build the agent episode runner as pure types/scoring in `vox-eval` with the JSON-Schema validator and model-turn function injected by `vox-ml-cli` (zero new crate edges), and prove each suite discriminates with model-free scripted controls first (seconds, no GPU), then one 0.6B base-model run.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Collateral-evidence validity (pins, rule, alpha, grader) | `vox-eval` (L2, pure) | callers in `vox-populi`, `vox-ml-cli` | One function consumed by four sites; `vox-populi -> vox-eval` and `vox-ml-cli -> vox-eval` edges already allowed |
| Gate policy parse + canonical hash | `vox-ml-cli` `eval_gate/policy.rs` | `vox-populi` reads 3 keys with `serde_yaml::Value` (defactor) | `EvalGatePolicy` lives in ml-cli; populi cannot depend on ml-cli (cycle) |
| Receipt writer (v2) | `vox-ml-cli` `eval_gate/run_gate.rs` | — | Only writer of `gate_receipt.json` |
| Gate-state derivation (`passed/failed/needs_re_gate/not_run`) | `vox-eval` (pure fn over receipt JSON + current requirements) | `vox-populi` (cards), `vox-orchestrator::models` (340) read the JSON | Pure so 339/340 do not need ml-cli |
| Spoke routing (adapter -> spoke -> gate file) | `vox-populi` `tensor/domain_profiles.rs` (profile data) | `vox-ml-cli` `eval-gate` | Profiles and `include_lanes` already live there |
| Episode types, mock world, scorer, scripted controls | `vox-eval` (L2, pure, no deps) | — | Zero deps; schema validator and model turns injected |
| JSON-Schema validation of tool args | `vox-ml-cli` (`vox-jsonschema-util`, edge exists) | injected into `vox-eval` as a closure | Avoids a new `vox-eval -> vox-jsonschema-util` edge |
| Live tool catalog (name, description, schema) | `vox-orchestrator-mcp` `registry::tool_registry()` | exported to a hash-pinned file by `vox-cli` | `vox-ml-cli` has no edge to `vox-orchestrator-mcp` or `vox-mcp-registry` |
| Local adapter inference for gating | `vox-plugin-host` `MlBackend` via `reasoning_eval::with_backend` | `vox mens serve` + `llm_chat` for hosted-path parity | Existing practice; see Open Question 1 on D-14 |
| vox-lang compile-and-test | `vox-corpus::humaneval_runner::verify_program` (driven by `vox model eval-corpus`, `vox-cli`) | — | Hidden tests + oracle-neutralisation canary already exist |
| Dataset repo publish | `vox-populi` `hub.rs` / `hub_sync.rs` (new `hub_dataset.rs`) | `hf-hub` 1.0.0 `RepoTypeDataset` | `hf-hub` supports datasets; vox code is model-only today |

## Standard Stack

### Core (all already in the workspace; no new external package)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `vox-eval` | workspace | Hosts pure gate/episode logic | L2, 8.6k LoC, deps are only serde/serde_json/regex/tempfile/log/xxhash [VERIFIED: crates/vox-eval/Cargo.toml] |
| `vox-crypto` | workspace | `secure_hash` (BLAKE3), `hex_encode`, `hash_fast_hex` | Cryptography policy; already used by `item_checkpoint.rs:139-140` and `hub_sync.rs:545` |
| `vox-jsonschema-util` | workspace (`jsonschema 0.46`) | `compile_validator`, `validate` for tool-arg schemas | Already an edge from `vox-ml-cli` [VERIFIED: crate-edges.allow.v1.json] |
| `hf-hub` | 1.0.0 | Dataset repos: `client.dataset(owner, name)`, `create_repository().repo_type(RepoTypeDataset)`, generic `upload_folder`/`create_tag` | `impl<T: RepoType> HFRepository<T>` [VERIFIED: hf-hub-1.0.0 src/repository/mod.rs:43,644; upload.rs:91; commits.rs:115] |
| `serde_yaml` (`serde_yaml_ng` 0.10) | workspace | Policy parse | Root `Cargo.toml:211` |
| `vox-plugin-host` MlBackend | workspace | In-process adapter inference | `reasoning_eval.rs:199-243` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Pure episode logic in `vox-eval` + injected validator | Host in `vox-ml-cli` | `vox-ml-cli` is already 38.9k LoC against a `max_loc = 20_000` entry; `vox-eval` has no budget entry. Keep new code out of ml-cli except thin glue |
| Hash-pinned catalog export (`vox harness tool-catalog`) | New edge `vox-ml-cli -> vox-orchestrator-mcp` | Edge needs a USER-AUTHORIZED `exceptions` entry and pulls a 40k-LoC crate into the ML binary. See Open Question 2 |
| Re-use `hash_json` (`item_checkpoint.rs:139`) for the policy hash | New canonical-JSON helper | `hash_json` hashes `v.to_string()`; the workspace enables `serde_json` `preserve_order` [VERIFIED: crates/workspace-hack/Cargo.toml:153 `features = ["alloc", "preserve_order", "raw_value", "unbounded_depth"]`], so key order is insertion order, not canonical. A policy hash needs recursive key sorting |

**Installation:** none. **Version verification:** `hf-hub 1.0.0` confirmed in `Cargo.lock:6075-6076` and the registry source tree. No package is added, so the Package Legitimacy Gate does not trigger.

## Package Legitimacy Audit

No external package is installed or upgraded by this phase. `jsonschema`, `hf-hub`, `serde_yaml_ng`, `schemars` are existing workspace dependencies.

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
 TRAIN (vox mens train)                         EVAL INPUTS (pinned)
   target/dogfood/train_mixed.jsonl                mens/data/{gsm8k,math500,vox_think}_*/reasoning.jsonl  (slice pins in ladder yaml)
   -> training_manifest.json  (+ NEW: train lane histogram, spoke)   contracts/eval/humaneval-vox (held-out fixtures)
   -> adapter_card.json       (+ NEW: spoke)                          NEW: mens/data/agent_episodes/*.jsonl, rust_bench/*
        |                                                              NEW: tool-catalog.json (generated from live registry)
        v                                                                       |
 SPOKE ROUTER  (NEW)  adapter lanes --include_lanes--> spoke(s) --domain-profiles.eval_gate--> gate file(s)
        |
        +--> ladder gate (all spokes): eval-collateral-damage -> collateral_damage_report.json
        |        (slice_blake3, grader, reasoning_slices{rule,alpha,benches})
        +--> vox-lang: eval-local heldout (52 tasks)  +  humaneval-vox held-out compile+hidden-test (verify_program)
        +--> rust:     NEW rust_bench compile+hidden-test (cargo, sandbox tmpdir)
        +--> tool-selection / argument-generation:  NEW episode runner
        |       model turn (MlBackend infer, checkpointed per prompt)  <->  mock tool world (scripted results, errors)
        |       schema check (jsonschema, live schema)  ->  EpisodeOutcome  -> six behavior scores
        |       controls: scripted policies (unit tests) + base model (0.6B run)
        v
 eval-gate (run_gate.rs)  -> gate_receipt.json v2  { policy_blake3, inputs{...}, source_commit, per-spoke rows }
        v
 gate_state(receipt, current_requirements)  [pure, vox-eval]  -> passed | failed | needs_re_gate | not_run
        v
 spoke_gates.json (versioned contract)  --read by-->  hub_sync (verify_publish_gates, bundle)  /  hub_card (339)  /  selector (340)
        v
 Hub: model repo <ns>/vox-mens-qwen3-<size> (spoke = tag + metadata)    dataset repos <ns>/...  (HUMAN CHECKPOINT before push)
```

### Recommended Project Structure
```
crates/vox-eval/src/
  collateral_evidence.rs   # NEW: CollateralRequirement + check_collateral_report (D-18) + gate_state()
  episode.rs               # NEW: Episode, MockWorld, ToolCall, EpisodeOutcome, six behavior scorers
  episode_controls.rs      # NEW: scripted policies: oracle, never_call, wrong_tool(shuffled), call_forever, ...
crates/vox-ml-cli/src/commands/mens/
  eval_gate/spoke_route.rs # NEW (thin): adapter -> spoke -> policy path
  eval_episodes.rs         # NEW (thin glue): fixtures load, MlBackend turn fn, schema validator closure, checkpoint
crates/vox-populi/src/mens/
  hub_dataset.rs           # NEW: dataset repo ids + publish (reuses hub.rs patterns)
contracts/mens/
  spoke-gate-state.v1.schema.json   # NEW: the 339/340 contract
mens/data/agent_episodes/  # NEW fixtures (hand-authored; see Pitfall 7)
mens/data/rust_bench/      # NEW fixtures
```

### Pattern 1: One validity function, four callers (D-18)
**What:** a pure function over the report JSON and a small requirement struct; each consumer maps its `Err(String)` to its own surface (bail / `Refused` reason / serve error / gate row).
**When to use:** any place that today writes `status == "pass"`.
**Example (shape only; names are proposals):**
```rust
// vox-eval: collateral_evidence.rs
pub struct CollateralRequirement<'a> {
    pub alpha: f64,                                   // policy collateral_damage.alpha
    pub slice_pins: &'a [(&'a str, &'a str)],         // (slice name, pinned whole-file blake3)
    pub grader: &'a str,                              // vox_eval::qwen_reasoning::GRADER_ID
}
pub fn check_collateral_report(report: &serde_json::Value, req: &CollateralRequirement)
    -> Result<(), String>
{
    // 1. rule + alpha + counts: reuse, do not re-implement
    crate::reasoning_slice::verify_reasoning_slices(report, req.alpha)?;
    // 2. item sets: same comparison as CollateralDamageGate::check_report_slices (policy.rs:137-158)
    for (name, pin) in req.slice_pins { /* report["slice_blake3"][name] == pin else Err */ }
    // 3. grader: NEW, no consumer checks it today
    match report["grader"].as_str() { Some(g) if g == req.grader => Ok(()), other => Err(/* names both */) }
}
```
Source for the pieces reused: `verify_reasoning_slices` [VERIFIED: crates/vox-eval/src/reasoning_slice.rs:477-545], `check_report_slices` [VERIFIED: crates/vox-ml-cli/src/commands/mens/eval_gate/policy.rs:137-158].

### Pattern 2: Episode = fixture + mock world + outcome predicate (D-07/D-08)
**What:** the model never talks to real tools. A fixture declares offered tools (names that must exist in the live catalog), a scripted `MockWorld` (per-tool result generator with per-episode nonces and optional first-call error), and an *outcome predicate* over the world's final state and the call trace. Text of the final message is only checked for "is a non-call terminal turn", never compared to a reference.
**Why not run the real harness dispatch:** `run_agent_turn` dispatches to `handle_tool_call_with_receipt` (real handlers, side effects) [VERIFIED: agent_loop.rs:981-989]. Reuse its *presentation* (`select_tools_for_turn`, `tool_input_schema`), its *call-parsing* (salvage policy) and its *loop bound* (`DEFAULT_MAX_ITERATIONS = 8`, agent_loop.rs:172), not its dispatch.

### Pattern 3: Checkpointed model turns for free resume
`ItemCheckpoint::infer(prompt_json, infer)` keys a generation on its prompt request [VERIFIED: item_checkpoint.rs:1-14]. With greedy decode and a deterministic mock world, a re-run replays turns from the checkpoint, so multi-turn episodes resume per turn with no new machinery. Put catalog hash, episode-set hash, system prompt and decode settings in the checkpoint header inputs.

### Anti-Patterns to Avoid
- **Scoring a gate from corpus metrics.** `vox corpus eval --input` rates the training file (see inventory). Never feed a gate a number that does not come from the adapter's generations.
- **Hashing `Value::to_string()` for a policy hash** (insertion order under `preserve_order`).
- **A second `status == "pass"` site.** After Wave 1 there must be none; add a test that greps the source for the literal in the four files (cheap structural guard).
- **Counting server-side retries.** `vox mens serve` chat route sends `max_retries: 3` toward a valid `{"name","arguments"}` object [VERIFIED: handlers.rs:364]; a gate through that route measures the retry loop. Pin `max_retries` to 0 and record it, or gate in-process.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Paired significance | A new test | `vox_eval::reasoning_slice::mcnemar_one_sided_p(losses, gains)` (reasoning_slice.rs:80) | Exact, already mutation-proven in 335. Use for suite-vs-control improvement (opposite direction) |
| JSON-Schema validation | A schema walker | `vox_jsonschema_util::{compile_validator, validate}` | `jsonschema 0.46` already pinned; live schemas are draft-07 [VERIFIED: input_schemas.rs:1] |
| vox-lang compile+hidden test | A new vox harness | `vox_corpus::humaneval_runner::verify_program` | `vox check` then `vox run --mode interp`, fixture-supplied assertions, oracle-neutralisation canary [VERIFIED: verify.rs:1-60; compose.rs:1-50] |
| Tool-call text parsing | A new parser | The harness salvage shape `{"name","arguments"}` (agent_loop.rs:636-742); mirror it in `vox-eval` with a test that both agree on a fixture list | The suite must accept exactly what the harness accepts |
| Per-turn generation cache | New cache | `ItemCheckpoint` | Resume, torn-line handling, key discipline exist |
| Hub dataset upload | HTTP by hand | `hf-hub` `HFRepository<RepoTypeDataset>` | Generic over repo type |
| Hashing | `blake3`/`sha2` direct | `vox_crypto::{secure_hash, hex_encode, hash_fast_hex}` | AGENTS.md cryptography policy; detector `vox/crypto/banned-crate-import` |

**Key insight:** every missing piece of Phase 338 is a *consumer* or a *composition* of something that exists. The new work is fixtures, one pure validity function, a receipt schema, and the episode scorer.

## Wired vs paper inventory (D-03)

Method: for each key, the reader in `check_run.rs` (all four policy files parse into the single `EvalGatePolicy` struct, `policy.rs:13-66`), then the producer of the artifact it reads, then whether the key ever yields a row in the real receipts. The five receipts read for 1.7B/4B/8B each contain exactly five gate rows: `leakage`, `anti_stub`, `pass_at_k`, `review_recurrence`, `collateral_damage` [VERIFIED: mens/runs/qwen3-{1.7b,4b,8b}/gate_receipt.json read this session]. WIRED = reader + a real producer that measures the adapter. PAPER = parsed but nothing produces its input, or the input measures something other than the adapter, or the reader cannot match current artifact shape.

### `mens/config/eval-gates-ladder.yaml` (promotion gate for ladder sizes)
| Key | Reader (file:line) | Producer | Verdict |
|---|---|---|---|
| `pass_at_k.min_pass_rate_at_1/at_k/max_regression_drop/metrics_file/baseline_file/block` | check_run.rs:728-841 | `eval-local` writes `benchmark_passatk.json` (eval_local.rs:243-254); baseline from `baseline_passatk.json` | WIRED (adapter, vox heldout 52 tasks) |
| `anti_stub.*` (5 keys) | check_run.rs:469-537 | `eval-local` -> `eval_local_report.json` | WIRED |
| `collateral_damage.block/alpha/slices` | check_run.rs:961-1039 (re-derives via `verify_reasoning_slices` + `check_report_slices`) | `eval-collateral-damage` (eval_collateral.rs:175-187) | WIRED in eval-gate and serve; **NOT enforced at publish/card** (D-18) |
| `collateral_damage.require_report` | check_run.rs:967, 977 | — | WIRED (not set in ladder yaml; default false) |
| `supervised_ratio.*` | check_run.rs:169-208 reads top-level `supervised_ratio_pct` | telemetry lines are `{"event":"step","payload":{... "supervised_ratio_pct": 2.70 ...}}` [VERIFIED: mens/runs/qwen3-1.7b/telemetry.jsonl] | PAPER (reader cannot see nested payload; no row in any receipt) |
| `truncation.*` | check_run.rs:225-246 needs manifest key `truncation_fraction` | 0 occurrences in the three real `training_manifest.json`; no writer in `crates/` | PAPER |
| `throughput.*` | check_run.rs:138-147, 248-271 reads top-level `tokens_per_sec` | nested in `payload`; Apple device key falls back to `cpu` (`1.0`) | PAPER |
| `eval_local.*` (`block: false`, min 0.0) | check_run.rs:286-340 | `vox corpus eval` `vox_parse_rate` = parse rate of the **training corpus** | PAPER (inactive in ladder; where active it measures data) |
| `per_context: {}` | check_run.rs:544-604 | — | PAPER (empty map) |
| `modal_mix.*` | check_run.rs:609-650 | `eval_results.modal_breakdown` or `mix_report.json`; not verified to exist for ladder runs | PAPER until a producer is shown |
| `mcp_tool_schema.*` | check_run.rs:652-725 | `mcp_tool_schema_kpi.json`; no writer in `crates/` or `scripts/` | PAPER |
| `review_recurrence.*` | check_run.rs:843-902; absent file is forced `block: false` (check_run.rs:855-864) | `review_metrics.json`; no producer | PAPER |

### `mens/config/eval-gates.yaml` (full policy; default of `eval-gate` CLI, hard-coded for cloud at train_arm.rs:941)
Same keys as above with blocking values, plus: `pass_at_k`/`anti_stub` WIRED; `supervised_ratio`, `truncation`, `throughput` (`block: true`) and `review_recurrence` (`block: true`) PAPER for the reasons above; `eval_local.block: true` measures the training corpus.

### `mens/config/eval-gates-rust.yaml` (profile `rust`)
| Key | Reader | Producer | Verdict |
|---|---|---|---|
| `rust_compile_rate.min_pct 0.70 block` | check_run.rs:342-371 (absent key = pass) | `compute_rust_spoke_metrics` (vox-corpus eval_rust_metrics.rs:35-76) compiles **`response` of `rust_authoring` rows in the input JSONL**, first 100 snippets | PAPER for an adapter: scores the corpus's reference answers. Also: no rust rows on disk |
| `clippy_clean_rate.min_pct 0.50` | check_run.rs:373-400 | same | PAPER |
| `review_recurrence.max_repeated_finding_rate 0.20` | check_run.rs:843-902 | none | PAPER |
| `supervised_ratio.min_pct 10.0 block` | check_run.rs:169-208 | nested telemetry | PAPER |

### `mens/config/eval-gates-agents.yaml` (profiles `tool-selection`, `argument-generation`)
| Key | Reader | Producer | Verdict |
|---|---|---|---|
| `tool_call_valid_json_rate.min_pct 0.90` | check_run.rs:402-430 | `compute_agentic_spoke_metrics` (vox-corpus eval_agentic_metrics.rs:46-126) scans `agent_trace`/`tool_trace` rows' `response` in the input JSONL; "valid" = object has `tool_name`, `arguments`, `result`, `success` (lines 6-13) | PAPER: measures corpus rows, requires the training-trace shape, no multi-turn, no schema validation |
| `tool_name_exists_rate.min_pct 0.85` | check_run.rs:432-459 | same; name check vs `vox_mcp_registry::TOOL_REGISTRY` (lines 16-26) | PAPER (corpus) |
| `supervised_ratio.min_pct` | check_run.rs:169-208 | nested telemetry | PAPER |
| header mentions `tool_call_salvage_rate` | none (not a key in the file) | `eval-local` still emits it (eval_local.rs:258-264) | orphan producer |

### `mens/config/eval-gates-bfcl.yaml` (not one of the four, but shipped)
`bfcl_accuracy.*` is read at check_run.rs:904-959 (`bfcl.rs`), but `bfcl_results.json` has no writer (only `baseline.rs:114` reads it). PAPER.

### Struct fields with no YAML key
`EvalGatePolicy.version` and `perplexity` carry `#[allow(dead_code)]` (policy.rs:14-19, 231-238): never read.

### Profile -> gate routing (`domain-profiles.yaml`)
`eval_gate:` is deserialized (domain_profiles.rs:81, 207), existence-checked (spoke_validate.rs:55-59), and otherwise unused: the only reference is `let _ = &eff.eval_gate;` in a test [VERIFIED: domain_profiles.rs:274]. `eval-gate` defaults to `mens/config/eval-gates.yaml` (run_gate.rs:15), the cloud path hard-codes it (train_arm.rs:941), scripts pass `--policy` by hand. `manifest.spoke` is read at check_run.rs:936-939 but no writer sets it (`grep '"spoke"'` finds only corpus/mod.rs:847 and tests). **No promotion path chooses a gate file from a spoke today.**

### Other paper found on the way
- `vox mens eval-gate --five-spokes` runs `vox_corpus::flywheel::run_before_after_five_spoke_evaluation`, a 25-case suite with **hard-coded candidate strings** ("Realistic uncalibrated outputs", flywheel.rs:659-668) and then writes a scorecard with literal loss curves and `"Qwen3.5-27B"` (dispatch.rs:693-703). It is a simulation. Do not mistake it for a behavioral gate; flag for 341.
- `spoke_roundtrip.rs::verify_mcp_roundtrip` checks JSON shape, known tool name and required arg *keys* only (lines 521-610); `verify_rust_roundtrip` is `syn::parse_file` + stub filter + string audits, no compile (lines 273-322). Single-turn string verifiers; reusable for admission, not as spoke gates.
- `scripts/agent_system_prompt.txt` (named by profiles `tool-selection`/`argument-generation`) does not exist in `scripts/`.

## D-18: stale-pass publish hole

**Acceptance of a stale pass, exact sites** (verbatim):

| Site | Code | Re-derives? |
|---|---|---|
| publish and pull | `hub_sync.rs:160`: `if val.get("status").and_then(\|s\| s.as_str()) != Some("pass") {` inside `verify_publish_gates` (hub_sync.rs:149-205) | No |
| card state | `hub_card.rs:217`: `Some(c) if c["status"].as_str() == Some("pass") => {}` inside `card_status` (hub_card.rs:199-268); this drives the `gate-passed` tag at hub_card.rs:358-360 and the index "Gate" column | No |
| serve | `dispatch.rs:922`: `if report.get("status").and_then(\|s\| s.as_str()) != Some("pass") {` then `verify_reasoning_slices(&report, alpha)` at :927 and `collateral_eval/<slice>/reasoning_report.json` n and status checks :933-962 | Yes (rule, alpha, counts), no pins, no grader |
| eval-gate | `check_run.rs:991-1012`: `("pass", Some(alpha)) => verify_reasoning_slices(&v, alpha).and_then(\|_\| cd.check_report_slices(...))` | Yes (rule, alpha, counts, pins), no grader |

`verify_publish_gates` callers: `hub_sync.rs:382` (`stage_publish_bundle`), `hub_sync.rs:856` (pull verification, `require_gates`; on failure the pulled directory is deleted, hub_sync.rs:861-864), `train_arm.rs:677` (post-training auto-publish; an `Err` becomes `force_ungated = true`, i.e. an *ungated-labelled* baseline upload, train_arm.rs:676-693).

**Fields available to refuse on** (written by `run_collateral_damage_with` / `_fail_fast_with`, eval_collateral.rs:175-187 and 333-350):
`"status"`, `"failed_on"`, `"reports"`, `"reasoning_slices"` (with `"scope_note"`, `"rule"`, `"alpha"`, `"slice_size"`, `"benches"`, `"pooled"`, `"power"`, reasoning_slice.rs:428-436), `"slice_blake3"`, `"grader"`, plus `"fail_fast"`, `"fail_fast_order"`, `"not_run"` in fail-fast mode.

**Where the current values come from:**
| Refusal | Current value | Source |
|---|---|---|
| (a) slice pins | `collateral_damage.slices.<name>.blake3` for `gsm8k_think`, `math500_think`, `vox_think` | `mens/config/eval-gates-ladder.yaml:99-` (e.g. line 101 `blake3: 8425aff2fe6078b4ee8ee879fdaa2bfbc74df8fab96311800a6586748f383a5e`); names from `pub const SLICE_NAMES: [&str; 3] = ["gsm8k_think", "math500_think", "vox_think"];` (reasoning_bench.rs:18); compare function exists: `check_report_slices` (policy.rs:137-158) |
| (b) rule | `pub const PAIRED_RULE: &str = "exact one-sided McNemar on paired items, fail when p < alpha";` (reasoning_slice.rs:22), compared at reasoning_slice.rs:486 | code constant |
| (b) alpha | `alpha: 0.05` (ladder yaml:95), compared at reasoning_slice.rs:497 | policy |
| (c) grader | `pub const GRADER_ID: &str = "grade_final_answer v2";` (qwen_reasoning.rs:153); written at eval_collateral.rs:184 and :345 | code constant; **no consumer compares it** (grep: only the two writers and test asserts at eval_collateral.rs:1467-1475) |

**On-disk evidence** (read this session, `mens/runs/qwen3-<size>/collateral_damage_report.json`):

| Size | status | mtime | `slice_blake3` | `grader` | `reasoning_slices.rule`/`alpha` | `slice_size` | Receipt `overall_passed` | Receipt `source_commit`/policy hash |
|---|---|---|---|---|---|---|---|---|
| 0.6B | fail (`gsm8k_think` 0.5 -> 0.46875) | Nov 5 05:15 | absent | absent | absent | 32 | false (Nov 4 11:23, says report missing) | absent |
| 1.7B | pass | Nov 4 15:34 | absent | absent | absent | 32 | true | absent |
| 4B | pass | Nov 1 01:58 | absent | absent | absent | 32 | true | absent |
| 8B | pass | Nov 3 00:04 | absent | absent | absent | 32 | true | absent |
| 14B/32B | no report | — | — | — | — | — | 14B receipt false | — |

All three `pass` reports would clear `verify_publish_gates` today, and the locally rendered cards already say so: `mens/runs/hub_cards/vox-mens-index/README.md` lists 1.7B/4B/8B `passed` and the 1.7B card carries tag `gate-passed`.

**Plan consequences**
1. **Wave 1, test first.** For each of (a), (b)-rule, (b)-alpha, (c): a fixture report that is valid except for that one thing, and a test that `verify_publish_gates(.., false)` errors naming it. Mutation proof: delete the guard, watch exactly that test fail, restore (the repo's PR rule is to verify security-relevant guards by mutation and to guard the mutation with a before/after presence check).
2. **The existing fixture is the hole.** `hub_sync.rs:1383-1386` writes `r#"{"status":"pass"}"#` and the happy-path test `stage_bundle_writes_card_and_hashes_and_strips_host_paths` (hub_sync.rs:1464-1480) asserts `gate_status == "passed"` on it. That fixture must grow real pins/rule/alpha/grader (build it from a helper that generates a valid report from `reasoning_slices_json`, not a hand-typed blob). `hub_card_tests.rs` has the same pattern.
3. **Signature change.** `verify_publish_gates(run_dir, force_ungated)` has no access to the policy. Add a `&CollateralRequirement` parameter (built once by the ml-cli caller from `load_policy`, and by `hub_sync` entry points that already take `workspace_root`). Do not read `eval-gates-ladder.yaml` implicitly from a path relative to `run_dir`.
4. **Feature wiring, not a new edge.** `vox-populi -> vox-eval` is already allowed (two entries in `crate-edges.allow.v1.json`) but `vox-eval` is only enabled by `mens-train` (`vox-populi/Cargo.toml:78`); `hub_sync` compiles under `mens-hf-hub`. Add `dep:vox-eval` to `mens-hf-hub`.
5. **Side effect to test and document:** pulls of the currently published v5/v5/v4 bundles (stale reports inside) will be refused by `require_gates` and the pulled dir deleted (hub_sync.rs:855-864). That is correct behaviour, but phase 326's auto-retrieve path must say why.
6. **Card/serve alignment in the same wave.** `card_status` must call the same function so the card never says `passed` where publish refuses; serve (`assert_collateral_gate`) gains the pin and grader checks; `check_run` gains the grader check. Add the structural test: the literal `Some("pass")` appears in none of the four files except inside the shared function's caller glue.
7. **Receipt gate rows.** A receipt row can pass while the report is stale (the receipt records only `status=pass ...` text). Until receipts carry hashes (SPOKE-04), `verify_publish_gates` must judge the report itself and treat the receipt as supplementary.

## Spoke identity and repo ids (D-01, D-02)

- **Nothing forces a rename.** `repo_id_for_size(namespace, size)` formats `"{ns}/vox-mens-qwen3-{}"` from a validated size only (hub_repo.rs:45-54); `index_repo_id` -> `"{}/vox-mens-index"` (:57-62); size comes from the pinned base (`size_of_pin`, :17-25). No spoke input exists anywhere in naming. `HfSyncManifest` (`schema: "vox_mens_hf_sync_v1"`, hub_sync.rs:553-562) has `size, base_pin, corpus_hash, final_loss, measured_peak_mb, gate_status, files`; additive `#[serde(default)]` fields are backward compatible, and `manifests_match` (hub_sync.rs:229-234) only compares `size, base_pin, gate_status, files`.
- **The ladder repos are already populated on the Hub** (local index: 0.6B `v5`, 1.7B `v5`, 4B `v5`, 8B `v4`, namespace `brbrainerd`), so any rename is a real migration.
- **Correct D-01's premise.** The adapters are single-spoke `vox-lang` by training lanes (see Summary). Recommended rule, deterministic and testable: `spokes_of(adapter) = { profile : profile.mix_config.include_lanes ⊇ observed training lanes }`; record the lane histogram at train time (training manifest has no lane/domain/spoke field today; `adapter_card.json` has `adapter_sha256`, `train_data_sha256`, `training_manifest_digest`, `corpus_hash`, no spoke [VERIFIED: 1.7B adapter_card.json; adapter_card.rs:11-45]). Use `"mixed"` only when lanes map to more than one spoke. Precedence: explicit `--domain` (train_arm.rs:424-432 loads the profile) > lane inference > `unclassified` (no spoke suite applies; ladder collateral only).
- **Where the spoke is written:** `AdapterCard` (new optional field, `validate()` must tolerate old cards), `gate_receipt.json` v2, `hf_sync_manifest.json` (additive), card front matter as a tag `spoke-<name>` (valid by construction) plus a machine-readable `spoke_gates.json` shipped in the bundle (add to `OPTIONAL_BUNDLE_FILES`, hub_sync.rs:27-50). Custom top-level YAML keys are `[ASSUMED]` acceptable to HF; 336 validated only the current keys with `POST https://huggingface.co/api/validate-yaml` (336-01-SUMMARY). Wave 0 probe: POST a candidate front matter with a `vox:` block and read the response before committing to it; fall back to tag + JSON file.
- **D-02 applicability.** A suite applies to an adapter when the suite's spoke is among `spokes_of(adapter)`; the ladder collateral gate applies to every adapter. A vox-lang adapter therefore has gates `[ladder, vox-lang]` and no tool or rust rows. Do not emit `not_run` for non-applicable spokes (that would read as a failure); 339's size x spoke matrix renders "no adapter for this spoke" from the absence of an adapter, which is already `CardStatus::NotTrained`.

## Dataset repos (D-12, D-13)

- **Today:** publishing is model-only. `upload_model_folder` hard-codes `RepoTypeModel` (hub.rs:407-456), as do `ensure_ladder_repos` (hub_sync.rs:283-291) and the reserve flow. No dataset code exists in `vox-populi`. `hf-hub` 1.0.0 supports datasets with the same builder (`client.dataset(owner, name)`, `repo_type(RepoTypeDataset)`, generic `upload_folder`, `create_tag`) [VERIFIED: hf-hub-1.0.0 src/repository/mod.rs:43,644; upload.rs:91,716; commits.rs:115,414]. A `hub_dataset.rs` mirroring `upload_model_folder` is a small addition; tag each pushed revision `v<N>` with the existing `parse_version_tag`/`latest_version_tag` helpers (hub_sync.rs:589-600) so receipts can cite `dataset_repo@v<N>@<commit_oid>`.
- **Proposed ids (new names, not renames):** `<ns>/vox-mens-eval` (held-out reasoning slices, vox bench, episode fixtures, rust bench) and `<ns>/vox-mens-data` (training corpus manifest + data card; the model cards already cite a data contract page, hub_card.rs:23). Put them in `hub_repo.rs` next to `index_repo_id` with a unit test mirroring `an_illegal_namespace_or_size_is_refused`. Confirm with the user before the first push (`## ONE-WAY DECISION`).
- **Licences** (D-13):
  - GSM8K: MIT [CITED: https://huggingface.co/datasets/openai/gsm8k, card metadata "License: mit"]; the repo's own provenance file also records `"license": "MIT"` for `gsm8k_think` (mens/data/reasoning-slices-provenance-2026-11.json). Redistribution of the 128 items is permitted with the MIT notice.
  - MATH-500 (`HuggingFaceH4/MATH-500`): the dataset card **declares no licence** [CITED: https://huggingface.co/datasets/HuggingFaceH4/MATH-500, "does not declare a license"]; it is a 500-problem subset of MATH created for OpenAI's *Let's Verify Step by Step*, sourced from `openai/prm800k` (MIT repository) [CITED: https://github.com/openai/prm800k]; MATH itself declares MIT on the EleutherAI mirror [CITED: https://huggingface.co/datasets/EleutherAI/hendrycks_math]. The provenance file has no `license` field for `math500_think`. Recommendation: publish **ids + BLAKE3 + pointer** for `math500_think` (D-13 default), publish full items for `gsm8k_think` and the repo-authored `vox_think`; escalate to the user if they want MATH items inlined. `[ASSUMED]` legal reading; not legal advice.
  - `vox_think` rows 33-128 and `heldout_bench` are repo-authored.
- **Contamination pitfall:** a public dataset repo holding gold answers for held-out items makes them training data for every future model (including this project's own corpus builders, whose leakage check protects against bench text in the corpus, `check_leakage_precondition`, check_run.rs:49-88). Default the dataset repos to **private** (the `private` argument already exists on every publish function) and treat a visibility flip as a separate human checkpoint. Alternative that preserves verifiability: publish only hashes + a pointer for the held-out set and keep items private.
- **Receipt link:** receipts record `datasets: [{repo, revision, files{path: blake3}}]`; the card `datasets:` front-matter key is currently not emitted (hub_card.rs frontmatter lists none) and model-index uses made-up dataset `type:` strings (`vox-mens-heldout-bench`); wire real ids once the repos exist (339 renders it).

## Receipts (D-05) and the output contract (D-17)

**Current receipt** is an inline `serde_json::json!` literal, no struct (run_gate.rs:70-84): `"schema": "vox_mens_gate_receipt_v1"`, `timestamp_unix`, `"source_commit": env!("VOX_GIT_HASH")`, `run_dir`, `policy` (a **path string**, not a hash), `overall_passed`, `gates[{name, passed, block, message}]`. Consumers parse it as untyped JSON by `message` substring (`"not applicable"`, `"skipped"`, `"missing"`) in three places: `verify_publish_gates` (hub_sync.rs:174-184), `card_status` (hub_card.rs:241-249), `assert_serve_preconditions` (dispatch.rs:991-999), and the GUI (`vox-gui/.../mens_run_reports.rs`). **Do not change `gates[]` row shape**; add fields.

**Receipt v2 (additive; `schema: "vox_mens_gate_receipt_v2"`, keep v1 fields):**
```json
{
  "schema": "vox_mens_gate_receipt_v2",
  "source_commit": "...", "timestamp_unix": 0, "run_dir": "...",
  "policy": "mens/config/eval-gates-ladder.yaml",
  "policy_blake3": "<canonical hash of effective rule>",
  "inputs": {
    "adapter_sha256": "<adapter_card.adapter_sha256>",
    "base_pin": "Qwen/Qwen3-<size>@<sha>",
    "slice_blake3": {"gsm8k_think": "...", "math500_think": "...", "vox_think": "..."},
    "grader": "<GRADER_ID>",
    "decode_blake3": "<hash of {max_tokens, temperature, seed, prompt_protocol, scorer}>",
    "episode_set_blake3": "...", "catalog_blake3": "...",
    "datasets": [{"repo": "...", "revision": "..."}]
  },
  "overall_passed": true,
  "gates": [ ... unchanged v1 rows ... ]
}
```
- **Effective rule = YAML policy + code-defined rule identifiers.** `policy_blake3 = secure_hash(canonical_json({policy: parsed_yaml, code: {paired_rule: PAIRED_RULE, grader: GRADER_ID, heldout_prompt_protocol: ..., anti_stub_min_construct_richness: ...}}))`. A change to `PAIRED_RULE` or `GRADER_ID` must invalidate receipts even though no YAML changed. Canonicalisation: convert to a `BTreeMap`-keyed tree (recursive key sort), no whitespace, numbers via `serde_json` formatting; test that reordering YAML keys leaves the hash unchanged and changing any value changes it (mutation-proven).
- **Inputs already hashed elsewhere (reuse, do not rehash):** adapter weights `adapter_sha256` (`adapter_card.json`) and `model_identity` (item_checkpoint.rs:148-156 hashes weights and `adapter_manifest.json`); slice pins from the report; decode settings from the checkpoint header inputs (item_checkpoint.rs:48-56, eval_local.rs:298-318).
- **State derivation (pure, in `vox-eval`):**
  `gate_state(receipt, current) -> {Passed | Failed | NeedsReGate(reason) | NotRun}`:
  1. no receipt, or no row for this spoke's suite -> `NotRun`;
  2. receipt schema < v2, or `policy_blake3`/`inputs.*` missing, or any hash differs from `current` -> `NeedsReGate(<which hash>)`, regardless of pass/fail;
  3. current and a blocking row failed -> `Failed`;
  4. current, all blocking rows passed, at least 2 substantive rows (the existing rule, hub_sync.rs:187-191), suite control recorded as failing -> `Passed`.
  Rule 2 is what makes D-06 automatic: the 1.7B/4B/8B receipts have no hashes, so they render `needs re-gate` with no special-casing.
- **Per-spoke contract file** `spoke_gates.json` (schema committed at `contracts/mens/spoke-gate-state.v1.schema.json`, validated in a test with `vox-jsonschema-util`; consumers need no crate edge, they parse JSON):
```json
{ "schema": "vox_mens_spoke_gates.v1",
  "adapter": {"size": "1.7B", "base_pin": "...", "adapter_sha256": "...", "spokes": ["vox-lang"], "spoke_basis": "training_lanes"},
  "gates": [
    {"spoke": "ladder",   "suite": "reasoning-collateral", "policy": "mens/config/eval-gates-ladder.yaml",
     "state": "needs_re_gate", "reason": "policy_blake3 absent", "receipt": "gate_receipt.json"},
    {"spoke": "vox-lang", "suite": "humaneval-vox-heldout", "policy": "mens/config/eval-gates.yaml",
     "state": "not_run", "reason": "no result file"}
  ]}
```
  States are exactly `passed | failed | needs_re_gate | not_run`. 339 renders them; 340's selector must treat only `passed` as selectable (CONTEXT/ROADMAP: "Ungated or stale sizes are never auto-picked").
- **`HfSyncManifest.gate_status`** is a free string compared by `manifests_match`; keep `"passed"`/`"ungated (...)` for compatibility and add `spoke_gates_blake3` so a state change re-uploads.

## Behavioral spoke suites (TOP PRIORITY)

### What exists (all verified this session)
| Piece | Location | What it does | Gap for 338 |
|---|---|---|---|
| Live tool registry | `contracts/mcp/tool-registry.canonical.yaml` -> `vox_mcp_registry::TOOL_REGISTRY` (compile-time, `build.rs`); generated doc lists 376 tools | name, description, `product_lane`, `exposure`, `side_effect_class`, `http_read_role_eligible`, `requires` | **No input schemas** in the registry |
| Live tool schemas | `vox-orchestrator-mcp/src/input_schemas.rs:44` `tool_input_schema(name)` is `pub(super)` (crate-private); public path is `registry::tool_registry() -> Vec<rmcp::model::Tool>` (registry.rs:55-97) and `tool_search` JSON (`"input_schema"`, tool_search.rs:158) | draft-07 schemas, schemars-derived plus hand `parse_obj` | Only linkable from crates that depend on `vox-orchestrator-mcp` |
| Real turn tool selection | `llm_bridge::tool_selection::select_tools_for_turn` (`DEFAULT_MAX_TOOLS = 40`, pins, lane filter) | what the harness actually offers a model per turn | same |
| Real agent loop | `chat_tools/chat/agent_loop.rs::run_agent_turn` (pub(crate)); `DEFAULT_MAX_ITERATIONS = 8` (:172); text salvage of `{"name","arguments"}` (:636-742); hermetic test with `ServerState::hermetic_stub` + wiremock (:1195-1300, feature `eval-gate`) | multi-turn loop with tool results appended as `role:"tool"` | dispatches to real handlers; no mock executor; salvage validates tool *name* only, never args vs schema |
| Local serve, tool-aware chat | `vox mens serve` `POST /v1/chat/completions` (serve/handlers.rs:322-433) | flattens tools to `- name: description` only (**parameter schemas are not shown to the model**, :266-276), validates reply shape `{"name":string,"arguments":object}` only, `max_retries: 3` | schema invisible to local models; retries inflate validity |
| Hermetic harness eval | `vox harness eval` (vox-cli harness/eval.rs): pass^k golden tasks of the **harness code**, no model | not a model eval | — |
| Live harness eval | `vox harness eval --live` (harness/live_eval.rs): real hosted model through `chat_message`, end-state checks on the real filesystem, spend cap | outcome-based precedent; hosted models only | not local adapters |
| Single-turn spoke verifiers | `vox-corpus/training/spoke_roundtrip.rs` | JSON shape + known name + required arg keys; rust = `syn` + policy strings | not multi-turn, no schema validation, no compile/test |
| Corpus-derived agent metrics | `eval_agentic_metrics.rs` | see inventory: scores the training file | — |
| vox-lang compile+hidden test | `vox_corpus::humaneval_runner::verify_program` (`vox check` then `vox run --mode interp`, fixture `tests.vox` assertions, `is_oracle_neutralized` canary); driven by `vox model eval-corpus` (vox-cli, 2448 lines), generation **through `vox_actor_runtime::llm`**, local adapters served by `vox mens serve` (:11435) with `check_for_eval` adapter-card fingerprint check and a served-model-mismatch abort (eval_corpus.rs:1-45, 272-290, 332-350) | held-out fixtures (`training_eligible = false`; README: 30 of 164, manifest says 31 in `heldout_bench` text) | needs a gate wrapper + floor + control; 164 fixtures, `tests.vox` real assertions |
| vox-lang heldout (eval-local) | `eval_local.rs` + `metrics.rs::verify_completion` (metrics.rs:407-506): in-process parse+typecheck (`run_frontend_str`), anti-stub, and `run_test_assertions(&res.hir)` over the **model's own** `@test` blocks (`None` when it wrote none -> treated as pass, metrics.rs:122-139, 466) | 52 tasks; gate keys `pass_at_k`, `anti_stub` | passes without any hidden test. This is the existing ladder "pass@1" |
| Rust compile only | `vox_corpus::corpus::rust_authoring::{compile_batch_in_workspace, clippy_batch_in_workspace}` (`cargo check` JSON in a tmp crate) | batch compile of snippets in `mod snippet_i` | no tests run, no held-out rust items, no rust adapter or data on disk |
| JSON-Schema validator | `jsonschema 0.46` via `vox-jsonschema-util` (`compile_validator`, `validate`) | ready | — |
| Local inference | `reasoning_eval::with_backend` -> `vox_plugin_host::cached_code_plugin(...).as_ml_backend()` -> `load_model` / `run_inference(prompt_json)`; batch mode `{"batch": [...]}` + `{"batch_next":true}` (reasoning_eval.rs:199-243, 295-345) | candle QLoRA plugin (Metal/CUDA), **not** `vox_actor_runtime::llm` | see Open Question 1 |
| Phases 321/322 | 321 = `vox mens serve` size routing + before/after; 322 = turn-decision pack and dynamic action catalog in `vox-orchestrator-mcp` (selective prediction over routing questions) | neither is an episode runner or mock executor | — |

### Design: six behaviors on those pieces

**Episode fixture** (`mens/data/agent_episodes/<suite>.jsonl`, one JSON object per line, hashed as a whole by BLAKE3 and recorded in the receipt):
`{id, behavior, user_request, offered_tools[names], world{...scripted state...}, script{tool -> [{when: <arg predicate>, result|error}]}, expect{...outcome predicate...}, max_turns}`. Offered-tool names are validated at load against the live catalog (a fixture naming a tool that does not exist fails the load, never silently skips).

| Behavior | Episode shape | Outcome predicate (no reference-text matching) |
|---|---|---|
| right tool | request needs exactly one of 4-12 offered tools including same-lane hard negatives; request text **must not contain the tool name or its description wording** (Pitfall 7) | first call's tool == `expect.tool` |
| schema-valid args | same, args required by the live schema | `jsonschema::validate(call.arguments, live_schema)` succeeds **and** the mock world accepts the call (world-level constraint, e.g. referenced id exists) |
| consumes results | call 1 returns a per-episode nonce/id (random per seed, so it cannot be guessed); call 2 must carry it | call 2's argument at JSON-pointer P equals the nonce from call 1's result |
| recovers from tool error | call 1 (any valid call) returns a scripted error carrying a repairable cause (missing field, wrong enum) | a later call succeeds in the mock world (state mutation recorded) within the turn budget |
| stops when done | goal reachable in k calls; the world marks any call after the goal state as `redundant` | goal state reached **and** zero redundant calls **and** a terminal non-call assistant turn before `max_turns` |
| refuses out-of-scope | request needs a tool that is not offered, or a `side_effect_class` forbidden by the episode's `permission_mode` | zero calls to any tool outside the offered/allowed set, world state unchanged, terminal assistant turn is non-empty and contains no tool-call shape |

Limitation to state in the suite doc: "refuses" is detected as "did nothing out of scope and said something", not as a correct explanation.

**Runner shape.** `Policy = FnMut(&[ChatMessage], &[ToolDef]) -> ModelTurn` (a text reply). The runner: build the prompt (tool block with names, descriptions and the compact live parameter schema; ChatML as `flatten_chat_messages` does, handlers.rs:254-305, so the eval prompt is the serve prompt), call the policy, parse a call with the salvage shape, validate args, execute against the mock world, append a `tool` turn, repeat to `max_turns` or a terminal turn. Decode: non-thinking, greedy, `max_tokens` small and recorded, `max_retries` 0. All of that goes in `decode_blake3`.

**Controls (D-10) — the discrimination proof.** Two layers:
1. *Model-free scripted policies, in unit tests (seconds, no GPU, no weights).* A suite is admitted only if a matrix test asserts, per behavior: `oracle` (follows the fixture's reference trajectory) scores 100%; and a named control scores below the floor:
   `never_call` (fails right-tool, args, results, recovery; passes refuse), `refuse_everything` (passes refuse; fails the rest — proves the refuse items are not a free pass), `shuffled_tool` (calls the permuted tool name with the correct args — fails right-tool, and fails args because the schema belongs to another tool), `first_offered_tool`, `random_seeded`, `ignore_results` (replays call 1's args), `no_recovery` (repeats the failing call), `call_forever` (never emits a terminal turn; fails stop), `extra_call_after_goal`. Each assertion is mutation-proven (e.g. make `redundant` always false; the `extra_call_after_goal` test must then fail). This is the "known-bad control" for the *scorer*; it needs no model and runs in CI.
2. *Model-based controls, one run at 0.6B.* Base `Qwen3-0.6B` (same decode) must fall below the gate floor; plus a **shuffled-catalog** control (names and descriptions permuted in the prompt, free) which must fail right-tool. A trained shuffled-tool *adapter* is the roadmap's example but costs a training run; list it as optional. A suite whose model control passes is recorded `non_discriminating` and cannot gate (D-10), and `gate_state` returns `NotRun` for it with that reason.
3. *Pass criterion for an adapter:* success rate >= a floor stated in the gate YAML **and** exact one-sided McNemar improvement over the base model on paired episodes at `alpha` (`mcnemar_one_sided_p(losses, gains)`, direction reversed from collateral). Choose n so the test has stated power (reuse `power_json` style table).

**Hosting and crate edges (D-14).**
- Pure types/scorer/mock world/controls: `vox-eval` (no new deps). Schema validator injected as `&dyn Fn(&Value, &Value) -> Result<(), String>`, supplied by `vox-ml-cli` from `vox-jsonschema-util` (edge exists).
- Tool catalog: `vox-ml-cli` has **no** edge to `vox-orchestrator-mcp` or `vox-mcp-registry` (edges list verified). Two options, recommend A:
  - **A (no exception):** add `vox harness tool-catalog --out <file>` to `vox-cli` (which already links `vox-orchestrator-mcp` with feature `eval-gate` and `vox-mcp-registry`, vox-cli/Cargo.toml:217,270). It writes `{tools:[{name, description, input_schema, lane, side_effect_class}], selection:{...}}` from `registry::tool_registry()`; the runner reads that file; the receipt records `catalog_blake3`. A `vox-orchestrator-mcp` test regenerates the catalog and compares it with a committed lock file (the repo's generated-and-drift-checked pattern, like `mcp-tools.generated.md`) so staleness fails CI instead of silently passing. `[ASSUMED]` this satisfies D-11 "live registry, not hand-copied": the file is generated, never hand-edited, and drift-tested.
  - **B (needs a USER-AUTHORIZED exception):** `vox-ml-cli -> vox-orchestrator-mcp` (same layer L3). Reads the registry in-process at eval time (literal D-11), at the cost of a 40k-LoC crate in the ML binary. Propose in the PR description and stop; do not write the ledger entry.
- Placement budgets: `layers.toml` gives `vox-ml-cli` `max_loc = 20_000` while the tree is 38.9k LoC; do not add more than thin glue there. Run `cargo run -p vox-arch-check` and `vox ci crate-edges` in the plan's verify step.

**vox-lang (D-09).** Gate = `vox_corpus::humaneval_runner::verify_program` pass rate on the held-out HumanEval-Vox fixtures (`training_eligible = false`), run through `vox model eval-corpus` against a served adapter, plus the existing heldout `pass_at_k`/`anti_stub` rows kept as secondary. Reasons: hidden `tests.vox` assertions (the current `eval-local` pass can be earned without any test), canary against oracle-neutralising output, facade-routed generation, adapter-card fingerprint guard. Needs: a floor in `eval-gates.yaml` (new key, e.g. `humaneval_vox_pass_rate`), a result file the gate reads (`eval-corpus --output` writes a report), a control (base model, same condition `C1`) and the receipt hashes of the fixture set. The fixture-count facts differ between README (30 of 164) and the manifest note (31); count from `held_out(&load_corpus(..))` in a test, never from prose.

**rust (D-09).** Nothing to reuse for held-out items or tests, and no rust adapter or data exist (`mens/data/mix_sources/` absent; `mix-rust.yaml` sources missing). Build `mens/data/rust_bench/` fixtures (`spec.toml`-style like humaneval-vox: prompt, hidden `tests.rs`) and run `cargo test` in a per-run temp crate through the same tmpdir pattern as `rust_authoring.rs:75-141` (file-redirected output, timeout, no network, `CARGO_TARGET_DIR` shared read-only cache is a Wave-0 question). Replace the paper `rust_compile_rate`/`clippy_clean_rate` keys with `rust_test_pass_rate`. Because no adapter can run it, the plan's proof is oracle + scripted controls + base model, and every rust cell is `not_run` until a rust adapter exists. Scope risk: this is a new benchmark; consider shipping rust as "suite + controls proven, no gate yet" and say so on cards.

### Eval cost (D-16)

Anchors (all `[ASSUMED]` unless noted): measured 0.6B batch decode, Metal f32, cap 512 tokens, loaded host: 26.60 s/item at batch 1 and 10.75 s/item at batch 16, byte-identical outputs [VERIFIED: 335-RESEARCH.md §7]. Reasoning-compare checkpoint mtimes give batch-1, cap-2048 greedy wall-clock per item: 1.7B adapter 136-187 s, 4B base ~530 s, 8B base 526-718 s, 14B base ~1021 s (11 items) [derived from `mens/runs/reasoning_compare/checkpoints/*/` file mtimes this session; includes model load and host contention; upper-ish]. Sizes 8B+ were measured on a shared, loaded machine.

Episode cost model (3 turns, ~2k-token prompt per turn, ~80 generated tokens per turn, non-thinking): 128 episodes.
| Size | per-episode (batch 1) | 128 episodes, batch 1 | 128 episodes, batch ~8-16 (337) | Notes |
|---|---|---|---|---|
| 0.6B | ~25 s | ~55 min | ~20-25 min | anchor measured |
| 1.7B | ~40-60 s | ~1.5-2 h | ~40-60 min | |
| 4B | ~150-250 s | ~5-9 h | ~2-4 h | overnight only after 337 |
| 8B | ~300-500 s | ~11-18 h | ~5-8 h | not in this phase |
| 14B+ | — | — | — | not in this phase |
Base-side runs are reused across adapters of the same snapshot via the checkpoint key; the control costs one base run per size, once. vox-lang HumanEval-Vox held-out (~30 fixtures x direct generation <= 4096 tokens) is roughly the 0.6B/1.7B/4B heldout-bench cost (52 items) or less. **Plan rule:** prove everything at 0.6B (<= ~1.5 h total incl. controls), state 4B/8B as "needs re-gate until run" (D-06). Scripted-control and scorer tests cost no GPU.

## Overlap with phase 335 / 337 (D-15)

Files 335 is touching or has just changed, which any 338 task must re-read at execution time: `crates/vox-eval/src/reasoning_slice.rs` (D-18 function composes it), `reasoning_bench.rs`, `qwen_reasoning.rs` (`GRADER_ID`), `crates/vox-ml-cli/src/commands/mens/{eval_collateral.rs, eval_gate/{policy,check_run,run_gate}.rs, item_checkpoint.rs, reasoning_eval.rs, reasoning_compare.rs, eval_local.rs}`, `mens/config/eval-gates-ladder.yaml`, and (335 task 6) the sync run itself through `hub_sync.rs`. Planned 338 changes by overlap:
| 338 change | Files | Overlap |
|---|---|---|
| D-18 shared validity fn | new `vox-eval/collateral_evidence.rs`; edits to `hub_sync.rs`, `hub_card.rs`, `dispatch.rs`, `check_run.rs` | `check_run.rs`, `dispatch.rs`, `reasoning_slice.rs` shared with 335: sequence after |
| Receipt v2 + canonical hash | `run_gate.rs`, `policy.rs` | shared: after 335 |
| Spoke router, `domain-profiles` use | `eval_gate/` new file, `populi/dispatch.rs`, `train_arm.rs:941` | `dispatch.rs` shared |
| Episode runner | new `vox-eval` files, new `vox-ml-cli` glue file, `mod.rs`, `mens_tail_subcommands.rs` (CLI enum) | only `mod.rs`/CLI enum conflicts; low |
| Dataset repos | new `hub_dataset.rs`, `hub_repo.rs`, `hub_sync.rs` | `hub_sync.rs` shared with 335 sync |
337 (eval throughput: batched decode, base-generation reuse, early-stop McNemar) lands before 338 per the roadmap and changes the decode/checkpoint inputs the receipt must hash. Treat `decode_blake3` as a function of whatever 337 leaves in the checkpoint header, not of today's `reasoning_checkpoint` inputs.

## Common Pitfalls

### Pitfall 1: Fixing publish but not the card
**What goes wrong:** `verify_publish_gates` refuses, yet `card_status` still renders `passed` and the index says so (it does today for 1.7B/4B/8B).
**How to avoid:** one function, four callers, structural test that `Some("pass")` appears nowhere else.
**Warning signs:** `mens/runs/hub_cards/**/README.md` still shows `gate-passed` after Wave 1 for a size whose report lacks pins.

### Pitfall 2: The mutation-proof fixture is itself stale
**What goes wrong:** the happy-path test keeps `{"status":"pass"}` (hub_sync.rs:1383) so the refusal tests pass and the happy path can only pass if the guard is off.
**How to avoid:** generate fixtures from the real writers (`reasoning_slices_json`, `slice_blake3`) in a test helper; mutation: comment out each guard, exactly one test must fail.

### Pitfall 3: Non-canonical policy hash
`preserve_order` is on workspace-wide (workspace-hack:153), so `hash_json` is order-sensitive. Sort keys recursively; include code-side rule constants; test with reordered YAML.

### Pitfall 4: Hashing the receipt's own timestamp/commit into "policy"
`timestamp_unix` and `source_commit` must stay outside `policy_blake3` or every re-run is "needs re-gate". `source_commit` is `env!("VOX_GIT_HASH")` (run_gate.rs:74): meaningful only from a clean tree; do not make state depend on it.

### Pitfall 5: Serve route hides schemas and retries
Gating through `vox mens serve` chat with `tools` measures name:description only and up to 3 server-side retries (handlers.rs:266-276, 364). Either gate in-process with a prompt that includes compact parameter schemas, or fix `flatten_chat_messages` test-first, and always record and pin `max_retries`.

### Pitfall 6: Salvage leniency
The harness accepts a call anywhere in free text (`bare_json_candidate`, agent_loop.rs:679-694) and never checks args against the schema. The suite must score what the model *emitted*, with schema validation, and count salvaged-vs-clean separately (the harness already tags `tool_call_salvaged`). Decide up front whether a salvaged call counts as right-tool; recommended: yes for right-tool, no for "clean structured" metric, report both.

### Pitfall 7: Eval items leak through the training generator
`tool_selection_synth.rs` builds tasks as `"Use {tool} to {description}"` / `"I need to {description} (use {tool})"` from a hand-copied 'TOOL_CATALOG' (lines 156-162). Fixtures phrased the same way would be answered by lexical match. Author episode requests independently, forbid the tool name and description n-grams in `user_request` (a load-time check), and extend `check_leakage_precondition` to the episode set.

### Pitfall 8: Pulling a stale published bundle now deletes it
See D-18 consequence 5.

### Pitfall 9: `needs re-gate` flapping on non-semantic changes
Any edit to `eval-gates-ladder.yaml` (even a comment) must not change `policy_blake3`; hash the parsed tree, not file bytes. A reformat of a `.vox` fixture does change `episode_set_blake3` by design.

### Pitfall 10: Counting a control that never ran
A control result file missing must yield `NotRun`, not `Passed`. Mirror the repo's "absent evidence is not evidence" stance (`assert_serve_preconditions`, dispatch.rs:966-999).

## Code Examples

### Canonical policy hash (shape)
```rust
// Source: pattern from vox-crypto usage in item_checkpoint.rs:139-141 + sorting for preserve_order
fn canonical(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(m) => serde_json::Value::Object(
            m.iter().collect::<std::collections::BTreeMap<_, _>>()
             .into_iter().map(|(k, v)| (k.clone(), canonical(v))).collect()),
        serde_json::Value::Array(a) => serde_json::Value::Array(a.iter().map(canonical).collect()),
        other => other.clone(),
    }
}
// NOTE: with preserve_order, collecting into a serde_json::Map keeps insertion order;
// the BTreeMap above fixes the order, and the output must be serialized from a
// structure whose iteration order is the sorted one (build the final string by hand
// from the BTreeMap tree, or assert in a test that two key orders hash equal).
```
The note is load-bearing: the test "two orderings of the same YAML hash equal" is the only proof; do not trust the snippet.

### Gate-state truth table (test table)
| receipt | policy hash | inputs | blocking row | control | state |
|---|---|---|---|---|---|
| none | — | — | — | — | not_run |
| v1 (no hashes) | — | — | pass | — | needs_re_gate |
| v2 | differs | — | pass | — | needs_re_gate |
| v2 | same | differs (slice/grader/catalog) | pass | — | needs_re_gate |
| v2 | same | same | fail | — | failed |
| v2 | same | same | pass | control passed | not_run (non_discriminating) |
| v2 | same | same | pass | control failed | passed |

## State of the Art

| Old approach | Current approach | Changed | Impact |
|---|---|---|---|
| 5%-relative drop, n=32 | exact one-sided McNemar, 128/slice, alpha 0.05 | 7ccb4adaa (Nov 5) | every 1.7B/4B/8B pass is stale |
| grader v1 (answer region only) | `grade_final_answer v2` | 88ff9cebf | verdicts can change; report names grader since f51c569c0 |
| receipt without provenance | `source_commit` in receipt | 2e9f45caf | still no policy/input hashes |

**Deprecated/outdated:** the `--five-spokes` simulated scorecard; corpus-derived agent/rust gate keys.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | HF `validate-yaml` accepts a custom top-level `vox:` key without error/warning | Spoke identity | Card metadata approach changes to tag + `spoke_gates.json` only (cheap fallback) |
| A2 | A generated, drift-tested catalog file satisfies D-11 "live registry at eval time" | Hosting/edges | If the user means literal in-process read, option B (edge exception) is required |
| A3 | Per-size episode wall-clock estimates (0.6B ~25 s/episode; larger sizes scaled from reasoning-compare mtimes) | Eval cost | Budget decisions for 4B/8B; re-measure in the 0.6B proof run |
| A4 | MATH-500 items cannot be assumed redistributable (card declares no licence) so ids + hashes only | Dataset repos | If user accepts MIT via upstream, items can be inlined |
| A5 | Counting a salvaged tool call as "right tool" but not "clean structured" | Pitfall 6 | Changes suite floors |
| A6 | Non-thinking greedy decode with small `max_tokens` for episodes | Runner shape | Agent lane in serve leaves the assistant turn open (thinks, handlers.rs:299-303); results differ |
| A7 | Hub dataset repos default private | Dataset repos | Public release is a separate checkpoint |
| A8 | `humaneval-vox` held-out count (30 vs 31) | vox-lang | Floor/power computed on the wrong n; count in a test |
| A9 | Ladder adapters `spoke = vox-lang` from lane inference is acceptable to the user in place of CONTEXT's "mixed" | Spoke identity | Card metadata value and 340 selector keys |

## Open Questions (RESOLVED)

1. **(RESOLVED: CONTEXT D-20)** **D-14 literal reading vs existing practice.** "All LLM calls through `vox_actor_runtime::llm`." Existing local-adapter evals (`eval-local`, reasoning compare, collateral) call the `MlBackend` plugin directly (reasoning_eval.rs:199-243); only `vox model eval-corpus` goes through the facade (to `vox mens serve`). Recommendation: gate episodes in-process via `with_backend` (identical weights, batchable, checkpointed, same as every other gate here) and add one facade-routed parity test against `vox mens serve` on 5 episodes. Needs user confirmation that in-process counts as compliant for local weights.
2. **(RESOLVED: CONTEXT D-21, option A)** **Catalog sourcing option A vs B** (above). Default A; B needs a ledger entry only the user can authorise.
3. **(RESOLVED: CONTEXT D-22, provisional floors)** **Where does the tool-selection / argument-generation floor come from?** No adapter exists, so there is no trained number to anchor a floor. Recommend floors set from oracle=100% and base-model control + margin, stated in YAML with the power table, and marked provisional until a first trained spoke adapter exists.
4. **(RESOLVED: CONTEXT D-23, runner shows schemas; serve fix deferred)** **Do agent-lane local models get parameter schemas in the harness?** `vox mens serve` drops them (handlers.rs:266-276). If production harness use with local MENS adapters is real, argument-generation cannot be good without a serve fix and matching training-prompt format. Needs a look at the arg-generation training rows (none on disk) before fixing the suite prompt.
5. **(RESOLVED: CONTEXT D-22, small held-out bench now)** **Rust:** build a new benchmark now, or ship "suite + controls proven, gate not yet defined"? (scope/cost)
6. **(RESOLVED as a blocking user checkpoint: CONTEXT D-25, plan 338-09)** **Dataset ids and visibility** (see ONE-WAY DECISION).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain via build broker | all code | ✓ | per `rust-toolchain.toml` | — |
| Metal GPU + `vox-plugin-mens-candle-metal` | model-based controls | ✓ (training/eval ran on this host) | — | scripted controls need no GPU |
| Adapters `mens/runs/qwen3-{0.6b,1.7b,4b,8b}` | receipts/states | ✓ (local, gitignored) | — | — |
| `vox` binary (for `verify_program`) | vox-lang gate | `[ASSUMED]` built under `target/` | — | `--vox-bin` flag exists (eval_corpus.rs) |
| HF token via Clavis (`HuggingFaceToken`) | any push | `[ASSUMED]` configured (ladder already published) | — | no push without it; pushes are human checkpoints anyway |
| Network | validate-yaml probe, dataset push | ✓ for probes | — | offline: skip probe, use tag-only |
| `python3` | scratch analysis only | ✓ (`-I` file scripts; inline `-c` is blocked by lean-ctx) | — | — |

**Missing dependencies with no fallback:** none for planning.

## Validation Architecture

> Config key `workflow.nyquist_validation` not inspected (no change to treat as enabled). Project rule: executors run `cargo check -p <crate>` and the new tests only; hosted CI is the gate (AGENTS.md "Local CI Gate Tiers"). Route cargo through the build broker, background long builds.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` / nextest; `tempfile` fixtures; `wiremock` for HTTP stubs |
| Config file | workspace `Cargo.toml`, `.config/nextest.toml` (profile `ci-gate`) |
| Quick run command | `cargo test -p vox-eval --lib collateral_evidence episode` |
| Full suite command (per touched crate) | `cargo test -p vox-eval --lib`; `cargo test -p vox-populi --features mens-hf-hub --lib hub_`; `cargo test -p vox-ml-cli --features gpu --lib -- eval_gate eval_collateral item_checkpoint episodes` |

### Phase Requirements -> Test Map
| Req | Behavior | Test type | Automated command | File exists? |
|---|---|---|---|---|
| SPOKE-01a | publish refuses report without pins | unit + mutation | `cargo test -p vox-populi --features mens-hf-hub --lib hub_sync::tests::stale_pins` | ❌ Wave 0 |
| SPOKE-01b | refuses wrong rule / wrong alpha | unit + mutation | `... hub_sync::tests::stale_rule_alpha` | ❌ |
| SPOKE-01c | refuses wrong/absent grader | unit + mutation | `... hub_sync::tests::stale_grader` | ❌ |
| SPOKE-01d | card + serve + eval-gate use the same function; no stray `Some("pass")` | unit/structural | `cargo test -p vox-populi --features mens-hf-hub --lib hub_card`; `cargo test -p vox-ml-cli --features gpu --lib collateral_gate_tests` | partial |
| SPOKE-02 | each gate key has producer test or is deleted | unit | `cargo test -p vox-ml-cli --features gpu --lib eval_gate` | partial |
| SPOKE-03 | spoke -> policy path; `include_lanes` inference | unit | `cargo test -p vox-ml-cli --lib spoke_route` | ❌ |
| SPOKE-04 | canonical hash stable under key order, changes under value change; `gate_state` truth table | unit + mutation | `cargo test -p vox-eval --lib gate_state policy_hash` | ❌ |
| SPOKE-05 | spoke in card/receipt/manifest; repo id unchanged | unit + golden | `cargo test -p vox-populi --features mens-hf-hub --lib hub_card hub_repo` | partial (goldens exist: `crates/vox-populi/tests/golden/hub_cards/*.md`) |
| SPOKE-06 | scorer: oracle 100%, each control fails its behavior; mutation per guard | unit | `cargo test -p vox-eval --lib episode` | ❌ |
| SPOKE-06b | live-catalog drift | unit | `cargo test -p vox-orchestrator-mcp --lib tool_catalog_lock` | ❌ |
| SPOKE-06c | catalog names/schemas valid; fixtures name real tools; no tool-name leakage in requests | unit | `cargo test -p vox-ml-cli --lib episodes` | ❌ |
| SPOKE-07 | vox-lang gate reads eval-corpus report; floor; control | unit | `cargo test -p vox-ml-cli --features gpu --lib eval_gate::tests::humaneval` | ❌ |
| SPOKE-08 | dataset id fn + manifest of hashes + licence policy | unit | `cargo test -p vox-populi --features mens-hf-hub --lib hub_dataset hub_repo` | ❌ |
| Proof run | 0.6B base control fails suite; scripted oracle passes | manual-GPU (long) | `vox-ml-cli mens eval-episodes --model Qwen3-0.6B ...` (name TBD) | Wave N, human-approved wall-clock |

### Sampling Rate
- **Per task commit:** `cargo check -p <crate>` + that task's new tests.
- **Per wave merge:** the per-crate full commands above for crates touched.
- **Phase gate:** hosted CI (`gate` job); local: `vox ci crate-edges`, `cargo run -p vox-arch-check` if a crate/feature edge moved; `vox run scripts/fmt.vox` for dirty files.

### Wave 0 Gaps
- [ ] Shared test helper building a *valid* collateral report from `reasoning_slices_json` + pins (replaces `{"status":"pass"}`)
- [ ] `contracts/mens/spoke-gate-state.v1.schema.json` + schema-validating test
- [ ] Probe: HF `validate-yaml` with a `vox:` block (A1)
- [ ] Count held-out HumanEval-Vox fixtures in a test (A8)
- [ ] Decide catalog option A/B (Open Question 2)
- [ ] Episode fixture authoring guide + load-time leakage check

## Security Domain

> `security_enforcement` not set to false in what was read; section included.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes (Hub push) | token only via `vox_secrets::resolve_secret(SecretId::HuggingFaceToken)` (`require_clavis_hf_token`, hub_sync.rs:113-122) |
| V3 Session Management | no | — |
| V4 Access Control | yes | dataset repos private by default; pushes are human checkpoints; episode runner uses a **mock** world, never real tool dispatch |
| V5 Input Validation | yes | `jsonschema` on tool args; strict serde on fixtures; reject path traversal in fixture file refs (use `vox_bounded_fs` caps as `load_policy` does) |
| V6 Cryptography | yes | `vox-crypto` only (`secure_hash`, `hash_fast_hex`); no direct `blake3`/`sha2` outside it; no hand-rolled canonicaliser beyond key sorting |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Stale or edited evidence accepted as a pass | Tampering / Repudiation | recompute from counts (`verify_reasoning_slices`), pins, grader, policy hash; mutation-proven guards |
| Oracle neutralisation (model writes output that defeats the checker) | Tampering | `is_oracle_neutralized` canary for vox-lang; world-state outcomes (not text) for episodes |
| Eval-set contamination via public dataset | Information disclosure | private repos by default; hash-only for held-out; leakage precheck on episode set |
| Secrets in published bundle | Information disclosure | existing `sanitize_json_paths` (hub_sync.rs:238-263) must also run on any new JSON shipped (`spoke_gates.json`) |
| Prompt-injection through tool results in episodes | Tampering | results come only from the fixture's script; fixtures are repo-reviewed, hashed |
| Subprocess abuse in rust suite | Elevation | temp crate, no network, timeouts, output to files (as `verify.rs` does) |

## Project Constraints (from CLAUDE.md / AGENTS.md)
- Test-first; mutation-verify security-relevant guards; verify a mutation held by a before/after presence check.
- No new crate edge without a proposed `exceptions` entry; `exceptions` and baselines are USER-AUTHORIZED-ONLY; never regenerate `crate-edges.allow.v1.json` to admit an edge.
- LLM calls via `vox_actor_runtime::llm`; no vendor hostnames; crypto via `vox-crypto`; secrets via `vox_secrets::resolve_secret`.
- New `pub fn` in `crates/*/src/**` needs an adjacent `#[test]` (TOESTUB `skeleton/untested-pub-api`).
- Automation as `.vox` scripts run with `vox run`; no new `.sh`/`.ps1`/`.py` glue.
- Authored docs under `docs/src/` need frontmatter; research findings go to `docs/src/architecture/` (this file is a GSD planning artifact and is exempt).
- Never `cargo fmt --all`; use `vox run scripts/fmt.vox`. Route every cargo call through the build broker; background long builds; no workspace-wide clippy/test locally.
- Do not push, open PRs or touch the Hub without the user's approval (CONTEXT D-12 also requires a human checkpoint).
- TypeScript-only rule is not applicable (no TS in this phase).

## ONE-WAY DECISION

**Model repo ids: nothing here forces a rename.** `repo_id_for_size` has no spoke input and the new spoke field is additive everywhere (adapter card, receipt, sync manifest, card tag). D-01 stands.

**What is one-way and the CONTEXT defaults cannot settle:**
1. **Dataset repo ids** (proposed `<ns>/vox-mens-eval`, `<ns>/vox-mens-data`) and **visibility**. Receipts will embed `dataset_repo@revision`; ids become part of every later receipt, and making held-out items public cannot be undone (crawlers). Needs the user's explicit choice before the first dataset push; planning and code can proceed with the ids as constants behind one function.
2. **Future single-spoke model repos** (already deferred): one repo per size holds one adapter bundle (`serve` expects top-level files and `v<N>` tags), so a tool-selection 4B adapter cannot live in `vox-mens-qwen3-4b`. The scheme (suffix vs separate namespace/org) must be raised before the first such repo; not needed in this phase.
3. Not one-way but needs the user: the `spoke` value for today's ladder adapters is `vox-lang` by training lanes, not "mixed" (A9). Card metadata is re-rendered on every sync, so this is reversible.

## Sources

### Primary (HIGH confidence, files read this session)
- `mens/config/{eval-gates,eval-gates-ladder,eval-gates-rust,eval-gates-agents,eval-gates-bfcl,domain-profiles,mix-vox-lang,mix-tool-selection}.yaml`
- `crates/vox-ml-cli/src/commands/mens/eval_gate/{policy,check_run,run_gate}.rs`, `eval_collateral.rs`, `eval_local.rs`, `reasoning_eval.rs`, `item_checkpoint.rs`, `metrics.rs`, `populi/{dispatch,train_arm,mens_tail_subcommands}.rs`, `ai/serve/handlers.rs`, `corpus/stats.rs`
- `crates/vox-populi/src/mens/{hub_sync,hub_card,hub_repo,hub,mod}.rs`, `tensor/{domain_profiles,spoke_validate,adapter_card}.rs`
- `crates/vox-eval/src/{reasoning_slice,reasoning_bench,qwen_reasoning}.rs`, `crates/vox-eval/Cargo.toml`
- `crates/vox-corpus/src/corpus/{eval_agentic_metrics,eval_rust_metrics,tool_selection_synth,rust_authoring}.rs`, `humaneval_runner/{mod,verify,compose}.rs`, `training/spoke_roundtrip.rs`, `flywheel.rs`
- `crates/vox-orchestrator-mcp/src/{input_schemas,registry,tool_search}.rs`, `llm_bridge/tool_selection.rs`, `chat_tools/chat/agent_loop.rs`; `crates/vox-cli/src/commands/{harness/{eval,live_eval}.rs,model/eval_corpus.rs}`; `crates/vox-jsonschema-util/src/lib.rs`
- `contracts/ci/crate-edges.allow.v1.json`, `docs/src/architecture/layers.toml`, `contracts/mcp/tool-registry.canonical.yaml`, `contracts/eval/humaneval-vox/README.md`, `crates/workspace-hack/Cargo.toml`
- `mens/runs/qwen3-*/{collateral_damage_report,gate_receipt,adapter_card,training_manifest,telemetry}.json*`, `mens/runs/hub_cards/**`, `target/dogfood/train_mixed.jsonl`, `mens/data/reasoning-slices-provenance-2026-11.json`
- `.planning/phases/{335 RESEARCH+PLAN, 336-01-SUMMARY, 321-RESEARCH, 322-EVAL-REPORT, 324-01-SUMMARY, 338-CONTEXT}`, `ROADMAP.md` phases 335-341, `STATE.md`
- `hf-hub-1.0.0` source in `~/.cargo/registry`

### Secondary (MEDIUM, fetched official pages)
- https://huggingface.co/datasets/openai/gsm8k (MIT), https://huggingface.co/datasets/HuggingFaceH4/MATH-500 (no licence declared), https://github.com/openai/prm800k (MIT repo), https://huggingface.co/datasets/EleutherAI/hendrycks_math (MIT)

### Tertiary (LOW)
- Per-size wall-clock figures (checkpoint mtimes; one measured 0.6B row from 335-RESEARCH §7).

## Metadata

**Confidence breakdown:**
- Wired-vs-paper inventory: HIGH (readers and producers opened; receipts and manifests inspected)
- D-18 plan: HIGH (exact lines, callers, fixtures, on-disk reports)
- Episode design: MEDIUM (pieces verified, composition new; catalog sourcing needs user pick)
- Dataset licensing: MEDIUM (cards fetched; legal reading assumed)
- Wall-clock: LOW

**Research date:** 2026-11-05
**Valid until:** 2026-11-12 (335 and 337 are landing; re-read `crates/vox-eval` and `eval_*` before planning each task)
