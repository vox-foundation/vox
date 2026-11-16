# Phase 313: Lane Prefix (LANE-01, LANE-02) - Research

**Researched:** 2026-10-26
**Domain:** Rust serve path (axum), Candle plugin prompt assembly, Qwen3 thinking-mode fusion strings
**Confidence:** HIGH (all in-repo; every value below was read from source this session)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Optional JSON `lane` on `ChatCompletionRequest` and `GenerateRequest`: `codegen | hub | agent`. Missing means codegen.
- Unknown lane fails before any model load; error names the allowed values (`codegen`, `hub`, `agent`). Chat and raw generate fail the same way.
- Hub and agent share one prefix (bare `assistant` turn, no `/no_think`, no empty think block). Lane wins over `/no_think` already in the user text.
- Only the last user turn is rewritten, plus the assistant opener. Earlier turns stay as sent. No new system prompt.
- `assemble_prompt` on CUDA and Metal gains a `lane` argument. A prompt that already starts with `<|im_start|>` is left unchanged.
- Codegen text that already contains `/think` or `/no_think` is left alone (no second flag).
- Training: no lane column, no retag. Lock existing fusion tests (empty think outside the loss; non-empty think body not rewritten).
- Proof is unit tests only. No model load, no serve gate change, no training-mix retag.

### Claude's Discretion
None.

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LANE-01 | Codegen request = ` /no_think` + empty think block; hub/agent = bare thinking prefix ending `assistant\n`; a test locks both strings | New `Lane` helper in `vox_eval::qwen_reasoning`; threaded through `flatten_chat_messages`, `InferenceRequest` payload, and both `assemble_prompt`s |
| LANE-02 | Codegen training row uses same non-thinking prefix, empty think outside loss; a row with a non-empty think body is not rewritten with `/no_think` | Already implemented in `candle-core` `training_text.rs`; phase = add missing lock tests (incl. the weaker `vox-populi` copy) |
</phase_requirements>

## Summary

The codegen prefix already exists and is shared through `vox_eval::qwen_reasoning`. What is missing is (1) a lane selector, (2) a hub/agent branch at the three prefix-building sites, and (3) lock tests. Nothing needs a new crate, dependency, or crate edge: `vox-ml-cli`, `vox-plugin-mens-candle-core`, `-cuda` and `-metal` already depend on `vox-eval` directly (`vox-eval = { workspace = true }` in each `Cargo.toml`), so one `Lane` type in `vox-eval` serves every consumer and the `crate-edges` ratchet is untouched.

The lane has to cross a process-internal JSON boundary, which is the part most easily forgotten: `do_generate` -> `InferenceRequest` -> `inference_payload()` JSON -> plugin `PromptRequest` (serde) -> `assemble_prompt`. The chat route is different: `flatten_chat_messages` already emits ChatML, so the plugin leaves it alone (`starts_with("<|im_start|>")`); chat lane handling is entirely in the host's `flatten_chat_messages`. Raw `/generate` lane handling is entirely in the plugin's `assemble_prompt`. Both must be edited identically on CUDA and Metal (the files are copy-pasted twins).

LANE-02 needs no production change. The behavior is already in `chatml_prefix_open_assistant` / `chatml_supervised_text` / `chatml_turns_*`; the work is closing test gaps (notably the `vox-populi` copy has no fusion tests, and nothing locks the message-rows reasoning case).

**Primary recommendation:** Add `Lane` + `user_for_lane` + `assistant_opener_for_lane` to `vox-eval/src/qwen_reasoning.rs` (TDD first), then wire it through serve and both plugins; leave training code untouched and add tests only.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Lane parse + prefix strings (single source) | `vox-eval` (L2) | — | Already owns `QWEN_EMPTY_THINK`, `user_for_mode`, `thinking_mode_prompt`; every consumer already depends on it |
| Wire field + validation, chat flattening | `vox-ml-cli` serve (`schema.rs`, `handlers.rs`) | — | Fails before the worker channel is touched, i.e. before model load |
| Raw-prompt wrap by lane | Candle plugins (`assemble_prompt`, CUDA + Metal) | `candle-core` `training_text.rs` (shared builder) | Raw `/generate` prompt is wrapped plugin-side today |
| Host -> plugin transport | `serve/worker.rs` `inference_payload` | — | Only place the JSON handed to `run_inference` is built for serve |
| Training fusion (LANE-02) | `candle-core` `training_text.rs` | `vox-populi` `training_text.rs` (legacy copy) | Existing behavior; tests only |

