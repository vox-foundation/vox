# Phase 444: Warp Input Editor, Shell Completions & Executable Runbooks - Pattern Map

**Mapped:** 2026-11-15
**Phase:** `444` (`444-warp-editor-completions-and-runbooks`)
**Requirements:** `TERM-WARP-01`, `TERM-WARP-02`, `TERM-WARP-03`, `TERM-WARP-04`

---

## File Classification

Every new or substantially refactored file MUST stay strictly **`< 500` non-blank LoC** (`vox-code-audit` `god_object` detector). Several adjacent files are already near or above 500 lines (`crates/vox-terminal-core/src/host.rs` = 1,038 lines, `shell_init.rs` = 1,271 lines, `crates/vox-orchestrator-d/src/term_dispatch.rs` = 1,119 lines, `crates/vox-gui/src/commands/pty.rs` = 856 lines, `crates/vox-gui/ui/src/lib/intellisenseEngine.ts` = 650 lines, `Omnibar.tsx` = 798 lines, `App.tsx` = 2,347 lines). **Never dump new Phase 444 implementation logic into those files** — place new Rust and TypeScript logic in dedicated `< 500` LoC modules and connect them via thin 2–15 line seams.

| New / Modified File | Est. LoC | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|---|
| `crates/vox-terminal-core/src/history.rs` *(NEW)* | ~240 | Durable JSONL history store (`HistoryEntry`, `HistoryQuery`, `HistoryStore`), `HISTCONTROL=ignorespace` + credential-prompt filter, 3-stage redaction pipeline hook, prefix/fuzzy/cwd-boosted search | File/Memory JSONL | `crates/vox-terminal-core/src/corpus/writer.rs` & `corpus/redact.rs` | Exact |
| `crates/vox-terminal-core/src/completions.rs` *(NEW)* | ~320 | Contextual shell completions (`CompletionItem`, `CompletionKind`, `CompletionRequest`): `$PATH` executable cache (TTL) + shell builtins, `OSC 7` `cwd`-relative path entries (`/` dir suffix, traversal-safe), pure-Rust `.git/refs/{heads,tags}` + `packed-refs` reader, curated CLI flag/subcommand specs (`git`, `cargo`, `pnpm`, `npm`, `docker`) | Filesystem / Env (`std::fs`, `std::env`) | `crates/vox-gui/src/commands/discovery.rs` | High |
| `crates/vox-terminal-core/src/runbooks.rs` *(NEW)* | ~280 | `.vox` & `.md` executable runbook scanner/parser (`RunbookDoc`, `RunbookStep`, `RunbookParam` for `{{param}}` / `{{param=default}}`) over `runbooks/`, `.vox/runbooks/`, and `docs/` opt-in runbooks | Filesystem Read-Only | `crates/vox-gui/src/commands/action_manifest.rs` | Medium-High |
| `crates/vox-terminal-core/src/voxify.rs` *(NEW)* | ~160 | Nested-shell `OSC 633` + `OSC 7` hook payload builder (`SubshellKind`: `Bash`, `Zsh`, `Fish`, `Pwsh`, `Nu`), wrapping `pty::shell_integration_snippet` + `__vox_emit_cwd` in silent `stty -echo` / base64 one-liners | Pure String / Bytes | `crates/vox-terminal-core/src/pty.rs:33-186` & `shell_init.rs:150-320` | Exact |
| `crates/vox-terminal-core/src/lib.rs` *(MODIFY)* | ~75 | Export `pub mod completions; pub mod history; pub mod runbooks; pub mod voxify;` and public DTOs | Module Re-exports | `crates/vox-terminal-core/src/lib.rs:1-62` | Exact |
| `crates/vox-terminal-core/src/input.rs` *(MODIFY)* | ~130 | Extend `InputIntent` / `classify_Auto` natural-language vs command heuristic helper while preserving backward-compatible `classify(&str) -> Option<InputIntent>` tests (`L56-77`) | Pure Parser | `crates/vox-terminal-core/src/input.rs:1-78` | Exact |
| `crates/vox-gui/src/commands/term_warp.rs` *(NEW)* | ~260 | Tauri IPC commands: `term_history_query`, `term_history_record`, `term_complete`, `term_runbooks_list`, `term_session_voxify`; wires `vox_secrets::scrub_resolved_secrets` into `HistoryStore` | Tauri IPC (`State<Arc<PtyManager>>`) | `crates/vox-gui/src/commands/pty.rs:251-561` & `discovery.rs:414-526` | Exact |
| `crates/vox-gui/src/commands/mod.rs` *(MODIFY)* | ~71 | Add `pub mod term_warp;` | Module Registry | `crates/vox-gui/src/commands/mod.rs:1-70` | Exact |
| `crates/vox-gui/src/commands/pty.rs` *(MODIFY — thin seam)* | ~865 total (~565 prod) | Expose `pub(crate) local_host` / `session_cwd(&str)` helper and `pub(crate) history: Arc<HistoryStore>` on `PtyManager` | State Container | `crates/vox-gui/src/commands/pty.rs:125-210` | Exact |
| `crates/vox-gui/src/main.rs` *(MODIFY — thin seam)* | ~495 | Register `commands::term_warp::{term_history_query, term_history_record, term_complete, term_runbooks_list, term_session_voxify}` inside `axis_invoke_handler!` | Tauri Handler Macro | `crates/vox-gui/src/main.rs:265-285` | Exact |
| `contracts/gui/command-backends.v1.yaml` *(MODIFY — 1 row)* | +1 line | Add `- { module: term_warp, class: host, surfaces: [console], notes: "..." }` in alphabetical order between `stt_config` (`L70`) and `user_config` (`L71`) | Remote Gate Contract | `contracts/gui/command-backends.v1.yaml:65` (`pty`) | Exact |
| `crates/vox-gui/ui/src/transport.ts` *(MODIFY)* | +85 lines | Typed DTOs + `safeInvoke` wrappers: `termHistoryQuery`, `termHistoryRecord`, `termComplete`, `termRunbooksList`, `termSessionVoxify` | IPC Client | `crates/vox-gui/ui/src/transport.ts:1076-1235` | Exact |
| `crates/vox-gui/ui/src/lib/terminalBridge.ts` *(MODIFY)* | +35 lines | Add `vox://open-runbook` custom window event + `requestOpenRunbook(runbookId)` helper alongside `requestTerminalCommand` (`L61-82`) | Window Event Bus | `crates/vox-gui/ui/src/lib/terminalBridge.ts:61-82` | Exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/intentRouter.ts` *(NEW)* | ~140 | 5-mode lockable intent model (`auto`, `shell`, `vox`, `nu`, `agent`), prefix override rules (`!`, `?`, `!nu `, `/`), and deterministic natural-language vs `$PATH`/builtin heuristic | Pure TS Logic | `crates/vox-terminal-core/src/input.rs` | Exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/promptHighlighter.ts` *(NEW)* | ~150 | Tokenizes multi-line buffer into command, subcommand, `--flag`, `'string'`/`"string"`, `| && >` operators, `{{param}}`, and `/slash` or `?` agent tokens for transparent `<textarea>` + `<pre aria-hidden="true">` overlay | Pure TS Tokenizer | `crates/vox-gui/ui/src/components/ui/IntelliSensePopover.tsx` | High |
| `crates/vox-gui/ui/src/components/surfaces/Console/terminalCompletions.ts` *(NEW)* | ~190 | Client-side completion coordinator merging `termComplete(...)` + `discoverySuggest(...)` into `CompletionSuggestion[]` for `IntelliSensePopover` (avoids bloating 650-LoC `intellisenseEngine.ts`) | Async Provider | `crates/vox-gui/ui/src/lib/intellisenseEngine.ts:440-650` | High |
| `crates/vox-gui/ui/src/components/surfaces/Console/historyStore.ts` *(NEW)* | ~140 | Client history adapter calling `termHistoryQuery`/`termHistoryRecord` with `no-tauri` redacted `localStorage` fallback (`HISTCONTROL=ignorespace` + client secret regex guard) | State / Storage | `crates/vox-gui/ui/src/components/surfaces/Console/sessionReducer.ts` | High |
| `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx` *(NEW)* | ~180 | `Ctrl+R` anchored reverse-i-search popover (`returns null` when closed so `getByRole('textbox')` stays unique!) using `<input type="search">` (`role="searchbox"`) + keyboard navigation | Interactive Popover | `crates/vox-gui/ui/src/components/ui/IntelliSensePopover.tsx` | Exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx` *(NEW)* | ~310 | Warp-grade multi-line `<textarea aria-label="console input">` + syntax highlight mirror + `<VoxSelect>` mode selector + `Up`/`Down` caret boundary history + `Ctrl+R` + universal `IntelliSensePopover` | Controlled / Stateful UI | `crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.tsx` | Exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.tsx` *(MODIFY)* | ~55 | Thin backward-compatible wrapper delegating to `ScriptoriumPromptEditor` so all existing imports & `InputEditor.test.tsx` / `Console.test.tsx` pass untouched | Adapter Component | `crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.tsx:1-214` | Exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx` *(NEW)* | ~210 | Interactive `@tanstack/react-table` **v9** (`tableFeatures` + `useTable`) sortable/filterable table for structured JSON blocks with `JSON` / `Table` segmented view toggle + cell copy | Block Sub-Renderer | `crates/vox-gui/ui/src/components/ui/DataTable.tsx:1-190` | Exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx` *(MODIFY)* | ~300 | Mount `<JsonLedgerTable>` toggle on `structured === true` blocks (`L179-183, 211-215`) while keeping default/accessible raw JSON assertion parity with `BlockStreamView.test.tsx` | Block Card UI | `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx:77-230` | Exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx` *(NEW)* | ~260 | Parameterized `.vox` & `.md` runbook browser + `{{param}}` inline input form (`role="searchbox"` / labeled inputs only when runbook active) + `Run Step` / `Stage in Prompt` dispatch | Rail Sub-Panel | `crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx` | High |
| `crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx` *(MODIFY)* | ~220 | Add `Help` \| `Runbooks` tab switcher (defaults to `Help` so all `DiscoveryRail.test.tsx` assertions pass unchanged; switches to `Runbooks` on `vox://open-runbook`) | Context Rail | `crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx:1-173` | Exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx` *(MODIFY)* | ~195 | Add `Voxify Subshell` button (`data-testid="term-voxify-subshell"`) + optional shell flavor picker (`VoxSelect`) dispatching `onVoxifySubshell` | Session Chrome | `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx:1-163` | Exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx` *(MODIFY)* | ~365 | Wire `activeCwd`/`gitBranch` into `InputEditor`, handle `onVoxifySubshell` + `onExecuteRunbookStep` + `vox://open-runbook`, update `Cmd+J` selector (`L143`) from `'input[aria-label="console input"]'` to `'[aria-label="console input"]'` | Surface Orchestrator | `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx:1-330` | Exact |

