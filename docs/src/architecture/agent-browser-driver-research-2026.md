---
title: "Agent browser driver research (2026)"
description: "Audit of Vox's shipped CDP/GUI browser stack against Stagehand, Claude Chrome, ChatGPT Atlas, Playwright MCP, and Rust driver libraries, with a no-bloat recommendation."
category: "Architecture SSOTs"
status: "research"
training_eligible: true
training_rationale: "Competitive and library research for semantic agent browser driving on the existing chromiumoxide + Tauri stack."
---

# Agent browser driver research (2026)

**Question:** How does Vox add Stagehand-class semantic browser driving (chat-driven, viewable, cookie-aware) without bloating a Rust + Tauri product?

**Answer:** Do not add Playwright, Stagehand, or a second Chromium. Close three gaps on the stack that already ships: accessibility-ref snapshots, named persistent profiles with explicit cookie consent, and an optional attach-to-user-Chrome path. Tauri's WebView is the operator shell, not the driven browser.

Research date: 2026-09-07. Sources are official docs and product pages unless marked analysis.

## 1. What Vox already has

Vox is not starting from a blank browser. The June 2026 scoping doc and the GUI browser SSOT already chose **chromiumoxide CDP** as the agent engine and **Playwright as CI/validation only**. That decision is still correct.

| Layer | Where | Shipped today |
| --- | --- | --- |
| CDP engine | `crates/vox-plugin-browser` | chromiumoxide 0.9.1: open/goto/click/fill/scroll/keys, screenshots, screencast, `visible_text_summary`, `GetFullAxTree` |
| Plugin ABI | `vox-plugin-api` `BrowserAutomation` | Sabi trait, revision 4; loaded via `vox-plugin-host` |
| MCP | `vox-orchestrator-mcp` `browser_tools.rs` | 26 `vox_browser_*` tools + `browser_act` / `browser_extract` / `browser_extract_json` |
| Vox builtins | `vox-actor-runtime` `Browser.*` | 9 verbs; no snapshot/ref builtins |
| GUI | `vox-gui` Browser surface | Preview iframe (localhost) + agent CDP live view (`vox://browser-frame`) + human/agent control lock |
| Chat wiring | Loquela → `chat_turn` → `vox-orchestrator-d` | Chat can already invoke `vox_browser_*` |
| Static scrape | `Scrape.*`, `vox-search` | reqwest + `scraper`; no JS |
| Visus | CLI + MCP | Screenshot + raw AX tree for visual audit |
| Playwright | `vox-gui/ui/e2e`, integration tests | Validation / golden-route only |

**Confirmed absences (code, not docs):**

- No `user_data_dir`, cookie jar, or `storage_state` in `vox-plugin-browser`. Sessions die when the last tab closes.
- No `vox_browser_snapshot` / `vox_browser_click_ref`. AX tree exists on the plugin and visus path, not as an agent-loop tool.
- `browser_act` is a **text-summary → LLM JSON → CSS/XPath** loop, not a semantic-ref loop. That is closer to a cheap Stagehand `act()` than to Playwright MCP.
- No Chrome extension / native-messaging attach to the user's real Chrome.
- No OS-level computer-use. Warp research mapped it; nothing shipped.

**Tauri is not a second browser.** The GUI host is wry / WebView2 / WebKitGTK with a tight CSP. The Preview tab iframes localhost. The Agent tab mirrors CDP PNG frames and maps clicks to `vox_browser_click_xy`. That is already the Claude Desktop "in-app pane" pattern, implemented without embedding a second Chromium into the WebView.

## 2. What competitors actually ship (2026)

The market split in 2026 is **identity**, not horsepower. Everyone can click. The product question is: whose cookies, whose Chrome, how many tokens per step?

### 2.1 Claude Code Desktop — three browsers, one identity axis

Official docs (`code.claude.com/docs/en/desktop`, `code.claude.com/docs/en/chrome`, Claude Help Center, 2026-08):

