---
title: "Computer Use and Browser Control: Audit and Roadmap (2026)"
description: "Audit of Vox's browser automation and OS-level computer control against Claude Code, Codex, Antigravity and Cursor as of October 2026, with a recommended architecture for background desktop control and the CDP browser driver."
category: "Architecture SSOTs"
status: "research"
topic: "mcp-tools"
domain: "orchestration"
authorship: "llm-assisted"
authorship_model: "claude-opus-5.5"
training_eligible: false
---

# Computer Use and Browser Control: Audit and Roadmap (2026)

<!-- kb:summary -->
Audits what Vox can do today to control a browser and a desktop, compares it with the
harnesses that set the bar in October 2026 (Claude Code, Codex, Antigravity, Cursor),
and recommends an architecture. Vox already has a solid Chrome DevTools Protocol
browser plugin, but no OS-level computer use at all. The recommendation is to keep
CDP and harden it, which is where the industry converged, and to build a new
accessibility-first, background-input desktop driver behind a plugin extension point,
with one action schema adapted to each provider's computer-use tool.
<!-- /kb:summary -->

## 1. Verdict

- **Browser: keep the architecture and close the gaps.** Vox already drives real Chrome
  over the Chrome DevTools Protocol (CDP) through `chromiumoxide`, using compact
  accessibility-tree snapshots with element refs. That is the same design Antigravity,
  Codex (developer mode) and Chrome's own `chrome-devtools-mcp` use. The industry did not
  move to *headless* Chrome so much as to *real Chrome driven over CDP*, usually visible
  and headed, with headless kept for CI and unattended runs. An in-app webview survives in
  Codex and Cursor only as an isolated preview for local dev servers, which matches Vox's
  existing Preview tab. What Vox lacks is robustness: auto-waiting, iframes, dialogs,
  downloads, uploads, console and network capture, streaming live view, and consented
  attach to the user's own Chrome.
- **Computer use: build it from scratch; nothing exists.** There is no mouse, keyboard,
  screen-capture or desktop-accessibility code anywhere in `crates/`. The quality bar in
  2026 is *background* control: read the target app's accessibility tree, act on named
  elements, deliver input to a specific process without moving the user's cursor or
  stealing focus, and capture a specific window even when it is covered. Fall back to
  pixel coordinates only for canvas and custom-drawn UI.

## 2. What Vox has today

### 2.1 Browser (`crates/vox-plugin-browser`, plugin layer L4)

| Area | State | Where |
| --- | --- | --- |
| Engine | `chromiumoxide` 0.9.1 over CDP, own two-thread Tokio runtime behind the `abi_stable` plugin ABI | `src/browser.rs`, `Cargo.toml` |
| Launch modes | `ephemeral` (default), `named` persistent profile (consent-gated), `attach` to a loopback `cdp_url` | `src/policy.rs`, `src/hosts.rs` |
| Perception | `Accessibility.getFullAXTree` compacted into a text tree with `e1`, `e2`… refs; interactive-only, depth 12, 80 nodes by default; wrapped in nonce markers against prompt injection | `src/snapshot.rs`, `src/resolve.rs:163` |
| Actions | `click_ref`/`fill_ref` resolve `backendDOMNodeId` to a box-model center; CDP `Input.dispatchMouseEvent`, `Input.insertText`, `Input.dispatchKeyEvent` with key-chord parsing; CSS and `xpath:` selectors | `src/ref_actions.rs`, `src/input.rs` |
| Media | Full-page and viewport PNG, a single screencast frame as base64 JPEG | `src/input.rs` |
| Safety | Host allowlist (`VOX_BROWSER_ALLOWED_HOSTS`), sensitive-field heuristic returning `needs_human`, per-page human/agent control lock, human-only cookie export | `src/policy.rs`, `crates/vox-orchestrator-mcp/src/browser_tools.rs` |
| Surfaces | Trait revision 5 (32 methods); about 30 `vox_browser_*` MCP tools including the LLM-driven `vox_browser_act`; GUI Browser surface; 9 `Browser.*` builtins in the Vox language | `crates/vox-plugin-api/src/extensions/browser_automation.rs:10`, `contracts/mcp/tool-registry.canonical.yaml`, `crates/vox-compiler/src/builtin_registry.rs` |
| GUI | **Preview** tab is a loopback `<iframe>`; **Agent** tab polls a screenshot every 3 s and shows it as an image | `crates/vox-gui/src/commands/browser.rs:28` |