---

## 1. Rust Backend Architecture & Registration Seams

### 1.1 `axis_invoke_handler!` + `remote::gate` + `command-backends.v1.yaml` Rules

**Critical Macro Constraint (`crates/vox-gui/src/main.rs:9-18`):**
```rust
macro_rules! axis_invoke_handler {
    ($($root:ident :: $module:ident :: $cmd:ident),* $(,)?) => {{
        static TABLE: $crate::remote::gate::CommandTable =
            $crate::remote::gate::CommandTable::new(&[
                $((stringify!($cmd), stringify!($module))),*
            ]);
        (TABLE, tauri::generate_handler![$($root :: $module :: $cmd),*])
    }};
}
```
1. **Strict 3-segment path required:** Every entry MUST have the exact token shape `commands::<module>::<cmd>`. You **cannot** pass 4-segment paths (`commands::pty::warp::cmd`), and `#[tauri::command]` generates `pub(crate) macro __cmd_*` items that do not cleanly glob-re-export across nested submodules without hidden symbol re-exports.
2. **Use top-level `crates/vox-gui/src/commands/term_warp.rs`:**
   - Declare `pub mod term_warp;` in `crates/vox-gui/src/commands/mod.rs` (alphabetical after `pub mod telemetry;` at `L65`).
   - Register in `crates/vox-gui/src/main.rs` right below `commands::pty::term_session_set_control_mode` (`L280`):
     ```rust
     commands::term_warp::term_history_query,
     commands::term_warp::term_history_record,
     commands::term_warp::term_complete,
     commands::term_warp::term_runbooks_list,
     commands::term_warp::term_session_voxify,
     ```
3. **`contracts/gui/command-backends.v1.yaml` Single-Row Registration:**
   - `crates/vox-gui/src/remote/gate.rs:13-36` (`local_only_denial`) looks up `table.module_of(cmd)` in `remote::manifest::module_class(module)` (`crates/vox-gui/src/remote/manifest.rs:120-126`).
   - Add **one** row in alphabetical order between `stt_config` (`L70`) and `user_config` (`L71`) in `contracts/gui/command-backends.v1.yaml`:
     ```yaml
       - { module: term_warp, class: host, surfaces: [console], notes: "Phase 444 Warp terminal history, completions, runbooks, and subshell voxify" }
     ```
   - **Verified Zero Fixture Churn:** `crates/vox-gui/src/remote/manifest.rs:338-389` (`console_stays_needs_local_machine_remotely_after_console_a2a_migration` and `remote_gate_fixture_matches_real_gate`) asserts `host_on_console.contains(&"pty") && !host_on_console.contains(&"terminal_core")` and compares surface-level availability against `ui/e2e/fixtures/remote-gate-mesh-console.json`. Because `console` already resolves to `UnavailableReason::NeedsLocalMachine` via `pty`, adding `term_warp` (`class: host, surfaces: [console]`) preserves the Playwright gate fixture byte-for-byte.

### 1.2 Zero New Workspace Crate Edges (`vox-arch-check`)

