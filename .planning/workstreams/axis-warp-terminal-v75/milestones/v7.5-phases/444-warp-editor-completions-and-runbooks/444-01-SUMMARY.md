---
phase: 444-warp-editor-completions-and-runbooks
plan: 01
subsystem: terminal
tags: [rust, tauri, typescript, terminal, history, completions, runbooks, voxify, osc633, redact]

requires:
  - phase: 443-unified-terminal-pane-webgl-and-split-panes
    provides: "SessionHost / DaemonHost RPC transport, LocalHost PTY lifecycle, and Console <-> Chat terminal bridge"
provides:
  - "HistoryStore in vox-terminal-core with pre-trim HISTCONTROL=ignorespace skip, 3-stage redaction (ANSI strip -> Clavis scrubber closure -> vox-redact), 0o700/0o600 atomic JSONL persistence, consecutive dedup, 10k retention, and subsequence + recency + cwd-affinity fuzzy ranking"
  - "CompletionEngine + completions_specs in vox-terminal-core providing <50ms zero-subprocess completions across $PATH binaries (30s TTL cache), shell builtins, traversal-safe cwd-relative paths, pure-Rust .git branches/tags/remotes (including linked git worktrees), and curated CLI flag/subcommand specs"
  - "Workspace runbook scanner (scan_workspace_runbooks) with symlink-containment canonicalization across scripts/*.vox, .vox/runbooks/*.{vox,md}, and @runbook-tagged docs/src/how-to/*.md plus {{param}} extraction"
  - "Closed-allowlist base64 OSC 633 + OSC 7 subshell hook builder (build_voxify_payload + detect_subshell_command) wired through SessionHost::voxify_subshell and TermDispatch term.voxify gated by refuse_agent_origin"
  - "Modular <430 LoC commands/pty.rs + pty/warp.rs + pty/tests.rs exposing 6 #[tauri::command] endpoints inheriting ModuleClass::Host, paired with typed transport.ts safeInvoke wrappers and terminalBridge.ts Runbook launch pub/sub"
affects:
  - 444-02-PLAN
  - 444-03-PLAN
  - 444-04-PLAN
  - 445-terminal-agentic-mode-and-mens-flywheel

actuals:
  tokens: 30500
  tasks: 3
  commits: 0

tech-stack:
  added: []
  patterns:
    - "Caller-injected secret scrubber closure (&dyn Fn(&str) -> String) keeping vox-terminal-core Layer 2 free of vox-secrets / vox-config / vox-repository edges while wiring scrub_resolved_secrets in vox-gui"
    - "Pure-Rust filesystem & .git completion engine (zero Command::new subprocesses) supporting both standard .git directories and linked git worktree gitdir:/commondir indirection"
    - "Submodule split (commands/pty.rs + commands/pty/warp.rs + commands/pty/tests.rs) keeping #[tauri::command] entrypoints in pty.rs so gui_command_backends automatically inherits ModuleClass::Host with zero YAML/fixture churn"

key-files:
  created:
    - crates/vox-terminal-core/src/history.rs
    - crates/vox-terminal-core/src/completions.rs
    - crates/vox-terminal-core/src/completions_specs.rs
    - crates/vox-terminal-core/src/runbooks.rs
    - crates/vox-terminal-core/src/voxify.rs
    - crates/vox-gui/src/commands/pty/warp.rs
    - crates/vox-gui/src/commands/pty/tests.rs
  modified:
    - crates/vox-terminal-core/src/pty.rs
    - crates/vox-terminal-core/src/host.rs
    - crates/vox-terminal-core/src/daemon_host.rs
    - crates/vox-terminal-core/src/lib.rs
    - docs/src/reference/cli/crate-api/vox-terminal-core.generated.md
    - crates/vox-orchestrator-d/src/term_dispatch.rs
    - crates/vox-gui/src/commands/pty.rs
    - crates/vox-gui/src/main.rs
    - crates/vox-gui/ui/src/transport.ts
    - crates/vox-gui/ui/src/lib/terminalBridge.ts
    - crates/vox-gui/ui/src/transport.console.test.ts

key-decisions:
  - "Checked leading space/tab BEFORE trimming in HistoryStore::record_with_scrubber and injected HISTCONTROL=ignorespace + HIST_IGNORE_SPACE=1 in apply_pty_history_env so space-prefixed commands never persist to either shell or Vox JSONL history."
  - "Kept vox-terminal-core dependency graph completely unchanged by passing &vox_secrets::scrub_resolved_secrets, vox_config::paths::user_home_dir, and vox_repository::discover_repository_or_fallback from crates/vox-gui/src/commands/pty/warp.rs."
  - "Moved spawn_pty_forwarder and spawn_term_forwarder into commands/pty/warp.rs and tests into commands/pty/tests.rs so commands/pty.rs sits at 402 non-blank LoC (<430 budget) while retaining all #[tauri::command] attributes directly in pty.rs to inherit ModuleClass::Host."
  - "Gated term.voxify in crates/vox-orchestrator-d/src/term_dispatch.rs with Self::refuse_agent_origin(params, VOXIFY)? before reading session_id so MCP/agent origins cannot inject shell hook payloads into interactive subshells."

