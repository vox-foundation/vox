---
title: "Chat / harness research loop — Design Spec"
description: "Normative contract so Axis chat and MCP clients see the driven page: MCP image parts, additive LLM content_parts, Tier D frame cache, and a skill nudge — no mega-tool."
category: "Architecture SSOTs"
status: "roadmap"
training_eligible: true
training_rationale: "Contract for closing the snapshot → extract → cite → summarize loop on existing vox_browser_* tools."
---

# Chat / harness research loop — Design Spec

**Date:** 2026-09-08
**Parent program:** [`2026-09-07-agent-browser-driver-design.md`](./2026-09-07-agent-browser-driver-design.md)
**Research:** [`docs/src/architecture/agent-browser-driver-research-2026.md`](../../src/architecture/agent-browser-driver-research-2026.md)
**Plan:** [`docs/superpowers/plans/2026-09-08-chat-harness-research-loop.md`](../plans/2026-09-08-chat-harness-research-loop.md)
**GUI SSOT:** [`docs/src/architecture/vox-gui-browser-support-2026.md`](../../src/architecture/vox-gui-browser-support-2026.md)

This spec is the contract. Executors do not invent types, env vars, crate edges, tool names, or a forced pipeline that are not named here.

## 0. Goal

Axis (Loquela) chat and external MCP clients (Claude Code and anything else on `vox-orchestrator-mcp`) can **see** the driven page and run the existing loop — snapshot → extract → cite → summarize — without a new mega-tool and without Playwright.

Today the tools exist (`vox_browser_snapshot`, `vox_browser_extract`, `vox_browser_screenshot_viewport`, …) but the model never receives pixels: `call_tool` always wraps the result as `Content::text`, viewport JSON embeds `image_base64` in a string the chat loop treats as text, and `LlmChatMessage.content` is a `String`. Closing the loop is a **transport + skill** change, not a new browser engine.

Success is **C**: a live Axis demo (operator, not CI) **and** a harness/eval merge bar of unit tests that do not launch Chrome.

## 1. Non-goals

- A `vox_browser_research` (or any other) mega-tool that hides snapshot / extract / screenshot behind one RPC.
- A **forced** pipeline in dispatch. The model may still call tools in any order. Enforcement is **D**: first-party skill + tier-1 catalog / system-prompt nudge.
- Chat driving a **human-locked** GUI tab (`CallerRole::Human` lock stays).
- Merging cross-origin iframe AX trees, SPA `MutationObserver` invalidation, NeedsYou rows, or Vox `Browser.snapshot` builtins (driver-spec leftovers).
- Playwright, Stagehand, Node, or a second Chromium on the product path.
- A new workspace crate. New modules land in crates listed in §4.
- New crate-to-crate dependency edges. Do not add `url`, `regex`, `image`, or `vox-crypto` to `vox-plugin-browser` or `vox-orchestrator-mcp`.
- JPEG / resize / long-edge downscale. If a decoded PNG exceeds the image-part cap, **omit the image part** and keep the cache `path` in JSON.
- An `image_base64` key in MCP **tool JSON** after this program. The MCP **image content part** carries pixels; the JSON does not.
- A new `VOX_*` env var. `VOX_CACHE_DIR` already exists in `contracts/config/env-vars.v1.yaml`. This program adds the missing `vox_config::paths::cache_dir()` resolver (F44 / M-57 **cache only** — do not implement spool/state in this spec).
- Changing `browser_act` off `llm_bridge::call_llm`. Keep that path. Do not introduce `vox_actor_runtime::llm` at the MCP `browser_act` layer.
- Cookie **values** in MCP tool JSON (unchanged from the driver spec).
- Persisting frames in Turso / Tier A, `vox-spool`, or checksum-manifest. Frames are Tier D cache.

## 2. Already shipped (do not rebuild)

