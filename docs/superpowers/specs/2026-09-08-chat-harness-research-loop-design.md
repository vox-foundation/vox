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
**Follow-on (do not implement in this plan):** [`2026-09-08-browser-hitl-design.md`](./2026-09-08-browser-hitl-design.md) — Axis vs human-locked tab. This plan stays pixels + skill.

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
| Modify: `crates/vox-config/src/paths.rs` | Add `path_is_under` (cookie helper becomes a thin alias). `persist` / GUI jail call this name |
| Modify: `contracts/operations/catalog.v1.yaml` | Hand-edit screencast + viewport **descriptions** (they still say `image_base64` / “return base64”). Then `vox ci operations-sync --target all --write` — do **not** hand-edit generated capability / tool-registry YAML |
| Modify: GUI / research / where-things-live SSOTs | Record the loop (implementation Task 5) |

Do **not** grow `engine.rs`. Do **not** add persist/screencast helpers to `browser_tools.rs` (already ~1498 lines). Those bodies live in `tool_images.rs`. Do **not** hand-edit generated capability / tool-registry YAML. No new MCP tool names.

`vox-gui/ui` `BrowserView` reads `frame.image_base64` from the **Tauri** `vox://browser-frame` event only (`transport.ts`). It does not parse MCP tool JSON. No UI change required for the key removal.

## 5. Image transport

### 5.1 Promote helper (pure + persist)

Module: `crates/vox-orchestrator-mcp/src/tool_images.rs`.

```rust
pub const BROWSER_FRAME_IMAGE_PART_MAX_BYTES: usize = 400_000;

/// Only these MCP tool names may produce an image part (stdio `call_tool` + Axis).
pub const FRAME_IMAGE_TOOL_NAMES: &[&str] = &[
    "vox_browser_screenshot_viewport",
    "vox_browser_screencast_frame",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBlob {
    pub mime: String, // "image/png" or "image/jpeg" from magic bytes — never guessed
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
///
/// `vox_visual_rag_query` uses `image_base64` on **params** only (result is an error
/// envelope today). Recursive strip on **results** does not touch request params.
pub fn promote_tool_image(result_json: &str) -> PromotedToolImage;

/// `json` serialized (compact). Used as MCP text and as `LlmChatMessage.content`.
pub fn json_text(promoted: &PromotedToolImage) -> String;

/// When `tool_name` is in `FRAME_IMAGE_TOOL_NAMES`: text + optional image.
/// Otherwise: `vec![Content::text(result_json)]` with **no** path-read and **no**
/// strip (cookie `{count,path}` must stay text-only; a `data.path` must never
/// become pixels unless the tool is allowlisted).
/// Tests pass a temp `cache_root`. Production passes `browser_frames_cache_dir()`.
/// Do not hide the jail behind `Option` or a process-global `VOX_CACHE_DIR` flip.
/// If `success` is not `true`, return text only (no path-read) after promote/strip.
pub fn mcp_contents_for_tool_json(
    tool_name: &str,
    result_json: &str,
    cache_root: &Path,
) -> Vec<rmcp::model::Content>;

pub enum FramePersistMode {
    /// GUI poll / screencast: `{safe_page_id}-live.png` (replace). One file per page.
    LiveReplace,
    /// Explicit viewport screenshot: `{safe_page_id}-{unix_ms}.png`, then
    /// best-effort delete siblings in the cache dir older than 1 hour.
    Snapshot,
}

/// Write PNG under `cache_root` (tests pass a temp dir; production passes
/// `browser_frames_cache_dir()`). Jail with `path_is_under` **after** write
/// (`canonicalize` requires the file to exist).
pub fn persist_browser_frame_png(
    cache_root: &Path,
    page_id: &str,
    bytes: &[u8],
    mode: FramePersistMode,
) -> Result<PathBuf, String>;

/// If `data.path` is an **absolute** file under `cache_root`, `image` is still
/// None, `len <= MAX`, and bytes sniff as PNG or JPEG, attach. Relative paths
/// and non-image files (e.g. `cookies.json` under a mis-nested jail) stay
/// text-only. Callers must only invoke this for allowlisted successful tools.
pub fn attach_image_from_cached_path(cache_root: &Path, promoted: &mut PromotedToolImage);
```

