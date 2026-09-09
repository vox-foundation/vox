---
title: "Axis Chat E2E — OpenRouter + Mac Metal Qwen + Drive Event Dump"
description: "Prove Axis Drive chat end-to-end on OpenRouter and a Mac Metal Qwen3.5-0.8B train→serve pack, with a full bounded stream/event dump for debugging."
category: "architecture"
status: "design"
date: 2026-09-09
---

# Axis Chat E2E Design (OpenRouter + Mac Metal + Drive Events)

## 1. Problem

Axis Drive can start, set knobs, send, and surface some `last_error` / bubbles, but operators and agents are not convinced chat works end to end because:

1. **OpenRouter success through Drive** has not been proven as an acceptance gate.
2. **Local success through Drive** failed on `mens/e2e-smoke` with `load tokenizer` — a pack/asset failure, not a silent transport bug — and there is no documented Mac GPU train→serve path that Drive can pin.
3. **Debugging is incomplete**: Drive does not retain the full stream / orchestrator event trail for a turn, so agents cannot parse “all outputs” after a send.

Locked product decision (brainstorming 2026-09-09): success means **both** green inference legs **and** full parseable event dumps (Approach 1 — extend Drive first, then prove providers).

## 2. Locked decisions

| Decision | Choice |
|---|---|
| Success criteria | **C** — green OpenRouter reply + green local reply + full event dump on every path |
| Local acceptance model | **Train/serve a Mac Metal Qwen pack** (not repair smoke) |
| Mac pipeline depth | **Documented download → train → serve** operators can re-run; Drive proof is the exit gate |
| OpenRouter acceptance | **Any selectable OpenRouter catalog entry** that returns a short reply |
| Output capture | **Full stream/event dump** — every streamed chunk / orch event available via Drive `state` and/or a dedicated dump verb |
| Control plane | **Live Axis Drive only** for acceptance (same path as the human composer). Headless may assist unit tests but cannot claim e2e |
| Mac base model | **`Qwen/Qwen3.5-0.8B`** — operator shorthand “3.8 Qwen” maps to this **0.8B** Mac-tier ladder step (VRAM-safe). Override via `--model` only when explicitly needed |
| Local Drive slug | **`mens/qwen35-08b-metal-e2e`** (run dir under `~/.vox/gui-drive/...` or documented `mens/runs/...`) |
| Metal training | **Must remove the current `vox mens train --device metal` dead gate** and route Candle QLoRA through `vox-plugin-mens-candle-metal` `run_full_training` |

## 3. Non-goals

- Repairing `mens/e2e-smoke` as the acceptance pack
- Attaching to the user’s everyday Axis window
- Full SP3-D streaming train-step protocol (`run_train_step` / `run_eval_step` stubs stay stubs)
- CUDA training on Mac; NVIDIA remains a separate machine path
- Unbounded event retention or shipping raw secrets in Drive JSON
- Making OpenRouter slug a fixed regression pin (any selectable cloud row is enough)
- Changing `vox chat` raw LLM CLI (out of Axis scope)

## 4. Relationship to existing surfaces

| Surface | Role after this spec |
|---|---|
| [Axis Drive design](2026-09-08-axis-drive-design.md) | Parent control plane; this spec extends `DriveState` + wait predicates + Mac acceptance |
| `contracts/gui/axis-drive.v1.yaml` | Add event-dump fields / optional `events` verb; bump carefully |
| `App.tsx` agent-event listen (`token_streamed`, etc.) | Source of truth for turn events; Drive mirrors a bounded ring |
| `handleLoquelaSubmit` / `interpretDriveSubmit` | Still the send path; must continue to populate bubbles + `last_error` |
| `vox mens train --backend qlora` | Mac Metal path becomes real; CLI dead gate in `run_train.rs` is removed once plugin path works |
| `vox-plugin-mens-candle-metal` | Host for Metal QLoRA `run_full_training` + inference serve |
| `vox mens serve` / `vox schola serve` | Serves the trained run dir for VoxLocal / Drive local pin |
| `mens/e2e-smoke` | Remains a fixture slug for unit tests; **not** the e2e acceptance pack |

## 5. Architecture

Three tracks, one acceptance gate:

