# Architecture Research

**Domain:** Roman Scriptorium Warp-Class Agentic Terminal, Stateful PTY Engine, Bidirectional Chat Harness & CV Flywheel Convergence (`v7.5` / `axis-warp-terminal-v75`)
**Researched:** 2026-11-14
**Confidence:** HIGH (verified directly against `layers.toml`, `crate-edges.allow.v1.json`, `vox-terminal-core`, `vox-orchestrator-d`, `vox-orchestrator-mcp`, `vox-gui`, and `contracts/`)

## Standard Architecture

### System Overview

All components strictly obey `docs/src/architecture/layers.toml` (`L0 -> L1 -> L2 -> L3 -> L4 -> L5`, descending dependencies only) and require **zero new workspace Cargo edges** in `contracts/ci/crate-edges.allow.v1.json` and **zero vendor LLM coupling** outside `vox_actor_runtime::llm`.

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ L5 SURFACES (Binaries & UI Shells)                                                     │
├────────────────────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────────────────────┐  ┌──────────────────────────┐  ┌──────────────────┐  │
│  │ vox-gui (Axis Tauri + React) │  │ vox-orchestrator-d       │  │ vox-cli          │  │
│  │ - ConsoleSurface             │  │ - TermDispatch (term.*)  │  │ - vox term       │  │
│  │ - DockWorkspaceShell         │  │ - Injects TerminalBridge │  │ - vox shell check│  │
│  │   └─ TerminalDockPanel       │  │   adapter into MCP state │  │ - operations-sync│  │
│  │ - Shared TerminalSessionPane │  │ - Owns singleton         │  └────────┬─────────┘  │
│  │   (BlockStream ↔ Auto-TUI)   │  │   Arc<LocalHost>         │           │            │
│  │ - CV Flywheel & 4-Theme Gate │  └────────────┬─────────────┘           │            │
│  └──────────────┬───────────────┘               │                         │            │
├─────────────────┼───────────────────────────────┼─────────────────────────┼────────────┤
│ L4 TUI FRONTEND │  ┌────────────────────┐       │                         │            │
│                 │  │ vox-term (ratatui) │───────┼────┐                    │            │
│                 │  └────────────────────┘       │    │                    │            │
├─────────────────┴───────────────────────────────┼────┼────────────────────┼────────────┤
│ L3 HEAVY RUNTIMES (Engine, Orchestrator, MCP)   ▼    ▼                    ▼            │
├────────────────────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────────────────────┐  ┌──────────────────────────┐  ┌──────────────────┐  │
│  │ vox-terminal-core            │  │ vox-orchestrator         │  │ vox-actor-runtime│  │
│  │ - Stateful PtyPool           │  │ - terminal_bridge trait  │  │ - llm::{llm_chat,│  │
│  │   (retains MasterPty resize, │─▶│ - exec_policy evaluator  │◀─│   llm_stream,    │  │
│  │    ZDOTDIR/--rcfile/-NoExit) │  │   (exec-policy.v1.yaml)  │  │   infer_with_    │  │
│  │ - Streaming EscapeScanner    │  │ - oversight::floor       │  │   retry}         │  │
│  │   (OSC 633 + OSC 7 + ?1049h) │  │   (L0..L3 safety floor)  │  │   (SSOT LLM wire)│  │
│  │ - Seq-indexed Session buffer │  │ - OrchDaemonClient       │  └────────▲─────────┘  │
│  └──────────────────────────────┘  └────────────▲─────────────┘           │            │
│                                                 │                         │            │
│  ┌──────────────────────────────────────────────┴─────────────────────────┴─────────┐  │
│  │ vox-orchestrator-mcp                                                             │  │
│  │ - Tools: vox_terminal_{exec,read,send_input,sessions} (via TerminalBridge/RPC)   │  │
│  │ - Co-Pilot Gate: ExecPolicyV1 + SHELL_COMMAND_TOOLS + PendingApprovals HITL      │  │
│  │ - Chat Loop: @term mention resolver + ambient failure slice -> vox_actor_runtime │  │
│  └──────────────────────────────────────────────┬───────────────────────────────────┘  │
├─────────────────────────────────────────────────┼──────────────────────────────────────┤
│ L2 DOMAIN LIBRARIES                             ▼                                      │
│  ┌───────────────────────┐  ┌────────────────────────────────┐  ┌───────────────────┐  │
│  │ vox-mcp-registry      │  │ vox-config                     │  │ vox-journal       │  │
│  │ (TOOL_REGISTRY from   │  │ (paths, config keys, YAML SSOT │  │ (JSONL block/cmd  │  │
│  │  operations catalog)  │  │  loaders)                      │  │  history store)   │  │
│  └───────────┬───────────┘  └───────────────┬────────────────┘  └─────────┬─────────┘  │
├──────────────┼──────────────────────────────┼─────────────────────────────┼────────────┤
│ L1 & L0 PRIMITIVES / PURE TYPES             ▼                             ▼            │
│  ┌───────────────────────┐  ┌────────────────────────────────┐  ┌───────────────────┐  │
│  │ vox-foundation (L1)   │  │ vox-redact (L1)                │  │ vox-secrets (L1)  │  │
│  │ protocol::term_method │  │ redact_text() before LLM/UI    │  │ Clavis SecretId   │  │
│  └───────────────────────┘  └────────────────────────────────┘  └───────────────────┘  │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Layer | Responsibility | Architectural Seam & Edge Discipline |
|-----------|-------|----------------|--------------------------------------|
| `vox-foundation::protocol::term_method` | `L1` | Canonical wire method names (`term.open`, `term.list`, `term.attach`, `term.input`, `term.submit`, `term.resize`, `term.close`, plus v7.5 `term.signal`, `term.delta`, `term.control`, `term.read_blocks`) and `DispatchRequest` / `DispatchResponse`. | Pure protocol constants/envelopes; mirrored by string parity test in `vox-terminal-core::daemon_host` (`term_dispatch.rs:448`). |
| `vox-redact` | `L1` | Conservative secret/token/PII scrubbing (`redact_text`) on terminal blocks before they enter LLM prompts, MCP responses, or persisted journals. | Already linked by both `vox-terminal-core` and `vox-orchestrator-mcp`. Never read raw env secrets; Clavis (`vox-secrets`) owns secret resolution. |
| `vox-mcp-registry` | `L2` | Compile-time `TOOL_REGISTRY` generated from `contracts/mcp/tool-registry.canonical.yaml` (synced from `contracts/operations/catalog.v1.yaml`). | `vox-orchestrator::tool_receipt` fails closed on any tool absent from `TOOL_REGISTRY`. |
| `vox-orchestrator` (`terminal_bridge` + `oversight`) | `L3` | Owns the pure-Rust `TerminalBridge` async trait, `ExecPolicyV1` loader/evaluator (`contracts/terminal/exec-policy.v1.yaml`), and `oversight::floor` (`L0`–`L3` safety floor). | **Key zero-edge seam:** `vox-orchestrator-mcp` (`L3`), `vox-terminal-core` (`L3`), `vox-orchestrator-d` (`L5`), `vox-cli-research` (`L5`), and `vox-gui` (`L5`) *already* depend on `vox-orchestrator` (`serde_yaml` and `regex` are already in `vox-orchestrator/Cargo.toml`). |
| `vox-terminal-core` | `L3` | Stateful `PtyPool` (`MasterPty` retained for `resize`, non-echo shell bootstrap, `SIGINT` cancellation), single-pass streaming `EscapeScanner` (`OSC 633` + `OSC 7` + `DECSET 1049`), sequence-indexed `Session` blocks/ring buffer, `LocalHost` & `DaemonHost`. | Depends only on `vox-compiler`, `vox-journal`, `vox-orchestrator`, `vox-redact`. Zero GUI/Tauri/MCP dependencies. |
| `vox-orchestrator-mcp` | `L3` | Implements `vox_terminal_{exec,read,send_input,sessions}` MCP tools, `@term` / `@term:<session>` chat mentions, Co-Pilot `PendingApprovals` HITL gating, and chat tool selection (`llm_bridge::tool_selection`). | Calls `ServerState.terminal_bridge` (in-proc adapter in `vox-orchestrator-d`, or `OrchDaemonClient` `term.*` RPC fallback in standalone stdio `vox mcp`). **No Cargo edge to `vox-terminal-core` required.** All LLM calls stay on `vox_actor_runtime::llm`. |
| `vox-term` | `L4` | Standalone ratatui TUI binary (`vox term`) consuming `TerminalHost` (`LocalHost` or `DaemonHost`). | Benefits automatically from stateful PTY, `OSC 7` cwd, `SIGINT`, and streaming `OutputAppended` events. |
| `vox-orchestrator-d` | `L5` | Instantiates singleton `Arc<vox_terminal_core::LocalHost>`, wires `TermDispatch` for `term.*` JSON-line IPC, and injects `LocalHostTerminalBridge` into `vox-orchestrator-mcp` `ServerState`. | Already depends on both `vox-terminal-core` and `vox-orchestrator-mcp` in `crate-edges.allow.v1.json`. |
| `vox-gui` (Rust + React UI) | `L5` | Tauri `term_session_*` IPC (`commands/pty.rs`), unified `<TerminalSessionPane>` (Roman Scriptorium Warp Block Stream + Auto-TUI xterm.js WebGL takeover), `Console` surface, `DockWorkspaceShell` `TerminalDockPanel`, 4-profile theme contract, and Playwright CV Flywheel gates. | Retires legacy stateless/ephemeral `pty_spawn` path in Console/Dock in favor of daemon-backed `term_session_*` with sequence-delta streaming (`after_seq`). |

