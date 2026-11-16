# Phase 444: Warp-Grade Universal Multi-Line Editor, Context Completions, History & Local `.vox` Runbooks - Context

**Gathered:** 2026-11-15
**Status:** Ready for UI spec & planning
**Mode:** Autonomous Smart Discuss (`--ws axis-warp-terminal-v75`)

<domain>
## Phase Boundary

Deliver 95% Warp 2.0 operator ergonomics across Vox Axis `Console` (and compact `TerminalDockPanel`) + `vox-gui` / `vox-terminal-core`:
1. **`ScriptoriumPromptEditor` (`TERM-WARP-01`)**: Multi-line (`Shift+Enter`) syntax-highlighted Universal Input composer replacing the single-line `<input>` in `InputEditor.tsx` (retaining `InputEditor` export alias + `screen.getByRole('textbox')` uniqueness so existing tests stay 100% green), with automatic and lockable routing across `Shell`, `?` Agent (including natural-language prose heuristic), `!nu`, `.vox`, and `/slash` modes.
2. **Persistent Redacted Command History & `Ctrl+R` Fuzzy Recall (`TERM-WARP-02`)**: Backend-persisted `vox-redact` + Clavis-scrubbed workspace command history (`term_history_query` / `term_history_record` Tauri IPC + local fallback when running headless `no-tauri`) with `Up`/`Down` arrow recall (preserving multi-line cursor movement on inner lines, recalling history on first/last line or single-line input) and `Ctrl+R` fuzzy search popover.
3. **Universal `$PATH`, Path, Git & CLI-Flag Completions (`TERM-WARP-03`)**: Remove the `vox`-only prefix gate in `InputEditor.tsx:72-80` and add low-latency backend + frontend completions (`term_complete` Tauri IPC over `$PATH` executables, `cwd`-relative files/directories, `git` branches/tags, and common CLI subcommands/flags across `git`, `cargo`, `pnpm`, `docker`, `kubectl`, `gh`, `vox`, etc.).
4. **Parameterized `.vox` & Markdown Runbooks, Structured Ledger Table & 1-Click Subshell "Voxify" (`TERM-WARP-04`)**:
   - Discover and launch parameterized local git-tracked `.vox` scripts and runnable Markdown runbooks (`scripts/**/*.vox`, `docs/**/*.md` / `.vox/runbooks/**` with `{{param}}` or `--flag` parameter forms) from both `Cmd+P` (`CommandPalette`) and `DiscoveryRail`.
   - Upgrade structured JSON block rendering (`BlockCard.tsx` / `JsonLedgerTable.tsx` powered by already-installed `@tanstack/react-table@^9.3.1`) so `!nu` (`| to json`) and `.vox` array/object JSON outputs toggle cleanly between raw/pretty JSON and a sortable, filterable Roman Scriptorium Ledger Table.
   - Provide 1-click **"Voxify Subshell"** (`term_session_voxify` / `OSC 633` shell-hook injection) when a session enters a nested shell (`ssh`, `docker exec -it`, `bash`, `zsh`, `fish`, `pwsh`, `nu`) so nested subshells emit structured `OSC 633` command blocks and `OSC 7` `cwd` updates cleanly.

Covers requirements: `TERM-WARP-01`, `TERM-WARP-02`, `TERM-WARP-03`, `TERM-WARP-04`.

</domain>

<decisions>
## Implementation Decisions

