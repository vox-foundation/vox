---
title: "Terminal Session Host (daemon-attached, CLI/GUI seamless switching)"
description: "Design for hosting vox-terminal-core sessions in vox-orchestrator-d so the vox-term TUI and the Axis GUI attach to the same live session, with an in-process fallback for headless builds."
category: "Architecture SSOTs"
status: "roadmap"
training_eligible: true
training_rationale: "Architecture design with verified current-state audit and protocol contract."
---

# Terminal Session Host (2026-10-08)

## Decision context

Option D from the 2026-10-08 Warp evaluation: no Warp fork. The renderers stay as
they are (Tauri/React + xterm.js in Vox Axis (Axis), ratatui in `vox-term`).
`vox-terminal-core` is the one terminal engine, and Warp is a clean-room design
reference only, per [ADR-026](../adr/026-third-party-code-provenance.md). This page
adds the piece the [Warp-parity plan](../../superpowers/plans/2026-06-20-vox-terminal-warp-parity.md)
lacked: a **session host** that lets both front-ends attach to the same live session.
That gives seamless CLI↔GUI switching and a single shared context between the
terminal and the GUI chat.

Why not a GPU-rendered UI or an AGPL Warp fork: see §8.

## Goals

1. **One source of truth.** A terminal session (PTY, block list, OSC-633 state,
   agent messages) lives in exactly one place. Renderers display it; they never own it.
2. **Seamless switching.** A session opened in Axis can be attached from `vox term`
   (and the reverse) with the same blocks, scrollback and running process.
3. **Shared context both ways.** GUI chat reads terminal blocks as context. Agents
   drive the terminal through the same session the human sees, gated by the
   existing approval path.
4. **Headless / small builds keep working.** `vox-term` runs with no daemon (in-process host).
5. **GPU-accelerated rendering** in the GUI terminal pane, with no new toolchain.

Non-goals: surviving a daemon restart (PTYs die with their host process), remote/mesh
attach, and multi-user sessions. See §7.

## 1. Current state (verified 2026-10-08)

