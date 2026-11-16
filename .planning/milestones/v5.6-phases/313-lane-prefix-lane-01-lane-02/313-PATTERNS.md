# Phase 313: Lane Prefix (LANE-01, LANE-02) - Pattern Map

**Mapped:** 2026-10-26
**Files analyzed:** 9 (all modified; 0 new files)
**Analogs found:** 9 / 9 (each file is its own best analog; new code mirrors an adjacent function in the same file)

Line numbers below were read from the working tree on 2026-10-26.

## Tracked-source warning (read first)

`crates/vox-eval/src/qwen_reasoning.rs` is **untracked** (`?? ` in `git status`; `git ls-files` does not list it). It is real source under `crates/`, not a gitignored mirror, so it is a valid analog. But it is not yet in any commit, and neither are the `qwen_reasoning` call sites in the files below. These are all uncommitted working-tree edits:

- `vox-eval/src/lib.rs`
- `vox-ml-cli/.../serve/handlers.rs`
- `vox-plugin-mens-candle-core/src/training_text.rs`
- `vox-plugin-mens-candle-{cuda,metal}/src/inference.rs`
- `vox-populi/src/mens/tensor/training_text.rs`

The planner should add a first task or precondition: the prior fusion work must be committed with this phase's commits, or it must be staged deliberately alongside them. Stage only this phase's files (`vox-ml-cli/.../worker.rs` and `schema.rs` are clean at HEAD).

## File Classification

| Modified file | Role | Data flow | Closest analog | Match |
|---|---|---|---|---|
| `crates/vox-eval/src/qwen_reasoning.rs` | utility (pure string helpers) | transform | itself: `user_for_mode`, `assistant_body`, `thinking_mode_prompt` (L42-66) | exact |
| `crates/vox-plugin-mens-candle-core/src/training_text.rs` | utility (prompt builder) | transform | itself: `chatml_prefix_open_assistant` (L11-32) | exact |
| `crates/vox-ml-cli/src/commands/ai/serve/schema.rs` | model (wire DTO) | request-response | itself: `seed`/`top_p`/`system_prompt` optional fields (L14-43, L128-133) | exact |
| `crates/vox-ml-cli/src/commands/ai/serve/handlers.rs` | controller (axum) | request-response | itself: 409 branch in `do_generate` (L87-102) | exact |
| `crates/vox-ml-cli/src/commands/ai/serve/worker.rs` | service (host to plugin JSON bridge) | request-response | itself: `inference_payload` (L132-149), `seed`/`top_p` plumbing | exact |
| `crates/vox-plugin-mens-candle-cuda/src/inference.rs` | service (plugin prompt assembly) | request-response | itself: `PromptRequest` (L460-478), `assemble_prompt` (L828-858) | exact |
| `crates/vox-plugin-mens-candle-metal/src/inference.rs` | service (plugin prompt assembly) | request-response | CUDA twin: `PromptRequest` (L602-620), `assemble_prompt` (L991-1021) | exact (copy-paste twin) |
| `crates/vox-populi/src/mens/tensor/training_text.rs` | utility (legacy copy) | transform | `vox-plugin-mens-candle-core` `training_text.rs` tests (L243-285) | exact (test copy) |
| (tests) `vox-eval` `mod tests` L229+ | test | transform | `direct_answers_gain_no_think_and_an_empty_block` (L241) | exact |

## Pattern Assignments

### 1. `crates/vox-eval/src/qwen_reasoning.rs` (utility, transform): add `Lane`, `user_for_lane`, `assistant_opener_for_lane`

**Analog:** same file, current helpers (L12-66).

**Constants and existing helpers to build on** (L12, L42-49, L64-67):

```rust
pub const QWEN_EMPTY_THINK: &str = "<think>\n\n</think>\n\n";

#[must_use]
pub fn user_for_mode(user: &str, reasoning: bool) -> String {
    if reasoning || user.contains("/no_think") || user.contains("/think") {
        user.to_string()
    } else {
        format!("{} /no_think", user.trim_end())
    }
}

#[must_use]
pub fn thinking_mode_prompt(question: &str) -> String {
    format!("<|im_start|>user\n{question}<|im_end|>\n<|im_start|>assistant\n")
}
```

