# Stack Research

**Domain:** Roman Scriptorium Warp-Class Agentic Terminal, Stateful PTY Engine, Bidirectional Chat Harness & CV Flywheel Convergence (`v7.5` / `axis-warp-terminal-v75`)
**Researched:** 2026-11-14
**Confidence:** HIGH

---

## Executive Baseline: What Is Already Installed vs. What Changes

Auditing `Cargo.toml`, `Cargo.lock`, `crates/vox-terminal-core/`, `crates/vox-term/`, `crates/vox-gui/`, and `crates/vox-gui/ui/package.json` reveals that **100% of the required Rust crates are already resolved in `Cargo.lock`** (`portable-pty 0.9.0`, `alacritty_terminal 0.26.0`, `vte 0.15.0`, `tempfile 3`, `vox-redact 0.6.0`), while the frontend requires **4 small official `@xterm/*` scoped packages** (plus importing the currently-missing `@xterm/xterm/css/xterm.css` stylesheet) to replace a hollow WebGL stub (`crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts`).

### Critical Baseline Audit Findings (Codebase Reality)

1. **`webglAddon.ts` is currently a detached-canvas stub, not a renderer:** `crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts` creates an unmounted `document.createElement('canvas')` and calls `getContext('webgl2')` without hooking xterm's glyph atlas or render service. Official `@xterm/addon-webgl@^0.19.0` must be installed and wrapped by `webglAddon.ts` (keeping deterministic `jsdom` / headless Playwright `no-tauri` fallback).
2. **`@xterm/xterm/css/xterm.css` is not imported anywhere in `crates/vox-gui/ui/src/`:** `@xterm/xterm@6.0.0` rewrote `.xterm-viewport` into a VS Code-style scrollable layout element; without `xterm.css`, `.xterm-helper-textarea` (IME/clipboard/focus), selection overlays, and WebGL canvas layering misalign or clip.
3. **`PtyHandle::resize` (`crates/vox-terminal-core/src/pty.rs:319-324`) is a no-op:** `spawn_pty` moves `pair.master` into the blocking reader thread (`let _keep_master = master;`) instead of retaining `Box<dyn portable_pty::MasterPty + Send>` on `PtyHandle`. Because `MasterPty` implements `Send` in `portable-pty 0.9.0`, storing it inside `PtyHandle` enables real `SIGWINCH` / ConPTY `resize(PtySize)` with zero new crates.
4. **Shell startup currently causes raw stdin echo spam (`pty.rs:368-371`):** `spawn_pty` writes `ZSH_OSC633` / `BASH_OSC633` / `PWSH_OSC633` / `FISH_OSC633` / `NU_OSC633` directly to the PTY stdin writer (`writer.write_all(...)`), causing interactive line editors (`zle`, `readline`, `PSReadLine`, `reedline`) to echo 15–25 lines of shell hook code onto the screen and into command history. Replacing stdin injection with per-shell startup files in an RAII `tempfile::TempDir` eliminates echo spam completely.
5. **`Session::submit` (`session.rs:78-88`) currently runs stateless `std::process::Command`:** `InputIntent::Shell` calls `run_shell_capture(&cmd)` instead of driving the stateful PTY (`cd`, `export`, virtualenvs, SSH/REPL state, and aliases do not persist across blocks), and `Session` has no VT mode tracker for `CSI ? 1049 h` / `CSI ? 1049 l` (`ALT_SCREEN`) when full-screen TUIs (`vim`, `nvim`, `htop`, `less`, `git log`) take over the PTY.

---

## Recommended Stack

### Core Technologies (Already Installed vs. Minimal Additions)

