---
title: "Axis Chat E2E — OpenRouter + Mac Metal Qwen + Drive Event Dump"
description: "Prove Axis Drive chat end-to-end on OpenRouter and a Mac Metal Qwen train→serve pack, with a full bounded stream/event dump for debugging. Revised after six-track codebase audit."
category: "architecture"
status: "design"
date: 2026-09-09
revised: 2026-09-09
---

# Axis Chat E2E Design (OpenRouter + Mac Metal + Drive Events)

## 0. Audit revision (2026-09-09)

Six parallel codebase audits (Drive events, Metal train/serve, OpenRouter proof, CLI wait/contracts, acceptance FP/FN, VoxScript/slug) found systemic false greens and missing blockers. This revision incorporates them. Do **not** implement the pre-audit plan without these corrections.

| Severity | Finding | Correction in this spec |
|---|---|---|
| **P0** | `wait --until reply` matches when `last_error` is set (`client.rs`) | Add `wait --until reply_ok`; scripts must never treat bare `reply` as success |
| **P0** | Metal: removing CLI bail is **insufficient** — `backend_candle_qlora.rs` and `serve/worker.rs` hardcode `mens-candle-cuda` | Train + serve must host-resolve `mens-candle-metal` (mirror `eval_local` / `ML_BACKEND_CANDIDATES`) |
| **P0** | `vox gui drive state` returns **empty catalog** (`driveVerbNeedsCatalog` only `set`/`send`) | Proof scripts must load catalog via `set`; extend catalog rows with provider fields |
| **P0** | Token text is on `frame.kind.text`, not `frame.text` | Capture mapping must use `kind.text` |
| **P1** | Qwen3.5 family often ships multimodal (`ForConditionalGeneration` / `vision_config`) — text QLoRA rejects in `vox-hf-layout` | Task 0 spike; fallback to operator-approved text-only Mac-tier if 0.8B rejects |
| **P1** | Serve QLoRA dirs require `collateral_damage_report.json` with `"status":"pass"` | Pipeline must produce or explicitly exempt |
| **P1** | `MIN_CORPUS_PAIRS = 100` + workspace `mens/config/training_contract.yaml` hijacks `--data-dir` | E2e uses per-run contract under `--data-dir`; no tiny `train.jsonl` that trips the gate |
| **P1** | `--max-steps` flag does not exist | Use `--epochs 1` + `--max-runtime-secs` |
| **P1** | MensCatalog may not list QLoRA run dirs (expects `final` / `checkpoint-*` dirs) | Pin selectability via VoxLocal stem match + serve, or fix catalog heuristic |
| **P1** | Metal plugin hub download stubbed; `gpu` feature required for mens train/serve | Pre-download base weights; use GPU-enabled CLI; document in how-to |
| **P2** | Global agent bus pollutes Drive ring; clear-on-send races late tokens | `active_turn_id` ref + session_id filter; ignore unrelated frames |
| **P2** | TS-only `DriveState` drift vs Rust `protocol.rs` | Both planes must grow the same fields + `claims.events` |

## 1. Problem

Axis Drive can start, set knobs, send, and surface some `last_error` / bubbles, but operators and agents are not convinced chat works end to end because:

1. **OpenRouter success through Drive** has not been proven as an acceptance gate.
2. **Local success through Drive** failed on `mens/e2e-smoke` with `load tokenizer` — a pack/asset failure, not a silent transport bug — and there is no documented Mac GPU train→serve path that Drive can pin.
3. **Debugging is incomplete**: Drive does not retain the full stream / orchestrator event trail for a turn, so agents cannot parse “all outputs” after a send.
4. **Existing wait semantics lie**: `wait --until reply` succeeds on errors, so a naive e2e script can false-green.

Locked product decision (brainstorming 2026-09-09): success means **both** green inference legs **and** full parseable event dumps (Approach 1 — extend Drive first, then prove providers).

## 2. Locked decisions

| Decision | Choice |
|---|---|
| Success criteria | **C** — green OpenRouter reply + green local reply + full event dump on every path |
| Local acceptance model | **Train/serve a Mac Metal Qwen pack** (not repair smoke) |
| Mac pipeline depth | **Documented download → train → serve** operators can re-run; Drive proof is the exit gate |
| OpenRouter acceptance | **Any selectable OpenRouter-backed catalog entry** that returns a short reply (prefer `openrouter/auto` or `openrouter/*`) |
| Output capture | **Full stream/event dump** — every streamed chunk / orch event available via Drive `state` |
| Control plane | **Live Axis Drive only** for acceptance (same path as the human composer). Headless may assist unit tests but cannot claim e2e |
| Mac base model | **Primary:** `Qwen/Qwen3.5-0.8B` (operator “3.8 Qwen” = this 0.8B Mac-tier ladder step). **Spike-gated:** if `vox-hf-layout` multimodal reject fires, fall back to an operator-approved **text-only** Mac-tier id recorded in the how-to (do not silently repair smoke) |
| Local Drive slug | **`mens/qwen35-08b-metal-e2e`** — run dir **must** be `mens/runs/qwen35-08b-metal-e2e` so serve stem matches |
| Metal training | Remove CLI dead gate **and** host-aware QLoRA dispatch **and** Metal serve worker — not bail removal alone |
| Wait success | **`reply_ok`** = assistant bubble present, non-error, **and** `last_error == null`. Bare `reply` remains “turn settled” (may be error) |

