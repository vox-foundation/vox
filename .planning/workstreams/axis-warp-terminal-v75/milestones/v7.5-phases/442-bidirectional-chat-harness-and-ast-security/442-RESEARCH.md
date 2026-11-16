# Phase 442: Bidirectional Chat Harness & AST Security - Research

**Researched:** 2026-11-14
**Domain:** Shared Stateful PTY MCP/Chat Harness (`TerminalBridge`), Non-Interactive Pager & Stall Detection (`AwaitingInput`), Pure-Rust AST/Lexical `ExecPolicyV1` + `term.control` Synchronization, and 3-Stage Backend ANSI/`vox-redact`/Clavis Scrubbing (`@term`)
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### TerminalBridge, Catalog Sync & Foreground/Background Execution (`TERM-HARNESS-01`)
- Define the pure-Rust async `TerminalBridge` trait and wire DTOs in `vox_orchestrator::terminal_bridge` (`L3`) so `vox-orchestrator-mcp` (`L3`), `vox-terminal-core` (`L3`), `vox-orchestrator-d` (`L5`), and `vox-gui` (`L5`) connect with **zero new workspace Cargo edges** in `contracts/ci/crate-edges.allow.v1.json`.
- `vox-orchestrator-d` injects an in-process `LocalHostTerminalBridge` wrapping its singleton `Arc<LocalHost>` into MCP `ServerState`, while standalone stdio `vox mcp` uses an `OrchDaemonClient` `term.*` JSON-RPC adapter (falling back to an ephemeral in-proc host only when the daemon is offline).
- `vox_terminal_exec` defaults to `wait_ms = 10_000` (configurable `0..=120_000`); when `background: true` is passed, when `Ctrl+B` promotes an active foreground job, or when `wait_ms` expires while the command is still running, return `status: "running"` (`session_id`, `block_id`, `seq`, redacted output prefix) **without** killing the PTY job so `vox_terminal_read` continues streaming.
- When `session_id` is omitted, bind `chat_thread_id → session_id` inside `ServerState` (reusing the bound or latest live session, or opening a `120×32` default session on first exec) so multi-turn Chat and the Console Dock share stateful `cwd` and environment variables.
- Register `vox_terminal_exec`, `vox_terminal_read`, `vox_terminal_send_input`, and `vox_terminal_sessions` at their sorted positions in `contracts/operations/catalog.v1.yaml`, run `operations-sync --write` to update `contracts/mcp/tool-registry.canonical.yaml` / `TOOL_REGISTRY`, and expose them in `llm_bridge::tool_selection` without mutating `FROZEN_PREFIX_CORE_TOOLS`.

#### Pager Neutralization, Stall Detection & AST `ExecPolicyV1` + `term.control` (`TERM-HARNESS-02`, `TERM-HARNESS-03`)
- Scope non-interactive pager environment defaults (`PAGER=cat`, `GIT_PAGER=cat`, `SYSTEMD_PAGER=cat`, `BAT_PAGER=cat`, `MANPAGER=cat`, `GH_PAGER=cat`; PowerShell/Nushell equivalents on `pwsh`/`nu`) to `BlockOrigin::Agent` command executions so `git log`, `git diff`, `gh`, and `systemctl` never hang on `less`, while leaving human-typed interactive commands untouched.
- Transition running blocks to `BlockStatus::AwaitingInput` immediately when `ALT_SCREEN` activates (`?1049h` / `?47h` / `?1047h`) or after `1_500 ms` of PTY output quiescence when the unterminated trailing line matches interactive prompt signatures (`[y/N]`, `(y/n)`, `Password:`, `passphrase`, `[sudo]`, `Continue?`), returning `status: "awaiting_input"` and prompt context instead of hanging the agent turn.
- Host the shared pure-Rust POSIX + PowerShell lexical/pipeline `ExecPolicyV1` (`contracts/terminal/exec-policy.v1.yaml`) + `classify_floor` evaluator in `vox_orchestrator::oversight::exec_policy` (consumed by `vox-cli-research::check_terminal`, `vox-terminal-core`, and `vox-orchestrator-mcp`), blocking `$()`, backticks, process substitution, `L1` redirections, and wrapper-binary (`pwsh -EncodedCommand`, `bash -c`, `sh -c`, `find -exec`/`-delete`, `env`, `xargs`) bypasses across `L1`–`L5`.
- Persist and synchronize `TermSecurityLevel` (`L1`–`L5`) and `ControlMode` (`Manual`, `CoPilot`, `Autopilot`) via `term.security` and `term.control` daemon RPCs (`CoPilot` routes mutating/above-floor commands through `PendingApprovals` HITL; `Manual` blocks unapproved agent execution).
- Fail-closed on `vox_terminal_send_input` from `BlockOrigin::Agent` whenever `AwaitingInput` classified a credential/secret prompt (`Password:`, `passphrase`, `[sudo]`), requiring the human operator to enter secrets directly in the attached terminal pane.

#### `@term` Citations, Backend ANSI + `vox-redact` / Clavis Scrubbing & Context Budgets (`TERM-HARNESS-04`)
- Parse `@term` mentions via a dedicated `TERM_MENTION_RE` in `crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs` (running before file `MENTION_RE` so `#` and `:L` tokens survive intact), supporting `@term`, `@term:last`, `@term:failed`, `@term:pinned` (`Block.pinned: bool` + `term.pin` RPC), `@term:<sid>`, `@term:<sid>#<id>`, and 1-indexed inclusive `@term:<sid>#<id>:L<a-b>`.
- Enforce strict backend scrubbing order across `vox_terminal_*`, `@term`, `term.context`, and `SendToAgent`: (1) strip all ANSI CSI/OSC/DCS escape sequences and normalize `\r` overwrites first, (2) run upgraded `vox-redact` covering `/Users/<name>`, `/home/<name>`, `C:\Users\<name>`, PEM private keys, URL `user:pass@host` credentials, and `AIza`/`hf_`/`npm_`/`ghp_`/`sk-` tokens, and (3) scrub exact matches of resolved Clavis secret values (`vox-secrets`, length `>= 6` → `[REDACTED:CLAVIS]`).
- Enforce a `16 KiB` / `200`-line default per-block budget using **head `40` lines + tail `160` lines** (preserving exit diagnostics) with a pagination hint for `vox_terminal_read(session_id, block_id, line_start, line_end)`, and a `32 KiB` total `@term` prompt envelope per turn.
- Route `SendToAgent` via structured `{ session_id, block_id, line_range }` / `@term:<sid>#<id>[:L<a-b>]` references resolved on the Rust backend so raw unredacted frontend DOM text never enters an LLM prompt.

### the agent's Discretion
- Exact method signatures on `TerminalBridge` and exact wire field names on `term.control` / `term.pin` / `term.promote_bg` so long as `LocalHost`, `DaemonHost`, `TermDispatch`, and `vox-orchestrator-mcp` stay symmetric and stay under the `< 500` non-blank LoC `arch/god_object` budget per file.

### Deferred Ideas (OUT OF SCOPE)
- Full Roman wax-seal UI rendering for Co-Pilot Approval Cards and `Console.tsx` / `ChatTranscript.tsx` React components (scheduled in Phases 443 and 445; Phase 442 wires the backend/MCP/RPC approval & session plumbing).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| `TERM-HARNESS-01` | Chat agent loop and MCP clients execute, inspect, stream, and send stdin to shared `SessionHost` sessions via `vox_terminal_exec`, `vox_terminal_read`, `vox_terminal_send_input`, and `vox_terminal_sessions` registered in `contracts/operations/catalog.v1.yaml` over `vox-orchestrator::terminal_bridge` (zero new workspace Cargo edges), supporting foreground execution and `Ctrl+B` / `background: true` promotion. | `vox_orchestrator::terminal_bridge` (`L3`) connects `vox-orchestrator-mcp` (`L3`), `vox-terminal-core` (`L3`), `vox-orchestrator-d` (`L5`), and `vox-gui` (`L5`) with 0 `Cargo.toml` edits; sorted insertion in `catalog.v1.yaml` + `http-read-role-governance.yaml` + `operations-sync --target all --write` wires all 5 MCP static checks; `term.promote_bg` + non-killing `wait_ms` handles `Ctrl+B` / `background: true`. |
| `TERM-HARNESS-02` | Agent-dispatched PTY commands neutralize blocking pagers (`PAGER=cat`, `GIT_PAGER=cat`, `SYSTEMD_PAGER=cat`) and detect interactive prompt/alt-screen stalls (`[y/N]`, `Password:`, `?1049h`, foreground PGID idle stdin), transitioning blocks to `AwaitingInput` and surfacing interactive controls instead of deadlocking the agent turn. | Current-shell save/restore pager wrapper in `shell_init.rs` + `agent_exec.rs` neutralizes pagers on `BlockOrigin::Agent` across `zsh`/`bash`/`fish`/`pwsh`/`nu` without subshell `cwd` loss; `stall_detector.rs` monitors `VtShadow::is_alt_screen()` + `1_500 ms` output quiescence against confirmation vs credential prompt signatures. |
| `TERM-HARNESS-03` | Every agent-issued terminal command and `Console` security/control selector (`L1`–`L5`, `Manual` / `Co-Pilot` / `Autopilot`) is synced via `term.security` / `term.control` RPCs and gated by an AST/lexical `ExecPolicyV1` (`contracts/terminal/exec-policy.v1.yaml`) + `classify_floor` check that blocks subshell `$()`, backtick, redirection, and wrapper-binary (`pwsh -EncodedCommand`, `find -exec`) bypasses. | Pure-Rust POSIX + PowerShell lexical/pipeline evaluator in `vox_orchestrator::oversight::exec_policy` unifies `check_terminal.rs`, `TermSecurityLevel`, and `classify_floor`; `term.security`/`term.control` sync across `LocalHost`, `DaemonHost`, `TermDispatch`, `vox-gui/src/commands/pty.rs`, and `PendingApprovals` HITL. |
| `TERM-HARNESS-04` | Referencing `@term`, `@term:last`, `@term:failed`, `@term:pinned`, `@term:<sid>#<id>:L<a-b>`, or clicking `Send to Agent` resolves exclusively through backend ANSI-stripped `vox-redact` (including macOS `/Users/<name>`, PEM keys, URL credentials, `AIza`/`hf_`/`npm_`) + Clavis exact-value secret scrubbing before entering any LLM prompt. | 3-stage pipeline: `vox_terminal_core::block::strip_ansi_and_normalize` → upgraded `vox_redact::redact_owned` (`L0`) → `vox_secrets::redact::scrub_resolved_secrets` (`L1`, zero `secret-env-guard` violations); pre-canonicalizer `TERM_MENTION_RE` in `chat_tools/chat/` + head-40/tail-160 (`16 KiB` / `32 KiB`) truncation + backend `console_a2a::send_to_agent` resolution. |
</phase_requirements>

