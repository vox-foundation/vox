---
title: "Axis Drive — Terminal Control Plane for Chat Honesty"
description: "Dedicated debug Axis plus a localhost drive bus so agents can set composer knobs, pick models, send, and read picker/catalog truth without touching the user's window."
category: "architecture"
status: "design"
date: 2026-09-08
---

# Axis Drive Design

## 1. Problem

An agent cannot Send, change composer knobs, or verify picker honesty in a live Axis window. WKWebView is not Accessibility-automatable. Playwright e2e mocks Tauri. `orch.tool_call` / `vox_chat_message` on `:9745` bypasses the shell, so a green RPC does not prove picker selectability, pin state, or VoxLocal probe.

The operator needs a first-party terminal path that can:

1. Send a chat turn through the same submit path as a click (`handleLoquelaSubmit` → `buildChatTurn` → `chat_turn`).
2. Set every send-changing composer knob by mutating **authoritative React state** (App + Loquela), not a parallel mirror.
3. Read picker/catalog truth (selectable vs disabled, probe, pin).

## 2. Locked decisions

| Decision | Choice |
|---|---|
| Control-plane meaning | **Both** planes: live-window AxisDriveHost **and** headless handler path |
| Axis presence (live) | **Always launch a dedicated debug Axis.** Never attach to the window the user is looking at |
| Visibility | Hidden by default; `--show` (or `vox gui drive show`) to watch |
| v1 surface | Chat send + every send-changing knob + picker/catalog truth |
| Transport | Loopback HTTP/JSON + per-session **file** token. CLI: `vox gui drive …` |
| Name | **Axis Drive**. Not “Drive Console” (that is the clutch/cost/risk strip in Loquela) |
| Pin isolation | Drive **skips `localStorage` read** of `vox_chat_model.v1`. Do **not** claim `$HOME` / `VOX_GUI_DATA_DIR` isolates WKWebView storage |
| Transcript isolation | Override workspace journey store to `~/.vox/gui-drive/<profile>/store.db` |
| Liveness | Session `pid` is a hint. Adopt/refuse/`stop` only after **bearer ping** on the recorded loopback port |
| `ready` | `false` until AxisDriveHost registers. Never flip true at bind |
| Headless claims | `plane: "headless"` plus `claims: { picker_ui: false, composer_knobs: false, bubbles: false }` |
| `pin_policy: fail` | **Drive-only**, stricter than the click path (click does not 409 an unselectable pin) |

## 3. Non-goals (v1)

- Attaching to, inspecting, or driving the user's everyday Axis process
- Playwright/CDP as the agent control plane (visual e2e may stay as they are)
- Changing `vox chat` (that is raw `llm_chat`, not orchestrator/GUI)
- Mission Control panel, `ClutchProfile` scorer wiring, model attestation / `io_digest`
- Training / Metal / 32B work
- LAN bind, mDNS, or any non-loopback listen
- Free-form JS eval in the webview
- New workspace crate or crate-edge exception (`vox-cli` must not depend on `vox-gui`)
- Regenerating SSOT by hand after merge (`ssot-autoregen` owns that)
- Separate macOS bundle identifier for WebKit store isolation (follow-up)
- Passing the drive bearer in the child environment (`VOX_GUI_DRIVE_TOKEN` is forbidden — `ps` leak)

## 4. Relationship to existing surfaces

| Surface | Role after this spec |
|---|---|
| Drive Console (`contracts/gui/drive-console.v1.yaml`, `DriveConsole.tsx`) | Unchanged presentational clutch/cost/risk strip. Axis Drive **imports** clutch/risk enums from that YAML (parity test); it does not fork them |
| `chat_turn` / `buildChatTurn` | The only submit path AxisDriveHost may call |
| App `onSubmit` wrapper | Overwrites `model_override` and `grounding_check_enabled` from App state. Drive `send` must go through this wrapper |
| Loquela local state | Owns `tier`, `clutch`, `risk`, `mode`, `dry_run`, `execution`. Drive `set` must call Loquela setters via ref |
| Orchestrator daemon `:9745` | Debug Axis may still adopt it for real inference. Honesty tests **must not** call it instead of Drive |
| `vox chat` | Unchanged; out of scope |

## 5. Architecture

Two planes, one command vocabulary (`set`, `send`, `state`, `wait`):