## 3. Non-goals

- Repairing `mens/e2e-smoke` as the acceptance pack
- Attaching to the user’s everyday Axis window
- Full SP3-D streaming train-step protocol (`run_train_step` / `run_eval_step` stubs stay stubs — **`run_full_training` is the e2e path**)
- CUDA training on Mac; NVIDIA remains a separate machine path
- Unbounded event retention or shipping raw secrets in Drive JSON
- Making OpenRouter slug a fixed regression pin (any selectable OpenRouter-backed row is enough)
- Changing `vox chat` raw LLM CLI (out of Axis scope)
- Optional `keep_events` knob in v1 (YAGNI — default clear-on-send only)

## 4. Relationship to existing surfaces

| Surface | Role after this spec |
|---|---|
| [Axis Drive design](2026-09-08-axis-drive-design.md) | Parent control plane; this spec extends `DriveState` + wait predicates + Mac acceptance |
| `contracts/gui/axis-drive.v1.yaml` | Additive v1: `wait_until`, `state_snapshot` docs; no v2 rename |
| `App.tsx` / `listenAgentEvents` | Existing UI listener; **Drive must subscribe separately** in `AxisDriveHost` (App does not feed Drive) |
| `handleLoquelaSubmit` / `interpretDriveSubmit` | Still the send path; populate bubbles + `last_error` + `submit_*` events |
| `vox mens train --backend qlora` | Metal path becomes real via plugin `run_full_training` |
| `vox-populi` `backend_candle_qlora.rs` | Must stop hardcoding `mens-candle-cuda`; resolve Metal on Apple Silicon |
| `vox-ml-cli` `serve/worker.rs` | Must resolve Metal plugin for Mac inference |
| `vox-plugin-mens-candle-metal` | Host for Metal QLoRA `run_full_training` + inference |
| `mens/e2e-smoke` | Fixture only; **not** acceptance pack |

## 5. Architecture

```text
Track 0 — Spike (blocking Metal)
  plugin load → config parse for Qwen/Qwen3.5-0.8B → run_full_training micro-run
  → record model id or text-only fallback

Track A — Drive observability
  submit_* + listenAgentEvents (kind.text) → capped ring → state.events
  → wait --until event=<kind> | reply_ok

Track B — Mac Metal Qwen pipeline
  install mens-candle-metal → download → train (host-aware dispatch)
  → collateral report → serve (Metal worker) → Drive pin

Track C — OpenRouter proof
  set (loads catalog) → pick openrouter/* → send → reply_ok + events
```

### 5.1 Drive turn event model

Add to live `DriveState` (**TypeScript and Rust `protocol.rs`**):

```ts
export interface DriveTurnEvent {
  seq: number;
  ts_ms: number;
  kind: string;           // token_streamed, task_started, submit_ok, submit_err, …
  turn_id: string;
  text?: string;          // truncated + redacted
  raw?: unknown;          // size-capped JSON after redaction
}

// on DriveState:
events: DriveTurnEvent[];
events_dropped: number;
last_turn_id: string | null;
next_seq: number;         // monotonic across clears; never reconstruct from events[]
```

| Knob | Value |
|---|---|
| Ring capacity | **500** |
| Per-event `text` cap | **4 KiB** |
| Per-event `raw` cap | **8 KiB** serialized |
| Retention | Cleared at **start** of each `send` |
| Redaction | Strip Bearer / `sk-…` from `text` **and** stringified `raw` |
| Correlation | Host keeps `activeTurnIdRef` set at send start; listener uses that ref (not `last_turn_id`, which updates on every append) |
| Filter | Prefer frames whose `kind.session_id` matches Drive chat session; drop unrelated orch noise when a turn is active |

`DriveClaims` gains **`events: boolean`**: live `true`, headless **`false` (explicit)**.

`DriveCatalogRow` gains optional **`provider`** and **`provider_type`** mirrored from picker cards so scripts can select OpenRouter without guessing from id alone.

### 5.2 Capture points

1. **Submit envelope** — mint `turn_id`, clear ring, await submit, append `submit_ok` / `submit_err`.
2. **Agent event bus** — `AxisDriveHost` calls `listenAgentEvents` when mounted (do **not** depend on App’s listener). Map:
   - `kind = frame.kind?.type ?? 'agent_event'`
   - `text = typeof frame.kind?.text === 'string' ? frame.kind.text : undefined`