## Standard Stack

No new packages. Package Legitimacy Audit: **not applicable — no external packages are installed** (`serde`, `serde_json`, `axum`, `anyhow` are already workspace deps).

| Existing item | Where | Use |
|---|---|---|
| `QWEN_EMPTY_THINK`, `user_for_mode`, `thinking_mode_prompt` | `crates/vox-eval/src/qwen_reasoning.rs` | Reuse; add lane helpers beside them |
| `ChatmlConfig`, `chatml_prefix_open_assistant` | `crates/vox-plugin-mens-candle-core/src/{config,training_text}.rs` | Codegen path stays byte-identical |

### Verified current strings (in-repo, read this session)

`[VERIFIED: crates/vox-eval/src/qwen_reasoning.rs:12]`
```rust
pub const QWEN_EMPTY_THINK: &str = "<think>\n\n</think>\n\n";
```
`[VERIFIED: qwen_reasoning.rs:39-48]`
```rust
pub fn user_for_mode(user: &str, reasoning: bool) -> String {
    if reasoning || user.contains("/no_think") || user.contains("/think") {
        user.to_string()
    } else {
        format!("{} /no_think", user.trim_end())
    }
}
```
`[VERIFIED: qwen_reasoning.rs:64-67]`
```rust
pub fn thinking_mode_prompt(question: &str) -> String {
    format!("<|im_start|>user\n{question}<|im_end|>\n<|im_start|>assistant\n")
}
```
`[VERIFIED: crates/vox-ml-cli/src/commands/ai/serve/handlers.rs:237-281]` — `flatten_chat_messages(messages, tools)` calls `user_for_mode(&content, false)` on the last user turn, then ends with:
```rust
    // Direct-answer generation, matching QLoRA training: prefilled empty think.
    out.push_str("<|im_start|>assistant\n");
    out.push_str(vox_eval::qwen_reasoning::QWEN_EMPTY_THINK);
```
`[VERIFIED: cuda/src/inference.rs:828-857 (metal twin at :991-)]` — `fn assemble_prompt(system: Option<&str>, prompt: &str) -> String`; early-returns for `prompt.starts_with("<|im_start|>")`; otherwise calls `chatml_prefix_open_assistant(system.unwrap_or(""), prompt, "", &cfg)` and strips the empty system turn `"<|im_start|>system\n<|im_end|>\n"` when `system.is_none()`.

## Architecture Patterns

### Data flow

```
JSON body {lane?}
   |
   +-- POST /v1/chat/completions -> do_chat_completions
   |      parse lane (400 on unknown, before anything else)
   |      flatten_chat_messages(msgs, tools, lane)  -> ChatML ending in lane opener
   |      GenerateRequest{prompt: ChatML, lane}  -> do_generate
   |
   +-- POST /generate (raw) -> do_generate
          parse lane (400 on unknown)
          prompt_for_output_mode(prompt)         (may append JSON instruction)
          InferenceRequest{.., lane}
             -> worker::inference_payload -> JSON {"system","prompt",.., "lane"}
                -> plugin PromptRequest{lane} -> Lane::parse -> assemble_prompt(system, prompt, lane)
                       prompt starts with <|im_start|>  -> unchanged (chat path, eval paths)
                       else Codegen -> existing wrapper (byte-identical)
                       else Hub/Agent -> bare-thinking wrapper
```

### Recommended helper (in `vox-eval`, TDD first)

