---
title: "Warp-Class Roman Scriptorium Terminal & Bidirectional Chat Harness Research (v7.5)"
description: "Architecture, stack, feature parity, and pitfall audit for Milestone v7.5 upgrading vox-terminal-core, vox-orchestrator-mcp, and Vox Axis Console/Chat into a stateful Warp-class agentic terminal."
category: "Architecture SSOTs"
status: "current"
topic: "desktop-app"
domain: "gui"
authorship: "llm-assisted"
authorship_model: "gemini-4-carbon"
---

# Warp-Class Roman Scriptorium Terminal & Bidirectional Chat Harness Research (v7.5)

<!-- kb:summary -->
Documents the root-cause audit and shipped architecture for Milestone v7.5 (`axis-warp-terminal-v75`), upgrading `vox-terminal-core`, `vox-orchestrator`, `vox-orchestrator-mcp`, `vox-term`, and Vox Axis `Console`/`Chat` into a zero-spam, stateful Warp-class agentic terminal. Covers zero-echo five-shell `OSC 633`/`OSC 7` bootstrap, retained `MasterPty` resize, `alacritty_terminal` `ALT_SCREEN` auto-handoff, AST-enforced `L1–L5` `ExecPolicyV1` + Co-Pilot approvals, server-redacted `@term` citations, Dual-Surface Roman Scriptorium theming (`--font-terminal-mono` + 64 WCAG ANSI-16 tokens), and multi-pass `5×4` CV Flywheel convergence.
<!-- /kb:summary -->

## 1. Baseline Root-Cause Audit (Resolved in `v7.5`)

Auditing `crates/vox-terminal-core`, `crates/vox-orchestrator-d`, and `crates/vox-gui` (`console-before.png`) identified six concrete defects explaining startup spam, broken shell state, and theme contrast leaks — all remediated across Phases 441–446:

| Defect | Baseline Source Location | Shipped Architectural Fix (`v7.5`) |
| --- | --- | --- |
| **Startup hook echo spam & broken `zsh` `$?`** | `crates/vox-terminal-core/src/pty.rs` (`spawn_pty` wrote `OSC 633` hooks to PTY stdin while `ECHO` was on; `__vox_trappost` unhooked) | Per-session `tempfile::TempDir` non-echo launch (`ZDOTDIR`, `--rcfile`, `fish -i -C`, `pwsh -NoLogo -NoExit -Command`, `nu -e`) + immediate `$?` capture in `__vox_precmd`. |
| **Stateless per-command subprocesses & dead resize** | `crates/vox-terminal-core/src/session.rs` (`run_shell_capture` spawned `sh -c`) & `pty.rs` (`MasterPty` dropped into reader thread) | Routes `Session::submit` over the live persistent `PtyHandle` (`cd`, `export`, virtualenvs, stdin persist) and retains `Box<dyn MasterPty + Send>` for real `SIGWINCH`/ConPTY `resize`. |
| **2 MiB `replay_utf8` re-write storm & UTF-8 split corruption** | `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx` & `osc633.rs` | Single attach per mount, raw byte/`after_seq` delta streaming, 4-byte `utf8_tail` reassembly across 4096-byte PTY reads, and escape-aligned ring eviction. |
| **Detached WebGL stub & split stacked boxes** | `webglAddon.ts` (unmounted `<canvas>`) & `Console.tsx` (`BlockStreamView` stacked over `TerminalTab`) | Installed `@xterm/xterm` + `@xterm/addon-fit` paired with in-repo addon adapters (`webglAddon.ts` WebGL2 probe + DOM fallback, `xtermAddons.ts` search/web-links/Unicode11/serialize) & `xterm.css`; unified `Console` and `ChatSurface` `TerminalDockPanel` on `<TerminalSessionPane>` with `<16 ms` `ALT_SCREEN` (`?1049h`) Auto-TUI takeover. |
| **`Palatino` `--font-mono` collision & `1.03:1` dark-on-dark `.text-brass`** | `crates/vox-gui/ui/src/index.css` & `ui_theme_contract.rs` | Introduced allowlisted `--font-terminal-mono` (`'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace`), isolated `:not(.xterm *)`, and shipped Dual-Surface Roman Scriptorium 16-color ANSI palettes across all 4 theme profiles. |
| **Client-side unredacted `@term` leaks & substring policy bypass** | `Loquela.tsx` (`termBridge.blocks` regex) & `host.rs` (`allows_agent_command`) | Moved `TerminalBridge` + AST/lexical `ExecPolicyV1` into `vox-orchestrator` (`L3`), gated `vox_terminal_*` with Co-Pilot `PendingApprovals`, and scrubbed all `@term` context on the backend via ANSI-stripped `vox-redact` + Clavis exact-value matching. |