### 1. Multi-Line `ScriptoriumPromptEditor`, Syntax Highlighting & Mode Routing (`TERM-WARP-01`)
- **Preserve Existing `InputEditor` Contract & Single `textbox` Landmark:** Create `ScriptoriumPromptEditor.tsx` and keep `InputEditor.tsx` re-exporting/wrapping it cleanly so `Console.tsx` and all `Console.test.tsx` tests (`screen.getByRole('textbox')`, `placeholder=/type a command/i`, `term-shell-mode-select`) continue passing with zero breakage.
- **Multi-Line `<textarea rows={1}>` + Transparent Syntax-Highlighted Mirror:** Use an auto-sizing `<textarea rows={1}>` (up to `max-h-40` with internal scroll) paired with a synchronized `<pre aria-hidden="true">` token-highlighting underlay (`promptHighlighter.ts`: flags `--foo`, strings `'...'`/`"..."`, pipes/operators `|`/`&&`/`||`, env vars `$VAR`, `@term`/`@file` mentions, `/slash` verbs, `?` agent prompts, `.vox` keywords) using Roman Scriptorium semantic tokens (`TERMINAL_MONO_FONT_STACK`, zero `dangerouslySetInnerHTML`, zero banned font literals).
- **Automatic + Lockable 5-Mode Intent Detection (`intentRouter.ts`):** Support both explicit mode locking via `<VoxSelect>` (`auto`, `shell`, `agent`, `nu`, `vox`, `slash` — keeping backward compatibility with `'default' | 'nu' | 'vox'`) and automatic intent detection in `auto`/`default` mode:
  - `/...` -> `slash`
  - `?...` or multi-word natural-language question/sentence not starting with a known `$PATH`/shell/builtin token -> `agent` (prefixed/dispatched cleanly so Phase 445's `?prompt` agent turn handler receives it directly)
  - `!nu ...` or locked `nu` -> `nu`
  - `.vox` expression / locked `vox` -> `vox`
  - `!cmd` or standard shell command -> `shell`
- **`Enter` vs `Shift+Enter`:** `Enter` submits (unless `IntelliSensePopover` or `Ctrl+R` history popover consumes it); `Shift+Enter` inserts a newline (`\n`) at the caret; pasting multi-line scripts keeps newlines intact inside the editor without prematurely executing line-by-line!

### 2. Redacted Persistent Workspace History & `Ctrl+R` Fuzzy Search (`TERM-WARP-02`)
- **3-Stage Scrubbed Backend History Store:** Persist workspace/global terminal history on the Rust backend (`vox-terminal-core::history` + Tauri commands `term_history_record` / `term_history_query`, classified in `remote::gate` and wrapped in `src/transport.ts`), scrubbing every recorded command through `strip_ansi_and_normalize` -> `vox_secrets::scrub_resolved_secrets` -> `vox_redact` BEFORE writing to disk (and skipping commands starting with a leading space `HISTCONTROL=ignorespace` or credential prompts!). Fall back seamlessly to redacted `localStorage` ring (`200` entries) when running in browser/jsdom `no-tauri` preview.
- **`Up`/`Down` Recall + `Ctrl+R` Fuzzy Search Popover (`HistorySearchPopover.tsx`):**
  - When caret is on the first line (for `ArrowUp`) or last line (for `ArrowDown`) and completion popover is closed, `ArrowUp`/`ArrowDown` cycles through prefix-matched history entries while preserving any in-progress draft at index `-1`.
  - Pressing `Ctrl+R` inside `Console` or `ScriptoriumPromptEditor` opens `data-testid="term-history-search"` fuzzy search (subsequence + recency + `cwd` affinity scoring) with keyboard navigation (`ArrowUp`/`ArrowDown`, `Enter` to accept into editor or run, `Tab` to insert into editor without running, `Escape` to close and restore editor focus without leaking extra `textbox` roles when closed!).

### 3. Universal `$PATH`, File, Git & CLI-Flag Completions (`TERM-WARP-03`)
- **Remove `vox`-Only Prefix Gate:** Delete the `trimmed.startsWith('vox')` gate in `InputEditor.tsx:72-80` so typing or pressing `Tab` / `Ctrl+Space` completes *any* command!
- **Low-Latency Backend Completion Provider (`term_complete`):** Add a bounded, non-blocking completion helper (`vox-terminal-core::completions` + `vox-gui` `term_complete(session_id, line, cursor)` IPC, capped at `<= 50` items and path-traversal-safe) returning:
  1. `$PATH` executables (cached with TTL so typing never rescans `$PATH` on every keystroke) + shell builtins (`cd`, `export`, `source`, `alias`, `echo`, `pwd`, etc.) when completing token 0.
  2. `cwd`-relative file & directory paths (using active session's live `OSC 7` `cwd`, appending `/` on directories) on path tokens or after `cd`/`cat`/`ls`/`vim`/`vox run`/`git add`.
  3. Local `git` branches & tags (reading `.git/refs/heads` & `.git/refs/tags` / packed-refs in pure Rust without spawning slow subprocesses!) after `git checkout`/`switch`/`merge`/`rebase`/`log`/`diff`.
  4. Curated + dynamic CLI subcommands & `--flags` for `git`, `cargo`, `pnpm`, `npm`, `docker`, `gh`, `rg`, and `vox` (`discoverySuggest` merged cleanly).
- **Modular Frontend Engine:** Keep `intellisenseEngine.ts` lean (`god_object` safe — extract terminal completion merging into `Console/terminalCompletions.ts` rather than growing `intellisenseEngine.ts`).

### 4. Local `.vox` & Markdown Runbooks, `@tanstack/react-table` Ledger & Subshell "Voxify" (`TERM-WARP-04`)
- **Parameterized Local Runbooks (`RunbookPanel.tsx` + `term_runbooks_list`):**
  - Scan workspace `scripts/*.vox`, `.vox/runbooks/*.{vox,md}`, and runnable `docs/src/how-to/*.md` fenced blocks (read-only bounded scan on backend + built-in starter runbooks fallback in `no-tauri`), extracting title, description, command/script template, and `{{param_name}}` placeholders.
  - Surface Runbooks in both **`DiscoveryRail.tsx`** (a toggleable `Runbooks` tab alongside `Action Help`) and **`CommandPalette`** (`Cmd+P` / global palette action dispatching to `Console` with optional inline parameter inputs before execution!).
- **Sortable/Filterable Roman Scriptorium Ledger Table (`JsonLedgerTable.tsx`):**
  - When a `BlockCard` contains structured JSON output (array of objects or record from `!nu ... | to json`, `.vox`, `jq`, or MCP/CLI `--json`), render a `Table` / `JSON` view toggle (`data-testid="term-ledger-table"`) powered by `@tanstack/react-table` (`getCoreRowModel`, `getSortedRowModel`, `getFilteredRowModel`) with column click sorting, quick filter input (`aria-hidden` or unmounted unless table view is active so initial `Console` `getByRole('textbox')` stays unique!), and Roman Scriptorium tabular styling.
- **1-Click Subshell "Voxify" (`term_session_voxify`):**
  - Expose `term_session_voxify(session_id, shell_hint?)` on `SessionHost` / `TermDispatch` / `vox-gui` / `transport.ts` that injects a compact, base64/printf-safe `OSC 633` + `OSC 7` hook one-liner (`stty -echo` bracketed where supported) into an active nested subshell (`ssh`, `docker exec`, `bash`, `zsh`, `fish`, `pwsh`, `nu`), surfaced as a 1-click **"Voxify Subshell"** button (`data-testid="term-voxify-subshell"`) in `ConsoleSessionBar` / `TerminalSessionPane` whenever a nested shell or raw subshell session is active.

### 5. Non-Negotiable Repo Guardrails
- **Zero new Cargo crates or npm packages** (`Cargo.lock` and `pnpm-lock.yaml` untouched; `@tanstack/react-table@^9.3.1` and `cmdk@^1.1.1` are already installed).
- **Zero new workspace Cargo crate edges** (`./target/debug/vox-arch-check` must exit `0`).
- **Every new/modified file `< 500` non-blank LoC** (split Rust & TS modules cleanly).
- **All UI guards green**: `zeroEmojiAndDingbat.test.ts` (zero emojis/dingbats/geometric shapes/return glyphs), `noRawNativeControlsGuard.test.ts` (`<VoxSelect>` only), `transportIpcGuard.test.ts` (all Tauri IPC in `src/transport.ts`, all new `#[tauri::command]` registered in `main.rs` and classified in `remote::gate`), `toastBodyGuard.test.ts` (`sanitizeErrorForToast`), `themeContract.test.ts`, and unique `screen.getByRole('textbox')` on initial `Console` render!

</decisions>

<canonical_refs>
## Canonical References

- `.planning/workstreams/axis-warp-terminal-v75/ROADMAP.md` — Phase 444 goal & 4 success criteria
- `.planning/workstreams/axis-warp-terminal-v75/REQUIREMENTS.md` — `TERM-WARP-01..04`
- `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` — v7.5 architecture & capability pillars
- `crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.tsx` — Current single-line prompt & `vox` prefix gate
- `crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx` — Right-hand discovery rail
- `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx` — Block output rendering (attach `JsonLedgerTable`)
- `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx` & `TerminalSessionPane.tsx` & `ConsoleSessionBar.tsx` — Session bar, shortcuts, and Voxify hook point
- `crates/vox-gui/ui/src/lib/intellisenseEngine.ts` & `crates/vox-gui/ui/src/components/ui/IntelliSensePopover.tsx` — Completion items & popover
- `crates/vox-terminal-core/src/{input.rs,pty.rs,shell_init.rs,host.rs,corpus/redact.rs}` — Intent classification, 5-shell OSC 633 snippets, and redaction
- `crates/vox-gui/src/{main.rs,commands/pty.rs,commands/discovery.rs,remote/gate.rs}` — Tauri command & remote gate registration

</canonical_refs>

<deferred>
## Deferred Ideas

- Live LLM streaming for `?prompt` / natural-language prose via `vox_actor_runtime::llm`, 1-click *Explain / Fix Failed Block*, Roman wax-seal Co-Pilot Approval Cards, and live `/budget` `/model` `/skills` `/memory` `/context` orchestrator slash mutations -> Phase 445 (`TERM-AGENT-01..04`).
- Global `index.css` `--font-terminal-mono` `:not(.xterm)` CSS isolation and 4-theme Playwright + VLM CV Flywheel convergence -> Phase 446 (`TERM-ROMA-CV-01..04`).

</deferred>
