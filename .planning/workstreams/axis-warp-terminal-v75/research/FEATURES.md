# Feature Research: Roman Scriptorium Warp-Class Agentic Terminal & Bidirectional Harness (v7.5)

**Domain:** Agentic Terminal Emulator, Stateful PTY Engine, Bidirectional Chat-Terminal Harness & MENS CV Flywheel
**Researched:** 2026-11-14
**Confidence:** HIGH (grounded in line-by-line audit of `crates/vox-terminal-core`, `crates/vox-gui/ui/src/components/surfaces/Console`, `crates/vox-term`, `crates/vox-orchestrator-d/src/term_dispatch.rs`, plus 2025–2026 Warp 2.0 ADE & Claude Code / Cursor / Windsurf surveys)

---

## Executive Gap Summary: Current Baseline vs. v7.5 Target

Vox already possesses a strong architectural spine—`vox-terminal-core` (`Block`, `Osc633Parser`, `Session`, `LocalHost`, `DaemonHost`), `vox-orchestrator-d` (`term.*` loopback stream/RPC dispatch), `vox-redact`, `vox-journal` corpus primitives, and the React `Console` + `Loquela` `@term` bridge. However, **critical runtime seams remain stubbed or split**, creating a sharp gap between today's prototype and **95% Warp.dev 2.0 parity + 2026 SOTA agentic harness + cohesive Roman Scriptorium design**:

| Subsystem | Current Vox Baseline (Verified in Repo) | Gap to v7.5 Target |
|---|---|---|
| **PTY Execution (`pty.rs`, `session.rs`)** | `Session::submit` (`session.rs:79`) executes `!cmd` via synchronous `run_shell_capture`, spawning a **fresh ephemeral `sh -c` / `pwsh -Command` subprocess per line**. `cd`, `export`, virtualenvs, and interactive stdin do not persist. `PtyHandle::resize` (`pty.rs:319`) is a no-op (`let _ = (cols, rows)`). | Route command submissions into the **live persistent `PtyHandle`** with non-blocking streaming chunks, real `MasterPty::resize`, `cwd`/`env` state tracking (`OSC 633;P;Cwd` / `OSC 7`), and interactive stdin forwarding. |
| **Console Layout & Renderer (`Console.tsx`, `TerminalTab.tsx`, `webglAddon.ts`)** | Split vertically between a `max-h-[340px]` dark card list (`BlockStreamView`) and a separate raw `xterm.js` box (`TerminalTab`) plus a bottom `<input>`. `WebglTerminalAddon` (`webglAddon.ts:19`) is a custom detached-canvas stub, not `@xterm/addon-webgl`. | Unified Warp-class scrollable **Block Stream** as primary viewport with live streaming active block, real `@xterm/addon-webgl` acceleration, horizontal/vertical **Split Panes**, and **Full-Screen TUI Auto-Switching** (`?1049h`/`?1049l` for `vim`/`lazygit`/`htop`). |
| **Universal Input & Completions (`InputEditor.tsx`, `intellisenseEngine.ts`)** | Single-line `<input>`, requires explicit `!` or `?` prefixes (bare words try `vox_interp::eval_line` first), no command history (`Up`/`Down`/`Ctrl+R`), and `discoverySuggest` only fires on `vox`, `/`, or `!nu` prefixes. | Multi-line (`Shift+Enter`) **Universal Input** with automatic Shell vs. Natural-Language vs. `.vox` intent detection, syntax highlighting, persistent redacted history search, `$PATH`/flag/git/cwd-file completions, and MENS Active AI next-command ghost predictions. |
| **Agent Turns & Slash Commands (`commands.rs`, `agent.rs`)** | `/budget`, `/skills`, `/memory`, `/context`, `/sh`, `/ai` return hardcoded placeholder strings (`commands.rs:130-183`). `?prompt` only runs heuristic `route_agent_prompt` and prints `[routed model=...]` without calling an LLM or agent loop. | Real streaming agent turns via `vox_actor_runtime::llm` + `vox-orchestrator` `EventBus`, live token/cost `/budget`, interactive `/skills` & `/memory` queries, and inline error-recovery ("Diagnose & Fix") on failed blocks. |
| **Agentic Harness, Policy & Approvals (`host.rs`, `term_dispatch.rs`, `Loquela.tsx`)** | `TermDispatch::run_agent_command` (`term_dispatch.rs:36`) is `dead_code` in non-test builds. `Console.tsx` security/control pills are local `useState` never sent to `term.security`/`term.control`. `allows_agent_command` uses naive substring matching. `Loquela.tsx:1045` injects unredacted last-2 blocks via regex. | Live foreground/background `term_run` MCP/orchestrator tool (`Ctrl+B` backgrounding, interactive prompt/hang detection), AST policy check (`contracts/terminal/exec-policy.v1.yaml`), **Co-Pilot Command Approval Cards**, and server-redacted `@term:<sid>#<id>:L<a-b>` citations. |
| **TUI Continuity & CV Flywheel (`vox-term/src/app.rs`, `corpus/`)** | `vox-term` parses `--attach` into unused `_cli_args` (`app.rs:104`) and spins an isolated `Session::new("main")`. `JournalSink` & `curate` are unwired to `Session` and have no UI feedback triggers. | Wire `vox-term` to `resolve_session_host` for true CLI↔GUI live handoff; connect `Session` → `JournalSink` → `curate` → `vox-redact` → `CorpusWriter` with per-block Accept/Reject/Correct training signals. |
| **Visual Theme (`BlockStreamView.tsx`, `SendToAgent.tsx`)** | `RomanSurfaceHeader`, session bar, and `DiscoveryRail` use Roman Scriptorium tokens, while `BlockStreamView` uses `bg-zinc-950/70` + neon cyan and `SendToAgent` uses unstyled inline HTML. | 100% **Roman Scriptorium ("Limes")** surface harmony: travertine/parchment & basalt block tablets, cast-brass borders/gauges, verdigris structured badges, and crimson wax-seal approval/failure medallions. |