| Surface | Session | Who drives | Available from |
| --- | --- | --- | --- |
| **Claude in Chrome** extension | User's Chrome, user's logins | MCP tools `mcp__claude-in-chrome__*` | CLI (`--chrome`), VS Code, Desktop connectors |
| **Desktop Browser pane** | Clean Chromium profile inside Electron | App-native tools | Desktop only (`Cmd+Shift+B`) |
| **chrome-devtools-mcp** | Clean Chrome launched by the MCP | Puppeteer + DevTools | Any MCP client |
| **Computer use** | The real desktop | Screen + accessibility | Desktop, Pro/Max, off by default |

The pane is **not** the user's Chrome. Persist-sessions is an opt-in so localhost cookies survive restarts. Site allowlists are shared with the extension. Claude pauses on login/CAPTCHA and will not purchase, create accounts, or bypass CAPTCHAs without the user.

Computer use is last resort: connectors, then Bash, then Chrome, then iOS Simulator, then screen control. Browsers are capped at view-only under computer-use so Claude is steered back to the dedicated browser tool.

**Lesson for Vox:** ship two identities, not one mega-browser. Clean profile for "build and test." Attach-to-user-Chrome for "act as me." Do not start with OS computer-use.

### 2.2 ChatGPT Atlas — standalone browser died

OpenAI launched Atlas (Chromium + ChatGPT, macOS-first) on 2025-10-21. Official help (2026-07): **deprecated**, scheduled to stop working **2026-08-09**. Features moved to:

1. ChatGPT desktop built-in browser (tabs, downloads, logins).
2. ChatGPT Chrome extension / sidebar.
3. A **cloud** browser for unattended agents.

TechCrunch (2026-07-09): OpenAI treated the browser as a feature of ChatGPT, not a destination. HUMAN Security (May 2026) had Comet at ~47% of measured agentic traffic vs Atlas ~20%. Atlas never shipped Windows.

**Lesson for Vox:** do not fork Chromium or ship a competing default browser. Embed a pane + optional real-Chrome attach. Atlas is the expensive counter-example.

### 2.3 Stagehand (Browserbase) — the "Songshand" reference