## Project Constraints (from CLAUDE.md / AGENTS.md)

- **Zero New Crate Edges (`layers.toml` & `crate-edges.allow.v1.json`):** `vox-orchestrator-mcp` (`L3`) MUST NOT depend on `vox-terminal-core` (`L3`), and `vox-orchestrator` (`L3`) MUST NOT depend on `vox-container` / `vox-container-types`. Keep `contracts/ci/crate-edges.allow.v1.json` untouched (`0` new exceptions).
- **`arch/god_object` Budget (`crates/vox-code-audit/src/detectors/god_object.rs`):** Every new or modified `.rs` module MUST stay below `hard_max_lines: 500` (ideally `< 400` `warn_max_lines`) non-blank, non-comment, non-`#[cfg(test)]` lines and `<= 12` methods per `impl` block [VERIFIED: crates/vox-code-audit/src/detectors/god_object.rs:14-21].
- **`[no-hardcoded-abs-path]` (`docs/src/architecture/layers.toml`):** Non-test Rust string literals MUST NOT match `"(/(tmp|usr|etc|var|home|opt|bin|root)\b|[A-Za-z]:\\)` [VERIFIED: docs/src/architecture/layers.toml:529-554]. Construct path regexes with leading non-capturing groups such as `r"(?:/(?:Users|home)/[^/\s]+|[A-Za-z]:\\Users\\[^\\\s]+)"` (where `"` is followed by `(?:`, never `/` or `C:\`).
- **Secret Management (`AGENTS.md §Secret Management` & `vox ci secret-env-guard`):** Never call `std::env::var` on managed secret names. Furthermore, `secret_dataflow_leak_categories` in `crates/vox-cli-research/src/ci_helpers/guards.rs:824-884` flags any statement outside `crates/vox-secrets/` that passes an identifier matching `(api[_-]?key|access[_-]?token|bearer|secret|password|authorization|\.expose\(\))` into `json!`, `format!`, `serde_json::to_string`, `serde_json::to_value`, or `tracing::*!` [VERIFIED: crates/vox-cli-research/src/ci_helpers/guards.rs:831-857]. Keep `.expose()` inside `crates/vox-secrets/src/redact.rs` and name prompt-classification fields `credential_prompt: bool` (never `password_*` or `secret_*`).
- **Big Contract Files (`00-core.md` Rule 11):** Never re-serialize or rewrite `contracts/operations/catalog.v1.yaml` (37,087 lines). Insert the 4 `terminal.*` rows with a single targeted edit at lines 35681–35682 and run `cargo run -p vox-cli -- ci operations-sync --target all --write` (NEVER `--target catalog --write`).
- **Model-Agnostic LLM & KV-Cache Prefix (`AGENTS.md §Model-Agnostic LLM Boundary`):** Never modify the 12-tool `FROZEN_PREFIX_CORE_TOOLS` array in `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/ast_slice_and_preflight.rs:23-36` [VERIFIED: crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/ast_slice_and_preflight.rs:23-36].
- **Shell & Test Execution (`00-core.md` Rules 7–9):** Scope builds/tests with `timeout 300s cargo test -p <crate> ...` (one command per call; never `cargo test --workspace`; format touched `.rs` files only via `rustfmt --edition 2024 <file>`).

## Summary

Phase 441 delivered a stateful multi-shell PTY engine (`vox-terminal-core`) and authenticated daemon RPC surface (`TermDispatch` in `vox-orchestrator-d`), but MCP clients and the Chat agent loop still cannot execute, stream, or send input to those stateful terminal sessions (`vox_terminal_*` tools do not exist yet), agent commands can deadlock on `less` or `[y/N]`/`Password:` prompts, `TermSecurityLevel` in `host.rs` uses a naive whitespace word-split heuristic disconnected from `contracts/terminal/exec-policy.v1.yaml` and `classify_floor`, `Console.tsx` mutates security/control/pin state purely in local React state without backend RPC calls, and `SendToAgent` / `term.context` lack unified ANSI + `vox-redact` + Clavis exact-value secret scrubbing.

Codebase inspection confirms that the entire Phase 442 architecture lands with **zero new `[dependencies]` in any `Cargo.toml` and zero new entries in `contracts/ci/crate-edges.allow.v1.json`**. Specifically:
1. `vox-orchestrator` (`L3`) already depends on `vox-foundation`, `vox-secrets`, `serde_yaml`, `regex`, `async-trait`, `tokio`, and `parking_lot`, making it the natural home for both `vox_orchestrator::terminal_bridge` (`TerminalBridge` trait + wire DTOs + `OrchDaemonTerminalBridge` over existing `OrchDaemonClient`) and `vox_orchestrator::oversight::exec_policy` (pure-Rust POSIX + PowerShell lexical/pipeline `ExecPolicyV1` + `classify_floor` evaluator).
2. `vox-secrets` (`L1`) already depends on `aho-corasick` and owns `crates/vox-secrets/src/redact.rs` (exempt from `secret-env-guard`'s `.expose()` sink check), allowing `vox_secrets::redact::scrub_resolved_secrets` (`>= 6` chars → `"[REDACTED:CLAVIS]"`) to be called directly from `vox-orchestrator`, `vox-orchestrator-mcp`, `vox-orchestrator-d`, and `vox-gui` without adding `vox-redact` or `aho-corasick` edges anywhere.
3. Several existing files in `vox-terminal-core` (`host.rs` at 972 pre-test lines, `session.rs` at 563 pre-test lines, `shell_init.rs` at 1,004 lines) already exceed `GodObjectDetector`'s 500-line ceiling; decomposing new Phase 442 logic into focused `< 350`-line sibling modules (`agent_exec.rs`, `stall_detector.rs`, `term_bridge_impl.rs`, `terminal_schemas.rs`, `terminal_tools.rs`, `term_mentions.rs`) and trimming touched files keeps `vox audit code` / `vox-arch-check` green.

**Primary recommendation:** Build Phase 442 in four vertical, independently testable waves: (1) shared `exec_policy` + 3-stage scrubber (`vox-redact`, `vox-secrets::redact`, `vox-terminal-core` ANSI normalizer), (2) `vox-terminal-core` stateful `agent_exec` (non-subshell 5-shell pager guard, `AwaitingInput` stall/credential detector, `Block.pinned`, `Ctrl+B` `promote_bg`, `ControlMode`/`TermSecurityLevel` events) + `vox_orchestrator::terminal_bridge` + daemon `TermDispatch` RPCs, (3) MCP `catalog.v1.yaml` registration + `vox_terminal_*` dispatch + `PendingApprovals` HITL gating, and (4) `@term` pre-canonicalizer citations + `vox-gui` `pty.rs` / `console_a2a.rs` backend wiring.

## Architectural Responsibility Map

| Capability | Primary Tier | Owning Crate (`Layer`) | Exact Module Path(s) | Zero-Edge Proof |
|------------|-------------|------------------------|----------------------|-----------------|
| `TerminalBridge` trait, wire DTOs & `OrchDaemonClient` RPC adapter | Orchestrator Core | `vox-orchestrator` (`L3`) | `crates/vox-orchestrator/src/terminal_bridge.rs` (unconditional `pub mod terminal_bridge;`) | Uses existing `vox-foundation`, `async-trait`, `serde`, `orch_daemon::OrchDaemonClient` [VERIFIED: crates/vox-orchestrator/Cargo.toml:61-118]. |
| Pure-Rust POSIX + PowerShell `ExecPolicyV1` + `classify_floor` | Orchestrator Oversight | `vox-orchestrator` (`L3`) | `crates/vox-orchestrator/src/oversight/{exec_policy.rs,floor.rs,mod.rs}` | Uses existing `serde_yaml`, `regex`, `vox-bounded-fs`; `vox-cli-research`, `vox-terminal-core`, `vox-orchestrator-mcp` all already depend on `vox-orchestrator`. |
| Upgraded regex PII/token/PEM/URL redaction | Foundation Leaf | `vox-redact` (`L0`) | `crates/vox-redact/src/lib.rs` | Leaf crate (`regex`, `serde_json` only) [VERIFIED: crates/vox-redact/Cargo.toml:12-14]. |
| Clavis exact-value secret scrubbing (`>= 6` chars → `[REDACTED:CLAVIS]`) | Platform Secrets | `vox-secrets` (`L1`) | `crates/vox-secrets/src/redact.rs` | Already depends on `aho-corasick` and owns `all_specs()` + `resolve_secret()` [VERIFIED: crates/vox-secrets/Cargo.toml:26]. |
| ANSI stripping, `\r` normalization, head-40/tail-160 block budget, `AwaitingInput` stall detection, 5-shell pager guard, `Block.pinned`, `promote_bg` | Terminal Engine | `vox-terminal-core` (`L3`) | `crates/vox-terminal-core/src/{block.rs,agent_exec.rs,stall_detector.rs,corpus/redact.rs,host.rs,daemon_host.rs}` | Already depends on `vox-orchestrator` and `vox-redact` [VERIFIED: crates/vox-terminal-core/Cargo.toml:25-27]. |
| In-proc `LocalHostTerminalBridge` + `term.{security,control,pin,promote_bg,agent_exec,context}` JSON-RPC | Daemon Binary/Lib | `vox-orchestrator-d` (`L5`) | `crates/vox-orchestrator-d/src/{term_dispatch.rs,term_bridge_impl.rs,bin/vox_orchestrator_d.rs}` | Already depends on `vox-orchestrator`, `vox-orchestrator-mcp`, `vox-terminal-core`, `vox-secrets`, `vox-foundation` [VERIFIED: crates/vox-orchestrator-d/Cargo.toml:15-28]. |
| `vox_terminal_*` MCP schemas, dispatch, `PendingApprovals` HITL, `chat_thread_id -> session_id`, `@term` citations | MCP & Chat Loop | `vox-orchestrator-mcp` (`L3`) | `crates/vox-orchestrator-mcp/src/{terminal_schemas.rs,dispatch_domains/terminal_tools.rs,chat_tools/chat/term_mentions.rs}` | Already depends on `vox-orchestrator`, `vox-redact`, `vox-secrets`, `vox-foundation` [VERIFIED: crates/vox-orchestrator-mcp/Cargo.toml:47,88,97,101]. |
| `vox shell check` delegation to shared `exec_policy` | CLI Diagnostics | `vox-cli-research` (`L5`) | `crates/vox-cli-research/src/check_terminal.rs` | Already depends on `vox-orchestrator` (`check_terminal.rs` shrinks from 764 lines to a thin CLI presenter). |
| Desktop `term_session_{security,control,pin,promote_bg}` & backend-scrubbed `SendToAgent` | Desktop Shell | `vox-gui` (`L5`) | `crates/vox-gui/src/commands/{pty.rs,console_a2a.rs}`, `crates/vox-gui/src/main.rs` | Already depends on `vox-terminal-core`, `vox-secrets`, `vox-orchestrator`, `vox-foundation` [VERIFIED: crates/vox-gui/Cargo.toml:22-31,62]. |

## Standard Stack

### Core (Already Pinned in Workspace `Cargo.toml` — Zero Additions)
| Library | Workspace Use | Purpose in Phase 442 | Why Standard |
|---------|---------------|----------------------|--------------|
| `async-trait` + `tokio` + `parking_lot` | `vox-orchestrator`, `vox-terminal-core`, `vox-orchestrator-mcp` | Async `TerminalBridge` trait, `ServerState` `PrRwLock` session map, `Condvar` / `Instant` `wait_ms` + `1_500 ms` quiescence polling | Already linked in every participating crate [VERIFIED: crates/vox-orchestrator/Cargo.toml:85-100]. |
| `serde` + `serde_json` + `serde_yaml` | `vox-orchestrator`, `vox-foundation` | `ExecPolicyV1` YAML loading (`contracts/terminal/exec-policy.v1.yaml`) and `term.*` JSON-RPC wire envelopes | Already linked in `vox-orchestrator` (`serde_yaml` at line 99) [VERIFIED: crates/vox-orchestrator/Cargo.toml:97-99]. |
| `regex` | `vox-redact`, `vox-orchestrator`, `vox-orchestrator-mcp` | `TERM_MENTION_RE`, `ExecPolicyV1` `forbidden_regex`, and `vox-redact` PII/PEM/URL/token patterns | Already workspace-standard with `OnceLock<Regex>` caching [VERIFIED: crates/vox-redact/src/lib.rs:34-67]. |
| `aho-corasick` | `vox-secrets` | Single-pass `LeftmostLongest` exact-value scrubbing of resolved Clavis secrets (`>= 6` chars) | Already a direct dependency of `vox-secrets` and used in `crates/vox-secrets/src/redact.rs` [VERIFIED: crates/vox-secrets/Cargo.toml:26]. |
| `schemars` | `vox-orchestrator-mcp` | Strongly-typed JSON Schema generation (`schema_for!`) for `vox_terminal_exec`, `vox_terminal_read`, `vox_terminal_send_input`, `vox_terminal_sessions` | Required by `input_schemas.rs` schemars-first convention [VERIFIED: crates/vox-orchestrator-mcp/src/input_schemas.rs:7-10]. |

### Alternatives Considered (Rejected)
| Instead of | Could Use | Tradeoff / Why Rejected |
|------------|-----------|-------------------------|
| Pure-Rust POSIX + PowerShell lexical/pipeline analyzer in `vox_orchestrator::oversight::exec_policy` | Importing `vox-container-types::exec_grammar` or `tree-sitter-bash` | **Rejected:** `vox-orchestrator` does NOT depend on `vox-container-types` (adding it would require a new crate edge), `vox-container-types::exec_grammar` only parses POSIX (no PowerShell `-EncodedCommand` / `Invoke-Expression` support), and `tree-sitter-*` C grammars violate the minimal C toolchain rule when pure-Rust lexical analysis handles POSIX + PowerShell cleanly in ~280 LoC. |
| Placing Clavis value scrubber in `vox-secrets::redact` | Calling `resolve_secret(...).expose()` inside `vox-orchestrator-mcp` or `vox-gui` | **Rejected:** `vox-gui` and `vox-orchestrator-d` do not directly depend on `aho-corasick` or `vox-redact`, and calling `.expose()` near `json!` / `format!` outside `crates/vox-secrets/` trips `vox ci secret-env-guard` (`secret_dataflow_leak_categories`). |
| Wrapping agent PTY commands in a POSIX subshell `( export PAGER=cat ...; <cmd> )` | Current-shell pager save/restore helper in `shell_init.rs` | **Rejected:** Running `<cmd>` inside `( ... )` discards `cd <dir>`, `export FOO=bar`, and `source .venv/bin/activate`, breaking `TERM-HARNESS-01` / `TERM-PTY-02` stateful PTY persistence! |

## Architecture Patterns

### System Data-Flow Diagram
```
Chat User ("fix @term:failed") / MCP Client
   │
   ├─► 1. term_mentions.rs (BEFORE mentions.rs & canonicalize_prompt)
   │      └─► TerminalBridge::resolve_citation / read ──► 3-Stage Scrubber (ANSI -> vox-redact -> Clavis)
   │
   ├─► 2. LLM selects `vox_terminal_exec` (spoke_priority_tools; FROZEN_PREFIX_CORE_TOOLS untouched)
   │
   └─► 3. dispatch.rs -> terminal_tools.rs
          ├─► ExecPolicyV1 + classify_floor (`vox_orchestrator::oversight::exec_policy`)
          ├─► Session `TermSecurityLevel` (`L1..L5`) & `ControlMode` (`Manual` | `CoPilot` | `Autopilot`)
          │     ├─► Denied by L1..L5 / ExecPolicyV1 ──► Hard Error (ToolResult::err)
          │     ├─► Manual OR CoPilot (L2+/mutating) OR Floor Hit ──► Park on `PendingApprovals` (HITL)
          │     └─► Approved / Auto-allowed ──► TerminalBridge::exec(session_id, command, wait_ms, background)
          │
          └─► LocalHostTerminalBridge (in-proc daemon) OR OrchDaemonTerminalBridge (stdio MCP TCP)
                ├─► Agent Pager Guard (PAGER=cat, GIT_PAGER=cat, SYSTEMD_PAGER=cat, BAT_PAGER=cat, MANPAGER=cat, GH_PAGER=cat)
                ├─► Wait loop (`0..=120_000ms`, default `10_000ms`):
                │     ├─► OSC 633;D exit ──────────────► status: "completed" (exit_code, head-40 + tail-160 scrubbed)
                │     ├─► ALT_SCREEN or 1500ms prompt ─► BlockStatus::AwaitingInput -> status: "awaiting_input"
                │     └─► wait_ms timeout / Ctrl+B ────► status: "running" (PTY job stays alive; stream via vox_terminal_read)
                └─► ToolReceipt issued & fulfilled automatically by `dispatch.rs`
```

### Pattern 1: `TerminalBridge` Trait, `ServerState` Session Binding & Dual-Mode Transport (`TERM-HARNESS-01`)

- **Feature-Flag Invariant in `vox-orchestrator`:** `vox-orchestrator-mcp` depends on `vox-orchestrator` with `default-features = false, features = ["runtime"]` [VERIFIED: crates/vox-orchestrator-mcp/Cargo.toml:47]. Declare `pub mod terminal_bridge;` unconditionally in `crates/vox-orchestrator/src/lib.rs` and `pub mod exec_policy;` unconditionally in `crates/vox-orchestrator/src/oversight/mod.rs`.
- **Shared Interior-Mutable Slot on `ServerState`:** In `crates/vox-orchestrator-d/src/daemon_steps/run.rs:175-207`, `step_agent_fleet` clones `ServerState` **before** `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs:288-294` constructs `TermDispatch` [VERIFIED: crates/vox-orchestrator-d/src/daemon_steps/run.rs:199-207]. Therefore `ServerState` MUST hold:
  ```rust
  pub terminal_bridge: Arc<PrRwLock<Option<Arc<dyn vox_orchestrator::terminal_bridge::TerminalBridge>>>>,
  pub chat_terminal_sessions: Arc<PrRwLock<HashMap<String, String>>>,
  ```
  following the exact `state.set_boot_report(...)` pattern (`server_state.rs:143`, `vox_orchestrator_d.rs:276`). In `vox_orchestrator_d.rs:288-296`, construct one `let local_host = Arc::new(vox_terminal_core::LocalHost::default());`, pass `local_host.clone()` to `TermDispatch::new(local_host.clone())`, and call `state.set_terminal_bridge(Arc::new(term_bridge_impl::LocalHostTerminalBridge::new(local_host)))` so `step_agent_fleet`, `McpExtraDispatch`, `http_gateway`, and `TermDispatch` all share the **exact same** `Arc<LocalHost>`!
- **Standalone `vox mcp` Routing:** Note two complementary layers:
  1. `crates/vox-orchestrator-mcp/src/daemon_route.rs:32-51` already proxies stdio MCP `call_tool` over `orch.tool_call` to `vox-orchestrator-d` when the daemon is reachable!
  2. When `ServerState` executes locally without `LocalHostTerminalBridge` injected (e.g., standalone embedders, direct unit/integration harnesses, or daemon-offline fallback), a lazy resolver on `ServerState` first tries `OrchDaemonTerminalBridge` (calling `OrchDaemonClient::new_from_env()` `term.*` JSON-RPC methods in `vox_orchestrator::terminal_bridge`) and, only if the daemon is unreachable, lazily instantiates an ephemeral fallback bridge.
- **`chat_thread_id → session_id` Sticky Binding:** When `vox_terminal_exec` or `vox_terminal_read` omits `session_id`:
  1. Look up `chat_thread_id` (passed from `agent_loop` tool context or `"default"`) in `state.chat_terminal_sessions`; if bound and still listed in `bridge.list_sessions()`, reuse it.
  2. Otherwise, if `bridge.list_sessions()` has a live session (e.g., opened by `Console` / `TerminalDockPanel`), bind and reuse the latest live session so Chat and GUI share `cwd`/env immediately.
  3. Otherwise call `bridge.open_session(120, 32, workspace_root)` and record the binding in `state.chat_terminal_sessions`.

### Pattern 2: Stateful 5-Shell Pager Neutralization, `AwaitingInput` Stall Detection & `Ctrl+B` Promotion (`TERM-HARNESS-01`, `TERM-HARNESS-02`)

- **Why Subshells Break Stateful PTYs:** Wrapping an agent command as `(export PAGER=cat ...; cd subdir)` runs `cd subdir` in a child subshell whose `cwd` disappears on exit. Instead, keep execution in the **top-level interactive shell** while scoping the 6 pager environment variables (`PAGER=cat`, `GIT_PAGER=cat`, `SYSTEMD_PAGER=cat`, `BAT_PAGER=cat`, `MANPAGER=cat`, `GH_PAGER=cat`) strictly to `BlockOrigin::Agent` submissions:
  - Store `shell_name: String` on `Entry` in `LocalHost::open` (`pty.shell_name.clone()`, already detected in `PtyHandle::spawn_default` at `crates/vox-terminal-core/src/pty.rs:148`).
  - Define a lightweight `__vox_agent_pager_enter` / restore hook in `shell_init.rs` (or per-shell inline save/restore wrapper in `agent_exec.rs`):
    - **POSIX (`zsh` / `bash`):** In `shell_init.rs`, define `__vox_pager_enter` (saves current `${PAGER}`, `${GIT_PAGER}`, `${SYSTEMD_PAGER}`, `${BAT_PAGER}`, `${MANPAGER}`, `${GH_PAGER}` + sets `__VOX_PAGER_GUARD=1` and exports the 6 vars to `cat`) and restore them inside the existing `__vox_precmd` (`zsh` / `bash` prompt hook, `shell_init.rs` before `633;D` after capturing `local ec=$?`) whenever `__VOX_PAGER_GUARD=1`! That way `git log && cd subdir && export FOO=1` runs in the **current shell**, preserves `$?`, mutates `cwd` and `FOO`, never leaks `PAGER=cat` to the next human-typed command, and `Session::begin_pty_submission(display_input, BlockOrigin::Agent)` (`session.rs:189-201`) records only the clean original `command` string in `Block.input` (because `Osc633Event::CommandLine` at `session.rs:468` only overwrites `b.input` `if b.input.is_empty()`!).
    - **`fish`:** Same save/restore in `__vox_fish_postexec` / `__vox_fish_prompt` when `$__VOX_PAGER_GUARD = 1`.
    - **`pwsh`:** Save/set `$env:PAGER='cat'; $env:GIT_PAGER='cat'; $env:SYSTEMD_PAGER='cat'; $env:BAT_PAGER='cat'; $env:MANPAGER='cat'; $env:GH_PAGER='cat'; $__vox_pager_guard = $true` and restore in the `prompt` function after capturing `$LASTEXITCODE`.
    - **`nu`:** Nu closures/hooks cannot mutate parent `env` unless `--env` `def --env` is used: define `def --env __vox_pager_enter []` and restore in `env.config.hooks.pre_prompt` (which Nushell evaluates with environment mutation privileges).
- **`AwaitingInput` Stall Detection (`crates/vox-terminal-core/src/stall_detector.rs`):**
  - Add `AwaitingInput` to `BlockStatus` (`crates/vox-terminal-core/src/block.rs:16-21`):
    ```rust
    pub enum BlockStatus {
        Running,
        AwaitingInput,
        Ok,
        Failed,
        Cancelled,
    }
    ```
    (Also update `TermBlockStatus` in `crates/vox-gui/ui/src/transport.ts:861` to include `'AwaitingInput'` so TypeScript stays aligned.)
  - Add `#[serde(default)] pub pinned: bool` and `#[serde(default)] pub credential_prompt: bool` to `Block` (`block.rs:57-70`) — note the field name `credential_prompt` (never `password_*` or `secret_*`, preserving `secret-env-guard` compliance).
  - Track `last_output_at: Mutex<Instant>` and `promoted_bg: AtomicBool` on `Entry` in `host.rs` (updated in `spawn_pty_reader` whenever `n > 0` bytes arrive).
  - In `LocalHost::exec_agent_command(id, cmd, wait_ms, background)`:
    1. Clamp `wait_ms` to `0..=120_000` (default `10_000`); if `background == true`, return immediately after PTY write with `ExecOutcomeStatus::Running` (`session_id`, `block_id`, `seq`, scrubbed output prefix).
    2. Poll `settled_cv.wait_timeout` in `100 ms` slices up to `wait_ms`:
       - **Exited:** `sess.open_block_id() != Some(block_id) && sess.is_at_prompt()` → return `Completed { exit_code }`.
       - **Promoted (`Ctrl+B` / `term.promote_bg`):** `entry.promoted_bg.swap(false, AcqRel)` → return `Running` immediately without sending `SIGINT`!
       - **Immediate `ALT_SCREEN`:** `sess.is_alt_screen()` (`?1049h` / `?47h` / `?1047h` tracked by `VtShadow` at `vt_shadow.rs:36-45`) → transition open block to `BlockStatus::AwaitingInput` (`credential_prompt: false`), emit `SessionEvent::BlockAwaitingInput { id: block_id, credential_prompt: false, reason: "alt_screen" }`, and return `AwaitingInput`.
       - **`1_500 ms` Output Quiescence Prompt Check:** When `Instant::now().saturating_duration_since(*entry.last_output_at.lock()) >= Duration::from_millis(1500)` while `sess.open_block_id() == Some(block_id)`, run `classify_trailing_prompt(plain_tail)` on the last non-empty unterminated trailing text of the open block:
         - **Credential signatures (`credential_prompt = true`):** case-insensitive `password:`, `passphrase`, `[sudo]`, `enter pin`, `verification code`, `secret key` (matched as string literals, never variable names).
         - **Confirmation signatures (`credential_prompt = false`):** case-insensitive `[y/n]`, `(y/n)`, `[yes/no]`, `(yes/no)`, `continue?`, `proceed?`, `overwrite?`, `are you sure`.
         - Transition block to `BlockStatus::AwaitingInput`, set `block.credential_prompt`, emit `SessionEvent`, and return `AwaitingInput`.
       - **Timeout (`wait_ms` elapsed):** Return `Running` **without** killing or signalling the PTY process so `vox_terminal_read` continues streaming live deltas via `after_seq`.
- **Fail-Closed `vox_terminal_send_input`:**
  - When `LocalHost::agent_send_input(session_id, data)` is called while the open block has `credential_prompt == true`, reject immediately with an explicit fail-closed error (`"terminal session is awaiting a credential/password prompt; ask the human operator to enter it directly in the terminal pane"`).
  - When valid non-credential input is written (e.g., `"y\n"` on `[y/N]`), reset `block.status = BlockStatus::Running` so subsequent output resumes normal completion/stall tracking.

### Pattern 3: Pure-Rust AST/Lexical `ExecPolicyV1` + `classify_floor` + `term.security`/`term.control` Sync (`TERM-HARNESS-03`)

- **Unifying Three Divergent Policy Checks into `vox_orchestrator::oversight::exec_policy`:**
  Today the repo has three separate shell-safety implementations:
  1. `crates/vox-cli-research/src/check_terminal.rs` (764 lines — deserializes `contracts/terminal/exec-policy.v1.yaml` and runs PowerShell/Bash regex checks for `vox shell check`, tested by `vox ci exec-policy-contract` in `crates/vox-cli/src/commands/ci/exec_policy_contract.rs:19-33`).
  2. `crates/vox-orchestrator/src/oversight/floor.rs:124-179` (`classify_shell_command` — linear prefix match on `git reset --hard`, `git push --force`, `gh pr create`, etc., which currently **misses** `echo ok && git push --force` or `bash -c "git reset --hard"`!).
  3. `crates/vox-terminal-core/src/host.rs:78-310` (`TermSecurityLevel::allows_agent_command` — whitespace-only `split(['|', ';', '&'])` check that `L3ProjectWrite` passes with a `true` catch-all at line 294!).
- **Unified Pure-Rust Lexical + Pipeline + Wrapper Evaluator (`crates/vox-orchestrator/src/oversight/exec_policy.rs`):**
  1. Move `ExecPolicyV1` contract types (`contracts/terminal/exec-policy.v1.yaml`, embedded via `include_str!("../../../../contracts/terminal/exec-policy.v1.yaml")` fallback plus workspace file loader so unit tests work without `cwd` assumptions) into `vox_orchestrator::oversight::exec_policy`.
  2. Implement a pure-Rust quote-aware POSIX + PowerShell scanner (`tokenize_and_split_segments`) that:
     - Tracks single quotes `'...'`, double quotes `"..."`, POSIX `\` escapes, and PowerShell backtick escapes while detecting unquoted/double-quoted:
       - **Command substitution / process substitution:** `$(...)`, POSIX backticks `` `...` ``, `<(...)`, `>(...)`, PowerShell `@(...)` subexpressions, `& { ... }` scriptblocks, and `Invoke-Expression` / `iex`.
       - **Redirections:** `>`, `>>`, `2>`, `2>>`, `&>`, `&>>`, `>|`, `<`, `<<`, `<<<`, plus PowerShell file-write cmdlets (`Out-File`, `Set-Content`, `Add-Content`, `Tee-Object`).
       - **Compound operators:** `;`, `|`, `||`, `&&`, background `&`, `\n`, `\r` — splitting every command into explicit `CommandSegment { argv, has_output_redirect, has_input_redirect, raw }`.
     - **Wrapper-binary unwrapping / blocking:** Recursively inspects or blocks wrapper invocations so no wrapper can launder a forbidden payload:
       - Env/exec prefixes (`env VAR=val <inner>`, `command <inner>`, `builtin <inner>`, `nohup <inner>`, `nice -n N <inner>`, `time <inner>`, `timeout N <inner>`): strip wrapper prefix and evaluate `<inner>` against the exact same security level and floor rules!
       - Shell/interpreter string-eval wrappers (`sh -c`, `bash -c`, `zsh -c`, `fish -c`, `nu -c`, `cmd /c`, `pwsh -c` / `-Command`, `python -c`, `node -e`, `perl -e`, `ruby -e`): either recursively parse the `-c` argument string at `L2+` or block outright at `L1–L3`; **always block** encoded/opaque wrappers (`pwsh` / `powershell` `-EncodedCommand` / `-enc` / `-ec` / `-e` prefix flags, `base64 -d | sh`, `eval`).
       - `find` mutation/exec flags (`-exec`, `-execdir`, `-ok`, `-okdir`, `-delete`, `-fprint`, `-fprintf`) and `xargs`: blocked in `L1ReadOnly`; in `L2–L4`, `find -exec <inner>` / `xargs <inner>` evaluates `<inner>` against the active policy + `classify_floor`!
     - **Path normalization without tripping `[no-hardcoded-abs-path]`:** Normalize `/path/to/git` or `C:\Program Files\Git\cmd\git.exe` to `"git"` via `.rsplit(['/', '\\']).next()` + `.trim_end_matches(".exe")` (character literals `'/'` and `'\\'`, zero forbidden string literals).
  3. **Evaluate Every Extracted Segment Against Both:**
     - `ExecPolicyV1` (`forbidden_regex`, `deny_global_prefixes`, PowerShell encoded-command flags, bash chain policy — keeping `vox shell check` and `vox ci exec-policy-contract` 100% green).
     - `classify_floor` (`floor.rs` upgraded to inspect **every** unpacked segment from `exec_policy::extract_effective_commands(cmd)`, so `cd repo && git push --force` and `env FOO=1 gh pr create` reliably trigger `FloorCategory::HistoryRewrite` / `ExternalEffect`!).
     - `SecurityTier` (`L1ReadOnly`, `L2WorkspaceSafe`, `L3ProjectWrite`, `L4SystemWrite`, `L5Unrestricted`), replacing the buggy catch-all in `vox-terminal-core/src/host.rs:78-310`.
- **Synchronized `ControlMode` × `TermSecurityLevel` Gate (`TERM-HARNESS-03`):**
  | Session `ControlMode` | `L1` Read-Only Safe Cmd (no floor) | `L2..=SessionLevel` Cmd (no floor) | Command Above `SessionLevel` or `ExecPolicyV1` Hard Deny | `classify_floor` Hit (`HistoryRewrite`, `ExternalEffect`, `PolicyGrant`, `SecretsWrite`) |
  |---|---|---|---|---|
  | `Manual` | Park on `PendingApprovals` HITL | Park on `PendingApprovals` HITL | Reject immediately (`ToolResult::err`) | Park on `PendingApprovals` HITL (or reject if above `SessionLevel`) |
  | `CoPilot` *(default)* | **Auto-execute** | Park on `PendingApprovals` HITL | Reject immediately (`ToolResult::err`) | Park on `PendingApprovals` HITL |
  | `Autopilot` | **Auto-execute** | **Auto-execute** | Reject immediately (`ToolResult::err`) | **Park on `PendingApprovals` HITL** (inviolable floor) |
  - Additionally, in `LocalHost::submit` (`host.rs:754-757`) and `LocalHost::input` (`host.rs:898-900`), whenever user input transitions `ControlMode::Autopilot -> ControlMode::Manual`, emit `SessionEvent::ControlModeChanged { mode: ControlMode::Manual }` (and emit `SecurityLevelChanged` / `ControlModeChanged` from `set_security_level` / `set_control_mode`) so attached GUI `Console` and `vox-term` clients update immediately without polling.

### Pattern 4: 3-Stage Backend Scrubbing, `@term` Pre-Canonicalizer Citations & `SendToAgent` (`TERM-HARNESS-04`)

- **Strict 3-Stage Backend Scrubbing Order:**
  1. **Stage 1 — Full ANSI Escape Stripping & `\r` Overwrite Normalization (`vox_terminal_core::block::strip_ansi_and_normalize`):**
     - First normalize carriage-return progress overwrites (`normalize_cr_overwrites`, already in `block.rs:92-119`), then strip **all** ECMA-48 / VT sequences: CSI (`ESC [ ... @..~`), OSC (`ESC ] ... BEL/ST`), DCS (`ESC P ... ST`), SOS (`ESC X ... ST`), PM (`ESC ^ ... ST`), APC (`ESC _ ... ST`), charset designations (`ESC ( X`, `ESC ) X`), 2-byte ESC sequences (`ESC 7`, `ESC 8`, `ESC c`, `ESC =`, `ESC >`), and stray C0 control characters (except `\n` and `\t`).
     - Why first: split ANSI color sequences inside a token (`sk-\x1b[31mproj-abc...`) would otherwise defeat regex and Aho-Corasick secret matching!
  2. **Stage 2 — Upgraded `vox-redact` (`crates/vox-redact/src/lib.rs`):**
     - `RE_HOME`: `r"(?:/(?:Users|home)/[^/\s]+|[A-Za-z]:\\Users\\[^\\\s]+)"` → `"[REDACTED_HOME]"` (adds macOS `/Users/<name>`, stops Windows match on whitespace, and avoids `[no-hardcoded-abs-path]`!).
     - `RE_PEM`: `r"(?s)-----BEGIN [A-Z0-9 _-]+PRIVATE KEY-----.*?-----END [A-Z0-9 _-]+PRIVATE KEY-----"` → `"[REDACTED_PEM_KEY]"`.
     - `RE_URL_CRED`: `r"([a-zA-Z][a-zA-Z0-9+.-]*://[^/\s:@]+):([^@\s/]+)@"` → `"$1:[REDACTED_URL_PASSWORD]@"`.
     - `RE_TOKEN`: expand existing alternation (`lib.rs:50-63`) with Google `AIza[0-9A-Za-z\-_]{35}`, HuggingFace `hf_[A-Za-z0-9]{30,}`, npm `npm_[A-Za-z0-9]{30,}`, Anthropic `sk-ant-[A-Za-z0-9\-_]{20,}`, OpenAI `sk-(?:proj-|svcacct-)?[A-Za-z0-9_\-]{20,}`, GitHub `gh[posru]_[A-Za-z0-9]{36,}` / `github_pat_[A-Za-z0-9_]{22,}`, AWS `AKIA[0-9A-Z]{16}`, Slack `xox[baprs]-[A-Za-z0-9\-]{10,}`, and JWTs.
  3. **Stage 3 — Clavis Exact-Value Scrubbing (`vox_secrets::redact::scrub_resolved_secrets`):**
     - Iterate `vox_secrets::all_specs()`, skip `spec.metadata.taxonomy_class.is_config_only()` and `matches!(spec.metadata.material_kind, SecretMaterialKind::ConfigValue | SecretMaterialKind::EndpointUrl)` (preventing false-positive redaction of non-secret tuning values like `"ollama"` or `"http://localhost:11434"`), call `vox_secrets::resolve_secret(spec.id)`, collect trimmed `.expose()` strings with `.len() >= 6`, deduplicate, build `AhoCorasick::builder().match_kind(MatchKind::LeftmostLongest).build(&patterns)`, and `replace_all` with `"[REDACTED:CLAVIS]"`.
- **Budgeted Truncation (`16 KiB` / `200` lines per block; `32 KiB` per `@term` turn):**
  - When a scrubbed block exceeds `200` lines or `16_384` bytes and no explicit `line_start..=line_end` was requested: keep **head `40` lines + tail `160` lines** (capped to `16 KiB` on UTF-8 char boundaries) with an explicit marker:
    `... [omitted N lines (L41..LM); call vox_terminal_read(session_id="<sid>", block_id=<id>, line_start=41, line_end=<next>) to inspect] ...`
- **`@term` Citation Grammar & Pre-Canonicalizer Extraction (`term_mentions.rs`):**
  - Regex: `r"@term(?::([A-Za-z0-9_-]+)(?:#(\d+)(?::L(\d+)(?:-(\d+))?)?)?)?(?=$|[\s,.;:!?)}\]])"`
  - Supported selectors:
    - `@term` / `@term:last` → most recent block in bound/latest session
    - `@term:failed` → most recent block with `status == BlockStatus::Failed` or `exit_code.is_some_and(|c| c != 0)`
    - `@term:pinned` → all blocks with `pinned == true` in bound/latest session (newest-last, capped to `32 KiB` envelope)
    - `@term:<sid>` → recent blocks summary + latest block output of `<sid>`
    - `@term:<sid>#<id>` → full (head-40/tail-160 budgeted) block `<id>` in `<sid>`
    - `@term:<sid>#<id>:L<a>` / `@term:<sid>#<id>:L<a>-<b>` → 1-indexed inclusive line slice of scrubbed plain output
  - **Crucial Ordering in `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs`:**
    1. Run `extract_and_resolve_term_mentions(&params.prompt, state)` **before** `resolve_mentions_with_rules` (`message.rs:118`), replacing `@term...` tokens in the user prompt with compact placeholders (`[terminal citation @term:...]`) while collecting the scrubbed `<terminal_context>` payload separately.
    2. Run `prompt_canonical::canonicalize_prompt` (`message.rs:148`) on the user's prose **before** appending `<terminal_context>`, so compiler diagnostics or stack traces inside terminal output never false-trigger `canonicalize_prompt` prompt-injection heuristics or rewrite terminal lines into `"Objectives: 1. ..."`!
- **`SendToAgent` Backend Resolution (`crates/vox-gui/src/commands/console_a2a.rs`):**
  - Add structured parameters (`session_id: Option<String>`, `block_id: Option<u64>`, `line_start: Option<usize>`, `line_end: Option<usize>`) to `send_to_agent` (or a sibling `send_term_block_to_agent` Tauri command) so `Console.tsx` sends `{ agentId, sessionId, blockId, lineStart, lineEnd, note }` instead of raw DOM text, and `console_a2a.rs` resolves the block from `SessionHost` (`pty::resolve_host()`), runs the 3-stage scrubber (`vox_terminal_core` ANSI+`vox-redact` → `vox_secrets::redact::scrub_resolved_secrets`), and forwards only scrubbed content to `vox_a2a_send`. Even if legacy free-text `body` is passed, scrub `body` through the exact same 3-stage pipeline on the Rust backend!

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| MCP cryptographic tool execution receipts | Custom receipt signing or DB table for `vox_terminal_*` | Existing `issue_dispatch_receipt` + `fulfill_tool_receipt` in `crates/vox-orchestrator-mcp/src/dispatch.rs:393,438-456` | Automatic for every tool registered in `TOOL_REGISTRY` (`contracts/operations/catalog.v1.yaml` → `operations-sync`) [VERIFIED: crates/vox-orchestrator/src/tool_receipt.rs:120-124]. |
| Cross-process HITL approval parking & resolution | Custom terminal approval channel or one-off oneshot map | `state.pending_approvals` (`crates/vox-orchestrator-mcp/src/pending_approvals.rs` + `floor_gate.rs`) | Already wired end-to-end across `vox_pending_approvals`, `vox_resolve_approval`, `orch.list_pending_approvals`, and `orch.resolve_approval` [VERIFIED: crates/vox-orchestrator-mcp/src/server_state.rs:118-121]. |
| Daemon TCP discovery, loopback validation & bearer token framing | New socket client in `vox-orchestrator-mcp` | `vox_orchestrator::orch_daemon::OrchDaemonClient` (`crates/vox-orchestrator/src/orch_daemon/client.rs`) | Already handles `VOX_ORCHESTRATOR_DAEMON_SOCKET`, token auth, `195s` read deadline, and JSON-RPC framing [VERIFIED: crates/vox-orchestrator/src/orch_daemon/client.rs:14-60]. |
| Multi-pattern Clavis secret replacement | Repeated `String::replace` loops per secret | `aho_corasick::AhoCorasick` (`MatchKind::LeftmostLongest`) inside `crates/vox-secrets/src/redact.rs` | Single linear pass, handles overlapping prefixes safely, and already imported in `vox-secrets/src/redact.rs:5` [VERIFIED: crates/vox-secrets/src/redact.rs:5-49]. |
| `ALT_SCREEN` sequence parsing | Ad-hoc regex scan for `\x1b[?1049h` on raw chunks | `Session::is_alt_screen()` backed by `VtShadow` (`alacritty_terminal` `TermMode::ALT_SCREEN`) | Phase 441 already tracks `?1049h`/`?1049l`, `?47h`/`?47l`, and `?1047h`/`?1047l` across split PTY reads [VERIFIED: crates/vox-terminal-core/src/vt_shadow.rs:36-45]. |

## Common Pitfalls

### Pitfall 1: `arch/god_object` (`< 500` Non-Blank LoC) Violations on Existing Terminal Files
- **What goes wrong:** Adding Phase 442 methods directly into `crates/vox-terminal-core/src/host.rs` (already 972 pre-test lines), `session.rs` (563 pre-test lines), `shell_init.rs` (1,004 lines), `pty.rs` in `vox-gui` (452 pre-test lines), or `term_dispatch.rs` (350 pre-test lines) trips `vox audit code`'s `arch/god_object` detector (`hard_max_lines: 500`, `warn_max_lines: 400`, `max_methods: 12`) [VERIFIED: crates/vox-code-audit/src/detectors/god_object.rs:14-21].
- **How to avoid:** Put all new Phase 442 logic in dedicated `< 350`-line sibling modules (`vox-terminal-core/src/{agent_exec.rs,stall_detector.rs}`, `vox-orchestrator/src/{terminal_bridge.rs,oversight/exec_policy.rs}`, `vox-orchestrator-d/src/term_bridge_impl.rs`, `vox-orchestrator-mcp/src/{terminal_schemas.rs,dispatch_domains/terminal_tools.rs,chat_tools/chat/term_mentions.rs}`) and extract `host.rs`'s legacy security code out when replacing `TermSecurityLevel::allows_agent_command`.

### Pitfall 2: `[no-hardcoded-abs-path]` False Positives in `vox-redact` and `exec_policy`
- **What goes wrong:** Writing `"\"/home/"` or `r"/home/[^/\s]+"` (`"` immediately followed by `/home` or `/usr` or `/bin` or `/etc` or `/tmp` or `C:\`) in non-test Rust code fails `cargo run -p vox-arch-check` under `[no-hardcoded-abs-path]`, whose exact regex is `"(/(tmp|usr|etc|var|home|opt|bin|root)\b|[A-Za-z]:\\)` [VERIFIED: docs/src/architecture/layers.toml:529-534].
- **How to avoid:** Start regex string literals with a non-capturing group `r"(?:/(?:Users|home)/[^/\s]+|[A-Za-z]:\\Users\\[^\\\s]+)"` (so `"` is followed by `(?:`, never `/` or `C:\`) and normalize executable basenames in `exec_policy.rs` using `.rsplit(['/', '\\']).next()`. Always verify with `timeout 300s cargo run -p vox-arch-check`.

### Pitfall 3: `vox ci secret-env-guard` Dataflow Heuristic (`secret_dataflow_leak_categories`)
- **What goes wrong:** `run_secret_env_guard` (`guards.rs:824-884`) not only bans raw `std::env::var` on secret names; it also scans every statement outside `crates/vox-secrets/` and fails if `json!(...)`, `format!(...)`, `serde_json::to_value(...)`, or `tracing::*!(...)` references any identifier matching `(api[_-]?key|access[_-]?token|bearer|secret|password|authorization|\.expose\(\))` [VERIFIED: crates/vox-cli-research/src/ci_helpers/guards.rs:831-857].
- **How to avoid:** (1) Implement `scrub_resolved_secrets(text: &str) -> String` inside `crates/vox-secrets/src/redact.rs` so `.expose()` never appears in consumer crates; (2) name prompt-classification variables/fields `credential_prompt` or `auth_prompt` (never `is_password` or `secret_prompt`).

### Pitfall 4: Subshell `( export PAGER=cat; <cmd> )` Destroying Stateful `cwd` and Env
- **What goes wrong:** Wrapping agent commands in `( ... )` to scope `PAGER=cat` executes `cd` and `export` in a forked subshell, silently discarding directory and virtualenv changes after the command exits.
- **How to avoid:** Set and restore the 6 pager vars (`PAGER`, `GIT_PAGER`, `SYSTEMD_PAGER`, `BAT_PAGER`, `MANPAGER`, `GH_PAGER`) in the **current** shell via `__vox_pager_enter` + prompt-hook restore (`zsh`, `bash`, `fish`, `pwsh`, `nu`), keeping `cd`, `export`, and `$?` / `$LASTEXITCODE` 100% intact.

### Pitfall 5: Missing One of the 5 Static MCP Registry / Governance Synchronization Gates
- **What goes wrong:** Adding `vox_terminal_*` only to `catalog.v1.yaml` fails `vox-orchestrator-mcp` unit tests or `vox ci operations-verify` across 5 separate enforcement tests:
  1. `operations_catalog.rs:682-730` brace-scans `crates/vox-orchestrator-mcp/src/input_schemas.rs` for string literals inside `tool_input_schema` (`#[cfg(any())] match name`).
  2. `dispatch.rs:867-935` (`yaml_registry_tools_have_dispatch_match_arms`) `include_str!`-scans `dispatch_domains/*.rs` — new `dispatch_domains/terminal_tools.rs` MUST be added to that `include_str!` list, AND `vox_terminal_exec` / `vox_terminal_send_input` MUST be added to `SKIP_DISPATCH_PROBE` (`dispatch.rs:942-990`) so `every_registry_tool_has_static_dispatch` doesn't park for 300s waiting for HITL approval!
  3. `contracts/mcp/http-read-role-governance.yaml` MUST list `vox_terminal_read` and `vox_terminal_sessions` in sorted order after `vox_task_status` because `operations-verify` enforces exact set equality whenever `http_read_role_eligible: true` [VERIFIED: crates/vox-mcp-registry/src/classification.rs:72-100].
  4. `pending_approvals_tests.rs:559-670` enforces bidirectional parity between `RISK_CLASSES` (`permission_modes.rs`), `contracts/orchestration/permission-modes.v2.yaml`, and `classify_floor` (`oversight/floor.rs`) — including `classify_floor(tool, &json!({})) == row.floor` (`None` / `null` when `vox_terminal_exec` is argument-driven in `SHELL_COMMAND_TOOLS`!).
  5. `dispatch_timeout.rs:27` defaults MCP tool calls to `120s`; since `vox_terminal_exec` allows `wait_ms` up to `120_000`, register `"vox_terminal_exec" => Duration::from_secs(150)` in `EXPLICIT_TIMEOUTS` (still well under `ORCH_CLIENT_READ_DEADLINE = 195s` in `client.rs:14`) so a `120_000ms` wait returns a clean `status: "running"` JSON envelope rather than an outer timeout error.

### Pitfall 6: `ServerState` Cloned Before `TermDispatch` in `vox-orchestrator-d` Boot
- **What goes wrong:** In `crates/vox-orchestrator-d/src/daemon_steps/run.rs:192-207`, `step_agent_fleet` clones `ServerState` into `McpToolDispatcher` **before** `vox_orchestrator_d.rs:288-294` creates `LocalHost`. Storing `Option<Arc<dyn TerminalBridge>>` by value on `ServerState` leaves `step_agent_fleet`'s clone `None`!
- **How to avoid:** Store `Arc<PrRwLock<Option<Arc<dyn TerminalBridge>>>>` on `ServerState` (matching `boot_report: Arc<PrRwLock<BootReport>>` at `server_state.rs:143`) so `state.set_terminal_bridge(...)` updates every pre-existing `ServerState` clone atomically.

### Pitfall 7: File `MENTION_RE` & `canonicalize_prompt` Corrupting `@term` Citations
- **What goes wrong:** `MENTION_RE` in `mentions.rs:9` (`@([A-Za-z0-9_.:/\\-]+)`) stops at `#`, turning `@term:term-1#3:L10-20` into a broken `@term:term-1` file lookup plus orphan `#3:L10-20` text; and expanding raw terminal logs *before* `prompt_canonical::canonicalize_prompt` (`message.rs:148`) runs injection/objective rewriting over compiler output.
- **How to avoid:** Extract `@term...` tokens **before** `resolve_mentions_with_rules`, canonicalize only the user's prompt text, and append the scrubbed `<terminal_context>` envelope immediately after `canonicalize_prompt`.

### Pitfall 8: Scrubbing Order — Redacting Before Stripping ANSI Sequences
- **What goes wrong:** Colored terminal output often embeds SGR resets inside URLs, paths, or tokens (`https://user:\x1b[33ms3cr3t\x1b[0m@db.internal` or `/Users/\x1b[1mbrbrainerd\x1b[0m`). Running `vox-redact` or Clavis `AhoCorasick` before stripping ANSI misses the secret, and stripping ANSI later exposes the raw secret to the LLM!
- **How to avoid:** Enforce a single helper function `scrub_terminal_output(raw: &str)` that **always** executes: (1) `normalize_cr_overwrites` + full ECMA-48 `strip_ansi`, (2) `vox_redact::redact_owned`, (3) `vox_secrets::redact::scrub_resolved_secrets`.

## Code Examples

### 1. `[no-hardcoded-abs-path]`-Clean Home/PEM/URL/Token Redaction (`crates/vox-redact/src/lib.rs`)
```rust
// Passes `vox-arch-check` `[no-hardcoded-abs-path]` (`"(/(tmp|usr|etc|var|home|opt|bin|root)\b|[A-Za-z]:\\)`)
// because `"` is followed by `(?:`, never `/` or `C:\`.
fn re_home() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?:/(?:Users|home)/[^/\s]+|[A-Za-z]:\\Users\\[^\\\s]+)")
            .expect("valid home regex")
    })
}