Inspecting `crates/vox-terminal-core/Cargo.toml` (`L1-30`) vs `crates/vox-gui/Cargo.toml` (`L1-83`):
- `vox-terminal-core` depends on: `vox-redact`, `vox-journal`, `vox-orchestrator`, `serde`, `serde_json`, `tokio`, `portable-pty`, `tempfile`, `base64`, `alacritty_terminal`, `anyhow`, `tracing`. It does **NOT** depend on `vox-secrets` or `dirs`.
- `vox-gui` and `vox-orchestrator-d` **already** depend on `vox-secrets` (`vox_secrets::scrub_resolved_secrets(&str) -> String`, see `crates/vox-orchestrator-d/src/term_dispatch.rs:315-316`) and `vox-gui` already depends on `dirs = "6"` (`crates/vox-gui/Cargo.toml:55`) and `vox-repository` (`L43`).
- **Rule:** Do NOT add `vox-secrets` or `dirs` to `vox-terminal-core/Cargo.toml`. Instead:
  1. `vox_terminal_core::history::HistoryStore::new(path: PathBuf)` takes an explicit `PathBuf` (resolved in `vox-gui` as `.vox/cache/terminal-history.jsonl` under repo root or `dirs::data_local_dir()/vox/terminal-history.jsonl` — note `.vox/cache/` is already gitignored at `.gitignore:109`).
  2. `HistoryStore::record(raw_cmd, cwd, exit_code, session_id, extra_scrub: Option<&dyn Fn(&str) -> String>)` runs:
     - **Gate 0 (Skip rules):** Empty/whitespace-only OR starts with leading space (`raw_cmd.starts_with(' ')`, `HISTCONTROL=ignorespace`) OR `is_credential_input` -> return `Ok(false)` without persisting.
     - **Stage 1:** `crate::corpus::redact::scrub_ansi_and_redact(raw_cmd)` (`crates/vox-terminal-core/src/corpus/redact.rs:15-21`: strips ANSI escapes/OSC sequences + runs `vox_redact::redact`).
     - **Stage 2:** Apply `extra_scrub` when provided — in `vox-gui/src/commands/term_warp.rs`, pass `&vox_secrets::scrub_resolved_secrets` (exact same two-layer pattern as `crates/vox-orchestrator-d/src/term_dispatch.rs:313-317`).
     - **Stage 3:** Final `vox_redact::redact_owned(&stage2)` pass to guarantee no residual token patterns hit disk.

### 1.3 `PtyManager` & `SessionHost` Seams (`pty.rs`, `host.rs`, `daemon_host.rs`, `term_dispatch.rs`)

- **`crates/vox-gui/src/commands/pty.rs:125-210` (`PtyManager`):**
  - Currently `local_host: Arc<LocalHost>` (`L128`) and `remote_host: tokio::sync::Mutex<Option<Arc<DaemonHost>>>` (`L131`) are private to `pty.rs`, while `pub async fn session_host(&self) -> Arc<dyn SessionHost>` (`L197`) is already `pub`.
  - Expose two tiny `pub(crate)` accessors on `PtyManager` (~12 lines total):
    ```rust
    pub(crate) fn history_store(&self) -> &Arc<vox_terminal_core::history::HistoryStore> {
        &self.history
    }

    pub(crate) async fn active_session_cwd(&self, session_id: Option<&str>) -> Option<PathBuf> {
        if let Some(sid) = session_id {
            if let Ok(Some(cwd)) = self.local_host.session_cwd(sid) {
                return Some(PathBuf::from(cwd));
            }
            if let Ok(list) = self.session_host().await.list().await {
                if let Some(found) = list.into_iter().find(|s| s.id == sid) {
                    if let Some(cwd) = found.cwd {
                        return Some(PathBuf::from(cwd));
                    }
                }
            }
        }
        std::env::current_dir().ok()
    }
    ```
  - Notice `LocalHost::session_cwd(&self, id: &str) -> Result<Option<String>>` (`crates/vox-terminal-core/src/host.rs:724-727`) and `TermSessionInfo.cwd: Option<String>` (returned by both `LocalHost::list` and `DaemonHost::list` via `term.list`) **already exist** from Phase 442/443!
- **Why `term_session_voxify` Needs Zero Daemon Wire Protocol Churn:**
  - Both `LocalHost` (`crates/vox-terminal-core/src/host.rs:672`) and `DaemonHost` (`crates/vox-terminal-core/src/daemon_host.rs:549`, forwarding `term.input` to `crates/vox-orchestrator-d/src/term_dispatch.rs:285-293`) already implement `SessionHost::input(&self, id: &str, data: &[u8]) -> Result<()>`.
  - `term_session_voxify` builds the silent one-liner bytes via `vox_terminal_core::voxify::build_subshell_hook_payload(shell)` and calls `state.session_host().await.input(&session_id, payload.as_bytes()).await`. It works identically on local PTYs, remote `DaemonHost` PTYs, `ssh` sessions, and `docker exec -it` subshells without touching 1,119-line `term_dispatch.rs`.

---

## 2. Frontend Transport & IPC Alignment (`crates/vox-gui/ui/src/transport.ts`)

### 2.1 `transportIpcGuard` Rule (`src/guards/transportIpcGuard.test.ts:13-28`)
- Raw `invoke(` / `listen(` calls are **strictly forbidden** outside `// __VOX_RAW_IPC_BEGIN__` .. `// __VOX_RAW_IPC_END__` (`transport.ts:46-91`).
- Every new wrapper in `transport.ts` MUST call `safeInvoke<T>(cmd, args)` (`transport.ts:62`).
- Tauri v2 automatically maps Rust `snake_case` `#[tauri::command]` parameters to `camelCase` JSON keys on the wire (e.g., `session_id: Option<String>` -> `{ sessionId }`, `exit_code: Option<i32>` -> `{ exitCode }`, `cursor_offset: usize` -> `{ cursorOffset }`).

### 2.2 Exact DTOs & Wrapper Signatures to Add Below `termSessionSetControlMode` (`transport.ts:1235`)

```ts
export interface TermHistoryEntryDto {
  id: number;
  command: string;
  cwd?: string | null;
  exit_code?: number | null;
  timestamp_ms: number;
  session_id?: string | null;
}

export interface TermCompletionItemDto {
  label: string;
  insert_text: string;
  replace_start: number;
  replace_end: number;
  kind: 'command' | 'path' | 'git_branch' | 'flag' | 'subcommand' | 'vox' | 'history';
  detail?: string | null;
  score: number;
}

export interface TermRunbookParamDto {
  name: string;
  default_value?: string | null;
  description?: string | null;
}

export interface TermRunbookStepDto {
  id: string;
  title: string;
  lang: 'sh' | 'bash' | 'zsh' | 'pwsh' | 'nu' | 'vox';
  code: string;
  params: TermRunbookParamDto[];
}

export interface TermRunbookDto {
  id: string;
  title: string;
  path: string;
  kind: 'vox' | 'markdown';
  description?: string | null;
  params: TermRunbookParamDto[];
  steps: TermRunbookStepDto[];
}

export type SubshellShellId = 'bash' | 'zsh' | 'fish' | 'pwsh' | 'nu';

export async function termHistoryQuery(
  query: string,
  cwd?: string | null,
  limit = 50,
): Promise<TermHistoryEntryDto[]> {
  return safeInvoke<TermHistoryEntryDto[]>('term_history_query', {
    query,
    ...(cwd != null ? { cwd } : {}),
    limit,
  });
}

export async function termHistoryRecord(
  command: string,
  cwd?: string | null,
  exitCode?: number | null,
  sessionId?: string | null,
): Promise<boolean> {
  return safeInvoke<boolean>('term_history_record', {
    command,
    ...(cwd != null ? { cwd } : {}),
    ...(exitCode != null ? { exitCode } : {}),
    ...(sessionId != null ? { sessionId } : {}),
  });
}

export async function termComplete(
  buffer: string,
  cursorOffset: number,
  sessionId?: string | null,
  limit = 25,
): Promise<TermCompletionItemDto[]> {
  return safeInvoke<TermCompletionItemDto[]>('term_complete', {
    buffer,
    cursorOffset,
    ...(sessionId != null ? { sessionId } : {}),
    limit,
  });
}

export async function termRunbooksList(): Promise<TermRunbookDto[]> {
  return safeInvoke<TermRunbookDto[]>('term_runbooks_list');
}

export async function termSessionVoxify(
  sessionId: string,
  shell?: SubshellShellId | null,
): Promise<void> {
  return safeInvoke<void>('term_session_voxify', {
    sessionId,
    ...(shell != null ? { shell } : {}),
  });
}
```