patterns-established:
  - "3-stage terminal persistence scrubbing: strip_ansi_and_normalize -> Clavis scrub_resolved_secrets closure -> vox_redact::redact_owned prior to 0o600 atomic tmp+rename JSONL flush."
  - "Symlink-safe workspace discovery: canonicalize workspace root first, then canonicalize every candidate file and enforce can_file.starts_with(&can_root) before reading."

requirements-completed:
  - TERM-WARP-02
  - TERM-WARP-03
  - TERM-WARP-04

coverage:
  - id: D1
    description: "HistoryStore (HISTCONTROL=ignorespace, 3-stage Clavis+vox-redact scrubbing, 0o700/0o600 atomic JSONL, subsequence+recency+cwd ranking) and PTY history env injection"
    requirement: TERM-WARP-02
    verification:
      - kind: unit
        ref: "crates/vox-terminal-core/src/history.rs"
        status: pass
      - kind: unit
        ref: "crates/vox-terminal-core/src/pty.rs"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/src/commands/pty/tests.rs"
        status: pass
    human_judgment: false
  - id: D2
    description: "Zero-subprocess CompletionEngine + curated CLI specs ($PATH TTL cache, shell builtins, traversal-safe cwd paths, pure-Rust .git + worktree refs, flags/subcommands in <50ms)"
    requirement: TERM-WARP-03
    verification:
      - kind: unit
        ref: "crates/vox-terminal-core/src/completions.rs"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/src/commands/pty/tests.rs"
        status: pass
    human_judgment: false
  - id: D3
    description: "Workspace runbook scanner (scripts/*.vox, .vox/runbooks/*.{vox,md}, @runbook docs/src/how-to/*.md), closed-allowlist base64 OSC 633+OSC 7 subshell Voxify via SessionHost + TermDispatch refuse_agent_origin, 6 Tauri Host IPC commands, transport.ts wrappers, and terminalBridge.ts Runbook pub/sub"
    requirement: TERM-WARP-04
    verification:
      - kind: unit
        ref: "crates/vox-terminal-core/src/runbooks.rs"
        status: pass
      - kind: unit
        ref: "crates/vox-terminal-core/src/voxify.rs"
        status: pass
      - kind: unit
        ref: "crates/vox-orchestrator-d/src/term_dispatch.rs"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/transport.console.test.ts"
        status: pass
    human_judgment: false

duration: 75min
completed: 2026-11-15
status: complete
---

# Phase 444 Plan 01: Warp Terminal Backend Engines, Subshell Voxify & IPC Contracts Summary

**Redacted `HistoryStore`, zero-subprocess `< 50 ms` `CompletionEngine`, symlink-safe workspace Runbook scanner, base64 `OSC 633 + OSC 7` subshell `Voxify` RPC, `< 430` LoC `commands/pty.rs` Tauri endpoints, and typed `transport.ts` + `terminalBridge.ts` contracts (`TERM-WARP-02`, `TERM-WARP-03`, `TERM-WARP-04`).**

## Performance

- **Duration:** 75 min
- **Started:** 2026-11-15T13:42:29Z
- **Completed:** 2026-11-15T14:57:30Z
- **Tasks:** 3
- **Files modified:** 18 (7 created, 11 modified)

## Accomplishments

- **Task 1 (`HistoryStore`, `CompletionEngine`, PTY history env):**
  - Built `crates/vox-terminal-core/src/history.rs` (`399` non-blank LoC) enforcing pre-trim `HISTCONTROL=ignorespace` (`' '` / `'\t'`), 3-stage secret scrubbing (`strip_ansi_and_normalize` -> caller Clavis scrubber -> `vox_redact::redact_owned`), `0o700` dir / `0o600` atomic `.tmp` rename JSONL persistence, consecutive command deduplication, `10,000`-entry ring cap, and subsequence + recency + `cwd`-affinity fuzzy search.
  - Updated `crates/vox-terminal-core/src/pty.rs` (`apply_pty_history_env`) to inject `HISTCONTROL=ignorespace` and `HIST_IGNORE_SPACE=1` on spawned interactive shells without overriding explicit caller overrides.
  - Built `crates/vox-terminal-core/src/completions.rs` (`449` non-blank LoC) and `completions_specs.rs` (`422` non-blank LoC) delivering `< 50 ms` zero-subprocess completions across `$PATH` executables (`30s` TTL cache), shell builtins, traversal-safe `cwd`-relative paths (dot-entries gated on leading `.` and directories suffixed with `/`), pure-Rust `.git` branches/tags/remotes (`packed-refs` + loose refs + linked git worktree `gitdir:`/`commondir` support), and curated CLI subcommand/flag specs (`vox`, `git`, `cargo`, `pnpm`, `npm`, `docker`, `kubectl`, `gh`, `rg`, `pytest`, `python`, `nu`, `pwsh`).