| Piece | Reality |
|---|---|
| GUI live terminal path | `transport.ts` → `pty_spawn` (`crates/vox-gui/src/commands/pty.rs`, `PtyManager`) spawns the PTY **inside the GUI process** and emits raw bytes to xterm.js. Blocks come from the **TypeScript** reducer `Console/osc633.ts`. |
| GUI core path | `commands/terminal_core.rs` (`TerminalSessionManager`, `term_pty_bytes` / `term_get_blocks` / `term_submit`) is registered in `main.rs` but **has no front-end caller**. `term_submit` only classifies the input. |
| `vox-term` | Owns its own in-process `Session` (`vox-terminal-core::session::Session`). |
| `Session` API | `new`, `subscribe() -> broadcast::Receiver<SessionEvent>`, `on_pty_bytes`, `blocks`, `submit(InputIntent)`. `SessionEvent` = `BlockOpened`/`OutputAppended`/`BlockClosed`/`AgentMessage`. |
| Daemon transport | `vox-orchestrator-d`: line-JSON `DispatchRequest`/`DispatchResponse` over loopback TCP (`127.0.0.1:9745`, auth token required) or `stdio`. The GUI already uses `OrchDaemonClient` (`control_plane.rs`, `mission_control.rs`, …). |
| Daemon extension point | `orch_daemon::ExtraDispatch::try_handle`. **Request/response only.** Push streams are hard-coded to `SUBSCRIBE` / `SUBSCRIBE_EVENTS` in both serve loops (`orch_daemon/mod.rs`). Only **one** `extra` is wired (MCP's `McpExtraDispatch`). |
| xterm.js | `@xterm/xterm` 6 + `@xterm/addon-fit`; **no WebGL addon** (DOM renderer). |

Net: two terminal engines (TS reducer and Rust core) and a PTY whose lifetime is the
GUI window. Both have to be fixed before switching between front-ends can work.

## 2. Architecture

```text
             ┌──────────── vox-orchestrator-d ────────────┐
             │ Orchestrator  ◄── agents / chat / MCP tools │
             │      ▲                                     │
             │      │ transcript + term_run (approval)    │
             │  SessionHost (DaemonSide)                  │
             │   Session{PTY, blocks, osc633, ring buf}   │
             └──────▲──────────────────────────▲──────────┘
       term.* (TCP/stdio, token)        term.* (TCP)
                    │                          │
          vox-term (DaemonHost)        Axis GUI (DaemonHost)
          ratatui + alacritty grid     React + xterm.js (WebGL)

 No daemon:  vox-term ──► LocalHost (same Session, in-process)
```

### 2.1 `SessionHost` trait (in `vox-terminal-core`)

> **Status:** `LocalHost` shipped as a concrete struct (H1/H2, `crates/vox-terminal-core/src/host.rs`).
> The trait below is extracted at H5, when `DaemonHost` gives it a second implementation.
> `attach` returns blocks + `replay` bytes + `seq` + two live receivers (session events, raw output frames).

```text
trait SessionHost: Send + Sync {
    async fn open(&self, spec: OpenSpec) -> Result<SessionId>;          // shell, cwd, cols, rows
    async fn list(&self) -> Result<Vec<SessionInfo>>;
    async fn attach(&self, id: &SessionId) -> Result<Attachment>;        // Snapshot + event stream
    async fn input(&self, id: &SessionId, bytes: &[u8], origin: Origin) -> Result<()>;
    async fn submit(&self, id: &SessionId, line: &str, origin: Origin) -> Result<BlockId>;
    async fn resize(&self, id: &SessionId, cols: u16, rows: u16) -> Result<()>;
    async fn close(&self, id: &SessionId) -> Result<()>;
}
```

There are two implementations, and both run the same `Session` code:

- **`LocalHost`** holds sessions in-process. It absorbs the PTY spawn/read loop that
  lives in `vox-gui/src/commands/pty.rs` today, so that file shrinks to Tauri glue.
  `vox-term` uses it when no daemon answers, and the GUI falls back to it the same way.
- **`DaemonHost`** is a thin client that maps each method to a `term.*` request
  through `OrchDaemonClient`. It adds no dependencies: `vox-terminal-core` already
  depends on `vox-orchestrator`.

The daemon side is a `LocalHost` exposed over the wire. That leaves one
session-management implementation, not two.

### 2.2 Session additions

- **Replay ring buffer.** `Session` keeps the last N raw PTY bytes (default 2 MiB,
  configured through `vox-config`). On attach, the renderer re-parses them through its
  own VT engine (xterm.js / `alacritty_terminal`), so full-screen programs (vim,
  htop, the alternate screen) come back correctly without the host serializing a
  screen grid. The block list carries the structure; the ring buffer carries the pixels.
- **Sequence numbers.** Each output frame carries a monotonically increasing `seq`.
  When a client sees a gap (broadcast lag), it re-attaches and gets a fresh snapshot.
  There is no per-frame acknowledgement.
- **`Origin` on input and blocks** (`User` | `Agent{agent_id}`), so the UI can mark
  agent-typed commands and the transcript/MENS corpus can label them.
- **Resize policy:** the last client to send input or a resize owns the size.
  `ponytail:` single-owner sizing; adopt tmux-style smallest-client sizing only if
  side-by-side attached clients become common.

### 2.3 Wire protocol (`vox_foundation::protocol::term_method`)

| Method | Kind | Params → Result |
|---|---|---|
| `term.open` | req | `{shell?, cwd?, cols, rows}` → `{session_id}` |
| `term.list` | req | `{}` → `[{session_id, title, cwd, created_ms, attached}]` |
| `term.attach` | **stream** | `{session_id}` → first frame `Snapshot{blocks, replay_b64, cols, rows, seq}`, then `Output{seq, bytes_b64}` / `Block{opened\|closed, block}` / `Agent{text}` / `Exit{code}` |
| `term.input` | req | `{session_id, bytes_b64, origin}` → `{}` |
| `term.submit` | req | `{session_id, line, origin}` → `{block_id}` (goes through `classify`) |
| `term.resize` | req | `{session_id, cols, rows}` → `{}` |
| `term.close` | req | `{session_id}` → `{}` |

Output bytes are coalesced per frame (a new method, so unlike `SUBSCRIBE_EVENTS`
there is no existing one-event-per-frame contract to preserve).

### 2.4 Daemon changes (small, additive)

1. **A streaming hook on `ExtraDispatch`:**
   `async fn try_stream(&self, req: &DispatchRequest, sink: &mut FrameSink) -> Option<anyhow::Result<()>>`,
   defaulting to `None`. Both serve loops (TCP and stdio) call it before
   `dispatch_one_framed`, next to the existing `SUBSCRIBE*` branches. The wire format
   doesn't change.
2. **Chaining extras:** a `ChainDispatch(Vec<Arc<dyn ExtraDispatch>>)` that takes the
   first `Some`. The daemon binary wires `[McpExtraDispatch, TermDispatch]`.
3. **`TermDispatch`** lives in the daemon crate (`crates/vox-orchestrator-d/src/term_dispatch.rs`) and
   wraps a `LocalHost`. It is there, not in `vox-terminal-core`, because the wire types
   (`DispatchRequest`/`DispatchResponse`) live in `vox-foundation` and the core crate has no edge to it;
   this keeps the total at **one** new edge, `vox-orchestrator-d → vox-terminal-core` (L5 → L3).
   That edge is in the `crate-edges` exceptions ledger, approved by the owner on 2026-10-08.
   It is registered only on loopback/stdio binds (§5).

## 3. Seamless switching (UX)

- **`vox term`:** `orch.ping` the daemon. If it answers, use `DaemonHost` and offer
  `term.list` sessions in the palette. Otherwise use `LocalHost`. `vox term --attach <id>`
  attaches directly, and `--local` forces in-process.
- **Axis GUI:** each Console tab is a `DaemonHost` session. A tab action,
  "Continue in terminal", copies `vox term --attach <id>`. The Console session picker
  lists sessions opened from `vox term` as well.
- **Detach is free:** closing a GUI tab or quitting `vox term` detaches the client;
  `term.close` is explicit. Orphaned sessions show up in `term.list` (shown as
  `attached: 0`) and are reaped after a configurable idle TTL.

## 4. Chat ↔ terminal

- **Terminal → chat (context).** The orchestrator subscribes to each session's block
  events in-process (it runs in the same daemon). The chat composer's context bundle
  includes recent blocks (`plain_output()`, redacted via `vox-redact`) for sessions
  linked to the chat. There's no extra round-trip: this replaces the TS-side
  `renderBlockForAgent` in `Console.tsx`.
- **Chat → terminal (drive).** Add a tool, `term_run{session_id, line}`, to the
  orchestrator/MCP tool surface. It routes through the **existing MCP approval gate**
  and `vox shell check` exec policy, then `term.submit` with `Origin::Agent`. The
  human sees the command appear live in whichever front-end is attached, tagged as
  agent-originated.
- **Human → agent.** The existing `/ai` / `?` input intents (`InputIntent::Agent`)
  produce `AgentMessage` events on the same session, so either front-end can hold the
  conversation.

## 5. Security

- `term.*` is arbitrary code execution as the daemon user. It is **never** served
  without the daemon auth token, and it is **refused on non-loopback binds**, even with
  an explicit token, until remote attach is designed (§7).
- Agent input never reaches a PTY except through `term_run` and the approval gate (H8). Over the
  wire, `origin: "agent"` is **refused on both `term.input` and `term.submit`** (a client could
  otherwise claim any label); the in-process `term_run` path is the only way to act as an agent. Human raw keystrokes (`Origin::User`)
  go through ungated, as in any terminal.
- The replay buffer can hold secrets typed or printed in the terminal. It stays in
  daemon memory and is never persisted. The transcript/corpus sink keeps redacting as
  it does today.

## 6. Rendering and builds

- **GUI:** add `@xterm/addon-webgl` to `TerminalTab.tsx`, falling back to the DOM
  renderer on `onContextLoss`. This is the VS Code approach: GPU text rendering for the
  one surface where throughput matters, with the rest of Axis staying web UI.
- **CLI-only / headless:** `vox` + `vox-term` with no Tauri. `LocalHost` needs no
  daemon; `DaemonHost` is a client only. No new dependencies or cargo features, and
  the build-toolchain invariant is unchanged (WebGL comes from the webview, not a
  native GPU stack).
- **Engine retirement:** once the GUI consumes `term.attach`, delete `Console/osc633.ts`
  (+ tests) and `TerminalSessionManager`. This is Track 3 of the parity plan. The
  Rust parser becomes the only OSC-633 implementation.

## 7. Open questions / later

| Item | Default now | Revisit when |
|---|---|---|
| Session survives a daemon restart | No (PTYs die) | Users rely on long-lived sessions; would need a separate `vox-term-host` process per session (tmux-style) |
| Remote / mesh attach | Refused (loopback only) | A mesh use case exists; needs per-session capability tokens, not the daemon token |
| Resize with several attached clients | Last-active wins | Side-by-side attach becomes common |
| Ring buffer size | 2 MiB | Measured memory pressure with many sessions |

## 8. Alternatives considered (2026-10-08)

- **AGPL Warp fork as a separate binary (B).** Warp has its own PTY/block/session
  model, which would make a second source of truth. Its Agent Mode/Drive call Warp's
  closed backend, so routing them through `vox_actor_runtime::llm` means
  permanently carrying patches in Warp's highest-churn code (`app/ai`). A Warp window
  can't embed in the Tauri webview, so switching wouldn't be seamless. On top of that,
  it splits the product licence. Rejected.
- **GPU-rendered UI on MIT `warpui` (C).** It wins on frame rate, latency and memory
  for a terminal-centric product. It loses most of Axis: chat/markdown/dashboard
  widgets, text shaping/IME, accessibility, Playwright visual verification and the
  design-token pipeline. `warpui` is also an internal framework with no stability
  promise. Rejected for Axis; GPU rendering is applied to the terminal pane only (§6).
- **Warp as reference.** Blocks, input classification, command signatures and
  Agent Mode UX are studied clean-room (gate G-WARP of the parity plan).
  `warpui_core`'s TUI cell-grid (MIT) may be read as a reference for `vox-term`.

## 9. Implementation tasks (TDD order)

Each task starts with its failing test (AGENTS.md §Test-First).

1. **H1. `SessionHost` + `LocalHost`.** Move the PTY spawn/read loop from
   `vox-gui/src/commands/pty.rs` into `vox-terminal-core`. *Test:* `open` → `submit("!echo hi")`
   → a block with output `hi`; `attach` on a second handle sees the same block.
2. **H2. Replay ring buffer + `seq`.** *Test:* write 3 MiB to a 2 MiB buffer and get back
   the last 2 MiB; the snapshot `seq` equals the last emitted `seq`.
3. **H3. `ExtraDispatch::try_stream` + `ChainDispatch`.** *Test:* a stream-handling extra
   receives `term.attach` over a real loopback TCP serve loop; a non-stream request
   still reaches `McpExtraDispatch`. Mutation check: delete the `try_stream` call in
   one serve loop and confirm the test fails.
4. **H4. `TermDispatch` + `term_method` constants.** *Test:* each method round-trips
   over `run_tcp_server_with_extra`; a request without a token is rejected;
   `Origin::Agent` on `term.input` is rejected.
5. **H5. `DaemonHost`.** *Test:* the H1 scenario passes unchanged with `DaemonHost` against
   an in-test daemon. This is the parity proof between the two hosts.
6. **H6. GUI on `term.attach`.** Replace the `pty_spawn` path; add `addon-webgl`; retire
   `osc633.ts` and `TerminalSessionManager` (merges with Track 3). *Test:* vitest with a
   mocked snapshot renders blocks; a Playwright spec on `installTauriMock` captures the
   Console with the WebGL renderer into `review-bundle/latest/`.
7. **H7. `vox term` auto-detect / `--attach` / `--local`.** *Test:* with no daemon it
   falls back to `LocalHost`; with `TERM=dumb` it still starts.
8. **H8. `term_run` tool + chat context bundle.** *Test:* an agent `term_run` waits on a
   pending approval and opens no block until resolved; after approval, the block's
   origin is `Agent`.