```text
Track A — Drive observability
  agent-events / token_streamed / submit result
       → DriveTurnEvent ring (bounded)
       → state.events (+ optional dump verb)
       → wait --until reply|error|event=*

Track B — Mac Metal Qwen pipeline
  download Qwen/Qwen3.5-0.8B
       → mens train --backend qlora --device metal
       → serve run dir
       → Drive set model_override=mens/qwen35-08b-metal-e2e
       → send + wait reply

Track C — OpenRouter proof
  Drive catalog → any selectable OpenRouter id
       → set + send + wait reply
       → events dump non-empty on success and failure
```

Live Drive remains the only plane that may claim Axis chat e2e.

### 5.1 Drive turn event model

Add to live `DriveState`:

```ts
export interface DriveTurnEvent {
  seq: number;
  ts_ms: number;
  kind: string;           // e.g. token_streamed, task_started, submit_ok, submit_err
  turn_id: string;        // correlates one send
  text?: string;          // chunk or message body (may be truncated)
  raw?: unknown;          // optional structured frame (size-capped JSON)
}

// on DriveState:
events: DriveTurnEvent[];      // ring, newest last
events_dropped: number;        // how many fell off the ring
last_turn_id: string | null;
```

Defaults:

| Knob | Value |
|---|---|
| Ring capacity | **500** events |
| Per-event `text` cap | **4 KiB** |
| Per-event `raw` cap | **8 KiB** serialized |
| Retention | Cleared on next `send` (or kept with `keep_events=true` set key — optional; default clear) |
| Redaction | Strip Authorization / API key shaped strings before store |

`claims` gains `events: true` on live plane; headless must set `events: false`.

### 5.2 Capture points

1. **Submit envelope** — `interpretDriveSubmit` already yields ok/error; emit `submit_ok` / `submit_err`.
2. **Agent event bus** — same listener App uses for `vox://agent-events` (including `token_streamed`); Drive host appends every frame for the active Drive session/turn.
3. **Preview / IPC failures** — optional `diag` kinds; not required for chat acceptance.

CLI:

- `vox gui drive state` includes `events` + `events_dropped`.
- `vox gui drive wait --until reply|error` unchanged semantically; add `--until event=<kind>` when useful.
- Optional: `vox gui drive events --turn <id>|--last` prints JSONL for debugging (may be CLI-only polling of `state`).

### 5.3 Mac Metal train → serve

Today `vox-ml-cli` **unconditionally bails** on `--device metal` for Candle QLoRA even though `vox-plugin-mens-candle-metal` already implements device select + `run_full_training`. This spec requires:

1. Remove/replace that dead gate with a path that loads the Metal plugin and calls `run_full_training`.
2. Document operator steps in a how-to under `docs/src/how-to/` (Mac GPU, Apple Silicon).
3. Prefer a committed `.vox` automation script under `scripts/` (VoxScript-first) that:
   - downloads the base model,
   - trains a short dogfood dataset,
   - writes artifacts to a stable run dir,
   - starts serve (or prints the serve command),
   - prints the Drive pin slug.

Minimal train acceptance: one short supervised ChatML example is enough if the served pack answers one Drive prompt. Prefer a tiny in-repo dogfood JSONL over a large corpus.

Serve must register a catalog-visible local id (or VoxLocal `/generate` model name) that Drive can `set` as `model_override=mens/qwen35-08b-metal-e2e`.

### 5.4 OpenRouter proof

1. `vox gui drive start` (session ready).
2. `state` → pick first catalog row with OpenRouter provider / selectable.
3. `set --knob model_override=<id>`.
4. `send --text 'Reply with exactly: openrouter-ok'`.
5. `wait --until reply` within timeout; assert assistant bubble contains non-empty text and `last_error` is null.
6. Assert `events` contains at least one non-submit event **or** document that sync OpenRouter path only emits submit_* (then require submit_ok + bubble text).

Requires OpenRouter key present via secrets SSOT (`vox_secrets::resolve_secret`); proof fails closed with a clear Drive error if missing — that is still a successful observability demo if events/`last_error` are correct, but **acceptance green** requires a real reply.

## 6. Components

### 6.1 UI / Drive bus

| File | Responsibility |
|---|---|
| `crates/vox-gui/ui/src/lib/axisDrive.ts` | `DriveTurnEvent`, state fields, helpers |
| `crates/vox-gui/ui/src/lib/useDriveBus.ts` | append/clear ring; wire into send |
| `crates/vox-gui/ui/src/components/drive/AxisDriveHost.tsx` | subscribe to agent events → ring |
| `crates/vox-gui/ui/src/lib/useDriveBus.test.ts` / `axisDrive` tests | ring caps, clear-on-send, redaction |