`promote_tool_image` never inserts `image_base64`. It strips that **key only** — it does not touch `screenshot_base64` (`vox_visus_audit`). After STANDARD-decode, sniff magic: PNG `89 50 4E 47…` or JPEG `FF D8 FF`. No magic → strip, no part. Do **not** add the `image` crate or transcode.

Cookie export public JSON is `{count, path}` under the **profile** dir. Defense in depth: allowlist + absolute path + magic sniff. A `cookies.json` under a mis-nested jail must not become an image part (`attach_skips_non_image_path_under_jail`).

`browser_screenshot` (caller-supplied path) is **not** in `FRAME_IMAGE_TOOL_NAMES`. Path-only JSON stays path-only.

stdio `call_tool` runs `mcp_contents_for_tool_json` **after** `call_tool_via_daemon`. Persist happens in the daemon; attach re-reads the same absolute path on the same machine. Do not support a remote GUI reading a daemon-local path in this program.

rmcp `Content::image(data, mime)` is **raw standard-base64**, not a `data:` URL (`RawImageContent`, `rename_all = "camelCase"` → serialized `mimeType`). OpenAI wire uses `data:image/png;base64,…` in `image_url.url`. Do not mix the two.

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

`mime` is **honest**: viewport bytes from `screenshot_viewport_bytes` are PNG → `image/png` and `{safe}-{ms}.png` / `{safe}-live.png`. CDP screencast is **JPEG** (`StartScreencastFormat::Jpeg` in `vox-plugin-browser` `resolve.rs`) → `image/jpeg` and `{safe}-live.jpg`. Do not label JPEG as `image/png`. Do not transcode.

`width` / `height` come from `png_dimensions` (viewport) or plugin `viewport_*` (screencast). Use `0` when unknown.

`vox_browser_screenshot` (full page to a caller-supplied path) stays `{ "path": "..." }` and **never** gets an MCP image part — it is not in `FRAME_IMAGE_TOOL_NAMES`. Caller-supplied paths are not read into pixels.

### 5.3 MCP `call_tool`

In `server.rs`, replace `let content = vec![Content::text(result_json)];` with `let content = crate::tool_images::mcp_contents_for_tool_json(&name_str, &result_json, &vox_config::paths::browser_frames_cache_dir());`. The helper allowlists the tool name, then promote + attach from the passed jail.

Error envelopes (`success: false`) stay text-only.

### 5.4 Cache paths

Add to `crates/vox-config/src/paths.rs` (no new env name):

- `cache_dir() -> PathBuf` — `VOX_CACHE_DIR` if non-empty (used as-is, same as `data_dir()` override), else platform cache (`LOCALAPPDATA` / `~/Library/Caches` / `XDG_CACHE_HOME` or `~/.cache`) joined with `APP_DIR_NAME` (`vox`). This is F44 **cache only** — do not implement spool/state. When landing the resolver, set `contracts/config/env-vars.v1.yaml` `VOX_CACHE_DIR.owner_crate` to `vox-config` (today it is still `vox-cli`).
- `pub const BROWSER_FRAMES_CACHE_LEAF: &str = "browser-frames";`
- `browser_frames_cache_dir() -> PathBuf` — `cache_dir().join(BROWSER_FRAMES_CACHE_LEAF)`.
- `pub fn path_is_under(root: &Path, candidate: &Path) -> bool` — move the body of `cookie_import_path_ok` here; `cookie_import_path_ok` becomes `{ path_is_under(root, candidate) }`. Both require canonicalize (file must exist).

Frames are Tier D: deleteable without data loss. Do not put a `.vox/browser-frames` string literal in MCP or GUI.

**Retention (required).** GUI live view polls ~every 3s (`FRAME_INTERVAL_MS`). Writing a new timestamped file each tick would grow unbounded. Live/screencast persist uses `FramePersistMode::LiveReplace`. Explicit `screenshot_viewport` uses `Snapshot` plus best-effort unlink of `browser-frames` files whose mtime is older than 3600s. No Turso rows. No new `VOX_*`.

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
    pub url: String, // `data:{mime};base64,...` with sniffed mime (png or jpeg)
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

`LlmChatMessage` in `vox-actor-runtime` gets the same field (`serde` default + skip). **Both** `chat.rs` and `stream.rs` must copy `content_parts` when mapping to `vox_llm_egress::ChatMessage`. Omitting either map **silently drops** images (the type compiles).

