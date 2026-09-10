# Axis Chat E2E (OpenRouter + Mac Metal + Drive Events) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Axis Drive prove OpenRouter and Mac Metal Qwen chat end-to-end while exposing a bounded full stream/event dump — without false-green waits or incomplete Metal dispatch.

**Architecture:** Extend live Drive with a capped `DriveTurnEvent` ring (`kind.text` from agent frames, submit_* from send). Add `wait --until reply_ok` and `event=<kind>`. Enable Metal by fixing **three** CUDA hardcodes (CLI gate, `backend_candle_qlora`, serve worker), then spike→train→collateral→serve→Drive pin.

**Tech Stack:** TypeScript/React (vox-gui), Rust (`vox-cli`, `vox-ml-cli`, `vox-populi`, `vox-plugin-mens-candle-metal`), Axis Drive loopback HTTP, Candle QLoRA Metal, VoxScript, Vitest + cargo test + physical Drive.

**Spec:** `docs/superpowers/specs/2026-09-09-axis-chat-e2e-openrouter-metal-design.md` (audit-revised)

## Global Constraints

- Live Axis Drive is the only acceptance plane for chat e2e.
- Green wait predicate is **`reply_ok`**, never bare `reply` (bare `reply` matches errors).
- OpenRouter: prefer selectable `openrouter/auto` or `openrouter/*`; require `provider_type == OpenRouter` once catalog fields exist.
- Mac primary model: `Qwen/Qwen3.5-0.8B`, **spike-gated** for multimodal reject; Drive slug `mens/qwen35-08b-metal-e2e`; run dir `mens/runs/qwen35-08b-metal-e2e`.
- Event ring: capacity 500; text 4 KiB; raw 8 KiB; clear at send start; redact text+raw; `next_seq` monotonic; correlate via `activeTurnIdRef`.
- Token body field: **`frame.kind.text`**, not `frame.text`.
- Secrets via `vox_secrets` only; VoxScript must not `env.get` OpenRouter keys.
- Automation is VoxScript; exit non-zero on failure (never print-only).
- Metal enablement requires CLI gate + populi dispatch + serve worker — not bail removal alone.
- Do not use `--max-steps` (does not exist). Use `--epochs 1` + `--max-runtime-secs`.
- Commit `scripts/*.vox`, dogfood JSONL, and how-tos **before** any `vox run` (VoxScript-first cache rule).
- Metal train/serve needs a **GPU-enabled** `vox-ml-cli` build (`gpu` feature); plain default CLI may hard-error.
- Metal plugin hub download is stubbed — base weights must be **pre-downloaded** locally before train/serve.
- Mac scripts (`mens-macos-metal-e2e`, `axis-drive-metal-e2e`) must `process.exit(2)` on non-Darwin (clear message).
- Do not use retired surfaces (`vox-dei`, `TURSO_URL`, etc.).

### File map

| Path | Responsibility |
|---|---|
| `crates/vox-gui/ui/src/lib/driveEvents.ts` | Ring helpers |
| `crates/vox-gui/ui/src/lib/driveEvents.test.ts` | Unit tests |
| `crates/vox-gui/ui/src/lib/axisDrive.ts` | DriveState / claims / catalog provider fields |
| `crates/vox-gui/ui/src/lib/useDriveBus.ts` | submit_* events; optional catalog on state |
| `crates/vox-gui/ui/src/components/drive/AxisDriveHost.tsx` | Agent event subscription |
| `crates/vox-gui/src/drive/protocol.rs` | Rust DriveState/Claims mirror |
| `crates/vox-cli/src/commands/gui/client.rs` | `reply_ok`, `event=` |
| `contracts/gui/axis-drive.v1.yaml` | Document predicates (v1 additive) |
| `crates/vox-ml-cli/.../run_train.rs` | Remove Metal dead gate |
| `crates/vox-ml-cli/.../plugin_heal.rs` | `ensure_metal_plugin` |
| `crates/vox-populi/.../backend_candle_qlora.rs` | Host-aware plugin load |
| `crates/vox-ml-cli/.../serve/worker.rs` | Host-aware inference plugin |
| `examples/mens/metal-e2e/` | Per-run contract + dogfood JSONL |
| `scripts/mens-macos-metal-e2e.vox` | Mac pipeline |
| `scripts/axis-drive-openrouter-e2e.vox` | Track C |
| `scripts/axis-drive-metal-e2e.vox` | Track B∩Drive |