**Conventions to copy:**
- Every `pub fn` has `/// doc` plus `#[must_use]`.
- Plain `&str` in and owned `String` out, with no `Result<_, anyhow>` and no dependencies.
- `Lane::parse` returns `Result<Lane, String>`, where the error names `ALLOWED` and quotes the offending value truncated to about 32 chars (RESEARCH security table).
- Codegen branch: `user_for_lane(user, Lane::Codegen) == user_for_mode(user, false)`, and `assistant_opener_for_lane(Codegen) == QWEN_EMPTY_THINK`. This keeps the codegen output byte-identical.
- Hub/agent branch (assumption A1, which the planner must pin in the task text): `user.replace("/no_think", "")` then `trim_end()`. Leave `/think` alone. The opener is `""`.

**Test pattern** (L240-251): inline `mod tests`, `use super::*;`, `assert_eq!` on literal strings:

```rust
    #[test]
    fn direct_answers_gain_no_think_and_an_empty_block() {
        assert_eq!(user_for_mode("write it", false), "write it /no_think");
        assert_eq!(user_for_mode("write it /think", false), "write it /think");
        assert_eq!(assistant_body("code"), format!("{QWEN_EMPTY_THINK}code"));
    }
```

Write the failing tests for `Lane::parse` (accepts exactly 3 lowercase values, error names all three, `Lane::default() == Codegen`), `user_for_lane`, and `assistant_opener_for_lane`. Add the LANE-02 `assistant_body(format!("{QWEN_EMPTY_THINK}x"))` no-double-prefix `starts_with` case here too. `skeleton/untested-pub-api` requires an in-file test per new `pub fn`.

**Export:** `vox-eval/src/lib.rs` already declares `qwen_reasoning` (dirty). Consumers use the path `vox_eval::qwen_reasoning::...`, so no re-export is needed.

---

### 2. `crates/vox-plugin-mens-candle-core/src/training_text.rs` (utility, transform): add `chatml_prefix_for_lane`

**Analog:** `chatml_prefix_open_assistant` (L11-32). Do **not** call it with a fake assistant string for hub/agent (RESEARCH "Don't hand-roll").

```rust
use crate::config::ChatmlConfig;
use vox_eval::qwen_reasoning::{self, QWEN_EMPTY_THINK};

#[must_use]
pub fn chatml_prefix_open_assistant(
    system: &str,
    user: &str,
    assistant: &str,
    cfg: &ChatmlConfig,
) -> String {
    let reasoning = qwen_reasoning::has_reasoning_trace(assistant);
    let user = qwen_reasoning::user_for_mode(user, reasoning);
    let tail = if reasoning { "" } else { QWEN_EMPTY_THINK };
    format!(
        "{start}{sys}\n{system}{end}\n\
         {start}{usr}\n{user}{end}\n\
         {start}{asst}\n{tail}",
        start = cfg.im_start,
        end = cfg.im_end,
        sys = cfg.role_system,
        usr = cfg.role_user,
        asst = cfg.role_assistant,
        tail = tail,
    )
}
```

**Pattern for the new fn:** same `format!` template, but `user = qwen_reasoning::user_for_lane(user, lane)` and `tail = qwen_reasoning::assistant_opener_for_lane(lane)`. `Lane::Codegen` should delegate to `chatml_prefix_open_assistant(system, user, "", cfg)` so codegen stays byte-identical. This is the same call the plugins make today.

**Existing LANE-02 tests to keep green and mirror** (L243-285): `direct_answer_puts_empty_think_in_the_prompt_not_the_loss`, `reasoning_trace_keeps_the_thinking_mode_entry_point`, `message_rows_mask_the_empty_think_block_the_same_way`. The test module uses `use super::*;`, a local `fn turn(role, content) -> ChatmlTurn` helper (L186), and `ChatmlConfig::default()`.

Loss-span assertion shape to copy:

```rust
        assert_eq!(
            &full[prefix.len()..],
            format!("code{}", cfg.im_end),
            "loss span is the answer only"
        );
```

**LANE-02 gap to add here** (tests only, do not touch `fuse_turns`, L107-125): a message-row case with an assistant turn `"<think>\nplan\n</think>\n\ncode"`. Assert that `chatml_turns_text` and `chatml_turns_prefix_open_assistant` leave both that assistant turn **and its user turn** unrewritten: no `/no_think`, and the prefix ends `<|im_start|>assistant\n`. Add a codegen user turn already carrying `/no_think` or `/think` that gets no second flag.

---

### 3. `crates/vox-ml-cli/src/commands/ai/serve/schema.rs` (model, request-response): add `lane: Option<String>` to both request structs

**Analog:** the optional fields already on both structs. Copy the `#[serde(default)]` plus doc-comment style:

