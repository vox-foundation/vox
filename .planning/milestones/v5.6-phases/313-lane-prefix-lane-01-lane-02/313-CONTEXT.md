# Phase 313: Lane Prefix (`LANE-01`, `LANE-02`) - Context

**Gathered:** 2026-10-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Codegen requests and codegen training rows stay on the non-thinking prefix. Hub and agent requests use the bare thinking-mode prefix. Both strings are locked by unit tests. This phase does not retag the training mix, load Qwen3-8B, or change the serve gate.

</domain>

<decisions>
## Implementation Decisions

### Request contract
- An explicit `lane` of `codegen`, `hub`, or `agent` on the serve request selects the prefix.
- Callers that omit `lane` get `codegen` (non-thinking), so today's chat and `assemble_prompt` callers stay as they are.
- This phase does not retag training rows by lane. Lock the fusion that is already there: codegen direct answers keep the empty think block outside the loss, and a real think body is not rewritten.
- Proof is unit tests on the prefix helper, `flatten_chat_messages`, and `assemble_prompt`. No model load.

### Which calls branch
- Both chat completions and raw `assemble_prompt` take `lane`, on CUDA and Metal, through one helper.
- Hub and agent share one prefix: a bare `assistant` turn, no `/no_think`, no empty think block. The lane name is kept; the string is the same.
- For hub and agent, the lane wins over `/no_think` already present in the user text. Those lanes never prefill the empty think block.
- An unknown `lane` fails the request. It is not silently treated as codegen.

### Wire shape
- Optional `lane` on `ChatCompletionRequest` and `GenerateRequest`. Missing means codegen.
- Only the last user turn is rewritten, plus the assistant opener. Earlier turns stay as sent.
- `assemble_prompt` on CUDA and Metal gains a `lane` argument. A prompt that already starts with `<|im_start|>` is left unchanged, so thinking-mode eval keeps working.
- Hub and agent do not get a new system prompt. The caller's system text stays. Only the last user turn and the assistant opener change.

### Failures and the training lock
- An unknown `lane` fails before any model load, and the error names the allowed values: `codegen`, `hub`, `agent`.
- Codegen text that already contains `/think` or `/no_think` is left alone. Do not append a second flag.
- Training proof is the existing fusion tests: the empty think block stays outside the loss, and a non-empty think body is not rewritten. No new lane column on training rows.
- Chat and raw generate reject an unknown lane the same way, before inference.

### Claude's Discretion
None. The four areas were accepted as recommended.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `vox_eval::qwen_reasoning`: `user_for_mode`, `QWEN_EMPTY_THINK`, `thinking_mode_prompt`.
- `flatten_chat_messages` in `crates/vox-ml-cli/src/commands/ai/serve/handlers.rs` already tags the last user turn and prefills the empty think block.
- `assemble_prompt` in the CUDA and Metal inference modules already wraps raw prompts as direct answers and leaves a leading `<|im_start|>` prompt unchanged.
- `chatml_prefix_open_assistant` in candle-core and populi `training_text` already puts the empty think block in the prefix for direct answers.

### Established Patterns
- ChatML fusion is shared through `vox_eval::qwen_reasoning`, not copied per backend.
- Request structs live in `crates/vox-ml-cli/src/commands/ai/serve/schema.rs`. Missing JSON keys deserialize as `None` and keep old callers working.
- Serve tests sit next to `flatten_chat_messages`. Inference tests sit next to `assemble_prompt`.

### Integration Points
- `ChatCompletionRequest` and `GenerateRequest` are the wire types for `lane`.
- `flatten_chat_messages` and both `assemble_prompt` implementations are the prefix builders.
- Training fusion stays in `training_text`. This phase locks it with tests and does not add a lane column.

</code_context>

<specifics>
## Specific Ideas

No specific requirements beyond the accepted answers. Match the existing empty-think string `<think>\n\n</think>\n\n` and the bare thinking opener already used by `thinking_mode_prompt`.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