---

## Feature Landscape

### Table Stakes (Users Expect These — 95% Warp 2.0 + 2026 Agentic Harness Core)

Features users and AI coding operators assume exist in a Warp-class agentic terminal. Missing any of these makes the surface feel like a toy wrapper rather than a daily-driver shell and coding harness.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **1. Stateful Persistent PTY Execution (`cd` / `env` / Stdin Continuity)** | Every terminal command must inherit working directory, exported env vars, shell aliases, and active virtualenvs from prior commands in the session, and stream stdout/stderr live rather than blocking until exit. | HIGH | Replace `run_shell_capture` in `Session::submit` (`session.rs:79`) with live `PtyHandle` command write + `OSC 633` boundary tracking (`A/B/C/D/E` + `P;Cwd` / `OSC 7`). Retain `MasterPty` handle inside `PtyHandle` so `resize(cols, rows)` (`pty.rs:319`) resizes the real OS ConPTY/PTY. |
| **2. Full-Screen TUI & Alternate-Screen Auto-Switching** | Running `vim`, `nvim`, `lazygit`, `htop`, `less`, or nested `vox term` must seamlessly switch from Block Stream mode into full-bleed interactive VT canvas and restore Universal Input upon exit. | MEDIUM | Detect DECSET `?1049h` / `?47h` / `?1047h` (enter alt-screen) and `?1049l` (exit) in `vox-terminal-core` stream scanner; emit `SessionEvent::AltScreenChanged(bool)` so `Console.tsx` maximizes `@xterm/addon-webgl` (`TerminalTab`) and routes raw keystrokes directly. |
| **3. Universal Multi-Line Input Editor with Auto-Intent Detection** | Warp 2.0's signature UX: one bottom composer accepting shell commands, multi-line scripts (`Shift+Enter`), natural language prompts, `/slash` commands, and `.vox` expressions without forcing manual mode prefixes. | MEDIUM | Upgrade `InputEditor.tsx` from `<input>` to multi-line syntax-aware editor. Add lightweight deterministic classifier distinguishing shell syntax (`git`, `cargo`, pipes, paths, `$PATH` hit) vs. natural language prose vs. `.vox` (`let`, `fn`, `@`), while keeping explicit `!`, `!nu`, `?`, `/` overrides and a lockable mode pill (`Auto` / `Shell` / `Agent` / `Nu` / `Vox`). |
| **4. Rich Interactive Command Blocks (Unified Stream)** | Warp's foundational primitive: each command + live streaming ANSI output + metadata (`cwd`, git branch, exit code, duration, timestamp, origin badge) forms a navigable, collapsible, searchable unit. | MEDIUM | Unify `BlockStreamView` and `TerminalTab` so blocks render ANSI colors/styles cleanly, support collapse/expand, sticky command headers while scrolling long logs, copy command/output/markdown, rerun, bookmark/pin, and clickable `path/to/file.rs:42:5` links. Extend Rust `Block` (`block.rs`) with `cwd`, `git_branch`, `started_ms`, `ended_ms`. |
| **5. Command History Recall & Fuzzy Block/History Search** | Terminal muscle memory requires `Up`/`Down` arrow history cycling, `Ctrl+R` fuzzy command-history search, and `Cmd+F` / `Ctrl+F` search/filtering within a block or across session blocks. | MEDIUM | Persist redacted per-workspace command history via `vox-db` / `SessionHost`; integrate nucleo-style fuzzy scoring in `intellisenseEngine.ts` for `Ctrl+R` popover and add in-stream regex/substring filter bar on `BlockStreamView`. |
| **6. Context-Aware Shell, Path, Git & Flag Completions** | Pressing `Tab` or typing in a modern terminal must complete `$PATH` executables, `cwd`-relative files/directories, `git` branches/tags, `cargo`/`pnpm` targets, and CLI flags—not only `vox *` subcommands. | MEDIUM | Remove the `vox`/`/`/`!nu`-only guard in `InputEditor.tsx:72`. Add a fast daemon/Tauri completion provider returning `cwd` entries, cached `$PATH` binaries, git refs, and `CommandCatalog` / clap flag specs into `computeIntelliSenseItems`. |
| **7. Split Panes, Rich Session Tabs & Real WebGL Rendering** | Developers run a dev server or watcher alongside a build/test shell; tabs need meaningful titles (`cwd` + foreground process), close/rename actions, and GPU text throughput. | MEDIUM | Replace the no-op canvas in `webglAddon.ts` with real `@xterm/addon-webgl` (with DOM fallback on `webglcontextlost`). Support horizontal/vertical pane splits in `Console.tsx` and enrich `SessionInfo` (`host.rs:158`) with `title`, `cwd`, `shell`, `attached_clients`, and `running_cmd`. |
| **8. Subshell Detection & One-Click "Voxify"** | Entering `bash`, `zsh`, `fish`, `pwsh`, `nu`, `docker exec -it`, or `ssh` inside an existing session drops `OSC 633` hooks unless detected and re-injected ("Warpify" parity). | MEDIUM | Detect child shell prompts / subshell spawns without `OSC 633` markers and offer automatic or 1-click injection of `pty::shell_integration_snippet` (`pty.rs:30`) with a nested subshell badge on resulting blocks. |
| **9. Foreground & Background Agentic PTY Tool (`term_run`) + Prompt/Hang Detection** | 2026 coding harnesses (Claude Code, Cursor, Windsurf) require agents to run builds/tests in foreground AND spawn long-running dev servers in background (`run_in_background` / `Ctrl+B`), without freezing on `[y/N]`, `Password:`, or `sudo` stdin prompts. | HIGH | Wire `term_run` into `vox-orchestrator-d` + MCP tool surface (`term_dispatch.rs:37`). Support `background: bool` + human `Ctrl+B` detach-to-background, output tailing/kill by `block_id`, and PTY idle-line prompt heuristics (`[y/n]`, `password:`, `passphrase`, `overwrite?`) that immediately surface a `NeedsYou` / interactive stdin prompt card instead of hanging until timeout. |
| **10. L1–L5 AST Exec Policy & Co-Pilot Command Approval Cards** | Agent-driven shell execution requires transparent, deadlock-free human oversight: inline approval cards showing exact command, `cwd`, risk reason, and AST policy verdict before any PTY bytes are written. | HIGH | Sync `Console` security/control selectors to `term.security` / `term.control` RPCs. Upgrade `TermSecurityLevel::allows_agent_command` (`host.rs:76`) to evaluate `contracts/terminal/exec-policy.v1.yaml` + `classify_floor` (`term_dispatch.rs:51`). Render inline **Co-Pilot Approval Cards** in both Console and Chat (`Approve`, `Edit & Run`, `Allow Pattern in Session`, `Reject + Steer`), keeping instant user keystroke takeover (`Autopilot → Manual`). |
| **11. Redacted Precision `@term` Citations (`@term:<sid>#<block>:L<a-b>`)** | Referencing terminal errors in Chat (`@term:failed`, `@term:pinned`, `@term:s-1#4:L12-L30`) and clicking "Diagnose Failure" on a red block are core daily AI debugging loops. | LOW | Replace `Loquela.tsx:1035`'s client-side unredacted slice with `term.context` RPC (`term_dispatch.rs:188`) extended to support `last`, `failed`, `pinned`, specific `block_id`, and `line_range`—always scrubbed through `vox-redact` before entering LLM prompts. |
| **12. Live Slash Commands & Model-Agnostic Streaming Agent Turns** | Typing `?why did cargo test fail` or `/budget` must stream a real answer or live session telemetry, not static placeholder text. | MEDIUM | Replace stubs in `commands.rs` and `session.rs:166` by routing `InputIntent::Agent` through `vox_actor_runtime::llm` (`llm_stream`) + `vox-orchestrator` session context, and wiring `/budget`, `/model`, `/skills`, `/memory`, `/context`, `/clear`, and `/compact` to live orchestrator state. |

