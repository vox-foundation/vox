---
phase: 91-measurement-harness-advisory-closure
reviewed: 2026-10-28T01:01:18Z
depth: deep
files_reviewed: 24
files_reviewed_list:
  - crates/vox-eval/src/paired_ni.rs
  - crates/vox-eval/src/lib.rs
  - crates/vox-cli/src/commands/model/mod.rs
  - crates/vox-cli/src/commands/model/eval_diagram/mod.rs
  - crates/vox-cli/src/commands/model/eval_diagram/live.rs
  - crates/vox-cli/src/commands/model/eval_diagram/live_tests.rs
  - crates/vox-cli/src/commands/model/eval_diagram/oracle.rs
  - crates/vox-cli/src/commands/model/eval_diagram/prereg.rs
  - crates/vox-cli/src/commands/model/eval_diagram/prompt.rs
  - crates/vox-cli/src/commands/model/eval_diagram/render.rs
  - crates/vox-cli/src/commands/model/eval_diagram/run.rs
  - crates/vox-cli/src/commands/model/eval_diagram/run_tests.rs
  - crates/vox-cli/src/commands/model/eval_diagram/score.rs
  - crates/vox-cli/src/commands/model/eval_diagram/stats.rs
  - crates/vox-cli/src/commands/model/eval_diagram/tally.rs
  - crates/vox-cli/tests/it/eval_diagram_frozen.rs
  - crates/vox-cli/tests/it/eval_diagram_live.rs
  - crates/vox-cli/tests/it/eval_diagram_rescore.rs
  - crates/vox-cli/tests/it/eval_diagram_tasks.rs
  - docs-astro/tests/lib/mermaid-batch.mjs
  - docs-astro/tests/lib/mermaid-oracle.mjs
  - docs-astro/tests/unit/diagram-authoring-rescore.test.mjs
  - docs/src/architecture/voxdiagram-authoring-reliability-findings-2026.md
  - crates/vox-diagram/src/layout/finish.rs (+ svg/paths.rs, layout/input.rs, layout/mod.rs; commit e828285b5)
findings:
  critical: 1
  warning: 9
  info: 6
  total: 16
status: issues_found
---

# Phase 91: Code Review Report

## Fix status (2026-10-28, gsd-code-fixer)

Scope: the critical finding and all warnings. The recorded run, `probe-spend.jsonl`, the frozen inputs and the report's pre-registration block are unchanged. Every harness fix applies to later runs only.

| Finding | Status | Commit |
|---|---|---|
| CR-01 | Fixed. A failed call that may have been billed (anything but a 4xx or rate-limit refusal) is charged the largest per-call cost seen in the run. A billable failure before any call has been priced stops the run as `cost_unknown`. An attempt cut off by the cap mid-retry is still recorded. README and report wording corrected. | `9dda9a50d`, docs `a7e281e14` |
| WR-01 | Fixed in the CR-01 commit. The largest per-call cost seen is reserved before each call; the spend-cap test now asserts `spent <= cap` (5 calls, $0.05 under a $0.055 cap). Residual: a run's first call, before anything is priced, is admitted on headroom alone (`ponytail:` note on `Budget::reserve`). | `9dda9a50d` |
| WR-02 | Fixed. Probe costs must be finite and non-negative. An unpriced or billable failed probe is ledgered, then probing stops with HUMAN ACTION REQUIRED; a provider refusal still moves on. `prior_spend` refuses negative or non-finite costs. | `833ee04c0` |
| WR-03 | Fixed. `check_run_name`: `[A-Za-z0-9._-]`, not starting with `.`, checked in `run_dataset` and the live preconditions before any call. | `60a6cbd60` |
| WR-04 | Fixed. `refuse_override` accepts only `SelectionReason::Scored`. | `9e4404abc` |
| WR-05 | Report only (rule 6 is pre-registered). The limitation bullet gives the g056 sensitivity: kept, B first pass is 0.0 [-3.6, +3.6], McNemar 1.0, same verdict (checked with the independent `newcombe.mjs`). It also recommends per-endpoint exclusion for any future pre-registration. | `a7e281e14` |
| WR-06 | Handled separately (coordinator: finish-reason recording via `vox-llm-egress` / `vox-actor-runtime`). Only the report limitation for the recorded run was added here. The `Attempt` struct is untouched. | `a7e281e14` (report only) |
| WR-07 | Fixed. `redact_error` persists only the facade's error class and HTTP status (else "unclassified error") in `attempts.jsonl` and `probe-spend.jsonl`. Tests feed an `sk-proj-...` body and assert that it never reaches either file. | `3908b7f9b` |
| WR-08 | Fixed. `Backend::backoff` waits 2, 4 and 8 s before retries 1-3; the stub records the schedule. | `cd46ba65f` |
| WR-09 | Report only: a limitation bullet on the exporter-derived fidelity bias. No exporter-independent comparison was run. | `a7e281e14` |
| IN-04 | Report limitation added (the other Info findings were optional and are not addressed). | `a7e281e14` |
| IN-01, IN-02, IN-03, IN-05, IN-06 | Not addressed (optional). | — |