```rust
    /// Per-request system prompt override. ... `#[serde(default)]` keeps old
    /// callers (no key in the JSON body at all) unaffected.
    #[serde(default)]
    pub system_prompt: Option<String>,   // GenerateRequest, L36-43
```

```rust
    /// See `GenerateRequest::seed`.
    #[serde(default)]
    pub seed: Option<u64>,               // ChatCompletionRequest, L128-130
```

**Pattern:**
- `GenerateRequest` gets the documented `lane: Option<String>` after `system_prompt` (L43).
- `ChatCompletionRequest` gets `/// See GenerateRequest::lane.` plus `#[serde(default)] pub lane: Option<String>` after `tools` (L140).
- Use a `String`, not a serde enum. A serde enum would yield axum's plain-text 422, which breaks `VoxLocalAdapter`.
- Every item is gated with `#[cfg(feature = "execution-api")]`.

**Test pattern** (L189-222) goes in the existing `mod tests`, which is `#[cfg(all(test, feature = "execution-api"))]` with `use super::{ChatCompletionRequest, GenerateRequest};`:

```rust
    #[test]
    fn generate_request_without_system_prompt_key_defaults_to_none() {
        let req: GenerateRequest =
            serde_json::from_str(r#"{"prompt": "hello", "max_tokens": 64}"#).unwrap();
        assert_eq!(req.system_prompt, None);
    }
```

Add three cases per struct: lane key absent gives `None`, `"lane": null` gives `None`, `"lane": "hub"` gives `Some("hub")`.

---

### 4. `crates/vox-ml-cli/src/commands/ai/serve/handlers.rs` (controller, request-response)

**Analog for the 400 branch:** the 409 model-mismatch early return in `do_generate` (L87-102):

```rust
    if let Some(msg) = mismatched_model_error(req.model.as_deref(), state.model_name.as_ref()) {
        return (
            StatusCode::CONFLICT,
            Json(GenerateResponse {
                text: msg.clone(),
                code: msg.clone(),
                tokens_generated: 0,
                model: state.model_name.to_string(),
                object: "text_completion",
                choices: vec![],
                repair_attempts: None,
                valid: false,
                errors: vec![msg],
            }),
        );
    }
```

**Edits:**
1. **`do_generate`:** right after the 409 block (L102), parse the lane with `vox_eval::qwen_reasoning::Lane::parse` (default `Codegen` when `req.lane` is `None`). On `Err(msg)`, return `StatusCode::BAD_REQUEST` with the same `GenerateResponse` shape. Do this before `output_mode` (L103) and before any channel use, so the worker receives zero requests. Set `lane` once in the single `InferenceRequest { .. }` literal (L133-143) inside `loop`, so repair retries keep it (the RESEARCH anti-pattern about attempt 0). The raw-prompt wrap for a non-ChatML prompt happens plugin-side, so the handler only forwards the lane.
2. **`do_chat_completions`** (L296-301): parse `req.lane` first, before `flatten_chat_messages`. On an unknown lane return 400 in the `ChatCompletionResponse` shape. Copy the way this function already surfaces delegate errors: the `content: Some(gen_resp.text.clone())` fallback in the `None =>` arm (L257-261), which `status` carries through. The simplest route is to build a `ChatCompletionResponse` with `content: Some(msg)`, `finish_reason: "stop"`, and `model: state.model_name.to_string()`. Alternatively, forward `lane: req.lane.clone()` in the `GenerateRequest` literal (L303) and let `do_generate`'s 400 flow through the existing `(status, Json(resp))` return (L280). That second option is the least code and keeps one validation site. But `flatten_chat_messages` runs before it, so make `flatten_chat_messages` take an already-parsed `Lane`. That forces parsing in the chat handler as well. The planner should choose one parse site per route and test both routes.
3. **`flatten_chat_messages`** (L237-279): add `lane: Lane`. Core edits are L262-264 and L275-277:

```rust
        if Some(i) == last_user {
            content = vox_eval::qwen_reasoning::user_for_mode(&content, false);
        }
...
    // Direct-answer generation, matching QLoRA training: prefilled empty think.
    out.push_str("<|im_start|>assistant\n");
    out.push_str(vox_eval::qwen_reasoning::QWEN_EMPTY_THINK);
```

   become `user_for_lane(&content, lane)` and `out.push_str(assistant_opener_for_lane(lane))`. Only the last user turn changes. Earlier turns, the system turn, and the tool catalog stay as sent.

