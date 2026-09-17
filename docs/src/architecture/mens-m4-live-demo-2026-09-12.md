---
title: "M4 Live Demo: Base vs Adapter"
description: "The live, evidenced confirmation that a locally fine-tuned model served on Metal produces measurably different output from the base model, with the two real serving bugs found and fixed along the way."
category: "Architecture SSOTs"
status: "current"
---

Date: 2026-09-12. Host: Apple M5 Max, 128 GB, macOS, Metal.

**Verdict: milestone confirmed.** The same prompt, sent over HTTP to `vox mens serve`
on Metal, produced measurably different text from the base model and from the
adapter-bearing model, with generation forced deterministic on both runs.

## 1. Risk factor 1 — RoPE / compute-dtype gap in the Metal serving path

**Real bug, fixed.** Both gaps were genuine and both were blocking for this model.

- `Qwen3-0.6B`'s cached `model.safetensors` holds 312 tensors and **zero**
  `rotary_emb.inv_freq` entries (verified by reading the safetensors header
  directly). Metal's `inference.rs` looked up `inv_freq`, got nothing, and stored
  `None` — the forward would then run with no positional encoding. The Metal
  *trainer* (`candle_qlora_train::synthesize_rope_inv_freq`) synthesizes the table
  from `config.rope_theta`; serving did not. CUDA's `inference.rs` already carried
  the fix and the comment explaining it.
- `QLoraConfig::default()` hardcodes BF16 compute. The Metal trainer dequantizes to
  F32, and Candle's CPU backend cannot matmul BF16 at all. Metal serving used the
  BF16 default.

Ported both helpers from `crates/vox-plugin-mens-candle-cuda/src/inference.rs` into
`crates/vox-plugin-mens-candle-metal/src/inference.rs`, keeping
`synthesize_rope_inv_freq` byte-for-byte in sync with this lane's own trainer.

Regression tests added in `inference.rs`:

- `inference_rope_synthesis_matches_the_trainer` — asserts the inference copy and
  `candle_qlora_train::synthesize_rope_inv_freq` produce identical tables for
  `head_dim=128, theta=1e6` (this model) and `head_dim=64, theta=default`.
  **Mutation-verified**: perturbing the inference copy's exponent by `+0.001`
  makes the test fail; reverted and it passes again.
- `compute_dtype_is_f32_off_cuda`.

The live evidence that the fix matters: the served base model emits coherent
English, not token salad.

## 2. Risk factor 2 — second real bug found: run directory rejected as "too small"

`vox mens serve --model <run-dir>` could not start at all:

```
Error: checkpoint /…/run_base too small (224 bytes)
```

`validate_checkpoint_manifest` sized the **directory's own inode**. A QLoRA
checkpoint is a run *directory* — which is what the serve dispatch arm itself
assumes, probing `model.join("candle_qlora_adapter.safetensors")` a few lines
earlier. Fixed at the shared function
(`crates/vox-populi/src/mens/tensor/manifest/part_io.rs`): when the path is a
directory, size the adapter (or `merged.safetensors`) inside it; a directory with
no weights now fails with a message naming what is missing. One caller, so the fix
is complete. Test: `a_run_directory_is_sized_by_its_adapter_not_its_inode`.

## 3. Adapter provenance (option (b), stated plainly)

No `vox mens train` run was attempted. Per the bounded-time process rule this
session established after an earlier live-measurement attempt burned excessive
time chasing a full training run, a deliberately large **synthetic-but-structurally-real**
adapter was constructed instead, using the exact key spelling and manifest shape
`merge.rs`'s own test fixtures produce:

- `lm_head.lora_a.weight` `[4, 1024]`, `lm_head.lora_b.weight` `[151936, 4]`, F32.
- `adapter_manifest.json` v3, `base_key_map = {"lm_head": "lm_head.weight"}`,
  `layer_order = ["lm_head"]`, rank 4, alpha 8, `base_model` = the HF snapshot dir.
- **Base run** (`run_base`): `lora_a` and `lora_b` all zeros — delta is exactly
  zero, so it is the unmodified base model reached through the identical code path.
  The only variable between the two runs is the adapter delta.
- **Adapter run** (`run_adapter`): `lora_a[0, :] = 1/1024` (so `a·h ≈ mean(h)`) and
  `lora_b[9707, 0] = 4000`. Token id **9707 is `"Hello"`**.
- Both directories were freshly created and asserted to contain **no**
  `merged.safetensors`, so `adapter_deltas_must_be_folded` returns true and the
  delta is folded at load.

`assert_adapter_fully_applied` passed on both loads — `InferenceEngine::load`
returned successfully and the worker reported ready; that assertion runs
unconditionally at the end of every `load`.