---

## Recommended Project Structure

Every file modification or new module stays strictly below the `arch/god_object` ceiling (**< 500 non-blank lines per file**, enforced as `Error` by `vox-arch-check` / `vox-code-audit`).

```
contracts/
├── operations/
│   └── catalog.v1.yaml                          # Add vox_terminal_{exec,read,send_input,sessions}
├── mcp/
│   └── tool-registry.canonical.yaml             # Generated via `vox ci operations-sync --target mcp --write`
├── terminal/
│   └── exec-policy.v1.yaml                      # SSOT allowlist/risk tiers for shell & PowerShell commands
└── gui/
    └── theme-contract.v1.json                   # Roman Scriptorium 4-profile token & terminal palette contract

crates/
├── vox-foundation/src/
│   └── protocol.rs                              # Extend `term_method`: SIGNAL, DELTA, CONTROL, READ_BLOCKS
├── vox-orchestrator/src/
│   ├── terminal_bridge.rs                       # NEW (<250 LoC): `TerminalBridge` trait + DTOs (BlockSummary, ExecOutcome, SessionDelta)
│   └── oversight/
│       ├── floor.rs                             # Add `vox_terminal_exec` / `vox_terminal_send_input` to SHELL_COMMAND_TOOLS
│       └── exec_policy.rs                       # NEW (<300 LoC): Shared `ExecPolicyV1` loader + pure-Rust lexical/pipeline checker
├── vox-terminal-core/src/
│   ├── pty.rs                                   # Retain `MasterPty` for real resize; `OpenSpec`; `interrupt()`; retire per-cmd `Command::new`
│   ├── shell_init.rs                            # NEW (<280 LoC): Non-echo bootstrap (`ZDOTDIR`, `--rcfile`, `-NoExit`, `fish`, `nu`) via `std::env::temp_dir()`
│   ├── escape_scanner.rs                        # NEW (<350 LoC): Streaming `OSC 633` (A/B/C/D/E/P) + `OSC 7` cwd + `DECSET 1049/1047/47` alt-screen
│   ├── osc633.rs                                # Thin compat re-export / unit tests delegating to `escape_scanner`
│   ├── block.rs                                 # Add `cwd`, `alt_screen_used`, `signal`, `truncated`, `seq_range` metadata on `Block`
│   ├── session.rs                               # Route `InputIntent::Shell` over live stateful PTY w/ boundary markers; monotonic `seq` event log
│   ├── host.rs                                  # Extend `TerminalHost`: `open_with(OpenSpec)`, `delta(id, after_seq)`, `signal(id, sig)`, `set_policy(...)`
│   └── daemon_host.rs                           # Client codec for `term.delta`, `term.signal`, `term.control`, `term.read_blocks`
├── vox-orchestrator-d/src/
│   ├── term_dispatch.rs                         # Dispatch `term.delta`, `term.signal`, `term.control`, `term.read_blocks` + LocalHostTerminalBridge impl
│   └── bin/vox_orchestrator_d.rs                # Inject `Arc<LocalHostTerminalBridge>` into MCP `ServerState` at daemon boot
├── vox-orchestrator-mcp/src/
│   ├── server_state.rs                          # Hold `terminal_bridge: Arc<dyn TerminalBridge>` (in-proc or `OrchDaemonClient` RPC adapter)
│   ├── input_schemas/
│   │   └── terminal_schemas.rs                  # NEW (<180 LoC): `schemars::JsonSchema` params for `vox_terminal_*`
│   ├── dispatch_domains/
│   │   └── terminal_tools.rs                    # NEW (<350 LoC): `vox_terminal_{exec,read,send_input,sessions}` + `ExecPolicyV1` + `redact_text`
│   ├── permission_modes.rs                      # Register `vox_terminal_*` in `RISK_CLASSES` (`Read` vs `ShellExec`)
│   ├── floor_gate.rs                            # Gate `vox_terminal_exec` & `vox_terminal_send_input` under Plan / Co-Pilot HITL modes
│   ├── llm_bridge/tool_selection.rs             # Surface `vox_terminal_*` in spoke/turn-ranked selection (keep `FROZEN_PREFIX_CORE_TOOLS` intact)
│   └── chat_tools/chat/
│       ├── mentions.rs                          # Add `@term` (active session last blocks) & `@term:<session_id>` mention resolution
│       └── agent_loop.rs                        # Inject bounded, redacted terminal failure/cwd slice when session is bound to chat thread
├── vox-cli-research/src/
│   └── check_terminal.rs                        # Delegate `ExecPolicyV1` parsing & Rust lexical check to `vox_orchestrator::oversight::exec_policy`
└── vox-gui/
    ├── src/commands/
    │   ├── pty.rs                               # Add `term_session_delta`, `term_session_signal`, `term_session_control`; dedup attach forwarder
    │   └── chat_turn.rs                         # Pass bound `terminal_session_id` + surface HITL terminal approval events to UI
    └── ui/
        ├── src/lib/
        │   ├── terminalBridge.ts                # Typed wrappers for `OpenSpec`, `termSessionDelta`, `termSessionSignal`, `termSessionControl`
        │   └── terminalTheme.ts                 # NEW (<140 LoC): Derive xterm `ITheme` + block tokens from active `ThemeProfileId` CSS vars
        ├── src/components/surfaces/Console/
        │   ├── Console.tsx                      # Roman Scriptorium header, server-synced security/control pills, `<UnifiedSelect>` knobs
        │   ├── TerminalSessionPane.tsx          # NEW (<320 LoC): Unified controller switching BlockStream+InputEditor ↔ Auto-TUI Raw xterm.js
        │   ├── BlockStreamView.tsx              # Tokenized Scriptorium cards (zero `zinc-*`), live streaming output, cwd badge, Copy/Rerun/Ask Loquela
        │   ├── ScriptoriumPromptEditor.tsx      # NEW (<260 LoC): Stateful prompt bar w/ live `OSC 7` cwd, `!`/`?`/: mode sigils, `Ctrl+C` cancel, history
        │   └── TerminalTab.tsx                  # Pure xterm.js + WebGL surface consuming `after_seq` deltas (eliminates `replay_utf8` re-attach storm)
        ├── src/components/surfaces/Chat/
        │   ├── ChatSurface.tsx                  # Replace mock `TerminalDockPanel` lines w/ `<TerminalSessionPane compact />` + `@term` quick-attach
        │   └── ChatTranscript.tsx               # Render inline `vox_terminal_*` tool cards + Co-Pilot Approve/Reject HITL banner
        └── e2e/review/
            └── cv-flywheel.spec.ts              # Add Console + Chat Terminal Dock 4-theme × multi-resolution assertions (0 contrast/clip/select leaks)
```