**Struct-literal ripple** (RESEARCH Pitfall 1). Add `lane: None` or the parsed value at:
- `GenerateRequest` literal, `handlers.rs:303`
- `InferenceRequest` literal, `handlers.rs:133`
- `ChatCompletionRequest` literals in tests, `handlers.rs:548`, `:584`, `:612`
- the 5 `InferenceRequest` literals in `worker.rs` tests (see §5)

**Test patterns to copy** (in the existing `mod tests`):

Flatten lock (L513-525):
```rust
        let flattened = flatten_chat_messages(&messages, None);
        assert_eq!(
            flattened,
            "<|im_start|>user\nhi /no_think<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
        );
```
Update both existing `flatten_chat_messages_*` tests (L504, L519) to pass `Lane::Codegen`. Add hub and agent cases asserting exactly `"<|im_start|>user\nhi<|im_end|>\n<|im_start|>assistant\n"`. Add a hub case where the user text already has `/no_think`. Add a multi-turn case showing that only the last user turn changes.

Fake worker (L527-543): `fake_app_state(reply)` spawns a thread that replies to every `InferenceRequest`. For the "zero worker receives" proof on unknown lane, add a variant that bumps an `Arc<AtomicUsize>` per `rx.recv()`, then assert it is `0` after the 400. Keep the same `sync_channel::<InferenceRequest>(8)` and `AppState { tx, model_name, ready }` construction.

Async handler test style (L545): `#[tokio::test] async fn do_chat_completions_...() { let state = fake_app_state(..); let (status, Json(resp)) = do_chat_completions(State(state), Json(req)).await; assert_eq!(status, StatusCode::OK); ... }`. The unknown-lane tests assert `StatusCode::BAD_REQUEST` and that the body text contains `codegen`, `hub`, and `agent`.

---

### 5. `crates/vox-ml-cli/src/commands/ai/serve/worker.rs` (service, request-response): carry `lane` to the plugin

**Analog:** how `seed` / `top_p` already cross the boundary: a field on `InferenceRequest` (L22-23) and a key in the explicit `json!` (L138-147):

```rust
    serde_json::json!({
        "system": system_prompt,
        "prompt": req.prompt,
        "max_tokens": req.max_tokens,
        "temperature": req.temperature,
        "top_k": req.top_k,
        "seed": req.seed,
        "top_p": req.top_p,
        "output_mode": req.output_mode,
    })
    .to_string()
```

**Edits:**
- `InferenceRequest` gets `pub lane: vox_eval::qwen_reasoning::Lane`. This should be `Lane` (`Copy`), not an `Option`. Doc comment style as on `system_prompt` (L26-30).
- The `json!` gets `"lane": req.lane.as_str()`. Codegen serializes as `"codegen"`, so absent and codegen are identical on the wire.
- `vox-ml-cli` already depends on `vox-eval` (`handlers.rs` uses `vox_eval::qwen_reasoning`), so no new crate edge.

**Test pattern** (L155-203): construct `InferenceRequest { prompt, max_tokens: 8, temperature: 0.0, seed: None, top_p: None, top_k: 40, output_mode: None, system_prompt: None, reply: tx }` with `let (tx, _rx) = tokio::sync::oneshot::channel();`, call `inference_payload("S", &req)`, parse with `serde_json::from_str`, and assert on `v["..."]`. Add `lane: Lane::Hub` and assert `v["lane"] == "hub"`. Add `Lane::Codegen` and assert `v["lane"] == "codegen"`. The five existing literals (L158, 185, 211, 241, 263) each need the new field. **Mutation check:** delete the `"lane"` key and confirm the new payload test fails.

---

### 6 & 7. `crates/vox-plugin-mens-candle-cuda/src/inference.rs` and `...-metal/src/inference.rs` (service, request-response)

These are copy-paste twins. Apply **identical** edits to both. CUDA line numbers are given first, then Metal in brackets.

**`PromptRequest`** (CUDA L460-478 [Metal L602-620]): add after `output_mode`. The existing fields are `#[serde(default)]` with no `deny_unknown_fields`:

```rust
            #[serde(default)]
            output_mode: Option<String>,
            // new:
            #[serde(default)]
            lane: Option<String>,
```

**Call site** (CUDA L491 [Metal L633]):

```rust
        check_output_mode(req.output_mode.as_deref())?;

        let prompt = assemble_prompt(req.system.as_deref(), &req.prompt);
```

Parse before assembling, with the same error idiom used just above (`.map_err(|e| anyhow::anyhow!("parse prompt_json: {e}"))`):

