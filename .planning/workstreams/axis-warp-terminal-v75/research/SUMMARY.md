# Project Research Summary

**Project:** Vox (`v7.5` / `--ws axis-warp-terminal-v75`)
**Domain:** Roman Scriptorium Warp-Class Agentic Terminal, Stateful PTY Engine, Bidirectional Chat Harness & CV Flywheel Convergence
**Researched:** 2026-11-14
**Confidence:** HIGH

## Executive Summary

Milestone `v7.5` transforms Vox's prototype `Console` surface and `vox-terminal-core` engine into a zero-spam, stateful Warp-class (`95%` capability parity) agentic terminal tightly integrated with the Loquela `Chat` harness and Vox Axis's 4-profile Roman Scriptorium ("Limes") theme contract. Line-by-line codebase auditing pinpointed the exact root causes of today's broken terminal behavior: `spawn_pty` (`pty.rs:368`) writes 15–25 lines of raw `OSC 633` shell hook source straight to PTY stdin while `ECHO` is active; `ZSH_OSC633` defines `__vox_trappost` without registering it (reporting `exit 0` for every failed zsh command); `PtyHandle::resize` (`pty.rs:319`) is a literal no-op; `Session::submit` (`session.rs:79`) spawns isolated stateless `Command::new(shell) -c` subprocesses (losing `cd`, `export`, virtualenvs, and streaming stdin/stdout); `TerminalTab.tsx` re-attaches and re-writes the full 2 MiB `replay_utf8` ring buffer onto `xterm.js` on every command; and `webglAddon.ts` is an unmounted `<canvas>` stub lacking `@xterm/addon-webgl` and `@xterm/xterm/css/xterm.css`.

The recommended architecture resolves every defect with **zero new external Rust crates in `Cargo.lock`** (`portable-pty 0.9.0`, `alacritty_terminal 0.26.0`, `vte 0.15.0`, `tempfile 3`, and `vox-redact` are already locked in the workspace) and **zero new workspace Cargo edges** in `contracts/ci/crate-edges.allow.v1.json`. Placing the pure `TerminalBridge` async trait and `ExecPolicyV1` AST/lexical evaluator in `vox-orchestrator` (`L3`) bridges `vox-terminal-core` (`L3`), `vox-orchestrator-mcp` (`L3`), `vox-orchestrator-d` (`L5`), `vox-term` (`L4`), and `vox-gui` (`L5`) cleanly downward across `layers.toml`. On the frontend, `Console.tsx` and `ChatSurface.tsx` (`DockWorkspaceShell` `TerminalDockPanel`) converge onto a single `<TerminalSessionPane>` that renders Roman Scriptorium Command Blocks (`sgrToSpans.ts` + `@tanstack/react-virtual`) in normal shell/agent operation and seamlessly hands off (`<16 ms`) to GPU-accelerated `@xterm/addon-webgl` when `alacritty_terminal` detects `DECSET 1049` (`ALT_SCREEN`) for full-screen TUIs (`vim`, `lazygit`, `htop`).

Key engineering risks—including Vox Axis's global proportional `Palatino` `--font-mono` remap (`index.css`), `1.03:1` dark-on-dark `.text-brass` contrast leaks (`console-before.png`), UTF-8 multi-byte splitting across 4096-byte PTY reads, interactive pager/prompt deadlocks in agent tool loops, and unredacted client-side `@term` secret leaks (`Loquela.tsx:1035`)—are mitigated upfront through dedicated `--font-terminal-mono` CSS containment (`:not(.xterm *)`), stateful UTF-8 + escape boundary framing, `PAGER=cat` + `AwaitingInput` stall detection + `SIGINT` (`term.signal`), and mandatory backend `vox-redact` + Clavis secret scrubbing before any terminal output reaches `vox_actor_runtime::llm`.

## Key Findings

### Recommended Stack

All Rust engine capabilities are satisfied by promoting workspace-locked crates already present in `Cargo.lock` into `crates/vox-terminal-core/Cargo.toml` (`alacritty_terminal 0.26.0` for authoritative `ALT_SCREEN` / `BRACKETED_PASTE` / `\r` overwrite normalization, and `tempfile 3` for RAII zero-echo shell startup directories), keeping 100% compliance with Vox's pure-Rust build-toolchain invariant (no `cmake`, `nasm`, `Go`, `perl`, or `libclang`) and Apache-2.0 clean-room rule (`D20`, zero AGPL Warp code).