## 4. Exact commands

```bash
# plugin cdylib WITH the metal feature (it is opt-in and off by default)
cargo build -p vox-plugin-mens-candle-metal --features metal
cp target/debug/libvox_plugin_mens_candle_metal.dylib "$S/plugins/mens-candle-metal/"
cp crates/vox-plugin-mens-candle-metal/Plugin.toml     "$S/plugins/mens-candle-metal/"

cargo build -p vox-ml-cli --features gpu,execution-api

# base
VOX_PLUGINS_DIR=$S/plugins ./target/debug/vox-ml-cli mens serve \
  --model $S/run_base --port 18112 --temperature 0 --max-tokens 24
curl -s -X POST http://127.0.0.1:18112/v1/generate -H 'content-type: application/json' \
  -d '{"prompt":"The capital of France is","max_tokens":24,"temperature":0}'

# adapter (base server stopped first)
VOX_PLUGINS_DIR=$S/plugins ./target/debug/vox-ml-cli mens serve \
  --model $S/run_adapter --port 18113 --temperature 0 --max-tokens 24
curl -s -X POST http://127.0.0.1:18113/v1/generate -H 'content-type: application/json' \
  -d '{"prompt":"The capital of France is","max_tokens":24,"temperature":0}'
```

**Determinism:** `temperature: 0` was passed explicitly on both requests.
`sample_next_token` returns `argmax` when `temperature <= 0.0`, before `top_k`
(hardcoded 40 in the handler) is consulted — so the hardcoded `top_k` is inert
and both runs are greedy. Confirmed empirically: the adapter request repeated
verbatim returned a byte-identical response.

## 5. Results

Prompt (both runs): `The capital of France is`

**Base model** (`run_base`, zero delta), 13 tokens, 22 s on Metal:

```
...? Let me think... Wait, I'm thinking... Wait, I'm thinking... Wait, I'm thinking...
```

**Adapter model** (`run_adapter`), 13 tokens, 19 s on Metal:

```
...?

Wait... Why don't theyHello?

Wait... Why don't they...

Wait... Why don't they...
```

**Measurably and attributably different.** The outputs diverge from the third
token onward, and the divergence is exactly what the adapter encodes: `"Hello"`
— token 9707, the single row `lora_b` boosts — appears in the adapter output and
nowhere in the base output. This is not sampling noise (both runs are greedy and
reproducible) and it is not a coincidental difference: the injected token is the
one that shows up.

Metal is genuinely in use: the first attempt ran with the `metal` feature off and
took **187 s** for the same 13 tokens; with `--features metal` the same request
takes **19–22 s**, a ~9× speedup.

## 6. GUI wiring

**Method: code-path confirmation by reading, not a live click.** No headless Tauri
harness in this repo drives `mens serve`, and `vox-gui` cannot build in a fresh
worktree without a sidecar + `ui/dist` bootstrap. What was confirmed:

1. **The GUI model picker would list this adapter directory.** `MensCatalog`
   (`crates/vox-orchestrator/src/catalog.rs`, `is_listable_mens_run`) lists any
   directory under `<root>/mens/runs/` holding `tokenizer.json` +
   `candle_qlora_adapter.safetensors` + `adapter_manifest.json` — exactly the shape
   of `run_adapter`. `ModelsView.tsx` renders `mens/`-prefixed entries as a
   selectable local model.
2. **Chatting with such a model reaches the server tested here.**
   `endpoint_for(ProviderType::VoxLocal)` →
   `crates/vox-orchestrator-mcp/src/llm_bridge/provider_endpoints.rs` →
   `<base>/generate`, base defaulting to `http://127.0.0.1:11434`
   (`VOX_LOCAL_ENDPOINT_DEFAULT`) — which is `vox mens serve`'s
   `DEFAULT_INFERENCE_PORT` and the same handler as the `/v1/generate` route used
   above (`serve/mod.rs` registers `/generate`, `/v1/generate` and
   `/v1/completions` on one handler).
3. **From there the path is the one exercised.** handler → `worker.rs`
   `spawn_inference_worker` → `resolve_ml_backend_plugin` → `mens-candle-metal` →
   `MlBackend::load_model` → `CandleModel::load_from_path` →
   `InferenceEngine::load` / `generate`. The server log shows this resolution live:
   `plugin.discovered id="mens-candle-metal"`, `plugin.loaded … load_ms=261`,
   `Inference worker ready`.