| Technology | Version | Status | Purpose | Why Recommended |
|------------|---------|--------|---------|-----------------|
| `portable-pty` | `0.9.0` (`workspace = true`) | **Installed** (`vox-terminal-core`, `vox-gui`) | Cross-platform PTY allocation (`openpty` on macOS/Linux, ConPTY on Windows), `CommandBuilder` env/arg/cwd control, and `MasterPty::resize` | Pure Rust + platform libc/Win32 (`windows-sys`), zero `cmake`/`libclang`, already battle-tested in `Cargo.lock` (WezTerm's standalone MIT PTY crate). |
| `alacritty_terminal` (+ `vte`) | `0.26.0` / `vte 0.15.0` (`workspace = true`) | **In `Cargo.lock`** (currently in `vox-term`; wire into `vox-terminal-core`) | Authoritative headless VT100/VT220/xterm state machine, `TermMode::ALT_SCREEN` (`?1049h`/`?1049l`, `?1047h`/`?47h`), `BRACKETED_PASTE` (`?2004h`), `APP_CURSOR`, SGR cell grid, and ANSI-clean text projection | Already in `[workspace.dependencies]` and `Cargo.lock` (`crates/vox-term/src/vt.rs`). 100% pure Rust (`rustix`, `vte`, `bitflags`, `unicode-width`, `parking_lot`), Apache-2.0/MIT, zero `Cargo.lock` churn, and prevents adding a competing second VT parser. |
| `tempfile` | `3.x` (`workspace = true`) | **In `Cargo.lock`** (add to `vox-terminal-core` `[dependencies]`) | Per-session RAII `TempDir` holding zero-echo shell startup scripts (`ZDOTDIR/.zshrc`, `bash --rcfile`, `fish`, `pwsh`, `nu`) | Already in `[workspace.dependencies]`. Tying `TempDir` ownership to `PtyHandle` guarantees automatic cleanup when a session closes or drops. |
| `@xterm/xterm` | `^6.0.0` (`6.0.0` locked) | **Installed** (`vox-gui/ui`) | Interactive full-screen / raw PTY terminal emulator in Axis GUI (`Console` and `ChatSurface` `TerminalDockPanel`) | Latest major release; already installed. Must pair with `import '@xterm/xterm/css/xterm.css'` and `allowProposedApi: true`. |
| `@xterm/addon-fit` | `^0.11.0` (`0.11.0` locked) | **Installed** (`vox-gui/ui`) | Container-driven `cols`/`rows` measurement paired with `ResizeObserver` -> `termSessionResize` | Official v6-compatible fit addon; already installed. |
| `@xterm/addon-webgl` | `^0.19.0` | **ADD** (`vox-gui/ui`) | GPU-accelerated WebGL2 glyph-atlas renderer for `@xterm/xterm@6.0.0` | Official replacement for the discontinued `@xterm/addon-canvas` and Vox's current detached-canvas stub (`webglAddon.ts`). Delivers 60–120fps throughput using the Tauri webview's WebGL2 context with zero native Rust GPU dependencies. |

### Supporting Libraries

| Library | Version | Status | Purpose | When to Use |
|---------|---------|--------|---------|-------------|
| `@xterm/addon-search` | `^0.16.0` | **ADD** (`vox-gui/ui`) | Buffer-wide regex/case-sensitive search (`Cmd/Ctrl+F`) with custom Roman Scriptorium highlight decorations | Interactive find bar across raw terminal scrollback and `ALT_SCREEN` sessions. |
| `@xterm/addon-web-links` | `^0.12.0` | **ADD** (`vox-gui/ui`) | Clickable URL & localhost port detection inside xterm canvas (already mocked in `App.test.tsx:28`) | Route clicked links through Tauri opener / Axis `BrowserView` instead of navigating the webview away from Axis. |
| `@xterm/addon-unicode11` | `^0.9.0` | **ADD** (`vox-gui/ui`) | Unicode 11 wide-character, box-drawing, Braille sparkline, and emoji cell-width tables (`term.unicode.activeVersion = '11'`) | Prevents cursor drift and column misalignment on TUI borders (`ratatui`, `btop`, `lazygit`), Nerd Font glyphs, and UTF-8 CLI tables. |
| `@xterm/addon-serialize` | `^0.14.0` | **ADD** (`vox-gui/ui`) | Deterministic VT state & HTML serialization (`serialize()`, `serializeAsHTML()`) | Exporting rich styled terminal snapshots, clipboard HTML copy, and verifying re-attach screen parity in Vitest/Playwright. |
| `@tanstack/react-virtual` | `^3.14.10` | **Installed** (`vox-gui/ui`) | Windowed virtualization of Warp-style command/output blocks in `BlockStreamView.tsx` and `TerminalDockPanel` | Mandatory once sessions exceed ~30 blocks so Roman Scriptorium parchment block cards stay at 60fps with zero DOM bloat. |
| `dockview` | `6.6.1` | **Installed** (`vox-gui/ui`) | Multi-panel Chat workbench hosting `TerminalDockPanel` (`id: 'terminal'`) alongside `transcript` and `executionRail` | Upgrade `TerminalDockPanel` (`ChatSurface.tsx:281-314`) from passive `<pre>` viewer to a live bidirectional mini-Scriptorium terminal sharing the active `SessionHost` session. |
| `vox-redact` | `0.6.0` (`workspace = true`) | **Installed** (`vox-terminal-core`) | PII, bearer-token, and Clavis secret scrubbing before terminal blocks enter Chat `@term` context or MENS training jsonl | Always invoke on `RedactedBlockContext` (`host.rs:428`), MCP `term_run` outputs, and MENS trajectory harvesting. |

### Development & Verification Tools (Already Installed)

| Tool | Version | Purpose | Notes |
|------|---------|---------|-------|
| `@playwright/test` + `@axe-core/playwright` + `@ngi/platform-ssot` | `^1.62.1` / `^4.13.0` | Multi-resolution (`900x600`..`1920x1080`) + 4-theme (`arcane`, `slate`, `glacier`, `high-contrast`) WCAG 2.2 AA & CV Flywheel convergence | In `no-tauri` headless Chromium/Firefox (`main.tsx:28`), `webglAddon.ts` must gracefully fall back to xterm's DOM renderer when WebGL2 context creation fails under software compositing. |
| `vitest` + `jsdom` | `^3.2.6` / `^30.0.1` | Unit & component contract tests for `Console`, `TerminalTab`, `BlockStreamView`, SGR span parser, and `terminalBridge` | Keep `vi.mock` stubs aligned with `@xterm/addon-{webgl,search,web-links,unicode11,serialize}`. |
| `knip` | `5.50.0` | Unused dependency / export gate (`pnpm --dir crates/vox-gui/ui knip`) | Every `@xterm/addon-*` added to `package.json` **must** be imported by production code in `crates/vox-gui/ui/src/` in the same phase or `knip:files` will fail CI. |
| `vox-arch-check` & `vox ci crate-edges` | Workspace L0/L5 | Enforces `layers.toml`, `where-things-live.md`, `CargoDepHygieneDetector` (`vox/cargo/dep-hygiene`), and `contracts/ci/crate-edges.allow.v1.json` | Adding `alacritty_terminal = { workspace = true }` and `tempfile = { workspace = true }` to `vox-terminal-core` adds **zero** first-party `vox-*` edges (no `crate-edges.allow.v1.json` exception needed), provided both crates are actively used in `vox-terminal-core/src/`. |

---

## Installation & Manifest Diffs

### 1. Frontend (`crates/vox-gui/ui/package.json`)

```bash
pnpm --dir crates/vox-gui/ui add @xterm/addon-webgl@^0.19.0 @xterm/addon-search@^0.16.0 @xterm/addon-web-links@^0.12.0 @xterm/addon-unicode11@^0.9.0 @xterm/addon-serialize@^0.14.0
```

Required stylesheet import (currently missing):
```ts
// In crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx (or index.css layer(base))
import '@xterm/xterm/css/xterm.css';
```

### 2. Rust Engine (`crates/vox-terminal-core/Cargo.toml`)

Zero `Cargo.lock` network fetches or new workspace crates required — promote existing `[workspace.dependencies]` entries:

```toml
[dependencies]
alacritty_terminal = { workspace = true }
tempfile           = { workspace = true }
```

*(Note: Under `.agents/rules/00-core.md` and `vox/cargo/dep-hygiene`, only add these lines in the exact task that wires `alacritty_terminal` and `tempfile` into `crates/vox-terminal-core/src/`, and confirm `Cargo.lock` only records local workspace member edge wiring with zero external crate downloads.)*

---

## Architecture-Critical Stack Patterns

### Pattern 1: Zero-Echo Shell Startup Integration Across All 5 Shells

**Problem in current codebase:** `spawn_pty` (`crates/vox-terminal-core/src/pty.rs:368-371`) writes multi-line hook scripts directly to PTY stdin (`writer.write_all`), echoing raw script source across the user's terminal and polluting shell history.

**Verified 2026 Solution (`CommandBuilder` + `tempfile::TempDir` per `PtyHandle`):**
Allocate a session-scoped `TempDir` stored as `_integration_dir: Option<tempfile::TempDir>` inside `PtyHandle`. Set common env vars via `CommandBuilder::env`:
- `TERM=xterm-256color`, `COLORTERM=truecolor`, `TERM_PROGRAM=VoxAxis`, `VOX_SHELL_INTEGRATION=1`

| Shell | Launch Arguments & Env (`CommandBuilder`) | User Dotfile Preservation & Zero-Echo Hook Mechanism |
|-------|-------------------------------------------|------------------------------------------------------|
| **`zsh`** | No extra CLI flags; set `ZDOTDIR=<temp_dir>` and `VOX_USER_ZDOTDIR=<orig ZDOTDIR or $HOME>` | Write `<temp_dir>/{.zshenv,.zprofile,.zshrc,.zlogin}`. `.zshenv` sources `$VOX_USER_ZDOTDIR/.zshenv` (capturing any custom `ZDOTDIR` mutation into `VOX_USER_ZDOTDIR`) and re-pins `ZDOTDIR=<temp_dir>`. `.zshrc` **immediately restores `ZDOTDIR` (or unsets it if originally unset)** so Oh My Zsh, Starship, `.zcompdump`, and `.zsh_history` resolve to the user's real `$HOME`/`ZDOTDIR`, sources `$VOX_USER_ZDOTDIR/.zshrc`, then registers `__vox_preexec`, `__vox_precmd`, and `OSC 633;P;Cwd=%d` via `add-zsh-hook`. |
| **`bash`** | `bash --rcfile <temp_dir>/init.bash` *(do **not** pass `-l`, which forces Bash to ignore `--rcfile`)* | `init.bash` sources `/etc/profile` (if readable) and `~/.bashrc` first, then appends `__vox_prompt` safely to `PROMPT_COMMAND` (handling both Bash 5.1+ arrays and macOS `/bin/bash` 3.2 strings) and chains any existing `trap -p DEBUG`. Sets `OSC 633;P;Cwd=$PWD`. |
| **`fish`** | `fish -i -C "source '<temp_dir>/init.fish'"` | `-C` executes **after** `~/.config/fish/config.fish` (unlike `XDG_DATA_DIRS` `vendor_conf.d`, which runs *before* `config.fish` and gets clobbered if the user defines `fish_prompt` or loads Tide/Starship) with zero echo and zero history entry. Wraps `fish_prompt` via `functions --copy` and binds `--on-event fish_preexec` / `fish_postexec` + `$PWD` reporting. |
| **`pwsh` / `powershell`** | `pwsh -NoLogo -NoExit -Command ". '<temp_dir>/init.ps1'"` | `-NoLogo` suppresses startup copyright noise; omitting `-NoProfile` lets the user's `$PROFILE` load first, then dot-sources `init.ps1` silently into global scope without echoing script lines or recording in `PSReadLine` history. Wraps `function global:prompt` and `Set-PSReadLineKeyHandler -Key Enter`. |
| **`nu` (Nushell)** | `nu -e "source '<temp_dir>/init.nu'"` *(use `-e` / `--execute`, **never** `-c` which exits immediately)* | Nushell loads the user's `env.nu` and `config.nu` first, evaluates `-e` silently, and enters the interactive `reedline` REPL. **Critical deduplication rule:** `init.nu` must set `$env.config.shell_integration.osc633 = false` before appending Vox's `pre_prompt` / `pre_execution` hooks so Nushell's built-in `osc633` emitter does not fire duplicate `633;A/C/D` markers and create empty ghost blocks. |

### Pattern 2: Stateful PTY Block Capture + Alternate-Screen (`?1049h`/`?1049l`) Mode Switching

- **Single Stateful PTY Execution Path:** Route user and agent shell submissions (`InputIntent::Shell` / `term.submit` / MCP `term_run`) through the live session PTY (with OSC 633 `E`/`C`/`D` boundary tracking + `OSC 633;P;Cwd=...` working-directory updates) rather than spawning isolated `std::process::Command` subshells via `run_shell_capture`. Retain `run_shell_capture` strictly as a non-PTY headless fallback when a `Session` is instantiated without a `PtyHandle` in unit tests.
- **Authoritative `ALT_SCREEN` Detection in Rust:** Feed every PTY output chunk in `Session` through `alacritty_terminal::vte::ansi::Processor` + `alacritty_terminal::Term` (matching `crates/vox-term/src/vt.rs`). Query `term.mode().contains(TermMode::ALT_SCREEN)` (plus `BRACKETED_PASTE` and `APP_CURSOR`):
  - When `ALT_SCREEN` transitions `false -> true` (`vim`, `nvim`, `htop`, `less`, `git diff`, `tmux`), emit `SessionEvent::AltScreenChanged { active: true }`: the Axis GUI smoothly expands the WebGL `TerminalTab` canvas to full height and suspends Scriptorium block-card accumulation so TUI redraw frames do not flood `Block.output`.
  - When `ALT_SCREEN` transitions `true -> false` (`?1049l`), emit `SessionEvent::AltScreenChanged { active: false }`: the GUI returns to the Roman Scriptorium Block Stream view and closes the interactive command block cleanly on the ensuing `633;D;<exit>` marker.
- **Carriage-Return (`\r`) & SGR Normalization for Blocks:** Progress bars (`cargo build`, `pnpm`, `curl`) emit `\r\x1b[K` overwrites. Project block display lines and `RedactedBlockContext` through VT line-overwrite normalization (`\r` without `\n` rewrites the current line instead of concatenating 500 spinner frames into an LLM context bomb).

### Pattern 3: Styled Block Output Without XSS (`sgrToSpans.ts` vs. HTML Injection)

- In `BlockStreamView.tsx` and `ChatSurface.tsx` (`TerminalDockPanel`), render ANSI SGR sequences (`\x1b[...m`: 16 semantic ANSI colors mapped to Vox's 4-theme Style Dictionary tokens, 256-color palette, 24-bit RGB, bold/dim/italic/underline, and `OSC 8` hyperlinks) via a **pure TypeScript React span tokenizer (`sgrToSpans.ts`, ~90 LoC)** returning typed `SgrSpan[]` rendered as React `<span>` elements.
- Map ANSI 16 colors to theme-aware CSS variables so sunlit `arcane` (`#FAF6EE` / `#EDE3D1` parchment slabs with `#1C140C` espresso ink) and dark `slate` (`#0c0e10` / `#15191c` with `#e2e8f0` ink) both pass WCAG 2.2 AA (`>= 4.5:1` contrast) in the Playwright CV Flywheel.

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `alacritty_terminal 0.26.0` + `vte 0.15.0` (already in `Cargo.lock`) | `vt100` (`0.16.2`) | Only in greenfield tiny CLI tools that do not already compile `alacritty_terminal`. In Vox, `vox-term` already compiles `alacritty_terminal 0.26.0`; adding `vt100` duplicates VT state machines and adds unnecessary `Cargo.lock` surface. |
| Pure TypeScript SGR -> React `<span>` tokenizer + `@xterm/addon-serialize` (`0.14.0`) | `anser` (`^2.3.2`) | Acceptable lightweight fallback if team prefers a third-party npm SGR JSON parser over ~90 LoC of first-party TypeScript; still zero native deps and avoids `dangerouslySetInnerHTML`. |
| `fish -i -C "source '<temp>/init.fish'"` | `XDG_DATA_DIRS` `vendor_conf.d` injection | Use `XDG_DATA_DIRS` only if launching `fish` without CLI argument control. When `CommandBuilder` owns args, `-C` is strictly superior because it runs *after* user `config.fish` prompt themes (`fish_prompt`). |
| `nu -e "source '<temp>/init.nu'"` | `--env-config` / `--config` replacement | Only use `--config <temp>/config.nu` in hermetic tests (`--no-config-file`), because overriding `--config` in user sessions discards the user's custom Nushell completions, aliases, and keybindings unless manually chained. |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| **Any AGPL Warp crate or source (`warp-terminal`, Warp `app/ai` or block code)** | Violates Decision `D20`, `ADR-026` (third-party provenance), and Vox's Apache-2.0 license. | Clean-room `vox-terminal-core` (`SessionHost` + `LocalHost` + `DaemonHost`) + Tauri/React Scriptorium UI. |
| **Native GPU / C++ font stacks (`wgpu`, `winit`, `glutin`, `gpui`, `warpui`, `freetype-sys`, `harfbuzz_rs`, `font-kit`)** | Violates `AGENTS.md` Build-Toolchain Invariant (drags `cmake`, `pkg-config`, or `libclang` across platforms) and breaks Axis's 4-theme Style Dictionary + Playwright CV Flywheel (`terminal-session-host-design-2026.md §8`). | `@xterm/addon-webgl@^0.19.0` inside Tauri's WebView2/WKWebView/WebKitGTK + `ratatui`/`alacritty_terminal` for CLI `vox-term`. |
| **`@xterm/addon-canvas` or unscoped `xterm` / `xterm-addon-*`** | `@xterm/addon-canvas` was **discontinued and removed** in `@xterm/xterm@6.0.0`; unscoped `xterm` (`5.x`) packages are deprecated and type-incompatible with `@xterm/xterm@6.0.0`. | `@xterm/addon-webgl@^0.19.0` with automatic `onContextLoss` fallback to `@xterm/xterm@6.0.0`'s built-in DOM renderer. |
| **`ansi-to-html` (Rust `0.2.4` or npm `ansi-to-html` / `ansi_up` via `dangerouslySetInnerHTML`)** | Requires raw HTML string injection (`dangerouslySetInnerHTML`) on arbitrary terminal stdout/stderr, creating XSS/DOM-injection hazards and hardcoding dark-only hex colors that fail contrast on sunlit Carrara/Parchment (`arcane`/`travertine`) themes. | Typed React `<span>` rendering with theme-token CSS variables (or `@xterm/addon-serialize` for explicit export). |
| **Raw PTY stdin hook writing (`writer.write_all(snippet)` in `pty.rs:370`)** | Echoes 20+ lines of hook code onto the terminal viewport at startup, pollutes `.zsh_history`/`PSReadLine` history, and races fast user input. | `CommandBuilder` env/flag bootstrap (`ZDOTDIR`, `--rcfile`, `fish -C`, `pwsh -NoLogo -NoExit -Command`, `nu -e`) backed by `tempfile::TempDir`. |
| **Direct `std::env::var` for secrets or vendor LLM SDKs in terminal AI features** | Violates `AGENTS.md` Clavis SSOT (`vox_secrets::resolve_secret`) and Model-Agnostic LLM Boundary (`vox_actor_runtime::llm`). | Route all terminal AI / command explanation / error repair calls through `vox-orchestrator` -> `vox_actor_runtime::llm` and redact via `vox-redact`. |

---

## Version Compatibility Matrix

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `@xterm/xterm@^6.0.0` | `@xterm/addon-fit@^0.11.0` | Already installed; requires `@xterm/xterm/css/xterm.css` for v6 scrollable viewport layout. |
| `@xterm/xterm@^6.0.0` | `@xterm/addon-webgl@^0.19.0` | Requires WebGL2; must wrap `activate()` and `onContextLoss()` so `jsdom` (Vitest) and `no-tauri` software-rasterized Playwright runs fall back cleanly to DOM rendering. |
| `@xterm/xterm@^6.0.0` | `@xterm/addon-search@^0.16.0` | Supports custom `ISearchOptions.decorations` (`matchBackground`, `activeMatchBackground`) — requires `allowProposedApi: true` on `Terminal`. |
| `@xterm/xterm@^6.0.0` | `@xterm/addon-web-links@^0.12.0` | Pass custom `handler(event, uri)` so links open via Tauri shell/Axis Browser rather than `window.open` popup blocking. |
| `@xterm/xterm@^6.0.0` | `@xterm/addon-unicode11@^0.9.0` | Requires `allowProposedApi: true` on `new Terminal({ allowProposedApi: true })` before setting `term.unicode.activeVersion = '11'`. |
| `@xterm/xterm@^6.0.0` | `@xterm/addon-serialize@^0.14.0` | Exposes `serialize()` and `serializeAsHTML()`; compatible with both DOM and WebGL2 renderers. |
| `portable-pty@0.9.0` | `alacritty_terminal@0.26.0` + `vte@0.15.0` | Both already resolved in `Cargo.lock` under Rust `1.96`/`1.98.1` (`edition = "2024"`). Pure Rust, zero C++/CMake build scripts. |
| `vox-terminal-core` (`L3`) | `vox-term` (`L4`), `vox-orchestrator-d` (`L5`), `vox-gui` (`L5`) | Matches `docs/src/architecture/layers.toml` downward dependency flow (`L5/L4 -> L3`) with zero layer inversions. |

---

## Sources

- Workspace manifests & lockfiles verified directly: `Cargo.toml`, `Cargo.lock`, `crates/vox-terminal-core/Cargo.toml`, `crates/vox-term/Cargo.toml`, `crates/vox-gui/Cargo.toml`, `crates/vox-gui/ui/package.json`, `crates/vox-gui/ui/pnpm-lock.yaml`, `docs/src/architecture/layers.toml`, `contracts/ci/crate-edges.allow.v1.json`.
- Architecture & baseline source verified directly: `docs/src/architecture/terminal-session-host-design-2026.md`, `crates/vox-terminal-core/src/{pty.rs,osc633.rs,session.rs,host.rs,daemon_host.rs}`, `crates/vox-term/src/vt.rs`, `crates/vox-gui/src/commands/pty.rs`, `crates/vox-gui/ui/src/components/surfaces/Console/{Console.tsx,TerminalTab.tsx,BlockStreamView.tsx,InputEditor.tsx,webglAddon.ts}`, `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx`.
- `@xterm/xterm` v6.0.0 release notes & official scoped addon compatibility (`@xterm/addon-webgl@0.19.0`, `@xterm/addon-search@0.16.0`, `@xterm/addon-web-links@0.12.0`, `@xterm/addon-unicode11@0.9.0`, `@xterm/addon-serialize@0.14.0`, `@xterm/addon-fit@0.11.0`, removal of `@xterm/addon-canvas`, and v6 viewport rewrite) — verified via npm / GitHub `xtermjs/xterm.js` (HIGH confidence).
- Rust crates.io 2026 registry verification (`portable-pty 0.9.0`, `alacritty_terminal 0.26.0`, `vte 0.15.0`, `vt100 0.16.2`, `ansi-to-html 0.2.4`) & `alacritty_terminal::term::TermMode::ALT_SCREEN` pure-Rust dependency audit — verified via docs.rs / crates.io + workspace `Cargo.lock` (HIGH confidence).
- Shell startup integration mechanisms (`zsh` `ZDOTDIR` swap/restore, `bash --rcfile`, `fish -i -C`, `pwsh -NoLogo -NoExit -Command`, and Nushell `-e` / `--execute` vs `-c` plus `$env.config.shell_integration.osc633` deduplication) — verified via VS Code Terminal Shell Integration docs, Zsh manual, and Nushell documentation (HIGH confidence).

---
*Stack research for: Milestone v7.5 — Roman Scriptorium Warp-Class Agentic Terminal, Stateful PTY Engine, Bidirectional Chat Harness & CV Flywheel Convergence*
*Researched: 2026-11-14*
