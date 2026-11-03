---
title: "Browser operator HITL — Design Spec"
description: "Per-call CallerContext, exclusive tab lock, Axis park/yield, model screenshot mute, and sensitive-verb confirm — Operator-class takeover on the existing chromiumoxide GUI path."
category: "Architecture SSOTs"
status: "roadmap"
training_eligible: true
training_rationale: "Contract so Axis cannot fight a human-locked tab; yield parks the turn; screenshots to the model mute during takeover."
---

# Browser operator HITL — Design Spec

**Date:** 2026-09-08
**Parent program:** [`2026-09-07-agent-browser-driver-design.md`](./2026-09-07-agent-browser-driver-design.md)
**Does not replace:** [`2026-09-08-chat-harness-research-loop-design.md`](./2026-09-08-chat-harness-research-loop-design.md) (pixels to the model). Implement that plan first; this spec must not grow its tasks.
**GUI SSOT:** [`docs/src/architecture/vox-gui-browser-support-2026.md`](../../src/architecture/vox-gui-browser-support-2026.md)
**Soft HITL (do not reuse):** [`2026-06-19-attention-aware-soft-hitl-design.md`](./2026-06-19-attention-aware-soft-hitl-design.md) — NeedsYou / `pending_approvals` stay shell/task feedback. Browser park is a **daemon-local waiter**.

This spec is the contract. Executors do not invent types, env vars, crate edges, or tool names that are not named here.

## 0. Goal

On the live Axis + GUI path, the operator always wins the tab. Axis chat cannot mutate a You-locked page even though the GUI daemon is spawned with `VOX_MCP_CALLER_ROLE=human`. The agent can **yield** (“please log in”), the turn **parks** until Done, agent Input is dropped, and **model** screenshot/screencast parts are muted while `lock=human`. Sensitive verbs (password/OTP fill, cookie import, existing `needs_human` reasons) do not execute until the operator confirms or does the action themselves.

Success: live Axis demo (operator, not CI) **and** unit tests with no live Chrome.

## 1. Non-goals

- Iframe AX-tree merge, OOPIF `Target.setAutoAttach`, namespaced `ax:{frame}:{node}` refs.
- SPA `MutationObserver`, `networkidle`, or `wait_settled` (lifecycle + quiet + AX hash). Track as a later spec; see §10.
- NeedsYou rows, `pending_approvals`, or the 2026-06-19 soft-HITL inbox.
- Two daemons, durable lock across daemon restart, or lock state in Turso / Tier A.
- Watch-mode host allowlists, purchase/email classifiers, prompt-injection vision monitors.
- Playwright, Stagehand, Node, `page.pause()`, Chrome DevTools MCP, or a second Chromium.
- A new workspace crate or new crate-to-crate edges.
- A new `VOX_*` env var. `VOX_MCP_CALLER_ROLE` stays the **stdio** identity only.
- Changing `browser_act` off `llm_bridge::call_llm`.
- Inventing `vox_browser_download`. If a download verb ships later, it is sensitive under this policy; this spec does not add it.
- Muting snapshot / extract / text / html to the model during takeover (text observe stays).
- Locking `vox_browser_close` (stays unlocked so an agent can close a stuck tab).

## 2. Already shipped (do not rebuild)

| Surface | Reality |
| --- | --- |
| In-memory lock | `control_locks(): Mutex<HashMap<page_id, owner>>` in `browser_tools.rs`; owners `human` / `agent` |
| Process identity | `CallerRole` + `trusted_caller_role()` `OnceLock` from `VOX_MCP_CALLER_ROLE` (`caller_role.rs`). Literal `"human"` is Human; everything else is Agent |
| GUI daemon | Spawned with `VOX_MCP_CALLER_ROLE=human`; refuses to adopt a non-human daemon (`vox-gui` `daemon.rs`) |
| GUI claim | `browser_open_session` → `vox_browser_set_control_lock` `{ owner: "human" }`; toolbar You/Agent toggles the same tool |
| Axis | In-process `run_agent_turn` on that daemon → `trusted_caller_role()` is **Human** today (the inversion) |
| `needs_human` | Password/CAPTCHA JSON `{ ok: false, needs_human: true, reason }`. `ToolResult` success bit may be true. Axis **does not stop** |
| Observe vs mutate | Snapshot / screenshot / screencast / extract / wait_for / text / html / list / page_info / **close** are not lock-gated |
| Soft HITL | NeedsYou / `pending_approvals` are shell/task surfaces, not browser |