fn re_pem() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?s)-----BEGIN [A-Z0-9 _-]+PRIVATE KEY-----.*?-----END [A-Z0-9 _-]+PRIVATE KEY-----")
            .expect("valid pem regex")
    })
}

fn re_url_cred() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"([a-zA-Z][a-zA-Z0-9+.-]*://[^/\s:@]+):([^@\s/]+)@")
            .expect("valid url credential regex")
    })
}
```

### 2. Zero-`secret-env-guard`-Violation Clavis Exact-Value Scrubber (`crates/vox-secrets/src/redact.rs`)
```rust
/// Scrub exact occurrences of currently resolved Clavis secrets (`len >= 6`) with `[REDACTED:CLAVIS]`.
/// Lives inside `vox-secrets` so `.expose()` never crosses crate boundaries (`secret-env-guard`).
#[must_use]
pub fn scrub_resolved_secrets(text: &str) -> String {
    if text.len() < 6 {
        return text.to_string();
    }
    let mut needles: Vec<String> = crate::all_specs()
        .iter()
        .filter(|spec| {
            !spec.metadata.taxonomy_class.is_config_only()
                && !matches!(
                    spec.metadata.material_kind,
                    crate::SecretMaterialKind::ConfigValue | crate::SecretMaterialKind::EndpointUrl
                )
        })
        .filter_map(|spec| {
            let resolved = crate::resolve_secret(spec.id);
            resolved.expose().map(str::trim).filter(|v| v.len() >= 6).map(str::to_owned)
        })
        .collect();
    needles.sort();
    needles.dedup();
    if needles.is_empty() {
        return text.to_string();
    }
    let Ok(ac) = aho_corasick::AhoCorasick::builder()
        .match_kind(aho_corasick::MatchKind::LeftmostLongest)
        .build(&needles)
    else {
        return text.to_string();
    };
    let replacements = vec!["[REDACTED:CLAVIS]"; needles.len()];
    ac.replace_all(text, &replacements)
}
```

### 3. Sorted `contracts/operations/catalog.v1.yaml` Entries (Insert Between `term` and `test` at Line 35681)
```yaml
- id: terminal.exec
  title: Execute Stateful Terminal Command
  summary: Execute a shell command in a shared stateful SessionHost PTY with AST policy gating, pager neutralization, and stall detection.
  capability_group: AppExperience
  intent_tags:
  - terminal
  - shell
  - pty
  product_lane: ai
  status: active
  mcp:
    name: vox_terminal_exec
    http_read_role_eligible: false
    exposure: operator
    side_effect_class: spawns_process
    requires: []