**Core technologies:**
- `portable-pty 0.9.0` + `tempfile 3` (`vox-terminal-core`): Retain `Box<dyn MasterPty + Send>` on `PtyHandle` for real `SIGWINCH`/ConPTY resize and launch `zsh` (`ZDOTDIR`), `bash` (`--rcfile`), `fish` (`-i -C`), `pwsh` (`-NoLogo -NoExit -Command`), and `nu` (`-e` with built-in `osc633 = false`) with zero stdin echo spam.
- `alacritty_terminal 0.26.0` + `vte 0.15.0` (`vox-terminal-core`): Single authoritative VT state machine tracking `TermMode::ALT_SCREEN` (`?1049h`/`?1049l`), `OSC 633` (`A/B/C/D/E/P;Cwd`), `OSC 7` cwd, and `\r` progress-bar line normalization.
- `@xterm/xterm@^6.0.0` + `@xterm/addon-{webgl,fit,search,web-links,unicode11,serialize}` (`vox-gui/ui`): Real WebGL2 glyph-atlas terminal canvas (with `onContextLoss` + `no-tauri` DOM fallback) paired with `@xterm/xterm/css/xterm.css` and first-party theme-aware `sgrToSpans.ts` React block rendering.

### Expected Features

Detailed in [`FEATURES.md`](FEATURES.md): targets 95% Warp 2.0 capability parity + 2026 SOTA agentic harness integration while explicitly excluding the 5% of Warp cloud/SaaS lock-in.

**Must have (table stakes):**
- Stateful persistent PTY command execution (`cd`, `export`, virtualenvs, streaming stdout/stderr, real `MasterPty::resize`, `SIGINT`)
- Unified Command Block stream (`cwd`, git branch, exit code, duration, ANSI colors, collapse/expand, sticky headers, `file:line:col` links) + automatic `ALT_SCREEN` TUI takeover (`vim`, `htop`, `lazygit`)
- Universal multi-line input editor (`Shift+Enter`) with auto-intent detection (Shell / Agent `?` / Nushell `!nu` / `.vox` / `/slash`), `Up`/`Down` & `Ctrl+R` history, and `$PATH`/file/git/CLI-flag completions
- Bidirectional Chat ↔ Terminal `vox_terminal_{exec,read,send_input,sessions}` tools (foreground + `Ctrl+B` background promotion, interactive `[y/N]`/`Password:` stall detection, `L1–L5` AST `exec-policy.v1.yaml` + Co-Pilot Approval Cards)
- Server-redacted `@term`, `@term:last`, `@term:failed`, `@term:pinned`, and `@term:<sid>#<id>:L<a-b>` citations + real streaming terminal Agent Mode via `vox_actor_runtime::llm`

**Should have (competitive differentiators):**
- Shared `SessionHost` live continuity across headless `vox term --attach`, Axis `Console`, and `Chat` `TerminalDockPanel`
- Dual-Surface Roman Scriptorium ("Limes") theme system: Carrara marble & burnished gold chrome + warm dark-bronze/obsidian high-contrast 16-color ANSI viewport in `arcane`, synced across all 4 theme profiles
- Local git-tracked `.vox` & runnable Markdown Runbooks (`Cmd+P` palette), split panes & live `cwd`/process tab titles, 1-click subshell "Voxify", structured `!nu`/`.vox` table viewer, and redacted `vox-journal` → MENS trajectory feedback

**Defer (v8+ / Out of Scope):**
- Mandatory cloud login/accounts, hosted SaaS "Warp Drive" secrets, public cloud block permalinks, full multi-tab code editor inside Console, AGPL Warp code, and speculative post-command ghost LLM calls before local corpus maturity.

### Architecture Approach

Detailed in [`ARCHITECTURE.md`](ARCHITECTURE.md): strict `L0 -> L5` downward dependency flow with zero new Cargo crate edges and sub-500-LoC modular files.

**Major components:**
1. `vox-terminal-core` (`L3`): `shell_init.rs` (non-echo `TempDir` rcfiles), `pty.rs` (`MasterPty` resize + `SIGINT`), `escape_scanner.rs` (UTF-8-safe streaming `OSC 633`/`OSC 7`/`?1049h`), and stateful `Session` + `LocalHost`/`DaemonHost` with `after_seq` delta streaming.
2. `vox-orchestrator` (`L3`) & `vox-orchestrator-d` (`L5`): Shared `TerminalBridge` trait + `ExecPolicyV1` AST/lexical policy checker in `vox-orchestrator`, backed by `TermDispatch` (`term.{delta,signal,control,context}`) in `vox-orchestrator-d`.
3. `vox-orchestrator-mcp` (`L3`): `vox_terminal_{exec,read,send_input,sessions}` registered in `contracts/operations/catalog.v1.yaml`, Co-Pilot `PendingApprovals` HITL gating, server-redacted `@term` mention expansion, and `vox_actor_runtime::llm` streaming terminal agent turns.
4. `vox-gui` (`L5`) & `vox-term` (`L4`): Shared `<TerminalSessionPane>` across `Console` and `DockWorkspaceShell` `TerminalDockPanel`, `--font-terminal-mono` + 4-profile `terminalTheme.ts`, Runbooks/panes/completions, `vox term --attach` wiring, and Playwright + VLM CV Flywheel verification.

