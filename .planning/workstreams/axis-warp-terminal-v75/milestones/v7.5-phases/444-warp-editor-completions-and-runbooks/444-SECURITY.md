---
phase: "444"
slug: "warp-editor-completions-and-runbooks"
status: verified
threats_open: 0
asvs_level: 1
block_on: high
created: "2026-11-15"
---

# Phase 444 — Security

> Per-phase security contract: Threat register & verification audit for Warp Editor, Completions, Redacted History, Subshell Voxify, Structured JSON Ledger, and Workspace Runbooks (`TERM-WARP-01..04`).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Prompt Input -> Persistent History (`HistoryStore` / `localStorage`) | User shell commands submitted via `ScriptoriumPromptEditor` cross `term_history_record` into `~/.vox/terminal/history.jsonl` (or browser `localStorage` fallback when offline). | Raw command strings potentially containing inline API keys, Bearer tokens, PEM headers, password flags, or ANSI escape sequences |
| Webview Completion Request -> Host Filesystem & `.git` (`term_complete`) | Typing in `ScriptoriumPromptEditor` invokes `term_complete` (`line`, `cursor`, `cwd`, `shellMode`, `limit`). | User-typed path fragments (`../`, `.env`, `~/`, multi-byte UTF-8 tokens) and active session `cwd` |
| Workspace Files -> Runbook Scanner & Parameter Form (`term_runbooks_list` / `RunbookPanel` / `OmnibarRunbookSheet`) | Scanning `scripts/**/*.vox` and `docs/src/how-to/*.md` and interpolating user-supplied `{{param}}` values into shell commands. | Workspace file paths (including potential symlinks), `@param` metadata, and arbitrary user parameter input containing quotes, spaces, `$()`, backticks, or `;` |
| Subshell Voxify Action -> Live PTY Stdin (`term_session_voxify` / `term.voxify`) | Human click on `Voxify Subshell` (or `/voxify`) writes an `OSC 633` + `OSC 7` hook snippet into the active session's nested subshell (`ssh`, `docker`/`podman`/`kubectl exec -it`, `sudo -i`, `bash`/`zsh`/`fish`/`pwsh`/`nu`). | Optional `shell_hint` string from frontend + nested subshell `$BASH_COMMAND` / `$1` / `$argv[1]` / `PSConsoleReadLine` buffer echoed back via `OSC 633;E` |
| Untrusted Command Stdout -> Structured JSON Ledger (`BlockCard` / `JsonLedgerTable`) | Terminal block output detected as structured JSON (or `!nu` / `/nu` output) is parsed and rendered as a sortable interactive table. | Arbitrary process stdout payloads (potentially multi-megabyte arrays, wide objects, or HTML/script payloads) |
| Clipboard Paste -> Prompt Composer (`ScriptoriumPromptEditor`) | Multi-line clipboard text pasted into the terminal prompt editor. | Multi-line shell payloads containing embedded `\n` / `\r\n` that traditional terminals would execute immediately |
| Backend IPC Errors -> UI Toast Surface (`useConsoleWarpActions` / `RunbookPanel` / `Console`) | Rejected `term_*` Tauri IPC calls surfaced to operator toasts. | Raw Rust/anyhow/daemon error strings that could carry filesystem paths or credentials |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-444-01 | Information Disclosure | `crates/vox-terminal-core/src/history.rs` + `crates/vox-gui/src/commands/pty/warp.rs` + `crates/vox-terminal-core/src/pty.rs` | mitigate | 4-layer secret defense before any disk write: (1) `HistoryStore::record_with_scrubber` (`history.rs:84-86`) rejects leading-space commands BEFORE `.trim()` (`HISTCONTROL=ignorespace`) and `apply_pty_history_env` (`pty.rs:445-448,473`) sets `HISTCONTROL=ignorespace` + `HIST_IGNORE_SPACE=1` on spawned PTYs; (2) `strip_ansi_and_normalize` (`history.rs:90,342-383`) strips CSI/OSC escapes; (3) `history_record_impl` (`warp.rs:238-253`) passes `&vox_secrets::scrub_resolved_secrets` to scrub live Clavis secret values; (4) `vox_redact::redact_owned` (`history.rs:96`) redacts `sk-*`, `ghp_*`, `AKIA*`, `Bearer <jwt>`, PEM blocks, and `PASSWORD=`/`TOKEN=`/`SECRET=` assignments, marking `redacted: true`. Persisted via tmp+rename (`history.jsonl.tmp` -> `history.jsonl`) with `0o700` dir and `0o600` file permissions on Unix (`history.rs:257-295`). | closed |
| T-444-01b | Information Disclosure | `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx` | mitigate | Offline browser `localStorage` fallback (`gui.console.history_fallback.v1`, capped at `MAX_LOCAL_FALLBACK = 100`) enforces leading-space `ignorespace` drop (`HistorySearchPopover.tsx:78`) and runs `scrubClientFallbackSecrets` (`HistorySearchPopover.tsx:42-55,81`) to redact `Bearer`, `sk-*`, `ghp_*`, `gho_*`, `github_pat_*`, `AKIA*`, PEM blocks, and secret env/flag assignments before `writeLocalHistory` (`L57-70`), falling back cleanly to in-memory ring when Web Storage is disabled (`L30-40`). | closed |
| T-444-02 | Information Disclosure / Denial of Service | `crates/vox-terminal-core/src/completions.rs` + `crates/vox-terminal-core/src/completions_specs.rs` | mitigate | `complete_paths` (`completions.rs:298-389`) canonicalizes `cwd` via `std::fs::canonicalize`, caps upward parent traversal to at most 1 `..` hop above canonical `cwd` (`parent_hops <= 1` at `L342-353`), hides dotfiles unless the leaf prefix explicitly begins with `.` (`L359`), caps `std::fs::read_dir` iteration to `MAX_DIR_SCAN_ENTRIES = 256` (`L16,356`), caches `$PATH` scans for 30s (`PATH_CACHE_TTL = 30s` at `L14,213-223`), and reads `.git/HEAD`, `refs/`, and `packed-refs` (`<= 2 MiB`, `<= 256` refs) in pure Rust with zero `git` subprocess spawns (`completions_specs.rs:349-448`). | closed |
| T-444-03 | Tampering / Elevation of Privilege (Workspace Escape) | `crates/vox-terminal-core/src/runbooks.rs` + `crates/vox-gui/src/commands/pty/warp.rs` | mitigate | `scan_workspace_runbooks` (`runbooks.rs:29-56`) resolves `canon_root = fs::canonicalize(workspace_root)` (anchored via `vox_repository::discover_repository_or_fallback` in `warp.rs:303-310`), walks only `scripts/` and `docs/src/how-to/` to depth `<= 3` (`walk_runbook_dir` at `L85-125`), canonicalizes every candidate entry (`fs::canonicalize(&path)` at `L104`), and drops any file or symlink whose canonical path does not satisfy `canon_entry.starts_with(&canon_root)` (`L107-109`), while enforcing `MAX_RUNBOOKS = 128` and `MAX_RUNBOOK_FILE_BYTES = 256 KiB` (`L8-9,116-119`). | closed |
| T-444-04 | Tampering / Command Injection | `crates/vox-terminal-core/src/voxify.rs` + `crates/vox-orchestrator-d/src/term_dispatch.rs` + `crates/vox-gui/ui/src/components/surfaces/Console/{ConsoleSessionBar.tsx,TerminalSessionPane.tsx}` | mitigate | `VoxifyShellKind::from_hint` (`voxify.rs:21-41`) is a closed 5-arm enum (`Bash`, `Zsh`, `Fish`, `Pwsh`, `Nu`); arbitrary hint strings never interpolate into shell code (`build_voxify_payload` selects compile-time static hook constants at `voxify.rs:196-233` and base64-encodes them with a leading space for `ignorespace` and POSIX `stty -echo` ... `stty echo; clear`). Every shell hook (`voxify.rs:235-315`) escapes `\`, `;` (`\x3b`), `\r` (`\x0d`), and `\n` (`\x0a`) in command text before emitting `OSC 633;E;%s\a` so user commands containing `;` or newlines cannot forge `OSC 633` frames. Daemon `term.voxify` (`term_dispatch.rs:373-379`) enforces `refuse_agent_origin(p)?` so agents cannot invoke subshell hook injection; UI triggers require an explicit human click (`ConsoleSessionBar.tsx:149-164`, `TerminalSessionPane.tsx:409-417`, or user-typed `/voxify`). | closed |
| T-444-04b | Tampering (Clipboard Multi-Line Auto-Exec) | `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx` | mitigate | `handlePaste` (`ScriptoriumPromptEditor.tsx:260-286`) intercepts clipboard text containing `\n` or `\r`, calls `e.preventDefault()`, inserts the normalized text into the draft without submitting to the PTY, and displays a dismissible `[role="status"]` warning banner (`data-testid="term-multiline-paste-banner"`, `L339-358`) showing exact line count so pasted multi-line scripts never auto-execute before human review. | closed |
| T-444-05 | Denial of Service / Cross-Site Scripting | `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx` | mitigate | `tryParseStructuredJson` and `JsonLedgerTable` (`JsonLedgerTable.tsx:16-19,53-70,113-118`) enforce `MAX_JSON_PARSE_BYTES = 1 MiB` (`1,048,576` bytes) before `JSON.parse`, cap rendered rows at `MAX_LEDGER_ROWS = 200` (with an explicit `Showing first 200 of N rows` footer at `L281-289`), cap columns at `MAX_LEDGER_COLS = 12` (`+K more columns` indicator at `L268-276`), and render all keys/cells strictly as React text nodes with zero `dangerouslySetInnerHTML` or `innerHTML`. | closed |
| T-444-06 | Tampering / Command Injection (`{{param}}` Interpolation) | `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx` + `crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx` | mitigate | `quoteShellParam` (`RunbookPanel.tsx:27-31`, shared by `OmnibarRunbookSheet.tsx:10-13`) wraps non-safe values in POSIX single quotes and escapes embedded single quotes as `'\''` (returning `"''"` for empty strings so positional flags never collapse), neutralizing `;`, `&&`, `|`, `$()`, and backticks (`RunbookPanel.test.tsx` verifies `'safe; rm -rf /'` -> `"'safe; rm -rf /'"`). `getMissingRequiredParams` (`RunbookPanel.tsx:44-52`) blocks `Run Runbook` and `Insert into Prompt` with `role="alert"` until all required template parameters are non-empty. | closed |
| T-444-07 | Information Disclosure (IPC Error Toasts) | `crates/vox-gui/ui/src/components/surfaces/Console/{useConsoleWarpActions.ts,RunbookPanel.tsx,Console.tsx}` + `crates/vox-gui/ui/src/guards/toastBodyGuard.test.ts` | mitigate | Every backend catch path (`useConsoleWarpActions.ts:107`, `Console.tsx:251,269,306,330,372`) routes error payloads through `sanitizeErrorForToast(err)` from `lib/backendGuard.ts`; `RunbookPanel.tsx:102` surfaces static user-safe fallback copy. Enforced continuously by `src/guards/toastBodyGuard.test.ts` (`1/1` green). | closed |
| T-444-SC | Supply Chain / Contract Drift | `Cargo.lock`, `crates/vox-terminal-core/Cargo.toml`, `crates/vox-gui/ui/package.json`, `contracts/gui/command-backends.v1.yaml` | mitigate | Zero new Rust crates or npm packages introduced (`git diff` on `crates/vox-terminal-core/Cargo.toml`, `crates/vox-gui/ui/package.json`, and `contracts/gui/command-backends.v1.yaml` is empty). `@tanstack/react-table` (`^9.0.0-alpha.11`), `vox-redact`, `vox-secrets`, `vox-repository`, and `base64` were already in workspace manifests; the 6 `pty.rs` Tauri commands inherit `ModuleClass::Host` (`commandBackendsMigration.test.ts` `11/11` green). | closed |

*Status legend: `open` · `closed` · `accepted` (mitigate required for all HIGH severity threats since `block_on: high`).*

---

## Accepted Risks Log

No accepted risks.

---

## Security Audit Trail

| Audit Date | Stage | Threats Total | Closed | Open | Auditor | Notes |
|------------|-------|---------------|--------|------|---------|-------|
| 2026-11-15 | Plan ASVS L1 Threat Model (`444-01..04-PLAN.md`) | 10 | 0 | 10 | `gsd-planner` | Enumerated STRIDE threats `T-444-01..07` + `T-444-01b`, `T-444-04b`, `T-444-SC` across history persistence, completion confinement, runbook symlink confinement & shell quoting, subshell `voxify` injection, multi-line paste safety, JSON ledger DoS/XSS, and toast sanitization. |
| 2026-11-15 | Code + UI Review & Remediation (`444-REVIEW-FIX.md`) | 10 | 10 | 0 | `gsd-code-reviewer` + `gsd-debugger` | Hardened `RunbookPanel` empty-param shell quoting (`"''"`) + missing required-param gate (`CR-03`), UTF-8 byte-offset completion replacement (`CR-01`), `localStorage` probe safety in `HistorySearchPopover`, and verified `137/137` `vox-terminal-core` tests. |
| 2026-11-15 | Phase Verification (`444-VERIFICATION.md`) | 10 | 10 | 0 | `gsd-verifier` | Direct source audit of all Rust + TS/TSX security boundaries plus live Vitest verification (`80/80` Console/Omnibar tests + `39/39` transport & static security guard tests + `tsc --noEmit` exit `0`). |

---

## Sign-Off

- [x] All threats in Threat Register have a valid disposition (`mitigate` / `accept` / `transfer`)
- [x] All `block_on: high` severity threats are `closed` (`threats_open: 0`)
- [x] Pre-persistence 4-layer secret redaction (`ignorespace` + ANSI strip + `vox_secrets::scrub_resolved_secrets` + `vox_redact::redact_owned` + `0o700`/`0o600` permissions + browser fallback scrubber) verified in source and unit tests
- [x] Path completion 1-parent-hop confinement, dotfile gating, zero-subprocess `.git` ref scanning, and UTF-16 <-> UTF-8 byte-offset conversion verified
- [x] Runbook workspace canonical symlink confinement (`canon_entry.starts_with(&canon_root)`), `quoteShellParam` single-quote escaping, and required-parameter validation verified
- [x] Subshell `voxify` closed allowlist, `OSC 633;E` `\x3b`/`\x5c`/`\x0a` escaping, daemon `refuse_agent_origin` gate, and human-click UI gate verified
- [x] Multi-line clipboard paste intercept banner, `JsonLedgerTable` `1 MiB` / `200`-row / `12`-col DoS + zero-XSS caps, and `toastBodyGuard` verified

**Approval:** verified 2026-11-15