### Structure Rationale

- **`vox-orchestrator` (`L3`) as the Bridge + Policy Hub:** `vox-orchestrator-mcp` (`L3`) cannot import `vox-terminal-core` (`L3`) without adding a new edge to `contracts/ci/crate-edges.allow.v1.json`, and neither `L3` crate can import `vox-cli-research` (`L5`) where `ExecPolicyV1` currently resides. Placing the 6-method `TerminalBridge` trait and `ExecPolicyV1` evaluator in `vox-orchestrator` (`L3`) connects `vox-terminal-core`, `vox-orchestrator-mcp`, `vox-orchestrator-d`, `vox-cli-research`, and `vox-gui` using **100% existing Cargo edges**.
- **Dedicated `shell_init.rs` & `escape_scanner.rs` in `vox-terminal-core`:** Keeps `pty.rs` (< 350 LoC) focused on `portable-pty` lifecycle/handles while isolating OS/shell-specific non-echo rcfile generation and byte-level VT/OSC state machines for deterministic property testing.
- **Shared `<TerminalSessionPane>` across `Console` and `DockWorkspaceShell`:** Eliminates the split-brain between `Console.tsx` (currently running two disconnected tabs: `TerminalTab` raw PTY and `BlockStreamView` stateless subprocesses) and `ChatSurface.tsx` (`TerminalDockPanel`, currently a mocked string array). Both surfaces mount the exact same component in full or `compact` dock mode.

---

## Architectural Patterns

### Pattern 1: Stateful Live-PTY Execution with Non-Echo Shell Bootstrap & Retained `MasterPty`

**What:** Every `Session` owns **one persistent interactive shell PTY** in `PtyPool`. All user commands (`InputIntent::Shell`), raw TUI keystrokes, and approved agent commands (`vox_terminal_exec`) execute inside that live PTY rather than spawning stateless `std::process::Command::new(&shell) -c` subprocesses (`pty.rs:181`). `PtyHandle` retains `Box<dyn MasterPty + Send>` alongside `writer` and `child` so `master.resize(PtySize { rows, cols, .. })` updates kernel `TIOCSWINSZ` / Windows ConPTY dimensions on every pane/dock resize. Shell integration hooks (`OSC 633` + `OSC 7`) are loaded silently at shell startup via ephemeral temp init files under `std::env::temp_dir()` instead of writing hook text into stdin.

**When to use:** Always for session lifecycle (`open`, `submit`, `resize`, `signal`, `close`).