## 3. Locked decisions

| Topic | Choice |
| --- | --- |
| Architecture | **A** — one daemon, per-call `CallerContext`. No second process, no MCP `_meta` role (spoofable) |
| Identity | GUI IPC = Human. Axis / `run_agent_turn` = Agent. Stdio = `VOX_MCP_CALLER_ROLE` |
| User wins | Human surface may always call `vox_browser_set_control_lock`. Agent cannot steal a human lock |
| Yield | Existing You/Agent toggle **plus** `vox_browser_yield` `{ page_id, message }` |
| Park | Daemon-local waiter. Axis stops the tool loop. Resume only on Done / Agent toggle (`lock=agent`) |
| Mute | While `lock=human`: drop pending agent Input; Axis/stdio-Agent screenshot/screencast **omit image parts** and do not persist new model frames. GUI local view still paints. Snapshot/extract/text/html stay |
| Approvals | Sensitive verbs only (not every mutation). Confirm does **not** force You mode afterward |
| Confirm / Deny | Named APIs only (§4.1, §7.2). Confirm always replays as `CallerContext::gui()`. Deny clears pending + turn-scoped denial |
| `respect_sensitive` | `ctx.role != Human` — never `trusted_caller_role()` for GUI/Axis (§7.0) |
| Denial wire | Mutating denials are `ToolResult::ok` + `{ ok:false, needs_human:true, reason }` (§5.1) |
| `close` | Unlocked |
| NeedsYou | Do not wire |

## 4. Types (copy these names)

```rust
/// How this tool call entered the daemon. Not taken from the tool JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallerSurface {
    Gui,
    Axis,
    Stdio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallerContext {
    pub surface: CallerSurface,
    pub role: CallerRole, // Gui => Human, Axis => Agent, Stdio => CallerRole::from_env()
}

impl CallerContext {
    pub fn gui() -> Self {
        Self { surface: CallerSurface::Gui, role: CallerRole::Human }
    }
    pub fn axis() -> Self {
        Self { surface: CallerSurface::Axis, role: CallerRole::Agent }
    }
    pub fn stdio() -> Self {
        Self { surface: CallerSurface::Stdio, role: CallerRole::from_env() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YieldRequest {
    pub page_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitiveReason {
    PasswordField,
    Captcha,
    OtpField,
    CookieImport,
}

/// Holds password / OTP / cookie blobs. MUST NOT appear in `Debug`,
/// `action_log`, toast copy, telemetry, or disk. Prefer a redacted
/// wrapper type, or omit `args_json` from `Debug` entirely.
#[derive(Clone, PartialEq, Eq)]
pub struct PendingSensitive {
    pub page_id: String,
    pub tool: String,
    pub args_json: String, // secrets — not Debug
    pub reason: SensitiveReason,
}

impl std::fmt::Debug for PendingSensitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingSensitive")
            .field("page_id", &self.page_id)
            .field("tool", &self.tool)
            .field("args_json", &"<redacted>")
            .field("reason", &self.reason)
            .finish()
    }
}
```

### 4.1 Confirm / Deny API names (required — do not invent alternatives)

| Surface | Confirm | Deny | Args |
| --- | --- | --- | --- |
| Tauri / GUI IPC | `browser_hitl_confirm` | `browser_hitl_deny` | `{ page_id }` |
| MCP (stdio / GUI tool catalog if exposed) | `vox_browser_confirm_sensitive` | `vox_browser_deny_sensitive` | `{ page_id }` |