### 6.2 CLI

| File | Responsibility |
|---|---|
| `crates/vox-cli/src/commands/gui/client.rs` | `wait --until event=` parsing under nested `state` |
| `crates/vox-cli/src/commands/gui/drive.rs` | optional `events` dump subcommand |
| `contracts/gui/axis-drive.v1.yaml` | document new fields / verbs |

### 6.3 Metal train path

| File | Responsibility |
|---|---|
| `crates/vox-ml-cli/src/commands/schola/train/run_train.rs` | remove metal dead gate; dispatch plugin |
| `crates/vox-plugin-mens-candle-metal/**` | ensure `run_full_training` works for short runs on Apple Silicon |
| `docs/src/how-to/how-to-train-mens-macos-metal.md` | operator SSOT |
| `scripts/mens-macos-metal-e2e.vox` | download → train → serve handoff |

### 6.4 Proof scripts (operator / agent)

| Artifact | Role |
|---|---|
| `scripts/axis-drive-openrouter-e2e.vox` | Track C |
| `scripts/axis-drive-metal-e2e.vox` | Track B after serve is up |
| Both print parseable JSON summaries (status, model, last_error, event kinds) |

## 7. Error handling

| Failure | Drive surface |
|---|---|
| Missing OpenRouter key | `last_error` + `submit_err` event; no false-empty success |
| Metal train dead gate / plugin missing | CLI train fails with actionable message; Drive not green |
| Tokenizer / weights missing on serve | Same class as smoke: `last_error` populated; events include submit_err |
| Send timeout / 504 | CLI error; child log path already recorded by Drive start |
| Event ring overflow | Increment `events_dropped`; never OOM the webview |

## 8. Testing

| Layer | What |
|---|---|
| Unit | Ring append/cap/drop; clear on send; redaction; wait `event=` matcher |
| Integration | Drive host mock agent-event frames → state.events |
| Physical (required) | OpenRouter Drive send shows visible reply + events dump |
| Physical (required) | After Metal pipeline, Drive pin to `mens/qwen35-08b-metal-e2e` returns a short reply |
| Docs doctest | How-to commands compile or `// vox:skip` with reason |

Physical proof remains mandatory: no feature is “done” until driven.

## 9. Acceptance checklist

- [ ] `vox gui drive send` on a selectable OpenRouter model returns a non-empty assistant bubble; `last_error` is null.
- [ ] Mac Metal documented pipeline downloads `Qwen/Qwen3.5-0.8B`, trains, serves, and Drive pin `mens/qwen35-08b-metal-e2e` returns a non-empty assistant bubble.
- [ ] After each send, `state.events` (or `drive events`) lists the turn’s stream/orch/submit frames; caps and `events_dropped` behave as specified.
- [ ] Failure paths (missing key, missing tokenizer) still populate `last_error` + events — never silent success.
- [ ] Headless plane does not claim `events: true`.

## 10. Implementation sequencing

1. **Track A** — event ring + CLI wait/dump (unlocks debugging immediately).
2. **Track B** — enable Metal QLoRA train path + how-to + script.
3. **Track C** — OpenRouter physical proof script (can run as soon as Track A lands if a key exists).
4. **Track B∩Drive** — Metal physical Drive proof after serve.

Tracks A and C can proceed without waiting for full Metal training wall-clock.

## 11. Open risks

| Risk | Mitigation |
|---|---|
| Metal QLoRA still incomplete inside the plugin despite `run_full_training` | Spike early; if blocked, document exact gap and fall back only with operator approval (not smoke) |
| Sync chat path emits few agent events | Still require `submit_*` + bubble; treat stream frames as best-effort enrichment |
| Long Metal train times | Minimal dogfood dataset; short max-steps preset for e2e |
| Catalog id ≠ serve model name | Explicit mapping table in how-to + Drive set uses the served id |

## 12. Out-of-scope follow-ups

- Separate WebKit bundle id for Drive storage isolation (already noted in Axis Drive design)
- SP3-D step streaming protocol
- Fixed OpenRouter regression slug in CI without secrets
- Automated nightly Metal train on GitHub-hosted runners (needs self-hosted Mac GPU)