Gaps found in the audit:

1. **No auto-wait.** `wait_for` polls a selector every 200 ms (`src/resolve.rs:35`).
   Actions do not check that the target is visible, stable, enabled and not covered
   before clicking, which is the single largest source of flaky agent clicks.
2. **Main frame only.** No out-of-process iframe (OOPIF) or same-origin iframe targeting,
   so embedded payment forms, OAuth popups and many enterprise apps are unreachable.
3. **No JavaScript dialogs, downloads, file upload, console log or network capture.**
   These are the features coding harnesses use most for "debug my web app".
4. **Live view is a 3-second screenshot poll**, not a CDP screencast stream, and there is
   no session recording.
5. **Attach requires a manual debug port.** Since Chrome 136, `--remote-debugging-port`
   is ignored on the default profile, so "use my logged-in Chrome" needs Chrome 144's
   consented flow (`chrome://inspect/#remote-debugging` plus `DevToolsActivePort`), which
   Vox does not implement.
6. **Refs die on every navigation and every snapshot.** There is no stable identity for
   an element across snapshots and no snapshot diff, so the model re-reads the whole page.
7. **Control lock is in memory** and lost when the MCP server restarts.
8. **Real-Chrome tests are `#[ignore]`d**, so hosted CI does not exercise a browser.
9. **The Vox language surface is thin**: no snapshot, refs or `open_ex` in `Browser.*`.

### 2.2 Desktop computer use

Status as of 2026-10-26: phase C1 has started on macOS. `crates/vox-plugin-computer`
implements the `ComputerControl` extension point (plugin ABI 14): accessibility snapshots
with refs, per-window capture, and `ax` / `event` / `activate` input tiers. The MCP tools
are `vox_computer_permissions`, `vox_computer_list_apps`, `vox_computer_list_windows`,
`vox_computer_snapshot`, `vox_computer_screenshot` and `vox_computer_act`. They sit behind
the opt-in `computer` capability profile and the lazy `computer` daemon subsystem.
`vox_computer_act` is a destructive risk class, so it parks for approval in every
permission mode except accept-all. Windows and Linux refuse with `unsupported_platform`
until their backends land.

Before this work there was nothing: no OS accessibility or input dependency and no
`vox_computer_*` tools. The browser design spec
(`docs/superpowers/specs/2026-09-07-agent-browser-driver-design.md`) scoped OS-level
control out. The desktop tools reuse vision input in the LLM boundary, image promotion
for tool results (`crates/vox-orchestrator-mcp/src/tool_images.rs`), and the browser
plugin's control-lock and sensitive-field patterns.

## 3. How the reference harnesses do it (October 2026)

### 3.1 Browser

| Harness | Mechanism | Notes |
| --- | --- | --- |
| Antigravity | A `browser` subagent drives Google Chrome through the Chrome DevTools MCP server (Puppeteer over CDP), started automatically | Separate context window; every session recorded as WebM; `evaluate_script` needs approval. The 2025 build used a Chrome extension calling `chrome.debugger` |
| Claude Code | Claude in Chrome extension (`claude --chrome`) | Uses the user's real profile and logins; tabs grouped per session; pauses at logins and CAPTCHAs; needs a Claude plan, falls back to MCP servers otherwise |
| Codex | Three surfaces: an isolated in-app browser for dev servers, a Chrome extension for the user's logged-in browser, and full CDP "developer mode" | CDP is off by default, approved per site, and can be disabled per organization |
| Cursor | Built-in browser tab with its own per-workspace cookies, plus a raw CDP tool | Users wire Playwright MCP or Chrome DevTools MCP for their own Chrome |
| Anthropic API | `browser_toolset_20260801`, a client toolset that reads and acts on the page | Vox would execute it with its own driver |

