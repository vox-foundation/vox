---
phase: 444-warp-editor-completions-and-runbooks
reviewed: 2026-11-15T20:12:00Z
depth: standard
files_reviewed: 31
files_reviewed_list:
  - crates/vox-terminal-core/src/history.rs
  - crates/vox-terminal-core/src/completions.rs
  - crates/vox-terminal-core/src/completions_specs.rs
  - crates/vox-terminal-core/src/runbooks.rs
  - crates/vox-terminal-core/src/voxify.rs
  - crates/vox-terminal-core/src/pty.rs
  - crates/vox-terminal-core/src/host.rs
  - crates/vox-terminal-core/src/daemon_host.rs
  - crates/vox-terminal-core/src/lib.rs
  - crates/vox-orchestrator-d/src/term_dispatch.rs
  - crates/vox-gui/src/commands/pty.rs
  - crates/vox-gui/src/commands/pty/warp.rs
  - crates/vox-gui/ui/src/transport.ts
  - crates/vox-gui/ui/src/lib/terminalBridge.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/promptHighlighter.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/intentRouter.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/terminalCompletions.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/useConsoleWarpActions.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx
  - crates/vox-gui/ui/src/components/ui/IntelliSensePopover.tsx
  - crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx
  - crates/vox-gui/ui/src/components/layout/Omnibar.tsx
findings:
  critical: 3
  warning: 8
  info: 2
  total: 13
status: issues_found
---

# Phase 444: Code Review Report

**Reviewed:** 2026-11-15T20:12:00Z
**Depth:** standard
**Files Reviewed:** 31 production source files (+ `crates/vox-gui/src/commands/pty/tests.rs` inspected for test coverage)
**Status:** issues_found

## Summary

Phase 444 delivers a clean modular architecture across `vox-terminal-core`, `vox-orchestrator-d`, `vox-gui`, and the Console/Omnibar React surfaces: secret-scrubbed JSONL history (`history.rs` + `vox_secrets::scrub_resolved_secrets`), zero-subprocess contextual completions (`completions.rs` / `completions_specs.rs`), confined workspace runbook scanning (`runbooks.rs`), base64-framed subshell hook injection (`voxify.rs` + `HISTCONTROL=ignorespace`), multi-line `ScriptoriumPromptEditor` with transparent syntax-highlighted mirror, `JsonLedgerTable`, and `RunbookPanel` / `OmnibarRunbookSheet`.

Cross-layer seam inspection uncovered **3 `BLOCKER` (`Critical`) bugs**, **8 `WARNING` issues**, and **2 `INFO` items** affecting functional correctness on `TERM-WARP-01..04` paths:

1. **`CR-01` (`BLOCKER`):** JS UTF-16 `selectionStart` / `String.prototype.slice` indices are passed directly into Rust `CompletionEngine::complete` (which indexes UTF-8 bytes) and applied back onto JS UTF-16 strings without conversion, corrupting the prompt buffer whenever any multi-byte character precedes the cursor.
2. **`CR-02` (`BLOCKER`):** `HistorySearchPopover` emits `onSelect(cmd, 'accept')` on `Enter` / row click ("Enter Run"), but `ScriptoriumPromptEditor` ignores the `action` parameter and only inserts into the draft without executing (and omits `initialQuery`).
3. **`CR-03` (`BLOCKER`):** Runbook parameter interpolation rejects `{{ident=default}}` syntax emitted by `runbooks.rs`, drops all `@param` values on `.vox` runbooks that lack an explicit `// @cmd:` override, and allows executing runbooks with blank required parameters.