- **Confirm:** always dispatches the pending replay with `CallerContext::gui()` under the §5.2 Confirm exception (never Axis/stdio-Agent context).
- **Deny:** clears `PendingSensitive` for that `page_id` and records a turn-scoped denial so the same args do not re-queue for the rest of this Axis turn.

`trusted_caller_role()` remains for **stdio** and for tests that set the env. Dispatch must not use it for GUI or Axis.

## 5. Lock and dispatch rules

### 5.1 Who may mutate

`ensure_control_lock(page_id, ctx)`:

| Lock owner | `CallerContext::gui()` (Human) | `CallerContext::axis()` (Agent) | Stdio Human | Stdio Agent |
| --- | --- | --- | --- | --- |
| none | allow | allow | allow | allow |
| `human` | allow | **deny** | allow | **deny** |
| `agent` | **deny** (except §5.2) | allow | **deny** (except §5.2) | allow |

Deny payload: `{ ok: false, needs_human: true, reason: "human_locked" }` (or `"agent_locked"` when a Human surface hits `lock=agent` on a mutating tool other than §5.2). Axis **parks** on `human_locked`.

**Wire change (required):** Mutating denials MUST return `ToolResult::ok` whose JSON body is `{ ok: false, needs_human: true, reason }` — **not** `ToolResult::err(String)`. Axis park parses `/data/needs_human` on the tool envelope (same path as existing password/CAPTCHA tools). Plain `Err(String)` / `ToolResult::err` must not be the sole signal. Helper (recommended name in `browser_hitl.rs`): `needs_human_ok(reason) -> ToolResult` that builds this envelope.

### 5.2 Lock-admin exceptions

- `vox_browser_set_control_lock` — GUI / stdio-Human only. Agent callers get a hard error (not a park). User always wins: Human may set `owner` to `human` or `agent` regardless of the current owner.
- `vox_browser_yield` — **Agent** (Axis or stdio-Agent) only. Sets `owner=human`, stores `message` for the GUI, parks Axis. Human calling yield is a no-op success.
- **Confirm / Deny** (§4.1): `browser_hitl_confirm` / `vox_browser_confirm_sensitive` and `browser_hitl_deny` / `vox_browser_deny_sensitive` — `{ page_id }` only. Confirm replay of `PendingSensitive` always runs as `CallerContext::gui()` and is lock-admin for that single replay only (may execute even if the toolbar already flipped to `lock=agent`). Does not change the toolbar lock afterward. Deny clears pending and records a turn-scoped denial.

### 5.3 Unlocked verbs (unchanged except mute)

`close`, `snapshot`, `extract`, `wait_for`, `text`, `html`, `list_pages`, `page_info`.

Screenshot / screencast stay callable (so GUI can paint) but §6 mutes **model** pixels when `lock=human` and `ctx.role == Agent`.

### 5.4 Threading `CallerContext`

Set at the daemon edge, never from tool arguments:

| Entry | Context |
| --- | --- |
| Tauri / GUI `mcp_tool_call` / `TOOL_CALL` from `vox-gui` | `CallerContext::gui()` |
| `run_agent_turn` / Axis `agent_loop` dispatch | `CallerContext::axis()` |
| rmcp stdio `call_tool` | `CallerContext::stdio()` |

Do not read `actor` from the JSON body (agents could spoof “human”).

## 6. Yield, park, mute

### 6.1 Yield

`vox_browser_yield` arguments: `{ page_id: string, message: string }`.

Effects: `set_control_lock(page_id, "human")`; publish `message` on the existing Browser `action_log` / toast path; `park_axis(page_id)`.

The You/Agent toolbar remains the operator control. Switching to **You** is the same lock write. Switching to **Agent** is **Done**: `owner=agent` + `resume_axis(page_id)`.

### 6.2 Park

When Axis should stop:

1. `data.needs_human == true` (any reason, including `human_locked`, `password_field`, `captcha`, `cookie_import`, `otp_field`)
2. Explicit `vox_browser_yield`