### 2.3 `no-tauri` Preview Graceful Degradation
When running in browser preview without Tauri (`isBackendUnavailable(err)` from `src/lib/backendGuard.ts`):
- `termHistoryQuery` / `termHistoryRecord` fall back cleanly in `Console/historyStore.ts` to a redacted `localStorage` ring buffer (`gui.console.history.v1`, max 200 entries, leading-space skip + secret pattern scrub).
- `termComplete` falls back cleanly in `Console/terminalCompletions.ts` to curated client command/flag tables + `discoverySuggest` fallback so `Tab` / `ArrowDown` still works offline.
- `termRunbooksList` resolves to `[]` with an informative empty state in `RunbookPanel.tsx` (`No runbooks found in runbooks/ or .vox/runbooks/`).

---

## 3. Pattern Assignments per Requirement (`TERM-WARP-01..04`)

### 3.1 `TERM-WARP-01`: Multi-Line Scriptorium Prompt Editor & Lockable Intent Routing

**Existing Analogs:**
- `crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.tsx` (`L1-214`): single-line `<input aria-label="console input">`, `classifyPreview(value)` (`L27-35`), `discoverySuggest` gated on `trimmed.startsWith('vox')` (`L72-80`).
- `crates/vox-terminal-core/src/input.rs` (`L10-54`): `InputIntent::{Vox, Shell, StructuredShell, Agent}` prefix classifier (`!nu `, `!`, `?`, bare `vox ...`).
- `crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.test.tsx` (`L1-106`) & `Console.test.tsx` (`L88-235`): **Existing regression contracts you MUST preserve:**
  1. `screen.getByRole('textbox')` MUST resolve to the single primary prompt editor (`<textarea aria-label="console input">`) on initial mount (`InputEditor.test.tsx:18,27,38,57,79`, `Console.test.tsx:91,99,153,165,226`).
  2. Typing `!ls -la` renders intent badge text `shell` (`InputEditor.test.tsx:31`).
  3. Typing `?why did build fail` renders intent badge text `agent` (`InputEditor.test.tsx:36`).
  4. Typing `!nu ls | where size > 1kb` renders intent badge text `nu` (`InputEditor.test.tsx:41`).
  5. Typing `vox check` renders intent badge text `vox` (`InputEditor.test.tsx:22`).
  6. `Enter` (without `Shift`) calls `onSubmit(submittedWireText)` and clears the buffer (`InputEditor.test.tsx:48-51`).
  7. In `Console.tsx:143`, update `document.querySelector<HTMLInputElement>('input[aria-label="console input"]')` to `document.querySelector<HTMLElement>('[aria-label="console input"]')` so `Cmd+J` focuses `<textarea aria-label="console input">`.

**5-Mode Lockable Intent Router (`Console/intentRouter.ts`):**
- Modes: `'auto' | 'shell' | 'vox' | 'nu' | 'agent'` (persisted in `localStorage` under `gui.console.intent_mode.v1`, defaulting to `'auto'`).
- Selector UI: `<VoxSelect>` (`src/components/ui/Select.tsx`) — **NEVER** raw `<select>` (`noRawNativeControlsGuard.test.ts` bans `<select>`!). Note: Radix `<SelectTrigger>` has `role="combobox"`, NOT `role="textbox"`, so `screen.getByRole('textbox')` remains uniquely the `<textarea>`!
- **Routing Resolution Rules (`resolvePromptSubmission(raw: string, lockedMode: PromptIntentMode)`):**
  1. **Explicit Prefix Wins in `auto` Mode:**
     - `trimmed.startsWith('!nu ')` -> `{ badge: 'nu', wireLine: trimmed }`
     - `trimmed.startsWith('!')` -> `{ badge: 'shell', wireLine: trimmed }`
     - `trimmed.startsWith('?')` -> `{ badge: 'agent', wireLine: trimmed }`
     - `trimmed.startsWith('/')` -> `{ badge: 'agent', wireLine: '?' + trimmed }` (or routes slash-agent query cleanly to `InputIntent::Agent`)
     - `trimmed === 'vox' || trimmed.startsWith('vox ')` -> `{ badge: 'vox', wireLine: trimmed }`
  2. **Natural-Language vs Shell Heuristic in `auto` Mode (when no explicit prefix):**
     - Classify as `'shell'` if the first token is a known shell builtin/keyword (`cd`, `ls`, `pwd`, `echo`, `cat`, `grep`, `rg`, `git`, `cargo`, `pnpm`, `npm`, `npx`, `node`, `rustfmt`, `docker`, `kubectl`, `make`, `env`, `export`, `source`, `which`, `mkdir`, `rm`, `cp`, `mv`, `chmod`, `curl`, `ssh`, `tar`, `find`, `sed`, `awk`, `ps`, `kill`, `clear`, `exit`, `true`, `false`), starts with `./`, `../`, `/`, `~/`, contains shell syntax (`|`, `&&`, `||`, `;`, `>`, `<`, `$(`, `=`), or matches the known `$PATH` command set hydrated from `termComplete`.
     - Classify as `'agent'` only when the line is multi-word prose (≥ 4 words, starts with interrogative/conversational lead like `why `, `how `, `what `, `explain `, `help me `, `can you `, `fix the `, `summarize `, or ends with `?`, and contains zero shell operators/paths) -> rewrite wire submission to `?${trimmed}` so `LocalHost::submit` routes to `InputIntent::Agent`.
     - Otherwise default bare single/command-shaped lines without prefix to `{ badge: 'shell', wireLine: '!' + trimmed }` when user locked `'shell'`, OR preserve `{ badge: 'vox', wireLine: trimmed }` when `vox`-shaped!
     - **CRITICAL BACKWARD-COMPATIBILITY CHECK (`Console.test.tsx` & `InputEditor.test.tsx`):** Check what `InputEditor.test.tsx:16-23` and `Console.test.tsx:99-105` assert!
       - `InputEditor.test.tsx:21`: `fireEvent.change(box, { target: { value: 'vox check' } }); expect(screen.getByText('vox')).toBeTruthy();`
       - `Console.test.tsx:99-103`: `fireEvent.change(input, { target: { value: 'vox doctor' } }); fireEvent.submit(input.closest('form')!); expect(termSessionSubmitMock).toHaveBeenCalledWith('s-1', 'vox doctor');`
       - `Console.test.tsx:165-166`: `fireEvent.change(input, { target: { value: '!ls' } }); fireEvent.submit(input.closest('form')!); expect(termSessionSubmitMock).toHaveBeenCalledWith('s-1', '!ls');`
       - When `lockedMode !== 'auto'`:
         - `'shell'`: if not already prefixed with `!`, prepend `!` on submit (`wireLine = trimmed.startsWith('!') ? trimmed : '!' + trimmed`), badge = `'shell'`.
         - `'nu'`: prepend `!nu ` if not prefixed, badge = `'nu'`.
         - `'vox'`: submit bare `vox ...` (prepend `vox ` if not starting with `vox`), badge = `'vox'`.
         - `'agent'`: prepend `?` if not starting with `?`, badge = `'agent'`.
  3. **Multi-Line Editing Mechanics (`ScriptoriumPromptEditor.tsx`):**
     - `<textarea aria-label="console input" rows={1}>` auto-grows up to `max-h-44` (`176px`) based on line count / `scrollHeight`.
     - `Enter` (no modifiers, when completion popover is not consuming `Enter`) calls `e.preventDefault()` and submits the full multi-line buffer.
     - `Shift+Enter` inserts `\n` naturally without submitting.
     - Pasting multi-line text (`onPaste`) keeps all newlines in the `<textarea>` state **without** auto-executing (preventing accidental paste-execution exploits!).
     - Syntax highlight mirror: stacked `<pre aria-hidden="true" data-testid="prompt-syntax-layer">` behind `<textarea className="bg-transparent text-transparent caret-[var(--color-text-primary)] ...">` with identical padding, `font-mono`, `text-xs`, `leading-5`, `whitespace-pre-wrap`, `break-words`, and scroll sync (`onScroll`).

