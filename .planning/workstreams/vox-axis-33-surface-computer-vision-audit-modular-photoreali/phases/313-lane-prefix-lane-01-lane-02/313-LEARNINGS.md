---
phase: 313
phase_name: "lane-prefix-lane-01-lane-02"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 21
  lessons: 13
  patterns: 9
  surprises: 5
missing_artifacts:
  - "UAT.md"
---

# Phase 313 Learnings: lane-prefix-lane-01-lane-02

## Decisions

### Lane::parse is exact lowercase, no trim or case folding; error text quot...
Lane::parse is exact lowercase, no trim or case folding; error text quotes the value cut to 32 chars

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 313-01-SUMMARY.md

---

### Hub/agent user_for_lane strips /no_think then trim_end, leaves /think; c...
Hub/agent user_for_lane strips /no_think then trim_end, leaves /think; codegen delegates to user_for_mode(user, false)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 313-01-SUMMARY.md

---

### do_chat_completions GenerateRequest literal gets lane
do_chat_completions GenerateRequest literal gets lane: None here; plan 313-03 forwards req.lane

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 313-01-SUMMARY.md

---

### Lane enum plus user_for_lane / assistant_opener_for_lane in vox-eval (11...
`Lane` enum plus `user_for_lane` / `assistant_opener_for_lane` in `vox-eval` (11 tests in `qwen_reasoning`, 6 new).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 313-01-SUMMARY.md

---

### GenerateRequest.lane, InferenceRequest.lane, "lane" in inference_payload...
`GenerateRequest.lane`, `InferenceRequest.lane`, `"lane"` in `inference_payload`; the five existing worker test literals gained `lane: Lane::Codegen`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 313-01-SUMMARY.md

---

### do_generate resolves the lane right after the 409 block
`do_generate` resolves the lane right after the 409 block. Missing lane is codegen; an unknown, mis-cased or empty lane returns a JSON `GenerateResponse` 400 naming `codegen, hub, agent`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 313-01-SUMMARY.md

---

### Tracer test do_generate_hub_lane_reaches_the_worker_payload proves HTTP ...
Tracer test `do_generate_hub_lane_reaches_the_worker_payload` proves HTTP JSON to `inference_payload` end to end. `inference_payload` is now `pub(crate)`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 313-01-SUMMARY.md

---

### Hub/agent do not call chatml_prefix_open_assistant with a fake assistant...
Hub/agent do not call chatml_prefix_open_assistant with a fake assistant string (avoids coupling to has_reasoning_trace); they reuse its template inline

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 313-02-SUMMARY.md

---

### resolve_lane runs right after check_output_mode, so an unknown lane erro...
resolve_lane runs right after check_output_mode, so an unknown lane errors before any token is generated (defense in depth; host already validates)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 313-02-SUMMARY.md

---

### Early <|im_start|> pre-wrapped branch untouched for every lane
Early <|im_start|> pre-wrapped branch untouched for every lane

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 313-02-SUMMARY.md

---

### chatml_prefix_for_lane (#[must_use], documented) in candle-core training...
`chatml_prefix_for_lane` (`#[must_use]`, documented) in candle-core `training_text.rs`; `chatml_prefix_open_assistant`, `fuse_turns` and training paths unchanged (diff vs pre-copy removes only the import line).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 313-02-SUMMARY.md

---

### Both plugins
Both plugins: `#[serde(default)] lane: Option<String>`, private `resolve_lane` (None -> Codegen, else `Lane::parse`, Err text lists codegen, hub, agent), called right after `check_output_mode`; `assemble_prompt` gains `lane: Lane`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 313-02-SUMMARY.md

---

### Three pre-existing assemble_prompt_* tests per plugin keep their literal...
Three pre-existing `assemble_prompt_*` tests per plugin keep their literal expectations; only the call gained `Lane::Codegen` (verified by diff of removed lines).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 313-02-SUMMARY.md

---

### No system prompt added for hub/agent; earlier turns and the tool catalog...
No system prompt added for hub/agent; earlier turns and the tool catalog are untouched

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 313-03-SUMMARY.md

---

### prompt_for_output_mode (tools suffix) left unchanged, as scoped
prompt_for_output_mode (tools suffix) left unchanged, as scoped

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 313-03-SUMMARY.md

---

### do_chat_completions keeps its own lane check even though do_generate re-...
do_chat_completions keeps its own lane check even though do_generate re-validates the forwarded lane: the chat 400 is a ChatCompletionResponse body, not a GenerateResponse mapped through the chat wrapper

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 313-03-SUMMARY.md