## 2. Zero-New-Edge Layer Architecture (`L0–L5`)

Placing `terminal_bridge::TerminalBridge` and `oversight::exec_policy::ExecPolicyV1` in `vox-orchestrator` (`L3`) connects `vox-terminal-core` (`L3`), `vox-orchestrator-mcp` (`L3`), `vox-term` (`L4`), `vox-orchestrator-d` (`L5`), and `vox-gui` (`L5`) with **zero new external Rust crates in `Cargo.lock`** and **zero new workspace Cargo edges** in `contracts/ci/crate-edges.allow.v1.json`:

```vox-diagram
graph {
    direction: down
    node GUI: "vox-gui (L5): Console + Chat TerminalDockPanel"
    node TUI: "vox-term (L4): ratatui --attach"
    node DAEMON: "vox-orchestrator-d (L5): TermDispatch"
    node CORE: "vox-terminal-core (L3): Stateful PtyHandle + alacritty_terminal"
    node MCP: "vox-orchestrator-mcp (L3): vox_terminal_* + @term"
    node ORCH: "vox-orchestrator (L3): TerminalBridge + ExecPolicyV1 + Floor"
    node LLM: "vox-actor-runtime::llm (L3): Model-Agnostic Facade"
    node REDACT: "vox-redact + vox-secrets (L1)"
    GUI -> DAEMON
    TUI -> DAEMON
    DAEMON -> CORE
    DAEMON -> MCP
    CORE -> ORCH
    MCP -> ORCH
    MCP -> LLM
    CORE -> REDACT
    MCP -> REDACT
}
```

## 3. Shipped Capability Pillars (Phases 441–446)

1. **Stateful PTY & Lossless Streaming (`Phase 441`):** Non-echo 5-shell startup, real `MasterPty::resize`, `SIGINT` (`term.signal`), `alacritty_terminal` `ALT_SCREEN` + `\r` progress overwrite normalization, `OSC 7` / `OSC 633;P;Cwd`, and `vox term --attach` continuity.
2. **Bidirectional Chat ↔ Terminal Harness (`Phase 442`):** `vox_terminal_{exec,read,send_input,sessions}` in `contracts/operations/catalog.v1.yaml`, `PAGER=cat`, `AwaitingInput` interactive prompt/stall detector, `Ctrl+B` background promotion, AST `L1–L5` `ExecPolicyV1` + `classify_floor`, and server-redacted `@term:<sid>#<id>:L<a-b>` citations.
3. **Unified `<TerminalSessionPane>` & Roman Scriptorium Theme (`Phase 443`):** `@xterm/xterm` + `@xterm/addon-fit` paired with in-repo `WebglTerminalAddon` (DOM fallback on context loss) & `xtermAddons.ts`, `sgrToSpans.ts` virtualized blocks, Auto-TUI `?1049h` takeover, split panes & named tabs, live `DockWorkspaceShell` `TerminalDockPanel`, and WCAG >= 4.5:1 Dual-Surface Roman Scriptorium palettes across `arcane`, `slate`, `travertine`, and `high-contrast`.
4. **Warp-Grade Editor, Completions & Local Runbooks (`Phase 444`):** Multi-line `ScriptoriumPromptEditor` (`Shift+Enter`, auto Shell/Agent/`!nu`/`.vox`/`/slash` routing), `Up`/`Down` + `Ctrl+R` fuzzy history, `$PATH`/file/git/CLI-flag completions, structured `!nu`/`.vox` table inspector, 1-click subshell "Voxify", and git-tracked `.vox` / Markdown Runbooks.
5. **Terminal-Native Agentic Mode & Trajectory Flywheel (`Phase 445`):** Streaming `vox_actor_runtime::llm` terminal agent turns, 1-click *Explain / Fix Failed Block*, inline Co-Pilot Approval Cards (`Approve`, `Edit & Run`, `Allow in Session`, `Reject + Steer`), live `/budget` `/model` `/skills` `/memory` `/context` slash commands, and redacted `vox-journal` → MENS feedback signals.
6. **Multi-Pass CV Flywheel Convergence (`Phase 446`):** Shipped `TERM-ROMA-CV-01..04`: `23 / 23` Theme Contract tokens verified (`ENFORCED · ZERO DRIFT`), `64`-color WCAG ANSI-16 palettes, `:not(.xterm)` / `[data-terminal-well="true"]` typography containment, `terminalAudits.ts` DOM/contrast invariants, and `112 / 112` passing Playwright + `@ngi/platform-ssot` + VLM convergence across `900x600`..`1920x1080` and all 4 themes (`minVisualAfterScore = 0.9282 >= 0.92`, `minVlmOverallScore = 0.9506 >= 0.90`, `0` violations).