| Plane | Process | What it may claim |
|---|---|---|
| **Live** | Dedicated `vox-gui --drive`, hidden unless `--show` | Composer knobs, picker honesty, bubbles, last error, orch freshness |
| **Headless** | `vox-gui --drive-headless` one-shot (spawned by the CLI) | Rust `chat_turn` / catalog / probe JSON only. Must set `plane: "headless"` and `claims.*.false` |

Live process:

1. `vox gui drive start` acquires `gui-drive.lock` (`flock`), writes a `0600` token file, launches a **second** `vox-gui --drive` (detached — not `gui.rs` `child.wait()`). Child reads the token from the file or `--token-fd`.
2. If a session already answers a **bearer ping** on the recorded port, `start` **refuses** (exit 2). It never attaches. `pid` is not used as the sole liveness check (zombies, reuse, Windows).
3. React mounts `AxisDriveHost` (not extra wiring dumped into `App.tsx`). `set` calls real App + Loquela setters. `send` calls the existing `onSubmit` wrapper → `handleLoquelaSubmit` → `buildChatTurn` → `chat_turn`.
4. `state` snapshots knobs from those same objects, pin, catalog rows (`id`, `selectable`, `reason`), VoxLocal probe, last bubbles, last error, orch freshness.
5. Agents use only `vox gui drive …`. They do not call `:9745` for honesty checks.

Auth: loopback bind `127.0.0.1` only, ephemeral port, token on every `/v1/*` request (`constant_time_eq`). `/health` is unauthenticated and minimal (`service`, `ready`). Process exit closes the port. CLI never prints the token; it prints the session path.

Listener runs on `tauri::async_runtime::spawn`. Window `show` uses `run_on_main_thread`.

## 6. Components

### 6.1 Drive session (`vox gui drive start|stop|show`)

Writes `~/.vox/run/gui-drive.json` (mode `0600`, dir `0700`):

```json
{
  "schema_version": 1,
  "pid": 12345,
  "port": 49152,
  "token_path": "/Users/…/.vox/run/gui-drive.token",
  "store_path": "/Users/…/.vox/gui-drive/default/store.db",
  "show": false
}
```

`token_path` is written by the parent and **never honored from untrusted JSON** on later reads — the path is always `$VOX_HOME/run/gui-drive.token` (or `~/.vox/run/…`). Open the token with `O_NOFOLLOW` on Unix.

`stop` sends SIGTERM only after a failed-or-succeeded bearer ping confirms the port still serves Axis Drive; pid is a hint for the signal target and must match a `vox-gui --drive` cmdline when `sysinfo` is available.

`show` asks the live session to reveal the window (`title`: `Axis (drive)`). Isolation:

- Skip `localStorage` **read** of `vox_chat_model.v1` (and sibling chat keys) in the drive process. Pin lives in React memory until `set`.
- Journey store → `~/.vox/gui-drive/<profile>/store.db` so drive `chat_turn` does not append to the user's transcript.
- Do **not** claim WKWebView `localStorage` is isolated (same bundle id `org.vox-foundation.gui`).

Acceptance: user Axis pin is X; drive `state.pin` is unset until `set`.

### 6.2 Drive contract

SSOT: `contracts/gui/axis-drive.v1.yaml`. Listener verbs: `health`, `set`, `send`, `state`, `show`. `wait` is CLI-only (polls `state`). No eval verb.

`clutch` / `risk` values are imported from `contracts/gui/drive-console.v1.yaml` (parity test). Do not fork those enums.

`set` keys (snake_case):

| Key | Values | Authority |
|---|---|---|
| `model_override` / `pin` | model id string | App `setChatModelOverride` |
| `pin_policy` | `fail` (default) or `coerce` | Drive-only; `fail` is stricter than click |
| `execution` | `sync` \| `background` \| `plan` | Loquela; `plan` is slash-equivalent, not a composer toggle |
| `tier` | **string** — `local` \| `mesh` \| `cloud` \| `auto` **or** a runtime `model_id` from `listModels` | Loquela |
| `clutch` | `free` \| `efficiency` \| `balanced` \| `genius` | Loquela (Drive Console SSOT) |
| `risk` | `high` \| `moderate` \| `low` | Loquela (Drive Console SSOT) |
| `grounding_check_enabled` | bool | App `setGroundingCheckEnabled` |
| `active_skill` | string or null | App |
| `skill_exclusions` | string list | App |
| `session_id` / `chat_session_id` | string | App; background send still uses `newBackgroundSessionId()` for `session_id` |
| `priority` | string | Intent fold (`effortToPriority`); payload injection |
| `dry_run` | bool | Loquela |
| `allow_duplicate` | bool | Handler currently hardcodes `false` except retry |
| `mode` | `plan` \| `act` \| `verify` | Loquela; **ignored on the sync daemon wire** |
| `context_files` | string list | Loquela chips |
| `refresh_catalog` | bool (action) | Re-invoke `list_model_cards` + `inference_provider_status` |