```rust
let lane = match req.lane.as_deref() {
    None => Lane::Codegen,
    Some(s) => Lane::parse(s).map_err(|e| anyhow::anyhow!("{e}"))?,
};
let prompt = assemble_prompt(req.system.as_deref(), &req.prompt, lane);
```

This is defense in depth (A4), since the host already validated.

**`assemble_prompt`** (CUDA L828-858 [Metal L991-1021]): add a `lane: Lane` parameter. Keep the early-return ChatML branch (L831-838) untouched. A prompt starting with `<|im_start|>` is left unchanged for every lane. Replace the wrap call (L842-848):

```rust
    let cfg = vox_plugin_mens_candle_core::config::ChatmlConfig::default();
    let wrapped = vox_plugin_mens_candle_core::training_text::chatml_prefix_open_assistant(
        system.unwrap_or(""),
        prompt,
        "",
        &cfg,
    );
    if system.is_none() {
        let empty_system = "<|im_start|>system\n<|im_end|>\n";
        wrapped.strip_prefix(empty_system).unwrap_or(&wrapped).to_string()
    } else {
        wrapped
    }
```

with `chatml_prefix_for_lane(system.unwrap_or(""), prompt, lane, &cfg)`, from the shared builder in §2. Leave the empty-system strip logic as is. Update the doc comment (L824-827 [L987-990]), which names `chatml_prefix_open_assistant`.

**Existing tests** (CUDA L1243-1270 [Metal L1624-1651]) use the exact literal assertions:

```rust
        let out = assemble_prompt(Some("YOU ARE VOX"), "hello");
        assert_eq!(
            out,
            "<|im_start|>system\nYOU ARE VOX<|im_end|>\n<|im_start|>user\nhello /no_think<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
        );
```

Update the three existing tests to pass `Lane::Codegen`. Assertions stay literally identical, which proves byte-identical codegen. Add:
- hub and agent, with and without system. User turn has no `/no_think`, and the output ends `assistant\n`.
- A pre-wrapped `<|im_start|>` prompt unchanged for all three lanes.
- Unknown lane string through `generate_from_json` or a lane-parse helper returns `Err`. This must not need a GPU. The plugin features default to `[]` and `inference` is not feature-gated (RESEARCH Pitfall 6), so do not enable `cuda` or `metal`.

Test commands: `cargo test -p vox-plugin-mens-candle-cuda assemble_prompt` and `cargo test -p vox-plugin-mens-candle-metal assemble_prompt`.

---

### 8. `crates/vox-populi/src/mens/tensor/training_text.rs` (utility, transform): LANE-02 tests only

**Analog:** the candle-core tests (see §2). The populi copy of `chatml_prefix_open_assistant` (L17-32) has the same logic and is already fused:

```rust
pub fn chatml_prefix_open_assistant(
    system: &str, user: &str, assistant: &str,
    cfg: &crate::mens::tensor::training_config::ChatmlConfig,
) -> String {
    let reasoning = vox_eval::qwen_reasoning::has_reasoning_trace(assistant);
    let user = vox_eval::qwen_reasoning::user_for_mode(user, reasoning);
```

**Existing populi tests are weak** (L213-239). They are `plain_format_has_newlines`, `chatml_contains_markers`, and `chatml_supervised_starts_with_open_assistant_prefix`, and none asserts `/no_think`, the empty think block, or the loss span. Add copies of the two core tests (`direct_answer_puts_empty_think_in_the_prompt_not_the_loss`, `reasoning_trace_keeps_the_thinking_mode_entry_point`). Use the existing style:

```rust
        let cfg = crate::mens::tensor::training_config::ChatmlConfig::default();
        let full = chatml_supervised_text("S", "U", "code", &cfg);
        let prefix = chatml_prefix_open_assistant("S", "U", "code", &cfg);
```

For the constant use `vox_eval::qwen_reasoning::QWEN_EMPTY_THINK` (populi uses full paths, not a `use`).