| Surface | Reality |
| --- | --- |
| CDP + snapshot/refs | `vox-plugin-browser` revision 5; `vox_browser_snapshot` / `*_ref` |
| MCP tools | `heavy-browser` `vox_browser_*` including screenshot, screencast, extract, act |
| MCP `call_tool` | [`crates/vox-orchestrator-mcp/src/server.rs`](../../../crates/vox-orchestrator-mcp/src/server.rs) — **text only**: `Content::text(result_json)` |
| Viewport PNG | `browser_screenshot_viewport` returns `ToolResult` JSON with `data.image_base64` |
| Screencast | Plugin JSON `{ image_base64, viewport_width, viewport_height }`; MCP forwards it |
| Full-page file shot | `browser_screenshot` returns `{ path }` only |
| Axis loop | [`agent_loop.rs`](../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs) pushes `LlmChatMessage { role: "tool", content: <json string> }` |
| LLM facade | `LlmChatMessage.content: String`; `vox_llm_egress::ChatMessage.content: String`; `WireMessage.content: &'a str` |
| GUI live view | `capture_frame_png_base64` **requires** `data.image_base64` from screencast then viewport |
| Skills | Tier-1 `render_skill_catalog` + `vox_skill_use`; first-party pattern `assets/skills/vox-graph/` + `[[skill-bundle]]` in `catalog.toml` |
| rmcp | `Content::image(data_base64, mime_type)` already exists (`rmcp` 1.7 `model/content.rs`) |

## 3. Locked decisions

| Topic | Choice |
| --- | --- |
| Goal | Axis + MCP clients see the page; existing tools; no mega-tool |
| Enforcement | Skill + catalog nudge. No forced dispatch pipeline |
| Success | Live Axis demo + unit-test merge bar (no live Chrome in CI) |
| Transport | Screenshot verbs return small JSON (`page_id`, `path`, `width`, `height`, `mime`) **plus** an MCP image part. **No `image_base64` key in JSON** |
| Size | Const `BROWSER_FRAME_IMAGE_PART_MAX_BYTES = 400_000` (decoded PNG bytes). Over cap → omit part, keep `path` |
| Axis | Map the image part through `vox_actor_runtime::llm` as additive `content_parts` |
| GUI | Read cache `path` (and legacy `image_base64` only if present). Tauri `vox://browser-frame` may still carry base64 to the UI — that is not MCP JSON |
| Cache | `$VOX_CACHE_DIR/browser-frames/` via new `vox_config::paths::{cache_dir, browser_frames_cache_dir}` |

## 4. File map