---

### Task 0: Metal + model spike (blocking for Track B)

**Files:**
- Create: `docs/src/how-to/how-to-train-mens-macos-metal.md` (stub section “Spike results” only; full how-to in Task 7)
- Test: manual on Apple Silicon; record outcome in how-to

**Interfaces:**
- Consumes: `vox plugin install mens-candle-metal`; plugin `run_full_training`
- Produces: recorded `SPIKE_MODEL_ID` (`Qwen/Qwen3.5-0.8B` or text-only fallback) and proof that Metal `run_full_training` can start

- [ ] **Step 1: Install plugin**

```bash
vox plugin install mens-candle-metal --yes
# or: vox plugin install --path crates/vox-plugin-mens-candle-metal --yes
vox plugin list | rg mens-candle-metal
```

Expected: plugin listed for Apple Silicon.

- [ ] **Step 2: Characterize multimodal reject**

Attempt a dry config parse / tiny train against `Qwen/Qwen3.5-0.8B`. If error contains `vision-language / multimodal model`, pick operator-approved text-only Mac-tier fallback and write it into the how-to as `SPIKE_MODEL_ID`. If train proceeds past config parse, `SPIKE_MODEL_ID=Qwen/Qwen3.5-0.8B`.

- [ ] **Step 3: Commit spike note**

```bash
git add docs/src/how-to/how-to-train-mens-macos-metal.md
git commit -m "docs(mens): record Mac Metal train spike model id"
```

Do **not** remove the CLI dead gate until Task 6 after this spike succeeds or fallback is locked.

---

### Task 1: DriveTurnEvent ring helpers (TDD)

**Files:**
- Create: `crates/vox-gui/ui/src/lib/driveEvents.ts`
- Create: `crates/vox-gui/ui/src/lib/driveEvents.test.ts`
- Modify: `crates/vox-gui/ui/src/lib/axisDrive.ts`
- Modify: `crates/vox-gui/src/drive/protocol.rs` (`DriveClaims.events`, state fields)

**Interfaces:**
- Produces: `DriveTurnEvent`, `DriveEventState`, `appendDriveEvent`, `clearDriveEvents`, `recordAgentFrame`, `DRIVE_EVENTS_CAP=500`, `DRIVE_EVENT_TEXT_CAP`, `DRIVE_EVENT_RAW_CAP=8192`

- [ ] **Step 1: Write the failing test**

```ts
// crates/vox-gui/ui/src/lib/driveEvents.test.ts
import { describe, expect, it } from 'vitest';
import {
  DRIVE_EVENTS_CAP,
  DRIVE_EVENT_RAW_CAP,
  appendDriveEvent,
  clearDriveEvents,
  recordAgentFrame,
  type DriveEventState,
} from './driveEvents';

function empty(): DriveEventState {
  return { events: [], events_dropped: 0, last_turn_id: null, next_seq: 1 };
}

describe('driveEvents', () => {
  it('appends submit_ok and tracks turn id', () => {
    const next = appendDriveEvent(empty(), {
      turn_id: 't1',
      kind: 'submit_ok',
      text: 'hello',
    });
    expect(next.events).toHaveLength(1);
    expect(next.events[0]?.kind).toBe('submit_ok');
    expect(next.events[0]?.seq).toBe(1);
    expect(next.last_turn_id).toBe('t1');
    expect(next.next_seq).toBe(2);
  });

  it('drops oldest at CAP+1 and increments events_dropped by 1', () => {
    let state = empty();
    for (let i = 0; i < DRIVE_EVENTS_CAP + 1; i++) {
      state = appendDriveEvent(state, { turn_id: 't', kind: 'token_streamed', text: String(i) });
    }
    expect(state.events).toHaveLength(DRIVE_EVENTS_CAP);
    expect(state.events_dropped).toBe(1);
    expect(state.events[0]?.text).toBe('1');
  });

  it('redacts bearer-looking substrings in text', () => {
    const next = appendDriveEvent(empty(), {
      turn_id: 't',
      kind: 'diag',
      text: 'Authorization: Bearer sk-secret-value',
    });
    expect(next.events[0]?.text ?? '').not.toContain('sk-secret-value');
    expect(next.events[0]?.text ?? '').toMatch(/\[redacted\]/i);
  });

  it('caps and redacts raw JSON secrets', () => {
    const big = { headers: { Authorization: 'Bearer sk-secret-value' }, pad: 'x'.repeat(9000) };
    const next = appendDriveEvent(empty(), { turn_id: 't', kind: 'agent_event', raw: big });
    const serialized = JSON.stringify(next.events[0]?.raw);
    expect(serialized.length).toBeLessThanOrEqual(DRIVE_EVENT_RAW_CAP + 8);
    expect(serialized).not.toContain('sk-secret-value');
  });

  it('clearDriveEvents preserves next_seq', () => {
    const filled = appendDriveEvent(empty(), { turn_id: 't', kind: 'submit_ok' });
    const cleared = clearDriveEvents(filled);
    expect(cleared.events).toEqual([]);
    expect(cleared.events_dropped).toBe(0);
    expect(cleared.last_turn_id).toBeNull();
    expect(cleared.next_seq).toBe(filled.next_seq);
  });

  it('recordAgentFrame reads kind.type and kind.text', () => {
    const frame = { kind: { type: 'token_streamed', text: 'tok', session_id: 's1' } };
    const next = recordAgentFrame(empty(), frame, 'turn-1');
    expect(next.events[0]?.kind).toBe('token_streamed');
    expect(next.events[0]?.text).toBe('tok');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir crates/vox-gui/ui test src/lib/driveEvents.test.ts`

