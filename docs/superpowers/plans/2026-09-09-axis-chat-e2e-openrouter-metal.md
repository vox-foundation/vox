# Axis Chat E2E (OpenRouter + Mac Metal + Drive Events) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Axis Drive prove OpenRouter and Mac Metal Qwen3.5-0.8B chat end-to-end while exposing a bounded full stream/event dump for debugging.

**Architecture:** Extend live Drive state with a capped `DriveTurnEvent` ring fed from submit results and `listenAgentEvents`. Remove the Metal QLoRA CLI dead gate and route training through `vox-plugin-mens-candle-metal` `run_full_training`. Document and automate download→train→serve, then physically drive OpenRouter + `mens/qwen35-08b-metal-e2e`.

**Tech Stack:** TypeScript/React (vox-gui UI), Rust (`vox-cli`, `vox-ml-cli`, `vox-plugin-mens-candle-metal`), Axis Drive loopback HTTP, Candle QLoRA Metal, VoxScript (`.vox`) automation, Vitest + cargo test + physical Drive.

**Spec:** `docs/superpowers/specs/2026-09-09-axis-chat-e2e-openrouter-metal-design.md`

## Global Constraints

- Live Axis Drive is the only acceptance plane for chat e2e (not headless-only, not raw `vox chat`).
- OpenRouter acceptance: any selectable OpenRouter catalog entry returning a short reply.
- Mac base model: `Qwen/Qwen3.5-0.8B` (operator “3.8 Qwen” = this 0.8B Mac-tier step).
- Local Drive slug: `mens/qwen35-08b-metal-e2e`.
- Event ring capacity 500; text cap 4 KiB; raw cap 8 KiB; clear on next `send` by default.
- Never claim silent success: failures must set `last_error` and emit `submit_err`.
- Secrets via `vox_secrets::resolve_secret` only; no new raw env key reads in consumers.
- Project automation is VoxScript (`vox run scripts/…`); no new `.ps1`/`.sh`/`.py` glue.
- Physical drive required before calling a track done.
- Do not use retired surfaces (`vox-dei`, `TURSO_URL`, etc.).

### File map

| Path | Responsibility |
|---|---|
| `crates/vox-gui/ui/src/lib/axisDrive.ts` | `DriveTurnEvent` types + empty state fields + ring helpers |
| `crates/vox-gui/ui/src/lib/driveEvents.ts` | append/cap/redact/clear helpers (keep `axisDrive.ts` from growing further) |
| `crates/vox-gui/ui/src/lib/driveEvents.test.ts` | unit tests for ring behavior |
| `crates/vox-gui/ui/src/lib/useDriveBus.ts` | clear ring on send; emit submit_ok/submit_err |
| `crates/vox-gui/ui/src/components/drive/AxisDriveHost.tsx` | subscribe agent events → ring |
| `crates/vox-cli/src/commands/gui/client.rs` | `wait --until event=<kind>` |
| `contracts/gui/axis-drive.v1.yaml` | document events fields / wait predicate |
| `crates/vox-ml-cli/src/commands/schola/train/run_train.rs` | remove Metal dead gate; dispatch Metal plugin |
| `docs/src/how-to/how-to-train-mens-macos-metal.md` | operator SSOT |
| `scripts/mens-macos-metal-e2e.vox` | download→train→serve handoff |
| `scripts/axis-drive-openrouter-e2e.vox` | Track C physical proof |
| `scripts/axis-drive-metal-e2e.vox` | Track B∩Drive physical proof |

---

### Task 1: DriveTurnEvent ring helpers (TDD)

**Files:**
- Create: `crates/vox-gui/ui/src/lib/driveEvents.ts`
- Create: `crates/vox-gui/ui/src/lib/driveEvents.test.ts`
- Modify: `crates/vox-gui/ui/src/lib/axisDrive.ts`

**Interfaces:**
- Consumes: none
- Produces: `DriveTurnEvent`, `appendDriveEvent`, `clearDriveEvents`, `DRIVE_EVENTS_CAP` (500)

- [ ] **Step 1: Write the failing test**