**Any** Axis-originated `needs_human` (sensitive park, captcha, cookie import, lock deny, yield) **sets `owner=human`** for that `page_id` — same as password/cookie sensitive park. Captcha is not exempt.

`agent_loop` (`crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs`) after a tool result: if `needs_human`, **stop iterating** (do not call the model again with “try another click”). Emit a chat event that the turn is waiting on the operator. **Async wait only** — `Notify` / oneshot `.await` (or equivalent) on the daemon that still must serve GUI Confirm/Done IPC. A sync `recv` / `blocking_lock` on the same worker **deadlocks** the toolbar. Resume only when `resume_axis` fires.

Do **not** poll the DOM for “is the human done?”
Do **not** enqueue NeedsYou / `pending_approvals`.

Park state is in-memory. Daemon restart drops it (operator re-toggles Agent). Acceptable for v1.

### 6.3 Mute (Operator screenshot-off)

While `lock=human` **and** `ctx.role == Agent`:

- Drop any in-flight agent `Input.*` for that `page_id` (best-effort; next mutate is deny anyway).
- `vox_browser_screenshot_viewport` and `vox_browser_screencast_frame` return JSON with `page_id`, `width`, `height`, `mime` as needed **and must omit `/data/path` (null or absent)**. Do **not** attach an MCP image part.
- **Mute must skip `persist_browser_frame_png` entirely** — do not write a new file under `browser-frames/` and then omit the path. Skipping the path after write still leaves a secret-adjacent artifact on disk.
- Rationale: shipped chat-harness `attach_image_from_cached_path` reloads any present `path` into `content_parts`, so returning a disk path would defeat mute even with no MCP image part.
- GUI Human-surface frame capture for the local `vox://browser-frame` view is **not** muted.

`lock=agent` or Human-surface calls: unchanged (chat-harness image-part rules still apply).

## 7. Sensitive-verb approvals

### 7.0 `respect_sensitive` and `CallerContext` (normative)

```text
respect_sensitive = (ctx.role != CallerRole::Human)
```

- Agent surfaces (Axis, stdio-Agent) park / require Confirm on sensitive verbs.
- Human surfaces (GUI, stdio-Human) execute without Confirm.
- Human-only cookie **export** (and any other human-gated export) MUST gate on `ctx.role`, not `trusted_caller_role()`.
- Call sites that today use `trusted_caller_role()` for sensitive detection or export gating MUST switch to `ctx` — listed in §8.

### 7.1 Reasons that require confirm (Agent surface)

| Reason | When |
| --- | --- |
| `password_field` | Existing `AxRef.sensitive` / password fill (already shipped) |
| `otp_field` | Fill of a ref whose name matches OTP / 2fa / one-time / verification patterns. **This contract expands `is_sensitive_name`** so those names set `AxRef.sensitive` — do **not** defer OTP marking to a later plan; do not add an NLP classifier |
| `captcha` | Existing snapshot-tree substring check |
| `cookie_import` | `vox_browser_cookies_import` from Agent |

**CSS / act fill bypass (same park/Confirm):** Agent-surface `browser_fill`, `browser_type`, and CSS-selector fill / type arms inside `browser_act` that target password-ish inputs (password type, or name/id matching the expanded `is_sensitive_name` set) are sensitive verbs under this section — same `PendingSensitive` park + Confirm/Deny as AX-ref fill. Do not allow CSS fill to skip §7.

No watch-mode hosts. No “goto new registrable domain” confirm in v1.

### 7.2 Flow