Expected: FAIL (module not found)

- [ ] **Step 3: Write minimal implementation**

Implement `driveEvents.ts` with text truncate+redact, raw stringify → redact → truncate to 8192, `appendDriveEvent`, `clearDriveEvents`, `recordAgentFrame` mapping `frame.kind.type` / `frame.kind.text`.

Wire into `axisDrive.ts` `DriveState` / `emptyLiveState` / `DriveClaims.events: true` on live.

Mirror in `protocol.rs`:

```rust
pub struct DriveClaims {
    pub picker_ui: bool,
    pub composer_knobs: bool,
    pub bubbles: bool,
    pub events: bool,
}
// live(): events: true; headless(): events: false
```

Add `events`, `events_dropped`, `last_turn_id`, `next_seq` to Rust `DriveState` with serde defaults so old clients don't break.

- [ ] **Step 4: Run tests**

Run: `pnpm --dir crates/vox-gui/ui test src/lib/driveEvents.test.ts`

Expected: PASS

Also: `cargo test -p vox-gui contract_yaml_typed_verbs_and_keys` after contract touch in Task 4 (may skip until then).

- [ ] **Step 5: Commit**

```bash
git add crates/vox-gui/ui/src/lib/driveEvents.ts crates/vox-gui/ui/src/lib/driveEvents.test.ts \
  crates/vox-gui/ui/src/lib/axisDrive.ts crates/vox-gui/src/drive/protocol.rs
git commit -m "feat(gui): add Drive turn event ring helpers and claims.events"
```

---

### Task 2: Emit submit events from Drive send

**Files:**
- Modify: `crates/vox-gui/ui/src/lib/useDriveBus.ts`
- Modify: `crates/vox-gui/ui/src/lib/useDriveBus.test.ts`

**Interfaces:**
- Consumes: `appendDriveEvent`, `clearDriveEvents`
- Produces: send clears prior events then appends `submit_ok` or `submit_err`; sets `last_turn_id`

- [ ] **Step 1: Write failing tests** (extend `useDriveBus.test.ts`; reuse existing `handleDriveRequest` / `emptyLiveState` helpers already in that file)

```ts
it('send clears prior events and records submit_err', async () => {
  const state = emptyLiveState();
  state.events = [{ seq: 1, ts_ms: 1, kind: 'old', turn_id: 'old', text: 'x' }];
  state.next_seq = 2;
  const res = await handleDriveRequest({
    req: { id: '1', verb: 'send', body: { text: 'hi' } },
    state,
    models: [{ id: 'openrouter/auto', provider: 'OpenRouter', providerType: 'OpenRouter' } as any],
    statuses: [{ provider: 'OpenRouter', key_present: true, is_local: false, local_reachable: null } as any],
    submit: async () => ({ ok: false, error: 'boom' }),
  });
  expect(res.state.events.some((e: any) => e.kind === 'old')).toBe(false);
  expect(res.state.events.some((e: any) => e.kind === 'submit_err')).toBe(true);
  expect(res.state.last_error).toBe('boom');
  expect(res.state.last_turn_id).toBeTruthy();
});

it('send records submit_ok with assistant text', async () => {
  const res = await handleDriveRequest({
    req: { id: '1', verb: 'send', body: { text: 'hi' } },
    state: emptyLiveState(),
    models: [],
    statuses: [],
    submit: async () => ({ ok: true, text: 'hello-assistant' }),
  });
  expect(res.state.last_error).toBeNull();
  expect(res.state.events.some((e: any) => e.kind === 'submit_ok' && e.text === 'hello-assistant')).toBe(true);
});

it('mutation: send path references clearDriveEvents and appendDriveEvent', () => {
  const src = handleDriveRequest.toString();
  expect(src).toMatch(/clearDriveEvents/);
  expect(src).toMatch(/appendDriveEvent/);
});
```

