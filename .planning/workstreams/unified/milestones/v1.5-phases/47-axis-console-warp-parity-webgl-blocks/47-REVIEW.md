---
phase: 47-axis-console-warp-parity-webgl-blocks
reviewed: 2026-10-14T01:52:00-07:00
depth: standard
files_reviewed: 11
files_reviewed_list:
  - crates/vox-gui/src/commands/pty.rs
  - crates/vox-gui/ui/src/transport.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.tsx
  - crates/vox-gui/ui/src/components/surfaces/Models/ModelsView.tsx
  - crates/vox-gui/ui/e2e/lib/tauriMock.ts
  - crates/vox-gui/ui/e2e/lib/operatorShellMock.ts
  - crates/vox-gui/ui/e2e/console-workbench.spec.ts
findings:
  critical: 1
  warning: 3
  info: 1
  total: 5
status: issues_found
---

# Phase 47: Code Review Report (`--fix` applied)

**Reviewed:** 2026-10-14  
**Depth:** Standard + Playwright & Computer Vision Audit  
**Files Reviewed:** 11  
**Status:** All findings fixed and verified (`status: fixed`)

## Summary

Phase 47 implements `TERM-05`..`TERM-08`: Tauri `term_session_*` commands in `pty.rs`, `WebglTerminalAddon` with context-loss fallback, `BlockStreamView` Warp-style structured block cards, multi-session tab bar + `continue in vox term` handoff in `Console.tsx`, and retirement of duplicate `terminal_core.rs` and `osc633.ts`.

During code review and computer-vision inspection of Playwright screenshots (`console-warp-parity-blocks.png`, `routing-explainer.png`), 1 critical issue and 3 warnings were identified and fixed.

## Critical Issues (Fixed)

### CR-01: `TerminalTab.tsx` skipped `SessionHost` lifecycle in Playwright mock mode and re-attached on every non-closing `SessionEvent`

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx`  
**Issue:**
1. `if (isMock) return;` at the top of `useEffect` prevented `termSessionOpen`, `termSessionAttach`, `onBlocksChange`, and `termSessionSubmit` from running when `window.__TAURI_CALLS__` was present in Playwright E2E tests.
2. `listenTermEvent` called `termSessionAttach(sid)` on every event (`BlockOpened`, `OutputAppended`, `BlockClosed`), which re-triggered backend forwarder replacement on intermediate events.
3. Mounting `TerminalTab` with `sessionId === null` before `Console.refreshSessions()` resolved caused `TerminalTab` to call `termSessionOpen` and create a duplicate empty session instead of attaching to `s-1`.

**Fix Applied:**
- Updated `TerminalTab.tsx` to execute the full `SessionHost` attach/submit/event lifecycle in both live and mock modes (only skipping xterm.js canvas creation when `isMock` is true).
- Restricted `listenTermEvent` re-attachment to closing/agent events (`isClosingOrAgentEvent`) and refreshed blocks directly after `termSessionSubmit` resolves.
- Guarded `sessionId === null` in `TerminalTab.tsx` while `Console.refreshSessions()` resolves `termSessionList()`.

## Warnings (Fixed)

### WR-01: Un-guarded `.unwrap()` on `Mutex::lock()` in `crates/vox-gui/src/commands/pty.rs`

**File:** `crates/vox-gui/src/commands/pty.rs`  
**Issue:** `manager.sessions.lock().unwrap()` and `manager.attached_forwarders.lock().unwrap()` could panic if a thread panicked while holding the mutex.  
**Fix Applied:** Added poison-safe `fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T>` (`m.lock().unwrap_or_else(|e| e.into_inner())`) across `pty.rs`.

### WR-02: Session tab block count badge in `Console.tsx` did not update when new blocks were appended

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx`  
**Issue:** `onBlocksChange={setBlocks}` updated `blocks` state for `BlockStreamView` but left `sessions[i].blocks` stale in `term-session-bar`.  
**Fix Applied:** Updated `onBlocksChange` in `Console.tsx` to synchronize the active session's `blocks` count in `sessions` state.

### WR-03: Computer-vision inspection of `routing-explainer.png` caught null dereference in `ModelsView.tsx` (`snap.cards`)

**File:** `crates/vox-gui/ui/src/components/surfaces/Models/ModelsView.tsx`  
**Issue:** When `get_models_snapshot` resolved to `null` in isolated routing E2E specs, `const cards = snap.cards` threw `TypeError: Cannot read properties of null (reading 'cards')`, surfacing an error toast in the bottom-right corner of `routing-explainer.png`.  
**Fix Applied:** Null-guarded `snap?.cards`, `snap?.routing`, `snap?.active_model`, and `snap?.providers` in `ModelsView.tsx`, and verified via computer vision on `routing-explainer.png` that the error toast is gone.

## Info

### IN-01: Pinned block visual affordance and `InputEditor` prompt placeholder

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.tsx`  
**Fix Applied:** Added `data-block-id`, `data-pinned`, and an amber border highlight (`border-amber-400/50`) for pinned blocks in `BlockStreamView.tsx`, plus a clear placeholder in `InputEditor.tsx`.