```rust
// Source: shape derived from existing user_for_mode / thinking_mode_prompt in qwen_reasoning.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lane { #[default] Codegen, Hub, Agent }
impl Lane {
    pub const ALLOWED: &'static str = "codegen, hub, agent";
    pub fn parse(s: &str) -> Result<Lane, String>;   // exact lowercase; Err names ALLOWED
    pub fn as_str(self) -> &'static str;
    pub fn is_thinking(self) -> bool;                 // Hub | Agent
}
pub fn user_for_lane(user: &str, lane: Lane) -> String;       // Codegen = user_for_mode(user,false)
pub fn assistant_opener_for_lane(lane: Lane) -> &'static str; // Codegen = QWEN_EMPTY_THINK, else ""
```

Expected strings (the lock tests; both literal, per the phase goal):

| Lane | Last user turn (`"hi"`) | After `<|im_start|>assistant\n` |
|---|---|---|
| codegen | `hi /no_think` | `<think>\n\n</think>\n\n` |
| hub / agent | `hi` | *(nothing — string ends at `assistant\n`)* |

Full chat string, no system, no tools: codegen = `"<|im_start|>user\nhi /no_think<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"` (already asserted today, handlers.rs test); hub/agent = `"<|im_start|>user\nhi<|im_end|>\n<|im_start|>assistant\n"`.

### Error shape (recommend; CONTEXT is silent on the wire format)

Use `lane: Option<String>` on both structs and validate in the handlers with `Lane::parse`, returning **400** in each route's existing JSON shape (precedent: the 409 model-mismatch branch of `do_generate` returns `GenerateResponse{text: msg, errors: vec![msg], valid:false}`; `do_chat_completions` already copies `gen_resp.text` into `content` with the delegate's status). A `serde` enum field would also name the variants but yields axum's plain-text 422 from the extractor, which breaks clients that always decode JSON (`VoxLocalAdapter` reads `{code, valid, errors}`). Message must contain `codegen`, `hub`, `agent`. `[ASSUMED]` that exact-case-sensitive matching is wanted (`"Hub"` -> error).

### Anti-Patterns to Avoid
- **Re-implementing the strings in the plugins or handlers.** Use the `vox-eval` helper; training/serve/plugin parity is the point of that module.
- **Editing only one plugin.** `assemble_prompt` exists in `vox-plugin-mens-candle-cuda` and `-metal`; keep them identical and test both.
- **Defaulting an unknown lane to codegen** (locked: must fail).
- **Passing the parsed `Lane` through the repair loop only on attempt 0.** `do_generate` rebuilds `InferenceRequest` every retry iteration; `lane` must be set in that single literal so retries keep it.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---|---|---|---|
| ChatML raw wrap for hub/agent | A third string builder in each plugin | One new builder in `candle-core` `training_text.rs` (e.g. `chatml_prefix_for_lane(system, user, lane, cfg)`), with Codegen delegating to existing `chatml_prefix_open_assistant` | CUDA and Metal both link `candle-core`; avoids two more copies of a copy-pasted function. Hub/agent must not call `chatml_prefix_open_assistant` with a fake assistant string (hack couples to `has_reasoning_trace`). |
| Lane parsing in three crates | Per-crate `match` on strings | `vox_eval::qwen_reasoning::Lane::parse` | One allowed-values message |

## Runtime State Inventory

Not a rename/refactor/migration phase. None — verified: no persisted data, service config, OS registration, secret, or build artifact carries a lane name. `lane` is a new optional request field; old callers omit it and get codegen.

## Common Pitfalls