`vox-orchestrator-mcp` does **not** depend on `vox-llm-egress` and must not gain that edge. Re-export `LlmContentPart` and `LlmImageUrl` from `vox_actor_runtime::llm`. MCP `tool_images` and its tests use `vox_actor_runtime::llm::LlmContentPart` only.

When `content_parts` is `None`, `ChatMessage` serialization stays `{role, content}` (+ tool keys). Existing `plain_text_message_serializes_with_no_tool_keys` must still pass — it must also assert no `content_parts` key.

`ChatMessage` has no `Default`. Exact literals that must name `content_parts: None`: **6** in `vox-llm-egress` (`lib.rs` tests + `wire_mock.rs`), **2** in `vox-gamify` `transport.rs`, **2** in `vox-code-audit` `review/client.rs`. `skill_promotion.rs` has the only `LlmChatMessage` full literal without `..Default::default()` — add `content_parts: None` or switch to Default.

### 6.2 Wire

`WireMessage.content` becomes `serde_json::Value`:

- `content_parts` is `None` or empty → `Value::String(content.clone())` (today’s string).
- On roles that accept multimodal content, `content_parts` is `Some(parts)` → JSON array:
  1. `{ "type": "text", "text": <content> }`
  2. each `LlmContentPart::ImageUrl` as `{ "type": "image_url", "image_url": { "url": "data:image/png;base64,..." } }`
  3. ignore extra `LlmContentPart::Text` in `content_parts` (Axis must not duplicate text there)
- OpenAI-compatible `role: tool` messages accept text content only. A tool message
  carrying an image is serialized as the original text-only tool response followed by
  a synthetic `role: user` multimodal message containing the image parts.

Do not put raw `image_base64` object keys on the wire. OpenAI-compatible `image_url` is the only image shape.

`stream_once` already calls the same `build_request` as `chat_once`. Image parts work on the Axis streaming path with no second serializer. `vox-llm-egress` has **six** `ChatMessage {` literals (`lib.rs` tests + `tests/wire_mock.rs`) that must gain `content_parts: None`.

Ghost text, judges, harness-issue judge, and skill-promotion stay string content (`content_parts: None`).

### 6.3 Axis agent loop

`run_agent_turn` calls `handle_tool_call_with_mode` **in-process** (daemon), not stdio `call_tool`. MCP image parts do not reach Axis. Use `llm_tool_message(tool_call_id, tool_name, result_json, cache_root)`:

1. If `tool_name` is not in `FRAME_IMAGE_TOOL_NAMES`: `LlmChatMessage` with raw `content` and `content_parts: None` (do not read `data.path`).
2. Else promote + `attach_image_from_cached_path(cache_root, …)`.
3. `content` = `json_text`; `content_parts` = one `ImageUrl` or `None`.

Production passes `browser_frames_cache_dir()`. Tests pass a temp dir. Do not `set_var(VOX_CACHE_DIR)` then `remove_var` before attach.

**Latest image only.** After pushing a tool message, call `retain_latest_tool_image(&mut messages)`: set `content_parts = None` on every prior `role: tool` message. Path JSON stays in `content`. Ten screenshots in one turn must not send ten PNGs. Test: two image tool results → only the last has parts.

Do **not** add an `approx_tokens` image surcharge in `conversation.rs`. `bound_messages_by_tokens` runs only on `load_conversation`, which remaps transcript rows with `LlmChatMessage { ..Default::default() }` — no `content_parts` ever reach it. A surcharge test there is a false green. In-loop pixel cost is controlled by `retain_latest_tool_image` only. If a later program persists parts into the transcript, that program adds `approx_message_tokens` and uses it at the bound.

Harness scoring records **empty** `redacted_content` on success (`result: (ok)`). It does not see stripped JSON or pixels. Leave that path unchanged.

Transcript persistence stores the stripped JSON string, not image bytes. Reloaded history does not replay pixels.

## 7. GUI path

`capture_frame_png_base64` today fails if `image_base64` is missing. After this program:

1. Screencast then viewport, same order.
2. `mcp_data` as today.
3. `frame_bytes_from_mcp_data(&data, &vox_config::paths::browser_frames_cache_dir())` — never `None` for the jail root in production. Tests pass a temp dir as that same argument (no `Option` that hides the production path).
4. Read `data.path` only when the path is **absolute**, `path_is_under(cache_root, path)`, `len <= BROWSER_FRAME_IMAGE_PART_MAX_BYTES`, and magic is PNG or JPEG. STANDARD-encode those bytes for `BrowserFramePayload.image_base64`.
5. Else if `data.image_base64` is still present (old daemon), use it.
6. Else error `screenshot_viewport returned no path or image_base64`.

