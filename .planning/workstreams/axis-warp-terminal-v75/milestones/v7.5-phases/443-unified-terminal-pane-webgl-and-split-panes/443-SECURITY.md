---
phase: "443"
slug: "unified-terminal-pane-webgl-and-split-panes"
status: verified
threats_open: 0
asvs_level: 1
block_on: high
created: "2026-11-15"
---

# Phase 443 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail for Phase 443 (`443-unified-terminal-pane-webgl-and-split-panes`).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Rust Tauri IPC (`vox://terminal/event`, `term_session_attach`) → Frontend `TerminalSessionPane` / `sessionReducer.ts` / `TerminalTab` | Live PTY byte stream, sequence counters, alt-screen transitions, and structured block metadata crossing from the local host/daemon PTY into the webview | `TermSessionSnapshotWire` (`replay_utf8`, `blocks`, `seq`, `resync_required`, `alt_screen`, `security_level`, `control_mode`), externally-tagged `SessionEvent` frames |
| Untrusted PTY stdout/stderr → `sgrToSpans.ts`, `BlockCard.tsx`, and `ChatTerminalBlockCard.tsx` | Terminal output produced by local processes, git logs, build tools, or agent-run commands rendered as rich SGR cards in both Console and Chat | Raw UTF-8 + ECMA-48 CSI/OSC/SGR byte sequences, `\r` / `CSI K` overwrites, clickable URLs, `file:line(:col)` stack-trace locators, HTML/script metacharacters |
| Interactive prompt stdin (`BlockCard.tsx` `AwaitingInput`) → `voxTransport.termSessionSendInput` | Operator responses to stalled interactive prompts (`[y/N]`, REPL, `sudo`/SSH/OTP credential prompts) crossing back to the PTY | Interactive input text, `credential_prompt: boolean` classification, `block_id` |
| Console toolbar / keyboard shortcuts / PTY inputs → `termSessionSetSecurity` & `termSessionSetControl` (`Console.tsx`, `TerminalSessionPane.tsx`) | Operator session governance controls (`L1..L5` security tier and `manual` / `co_pilot` / `autopilot` control mode) and human-preemption signals | `TermSecurityLevelWire`, `TermControlModeWire`, human input preemption across `InputEditor`, `TerminalTab` raw VT keys, and `BlockCard` stdin |
| Cross-surface focus & Dockview panel updates (`ChatTerminalBlockCard` → `terminalBridge.ts` → `Console.tsx`; `ChatSurface.tsx` `closedPanelIds`) | Cross-surface navigation events and Dockview parameter updates between Chat turn traces/transcripts and the docked/standalone Console | `{ sessionId, blockId }` focus intents, `vox_terminal_*` tool event args/previews, Dockview `terminal` panel lifecycle |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-443-01 | Information Disclosure / Denial of Service | `crates/vox-gui/ui/src/transport.ts`, `TerminalTab.tsx`, `TerminalSessionPane.tsx` | medium | mitigate | `voxTransport.termSessionAttach(sessionId, afterSeq?)` routes strictly through `safeInvoke` (`transport.ts:942-947`, preserving `transportIpcGuard.test.ts`); `TerminalSessionPane.tsx:46-71,156-254` coalesces initial attaches per `sessionId` via `acquireInitialAttach` (`CR-02`), tracks live `seq` via `onOutputSeq`, calls `termSessionAttach(sid, lastSeqRef.current)` strictly on `SnapshotResync` or `Reconnect`, and removes post-submit re-attach (`TerminalTab.tsx:349-352`), eliminating `64 KiB` `replay_utf8` IPC storms. | closed |
| T-443-02 | Tampering / Cross-Site Scripting (XSS) | `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts`, `BlockCard.tsx`, `ChatTerminalBlockCard.tsx` | high | mitigate | `sgrToSpans.ts:1-380` parses SGR attributes, `\r` / `CSI K` overwrites (`WR-04`), and link tokens into pure React `<span>` and `<button type="button">` elements with **zero** `dangerouslySetInnerHTML` or `innerHTML` assignments across all Console and Chat terminal components (verified via `rg` exit `1` and `<script>alert(1)</script>` unit tests in `sgrToSpans.test.tsx` and `ChatTerminalBlockCard.test.tsx`). | closed |
| T-443-03 | Elevation of Privilege / Malicious URI & Argument Injection | `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts`, `xtermAddons.ts` | high | mitigate | `isSafeTerminalUrl` (`xtermAddons.ts:16-30`) parses URLs via `new URL()` and strictly allowlists `http:` and `https:` protocols (rejecting `javascript:`, `file:`, `data:`, `vbscript:`). For `file:line(:col)` matches, `sanitizeWorkspaceFileLocator` (`sgrToSpans.ts:39-63`, `CR-01`) strips trailing `file:///` URL suffixes so `file:///etc/passwd:1` cannot smuggle `/etc/passwd:1`, rejects `-` CLI flag injection (`--goto`), `..` traversal segments, leading `/` or `\`, Windows drive prefixes (`C:\`), URL schemes, and control chars before invoking `voxTransport.openLocator`. | closed |
| T-443-04 | Information Disclosure (Credential Shoulder-Surfing & State Retention) | `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx` | high | mitigate | When `block.credential_prompt === true`, `BlockCard.tsx:194-224` renders `data-testid="term-block-stdin-input"` with `type="password"`, `autoComplete="off"`, `spellCheck={false}`, clears React `stdinText` state immediately on submit (`setStdinText('')`) and when leaving `AwaitingInput` (`useEffect` at `L75-77`), and never echoes submitted credential input into `BlockCard`, `ChatTerminalBlockCard`, or toast notifications. | closed |
| T-443-05 | Tampering / UI-Backend Desynchronization | `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx`, `TerminalSessionPane.tsx` | high | mitigate | Security tier (`L1..L5`) and control mode (`manual` / `co_pilot` / `autopilot`) changes invoke `voxTransport.termSessionSetSecurity` and `voxTransport.termSessionSetControl` (`Console.tsx:266-288`) and reconcile incoming `SecurityLevelChanged`, `ControlModeChanged`, and `SnapshotResync` events per session (`WR-06`). Any human input across `InputEditor` submit, raw VT `TerminalTab` keystrokes, or `BlockCard` interactive stdin (`TerminalSessionPane.tsx:314-325`, `Console.tsx:251-264`, `CR-05`) immediately demotes `autopilot -> manual` locally and on the backend. | closed |
| T-443-06 | Information Disclosure (Error & Secret Leakage in Toasts) | `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx`, `TerminalSessionPane.tsx`, `BlockCard.tsx`, `ChatTerminalBlockCard.tsx` | low | mitigate | Every `pushToast` error path passes `err` through `sanitizeErrorForToast(err)` and never interpolates raw command strings, stdin payloads, or terminal output bodies (`toastBodyGuard.test.ts` green). | closed |
| T-443-07 | Denial of Service / Unsolicited Panel Resurrection & Stale Stream Mutation | `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx`, `ChatTranscript.tsx`, `TurnTrace.tsx` | medium | mitigate | `ChatSurface.tsx:1312` keeps `dockApiRef.current?.getPanel('terminal')?.update({ params: {} })` strictly update-only without calling `addPanel('terminal')`, honoring `closedPanelIds` (`T-443-07`); `TerminalDockPanel` queries `termSessionList()` before opening a session (`WR-03`); `ChatTranscript.tsx:207-232` and `TurnTrace.tsx:41-53` gate live `vox://terminal/event` streaming strictly to active turns and `vox_terminal_exec` / `vox_terminal_send_input` tools so historical completed turn receipts remain immutable (`CR-04`). | closed |
| T-443-SC | Supply Chain / Resource Exhaustion | `crates/vox-gui/ui/package.json`, `Cargo.lock`, `webglAddon.ts` | medium | mitigate | Zero new npm packages or Rust crates were added in Phase 443 (all `@xterm/*`, `@tanstack/react-virtual`, and `react-resizable-panels` dependencies were pre-installed). `webglAddon.ts:26-51` (`CR-03`) immediately releases its temporary WebGL2 capability probe context via `WEBGL_lose_context.loseContext()` and falls back cleanly to DOM rendering on `onContextLoss`, preventing Chromium 4-pane WebGL context-pool exhaustion. | closed |

---

## Accepted Risks Log

No accepted risks.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-15 | 8 | 8 | 0 | Antigravity (`gsd-verifier` adversarial source & test verification + `443-REVIEW-FIX`) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-15