3. Sync OpenRouter **often** emits `token_streamed` with `session_id`, but empty-stream → `llm_chat` fallback can emit **zero** tokens. Acceptance requires `submit_ok` + non-empty assistant bubble; stream frames are best-effort enrichment.

CLI:

- `vox gui drive state` includes `events` + `events_dropped` + catalog (see §5.4 catalog load).
- `wait --until reply` — turn settled (error **or** assistant). **Do not use for green acceptance.**
- `wait --until reply_ok` — `last_error == null` **and** last bubble `role == assistant` **and** `error !== true` **and** non-empty content.
- `wait --until event=<kind>` — any `state.events[].kind` match.
- `wait --until error` — unchanged.

### 5.3 Mac Metal train → serve

Blockers today (all in scope):

1. CLI dead gate in `run_train.rs` (macOS Candle QLoRA + Metal).
2. `backend_candle_qlora.rs` always loads `mens-candle-cuda`.
3. `serve/worker.rs` always loads `mens-candle-cuda`.
4. No `ensure_metal_plugin` heal (CUDA has `ensure_cuda_plugin`).
5. Serve collateral-damage gate for adapter run dirs.
6. Possible multimodal reject for `Qwen/Qwen3.5-0.8B` at `vox-hf-layout`.
7. Workspace training contract + `MIN_CORPUS_PAIRS`.

Required pipeline:

1. `vox plugin install mens-candle-metal --yes` (catalog id **`mens-candle-metal`**).
2. Spike: micro `run_full_training` proves Metal path before removing the bail.
3. Host-aware train dispatch + remove dead gate.
4. Per-run data dir with local `training_contract.yaml` (do not rely on workspace `mens/config/training_contract.yaml` alone).
5. Output: `mens/runs/qwen35-08b-metal-e2e` containing adapter **and** `tokenizer.json`.
6. Collateral report with `"status":"pass"` (or documented e2e eval step that writes it).
7. Metal-aware serve: `vox mens serve --model mens/runs/qwen35-08b-metal-e2e` (directory; port may need non-11434 if Ollama occupies default).
8. Drive pin `mens/qwen35-08b-metal-e2e` with VoxLocal reachable + stem listed in `/v1/models`.

### 5.4 OpenRouter proof

1. `vox gui drive start` (session ready).
2. **`set --knob execution=sync`** (or any set) to force catalog IPC — bare `state` alone yields `catalog: []`.
3. Pick model: prefer selectable `openrouter/auto` or `id` starting with `openrouter/`; else first selectable row with `provider_type == "OpenRouter"` once catalog fields ship. Reject `mens/*` and tier ids (`auto`/`local`/`mesh`/`cloud` alone).
4. `set --knob model_override=<id>`.
5. `send --text 'Reply with one short line containing openrouter-ok'` (prompt only — **do not** require exact string match).
6. `wait --until reply_ok --timeout 120s`.
7. Assert: `plane == live`, `last_error == null`, last assistant bubble non-empty and `error !== true`, `events` contains `submit_ok` for `last_turn_id`, `knobs.model_override` equals chosen id.
8. Missing key: catalog `reason: key_missing` / non-selectable → script exit non-zero **before** claiming green (observability of failure is checkbox 4, not checkbox 1).

Requires `SecretId::OpenRouterApiKey` via secrets SSOT. Scripts must **not** `env.get("OPENROUTER_API_KEY")`.

## 6. Components

### 6.1 UI / Drive bus

| File | Responsibility |
|---|---|
| `crates/vox-gui/ui/src/lib/driveEvents.ts` | Ring helpers, caps, redaction |
| `crates/vox-gui/ui/src/lib/axisDrive.ts` | `DriveState` / claims / catalog row fields |
| `crates/vox-gui/ui/src/lib/useDriveBus.ts` | clear + submit_* ; catalog on state optional |
| `crates/vox-gui/ui/src/components/drive/AxisDriveHost.tsx` | `listenAgentEvents` → ring via `activeTurnIdRef` |
| `crates/vox-gui/src/drive/protocol.rs` | Rust mirror of claims/state fields |

### 6.2 CLI / contract

| File | Responsibility |
|---|---|
| `crates/vox-cli/src/commands/gui/client.rs` | `event=` + `reply_ok` matchers |
| `contracts/gui/axis-drive.v1.yaml` | Document wait predicates + snapshot fields (v1 additive) |
| `docs/src/reference/cli.md` + parent Drive design | Wait grammar |

### 6.3 Metal train / serve