---

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: [`BLOCKER`] UTF-16 Code-Unit vs UTF-8 Byte Offset Mismatch Corrupts Buffer on Completion Acceptance

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/terminalCompletions.ts:77-83,241,248-255` and `crates/vox-terminal-core/src/completions.rs:84-95,278-290`

**Issue:** `ScriptoriumPromptEditor` tracks `cursorOffset` (`textarea.selectionStart`) and `value.length` in JavaScript **UTF-16 code units**. `resolveTerminalCompletions` (`terminalCompletions.ts:241`) passes `cursor: clamped` directly over Tauri IPC to `term_complete`. On the Rust side, `CompletionEngine::complete` (`completions.rs:84-95,278-290`) treats `req.cursor` as a **UTF-8 byte offset** (`req.line.is_char_boundary(i)`) and returns `replace_start` and `replace_end` as **UTF-8 byte offsets**. Then `applyTerminalSuggestion` (`terminalCompletions.ts:248-255`) slices the JS UTF-16 string directly with those UTF-8 byte offsets (`line.slice(0, start)` / `line.slice(end)`), and `computeCaseSensitiveGhost` (`terminalCompletions.ts:77-83`) likewise slices `line.slice(top.replaceStart, clampedCursor)`.

**Reproduction trace:**
1. Type `# — deploy\ngit che` (`—` U+2014 is 3 UTF-8 bytes vs 1 UTF-16 unit; byte/unit delta = `+2`) or `echo 🚀 && git che` (`🚀` U+1F680 is 4 UTF-8 bytes vs 2 UTF-16 units; delta = `+2`).
2. At end-of-line (`cursorOffset = 18` UTF-16 units), `termComplete` receives `cursor = 18`.
3. Rust slices `&req.line[..18]` -> `"# — deploy\ngit c"` (truncating `"he"`!) and returns `insert_text: "checkout"`, `replace_start: 17`, `replace_end: 18` (UTF-8 byte indices).
4. Pressing `Tab` runs `line.slice(0, 17) + "checkout" + line.slice(18)` on the UTF-16 string `"# — deploy\ngit che"`, producing `"# — deploy\ngit chcheckout"` (or splitting UTF-16 surrogate pairs when emoji appear near the token boundary).

**Fix:** Convert UTF-16 string offsets to UTF-8 byte offsets before calling `termComplete`, and map returned `dto.replace_start` / `dto.replace_end` UTF-8 byte offsets back to UTF-16 code-unit offsets inside `terminalCompletions.ts` before constructing `TerminalSuggestion`:

```ts
const utf8Encoder = new TextEncoder();

function utf16IndexToUtf8ByteOffset(str: string, utf16Index: number): number {
  const clamped = Math.max(0, Math.min(utf16Index, str.length));
  return utf8Encoder.encode(str.slice(0, clamped)).byteLength;
}

function utf8ByteOffsetToUtf16Index(str: string, targetByteOffset: number): number {
  if (targetByteOffset <= 0) return 0;
  let bytes = 0;
  for (let i = 0; i < str.length; i++) {
    const cp = str.codePointAt(i)!;
    const cpBytes = cp <= 0x7f ? 1 : cp <= 0x7ff ? 2 : cp <= 0xffff ? 3 : 4;
    const cuWidth = cp > 0xffff ? 2 : 1;
    if (bytes + cpBytes > targetByteOffset) return i;
    bytes += cpBytes;
    if (cuWidth === 2) i++;
    if (bytes === targetByteOffset) return i + 1;
  }
  return str.length;
}
```

Use `utf16IndexToUtf8ByteOffset(line, clamped)` for `cursor` in `resolveTerminalCompletions` (`line 241`) and map `dto.replace_start` / `dto.replace_end` through `utf8ByteOffsetToUtf16Index(line, ...)` in `assembleCompletions` (`lines 166-167`).

---