1. Axis calls the verb with `CallerContext::axis()`.
2. The verb **does not execute**. Record `PendingSensitive` in `browser_hitl.rs` **and set `owner=human`** (same as yield — so You-mode self-serve and Confirm-as-Gui both pass §5.1 without a separate lock flip). Captcha and any other Axis `needs_human` reason follow the same `owner=human` rule (§6.2).
3. Return `ToolResult::ok` with `{ ok: false, needs_human: true, reason }` via `needs_human_ok` (§5.1). Axis parks (§6.2).
4. Operator either:
   - Does the action in **You** mode (human tools execute; pending is cleared without replay), or
   - Hits **Confirm** once via `browser_hitl_confirm` / `vox_browser_confirm_sensitive` `{ page_id }`: replay that tool with `CallerContext::gui()` under the §5.2 Confirm exception. Toolbar lock is unchanged afterward (Confirm does not force You mode as a lasting toggle). Confirm may run even when the toolbar already shows `lock=agent`.
5. **Deny** via `browser_hitl_deny` / `vox_browser_deny_sensitive` `{ page_id }`: drop `PendingSensitive`; fail closed; remember the denial for the rest of this Axis turn so the same args do not re-queue.

**Secrets:** `PendingSensitive.args_json` may hold passwords / OTP / cookie blobs. It MUST NOT appear in `Debug` (§4). Toast / `action_log` show **reason only** — never `args_json` or secret substrings. Never write args to telemetry or disk. Clear the pending record (and args) on Confirm success, Deny, Done/`resume_axis`, or daemon drop.

GUI: reuse the existing `needs_human` toast + `action_log`. Wire Confirm / Deny to the named Tauri commands above. No dedicated HITL panel.

## 8. File map

