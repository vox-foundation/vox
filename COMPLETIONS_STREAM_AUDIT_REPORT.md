# Audit: removal of `POST /v1/completions/stream`

## Verdict

**Deliberate, root-caused removal — under-disclosed in docs, now fixed. Not a regression.**

## What happened

Commit `4bd416eb0` (`fix(vox-ml-cli): delete the dead /v1/completions/stream route`,
2026-09-11, this session's history) removed:

- `handlers::do_completions_stream` and its SSE plumbing (`handlers.rs`)
- the route registration in `mod.rs`
- the now-unused `stream_tx` field on `InferenceRequest` (`worker.rs`)

The commit message documents root-causing the bug, not just deleting the symptom:
`do_completions_stream` attached a `stream_tx` to the `InferenceRequest`, but the
worker loop (`worker.rs`) only ever replies on `req.reply` — the `oneshot` receiver
paired with `stream_tx` was dropped immediately at construction
(`handlers.rs:229`'s `oneshot::channel()` tuple discarded the `rx`). The route
therefore returned `200` with an SSE stream that emitted keep-alives forever and
never a single token — worse than a `404`, because a caller can't distinguish
"not implemented" from "hung."

The commit's own message includes the exact grep it ran to check for callers:

```
rg -n 'completions/stream' --glob '!target' .
```

which at the time (and confirmed again in this audit) returned only the route's own
registration and the plan doc describing the bug — no client, no test.

## Was this part of a disclosed larger effort?

The `4bd416eb0` commit sits in the same serve-cleanup sequence as:
- `f36386ae3` — fix(serve): stop discarding the system prompt, top_k and output_mode
- `a730ba1cf` — fix(vox-ml-cli): update serve_config_defaults test to match intentional port
- `42ceed7a7` — fix(vox-ml-cli): make /ready reflect actual worker model-load state

All four are `vox-ml-cli`/serve correctness fixes from the same session ("OOM
reporting / serving cleanup" per the task brief). The streaming-route deletion is
commit-message-disclosed (full root cause, explicit "decided to delete rather than
implement" rationale, explicit no-caller grep) — but the **doc-facing** SSOT
(`docs/src/reference/mens-serving-ssot.md`) was updated elsewhere in this session's
diff to correct the `/api/generate` claim without mentioning that a *different*,
dedicated streaming route (`/v1/completions/stream`) had existed and was removed.
That is the actual gap this audit was asked to check for, and it was real: the doc's
route list, as it stood, could read as "there was never a streaming route" — which
is not what happened.

## Real-caller check (Step 3)

Grepped for any current caller that would notice the removal:

```
rg -n "completions/stream" -g '!target' .          # zero hits anywhere in the tree
```

Checked every consumer of `POPULI_URL` (`vox-gamify::ai`, `vox-orchestrator-mcp`,
`vox-config::inference`, etc.) for SSE/streaming semantics against **this specific
server**:

- `vox-gamify`'s `FreeAiClient` (the orchestrator's local-Ollama lane) calls
  `POST …/api/generate` — a route this server has never implemented (already
  documented) — and has no streaming code path at all.
- `vox-orchestrator-mcp::chat_tools::chat::agent_loop`'s `stream_tokens` /
  `ttft_ms` machinery streams through the model-agnostic
  `vox_actor_runtime::llm` facade (`llm_chat`/`llm_stream`), which dispatches to
  whichever `ModelRegistryEntry` is selected (Claude, OpenAI, etc.) — it does not
  target `vox mens serve`'s Axum routes and was unaffected by this deletion.

No real, current caller of `/v1/completions/stream` exists anywhere in this repo.
This confirms the commit's own claim and rules out a hidden regression.

## Action taken

Per the task's decision tree (deliberate + no real regression + real doc gap ⇒ fix
docs only, no code changes):

1. **`docs/src/reference/mens-serving-ssot.md`** — added a "No streaming route"
   paragraph to the route list stating: the route existed, why it was broken, when
   and in what commit it was removed, why deletion (not a minimal fix) was chosen,
   and that there is currently no chunked/SSE alternative — `/v1/completions` and
   `/generate` are both single non-streaming JSON responses.
2. **`CHANGELOG.md`** — added a `### Fixed` entry under `[Unreleased]` describing
   the removed route and its symptom (hung `200` SSE stream), so anyone reading the
   changelog independently of the SSOT doc also sees this was a deliberate removal,
   not silent scope loss.

No source files were touched (the removal itself was already correct and complete
in `4bd416eb0`; nothing in the codebase depends on the deleted route).

## Test / verification summary

- `cargo test -p vox-ml-cli --features gpu,execution-api --lib -- serve` —
  **21 passed, 0 failed** (all `commands::ai::serve::*` tests, including
  `serve_config_defaults`, `serve_fails_on_missing_model`, worker payload tests,
  and the `/ready` state-machine test).
- `cargo run -p vox-doc-pipeline -- --lint-only --paths reference/mens-serving-ssot.md`
  (run from `docs/src/`) — **no hard errors**.
- No `.rs` files changed, so `cargo clippy` was not required; skipped.

## Status

**DONE** — deliberate root-caused removal, confirmed no real caller, doc gap closed
in `mens-serving-ssot.md` and `CHANGELOG.md`.