**Trade-offs:**
- *Pros:* `cd`, `export`, `venv`/`conda`, `ssh`, shell aliases, and background jobs persist across blocks; zero echoed hook spam on startup; real `SIGWINCH`/ConPTY reflow.
- *Cons:* Requires state-machine discipline when a command enters a sub-REPL or awaits interactive stdin (solved by `OSC 633;C` -> `Running` + `DECSET 1049` Auto-TUI + `vox_terminal_send_input` / `SIGINT` timeout).

**Non-Echo Shell Bootstrap Matrix (Forbidden-Pattern Safe):**

| Shell | Detection (no hardcoded `/bin/...` literals) | Non-Echo Launch Strategy |
|-------|----------------------------------------------|--------------------------|
| `zsh` | `$SHELL` / `which::which("zsh")` | Set `ZDOTDIR=<temp_dir>/vox-zsh-<sid>` containing `.zshenv` / `.zshrc` that restores user `ZDOTDIR`, sources user `~/.zshrc`, and installs `precmd`/`preexec` emitting `OSC 633;A/B/C/D` + `OSC 7`. |
| `bash` | `$SHELL` / `which::which("bash")` | Spawn with `--rcfile <temp_dir>/vox-bash-<sid>.bashrc` sourcing `~/.bashrc` then installing `PROMPT_COMMAND` + `PS0` (`OSC 633;C`). |
| `pwsh` / `powershell` | `which::which("pwsh").or_else(\|_\| which::which("powershell"))` | Spawn with `-NoLogo -NoExit -File <temp_dir>/vox-pwsh-<sid>.ps1` wrapping `function global:prompt` (`$LASTEXITCODE` -> `OSC 633;D`, `$PWD.Path` -> `OSC 7`, `OSC 633;A/B`) + `PSReadLine` `Enter` handler (`OSC 633;C`). |
| `fish` | `which::which("fish")` | Spawn with `--init-command "source <temp_dir>/vox-fish-<sid>.fish"` hooking `fish_prompt` and `fish_preexec`. |
| Fallback (`sh` / `cmd` / `nu`) | `CommandBuilder::new_default_prog()` | Boundary-sentinel wrapper (`printf '\033]633;D;%d\007' $?`) injected only when `Session::submit` dispatches structured commands and shell reports `integration_active == false`. |

> **Guardrail (`layers.toml` Rules 11a/11b):** Never write `Command::new("pwsh")`, `Command::new("bash")`, or string literals matching `/(tmp|usr|etc|var|home|opt|bin|root)\b`. Always construct paths from `std::env::temp_dir()`, `dirs::home_dir()`, and `which::which(...)`, and spawn PTYs via `portable_pty::CommandBuilder`.

**Example (`crates/vox-terminal-core/src/pty.rs`):**
```rust
struct PtyHandle {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    init_guard: Option<TempShellInitDir>, // cleans up temp ZDOTDIR/rcfile on Drop
}

impl PtyPool {
    pub fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<()> {
        if let Some(entry) = self.inner.lock().unwrap().get(id) {
            let mut h = entry.lock().unwrap();
            h.master.resize(PtySize {
                rows: rows.max(2),
                cols: cols.max(8),
                pixel_width: 0,
                pixel_height: 0,
            })?;
        }
        Ok(())
    }

    pub fn interrupt(&self, id: &str) -> Result<()> {
        // Cross-platform ConPTY + Unix PTY foreground interrupt (ETX = 0x03)
        self.write_raw(id, b"\x03")
    }
}
```

---

### Pattern 2: Single-Pass Streaming `EscapeScanner` & Sequence-Delta Replay (`after_seq`)

**What:** Replace the batch-buffered `Osc633Parser` (`osc633.rs:144`, which currently withholds all command stdout until `OSC 633;D` closes the block) with a streaming byte/UTF-8 state machine (`EscapeScanner`) that runs on every PTY read chunk and emits:
1. **Clean passthrough bytes + monotonic `seq: u64`** into the bounded 2 MiB `Session` ring buffer (`PtyChunk { seq, bytes_b64 }`).
2. **Incremental `SessionEvent::OutputAppended { id, chunk, seq }`** immediately while a block is `Running` (coalesced at ≤ 16 ms / 64 KiB boundaries), holding back only incomplete `\x1b...` escape prefixes across read boundaries.
3. **`SessionEvent::CwdChanged { cwd }`** on `OSC 7` (`\x1b]7;file://host/path\x07` or `\x1b]7;file://.../...\x1b\\`) and `OSC 633;P;Cwd=...`.
4. **`SessionEvent::AltScreenChanged { active: bool }`** on `CSI ? 1049 h/l` (and legacy `?1047h/l`, `?47h/l`) so full-screen TUIs (`vim`, `nvim`, `htop`, `lazygit`, `less`, `vox term`) trigger instant Auto-TUI takeover and clean restoration to Block View on exit.

**When to use:** On the single reader path from `PtyPool` into `Session`.

**Trade-offs:**
- *Pros:* Sub-frame live output inside Warp blocks; zero `replay_utf8` duplication because UI clients attach **once** (`snapshot.last_seq`) and recover missed chunks on reconnect via `term.delta { id, after_seq }` instead of re-writing 2 MiB into xterm.js on every `BlockClosed`.
- *Cons:* Escape sequences split across two `reader.read()` calls require a small 256-byte pending-escape carryover buffer in `EscapeScanner` (flushed as literal bytes if no terminator arrives within 256 bytes).

---

### Pattern 3: Zero-New-Edge `TerminalBridge` + Two-Stage Exec-Policy & Co-Pilot HITL Gate

**What:** `vox-orchestrator::terminal_bridge::TerminalBridge` decouples MCP/Chat (`vox-orchestrator-mcp`) from the concrete PTY engine (`vox-terminal-core`). Every agent tool call (`vox_terminal_exec`, `vox_terminal_send_input`) passes through a two-stage server-side gate before any bytes touch a PTY:
1. **Stage A — Static Policy & Floor Classification (`vox-orchestrator::oversight`):**
   - Evaluates the command against `ExecPolicyV1` (`contracts/terminal/exec-policy.v1.yaml`: forbidden metacharacters/subshells, `git`/`cargo`/`vox`/`pwsh` verb allowlists, destructive flags) and `classify_floor(tool_name, &args)` (`SHELL_COMMAND_TOOLS`).
   - Checks the target session's server-authoritative `TermSecurityLevel` (`Sandbox` / `Balanced` / `Trusted`) and `TermControlMode` (`Manual` / `CoPilot` / `Autopilot`).