- id: terminal.read
  title: Read Terminal Session Output
  summary: Read ANSI-stripped, redacted block output, line ranges, or after_seq streaming deltas from a SessionHost session.
  capability_group: AppExperience
  intent_tags:
  - terminal
  - logs
  product_lane: ai
  status: active
  mcp:
    name: vox_terminal_read
    http_read_role_eligible: true
    exposure: public
    side_effect_class: read_only
    requires: []
- id: terminal.send_input
  title: Send Stdin to Awaiting Terminal Block
  summary: Write interactive stdin or control signal bytes to a running or awaiting_input SessionHost block (fail-closed on credential prompts).
  capability_group: AppExperience
  intent_tags:
  - terminal
  - interactive
  product_lane: ai
  status: active
  mcp:
    name: vox_terminal_send_input
    http_read_role_eligible: false
    exposure: operator
    side_effect_class: spawns_process
    requires: []
- id: terminal.sessions
  title: List Shared Terminal Sessions
  summary: List live shared SessionHost terminal sessions, cwd, security level, control mode, and recent block statuses.
  capability_group: AppExperience
  intent_tags:
  - terminal
  - sessions
  product_lane: ai
  status: active
  mcp:
    name: vox_terminal_sessions
    http_read_role_eligible: true
    exposure: public
    side_effect_class: read_only
    requires: []