Verification:
- Red: all 11 new or changed tests failed against guard-free stubs and the old code, each for the expected reason.
- Mutation: removing the backoff call, the mid-retry record, or the refusal classification each failed its test.
- Green: `cargo test -p vox-cli --lib eval_diagram` passed 57/57 at each commit. `cargo test -p vox-cli --test it eval_diagram` passed 11, with 1 ignored (the live run). This includes `diagram_authoring_report_matches_run` and both re-score tests.
- The Node re-score passed 8/8.
- Everything ran in the main checkout (`workflow.use_worktrees: false`).

**Reviewed:** 2026-10-28T01:01:18Z
**Depth:** deep
**Files Reviewed:** 24
**Status:** issues_found

Range: `git diff 82bf6b9a0^..HEAD`. Excluded from line-by-line reading as instructed: `contracts/eval/diagram-authoring/runs/**`, `probe-spend.jsonl`, `.planning/**`, `Cargo.lock`. The recording was queried with `jq` only to confirm or refute specific findings (g056, empty replies, retries).

## Summary

The statistics are correct. I re-implemented Newcombe method 10 (Wilson MOVER, uncorrected phi, phi = 0 on a zero marginal) and the exact McNemar test independently in a separate JavaScript script. That script reproduces all six worked vectors in `paired_ni.rs` and all four paired rows of the live run's `tables.md` to 1e-5:

| Table | Newcombe 95% CI | McNemar p |
|---|---|---|
| A first pass (188/8/4/0) | [-0.01655, +0.05923] | 0.3877 |
| B first pass (190/4/5/0) | [-0.03987, +0.02892] | 1.0000 |
| A after repair | [-0.01019, +0.03572] | 0.5000 |
| B after repair | [-0.01436, +0.02791] | 1.0000 |

The pre-registered verdict is mapped correctly: strict `L > -margin` / `U < -margin`, the descriptive flags, `not_determined` on missing turn-0 attempts or on exclusions above 5% of 200 tasks, and confidence pinned to 0.95 by `parse_manifest`.

- **Facade and hostnames.** Every model call goes through `vox_actor_runtime::llm::llm_chat` (`live.rs:113-129`). No vendor hostname appears anywhere in `eval_diagram/`, the tests or the Node scripts.
- **Secrets.** Keys are resolved by the facade through Clavis (`vox_config::resolve_egress` -> `vox_secrets::resolve_secret`) and sent as a bearer header, never in the URL. The harness never touches a key.
- **Freeze.** No harness file changed between the freeze commit `f1ef3bb50` and the run commit `ea0af22a4` (only `eval_diagram_rescore.rs`, a test).

The defects are in **spend-cap enforcement**:

- Transport-failed calls, which can be billed, count $0 and do not stop the run.
- Probe replies without a cost count $0.
- The final call can overshoot the cap.
- A run name containing `/` hides a run's spend from later cap checks.

