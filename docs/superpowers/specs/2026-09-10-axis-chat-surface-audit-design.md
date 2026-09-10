---
title: "Axis Chat Surface Audit — Hop Trace + Reliability"
description: "Hop-gated ChatHop JSONL, TCP framing, Drive honesty, secrets/schema fixes, and OpenRouter+Metal acceptance on the shared chat_turn path."
category: "architecture"
status: "design"
date: 2026-09-10
---

# Axis Chat Surface Audit Design (Hop Trace + Reliability)

## 0. Relationship

Extends [2026-09-09-axis-chat-e2e-openrouter-metal-design.md](2026-09-09-axis-chat-e2e-openrouter-metal-design.md) (provider e2e + Drive event ring). That spec remains SSOT for Metal train/serve mechanics. **This** spec owns:

- Correlated ChatHop JSONL (dogfood Tier D)
- False-green / false-red elimination on the chat feed path
- TCP `orch.tool_call` framing guarantees
- Drive listener / `reply_ok` / send-exit honesty
- Secrets adopt parity and safe LegacySchema heal
- Catalog limit alignment

Implementation plan: Cursor plan `chat_surface_audit` (audit-revised); mirror under `docs/superpowers/plans/` at closeout if committed.

## 1. Problem

OpenRouter works on CLI `vox chat`, but Axis Drive sync chat fails with empty `orch.tool_call` frames, “reply still in progress”, `key_missing` vs doctor Present split-brain, schema 90 vs on-disk 93 pool empties, and Metal train that exits 0 without `gpu`. Debugging is fragmented: Drive event ring is client-local; orch telemetry marks failed tools as `success` via `Result::is_ok()`.

## 2. Locked decisions (brainstorming + six-track audit)

| Decision | Choice |
|---|---|
| Success criteria | Instrument then fix until OpenRouter **and** Metal green |
| Entry points | Drive e2e proves chat; Loquela shares `chat_turn` (Playwright ≠ acceptance) |
| Debug trail | Drive event ring **and** ChatHop JSONL |
| ChatHop writers | **Orch MCP only**; GUI passes `trace_id`/`turn_id` as tool args |
| ChatHop path | `dogfood_trace_path_for("chat_hops.jsonl")` / `VOX_DOGFOOD_TRACE_PATH` |
| Tool success | Envelope (`tool_json_envelope_is_error`), never bare `Result::is_ok()` |
| Empty TCP frame | Per-request `tokio::spawn` + panic → Error frame; not `catch_unwind` across await |
| Schema 93 | No fake `BASELINE_VERSION=93` without DDL; gated wipe + export/import |
| Secrets | Vault absolute path; Drive spawn keys; refuse foreign orch adopt without parity |
| Catalog | Drive `listModels` uses same limit as Loquela (`MODEL_LIST_LIMIT`) |

## 3. Six-track FP/FN table

| Track | Worst FP / FN | Correction |
|---|---|---|
| ChatHop / tracing | `success = result.is_ok()` | Envelope success; orch-only hops; wire `trace_id` |
| Empty tool_call | Empty = EOF (panic/kill) | Spawn boundary; timeout SSOT; respawn drain |
| Drive listen / reply_ok | Unstable `setters`; send exit 0 with `last_error` | Memoize setters; non-zero send; turn-scoped `reply_ok` |
| Schema 90 vs 93 | No 91–93 DDL in tree | Gate wipe; doctor export; no invent-baseline |
| Secrets / adopt | GUI key Present, orch empty | Force spawn or probe parity |
| OpenRouter / Metal | Catalog 120 FN; train without gpu exits 0 | Limit 2000; gpu+execution-api preflight |

## 4. Feed path