The pattern: **CDP against real Chrome is the engine everywhere**. The two ways in are
(a) a dedicated automation profile launched by the harness, and (b) the user's own Chrome
reached by an extension or by Chrome 144's consented auto-connect. Perception is the
accessibility tree first, screenshots second. An in-app webview cannot be the agent
engine for a cross-platform Tauri app in any case: only WebView2 on Windows exposes CDP;
WKWebView (macOS) and WebKitGTK (Linux) do not.

### 3.2 Desktop

| Implementation | Perception | Input | Capture |
| --- | --- | --- | --- |
| Cursor computer use | macOS accessibility tree with element ids, snapshot ids reused across screenshots | Background, per app; does not move the real cursor; explicit "take over the display" escalation | Per-app screenshot on a fixed canvas |
| cua-driver (Hermes, others) | AX on macOS, UI Automation on Windows, AT-SPI on Linux | macOS: pid-targeted events (including private SkyLight calls so Chrome accepts them); Windows: `SendInput` plus `PostMessage` without focus steal; Linux: XTest or the Wayland virtual keyboard | ScreenCaptureKit, per window |
| `computer-use` crate (Rust, 0.3.x) | AX / UI Automation / AT-SPI | Tiers: `ax` (semantic action), `event` (`CGEventPostToPid`, XTest, RemoteDesktop portal), `activate` (raise, act, restore under a lock), optional `synthetic` | ScreenCaptureKit, `Windows.Graphics.Capture`, PipeWire portal; OCR; permission doctor |
| Warp `computer_use` crate | Screenshots | macOS `CGEventPostToPid`; X11 via a second XInput2 seat so the user's cursor is untouched | Whole screen on Wayland and Windows |
| Anthropic `computer_toolset_20260801` | Screenshots plus `zoom` on a region | 17 members: clicks, drag, `left_mouse_down/up`, `scroll`, `type`, `key` with `repeat`, `hold_key`, `wait` | Client supplies; coordinates are in screenshot pixels and must be rescaled |
| OpenAI Responses `{"type": "computer"}` | Screenshots | Client executes the returned actions | Client supplies |

Hard platform facts behind those designs:

- **macOS:** `CGEventPost` moves the shared cursor; `CGEventPostToPid` does not, but
  Chromium's renderer drops pid-posted mouse events that lack real-gesture metadata, so
  Chrome-based apps need either private SkyLight calls or a brief activate. Requires the
  Accessibility and Screen Recording permissions.
- **Windows:** UI Automation control patterns (`Invoke`, `Value`, `Text`, `ScrollItem`,
  `ExpandCollapse`) act without focus. `SendInput` is foreground-only and blocked by
  integrity levels (UIPI) against elevated windows. The process must run in an
  interactive session, not Session 0. Per-window capture is `Windows.Graphics.Capture`.
- **Linux X11:** XTest works but is indistinguishable from the user's input unless a
  second XInput2 master seat is used. **Wayland:** no global injection by design; use the
  xdg-desktop-portal RemoteDesktop and ScreenCast interfaces (libei and PipeWire), with
  per-compositor limits. A driver must refuse loudly rather than silently drop events.

## 4. The quality bar

"As good as the best harness" means all of the following, and each is testable:

1. **Semantic first.** Act on accessibility elements by stable handle; coordinates are a
   fallback that is reported as such in the tool result.
2. **Background by default.** No cursor warp, no focus change, no Space or virtual-desktop
   switch. Foreground control is a named, locked, user-visible escalation.
3. **Verify every step.** Each action returns the post-action state (or a diff) so the
   model never assumes success. Settling waits are event-driven, not fixed sleeps.
4. **Stale-proof handles.** A handle is bound to the snapshot that produced it and is
   rejected, not misapplied, once the UI changes.
5. **Exact typing.** Unicode, IME and non-US layouts via text insertion rather than
   keycode synthesis, with key chords only for shortcuts.
6. **Fine perception.** Per-window capture of occluded windows, a `zoom` for dense UI,
   coordinate scaling handled in one place, and DPI awareness on mixed-scale monitors.