Smaller defects are in **selection** (a config pin is admitted) and in **what the report says** about the exclusion of g056, the empty replies and the fidelity asymmetry.

Checks run, per crate, in the background with `timeout`:

- `cargo test -p vox-eval --lib paired_ni`: 4/4 passed.
- `cargo test -p vox-cli --lib eval_diagram`: see the note at the end.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: Transport-failed calls are never charged against the spend cap, and they do not stop the run

**File:** `crates/vox-cli/src/commands/model/eval_diagram/run.rs:355-378` (also `live.rs:84-96`)

**Issue:** `call()` adds spend only on `Ok(reply)`. A failed call is handled in one of two ways:

- It is retried immediately, with `a.retries += 1` and nothing charged.
- On the 4th failure it is recorded with `cost_usd: 0.0`, and `Some(true)` is returned. "Cost known" means the run keeps going.

Several errors in `Backend::chat` happen after the provider has already generated, and billed, a completion:

- `EgressError::Decode`: HTTP 200 with a body that does not decode.
- A client timeout after the request was sent.
- A dropped connection mid-body.

The live run hit exactly this case. Task g056, slot B, Mermaid turn 1 ended with `"Failed to parse response JSON: error decoding response body"` after `retries: 3`. That is up to 4 billed calls recorded as $0.

No bound limits this. A provider or gateway that consistently returns undecodable 200s makes every attempt cost up to 4 billed 4096-token calls while `budget.spent` stays flat. The run then completes all 2 x 200 x up-to-3 turns per slot with the cap never tripping. On a premium model that is hundreds of dollars against a "$25 hard cap".

The probe path has the same hole: a probe `Err` is ledgered with `cost_usd: None`, which counts 0.

The README (`contracts/eval/diagram-authoring/README.md:92`, "The $25 cap covers every call, probes included") and the report's "Spend cap" paragraph (findings doc line 69) overstate the guarantee.

**Fix:** Treat a failed call as costing an unknown amount, and handle it the way an unpriced reply is handled. Either stop the run, or charge a conservative reservation. For example:

```rust
// run.rs, in call(): charge a worst case for every failed call that may have reached the provider
Err(e) => {
    budget.spent += worst_case_usd; // max_tokens * output price + prompt estimate, from the slot config
    a.cost_usd += worst_case_usd;   // so prior_spend sees it in later runs
    if a.retries < MAX_TRANSPORT_RETRIES { a.retries += 1; continue; }
    a.transport_error = Some(e);
    return Some(true);
}
```

Alternatively, return `Some(false)` (`cost_unknown` stop) when the failure is not a pre-send error. Do the same in `probe_candidates`. Add a stub test where `chat` always fails after "billing" and assert that the run stops before the cap. Then correct the README and report wording.

## Warnings

### WR-01: The cap is checked before a call with no reservation, so the last call overshoots it (and the tests assert the overshoot)

**File:** `crates/vox-cli/src/commands/model/eval_diagram/run.rs:356`; `run_tests.rs:220-235`

**Issue:** `if budget.spent >= budget.cap { return None; }` admits a call whenever any headroom is left, however small. That call's full cost is then added. `spend_cap_allows_exactly_k_calls_and_counts_prior_spend` sets a cap of 0.055 and asserts `spent_usd == 0.06`, so the overshoot is encoded as expected behaviour.

With `max_tokens = 4096` on an expensive model, one call can exceed the cap by tens of cents. The probe loop (`live.rs:78-82`) has the same shape.

**Fix:** Reserve the worst-case cost of the next call:

```rust
if budget.spent + budget.next_call_ceiling > budget.cap { return None; }
```

Derive `next_call_ceiling` from the slot's `cost_per_1k`/output price and `max_tokens`. Change the test to assert `spent_usd <= cap`.

### WR-02: Probe replies without a cost are charged $0, and probe costs are not validated

**File:** `crates/vox-cli/src/commands/model/eval_diagram/live.rs:83-96`

**Issue:**