---

### 3.2 `TERM-WARP-02`: Durable Redacted History & Fuzzy Search (`Ctrl+R` / `Up` / `Down`)

**Existing Redaction & JSONL Analogs:**
- `crates/vox-terminal-core/src/corpus/redact.rs:15-21`:
  ```rust
  pub fn scrub_ansi_and_redact(raw: &str) -> String {
      let plain = strip_ansi_and_normalize(raw);
      vox_redact::redact(&plain).into_owned()
  }
  ```
- `crates/vox-terminal-core/src/corpus/writer.rs:13-41` (`CorpusWriter`): `OpenOptions::new().create(true).append(true)` + `serde_json::to_writer` + `\n` + `flush()`.
- `crates/vox-orchestrator-d/src/term_dispatch.rs:315-316`: `vox_secrets::scrub_resolved_secrets(&str) -> String`.

**Rust `vox-terminal-core/src/history.rs` Design:**
1. **Skip Rules (Never Recorded):**
   - `raw.trim().is_empty()` -> skip.
   - `raw.starts_with(' ')` (`HISTCONTROL=ignorespace` parity) -> skip **before** trimming!
   - Credential-input heuristic (`is_credential_line(raw)`): skip lines matching standalone secret assignments or interactive password prompts (e.g. `(?i)^\s*(export\s+)?\w*(password|passwd|secret|api_key|apikey|access_token|auth_token|private_key)\w*\s*=`). Wait: if a command has inline `--token sk-...` or `OPENAI_API_KEY=sk-... cmd`, Stage 1–3 redaction scrubs the value to `[REDACTED]` (or skip vs scrub: scrub via `vox_secrets::scrub_resolved_secrets` + `vox_redact::redact_owned` ensures no secret value ever lands on disk, AND add a dedicated unit test asserting raw secret strings never appear in the JSONL file!). Also provide `record_with_prompt_context(..., awaiting_secret_prompt: bool)` so password/sudo prompts (`Password:`, `passphrase`, `[sudo] password for`) are skipped completely.
2. **Deduplication & Compaction:**
   - Consecutive identical commands update `timestamp_ms` / `cwd` / `exit_code` in memory and append at most once, capping retained file entries (e.g. `MAX_HISTORY_ENTRIES = 5_000`).
3. **Query & Ranking (`HistoryStore::query(&self, q: &str, current_cwd: Option<&str>, limit: usize)`):**
   - Empty `q`: most-recent-first (`timestamp_ms` desc), deduplicated by `command`.
   - Non-empty `q`: case-insensitive prefix match (+100) > substring match (+60) > subsequence fuzzy match (+30), plus `+25` boost when `entry.cwd.as_deref() == current_cwd`, tie-broken by `timestamp_ms` desc.

**Frontend `Up`/`Down` & `Ctrl+R` Interaction (`ScriptoriumPromptEditor.tsx` + `HistorySearchPopover.tsx`):**
- **Multi-Line Caret Boundary Rule for `ArrowUp` / `ArrowDown`:**
  - Compute whether the caret (`textarea.selectionStart === textarea.selectionEnd`) is on the **first line** (`!value.slice(0, caret).includes('\n')`) for `ArrowUp`, or on the **last line** (`!value.slice(caret).includes('\n')`) for `ArrowDown`.
  - When on line 1 and `ArrowUp` is pressed: save current draft (`draftBeforeHistoryRef`), step backward through prefix-filtered history (`historyEntries` matching draft prefix), and place caret at end (or start) of recalled entry.
  - When on an interior line of a multi-line buffer: let browser move the caret vertically within the `<textarea>`!
- **`Ctrl+R` (`HistorySearchPopover.tsx`):**
  - Pressing `Ctrl+R` (or clicking the History clock icon button `data-testid="term-history-search-trigger"`) opens `HistorySearchPopover` anchored above the prompt editor (`bottom-full mb-1.5`).
  - **Landmark Safety:** When `open === false`, `HistorySearchPopover` MUST `return null`. When `open === true`, its filter input MUST use `<input type="search" aria-label="Search command history" data-testid="term-history-search-input" />` (`type="search"` has ARIA role `searchbox`, NOT `textbox`, so `screen.getByRole('textbox')` STILL uniquely resolves to the prompt editor!).
  - `ArrowUp` / `ArrowDown` cycles results; `Enter` loads the selected command into the prompt editor (without auto-executing so the operator can inspect/edit, or `Mod+Enter` executes immediately); `Escape` closes and restores focus to `[aria-label="console input"]`.

---

### 3.3 `TERM-WARP-03`: Universal Contextual Shell & Vox Completions

**Root Cause of Current Limitation (`InputEditor.tsx:72-80`):**
```tsx
// EXISTING CODE IN InputEditor.tsx:72-80:
const trimmed = value.trim();
if (!trimmed || !trimmed.startsWith('vox')) {
  setSuggestions([]);
  return;
}
```
Everything not starting with `'vox'` is currently dropped on the floor!

**Rust `vox-terminal-core/src/completions.rs` Providers:**
Given `(buffer: &str, cursor_offset: usize, cwd: &Path, limit: usize) -> Vec<CompletionItem>`:
1. **Token Extraction at Cursor:**
   - Slice `before_cursor = &buffer[..cursor_offset.min(buffer.len())]` (snap to UTF-8 char boundary via `floor_char_boundary` or char indices!).
   - Strip leading intent prefix (`!nu `, `!`, `?`) when computing command position, while keeping exact byte/UTF-16 `replace_start`..`replace_end` offsets relative to `buffer`.
   - Identify whether cursor is in **command position** (first token of buffer or first token after `|`, `&&`, `||`, `;`) or **argument position** (subsequent token for `head_cmd`).
2. **Provider 1 — `$PATH` Executables & Shell Builtins (Command Position):**
   - Cache executable names from `std::env::split_paths(&std::env::var_os("PATH"))` in a `Mutex<Option<(Instant, Vec<String>)>>` with a 30-second TTL (non-blocking, bounded scan; on Unix check executable mode bit `mode & 0o111 != 0`, on Windows check `PATHEXT` `.exe`/`.cmd`/`.bat`/`.ps1`), merged with shell builtins (`cd`, `pwd`, `echo`, `export`, `source`, `history`, `clear`, `vox`).
3. **Provider 2 — `OSC 7` `cwd`-Relative File & Directory Paths:**
   - Triggered whenever the active token starts with `./`, `../`, `~/`, `/`, contains `/`, or follows path-taking commands (`cd`, `ls`, `cat`, `rm`, `cp`, `mv`, `mkdir`, `code`, `vim`, `vox run`, `vox check`).
   - Resolve against `cwd` (`cd` filters to directories only); append trailing `/` on directories (`insert_text: "src/"`, `kind: CompletionKind::Path`); enforce canonical/component guard (`max_entries = 100`, never follow symlink loops, skip `.` and `..` pseudo-entries).