7. **Human in charge.** Visible agent cursor, interrupt detection (the user touches the
   app and the agent yields), a global stop hotkey, per-app allowlist, password-field
   refusal, confirmation before irreversible actions, and an audit trail.
8. **Model-agnostic.** One Vox action schema, adapted to Anthropic and OpenAI native tools
   and exposed as plain MCP tools for every other model.

## 5. Recommended architecture

### 5.1 Desktop: a new `ComputerControl` extension point

Mirror the browser plugin, which already proved the plugin ABI, policy and MCP pattern:

- **`vox-plugin-api`**: add a `ComputerControl` extension trait (revisioned like
  `BrowserAutomation`). Core methods: `list_apps`, `list_windows`, `snapshot(target,
  options)` returning a compact AX text tree with snapshot-scoped refs, `screenshot(target)`,
  `zoom(target, region)`, `act(target, action, tier)`, `set_value`, `type_text`, `key`,
  `scroll`, `drag`, `wait_for_settle`, `permissions_doctor`.
- **`vox-plugin-computer`** (new, layer L4 in `layers.toml`): the native implementation,
  one module per platform, each with three input tiers:
  `ax` (semantic action, no focus) → `event` (process-targeted events, no cursor) →
  `activate` (raise, act, restore, under a machine-wide foreground lock). The result
  always reports which tier ran. `require_background` refuses `activate`.
- **Platform crates** (all public APIs, no private frameworks by default):
  macOS `objc2-app-kit`, `objc2-application-services` (AX), `objc2-core-graphics`
  (`CGEventPostToPid`), `objc2-screen-capture-kit`; Windows `windows` plus `uiautomation`,
  `Windows.Graphics.Capture`; Linux `atspi`, `x11rb` (XTest, XInput2 second seat),
  `ashpd` (RemoteDesktop and ScreenCast portals) with PipeWire. Private SkyLight calls stay
  behind an off-by-default feature because they can break on any macOS update.
- **MCP**: `vox_computer_*` tools registered in `contracts/mcp/tool-registry.canonical.yaml`
  and the capability registry, mirroring the browser tool names (`snapshot`, `act`,
  `screenshot`, `zoom`, `set_control_lock`).
- **Provider adapters** in the orchestrator, not at call sites: map
  `computer_toolset_20260801` members and OpenAI's `computer` actions onto the Vox action
  schema, including screenshot-to-display coordinate rescaling. This requires the
  `vox_actor_runtime::llm` facade to pass provider-defined tool types through, which it
  does not today.
- **Policy and consent**: a new capability (for example `computer.control`) that is off by
  default and granted per session; per-app allowlist; `IsPassword`/`AXSecureTextField`
  refusal reusing the browser's `needs_human` shape; the browser's control lock, persisted
  instead of in-memory; every action logged as a Tier B event through `vox-spool`;
  screenshots treated as untrusted model input, like page snapshots.
- **Unattended tier (later)**: run the same driver inside a disposable desktop (Xvfb plus a
  window manager in the container runtime, macOS VMs via Virtualization.framework, Windows
  Sandbox) so long jobs never touch the user's session.

**Build versus adopt.** Three routes were considered:

| Route | For | Against |
| --- | --- | --- |
| Native Rust plugin (recommended) | Fits the plugin ABI, policy, telemetry and crate-edge rules; no subprocess; we own the quality bar | Most work, especially Wayland and Chromium-in-background on macOS |
| Depend on the `computer-use` crate | Already implements tiers, capture, OCR and a doctor in Rust | Young (0.3.x), single upstream, wide native build dependencies (`libclang`, PipeWire) that conflict with the AGENTS.md build-toolchain invariant |
| Spawn `cua-driver` over MCP | Fastest to try; strongest macOS background story today | External binary and runtime, private Apple APIs, policy enforced outside Vox |

Use `cua-driver` and the `computer-use` crate as **reference implementations and a
benchmark** during a short spike, then build natively. Verify licenses before copying any
code.

### 5.2 Browser: harden the CDP driver

Keep CDP. Do not replace the agent engine with the Tauri webview. In priority order:

1. **Actionability and auto-wait** inside every ref and selector action: scroll into view,
   wait until visible, stable (box unchanged across two animation frames), enabled and
   hit-testable at the click point (`DOM.getNodeForLocation`), then act. Return
   `not_actionable` with the reason instead of clicking blind.
2. **Frames**: `Target.setAutoAttach` with `flatten: true`; include iframe subtrees in the
   snapshot with frame-qualified refs.
3. **Dialogs, downloads, uploads**: `Page.javascriptDialogOpening` surfaced as a tool
   state; `Browser.setDownloadBehavior` into a jailed directory; `DOM.setFileInputFiles`
   behind a path jail that refuses credential and config locations.
4. **Console and network**: ring buffers fed by `Runtime.consoleAPICalled`,
   `Runtime.exceptionThrown` and `Network.*`, bounded by bytes, exposed as
   `vox_browser_console` and `vox_browser_network`.
5. **Streaming live view and recording**: replace the 3 s screenshot poll with
   `Page.startScreencast` into the GUI, and optionally record sessions to WebM as
   Antigravity does.
6. **Consented attach to the user's Chrome**: read `DevToolsActivePort` from the
   profile and connect after the user enables `chrome://inspect/#remote-debugging`; keep
   launched automation profiles on a separate `--user-data-dir`; pin a Chrome for Testing
   build for CI and unattended runs.
7. **Stable refs and diffs**: key refs on `backendDOMNodeId` so they survive re-snapshots,
   and return a diff after each action.
8. **Provider adapter** for Anthropic `browser_toolset_20260801` mapped onto the existing
   tools.
9. **Library health**: `chromiumoxide` last shipped 0.9.1 in February 2026 (CDP
   r1566079). `chromey`, a maintained fork, ships monthly. Spike a swap behind the plugin
   boundary; note that either pulls `reqwest` 0.13 and therefore the `aws-lc-rs` transport
   provider tracked in `contracts/crypto/transport-providers.v1.json`, whose ledger entries
   are user-authorized only.
10. **CI**: run the ignored Chrome smoke tests in nightly against a pinned headless build,
    and extend the Vox `Browser.*` builtins with `snapshot`, `click_ref` and `fill_ref`.

## 6. Roadmap

| Phase | Scope | Exit criterion |
| --- | --- | --- |
| B1 | Browser actionability, dialogs, console/network capture, nightly real-Chrome tests | A fixed set of flaky-page fixtures passes 20 runs in a row |
| B2 | Frames, uploads/downloads, consented attach, streaming live view | Agent completes an OAuth-in-iframe and a file-upload flow on fixtures |
| C0 | Two-week spike: `ComputerControl` trait draft; benchmark `cua-driver` and the `computer-use` crate on a fixed task list | Written comparison; layer and capability names approved |
| C1 | macOS and Windows native backends, `ax` and `event` tiers, per-window capture, permissions doctor, MCP tools, consent capability, audit log | `focus_invariance` tests prove no cursor or focus change; password fields refused |
| C2 | Provider adapters for Anthropic and OpenAI computer tools; zoom; interrupt detection; agent cursor overlay; global stop hotkey | Same task suite passes through Claude, OpenAI and a local MCP-only model |
| C3 | Linux X11 (second seat) and Wayland (portals); `activate` tier with foreground lock | Refuses loudly on compositors that drop input |
| C4 | Unattended disposable desktops; OSWorld-Verified and ScreenSpot-Pro style evaluation in nightly | Scores tracked per model in the registry |

### 6.1 Harness integration and evaluation

A capability that is registered but never offered, or offered but never chosen, is
invisible to the user. Following 2026 harness practice, each of four layers has its own
test.

| Layer | Question | Where it is checked |
| --- | --- | --- |
| Exposure | Is the tool registered, in a profile, and dispatchable? | `contracts/profiles/computer.v1.yaml` (validated by `vox-mcp-registry/build.rs`), `computer_profile_is_exactly_the_desktop_tools` |
| Selection | Is it offered on a relevant turn and withheld on an unrelated one? | `TRIGGER_CASES` in `llm_bridge/tool_selection.rs`; unit tests plus the hermetic `tool-families-trigger-on-relevant-turns` golden task |
| Activation | Does a live model actually pick it first? | `chat_tools/chat/tool_choice_probe.rs`; nightly `tool_trigger` tasks in `vox harness eval --live` |
| Safety | Does a desktop action wait for approval? | `computer-act-requires-approval` golden task over `permission_modes::RISK_CLASSES` |