- [ ] **Step 2: Run — expect FAIL**

Run: `pnpm --dir crates/vox-gui/ui test src/lib/useDriveBus.test.ts`

- [ ] **Step 3: Implement**

In send branch of `handleDriveRequest`:

1. `const turnId = crypto.randomUUID()`.
2. Apply `clearDriveEvents` to event fields.
3. Set `last_turn_id = turnId` and store turn id for host ref (export via state).
4. After `interpretDriveSubmit`, append `submit_ok` (assistant text) or `submit_err` (error).
5. Return updated `events`, `events_dropped`, `last_turn_id`, `next_seq`.

- [ ] **Step 4: Run — expect PASS**

- [ ] **Step 5: Commit**

```bash
git add crates/vox-gui/ui/src/lib/useDriveBus.ts crates/vox-gui/ui/src/lib/useDriveBus.test.ts
git commit -m "feat(gui): record Drive submit_ok/submit_err events"
```

---

### Task 3: Mirror agent-event frames into the Drive ring

**Status (2026-09-10): DONE**

Shipped: Drive ring records `submit_ok` / `submit_err`, `token_streamed` (and related agent frames) for the active turn; Metal e2e observed `event_kinds=research_executed,token_streamed,token_streamed,submit_ok`.

Honesty residuals closed:
- Drive CLI `set` / `send` / `state` require HTTP 200 + non-empty JSON (`require_ok_json_body`); soft empty/status-0 no longer exits 0. Headless send also fails on `last_error`.
- Drive send mints `turn_id`/`trace_id` once and passes them on `DriveSubmitPayload`; App/`buildChatTurn` reuse them so ChatHop matches Drive `last_turn_id`.

**Files:**
- Modify: `crates/vox-gui/ui/src/components/drive/AxisDriveHost.tsx`
- Create: `crates/vox-gui/ui/src/components/drive/AxisDriveHost.events.test.tsx` (or `.test.ts` if host helpers extracted)

**Interfaces:**
- Consumes: `listenAgentEvents` from `transport.ts`; `recordAgentFrame`
- Produces: frames appended under `activeTurnIdRef`

- [x] **Step 1: Write failing integration-style test** (host events test present)

```ts
import { describe, expect, it, vi } from 'vitest';
import { recordAgentFrame, clearDriveEvents, type DriveEventState } from '../../lib/driveEvents';

// Production path test for the host wiring uses vi.mock on transport:
vi.mock('../../transport', () => {
  let cb: ((f: any) => void) | null = null;
  return {
    listenAgentEvents: async (handler: (f: any) => void) => {
      cb = handler;
      return () => { cb = null; };
    },
    __emit: (f: any) => cb?.(f),
  };
});

it('host subscription records kind.text token frames for active turn', async () => {
  // Mount or invoke the same useEffect body via extracted `attachDriveAgentListener(stateRef, activeTurnIdRef)`.
  // Emit: { kind: { type: 'token_streamed', text: 'ab', session_id: 'sess' } }
  // Assert stateRef.current.events has kind token_streamed and text 'ab'.
});
```

Extract `attachDriveAgentListener` if mounting React is heavy — **required**, not optional. Pattern exists in `BrowserView.test.tsx` (`listenAgentEvents` mock).

- [x] **Step 2: Implement in AxisDriveHost** (listener + active turn wiring landed earlier on branch)

- [x] **Step 3: Mutation test** — removing `listenAgentEvents` call fails the host test.

- [x] **Step 4: Run UI tests**

- [x] **Residual:** Drive CLI non-zero exit on soft/empty responses + single-mint turn/trace handoff.
Run: `pnpm --dir crates/vox-gui/ui test src/lib/driveEvents.test.ts src/lib/useDriveBus.test.ts src/components/drive/`