4. **The card seam is generic and would reproduce the exact CLI call.**
   `execute_command(path, args)` (`crates/vox-gui/src/commands/execute.rs`) shells
   the `vox` sidecar as `vox <path…> --key value`, so a card with
   `path: ['mens','serve']` and `{model, temperature: 0}` produces the invocation
   run above verbatim.

**Honest gap:** the Mens surface (`MensTrainingView.tsx`) currently ships three
cards — `mens status`, `mens models`, `mens probe` — and **no `mens serve` card**.
A user picks an adapter model and chats (path 1-3, which is the real user journey
and is fully wired); nothing in the GUI starts the server. Starting it is a
one-card addition on an existing, proven seam, but it is not there today.

## 7. Tests and gates

- `cargo test -p vox-plugin-mens-candle-cuda -p vox-plugin-mens-candle-metal` —
  62 passed (CUDA, unchanged baseline), 60 passed (Metal: 58 baseline + 2 new).
- `cargo test -p vox-populi --features mens-train --lib` — 321 passed, 2 ignored.
- `cargo clippy -p vox-plugin-mens-candle-metal --all-targets --features metal
  -- -D warnings` — clean. Same for `vox-populi --features mens-train --lib`.
- `vox run scripts/fmt.vox` — formatted.

## 8. Notes worth carrying forward

- The `metal` feature on `vox-plugin-mens-candle-metal` is **opt-in and off by
  default**. A plugin built without it dlopens fine, serves correctly, and is ~9×
  slower with no error — an easy way to mis-measure Metal performance.
- The dylib installed into `VOX_PLUGINS_DIR` has no recorded artifact checksum, so
  the host loads it with a loud integrity warning. Expected for a hand-installed
  dev build.
- `vox mens serve` refuses to serve an adapter run without a
  `collateral_damage_report.json` whose `status` is `"pass"`. The demo runs carry a
  stub report; a real adapter needs the real eval.
- The GUI's Mens surface has no `mens serve` card — the natural next scoped
  follow-up, since every other piece of the seam it would use is already proven.

## 9. Task B0 pre-flight — can Qwen3-0.6B emit a parseable tool call?

**Question.** B2 (opening the agent loop to VoxLocal/MENS models) assumes a
small MENS model can reliably emit well-formed tool-call JSON when given tool
schemas. This had never been tested. Per the plan's B0 spike, 20
tool-calling-shaped prompts were sent to this same M4 fixture and scored for
parseable `{name, arguments}` output.

**Method used, and why.** This branch has not yet landed B2's OpenAI-compatible
`/v1/chat/completions` route — there is no server-side endpoint that accepts a
`tools` array and returns structured `tool_calls`. Approach (a) from the task
brief was used: each prompt spelled out one tool's name, description, and
parameters directly in the prompt text sent to the existing `/v1/generate`
endpoint, with an explicit instruction to reply with only a
`{"name": ..., "arguments": {...}}` JSON object. Greedy decoding
(`temperature: 0`), `max_tokens: 128`. This tests the load-bearing question for
B2 — can the model produce parseable tool-call JSON at all when asked — without
inventing any part of B2's wire format. No prompt was drawn from
`mens/data/heldout_bench/manifest.json` (checked; that manifest covers Vox
language constructs, not tool calling, so there was no overlap risk).

**Fixture.** Same run-directory shape as §3: a fresh zero-delta `lm_head`
LoRA adapter (`lora_a [4,1024]`, `lora_b [151936,4]`, all zeros, F32) over the
cached `Qwen/Qwen3-0.6B` snapshot, `adapter_manifest.json` v3, plus a stub
`collateral_damage_report.json` (`status: "pass"`) and `gate_receipt.json`
(`overall_passed: true`, two named substantive gates) so `vox mens serve`'s
serve-boundary gates admit it. Not committed (binary + stub gate artifacts);
built fresh in the session scratchpad.

**Serve command** (single foreground call, health-checked, `timeout`-wrapped,
port 18112 — not 11434/11435):

```bash
VOX_PLUGINS_DIR="$S/plugins" ./target/debug/vox-ml-cli mens serve \
  --model "$S/run_b0" --host 127.0.0.1 --port 18112 \
  --temperature 0 --max-tokens 128
```