```

## State of the Art

| Legacy Pattern in Repo | Phase 442 Target Pattern | Impact |
|------------------------|--------------------------|--------|
| Stateless subprocess shell tools or blind `1_500 ms` fire-and-forget `term.agent_command` (`host.rs:859`) | Stateful `vox_terminal_exec` (`wait_ms = 10_000`, `0..=120_000`, `background`, `Ctrl+B` `term.promote_bg`) + `vox_terminal_read` (`after_seq` / `L<a-b>`) | Preserves `cd`/`export`/venvs across turns and supports long dev servers/builds without deadlocking or killing PTY jobs. |
| Blocking `less` pagers & silent `[y/N]` / `Password:` hangs | Scoped 5-shell pager guard + `VtShadow` `ALT_SCREEN` / `1_500 ms` quiescence `BlockStatus::AwaitingInput` (fail-closed on credential prompts) | Zero hung agent turns; secrets stay strictly human-entered. |
| Three split shell guards (`check_terminal.rs`, `floor.rs`, `host.rs` whitespace split) | Single `vox_orchestrator::oversight::exec_policy` + synced `term.security` / `term.control` + `PendingApprovals` HITL | Eliminates `$()`, backtick, redirection, `pwsh -EncodedCommand`, and `find -exec` bypasses everywhere. |
| Browser-side raw DOM `SendToAgent` (`Console.tsx:250`) & partial `strip_ansi` | 3-stage backend scrubber (full ECMA-48 strip + `\r` normalize → upgraded `vox-redact` → Clavis `AhoCorasick`) + `@term` citations | Prevents PII, PEM keys, URL passwords, API tokens, and Clavis secrets from ever entering an LLM context window. |

## Assumptions Log

| # | Claim / Design Choice | Section | Risk if Wrong | Mitigation |
|---|-----------------------|---------|---------------|------------|
| A1 | `vox_terminal_read` and `vox_terminal_sessions` set `http_read_role_eligible: true` (added to `contracts/mcp/http-read-role-governance.yaml`) so `"plan"` mode (`tool_selection.rs:222`) can read terminal output while `vox_terminal_exec` / `vox_terminal_send_input` remain `false`. | Pattern 1 / Catalog | Low — `vox_terminal_read` returns strictly 3-stage redacted output and mutates zero state. | Verified against `crates/vox-mcp-registry/src/classification.rs:72-100` and `tool_selection.rs:222`. |
| A2 | `vox_terminal_exec` uses parameter name `"command"` (with optional alias `"cmd"` normalized before floor check) so `oversight/floor.rs:189` (`args.get("command")`) works identically to `bash` / `run_command`. | Pattern 3 | None — matches existing MCP schema conventions. | Enforce `"command"` as the canonical schema field in `terminal_schemas.rs`. |

## Open Questions (RESOLVED)

1. **How do `vox-gui` and `vox-orchestrator-d` run both `vox-redact` and Clavis scrubbing without adding `vox-redact` to their `Cargo.toml`?**
   - **Resolution:** `vox-terminal-core` (`L3`) already depends on `vox-redact` (`L0`), and both `vox-gui` (`L5`) and `vox-orchestrator-d` (`L5`) already depend on `vox-terminal-core` + `vox-secrets` (`L1`). Exporting `scrub_ansi_and_redact(raw: &str) -> String` from `vox_terminal_core::corpus::redact` and `scrub_resolved_secrets(text: &str) -> String` from `vox_secrets::redact` gives every consumer the full 3-stage pipeline with **zero `Cargo.toml` changes**.
2. **How does `vox-orchestrator::oversight::exec_policy` parse POSIX and PowerShell without adding `vox-container-types` or `tree-sitter` edges?**
   - **Resolution:** Implement a self-contained ~280-LoC quote-aware POSIX + PowerShell lexical/pipeline scanner in `crates/vox-orchestrator/src/oversight/exec_policy.rs` using `serde_yaml` and `regex` (both already in `vox-orchestrator/Cargo.toml`).

## Environment Availability

| Dependency | Required By | Available | Notes |
|------------|------------|-----------|-------|
| Rust workspace toolchain + `cargo` broker shim | All crates under test | ✓ | Scope every test/check with `timeout 300s cargo ... -p <crate>`. |
| `zsh`, `bash`, `sh` | `vox-terminal-core` PTY & pager/stall integration tests | ✓ | Present on macOS/Linux host; `pwsh`/`nu`/`fish` unit-tested via pure-Rust shim & `exec_policy` tests when binary absent. |
| `node` + `gsd-tools.cjs` | Planning / phase tools (`--ws axis-warp-terminal-v75`) | ✓ | Verified on host. |

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` / `#[tokio::test]` (`cargo test`) + `vox-arch-check` + `vox-cli ci` contract gates |
| Config file | Workspace `Cargo.toml` (note target flags below: `vox-gui` is bin-only so `--lib` fails on `vox-gui`; `vox-orchestrator-d` has both `[lib]` and `[[bin]]`) |
| Quick run command | `timeout 300s cargo test -p vox-terminal-core --lib` |
| Full suite command | Run each per-crate command below sequentially (one command per call; never `cargo test --workspace`) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | Target / File Location |
|--------|----------|-----------|-------------------|------------------------|
| `TERM-HARNESS-01` | `TerminalBridge` foreground `wait_ms`, `background: true`, `Ctrl+B` `promote_bg` non-killing return, `chat_thread_id -> session_id` sticky binding, `vox_terminal_*` MCP schema/dispatch/receipts | Unit + Integration | `timeout 300s cargo test -p vox-orchestrator --lib terminal_bridge` | `crates/vox-orchestrator/src/terminal_bridge.rs` |
| `TERM-HARNESS-01` | Daemon `term.{agent_exec,promote_bg,pin,control,security,context}` JSON-RPC + `LocalHostTerminalBridge` | Unit + Integration | `timeout 300s cargo test -p vox-orchestrator-d --lib term_` | `crates/vox-orchestrator-d/src/{term_dispatch.rs,term_bridge_impl.rs}` |
| `TERM-HARNESS-01` | MCP static registry parity (`input_schemas`, `dispatch`, `SKIP_DISPATCH_PROBE`, `permission_modes`, `PendingApprovals` HITL) | Integration | `timeout 300s cargo test -p vox-orchestrator-mcp --lib terminal` | `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs` |
| `TERM-HARNESS-01` | Catalog + read-role governance + generated `TOOL_REGISTRY` parity | Contract Gate | `timeout 300s cargo run -p vox-cli -- ci operations-verify` | `contracts/operations/catalog.v1.yaml`, `contracts/mcp/http-read-role-governance.yaml` |
| `TERM-HARNESS-02` | Scoped 5-shell pager guard (`PAGER=cat`, `GIT_PAGER=cat`, etc. without breaking stateful `cd`/`export`), `ALT_SCREEN` & `1_500 ms` quiescence `AwaitingInput`, fail-closed credential `send_input` | Unit + PTY Integration | `timeout 300s cargo test -p vox-terminal-core --lib` | `crates/vox-terminal-core/src/{agent_exec.rs,stall_detector.rs}` |
| `TERM-HARNESS-03` | AST/lexical `ExecPolicyV1` + `classify_floor` blocking `$()`, backticks, `L1` redirects, `pwsh -EncodedCommand`, `bash -c`, `find -exec`/`-delete`, `env`/`xargs` wrappers; `Manual`/`CoPilot`/`Autopilot` gating | Unit + Contract | `timeout 300s cargo test -p vox-orchestrator --lib exec_policy` | `crates/vox-orchestrator/src/oversight/{exec_policy.rs,floor.rs}` |
| `TERM-HARNESS-03` | `vox shell check` delegation + `exec-policy.v1.yaml` smoke/reject contract parity + `pending_approvals_tests` | Unit + Contract Gate | `timeout 300s cargo test -p vox-cli-research --lib check_terminal` and `timeout 300s cargo run -p vox-cli -- ci exec-policy-contract` and `timeout 300s cargo test -p vox-orchestrator-mcp --test pending_approvals_tests` | `crates/vox-cli-research/src/check_terminal.rs`, `crates/vox-orchestrator-mcp/tests/pending_approvals_tests.rs` |
| `TERM-HARNESS-04` | 3-stage scrubber: full ECMA-48 ANSI + `\r` strip → upgraded `vox-redact` (macOS `/Users/`, `/home/`, `C:\Users\`, PEM, URL creds, `AIza`/`hf_`/`npm_`/`ghp_`/`sk-`) → Clavis `scrub_resolved_secrets` (`>= 6` chars → `[REDACTED:CLAVIS]`) | Unit | `timeout 300s cargo test -p vox-redact --lib` and `timeout 300s cargo test -p vox-secrets --lib redact` | `crates/vox-redact/src/lib.rs`, `crates/vox-secrets/src/redact.rs` |
| `TERM-HARNESS-04` | `@term`, `@term:last`, `@term:failed`, `@term:pinned`, `@term:<sid>#<id>:L<a-b>` pre-canonicalizer parsing, head-40/tail-160 (`16 KiB` / `32 KiB`) truncation, and backend `SendToAgent` (`vox-gui` bin target!) | Unit + Integration | `timeout 300s cargo test -p vox-orchestrator-mcp --lib term_mentions` and `timeout 300s cargo check -p vox-gui --bins` | `crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs`, `crates/vox-gui/src/commands/{pty.rs,console_a2a.rs}` |
| Cross-Cutting | Zero crate-edge drift, zero `[no-hardcoded-abs-path]`, zero `god_object`, zero `secret-env-guard` violations | Architecture & Security Gates | `timeout 300s cargo run -p vox-arch-check` and `timeout 300s cargo run -p vox-cli -- ci secret-env-guard` | Workspace-wide |

### Wave 0 / Pre-Implementation Notes
- **Target flag accuracy:** `vox-gui` has **no `[lib]` target** (`src/main.rs` binary only) — always use `--bins` (or omit `--lib`) when checking/testing `vox-gui`. `vox-orchestrator-d` has both `[lib]` (`src/lib.rs`) and `[[bin]]` (`vox-orchestrator-d`), so verify binary wiring (`src/bin/vox_orchestrator_d.rs`) with `timeout 300s cargo check -p vox-orchestrator-d --bins` in addition to `--lib` tests.

## Security Domain

### Applicable ASVS Categories
| ASVS Category | Applies | Control Mapped to Phase 442 |
|---------------|---------|-----------------------------|
| V5 Input Validation / Command Injection | Yes | `vox_orchestrator::oversight::exec_policy` rejects/unwraps `$()`, backticks, process substitution, redirections (`L1`), and wrapper binaries (`pwsh -EncodedCommand`, `bash -c`, `find -exec`/`-delete`, `env`, `xargs`) before any agent string touches a PTY. |
| V4 Access Control & HITL | Yes | Loopback-only `term.*` daemon guard (`vox_orchestrator_d.rs:288-296`), bearer-token RPC auth (`term_dispatch.rs`), `ControlMode` (`Manual` / `CoPilot` / `Autopilot`) × `TermSecurityLevel` (`L1–L5`), and inviolable `classify_floor` → `PendingApprovals` HITL gate. |
| V2 / V6 Credential & Secret Protection | Yes | Fail-closed `vox_terminal_send_input` when `Block.credential_prompt == true` (agents can never inject passwords/sudo/passphrases), plus mandatory 3-stage ANSI → `vox-redact` → Clavis exact-value (`[REDACTED:CLAVIS]`) scrubbing before any LLM tool result, `@term` citation, or `SendToAgent` message. |
| V10 Auditability / Non-Repudiation | Yes | Every `vox_terminal_*` tool call automatically receives a signed `ToolReceipt` via `dispatch.rs` (`issue_dispatch_receipt` + `fulfill_tool_receipt`) and tags PTY blocks with `BlockOrigin::Agent`. |

## Sources

### Primary (HIGH Confidence — Direct Codebase Verification)
- `docs/src/architecture/layers.toml:529-554` — `[VERIFIED]` verbatim forbidden pattern `"(/(tmp|usr|etc|var|home|opt|bin|root)\b|[A-Za-z]:\\)` with `exempt_tests = true`.
- `crates/vox-code-audit/src/detectors/god_object.rs:13-22` — `[VERIFIED]` `hard_max_lines: 500`, `warn_max_lines: 400`, `max_methods: 12`.
- `crates/vox-cli-research/src/ci_helpers/guards.rs:824-884, 943-990` — `[VERIFIED]` `run_secret_env_guard` and `secret_dataflow_leak_categories` (`secret_word_re` matching `(api[_-]?key|access[_-]?token|bearer|secret|password|authorization|\.expose\(\))` inside serialize/log sinks outside `crates/vox-secrets/`).
- `contracts/operations/catalog.v1.yaml:35656-35685` & `contracts/mcp/http-read-role-governance.yaml:1-47` — `[VERIFIED]` exact sorted insertion slot between `id: term` and `id: test` and read-role governance allowlist requirement (`crates/vox-mcp-registry/src/classification.rs:72-100`).
- `crates/vox-orchestrator-mcp/src/{server_state.rs:60-175, input_schemas.rs:29-60, dispatch.rs:867-1004, dispatch_timeout.rs:20-65, permission_modes.rs:56-106}` & `crates/vox-orchestrator-mcp/tests/pending_approvals_tests.rs:559-670` — `[VERIFIED]` `ServerState` interior mutability, static schema/dispatch scanners, `SKIP_DISPATCH_PROBE`, `RISK_CLASSES` ↔ `permission-modes.v2.yaml` ↔ `classify_floor` parity invariants.
- `crates/vox-orchestrator-d/src/{daemon_steps/run.rs:175-208, term_dispatch.rs:1-350, bin/vox_orchestrator_d.rs:272-308}` — `[VERIFIED]` daemon boot order (`step_agent_fleet` clones `ServerState` before `TermDispatch` creation) and existing `term.*` JSON-RPC handlers.
- `crates/vox-terminal-core/src/{block.rs:16-161, session.rs:189-205, 465-510, host.rs:42-310, 750-910, shell_init.rs, vt_shadow.rs:1-60}` — `[VERIFIED]` `BlockStatus`, `strip_ansi`, `normalize_cr_overwrites`, `begin_pty_submission`, `VtShadow::is_alt_screen`, and `TermSecurityLevel` / `ControlMode`.
- `crates/vox-redact/src/lib.rs:34-80` & `crates/vox-secrets/src/{redact.rs:1-65, spec/mod.rs:24-32, spec/types.rs:33-35, 102-115}` — `[VERIFIED]` regex redaction patterns, `all_specs()`, `resolve_secret()`, and `aho-corasick` dependency.
- `crates/vox-orchestrator-mcp/src/chat_tools/chat/{mentions.rs:1-90, message.rs:115-160, agent_loop/ast_slice_and_preflight.rs:23-36}` & `crates/vox-gui/src/commands/{pty.rs, console_a2a.rs}` — `[VERIFIED]` `MENTION_RE` `#` boundary behavior, `canonicalize_prompt` ordering, `FROZEN_PREFIX_CORE_TOOLS`, and `SendToAgent` IPC path.

## Metadata

**Confidence breakdown:**
- Standard stack & zero-edge crate topology: **HIGH** — every `Cargo.toml`, `layers.toml`, and `crate-edges.allow.v1.json` edge verified directly.
- Architecture & MCP/Daemon integration: **HIGH** — all 5 static MCP test scanners, `ServerState` clone ordering, and `PendingApprovals` parity tests inspected line-by-line.
- Pitfalls & CI guards: **HIGH** — uncovered exact regexes for `[no-hardcoded-abs-path]`, `god_object`, and `secret-env-guard`'s `secret_dataflow_leak_categories`.

**Research date:** 2026-11-14
**Valid until:** 2026-12-14