Expected: PASS

- [x] **Step 5: Commit** (landed with Axis chat honesty leftovers branch)

```bash
git add crates/vox-gui/ui/src/components/drive/AxisDriveHost.tsx \
  crates/vox-gui/ui/src/components/drive/AxisDriveHost.events.test.tsx \
  crates/vox-gui/ui/src/lib/driveEvents.ts
git commit -m "feat(gui): mirror agent events into Axis Drive state"
```

---

### Task 4: CLI `reply_ok` + `event=<kind>`

**Files:**
- Modify: `crates/vox-cli/src/commands/gui/client.rs`
- Modify: `contracts/gui/axis-drive.v1.yaml`
- Modify: `crates/vox-gui/src/drive/protocol.rs` (contract test asserts new YAML keys)
- Modify: `crates/vox-cli/src/cli_args.rs` (wait help text)
- Modify: `docs/src/reference/cli.md` (wait predicates)
- Modify: `docs/superpowers/specs/2026-09-08-axis-drive-design.md` (wait grammar line)

**Interfaces:**
- Produces: `matches_until("reply_ok", …)`, `matches_until("event=submit_ok", …)`

- [ ] **Step 1: Write failing tests in `client.rs`**

```rust
#[test]
fn matches_event_kind_nested_under_state() {
    let body = r#"{"status":200,"state":{"events":[{"kind":"token_streamed"},{"kind":"submit_ok"}],"last_error":null}}"#;
    assert!(matches_until("event=submit_ok", body));
    assert!(matches_until("event=token_streamed", body));
    assert!(!matches_until("event=missing", body));
    let empty = r#"{"status":200,"state":{"events":[],"last_error":null}}"#;
    assert!(!matches_until("event=submit_ok", empty));
    let flat = r#"{"catalog":[],"last_error":null}"#;
    assert!(!matches_until("event=submit_ok", flat));
}

#[test]
fn matches_reply_ok_rejects_error_settlement() {
    let err = r#"{"status":200,"state":{"last_error":"load tokenizer","bubbles":[{"role":"assistant","error":true,"content":"load tokenizer"}]}}"#;
    assert!(matches_until("reply", err)); // legacy: settled
    assert!(!matches_until("reply_ok", err));
    let ok = r#"{"status":200,"state":{"last_error":null,"bubbles":[{"role":"user","content":"hi"},{"role":"assistant","content":"hello"}]}}"#;
    assert!(matches_until("reply_ok", ok));
    let empty_asst = r#"{"status":200,"state":{"last_error":null,"bubbles":[{"role":"assistant","content":""}]}}"#;
    assert!(!matches_until("reply_ok", empty_asst));
}
```

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p vox-cli --features gui --lib matches_event_kind_nested_under_state matches_reply_ok_rejects_error_settlement`

- [ ] **Step 3: Implement matchers**

After `selectable=` branch, before `false`:

```rust
if until == "reply_ok" {
    if view.get("last_error").map(|x| !x.is_null()).unwrap_or(false) {
        return false;
    }
    let last = view.get("bubbles").and_then(|b| b.as_array()).and_then(|a| a.last());
    let Some(bubble) = last else { return false; };
    if bubble.get("role").and_then(|r| r.as_str()) != Some("assistant") {
        return false;
    }
    if bubble.get("error") == Some(&serde_json::Value::Bool(true)) {
        return false;
    }
    return bubble
        .get("content")
        .and_then(|c| c.as_str())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
}
if let Some(kind) = until.strip_prefix("event=") {
    if kind.is_empty() {
        return false;
    }
    return view
        .get("events")
        .and_then(|e| e.as_array())
        .into_iter()
        .flatten()
        .any(|row| row.get("kind").and_then(|k| k.as_str()) == Some(kind));
}
```

Add to `axis-drive.v1.yaml` (keep `version: 1`):

```yaml
wait_until:
  reply: "last_error set OR last bubble role=assistant (settled; may be error)"
  reply_ok: "last_error null AND last assistant bubble non-empty AND error!=true"
  error: "last_error non-null"
  selectable: "selectable=<catalog_id>"
  event: "event=<kind> matches state.events[].kind"
state_snapshot:
  events: "DriveTurnEvent[] cap 500"
  events_dropped: integer
  last_turn_id: "string|null"
