---
name: computer-use
description: "Use when the user wants something done in a native desktop app (Finder, System Settings, Calculator, Notes, any non-browser window). Snapshot the app's accessibility tree, act on refs in the background, verify with a fresh snapshot or screenshot. Not for web pages — use the browser tools there."
metadata:
  vox-id: "computer-use"
  vox-category: "interop"
  vox-tools: "vox_computer_permissions, vox_computer_list_apps, vox_computer_list_windows, vox_computer_snapshot, vox_computer_screenshot, vox_computer_act"
---

# Desktop computer use

The `vox_computer_*` tools drive one native app at a time without taking over the user's
screen. They are off by default; the session needs the `computer` capability profile.

## Loop

1. **Check access once.** If a call fails with a permission error, call
   `vox_computer_permissions` and tell the user which macOS permission to grant
   (Accessibility for snapshots and input, Screen Recording for screenshots). Do not retry
   blindly.
2. **Find the target.** `vox_computer_list_apps` (running apps) or
   `vox_computer_list_windows` (one app's windows). Address the app by name, bundle id, or
   `pid`.
3. **Snapshot.** `vox_computer_snapshot` with `interactive_only: true` returns a compact tree
   whose elements carry `[ref=eN]` plus a `snapshot_id`. Refs are only valid for that
   snapshot.
4. **Act on a ref.** `vox_computer_act` with `kind` (`click`, `double_click`, `right_click`,
   `press`, `set_value`, `type`, `key`, `scroll`, `move`, `drag`, `focus`), the `snapshot_id`,
   and `ref`. Prefer `set_value` for text fields and `press` for buttons: they use
   accessibility actions and never move the user's cursor.
5. **Verify.** Take a new snapshot (or `vox_computer_screenshot`) and confirm the change
   happened before reporting success. A successful `act` reply means the input was delivered,
   not that the app did what you wanted.

## Coordinates are the fallback

Only when an element has no usable ref (canvas, custom drawing), take
`vox_computer_screenshot` and pass `x`/`y` with `coords: "screenshot"` — pixels of that
screenshot. Re-screenshot after any window move or resize.

## Tiers

`tier` defaults to `auto`: accessibility action, else input posted to the app in the
background. `activate` brings the app to the front, acts, and restores the user's app and
cursor; use it only when background input is ignored, and say so. Set
`require_background: true` when the user is actively working and must not be interrupted.

## Safety

- `vox_computer_act` requires approval unless the session is in accept-all mode. Batch your
  intent into clear single actions so each approval is meaningful.
- Password managers and credential prompts are never read or driven; secure text fields are
  redacted in snapshots. Do not try to work around either.
- Never act on an app the user did not ask about.
