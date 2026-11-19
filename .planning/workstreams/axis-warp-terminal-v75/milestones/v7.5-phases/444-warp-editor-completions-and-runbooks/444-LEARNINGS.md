---
phase: 444
phase_name: "warp-editor-completions-and-runbooks"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 34
  lessons: 3
  patterns: 20
  surprises: 3
missing_artifacts:
  - "UAT.md"
---

# Phase 444 Learnings: warp-editor-completions-and-runbooks

## Decisions

### Checked leading space/tab BEFORE trimming in HistoryStore::record_with_s...
Checked leading space/tab BEFORE trimming in HistoryStore::record_with_scrubber and injected HISTCONTROL=ignorespace + HIST_IGNORE_SPACE=1 in apply_pty_history_env so space-prefixed commands never persist to either shell or Vox JSONL history.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 444-01-SUMMARY.md

---

### Kept vox-terminal-core dependency graph completely unchanged by passing ...
Kept vox-terminal-core dependency graph completely unchanged by passing &vox_secrets::scrub_resolved_secrets, vox_config::paths::user_home_dir, and vox_repository::discover_repository_or_fallback from crates/vox-gui/src/commands/pty/warp.rs.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 444-01-SUMMARY.md

---

### Moved spawn_pty_forwarder and spawn_term_forwarder into commands/pty/war...
Moved spawn_pty_forwarder and spawn_term_forwarder into commands/pty/warp.rs and tests into commands/pty/tests.rs so commands/pty.rs sits at 402 non-blank LoC (<430 budget) while retaining all #[tauri::command] attributes directly in pty.rs to inherit ModuleClass::Host.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 444-01-SUMMARY.md

---

### Gated term.voxify in crates/vox-orchestrator-d/src/term_dispatch.rs with...
Gated term.voxify in crates/vox-orchestrator-d/src/term_dispatch.rs with Self::refuse_agent_origin(params, VOXIFY)? before reading session_id so MCP/agent origins cannot inject shell hook payloads into interactive subshells.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 444-01-SUMMARY.md

---

### Built crates/vox-terminal-core/src/history.rs (399 non-blank LoC) enforc...
Built `crates/vox-terminal-core/src/history.rs` (`399` non-blank LoC) enforcing pre-trim `HISTCONTROL=ignorespace` (`' '` / `'\t'`), 3-stage secret scrubbing (`strip_ansi_and_normalize` -> caller Clavis scrubber -> `vox_redact::redact_owned`), `0o700` dir / `0o600` atomic `.tmp` rename JSONL persistence, consecutive command deduplication, `10,000`-entry ring cap, and subsequence + recency + `cwd`-affinity fuzzy search.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-01-SUMMARY.md

---

### Updated crates/vox-terminal-core/src/pty.rs (apply_pty_history_env) to i...
Updated `crates/vox-terminal-core/src/pty.rs` (`apply_pty_history_env`) to inject `HISTCONTROL=ignorespace` and `HIST_IGNORE_SPACE=1` on spawned interactive shells without overriding explicit caller overrides.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-01-SUMMARY.md

---