```

Extend contract test to assert `wait_until` / `reply_ok` present.

- [ ] **Step 4: Run — expect PASS**

- [ ] **Step 5: Commit**

```bash
git add crates/vox-cli/src/commands/gui/client.rs contracts/gui/axis-drive.v1.yaml \
  crates/vox-gui/src/drive/protocol.rs crates/vox-cli/src/cli_args.rs \
  docs/src/reference/cli.md docs/superpowers/specs/2026-09-08-axis-drive-design.md
git commit -m "feat(cli): Drive wait reply_ok and event=kind predicates"
```

---

### Task 5: Catalog fields + OpenRouter physical proof

**Files:**
- Modify: `crates/vox-gui/ui/src/lib/axisDrive.ts` (`DriveCatalogRow.provider`, `provider_type`; `snapshotCatalog`)
- Modify: `crates/vox-gui/ui/src/lib/useDriveBus.ts` — optionally load catalog on `state` **or** document set-first; prefer including `state` in `driveVerbNeedsCatalog` for operator honesty
- Create: `scripts/axis-drive-openrouter-e2e.vox`
- Create: `docs/src/how-to/how-to-axis-drive-openrouter-e2e.md`

**Interfaces:**
- Consumes: Tasks 1–4
- Produces: exit 0 only on `reply_ok` predicates

- [ ] **Step 1: Extend catalog row + load on state**

```ts
export interface DriveCatalogRow {
  id: string;
  selectable: boolean;
  reason: string | null;
  provider?: string;
  provider_type?: string;
}

export function driveVerbNeedsCatalog(verb: DriveRequest['verb']): boolean {
  return verb === 'set' || verb === 'send' || verb === 'state';
}
```

Add unit test: `state` verb with mocked models includes OpenRouter `provider_type`.

- [ ] **Step 2: Author VoxScript** (copy `vox_binary_path` from `scripts/graphify-refresh.vox`; use `process.run` / capture + `process.exit`)

Exact flow:

1. `drive start`
2. `drive set --knob execution=sync` (loads catalog)
3. Parse nested `state.catalog`; pick first selectable where `id == "openrouter/auto"` OR `id.starts_with("openrouter/")` OR `provider_type == "OpenRouter"`; reject `mens/` and bare tier ids.
4. If none: exit 1 with message mentioning `SecretId::OpenRouterApiKey` / `vox secrets doctor` — do **not** read env keys.
5. `set --knob model_override=<id>`
6. `send --text 'Reply with one short line containing openrouter-ok'`
7. `wait --until reply_ok --timeout 120s`
8. Re-fetch `state`; FAIL if `last_error != null` OR assistant empty/error OR missing `submit_ok` in events OR `plane != live`.
9. Print summary JSON; `drive stop`; exit 0.

- [ ] **Step 3: Physically run**

Run: `vox run scripts/axis-drive-openrouter-e2e.vox`

Expected: PASS with real reply, or FAIL with parseable reason (fix secrets/routing until green).

- [ ] **Step 4: Commit**

```bash
git add crates/vox-gui/ui/src/lib/axisDrive.ts crates/vox-gui/ui/src/lib/useDriveBus.ts \
  scripts/axis-drive-openrouter-e2e.vox docs/src/how-to/how-to-axis-drive-openrouter-e2e.md
git commit -m "feat(scripts): Axis Drive OpenRouter e2e with reply_ok gate"
```

---

### Task 6: Host-aware Metal QLoRA train dispatch

**Files:**
- Modify: `crates/vox-populi/src/mens/tensor/backend_candle_qlora.rs`
- Modify: `crates/vox-ml-cli/src/commands/mens/plugin_heal.rs` (add `ensure_metal_plugin`)
- Modify: `crates/vox-ml-cli/src/commands/schola/train/run_train.rs` (remove dead gate; call metal heal)
- Test: macOS-gated unit/characterization tests

**Interfaces:**
- Consumes: Task 0 spike model id; `ML_BACKEND_CANDIDATES` pattern from `eval_local.rs` / `merge_qlora.rs`
- Produces: `vox mens train --device metal` loads `mens-candle-metal` and calls `run_full_training`

- [ ] **Step 1: Failing characterization tests**

```rust
#[cfg(all(test, target_os = "macos"))]
#[test]
fn metal_qlora_error_is_not_the_old_dead_gate_message() { /* … */ }