### CR-02: [`BLOCKER`] `HistorySearchPopover` `Enter` / Row Click (`action === 'accept'`) Never Executes the Selected Command

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:267-273` and `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx:124-125,185-191,301,337`

**Issue:** `HistorySearchPopover` renders footer instructions `"Up/Down Navigate - Enter Run - Tab Insert - Esc Cancel"` (`HistorySearchPopover.tsx:337`, matching `444-UI-SPEC.md:213`) and distinguishes `onSelect(entry.command, 'accept')` on `Enter` (`line 187`) and row primary click (`line 301`) from `onSelect(entry.command, 'insert')` on `Tab` (`line 190`) and the `Insert` action button (`line 325`). However, `ScriptoriumPromptEditor.tsx:270-273` drops the `action` parameter completely:

```tsx
onSelect={(cmd) => {
  setHistoryPopoverOpen(false); setValue(cmd); setCursorOffset(cmd.length);
  requestAnimationFrame(() => { textareaRef.current?.focus(); textareaRef.current?.setSelectionRange(cmd.length, cmd.length); });
}}
```

As a result, pressing `Enter` or clicking a history row in `Ctrl+R` never runs the command — it behaves identically to `Tab`. In addition, `ScriptoriumPromptEditor.tsx:267-270` never passes `initialQuery` to `<HistorySearchPopover>`, even though `HistorySearchPopover` supports `initialQuery` (`HistorySearchPopover.tsx:124,130,142`), so pressing `Ctrl+R` after typing a partial single-line filter starts with an empty search box.

**Fix:** Seed `initialQuery` when the current buffer is single-line, and branch on `action === 'accept'` to route through `classifyPromptIntent` + history recording + `onSubmit`:

```tsx
<HistorySearchPopover
  open={historyPopoverOpen}
  cwd={cwd}
  initialQuery={value.includes('\n') ? '' : value}
  onClose={() => {
    setHistoryPopoverOpen(false);
    updateBuffer(stashedBeforeCtrlR.current);
    requestAnimationFrame(() => textareaRef.current?.focus());
  }}
  onSelect={(cmd, action) => {
    setHistoryPopoverOpen(false);
    if (action === 'accept' && !disabled && cmd.trim()) {
      const acceptedIntent = classifyPromptIntent(cmd, effectiveMode);
      const recordFn = getTransportFn('termHistoryRecord');
      if (recordFn) void Promise.resolve(recordFn(cmd, cwd)).catch(() => null);
      recordClientFallbackHistory(cmd, cwd);
      void refreshIpcHistoryCache();
      setValue('');
      setCursorOffset(0);
      setPastedLineCount(0);
      setHistoryRecallCursor(-1);
      liveDraftRef.current = '';
      onSubmit(acceptedIntent.toSubmitPayload(cmd));
      requestAnimationFrame(() => textareaRef.current?.focus());
      return;
    }
    setValue(cmd);
    setCursorOffset(cmd.length);
    requestAnimationFrame(() => {
      textareaRef.current?.focus();
      textareaRef.current?.setSelectionRange(cmd.length, cmd.length);
    });
  }}
/>
```

---

### CR-03: [`BLOCKER`] Runbook Interpolation Leaves `{{param=default}}` Literal, Drops `.vox` `@param` Arguments Without `@cmd`, and Skips Required-Param Validation

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx:10,26-43,188-193`, `crates/vox-terminal-core/src/runbooks.rs:135-146,353-358`, and `crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx:144-161,206-208`

**Issue:** Three interlocking defects break parameterized runbook execution (`TERM-WARP-04`):