- In the run loop, a reply with `cost_usd: None` stops the run (`cost_unknown`), because the cap can no longer hold. In `probe_candidates`, the same reply is ledgered with `cost_usd: None`, which `prior_spend` counts as 0, and probing moves on to the next candidate. Up to all of A's alternatives plus `B_CANDIDATES = 10` models can be probed this way, each with a 4096-token budget (the probe uses `slot_config`).
- `r.cost_usd` is stored unfiltered. `run.rs:362` filters `is_finite() && >= 0.0`, but the probe path does not, so a negative provider-reported cost would *reduce* `prior_spend`.
- The report sentence "A reply without a cost also stops the run" (findings doc line 71) is not true for probes.

**Fix:** Apply the same `filter(|c| c.is_finite() && *c >= 0.0)`. On an unpriced probe reply, either stop probing with a HUMAN ACTION REQUIRED error or ledger a conservative estimate. Make `prior_spend` reject negative costs.

### WR-03: An unvalidated `run_name` can hide a run's spend from later cap checks, or write outside `runs/`

**File:** `crates/vox-cli/src/commands/model/eval_diagram/run.rs:205`; `live.rs:243`; `crates/vox-cli/tests/it/eval_diagram_live.rs:37`

**Issue:** `run_name` comes from `EVAL_DIAGRAM_RUN_NAME` and is joined verbatim, so `runs/<name>/attempts.jsonl` is created with `create_dir_all`:

- A name like `2026-10-28/retry` nests the recording two levels deep. `prior_spend` only reads `runs/<entry>/attempts.jsonl` one level down (`run.rs:91-104`), so that run's spend never counts against the cap again. The Rust and Node re-score walkers miss it too.
- A name like `../x` writes outside `runs/`.

**Fix:** Refuse anything but one normal path component before any call:

```rust
ensure!(
    Path::new(&cfg.run_name).components().count() == 1
        && matches!(Path::new(&cfg.run_name).components().next(), Some(std::path::Component::Normal(_))),
    "run name {:?} must be a single directory name", cfg.run_name
);
```

### WR-04: `refuse_override` admits a config-pinned model (`SelectionReason::PremiumAlias`)

**File:** `crates/vox-cli/src/commands/model/eval_diagram/live.rs:54-61`, `313`

**Issue:** The pre-registered rule says "no model is pinned". Only `SelectionReason::EnvOverride` is refused. `decide()` first tries `select_via_premium_alias` (`vox-orchestrator/src/models/select.rs:853-890`), which returns `PremiumAlias` when `model-defaults.v1.yaml` maps the task's premium key (codegen has one, line 64) to a model whose key is present. Its own `Display` says "Pinned to …". On a machine with that vendor's key, model A would be a config pin and the run would proceed. This run was `Scored`, so it was not affected.

**Fix:** Accept only `SelectionReason::Scored`:

```rust
ensure!(matches!(reason, SelectionReason::Scored), "{HUMAN}: selection {reason} is not a registry score; the eval refuses pins");
```

### WR-05: Exclusions conditioned on repair turns censor the first-pass endpoint, and the report does not say so

**File:** `crates/vox-cli/src/commands/model/eval_diagram/tally.rs:66-67`; findings doc lines 92-93 and 246

**Issue:** A transport error in *any* turn excludes the task from *both* endpoints, including first pass. Repair turns exist only after a first-pass failure, so an exclusion is more likely exactly where the arms disagreed. This is informative censoring of the primary endpoint.

The run shows it. In slot B, g056 was `voxdiagram:0` valid and `mermaid:0` invalid, a "VoxDiagram only" cell. It was dropped because of the transport error on `mermaid:1`. Keeping it gives B a first pass of b=5, c=5: Δ = 0.0, [-3.6, +3.6], instead of the reported -0.5, [-4.0, +2.9]. The verdict is unchanged.

The code follows rule 6 as pre-registered, so this is a design and reporting defect rather than a coding error. The report presents the exclusion as neutral ("one call still failed in transport").