Apply `frame_bytes_from_mcp_data` to **both** the screencast-success arm and the viewport fallback (today each arm has its own `image_base64`-only read).

`BrowserView.tsx` currently hardcodes `data:image/png;base64,…`. After this program the live poll can be JPEG (screencast). Add `mime: String` to `BrowserFramePayload` (default `image/png`) and set `src={`data:${frame.mime};base64,${frame.image_base64}`}`. That is the only UI line that must change. Do not parse MCP JSON in the WebView.

`vox-gui` already depends on `base64` and `vox-config`. Persist failure is a tool error: the live view skips that tick. Do not fall back to embedding base64 in MCP JSON. Jailed path missing/unreadable: MCP attach stays silent (`image` None); GUI returns `Err` (cannot paint). Accepted divergence.

Do not send MCP image parts to the WebView.

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
| Decoded bytes over 400_000 | Strip / never add `image_base64`; no image part; `path` present. GUI must not base64-encode that file into the Tauri event |
| Cache write fails | Tool error envelope (`success: false`); no partial base64 fallback in JSON |
| Path outside cache jail / relative / non-image magic | Do not read into an image part; GUI `Err` |
| Jailed `path` missing/unreadable | MCP: no image part (text JSON remains). GUI: `Err`. Accepted divergence |
| Tool not in `FRAME_IMAGE_TOOL_NAMES` | Text-only MCP content even if `data.path` exists |
| `success: false` | Text-only MCP content; do not attach from `error` or `data.path` |
| Screencast JPEG labeled PNG | Forbidden. Honest `image/jpeg` + `.jpg` |
| `VOX_CACHE_DIR` differs across daemon / stdio MCP / GUI | Path attach fails closed (no part / no paint). Same-machine shared cache is required. Remote GUI is out of scope |
| Model ignores the skill | Allowed. No dispatch interceptor |

## 10. Testing (merge bar)

No live Chrome. Required tests (exact names in the plan):

- `promote_tool_image` strips `image_base64`, decodes a 1×1 PNG, omits part when over cap, leaves cookie-shaped JSON untouched. Over-cap may use a large zero buffer (not a real PNG); that still tests the byte cap.
- `mcp_contents_for_tool_json("vox_browser_screenshot_viewport", …, &tmp)` yields text + image. Serialized image uses **`mimeType`** (rmcp `camelCase`). Same helper with `"vox_browser_cookies_export"` and `{count,path}` is **length 1**.
- `mcp_contents_for_tool_json` on a path-only viewport JSON **fails** if `attach_image_from_cached_path` is deleted (path-only, not leftover `image_base64`).
- `server.rs` contains `mcp_contents_for_tool_json(&name_str, &result_json,` — `include_str!` mutation test (helper exists but `call_tool` still uses `Content::text` only).
- Wire: `content_parts: None` still a JSON **string**; `Some(image)` on a tool
  message becomes a text-only tool response plus a following user **array** with
  `image_url`. `stream_once` / `chat_once` share `build_request`.
- `plain_text_message_serializes_with_no_tool_keys` still length 2 (role + content).
- `retain_latest_tool_image`: two tool messages with parts → only the last keeps parts.
- GUI: `frame_bytes_from_mcp_data(&data, &jail)` — production call site passes `browser_frames_cache_dir()`, not `None`.
- `persist_browser_frame_png(tmp, …, LiveReplace)` twice → one `*-live.png` (PNG) or `*-live.jpg` (JPEG). Do **not** set `VOX_CACHE_DIR` and then `remove_var` before attach; pass `tmp` into persist **and** attach.
- `attach_skips_non_image_path_under_jail` — write `cookies.json` under the temp jail; attach stays `None`.
- `promote_screencast_jpeg_keeps_image_jpeg_mime` — JPEG magic → `mime: image/jpeg`, never `image/png`.
- `frame_bytes_from_mcp_data` rejects relative paths and files over the 400_000 cap.
- `cargo check -p vox-gamify -p vox-code-audit` after adding `content_parts`.
- `cache_dir` honors `VOX_CACHE_DIR` (existing `unsafe` set_var style). Prefer injectable roots for frame tests so they do not race env.
- `cargo test -p vox-plugin-catalog --test skill_bundle_parity`. Catalog vs SKILL.md descriptions need not be byte-identical.
- `vox ci operations-sync` / ssot-drift after catalog description edits.
- `vox ci agentskills-compliance` on the new skill.

