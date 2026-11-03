# Phase 102 Verification: Browser Playwright + Songhand AI Harness Bootstrap & Chat/Dock Integration

**Status:** VERIFIED (`passed`)
**Date:** 2026-10-19
**Milestone:** `v1.8` — Shared-Package CV Flywheel, Chat/Loquela Unification, VS Code Docking, Urbs Civilization & Browser Songhand/Playwright Harness

---

## 1. Root-Cause Diagnosis: Why `Browser` Showed "Open Idle / Melvor Engine" (or a 480px Blank White Box)

### Evidence Found
1. In [`BrowserView.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Browser/BrowserView.tsx):
   - `previewUrl` initialized to `'http://127.0.0.1:3000'`.
   - `rawPreviewUrl` was computed as:
     ```ts
     const rawPreviewUrl = preview?.url ?? (previewUrl.trim() || null);
     ```
     without checking `preview?.active`.
   - Whenever `rawPreviewUrl` was truthy (which was **always** true on initial render because `previewUrl` defaulted to `'http://127.0.0.1:3000'`), `BrowserView` unconditionally mounted:
     ```tsx
     <iframe src={iframeSrc} className="w-full h-[480px] rounded-xl border border-border-subtle bg-white" />
     ```
2. **Why `e2e/screens/browser.png` showed "Open Idle — Clean-Room Melvor Engine":**
   - When the baseline `e2e/screens/browser.png` screenshot was captured earlier, a developer happened to have an unrelated local app (`fableforge` / "Open Idle — Clean-Room Melvor Engine") listening on `127.0.0.1:3000`. Because `BrowserView` mounted `<iframe src="http://127.0.0.1:3000" />` even before `Start preview` was clicked, Playwright captured whatever unrelated process was bound to port `3000`.
3. **Why `e2e/review/cv-flywheel/browser-after.png` showed a 480px solid white rectangle:**
   - When nothing was listening on `127.0.0.1:3000`, the unstarted `<iframe className="... bg-white" />` rendered an empty `about:blank`/error document with a blinding `bg-white` background inside the dark obsidian workbench.

### Fix Applied (`BROWSER-HARNESS-01` & `BROWSER-HARNESS-02`)
- Changed `rawPreviewUrl` in [`BrowserView.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Browser/BrowserView.tsx) to:
  ```ts
  const rawPreviewUrl = preview?.active ? preview.url : null;
  ```
  and replaced `bg-white` with `bg-bg-base`.
- Replaced the unstarted preview area with the **Playwright & Songhand Browser Harness Control Deck** (`data-testid="browser-harness-quick-deck"`), providing immediate one-click entry points:
  - `Open Songhand & Agent View` (`data-testid="browser-launch-songhand-cta"`)
  - `Run Playwright Suite` (`data-testid="browser-run-playwright-cta"`)
  - `Send to Chat` (`data-testid="browser-preview-send-to-chat"`)

---

## 2. Full-Capacity Songhand + Playwright + Chat Harness Integration (`BROWSER-HARNESS-03..06`)

| Requirement | Surface / File | Implementation & Verification |
|---|---|---|
| `BROWSER-HARNESS-03` | [`commands/browser.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/commands/browser.rs), [`main.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/main.rs) | Added & registered 7 Tauri commands wrapping `vox-orchestrator-mcp` browser tools: `browser_act` (`vox_browser_act`), `browser_extract` (`vox_browser_extract`), `browser_extract_json` (`vox_browser_extract_json`), `browser_click_ref` (`vox_browser_click_ref`), `browser_fill_ref` (`vox_browser_fill_ref`), `browser_wait_for` (`vox_browser_wait_for`), and `browser_read_dom` (`vox_browser_text` / `vox_browser_html`). |
| `BROWSER-HARNESS-04` | [`BrowserView.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Browser/BrowserView.tsx), [`refOverlay.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Browser/refOverlay.ts) | Added the **Songhand Agentic Browser Bar** (`data-testid="browser-songhand-bar"`) supporting `act`, `extract`, `extract_json` (with JSON Schema draft-07 input), `wait_for`, and `read_dom`, plus the **Interactive Accessibility Snapshot Ref (`[eN]`) Inspector** (`data-testid="browser-ref-inspector"`) with clickable `[eN]` overlay badges and per-ref `Click` / `Fill` actions. Fixed null-return crash in `refreshPages` / `refreshSessionStatus`. |
| `BROWSER-HARNESS-05` | [`ChatSurface.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx), [`Loquela.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx) | Registered `'browser'` (`BrowserDockPanel`, `data-testid="chat-dock-browser"`) in `OPT_IN_PANEL_IDS`, `CHAT_DOCK_COMPONENTS`, and `panelDefs` so the Browser harness can be docked side-by-side with Chat, Terminal, Execution, and Urbs Civilization. Wired `vox:browser-context` + `vox:atlas-mention` so clicking `Send Context to Chat` attaches `@browser(<url>)` and a URL context chip (`N refs`) in `Loquela`. |
| `BROWSER-HARNESS-06` | [`BrowserView.test.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Browser/BrowserView.test.tsx), [`tauriMock.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/e2e/lib/tauriMock.ts) | Verified via Vitest (`BrowserView.test.tsx`, `ChatSurface.test.tsx`, `Loquela.test.tsx`: `125/125 passed`), Rust (`cargo test -p vox-gui --bin vox-gui commands::browser`), and the 32-surface `@ngi/platform-ssot` Playwright CV Flywheel. |