1. **`{{ident=default}}` placeholders are never substituted:** `collect_mustache_placeholders` in `crates/vox-terminal-core/src/runbooks.rs:353-358` parses `{{host=localhost}}` into `RunbookParam { name: "host", default_value: Some("localhost") }` while keeping `{{host=localhost}}` intact inside `command_template`. Yet `TEMPLATE_PARAM_RE` in `RunbookPanel.tsx:10` is `/\{\{\s*([A-Za-z_][A-Za-z0-9_]*)\s*\}\}/g`, which does not match `=default`. `interpolateRunbookCommand` leaves literal `{{host=localhost}}` in the command sent to the shell!
2. **`.vox` runbooks without `// @cmd:` silently discard all user-supplied `@param` values:** In `parse_vox_runbook` (`runbooks.rs:135-146`), `@param` headers populate `RunbookDescriptor.params`, while `command_template` defaults to `format!("vox run {rel_path}")` with zero `{{...}}` placeholders (verified by unit test fixture `scripts/deploy.vox` at `runbooks.rs:433,456-458`). When a user fills `env` and `tag` in `RunbookPanel` or `OmnibarRunbookSheet` and clicks **Run Runbook**, `interpolateRunbookCommand("vox run scripts/deploy.vox", values, params)` returns `"vox run scripts/deploy.vox"` unchanged — silently dropping every parameter!
3. **Empty required parameters execute unvalidated and unquoted:** `quoteShellParam('')` returns `''` (`RunbookPanel.tsx:27`) and neither `RunbookPanel.tsx:188-193` nor `OmnibarRunbookSheet.tsx:156-161,206-208` blocks execution or shows the `444-UI-SPEC.md:148` inline error when a required parameter (`default_value == null`) is empty. A template `deploy.sh --tag {{tag}} --prod` with empty `tag` executes as `deploy.sh --tag  --prod`, causing `--prod` to be parsed as the value of `--tag`.

**Fix:**
1. Update `TEMPLATE_PARAM_RE` in `RunbookPanel.tsx:10` to match optional `=default` suffixes, and quote empty strings as `''` when substituted:
   ```ts
   const TEMPLATE_PARAM_RE = /\{\{\s*([A-Za-z_][A-Za-z0-9_]*)(?:\s*=[^}]*)?\}\}/g;

   export function quoteShellParam(raw: string): string {
     if (raw.length === 0) return "''";
     if (/^[A-Za-z0-9_./:@=-]+$/.test(raw)) return raw;
     return `'${raw.replace(/'/g, `'\\''`)}'`;
   }
   ```
2. In `parse_vox_runbook` (`crates/vox-terminal-core/src/runbooks.rs:144-146`), when `cmd_override` is `None` and `!params.is_empty()`, synthesize `{{param}}` arguments onto `command_template` (e.g., `vox run scripts/deploy.vox -- --env {{env}} --tag {{tag}}`) or append unreferenced `params` in `interpolateRunbookCommand` when `runbook.kind === 'vox'`.
3. Enforce required parameter validation (`params.some((p) => !p.default_value && !(values[p.name] ?? '').trim())`) in both `RunbookPanel.tsx` and `OmnibarRunbookSheet.tsx`, disabling **Run Runbook** and rendering `<span role="alert">Parameter &lt;name&gt; is required before running this runbook.</span>` per `444-UI-SPEC.md:148,308`.

---

## Warnings

### WR-01: [`WARNING`] Backend `CompletionEngine` Scores (`74..101`) Are Unscaled vs Frontend Scores (`300..930`), and History Is Never Wired Into Ghost-Text Completions

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/terminalCompletions.ts:125,145,165,198,205-206`, `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:133,141`, `crates/vox-terminal-core/src/completions.rs:29,112-235`, and `crates/vox-gui/src/commands/pty/warp.rs:272-277`

**Issue:**
1. **10x score scale mismatch:** Rust `CompletionEngine` emits `score` values in the `74..101` range (`Executable` `82`, `Path` `86`, `Directory` `88`, `GitRef` `88`, `Flag` `90`, `Subcommand` `92`, `Builtin` `95`, `Slash` `96`), whereas frontend completion sources in `terminalCompletions.ts` use a `300..950` scale (`LOCAL_TERMINAL_SLASH` `930`/`700`, `@file:` `910`, `$ENV` `760`, plus `computeIntelliSenseItems` fuzzy matches in `300..1000`). `assembleCompletions` (`terminalCompletions.ts:165,205`) copies `score: dto.score` unscaled and sorts `b.score - a.score`, pushing every backend `$PATH` binary, filesystem path, git ref, and CLI flag below weak local fuzzy matches and preventing IPC completions from winning `items[0]` (`ghostSuffix`).
2. **History never reaches inline ghost-text autosuggest:** `TermCompletionKind::History` is declared in `completions.rs:29` and `PtyWarpState` holds `history: Arc<HistoryStore>` (`warp.rs:73`), yet `complete_impl` (`warp.rs:272-277`) never queries `warp.history`, and `ScriptoriumPromptEditor.tsx:133,141` never passes `history` in `CompletionContextOptions`! Consequently, typing a prefix of a previously run shell command produces no Fish-style history ghost-text autosuggestion.