### Pitfall 1: Struct-literal ripple breaks compile in tests
**What goes wrong:** Adding a field to `GenerateRequest`, `ChatCompletionRequest`, or `InferenceRequest` breaks every struct literal.
**Where (verified by grep):** `GenerateRequest` literal in `do_chat_completions` (handlers.rs ~:303); `ChatCompletionRequest` literals in 3 handler tests (`do_chat_completions_*`); `InferenceRequest` literal in `do_generate` plus 5 literals in `worker.rs` tests. No other crate constructs these (the `vox-audit` hit is a comment; `vox-orchestrator-mcp`'s `VoxLocalGenerateRequest` is a separate client struct that simply never sends `lane`).
**How to avoid:** Add `lane: None` to each; plan one task for the ripple so it lands with the red->green commit.

### Pitfall 2: Lane is dropped between host and plugin
**What goes wrong:** `inference_payload` builds an explicit `serde_json::json!({...})`; a `lane` on `InferenceRequest` that is not added there never reaches the plugin and raw hub/agent silently gets codegen.
**How to avoid:** Test in `worker.rs` that the payload contains `"lane"`, and a plugin-side test that `PromptRequest` honors it (see Validation). Plugin `PromptRequest` has `#[serde(default)]` fields and no `deny_unknown_fields`, so an unknown *key* would be ignored — which is why the plugin must also reject an unknown *value* via `Lane::parse` (defense in depth; host already validated).

### Pitfall 3: Tools + hub/agent chat does not end at `assistant\n`
**What goes wrong:** `do_generate` runs `prompt_for_output_mode`, and for `tool_args_json` that appends `"\n\nRespond with a single valid JSON object only. No markdown fences, no commentary."` (`structured_eval::strict_json_prompt`, `[VERIFIED: crates/vox-corpus/src/corpus/structured_eval.rs:46-50]`) **after** the opener. This already happens today for codegen (text trails the empty think block). So for a hub/agent chat request that offers `tools`, the string actually sent to the model does not end at `assistant\n`. The lock is on `flatten_chat_messages` output and `assemble_prompt`; do not try to "fix" the tools path in this phase.
**How to avoid:** Lock tests on `flatten_chat_messages` / `assemble_prompt` only; record in the plan that tools-mode trailing instruction is pre-existing and out of scope. Surface to the user as a follow-up (agent lane will normally offer tools).

### Pitfall 4: "Lane wins over `/no_think`" is ambiguous about the user text
**What goes wrong:** CONTEXT says hub/agent "never prefill the empty think block" and LANE-01 says "no `/no_think`" in the prefix; it does not say whether a `/no_think` already in the user text is *stripped*.
**Recommendation:** strip `/no_think` from the last user turn for hub/agent (`replace("/no_think","")` then `trim_end()`), so the prefix literally contains no `/no_think`, and leave `/think` alone. `[ASSUMED]` — confirm at plan time; either way lock it with a test. Only the **last user** turn is touched (locked).

### Pitfall 5: Codegen + user-supplied `/think` still gets the empty think opener
**What goes wrong:** Today `flatten_chat_messages` always prefills `QWEN_EMPTY_THINK` and `user_for_mode` only skips the *flag* when `/think` is present. Locked decision ("left alone") means codegen keeps that exact behavior. Do not "improve" it; just add a test documenting it so the contradiction is intentional.

### Pitfall 6: Tests in the plugins must not need a GPU
`cuda`/`metal` default features are `[]` (GPU kernels are opt-in), and `pub mod inference;` is not feature-gated in either `lib.rs`; the existing `assemble_prompt_*` tests run on CPU. `[VERIFIED: Cargo.toml features + lib.rs]`. Do not enable `cuda`/`metal` features for these tests.

### Pitfall 7: Repo rules that touch this work
- Test-first (AGENTS.md): every new `pub fn` in `crates/*/src/**` needs a test in the same file (`skeleton/untested-pub-api`). `Lane::parse`, `user_for_lane`, `assistant_opener_for_lane`, `chatml_prefix_for_lane` each need an in-file test.
- No new crate edges (none needed); do not touch `contracts/ci/crate-edges.allow.v1.json`.
- Run `vox run scripts/fmt.vox` (dirty-only) rather than `cargo fmt --all`.
- Local verification is per-crate only (`cargo check -p`, `cargo test -p`); never workspace clippy/test.
- Do not add a `VOX_*` env var; none is needed.

## LANE-02: what already exists vs the gap

`[VERIFIED: crates/vox-plugin-mens-candle-core/src/training_text.rs]` existing tests, by name:
`direct_answer_puts_empty_think_in_the_prompt_not_the_loss`, `reasoning_trace_keeps_the_thinking_mode_entry_point`, `message_rows_mask_the_empty_think_block_the_same_way`. They assert `prefix.contains(" /no_think")`, `prefix.ends_with(QWEN_EMPTY_THINK)`, `&full[prefix.len()..] == "code<im_end>"` (loss span is the answer only), and for a reasoning row `!prefix.contains("/no_think")`, `prefix.ends_with("<|im_start|>assistant\n")`, think body preserved.

Loss boundary mechanism `[VERIFIED: candle-core/src/candle_qlora_train/training_loop/encoding.rs:21-41]`: `prefix_len = aligned_prefix_token_len(tokenizer, prefix_text, text)`, so "outside the loss" is exactly "inside the prefix string". String-level prefix tests are therefore a sound proxy; no tokenizer is needed.

**Gaps to close (tests only):**
1. `vox-populi` `training_text.rs` tests (`plain_format_has_newlines`, `chatml_contains_markers`, `chatml_supervised_starts_with_open_assistant_prefix`) never assert `/no_think`, the empty think block, loss span, or reasoning pass-through. Add the direct-answer and reasoning-trace locks there (copy of the core tests, using `crate::mens::tensor::training_config::ChatmlConfig::default()`). Populi tests need feature `mens-candle-qlora` (the `#[cfg(test)]` block itself is ungated but `training_config` availability should be checked at plan time).
2. Message-row reasoning case: nothing locks that `chatml_turns_text`/`chatml_turns_prefix_open_assistant` leave a `<think>\nplan\n</think>` assistant turn **and its user turn** un-rewritten (`fuse_turns` skips both). Add in core and populi.
3. `assistant` already starting with `QWEN_EMPTY_THINK` is not double-prefixed (`assistant_body`, `fuse_turns`) — add to `vox-eval` tests (partly covered; add the explicit `starts_with` case).
4. A user turn already containing `/no_think` or `/think` in a codegen row gets no second flag, in both `user_for_mode` and the supervised path.

Do **not** add a lane column, retag, or change `fuse_turns`.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Hub/agent strips `/no_think` from the last user turn (vs. only dropping the opener prefill) | Pitfall 4 | Prefix could contain a stray `/no_think`, failing LANE-01 literal reading |
| A2 | Lane matching is exact, lowercase, no trimming | Error shape | Callers sending `"Hub"` get 400 rather than a match |
| A3 | 400 with each route's existing JSON error shape (not axum 422) | Error shape | Client-visible status/body differs from what user pictured; no impact on locked behavior |
| A4 | Both plugins reject an unknown `lane` value too (defense in depth) | Pitfall 2 | Low; host already validated |

## Open Questions

1. **Tools + agent lane trailing JSON instruction** (Pitfall 3) — pre-existing; recommend deferring and noting in the phase summary. Not blocking.
2. **A1** — planner should state the chosen behavior in the plan's task text so the executor does not choose.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|---|---|---|---|---|
| `cargo` / Rust stable | all tests | ✓ | cargo 1.98.1 | — |
| GPU / CUDA / Metal kernels | — | not needed | — | tests run CPU-only |
| Qwen3-8B weights | — | not needed (explicitly out of scope) | — | no model is loaded |

Note: `cargo` is shell-aliased through `lean_ctx_track`; the build-broker shim rule applies (plain `cargo` on PATH). Working tree on `cursor/t3-deeper-extraction-v55-510f` already has unrelated dirty files; stage only this phase's files.

## Validation Architecture

> No model is loaded, no number is produced, no score is claimed. All proof is deterministic string/serde unit tests. Commands below were taken from existing test names/crates; **they were not executed in this research pass** (read-only).

### Test Framework
| Property | Value |
|---|---|
| Framework | built-in `cargo test` (`#[test]`, `#[tokio::test]`); nextest optional |
| Config file | none needed |
| Feature flags | `vox-ml-cli` default features include `execution-api` via `mens-base`; serve tests are `#[cfg(all(test, feature = "execution-api"))]` so run with default features |
| Quick run | per-crate filters below |
| Full suite | not required locally (hosted CI is the gate) |

### Phase Requirements -> Test Map

| Req | Behavior | Crate | Command (filter) | File exists? |
|---|---|---|---|---|
| LANE-01 | `Lane::parse` accepts exactly `codegen/hub/agent`, error names all three; default is Codegen | `vox-eval` | `cargo test -p vox-eval qwen_reasoning` | ❌ Wave 0 (add in `qwen_reasoning.rs` `mod tests`) |
| LANE-01 | `user_for_lane`: codegen `"hi"` -> `"hi /no_think"`; `/think` or `/no_think` present -> unchanged; hub/agent strip `/no_think`; `assistant_opener_for_lane`: codegen = `QWEN_EMPTY_THINK`, hub/agent = `""`; hub == agent | `vox-eval` | `cargo test -p vox-eval qwen_reasoning` | ❌ Wave 0 |
| LANE-01 | Raw wrap by lane (system/no-system, hub/agent literal strings; codegen byte-identical to today) | `vox-plugin-mens-candle-core` | `cargo test -p vox-plugin-mens-candle-core training_text` | ❌ Wave 0 (new fn + tests); existing 3 tests ✅ |
| LANE-01 | `flatten_chat_messages(.., lane)`: codegen literal unchanged; hub and agent produce `"<|im_start|>user\nhi<|im_end|>\n<|im_start|>assistant\n"`; hub with `/no_think` in user text; only last user turn changes; earlier turns and system/tool catalog unchanged | `vox-ml-cli` | `cargo test -p vox-ml-cli flatten_chat_messages` | ✅ 2 existing (update call sites) + ❌ new |
| LANE-01 | Wire: `lane` absent/`null` -> `None`; `"hub"` parses; both `ChatCompletionRequest` and `GenerateRequest` | `vox-ml-cli` | `cargo test -p vox-ml-cli schema::tests` (module `tests` in `schema.rs`; or filter `chat_completion_request_accepts`/`generate_request_`) | ✅ module / ❌ new cases |
| LANE-01 | Unknown lane -> 400, message names `codegen`, `hub`, `agent`, for **both** `do_chat_completions` and `do_generate`, and the fake worker receives **zero** requests (proves failure before inference) | `vox-ml-cli` | `cargo test -p vox-ml-cli unknown_lane` | ❌ Wave 0 (extend `fake_app_state` with a received-counter) |
| LANE-01 | `inference_payload` carries `"lane"` (e.g. `"hub"`; absent -> `"codegen"`) | `vox-ml-cli` | `cargo test -p vox-ml-cli the_payload_carries` | ✅ module (`worker.rs`) / ❌ new case |
| LANE-01 | CUDA `assemble_prompt(system, prompt, lane)`: codegen outputs identical to the 3 existing assertions; hub/agent raw strings; pre-wrapped `<|im_start|>` prompt unchanged for every lane; unknown lane string rejected | `vox-plugin-mens-candle-cuda` | `cargo test -p vox-plugin-mens-candle-cuda assemble_prompt` | ✅ 3 existing (update args) + ❌ new |
| LANE-01 | Same, Metal twin | `vox-plugin-mens-candle-metal` | `cargo test -p vox-plugin-mens-candle-metal assemble_prompt` | ✅ 3 existing + ❌ new |
| LANE-02 | Codegen row: `/no_think` + empty think in prefix, loss span = answer only | `vox-plugin-mens-candle-core` | `cargo test -p vox-plugin-mens-candle-core direct_answer_puts_empty_think` | ✅ |
| LANE-02 | Non-empty think body keeps thinking entry; prefix has no `/no_think`, ends `assistant\n`, body intact | `vox-plugin-mens-candle-core` | `cargo test -p vox-plugin-mens-candle-core reasoning_trace_keeps` | ✅ |
| LANE-02 | Message rows: empty think masked; **reasoning assistant turn and its user turn not rewritten** | `vox-plugin-mens-candle-core` | `cargo test -p vox-plugin-mens-candle-core message_rows` | ✅ direct case / ❌ reasoning case |
| LANE-02 | Same locks on the legacy populi copy | `vox-populi` | `cargo test -p vox-populi training_text` | ❌ Wave 0 (no fusion assertions today); verify the feature set needed (`mens-candle-qlora`) before the plan commits to a command |
| LANE-02 | No double empty think / no second flag | `vox-eval` | `cargo test -p vox-eval direct_answers_gain_no_think` | ✅ partial / ❌ add `starts_with(QWEN_EMPTY_THINK)` case |

### Sampling Rate
- **Per task commit:** the single-crate filter for the crate touched.
- **Per wave merge:** all rows above, crate by crate (`vox-eval`, `vox-plugin-mens-candle-core`, `-cuda`, `-metal`, `vox-ml-cli`, `vox-populi`) — five-six short test runs, not a workspace run.
- **Phase gate:** those same commands green, plus `cargo check -p vox-ml-cli -p vox-plugin-mens-candle-cuda -p vox-plugin-mens-candle-metal` (catches the struct-literal ripple), and `cargo clippy -p <touched crate> -- -D warnings` per touched crate only.

### Mutation spot-checks (AGENTS.md: verify guards by breaking them)
- Delete the `lane` key from `inference_payload` -> payload test must fail.
- Make unknown lane fall back to codegen -> the 400 test must fail.
- Make hub/agent append `QWEN_EMPTY_THINK` -> the literal hub string test must fail.
- Guard each mutation with a before/after presence check.

### Wave 0 Gaps
- [ ] `vox-eval`: `Lane`, `user_for_lane`, `assistant_opener_for_lane` + tests (write tests first)
- [ ] `candle-core` `training_text.rs`: `chatml_prefix_for_lane` + tests; add message-row reasoning lock
- [ ] `vox-ml-cli`: `fake_app_state` variant that counts worker receives
- [ ] `vox-populi` `training_text.rs`: add the fusion locks

## Security Domain

`security_enforcement` not disabled; relevant surface is small.

| ASVS | Applies | Control |
|---|---|---|
| V5 Input validation | yes | `lane` is an enum-like string validated against a closed set before use; reject, never coerce |
| V2/V3/V4/V6 | no | no auth/session/crypto change |

| Threat | STRIDE | Mitigation |
|---|---|---|
| Prompt/format injection via `lane` value | Tampering | Closed-set parse; value never interpolated into the prompt (only mapped to fixed strings) |
| User text carrying `/no_think` to force non-thinking on hub/agent | Tampering | Lane wins (strip), locked by test |
| Error message echoing unbounded user input | Info/DoS | Truncate or quote the offending value in the error (suggest <=32 chars) |

## Sources

### Primary (HIGH — read in this session)
- `crates/vox-eval/src/qwen_reasoning.rs` (constants, `user_for_mode`, `assistant_body`, `thinking_mode_prompt`, tests)
- `crates/vox-ml-cli/src/commands/ai/serve/{schema,handlers,worker,prompt}.rs`
- `crates/vox-plugin-mens-candle-{cuda,metal}/src/inference.rs` (`PromptRequest`, `generate_from_json`, `assemble_prompt`, tests)
- `crates/vox-plugin-mens-candle-core/src/training_text.rs`, `.../training_loop/encoding.rs`
- `crates/vox-populi/src/mens/tensor/training_text.rs`
- `crates/vox-corpus/src/corpus/structured_eval.rs` (`strict_json_prompt`)
- Cargo manifests of `vox-ml-cli`, `-cuda`, `-metal`, `-core` (vox-eval edges, features)
- `.planning/phases/313-.../313-CONTEXT.md`, `.planning/REQUIREMENTS.md` (LANE-01/02), `.planning/research/SUMMARY.md`

### Tertiary (LOW)
- None used. No web sources; the Qwen3 fusion format is taken from the repo's own module doc, not external docs.

## Metadata

**Confidence breakdown:**
- Standard stack / wiring: HIGH — read every hop (handler -> worker -> payload -> plugin).
- Pitfalls: HIGH for 1-3, 6-7; MEDIUM for 4 (genuine spec ambiguity, flagged A1).
- Populi test command: MEDIUM — feature requirements not exercised.

**Research date:** 2026-10-26
**Valid until:** 30 days or until `qwen_reasoning.rs` / serve handlers change