```text
Loquela or AxisDrive
  → handleLoquelaSubmit (sync inFlight claim)
  → chat_turn (trace_id, turn_id in sync_tool_args)
  → PersistentDaemon.ensure (refuse stale adopt without keys)
  → OrchDaemonClient.call(TOOL_CALL, vox_chat_message)
  → handle_connection: spawn-scoped handler → always write_frame
  → McpExtraDispatch → handle_tool_call_with_mode
  → ChatHop begin/end + turn_outcome
  → bubble or error → Drive reply_ok + submit_ok
```

## 5. ChatHop schema (v1)

JSONL line (redacted):

```json
{
  "schema_version": "chat_hop.v1",
  "hop_kind": "mcp_tool | chat_llm | turn_boundary | retrieval",
  "trace_id": "uuid",
  "turn_id": "uuid",
  "session_id": "string",
  "hop_seq": 0,
  "tool_name": "optional",
  "success": true,
  "failure_class": "optional",
  "turn_outcome": "ok | llm_error | iteration_limit | budget_denied | empty_frame | …",
  "duration_ms": 0,
  "timestamp_ms": 0
}
```

Non-goals for v1: merging ChatHop with `DriveTurnEvent`; writing hops from `vox-gui`; training mix via dead `trace_tools::ToolTraceRecord`.

## 6. Acceptance matrix

| Provider | Proof | Interactive |
|---|---|---|
| OpenRouter | `scripts/axis-drive-openrouter-e2e.vox` + failure-honesty | Same `chat_turn`; optional Drive `--show` |
| Metal | `mens-macos-metal-e2e.vox` (gpu preflight) then `axis-drive-metal-e2e.vox` | Same |

`reply_ok` = no `last_error`, last assistant bubble non-error non-empty, **and** `submit_ok` for `last_turn_id`. Never bare `reply`. Never treat `send` exit 0 alone as success.

## 7. Non-goals

- Inventing schema baseline 93 without authored DDL + policy digest
- `catch_unwind` around async `.await` as the panic boundary
- GUI-side ChatHop writers or new GUI→MCP tracing imports
- Playwright as OpenRouter/Metal acceptance
- Silent wipe of interactive repo `.vox/store.db` or canonical `vox.db`
- Unbounded secret material in JSONL / Drive state

## 8. Store map

| Surface | DB |
|---|---|
| Interactive Axis | Repo `.vox/store.db` via journey optional |
| Axis Drive | `~/.vox/gui-drive/<profile>/.vox/store.db` when `VOX_GUI_DRIVE_STORE_ROOT` set |
| Doctor schema check | Canonical user `vox.db` |
| ChatHop | Dogfood dir under `VOX_DOGFOOD_TRACE_PATH` (Tier D) |

## 9. Timeout SSOT

| Layer | Ceiling |
|---|---|
| Drive HTTP bridge `recv_timeout` | `vox_config::timeouts::D_180S` |
| MCP dispatch `vox_chat_message` | `CHAT_MESSAGE_TIMEOUT` = 180s (`dispatch_timeout.rs`) |
| Orch TCP client read | `ORCH_CLIENT_READ_DEADLINE` = `D_195S` (margin after dispatch) |

Do not lower the client deadline below the dispatch timeout or empty-frame EOFs return before Error frames.

Drive LegacySchema wipe requires `VOX_GUI_DRIVE=1` **and** `VOX_GUI_DRIVE_ALLOW_STORE_RESET=1`. Doctor prints `path`, `max_version`, and binary `baseline` and never auto-deletes interactive/canonical DBs.

## 9. Timeout SSOT

Named durations live in `crates/vox-config/src/timeouts.rs` (`D_180S`, `D_195S`, …). Drive live HTTP hop (`bridge.rs` `recv_timeout`) uses **`D_180S`** so sync `chat_turn` can outlive cold orch + LLM while `wait --until reply_ok` remains the long poll. Orch client read deadline is **`D_195S`** (`ORCH_CLIENT_READ_DEADLINE`). Do not reintroduce bare `Duration::from_secs(180)` at these call sites — `vox-drift-check` flags common timeout literals.