2. **Stage B — Co-Pilot HITL Parking (`PendingApprovals`):**
   - `Manual`: rejects agent execution (`vox_terminal_exec` / `vox_terminal_send_input` denied; `vox_terminal_read` / `vox_terminal_sessions` allowed).
   - `CoPilot` (default): commands passing `ExecPolicyV1` read-only allowlist under `Balanced`/`Trusted` execute immediately and record an audited `Block` tagged `origin: Agent { tool_call_id }`; unlisted, mutating, compound, or `L0`/`L1` floor-classified commands park in `ServerState.pending_approvals` (`OperationKind::ApprovalRequested`), emit a live approval banner in Console + Chat Transcript, and block/return structured `approval_required` until a human resolves it (`refuse_agent_floor_resolution` prevents agent self-approval).
   - `Autopilot` + `Trusted`: executes non-`L0`-destructive commands automatically while still hard-blocking `L0` catastrophic patterns (`rm -rf /`, credential exfiltration, `.git` destruction).

**Example (`crates/vox-orchestrator/src/terminal_bridge.rs`):**
```rust
#[async_trait::async_trait]
pub trait TerminalBridge: Send + Sync {
    async fn list_sessions(&self) -> Result<Vec<TerminalSessionSummary>, String>;
    async fn ensure_session(&self, spec: TerminalOpenSpec) -> Result<TerminalSessionSummary, String>;
    async fn read_blocks(&self, session_id: &str, limit: usize) -> Result<TerminalReadResult, String>;
    async fn exec_command(&self, req: TerminalExecRequest) -> Result<TerminalExecOutcome, String>;
    async fn send_input(&self, req: TerminalSendInputRequest) -> Result<TerminalSendOutcome, String>;
    async fn signal_session(&self, session_id: &str, signal: TerminalSignal) -> Result<(), String>;
}
```

---

### Pattern 4: Unified `<TerminalSessionPane>` (Warp Block Stream ↔ Auto-TUI) & 4-Profile Roman Scriptorium Tokenization

**What:** Unify `Console.tsx` and `ChatSurface.tsx` (`TerminalDockPanel`) around a single `<TerminalSessionPane sessionId={...} compact={...} />` component backed by two synchronized layers over the same `sessionId`:
1. **Default Layer (`altScreen === false` & `viewMode === 'blocks'`):**
   - `BlockStreamView`: Virtualized/bounded list of Roman Scriptorium command tablets styled strictly with semantic CSS custom properties (`var(--color-bg-surface)`, `var(--color-bg-elevated)`, `var(--color-border-subtle)`, `var(--brass)`, `var(--color-status-pass)`, `var(--color-status-fail)`, `var(--font-mono)`). Each block renders command header, `OSC 7` cwd pill, live elapsed/duration badge, exit-code seal, ANSI-rendered output (via lightweight span tokenizer mapped to `--term-ansi-0..15`), and one-click actions (**Copy Command**, **Copy Output**, **Rerun**, **Attach `@term` / Ask Loquela** on non-zero exit).
   - `ScriptoriumPromptEditor`: Pinned bottom composer showing live `cwd`, git branch, session security/control pills (wired to `term.control` RPC), mode sigil (`!` shell, `?` agent via daemon chat/MCP, `:` Vox expression), multi-line editing, history navigation, and `Ctrl+C` (`term.signal { signal: "SIGINT" }`) while a block is `Running`.
2. **Auto-TUI Layer (`altScreen === true` OR user toggles `Raw PTY` inspector):**
   - `TerminalTab` (xterm.js + `@xterm/addon-fit` + `@xterm/addon-webgl` with automatic DOM/canvas fallback on WebGL context loss) stays mounted with `visibility` / layout promotion so switching on `DECSET 1049h` (`vim`, `htop`, `lazygit`) is instantaneous (< 16 ms), focuses xterm automatically, forwards raw keystrokes via `termSessionInput`, and returns cleanly to `BlockStreamView` on `DECSET 1049l`.
3. **4-Profile Theme Synchronization (`terminalTheme.ts`):**
   - Observes `document.documentElement.dataset.theme` (`arcane` Imperial Carrara White & Gold light default, `slate` Minimalist Grey-Blue dark, `glacier` Sunlit Travertine & Verdigris light, `high-contrast` Obsidian & Gold AAA) and updates xterm `terminal.options.theme` from computed CSS tokens — eliminating all `#1E170F` / `bg-zinc-950` light-theme contrast leaks.

---

## Data Flow

### Request & Event Flows

#### 1. Interactive User Command & Live Streaming Block Flow
```
[ScriptoriumPromptEditor: user submits `cargo check -p vox-terminal-core`]
    ↓ invoke("term_session_submit", { id, line })
[vox-gui::commands::pty] ──JSON-line IPC──▶ [vox-orchestrator-d::TermDispatch ("term.submit")]
    ↓
[LocalHost -> Session::submit(InputIntent::Shell)]
    ├─ Writes command line to persistent `PtyPool` writer (NO per-cmd subprocess!)
    ├─ Shell preexec/PS0 emits `\x1b]633;C\x07` -> `EscapeScanner` opens/transitions `Block::Running`
    ├─ Compiler stdout streams in chunks -> `EscapeScanner` emits:
    │     • `SessionEvent::PtyOutput { seq, bytes_b64 }` (for raw xterm layer)
    │     • `SessionEvent::OutputAppended { block_id, chunk, seq }` (live into Block card!)
    └─ Shell precmd/prompt emits `\x1b]7;file://.../cwd\x07` + `\x1b]633;D;0\x07`
          • `SessionEvent::CwdChanged { cwd }`
          • `SessionEvent::BlockClosed { block_id, exit_code: 0, duration_ms }`
    ↓ Tauri event `vox://term-event` (single forwarder per session_id)
[TerminalSessionPane] updates active Block card in-place + updates PromptEditor `cwd` badge
```

#### 2. Agent `vox_terminal_exec` + Co-Pilot Approval Flow
```
[Chat Agent Loop (`vox_actor_runtime::llm`)] emits tool_call `vox_terminal_exec { command, session_id?, cwd?, timeout_ms? }`
    ↓