4. **Provider 3 — Pure-Rust Git Branch & Tag Completions (Zero Subprocess):**
   - Triggered when `head_cmd == "git"` and subcommand is `checkout`, `switch`, `merge`, `rebase`, `branch`, `log`, `diff`, `push`, `pull`, `reset`.
   - Walk upward from `cwd` (max 16 ancestors) to locate `.git` directory (or `.git` gitdir file in worktrees!).
   - Read `.git/refs/heads/**`, `.git/refs/tags/**`, and parse `.git/packed-refs` lines (`<sha> refs/heads/<name>`) in pure Rust (`std::fs`) — **zero `git` subprocess spawns**, `< 5ms` latency!
5. **Provider 4 — Curated Subcommands & Flags:**
   - Static fast lookup tables for `git`, `cargo` (`build`, `check`, `test`, `run`, `clippy`, `-p`, `--features`, `--release`), `pnpm` (`install`, `run`, `test`, `exec`, `--dir`, `--filter`), `npm`, `docker`, plus `vox` commands (augmented in `terminalCompletions.ts` by `discoverySuggest` from `crates/vox-gui/src/commands/discovery.rs:414`).

**Frontend Popover Integration (`Console/terminalCompletions.ts` + `IntelliSensePopover.tsx`):**
- Reuse `IntelliSensePopover` (`crates/vox-gui/ui/src/components/ui/IntelliSensePopover.tsx:1-92`)!
- Look at `IntelliSensePopoverProps` (`L10-20`):
  ```ts
  export interface IntelliSensePopoverProps {
    suggestions: CompletionSuggestion[]; // { label, insertText, kind, detail?, documentation? }
    selectedIndex: number;
    onSelect: (suggestion: CompletionSuggestion) => void;
    onHover: (index: number) => void;
    visible: boolean;
    anchorStyle?: React.CSSProperties;
  }
  ```
- Map `TermCompletionItemDto.kind` onto `CompletionSuggestion['kind']` (`'command' | 'flag' | 'path' | 'history' | 'skill' | 'agent' | 'file' | 'url'`):
  - `'path'` -> `'path'` (`Folder` icon in `IntelliSensePopover.tsx:31`)
  - `'flag'` -> `'flag'` (`Terminal` icon in `L30`)
  - `'git_branch'` -> `'command'` with `detail: 'git branch'`
  - `'command' | 'subcommand' | 'vox'` -> `'command'`
  - `'history'` -> `'history'` (`Clock` icon in `L32`)
- **Debounce & Cancellation:** 40ms debounce + monotonic request sequence counter (`reqSeqRef.current++`) so stale async responses never overwrite newer keystrokes.
- **Keybindings when `IntelliSensePopover` is visible:**
  - `Tab`: accept `suggestions[selectedIndex]`, splice `insert_text` over `[replace_start, replace_end]` in the buffer, keep focus and advance caret to `replace_start + insert_text.length` (do NOT submit!).
  - `ArrowDown` / `ArrowUp`: cycle `selectedIndex` modulo `suggestions.length`.
  - `Enter`: if user explicitly navigated with `ArrowDown`/`ArrowUp` (or `Tab`), accept completion; if user has NOT navigated (`completionTouched === false`), `Enter` submits the command immediately so fast typists are never hijacked by an unsolicited popup!
  - `Escape`: dismiss popover (`setVisible(false)`).

---

### 3.4 `TERM-WARP-04`: Executable Runbooks, Structured JSON Table Blocks & 1-Click Subshell Voxify

#### Part A: Parameterized `.vox` & Markdown Runbooks + Command Palette (`Omnibar`) Integration

