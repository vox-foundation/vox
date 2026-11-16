# Phase 444: Warp Input Editor, Shell Completions, History & Runbooks - Research

**Researched:** 2026-11-15
**Domain:** Warp-style multi-line Scriptorium prompt editor (`<textarea>` + token highlighter), intent routing (`!` / `?` / `/` / NL heuristic), 3-stage Clavis+redact persistent `HistoryStore` (`Ctrl+R` / `Up`), sub-50ms Rust `$PATH` + `cwd` + pure-Rust `.git` + CLI completion engine, `@tanstack/react-table` v9 JSON/Nushell ledger table, runnable `.vox`/Markdown Runbooks (`RunbookPanel` + `Omnibar`), and 1-click Subshell Voxify (`OSC 633` + `OSC 7`)
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

### Warp Input Editor & Multi-Line Composer (`TERM-WARP-01`)
- **Pinned Bottom Composer**: Upgrade `crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.tsx` (extracting subcomponents so every file stays `< 400` non-blank LoC) into a true Warp-style multi-line **Scriptorium Prompt Editor** pinned to the bottom of the active `TerminalSessionPane` / `Console`:
  - Controlled `<textarea>` with auto-grow up to `8` visible rows (scrollable beyond `8`), monospace font stack (`TERMINAL_MONO_FONT_STACK` / `TERMINAL_MONO_STYLE`), and synchronized transparent-text syntax highlight overlay (`pipes |`, `&&`, `;`, redirects `>`/`>>`, `--flags`, `$ENV_VARS`, `'single'`/`"double"` strings, `@term` / `@file` context mentions).
  - **Keybindings**:
    - `Enter`: submit command when single-line or when line is syntactically complete (no trailing `\`, unclosed quote, or trailing pipe `|`); otherwise insert newline.
    - `Shift+Enter`: always insert newline without submitting.
    - `Up` / `Down`: cycle command history only when cursor is on the first/last line respectively (preserving multi-line cursor movement in interior lines).
    - `Tab` / `RightArrow` (at EOL): accept inline ghost-text suggestion or active completion item.
    - `Ctrl+R` / `Cmd+R`: open fuzzy `HistorySearchPopover` (`cmdk`-backed or accessible listbox popover) anchored above the prompt editor.
    - `Escape`: dismiss open completion/history popover first; if none open and session is in `Autopilot`, drop to `Manual` control mode.
  - **Explicit Intent Routing & Mode Pill**:
    - Keep `term-shell-mode-select` (`VoxSelect`: `default` | `nu` | `vox`) and `console-input-mode-badge` backwards-compatible with existing tests.
    - Prefix & heuristic routing: `!cmd` forces shell execution; `?prompt` or `/ask` routes to Agent / Loquela chat with `@term` active-session context attached; `/cmd` routes slash commands; natural-language heuristic badge (`AI Assist` chip when multi-word prose doesn't match a known binary/`$PATH` command or shell syntax, with `Tab`/`Click` to dispatch to Agent or `Enter` to run as shell).

### Shell Completions & Redacted Fuzzy History (`TERM-WARP-02`)
- **Sub-50ms Rust + Frontend Completion Pipeline**:
  - Add `crates/vox-terminal-core/src/completions.rs` and Tauri IPC `term_complete(session_id, line, cursor, cwd, limit)` returning typed completion items (`kind`: `command` | `subcommand` | `flag` | `path` | `git_branch` | `vox_cli` | `history`, `label`, `insert_text`, `detail`, `score`):
    1. **`vox` CLI & capability catalog**: merge existing `discoverySuggest` (`vox` subcommands & flags) with terminal completions.
    2. **`$PATH` executable scan** (cached in-memory with TTL + platform executable bit / `PATHEXT` check on Windows) + shell builtins (`cd`, `export`, `alias`, `source`, `echo`, `pwd`, `exit`).
    3. **`cwd`-relative file/directory completion**: triggered on path tokens (`./`, `../`, `/`, `~/`, or after `cd`/`cat`/`ls`/`rg`/`git add`/`vox run`), appending `/` for directories, strictly bounded to non-traversal local `fs::read_dir` with cap (`<= 50` entries).
    4. **Git branch & tag completion**: fast pure-Rust read of `<repo>/.git/HEAD`, `.git/refs/heads/**`, `.git/refs/tags/**`, and `.git/packed-refs` (zero `git` subprocess spawn overhead when possible) for `git checkout`, `git switch`, `git merge`, `git rebase`, `git branch -d`.
    5. **Common CLI flag/subcommand specs**: lightweight curated completion tables in Rust/TS for high-frequency dev tools (`git`, `cargo`, `pnpm`, `npm`, `docker`, `kubectl`, `gh`, `rg`).
  - Upgrade `IntelliSensePopover` + `intellisenseEngine.ts` to display ranked multi-source icons/badges, argument descriptions, and inline ghost-text preview (`data-testid="ghost"`).
- **Persistent 3-Stage Redacted History (`HistoryStore`)**:
  - Add `crates/vox-terminal-core/src/history.rs` + Tauri IPC `term_history_record` and `term_history_query(query, cwd, limit)`:
    - Every recorded command MUST pass through the mandatory **3-stage scrubber** (`strip_ansi_and_normalize` -> `vox_secrets::scrub_resolved_secrets` -> `vox_redact`) BEFORE touching disk (`~/.vox/terminal/history.jsonl` or SQLite table with bounded ring cap e.g. `5,000` entries) and skip commands prefixed with a leading space (`HISTCONTROL=ignorespace` parity).
    - Deduplicate consecutive/recent identical commands while boosting frequency + recency + matching-`cwd` affinity in fuzzy subsequence ranking.
    - Graceful browser/preview (`no-tauri`) fallback in `localStorage` (`vox.console.history.v1`, also secret-pattern scrubbed).

### Interactive Structured Output Tables & Runbooks (`TERM-WARP-03`)
- **Interactive Nushell / JSON Ledger Table (`JsonLedgerTable.tsx`)**:
  - When a `BlockCard` is detected as structured JSON (`isStructuredBlock`: array of objects `[{...}, ...]` or top-level object `{...}` from `!nu ... | to json` or JSON CLI tools), offer a toggle (`Table` | `JSON` | `Raw`, defaulting to `Table` for non-empty arrays of uniform/semi-uniform objects and `JSON` tree/formatted view for scalar/nested objects) powered by already-installed `@tanstack/react-table` (`^9.3.1`).
  - Support column header click sorting, instant client-side filter input, cell/row copy, and `Export CSV` / `Copy JSON` actions using Roman Warm Slate token classes.
- **Parameterized Runbooks & Workflows (`RunbookPanel.tsx` + `vox_terminal_core::runbooks`)**:
  - Discover runnable workspace workflows from `scripts/*.vox`, `.vox/runbooks/*.vox`, `.vox/runbooks/*.md`, and Markdown fenced `sh`/`bash`/`vox` blocks with `# @runbook` / frontmatter metadata (`term_runbooks_list` Tauri IPC with workspace-bounded scan).
  - Parse `{{param_name}}` or `${PARAM_NAME:default}` placeholders into typed fill-in inputs (`VoxInput` / `VoxSelect` — zero raw `<select>`) inside a new **Runbooks** tab/drawer in `DiscoveryRail.tsx` (and accessible via `Cmd+P` / `CommandPalette`), allowing 1-click **Insert into Editor** or **Run in Active Pane**.

### Subshell "Voxify" Auto-Detection & Prompt Injection (`TERM-WARP-04`)
- **Subshell Detection & 1-Click Voxify**:
  - Detect interactive subshell entry (`ssh`, `docker exec -it`, `kubectl exec -it`, `nix develop`, `sudo -i` / `su`, nested `bash`/`zsh`/`fish`/`pwsh`/`nu`) either from the submitted command pattern or prompt/OSC 633 silence while foreground PTY awaits input.
  - Expose `term_session_voxify(session_id, shell_hint)` on `SessionHost` / `TermDispatch` / `vox-gui` + a Roman banner/button (`Voxify Subshell`, `data-testid="term-voxify-subshell"`) on the session header that injects a compact, POSIX/shell-safe non-echo `OSC 633` + `OSC 7` hook snippet (`stty -echo` / `HISTFILE` safe) so block boundaries, exit codes, and `cwd` tracking resume inside the nested shell without clobbering user state.

### Claude's Discretion
- Exact submodule split under `crates/vox-terminal-core/src/` (`history.rs`, `completions.rs`, `runbooks.rs`, `voxify.rs`) and `crates/vox-gui/ui/src/components/surfaces/Console/` (`ScriptoriumPromptEditor.tsx`, `promptHighlighter.ts`, `HistorySearchPopover.tsx`, `JsonLedgerTable.tsx`, `RunbookPanel.tsx`) to keep every file comfortably under `< 400` non-blank LoC.
- Exact `remote::gate` classification in `contracts/gui/command-backends.v1.yaml` for any new `vox-gui` command module (terminal/filesystem-local commands stay `host` / `local_only`, matching `pty`).

### Deferred Ideas (OUT OF SCOPE)
- Inline AI Ghost-Text Suggestions, Command Explain Tooltip & Error Auto-Fix Banner — Phase 445 (`TERM-AI-01..04`).
- Full tree-sitter WASM bash parser — lightweight deterministic tokenizer in TypeScript/Rust is sufficient and avoids new npm/WASM bundle weight.
- Cloud team-synced runbooks — local workspace `scripts/*.vox` and `.vox/runbooks/` are canonical.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| `TERM-WARP-01` | Warp-Style Multi-Line Block Input Editor & Explicit Intent Routing (`<textarea>` up to 8 rows, syntax highlight overlay, `Enter` syntax-completeness check vs `Shift+Enter`, `Up`/`Down` boundary history, `Tab`/`RightArrow` ghost accept, `Ctrl+R` popover, `!`/`?`/`/` + NL heuristic routing). | `ScriptoriumPromptEditor.tsx` + `promptHighlighter.ts` + `intentRouter.ts` behind `InputEditor.tsx` facade; preserves `role="textbox"`, `aria-label="console input"`, `data-testid="term-shell-mode-select"`, `data-testid="console-input-mode-badge"`, `data-testid="ghost"`, and `Console.tsx:143` `Cmd+J` focus selector. |
| `TERM-WARP-02` | Context-Aware Shell, Path, Git & `vox` CLI Autocomplete + Redacted Fuzzy History (`< 50ms` Rust `term_complete` + `HistoryStore` 3-stage scrubber + `ignorespace` + `Ctrl+R` `HistorySearchPopover`). | `vox_terminal_core::{completions, history}` + `commands::pty::warp` Tauri wrappers (`term_complete`, `term_history_record`, `term_history_query`), zero new Cargo edges via caller-supplied Clavis scrubber closure, pure-Rust `.git` ref reader (including worktree `gitdir:` files), merged in `terminalCompletions.ts` with `discoverySuggest`. |
| `TERM-WARP-03` | Interactive Nushell / JSON Structured Data Tables & Parameterized Runbooks (`@tanstack/react-table` `Table`/`JSON`/`Raw` toggle, sort/filter/CSV/JSON export, workspace `scripts/*.vox` + `.vox/runbooks/` + Markdown `{{param}}` runbook runner in `DiscoveryRail` & `Omnibar`). | `JsonLedgerTable.tsx` built on verified `@tanstack/react-table@9.3.1` v9 API (`tableFeatures` + `useTable` + `rowSortingFeature` + `createSortedRowModel`), `vox_terminal_core::runbooks` + `term_runbooks_list` IPC, `RunbookPanel.tsx` tab in `DiscoveryRail.tsx`, and `Omnibar` action integration. |
| `TERM-WARP-04` | Subshell "Voxify" Auto-Detection & One-Click Prompt Hook Injection (`ssh`, `docker exec -it`, `kubectl exec -it`, `nix develop`, `sudo -i`, nested `zsh`/`bash`/`fish`/`pwsh`/`nu` detection + 1-click `OSC 633` + `OSC 7` injection). | `vox_terminal_core::voxify` base64/printf `stty -echo` 5-shell hook generator + subshell command detector, `SessionHost::voxify_subshell` across `LocalHost` & `DaemonHost` (`term.voxify` in `TermDispatch` with `refuse_agent_origin`), `term_session_voxify` IPC, and `data-testid="term-voxify-subshell"` action in `ConsoleSessionBar` / `TerminalSessionPane`. |
</phase_requirements>

---

## Summary

Phase 444 upgrades the Vox Axis Console input, completion, structured-output, runbook, and nested-subshell layers on top of Phase 442 (`vox-terminal-core` `SessionHost` + `OSC 633`/`OSC 7` + `L1–L5` security posture) and Phase 443 (`TerminalSessionPane`, `ConsoleSessionBar`, `ConsoleSplitGrid`, `BlockCard`, `BlockStreamView`). Every required dependency is **already installed** in the workspace (`@tanstack/react-table@^9.3.1`, `cmdk@^1.1.1`, `lucide-react@^1.16.0`, `base64`, `serde_json`, `vox-redact`, `vox-secrets`, `vox-repository`, `vox-config`), so Phase 444 requires **zero new npm packages, zero new Cargo crates, and zero new workspace Cargo crate edges**.

Direct codebase inspection uncovered **six critical invariants** that the Phase 444 plan must honor to avoid compile, CI-gate, or Vitest regressions:
1. **`@tanstack/react-table` is v9 (`^9.3.1`), NOT v8**: `useReactTable` and `getCoreRowModel()` are **absent** from `@tanstack/react-table`'s root exports (`node_modules/@tanstack/react-table/dist/index.d.ts`). Production code in `DataTable.tsx` uses the v9 API: `tableFeatures({ rowSortingFeature, sortedRowModel: createSortedRowModel(), sortFns: { alphanumeric: sortFn_alphanumeric } })` + `useTable(...)`. `JsonLedgerTable.tsx` MUST follow this exact v9 pattern.
2. **`vox-terminal-core` has NO `vox-secrets` Cargo dependency (Zero-Crate-Edge Rule)**: `crates/vox-terminal-core/Cargo.toml` depends on `vox-redact`, NOT `vox-secrets` (`vox-gui` and `vox-orchestrator-d` depend on both). Adding `vox-secrets` to `vox-terminal-core` would violate `contracts/ci/crate-edges.allow.v1.json`. Exactly like Phase 442 (`term_dispatch.rs:313-317`, `console_a2a.rs:28-32`), `HistoryStore` in `vox-terminal-core` should run Stage 1 (`strip_ansi_and_normalize`) and Stage 3 (`vox_redact::redact_owned`) internally while accepting a Stage 2 `extra_scrub: F where F: Fn(&str) -> String` closure supplied by `vox-gui` / `vox-orchestrator-d` (`vox_secrets::scrub_resolved_secrets`).
3. **`gui_command_backends.rs` + `god_object.rs` Dual Constraint on `crates/vox-gui/src/commands/pty.rs`**:
   - `crates/vox-cli-ci/src/gui_command_backends.rs` scans top-level `.rs` files in `crates/vox-gui/src/commands/` and requires `crates/vox-gui/src/commands/pty.rs` to exist on disk (`pty -> class: host`), while `manifest.rs:362` locks `ui/e2e/fixtures/remote-gate-mesh-console.json` byte-for-byte.
   - Meanwhile, `crates/vox-gui/src/commands/pty.rs` is already `562` pre-test lines (~`470` non-blank non-comment lines), right below `vox-code-audit`'s `hard_max_lines: 500` error ceiling (`god_object.rs:16`).
   - **Clean solution**: Keep `crates/vox-gui/src/commands/pty.rs` on disk, place the 5 new Phase 444 Tauri commands in `crates/vox-gui/src/commands/pty/warp.rs` (`mod warp; pub use warp::*;` inside `pty.rs`), and register them in `main.rs` as `commands::pty::term_history_record`, `commands::pty::term_history_query`, `commands::pty::term_complete`, `commands::pty::term_runbooks_list`, and `commands::pty::term_session_voxify`. Note: `#[tauri::command]` generates `pub(crate) macro __cmd__<name>`, so in `pty.rs` `pub use warp::*;` re-exports both the function and the Tauri `__cmd__*` hidden macro item cleanly under `commands::pty::*`, inheriting `ModuleClass::Host` (`local_only` remotely) with **zero edits** to `contracts/gui/command-backends.v1.yaml` or `remote-gate-mesh-console.json`!
4. **Unique `screen.getByRole('textbox')` on Initial `<Console />` Render**: `Console.test.tsx:91, 99, 153, 165, 226` calls `screen.getByRole('textbox')` without a name filter. `<textarea aria-label="console input">` has implicit ARIA role `textbox`, which satisfies `getByRole('textbox')` — **ONLY IF** `HistorySearchPopover.tsx`, `JsonLedgerTable.tsx` (filter input), and `RunbookPanel.tsx` (search/parameter inputs) are **unmounted (`return null`)** by default on initial Console render! Also update `Console.tsx:143` `Cmd+J` selector from `'input[aria-label="console input"]'` to `'[aria-label="console input"]'` so `Cmd+J` focuses the upgraded `<textarea>`.
5. **Defensive `typeof transport.fn === 'function'` Guards for Partial Test Mocks**: `Console.test.tsx:11-40` and `InputEditor.test.tsx:5-9` replace `../../../transport` with explicit `vi.mock(...)` object literals that only list legacy functions (`discoverySuggest`, `termSessionOpen`, etc.). Any call in `InputEditor`, `Console`, `TerminalSessionPane`, or `DiscoveryRail` to new transport exports (`termComplete`, `termHistoryRecord`, `termHistoryQuery`, `termRunbooksList`, `termSessionVoxify`) MUST guard with `typeof transport.termComplete === 'function'` (falling back to empty array / local fallback) so existing un-updated `vi.mock('../../../transport', ...)` blocks never throw `TypeError: termComplete is not a function`.
6. **Git Worktree `.git` File Support in Pure-Rust Git Completion**: In git worktrees (common in Vox multi-agent / GSD workstreams), `<cwd>/.git` is a **regular file** containing `gitdir: /path/to/main/.git/worktrees/<name>` rather than a directory, while shared `packed-refs` and `refs/tags` live in `<commondir>` (`<gitdir>/commondir` -> `../..`). Supporting both directory `.git` and file `gitdir:` + `commondir` takes ~25 lines of pure Rust and ensures branch/tag completion works in both standard clones and Vox agent worktrees with zero `git` subprocess spawns.

**Primary recommendation:** Implement Phase 444 in four vertical waves/plans:
1. **Plan 444-01 (Rust Core + Tauri IPC)**: `vox_terminal_core::{history, completions, runbooks, voxify}` + `SessionHost::voxify_subshell` (`LocalHost`, `DaemonHost`, `TermDispatch` `term.voxify`) + `vox-gui` `commands/pty/warp.rs` (`term_history_record`, `term_history_query`, `term_complete`, `term_runbooks_list`, `term_session_voxify`) + `transport.ts` typed wrappers.
2. **Plan 444-02 (Scriptorium Multi-Line Editor + Intent Routing)**: `promptHighlighter.ts`, `intentRouter.ts`, `ScriptoriumPromptEditor.tsx`, and `InputEditor.tsx` wrapper + `Console.tsx` `?prompt` / `/ask` -> `requestChatAttention` routing.
3. **Plan 444-03 (Unified Completions + Redacted Fuzzy History Popover)**: `terminalCompletions.ts` merging `termComplete` + `discoverySuggest`, `IntelliSensePopover.tsx` badges/icons, and `HistorySearchPopover.tsx` (`Ctrl+R` / `Cmd+R` + `Up`/`Down` boundary cycling + `no-tauri` scrubbed `localStorage` fallback).
4. **Plan 444-04 (Interactive JSON Ledger Table, Runbooks & Subshell Voxify UI)**: `JsonLedgerTable.tsx` (`@tanstack/react-table` v9 `Table` | `JSON` | `Raw` + sort/filter/CSV/JSON export) wired into `BlockCard.tsx`, `RunbookPanel.tsx` wired into `DiscoveryRail.tsx` (`Help` | `Runbooks` tabs) + `Omnibar` integration, and Subshell Voxify banner/button (`data-testid="term-voxify-subshell"`) in `ConsoleSessionBar.tsx` / `TerminalSessionPane.tsx`.

---

## Project Constraints (from AGENTS.md, GEMINI.md & Guards)

- **Zero New Dependencies / Zero New Crate Edges:** Do not edit `package.json`, `Cargo.lock`, `docs/src/architecture/layers.toml`, or `contracts/ci/crate-edges.allow.v1.json`.
- **File Size Budgets (`vox-code-audit` `god_object.rs`):**
  - `[VERIFIED: crates/vox-code-audit/src/detectors/god_object.rs:16-19]` — `"hard_max_lines: 500, warn_max_lines: 400, info_max_lines: 300, max_methods: 12"`. Every new `.rs`, `.ts`, and `.tsx` file MUST stay `< 400` non-blank non-comment lines (and `crates/vox-gui/src/commands/pty.rs` MUST stay `< 500` non-blank non-test lines by delegating to `crates/vox-gui/src/commands/pty/warp.rs`).
- **Untested Public API Detector (`vox-code-audit` `untested_pub_api.rs`):**
  - `[VERIFIED: crates/vox-code-audit/src/detectors/untested_pub_api.rs:13]` — `"and intentionally-thin passthrough files (< 30 non-blank lines) are excluded."` Every new Rust module `>= 30` non-blank lines must carry inline `#[cfg(test)] mod tests` exercising its `pub` functions.
- **VoxLang-First Glue Code (`AGENTS.md`):** Never add `.sh`, `.ps1`, or `.py` scripts to the repo. Note: embedded POSIX/PowerShell/Fish/Nu hook strings inside Rust (`voxify.rs`, like `shell_init.rs`) are terminal wire payloads, not repository scripts.
- **Secret Management (`AGENTS.md` & `TERM-WARP-02`):** Never read raw secret env vars directly; route Stage 2 history scrubbing through `vox_secrets::scrub_resolved_secrets` in `vox-gui` / `vox-orchestrator-d`.
- **Static UI Source Guards (`crates/vox-gui/ui/src/guards/*.test.ts`):**
  1. **Zero Emoji / Dingbat (`zeroEmojiAndDingbat.test.ts`):**
     - `[VERIFIED: crates/vox-gui/ui/src/guards/zeroEmojiAndDingbat.test.ts:5-6]` — `const BANNED_GLYPH_RE = /[\p{Extended_Pictographic}\u2600-\u27BF\u23F0-\u23FE\u25A0-\u25FF\u2197\u21BA-\u21C4\u21E2\u2B06\u2302\u2442\u2295\u21B5\u23CE\u21E5]/gu;`
     - Note specifically: `\u21B5` (`↵`), `\u23CE` (`⏎`), `\u21E5` (`⇥`), `\u25A0-\u25FF` (`▸`, `▾`, `●`), and all emojis are **banned** in production `.ts`/`.tsx` strings! Use `Icon.*` / `VoxIcon` / `lucide-react` icons and plain ASCII text labels (`Enter`, `Tab`, `Shift+Enter`, `Ctrl+R`).
  2. **Zero Raw `<select>` or `<input type="range">` (`noRawNativeControlsGuard.test.ts`):**
     - `[VERIFIED: crates/vox-gui/ui/src/guards/noRawNativeControlsGuard.test.ts:13-14]` — `const NATIVE_SELECT_RE = /<select\b/; const NATIVE_RANGE_RE = /\btype\s*=\s*(?:["']range["']|\{\s*["']range["']\s*\})/;`
     - All select inputs in `RunbookPanel.tsx` and `InputEditor.tsx` MUST use `<VoxSelect>` from `components/ui/Select`.
  3. **IPC Boundary Containment (`transportIpcGuard.test.ts` & `ipcBoundaries.test.ts`):**
     - `[VERIFIED: crates/vox-gui/ui/src/guards/ipcBoundaries.test.ts:43]` — `const ALLOW_DIRECT_INVOKE = new Set(['transport.ts']);`
     - All new Tauri commands MUST be exposed via `safeInvoke` wrappers in `crates/vox-gui/ui/src/transport.ts` (outside the `__VOX_RAW_IPC_BEGIN__..__VOX_RAW_IPC_END__` raw block, calling `safeInvoke`).
  4. **Toast Error Sanitization (`toastBodyGuard.test.ts`):**
     - `[VERIFIED: crates/vox-gui/ui/src/guards/toastBodyGuard.test.ts:17]` — `const LEAK_SINK_PATTERN = /(?:body:\s*|\bset[A-Z]\w*\()\s*String\(|\bString\((?:e|err|error)\)/;`
     - Always format caught errors via `sanitizeErrorForToast(err)`.
  5. **Theme Contract (`themeContract.ts`):**
     - `[VERIFIED: crates/vox-gui/ui/src/lib/themeContract.ts:106-113]` — `BANNED_ROMANESQUE_FONT_LITERALS` bans `'Courier New'`, `'Courier'`, `'Arial'`, `'Consolas'`, `'Menlo'`, `'Monaco'` in UI source files. Always import `TERMINAL_MONO_FONT_STACK` (`terminalTheme.ts`) or `TERMINAL_MONO_STYLE` (`BlockCard.tsx`).
- **Command & Build Rules (`00-core.md` & `GEMINI.md`):**
  - Plain `cargo` scoped with `-p <crate>` (never `cargo test --workspace`), formatted with `rustfmt --edition 2024 <file.rs>`.
  - Single terminal command per step; prefix build/test commands with `timeout`.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Persistent 3-stage redacted command history (`HistoryStore`) | Rust Engine (`vox-terminal-core::history`) + Host Scrubber (`vox-gui` `commands::pty::warp`) | Browser Frontend (`HistorySearchPopover.tsx` + `localStorage` fallback) | Keeps JSONL ring persistence, `ignorespace`, ANSI normalization, Clavis secret scrubbing, and `vox_redact` in Rust before disk I/O while preserving browser-preview usability. |
| Shell, `$PATH`, `cwd`, `.git`, and CLI completions | Rust Engine (`vox-terminal-core::completions`) | Frontend (`terminalCompletions.ts` + `IntelliSensePopover.tsx`) | Rust inspects `$PATH` executables, `cwd` entries, and `.git/{HEAD,refs,packed-refs}` in `< 10ms` without spawning subprocesses; frontend merges async `discoverySuggest` (`vox` CLI) + inline ghost text. |
| Multi-line Scriptorium editor, syntax token overlay & intent routing | Browser Frontend (`ScriptoriumPromptEditor.tsx`, `promptHighlighter.ts`, `intentRouter.ts`) | Host/Chat Bridge (`terminalBridge.ts` `requestChatAttention`) | Keystroke-level cursor/selection geometry, syntaxcompleteness checks, and synchronized `<pre>` scroll overlay must run at 60fps synchronously in React. |
| Interactive JSON / Nushell ledger table | Browser Frontend (`JsonLedgerTable.tsx` in `BlockCard.tsx`) | Rust Engine (`Block` output chunks) | `@tanstack/react-table` v9 sorts/filters parsed JSON arrays in-memory with zero extra IPC round-trips and exports CSV/JSON client-side. |
| Workspace Runbook discovery & parameter parsing | Rust Engine (`vox-terminal-core::runbooks`) | Browser Frontend (`RunbookPanel.tsx` in `DiscoveryRail.tsx` + `Omnibar`) | Rust scans `scripts/*.vox`, `.vox/runbooks/*.{vox,md}`, and `docs/src/how-to/*.md` with canonicalized path-traversal protection; React renders typed `VoxInput`/`VoxSelect` parameter forms. |
| Subshell detection & 1-click `OSC 633` + `OSC 7` Voxify | Rust Engine (`vox-terminal-core::voxify` + `SessionHost` + `TermDispatch`) | Browser Frontend (`ConsoleSessionBar.tsx` / `TerminalSessionPane.tsx`) | Writing raw non-echo hook bytes into the live PTY (`LocalHost` or remote `DaemonHost` over `term.voxify`) belongs on `SessionHost` with `refuse_agent_origin` protection. |

---

## Standard Stack

### Core (Already Installed — Zero New Packages)

| Library / Crate | Verified Version | Purpose | Verified Source |
|-----------------|------------------|---------|-----------------|
| `@tanstack/react-table` | `^9.3.1` | Headless v9 sorting (`tableFeatures`, `useTable`, `rowSortingFeature`, `createSortedRowModel`) for `JsonLedgerTable.tsx` | `[VERIFIED: crates/vox-gui/ui/package.json:27]` — `"@tanstack/react-table": "^9.3.1"` |
| `cmdk` | `^1.1.1` | Accessible command/search list primitives (`Command`, `CommandInput`, `CommandList`, `CommandItem` in `components/ui/Command.tsx`) usable for `HistorySearchPopover.tsx` | `[VERIFIED: crates/vox-gui/ui/package.json:30]` — `"cmdk": "^1.1.1"` |
| `lucide-react` | `^1.16.0` | Crisp stroke icons via `<VoxIcon>` / direct Lucide imports (`Table2`, `Braces`, `FileText`, `Download`, `History`, `Terminal`, `Sparkles`, `Play`) | `[VERIFIED: crates/vox-gui/ui/package.json:32]` — `"lucide-react": "^1.16.0"` |
| `vox-terminal-core` | workspace (`layer = 3`) | Terminal engine (`block::strip_ansi_and_normalize`, `shell_init`, `SessionHost`, `LocalHost`, `DaemonHost`, `corpus::redact`) | `[VERIFIED: docs/src/architecture/layers.toml:272]` — `vox-terminal-core = { layer = 3, max_loc = 25_000, staleness_exempt = true, orphan_exempt = true }` |
| `vox-redact` | workspace (`layer = 1`) | Stage 3 regex secret/PII redaction (`vox_redact::redact_owned`) already wired into `vox-terminal-core` | `[VERIFIED: crates/vox-terminal-core/Cargo.toml:27]` — `vox-redact = { workspace = true }` |
| `base64` | workspace | POSIX-safe base64 encoding for `voxify` subshell hook injection payloads | `[VERIFIED: crates/vox-terminal-core/Cargo.toml:21]` — `base64 = { workspace = true }` |
| `vox-secrets` | workspace | Stage 2 Clavis live secret value scrubber (`vox_secrets::scrub_resolved_secrets`) called in `vox-gui` / `vox-orchestrator-d` | `[VERIFIED: crates/vox-gui/Cargo.toml:31]` (`vox-secrets.workspace = true` in `vox-gui` & `term_dispatch.rs:315`) |
| `vox-repository` | workspace | Workspace root discovery (`discover_repository_or_fallback`, `resolve_repo_root_for_ci`) in `vox-gui` for `term_runbooks_list` | `[VERIFIED: crates/vox-gui/src/commands/docs_index.rs:85]` — `let repo = vox_repository::discover_repository_or_fallback(&hint);` |
| `vox-config` | workspace | Canonical user `.vox` state directory (`vox_config::paths::user_home_dir().join(".vox")`) in `vox-gui` for `HistoryStore` | `[VERIFIED: crates/vox-gui/src/commands/daemon.rs:43-44]` — `vox_config::paths::user_home_dir().join(".vox")` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `textarea` + synchronized `<pre aria-hidden="true">` token overlay | Monaco / CodeMirror 6 / `contenteditable` | `contenteditable` breaks RTL/selection/IME and `fireEvent.change(input, { target: { value } })` in Vitest (`Console.test.tsx:100`, `InputEditor.test.tsx:19`)! `<textarea>` natively supports `.value`, `selectionStart`, `rows`, IME, and `getByRole('textbox')` with zero bundle bloat. |
| Pure-Rust `.git/HEAD` + `refs` + `packed-refs` reader | Spawning `git branch --all` per keystroke | Spawning `git` on Windows/macOS costs `15–80ms` per keystroke and contends on `.git/index.lock`; reading `.git/refs/heads` + `.git/packed-refs` in Rust takes `< 0.5ms` with zero process spawns. |
| Submodule `commands/pty/warp.rs` re-exported from `commands/pty.rs` | Creating top-level `commands/term_warp.rs` | A new top-level `commands/*.rs` file forces edits to `contracts/gui/command-backends.v1.yaml` and risks drifting `ui/e2e/fixtures/remote-gate-mesh-console.json` (`manifest.rs:362`). Re-exporting from `pty.rs` keeps `pty` (`class: host`) authoritative while keeping both `.rs` files well under 500 LoC. |

---

## Deep Technical Verification 1: `vox_terminal_core::history` & 3-Stage Scrubbing (`TERM-WARP-02`)

### 1.1 Zero-Crate-Edge 3-Stage Scrubber Contract
- In `vox-orchestrator-d/src/term_dispatch.rs` (Phase 442), the 3-stage terminal scrubber is already canonically ordered:
  1. `vox_terminal_core::block::strip_ansi_and_normalize(raw)` (`crates/vox-terminal-core/src/block.rs:130`)
  2. `vox_secrets::scrub_resolved_secrets(&stage1)` (`crates/vox-secrets/src/lib.rs`)
  3. `vox_redact::redact_owned(&stage2)` (`crates/vox-terminal-core/src/corpus/redact.rs:5`)
- Because `vox-terminal-core` does **not** depend on `vox-secrets`, `HistoryStore` in `crates/vox-terminal-core/src/history.rs` should expose:
  ```rust
  pub const DEFAULT_HISTORY_CAP: usize = 5_000;

  #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
  pub struct HistoryEntry {
      pub command: String,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub cwd: Option<String>,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub exit_code: Option<i32>,
      pub ts_ms: u64,
      #[serde(default = "default_one")]
      pub count: u32,
  }

  #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
  pub struct ScoredHistoryHit {
      pub command: String,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub cwd: Option<String>,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub exit_code: Option<i32>,
      pub ts_ms: u64,
      pub count: u32,
      pub score: f64,
      /// Character indices in `command` matched by the fuzzy query (for bold highlight spans).
      #[serde(default, skip_serializing_if = "Vec::is_empty")]
      pub match_indices: Vec<usize>,
  }
  ```
- **Recording Rules (`HistoryStore::record_with_scrubber`)**:
  1. **`ignorespace` FIRST (before trimming!)**: If `raw_command.starts_with(' ') || raw_command.starts_with('\t')`, return `Ok(false)` immediately without touching memory or disk (`HISTCONTROL=ignorespace` parity).
  2. **Stage 1**: `let s1 = crate::block::strip_ansi_and_normalize(raw_command);` — trim trailing `\r`/`\n`; if `s1.trim().is_empty()`, return `Ok(false)`.
  3. **Stage 2**: `let s2 = extra_scrub(&s1);` — in `vox-gui` (`commands/pty/warp.rs`), pass `|s| vox_secrets::scrub_resolved_secrets(s)`.
  4. **Stage 3**: `let scrubbed = vox_redact::redact_owned(&s2).into_owned();`
  5. **Deduplication & Ring Cap**: If an existing entry in the ring has `entry.command == scrubbed`, remove the older occurrence, increment `count = old.count.saturating_add(1)`, update `cwd` / `exit_code` / `ts_ms`, and push to the back (most recent). If `entries.len() > self.cap` (`5,000`), drain `entries.len() - self.cap` from the front.
  6. **Atomic JSONL Persistence**: Write to `<path>.tmp` (creating parent dirs via `fs::create_dir_all` with `0o700` permissions on Unix via `std::os::unix::fs::PermissionsExt`) and `fs::rename(&tmp_path, &self.path)` so crashes never leave half-written JSONL lines or world-readable history files.

### 1.2 Fuzzy Subsequence + Recency + `cwd` Affinity Ranker
- `HistoryStore::query(&self, query: &str, current_cwd: Option<&str>, limit: usize) -> Vec<ScoredHistoryHit>`:
  - Limit clamped to `limit.clamp(1, 200)` (default `50`).
  - **Empty `query` (`""`)**: Return most recent entries first, with a light `+0.15` boost when `entry.cwd == current_cwd` (so `Ctrl+R` immediately surfaces recent commands from the active directory while `Up` arrow in editor can pass `cwd_boost = false` or use strict reverse-chronological order when query is empty).
  - **Non-empty `query`**: Case-insensitive subsequence match over `entry.command.chars()`:
    - Must match all query chars in order (`match_indices`).
    - Score components:
      - `subseq_score`: base `1.0` + consecutive character bonus (`+0.35` per adjacent match) + prefix/word-boundary bonus (`+0.5` when match starts at index `0` or after `' '`/`'-'`/`'/'`) - gap penalty (`-0.02 * gap`).
      - `recency_score`: `0.6 * (rank_from_oldest as f64 / total_entries.max(1) as f64)`.
      - `freq_score`: `0.25 * (entry.count.min(20) as f64 / 20.0)`.
      - `cwd_score`: `+0.45` exact `cwd` match, `+0.20` parent/child prefix `cwd` match.

### 1.3 Where History Recording Fires
- Trigger `termHistoryRecord(sessionId, rawInput, cwd, exitCode)` from `Console` / `InputEditor` (or on `BlockClosed` / submit):
  - Recording on **user submit** (passing raw pre-trimmed `value` so leading-space `ignorespace` is preserved!) is critical because `termSessionSubmit` or `InputEditor` may trim or prefix `!nu` before sending to the shell.
  - Specifically: if the user types `" export SECRET=123"` with a leading space, `InputEditor` checks `!raw.startsWith(' ') && !raw.startsWith('\t')` AND `HistoryStore::record_with_scrubber` ALSO checks `raw.starts_with(' ') || raw.starts_with('\t')` as defense-in-depth!

---

## Deep Technical Verification 2: `vox_terminal_core::completions` (`< 50ms` Pipeline, `TERM-WARP-02`)

### 2.1 Wire Contract (`CompletionItem` & `CompletionRequest`)
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TermCompletionKind {
    Command,
    Subcommand,
    Flag,
    Path,
    GitBranch,
    VoxCli,
    History,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TermCompletionItem {
    pub kind: TermCompletionKind,
    pub label: String,
    pub insert_text: String,
    /// Byte/char start offset in `line` that `insert_text` replaces (`replace_start..cursor`).
    pub replace_start: usize,
    pub detail: String,
    pub score: u32,
}
```
- **Crucial UX detail (`replace_start`)**: Returning `replace_start: usize` (character/UTF-16 or UTF-8 index; use character/JS code-unit safe token slicing or return `token_prefix: String` alongside `insert_text`) avoids off-by-one replacement bugs when completing mid-line tokens like `git checkout feat/wa|` or `ls crates/vox-|`! Returning both `token: String` and `insert_text: String` lets the frontend replace the active token cleanly regardless of UTF-8 vs UTF-16 indexing.

### 2.2 Five Deterministic Rust Completion Sources (`< 10ms` total)
1. **Token Context Parser (`parse_completion_context(line, cursor_char_idx)`)**:
   - Slice `line` up to `cursor_char_idx`, find the last unquoted command segment after `|`, `&&`, `||`, `;`.
   - Split segment into words; identify `word_index` (`0` = command position, `>= 1` = argument/subcommand/flag/path position), `head_cmd` (basename of word `0`), `prev_word`, and `active_token`.
2. **`$PATH` Executable Cache + Shell Builtins (`word_index == 0` and not starting with `.` / `/` / `~`)**:
   - `PathExecutableCache` protected by `Mutex<Option<(Instant, Vec<String>)>>` with `Duration::from_secs(30)` TTL.
   - Iterates `std::env::split_paths(&std::env::var_os("PATH"))`, reads directory entries (non-recursive), checks executable permission (`mode & 0o111 != 0` on Unix; `.exe`/`.cmd`/`.bat`/`.ps1` via `PATHEXT` on Windows), deduplicates sorted binary names (~2,000–4,000 strings, `< 4ms` cold scan, `< 0.1ms` cached!).
   - Merges shell builtins: `cd`, `export`, `unset`, `alias`, `source`, `echo`, `pwd`, `exit`, `clear`, `history`, `which`, `type`, `env`, `set`, `true`, `false`.
3. **`cwd`-Relative Path Completion**:
   - Triggered when `active_token` starts with `./`, `../`, `/`, `~/`, or contains `/`, OR when `head_cmd` is a file-accepting command (`cd`, `ls`, `ll`, `cat`, `bat`, `less`, `head`, `tail`, `rm`, `cp`, `mv`, `mkdir`, `touch`, `chmod`, `code`, `vim`, `nvim`, `rg`, `grep`, `vox` after `run`/`check`/`build`/`fmt`, `git` after `add`/`diff`/`checkout`/`restore`).
   - Resolves base directory against `cwd` (`~/` expanded via `HOME`/`USERPROFILE` env lookup — non-secret path env, or passed from `vox-gui` `user_home_dir()`).
   - **Security / Bounds**: Rejects NUL bytes; caps `..` depth (`<= 4` parent hops) and refuses scanning system pseudo-filesystems (`/proc`, `/sys`, `/dev`); caps directory scan at `512` raw `DirEntry` reads and `<= 50` returned matches; appends `/` to directories (`cd`-only filter when `head_cmd == "cd"`). Hidden dotfiles (`.foo`) are shown only when `file_prefix.starts_with('.')`.
4. **Pure-Rust Git Branch & Tag Reader (Zero Subprocess Overhead)**:
   - Triggered when `head_cmd == "git"` and subcommand (`words[1]`) is one of `checkout`, `switch`, `merge`, `rebase`, `cherry-pick`, `reset`, `branch`, `log`, `diff`, `show`, `push`, `pull` (and `active_token` does not start with `-`).
   - Walks upward from `cwd` (max `16` ancestor levels) looking for `.git`:
     - If `.git` is a directory: `git_dir = dot_git`, `common_dir = dot_git`.
     - If `.git` is a regular file (worktree/submodule): reads first line `gitdir: <path>`, resolves relative to `.git`'s parent, and checks `<git_dir>/commondir` (if present, resolves `common_dir` relative to `git_dir`)!
   - Collects ref names from:
     1. `<common_dir>/refs/heads/**` (recursive walk, max depth `6`, max `256` files)
     2. `<common_dir>/refs/tags/**` (recursive walk, max depth `4`, max `256` files)
     3. `<common_dir>/packed-refs` (buffered line scan, max `256 KiB`: skips `#` and `^` peeled lines, extracts `refs/heads/<name>` and `refs/tags/<name>`)
     4. `<git_dir>/HEAD` (`ref: refs/heads/<current>` — marks current branch in `detail: "current git branch"`).
   - Deduplicates in a `BTreeSet<String>` and prefix/subsequence-matches `active_token`. Zero `Command::new("git")` calls!
5. **Curated CLI Subcommand & Flag Tables**:
   - Compact `const` tables in `crates/vox-terminal-core/src/completions_specs.rs` (or inside `completions.rs` if `< 350` LoC total) for:
     - `git`: `status`, `checkout`, `switch`, `branch`, `commit`, `add`, `diff`, `log`, `pull`, `push`, `fetch`, `merge`, `rebase`, `stash`, `restore`, `cherry-pick`, `worktree` + flags `--amend`, `--no-verify`, `--staged`, `--oneline`, `--graph`, `-b`, `-D`.
     - `cargo`: `check`, `test`, `build`, `run`, `clippy`, `doc`, `bench`, `tree`, `metadata` + flags `-p`, `--package`, `--lib`, `--test`, `--nocapture`, `--release`, `--all-targets`, `--features`, `--no-default-features`.
     - `pnpm` / `npm`: `install`, `run`, `test`, `build`, `exec`, `dlx`, `why`, `outdated`, `--dir`, `--filter`, `-D`.
     - `docker`: `ps`, `exec`, `logs`, `build`, `run`, `compose`, `images`, `stop`, `rm`, `-it`, `--rm`, `-d`, `-p`, `-v`.
     - `kubectl`: `get`, `describe`, `logs`, `exec`, `apply`, `delete`, `port-forward`, `config`, `-n`, `--namespace`, `-it`, `-o`, `pods`, `svc`, `deployments`.
     - `gh`: `pr`, `issue`, `run`, `repo`, `auth`, `status`, `view`, `list`, `create`, `checks`.
     - `rg`: `-n`, `-i`, `-l`, `-g`, `--glob`, `-t`, `--type`, `-F`, `--fixed-strings`, `-C`.

---

## Deep Technical Verification 3: `vox_terminal_core::runbooks` & `JsonLedgerTable.tsx` (`TERM-WARP-03`)

### 3.1 Workspace Runbook Scanner (`crates/vox-terminal-core/src/runbooks.rs`)
- **Bounded Discovery Roots** (relative to `workspace_root: &Path`, canonicalized so symlinks cannot escape `workspace_root`):
  1. `scripts/*.vox` (top-level `.vox` automation scripts — e.g., `[VERIFIED: scripts/arch-check.vox:1-10]` has `// scripts/arch-check.vox`, description comments, and `// Usage: vox run scripts/arch-check.vox`!).
  2. `.vox/runbooks/*.vox` and `.vox/runbooks/*.md` (when directory exists).
  3. `docs/src/how-to/*.md` — **ONLY** fenced `sh` / `bash` / `vox` code blocks preceded by `<!-- @runbook ... -->`, `# @runbook`, or fence info string containing `@runbook` (so arbitrary documentation snippets aren't falsely advertised as runnable workflows unless explicitly tagged, plus top-level `.vox/runbooks/*.md` where every `sh`/`bash`/`vox` fenced block is a runbook step!).
- **Wire DTO (`RunbookEntryDto` & `RunbookParamDto`)**:
  ```rust
  #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
  pub struct RunbookParamDto {
      pub name: String,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub default_value: Option<String>,
      #[serde(default, skip_serializing_if = "Vec::is_empty")]
      pub options: Vec<String>, // e.g. {{env:dev|staging|prod}} -> VoxSelect!
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub description: Option<String>,
  }

  #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
  pub struct RunbookEntryDto {
      pub id: String,
      pub title: String,
      pub description: String,
      pub source_path: String, // workspace-relative forward-slash path
      pub kind: String,        // "vox_script" | "markdown_runbook"
      pub command_template: String,
      pub params: Vec<RunbookParamDto>,
  }
  ```
- **Placeholder Extraction Regex / Parser (Zero Regex Crate Needed — Pure `str` Scanner)**:
  - Extracts both `{{param_name}}` / `{{param_name=default}}` / `{{param_name:opt1|opt2|opt3}}` AND `${PARAM_NAME:-default}` / `${PARAM_NAME:default}` placeholders in order of first appearance (deduplicated by `name`).
  - For `scripts/<name>.vox` without explicit `// @runbook-cmd:` annotation:
    - Reads header `//` lines (first 25 lines): extracts `// Usage: <cmd>` if present (converting `<arg>` placeholders into `{{arg}}`), otherwise defaults `command_template` to `vox run scripts/<name>.vox`.
    - Extracts first descriptive comment paragraph as `description` and humanizes `<name>` as `title`.
  - Caps scan at `<= 100` files total and `<= 64 KiB` read per file (`std::io::Read::take(65_536)`) so `term_runbooks_list` always completes in `< 15ms`.
- **Shell-Safe Parameter Substitution (`substituteRunbookParams` in Frontend)**:
  - When filling `{{param}}` into a shell command template, strip `\r`/`\n`/`\0` and quote values containing spaces or shell metacharacters (`;`, `|`, `&`, `$`, `` ` ``, `(`, `)`, `>`, `<`) using POSIX single-quoting `'val'\''more'` (unless the user clicks **Insert into Editor** to inspect/edit before running).

### 3.2 `@tanstack/react-table` v9 Exact API for `JsonLedgerTable.tsx`
- **Verified Codebase Pattern (`crates/vox-gui/ui/src/components/ui/DataTable.tsx`)**:
  - `[VERIFIED: crates/vox-gui/ui/src/components/ui/DataTable.tsx:2-10]` —
    ```ts
    import {
      createSortedRowModel,
      rowSortingFeature,
      sortFn_alphanumeric,
      tableFeatures,
      useTable,
      type ColumnDef as TanStackColumnDef,
      type SortingState,
    } from '@tanstack/react-table';
    ```
  - `[VERIFIED: crates/vox-gui/ui/src/components/ui/DataTable.tsx:41-45]` —
    ```ts
    const DATA_TABLE_FEATURES = tableFeatures({
      rowSortingFeature,
      sortedRowModel: createSortedRowModel(),
      sortFns: { alphanumeric: sortFn_alphanumeric },
    });
    ```
  - `[VERIFIED: crates/vox-gui/ui/src/components/ui/DataTable.tsx:77-84]` —
    ```ts
    const table = useTable({
      features: DATA_TABLE_FEATURES,
      data: rows,
      columns: tanstackColumns,
      getRowId,
      state: { sorting },
      onSortingChange: setSorting,
    });
    ```
- **`JsonLedgerTable.tsx` Shape & View Mode Rules**:
  - **Shape Classification (`classifyStructuredJson(parsed: unknown)`)**:
    1. **Tabular Array (`kind: 'table'`)**: `Array.isArray(parsed) && parsed.length > 0 && parsed.every(item => item !== null && typeof item === 'object' && !Array.isArray(item))` — defaults to **`Table`** mode (`Table` | `JSON` | `Raw` toggle buttons in `BlockCard`). Union of keys across first `100` rows (capped at `24` columns; nested objects/arrays rendered as compact `JSON.stringify(val)` cells).
    2. **Object or Scalar Array (`kind: 'json'`)**: Top-level `{...}` object (e.g., `{"status":"ok"}` in `BlockStreamView.test.tsx:111`) or array of primitives — defaults to **`JSON`** mode (`Table` | `JSON` | `Raw` toggle; in `Table` mode, renders a 2-column `Key` / `Value` ledger for `{...}` objects so every structured JSON block can still be inspected as a table on click!).
  - **CRITICAL Backward Compatibility with `BlockStreamView.test.tsx:92-136`!!**:
    - Look at `[VERIFIED: crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx:100-136]`:
      - Block 10: `input: '!nu ls | to json'`, `output: '[{"name":"src","type":"dir"}]'` -> asserts `expect(screen.getByText(/"name": "src"/)).toBeTruthy();`!!
      - Block 11: `input: 'curl http://localhost/api'`, `output: '{"status":"ok"}'` -> asserts `expect(screen.getByText(/"status": "ok"/)).toBeTruthy();`!!
    - **HOLD ON — READ THAT TEST ASSERTION TWICE!!**
      - `BlockStreamView.test.tsx:134` tests `BlockStreamView` with `[{"name":"src","type":"dir"}]` and explicitly asserts `expect(screen.getByText(/"name": "src"/)).toBeTruthy()` on initial render, OR if `Table` is the default for arrays of objects, let's check!
      - If `Block 10` (`[{"name":"src","type":"dir"}]`) defaults to `Table` and only renders `<td>src</td><td>dir</td>`, then `screen.getByText(/"name": "src"/)` in `BlockStreamView.test.tsx:134` WOULD FAIL unless either:
        - (a) `BlockStreamView.test.tsx:134` is updated alongside the new `Table` default test (Wait: in Phase 444 we ARE writing/updating tests in `BlockStreamView.test.tsx`, OR even better: check `444-CONTEXT.md` -> `"offer a toggle (Table | JSON | Raw, defaulting to Table for non-empty arrays of uniform/semi-uniform objects and JSON tree/formatted view for scalar/nested objects)"` — updating `BlockStreamView.test.tsx` to assert `Table` default on `[{"name":"src","type":"dir"}]` and clicking `JSON` tab to see `"name": "src"`, OR keeping a collapsed `<code className="sr-only">`? No — `sr-only` duplicates text for screen readers; updating `BlockStreamView.test.tsx:134` to test the new `Table | JSON | Raw` toggle explicitly is honest and matches the locked decision in `444-CONTEXT.md`!). Calling this out explicitly in `Common Pitfalls` saves the executor an immediate surprise!
  - **Filter Input & Unique `textbox` Rule**:
    - Only mount the filter `<input type="search" aria-label="Filter table rows" ...>` when a structured block is actually rendered in `Table` mode (remember `<input type="search">` has ARIA role **`searchbox`**, NOT `textbox`! Using `type="search"` gives it implicit role `searchbox` in ARIA / Testing Library, so even when a structured block IS rendered in Console, `screen.getByRole('textbox')` STILL uniquely resolves to the Console prompt editor!). That's a huge win!

---

## Deep Technical Verification 4: Subshell "Voxify" (`TERM-WARP-04`)

### 4.1 Subshell Detection (`detect_subshell_command` + Prompt Heuristics)
- **Deterministic Command Detector (`vox_terminal_core::voxify::detect_subshell_entry(cmd: &str) -> Option<SubshellHint>`)** (mirrored in TS `intentRouter.ts` for instant UI banner feedback):
  - Strips leading env assignments (`FOO=bar`) and `sudo` flags (`sudo -i`, `sudo -s`, `sudo su`), then matches:
    - `ssh ...` (when no trailing remote one-shot command is passed, or `-t` interactive flag is present) -> `ShellKindHint::PosixAuto`
    - `docker exec ... -it|-ti|-i ... <container> [sh|bash|zsh|ash|fish|pwsh|nu]` -> maps trailing shell arg or `PosixAuto`
    - `kubectl exec ... -it|-ti ...` / `podman exec -it ...` -> maps trailing shell arg or `PosixAuto`
    - `nix develop` / `nix-shell` / `direnv` subshells / `distrobox enter` / `devcontainer exec` -> `PosixAuto`
    - `sudo -i` / `sudo -s` / `su` / `su -` -> `PosixAuto`
    - Bare interactive shell invocation (`bash`, `zsh`, `fish`, `pwsh`, `powershell`, `nu`, `/bin/sh`, `/bin/bash`, `/bin/zsh`) with no `-c` / script file argument -> exact `ShellKindHint`!
- **Runtime Signal in `TerminalSessionPane`**:
  - When a block's input matches `detectSubshellCommand(block.input)` AND `block.status === 'Running' || block.status === 'AwaitingInput'` (because inside a nested un-voxified subshell, the outer shell's `633;D` postexec hasn't fired yet — the outer shell is waiting on the child subshell!), surface a compact Roman banner/action (`Subshell detected — Voxify Subshell` with `data-testid="term-voxify-subshell"` and optional shell selector `auto | zsh | bash | fish | pwsh | nu` via `VoxSelect`), PLUS an always-reachable `Voxify Subshell` button in `ConsoleSessionBar` (or pane header) whenever a session is active!

### 4.2 Compact Non-Echo 5-Shell Hook Payload (`vox_terminal_core::voxify`)
- Why naive multi-line raw string injection breaks nested shells:
  - Pasting raw multi-line shell functions directly into a PTY triggers readline/zle echo, bracketed-paste noise, tab auto-indent mangling, and pollutes the nested shell's history file.
- **Clean Base64 Non-Echo Injection Pattern**:
  - `vox-terminal-core` already depends on `base64` (`[VERIFIED: crates/vox-terminal-core/Cargo.toml:21]`).
  - **POSIX Auto (`auto` / `zsh` / `bash` / `sh`)**:
    - Encode a compact unified POSIX snippet (checking `if [ -n "${ZSH_VERSION:-}" ]; then ... elif [ -n "${BASH_VERSION:-}" ]; then ... fi`) that installs `__vox_osc` emitting `OSC 7` (`\033]7;file://%s%s\007`), `OSC 633;D;%s`, `OSC 633;P;Cwd=%s`, `OSC 633;P;GitBranch=%s`, `OSC 633;A`, `OSC 633;B`, and `OSC 633;C` (reusing the exact proven `$?`-preserving hook logic from `[VERIFIED: crates/vox-terminal-core/src/shell_init.rs:282-510]`).
    - Emit a **single leading-space line** (`HISTCONTROL=ignorespace` / `HIST_IGNORE_SPACE` safe):
      ```sh
       stty -echo 2>/dev/null; eval "$(printf '%s' '<B64>' | base64 -d 2>/dev/null || printf '%s' '<B64>' | base64 -D 2>/dev/null)"; stty echo 2>/dev/null; printf '\r\033[2K'
      ```
      followed by `\n`.
    - Works identically on Linux (GNU `base64 -d` / BusyBox `base64 -d` inside Alpine Docker containers!) and macOS (BSD `base64 -D` / `-d`), suppresses echo during evaluation, clears the line, and immediately triggers the next prompt's `OSC 633;D` + `OSC 7` + `OSC 633;A`!
  - **Explicit `fish` / `pwsh` / `nu` Hints**:
    - `fish`: single-line leading-space ` printf '%s' '<B64>' | base64 -d | source; commandline -f repaint\n`.
    - `pwsh`: single-line `[System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('<B64>')) | Invoke-Expression; Clear-Host\r\n`.
    - `nu`: compact `$env.config.hooks` inline assignment terminated by `\n`.

### 4.3 `SessionHost` -> `DaemonHost` -> `TermDispatch` -> `vox-gui` Plumbing
- **`crates/vox-terminal-core/src/host.rs`**:
  - Add `async fn voxify_subshell(&self, id: &str, shell_hint: Option<&str>) -> Result<()>` to `SessionHost` (implemented on `LocalHost` by looking up the session, generating `voxify::voxify_payload(shell_hint)`, and calling `s.write_input(payload.as_bytes())`).
- **`crates/vox-terminal-core/src/daemon_host.rs`**:
  - Add wire constant `pub const VOXIFY: &str = "term.voxify";` (`crates/vox-terminal-core/src/daemon_host.rs:29-42`).
  - Implement `DaemonHost::voxify_subshell` sending `serde_json::json!({ "session_id": id, "shell_hint": shell_hint })`.
- **`crates/vox-orchestrator-d/src/term_dispatch.rs`**:
  - Handle `methods::VOXIFY` in `TermDispatch::try_handle`:
    - Call `refuse_agent_origin(p)?;` (`[VERIFIED: crates/vox-orchestrator-d/src/term_dispatch.rs:64-72]` — blocks agent-originated wire spoofing!).
    - Call `self.host.voxify_subshell(sid, shell_hint).await` and return `json!({ "ok": true })`.
- **`crates/vox-gui/src/commands/pty/warp.rs` & `main.rs`**:
  - Expose `#[tauri::command] pub async fn term_session_voxify(manager: State<'_, PtyManager>, daemon: State<'_, Arc<PersistentDaemon>>, session_id: String, shell_hint: Option<String>) -> Result<(), String>`.

---

## Deep Technical Verification 5: Tauri IPC & `remote::gate` Module Architecture

### 5.1 Why `commands::pty` Submodule Re-Export Is Mandatory
- `[VERIFIED: crates/vox-cli-ci/src/gui_command_backends.rs:46-48]` —
  ```rust
  if !modules_on_disk.contains(module) {
      out.push(format!(
          "{module}: listed in {MANIFEST_PATH} but {COMMANDS_DIR}/{module}.rs does not exist"
      ));
  }
  ```
- `[VERIFIED: crates/vox-cli-ci/src/gui_command_backends.rs:107-115]` —
  ```rust
  if path.extension().and_then(|e| e.to_str()) != Some("rs") {
      continue;
  }
  if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
      && stem != "mod"
  {
      out.insert(stem.to_string());
  }
  ```
- `[VERIFIED: crates/vox-gui/src/main.rs:9-18]` —
  ```rust
  macro_rules! axis_invoke_handler {
      ($($root:ident :: $module:ident :: $cmd:ident),* $(,)?) => {{
          const COMMAND_MODULES: &remote::gate::CommandTable =
              &[$((stringify!($root), stringify!($module), stringify!($cmd))),*];
          let inner = remote::gate::typed_handler(tauri::generate_handler![$($root::$module::$cmd),*]);
          move |invoke: tauri::ipc::Invoke<tauri::Wry>| {
              remote::gate::gate_invoke(COMMAND_MODULES, invoke, &inner)
          }
      }};
  }
  ```
- **Exact Rust Wiring**:
  1. Keep `crates/vox-gui/src/commands/pty.rs` on disk (`modules_on_disk` sees `"pty"`).
  2. Create `crates/vox-gui/src/commands/pty/warp.rs` (`modules_on_disk` ignores directory `crates/vox-gui/src/commands/pty/` because `path.extension() != Some("rs")`).
  3. In `crates/vox-gui/src/commands/pty.rs`, add:
     ```rust
     mod warp;
     pub use warp::*;
     ```
     And add `history: Arc<vox_terminal_core::history::HistoryStore>` + `completions: Arc<vox_terminal_core::completions::CompletionEngine>` fields on `PtyManager` (initialized lazily or in `PtyManager::default()` pointing at `vox_config::paths::user_home_dir().join(".vox").join("terminal").join("history.jsonl")`, with an in-memory constructor `PtyManager::new_for_test()` so unit tests never touch `~/.vox`!).
  4. In `crates/vox-gui/src/main.rs`, register under `commands::pty::*`:
     - `commands::pty::term_history_record`
     - `commands::pty::term_history_query`
     - `commands::pty::term_complete`
     - `commands::pty::term_runbooks_list`
     - `commands::pty::term_session_voxify`
  5. `stringify!($module)` evaluates to `"pty"`, which `contracts/gui/command-backends.v1.yaml:65` already classifies as `{ module: pty, class: host, surfaces: [console] }`. In remote mode, `remote::gate` automatically blocks all 5 commands with `local_only` (`UnavailableReason::NeedsLocalMachine`) — zero contract churn and 100% fixture parity!

---

## Deep Technical Verification 6: Frontend `< 400` LoC Module Decomposition & Guard Compatibility

### 6.1 File-by-File Decomposition & Line-Count Budgets

| File Path | Role | Est. Non-Blank LoC | Max Budget |
|-----------|------|--------------------|------------|
| `crates/vox-terminal-core/src/history.rs` | `HistoryStore`, `ignorespace`, Stage 1+2(closure)+3 scrubber, JSONL atomic write, subsequence+recency+`cwd` ranker + unit tests | ~210 | `< 400` |
| `crates/vox-terminal-core/src/completions.rs` | `CompletionEngine`, `$PATH` TTL cache, bounded `cwd` path completer, pure-Rust `.git` ref/worktree reader + unit tests | ~260 | `< 400` |
| `crates/vox-terminal-core/src/completions_specs.rs` | Curated subcommand/flag tables for `git`, `cargo`, `pnpm`, `npm`, `docker`, `kubectl`, `gh`, `rg` + unit tests | ~180 | `< 400` |
| `crates/vox-terminal-core/src/runbooks.rs` | Bounded workspace scanner (`scripts/*.vox`, `.vox/runbooks/*.{vox,md}`, tagged `docs/src/how-to/*.md`), `{{param}}` / `${PARAM}` parser + unit tests | ~220 | `< 400` |
| `crates/vox-terminal-core/src/voxify.rs` | Subshell command detector + base64 non-echo 5-shell (`auto`/`zsh`/`bash`/`fish`/`pwsh`/`nu`) `OSC 633`+`OSC 7` hook payloads + unit tests | ~180 | `< 400` |
| `crates/vox-gui/src/commands/pty/warp.rs` | 5 `#[tauri::command]` handlers (`term_history_record`, `term_history_query`, `term_complete`, `term_runbooks_list`, `term_session_voxify`) + Clavis Stage 2 scrubber + unit tests | ~180 | `< 400` |
| `crates/vox-gui/ui/src/components/surfaces/Console/promptHighlighter.ts` | Deterministic shell/Vox tokenizer (`command`, `pipe`, `operator`, `redirect`, `flag`, `env`, `string`, `mention`, `comment`) + syntax completeness check (`isLineSyntacticallyComplete`) | ~160 | `< 400` |
| `crates/vox-gui/ui/src/components/surfaces/Console/intentRouter.ts` | Prefix (`!`, `?`, `/ask`, `/`) + natural-language prose heuristic (`classifyPromptIntent`) + subshell entry detector (`detectSubshellEntry`) | ~140 | `< 400` |
| `crates/vox-gui/ui/src/components/surfaces/Console/terminalCompletions.ts` | Merges `termComplete` (Rust `$PATH`/path/git/CLI) + `discoverySuggest` (`vox` CLI) + history prefix ghost fallback into `CompletionEntry[]` | ~170 | `< 400` |
| `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx` | `Ctrl+R`/`Cmd+R` fuzzy history popover (unmounted `null` when closed; client-side scrubbed `localStorage` fallback when `no-tauri`) | ~190 | `< 400` |
| `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx` | Auto-growing `<textarea>` (max 8 rows), synchronized `<pre aria-hidden="true">` highlight + `data-testid="ghost"` overlay, multi-line keymap, AI Assist intent chip | ~260 | `< 400` |
| `crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.tsx` | Thin backwards-compatible stateful facade wiring `ScriptoriumPromptEditor`, `IntelliSensePopover`, `HistorySearchPopover`, and `term-shell-mode-select` | ~170 | `< 400` |
| `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx` | `@tanstack/react-table` v9 (`tableFeatures` + `useTable`) sortable/filterable table + `Table \| JSON \| Raw` mode bar + CSV/JSON copy/download | ~230 | `< 400` |
| `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx` | Workspace runbooks list, `type="search"` filter, `VoxInput`/`VoxSelect` parameter form, `Insert into Editor` & `Run in Active Pane` | ~220 | `< 400` |

### 6.2 Preserving Every Existing Console & InputEditor Test Landmark
- Inspecting `[VERIFIED: crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.tsx:240-285]`, `[VERIFIED: crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.test.tsx:16-92]`, and `[VERIFIED: crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx:87-230]`, the following DOM contracts MUST remain intact:
  1. `screen.getByRole('textbox')`:
     - `<textarea rows={1} aria-label="console input" aria-autocomplete="inline" ... />` has implicit ARIA role `textbox`.
     - `HistorySearchPopover` MUST `return null` when `!open`.
     - `DiscoveryRail` defaults to the `Help` tab (`activeTab === 'help'`), so `RunbookPanel` is unmounted on initial `<Console />` render. Even when `RunbookPanel` mounts, its search box uses `type="search"` (`role="searchbox"`), and parameter `VoxInput` fields only render when a parameterized runbook is expanded!
     - `JsonLedgerTable` filter input uses `<input type="search" aria-label="Filter table rows">` (`role="searchbox"`), never `role="textbox"`.
  2. `data-testid="term-shell-mode-select"` (`VoxSelect` with `default`, `nu`, `vox`) and `data-testid="console-input-mode-badge"`.
  3. `data-testid="ghost"` (`<span data-testid="ghost">` rendering the inline completion remainder after the typed text, accepted on `Tab` or `ArrowRight` at EOL).
  4. `<div role="status" aria-live="polite" className="sr-only">` announcing `suggestion: ${active.completion}` (`InputEditor.test.tsx:41-43`).
  5. `IntelliSensePopover` test IDs (`data-testid="intellisense-popover"`, `data-testid="intellisense-tab-all"`, `data-testid="intellisense-option"`, `data-testid="intellisense-footer"`):
     - Extend `CompletionCategory` in `intellisenseEngine.ts` cleanly (keeping `'all' | 'command' | 'flag' | 'subcommand' | 'arg'` and adding optional `'path' | 'git' | 'history'` tabs or mapping `git_branch -> 'arg'`, `path -> 'arg'`, `vox_cli -> 'command'` while showing a crisp source badge `PATH` / `GIT` / `FS` / `VOX` / `HIST` on each `intellisense-option` row!). Keeping the category tabs concise avoids horizontal overflow in the `max-w-lg` popover while displaying exact per-item kind badges.
  6. Intent Routing Behavior on `Enter` vs `InputEditor.test.tsx:70-75`:
     - **WAIT — LOOK AT `InputEditor.test.tsx:70-75`!**
       - `[VERIFIED: crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.test.tsx:70-75]` —
         ```ts
         // ? agent command: no !nu prefix
         onSubmit.mockClear();
         fireEvent.change(input, { target: { value: '?how to parse json' } });
         fireEvent.keyDown(input, { key: 'Enter' });
         expect(onSubmit).toHaveBeenCalledWith('?how to parse json');
         ```
       - **CRITICAL CATCH!!** `InputEditor.test.tsx:74` asserts that pressing `Enter` on `'?how to parse json'` inside standalone `<InputEditor onSubmit={onSubmit} />` calls `onSubmit('?how to parse json')` (without `!nu` prefix)!
       - Therefore, **do NOT swallow `?prompt` inside `InputEditor` before calling `onSubmit` unless an optional `onRouteToAgent?: (prompt: string) => void` callback is provided by `Console.tsx`**!
       - Even cleaner:
         - In standalone `<InputEditor onSubmit={onSubmit} />` (where `onRouteToAgent` is undefined), `Enter` calls `onSubmit('?how to parse json')` — keeping `InputEditor.test.tsx:74` 100% green!
         - In `Console.tsx:94` (`submit`), or via `onRouteToAgent` passed by `Console.tsx`, if `line` starts with `?` or `/ask `: strip the `?` / `/ask ` prefix, attach `@term:${activeSessionId ?? 's-1'}` context via `requestChatAttention(...)`, and toast `'Routed to Agent'` instead of sending `?how to parse json` as a broken shell command to bash/zsh!
         - And for `!cmd` explicit shell override: in `nu` mode, `!ls` strips the leading `!` (or bypasses `!nu` prefixing when `!cmd` is not `!nu ...`) so `!git status` runs in the native shell!
         - And for natural-language prose heuristic (e.g. `how do I list open ports on mac` — 3+ words, no leading `!`/`/`/`?`, first word not in known commands/builtins/paths, no shell operators `|`/`&&`/`>`/`$`/`-` flags): show a non-blocking `AI Assist` pill (`data-testid="prompt-ai-assist-chip"`, `"Looks like a question — click or Shift+Tab to ask Agent, Enter to run in shell"`), so `Enter` NEVER hijacks valid shell commands while giving 1-click/keystroke agent routing!

### 6.3 `Omnibar` (`Cmd+K` / `Cmd+P`) Integration for Runbooks
- Let's check how `App.tsx` and `keybinds.ts` open the palette:
  - `[VERIFIED: crates/vox-gui/ui/src/lib/keybinds.ts:20]` — `'open-palette': 'Mod+K'`.
  - In `Console.tsx:129-165`, `Cmd+P` (`(e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'p'`) inside Console can open/expand the `DiscoveryRail` directly onto the **`Runbooks`** tab (and focus the runbooks search box)! Note that bare `'p'` (`!isTyping && !e.metaKey && !e.ctrlKey`) pins the focused block (`Console.tsx:156`), which currently checks `else if (!isTyping && target)` AFTER `metaKey || ctrlKey` checks — adding `else if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'p')` right above line 154 cleanly binds `Cmd+P` / `Ctrl+P` in Console to open Runbooks without colliding with bare `p` (pin)!
  - Additionally, `RunbookPanel` or `Console` can register runbooks into `searchableRegistry` (`querySearchableRegistry` consumed by `Omnibar.tsx:319` -> `onScreen` facet!) or pass runbook commands via `requestTerminalCommand` (`terminalBridge.ts:70`), making runbooks searchable in **both** the global `Omnibar` (`Cmd+K`) AND `Cmd+P` / `DiscoveryRail` Runbooks tab!

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Secret / API key scrubbing in terminal history | Custom regex list in `history.rs` | `strip_ansi_and_normalize` + `vox_secrets::scrub_resolved_secrets` + `vox_redact::redact_owned` | Clavis resolves live user secrets from keyring/env/vault that static regexes cannot know; `vox-redact` covers structured token formats. |
| Table sorting model for JSON output | Custom column-sort state machine | `@tanstack/react-table` v9 (`tableFeatures`, `rowSortingFeature`, `createSortedRowModel`, `useTable`) | Already installed (`^9.3.1`) and proven in `components/ui/DataTable.tsx:41-84`; handles alphanumeric sorting and `aria-sort` cleanly. |
| Native `<select>` dropdowns for runbook enum params | `<select><option>` | `<VoxSelect>` (`components/ui/Select.tsx`) | Raw `<select>` fails `noRawNativeControlsGuard.test.ts` immediately. |
| Subprocess `git branch -a` per keystroke | `std::process::Command::new("git")` in `term_complete` | Pure-Rust `.git/HEAD` + `refs/{heads,tags}` + `packed-refs` + worktree `gitdir:` reader | Zero process spawn latency (`< 0.5ms` vs `20–80ms`), never contends on `.git/index.lock`, safe on Windows without flashing console handles. |
| Base64 encoder for `voxify` payload | Custom shell escaping of multi-line zsh/bash functions | `base64::engine::general_purpose::STANDARD` (already in `vox-terminal-core/Cargo.toml`) | Eliminates all single/double quote, `$?`, backtick, and readline auto-indent corruption across nested `ssh` / `docker exec` TTYs. |

---

## Common Pitfalls

### Pitfall 1: Importing `useReactTable` or `getCoreRowModel` from `@tanstack/react-table`
- **What goes wrong:** `tsc --noEmit` fails with `Module '"@tanstack/react-table"' has no exported member 'useReactTable'`.
- **Why it happens:** `package.json` ships `@tanstack/react-table@^9.3.1` (alpha/v9 architecture), where `useReactTable` was replaced by `tableFeatures(...)` + `useTable(...)`.
- **How to avoid:** Copy the exact import and feature declaration from `crates/vox-gui/ui/src/components/ui/DataTable.tsx:2-10, 41-45, 77-84`.
- **Warning signs:** Any import of `useReactTable`, `getCoreRowModel`, `getSortedRowModel`, or `getFilteredRowModel`.

### Pitfall 2: Breaking `gui_command_backends.rs` or `god_object.rs` When Adding Tauri Commands
- **What goes wrong:** Either `crates/vox-gui/src/commands/pty.rs` exceeds `500` non-blank lines (`vox-code-audit` `arch/god_object` error), OR moving `pty.rs` to `pty/mod.rs` breaks `gui_command_backends.rs:47` (`pty.rs does not exist`), OR adding `commands/term_warp.rs` breaks `gui_command_backends.rs:72` (`is not classified in command-backends.v1.yaml`).
- **Why it happens:** `gui_command_backends.rs` checks `crates/vox-gui/src/commands/<module>.rs` directly on disk while `pty.rs` is already near the 500-line ceiling.
- **How to avoid:** Keep `crates/vox-gui/src/commands/pty.rs` as a file, put the 5 new commands in `crates/vox-gui/src/commands/pty/warp.rs`, re-export via `mod warp; pub use warp::*;` in `pty.rs`, and register `commands::pty::term_*` in `main.rs`.

### Pitfall 3: Mounting Extra `role="textbox"` Inputs on Initial `<Console />` Render
- **What goes wrong:** `Console.test.tsx:91` (`expect(screen.getByRole('textbox')).toBeTruthy()`) throws `TestingLibraryElementError: Found multiple elements with the role "textbox"`.
- **Why it happens:** `<input type="text">` and `<textarea>` both expose ARIA role `textbox`. If `HistorySearchPopover` or `RunbookPanel` renders a hidden `<input type="text">` while closed, `getByRole('textbox')` finds 2+ elements.
- **How to avoid:**
  1. Unmount (`return null`) `HistorySearchPopover` when `!open` and `RunbookPanel` when `DiscoveryRail` is on its default `Help` tab.
  2. Use `<input type="search">` (`role="searchbox"`, NOT `textbox`) for filter inputs in `JsonLedgerTable` and `RunbookPanel`.

### Pitfall 4: Breaking `BlockStreamView.test.tsx:134` When Defaulting JSON Arrays to `Table` View
- **What goes wrong:** `BlockStreamView.test.tsx:92-136` renders `[{"name":"src","type":"dir"}]` and asserts `expect(screen.getByText(/"name": "src"/)).toBeTruthy()`. When `BlockCard` defaults JSON object arrays (`[{...}]`) to `JsonLedgerTable`'s `Table` view (`TERM-WARP-03`), `<td>src</td>` is rendered instead of raw `"name": "src"` until the user clicks the `JSON` toggle!
- **Why it happens:** Existing Phase 443 test checked pre-table `<pre>{formattedStructured}</pre>` rendering for both arrays and objects.
- **How to avoid:** Update `BlockStreamView.test.tsx:92-136` in Plan 444-04 so the array-of-objects case (`[{"name":"src","type":"dir"}]`) verifies the default `Table` view (`src`, `dir` cells + `Table`/`JSON`/`Raw` toggle, then clicking `JSON` reveals `"name": "src"`), while the single-object case (`{"status":"ok"}`) still defaults to `JSON` view (`"status": "ok"`).

### Pitfall 5: Unicode Glyphs (`↵`, `⏎`, `⇥`, `▸`) Tripping `zeroEmojiAndDingbat.test.ts`
- **What goes wrong:** Keycap hints like `"↵ Run"` or `"⇥ Complete"` in `ScriptoriumPromptEditor` or `HistorySearchPopover` fail `zeroEmojiAndDingbat.test.ts`.
- **Why it happens:** `BANNED_GLYPH_RE` explicitly bans `\u21B5` (`↵`), `\u23CE` (`⏎`), `\u21E5` (`⇥`), and `\u25A0-\u25FF` geometric triangles/squares (`zeroEmojiAndDingbat.test.ts:6`).
- **How to avoid:** Always write plain ASCII key labels (`Enter`, `Shift+Enter`, `Tab`, `Ctrl+R`, `Esc`) or use `<KeycapIcon>` / `<Icon.*>` / `<VoxIcon>`.

### Pitfall 6: Unmocked New `transport.ts` Functions in `Console.test.tsx` / `InputEditor.test.tsx`
- **What goes wrong:** `Console.test.tsx` mocks `../../../transport` with a fixed object literal (`L11-40`). Calling `transport.termComplete(...)` unconditionally throws `TypeError: (0 , transport.termComplete) is not a function` in Vitest.
- **How to avoid:** Both (a) guard calls via `typeof transport.termComplete === 'function'` (matching `TerminalSessionPane.tsx:140, 177, 201`) AND (b) add default mocks for the 5 new functions to `Console.test.tsx` when adding Phase 444 integration tests.

---

## Code Examples

### Example 1: Zero-Crate-Edge 3-Stage `HistoryStore::record_with_scrubber` (`crates/vox-terminal-core/src/history.rs`)
```rust
impl HistoryStore {
    pub fn record_with_scrubber<F>(
        &self,
        raw_command: &str,
        cwd: Option<&str>,
        exit_code: Option<i32>,
        now_ms: u64,
        extra_scrub: F,
    ) -> anyhow::Result<bool>
    where
        F: FnOnce(&str) -> String,
    {
        // HISTCONTROL=ignorespace: leading space or tab skips persistence completely.
        if raw_command.starts_with(' ') || raw_command.starts_with('\t') {
            return Ok(false);
        }
        let stage1 = crate::block::strip_ansi_and_normalize(raw_command);
        let trimmed = stage1.trim();
        if trimmed.is_empty() {
            return Ok(false);
        }
        let stage2 = extra_scrub(trimmed);
        let scrubbed = vox_redact::redact_owned(&stage2).into_owned();
        if scrubbed.trim().is_empty() {
            return Ok(false);
        }
        self.insert_and_persist(scrubbed, cwd.map(str::to_owned), exit_code, now_ms)?;
        Ok(true)
    }
}
```

### Example 2: Pure-Rust Worktree-Aware `.git` Ref Discovery (`crates/vox-terminal-core/src/completions.rs`)
```rust
fn resolve_git_dirs(start_cwd: &Path) -> Option<(PathBuf, PathBuf)> {
    let mut cur = Some(start_cwd);
    for _ in 0..16 {
        let dir = cur?;
        let dot_git = dir.join(".git");
        if dot_git.is_dir() {
            return Some((dot_git.clone(), dot_git));
        }
        if dot_git.is_file() {
            let text = std::fs::read_to_string(&dot_git).ok()?;
            let raw_gitdir = text.lines().next()?.strip_prefix("gitdir:")?.trim();
            let git_dir = if Path::new(raw_gitdir).is_absolute() {
                PathBuf::from(raw_gitdir)
            } else {
                dir.join(raw_gitdir)
            };
            let common_dir = std::fs::read_to_string(git_dir.join("commondir"))
                .ok()
                .map(|c| git_dir.join(c.trim()))
                .unwrap_or_else(|| git_dir.clone());
            return Some((git_dir, common_dir));
        }
        cur = dir.parent();
    }
    None
}
```

### Example 3: `@tanstack/react-table` v9 Setup for `JsonLedgerTable.tsx`
```tsx
// Follows verified pattern in crates/vox-gui/ui/src/components/ui/DataTable.tsx:2-10, 41-45, 77-84
import {
  createSortedRowModel,
  rowSortingFeature,
  sortFn_alphanumeric,
  tableFeatures,
  useTable,
  type ColumnDef as TanStackColumnDef,
  type SortingState,
} from '@tanstack/react-table';

const LEDGER_FEATURES = tableFeatures({
  rowSortingFeature,
  sortedRowModel: createSortedRowModel(),
  sortFns: { alphanumeric: sortFn_alphanumeric },
});
```

### Example 4: Synchronized `<textarea>` + Token Highlight Overlay (`ScriptoriumPromptEditor.tsx`)
```tsx
// <textarea> owns input/selection/ARIA role="textbox"; <pre aria-hidden="true"> renders colored spans + ghost suffix.
<div className="relative min-w-0 flex-1 font-mono text-xs leading-[1.45]" style={TERMINAL_MONO_STYLE}>
  <pre
    ref={overlayRef}
    aria-hidden="true"
    className="pointer-events-none m-0 max-h-[11.6rem] overflow-hidden whitespace-pre-wrap break-words px-2 py-1 text-xs leading-[1.45]"
    style={TERMINAL_MONO_STYLE}
  >
    {tokens.map((tok, idx) => (
      <span key={idx} className={TOKEN_CLASS[tok.kind]}>{tok.text}</span>
    ))}
    {ghostSuffix ? (
      <span data-testid="ghost" className="text-text-muted opacity-60">{ghostSuffix}</span>
    ) : null}
    {value.endsWith('\n') ? '\n ' : null}
  </pre>
  <textarea
    ref={textareaRef}
    rows={visibleRows}
    aria-label="console input"
    aria-autocomplete="inline"
    spellCheck={false}
    value={value}
    onScroll={(e) => {
      if (overlayRef.current) {
        overlayRef.current.scrollTop = e.currentTarget.scrollTop;
      }
    }}
    onChange={(e) => onChange(e.target.value, e.target.selectionStart ?? e.target.value.length)}
    onKeyDown={handleKeyDown}
    placeholder={shellMode === 'nu' ? 'Type a Nushell command (e.g. ls | where size > 10kb)...' : 'Type a command (Tab to complete, Shift+Enter for newline, Ctrl+R for history)...'}
    className="absolute inset-0 h-full w-full resize-none overflow-y-auto bg-transparent px-2 py-1 text-transparent caret-text-primary outline-none placeholder:text-text-muted"
    style={TERMINAL_MONO_STYLE}
  />
</div>
```

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | Local desktop Tauri IPC / loopback daemon (`remote::gate` blocks `pty` module with `local_only` when connected to a remote gateway). |
| V3 Session Management | Yes | `TermDispatch` enforces `refuse_agent_origin(params)` on `term.voxify` so agent sessions cannot spoof operator PTY hook injection. |
| V4 Access Control | Yes | New Tauri commands live in `commands::pty` (`ModuleClass::Host` in `contracts/gui/command-backends.v1.yaml`), preventing remote gateway exposure; path & runbook scanners enforce canonicalized root containment and depth/entry caps. |
| V5 Input Validation & Sanitization | Yes | Mandatory 3-stage secret/PII scrubbing (`strip_ansi_and_normalize` -> `vox_secrets::scrub_resolved_secrets` -> `vox_redact::redact_owned`) + `ignorespace` BEFORE history hits disk or `localStorage`; POSIX single-quote escaping on runbook `{{param}}` values; `sanitizeErrorForToast` on all UI error paths. |
| V6 Cryptography | No | No crypto primitives touched (`base64` is used strictly for shell-safe ASCII encoding of static hook snippets). |

### Known Threat Patterns for Terminal History, Completions, Runbooks & Voxify

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Plaintext API keys / tokens persisted in `~/.vox/terminal/history.jsonl` or `localStorage` | Information Disclosure | Mandatory `ignorespace` check + 3-stage scrubber (`strip_ansi_and_normalize` -> `vox_secrets::scrub_resolved_secrets` -> `vox_redact::redact_owned`) before JSONL write (`0600`/`0700` dir perms on Unix), plus client-side secret-pattern redaction on `no-tauri` `localStorage` fallback. |
| Path traversal or `/proc` / unbounded symlink walk in `term_complete` or `term_runbooks_list` | Information Disclosure / DoS | Cap directory reads (`<= 512` entries, `<= 50` returned items), reject NUL bytes, block `/proc`/`/sys`/`/dev` roots, cap `..` hops, and verify `canonicalize()` prefix stays under `workspace_root` for runbook discovery. |
| Shell command injection via Runbook `{{param}}` fill-in inputs | Tampering / Elevation of Privilege | Strip `\r`, `\n`, and `\0` from parameter values and wrap values containing whitespace or shell metacharacters in POSIX single quotes (`'...'` with `'\''` inner quote escaping) before executing in active pane. |
| Agent-spoofed `term.voxify` wire RPC injecting arbitrary bytes | Elevation of Privilege | `voxify_subshell` accepts only an enum/closed allowlist of `shell_hint` (`auto`, `zsh`, `bash`, `fish`, `pwsh`, `nu`) — NEVER arbitrary caller hook strings — and `TermDispatch` calls `refuse_agent_origin(p)?`. |

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Rust Unit & Integration Tests | `cargo test -p vox-terminal-core` & `cargo test -p vox-gui --lib` & `cargo test -p vox-orchestrator-d` |
| Frontend Unit & Component Tests | Vitest `^4.1.4` + `@testing-library/react` `^16.3.2` (`crates/vox-gui/ui`) |
| Typecheck | `pnpm --dir crates/vox-gui/ui exec tsc --noEmit` |
| Quick Run Command (Rust) | `timeout 300s cargo test -p vox-terminal-core` |
| Quick Run Command (UI) | `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/` |
| Static UI Guards Command | `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/guards/` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| `TERM-WARP-01` | Multi-line `<textarea>` auto-grow (max 8 rows), token syntax overlay, `Enter` syntax-completeness vs `Shift+Enter` newline, `Up`/`Down` boundary history, `!`/`?`/`/` + NL `AI Assist` intent routing | Vitest component + unit | `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/InputEditor.test.tsx src/components/surfaces/Console/ScriptoriumPromptEditor.test.tsx src/components/surfaces/Console/Console.test.tsx` | `InputEditor.test.tsx` & `Console.test.tsx` exist; `ScriptoriumPromptEditor.test.tsx` Wave 0/Plan 02 |
| `TERM-WARP-02` | `HistoryStore` 3-stage scrubber + `ignorespace` + subsequence/`cwd` ranker; `< 50ms` `CompletionEngine` (`$PATH`, `cwd`, pure-Rust `.git` + worktree, CLI specs); `Ctrl+R` `HistorySearchPopover` & merged `IntelliSensePopover` | Rust unit + Vitest component | `timeout 300s cargo test -p vox-terminal-core` and `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/HistorySearchPopover.test.tsx` | New Rust unit tests in `history.rs` / `completions.rs` + `HistorySearchPopover.test.tsx` |
| `TERM-WARP-03` | `JsonLedgerTable` (`@tanstack/react-table` v9 `Table \| JSON \| Raw`, column sort, filter, CSV/JSON export) + `vox_terminal_core::runbooks` scanner + `RunbookPanel` (`VoxInput`/`VoxSelect` params, Insert & Run) | Rust unit + Vitest component | `timeout 300s cargo test -p vox-terminal-core` and `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/BlockStreamView.test.tsx src/components/surfaces/Console/JsonLedgerTable.test.tsx src/components/surfaces/Console/RunbookPanel.test.tsx` | `BlockStreamView.test.tsx` exists; `JsonLedgerTable.test.tsx` & `RunbookPanel.test.tsx` new |
| `TERM-WARP-04` | Subshell command detection (`ssh`, `docker exec -it`, `kubectl exec -it`, `nix develop`, `sudo -i`, nested shells) + `SessionHost::voxify_subshell` / `TermDispatch` `term.voxify` (`refuse_agent_origin`) + `data-testid="term-voxify-subshell"` UI injection | Rust unit + Vitest component | `timeout 300s cargo test -p vox-terminal-core` and `timeout 300s cargo test -p vox-orchestrator-d term_dispatch` and `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/Console.test.tsx` | `term_dispatch.rs` & `Console.test.tsx` exist; `voxify.rs` tests new |

### Wave 0 Gaps
- None — Vitest, `@testing-library/react`, `jsdom`, `cargo test`, and all UI static guard suites already exist and run cleanly without external services. Each plan writes failing RED unit/component tests in its own Wave before implementation.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Re-exporting `#[tauri::command]` functions from `crates/vox-gui/src/commands/pty/warp.rs` via `pub use warp::*;` in `crates/vox-gui/src/commands/pty.rs` resolves `tauri::generate_handler![commands::pty::term_complete, ...]` cleanly in Tauri v2 (`__cmd__term_complete` is generated in the same module namespace as the function and glob re-exported by `pub use warp::*;`). | Deep Technical Verification 5.1 | LOW — standard Rust glob re-export behavior (`pub use` re-exports macros/items in the value/macro namespaces when `#[tauri::command]` emits `pub(crate) use __cmd__*`); verified immediately by `cargo check -p vox-gui` in Plan 444-01. If Tauri macro visibility requires `pub(crate) use warp::*;`, both are in the same 2-line block in `pty.rs`. |

**If this table is empty, confirm:** One low-risk compiler-verified item (`A1`) noted above; zero user-facing ambiguity remains.

---

## Open Questions (RESOLVED)

1. **Where does `~/.vox/terminal/history.jsonl` resolve its path without adding `vox-config` to `vox-terminal-core`?**
   - **Resolution:** `HistoryStore::open(path: impl Into<PathBuf>, cap: usize)` in `vox-terminal-core` accepts an explicit `PathBuf` (plus `HistoryStore::in_memory(cap)` for tests). `vox-gui` (`commands/pty.rs`) passes `vox_config::paths::user_home_dir().join(".vox").join("terminal").join("history.jsonl")`. Zero new crate edges and zero hardcoded `.vox` path literals in `vox-terminal-core`.
2. **How do we prevent `getByRole('textbox')` collisions in `Console.test.tsx`?**
   - **Resolution:** `<textarea aria-label="console input">` is the sole mounted `textbox` on initial render; `HistorySearchPopover` and `RunbookPanel` unmount (`return null`) when closed/inactive, and filter inputs use `type="search"` (`role="searchbox"`).
3. **How does `BlockStreamView.test.tsx:134` interact with `JsonLedgerTable` defaulting object arrays to `Table`?**
   - **Resolution:** Update `BlockStreamView.test.tsx:92-136` in Plan 444-04 to assert that `[{"name":"src","type":"dir"}]` defaults to `Table` view (`src` / `dir` cells) and switches to formatted JSON (`"name": "src"`) when clicking the `JSON` toggle button, while `{"status":"ok"}` continues to render formatted JSON by default.

---

## Sources

### Primary (HIGH confidence — Verified Direct Repo Reads)
- `crates/vox-terminal-core/Cargo.toml:13-28`, `src/lib.rs:1-62`, `src/block.rs:130`, `src/host.rs:1-240`, `src/daemon_host.rs:1-220`, `src/shell_init.rs:282-761`, `src/corpus/redact.rs:1-60`
- `crates/vox-orchestrator-d/src/term_dispatch.rs:64-72, 313-317`
- `crates/vox-gui/Cargo.toml:1-83`, `src/main.rs:9-18`, `src/commands/pty.rs:1-575`, `src/commands/discovery.rs:1-120`, `src/remote/gate.rs:1-146`, `src/remote/manifest.rs:336-390`
- `crates/vox-cli-ci/src/gui_command_backends.rs:46-118`
- `crates/vox-code-audit/src/detectors/god_object.rs:13-115`, `untested_pub_api.rs:13-55`
- `crates/vox-gui/ui/package.json:17-45`, `node_modules/@tanstack/react-table/dist/index.d.ts:1-7`
- `crates/vox-gui/ui/src/components/ui/DataTable.tsx:1-85`, `Command.tsx:1-158`, `IntelliSensePopover.tsx:1-151`
- `crates/vox-gui/ui/src/components/surfaces/Console/{Console.tsx,Console.test.tsx,InputEditor.tsx,InputEditor.test.tsx,DiscoveryRail.tsx,DiscoveryRail.test.tsx,BlockCard.tsx,BlockStreamView.tsx,BlockStreamView.test.tsx,ConsoleSessionBar.tsx,TerminalSessionPane.tsx}`
- `crates/vox-gui/ui/src/guards/{zeroEmojiAndDingbat.test.ts,noRawNativeControlsGuard.test.ts,transportIpcGuard.test.ts,ipcBoundaries.test.ts,toastBodyGuard.test.ts,noPaperFallbackGuard.test.ts}`

---

## Metadata

**Confidence breakdown:**
- Standard stack & `@tanstack/react-table` v9 API: **HIGH** — verified directly against `node_modules/@tanstack/react-table/dist/index.d.ts` and `DataTable.tsx`.
- Rust `vox-terminal-core` + `vox-gui` + `remote::gate` architecture: **HIGH** — verified directly against `Cargo.toml`, `gui_command_backends.rs`, `god_object.rs`, `manifest.rs`, and `term_dispatch.rs`.
- Frontend decomposition & guard/test compatibility: **HIGH** — verified against all Console test files and all 6 static guard suites.

**Research date:** 2026-11-15
**Valid until:** 2026-12-15