[vox-orchestrator-mcp::dispatch]
    ├─ 1. `floor_gate::plan_gate` (blocks if session is in read-only Plan mode)
    ├─ 2. `ExecPolicyV1::evaluate(command)` + `oversight::floor::classify_floor` + Session `TermControlMode`
    │     ├─ [Co-Pilot + Unlisted/Mutating Command] ──▶ Parks in `PendingApprovals`, emits `ApprovalRequested`
    │     │     └─ Axis UI renders inline Co-Pilot Approval Banner (Approve / Reject)
    │     │     └─ Human clicks Approve -> `vox_approval_resolve` resumes execution
    │     └─ [Allowlisted Read-Only OR Human-Approved]
    ↓
[ServerState.terminal_bridge.exec_command(...)] ──▶ [LocalHost / Session]
    ├─ Creates/attaches target session, tags Block `origin: Agent { tool_call_id }`
    ├─ Writes command + sentinel fallback (if shell lacks native OSC 633), awaits `BlockClosed` or `timeout_ms`
    │     (on timeout: sends `SIGINT` `\x03`, captures partial output, marks `timed_out: true`)
    └─ Scrubs output via `vox_redact::redact_text` + truncates middle/head-tail to token budget (e.g. 16 KiB)
    ↓
[Tool Result JSON { session_id, block_id, cwd, exit_code, timed_out, output }] ──▶ [vox_actor_runtime::llm next turn]
```

#### 3. Auto-TUI (`DECSET 1049`) Full-Screen Takeover & Return Flow
```
[User or Agent runs `nvim Cargo.toml` or `lazygit` in Block View]
    ↓
[PTY emits `\x1b[?1049h`] ──▶ `EscapeScanner` emits `SessionEvent::AltScreenChanged { active: true }`
    ↓
[TerminalSessionPane]
    ├─ Switches active viewport from `BlockStreamView` to `TerminalTab` (xterm.js WebGL)
    ├─ Calls `fitAddon.fit()` -> `termSessionResize(id, cols, rows)` -> `MasterPty::resize`
    └─ Focuses xterm.js; raw keystrokes stream via `termSessionInput(id, data_b64)`
    ↓
[User exits `nvim` (`:q`)] ──▶ PTY emits `\x1b[?1049l` + `\x1b]633;D;0\x07`
    ↓