**Fix:** In the report, state that the g056 exclusion came from a repair turn, which cannot affect first-pass validity, and that it removed a VoxDiagram-favouring discordant pair. For any future pre-registration, exclude per endpoint: first pass only on turn-0 transport errors. Optionally add a descriptive sensitivity row to `tables.md`.

### WR-06: Empty replies cannot be told apart from authoring failures; nothing is recorded to classify them

**File:** `crates/vox-cli/src/commands/model/eval_diagram/mod.rs:103-120` (Attempt schema); `live.rs:113-129`; root cause `vox-llm-egress/src/wire.rs:366-369`

**Issue:** The egress reads only `choices[0].message.content` and maps null to `""`. `Reply`/`Attempt` keep no `finish_reason`, refusal or reasoning-token field.

- 17 of 827 attempts are empty. 10 used the full 4096 tokens (`finish_reason=length` while reasoning). Others are short: g128 used 225 completion tokens, which points to content routed to a reasoning field or a filter.
- They decide the primary endpoint's discordant cells: 6 of 8 Mermaid first-pass failures and 2 of 4 VoxDiagram failures for model A.
- The report discloses the count, but the harness gives no way to tell runaway reasoning, a content filter or a gateway quirk apart. All of them are scored as "the model could not write the language".

**Fix:** Record `finish_reason` and reasoning or refusal fields in `Attempt` (`schema_version: 2`), plumbed through `LlmResponse`. Have `tally` report empty replies per arm as their own row, so readers can see the syntax-only comparison. Consider raising `max_tokens` for reasoning models, or excluding them via the registry capabilities.

### WR-07: Untrusted provider error text is committed verbatim, with no redaction

**File:** `crates/vox-cli/src/commands/model/eval_diagram/live.rs:89-91`, `320-321`; `run.rs:374`

**Issue:** Probe errors and transport errors are written to tracked files:

- the error string in `probe-spend.jsonl` and in the run header's `probes`;
- `Attempt.transport_error` in `attempts.jsonl`;
- the `bail!` message.

For HTTP errors that string embeds the provider's response body (`vox-actor-runtime/src/llm/chat.rs:69`, `"LLM API returned error ({code}): {body}"`). Some providers echo part of the key in 401 bodies (for example `Incorrect API key provided: sk-proj-****abcd`), and gateways can echo request headers. The harness itself never handles a key, but nothing stops a provider echo from reaching git. That falls short of "secrets must never be logged".

**Fix:** Persist only the HTTP status and error class, or pass the text through the Clavis redaction helper (`crates/vox-secrets/src/redact.rs`) with the resolved key patterns before writing. Add a test that feeds a fake `sk-…` key in an error body and asserts that it never reaches the ledger.

### WR-08: Transport retries fire immediately, with no backoff

**File:** `crates/vox-cli/src/commands/model/eval_diagram/run.rs:372`

**Issue:** `Err(_) if a.retries < MAX_TRANSPORT_RETRIES => a.retries += 1` loops straight back into `chat`. The facade only waits on a 429 that carries `Retry-After` (`vox-llm-egress/src/throttle.rs`). A brief 5xx or connection blip uses all 3 retries within milliseconds and becomes an infra exclusion, which counts toward the 5% ceiling that makes the primary verdict "not determined". It also multiplies the uncounted billed calls from CR-01.

**Fix:** Sleep with exponential backoff before each retry:

```rust
tokio::time::sleep(Duration::from_secs(2u64.pow(a.retries))).await;
```

### WR-09: The fidelity comparison favours VoxDiagram by construction; the report does not list this

**File:** `crates/vox-cli/src/commands/model/eval_diagram/score.rs:126-138`; `mod.rs:254-278`; findings doc lines 196-198, 213 and 216-230

**Issue:** The expected sets come from the VoxDiagram reference's own Mermaid export. A valid VoxDiagram reply is scored by passing it through the same exporter (`vd_verdict(..).exported`). Whatever the exporter canonicalises is therefore matched automatically by the VoxDiagram arm: the VoxDiagram -> Mermaid shape mapping (`SHAPE[v.type]`), the default direction, and the container-title paths. The Mermaid arm has to guess the exporter's choice wherever the prompt is loose, for example `(x)` versus `([x])` for a "rounded" node.