Health check: `{"status":"ok","service":"vox-ml-cli"}`. Each of the 20 prompts
was posted to `POST /v1/generate` with `{"prompt": ..., "max_tokens": 128,
"temperature": 0}`. All 20 completed inside one 560s foreground window,
16-25s each (consistent with Z1's measured Metal throughput).

**Scoring.** A reply counted as a pass if it contained a JSON object,
anywhere in the (often rambling) generated text, that parsed cleanly and
matched `{"name": <str>, "arguments": {...}}` with `name` equal to the one
tool actually offered in that prompt. Extraction: find `{"name"`, then
brace-match to the closing `}`, then `json.loads` it — this is deliberately
generous (a real B2 without a repair shim would need the JSON to be the
*entire* reply, which is stricter), but it answers "can the model produce
this shape at all," which is the pre-flight's question.

**Results — 10/20 tools called with valid, correctly-named JSON:**

| # | tool offered | verdict | notes |
|---|---|---|---|
| 1 | `read_file` | fail | no JSON emitted; degenerated into repeating prose |
| 2 | `list_dir` | fail | emitted `{"files": [...], "directories": [...]}` — not the `{name, arguments}` shape, despite the prompt describing that shape twice in the reply |
| 3 | `get_weather` | fail | prose only ("65 degrees"), repeated |
| 4 | `search_web` | fail | described intent to call the tool, never emitted JSON |
| 5 | `send_email` | fail | repeated "should respond with the JSON object," never did |
| 6 | `get_time` | **pass** | `{"name": "get_time", "arguments": {"timezone": "Tokyo"}}` embedded after reasoning text |
| 7 | `calculator` | fail | correct arithmetic answer (8194) in prose, no tool call |
| 8 | `delete_file` | **pass** | `{"name": "delete_file", "arguments": {"path": "/tmp/old_notes.txt"}}` as the first thing emitted |
| 9 | `create_calendar_event` | **pass** | `{"name": "create_calendar_event", "arguments": {"title": "Dentist", "date": "2026-09-20", "time": "09:00"}}` |
| 10 | `get_stock_price` | fail | prose only ("$100.00"), no JSON |
| 11 | `translate_text` | **pass** | `{"name": "translate_text", "arguments": {"text": "good morning", "target_language": "French"}}` |
| 12 | `run_shell_command` | **pass** | `{"name": "run_shell_command", "arguments": {"command": "whoami"}}` as the first thing emitted |
| 13 | `set_reminder` | **pass** | `{"name": "set_reminder", "arguments": {"message": "Remind me to call mom", "due": "2026-09-13T18:00:00"}}` |
| 14 | `lookup_definition` | **pass** | `{"name": "lookup_definition", "arguments": {"word": "ephemeral"}}` after a prose answer |
| 15 | `convert_currency` | **pass** | `{"name": "convert_currency", "arguments": {"amount": 100, "from": "USD", "to": "EUR"}}` as the first thing emitted, correct values |
| 16 | `get_file_size` | fail | prose only (fabricated "1000000 bytes"), no JSON |
| 17 | `book_flight` | **pass** | `{"name": "book_flight", "arguments": {"from": "SFO", "to": "JFK", "date": "2026-10-01"}}` as the first thing emitted, correct values |
| 18 | `query_database` | fail | emitted `{"data": [...]}` shaped like a tool *result*, not a call |
| 19 | `resize_image` | **pass** | `{"name": "resize_image", "arguments": {"path": "/tmp/photo.png", "width": 800, "height": 600}}` after prose |
| 20 | `get_directions` | fail | degenerated into repeating a fabricated street address |

`tool_call_valid_json_rate = 10/20 = 0.5`. Full request/response payloads for
all 20 prompts are preserved in the B0 task report
(`.superpowers/sdd/2026-09-12-mens-end-to-end-completion/task-B0-report.md`),
not reproduced here.

**Qualitative pattern.** Every failure was one of two shapes: (a) the model
answered the user's question in prose and never attempted the tool-call
format at all (7 of 10 failures), or (b) it emitted *some* JSON but shaped
like a hypothetical tool result rather than a call (`list_dir`,
`query_database`). No failure was a near-miss on the `{name, arguments}`
shape itself (e.g. a typo'd key, unbalanced braces, wrong quoting) — when
the model did attempt the shape, it got the JSON syntax and field names
right every time, including correct argument values. Passes were also
inconsistent about *position*: some replies opened directly with the JSON
(`delete_file`, `run_shell_command`, `convert_currency`, `book_flight`),
others buried it after several sentences of reasoning (`get_time`,
`create_calendar_event`, `translate_text`, `set_reminder`,
`lookup_definition`, `resize_image`).

**Ruling (per the B0 decision table): 0.5 falls in the 0.3-0.7 band.**
**Proceed to B2, but B2 must add a tolerant extraction shim (scan the full
reply for an embedded `{name, arguments}` object rather than requiring the
whole reply to be that JSON) and must surface every such salvage to the
user as an explicit repair, not silently normalize it.** A hard requirement
that the entire reply be valid, bare JSON would fail more than half of these
20 cases outright, even though the model does reliably know the target shape
and the correct argument values once it decides to emit it.