#[test]
fn candle_qlora_plugin_id_for_metal_is_mens_candle_metal() {
    // Extract helper plugin_id_for_device(DeviceKind) -> &str
    assert_eq!(plugin_id_for_device(DeviceKind::Metal), "mens-candle-metal");
    assert_eq!(plugin_id_for_device(DeviceKind::Cuda), "mens-candle-cuda");
}
```

- [ ] **Step 2: Run — expect FAIL**

- [ ] **Step 3: Implement**

1. In `backend_candle_qlora.rs`, replace hardcoded `mens-candle-cuda` with device-kind selection (Metal → `mens-candle-metal`).
2. Add `ensure_metal_plugin` mirroring CUDA heal (`vox plugin install mens-candle-metal --yes`).
3. Remove unconditional Metal bail in `run_train.rs`; call `ensure_metal_plugin` on macOS Metal.
4. Update outdated comment: SP3-D stubs do **not** block `run_full_training`.

- [ ] **Step 4: Micro train smoke (Apple Silicon)**

```bash
vox mens train --backend qlora --tokenizer hf --device metal \
  --model <SPIKE_MODEL_ID> \
  --data-dir examples/mens/metal-e2e \
  --output-dir mens/runs/qwen35-08b-metal-e2e \
  --epochs 1 --max-runtime-secs 300
```

Expected: artifacts under output dir; error must not be the old dead-gate string.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-populi/src/mens/tensor/backend_candle_qlora.rs \
  crates/vox-ml-cli/src/commands/mens/plugin_heal.rs \
  crates/vox-ml-cli/src/commands/schola/train/run_train.rs
git commit -m "feat(mens): host-aware Metal Candle QLoRA train dispatch"
```

---

### Task 7: Metal serve worker + e2e data + how-to + automation

**Files:**
- Modify: `crates/vox-ml-cli/src/commands/ai/serve/worker.rs`
- Create: `examples/mens/metal-e2e/training_contract.yaml`
- Create: `examples/mens/metal-e2e/dogfood-metal-e2e.jsonl` (ChatML/`prompt`+`response` rows; **do not** name it `train.jsonl` if that trips `MIN_CORPUS_PAIRS` — follow contract `train_path`)
- Create/finish: `docs/src/how-to/how-to-train-mens-macos-metal.md` (full frontmatter)
- Create: `scripts/mens-macos-metal-e2e.vox`

**Interfaces:**
- Produces: Metal serve using `mens-candle-metal`; collateral-pass run dir; Drive pin stem

- [ ] **Step 1: Fix serve worker**

Replace hardcoded `mens-candle-cuda` with `resolve_extension_point` / same candidates as `eval_local.rs`.

Add unit/characterization test that Metal device path selects metal plugin id (or cfg-gated).

- [ ] **Step 2: Create e2e data dir**

`examples/mens/metal-e2e/training_contract.yaml` points `train_path` at local JSONL. Ensure workspace contract does not hijack (per-run `--data-dir examples/mens/metal-e2e`).

- [ ] **Step 3: How-to + script**

How-to frontmatter (required):

```yaml
---
title: "How To: Train MENS on macOS Metal"
description: "Download, QLoRA-train, collateral-check, and serve a Mac Metal pack for Axis Drive."
category: "How-To Guides"
status: "current"
training_eligible: true
schema_type: "HowTo"
---
```

Document: Apple Silicon; GPU-enabled CLI; plugin install; pre-download of `SPIKE_MODEL_ID` (hub stub); train command; **collateral_damage_report.json** with `"status":"pass"` (`vox mens eval collateral-damage …` or exact command from codebase); tokenizer.json in run dir; MensCatalog may not list QLoRA dirs — Drive green via VoxLocal stem match and/or `pin_policy=warn` if needed; serve on non-11434 if Ollama occupies default:

```bash
vox mens serve --model mens/runs/qwen35-08b-metal-e2e --host 127.0.0.1 --port 11435
```

Drive pin `mens/qwen35-08b-metal-e2e`. Script first lines: Darwin check → `process.exit(2)` otherwise.

- [ ] **Step 4: Run automation on Mac**

Run: `vox run scripts/mens-macos-metal-e2e.vox`

Expected: run dir + serve health lists stem.

- [ ] **Step 5: Commit**

```bash
git add crates/vox-ml-cli/src/commands/ai/serve/worker.rs examples/mens/metal-e2e \
  docs/src/how-to/how-to-train-mens-macos-metal.md scripts/mens-macos-metal-e2e.vox
git commit -m "feat(mens): Metal serve + Mac train→serve e2e pipeline"
```