---

### Differentiators (Vox Competitive Advantage)

Where Vox decisively outperforms Warp 2.0, Cursor, Windsurf, and Claude Code by leveraging its unified Rust daemon, `.vox` language runtime, Roman Scriptorium aesthetic, and local MENS model flywheel.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **1. Shared `SessionHost` Continuity (`vox term` TUI ↔ Axis `Console` ↔ `Chat`)** | Start a build or interactive debugging session in headless `vox term` over SSH/CLI, open Vox Axis GUI and watch the exact same live blocks/PTY stream, or detach GUI and resume via `vox term --attach <id>`—zero state loss, one source of truth. | MEDIUM | `DaemonHost` (`daemon_host.rs`) and `TermDispatch` (`term_dispatch.rs`) already pass parity tests! Complete the last mile in `vox-term/src/app.rs` (consume `TermCliArgs` + `resolve_host`) and expose attached client count + 1-click attach/detach in Axis `Console`. Neither Warp (GUI-only) nor Cursor/Claude Code (siloed IDE vs. CLI sessions) offers dual TUI/GUI live block-session projection. |
| **2. VoxLang-First Executable Runbooks & Workflows (`.vox` + Runnable Markdown)** | Replaces Warp Drive's cloud-locked Notebooks/Workflows with git-versioned, type-checked `.vox` automation scripts (`scripts/*.vox`) and runnable Markdown runbooks (`docs/` / `.vox/runbooks/*.md`) executed in-band with capability sandboxes (`--caps`). | MEDIUM | Extend `vox_interp.rs` (`eval_line`) into a stateful session REPL + parameterized Runbook runner surfaced in the Console Command Palette (`Cmd+P`) and `DiscoveryRail`. Runbook code fences (`vox`, `nu`, `sh`) execute directly as Console blocks while remaining reviewable in Git PRs and 100% offline. |
| **3. Roman Scriptorium Visual Architecture ("Limes" Design System)** | Every AI terminal in 2026 looks like identical dark-zinc SaaS chrome. Vox's carved travertine/parchment & obsidian-basalt tablets, cast-brass telemetry gauges, verdigris structured badges, and crimson wax-seal safety medallions create unmistakable visual hierarchy and craft. | MEDIUM | Refactor `BlockStreamView.tsx`, `InputEditor.tsx`, `SendToAgent.tsx`, and Approval Cards onto semantic Scriptorium tokens (`RomanSurfaceHeader`, `RomanFlutedPillar`, brass rules, wax-seal badges for `L4/L5` floor gates and failed exit codes) with crisp WCAG AA contrast in both Parchment Light and Basalt Dark modes, mirrored in `vox-term/src/theme.rs` Nerd Font glyphs. |
| **4. First-Class Structured Shell (`!nu` Nushell + Typed `.vox` Inspector)** | Instead of regex-scraping text soup, operators and agents pipe structured tables (`!nu ls \| where size > 10kb`) and inspect them as interactive sortable/filterable grids in Console blocks while passing compact typed JSON schemas to agents. | MEDIUM | Upgrade `BlockStreamView.tsx`'s `isStructuredBlock` (`JSON.stringify`) into a rich **Scriptorium Ledger Table / JSON Tree View** with column sort, cell copy, and schema summary for `@term` context, backed by `diagnose_nu_posix_isms` (`pty.rs:208`) inline hints. |
| **5. Spaced-Repetition `DiscoveryRail` & Unified Capability Index** | Proactively teaches the operator `vox` CLI commands, `@mcp` tools, `/skills`, and Nushell idioms contextualized to what they are typing, backed by dwell-time spaced repetition (`discoveryRecord`) so hints fade once mastered. | LOW | Already scaffolded in `DiscoveryRail.tsx`! Expand live contextual triggers to surface relevant Runbooks, error-specific `vox audit` / `vox doctor` remedies when a block fails, and keyboard-shortcut mastery nudges. |
| **6. Continuous Verification (CV) & Local MENS Training Flywheel** | Every terminal session becomes a privacy-preserving, `vox-redact`-scrubbed training & verification stream (`vox-journal` → `curate` → `CorpusWriter`) that improves Vox's own local MENS models (`mens-8b`+) for zero-cost offline command predictions and routing. | MEDIUM | Wire `JournalSink` into `LocalHost` session lifecycle; add unobtrusive **Accept (✓) / Reject (✗) / Edit-Correction** actions on Agent & AI-predicted blocks (`TranscriptKind::Accepted` / `Rejected` / `Corrected` in `transcript.rs:29-40`); route Active AI next-command suggestions through `vox_actor_runtime::llm` so local MENS revisions compete on latency/cost/precision (`agent.rs:148`). |