Unknown keys → `400 unknown_key`. `pin_policy: fail` + unselectable model → `409 model_not_selectable` with `reason`.

`state.catalog` comes from **`list_model_cards` + `inference_provider_status`** (same as the picker), not `listModels` alone. Empty status list is **fail-closed** on the Drive plane (not the picker fail-open).

### 6.3 Drive listener (Rust, live Axis only)

HTTP/JSON on `127.0.0.1:<ephemeral>`. `/v1/{set,send,state,show}` requires `Authorization: Bearer <token>` compared with `constant_time_eq`. `/health` is unauthenticated and returns `{ "service": "vox-gui-drive", "ready": bool }` only. `ready` is true only after AxisDriveHost has registered.

### 6.4 AxisDriveHost (React)

The only live-plane applicator. Mounted as `<AxisDriveHost />` from App (thin). Reads shared picker honesty helpers extracted from `ChatModelPicker` / `modelPicker.ts`. `send` must not call `orch.tool_call`. `set` must invoke injected setters, not only a `DriveState` mirror.

### 6.5 Headless adapter

`vox gui drive headless <verb>` spawns `vox-gui --drive-headless` with a JSON request on stdin, JSON response on stdout, no webview. Response **always** includes `"plane": "headless"` and `claims: { picker_ui: false, composer_knobs: false, bubbles: false }`. Live honesty criteria 2–4 are live-only.

### 6.6 CLI

```text
vox gui drive start [--show] [--profile <id>]
vox gui drive show
vox gui drive stop
vox gui drive set --knob model_override=mens/e2e-smoke --knob execution=sync
vox gui drive send --text "ping"
vox gui drive state
vox gui drive wait --until reply|reply_ok|error|event=<kind>|selectable=<id> [--timeout 90s]
vox gui drive headless set|send|state …
```

`vox gui --command <view>` stays the launch path (optional `GuiCmd`; `DriveArgs` live in `cli_args.rs` only).

Catalog: **hand-author** `gui` + `gui.drive.*` rows in `contracts/operations/catalog.v1.yaml` with `feature_gate: gui`, then `vox ci operations-sync --target cli --write` and `vox ci command-sync --write`. Clap does not invent catalog rows.

Every new `VOX_*` env var is registered in `contracts/config/registry.v1.yaml`, `CONFIG_KEYS`, and `contracts/config/env-vars.v1.yaml`. Child must not receive `VOX_GUI_DRIVE_TOKEN`.

## 7. Data flow

### Live send

```text
CLI  --POST /v1/send-->  listener (async runtime)
                         --> emit drive://request {id, verb:send, body}
AxisDriveHost  --> App onSubmit wrapper (pin + grounding from App state)
               --> handleLoquelaSubmit → buildChatTurn → chat_turn
               --> emit drive://response {id, state snapshot}
CLI  <-- JSON { plane:"live", turn, state }
```

### Live set + state

`set` calls App + Loquela setters. If `refresh_catalog`, re-invoke picker catalog/probe commands. `state` is a structured snapshot, not a DOM dump.

### Headless send

```text
CLI  --stdin JSON-->  vox-gui --drive-headless
                      --> chat_turn / probe / catalog (no webview)
CLI  <--stdout JSON { plane:"headless", claims:{…false}, … }
```

### Wait

CLI polls `state` (live) or blocks on the one-shot (headless). Default timeout 90s (`PENDING_TIMEOUT_MS`). `wait --until selectable=mens/e2e-smoke` succeeds when that catalog row is `selectable: true`; `reply` accepts any settled assistant reply including errors, while `reply_ok` requires a non-empty, non-error assistant reply; `event=<kind>` matches `state.events[].kind`.

## 8. Error handling

| Case | Behavior |
|---|---|
| `start` while bearer ping succeeds | Exit 2; print port + session path; do not attach |
| `start` session file exists, ping fails | Replace file; start fresh |
| Listener not `ready` within 15s | `start` fails; SIGTERM child; delete session + lock |
| Missing or wrong token | `401` |
| Non-loopback peer | Connection refused (bind is `127.0.0.1` only) |
| Unknown `set` key | `400 unknown_key` |
| Unselectable pin and `pin_policy=fail` | `409 model_not_selectable` (Drive-only; click path does not 409) |
| Empty `send` text | `400 empty_text` |
| Sync send already in flight | `409 send_in_flight` |
| No session for live verb | Exit 1: “no drive session; run vox gui drive start” |
| Headless used for picker proof | Forbidden: consumers must check `plane` and `claims` |