### Built crates/vox-terminal-core/src/completions.rs (449 non-blank LoC) an...
Built `crates/vox-terminal-core/src/completions.rs` (`449` non-blank LoC) and `completions_specs.rs` (`422` non-blank LoC) delivering `< 50 ms` zero-subprocess completions across `$PATH` executables (`30s` TTL cache), shell builtins, traversal-safe `cwd`-relative paths (dot-entries gated on leading `.` and directories suffixed with `/`), pure-Rust `.git` branches/tags/remotes (`packed-refs` + loose refs + linked git worktree `gitdir:`/`commondir` support), and curated CLI subcommand/flag specs (`vox`, `git`, `cargo`, `pnpm`, `npm`, `docker`, `kubectl`, `gh`, `rg`, `pytest`, `python`, `nu`, `pwsh`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-01-SUMMARY.md

---

### Built crates/vox-terminal-core/src/runbooks.rs (441 non-blank LoC) scann...
Built `crates/vox-terminal-core/src/runbooks.rs` (`441` non-blank LoC) scanning canonicalized `scripts/*.vox` (depth 2), `.vox/runbooks/*.{vox,md}` (depth 2), and `@runbook`-tagged `docs/src/how-to/*.md` (depth 1, `64 KiB` cap), rejecting symlinks escaping `workspace_root`, and deduplicating `@param` / frontmatter / `{{ident}}` parameters.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-01-SUMMARY.md

---

### Built crates/vox-terminal-core/src/voxify.rs (380 non-blank LoC) impleme...
Built `crates/vox-terminal-core/src/voxify.rs` (`380` non-blank LoC) implementing `VoxifyShellKind` (`Bash`, `Zsh`, `Fish`, `Pwsh`, `Nu`), `detect_subshell_command`, and closed-allowlist base64 `stty -echo` `OSC 633` + `OSC 7` hook injection (`build_voxify_payload`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-01-SUMMARY.md

---

### Kept HistorySearchPopover completely unmounted (return null) when open=f...
Kept HistorySearchPopover completely unmounted (return null) when open=false and used role='region' + <input type='search'> (role='searchbox') so screen.getByRole('textbox') remains unique across Console and InputEditor test suites.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 444-02-SUMMARY.md

---

### Guarded all transport IPC lookups via try/catch typeof fn === 'function'...
Guarded all transport IPC lookups via try/catch typeof fn === 'function' + Promise.resolve(fn(...)).catch(...) so partial vi.mock('../../../transport') stubs without importOriginal or without returned Promises never crash.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 444-02-SUMMARY.md

---

### Exported full 6-variant ShellMode ('default' | 'shell' | 'agent' | 'nu' ...
Exported full 6-variant ShellMode ('default' | 'shell' | 'agent' | 'nu' | 'vox' | 'slash') from intentRouter.ts and PromptShellMode from InputEditor.tsx while keeping InputEditor.tsx's ShellMode alias mapped to TermShellModeId ('default' | 'nu' | 'vox') during Wave 2 so untouched Console.tsx:157 passes tsc --noEmit cleanly until Wave 3 (444-04) widens Console/bridge wiring.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 444-02-SUMMARY.md

---

### Syntax Highlighter & Multi-Mode Intent Router (promptHighlighter.ts, int...
**Syntax Highlighter & Multi-Mode Intent Router (`promptHighlighter.ts`, `intentRouter.ts`)**: Tokenizes shell commands, Vox keywords, `--flags`, quoted strings, `$ENV_VARS`, pipe/redirect operators (`|`, `&&`, `||`, `;`, `>`, `>>`, `2>&1`), `/slash` commands, and `@macros` into deterministic React spans (`zero dangerouslySetInnerHTML`). Detects trailing `\`, trailing `|`/`&&`/`||`, and unclosed quotes for multi-line Enter continuation, and routes `!cmd`, `?prompt`, `/ask`, `/slash`, `.vox`, `!nu`, and 3+ word natural-language AI Assist hints without hijacking standard CLI commands (`git status on main`, `cargo test --workspace`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-02-SUMMARY.md

---

### Hybrid Terminal Completions & Normalized Popover (terminalCompletions.ts...
**Hybrid Terminal Completions & Normalized Popover (`terminalCompletions.ts`, `IntelliSensePopover.tsx`)**: Merges `transport.termComplete` (`$PATH` binaries, cwd paths, git refs, CLI flags) with local `@last_exit`, `@cwd`, `@selection`, `@diff`, `@file:<relpath>` (`voxWorkspaceDiscover`), `/clear`, `/ask`, and Vox CLI fallback completions, while normalizing `IntelliSensePopover.tsx` to strict 4px spacing, 4 font sizes (`11px`/`12px`), 2 weights (`400`/`600`), and plain `Enter` keycap hint (removing banned `\u21b5` return glyph).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-02-SUMMARY.md

---

### Ctrl+R Fuzzy History Search Popover (HistorySearchPopover.tsx)
**Ctrl+R Fuzzy History Search Popover (`HistorySearchPopover.tsx`)**: Renders `null` when closed, uses `role="region"` and `<input type="search">` (`role="searchbox"`) to preserve `screen.getByRole('textbox')` uniqueness, queries `transport.termHistoryQuery` with a client-scrubbed no-Tauri fallback (`HISTCONTROL=ignorespace` + secret redaction), highlights matched subsequence characters, and implements 2-step inline `Clear History` -> `Confirm Clear` / `Keep` confirmation.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-02-SUMMARY.md

---

### Multi-Line Warp Prompt Editor & Wrapper (ScriptoriumPromptEditor.tsx, In...
**Multi-Line Warp Prompt Editor & Wrapper (`ScriptoriumPromptEditor.tsx`, `InputEditor.tsx`)**: Implements synchronized `<pre>` highlight/ghost mirror + `<textarea rows={1} aria-label="console input">`, `Shift+Enter` newline, incomplete-line Enter continuation, multi-line paste safety banner (`term-paste-safety-banner`), outer-line `ArrowUp`/`ArrowDown` history recall, `Ctrl+R` stashed-draft open/Escape restore, `externalDraftRequest` + `historyOpenRequestSeq` props for Wave 3 `Console.tsx` integration, and full backward compatibility via `InputEditor.tsx`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-02-SUMMARY.md

---

### Used a two-tier completion state (computeLocalTerminalCompletions synchr...
Used a two-tier completion state (`computeLocalTerminalCompletions` synchronous memo + `resolveTerminalCompletions` debounced async IPC merge) so local `/slash`, `@macro`, and Vox CLI ghost text appears synchronously on keystroke while `$PATH`, filesystem, git ref, and `@file:` completions merge seamlessly once resolved.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-02-SUMMARY.md

---

### Defaulted BlockCard structured output view to 'json' with an explicit JS...
Defaulted BlockCard structured output view to 'json' with an explicit JSON / Table segmented switch (data-testid='term-ledger-view-toggle') and type='search' on term-ledger-filter so all pre-existing BlockStreamView assertions and Console getByRole('textbox') queries remain undisturbed.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 444-03-SUMMARY.md

---

### Defaulted DiscoveryRail active tab to 'help' (Action Help), moved useLab...
Defaulted DiscoveryRail active tab to 'help' (Action Help), moved useLabel('con-discovery') above the collapsed early return to guarantee stable React hook ordering across collapse/expand transitions, and replaced '»'/'«' guillemets with Icon.chevR / Icon.chevL.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 444-03-SUMMARY.md

---

### Wrapped transport.termRunbooksList / voxTransport.termRunbooksList resol...
Wrapped transport.termRunbooksList / voxTransport.termRunbooksList resolution in a defensive try/catch helper in both RunbookPanel and OmnibarRunbookSheet so existing DiscoveryRail.test.tsx and Omnibar.test.tsx partial vi.mock('../../transport') stubs never throw missing-export errors.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 444-03-SUMMARY.md

---

### Extracted Cmd+P runbook state, filtering, Runbooks section, and inline p...
Extracted Cmd+P runbook state, filtering, Runbooks section, and inline parameter launch sheet into OmnibarRunbookSheet.tsx (192 non-blank LoC), keeping Omnibar.tsx delta to 16 added lines.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 444-03-SUMMARY.md

---

### Built crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable....
Built `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx` (`269` non-blank LoC) using `@tanstack/react-table@9.3.1` v9 APIs (`tableFeatures({ rowSortingFeature, sortedRowModel: createSortedRowModel(), sortFns: { alphanumeric: sortFn_alphanumeric } })` + `useTable`), handling arrays of objects, single plain objects (`field` / `value` 2-column ledger), and scalar arrays (`index` / `value` ledger), enforcing a `1 MiB` payload guard and `500`-row display cap, `type="search"` filtering (`data-testid="term-ledger-filter"`), inline nested JSON cell expansion, and `Copy JSON` + RFC-4180 `Copy CSV` actions.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-03-SUMMARY.md

---

### Updated crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx ...
Updated `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx` (`297` non-blank LoC) with a `JSON` / `Table` segmented switch (`data-testid="term-ledger-view-toggle"`) defaulting to `'json'` so pre-existing `BlockStreamView` JSON `<pre>` tests and `getByRole('textbox')` queries remain unaffected until switched to `Table`, plus normalized `size-3.5` to `size-4`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-03-SUMMARY.md

---

### Built crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx...
Built `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx` (`189` non-blank LoC) loading workspace `.vox` and Markdown runbooks via guarded `termRunbooksList`, rendering `type="search"` filtering (`data-testid="term-runbook-filter"`), `No workspace runbooks found` empty state, `<VoxSelect>` choice / `<input>` text `{{param}}` editors, and shell-quoted `quoteShellParam` + `interpolateRunbookCommand` preview with `Insert` and `Run` actions.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-03-SUMMARY.md

---

### Updated crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail....
Updated `crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx` (`171` non-blank LoC) with `Action Help` (default `'help'`) and `Runbooks` (`'runbooks'`) tabs, replaced `»`/`«` guillemet characters with `<Icon.chevR>` / `<Icon.chevL>`, normalized fractional spacing to the 4px grid and semantic `vox-*` tokens, and hoisted `useLabel('con-discovery')` above the collapsed early return.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-03-SUMMARY.md

---

### Built crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx (1...
Built `crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx` (`192` non-blank LoC) providing `useOmnibarRunbooks`, `OmnibarRunbooksSection` (`data-testid="omnibar-runbook-row-{id}"`), and `OmnibarRunbookParamsSheet` (`data-testid="omnibar-runbook-params"`) with `<VoxSelect>` / `<input>` controls, shell-quoted command preview, `Insert in Console` (`mode: 'insert'`), and `Run in Console` (`mode: 'run'`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-03-SUMMARY.md

---

### Extracted HistorySearch sequence, DiscoveryRail tab state, onOpenRunbook...
Extracted HistorySearch sequence, DiscoveryRail tab state, onOpenRunbook bridge subscription, subshell shell-hint detection, and Voxify session tracking into useConsoleWarpActions.ts (89 non-blank LoC) so Console.tsx remains at 404 non-blank LoC.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 444-04-SUMMARY.md

---

### Widened Console.tsx shellMode state to PromptShellMode ('default' | 'she...
Widened Console.tsx shellMode state to PromptShellMode ('default' | 'shell' | 'agent' | 'nu' | 'vox' | 'slash') while projecting bridgeShellMode ('default' | 'nu' | 'vox') into publishTerminalBridgeState so all 6 editor modes work end-to-end without touching terminalBridge.ts.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 444-04-SUMMARY.md

---

### Guarded Console.tsx window keydown listener with e.target instanceof Nod...
Guarded Console.tsx window keydown listener with e.target instanceof Node before consoleRootRef.current?.contains(e.target) so synthetic or browser window-targeted keydown events never throw jsdom Node.contains TypeErrors.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 444-04-SUMMARY.md

---

### Skipped Cargo test invocations during Wave 3 per orchestrator Cargo-cont...
Skipped Cargo test invocations during Wave 3 per orchestrator Cargo-contention avoidance rule (zero .rs files modified in 444-04; parallel Phase 445 backend executor active) while running pre-built ./target/debug/vox-arch-check, full LoC audits across all 19 Phase 444 files, tsc --noEmit, and all 31 Vitest Console/Omnibar/transport/guard/theme suites.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 444-04-SUMMARY.md

---

### Console Warp Orchestration Hook (useConsoleWarpActions.ts, 89 non-blank ...
**Console Warp Orchestration Hook (`useConsoleWarpActions.ts`, `89` non-blank LoC)**: Extracted `historyOpenRequestSeq`, `railTab` (`'help' | 'runbooks'`), `selectedRunbookId`, `consumePendingOpenRunbook` + `onOpenRunbook` subscription (`'run'`, `'insert'`, `'open_rail'`), pure `detectSubshellShellHint` (`bash`/`sh`, `zsh`, `fish`, `pwsh`/`powershell`, `nu`/`nushell`, `sudo -i/-s`, `ssh`, interactive `docker`/`podman`/`kubectl exec`), and partial-mock-safe `handleVoxifySubshell` + `resetSessionVoxifiedIfSubshell` with `sanitizeErrorForToast` error reporting.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-04-SUMMARY.md

---

### 1-Click Voxify Subshell Toolbar & Contextual Subshell Strip (ConsoleSess...
**1-Click `Voxify Subshell` Toolbar & Contextual Subshell Strip (`ConsoleSessionBar.tsx`, `TerminalSessionPane.tsx`)**: Added non-nested `data-testid="term-voxify-subshell"` (`Voxify Subshell` -> disabled `Subshell Voxified`) in `ConsoleSessionBar.tsx`, added `!compact` nested-subshell detection strip (`data-testid="term-subshell-voxify-banner"` with `data-testid="term-voxify-subshell-inline"`) in `TerminalSessionPane.tsx`, and normalized legacy `size-3.5` / `h-7` classes onto strict `4px` multiples (`size-4`, `h-8`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-04-SUMMARY.md

---

### Unified Console.tsx Integration (404 non-blank LoC)
**Unified `Console.tsx` Integration (`404` non-blank LoC)**: Widened `shellMode` to `PromptShellMode` (all 6 modes) with `TermShellModeId` projection for `publishTerminalBridgeState`, wired `activeSessionId` and `activeCwd` into `InputEditor` and `DiscoveryRail`, updated `Cmd+J` to focus `[aria-label="console input"]`, wired `Ctrl+R` to `openHistorySearch()` without opening any `role="dialog"`, and guarded `window` `keydown` `Node.contains` checks.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-04-SUMMARY.md

---

### Integration Test Suite (Console.test.tsx, 385 non-blank LoC)
**Integration Test Suite (`Console.test.tsx`, `385` non-blank LoC)**: Added end-to-end regression tests covering unique `screen.getByRole('textbox')` on mount, `Cmd+J` textarea focus, `Ctrl+R` `HistorySearchPopover` (`role="region"`, zero `role="dialog"`), `Shift+Enter` multi-line editing + `Enter` submit, `DiscoveryRail` Runbooks `Insert into Prompt` / `Run Runbook`, `requestOpenRunbook` bridge insertion, and `Voxify Subshell` success, subshell re-arm, and `sanitizeErrorForToast` failure handling.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 444-04-SUMMARY.md

## Lessons

### Initial formatting of ScriptoriumPromptEditor.tsx exceeded the < 430 non...
Initial formatting of `ScriptoriumPromptEditor.tsx` exceeded the `< 430` non-blank LoC cap (501 non-blank lines); consolidated verbose prop/state formatting down to 379 non-blank lines and re-verified all 9 Vitest suites + `tsc --noEmit`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 444-02-SUMMARY.md

---

### None required to gate Phase 444 completion
None required to gate Phase 444 completion. Live multi-theme Playwright + computer-vision screenshot convergence across all 4 Vox Roma themes (`light`, `dark`, `roman_light`, `roman_dark`) for the prompt editor, completion popover, JSON ledger, and runbook rail is tracked in **Phase 446 (`TERM-ROMA-CV-01..04`)**.

**Context:** Verified in phase acceptance checks.
**Source:** 444-VERIFICATION.md

---

### No gaps found
No gaps found. All 4 Phase 444 requirements (`TERM-WARP-01..04`), all 4 ROADMAP Success Criteria, all 13 Plan Must-Have Truths, all 12 Plan Prohibitions, all 26 Code & UI Review remediations (`444-REVIEW-FIX.md`), all 10 STRIDE threat mitigations (`444-SECURITY.md`), and all Nyquist task gates (`444-VALIDATION.md`) are verified and passing.

**Context:** Verified in phase acceptance checks.
**Source:** 444-VERIFICATION.md

## Patterns

### 3-stage terminal persistence scrubbing
3-stage terminal persistence scrubbing: strip_ansi_and_normalize -> Clavis scrub_resolved_secrets closure -> vox_redact::redact_owned prior to 0o600 atomic tmp+rename JSONL flush.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-01-SUMMARY.md

---

### Symlink-safe workspace discovery
Symlink-safe workspace discovery: canonicalize workspace root first, then canonicalize every candidate file and enforce can_file.starts_with(&can_root) before reading.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-01-SUMMARY.md

---

### Caller-injected secret scrubber closure (&dyn Fn(&str) -> String) keepin...
Caller-injected secret scrubber closure (&dyn Fn(&str) -> String) keeping vox-terminal-core Layer 2 free of vox-secrets / vox-config / vox-repository edges while wiring scrub_resolved_secrets in vox-gui

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-01-SUMMARY.md

---

### Pure-Rust filesystem & .git completion engine (zero Command::new subproc...
Pure-Rust filesystem & .git completion engine (zero Command::new subprocesses) supporting both standard .git directories and linked git worktree gitdir:/commondir indirection

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-01-SUMMARY.md

---

### Submodule split (commands/pty.rs + commands/pty/warp.rs + commands/pty/t...
Submodule split (commands/pty.rs + commands/pty/warp.rs + commands/pty/tests.rs) keeping #[tauri::command] entrypoints in pty.rs so gui_command_backends automatically inherits ModuleClass::Host with zero YAML/fixture churn

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-01-SUMMARY.md

---

### Explicit Arrow-navigation gate (completionNavigated) before Enter accept...
Explicit Arrow-navigation gate (completionNavigated) before Enter accepts a popover item: typing a command and pressing Enter submits immediately unless the operator explicitly navigated suggestions with ArrowUp/ArrowDown (Tab/ArrowRight accepts inline ghost text).

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-02-SUMMARY.md

---

### Client fallback history ring buffer mirrors backend HISTCONTROL=ignoresp...
Client fallback history ring buffer mirrors backend HISTCONTROL=ignorespace + secret scrubbing (sk-*, ghp_*, AKIA*, Bearer,TOKEN/SECRET/PASSWORD/API_KEY assignments) when running outside Tauri.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-02-SUMMARY.md

---

### Transparent-textarea + synchronized <pre> mirror overlay sharing TERMINA...
Transparent-textarea + synchronized <pre> mirror overlay sharing TERMINAL_MONO_FONT_STACK with zero dangerouslySetInnerHTML and ligatures disabled for pixel-exact caret alignment

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-02-SUMMARY.md

---

### Partial-mock-safe ESM transport lookup helper (getTransportFn + Promise....
Partial-mock-safe ESM transport lookup helper (getTransportFn + Promise.resolve().catch()) so legacy partial vi.mock('../../../transport') suites never throw

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-02-SUMMARY.md

---

### Unmounted-when-closed role=region + input[type=search] (role=searchbox) ...
Unmounted-when-closed role=region + input[type=search] (role=searchbox) secondary popover pattern preserving screen.getByRole('textbox') uniqueness on Console mount

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-02-SUMMARY.md

---

### Structured block ledger inspection
Structured block ledger inspection: JSON raw view by default with opt-in TanStack Table v9 Ledger Table (1 MiB byte ceiling + 500-row UI cap + RFC-4180 CSV export).

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-03-SUMMARY.md

---

### Shell-escaped runbook parameter substitution
Shell-escaped runbook parameter substitution: bare [A-Za-z0-9_./:@=-]+ values pass cleanly while whitespace/metacharacters/quotes are single-quote escaped via quoteShellParam.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-03-SUMMARY.md

---

### TanStack Table v9 (@tanstack/react-table@9.3.1) composable feature plugi...
TanStack Table v9 (@tanstack/react-table@9.3.1) composable feature plugin pattern (tableFeatures + rowSortingFeature + createSortedRowModel + sortFn_alphanumeric + useTable) matching DataTable.tsx

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-03-SUMMARY.md

---

### Shell-safe {{param}} interpolation (quoteShellParam wrapping non-bare to...
Shell-safe {{param}} interpolation (quoteShellParam wrapping non-bare tokens in single quotes with '\'' internal quote escaping) across RunbookPanel and OmnibarRunbookSheet

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-03-SUMMARY.md

---

### Partial Vitest ESM mock guard (try/catch typeof fn === 'function' over t...
Partial Vitest ESM mock guard (try/catch typeof fn === 'function' over transport.termRunbooksList and voxTransport.termRunbooksList) so partial vi.mock('transport') suites never throw

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-03-SUMMARY.md

---

### Operator-explicit subshell hook injection
Operator-explicit subshell hook injection: Voxify Subshell is triggered exclusively by human click, passes normalized shell hint ('bash'|'zsh'|'fish'|'pwsh'|'nu'), disables as Subshell Voxified once active, and re-arms automatically when a subsequent interactive subshell command is submitted.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-04-SUMMARY.md

---

### Sanitized toast error boundary
Sanitized toast error boundary: every Console IPC catch path routes errors through sanitizeErrorForToast(err) with cause: 'backend-error'.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-04-SUMMARY.md

---

### Dedicated Console Warp orchestration hook (useConsoleWarpActions.ts, 89 ...
Dedicated Console Warp orchestration hook (useConsoleWarpActions.ts, 89 non-blank LoC) keeping Console.tsx at 404 non-blank LoC well below the 450 god_object ceiling

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-04-SUMMARY.md

---

### Shared pure detectSubshellShellHint(command) parser mirroring vox-termin...
Shared pure detectSubshellShellHint(command) parser mirroring vox-terminal-core::voxify::detect_subshell_command across bash/zsh/fish/pwsh/nu, sudo -i/-s, ssh, and interactive docker/podman/kubectl exec

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-04-SUMMARY.md

---

### Distinct test landmarks for primary toolbar CTA (term-voxify-subshell in...
Distinct test landmarks for primary toolbar CTA (term-voxify-subshell in ConsoleSessionBar) vs contextual pane strip CTA (term-voxify-subshell-inline in TerminalSessionPane)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 444-04-SUMMARY.md

## Surprises

### Estimate variance on 444-02
Estimate variance on 444-02: planned 52000 tokens vs 22550 actual (0.43x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 444-02-PLAN.md, 444-02-SUMMARY.md

---

### Estimate variance on 444-03
Estimate variance on 444-03: planned 52000 tokens vs 21457 actual (0.41x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 444-03-PLAN.md, 444-03-SUMMARY.md

---

### Estimate variance on 444-04
Estimate variance on 444-04: planned 42000 tokens vs 16925 actual (0.40x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 444-04-PLAN.md, 444-04-SUMMARY.md