Live Axis demo is **manual**: open a page, ask chat to summarize what is on screen,
confirm the second provider request includes a text-only tool response followed by a
user `image_url` part (proxy log or debug). Not a CI job.

## 10.1 Closed critique (keep these; do not re-open)

Verified against the tree at plan time. Discarded items are listed so a later pass does not re-introduce them.

**Keep (true positives)**

- Tool-name allowlist. Path-only JSON is not enough: cookie export is `{count, path}`.
- Injectable `cache_root` on persist, attach, `mcp_contents_for_tool_json`, `llm_tool_message`, and `frame_bytes_from_mcp_data`. `set_var(VOX_CACHE_DIR)` + `remove_var` before attach is a false green/red.
- `path_is_under` extracted from `cookie_import_path_ok`; jail after write.
- `LiveReplace` vs `Snapshot` + prune siblings (GUI polls every 3s).
- `retain_latest_tool_image` in the Axis loop. Ten screenshots must not send ten PNGs.
- stdio `call_tool` must pass `&name_str` (and `cache_root`). Helper-only tests are not enough — `include_str!` the call site.
- rmcp image = raw b64 + `mimeType`. OpenAI wire = `data:` URL in `image_url.url`. Do not mix.
- Operations catalog descriptions still say `image_base64` / “return base64”. Hand-edit then `operations-sync`.
- `vox-gui` has no lib target; `build.rs` wants `target/release/vox[-triple]`. If GUI tests fail on sidecar, `vox run scripts/gui-build.vox` in **this** worktree.
- Screencast is JPEG. Honest mime + `BrowserView` `data:${mime}` — do not lie `image/png`.
- No `vox-orchestrator-mcp` → `vox-llm-egress` edge. Re-export parts from `vox_actor_runtime::llm`. Map `content_parts` in **both** `chat.rs` and `stream.rs`.
- `ChatMessage` literals: 6 egress + 2 gamify + 2 code-audit. `skill_promotion.rs` needs the new field or `Default`.
- GUI read cap + absolute path. Same-PR MCP JSON + GUI reader.
- `VOX_CACHE_DIR.owner_crate` → `vox-config`.

**Discard (false positives / out of scope)**

- `vox_visual_rag_query` leaking pixels: params-only `image_base64`; result is `NOT_IMPLEMENTED`. Allowlist + result-only strip covers it.
- Cookie `{path}` becoming an image if allowlist + jail + magic hold.
- `approx_tokens` +1024 in `conversation.rs`: that bound only runs on remapped transcript (no parts).
- `browser_screenshot` growing an image part “if the path happens to sit under the cache”. It is not allowlisted.
- Transcoding JPEG→PNG (would need the `image` crate; banned).
- `promote` stripping visus `screenshot_base64` (different key; out of scope).
- Remote GUI against another host.
- New crate / ABI bump / Playwright / mega-tool / forced pipeline.

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
- Do not add a `vox-orchestrator-mcp` → `vox-llm-egress` crate edge.
- Do not grow `engine.rs` past the god-object cap.
- Do not hand-edit generated capability / tool-registry YAML. **Do** hand-edit `contracts/operations/catalog.v1.yaml` descriptions, then regenerate.
- Do not bump `VOX_PLUGIN_ABI_VERSION`.
- Do not add persist helpers to `browser_tools.rs` (~1498 lines).
- Image parts only for `FRAME_IMAGE_TOOL_NAMES`.
- Latest in-loop image only; live frames overwrite `*-live.png`.
- `vox-orchestrator-mcp` browser **handlers** still need `--features heavy-browser` to compile those tests; `tool_images` tests do not.
- `vox-gui` has **no** `--lib` target. GUI tests are `cargo test -p vox-gui <name>` (binary crate tests). Stub `crates/vox-gui/ui/dist/` if `tauri-build` requires it (gitignored; do not commit).
- Route `cargo` through PATH (build broker). Never `cargo fmt --all`.