**Fix:** Scale `dto.score` into the 0–1000 band in `terminalCompletions.ts:165` (`score: Math.min(990, dto.score * 10)`), and pass cached history commands into `computeLocalTerminalCompletions` / `resolveTerminalCompletions` in `ScriptoriumPromptEditor.tsx` (or append prefix-matched `HistoryStore` items in `warp::complete_impl` with `TermCompletionKind::History`).

---

### WR-02: [`WARNING`] Nushell `Voxify` Hook Appends Duplicate Closures on Re-Injection, `FISH_HOOK` Emits Unescaped `OSC 633;E`, and `PWSH_HOOK` Omits Pre-Exec `633;C`

**File:** `crates/vox-terminal-core/src/voxify.rs:222-231,274-305`

**Issue:**
1. **Non-idempotent `NU_HOOK` (`voxify.rs:303-305`):** Unlike `BASH_HOOK`, `ZSH_HOOK`, `FISH_HOOK`, and `PWSH_HOOK` (which all guard on `__VOX_SUBSHELL_633`), `NU_HOOK` unconditionally appends closures to `$env.config.hooks.pre_prompt` and `hooks.pre_execution`. Re-triggering `Voxify Subshell` in a `nu` subshell emits duplicate `633;D`, `633;A`, `633;B`, and `633;C` sequences on every prompt, creating phantom empty blocks. Also `build_voxify_payload` (`lines 223-230`) sets an unused `$env.__vox_b64` variable and pastes raw `NU_HOOK` without clearing the screen or emitting `633;E`/`633;P;Cwd`.
2. **Unescaped `FISH_HOOK` `633;E` (`voxify.rs:275`):** `printf '\e]633;E;%s\a\e]633;C\a' "$argv[1]"` does not escape `\`, `;`, or newlines (unlike `FISH_OSC633` in `pty.rs:89`), so semicolons or multiline commands in `fish` subshells truncate/corrupt the `OSC 633;E` frame.
3. **`PWSH_HOOK` missing `633;E` / `633;C` (`voxify.rs:294-299`):** `PWSH_HOOK` wraps `function global:prompt` (`633;D`, `OSC 7`, `633;A`, `633;B`) without installing a `PSReadLine` `Enter` handler for `633;E` / `633;C` (compare `PWSH_OSC633` in `pty.rs:148-158`), and `"file://localhost/$cwd"` produces `"file://localhost//..."` on POSIX paths where `$cwd` already begins with `/`.

**Fix:** Guard `NU_HOOK` with `if (($env.__VOX_SUBSHELL_633? | default false) == false) { $env.__VOX_SUBSHELL_633 = true; ... }` (and drop the unused `$env.__vox_b64` assignment), escape `\`, `;`, and `\n` in `FISH_HOOK` (`string replace -a`), and add the `PSReadLine` `Enter` `633;C` hook + slash-normalized `OSC 7` path to `PWSH_HOOK`.

---

### WR-03: [`WARNING`] Omnibar `Cmd+P` Runbook Rows Are Unreachable via `ArrowUp` / `ArrowDown` / `Enter` Keyboard Navigation

**File:** `crates/vox-gui/ui/src/components/layout/Omnibar.tsx:386,545-568,668` and `crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx:101-123`

**Issue:** `Omnibar.tsx:668` renders `<OmnibarRunbooksSection runbooks={matchingRunbooks} ... />` directly above `{facets.map(...)}`. However, `Omnibar`'s keyboard navigation (`rows = omnibarRowsInOrder(facets)` at `line 386`, `ArrowDown` / `ArrowUp` / `Enter` handlers at `lines 545-568`) only indexes `facets`. When a user opens `Cmd+P`, types a runbook title, and presses `ArrowDown` or `Enter`:
- If only runbooks match (`rows.length === 0`), `ArrowDown` and `Enter` do nothing (`if (list.length === 0) return;`).
- If other facet items also match below the Runbooks header, pressing `Enter` activates `facets[0]` *underneath* the visible Runbooks list instead of the top Runbook!

**Fix:** Include `matchingRunbooks` as a first-class `'runbooks'` facet in `facets` (with an `OmnibarRow` variant `{ kind: 'runbook', runbook }` handled in `activateRow` via `selectOmnibarRunbook(row.runbook)`) so `ArrowUp`, `ArrowDown`, `Enter`, and `aria-selected` highlight work uniformly across Runbooks and all other Omnibar sections.

---

### WR-04: [`WARNING`] Inline `ArrowUp` / `ArrowDown` History Recall Ignores Prefix Filtering and Retains Cleared History in `ipcHistoryCacheRef`

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:84,91-98,195-196,218-238,391`