Official: [docs.stagehand.dev](https://docs.stagehand.dev/v4/first-steps/introduction), [github.com/browserbase/stagehand](https://github.com/browserbase/stagehand) (~23k stars, MIT). Latest browse package release 0.8.3 (2026-06-05).

Stagehand is an **SDK for browser agents**, not a desktop product:

- AI primitives: `act()`, `observe()`, `extract()` (schema-typed), plus `agent()` for multi-step.
- Deterministic `page` APIs (Playwright-shaped `goto` / `locator` / `screenshot`).
- v3+ drives Chromium over **CDP**. Docs now say there is **no Playwright or Puppeteer dependency** for the core engine; Playwright can still attach via `connectOverCDP`.
- Self-healing + observe-then-act caching. Server-side cache is **Browserbase-only**.
- SDKs: TypeScript, Python, Go. **No Rust SDK.**
- Models are pluggable; BYO client callback exists.

What Stagehand is *not*: a GUI, a cookie-consent product, or a Tauri-shaped embed. Hosted gravity pulls toward Browserbase hours and their Model Gateway.

Vox already has a weaker `browser_act` / `browser_extract` / `browser_extract_json`. The missing Stagehand pieces are **observe (candidate actions + cache)** and **typed extract against a snapshot**, not a new engine.

### 2.4 Playwright MCP — the semantic-loop standard

Official: [playwright.dev/mcp](https://playwright.dev/mcp/introduction), [github.com/microsoft/playwright-mcp](https://github.com/microsoft/playwright-mcp).

The loop every serious agent now copies:

1. `browser_snapshot` → compact AX tree with refs (`e5`).
2. Model picks a ref.
3. `browser_click` / `type` / `fill_form` on that ref.
4. Snapshot again.

Claims: ~200–400 tokens per snapshot vs thousands for DOM/screenshots; no vision model required; cookies + `storage_state` persist by default. 40+ tools including cookie/localStorage, network mock, tracing.

Microsoft itself now says coding agents should prefer **Playwright CLI + skills** for token cost, and keep MCP for long-running stateful loops. That matches Vox: MCP tools already live on the orchestrator; the missing tool is snapshot/ref, not a Node sidecar.

### 2.5 Vercel agent-browser — closest Rust cousin

Official: [github.com/vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser), [agent-browser.dev](https://agent-browser.dev/). Native Rust CLI + CDP daemon. Snapshot + `@eN` refs. Profiles, cookies, storage, proxy. No Playwright/Puppeteer in the native path.

This is the **pattern** to copy (compact AX text + ref map + `click @e2`), not a dependency to vendor. Vox already owns a chromiumoxide host, MCP dispatch, and a GUI frame mirror. Adding another daemon would duplicate `vox-plugin-browser`.

Caveat from Tavily/GitHub: persistent-profile cookie bugs exist (`--profile` / headless Windows). If Vox adds profiles, test headed + headless cookie round-trips; do not assume `user_data_dir` is enough.

### 2.6 chrome-devtools-mcp — debugger, not driver

[github.com/ChromeDevTools/chrome-devtools-mcp](https://github.com/ChromeDevTools/chrome-devtools-mcp) (~50k stars). Puppeteer + DevTools: traces, Lighthouse, heap snapshots, `take_snapshot` with uids, `--autoConnect` to a running Chrome 144+. Officially Chrome / Chrome for Testing only.

Use later as an **optional debug skill** for "why is LCP bad," not as the product driver. Node + Puppeteer is the bloat we already refused.

### 2.7 browser-use — heavy agent, not a library fit

[github.com/browser-use/browser-use](https://github.com/browser-use/browser-use). Python agent loop (vision + numbered elements), 2026 beta Rust core, cloud browsers, profile sync script. Fine as a hosted competitor; a Python/cloud agent inside Vox violates VoxScript-first, LLM-facade, and lean-plugin rules.

### 2.8 Consumer AI browsers (not embeddable)

| Product | Form | Agent model | Embed in Vox? |
| --- | --- | --- | --- |
| Perplexity Comet | Standalone Chromium | Assistant + rate-limited multi-step; free (2026-03) | No public driver API |
| Gemini in Chrome | Side panel + auto browse | Acts in the user's Chrome | Chrome-only, not a library |
| Edge Copilot | In-browser | Enterprise-gated; EEA exclusions reported | No |
| Dia (The Browser Company) | Standalone | Consumer | No |

These validate demand. None are crates we can take.

## 3. Library landscape (Rust / Tauri)

Reconfirming the 2026-06-03 scoping table against 2026-09 sources:

| Library | Protocol | Extra artifact | Fit |
| --- | --- | --- | --- |
| **chromiumoxide 0.9.1** | CDP / tokio | System or fetched Chromium | **Already in-tree. Keep.** |
| **ferrous-browser** | CDP / tokio | Chromium | Playwright-ish locators; young; switching engines is churn, not capability |
| **dravr-browser** | wraps chromiumoxide | Chromium | Persistent profile + stealth + cookie snapshot — **steal ideas, not the crate** |
| **stealth-oxide** | CDP patches on chromiumoxide | — | Optional fingerprint coherence; only if bot-walls become a product requirement |
| **headless_chrome** | sync CDP | Chromium | Fights Tokio. Skip. |
| **thirtyfour / fantoccini** | WebDriver | chromedriver | Extra process. Skip. |
| **playwright-rust** | Node Playwright driver | Node + browsers | The deploy bloat we deferred. Still defer. |
| **reqwest + scraper** | HTTP | none | Keep as Tier 1 (no JS) |
| wry / WebView2 cookie APIs | OS WebView | — | GUI host only. Do not drive arbitrary sites here. |

**Load-bearing fact:** language does not beat anti-bot. CDP looks like CDP. Stealth, proxies, and a real user profile matter more than Rust vs Node. Do not promise Cloudflare-bypass as a Vox feature.

## 4. Capability matrix (Vox vs the bar)

| Capability | Stagehand | Claude Desktop pane | Claude in Chrome | Playwright MCP | agent-browser | **Vox today** |
| --- | --- | --- | --- | --- | --- | --- |
| Native CDP, no Node | Yes (v3+) | Electron Chromium | Extension + native messaging | No (Node) | Yes (Rust) | **Yes** |
| Chat → drive page | `act` / `agent` | App tools | MCP | MCP | CLI/MCP | **Yes** (`vox_browser_*`, `browser_act`) |
| In-app live view | Session inspector (hosted) | Pane | Real Chrome window | Headed Playwright | Optional | **Yes** (frame mirror) |
| Human take-over lock | — | Yes | Pause on CAPTCHA | — | — | **Yes** (`set_control_lock`) |
| AX snapshot + refs | observe actions | — | — | **Yes** | **Yes** | Partial (raw AX in plugin/visus) |
| Typed extract | **Yes** | — | — | via snapshot | via snapshot | Partial (`extract_json`) |
| Observe + cache | **Yes** (BB cache) | — | — | — | — | No |
| Persistent cookies | Session / BB profile | Opt-in persist | **User Chrome** | Default persist | Profiles (bugs) | **No** |
| Attach to user Chrome | No | No | **Yes** | Extension mode | `--cdp` | **No** |
| Cookie consent UX | — | Clean vs persist | Site allowlist | — | — | **No** |
| Site allow/block | — | Shared with ext | **Yes** | — | Policies | No |
| Console / network | via Playwright attach | Limited | Yes | Yes | Yes | Weak |
| Lighthouse / traces | No | No | No | Limited | Some | visus only |
| OS computer-use | No | Separate preview | No | No | Electron via CDP | No (correct) |

## 5. Approaches (do not implement all three)

### A — Finish the in-tree driver (recommended)

Keep `vox-plugin-browser` + MCP + Browser surface. Add only:

1. **Semantic loop.** Compact AX snapshot with stable refs; `click_ref` / `fill_ref`; optional numbered-bbox overlay on the existing screenshot path (already deferred in `vox-gui-browser-support-2026.md`).
2. **Named profiles + consent.** Launch Chromium with `user_data_dir` under a `vox_config::paths` profile root (Tier D / deleteable). First-class UI: *ephemeral* (default) vs *save this session* vs *use profile X*. Cookie + localStorage export/import via CDP (`Network.getAllCookies` / `Storage.getCookies`) as a portable sidecar, not Turso rows.
3. **Optional attach.** CDP connect to a user-started Chrome (`remote-debugging-port` or Chrome 144+ remote debugging), later a thin native-messaging extension if product wants Claude-in-Chrome identity. Same MCP tools; different launch mode.
4. **Upgrade `browser_act`.** Observe-then-act against the snapshot (Stagehand pattern) instead of CSS-from-visible-text. Keep the LLM on `vox_actor_runtime::llm`.
5. **Safety.** Reuse control lock; add site allowlist; pause on password fields / payments / CAPTCHA (Claude's rule). Prompt-injection wrapping on snapshot text (agent-browser nonce markers).

**Bloat:** ~plugin + MCP + GUI, no new crate, no Node in the product path. Playwright stays CI.

### B — Sidecar Playwright MCP or chrome-devtools-mcp

`npx @playwright/mcp` or `npx chrome-devtools-mcp` next to the daemon.

**Gain:** snapshot/ref and DevTools overnight.
**Cost:** Node + browser download in the desktop app; two engines; MCP schema duplication; Windows/macOS install friction; violates "Playwright is validation only."
**Use only** as a contributor debug skill, not the shipped driver.

### C — Vendor Stagehand or agent-browser

Stagehand: no Rust, Browserbase gravity, extra LLM hop.
agent-browser: excellent CLI, but a second CDP daemon beside `vox-plugin-browser`.

**Use as a reference implementation** (read their snapshot serializer). Do not add the binary as a runtime dependency.

## 6. Cookie / identity design (the actual product work)

Claude's docs settle this: **the axis is identity, not power.**

| Mode | Profile | When | Default |
| --- | --- | --- | --- |
| **Workspace** | Ephemeral temp dir | Preview, scrape, untrusted sites | Yes |
| **Named** | `user_data_dir` the user opted to keep | Staging logins, recurring test accounts | Explicit save |
| **Attached** | User's real Chrome | Gmail / Docs / "as me" | Off; extra consent |

Do not silently persist cookies. Treat exported cookie files as secrets-adjacent (OpenAI's Atlas shutdown note: do not share session files). Resolve profile paths through `vox_config::paths`, not hardcoded `.vox/` strings. Do not put cookie blobs in Tier A Turso.

Tauri/WebView2 has cookie manager bindings in the Windows patch tree. Those cookies belong to the **operator shell**, not the agent browser. Mixing them would leak GUI session into driven sites. Keep them separate.

## 7. Recommendation

Ship **Approach A**. The 2026-06-03 scoping doc said the majority of benefit is assembly, not a new engine. Nine months later the competitive bar moved to **snapshot+ref + dual identity**. Vox already has the engine, the chat hook, the live view, and a stub `act`/`extract`. The bloat risk is adding Playwright, Stagehand, Electron, or a Chromium fork — exactly what Atlas proved not to do.

**Implementation (approved 2026-09-07):** spec [`docs/superpowers/specs/2026-09-07-agent-browser-driver-design.md`](../../superpowers/specs/2026-09-07-agent-browser-driver-design.md), plan [`docs/superpowers/plans/2026-09-07-agent-browser-driver.md`](../../superpowers/plans/2026-09-07-agent-browser-driver.md).

**Do next (when executing the plan), in order:**

1. `vox_browser_snapshot` + `vox_browser_click_ref` / `fill_ref` on the existing `GetFullAxTree` (interactive-only default, depth limit, ~200–400 token target).
2. Profile launch flags + GUI consent: ephemeral / save / pick named profile.
3. Point-and-approve overlay on the existing agent frame (human lock already exists).
4. Attach-to-running-Chrome as a third launch mode.
5. Leave computer-use and chrome-devtools-mcp as later optional skills.

**Do not:** add `playwright-rust`, embed Stagehand, drive sites inside the Tauri WebView, or start a Vox-branded Chromium.

## 8. Sources

### Official / primary

- [Claude Code Desktop](https://code.claude.com/docs/en/desktop)
- [Claude Code + Chrome](https://code.claude.com/docs/en/chrome)
- [Get started with Claude in Chrome](https://support.claude.com/en/articles/12012173-get-started-with-claude-in-chrome) (updated 2026-08-26)
- [Evolving Atlas into ChatGPT for browser-based agentic work](https://help.openai.com/en/articles/20001371-evolving-atlas-into-chatgpt-for-browser-based-agentic-work)
- [Introducing ChatGPT Atlas](https://openai.com/index/introducing-chatgpt-atlas/) (deprecated)
- [Stagehand introduction](https://docs.stagehand.dev/v4/first-steps/introduction)
- [Stagehand GitHub](https://github.com/browserbase/stagehand)
- [Playwright MCP](https://playwright.dev/mcp/introduction)
- [chrome-devtools-mcp](https://github.com/ChromeDevTools/chrome-devtools-mcp)
- [Vercel agent-browser](https://github.com/vercel-labs/agent-browser)
- [browser-use](https://github.com/browser-use/browser-use)

### Reporting (context, not SSOT)

- [TechCrunch: OpenAI is shutting down Atlas](https://techcrunch.com/2026/07/09/openai-is-shutting-down-atlas-but-its-ai-browser-ambitions-are-still-growing/) (2026-07-09)
- [wmedia: Claude Code's three browsers](https://wmedia.es/en/tips/claude-code-desktop-browser) (2026-08-02)
- [ChatForest: MCP browser landscape](https://chatforest.com/guides/mcp-browser-automation/) (2026-03-28)
- [Steve Kinney: Playwright MCP vs chrome-devtools-mcp](https://stevekinney.com/writing/driving-vs-debugging-the-browser) (2026-04-06)

### In-repo

- [vox-gui-browser-support-2026.md](vox-gui-browser-support-2026.md)
- [vox-native-scraping-scoping-2026-06-03.md](vox-native-scraping-scoping-2026-06-03.md)