**Feature gate (resolves RESEARCH's open question):**
- `pub mod training_text;` is `#[cfg(feature = "mens-train")]` (`tensor/mod.rs`).
- `vox-eval` is an optional dep pulled in by `mens-train` (`vox-populi/Cargo.toml:78, 153`).
- So the test command is `cargo test -p vox-populi --features mens-train training_text`. A plain `cargo test -p vox-populi` compiles zero of these tests.
- The populi copy has no `chatml_turns_*` / `fuse_turns`. Do not invent them; the message-row reasoning lock lives in candle-core only.
- Do **not** add `chatml_prefix_for_lane` to populi. It is the legacy copy and serve does not use it.

---

## Shared Patterns

### Lane parsing and error message
**Source:** `vox_eval::qwen_reasoning::Lane::parse` (new, §1).
**Apply to:** `handlers.rs` (both routes) and both plugins' `generate_from_json`.
Exact lowercase match (A2). `Err(String)` names `codegen, hub, agent` and quotes the offending value truncated to about 32 chars. The lane value is mapped to fixed strings only and never interpolated into the prompt. No per-crate string `match`.

### Optional wire field with a back-compat default
**Source:** `schema.rs` L14-43 (`seed`, `top_p`, `system_prompt`).
**Apply to:** `lane` on both request structs, and `lane` in `PromptRequest` (`#[serde(default)]`, no `deny_unknown_fields`). Old callers that omit the key behave exactly as before.

### Error response shape (never axum 422)
**Source:** the 409 branch, `handlers.rs` L87-102. `GenerateResponse { text: msg.clone(), code: msg.clone(), valid: false, errors: vec![msg], .. }`. `VoxLocalAdapter` decodes `{code, valid, errors}`.
**Apply to:** the unknown-lane 400 in `do_generate`. The chat route mirrors it as `content: Some(msg)`.

### Literal-string lock tests (the style the roadmap asks for)
**Source:** `handlers.rs` L521-524, cuda `inference.rs` L1246-1249, core `training_text.rs` L253-257: `assert_eq!` against a full literal containing `\n`, not `contains` alone. Use this for the hub/agent lock strings.

### Test-first and doc rules
**Source:** AGENTS.md Test-First Policy.
- Every new `pub fn` (`Lane::parse`, `Lane::as_str`, `Lane::is_thinking`, `user_for_lane`, `assistant_opener_for_lane`, `chatml_prefix_for_lane`) needs a `#[test]` in the same file.
- The failing test is committed or run first.
- Use `#[must_use]` on pure fns.
- No new crate edges; `vox-eval` is already a direct dep of `vox-ml-cli`, `-core`, `-cuda`, `-metal`. Do not touch `contracts/ci/crate-edges.allow.v1.json`.
- No `VOX_*` env var.
- Format with `vox run scripts/fmt.vox` (dirty only), never `cargo fmt --all`.

### Verification scope
Per-crate only:
- `cargo test -p vox-eval qwen_reasoning`
- `cargo test -p vox-plugin-mens-candle-core training_text`
- `cargo test -p vox-ml-cli flatten_chat_messages` (default features include `execution-api`)
- `cargo test -p vox-ml-cli unknown_lane`
- `cargo test -p vox-ml-cli the_payload_carries`
- `cargo test -p vox-plugin-mens-candle-cuda assemble_prompt`
- `cargo test -p vox-plugin-mens-candle-metal assemble_prompt`
- `cargo test -p vox-populi --features mens-train training_text`
- `cargo check -p vox-ml-cli -p vox-plugin-mens-candle-cuda -p vox-plugin-mens-candle-metal` for the struct-literal ripple

## No Analog Found

None. The only structurally new pieces are the `Lane` enum (no enum with `parse` and `ALLOWED` exists in `vox-eval`; follow the `ReasoningScore` doc and derive style at L100-108 and the helpers' `#[must_use]` convention) and the zero-receive counter variant of `fake_app_state` (extend the existing helper at `handlers.rs` L527-543).

## Planner notes (carried from RESEARCH)

- **Pitfall 3 (pre-existing, out of scope):** with `tools` offered, `do_generate` appends the strict-JSON instruction after the opener (`prompt_for_output_mode`, `prompt.rs`), so the agent-lane chat prompt does not literally end at `assistant\n`. The lock tests sit on `flatten_chat_messages` and `assemble_prompt` output only. Surface as a follow-up.
- **A1 must be fixed in the plan text:** hub/agent strips `/no_think` from the last user turn only. Leave `/think`. Lock it with a test.
- **Codegen plus a user-supplied `/think` still gets the empty think opener** (Pitfall 5). Do not "fix" it. Add a test documenting it.
- Do not add a lane column, retag training rows, or change `fuse_turns`.

## Metadata

**Analog search scope:** `crates/vox-eval/src`, `crates/vox-ml-cli/src/commands/ai/serve/`, `crates/vox-plugin-mens-candle-{core,cuda,metal}/src`, `crates/vox-populi/src/mens/tensor/`
**Files read:** 9 source files plus CONTEXT and RESEARCH
**Pattern extraction date:** 2026-10-26