**Issue:**
1. **Cleared history resurfaces on `ArrowUp`:** Clicking `Clear History` -> `Confirm Clear` in `HistorySearchPopover` (`HistorySearchPopover.tsx:165-172`) clears `~/.vox/terminal/history.jsonl` and `localStorage`, but `ScriptoriumPromptEditor`'s `ipcHistoryCacheRef.current` (`lines 84, 195-196`) is never cleared until a *new* command is submitted. Pressing `ArrowUp` immediately recalls the wiped commands from `ipcHistoryCacheRef.current`.
2. **Missing prefix filter on `ArrowUp`:** Both `444-CONTEXT.md:40` and `444-UI-SPEC.md:215` specify that `ArrowUp` on the first line cycles through *prefix-matched* history entries while preserving the live draft at index `-1`. Currently `handleKeyDown` (`lines 218-225`) indexes the unfiltered `getRecallHistoryList()`, so typing `cargo ` + `ArrowUp` replaces `cargo ` with an unrelated `ls -la` entry.
3. **Header toggle clobbers `stashedBeforeCtrlR`:** Clicking the `History` toolbar button (`line 391`) executes `stashedBeforeCtrlR.current = value` unconditionally even when `historyPopoverOpen` is already `true` (closing the popover).

**Fix:** Add an `onCleared?: () => void` prop to `HistorySearchPopover` that resets `ipcHistoryCacheRef.current = []` and `setHistoryRecallCursor(-1)` in `ScriptoriumPromptEditor`; filter `getRecallHistoryList()` by `liveDraftRef.current` prefix when `liveDraftRef.current.trim().length > 0`; and only snapshot `stashedBeforeCtrlR.current = value` when `!historyPopoverOpen`.

---

### WR-05: [`WARNING`] `scan_workspace_runbooks` Does Not Recurse Nested Directories, Truncates `read_dir` Before Filtering/Sorting, and Emits Empty Markdown Templates

**File:** `crates/vox-terminal-core/src/runbooks.rs:65-80,159-191`

**Issue:**
1. **No subdirectory recursion (`depth 1` only):** `read_confined_files` (`runbooks.rs:65-80`) reads only direct children of `scripts/`, `.vox/runbooks/`, and `runbooks/`, skipping nested layouts such as `scripts/ops/deploy.vox` or `.vox/runbooks/db/migrate.md` (`444-CONTEXT.md:15` specifies `scripts/**/*.vox` and `.vox/runbooks/**`).
2. **Premature `.take(MAX_RUNBOOKS * 2)` before extension filter and sort (`line 71`):** `entries.flatten().take(MAX_RUNBOOKS * 2)` caps raw `read_dir` entries *before* filtering for `.vox`/`.md` and *before* `paths.sort()`. In a `scripts/` directory with many helper assets/fixtures, valid runbooks can be non-deterministically omitted depending on filesystem `readdir` order.
3. **Empty `command_template` on Markdown files without runnable code fences (`lines 169-190`):** In `.vox/runbooks/` or `runbooks/` (`require_tag == false`), any `README.md` or doc without a `sh`/`bash`/`zsh`/`pwsh`/`nu`/`vox` fenced block yields `command_template: ""`, displaying an empty preview card that submits nothing when clicked.