### Critical Pitfalls

Detailed in [`PITFALLS.md`](PITFALLS.md):
1. **Stdin Hook Echo Spam & Broken `zsh` Exit Codes (`pty.rs:62, 368`)** — Never write init hooks to PTY stdin; use `ZDOTDIR`/`--rcfile`/`-NoLogo -NoExit -Command`/`fish -C`/`nu -e`, capture `$?` first in `__vox_precmd`, wrap prompt OSC in `%{...%}`, and handle macOS `PROMPT_SP` (`%`).
2. **Full `replay_utf8` Re-Write Storm & `RecvError::Lagged` Freeze (`TerminalTab.tsx:203`, `term_dispatch.rs:252`)** — Attach once per mount, feed raw `Uint8Array` deltas (`seq > lastSeq`), coalesce 16 ms frames, and auto-resync via `term.delta` on lag.
3. **UTF-8 Multi-Byte Boundary Corruption (`osc633.rs:84`, `pty.rs:227`)** — Retain a 4-byte `utf8_tail` across 4096-byte PTY reads and align ring-buffer eviction to valid UTF-8 + ANSI escape boundaries.
4. **Proportional `Palatino` `--font-mono` & Dark-on-Dark `.text-brass` Collisions (`index.css:21, 745`, `ui_theme_contract.rs:36`)** — Add allowlisted `--font-terminal-mono` (`'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace`), isolate `:not(.xterm *)` in `index.css` (protecting `.xterm-helper-textarea`), and validate WCAG >= 4.5:1 across all 16 ANSI colors in all 4 themes.
5. **Interactive Pager/Prompt Deadlocks & Unredacted `@term` Leaks (`Loquela.tsx:1035`, `host.rs:76`)** — Set `PAGER=cat`/`GIT_PAGER=cat`, detect `?1049h` + stdin prompt stalls (`AwaitingInput`), enforce AST `ExecPolicyV1` + `classify_floor`, and route all `@term`/`SendToAgent` payloads through ANSI-stripped `vox-redact` + Clavis exact-value scrubbing on the backend.

## Implications for Roadmap

Based on dependency order and pitfall prevention, Milestone `v7.5` decomposes cleanly into **6 phases (`Phases 441–446`)**:

### Phase 441: Zero-Spam Stateful PTY Engine, Non-Echo Shell Bootstrap & Lossless VT/OSC Streaming (`vox-terminal-core` + `vox-term`)
**Rationale:** Root foundation — eliminates startup echo spam, broken `zsh` exit codes, stateless `Command::new` split-brain, no-op `resize`, and UTF-8 boundary corruption before any higher-level harness or UI layer builds on top.
**Delivers:** `shell_init.rs` (5-shell zero-echo `TempDir` bootstrap), `MasterPty` resize + `SIGINT` (`pty.rs`), `alacritty_terminal`-backed `ALT_SCREEN` + UTF-8-safe `escape_scanner.rs` (`OSC 633` + `OSC 7` + `\r` overwrite normalization), stateful PTY `Session::submit`, `after_seq` delta buffer, and live `vox term --attach` CLI continuity.
**Addresses:** Stateful PTY, zero load spam, TUI `ALT_SCREEN` detection, `vox term` continuity.
**Avoids:** Pitfalls 1, 2, 3, and 7.

### Phase 442: AST Exec-Policy Gate, Daemon `term.*` Control Plane & Bidirectional MCP/Chat Terminal Harness
**Rationale:** Connects `vox-terminal-core` to `vox-orchestrator`, `vox-orchestrator-d`, and `vox-orchestrator-mcp` with zero new crate edges before exposing Chat/Agent execution in the UI.
**Delivers:** `vox-orchestrator::terminal_bridge` + shared AST/lexical `ExecPolicyV1` (`L1–L5` + `classify_floor`), `term.{delta,signal,control,security,context}` daemon RPCs, `vox_terminal_{exec,read,send_input,sessions}` MCP tools (foreground + `Ctrl+B` background + `PAGER=cat` + `AwaitingInput` stall detection), Co-Pilot `PendingApprovals` HITL gate, and server-redacted `@term:<sid>#<id>:L<a-b>` citations (ANSI-stripped `vox-redact` + Clavis scrub).
**Uses:** `vox-orchestrator`, `vox-orchestrator-d`, `vox-orchestrator-mcp`, `contracts/operations/catalog.v1.yaml`.
**Avoids:** Pitfalls 4 and 6.