| File | Role |
| --- | --- |
| Create: `crates/vox-orchestrator-mcp/src/browser_lock.rs` | `CallerContext`, lock map moved off `browser_tools.rs`, `ensure_control_lock(page_id, ctx)` |
| Create: `crates/vox-orchestrator-mcp/src/browser_hitl.rs` | Park/resume, `PendingSensitive`, mute predicate, `vox_browser_yield`, `needs_human_ok`, Confirm/Deny handlers (`vox_browser_confirm_sensitive` / `vox_browser_deny_sensitive`) |
| Modify: `caller_role.rs` | Keep `CallerRole` / `from_env`. Do not make Axis use `trusted_caller_role()` |
| Modify: daemon / `handle_tool_call` | Attach `CallerContext` from entry surface |
| Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs` | After tool dispatch only: if `/data/needs_human`, park (helper in `browser_hitl.rs`). Do not grow the loop with lock tables |
| Modify: `browser_tools.rs` | Call the new modules; **no** new lock/HITL bodies here (already ~1498 lines) |
| Modify: `is_sensitive_name` / AX sensitive marking (same crate as today) | Expand name patterns for OTP / 2fa / one-time / verification → `AxRef.sensitive` (§7.1) |
| Modify: fill / type / `browser_act` CSS arms | Agent password-ish CSS fill/type → §7 park (same as AX fill) |
| Switch to `ctx` (not `trusted_caller_role()`) for sensitive / export | Every call site that today gates `respect_sensitive`, password/OTP fill park, cookie import park, or human-only cookie **export** on `trusted_caller_role()` — pass `CallerContext` and use `ctx.role` (§7.0) |
| Modify: screenshot / screencast frame path | Mute branch **skips** `persist_browser_frame_png` (no new `browser-frames/` file) |
| Modify: `vox-gui` `commands/browser.rs` + toolbar | Done = Agent toggle → `resume_axis`; Tauri `browser_hitl_confirm` / `browser_hitl_deny` on the existing toast |
| Modify: `contracts/operations/catalog.v1.yaml` | Describe `vox_browser_yield`, `vox_browser_confirm_sensitive`, `vox_browser_deny_sensitive`; then `vox ci operations-sync --target all --write` |
| Modify: `docs/src/architecture/where-things-live.md` | One row for HITL lock + park |
| Modify: parent + chat-harness specs | Follow-on pointer only — do not absorb this work into those plans |

Do **not** grow `engine.rs`. Do **not** add Playwright. No new crate.

## 9. Tests (no live Chrome)

| Test | Asserts |
| --- | --- |
| Identity matrix | `Gui` × `{none,human,agent}` and `Axis` × `{none,human,agent}` match §5.1 |
| Stdio lock matrix | Stdio Human / Stdio Agent × `{none,human,agent}` match §5.1 columns |
| Inversion gone | Axis + `lock=human` cannot `goto` / `click` / `fill` |
| Lock admin | Axis cannot `set_control_lock`; GUI can flip either way |
| Yield | `vox_browser_yield` sets human + parks; Agent toggle resumes |
| `needs_human` parks | Password/CAPTCHA JSON stops the loop fixture (no second `llm_chat`); any Axis `needs_human` sets `owner=human` |
| Mute | Agent + `lock=human` → tool JSON has no `/data/path`; `mcp_contents_for_tool_json` **and** `llm_tool_message` have no image part |
| Mute skips persist | Mute branch does not call `persist_browser_frame_png` / creates no new file under `browser-frames/` |
| Sensitive | Cookie import / password fill from Axis does not call the plugin; Confirm replays as GUI |
| Named Confirm/Deny API | `browser_hitl_confirm` / `vox_browser_confirm_sensitive` and deny twins exist and take `{ page_id }`; Confirm dispatches with `CallerContext::gui()` |
| Confirm while `lock=agent` | Confirm still replays under §5.2 exception without changing toolbar lock |
| Deny | Same args in the same turn do not execute |
| `respect_sensitive` under Axis | Axis (`ctx.role == Agent`) parks on sensitive fill; GUI (`ctx.role == Human`) does not — gated on `ctx`, not `trusted_caller_role()` |
| CSS fill sensitive | Agent `browser_fill` / `browser_type` / `browser_act` CSS fill on password-ish input parks like AX fill |
| Secret non-logging | Toast / `action_log` / `Debug` of `PendingSensitive` may show `reason`; secret substrings from `args_json` must not appear |
| `close` | Axis may close a human-locked tab |
| Observe text | Axis `snapshot` on a human-locked tab still returns a tree |

## 10. Deferred (not this spec)

Recorded so they are not “forgotten” into this file as tasks:

1. **SPA settled** — `Page.setLifecycleEventsEnabled` + `navigatedWithinDocument` + 80–150ms quiet + AX-tree hash. Not Playwright `networkidle`. Clear refs on same-document nav. MutationObserver only later, isolated world, per frame.
2. **iframe merge** — same-origin `GetFullAXTree(frame_id)` first; OOPIF via `Target.setAutoAttach({ flatten: true })` + per-session trees + `ax:{frame}:{node}`. Never `getFullAXTree({ frameId })` on the parent for cross-origin (CDP “frame not found”).
3. Durable lock, two-daemon split, watch-mode hosts, vision prompt-injection pause.

## 11. Critique ledger (decisions already taken)

| Challenge | Resolution |
| --- | --- |
| Process-wide Human role makes Axis a lock peer of the operator | `CallerContext` at the dispatch edge; Axis is always Agent |
| MCP `_meta` role is spoofable | Rejected (architecture C) |
| Two daemons need a shared lock | Rejected (architecture B) |
| Reuse NeedsYou for park | Rejected — daemon-local waiter |
| Mute snapshot text during takeover | Rejected — Operator mutes **screenshots**; text observe stays |
| Lock `close` | Rejected — agent may close a stuck tab |
| Confirm every mutation | Rejected — sensitive verbs only |
| Confirm forces You mode as lasting toggle | Rejected — toolbar stays as left; sensitive park still sets `human` lock; Confirm is a one-shot Gui replay exception |
| Persist lock across restart | Rejected for v1 |

## 12. Implementation order (when planned)

Do **not** start this while the 2026-09-08 chat-harness plan still has open tasks. Suggested slices after that plan merges:

0. Move lock map + `CallerContext` + identity matrix tests.
1. Wire dispatch surfaces; Axis deny on `human_locked`.
2. Park/resume + `vox_browser_yield` + toolbar Done.
3. Mute image parts when Agent + `lock=human`.
4. `PendingSensitive` + Confirm/Deny on the existing toast.
5. SSOT / operations-sync / where-things-live.