## 4. Shipped Roman Scriptorium Theme & CV Flywheel Convergence (`Phase 446`)

### 4.1 Dual-Surface Terminal Well Tokens & Typography Containment (`TERM-ROMA-CV-01`)

`crates/vox-gui/ui/src/index.css` and `crates/vox-gui/ui/src/components/surfaces/Console/terminalTheme.ts` isolate terminal typography via `--font-terminal-mono: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace` (`fontVariantLigatures: 'none'`, `fontFeatureSettings: '"calt" 0, "liga" 0'`, `letterSpacing: '0px'`) while maintaining `23 / 23 Verified` (`ENFORCED · ZERO DRIFT`) in the Settings Theme Contract Ledger (`e2e/review/cv-flywheel.spec.ts` `-g "Phase 386"`).

Roman display/small-caps typography and input/button style rules in `index.css` explicitly exclude `.xterm`, `.xterm *`, `.xterm-helper-textarea`, `[data-terminal-mono="true"]`, `[data-terminal-mono="true"] *`, `[data-terminal-well="true"]`, and `[data-terminal-well="true"] *` via `:not(...)` selectors so Palatino / all-small-caps styling never leaks into terminal cells, raw VT canvases, or `ScriptoriumPromptEditor`.

| Theme (`data-theme`) | `--terminal-well-bg` | `--terminal-well-fg` | `--terminal-well-muted` | `--terminal-well-border` | `--terminal-well-cursor` | `--terminal-well-selection` |
| --- | --- | --- | --- | --- | --- | --- |
| `arcane` | `#15110d` | `#f5efe2` | `#a89d8e` | `rgba(197, 155, 39, 0.52)` | `#e0c478` | `rgba(212, 175, 55, 0.28)` |
| `travertine` | `#171512` | `#f6f1e7` | `#ada395` | `rgba(138, 106, 38, 0.52)` | `#79c8ba` | `rgba(121, 200, 186, 0.26)` |
| `slate` | `#0c0e10` | `#f1f5f9` | `#94a3b8` | `#2b343a` | `#38bdf8` | `rgba(56, 189, 248, 0.24)` |
| `high-contrast` | `#000000` | `#ffffff` | `#cbd5e1` | `#ffffff` | `#fde047` | `rgba(253, 224, 71, 0.32)` |

### 4.2 64-Color WCAG ANSI-16 Palette Matrix (`TERM-ROMA-CV-02`)

`terminalTheme.ts` exports 4 synchronized 16-color palettes (`64` ANSI tokens total) consumed dynamically via `resolveTerminalWellSpec`, `resolveTerminalTheme`, and `useDocumentThemeAttr` (`MutationObserver` on `data-theme` and `class`):

| ANSI Slot | `arcane` (`#15110d`) | `travertine` (`#171512`) | `slate` (`#0c0e10`) | `high-contrast` (`#000000`) |
| --- | --- | --- | --- | --- |
| `black` (SGR 30) | `#8b7e6d` | `#8e8477` | `#7c8b9e` | `#a3b1c6` |
| `red` (SGR 31) | `#e57368` | `#e5766b` | `#f87171` | `#ff9494` |
| `green` (SGR 32) | `#7cc47f` | `#7ec684` | `#4ade80` | `#86efac` |
| `yellow` (SGR 33) | `#e0b44c` | `#dfb653` | `#fbbf24` | `#fde047` |
| `blue` (SGR 34) | `#7aa8e8` | `#7daae8` | `#60a5fa` | `#93c5fd` |
| `magenta` (SGR 35) | `#c98fd8` | `#ca93d9` | `#c084fc` | `#e9d5ff` |
| `cyan` (SGR 36) | `#68bfc2` | `#6bc1c4` | `#38bdf8` | `#67e8f9` |
| `white` (SGR 37) | `#ebe3d3` | `#ede6d8` | `#e2e8f0` | `#ffffff` |
| `brightBlack` (SGR 90) | `#a89a87` | `#aba092` | `#94a3b8` | `#cbd5e1` |
| `brightRed` (SGR 91) | `#f48fb1` | `#f2948a` | `#fca5a5` | `#fecaca` |
| `brightGreen` (SGR 92) | `#98db9b` | `#9bdd9f` | `#86efac` | `#bbf7d0` |
| `brightYellow` (SGR 93) | `#f0cb69` | `#f1ce70` | `#fde047` | `#fef08a` |
| `brightBlue` (SGR 94) | `#9ec2f7` | `#a0c4f7` | `#93c5fd` | `#bfdbfe` |
| `brightMagenta` (SGR 95) | `#dfb0ec` | `#e1b3ee` | `#d8b4fe` | `#f3e8ff` |
| `brightCyan` (SGR 96) | `#8ad8db` | `#8ddadc` | `#7dd3fc` | `#a5f3fc` |
| `brightWhite` (SGR 97) | `#fbf7ee` | `#fcf9f2` | `#f8fafc` | `#ffffff` |