1. **Rust Scanner (`crates/vox-terminal-core/src/runbooks.rs`):**
   - Scan workspace directories (if present): `runbooks/`, `.vox/runbooks/`, and Markdown files under `docs/runbooks/` (or Markdown files with frontmatter `runbook: true` / `category: "Runbook"` so scanning stays fast `< 25ms` and doesn't parse 500 unrelated architecture docs).
   - **Markdown Runbook Parser:** Extract `# Title` / YAML frontmatter `title`/`description`, and every fenced code block tagged ```` ```sh ````, ```` ```bash ````, ```` ```zsh ````, ```` ```pwsh ````, ```` ```nu ````, or ```` ```vox ```` (plus optional `title="..."` fence attribute or preceding `##`/`###` heading as step title).
   - **`.vox` Runbook Parser:** Treat `.vox` files in `runbooks/` or `.vox/runbooks/` as executable `vox run <rel_path>` steps, extracting leading `//!` doc comments and `@param name = "default"` annotations or `{{param}}` placeholders.
   - **Parameter Extraction (`{{param}}` / `{{param=default}}`):** Regex-free simple scanner over `{{...}}` tokens deduplicating parameter names across steps and surfacing `TermRunbookParamDto { name, default_value, description }`.
2. **Console Rail UI (`RunbookPanel.tsx` + `DiscoveryRail.tsx`):**
   - `DiscoveryRail.tsx` (`crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx:1-173`) gets a compact header tab toggle: `Help` (default) | `Runbooks` (`data-testid="discovery-rail-tab-runbooks"`).
   - **CRITICAL TEST PARITY (`DiscoveryRail.test.tsx`):** Because `Help` remains the default active tab, every existing test in `DiscoveryRail.test.tsx` (`actionId` fetch, `2000ms` dwell `discoveryRecord`, `Use suggested action`, collapse toggle `»`/`«`, `aria-live="polite"`) passes with zero changes!
   - In `RunbookPanel.tsx`:
     - Selecting a runbook displays its description, parameter inputs (rendered only when a parameterized runbook is expanded; use `<input type="search">` for the runbook filter box so `role="searchbox"` does not collide with `getByRole('textbox')`, and only render parameter `<input>` fields when the user opens the `Runbooks` tab!), and ordered executable steps.
     - Each step has two actions:
       1. **`Stage`** (`onPickCommand(interpolatedLine)`): loads the interpolated command into `ScriptoriumPromptEditor` for inspection before pressing `Enter`.
       2. **`Run`** (`onExecuteStep(interpolatedLine)`): dispatches directly into the focused PTY pane via `handleSubmit` in `Console.tsx` (creating a real `OSC 633` block card!).
3. **Command Palette (`Omnibar.tsx` + `terminalBridge.ts`) Integration:**
   - In this codebase, the global `cmdk` Command Palette is `crates/vox-gui/ui/src/components/layout/Omnibar.tsx` (`cmdk` `CommandDialog` at `L1-26`, mounted in `crates/vox-gui/ui/src/App.tsx:2218-2269`).
   - Look at how `App.tsx:2242-2247` handles `onRunCommand(command)` from `Omnibar`:
     - Keep `Omnibar.tsx` (`798` lines) and `App.tsx` (`2,347` lines) clean by adding a tiny `< 35` line helper hook/module `src/lib/omnibarRunbooks.ts` (consumed in `Omnibar.tsx` in ≤ 12 lines):
       - Lazy-fetch `termRunbooksList()` when `Omnibar` opens (`open === true`).
       - Render matching runbooks under a `Runbooks` `<CommandGroup heading="Runbooks">` when the query matches `runbook.title`, `runbook.id`, or starts with `runbook`.
       - On select: call `onNavigate('console')` + `requestOpenRunbook(runbook.id)` (via `terminalBridge.ts` custom event `'vox://open-runbook'`, exact analog of `'vox://terminal-command'` at `terminalBridge.ts:61-82`!).
       - `Console.tsx` listens for `'vox://open-runbook'`, auto-expands `DiscoveryRail` if collapsed, switches to the `Runbooks` tab, and selects `runbookId` (focusing its first `{{param}}` field or staging its single step!).

#### Part B: Interactive Structured JSON Table Blocks (`JsonLedgerTable.tsx` in `BlockCard.tsx`)

**CRITICAL ANTI-HALLUCINATION — `@tanstack/react-table` is v9.3.1, NOT v8!**
Inspecting `crates/vox-gui/ui/node_modules/@tanstack/react-table/package.json` (`"version": "9.3.1"`) and `crates/vox-gui/ui/src/components/ui/DataTable.tsx:1-85`:
- `@tanstack/react-table@9.3.1` **does NOT export `useReactTable` or `getCoreRowModel` or `getSortedRowModel`**!
- It uses the v9 modular feature API (`tableFeatures` + `useTable`):
  ```ts
  import {
    columnFilteringFeature,
    createFilteredRowModel,
    createSortedRowModel,
    filterFn_includesString,
    globalFilteringFeature,
    rowSortingFeature,
    sortFn_alphanumeric,
    tableFeatures,
    useTable,
    type ColumnDef as TanStackColumnDef,
    type SortingState,
  } from '@tanstack/react-table';

  const LEDGER_TABLE_FEATURES = tableFeatures({
    columnFilteringFeature,
    globalFilteringFeature,
    rowSortingFeature,
    filteredRowModel: createFilteredRowModel(),
    sortedRowModel: createSortedRowModel(),
    filterFns: { includesString: filterFn_includesString },
    sortFns: { alphanumeric: sortFn_alphanumeric },
  });
  ```
  And inside `JsonLedgerTable`:
  ```ts
  const [sorting, setSorting] = useState<SortingState>([]);
  const [globalFilter, setGlobalFilter] = useState('');

  const table = useTable({
    features: LEDGER_TABLE_FEATURES,
    data: normalizedRows,
    columns: tanstackColumns,
    getRowId: (_row, idx) => String(idx),
    enableGlobalFilter: true,
    globalFilterFn: 'includesString',
    state: { sorting, globalFilter },
    onSortingChange: setSorting,
    onGlobalFilterChange: setGlobalFilter,
  });
  ```
- **JSON Normalization Shapes (`parseLedgerData(outputText: string)`):**
  1. Array of plain objects (`[{"name":"src","size":4096}, ...]`) -> columns = union of keys in first-seen order; rows = elements.
  2. Single plain object (`{"status":"ok","count":3}`) -> 2 columns (`Field`, `Value`), 1 row per key/value pair.
  3. Array of primitives (`["a","b"]`) -> 2 columns (`Index`, `Value`).
  4. Nested objects/arrays in cell values -> `JSON.stringify(val)` compact string for sorting/filtering + click-to-copy cell button (`navigator.clipboard.writeText`).
- **CRITICAL EXISTING TEST PARITY (`BlockStreamView.test.tsx:92-136`):**
  - Look at `BlockStreamView.test.tsx:114-136`:
    - Renders `{ id: 1, input: '!nu ls | to json', output: '[{"name":"src","type":"dir"}]', exit_code: 0 }` and `{ id: 2, input: '!echo \'{"status":"ok"}\'', output: '{"status":"ok"}', exit_code: 0 }`.
    - Asserts `expect(screen.getAllByTestId('term-block-structured-badge')).toHaveLength(2)`, `expect(screen.getByText(/"name": "src"/)).toBeTruthy()`, and `expect(screen.getByText(/"status": "ok"/)).toBeTruthy()`!
  - **Rule:** In `BlockCard.tsx`, keep `viewMode` defaulting to `'json'` (pretty-printed `<pre>`) AND render a segmented toggle button pair (`data-testid="term-block-view-json"` / `data-testid="term-block-view-table"`) right next to `data-testid="term-block-structured-badge"` when `structured && canRenderLedgerTable(outputText)`. Clicking `Table` mounts `<JsonLedgerTable>` (`data-testid="term-block-ledger-table"`), where `<input type="search" aria-label="Filter table rows" data-testid="term-ledger-filter" />` filters rows and clicking any column `<button>` header toggles ascending/descending sort (`aria-sort`). That keeps 100% of `BlockStreamView.test.tsx` green AND makes interactive table sorting/filtering 1-click accessible!

#### Part C: 1-Click "Voxify Subshell" (`OSC 633` + `OSC 7` Injection)

**Existing Shell Hook Reference (`crates/vox-terminal-core/src/pty.rs:33-186` & `shell_init.rs:150-320`):**
- `vox-terminal-core` already defines `OSC 633` prompt/preexec/precmd hooks (`A`, `B`, `C`, `D;<exit>`, `E;<cmd>`) in `pty.rs:65-186` (`ZSH_OSC633`, `FISH_OSC633`, `NU_OSC633`, `PWSH_OSC633`, `BASH_OSC633`) and `OSC 7` working-directory + git-branch reporting (`\x1b]7;file://localhost<cwd>\x07` + `OSC 633;P;Cwd=...`) in `shell_init.rs`.
- When an operator enters a nested subshell inside an existing PTY (`ssh prod-box`, `docker exec -it container bash`, `sudo -i`, `zsh`), the outer shim's hooks are no longer active in the child shell.
- **Rust `vox-terminal-core/src/voxify.rs` Payload Builder:**
  - `pub enum SubshellKind { Bash, Zsh, Fish, Pwsh, Nu }` (default `Bash` / POSIX-compatible auto-detect snippet on `bash`/`zsh` when `shell == None`! Think how clean POSIX auto-detect is: a tiny `if [ -n "${ZSH_VERSION:-}" ]; then ... elif [ -n "${BASH_VERSION:-}" ]; then ... fi` base64 payload works automatically in BOTH `bash` AND `zsh` SSH/Docker containers without forcing the user to guess!).
  - Base64-encode the combined `OSC 633` + `OSC 7` hook script using `base64::engine::general_purpose::STANDARD` (already in `vox-terminal-core/Cargo.toml:22`!).
  - Emit a silent, history-clean (`leading space` so remote `HISTCONTROL=ignorespace` ignores it!) injection line ending in `\n`:
    - **POSIX (`bash` / `zsh` auto-detect):**
      ```sh
       stty -echo 2>/dev/null; eval "$(printf '%s' '<BASE64>' | base64 -d 2>/dev/null || printf '%s' '<BASE64>' | base64 -D 2>/dev/null)"; stty echo 2>/dev/null; printf '\033[2K\r'
      ```
      *(Note the leading `' '` space for `ignorespace`, dual `base64 -d` GNU/Linux + `base64 -D` macOS BSD fallback, `stty echo` restore, and `\033[2K\r` line clear!)*
    - **`fish`:** leading space + `printf '%s' '<BASE64>' | base64 -d | source; commandline -f repaint`
    - **`pwsh`:** `[System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('<BASE64>')) | Invoke-Expression`
    - **`nu`:** Nu hook config assignment snippet.
- **Frontend Trigger (`ConsoleSessionBar.tsx` + `Console.tsx`):**
  - Add a compact button in `ConsoleSessionBar.tsx` right next to the Split controls:
    - `data-testid="term-voxify-subshell"`
    - `aria-label="Voxify subshell (inject OSC 633 and OSC 7 hooks)"`
    - Calls `onVoxifySubshell(activeSessionId)` -> `termSessionVoxify(sessionId)` -> pushes a subtle info toast (`title: 'Subshell hooks injected'`) or catches errors with `sanitizeErrorForToast(err)`.

---

## 4. UI Primitives, Icon Anti-Hallucination SSOT & Static Guardrails

### 4.1 Verified Icon Exports (NEVER Invent Unverified Icon Names)
- **`src/components/ui/Icons.tsx` (`Icon.*`):** `dashboard`, `edit`, `trash`, `list`, `flow`, `catalog`, `matrix`, `memory`, `settings`, `search`, `send`, `bolt`, `pause`, `play`, `doubt`, `gavel`, `alert`, `bell`, `check`, `spark`, `trophy`, `x`, `close`, `command`, `file`, `message`, `book`, `folder`, `terminal`, `cpu`, `scale`, `chevR`, `chevL`, `eye`, `mic`, `pin`, `refresh`, `clock`, `shield`, `plus`, `globe`, `link`, `chevronDown`, `chevronUp`, `branch`, `puzzle`, `beaker`, `brain`, `users`, `stop`, `trendUp`, `trendDown`, `minus`, `externalLink`, `rotateCcw`, `arrowUp`, `arrowDown`, `arrowLeft`, `arrowRight`, `arrowLeftRight`, `cornerDownLeft`, `gripVertical`, `lockKeyhole`, `scissors`, `star`, `power`, `ban`, `target`, `circleDot`, `circle`, `square`.
- **`lucide-react` (used in `DataTable.tsx`, `IntelliSensePopover.tsx`, `ConsoleSessionBar.tsx`):** `ArrowUp`, `ArrowDown`, `ArrowUpDown`, `ChevronDown`, `ChevronRight`, `Terminal`, `Folder`, `Clock`, `BookOpen`, `Play`, `Copy`, `Check`, `Search`, `Sparkles`, `Table2`, `Code2`, `Wand2`, `Columns2`, `Rows2`.

### 4.2 Mandatory Static Guardrails (Enforced by `src/guards/*.test.ts` on Every Vitest Run)

1. **`zeroEmojiAndDingbat.test.ts` (`src/guards/zeroEmojiAndDingbat.test.ts:5-7`):**
   - Scans **every** non-test `.ts`/`.tsx` file under `src/` (comments stripped, string literals INCLUDED!).
   - Banned codepoints: `/[\p{Extended_Pictographic}\u2600-\u27BF\u23F0-\u23FE\u25A0-\u25FF\u2197\u21BA-\u21C4\u21E2\u2B06\u2302\u2442\u2295\u21B5\u23CE\u21E5]/gu`.
   - **Watch out for keycap glyphs in prompt hints!** `\u21B5` (`↵`), `\u23CE` (`⏎`), `\u21E5` (`⇥`), `\u2B06` (`⬆`), `\u25B8` (`▸`), `\u25CF` (`●`), and all emojis are **BANNED** in `.ts`/`.tsx` string literals and JSX! Write plain ASCII key labels (`Enter`, `Shift+Enter`, `Tab`, `Ctrl+R`, `Esc`) or `<KeycapIcon>` (`src/components/ui/Icons.tsx:427`) + SVG icons (`Icon.*` / `lucide-react`).
2. **`noRawNativeControlsGuard.test.ts` (`src/guards/noRawNativeControlsGuard.test.ts:13-15`):**
   - Zero `<select` JSX tags anywhere under `src/` — ALWAYS use `<VoxSelect>` from `src/components/ui/Select.tsx`.
   - Zero `<input type="range">` tags anywhere under `src/`.
3. **`transportIpcGuard.test.ts` (`src/guards/transportIpcGuard.test.ts:23-27`):**
   - Zero `invoke(` or `listen(` calls outside the marked raw-IPC block in `src/transport.ts`. Import typed functions from `transport.ts`.
4. **`toastBodyGuard.test.ts` (`src/guards/toastBodyGuard.test.ts:17`):**
   - Never write `body: String(err)`, `setFoo(String(err))`, or bare `String(e)` / `String(err)` / `String(error)`. Always route caught errors through `sanitizeErrorForToast(err)` from `src/lib/backendGuard.ts`.
5. **`themeContract.ts` (`src/lib/themeContract.ts:61-114`):**
   - Zero hardcoded dark-brown hexes (`#0a0805`..`#2a2011`) and zero banned font-family literals (`Courier New`, `Courier`, `Arial`, `Consolas`, `Menlo`, `Monaco`) in UI components. Use Tailwind token classes (`bg-bg-base`, `bg-bg-surface`, `bg-bg-elevated`, `border-border-subtle`, `border-border-strong`, `text-text-primary`, `text-text-secondary`, `text-text-muted`, `text-brass`, `font-mono`, `font-sans`) or `var(--color-*)` / `var(--font-mono)`.
6. **Unique `textbox` Landmark on `Console` Mount:**
   - `Console.test.tsx` (`L91, 99, 153, 165, 226`) and `InputEditor.test.tsx` (`L18, 27, 38, 57, 79`) call `screen.getByRole('textbox')` without a name filter.
   - Keep `<textarea aria-label="console input">` as the **only** mounted `role="textbox"` on default Console render:
     - `HistorySearchPopover` MUST return `null` when closed, and use `<input type="search">` (`role="searchbox"`) when open.
     - `RunbookPanel` filter input MUST use `<input type="search">` (`role="searchbox"`), and `DiscoveryRail` defaults to the `Help` tab (`RunbookPanel` unmounted until `Runbooks` tab is clicked).
     - `JsonLedgerTable` filter input MUST use `<input type="search">` (`role="searchbox"`), and `BlockCard` defaults to `'json'` view until `Table` is toggled.
     - Intent mode picker MUST use `<VoxSelect>` (`role="combobox"`).

---

## 5. Copy-Ready Vitest Harness Template

Mirror `crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx` (`L1-75`) when testing Phase 444 Console components:

```tsx
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';

const termCompleteMock = vi.fn().mockResolvedValue([]);
const termHistoryQueryMock = vi.fn().mockResolvedValue([]);
const termHistoryRecordMock = vi.fn().mockResolvedValue(true);
const termRunbooksListMock = vi.fn().mockResolvedValue([]);
const termSessionVoxifyMock = vi.fn().mockResolvedValue(undefined);
const discoverySuggestMock = vi.fn().mockResolvedValue([]);

vi.mock('../../../transport', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../../transport')>();
  return {
    ...actual,
    termComplete: (...a: unknown[]) => termCompleteMock(...a),
    termHistoryQuery: (...a: unknown[]) => termHistoryQueryMock(...a),
    termHistoryRecord: (...a: unknown[]) => termHistoryRecordMock(...a),
    termRunbooksList: (...a: unknown[]) => termRunbooksListMock(...a),
    termSessionVoxify: (...a: unknown[]) => termSessionVoxifyMock(...a),
    discoverySuggest: (...a: unknown[]) => discoverySuggestMock(...a),
  };
});
```

### Verification Commands (Scoped — Never Workspace-Wide)
- **Frontend Unit + Static Guards:**
  `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console src/transport.console.test.ts src/guards`
- **Frontend Typecheck:**
  `timeout 300s pnpm --dir crates/vox-gui/ui exec tsc --noEmit`
- **Rust Scoped Tests & Formatting:**
  - `rustfmt --edition 2024 <changed .rs files>`
  - `timeout 900s cargo test -p vox-terminal-core`
  - `timeout 900s cargo test -p vox-gui --lib`