---

### Anti-Features / Explicit Non-Goals (The 5% Warp SaaS Bloat & Scope Traps to Exclude)

Deliberately excluded capabilities to keep Vox **100% local-first, offline-capable, license-clean, and architecturally focused**.

| Anti-Feature | Why Requested (Surface Appeal) | Why Problematic for Vox | Vox Local-First Alternative |
|--------------|-------------------------------|-------------------------|-----------------------------|
| **1. Mandatory Cloud Account, Login Gate, or Cloud Telemetry Sync** | Warp used accounts to sync settings/AI quotas across machines. | Breaks air-gapped/offline workflows, alienates privacy-conscious engineers (Warp's #1 historical user backlash), and violates Vox's local-first ethos. | Zero-login startup (`LocalHost` / loopback `vox-orchestrator-d`); optional explicit telemetry only per ADR-023 (`vox telemetry`); secrets managed locally via Clavis (`vox-secrets`). |
| **2. Cloud Team SaaS "Warp Drive" (Hosted Env Vars, Cloud Rules, Seat Billing)** | Central web dashboard for sharing team snippets and environment secrets. | Introduces cloud SaaS backend bloat, third-party secret custody risk, and split-brain drift away from the Git repository. | Git-tracked `.vox/runbooks/`, `AGENTS.md`, `.vox/skills/` (`agentskills.io` SSOT), and Clavis (`vox_secrets::resolve_secret`) with `.voxignore` / `vox-redact` guards. |
| **3. Public Cloud Block Permalinks & Multiplayer Cloud Terminal URLs** | One-click `warp.dev/s/...` web links to share terminal logs on Slack. | High risk of leaking internal hostnames, stack traces, or unredacted secrets to an external cloud bucket. | One-click **"Copy Redacted Block (Markdown / JSON)"** scrubbed through `vox-redact`, plus local `@term:<sid>#<id>` deep-links across Axis surfaces. |
| **4. Full Multi-Tab Code Editor & File Tree Inside Console ("Warp ADE" IDE Bloat)** | Warp 2.0 added a built-in code editor and file tree because standalone Warp has no surrounding desktop app. | Vox Console already lives inside **Vox Axis** (32 specialized surfaces) alongside dedicated IDE integrations. Embedding a full code editor inside Console creates cramped UI Inception and duplicates review surfaces. | Keep Console laser-focused on **blocks, PTYs, runbooks, and agent execution**; make `file:line:col` paths and git diffs clickable to jump to Axis review panels or `$EDITOR`. |
| **5. Forking AGPLv3 Warp Crates or Embedding `warpui` GPU Framework** | Shortcut to reuse Warp's block/AI Rust code or custom GPU widget toolkit. | Warp's terminal/blocks/AI crates are **AGPLv3** (network-copyleft incompatible with Vox, ADR-026); `warpui` loses HTML/CSS design tokens, IME, accessibility, and Playwright verification (`terminal-session-host-design-2026.md` §8). | Clean-room `vox-terminal-core` + `@xterm/addon-webgl` in Axis GUI + `ratatui`/`alacritty_terminal` (Apache-2.0/MIT) in `vox-term`. |
| **6. Direct Vendor LLM SDK Calls in Terminal UI or Core** | Quick hardcoded Anthropic/OpenAI API calls for "Active AI" command generation. | Violates `AGENTS.md §Model-Agnostic LLM Boundary` (enforced as `Error` by `vox-code-audit` `llm_provider_call` detector) and prevents local MENS offline fallback. | Route 100% of terminal LLM completions, translations, and agent turns through `vox_actor_runtime::llm` + `vox-orchestrator::models` registry. |
| **7. Per-PTY Detached Orphan Processes Surviving Daemon Kill (`tmux` Clone)** | Keeping shells alive even if `vox-orchestrator-d` crashes or restarts. | Requires a per-PTY shim process tree, complex FD passing on Windows ConPTY, and orphan-reaping hazards (`terminal-session-host-design-2026.md` §7). | `vox-orchestrator-d` already outlives both Axis GUI and `vox term` TUI restarts; persist structured block transcripts in `vox-journal` so session history restores cleanly after a machine reboot. |

---

## Feature Dependencies

```text
[1. Stateful Persistent PTY + Real Resize + OSC-633 Cwd/AltScreen]
    ├──requires──> [2. Full-Screen TUI Auto-Switching (?1049h/?1049l)]
    ├──requires──> [4. Rich Interactive Command Blocks (cwd, timing, ANSI)]
    ├──requires──> [8. Subshell Detection & "Voxify" Hook Injection]
    └──requires──> [9. Streaming Foreground/Background `term_run` + Prompt/Hang Detection]
                       └──requires──> [10. L1–L5 AST Exec Policy & Co-Pilot Approval Cards]

[4. Rich Interactive Command Blocks]
    ├──enhances──> [11. Redacted Precision `@term` Citations (`term.context`)]
    ├──enhances──> [Diff 3. Roman Scriptorium Visual Architecture]
    ├──enhances──> [Diff 4. Structured Shell Ledger Table (`!nu` + `.vox`)]
    └──requires──> [Diff 6. CV & MENS Training Flywheel (`JournalSink` + Accept/Reject UI)]

[12. Model-Agnostic Agent Turns (`vox_actor_runtime::llm`)]
    ├──enhances──> [3. Universal Multi-Line Input Editor (NL → Command / Agent Stream)]
    └──enhances──> [Diff 2. Executable `.vox` & Markdown Runbooks]

[Diff 1. Shared `SessionHost` Daemon Continuity]
    └──unifies───> [Axis `Console` GUI] + [`vox term` TUI] + [`Chat` / `Loquela` Harness]
```

### Dependency Notes

- **Stateful Persistent PTY (`pty.rs` / `session.rs`) is the root blocker:** Until `Session::submit` stops spawning one-shot `Command::new(shell) -c` subprocesses and instead drives the persistent `PtyHandle` with `OSC 633` markers + real `MasterPty::resize`, directory changes (`cd`), subshell Voxification, TUI alt-screen switching, interactive stdin prompts, and background `term_run` tasks cannot function authentically.
- **L1–L5 AST Policy & Co-Pilot Approval Cards gate `term_run`:** Per `term_dispatch.rs:56`, agent-originated commands fail closed without approval. Wiring `contracts/terminal/exec-policy.v1.yaml` + `classify_floor` + inline UI Approval Cards must land alongside `term_run` so autonomous/co-pilot workflows don't deadlock.
- **Roman Scriptorium unification depends on Block Stream consolidation:** Consolidating the awkward split `BlockStreamView` + raw `TerminalTab` layout into a single cohesive block canvas (with TUI alt-screen takeover) avoids styling throwaway dual-pane chrome twice.
- **CV Flywheel enhances Active AI Completions:** Wiring `Session` → `JournalSink` → `curate` → `vox-redact` → `CorpusWriter` first establishes the clean dataset and feedback events (`Accepted` / `Corrected`) that MENS routing consumes.

---

## MVP Definition (Milestone v7.5 Phasing)

### Launch With (v7.5 Core — Must-Have for 95% Warp + Harness Parity)

- [ ] **Stateful Persistent PTY & Real Resize (`vox-terminal-core`)** — Live `PtyHandle` command dispatch, `MasterPty::resize`, `OSC 633` `P;Cwd` / `OSC 7` tracking, and `Block` metadata (`cwd`, `git_branch`, `started_ms`, `ended_ms`).
- [ ] **Unified Block Stream + Full-Screen TUI Auto-Switching (`Console`)** — Single scrollable Warp-class block stream with ANSI rendering, collapsible outputs, clickable `file:line` citations, real `@xterm/addon-webgl`, and automatic alt-screen (`?1049h`/`?1049l`) takeover for full-screen TUIs (`vim`, `lazygit`, `htop`).
- [ ] **Roman Scriptorium Console Overhaul** — Full conversion of `BlockStreamView`, `InputEditor`, `SendToAgent`, session tabs, and Approval Cards to Limes Scriptorium tokens (travertine/basalt tablets, brass gauges/rules, crimson wax-seal status/floor badges).
- [ ] **Universal Multi-Line Input + Completions + History** — Multi-line composer (`Shift+Enter`), auto-detection of Shell vs. Agent vs. `.vox` vs. `!nu`, `Up`/`Down` & `Ctrl+R` fuzzy history, and `$PATH`/cwd-file/git/flag IntelliSense.
- [ ] **Bidirectional Agentic `term_run` + L1–L5 AST Policy + Co-Pilot Approval Cards** — Live foreground/background `term_run` tool (`Ctrl+B` backgrounding, interactive stdin/prompt hang detection), `exec-policy.v1.yaml` AST check, synced `term.security`/`term.control` RPCs, and inline Co-Pilot Approval Cards in Console & Chat.
- [ ] **Server-Redacted Precision `@term` Citations & Live Slash/Agent Turns** — `term.context` RPC backing `@term:last`, `@term:failed`, `@term:pinned`, and `@term:<sid>#<id>:L<a-b>` via `vox-redact`; real `vox_actor_runtime::llm` streaming for `?prompt` / `/ai` and live `/budget`, `/model`, `/skills`, `/memory`, `/context`.
- [ ] **End-to-End `vox term` ↔ Axis `Console` Continuity & CV Flywheel Wiring** — Wire `vox-term/src/app.rs` to `resolve_session_host` (`--attach <id>`) and connect `Session` → `JournalSink` → `curate` → `vox-redact` → `CorpusWriter` with block Accept/Reject/Correct actions.

### Add After Core Validation (v7.5 Polish / Fast-Follow)

- [ ] **Horizontal & Vertical Split Panes + Named Session Tabs** — Multi-pane terminal layouts within Console tabs with live foreground-process & `cwd` titles.
- [ ] **Interactive Structured Ledger Table (`!nu` & `.vox`)** — Sortable/filterable table & JSON tree viewer inside structured blocks.
- [ ] **Executable Local Runbooks (`.vox` + Markdown)** — Command Palette (`Cmd+P`) discovery and 1-click parameterized execution of `.vox/runbooks/*.md` and `scripts/*.vox`.
- [ ] **Subshell Auto-Detection & One-Click "Voxify"** — Automatic `OSC 633` snippet injection into nested local `bash`/`zsh`/`fish`/`pwsh`/`nu` and `docker exec` / `ssh` subshells.

### Future Consideration (v8+)

- [ ] **Active AI Inline Next-Command Ghost Predictions (MENS Speculative)** — Zero-latency background next-command synthesis after every block exit once curated CV flywheel volume matures.
- [ ] **Authenticated Loopback-Tunnel / Mesh Remote Session Attach** — Per-session capability tokens for attaching to remote `vox-orchestrator-d` nodes (`terminal-session-host-design-2026.md` §7).
- [ ] **Launch Configurations (Saved Multi-Pane Workspace Layouts)** — Declarative `.vox` workspace terminal layouts auto-spawning server/watcher/shell panes per repo.

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Stateful Persistent PTY + `MasterPty::resize` + `OSC 633` Cwd | HIGH | HIGH | **P1** |
| Unified Block Stream + Real `@xterm/addon-webgl` + Alt-Screen TUI Auto-Switch | HIGH | MEDIUM | **P1** |
| Streaming Foreground/Background `term_run` + Interactive Prompt/Hang Detection | HIGH | HIGH | **P1** |
| L1–L5 AST Exec Policy (`exec-policy.v1.yaml`) + Co-Pilot Command Approval Cards | HIGH | MEDIUM | **P1** |
| Roman Scriptorium Visual Unification (Blocks, Input, Approvals, `SendToAgent`) | HIGH | MEDIUM | **P1** |
| Server-Redacted Precision `@term` Citations (`term.context` + line ranges) | HIGH | LOW | **P1** |
| Universal Multi-Line Input + Auto-Intent + History (`Up`/`Down`/`Ctrl+R`) | HIGH | MEDIUM | **P1** |
| `$PATH`, Cwd-File, Git Branch & CLI Flag Completions | HIGH | MEDIUM | **P1** |
| Live Slash Commands & `vox_actor_runtime::llm` Streaming Agent Turns | HIGH | MEDIUM | **P1** |
| `vox term --attach` Live Daemon Wiring + CV Flywheel (`JournalSink` + UI Signals) | HIGH | LOW | **P1** |
| Split Panes (Horizontal/Vertical) & Live Process/Cwd Tab Titles | MEDIUM | MEDIUM | **P2** |
| Structured Ledger Table Viewer (`!nu` Nushell & `.vox` Values) | MEDIUM | MEDIUM | **P2** |
| Local `.vox` & Runnable Markdown Runbooks (`Cmd+P` Palette) | MEDIUM | MEDIUM | **P2** |
| Subshell Detection & One-Click "Voxify" (`ssh` / `docker exec` / nested shells) | MEDIUM | MEDIUM | **P2** |
| MENS Speculative Next-Command Ghost Predictions | MEDIUM | HIGH | **P3** |

**Priority key:**
- **P1:** Must have for Milestone v7.5 core completion
- **P2:** High-leverage v7.5 polish waves once P1 core is green
- **P3:** Defer until CV flywheel corpus & latency budgets are validated

---

## Competitor Feature Analysis

| Capability | Warp 2.0 (ADE) | Claude Code (CLI) | Cursor / Windsurf (IDE) | Vox v7.5 Approach |
|------------|----------------|-------------------|-------------------------|-------------------|
| **Command Blocks & Metadata** | Native GPU blocks with cwd, git, exit, duration, filter, cloud permalink | Scrolling CLI transcript; tool blocks collapsible | Raw VS Code xterm panel; OSC-633 frequently breaks on p10k/oh-my-zsh | **Clean-room Rust `Block` engine** in `vox-terminal-core` + Roman Scriptorium block cards + `@xterm/addon-webgl` |
| **Universal Input** | Multi-line input auto-detecting Shell vs. Agent Mode + `@` context | Single prompt box (`!cmd` for bash, plain text for agent, `/slash`) | Separate Chat sidebar vs. raw terminal prompt | **Universal Multi-Line Input** auto-routing Shell, Agent (`?`), Nushell (`!nu`), Slash (`/`), and `.vox` expressions |
| **TUI & Subshell Handling** | Smooth alt-screen TUIs + 1-click "Warpify" for SSH/Docker subshells | Runs inside host terminal; subshells opaque to agent | Integrated xterm handles TUIs manually; agent tool hangs on interactive prompts | **Alt-screen (`?1049h`) auto-switching** + **1-click "Voxify"** injecting `pty::shell_integration_snippet` |
| **Agent PTY Tool & Backgrounding** | Parallel local/cloud coding agents driving blocks | Streaming Bash tool + `Ctrl+B` / `run_in_background` + `/bashes` | Streaming PTY tool + background terminals; notorious "Running..." hang on stdin prompts | **Daemon `term_run` (foreground + `Ctrl+B` background)** + **stdin prompt/stall detector** surfacing `NeedsYou` cards |
| **Security & Command Approval** | Allowlists + Agent profile permissions | Tiered permissions + interactive CLI prompt + `--allowedTools` | Approval cards + allowlists (`.cursor/cli.json`), regex-brittle on compound pipes | **L1–L5 Security Tiers + AST-checked `exec-policy.v1.yaml` + `classify_floor` + Co-Pilot Approval Cards** + instant keystroke takeover |
| **Terminal ↔ Chat Citations** | Attach error block to Agent Mode conversation | Native in-band transcript (no separate GUI chat) | `@terminal` selection chip (unredacted raw text) | **Server-redacted `@term:<sid>#<id>:L<a-b>`**, `@term:failed`, `@term:pinned` via `vox-redact` |
| **Runbooks / Workflows** | Cloud-synced **Warp Drive** Notebooks & Workflows (SaaS lock-in) | `.claude/commands/*.md` slash prompts | `.cursor/rules` & `.windsurf/workflows` markdown prompts | **100% local git-tracked `.vox` scripts (`vox run`) + runnable Markdown Runbooks** + `agentskills.io` `SKILL.md` |
| **CLI ↔ GUI Session Continuity** | GUI only (no headless TUI client) | CLI only (IDE extension opens side-by-side diffs) | GUI only (closing IDE kills integrated terminals) | **Shared `SessionHost` in `vox-orchestrator-d`**: live attach/detach across `vox term` TUI, Axis `Console`, and `Chat` |
| **Privacy, Offline & Model Flywheel** | Cloud account ecosystem; vendor cloud LLMs | Cloud API bound (Anthropic / Bedrock / Vertex) | Cloud proxy bound; proprietary completion models | **100% local-first/offline-capable**, model-agnostic `vox_actor_runtime::llm`, and **redacted `vox-journal` → MENS CV flywheel** |

---

## Sources

### Verified In-Repo Codebase & Architecture Sources
- `crates/vox-gui/ui/src/components/surfaces/Console/` — `Console.tsx`, `BlockStreamView.tsx`, `InputEditor.tsx`, `DiscoveryRail.tsx`, `SendToAgent.tsx`, `AgentStrip.tsx`, `AgentTab.tsx`, `TerminalTab.tsx`, `webglAddon.ts`
- `crates/vox-gui/ui/src/lib/` — `terminalBridge.ts`, `intellisenseEngine.ts`
- `crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx` & `Chat/ChatSurface.tsx` — `@term` citation expansion & Approvals dock panel
- `crates/vox-gui/ui/src/components/ui/RomanSurfaceHeader.tsx` & `crates/vox-term/src/theme.rs` — Roman Scriptorium ("Limes") visual components & Nerd Font palette
- `crates/vox-terminal-core/src/` — `block.rs`, `commands.rs`, `input.rs`, `agent.rs`, `osc633.rs`, `session.rs`, `pty.rs`, `host.rs`, `daemon_host.rs`, `vox_interp.rs`, `transcript.rs`, `corpus/{mod,curate,redact,writer}.rs`
- `crates/vox-orchestrator-d/src/term_dispatch.rs` — `term.*` wire handlers, `classify_floor` safety gate, and `term.context` redaction RPC
- `crates/vox-term/src/app.rs` — Ratatui TUI entrypoint & CLI arg parsing
- `contracts/terminal/exec-policy.v1.yaml` — Canonical terminal execution policy contract
- `docs/src/architecture/terminal-session-host-design-2026.md` & `docs/superpowers/plans/2026-06-20-vox-terminal-warp-parity.md` — Clean-room Warp evaluation (ADR-026) and `SessionHost` protocol design

### External Competitor & Ecosystem Research (2025–2026)
- **Warp 2.0 / Agentic Development Environment (ADE):** Universal Input (Auto / Terminal / Agent modes), Interactive Blocks, Subshell "Warpification", TUI alternate-screen auto-switching, Active AI completions, Secret Redaction, Split Panes, and Warp Drive Notebooks/Runbooks (`warp.dev`, April 2026 open-source split-license evaluation).
- **Claude Code (2025–2026):** Streaming PTY Bash execution, `run_in_background` / `Ctrl+B` asynchronous background task promotion (`/tasks`, `/bashes`), tiered tool permission gates, and slash-command workflows.
- **Cursor & Windsurf Cascade (2025–2026):** Integrated terminal tool execution, `@terminal` context chips, Command Approval Cards, and documented industry failure modes around custom prompt `OSC 633` hook collisions (`p10k`/`oh-my-zsh`) and silent `stdin` interactive-prompt deadlocks.

---
*Feature research for: Vox Milestone v7.5 (`axis-warp-terminal-v75`)*
*Researched: 2026-11-14*