Do not log or print the bearer token. Doctor/debug output may say `token_path` only.

## 9. Security

- Bind `127.0.0.1` only. Loopback is not a multi-user boundary; the bearer file is.
- Token: 32 cryptographically random bytes, hex, file mode `0600`, deleted on `stop`. Fail closed if RNG fails. No timestamp fallback. No env bearer.
- Journey-store override so drive does not write the user's chat DB.
- Skip localStorage pin **read** in the drive process.
- No JS eval, no arbitrary Tauri invoke (allowlisted verbs only).
- Dedicated process only — never inject into the user's webview.
- `constant_time_eq` for bearer compare (same as orchestrator daemon).

## 10. Testing

| Layer | What |
|---|---|
| Contract | Typed YAML deserialize; clutch/risk match `drive-console.v1.yaml`; `execution` round-trips onto `state.knobs.execution` |
| CLI clap | `vox gui --command chat` still parses; drive subcommands parse; `--features gui`; `#[cfg(feature = "gui")]` on root tests |
| Session | Lock; token `0600` not in JSON; second start refuses on successful ping; no `kill(0)` as sole check |
| Listener | Missing **and wrong** token → 401; `/health` has `service: vox-gui-drive` only; unknown verb → 404 |
| AxisDriveHost (vitest) | `set model_override` calls `setChatModelOverride`; `send` maps `sync→chat`; unselectable + `fail` → 409; **mutation**: delete the check → test fails |
| Headless | `plane` + `claims` all false; empty text → `empty_text` |
| Forbidden | No new Playwright spec that claims to be the drive plane; no `cargo test -p vox-gui --lib` |

Live mens serve / Metal is **not** required for unit tests.

## 11. Implementation sketch (files)

Create:

- `contracts/gui/axis-drive.v1.yaml`
- `crates/vox-gui/src/drive/{mod,protocol,listener,headless,flags}.rs`
- `crates/vox-cli/src/commands/gui/{mod.rs,launch.rs,drive.rs,session.rs,client.rs}`
- `crates/vox-gui/ui/src/lib/axisDrive.ts` + test
- `crates/vox-gui/ui/src/lib/useDriveBus.ts` + test
- `crates/vox-gui/ui/src/components/drive/AxisDriveHost.tsx`

Modify:

- `crates/vox-cli/src/cli_args.rs` — `DriveArgs` / `GuiCmd` here only
- `crates/vox-cli/src/commands/gui.rs` → `gui/` module
- `crates/vox-gui/src/main.rs` — `--drive`, `--drive-headless`, `--show`, store override, window visibility/dock
- `crates/vox-gui/ui/src/App.tsx` — render `<AxisDriveHost />` only
- `contracts/operations/catalog.v1.yaml` — **hand-author** `gui` + `gui.drive.*` first
- `contracts/config/{registry,env-vars}.v1.yaml` + `CONFIG_KEYS`
- `docs/src/architecture/where-things-live.md` — Axis Drive row
- Generated CLI docs via `command-sync --write`

No new crate. Format with `cargo fmt -p vox-cli` / `cargo fmt -p vox-gui`, never `cargo fmt --all`.

## 12. Success criteria

1. `vox gui drive start` opens a second Axis that is hidden, listed in the session file, and answers `/health` with `ready: true` only after AxisDriveHost mounts.
2. `set` then `state` shows pin and knobs **from App + Loquela state**, not a disconnected mirror.
3. An unselectable local pin with `pin_policy=fail` returns `409` and `state.catalog` shows `selectable: false`.
4. Live `send` produces a snapshot whose last user bubble matches the text, via `handleLoquelaSubmit`.
5. Headless `send` returns `plane: "headless"` and `claims.picker_ui === false` without opening a window.
6. The user's everyday Axis pin, window, and chat DB are unchanged.

Criteria 2–4 are **live-only**. A green headless test does not satisfy them.

## 13. Open follow-ups (not v1)

- Separate bundle identifier / WebsiteDataStore for true WebKit isolation
- Optional Playwright visual check against the `--show` window
- Doctor check: “drive session stale”
- MCP wrapper `vox_gui_drive_*`