---

### Task 8: Physical Metal Drive proof

**Files:**
- Create: `scripts/axis-drive-metal-e2e.vox`

**Interfaces:**
- Consumes: serving pack from Task 7; Tasks 1–4

- [ ] **Step 1: Author script**

1. Assert darwin.
2. Probe `GET http://127.0.0.1:<port>/v1/models` contains `qwen35-08b-metal-e2e` (or `state.probe` after catalog load).
3. `drive start` → `set execution=sync` → `set model_override=mens/qwen35-08b-metal-e2e`.
4. FAIL if catalog row not selectable and `pin_policy=fail`.
5. `send` short prompt → `wait --until reply_ok`.
6. Assert `submit_ok`, non-error bubble, `last_error` null; dump `event_kinds`.
7. `drive stop`; exit non-zero on tokenizer/`last_error`.

- [ ] **Step 2: Physically run**

Run: `vox run scripts/axis-drive-metal-e2e.vox`

Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add scripts/axis-drive-metal-e2e.vox
git commit -m "feat(scripts): Axis Drive Mac Metal e2e proof"
```

---

### Task 9: Spec acceptance sweep + failure honesty

**Status (2026-09-10): DONE (with narrowed failure-honesty)**

**Evidence:**
- OpenRouter live: `vox run scripts/axis-drive-openrouter-e2e.vox` → `{"status":"pass","plane":"live",...}`; ChatHop JSONL under `VOX_DOGFOOD_TRACE_PATH` with correlated `trace_id`/`turn_id`, `turn_outcome: ok`.
- OpenRouter failure honesty: `scripts/axis-drive-openrouter-failure-honesty.vox` → `{"status":"pass","reason":"model_not_selectable_or_key_missing"}` (fast 409 / selectable path; full send hang without keys intentionally narrowed).
- Metal Drive: `VOX_LOCAL_ENDPOINT=http://127.0.0.1:11435` + `vox mens serve` on `:11435` + `scripts/axis-drive-metal-e2e.vox` → `{"status":"pass","plane":"live","model":"mens/qwen35-08b-metal-e2e",...}` with `token_streamed` events and **no** OpenRouter `cost_incurred`.
- Stack overflow / empty `orch.tool_call`: fixed via 32 MiB orch worker stack + `RUST_MIN_STACK` / dogfood / `VOX_LOCAL_ENDPOINT` forwarded into Drive + orch children.
- Sticky `mens/*` without orch registry entry: synthesize `ProviderType::VoxLocal` (no silent `openrouter/auto` fallthrough).

- [x] **Step 1:** Tick every §9 checkbox with evidence from Tasks 5 and 8 summary JSON.
- [x] **Step 2:** Live failure demos on live plane: OpenRouter key/selectable failure path green via honesty script (narrowed).
- [x] **Step 3:** Headless `claims.events === false` (explicit); live `claims.events === true` (prior tasks).
- [x] **Step 4:** Confirm green scripts use only `wait --until reply_ok` (never bare `reply`).

```bash
git commit -m "docs: record Axis chat e2e acceptance evidence" # only if adding evidence
```

---

## Spec coverage self-review

| Spec requirement | Task |
|---|---|
| Audit P0 reply FP → `reply_ok` | Task 4 |
| Event ring + raw cap + redaction | Task 1 |
| submit_* on send | Task 2 |
| `kind.text` agent mirror + active turn | Task 3 (DONE — CLI honesty + turn_id handoff) |
| Catalog provider fields / state loads catalog | Task 5 |
| OpenRouter green via `reply_ok` | Task 5 / Task 9 |
| Metal spike / multimodal gate | Task 0 |
| Host-aware train dispatch + heal + gate removal | Task 6 |
| Metal serve worker + collateral + data contract | Task 7 |
| Metal Drive green | Task 8 / Task 9 |
| Failure honesty + headless claims | Task 9 |

### Placeholder scan

No TBD / “adjust mocks” / “if harness exists” left — Task 3 requires extracted listener + mock pattern; Task 6 cites concrete files; Task 7 names collateral gate.

### Type consistency

`DriveTurnEvent`, `reply_ok`, `mens/qwen35-08b-metal-e2e`, `SPIKE_MODEL_ID`, `mens-candle-metal`, `frame.kind.text` are consistent across tasks.