[TerminalSessionPane] restores `BlockStreamView`, closes Block with badge `[Interactive TUI · 0]`, focuses PromptEditor
```

#### 4. Bidirectional `@term` Mention & Ambient Failure Context Flow
```
[User clicks "Ask Loquela" on failed Block #14 OR types `@term fix this borrow error` in Chat]
    ↓
[vox-orchestrator-mcp::chat_tools::chat::mentions::resolve_mentions]
    ├─ Resolves `@term` / `@term:<session_id>` via `state.terminal_bridge.read_blocks(sid, limit: 3)`
    ├─ Applies `vox_redact::redact_text` to command + tail output + `cwd` + `exit_code`
    └─ Injects `<terminal_context session="..." cwd="..." last_exit="101">...</terminal_context>` into user turn
    ↓
[vox_actor_runtime::llm] diagnoses failure and optionally calls `vox_terminal_exec` to verify the fix
```

---

## Scaling Considerations

| Scale / Stress Vector | What Breaks First Today | Architecture Adjustment in v7.5 |
|-----------------------|-------------------------|---------------------------------|
| **High-throughput stdout (`cargo build -vv`, `yes`, large logs)** | Per-chunk Tauri IPC floods React state and freezes main thread; unbounded block `output` exhausts heap. | Coalesce PTY reader output into **16 ms / 64 KiB batches** in `vox-terminal-core`; cap per-Block stored UTF-8 at **256 KiB** (retaining head 32 KiB + tail 224 KiB with `truncated_bytes` counter) alongside the 2 MiB raw PTY ring buffer. |
| **Multi-surface attach (Console + Chat `TerminalDockPanel` viewing same session)** | `TerminalTab.tsx` currently re-invokes `termSessionAttach` on every `BlockClosed`, replaying 2 MiB into xterm and spawning N duplicate Rust event-forwarder tasks in `commands/pty.rs`! | Track active forwarder subscriptions per `session_id` in `GuiAppState` (`HashSet<String>` / cancellation token); attach once to obtain `{ blocks, cwd, alt_screen, replay_utf8, last_seq }`, then apply `seq > last_seq` deltas idempotently. |
| **LLM Prompt KV-Cache & Context Budget** | Injecting terminal tools into `FROZEN_PREFIX_CORE_TOOLS` invalidates prefix KV-cache tests; dumping raw build logs blows LLM context windows. | Keep `FROZEN_PREFIX_CORE_TOOLS` (the 12-tool prefix in `ast_slice_and_preflight.rs`) unchanged; surface `vox_terminal_*` in dynamic tail slots (`select_tools_for_turn_ranked` / `spoke_priority_tools`), cap `vox_terminal_read`/`exec` tool output at 16 KiB redacted text with `offset`/`tail_lines` pagination. |
| **WebGL Context Limits in Multi-Dock Layouts** | Chromium/WebView allows ~8–16 active WebGL contexts; opening multiple tabs + docks crashes xterm WebGL renderer. | Activate `@xterm/addon-webgl` **only** on the visible/focused raw/TUI surface and dispose/downgrade background panes to xterm's standard canvas/DOM renderer on `onContextLoss` or tab blur. |

---

## Anti-Patterns

### Anti-Pattern 1: Re-Running `term.attach` on Every Command or `BlockClosed`
- **What people do:** Calling `termSessionAttach(sessionId)` inside `BlockClosed` / `pendingLine` effects (`TerminalTab.tsx:166, 206`) to refresh blocks.
- **Why it's wrong:** `term.attach` returns the full `replay_utf8` (up to 2 MiB) and spawns a brand-new `tokio::spawn` event loop in `crates/vox-gui/src/commands/pty.rs:146`, multiplying every subsequent event by `N` attaches and duplicating output inside xterm.js.
- **Do this instead:** Attach **once** on mount, record `last_seq`, deduplicate the Tauri forwarder task per `session_id` in `GuiAppState`, update blocks directly from `BlockOpened` / `OutputAppended` / `BlockClosed` / `CwdChanged` / `AltScreenChanged` events, and use `term.delta { id, after_seq }` on reconnect.

### Anti-Pattern 2: Spawning Stateless `Command::new(shell) -c` for Block Mode
- **What people do:** Routing `Session::submit(InputIntent::Shell(cmd))` to `pty::run_shell_capture` (`session.rs:79`, `pty.rs:181`) while `PtyPool` runs a separate idle shell for `TerminalTab`.
- **Why it's wrong:** Block mode and Raw PTY mode do not share `cwd`, env vars, shell history, or running processes, and output only appears after process exit.
- **Do this instead:** Write submitted commands directly into the session's persistent `PtyPool` writer and segment output into `Block` records via streaming `OSC 633` markers (with sentinel fallback when `integration_active == false`).

### Anti-Pattern 3: Injecting Shell Hooks via Stdin After PTY Spawn
- **What people do:** Calling `writer.write_all(snippet.as_bytes())` right after `slave.spawn_command(cmd)` (`pty.rs:371`).
- **Why it's wrong:** The PTY line discipline echoes all 20 lines of `precmd`/`PROMPT_COMMAND` hook source code visibly into the user's terminal banner and scrollback buffer.
- **Do this instead:** Materialize an ephemeral per-session init file under `std::env::temp_dir()` and pass `ZDOTDIR` (`zsh`), `--rcfile` (`bash`), `-NoExit -File` (`pwsh`), or `--init-command` (`fish`) at `CommandBuilder` spawn time.

### Anti-Pattern 4: Violating `crate-edges.allow.v1.json` or `layers.toml` Forbidden Patterns
- **What people do:** Adding `vox-terminal-core` to `vox-orchestrator-mcp/Cargo.toml`, importing `vox-cli-research` from `L3`, or writing `Command::new("pwsh")` / `"/bin/zsh"` / `"/tmp/vox-..."` literals.
- **Why it's wrong:** Immediately fails `vox ci crate-edges` and `cargo run -p vox-arch-check` (`no-hardcoded-shell-spawn`, `no-hardcoded-abs-path`).
- **Do this instead:** Route MCP -> Terminal through `vox_orchestrator::terminal_bridge::TerminalBridge` (`L3`), host `ExecPolicyV1` in `vox_orchestrator::oversight::exec_policy` (`L3`), resolve shells via `which::which` / `$SHELL` / `CommandBuilder::new_default_prog()`, and build temp paths via `std::env::temp_dir()`.

### Anti-Pattern 5: Client-Only Security/Control State & Hardcoded Dark Hexes in React
- **What people do:** Storing `securityLevel` / `controlMode` solely in `Console.tsx` `useState` (`Console.tsx:378-390`), rendering raw `<select>` elements (`Console.tsx:374`), and styling blocks/terminals with `bg-zinc-950`, `text-zinc-100`, `#1E170F`, or retired `white-marble-frieze-banner.jpg`.
- **Why it's wrong:** Daemon/MCP tools remain unaware of the operator's `Manual` / `Co-Pilot` / `Autopilot` toggle, and CV Flywheel (`cv-flywheel.spec.ts` + `audits.ts`) fails on `unstyledNativeSelectCount > 0` and `lightThemeContrastLeakCount > 0` under `arcane` (Carrara White) and `glacier` (Travertine).
- **Do this instead:** Persist `TermSecurityLevel` and `TermControlMode` on `Session` via `term.control` RPC, use `<UnifiedSelect>` (`data-unified-knob="true"`), and bind all terminal/block surfaces and xterm `ITheme` palettes to `THEME_CONTRACT` CSS tokens.

---

## Integration Points & Dependency-Ordered Build Plan

### Internal & Contract Boundaries

| Boundary | Contract / Protocol | Enforcement Gate |
|----------|---------------------|------------------|
| `contracts/operations/catalog.v1.yaml` ↔ `vox-mcp-registry` ↔ `vox-orchestrator-mcp` | `vox_terminal_{exec,read,send_input,sessions}` tool definitions, JSON schemas, and `RISK_CLASSES` | `vox ci operations-sync --target mcp --write`, `operations_catalog.rs` schema test, `tool_receipt.rs` fail-closed check |
| `contracts/terminal/exec-policy.v1.yaml` ↔ `vox-orchestrator::oversight::exec_policy` ↔ `vox-cli-research` | Shared allowlist/forbidden-pattern evaluator for both `vox shell check` CLI and runtime MCP/Co-Pilot gating | Unit parity tests ensuring `vox shell check` and `vox_terminal_exec` produce identical verdicts on golden fixtures |
| `vox-orchestrator-d` ↔ `vox-gui` / `vox-term` / `vox-orchestrator-mcp` | Loopback JSON-line `term.*` IPC (`open`, `list`, `attach`, `delta`, `input`, `submit`, `resize`, `signal`, `control`, `read_blocks`, `close`) + in-proc `LocalHostTerminalBridge` | `term_dispatch.rs` round-trip integration tests + wire method constant parity test |
| `Chat Agent Loop` ↔ `vox_actor_runtime::llm` | Model-agnostic inference (`llm_chat`, `llm_stream`, `infer_with_retry`) consuming `@term` context and `vox_terminal_*` tool results | `vox-code-audit` `llm_provider_call` detector (zero direct vendor SDK/URL calls) + `vox_redact::redact_text` |
| `vox-gui` React Surfaces ↔ `contracts/gui/theme-contract.v1.json` | 4-profile Roman Scriptorium CSS variables + `<TerminalSessionPane>` geometry/a11y invariants | `themeContract` unit tests + Playwright `cv-flywheel.spec.ts` across `arcane`, `slate`, `glacier`, `high-contrast` and `900×600`..`1920×1080` |

### Recommended Build Order (5 Dependency-Ordered Phases)

```
Phase 1: Stateful PTY Engine & Streaming Escape Scanner (`vox-terminal-core` L3)
   │     - Retain `MasterPty` for real `resize(cols, rows)` + `interrupt()` (`\x03` SIGINT)
   │     - Non-echo `shell_init.rs` (`ZDOTDIR` / `--rcfile` / `-NoExit` / `fish` via `temp_dir()`)
   │     - Single-pass `escape_scanner.rs`: live streaming `OSC 633` + `OSC 7` cwd + `DECSET 1049` alt-screen
   │     - Route `Session::submit` over live stateful PTY + monotonic `seq` delta log
   ▼
Phase 2: Daemon `term.*` Protocol, Shared `ExecPolicyV1` & `TerminalBridge` (`L1` + `L3` + `L5`)
   │     - Extract `ExecPolicyV1` loader/checker to `vox_orchestrator::oversight::exec_policy` (keep `vox shell check` wired)
   │     - Add `vox_orchestrator::terminal_bridge::TerminalBridge` trait
   │     - Extend `vox-foundation::protocol::term_method`, `DaemonHost`, and `vox-orchestrator-d::TermDispatch`
   │       with `term.delta`, `term.signal`, `term.control`, `term.read_blocks` + `OpenSpec`
   ▼
Phase 3: Bidirectional Chat Harness, `vox_terminal_*` MCP Tools & Co-Pilot HITL Gate (`L2` + `L3`)
   │     - Register `vox_terminal_{exec,read,send_input,sessions}` in `catalog.v1.yaml` -> `operations-sync`
   │     - Implement schemas, `terminal_tools.rs`, `OrchDaemonClient` fallback + `LocalHostTerminalBridge` injection
   │     - Wire `SHELL_COMMAND_TOOLS`, `RISK_CLASSES`, `ExecPolicyV1`, and `PendingApprovals` Co-Pilot HITL gate
   │     - Add `@term` / `@term:<id>` mention resolution + redacted terminal failure context in `chat_tools::chat`
   ▼
Phase 4: Roman Scriptorium Unified `<TerminalSessionPane>`, Auto-TUI & Live Chat Dock (`vox-gui` L5)
   │     - Deduplicate `commands/pty.rs` Tauri forwarders; add `term_session_{delta,signal,control}`
   │     - Build `<TerminalSessionPane>` + `ScriptoriumPromptEditor` + tokenized `BlockStreamView` + Auto-TUI `TerminalTab`
   │     - Sync `Console.tsx` security/control knobs (`<UnifiedSelect>`) with daemon `term.control`; remove retired banner/hexes
   │     - Replace mock `TerminalDockPanel` in `ChatSurface.tsx` (`DockWorkspaceShell`) with live `<TerminalSessionPane compact />`
   │       + inline Co-Pilot terminal approval cards in `ChatTranscript.tsx`
   ▼
Phase 5: 4-Profile Theme Contract, Legacy `pty_spawn` Retirement & CV Flywheel Convergence (`vox-gui` E2E + `vox-term` L4)
         - Verify `vox-term` (`L4`) against updated `TerminalHost` trait
         - Retire orphaned stateless/raw `pty_spawn` Console glue where superseded by `term_session_*`
         - Run Playwright `cv-flywheel.spec.ts` + `audits.ts` across all 4 themes (`arcane`, `slate`, `glacier`, `high-contrast`)
           and viewports (`900×600`..`1920×1080`) + `vox-arch-check` + `vox ci crate-edges` to zero findings
```

**Why this exact build order:**
1. **Engine before IPC (Phase 1 -> Phase 2):** Streaming `OSC 633`/`OSC 7`/`DECSET 1049`, `seq` indexing, and stateful PTY execution establish the exact event and snapshot types that `TerminalBridge` and `term.*` daemon RPCs serialize.
2. **Bridge & Policy before MCP Tools (Phase 2 -> Phase 3):** `vox_terminal_exec` cannot enforce Co-Pilot safety or talk to `vox-orchestrator-d` without `ExecPolicyV1` and `TerminalBridge` in `vox-orchestrator` (`L3`).
3. **Daemon + MCP before Unified React Surfaces (Phase 3 -> Phase 4):** Building `<TerminalSessionPane>`, Auto-TUI switching, Co-Pilot approval cards, and `@term` chat attachments against live `term_session_delta` / `term_session_control` RPCs avoids throwaway UI mocks and eliminates the `TerminalTab.tsx` re-attach bug in a single pass.
4. **Visual & Architectural Convergence Last (Phase 5):** CV Flywheel multi-theme/multi-viewport gates validate the real rendered Console + `DockWorkspaceShell` Terminal Dock with live blocks, TUI mode, and approval banners in place.

---

## Sources

- `docs/src/architecture/layers.toml` — L0–L5 crate hierarchy, `max_loc`, `arch/god_object` (500 non-blank LoC), `no-hardcoded-shell-spawn`, and `no-hardcoded-abs-path` guards
- `docs/src/architecture/where-things-live.md` & `contracts/ci/crate-edges.allow.v1.json` — workspace crate ownership and frozen dependency edge allowlist
- `docs/src/architecture/terminal-session-host-design-2026.md` & `.planning/v1.5-terminal-warp-parity-MILESTONE-AUDIT.md` — `TerminalHost` split (`LocalHost` / `DaemonHost`) and prior Warp-parity audit baseline
- `docs/src/architecture/terminal-exec-policy-ssot.md` & `contracts/terminal/exec-policy.v1.yaml` — terminal command allowlist and safety tier SSOT
- `crates/vox-terminal-core/src/{pty.rs,session.rs,host.rs,daemon_host.rs,osc633.rs,block.rs}` — current PTY pool, session model, and OSC 633 parser seams
- `crates/vox-orchestrator-d/src/{term_dispatch.rs,bin/vox_orchestrator_d.rs}` & `crates/vox-foundation/src/protocol.rs` — daemon `term.*` wire dispatch
- `crates/vox-orchestrator-mcp/src/{server_state.rs,dispatch.rs,floor_gate.rs,permission_modes.rs,llm_bridge/tool_selection.rs,chat_tools/chat/agent_loop.rs,chat_tools/chat/mentions.rs}` — MCP tool dispatch, `PendingApprovals` HITL gate, `@` mentions, and `FROZEN_PREFIX_CORE_TOOLS` KV-cache contract
- `crates/vox-gui/src/commands/{pty.rs,chat_turn.rs}`, `crates/vox-gui/ui/src/components/surfaces/{Console,Chat}/`, `contracts/gui/theme-contract.v1.json`, and `crates/vox-gui/ui/e2e/review/{cv-flywheel.spec.ts,audits.ts}` — Axis GUI Console/Dock surfaces, Roman Scriptorium 4-theme contract, and CV Flywheel audit gates

---
*Architecture research for: Milestone v7.5 (`axis-warp-terminal-v75` — Roman Scriptorium Warp-Class Agentic Terminal, Stateful PTY Engine, Bidirectional Chat Harness & CV Flywheel Convergence)*
*Researched: 2026-11-14*