| File | Responsibility |
|---|---|
| `crates/vox-ml-cli/.../run_train.rs` | Remove dead gate; metal heal |
| `crates/vox-ml-cli/.../plugin_heal.rs` | `ensure_metal_plugin` |
| `crates/vox-populi/.../backend_candle_qlora.rs` | Host-aware plugin id |
| `crates/vox-ml-cli/.../serve/worker.rs` | Host-aware inference plugin |
| `crates/vox-plugin-mens-candle-metal/**` | `run_full_training` (already wired) |
| `docs/src/how-to/how-to-train-mens-macos-metal.md` | Operator SSOT |
| `scripts/mens-macos-metal-e2e.vox` | download→train→collateral→serve handoff |

### 6.4 Proof scripts

| Artifact | Role |
|---|---|
| `scripts/axis-drive-openrouter-e2e.vox` | Track C — exit 0 only on `reply_ok` predicates |
| `scripts/axis-drive-metal-e2e.vox` | Track B∩Drive |
| Both | Nested `state` JSON parse; `process.exit(1)` on failure (never print-only) |

## 7. Error handling

| Failure | Drive / CLI surface |
|---|---|
| Missing OpenRouter key | Catalog `key_missing` / 409 on set; or `last_error` + `submit_err` |
| Metal plugin missing | Train error names `vox plugin install mens-candle-metal` |
| Multimodal config reject | Spike fails; how-to records fallback model |
| Tokenizer / weights missing | `last_error` + `submit_err` (smoke class) — **not** green |
| Serve CUDA plugin on Mac | Inference error — fix worker before Drive green |
| Collateral gate | Serve bail — pipeline must write report |
| Event ring overflow | `events_dropped++`; never OOM webview |
| Bare `wait --until reply` | May exit 0 on error — scripts must use `reply_ok` |

## 8. Testing

| Layer | What |
|---|---|
| Unit | Ring caps/drop/redact/`raw` cap; `reply_ok` vs `reply`; `event=` empty/flat negatives |
| Integration | Mock `listenAgentEvents` → host updates `stateRef.events` visible to state verb |
| Mutation | Delete `appendDriveEvent(submit_err)` / `listenAgentEvents` wiring → tests fail |
| Physical | OpenRouter + Metal Drive scripts green on live plane |
| Docs | How-to frontmatter; fenced `vox` compile or `// vox:skip` |

## 9. Acceptance checklist (anti-FP)

- [ ] **OpenRouter green:** live plane; `claims.bubbles`/`claims.events` true; chosen OpenRouter-backed id pinned; `wait --until reply_ok` succeeds; last assistant bubble non-empty and `error !== true`; `last_error` null; `events` contains `submit_ok` for `last_turn_id`; summary JSON printed.
- [ ] **Metal green:** Apple Silicon; plugin installed; spike-approved model trained to `mens/runs/qwen35-08b-metal-e2e` with tokenizer + adapter; collateral pass; serve `/v1/models` lists stem; Drive pin selectable / VoxLocal reachable; same bubble/`reply_ok`/`submit_ok` predicates as OpenRouter.
- [ ] **Events:** after each send, all retained events share `last_turn_id` (post-clear); caps + `events_dropped` behave; `raw` ≤ 8 KiB; secrets redacted.
- [ ] **Failure honesty (live):** missing key and broken local pack → exit non-zero; `last_error` set; `submit_err` present; **never** exit 0 on error bubbles.
- [ ] **Headless:** `claims.events === false` (explicit). Live: `claims.events === true`.
- [ ] **No silent wait FP:** physical scripts call `reply_ok`, not bare `reply`.

## 10. Implementation sequencing

1. **Track A** — events + `reply_ok` / `event=` (unlocks honest debugging).
2. **Track 0** — Metal spike (model + plugin) in parallel with Track A once ready.
3. **Track C** — OpenRouter physical proof after submit events + `reply_ok` (stream frames optional until host subscription lands).
4. **Track B** — host-aware train/serve + how-to + script.
5. **Track B∩Drive** — Metal physical Drive proof.

## 11. Open risks

| Risk | Mitigation |
|---|---|
| Multimodal reject on 0.8B | Spike-first; record fallback text-only id in how-to |
| Sync path emits few agent events | Require `submit_*` + bubble; stream best-effort |
| Long Metal train | `--epochs 1`, `--max-runtime-secs`, minimal corpus under per-run contract |
| Catalog heuristic miss | Prefer VoxLocal stem + `pin_policy` documented; optional MensCatalog fix |
| Port 11434 conflict with Ollama | Document alternate `--port` |
| CI does not run `vox-cli --features gui` tests | Local gate required; note CI follow-up |

## 12. Out-of-scope follow-ups

- Separate WebKit bundle id for Drive storage isolation
- SP3-D step streaming protocol
- Fixed OpenRouter regression slug in CI without secrets
- Automated nightly Metal train on GitHub-hosted runners
- Changing bare `reply` semantics globally (add `reply_ok` instead of breaking existing “settled” waits)