### 4.3 Terminal Audit Gate & Pixel/VLM Flywheel (`TERM-ROMA-CV-03` & `TERM-ROMA-CV-04`)

`crates/vox-gui/ui/e2e/review/terminalAudits.ts` and `crates/vox-gui/ui/e2e/review/terminal-cv-flywheel.spec.ts` enforce hard zero tolerances across all `8` `TERMINAL_CV_STATES` (`112` cells: 5 viewports × 4 themes on `block-stream-ansi`, `split-panes`, `auto-tui-alt-screen`, and `chat-terminal-dock`; `900x600` & `1440x900` × 4 themes on `history-search-open`, `json-ledger-table`, `runbooks-open`, and `approval-card-pending`):

- **Zero-tolerance terminal invariants:** `terminalStartupSpamCount === 0`, `xtermHelperLeakCount === 0`, `terminalFontProportionalLeakCount === 0`, `terminalContrastViolationCount === 0`.
- **Zero-tolerance DOM & Axe invariants (`audits.ts`):** `outerOverflowPx === 0`, `clippedInteractiveCount === 0`, `overlappingControlPairs === 0` (evaluated over visible unclipped bounding boxes within viewport + `overflow-x`/`overflow-y` ancestor containers), `unstyledNativeSelectCount === 0`, `axeViolations === 0` (WCAG 2.0 A/AA serious/critical).
- **CV Flywheel convergence thresholds (`@ngi/platform-ssot` `2026.10.19-v3.5` + `vlmVisionFlywheel.ts`):** `visualAfterScore >= 0.92` and `vlmOverallScore >= 0.90`.

Measured `CV_STAGE=after` ledger (`crates/vox-gui/ui/test-results/cv-flywheel-terminal/ledger-after.json` — **`112 / 112` cells passed, `0` failed**, `minVisualAfterScore: 0.9284`, `minVlmOverallScore: 0.9506`):

| Dimension | Slice | Cells (`n`) | Min `visualAfterScore` | Mean `visualAfterScore` | Max `visualAfterScore` | Min `vlmOverallScore` |
| --- | --- | --- | --- | --- | --- | --- |
| **Theme** | `arcane` | 28 | `0.9302` | `0.9463` | `0.9665` | `0.9665` |
| **Theme** | `slate` | 28 | `0.9284` | `0.9360` | `0.9405` | `0.9506` |
| **Theme** | `travertine` | 28 | `0.9288` | `0.9458` | `0.9665` | `0.9667` |
| **Theme** | `high-contrast` | 28 | `0.9396` | `0.9460` | `0.9517` | `0.9674` |
| **Viewport** | `900x600` | 32 | `0.9334` | `0.9450` | `0.9665` | `0.9529` |
| **Viewport** | `1100x720` | 16 | `0.9326` | `0.9449` | `0.9665` | `0.9508` |
| **Viewport** | `1280x800` | 16 | `0.9312` | `0.9436` | `0.9665` | `0.9512` |
| **Viewport** | `1440x900` | 32 | `0.9299` | `0.9418` | `0.9665` | `0.9517` |
| **Viewport** | `1920x1080` | 16 | `0.9284` | `0.9426` | `0.9665` | `0.9506` |
| **State** | `block-stream-ansi` (`console`) | 20 | `0.9307` | `0.9488` | `0.9665` | `0.9540` |
| **State** | `split-panes` (`console`) | 20 | `0.9288` | `0.9364` | `0.9504` | `0.9536` |
| **State** | `history-search-open` (`console`) | 8 | `0.9308` | `0.9379` | `0.9512` | `0.9525` |
| **State** | `json-ledger-table` (`console`) | 8 | `0.9304` | `0.9371` | `0.9496` | `0.9537` |
| **State** | `runbooks-open` (`console`) | 8 | `0.9354` | `0.9502` | `0.9616` | `0.9558` |
| **State** | `approval-card-pending` (`console`) | 8 | `0.9312` | `0.9380` | `0.9505` | `0.9523` |
| **State** | `auto-tui-alt-screen` (`console`) | 20 | `0.9284` | `0.9533` | `0.9665` | `0.9537` |
| **State** | `chat-terminal-dock` (`chat`) | 20 | `0.9306` | `0.9400` | `0.9491` | `0.9506` |