### Phase 443: Unified `<TerminalSessionPane>`, Real `@xterm/addon-webgl`, Auto-TUI Handoff & Dual-Surface Roman Scriptorium Theme
**Rationale:** Consolidates the split stacked `BlockStreamView` + `TerminalTab` layout and mocked `ChatSurface` `TerminalDockPanel` onto one shared component while fixing CSS/font/contrast collisions across all 4 theme profiles.
**Delivers:** `@xterm/addon-{webgl,search,web-links,unicode11,serialize}` + `xterm.css`, allowlisted `--font-terminal-mono` + `:not(.xterm *)` containment in `index.css`, Dual-Surface Roman Scriptorium `terminalTheme.ts` (Carrara marble & gold chrome + warm dark-bronze/obsidian WCAG-AA 16-color ANSI viewport in `arcane`, synced across `slate`, `travertine`, `high-contrast`), `sgrToSpans.ts` + virtualized `BlockStreamView` (`cwd`, git branch, collapse, sticky headers, `file:line` links), `<16 ms` `ALT_SCREEN` Auto-TUI takeover, split panes/named tabs, and live `DockWorkspaceShell` `TerminalDockPanel` + inline Chat streaming terminal cards.
**Addresses:** Unified block stream, TUI auto-handoff, split panes, Chat dock sync, Roman Scriptorium theming.
**Avoids:** Pitfalls 1 (`replay_utf8` storm) and 5 (`Palatino` grid & `1.03:1` `.text-brass` contrast leak).

### Phase 444: Warp-Grade Universal Multi-Line Input Editor, Context Completions, History & Local `.vox` / Markdown Runbooks
**Rationale:** Elevates daily operator ergonomics to 95% Warp 2.0 parity once the unified `<TerminalSessionPane>` and stateful PTY are in place.
**Delivers:** Multi-line `ScriptoriumPromptEditor` (`Shift+Enter`, syntax highlighting, lockable/auto Shell vs. `?` Agent vs. `!nu` vs. `.vox` vs. `/slash` mode pill), persistent redacted command history (`Up`/`Down` + `Ctrl+R` fuzzy search), `$PATH` + `cwd`-file + git ref + CLI flag IntelliSense, interactive structured `!nu`/`.vox` Ledger Table viewer, 1-click subshell "Voxify", and parameterized local `.vox` / Markdown Runbooks in `Cmd+P` & `DiscoveryRail`.
**Addresses:** Universal Input, completions, history/search, structured tables, subshell Voxify, Warp Drive local Runbooks.

### Phase 445: Terminal-Native Agentic Mode (`vox_actor_runtime::llm`), Live Slash Commands, Co-Pilot Approval Cards & Redacted MENS Trajectory Flywheel
**Rationale:** Completes agentic control *from* the terminal and closes the local MENS learning loop using the policy-gated `TerminalBridge` and universal editor.
**Delivers:** Real streaming terminal Agent Mode (`?` & natural-language prose routed through `vox_actor_runtime::llm` with multi-step command plans), 1-click *Explain / Fix Failed Block* recovery, inline Co-Pilot Command Approval Cards (`Approve`, `Edit & Run`, `Allow in Session`, `Reject + Steer`) in both Console and Chat with instant keystroke `Autopilot -> Manual` takeover, live `/budget` `/model` `/skills` `/memory` `/context` `/compact` slash commands, and redacted `vox-journal` → `curate` → `CorpusWriter` Accept/Reject/Correct feedback signals.
**Addresses:** Terminal-native Agentic Mode, live slash commands, Co-Pilot UX, MENS corpus flywheel.

### Phase 446: Multi-Resolution × 4-Theme Playwright & VLM Computer-Vision Flywheel Convergence
**Rationale:** Proves zero visual, accessibility, contrast, clipping, or functional regressions end-to-end across standalone `Console` and `Chat` + `TerminalDockPanel` layouts via iterative CV Flywheel passes.
**Delivers:** Deterministic terminal CV metrics in `audits.ts` / `cvFlywheel.ts` / `cv-flywheel.spec.ts` (ANSI contrast >= 4.5:1, true monospace cell alignment, zero `.xterm-helper-textarea` bleed, zero clipped controls/headers across `900x600`, `1100x720`, `1280x800`, `1440x900`, `1920x1080` and all 4 themes), multi-pass `CV_STAGE=before -> Pass 1 -> Pass 2+` convergence (`visualAfterScore >= 0.92`, `0` axe/overflow/clipping/theme-contract defects), and architecture SSOT update.
**Addresses:** Roman theme & CV Flywheel convergence gate.
**Avoids:** Pitfall 7 (cursor-blink / shimmer CV non-determinism).