```ts
// crates/vox-gui/ui/src/lib/driveEvents.test.ts
import { describe, expect, it } from 'vitest';
import {
  DRIVE_EVENTS_CAP,
  appendDriveEvent,
  clearDriveEvents,
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
  });

  it('drops oldest when over capacity and increments events_dropped', () => {
    let state = empty();
    for (let i = 0; i < DRIVE_EVENTS_CAP + 3; i++) {
      state = appendDriveEvent(state, { turn_id: 't', kind: 'token_streamed', text: String(i) });
    }
    expect(state.events).toHaveLength(DRIVE_EVENTS_CAP);
    expect(state.events_dropped).toBe(3);
    expect(state.events[0]?.text).toBe('3');
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

  it('clearDriveEvents resets ring but keeps next_seq monotonic preference', () => {
    const filled = appendDriveEvent(empty(), { turn_id: 't', kind: 'submit_ok' });
    const cleared = clearDriveEvents(filled);
    expect(cleared.events).toEqual([]);
    expect(cleared.events_dropped).toBe(0);
    expect(cleared.last_turn_id).toBeNull();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir crates/vox-gui/ui test src/lib/driveEvents.test.ts`

Expected: FAIL (module not found)

- [ ] **Step 3: Write minimal implementation**

```ts
// crates/vox-gui/ui/src/lib/driveEvents.ts
export const DRIVE_EVENTS_CAP = 500;
export const DRIVE_EVENT_TEXT_CAP = 4 * 1024;

export interface DriveTurnEvent {
  seq: number;
  ts_ms: number;
  kind: string;
  turn_id: string;
  text?: string;
  raw?: unknown;
}

export interface DriveEventState {
  events: DriveTurnEvent[];
  events_dropped: number;
  last_turn_id: string | null;
  next_seq: number;
}

export function redactDriveText(text: string): string {
  return text
    .replace(/Bearer\s+[A-Za-z0-9._\-]+/gi, 'Bearer [redacted]')
    .replace(/\bsk-[A-Za-z0-9]{8,}\b/g, '[redacted]');
}

export function truncateDriveText(text: string): string {
  if (text.length <= DRIVE_EVENT_TEXT_CAP) return text;
  return `${text.slice(0, DRIVE_EVENT_TEXT_CAP)}…`;
}

export function appendDriveEvent(
  state: DriveEventState,
  input: { turn_id: string; kind: string; text?: string; raw?: unknown; ts_ms?: number },
): DriveEventState {
  const text =
    input.text === undefined
      ? undefined
      : truncateDriveText(redactDriveText(input.text));
  const event: DriveTurnEvent = {
    seq: state.next_seq,
    ts_ms: input.ts_ms ?? Date.now(),
    kind: input.kind,
    turn_id: input.turn_id,
    text,
    raw: input.raw,
  };
  const events = [...state.events, event];
  let dropped = state.events_dropped;
  while (events.length > DRIVE_EVENTS_CAP) {
    events.shift();
    dropped += 1;
  }
  return {
    events,
    events_dropped: dropped,
    last_turn_id: input.turn_id,
    next_seq: state.next_seq + 1,
  };
}

export function clearDriveEvents(state: DriveEventState): DriveEventState {
  return {
    events: [],
    events_dropped: 0,
    last_turn_id: null,
    next_seq: state.next_seq,
  };
}
```

Wire fields into `DriveState` / `emptyLiveState` / headless claims (`events: false` on headless claims object if present; live `claims` may omit until Task 2).

- [ ] **Step 4: Run tests and make sure they pass**

Run: `pnpm --dir crates/vox-gui/ui test src/lib/driveEvents.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/vox-gui/ui/src/lib/driveEvents.ts crates/vox-gui/ui/src/lib/driveEvents.test.ts crates/vox-gui/ui/src/lib/axisDrive.ts
git commit -m "feat(gui): add Drive turn event ring helpers"
```

---

### Task 2: Emit submit events from Drive send

**Files:**
- Modify: `crates/vox-gui/ui/src/lib/useDriveBus.ts`
- Modify: `crates/vox-gui/ui/src/lib/useDriveBus.test.ts`

**Interfaces:**
- Consumes: `appendDriveEvent`, `clearDriveEvents` from Task 1
- Produces: send path clears prior events, then appends `submit_ok` or `submit_err` with a new `turn_id`

- [ ] **Step 1: Write the failing test**

Add to `useDriveBus.test.ts`:

```ts
it('send clears prior events and records submit_err', async () => {
  const state = emptyLiveState();
  state.events = [
    { seq: 1, ts_ms: 1, kind: 'old', turn_id: 'old', text: 'x' },
  ];
  const res = await handleDriveRequest({
    req: { id: '1', verb: 'send', body: { text: 'hi' } },
    state,
    models: [{ id: 'openrouter/x', provider: 'OpenRouter', providerType: 'cloud' } as any],
    statuses: [],
    submit: async () => ({ ok: false, error: 'boom' }),
  });
  expect(res.state.events.some((e: any) => e.kind === 'old')).toBe(false);
  expect(res.state.events.some((e: any) => e.kind === 'submit_err')).toBe(true);
  expect(res.state.last_error).toBe('boom');
  expect(res.state.last_turn_id).toBeTruthy();
});
```