**Fix:** Recurse up to bounded depth (e.g., depth 3) while enforcing `canonicalize()` + `starts_with(canon_root)` confinement on every directory/file, filter by extension *before* sorting and truncating, and add `if command_template.trim().is_empty() { return None; }` in `parse_markdown_runbook`.

---

### WR-06: [`WARNING`] Frontend `detectSubshellShellHint` Diverges from Backend `detect_subshell_command` on `kubectl exec`, Container Flags, and `env` Wrappers

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/useConsoleWarpActions.ts:17-42` (vs `crates/vox-terminal-core/src/voxify.rs:65-70,127-158`)

**Issue:**
1. Line `38` checks `a === '--interactive'` for `kubectl exec`, whereas `kubectl` uses `--stdin` (which backend `voxify.rs:130` checks). Running `kubectl exec --stdin --tty my-pod -- bash` fails to surface the `Subshell detected` banner in the Console UI.
2. Line `41` (`const tail = dashDash >= 0 ? rest.slice(dashDash + 1) : rest.slice(-1)`) inspects only the **final token** when `--` is omitted. For `docker exec -it web /bin/zsh -l`, `rest.slice(-1)` is `["-l"]`, which fails `detectSubshellShellHint("-l")` (`null`)! Conversely, if a user runs `docker exec -it bash` (container named `bash`), `rest.slice(-1)` falsely triggers on the container name. Backend `voxify.rs:138-156` properly strips valued flags (`-u`, `-w`, `-e`, `-c`, `-n`) and checks `pos.len() >= 2` -> `&pos[1..]`.
3. `detectSubshellShellHint` does not strip `env KEY=VAL zsh` wrappers (`voxify.rs:65-70`).

**Fix:** Align `detectSubshellShellHint` in `useConsoleWarpActions.ts` with `voxify.rs` by stripping `env` + `KEY=VAL` tokens, accepting `--stdin` on `kubectl exec`, and skipping container/pod positional arg 0 (`pos.slice(1)`) after valued flags (`-u`, `-w`, `-e`, `--env`, `--user`, `--workdir`, `-c`, `--container`, `-n`, `--namespace`).

---

### WR-07: [`WARNING`] `CompletionEngine::complete` Never Expands Bare `~` and Restricts Bare Filename Completion to 4 Commands

**File:** `crates/vox-terminal-core/src/completions.rs:118-127,301-328`

**Issue:**
1. **Bare `~` produces zero completions:** For `cd ~` (`prefix == "~"`), `is_path_tok` is `true` (`prefix.starts_with('~')`), `rfind('/')` is `None`, so `dir_part` is `""` and `name_prefix` is `"~"`. `complete_paths` (`line 304`) checks `dir_part.strip_prefix("~/")`, which is `None`, and then scans `base_cwd` for filenames literally starting with `"~"` instead of completing `"~/"` or entries in `user_home_dir()`.
2. **Overly narrow command allowlist for bare path arguments (`line 122`):** `is_path_tok` only triggers bare filename completion when `matches!(cmd0, "cd" | "ls" | "cat" | "source")`. Typing `git add Car`, `vox run scr`, `vim src`, or `rm tmp` without typing `./` first returns zero path completions (despite `444-CONTEXT.md:47` calling out `cd`/`cat`/`ls`/`vim`/`vox run`/`git add`).

**Fix:** Handle `prefix == "~"` explicitly in `complete_paths` (returning `~/` with `TermCompletionKind::Directory`), and allow `complete_paths` as a general fallback whenever `token_idx >= 1 && !prefix.starts_with('-') && !prefix.starts_with('$')` (keeping subcommands/git refs ranked above generic file matches when both apply).

---

### WR-08: [`WARNING`] UI Contract & Accessibility Gaps: Disabled `Subshell Voxified` Button Blocks Re-Injection, Static `Run` Label in Agent Mode, and Direct `lucide-react` Import

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx:147-157`, `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:293-299,397-405`, and `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx:11`