### Phase Ordering Rationale

- **Bottom-up causal order (`L3` PTY → `L3/L5` Bridge/MCP → `L5` Unified Pane/Theme → Editor/Runbooks → Agentic Mode → CV Flywheel):** Prevents building UI components or agent tools atop stateless subprocesses or echoing stdin hooks.
- **Zero-crate-edge invariant preserved at every boundary:** Phase 442 lands `TerminalBridge` and `ExecPolicyV1` in `vox-orchestrator` first so `vox-orchestrator-mcp` and `vox-gui` compile cleanly without `crate-edges.allow.v1.json` churn.
- **CV Flywheel runs as the dedicated closing convergence gate (Phase 446)** after all Console, Chat Transcript, Approval Card, Runbook, and Split-Pane surfaces are live.

### Research Flags

Phases needing targeted platform verification during phase planning:
- **Phase 441:** Verify `zsh` (`PROMPT_SP`), `bash` (`3.2` vs `5.1+`), and `pwsh` (`PSReadLine`) init scripts under PTY integration tests.
- **Phase 442:** Verify AST tokenization edge cases (`$()`, backticks, `pwsh -EncodedCommand`, `find -exec`) against `contracts/terminal/exec-policy.v1.yaml`.

Phases with well-established internal Vox patterns:
- **Phase 443 & Phase 446:** Follow established `v6.6` `themeContract.ts` and `v7.2` `@ngi/platform-ssot` `cv-flywheel.spec.ts` harnesses.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | 100% of Rust deps verified in `Cargo.lock`; `@xterm/*` v6 addon compatibility verified upstream. |
| Features | HIGH | Grounded in line-by-line audit of `vox-terminal-core`, `vox-gui`, and 2026 Warp 2.0 / Claude Code / Cursor capabilities. |
| Architecture | HIGH | Verified directly against `layers.toml`, `crate-edges.allow.v1.json`, `operations/catalog.v1.yaml`, and `vox_actor_runtime::llm`. |
| Pitfalls | HIGH | Reproduced directly from `console-before.png`, `pty.rs`, `TerminalTab.tsx`, `index.css`, `ui_theme_contract.rs`, and `Loquela.tsx`. |

**Overall confidence:** HIGH

### Gaps to Address

- **Headless WebGL2 availability in CI Playwright (`no-tauri`):** Wrap `@xterm/addon-webgl` activation in try/catch + `onContextLoss` so software-composited headless browsers fall back automatically to xterm's DOM renderer while preserving identical Roman Scriptorium theme colors and `--font-terminal-mono` metrics.
- **Foreground PGID inspection across macOS/Linux/Windows:** Use `tcgetpgrp` on Unix (`rustix`/`libc`) combined with output-idle + trailing prompt line heuristics (`[y/N]`, `Password:`) on Windows ConPTY for cross-platform `AwaitingInput` detection.

## Sources

### Primary (HIGH confidence)
- Workspace codebase audit: `crates/vox-terminal-core/src/{pty,session,host,daemon_host,osc633}.rs`, `crates/vox-orchestrator-d/src/term_dispatch.rs`, `crates/vox-gui/src/commands/pty.rs`, `crates/vox-gui/ui/src/components/surfaces/Console/*`, `crates/vox-gui/ui/src/index.css`, `crates/vox-code-audit/src/detectors/ui_theme_contract.rs`, `contracts/terminal/exec-policy.v1.yaml`.
- Official `@xterm/xterm` v6.0 & `@xterm/addon-{webgl,fit,search,web-links,unicode11,serialize}` upstream documentation & type definitions.
- VS Code Terminal Shell Integration (`OSC 633` specification), Ghostty shell-integration injection (`ZDOTDIR` / `bash --rcfile` / `fish` / `pwsh` / `nu`), and `alacritty_terminal` `0.26.0` `TermMode` API.

### Secondary (MEDIUM confidence)
- Warp 2.0 Agentic Development Environment (ADE) public documentation (Universal Input, Command Blocks, Subshell Warpify, Workflows/Runbooks, Active AI).
- 2026 coding harness terminal execution patterns (Claude Code background bash & prompt handling, Cursor/Windsurf approval gates).

---
*Research completed: 2026-11-14*
*Ready for roadmap: yes*