- **Task 2 (`runbooks`, `voxify`, `SessionHost::voxify_subshell`, `term.voxify` dispatch):**
  - Built `crates/vox-terminal-core/src/runbooks.rs` (`441` non-blank LoC) scanning canonicalized `scripts/*.vox` (depth 2), `.vox/runbooks/*.{vox,md}` (depth 2), and `@runbook`-tagged `docs/src/how-to/*.md` (depth 1, `64 KiB` cap), rejecting symlinks escaping `workspace_root`, and deduplicating `@param` / frontmatter / `{{ident}}` parameters.
  - Built `crates/vox-terminal-core/src/voxify.rs` (`380` non-blank LoC) implementing `VoxifyShellKind` (`Bash`, `Zsh`, `Fish`, `Pwsh`, `Nu`), `detect_subshell_command`, and closed-allowlist base64 `stty -echo` `OSC 633` + `OSC 7` hook injection (`build_voxify_payload`).
  - Wired `voxify_subshell` across `LocalHost`, `SessionHost`, `DaemonHost` (`VOXIFY = "term.voxify"`), and `crates/vox-orchestrator-d/src/term_dispatch.rs` (enforcing `Self::refuse_agent_origin(params, VOXIFY)?`), re-exported the new public types in `crates/vox-terminal-core/src/lib.rs`, and synced `docs/src/reference/cli/crate-api/vox-terminal-core.generated.md`.
- **Task 3 (`commands/pty` split + 6 Tauri IPC commands + `transport.ts` + `terminalBridge.ts`):**
  - Split `crates/vox-gui/src/commands/pty.rs` down to `402` non-blank LoC (`< 430` budget) with `commands/pty/warp.rs` (`361` non-blank LoC) and `commands/pty/tests.rs` (`260` non-blank LoC), exposing `term_history_record`, `term_history_query`, `term_history_clear`, `term_complete`, `term_runbooks_list`, and `term_session_voxify` via `tokio::task::spawn_blocking` and `PtyWarpState` in `crates/vox-gui/src/main.rs` while inheriting `ModuleClass::Host` with zero edits to `command-backends.v1.yaml` or `remote-gate-mesh-console.json`.
  - Added typed DTOs, `safeInvoke` wrappers, and `VoxTransport` methods in `crates/vox-gui/ui/src/transport.ts`, `RunbookLaunchRequest` pub/sub (`requestOpenRunbook`, `consumePendingOpenRunbook`, `onOpenRunbook`) in `crates/vox-gui/ui/src/lib/terminalBridge.ts`, and unit tests in `crates/vox-gui/ui/src/transport.console.test.ts`.

## Task Commits

Changes left uncommitted in working tree per orchestrator git-safety instructions (`commits: 0`):

1. **Task 1: Redacted `HistoryStore`, PTY `HISTCONTROL=ignorespace`, and zero-subprocess `CompletionEngine`** — uncommitted (`vox-terminal-core`)
2. **Task 2: Workspace `runbooks` scanner, base64 `voxify` payload, `SessionHost::voxify_subshell`, and `term.voxify` RPC** — uncommitted (`vox-terminal-core`, `vox-orchestrator-d`, `vox-terminal-core.generated.md`)
3. **Task 3: Modular `commands/pty` (`< 430` LoC) Tauri endpoints, `transport.ts` wrappers, and `terminalBridge.ts` Runbook pub/sub** — uncommitted (`vox-gui`, `transport.ts`, `terminalBridge.ts`, `transport.console.test.ts`)

## Files Created/Modified