---

### ChatCompletionRequest.lane added after tools (doc points at GenerateRequ...
`ChatCompletionRequest.lane` added after `tools` (doc points at `GenerateRequest::lane`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 313-03-SUMMARY.md

---

### flatten_chat_messages takes a final Lane; hub/agent output for hi is <|i...
`flatten_chat_messages` takes a final `Lane`; hub/agent output for `hi` is `<|im_start|>user\nhi<|im_end|>\n<|im_start|>assistant\n`, codegen is byte-identical to before.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 313-03-SUMMARY.md

---

### do_chat_completions parses the lane first (None -> Codegen), returns a J...
`do_chat_completions` parses the lane first (None -> Codegen), returns a JSON `ChatCompletionResponse` 400 (content = parse error, `finish_reason` stop, model = `state.model_name`) and passes the lane to flatten and to `GenerateRequest` (`lane: req.lane.clone()`, one occurrence).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 313-03-SUMMARY.md

---

### Task 1 committed as two commits (vox-eval, then candle-core), each namin...
Task 1 committed as two commits (vox-eval, then candle-core), each naming only its own file

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 313-04-SUMMARY.md

---

### Populi file committed whole
Populi file committed whole: it carried pre-existing uncommitted production hunks (see Deviations)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 313-04-SUMMARY.md

## Lessons

### qwen_reasoning.rs was untracked and handlers.rs carried uncommitted fusi...
`qwen_reasoning.rs` was untracked and `handlers.rs` carried uncommitted fusion edits (`user_for_mode`, `QWEN_EMPTY_THINK`). Both were committed with this plan, so commit `00375744d` includes that prior fusion work (432 added lines in `qwen_reasoning.rs`).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 313-01-SUMMARY.md

---

### crates/vox-eval/src/lib.rs (which declares pub mod qwen_reasoning) is st...
`crates/vox-eval/src/lib.rs` (which declares `pub mod qwen_reasoning`) is still modified and uncommitted; it was outside this plan's file list. A checkout of the commit alone will not compile `vox-eval` until it is committed.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 313-01-SUMMARY.md

---

### .planning/ is tracked; STATE.md and ROADMAP.md were not touched.
`.planning/` is tracked; STATE.md and ROADMAP.md were not touched.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 313-01-SUMMARY.md

---

### cargo clippy -p ..
`cargo clippy -p ... -- -D warnings` fails in `vox-eval` (3 `collapsible_if` errors in benchmark code at ~line 784), a dependency outside this plan's files. Out of scope, not fixed. Plain `cargo clippy` on the three crates reports nothing in the touched files.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 313-02-SUMMARY.md

---

### plan_head_before is 85e0e7efb (the HEAD named in the dispatch); the per-...
`plan_head_before` is `85e0e7efb` (the HEAD named in the dispatch); the per-plan ledger file was not created, so `commits: 2` is `git rev-list --count 85e0e7efb..HEAD`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 313-02-SUMMARY.md

---

### The three source files were already dirty with prior fusion work; explic...
The three source files were already dirty with prior fusion work; explicit-path staging included it, as the plan accepted.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 313-02-SUMMARY.md

---

### 1. [Rule 3 - Blocking] lane: None on the three existing ChatCompletionRe...
**1. [Rule 3 - Blocking] `lane: None` on the three existing `ChatCompletionRequest` test literals moved into Task 1.** Adding the field breaks those literals at compile time, so Task 1 could not compile its tests without it. Same files, committed in `a83d9c14e`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 313-03-SUMMARY.md

---

### 2. Task 2 mutation check could not be satisfied as written (see Verifica...
**2. Task 2 mutation check could not be satisfied as written** (see Verification); noted rather than weakened.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 313-03-SUMMARY.md

---

### cargo clippy -p vox-ml-cli --features gpu --lib --tests -- -D warnings d...
`cargo clippy -p vox-ml-cli --features gpu --lib --tests -- -D warnings` did not complete: it fails compiling `vox-eval` (3 `collapsible_if` errors in `vox-eval`, a crate this plan does not edit). Nothing in the edited files was reported, but clippy never reached `vox-ml-cli`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 313-03-SUMMARY.md

---

### 1. [Process] Task 1 split into two commits (one per file) instead of one...
**1. [Process] Task 1 split into two commits** (one per file) instead of one. No content impact.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 313-04-SUMMARY.md

---

### 2. [Scope note] The populi commit includes foreign production changes. B...
**2. [Scope note] The populi commit includes foreign production changes.** Before this plan started, `crates/vox-populi/src/mens/tensor/training_text.rs` was already modified and uncommitted (+54/-11 vs HEAD): `chatml_prefix_open_assistant` gained an `assistant` parameter and the empty-think tail, `chatml_supervised_text` gained fusion, `chatml_turns_*`/`fuse_turns` were added, and `hf_tokenize_chatml_supervised` call sites were updated. None of that is mine; I did not edit a non-test line (diff vs the pre-task copy shows added test lines only). The new tests need that signature, so a tests-only patch on HEAD would not compile; commit `013bd55c2` therefore shows 84 insertions / 11 deletions. Reviewer: treat the production hunks in that commit as another author's work. Other callers of the 4-argument `chatml_prefix_open_assistant` in `vox-plugin-mens-candle-{cuda,metal}` and `vox-ml-cli` (`eval_local_prompt.rs`) were not checked.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 313-04-SUMMARY.md

---

### WR-01 and WR-02
WR-01 and WR-02: fixed in `9cec48abd`. The tests pass (19/19 in `qwen_reasoning`).

**Context:** Verified in phase acceptance checks.
**Source:** 313-VERIFICATION.md

---

### WR-03 is deferred
WR-03 is deferred. LANE-02 forbids editing the fusion builders, and non-canonical empty think blocks are outside the locked contract.

**Context:** Verified in phase acceptance checks.
**Source:** 313-VERIFICATION.md

## Patterns

### Capturing fake worker (Arc<Mutex<Vec<InferenceRequest>>>) for handler te...
Capturing fake worker (Arc<Mutex<Vec<InferenceRequest>>>) for handler tests that need the request and its payload

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 313-01-SUMMARY.md

---

### Wire lane as String, parsed in the handler, so a bad value gets the JSON...
Wire lane as String, parsed in the handler, so a bad value gets the JSON error shape (not axum's plain-text 422)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 313-01-SUMMARY.md

---

### Lane is Copy and is set in the single InferenceRequest literal inside th...
Lane is Copy and is set in the single InferenceRequest literal inside the retry loop

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 313-01-SUMMARY.md

---

### Lane-aware raw wrap lives in candle-core; plugin twins are edited identi...
Lane-aware raw wrap lives in candle-core; plugin twins are edited identically

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 313-02-SUMMARY.md

---

### Codegen delegates to chatml_prefix_open_assistant(.., "", cfg) so it is ...
Codegen delegates to chatml_prefix_open_assistant(.., "", cfg) so it is byte-identical; hub/agent reuse the same format template with lane helpers

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 313-02-SUMMARY.md

---

### Plugins hold no lane strings
Plugins hold no lane strings: Lane::parse + chatml_prefix_for_lane are the only sources

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 313-02-SUMMARY.md

---

### Chat lane handling lives entirely in flatten_chat_messages; only the las...
Chat lane handling lives entirely in flatten_chat_messages; only the last user turn and the assistant opener depend on lane

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 313-03-SUMMARY.md

---

### Lane is validated at the top of each HTTP handler, before any other work
Lane is validated at the top of each HTTP handler, before any other work

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 313-03-SUMMARY.md

---

### Prefix string is the loss boundary (encoding.rs aligned_prefix_token_len...
Prefix string is the loss boundary (encoding.rs aligned_prefix_token_len), so string-level prefix asserts lock the mask

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 313-04-SUMMARY.md

## Surprises

### Estimate variance on 313-01
Estimate variance on 313-01: planned 70000 tokens vs 7600 actual (0.11x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 313-01-PLAN.md, 313-01-SUMMARY.md

---

### Estimate variance on 313-02
Estimate variance on 313-02: planned 55000 tokens vs 14000 actual (0.25x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 313-02-PLAN.md, 313-02-SUMMARY.md

---

### Estimate variance on 313-03
Estimate variance on 313-03: planned 50000 tokens vs 3600 actual (0.07x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 313-03-PLAN.md, 313-03-SUMMARY.md

---

### 1. [Rule 3 - Blocking] lane: None on the three existing ChatCompletionRe...
**1. [Rule 3 - Blocking] `lane: None` on the three existing `ChatCompletionRequest` test literals moved into Task 1.** Adding the field breaks those literals at compile time, so Task 1 could not compile its tests without it. Same files, committed in `a83d9c14e`.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 313-03-SUMMARY.md

---

### Estimate variance on 313-04
Estimate variance on 313-04: planned 40000 tokens vs 1500 actual (0.04x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 313-04-PLAN.md, 313-04-SUMMARY.md