Selection works through intent families. Chat turns offer only the `ai` and `app` lanes, so
browser and desktop tools (lane `interop`) used to be unreachable from chat. A request that
names a web page, a URL or a desktop app now triggers that family's core tools, which are
offered after the pinned delegation tools and inside the tool cap. When a turn does not
trigger a family, that family's tools are withheld, including the three browser tools that
sit in the `ai` lane. Every turn pins `vox_tool_search`, and tools it returns are added to
the turn's offered set. So the trigger word lists only decide what is offered up front; any
tool can still be reached by search.

The activation probe builds the turn exactly like `chat_message` (routed model, system
prompt, `chat_turn_context`) and makes one model call. It records the first tool the model
chose and never dispatches it, so a nightly run cannot drive a real browser or desktop.
Positive cases pass when the first tool belongs to the expected family. Negative and
confusable cases ("Refactor the window function…", "the app's state machine") pass when
it belongs to neither family.

Measured 2026-10-26 with `VOX_MODEL_FORCE_CHAT=anthropic/claude-haiku-4.5`: 14 of 15 cases
passed. The one failure (`browser-bare-host`) was the router refusing the pinned model
for a request it classified as needing web capability, not a wrong tool choice. Each probe
cost about $0.03, mostly system prompt and tool definitions.

## 7. Decisions needed

1. Approve a new L4 crate `vox-plugin-computer` and the `ComputerControl` extension point.
2. Approve the `computer.control` capability and its default-off consent model.
3. Decide whether private macOS SkyLight calls are ever allowed (recommended: feature flag,
   off by default, never in signed release builds until proven stable).
4. Approve the `chromiumoxide` to `chromey` spike, including any transport-provider
   ledger change it implies.

## Sources

- Claude Code, [Use Claude Code with Chrome](https://code.claude.com/docs/en/chrome)
- Anthropic, [Computer use tool](https://platform.claude.com/docs/en/agents-and-tools/tool-use/computer-use-tool) and [Tool reference](https://platform.claude.com/docs/en/agents-and-tools/tool-use/tool-reference)
- OpenAI, [Computer use](https://developers.openai.com/api/docs/guides/tools-computer-use)
- Google Antigravity, [Browser command](https://antigravity.google/docs/commands/browser/)
- Chrome for Developers, [Changes to remote debugging switches](https://developer.chrome.com/blog/remote-debugging-port), [Auto-connect](https://developer.chrome.com/docs/devtools/agents/use-cases/auto-connect), [Removing `--headless=old`](https://developer.chrome.com/blog/removing-headless-old-from-chrome)
- [chrome-devtools-mcp](https://github.com/ChromeDevTools/chrome-devtools-mcp)
- Codex developer mode, [Codex drives the browser with CDP](https://pasqualepillitteri.it/en/news/5088/codex-browser-chrome-devtools-protocol)
- [How to let your coding agent use your logged-in Chrome](https://browser-use.com/posts/coding-agent-logged-in-chrome) (October 2026 survey)
- Cua, [Platform support](https://cua.ai/docs/cua-driver/concepts/platform-support) and [Inside macOS window internals](https://github.com/trycua/cua/blob/main/blog/inside-macos-window-internals.md)
- [`computer-use` crate](https://docs.rs/crate/computer-use/latest), [Warp `computer_use` crate](https://github.com/warpdotdev/warp/blob/2b4a66f8/crates/computer_use/src/lib.rs), [mac-cua](https://github.com/hyprcat/mac-cua)
- [chromiumoxide changelog](https://github.com/mattsse/chromiumoxide/blob/main/CHANGELOG.md), [chromey](https://crates.io/crates/chromey)
- Related Vox docs: [Agent browser driver SSOT](agent-browser-driver-ssot-2026.md), [Vox GUI browser support](vox-gui-browser-support-2026.md)