(Adjust mocks to match existing test helpers in that file.)

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir crates/vox-gui/ui test src/lib/useDriveBus.test.ts`

Expected: FAIL on missing `events` behavior

- [ ] **Step 3: Write minimal implementation**

In `handleDriveRequest` send branch:

1. `const turnId = crypto.randomUUID()` (or `drive-turn-${Date.now()}`).
2. Start from `clearDriveEvents` applied to state’s event fields.
3. After `interpretDriveSubmit`, `appendDriveEvent` with `submit_ok` (assistant text) or `submit_err` (error).
4. Return `events`, `events_dropped`, `last_turn_id` on `state`.

- [ ] **Step 4: Run tests and make sure they pass**

Run: `pnpm --dir crates/vox-gui/ui test src/lib/useDriveBus.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/vox-gui/ui/src/lib/useDriveBus.ts crates/vox-gui/ui/src/lib/useDriveBus.test.ts
git commit -m "feat(gui): record Drive submit_ok/submit_err events"
```

---

### Task 3: Mirror agent-event frames into the Drive ring

**Files:**
- Modify: `crates/vox-gui/ui/src/components/drive/AxisDriveHost.tsx`
- Create or modify: `crates/vox-gui/ui/src/components/drive/AxisDriveHost.test.tsx` (if harness exists; else extend an adjacent Drive test)

**Interfaces:**
- Consumes: `listenAgentEvents` from `transport.ts`; `appendDriveEvent`
- Produces: live frames appended with `kind` from `frame.kind?.type ?? 'agent_event'`

- [ ] **Step 1: Write the failing test**

Mock `listenAgentEvents` to invoke the callback once with `{ kind: { type: 'token_streamed' }, text: 'tok' }`. Mount/host call path should leave `stateRef.current.events` containing `token_streamed`. Prefer a small extracted function `recordAgentFrame(state, frame, turnId)` tested without full React if host testing is heavy:

```ts
// in driveEvents.test.ts or AxisDriveHost helper test
it('maps agent frame kind into drive event', () => {
  const frame = { kind: { type: 'token_streamed' }, text: 'ab' };
  const next = appendDriveEvent(empty(), {
    turn_id: 't1',
    kind: String(frame.kind?.type ?? 'agent_event'),
    text: frame.text,
  });
  expect(next.events[0]?.kind).toBe('token_streamed');
});
```

Then wire host to call that on every frame using `stateRef.current.last_turn_id ?? 'pre-send'`.

- [ ] **Step 2: Run test to verify it fails / implement host subscription**

In `AxisDriveHost` `useEffect` (when `sessionReady`):

```ts
const stop = await listenAgentEvents((frame) => {
  const kind = frame.kind?.type ?? 'agent_event';
  const turnId = stateRef.current.last_turn_id ?? 'pre-send';
  const text =
    typeof (frame as { text?: unknown }).text === 'string'
      ? (frame as { text: string }).text
      : undefined;
  const ev = appendDriveEvent(
    {
      events: stateRef.current.events ?? [],
      events_dropped: stateRef.current.events_dropped ?? 0,
      last_turn_id: stateRef.current.last_turn_id,
      next_seq: (stateRef.current.events?.at(-1)?.seq ?? 0) + 1,
    },
    { turn_id: turnId, kind, text, raw: frame },
  );
  stateRef.current = { ...stateRef.current, ...ev };
});
```

Prefer storing `next_seq` on `DriveState` explicitly (extend Task 1 types onto `DriveState`) to avoid fragile seq reconstruction.

- [ ] **Step 3: Run UI tests**

Run: `pnpm --dir crates/vox-gui/ui test src/lib/driveEvents.test.ts src/lib/useDriveBus.test.ts`

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/vox-gui/ui/src/components/drive/AxisDriveHost.tsx crates/vox-gui/ui/src/lib/axisDrive.ts
git commit -m "feat(gui): mirror agent events into Axis Drive state"
```

---

### Task 4: CLI `wait --until event=<kind>`

**Files:**
- Modify: `crates/vox-cli/src/commands/gui/client.rs`
- Modify: `contracts/gui/axis-drive.v1.yaml` (document predicate)