**Issue:**
1. **`ConsoleSessionBar.tsx:151` disables re-injection (`disabled={isVoxified}`):** `444-UI-SPEC.md:135` specifies that the `Subshell Voxified` pill remains clickable (`aria-label="Subshell voxified - click to re-inject hooks"`) so operators who enter a nested subshell inside raw VT (`TerminalTab`, where `Console.tsx` does not intercept typed lines) can re-inject `OSC 633` hooks. Setting `disabled={isVoxified}` prevents clicking it once voxified.
2. **`ScriptoriumPromptEditor.tsx:397-405` static `Run` button label:** When `intent.kind === 'agent'` (`?` prefix or Agent mode), the submit button still displays `Run` instead of switching to `Ask Agent` (`444-UI-SPEC.md:133`).
3. **`JsonLedgerTable.tsx:11` bypasses `Icons.tsx`:** Imports `ArrowDown, ArrowUp, ArrowUpDown` directly from `'lucide-react'` instead of `../../ui/Icons` (`444-UI-SPEC.md:24,43`: "`Icons.tsx` — Exclusive icon & keycap glyph source").

**Fix:** Remove `disabled={isVoxified}` on `[data-testid="term-voxify-btn"]` in `ConsoleSessionBar.tsx` (updating `aria-label` to `"Subshell Voxified — click to re-inject hooks"` so visible text remains a prefix per WCAG 2.5.3), render `{intent.kind === 'agent' ? 'Ask Agent' : 'Run'}` in `ScriptoriumPromptEditor.tsx:404`, and re-export/import sort icons via `crates/vox-gui/ui/src/components/ui/Icons.tsx`.

---

## Info

### IN-01: [`INFO`] `tryParseStructuredJson` Rejects Empty JSON Arrays `[]` and Objects `{}`

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx:55,64` and `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx:207`

**Issue:** `tryParseStructuredJson` returns `null` when `parsed.length === 0` (`line 55`) or `Object.keys(rec).length === 0` (`line 64`). Because `BlockCard.tsx:207` gates the `JSON | Table` pill on `parsedJson !== null`, valid structured pipelines returning zero rows (e.g., `!nu ls | where size > 99gb | to json` -> `[]`) hide the `JSON | Table` toggle and can never render `444-UI-SPEC.md:144`'s empty collection state (`Command returned 0 records ([])`).

**Fix:** Allow `[]` in `tryParseStructuredJson` (`{ kind: 'records', columns: [], rows: [] }`) and render the empty state row inside `JsonLedgerTable` when `data.rows.length === 0`.

---

### IN-02: [`INFO`] Duplicate `termRunbooksList` Transport Helpers and Missing Active `cwd` in `OmnibarRunbookSheet`

**File:** `crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx:24-40` and `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx:45-63`

**Issue:** `loadOmnibarRunbooks` (`OmnibarRunbookSheet.tsx:24-40`) duplicates `loadWorkspaceRunbooks` (`RunbookPanel.tsx:45-63`) and invokes `termRunbooksList()` without passing the active terminal session's `cwd` (`getTerminalBridgeState()?.cwd ?? undefined`). If the active terminal session has navigated into a nested repository/worktree, `RunbookPanel` lists that worktree's runbooks while `Cmd+P` scans `.` (process launch directory).

**Fix:** Reuse `loadWorkspaceRunbooks(getTerminalBridgeState()?.cwd ?? undefined)` from `RunbookPanel.tsx` (or a shared `runbookLoader.ts` helper) inside `Omnibar.tsx`.

---

_Reviewed: 2026-11-15T20:12:00Z_
_Reviewer: Antigravity (gsd-code-reviewer, standard depth)_