The report gives fidelity as 96.5% vs 86.9% (A) and 91.5% vs 70.7% (B), but lists no limitation for this asymmetry.

**Fix:** Add a limitation stating that the expected sets are defined by the VoxDiagram exporter. Better: compare fidelity on an exporter-independent subset (labels and edge endpoints only) as well.

## Info

### IN-01: The freeze check does not cover the harness code or the oracles

**File:** `crates/vox-cli/src/commands/model/eval_diagram/live.rs:213-217`

**Issue:** The `git diff` pathspec covers only the dataset directory and the report. A change to `eval_diagram/*.rs`, `mermaid-batch.mjs`/`mermaid-oracle.mjs` or `crates/vox-diagram` after the freeze would go unnoticed. I verified by hand that none changed between `f1ef3bb50` and the run.

**Fix:** Add those paths to the diff pathspec.

### IN-02: A torn last line in any recording blocks every later run

**File:** `crates/vox-cli/src/commands/model/eval_diagram/run.rs:186-194`, `98-99`

**Issue:** `Recorder::push` flushes but never calls `sync_data`. A crash mid-write can leave an unterminated JSON line. `prior_spend` then fails on `read_jsonl` for every future run, unless someone hand-edits a recording the README says must never be edited.

**Fix:** In `prior_spend`, tolerate a single unterminated final line, or charge it conservatively. Call `sync_data` after each line.

### IN-03: The Newcombe test vectors are self-derived

**File:** `crates/vox-eval/src/paired_ni.rs:91-115`

**Issue:** The module says so itself. My independent re-implementation agrees, but the test still does not pin the method against the literature.

**Fix:** Add one published vector from Newcombe (1998) Table II, or from `ratesci::pairbinci`.

### IN-04: Mermaid "valid" accepts any diagram type

**File:** `docs-astro/tests/lib/mermaid-batch.mjs:8-17`

**Issue:** A reply that parses as a `pie` or `classDiagram` counts as first-pass valid, while the VoxDiagram arm is held to its own graph or sequence kinds. This is consistent with the pre-registered rule ("the parser accepts it") and fidelity catches it, but the report could say so.

**Fix:** Mention it in the report's limitations.

### IN-05: Candidate handling for slots A and B has two small gaps

**File:** `crates/vox-cli/src/commands/model/eval_diagram/live.rs:49-51`, `273-281`

**Issue:**

- `resolve_slot` does not de-duplicate `ids`, so a selected model that also appears in `alternatives` is probed (and paid for) twice.
- `vendor_org` takes the text before `/`. A direct-provider id with no slash (for example `claude-…` via Anthropic direct) yields the whole id, so slot B could end up with the same vendor as A.

**Fix:** De-duplicate the ids. Use `spec.provider` or the canonical slug's organisation for the vendor test.

### IN-06: `widen` can stop before a wider wrap that fits (renderer commit e828285b5)

**File:** `crates/vox-diagram/src/layout/finish.rs:104-129`

**Issue:** The loop `break`s at the first wrap limit whose box is wider than the content. The widest line of a greedy wrap does not always grow as the limit grows, so a larger limit that fits with fewer lines can be missed. The effect is cosmetic: it falls back to the old stacked title. The `clamp` is safe because `w <= c.w` is guaranteed.

**Fix:** Use `continue` instead of `break`, or keep the `break` and document the heuristic.

---

**Verification note:**

- `cargo test -p vox-eval --lib paired_ni` passed (4/4).
- The `cargo test -p vox-cli --lib eval_diagram` run was started in the background on the loaded machine; its outcome is reported in the reviewer's final message, not here.
- The independent Newcombe/McNemar cross-check script is at `/private/tmp/claude-501/-Users-brbrainerd-dev-vox-voxdiagram/1578e102-b2d3-4922-b18e-a965ec6b6147/scratchpad/newcombe.mjs`.

_Reviewed: 2026-10-28T01:01:18Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