- `crates/vox-terminal-core/src/history.rs` — `HistoryEntry`, `HistoryStore` (`open`, `in_memory`, `record_with_scrubber`, `query`, `clear`), 3-stage redaction, `0o700`/`0o600` atomic JSONL persistence (`399` non-blank LoC)
- `crates/vox-terminal-core/src/completions.rs` — `CompletionEngine`, `TermCompletionRequest`, `TermCompletionItem`, `TermCompletionKind`, zero-subprocess path/git/PATH/spec completion (`449` non-blank LoC)
- `crates/vox-terminal-core/src/completions_specs.rs` — Curated CLI subcommand/flag specs, `$PATH` executable scanner, and pure-Rust `.git` + worktree ref reader (`422` non-blank LoC)
- `crates/vox-terminal-core/src/runbooks.rs` — `RunbookDescriptor`, `RunbookKind`, `RunbookParam`, symlink-contained `scan_workspace_runbooks` (`441` non-blank LoC)
- `crates/vox-terminal-core/src/voxify.rs` — `VoxifyShellKind`, `detect_subshell_command`, `build_voxify_payload` (`380` non-blank LoC)
- `crates/vox-terminal-core/src/pty.rs` — `apply_pty_history_env` injecting `HISTCONTROL=ignorespace` and `HIST_IGNORE_SPACE=1`
- `crates/vox-terminal-core/src/host.rs` — `LocalHost::voxify_subshell` and `SessionHost::voxify_subshell`
- `crates/vox-terminal-core/src/daemon_host.rs` — `VOXIFY = "term.voxify"` and `DaemonHost::voxify_subshell`
- `crates/vox-terminal-core/src/lib.rs` — Module declarations and crate root re-exports
- `docs/src/reference/cli/crate-api/vox-terminal-core.generated.md` — Synced public crate API table via `./target/debug/vox ci crate-api-sync --write`
- `crates/vox-orchestrator-d/src/term_dispatch.rs` — `term.voxify` dispatch arm gated by `Self::refuse_agent_origin` + unit tests
- `crates/vox-gui/src/commands/pty.rs` — Modularized Tauri PTY command surface + 6 `term_*` `#[tauri::command]` wrappers (`402` non-blank LoC)
- `crates/vox-gui/src/commands/pty/warp.rs` — `PtyWarpState`, Clavis secret scrubber / workspace root wiring, and PTY/term forwarders (`361` non-blank LoC)
- `crates/vox-gui/src/commands/pty/tests.rs` — Extracted + expanded PTY & Warp command unit tests (`260` non-blank LoC)
- `crates/vox-gui/src/main.rs` — Registered `PtyWarpState::new_default()` and 6 `term_*` handlers in `axis_invoke_handler!`
- `crates/vox-gui/ui/src/transport.ts` — Typed Warp history/completion/runbook/voxify DTOs, `safeInvoke` functions, and `VoxTransport` methods
- `crates/vox-gui/ui/src/lib/terminalBridge.ts` — `RunbookLaunchRequest`, `requestOpenRunbook`, `consumePendingOpenRunbook`, `onOpenRunbook`
- `crates/vox-gui/ui/src/transport.console.test.ts` — Vitest coverage for all 6 new IPC methods and `terminalBridge` Runbook pub/sub (`189` non-blank LoC)

## Decisions Made

- Kept `vox-terminal-core` completely free of new workspace dependencies by accepting `&dyn Fn(&str) -> String` in `HistoryStore::record_with_scrubber` and injecting `vox_secrets::scrub_resolved_secrets` in `crates/vox-gui/src/commands/pty/warp.rs`.
- Placed `spawn_pty_forwarder` and `spawn_term_forwarder` in `pty/warp.rs` so `crates/vox-gui/src/commands/pty.rs` stays well below `430` non-blank LoC (`402` LoC) while keeping every `#[tauri::command]` entrypoint directly inside `pty.rs` for `ModuleClass::Host` inheritance.
- Annotated test-only `/tmp/vox-441` fixture path in standalone `crates/vox-gui/src/commands/pty/tests.rs` with `// vox-arch-check: allow abs-path` so `vox-arch-check` passes cleanly.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Static Gate Compliance] Added `vox-arch-check` abs-path allow comment in extracted `pty/tests.rs`**
- **Found during:** Task 3 (`vox-arch-check` verification)
- **Issue:** Extracting `#[cfg(test)] mod tests` from `pty.rs` into standalone `crates/vox-gui/src/commands/pty/tests.rs` caused `vox-arch-check` (`[no-hardcoded-abs-path]`) to inspect the pre-existing `Some("/tmp/vox-441")` test fixture literal on line 141.
- **Fix:** Added `// vox-arch-check: allow abs-path` directly above the test fixture call (matching existing line 22 in the same file).
- **Files modified:** `crates/vox-gui/src/commands/pty/tests.rs`
- **Verification:** `./target/debug/vox-arch-check` exited `0` with zero violations.

---

**Total deviations:** 1 auto-fixed (Rule 3 static gate compliance)
**Impact on plan:** Zero scope creep; preserved existing test fixture semantics while keeping `vox-arch-check` green.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All Wave 1 Rust backend engines (`HistoryStore`, `CompletionEngine`, `scan_workspace_runbooks`, `voxify_subshell`), `vox-orchestrator-d` `term.voxify` RPC, Tauri `#[tauri::command]` handlers, and `transport.ts` / `terminalBridge.ts` TypeScript contracts are green and ready for Wave 2 UI plans (`444-02-PLAN.md` and `444-03-PLAN.md`).
