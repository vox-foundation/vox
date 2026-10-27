---
name: browser-automation
description: "Use when the user wants something done on a web page — log in, fill a form, click through a flow, check that a page works. Snapshot, act on refs, wait for the result, read console and network events when something fails. For read-and-cite research use browser-research; for native desktop apps use computer-use."
metadata:
  vox-id: "browser-automation"
  vox-category: "interop"
  vox-tools: "vox_browser_open, vox_browser_goto, vox_browser_snapshot, vox_browser_click_ref, vox_browser_fill_ref, vox_browser_press, vox_browser_wait_for, vox_browser_events, vox_browser_handle_dialog, vox_browser_screenshot_viewport"
---

# Browser automation

## Loop

1. `vox_browser_open` (or `vox_browser_goto` on an existing page). Do not drive a page the
   user has locked in the GUI.
2. `vox_browser_snapshot` — accessibility tree with `[ref=eN]`. Act on refs, not CSS
   selectors.
3. `vox_browser_click_ref` / `vox_browser_fill_ref`. These wait for the element to be
   visible, stable, enabled, and unobscured before acting; pass `timeout_ms` (up to 30000)
   for slow pages instead of sleeping. `fill_ref` replaces the field's contents.
4. `vox_browser_press` for keys (`Enter`, `Tab`, `Control+a`).
5. Verify with `vox_browser_wait_for` or a fresh snapshot before reporting success.

## When something goes wrong

- **Nothing happened / page broken:** `vox_browser_events` with `kinds: ["console",
  "network"]` and `level: "error"` shows uncaught errors and failed requests since the last
  read (`since_seq`).
- **A dialog is blocking the page** (alert, confirm, prompt): `vox_browser_handle_dialog`
  with `accept` and, for prompts, `prompt_text`.
- **Element not actionable:** the error names the failed check (hidden, disabled, covered).
  Scroll, close the overlay, or wait — do not retry the same click unchanged.

## Safety

Password and other sensitive fields are refused for agent-driven fills; ask the user to
enter credentials themselves. Do not export or import cookies.