| File | Role |
| --- | --- |
| Create: `crates/vox-orchestrator-mcp/src/tool_images.rs` | Pure promote/strip; MCP content builder; persist frame under cache jail |
| Modify: `crates/vox-orchestrator-mcp/src/lib.rs` | `mod tool_images;` (always on, not `heavy-browser`) |
| Modify: `crates/vox-orchestrator-mcp/src/server.rs` | `call_tool` uses `mcp_contents_for_tool_json` instead of `vec![Content::text(...)]` |
| Modify: `crates/vox-orchestrator-mcp/src/browser_tools.rs` | Viewport + screencast: persist PNG, return path JSON, **no** `image_base64` in `ToolResult` |
| Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs` | Tool results: stripped JSON in `content`, optional `content_parts` |
| Modify: `crates/vox-llm-egress/src/lib.rs` | `LlmContentPart` + `ChatMessage.content_parts` |
| Modify: `crates/vox-llm-egress/src/wire.rs` | `WireMessage.content` is `serde_json::Value` (string or OpenAI array) |
| Modify: `crates/vox-actor-runtime/src/llm/types.rs` | `LlmChatMessage.content_parts` (same type, mapped in `chat.rs`) |
| Modify: `crates/vox-actor-runtime/src/llm/chat.rs` | Copy `content_parts` into egress `ChatMessage` |
| Modify: `crates/vox-config/src/paths.rs` | `cache_dir()`, `BROWSER_FRAMES_CACHE_LEAF`, `browser_frames_cache_dir()` |
| Modify: `crates/vox-gui/src/commands/browser.rs` | `capture_frame_png_base64` reads `path` (file) or leftover `image_base64` |
| Create: `assets/skills/browser-research/SKILL.md` | When/how to snapshot → screenshot → extract → cite |
| Modify: `crates/vox-plugin-catalog/catalog.toml` | `[[skill-bundle]]` id `browser-research` |
| Modify: `contracts/db/data-storage-policy.v1.yaml` | Allowlist `crates/vox-orchestrator-mcp/src/tool_images.rs` for the PNG write |
| Modify: GUI / research / where-things-live SSOTs | Record the loop (implementation Task 5) |

Do **not** grow `engine.rs` or turn `browser_tools.rs` into the promote logic. Do **not** hand-edit generated capability / tool-registry YAML. No new MCP tool names.

## 5. Image transport

### 5.1 Promote helper (pure + persist)

Module: `crates/vox-orchestrator-mcp/src/tool_images.rs`.

```rust
pub const BROWSER_FRAME_IMAGE_PART_MAX_BYTES: usize = 400_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBlob {
    pub mime: String, // always "image/png" in this program
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PromotedToolImage {
    pub json: serde_json::Value,
    pub image: Option<ImageBlob>,
}

/// Strip every `image_base64` object key (envelope `data` or nested).
/// Decode the first valid PNG-sized blob. If decoded len > MAX, `image` is None
/// but the key is still removed. Invalid base64: strip, no part, do not echo the raw key.
pub fn promote_tool_image(result_json: &str) -> PromotedToolImage;

/// `json` serialized (compact). Used as MCP text and as `LlmChatMessage.content`.
pub fn json_text(promoted: &PromotedToolImage) -> String;

/// `[Content::text(json_text), optional Content::image(standard_b64, mime)]`
pub fn mcp_contents_for_tool_json(result_json: &str) -> Vec<rmcp::model::Content>;

/// Write `bytes` under `browser_frames_cache_dir()` as `{safe_page_id}-{unix_ms}.png`.
/// `safe_page_id` keeps only `A-Za-z0-9_-`; empty → `page`.
/// Jail: `vox_config::paths::cookie_import_path_ok(cache_root, dest)` after create
/// (reuse the existing canonicalize prefix helper — it is generic).
/// Returns the dest `PathBuf`.
pub fn persist_browser_frame_png(page_id: &str, bytes: &[u8]) -> Result<PathBuf, String>;

/// If `data.path` is a file under the cache jail and `image` is still None,
/// read the file when `len <= MAX` and fill `image`. Used when handlers already
/// persisted and stripped base64 before returning JSON.
pub fn attach_image_from_cached_path(promoted: &mut PromotedToolImage);
```

`promote_tool_image` never inserts `image_base64`. Cookie tools have no such key; they are unchanged.

### 5.2 Screenshot / screencast JSON (after)

Success `data` for `vox_browser_screenshot_viewport` and `vox_browser_screencast_frame`:

```json
{
  "page_id": "<id>",
  "path": "<absolute path under cache_dir/browser-frames>",
  "width": 1280,
  "height": 800,
  "mime": "image/png"
}
```

`width` / `height` come from the existing `png_dimensions` helper (viewport) or plugin `viewport_*` (screencast). Use `0` when unknown.

`vox_browser_screenshot` (full page to a caller-supplied path) stays `{ "path": "..." }` and may also get an MCP image part if `promote` / `attach_image_from_cached_path` can jail-read that path **and** it sits under `browser_frames_cache_dir()`. Caller-supplied paths **outside** the cache jail are not read into an image part (path-only, same as today).

### 5.3 MCP `call_tool`

In `server.rs`, replace `let content = vec![Content::text(result_json)];` with `let content = crate::tool_images::mcp_contents_for_tool_json(&result_json);`. Then `attach_image_from_cached_path` inside that helper so viewport/screencast parts work after base64 is gone.

Error envelopes (`success: false`) stay text-only.

### 5.4 Cache paths

Add to `crates/vox-config/src/paths.rs` (no new env name):

- `cache_dir() -> PathBuf` — `VOX_CACHE_DIR` if non-empty, else platform cache (`LOCALAPPDATA` / `~/Library/Caches` / `XDG_CACHE_HOME` or `~/.cache`) joined with `vox`.
- `pub const BROWSER_FRAMES_CACHE_LEAF: &str = "browser-frames";`
- `browser_frames_cache_dir() -> PathBuf` — `cache_dir().join(BROWSER_FRAMES_CACHE_LEAF)`.

Frames are Tier D: deleteable without data loss. Do not put a `.vox/browser-frames` string literal in MCP or GUI.

## 6. LLM content parts (additive)

### 6.1 Types

In `vox-llm-egress` (wire owner):

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum LlmContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: LlmImageUrl },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LlmImageUrl {
    pub url: String, // `data:image/png;base64,...` only in this program
}

pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<EgressToolCall>>,
    pub tool_call_id: Option<String>,
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_parts: Option<Vec<LlmContentPart>>,
}
```

`LlmChatMessage` in `vox-actor-runtime` gets the same `content_parts: Option<Vec<vox_llm_egress::LlmContentPart>>` field (`serde` default + skip). `chat.rs` copies it when building egress messages.

When `content_parts` is `None`, `ChatMessage` serialization stays `{role, content}` (+ tool keys). Existing `plain_text_message_serializes_with_no_tool_keys` must still pass — it must also assert no `content_parts` key.

Every existing `ChatMessage { ... }` literal in `vox-llm-egress` (including `tests/wire_mock.rs`) must set `content_parts: None` (or `..` after a `Default` impl). Rustc lists the sites.

### 6.2 Wire

`WireMessage.content` becomes `serde_json::Value`:

- `content_parts` is `None` or empty → `Value::String(content.clone())` (today’s string).
- `content_parts` is `Some(parts)` → JSON array:
  1. `{ "type": "text", "text": <content> }`
  2. each `LlmContentPart::ImageUrl` as `{ "type": "image_url", "image_url": { "url": "data:image/png;base64,..." } }`
  3. ignore extra `LlmContentPart::Text` in `content_parts` (Axis must not duplicate text there)

Do not put raw `image_base64` object keys on the wire. OpenAI-compatible `image_url` is the only image shape.

Ghost text, judges, harness-issue judge, and skill-promotion stay string content (`content_parts: None`).

### 6.3 Axis agent loop

After `handle_tool_call_with_mode` returns a JSON string:

1. `let mut promoted = promote_tool_image(&content);`
2. `attach_image_from_cached_path(&mut promoted);`
3. `content` on the tool message = `json_text(&promoted)`.
4. `content_parts` = `Some(vec![ImageUrl { url: data_url }])` when `promoted.image` is `Some`; else `None`.

Harness scoring / redaction continue to see the **stripped** JSON string only (no base64 blob in the judge buffer).

Transcript persistence (`ChatTranscriptEntry.content`) stores the stripped JSON string, not image bytes. Reloaded history does not replay pixels; the model can call screenshot again. That is accepted.

## 7. GUI path

`capture_frame_png_base64` today fails if `image_base64` is missing. After this program:

1. Screencast then viewport, same order.
2. `mcp_data` as today.
3. If `data.path` is a string, read the file **only when** `cookie_import_path_ok(browser_frames_cache_dir(), path)` (or the dest canonicalizes under the cache root). Then STANDARD-encode bytes for `BrowserFramePayload.image_base64` (UI event, not MCP).
4. Else if `data.image_base64` is still present (old daemon), use it.
5. Else error `screenshot_viewport returned no path or image_base64`.

Do not send MCP image parts to the WebView. The Tauri event stays a base64 PNG.

## 8. Skill + nudge

Create `assets/skills/browser-research/SKILL.md`:

- `name: browser-research` (directory name).
- `description` (1–1024 chars) must mention: research a page, summarize a site, cite what the page shows, screenshot + snapshot. That is what tier-1 injection uses.
- Body: use existing tools only — `vox_browser_open` / `open_ex` → `vox_browser_snapshot` → `vox_browser_screenshot_viewport` (or screencast) → `vox_browser_extract` / `extract_json` → cite snapshot refs / quoted extract → summarize. If the screenshot tool result has no image part (over cap), rely on snapshot + extract; do not invent CSS.
- Do **not** tell the model to click a human-locked tab.
- Cookie export remains human + lock; the skill must not ask the model to dump cookies.

Register `[[skill-bundle]]` in `crates/vox-plugin-catalog/catalog.toml` (copy the `vox-graph` first-party shape: `license = "Apache-2.0"`, `source = "https://github.com/vox-foundation/vox"`, `bundle-path = "assets/skills/browser-research"`, non-empty `pin`).

No `SOURCES.toml` row — that file is vendored third-party provenance. First-party skills are catalog-only (same as `vox-graph`).

Tier-1 injection already lists every installed skill. Do **not** add a second hardcoded “always use browser-research” paragraph to `build_system_prompt` beyond the skill description. Pinning stays the existing `pinned_skill` path.

## 9. Error handling

| Case | Behavior |
| --- | --- |
| Invalid `image_base64` | Strip key; no image part; JSON otherwise intact |
| PNG larger than 400_000 bytes | Strip / never add `image_base64`; no image part; `path` present |
| Cache write fails | Tool error envelope (`success: false`); no partial base64 fallback in JSON |
| Path outside cache jail | Do not read into an image part; GUI does not load it |
| `success: false` | Text-only MCP content |
| Model ignores the skill | Allowed. No dispatch interceptor |

## 10. Testing (merge bar)

No live Chrome. Required tests (exact names in the plan):

- `promote_tool_image` strips `image_base64`, decodes a 1×1 PNG, omits part when over cap, leaves cookie-shaped JSON untouched.
- `mcp_contents_for_tool_json` yields text + image (`RawContent` / serialized `type: image`) for a small PNG, text-only when over cap.
- Wire: `content_parts: None` still a JSON **string**; `Some(image)` is an **array** with `image_url`.
- `plain_text_message_serializes_with_no_tool_keys` still length 2 (role + content).
- Agent-loop mock: second request `role: tool` has array `content` when the dispatched tool JSON had a cache PNG (or injected `image_base64` in the mock tool result). Prefer a synthetic tool result in a unit that calls `promote` + message build if wiring a real tool is heavier — the plan’s Task 4 names the test.
- GUI: `frame_bytes_from_mcp_data` (extract the path/base64 branch into a pure fn) reads path bytes / rejects a path outside the jail.
- `cache_dir` honors `VOX_CACHE_DIR` in a single-threaded test (`unsafe` set_var like existing path tests).
- `cargo test -p vox-plugin-catalog skill_bundle_parity` after the catalog row.
- `vox ci agentskills-compliance` on the new skill (or the repo’s existing gate).

Live Axis demo is **manual**: open a page, ask chat to summarize what is on screen, confirm the second provider request includes an `image_url` part (proxy log or debug). Not a CI job.

## 11. Global constraints

- Test-first for every new `pub fn`.
- Never `cargo fmt --all`. Use `vox run scripts/fmt.vox` or `cargo fmt -p <crate>`.
- No new workspace crate and no `exceptions` ledger edits.
- No Playwright / Stagehand / Node on the product path.
- Secrets only via `vox_secrets::resolve_secret`. No new `VOX_*`.
- `browser_act` stays on `llm_bridge::call_llm`.
- Cookie values never appear in MCP tool JSON.
- No `--no-verify` commits.
- Do not add `url`, `regex`, `image`, or `vox-crypto` to `vox-plugin-browser` or `vox-orchestrator-mcp`.
- Do not grow `engine.rs` past the god-object cap.
- Do not hand-edit generated capability / tool-registry YAML.
- Do not bump `VOX_PLUGIN_ABI_VERSION`.
- `vox-orchestrator-mcp` browser **handlers** still need `--features heavy-browser` to compile those tests; `tool_images` tests do not.
- `vox-gui` has **no** `--lib` target. GUI tests are `cargo test -p vox-gui <name>` (binary crate tests). Stub `crates/vox-gui/ui/dist/` if `tauri-build` requires it (gitignored; do not commit).
- Route `cargo` through PATH (build broker). Never `cargo fmt --all`.
)