**Interfaces:**
- Consumes: nested `state.events[]`
- Produces: `matches_until("event=submit_ok", body) == true`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn matches_event_kind_nested_under_state() {
    let body = r#"{"status":200,"state":{"events":[{"kind":"token_streamed"},{"kind":"submit_ok"}],"last_error":null}}"#;
    assert!(matches_until("event=submit_ok", body));
    assert!(matches_until("event=token_streamed", body));
    assert!(!matches_until("event=missing", body));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-cli --features gui matches_event_kind_nested_under_state`

Expected: FAIL

- [ ] **Step 3: Implement matcher**

```rust
if let Some(kind) = until.strip_prefix("event=") {
    return view
        .get("events")
        .and_then(|e| e.as_array())
        .into_iter()
        .flatten()
        .any(|row| row.get("kind").and_then(|k| k.as_str()) == Some(kind));
}
```

Update `axis-drive.v1.yaml` comments / `cli_only_verbs` wait docs.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-cli --features gui matches_event_kind_nested_under_state`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/vox-cli/src/commands/gui/client.rs contracts/gui/axis-drive.v1.yaml
git commit -m "feat(cli): wait for Drive event kinds"
```

---

### Task 5: Physical OpenRouter Drive proof script

**Files:**
- Create: `scripts/axis-drive-openrouter-e2e.vox`
- Create: `docs/src/how-to/how-to-axis-drive-openrouter-e2e.md` (short how-to with frontmatter)

**Interfaces:**
- Consumes: `vox gui drive start|state|set|send|wait|stop`
- Produces: exit 0 only when assistant reply non-empty and `last_error` null

- [ ] **Step 1: Author the VoxScript**

Script responsibilities (exact argv may use `std.process` builtins per existing scripts):

1. `vox gui drive start` (optionally `--show` via env `AXIS_DRIVE_SHOW=1`).
2. Fetch `state` JSON; select first catalog row whose id contains `openrouter/` or provider looks cloud/OpenRouter and `selectable=true`.
3. Fail with clear message if none / if key likely missing (`last_error` after a probe send).
4. `set --knob model_override=<id>`.
5. `send --text 'Reply with exactly: openrouter-ok'`.
6. `wait --until reply --timeout 120s`.
7. Print summary JSON: `{model, last_error, assistant, event_kinds}`.
8. Exit non-zero if `last_error != null` or assistant empty.

- [ ] **Step 2: Physically run it**

Run: `vox run scripts/axis-drive-openrouter-e2e.vox`

Expected: PASS with assistant text; or FAIL with parseable `last_error` + events (fix secrets / routing until green).

- [ ] **Step 3: Commit**

```bash
git add scripts/axis-drive-openrouter-e2e.vox docs/src/how-to/how-to-axis-drive-openrouter-e2e.md
git commit -m "feat(scripts): Axis Drive OpenRouter e2e proof"
```

---

### Task 6: Spike + enable Metal QLoRA train dispatch

**Files:**
- Modify: `crates/vox-ml-cli/src/commands/schola/train/run_train.rs` (remove dead gate)
- Modify / verify: `crates/vox-plugin-mens-candle-metal/src/training.rs` and train entry wiring
- Test: add a unit/integration test that Metal device selection no longer hits the unconditional bail string when plugin path is available (feature/`cfg(target_os = "macos")` gated)

**Interfaces:**
- Consumes: Metal plugin `run_full_training(config_json)`
- Produces: `vox mens train --backend qlora --device metal --model Qwen/Qwen3.5-0.8B …` proceeds past the old bail

- [ ] **Step 1: Write the failing characterization test**

```rust
#[cfg(all(test, target_os = "macos"))]
#[test]
fn metal_qlora_error_is_not_the_old_dead_gate_message() {
    // Call the shared preflight/dispatch helper extracted from run_train
    // with DeviceKind::Metal and assert the error (if any) does NOT contain
    // "not supported yet: there is no Metal-enabled Candle training backend".
}
```

If extracting a helper is required, do that in the same task.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-ml-cli metal_qlora_error_is_not_the_old_dead_gate_message`

Expected: FAIL (still dead gate) or compile until helper exists

- [ ] **Step 3: Implement dispatch**

Replace the unconditional `anyhow::bail!("`--device metal` for Candle QLoRA is not supported yet…")` block with:

1. Ensure Metal plugin installed/loadable (mirror CUDA heal pattern if a Metal heal helper exists; otherwise clear install instructions).
2. Build `TrainRequest` JSON and call plugin `run_full_training`.
3. On missing plugin feature/`metal`, error must name `vox plugin install … mens-candle-metal` (exact install id from catalog).

- [ ] **Step 4: Smoke train one micro-step on Apple Silicon**

Run (adjust paths to dogfood JSONL):

```bash
vox mens train --backend qlora --tokenizer hf \
  --model Qwen/Qwen3.5-0.8B \
  --device metal \
  --data-dir <tiny-dogfood> \
  --output-dir mens/runs/qwen35-08b-metal-e2e \
  --max-steps 2
```

Expected: produces adapter/manifest under output dir (not the old dead-gate string).

- [ ] **Step 5: Commit**

```bash
git add crates/vox-ml-cli/src/commands/schola/train/run_train.rs crates/vox-plugin-mens-candle-metal
git commit -m "feat(mens): enable Metal QLoRA train via candle-metal plugin"
```

---

### Task 7: Mac Metal how-to + automation script

**Files:**
- Create: `docs/src/how-to/how-to-train-mens-macos-metal.md`
- Create: `scripts/mens-macos-metal-e2e.vox`
- Create: tiny dogfood JSONL under an allowed path (e.g. `examples/mens/dogfood-metal-e2e.jsonl`) if none exists

**Interfaces:**
- Consumes: Task 6 train path; `vox mens serve` / `vox schola serve`
- Produces: run dir + serve instructions + Drive pin `mens/qwen35-08b-metal-e2e`

- [ ] **Step 1: Write how-to with required frontmatter**

Include: Apple Silicon requirement, plugin install, download, train, serve, Drive pin, troubleshooting tokenizer missing.

- [ ] **Step 2: Write `scripts/mens-macos-metal-e2e.vox`**

Steps: ensure plugin → download model → train short run → print `vox mens serve --model <run_dir>` and expected Drive `model_override`.

- [ ] **Step 3: Run automation on Mac**

Run: `vox run scripts/mens-macos-metal-e2e.vox`

Expected: artifacts on disk; serve health identifies models including the e2e slug or run-dir name documented as the pin.

- [ ] **Step 4: Commit**

```bash
git add docs/src/how-to/how-to-train-mens-macos-metal.md scripts/mens-macos-metal-e2e.vox examples/mens/dogfood-metal-e2e.jsonl
git commit -m "docs(mens): Mac Metal Qwen3.5-0.8B train→serve pipeline"
```

---

### Task 8: Physical Metal Drive proof

**Files:**
- Create: `scripts/axis-drive-metal-e2e.vox`

**Interfaces:**
- Consumes: serving local pack from Task 7; Drive event dump from Tasks 1–4
- Produces: exit 0 iff local pin returns non-empty assistant bubble

- [ ] **Step 1: Author script**

1. Assert VoxLocal/serve reachable (probe via Drive `state.probe` or HTTP health).
2. `drive start` → `set model_override=mens/qwen35-08b-metal-e2e` (or documented served id).
3. `send` short prompt → `wait --until reply`.
4. Dump `event_kinds`; fail on tokenizer/`last_error`.

- [ ] **Step 2: Physically run**

Run: `vox run scripts/axis-drive-metal-e2e.vox`

Expected: PASS with assistant text

- [ ] **Step 3: Commit**

```bash
git add scripts/axis-drive-metal-e2e.vox
git commit -m "feat(scripts): Axis Drive Mac Metal e2e proof"
```

---

### Task 9: Spec acceptance sweep

**Files:** none required unless gaps found

- [ ] **Step 1: Check every acceptance box in the design spec §9**
- [ ] **Step 2: Confirm headless claims do not set `events: true`**
- [ ] **Step 3: Final commit only if docs/scripts need a status note**

```bash
git commit -m "docs: record Axis chat e2e acceptance evidence" # only if adding evidence notes
```

---

## Spec coverage self-review

| Spec requirement | Task |
|---|---|
| Event ring + caps + redaction | Task 1 |
| submit_ok/submit_err on send | Task 2 |
| Full agent/stream frames in Drive | Task 3 |
| CLI wait `event=` | Task 4 |
| OpenRouter any-selectable green | Task 5 |
| Remove Metal dead gate / train path | Task 6 |
| Documented Mac download→train→serve | Task 7 |
| Drive pin `mens/qwen35-08b-metal-e2e` green | Task 8 |
| Acceptance checklist | Task 9 |

No TBD/TODO placeholders remain in this plan. Types (`DriveTurnEvent`, ring caps, slug, model id) are consistent across tasks.
