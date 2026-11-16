---
phase: 444
slug: warp-editor-completions-and-runbooks
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-11-15"
validated: "2026-11-15"
---

# Phase 444 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework (Rust)** | `cargo test` (`vox-terminal-core`, `vox-orchestrator-d`, `vox-gui` check) |
| **Framework (Frontend)** | `vitest` 4.1.2 + `@testing-library/react` 16.3.2 (`crates/vox-gui/ui`) |
| **Config file** | `crates/vox-gui/ui/vite.config.ts` |
| **Quick run command** | `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/` |
| **Full suite command** | `timeout 600s cargo test -p vox-terminal-core && timeout 600s cargo test -p vox-orchestrator-d term_dispatch && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/ src/components/layout/Omnibar.test.tsx src/guards/` |
| **Estimated runtime** | ~15 seconds (Vitest) + ~35 seconds (`vox-terminal-core` + `vox-orchestrator-d`) |

---

## Sampling Rate

- **After every task commit:** Run task-scoped `cargo test -p vox-terminal-core <filter>` or `pnpm --dir crates/vox-gui/ui exec vitest run <test-file>`
- **After every plan wave:** Run full `cargo test -p vox-terminal-core` + Console & guard Vitest suite + `pnpm --dir crates/vox-gui/ui exec tsc --noEmit`
- **Before `/gsd-verify-work`:** Full Rust + Frontend Console/Omnibar/Guard suites must be green
- **Max feedback latency:** 45 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 444-01-01 | 01 | 1 | TERM-WARP-02, TERM-WARP-03 | T-444-01, T-444-02 | `HistoryStore` scrubs secrets via `vox-redact`, skips leading-space (`ignorespace`), writes `0600` JSONL; `CompletionEngine` confines path lookup & caches `$PATH` | Rust unit | `timeout 300s cargo test -p vox-terminal-core history completions` | ✅ | ✅ green |
| 444-01-02 | 01 | 1 | TERM-WARP-04 | T-444-03, T-444-04 | `scan_workspace_runbooks` rejects symlink escapes outside workspace root; `voxify` builds closed-allowlist shell hooks with `OSC 633;E` escaping | Rust unit | `timeout 300s cargo test -p vox-terminal-core runbooks voxify` | ✅ | ✅ green |
| 444-01-03 | 01 | 1 | TERM-WARP-02, TERM-WARP-03, TERM-WARP-04 | T-444-01, T-444-04 | Tauri IPC `term_history_*`, `term_complete`, `term_runbooks_list`, `term_session_voxify` scrub via `vox-secrets` + `vox-redact`; daemon `term.voxify` enforces `refuse_agent_origin`; typed `transport.ts` wrappers degrade offline | Rust check + Vitest | `timeout 600s cargo test -p vox-orchestrator-d term_dispatch && timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/transport.console.test.ts src/guards/commandBackendsMigration.test.ts` | ✅ | ✅ green |
| 444-02-01 | 02 | 2 | TERM-WARP-01 | T-444-04b | Multi-line `ScriptoriumPromptEditor` (`Enter` submit, `Shift+Enter` newline, auto-grow 1-10 lines, multi-line paste warning banner, live token highlight) | Vitest | `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/ScriptoriumPromptEditor.test.tsx src/components/surfaces/Console/InputEditor.test.tsx` | ✅ | ✅ green |
| 444-02-02 | 02 | 2 | TERM-WARP-02 | T-444-01, T-444-01b | `HistorySearchPopover` (`Ctrl+R`, `ArrowUp` boundary recall, cwd-boosted fuzzy match, `[REDACTED]` badge, `Enter` run / `Tab` insert, browser fallback secret scrubbing, Clear Command History two-click confirm) | Vitest | `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/HistorySearchPopover.test.tsx` | ✅ | ✅ green |
| 444-03-01 | 03 | 3 | TERM-WARP-03 | T-444-02 | Hybrid `terminalCompletions` merges `termComplete` (`$PATH`, cwd files, git refs, CLI flags) with `intellisenseEngine` using UTF-8 byte offsets; fish-style ghost suffix accepts on `ArrowRight`/`End` | Vitest | `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/terminalCompletions.test.ts src/components/surfaces/Console/ScriptoriumPromptEditor.test.tsx` | ✅ | ✅ green |
| 444-03-02 | 03 | 3 | TERM-WARP-03 | T-444-05 | `JsonLedgerTable` (`@tanstack/react-table` v9 `useTable`) renders structured JSON blocks with `JSON`/`Table` toggle, column sorting, cell copy, 1 MiB / 200-row / 12-col caps, and zero `dangerouslySetInnerHTML` | Vitest | `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/JsonLedgerTable.test.tsx src/components/surfaces/Console/BlockStreamView.test.tsx` | ✅ | ✅ green |
| 444-04-01 | 04 | 3 | TERM-WARP-04 | T-444-03, T-444-06 | `RunbookPanel` + `DiscoveryRail` Runbooks tab + `Omnibar` (`Cmd+K`) Runbook facet & parameter sheet quote shell args safely, validate required params, and insert/execute in active session | Vitest | `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/RunbookPanel.test.tsx src/components/surfaces/Console/DiscoveryRail.test.tsx src/components/layout/Omnibar.test.tsx` | ✅ | ✅ green |
| 444-04-02 | 04 | 4 | TERM-WARP-01, TERM-WARP-02, TERM-WARP-03, TERM-WARP-04 | T-444-04, T-444-07 | `Console.tsx` + `ConsoleSessionBar.tsx` + `TerminalSessionPane.tsx` wire `ScriptoriumPromptEditor`, `Ctrl+R`, `DiscoveryRail` Runbooks, and `Voxify Subshell` button/banner with sanitized error toasts and static UI guards passing | Vitest + Guards | `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/Console.test.tsx src/guards/zeroEmojiAndDingbat.test.ts src/guards/noRawNativeControlsGuard.test.ts src/guards/transportIpcGuard.test.ts src/guards/toastBodyGuard.test.ts` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `crates/vox-terminal-core/src/history.rs` — inline `#[cfg(test)]` unit suite for redaction, `ignorespace`, dedup, `0600` permissions, and `cwd`-boosted fuzzy ranking (`5/5` green in `137/137` `vox-terminal-core` suite)
- [x] `crates/vox-terminal-core/src/completions_tests.rs` + `runbooks_tests.rs` + `voxify.rs` — unit tests for `$PATH` caching, path confinement, symlink escape rejection, `.vox` synthesized `--<param>` templates, and subshell `OSC 633`/`OSC 7` hook generation (`18` unit tests green in `137/137` `vox-terminal-core` suite)
- [x] `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.test.tsx` — multi-line editing, `Shift+Enter`, paste warning banner, live syntax token classes, and ghost text (`5/5` green)
- [x] `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.test.tsx` — `Ctrl+R` search, `cwd` boost, `[REDACTED]` badge, `Enter` run vs `Tab` insert, and Clear Command History confirm (`3/3` green)
- [x] `crates/vox-gui/ui/src/components/surfaces/Console/terminalCompletions.test.ts` — hybrid completion merge, multi-byte UTF-8 byte-offset conversion, and ghost-text suffix computation (`7/7` green)
- [x] `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.test.tsx` + `RunbookPanel.test.tsx` — `@tanstack/react-table` v9 sorting/truncation/empty-JSON states (`3/3` green) and Runbook parameter shell quoting + missing-required-param validation (`3/3` green)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Live SSH / `docker exec -it` subshell `Voxify Subshell` visual prompt re-framing in Tauri webview | TERM-WARP-04 | Requires an interactive nested SSH or container TTY session inside the desktop webview | Run `docker exec -it <container> bash` or `bash --norc` in Console, verify `Nested subshell detected - prompt framing paused` banner + `Voxify Subshell` button appear, click `Voxify Subshell`, run `ls` and `cd ..`, verify new `BlockCard`s frame cleanly with updated `cwd` chip. (Hook payload generation, base64 encoding, `stty -echo`, `OSC 633;E` `\x3b` escaping, and `Console.test.tsx` UI trigger wiring are covered by automated unit tests.) |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (`9/9` verification rows green; `137/137` `vox-terminal-core` Rust tests + `80/80` Console/Omnibar Vitest tests + `39/39` transport & UI guard tests + `tsc --noEmit` exit `0`)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 45s (`5.32s` Console/Omnibar Vitest + `2.72s` transport/guards Vitest)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-11-15
