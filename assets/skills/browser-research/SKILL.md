---
name: browser-research
description: "Use when the user wants to research, summarize, or cite what a web page shows in the agent browser. Snapshot first, then screenshot so you can see the page, then extract and quote — never invent CSS or dump cookies."
---

# Browser research

You already have `vox_browser_*` tools. Use them in this order unless the user asks for something narrower:

1. Open or attach (`vox_browser_open` / `vox_browser_open_ex`) on an unlocked page. Do not click a human-locked GUI tab.
2. `vox_browser_snapshot` — compact AX tree with `[ref=eN]`. Prefer `vox_browser_click_ref` / `vox_browser_fill_ref` over CSS.
3. `vox_browser_screenshot_viewport` (or screencast). You should receive an image part plus JSON `{page_id, path, width, height, mime}`. There is no `image_base64` key. If there is no image part (frame over 400_000 bytes), rely on snapshot + extract.
4. `vox_browser_extract` / `vox_browser_extract_json` for quotes you will cite.
5. Summarize with citations to snapshot refs or quoted extract text.

Do not call cookie export/import. Do not invent a `vox_browser_research` tool.
