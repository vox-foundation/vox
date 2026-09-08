# Chat / Harness Research Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Axis chat and MCP clients receive page pixels as MCP / OpenAI image parts while screenshot JSON stays small (path + size, no `image_base64`), guided by a `browser-research` skill — no mega-tool.

**Architecture:** A always-on `tool_images` module strips `image_base64`, optionally reads a Tier D cache PNG, and builds `Content::image` plus additive `LlmChatMessage.content_parts`. Viewport/screencast handlers persist under `vox_config::paths::browser_frames_cache_dir()`. Wire emits a string `content` unless `content_parts` is set, then an OpenAI `image_url` array. GUI loads the cache path for `vox://browser-frame`.

**Tech Stack:** Rust, rmcp `Content::image`, `vox_llm_egress` OpenAI-compatible chat, existing `vox_browser_*` MCP, first-party `assets/skills/`.

**Spec:** [`docs/superpowers/specs/2026-09-08-chat-harness-research-loop-design.md`](../specs/2026-09-08-chat-harness-research-loop-design.md)

## Global Constraints

Copied from the spec — every task inherits these.

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
- Browser handler tests need `--features heavy-browser`. `tool_images` tests do not.
- `vox-gui` has no lib target: `cargo test -p vox-gui <name>`. Do not commit `ui/dist`.
- PATH `cargo` only (build broker).

## File map

| File | Role |
| --- | --- |
| Create: `crates/vox-orchestrator-mcp/src/tool_images.rs` | Promote, persist, MCP contents |
| Modify: `crates/vox-orchestrator-mcp/src/lib.rs` | `mod tool_images;` |
| Modify: `crates/vox-orchestrator-mcp/src/server.rs` | Image + text contents |
| Modify: `crates/vox-orchestrator-mcp/src/browser_tools.rs` | Path JSON for viewport/screencast |
| Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs` | `content_parts` on tool messages |
| Modify: `crates/vox-llm-egress/src/{lib,wire}.rs` + `tests/wire_mock.rs` | Parts + wire array |
| Modify: `crates/vox-actor-runtime/src/llm/{types,chat}.rs` | Pass-through field |
| Modify: `crates/vox-config/src/paths.rs` | `cache_dir` / frames leaf |
| Modify: `crates/vox-gui/src/commands/browser.rs` | Read cache path |
| Create: `assets/skills/browser-research/SKILL.md` | Research loop |
| Modify: `crates/vox-plugin-catalog/catalog.toml` | skill-bundle |
| Modify: `contracts/db/data-storage-policy.v1.yaml` | FS write allowlist |
| Modify: SSOTs listed in Task 5 | Record the loop |

---

### Task 0: Pure `promote_tool_image`

**Files:**
- Create: `crates/vox-orchestrator-mcp/src/tool_images.rs`
- Modify: `crates/vox-orchestrator-mcp/src/lib.rs` (add `pub mod tool_images;` next to `pub mod caller_role;`)

**Interfaces:**
- Consumes: `ToolResult` JSON strings (`success` / `data`)
- Produces: `BROWSER_FRAME_IMAGE_PART_MAX_BYTES`, `ImageBlob`, `PromotedToolImage`, `promote_tool_image`, `json_text`

- [ ] **Step 1: Write the failing tests**

Add at the bottom of the new file (file will not compile until Step 3 — write tests first in the same file after a stub, or write tests that name the missing items so rustc fails).

1×1 PNG (67 bytes decoded):

```
iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==
```

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const PNG_1X1_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

    #[test]
    fn promote_strips_image_base64_and_keeps_decoded_bytes() {
        let raw = serde_json::json!({
            "success": true,
            "data": {
                "image_base64": PNG_1X1_B64,
                "viewport_width": 1280,
                "page_id": "p1"
            }
        })
        .to_string();
        let got = promote_tool_image(&raw);
        let data = got.json["data"].as_object().expect("data");
        assert!(!data.contains_key("image_base64"), "{data:?}");
        assert_eq!(data["viewport_width"], 1280);
        let img = got.image.expect("image");
        assert_eq!(img.mime, "image/png");
        assert_eq!(img.bytes.len(), 67);
        let text = json_text(&got);
        assert!(!text.contains("image_base64"));
    }

    #[test]
    fn promote_over_cap_strips_but_drops_part() {
        let big = vec![0u8; BROWSER_FRAME_IMAGE_PART_MAX_BYTES + 1];
        let b64 = base64::engine::general_purpose::STANDARD.encode(&big);
        let raw = serde_json::json!({
            "success": true,
            "data": { "image_base64": b64 }
        })
        .to_string();
        let got = promote_tool_image(&raw);
        assert!(got.image.is_none());
        assert!(
            !got.json["data"]
                .as_object()
                .unwrap()
                .contains_key("image_base64")
        );
    }

    #[test]
    fn promote_invalid_base64_strips_without_echo() {
        let raw = r#"{"success":true,"data":{"image_base64":"%%%not-valid%%%"}}"#;
        let got = promote_tool_image(raw);
        assert!(got.image.is_none());
        assert!(!json_text(&got).contains("image_base64"));
        assert!(!json_text(&got).contains("%%%not-valid%%%"));
    }

    #[test]
    fn promote_leaves_cookie_shaped_json_alone() {
        let raw = serde_json::json!({
            "success": true,
            "data": { "count": 3, "path": "consents.json" }
        })
        .to_string();
        let got = promote_tool_image(&raw);
        assert!(got.image.is_none());
        assert_eq!(got.json["data"]["count"], 3);
        assert_eq!(got.json["data"]["path"], "consents.json");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p vox-orchestrator-mcp --lib promote_strips_image_base64 -- --nocapture`

Expected: compile fail `cannot find function promote_tool_image` (or the module is missing).

- [ ] **Step 3: Write minimal implementation**

`crates/vox-orchestrator-mcp/src/tool_images.rs` (no persist yet):

```rust
//! Strip `image_base64` from MCP tool JSON and optionally keep the decoded PNG.

use base64::Engine;
use serde_json::Value;

pub const BROWSER_FRAME_IMAGE_PART_MAX_BYTES: usize = 400_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBlob {
    pub mime: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PromotedToolImage {
    pub json: Value,
    pub image: Option<ImageBlob>,
}

pub fn json_text(promoted: &PromotedToolImage) -> String {
    serde_json::to_string(&promoted.json).unwrap_or_else(|_| "{}".into())
}

pub fn promote_tool_image(result_json: &str) -> PromotedToolImage {
    let mut json: Value = serde_json::from_str(result_json)
        .unwrap_or_else(|_| Value::String(result_json.to_string()));
    let image = strip_image_base64(&mut json);
    PromotedToolImage { json, image }
}

fn strip_image_base64(value: &mut Value) -> Option<ImageBlob> {
    let mut found = None;
    match value {
        Value::Object(map) => {
            if let Some(Value::String(b64)) = map.remove("image_base64") {
                found = decode_png_b64(&b64);
            }
            for v in map.values_mut() {
                if found.is_none() {
                    found = strip_image_base64(v);
                } else {
                    let _ = strip_image_base64(v);
                }
            }
        }
        Value::Array(items) => {
            for v in items {
                if found.is_none() {
                    found = strip_image_base64(v);
                } else {
                    let _ = strip_image_base64(v);
                }
            }
        }
        _ => {}
    }
    found
}

fn decode_png_b64(b64: &str) -> Option<ImageBlob> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64.trim()).ok()?;
    if bytes.len() > BROWSER_FRAME_IMAGE_PART_MAX_BYTES {
        return None;
    }
    Some(ImageBlob {
        mime: "image/png".into(),
        bytes,
    })
}
```

`lib.rs`: `pub mod tool_images;`

Confirm `base64` is already a `[dependencies]` entry of `vox-orchestrator-mcp` (it is used by `browser_tools.rs`). If a default-features check fails to compile `tool_images` without `heavy-browser`, add `base64.workspace = true` next to the other deps (do not hide `tool_images` behind `heavy-browser`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p vox-orchestrator-mcp --lib promote_strips_image_base64 promote_over_cap_strips_but_drops_part promote_invalid_base64_strips_without_echo promote_leaves_cookie_shaped_json_alone`

Expected: four PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p vox-orchestrator-mcp
git add crates/vox-orchestrator-mcp/src/tool_images.rs crates/vox-orchestrator-mcp/src/lib.rs
git commit -m "$(cat <<'EOF'
feat: strip image_base64 from MCP tool JSON

Give chat a promote helper so screenshot pixels can move to MCP image parts
without riding in the text envelope.
EOF
)"
```

---

### Task 1: Cache paths, persist, screenshot JSON, GUI reader

**Files:**
- Modify: `crates/vox-config/src/paths.rs`
- Modify: `crates/vox-orchestrator-mcp/src/tool_images.rs` (persist + attach from path + `mcp_contents_for_tool_json`)
- Modify: `crates/vox-orchestrator-mcp/src/browser_tools.rs` (`browser_screenshot_viewport`, `browser_screencast_frame`)
- Modify: `crates/vox-gui/src/commands/browser.rs`
- Modify: `contracts/db/data-storage-policy.v1.yaml` (`direct_fs_write_allowlist` add `crates/vox-orchestrator-mcp/src/tool_images.rs`)

**Interfaces:**
- Consumes: `promote_tool_image`, `ImageBlob`, `BROWSER_FRAME_IMAGE_PART_MAX_BYTES`
- Produces: `cache_dir()`, `BROWSER_FRAMES_CACHE_LEAF`, `browser_frames_cache_dir()`, `persist_browser_frame_png`, `attach_image_from_cached_path`, `mcp_contents_for_tool_json`, `frame_bytes_from_mcp_data`

- [ ] **Step 1: Write the failing tests**

In `crates/vox-config/src/paths.rs` `#[cfg(test)]` (same `unsafe` set_var style as existing path tests):

```rust
#[test]
fn cache_dir_honors_vox_cache_dir() {
    #![allow(unsafe_code)]
    let tmp = std::env::temp_dir().join(format!("vox-cache-test-{}", std::process::id()));
    unsafe { std::env::set_var("VOX_CACHE_DIR", &tmp) };
    let got = cache_dir();
    unsafe { std::env::remove_var("VOX_CACHE_DIR") };
    assert_eq!(got, tmp);
    assert_eq!(browser_frames_cache_dir(), tmp.join(BROWSER_FRAMES_CACHE_LEAF));
}
```

In `tool_images.rs` tests:

```rust
    #[test]
    fn persist_then_attach_reads_jailed_png() {
        #![allow(unsafe_code)]
        let tmp = std::env::temp_dir().join(format!("vox-frames-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        unsafe { std::env::set_var("VOX_CACHE_DIR", &tmp) };
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(PNG_1X1_B64)
            .unwrap();
        let path = persist_browser_frame_png("page-1", &bytes).expect("write");
        unsafe { std::env::remove_var("VOX_CACHE_DIR") };
        let raw = serde_json::json!({
            "success": true,
            "data": {
                "page_id": "page-1",
                "path": path.to_string_lossy(),
                "width": 1,
                "height": 1,
                "mime": "image/png"
            }
        })
        .to_string();
        let mut got = promote_tool_image(&raw);
        attach_image_from_cached_path(&mut got);
        assert_eq!(got.image.as_ref().map(|i| i.bytes.len()), Some(67));
        let contents = mcp_contents_for_tool_json(&raw);
        assert_eq!(contents.len(), 2);
    }

    #[test]
    fn attach_rejects_path_outside_cache() {
        let raw = serde_json::json!({
            "success": true,
            "data": { "path": "/tmp/not-vox-cache/x.png" }
        })
        .to_string();
        let mut got = promote_tool_image(&raw);
        attach_image_from_cached_path(&mut got);
        assert!(got.image.is_none());
    }
```

In `crates/vox-gui/src/commands/browser.rs` `mod tests`:

```rust
    #[test]
    fn frame_bytes_from_mcp_data_reads_path() {
        let dir = std::env::temp_dir().join(format!("vox-gui-frame-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f.png");
        let png = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
        )
        .unwrap();
        std::fs::write(&path, &png).unwrap();
        let data = serde_json::json!({
            "path": path.to_string_lossy(),
            "width": 1,
            "height": 1
        });
        let (b64, w, h) = frame_bytes_from_mcp_data(&data, Some(&dir)).expect("read");
        assert_eq!(w, Some(1));
        assert_eq!(h, Some(1));
        assert!(!b64.is_empty());
    }

    #[test]
    fn frame_bytes_from_mcp_data_rejects_escape() {
        let jail = std::env::temp_dir().join(format!("vox-gui-jail-{}", std::process::id()));
        std::fs::create_dir_all(&jail).unwrap();
        let data = serde_json::json!({ "path": "/etc/hosts" });
        assert!(frame_bytes_from_mcp_data(&data, Some(&jail)).is_err());
    }
```

If `vox-gui` does not already depend on `base64`, decode in the test with the same crate the command file uses, or write raw `png` bytes with `include_bytes!` — do not add a new crate. Prefer `std::fs::write` of a hand-built 67-byte vec copied from `STANDARD.decode` in the test using `vox_gui`’s existing deps; if `base64` is missing, paste the 67 bytes as a `[u8; 67]` literal from a local decode you run once.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```
cargo test -p vox-config --lib cache_dir_honors_vox_cache_dir
cargo test -p vox-orchestrator-mcp --lib persist_then_attach_reads_jailed_png
cargo test -p vox-gui frame_bytes_from_mcp_data_reads_path
```

Expected: FAIL / missing `cache_dir` / `persist_browser_frame_png` / `frame_bytes_from_mcp_data`.

- [ ] **Step 3: Write minimal implementation**

`paths.rs` (next to `data_dir`):

```rust
pub const BROWSER_FRAMES_CACHE_LEAF: &str = "browser-frames";

pub fn cache_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("VOX_CACHE_DIR")
        && !dir.is_empty()
    {
        return PathBuf::from(dir);
    }
    platform_cache_dir()
        .unwrap_or_else(|| std::env::temp_dir().join("vox-cache"))
        .join(APP_DIR_NAME)
}

pub fn browser_frames_cache_dir() -> PathBuf {
    cache_dir().join(BROWSER_FRAMES_CACHE_LEAF)
}

fn platform_cache_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    return std::env::var("LOCALAPPDATA").ok().map(PathBuf::from);
    #[cfg(target_os = "macos")]
    return Some(user_home_dir().join("Library").join("Caches"));
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        if let Ok(xdg) = std::env::var("XDG_CACHE_HOME")
            && !xdg.is_empty()
        {
            return Some(PathBuf::from(xdg));
        }
        Some(user_home_dir().join(".cache"))
    }
}
```

`tool_images.rs` add:

```rust
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use rmcp::model::Content;

pub fn persist_browser_frame_png(page_id: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let root = vox_config::paths::browser_frames_cache_dir();
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let safe: String = page_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let safe = if safe.is_empty() {
        "page".into()
    } else {
        safe
    };
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let dest = root.join(format!("{safe}-{ms}.png"));
    std::fs::write(&dest, bytes).map_err(|e| e.to_string())?;
    if !vox_config::paths::cookie_import_path_ok(&root, &dest) {
        let _ = std::fs::remove_file(&dest);
        return Err("frame path escaped cache jail".into());
    }
    Ok(dest)
}

pub fn attach_image_from_cached_path(promoted: &mut PromotedToolImage) {
    if promoted.image.is_some() {
        return;
    }
    let Some(path) = promoted.json.pointer("/data/path").and_then(|v| v.as_str()) else {
        return;
    };
    let path = Path::new(path);
    let root = vox_config::paths::browser_frames_cache_dir();
    if !vox_config::paths::cookie_import_path_ok(&root, path) {
        return;
    }
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    if bytes.len() > BROWSER_FRAME_IMAGE_PART_MAX_BYTES {
        return;
    }
    promoted.image = Some(ImageBlob {
        mime: "image/png".into(),
        bytes,
    });
}

pub fn mcp_contents_for_tool_json(result_json: &str) -> Vec<Content> {
    let mut promoted = promote_tool_image(result_json);
    attach_image_from_cached_path(&mut promoted);
    let mut out = vec![Content::text(json_text(&promoted))];
    if let Some(img) = promoted.image {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&img.bytes);
        out.push(Content::image(b64, img.mime));
    }
    out
}
```

`browser_screenshot_viewport` success arm — replace the `image_base64` JSON with persist + path fields. Keep using `png_dimensions`:

```rust
        Ok(Ok(bytes)) => {
            let (width, height) = png_dimensions(&bytes).unwrap_or((0, 0));
            match crate::tool_images::persist_browser_frame_png(&page_id, &bytes) {
                Ok(path) => ToolResult::ok(serde_json::json!({
                    "page_id": page_id,
                    "path": path.to_string_lossy(),
                    "width": width,
                    "height": height,
                    "mime": "image/png"
                }))
                .to_json(),
                Err(e) => ToolResult::<serde_json::Value>::err(e).to_json(),
            }
        }
```

`browser_screencast_frame` success arm — after `ToolResult::ok(value)`, do not return the plugin object as-is. Persist if `value["image_base64"]` is a string:

```rust
        Ok(Ok(value)) => persist_screencast_json(&page_id, value),
```

```rust
fn persist_screencast_json(page_id: &str, value: serde_json::Value) -> String {
    let mut promoted = crate::tool_images::promote_tool_image(&value.to_string());
    if let Some(img) = promoted.image.take() {
        match crate::tool_images::persist_browser_frame_png(page_id, &img.bytes) {
            Ok(path) => {
                let width = value.get("viewport_width").cloned().unwrap_or(serde_json::json!(0));
                let height = value.get("viewport_height").cloned().unwrap_or(serde_json::json!(0));
                return ToolResult::ok(serde_json::json!({
                    "page_id": page_id,
                    "path": path.to_string_lossy(),
                    "width": width,
                    "height": height,
                    "mime": "image/png"
                }))
                .to_json();
            }
            Err(e) => return ToolResult::<serde_json::Value>::err(e).to_json(),
        }
    }
    ToolResult::ok(promoted.json).to_json()
}
```

Put `persist_screencast_json` in `browser_tools.rs` (not `engine.rs`). If `browser_tools.rs` is near the line cap, put it in `tool_images.rs` as `pub fn tool_json_from_screencast_value(page_id: &str, value: Value) -> String` instead.

GUI — extract and switch `capture_frame_png_base64` to:

```rust
fn frame_bytes_from_mcp_data(
    data: &serde_json::Value,
    cache_root: Option<&std::path::Path>,
) -> Result<(String, Option<u32>, Option<u32>), String> {
    let width = data.get("width").or_else(|| data.get("viewport_width")).and_then(|v| v.as_u64()).map(|v| v as u32);
    let height = data.get("height").or_else(|| data.get("viewport_height")).and_then(|v| v.as_u64()).map(|v| v as u32);
    if let Some(path) = data.get("path").and_then(|v| v.as_str()) {
        let root = cache_root
            .map(std::path::PathBuf::from)
            .unwrap_or_else(vox_config::paths::browser_frames_cache_dir);
        let p = std::path::Path::new(path);
        if !vox_config::paths::cookie_import_path_ok(&root, p) {
            return Err("screenshot path escaped cache jail".into());
        }
        let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
        let image_base64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        return Ok((image_base64, width, height));
    }
    if let Some(image_base64) = data.get("image_base64").and_then(|v| v.as_str()) {
        return Ok((image_base64.to_string(), width, height));
    }
    Err("screenshot_viewport returned no path or image_base64".into())
}
```

`capture_frame_png_base64` after `mcp_data`: `frame_bytes_from_mcp_data(&data, None)`.

Add `base64` to `vox-gui` only if the crate does not already encode anywhere; `browser.rs` already mentions `image_base64` so the dep should exist. Confirm `vox-gui` depends on `vox-config`.

Allowlist row in `contracts/db/data-storage-policy.v1.yaml` under `direct_fs_write_allowlist`:

```yaml
    - crates/vox-orchestrator-mcp/src/tool_images.rs
```

- [ ] **Step 4: Run tests to verify they pass**

```
cargo test -p vox-config --lib cache_dir_honors_vox_cache_dir
cargo test -p vox-orchestrator-mcp --lib persist_then_attach_reads_jailed_png attach_rejects_path_outside_cache
cargo test -p vox-gui frame_bytes_from_mcp_data_reads_path frame_bytes_from_mcp_data_rejects_escape preview_url_must_be_loopback
```

Expected: PASS. If `vox-gui` fails on missing `ui/dist`, create the empty stub directory locally (do not commit).

- [ ] **Step 5: Commit**

```bash
cargo fmt -p vox-config -p vox-orchestrator-mcp -p vox-gui
git add crates/vox-config/src/paths.rs crates/vox-orchestrator-mcp/src/tool_images.rs crates/vox-orchestrator-mcp/src/browser_tools.rs crates/vox-gui/src/commands/browser.rs contracts/db/data-storage-policy.v1.yaml
git commit -m "$(cat <<'EOF'
feat: persist browser frames under VOX_CACHE_DIR

Keep MCP JSON to a jailed path so chat text stays small and the GUI live
view can still paint without image_base64 in the tool envelope.
EOF
)"
```

---

### Task 2: MCP `Content::image` on `call_tool`

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/server.rs` (the `let content = vec![Content::text(result_json)];` line)

**Interfaces:**
- Consumes: `mcp_contents_for_tool_json`
- Produces: `CallToolResult` with text + optional image content

- [ ] **Step 1: Write the failing test**

Task 1 already asserts `mcp_contents_for_tool_json` length 2. Add a serialization lock in `tool_images.rs`:

```rust
    #[test]
    fn mcp_contents_serializes_image_type() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(PNG_1X1_B64)
            .unwrap();
        #![allow(unsafe_code)]
        let tmp = std::env::temp_dir().join(format!("vox-mcp-img-{}", std::process::id()));
        unsafe { std::env::set_var("VOX_CACHE_DIR", &tmp) };
        let path = persist_browser_frame_png("p", &bytes).unwrap();
        unsafe { std::env::remove_var("VOX_CACHE_DIR") };
        let raw = serde_json::json!({
            "success": true,
            "data": { "path": path.to_string_lossy() }
        })
        .to_string();
        let contents = mcp_contents_for_tool_json(&raw);
        let v = serde_json::to_value(&contents).expect("content json");
        let arr = v.as_array().expect("array");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[1]["type"], "image");
        assert_eq!(arr[1]["mimeType"], "image/png");
        assert!(arr[1]["data"].as_str().unwrap().len() > 8);
        assert_eq!(arr[0]["type"], "text");
        assert!(!arr[0]["text"].as_str().unwrap().contains("image_base64"));
    }

    #[test]
    fn mcp_contents_error_is_text_only() {
        let raw = r#"{"success":false,"error":"no page"}"#;
        let contents = mcp_contents_for_tool_json(raw);
        assert_eq!(contents.len(), 1);
    }
```

If rmcp serializes `mime_type` instead of `mimeType`, assert whichever key `serde_json::to_value(&contents[1])` actually emits — print once on first fail and lock that spelling.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-orchestrator-mcp --lib mcp_contents_serializes_image_type`

Expected: FAIL only if Step 1 of Task 1 did not yet cover serialization — if it already passes, keep the test as the lock and continue.

- [ ] **Step 3: Wire `server.rs`**

Replace:

```rust
        let content = vec![Content::text(result_json)];
```

with:

```rust
        let content = crate::tool_images::mcp_contents_for_tool_json(&result_json);
```

- [ ] **Step 4: Run tests**

```
cargo test -p vox-orchestrator-mcp --lib mcp_contents_serializes_image_type mcp_contents_error_is_text_only
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p vox-orchestrator-mcp
git add crates/vox-orchestrator-mcp/src/server.rs crates/vox-orchestrator-mcp/src/tool_images.rs
git commit -m "$(cat <<'EOF'
feat: attach MCP image parts for browser frames

External MCP clients can see the page without parsing a base64 field out
of the tool text.
EOF
)"
```

---

### Task 3: Additive `content_parts` on the LLM wire

**Files:**
- Modify: `crates/vox-llm-egress/src/lib.rs`
- Modify: `crates/vox-llm-egress/src/wire.rs`
- Modify: `crates/vox-llm-egress/tests/wire_mock.rs` (every `ChatMessage {` gains `content_parts: None`)
- Modify: `crates/vox-actor-runtime/src/llm/types.rs`
- Modify: `crates/vox-actor-runtime/src/llm/chat.rs`

**Interfaces:**
- Consumes: none from Tasks 0–2 (types only)
- Produces: `LlmContentPart`, `LlmImageUrl`, `ChatMessage.content_parts`, `LlmChatMessage.content_parts`, wire array `content`

- [ ] **Step 1: Write the failing tests**

In `crates/vox-llm-egress/src/lib.rs` `mod tests`, after `plain_text_message_serializes_with_no_tool_keys`, add:

```rust
    #[test]
    fn content_parts_none_omits_key() {
        let msg = ChatMessage {
            role: "user".into(),
            content: "hello".into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            content_parts: None,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert!(!json.as_object().unwrap().contains_key("content_parts"));
    }
```

Update `plain_text_message_serializes_with_no_tool_keys` to set `content_parts: None` and keep `obj.len() == 2`.

In `wire.rs` add `#[cfg(test)]` (or a `pub(crate)` helper tested from `lib.rs`):

```rust
#[cfg(test)]
pub(crate) fn wire_content_json(m: &ChatMessage) -> serde_json::Value {
    wire_content(m)
}
```

and tests in `lib.rs`:

```rust
    #[test]
    fn wire_content_stays_string_without_parts() {
        let msg = ChatMessage {
            role: "tool".into(),
            content: "{\"success\":true}".into(),
            tool_calls: None,
            tool_call_id: Some("c1".into()),
            name: Some("vox_browser_snapshot".into()),
            content_parts: None,
        };
        let c = crate::wire::wire_content_json(&msg);
        assert_eq!(c, serde_json::json!("{\"success\":true}"));
    }

    #[test]
    fn wire_content_array_with_image_url() {
        let msg = ChatMessage {
            role: "tool".into(),
            content: "{\"success\":true,\"data\":{\"path\":\"/x\"}}".into(),
            tool_calls: None,
            tool_call_id: Some("c1".into()),
            name: Some("vox_browser_screenshot_viewport".into()),
            content_parts: Some(vec![LlmContentPart::ImageUrl {
                image_url: LlmImageUrl {
                    url: "data:image/png;base64,aaa".into(),
                },
            }]),
        };
        let c = crate::wire::wire_content_json(&msg);
        let arr = c.as_array().expect("array");
        assert_eq!(arr[0]["type"], "text");
        assert_eq!(arr[1]["type"], "image_url");
        assert_eq!(arr[1]["image_url"]["url"], "data:image/png;base64,aaa");
        assert!(!c.to_string().contains("image_base64"));
    }
```

`wire_content_json` must be `pub(crate)` in the `wire` module. `mod wire` is private — `crate::wire::…` works inside `vox-llm-egress`.

Add a `wire_mock.rs` test that `body_partial_json` matches an array `content` when `content_parts` is set (copy `chat_once_sends_bearer_headers_and_parses_usage`, change the message, and assert the mock matcher):

```rust
#[tokio::test]
async fn chat_once_sends_image_url_array_content() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_partial_json(serde_json::json!({
            "messages": [{
                "role": "tool",
                "content": [
                    {"type": "text", "text": "{\"ok\":true}"},
                    {"type": "image_url", "image_url": {"url": "data:image/png;base64,aaa"}}
                ]
            }]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "model": "test/model",
            "choices": [{"message": {"role": "assistant", "content": "saw"}}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        })))
        .mount(&server)
        .await;
    let r = req(format!("{}/chat/completions", server.uri()));
    let msgs = vec![ChatMessage {
        role: "tool".into(),
        content: "{\"ok\":true}".into(),
        tool_calls: None,
        tool_call_id: Some("c1".into()),
        name: None,
        content_parts: Some(vec![vox_llm_egress::LlmContentPart::ImageUrl {
            image_url: vox_llm_egress::LlmImageUrl {
                url: "data:image/png;base64,aaa".into(),
            },
        }]),
    }];
    let out = chat_once(&r, &msgs, &ChatParams::default())
        .await
        .expect("ok");
    assert_eq!(out.content, "saw");
}
```

- [ ] **Step 2: Run tests to verify they fail**

```
cargo test -p vox-llm-egress --lib wire_content_array_with_image_url
```

Expected: FAIL (`LlmContentPart` / `content_parts` unknown, or `ChatMessage` missing field).

- [ ] **Step 3: Implement types + wire**

In `lib.rs` next to `ChatMessage`:

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
    pub url: String,
}
```

Add `content_parts` to `ChatMessage` with `#[serde(default, skip_serializing_if = "Option::is_none")]`.

`wire.rs` — change `WireMessage`:

```rust
    content: serde_json::Value,
```

```rust
fn wire_content(m: &ChatMessage) -> serde_json::Value {
    match m.content_parts.as_ref() {
        Some(parts) if !parts.is_empty() => {
            let mut arr = vec![serde_json::json!({"type":"text","text": m.content})];
            for p in parts {
                if let LlmContentPart::ImageUrl { image_url } = p {
                    arr.push(serde_json::json!({
                        "type": "image_url",
                        "image_url": { "url": image_url.url }
                    }));
                }
            }
            serde_json::Value::Array(arr)
        }
        _ => serde_json::Value::String(m.content.clone()),
    }
}
```

Import `LlmContentPart` in `wire.rs`. `From<&ChatMessage>` sets `content: wire_content(m)`.

`LlmChatMessage` in `types.rs`:

```rust
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_parts: Option<Vec<vox_llm_egress::LlmContentPart>>,
```

`chat.rs` map:

```rust
                .map(|m| vox_llm_egress::ChatMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                    tool_calls: m.tool_calls.clone(),
                    tool_call_id: m.tool_call_id.clone(),
                    name: m.name.clone(),
                    content_parts: m.content_parts.clone(),
                })
```

Fix every `ChatMessage {` in `vox-llm-egress` (lib tests + `wire_mock.rs`) with `content_parts: None`. `cargo test -p vox-llm-egress` lists leftovers.

`LlmChatMessage` already uses `Default` in the agent loop; the new field needs `#[serde(default)]` so `..Default::default()` keeps working. Add `content_parts: None` to the `Default` derive (automatic if `Option`).

- [ ] **Step 4: Run tests**

```
cargo test -p vox-llm-egress
cargo test -p vox-actor-runtime --lib
```

Expected: PASS. `plain_text_message_serializes_with_no_tool_keys` still `obj.len() == 2`.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p vox-llm-egress -p vox-actor-runtime
git add crates/vox-llm-egress crates/vox-actor-runtime/src/llm/types.rs crates/vox-actor-runtime/src/llm/chat.rs
git commit -m "$(cat <<'EOF'
feat: additive LLM content_parts for image_url

Keep string content for existing judges and ghost-text; Axis can now
forward a screenshot as an OpenAI image_url part.
EOF
)"
```

---

### Task 4: Axis agent loop attaches parts

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs` (the `messages.push(LlmChatMessage { role: "tool"` block)

**Interfaces:**
- Consumes: `promote_tool_image`, `attach_image_from_cached_path`, `json_text`, `LlmContentPart::ImageUrl`
- Produces: tool `LlmChatMessage` with stripped `content` and optional `content_parts`

- [ ] **Step 1: Write the failing test**

Add a unit next to `tool_call_dispatches_and_feeds_result_back_with_matching_call_id` that does **not** need Chrome. Build the tool message the same way the loop will:

```rust
    #[test]
    fn tool_result_message_gets_image_part_from_cached_png() {
        #![allow(unsafe_code)]
        let png_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(png_b64)
            .unwrap();
        let tmp = std::env::temp_dir().join(format!("vox-loop-img-{}", std::process::id()));
        unsafe { std::env::set_var("VOX_CACHE_DIR", &tmp) };
        let path = crate::tool_images::persist_browser_frame_png("pg", &bytes).unwrap();
        unsafe { std::env::remove_var("VOX_CACHE_DIR") };
        let content = serde_json::json!({
            "success": true,
            "data": { "path": path.to_string_lossy(), "mime": "image/png" }
        })
        .to_string();
        let msg = crate::tool_images::llm_tool_message("call_1", "vox_browser_screenshot_viewport", content);
        assert!(!msg.content.contains("image_base64"));
        let parts = msg.content_parts.expect("parts");
        match &parts[0] {
            vox_llm_egress::LlmContentPart::ImageUrl { image_url } => {
                assert!(image_url.url.starts_with("data:image/png;base64,"));
            }
            _ => panic!("expected image_url"),
        }
    }
```

This test lives in `tool_images.rs` (always compiled). Name the helper `llm_tool_message` so the loop does not reimplement promote.

- [ ] **Step 2: Run test to verify it fails**

```
cargo test -p vox-orchestrator-mcp --lib tool_result_message_gets_image_part_from_cached_png
```

Expected: FAIL `llm_tool_message` not found.

- [ ] **Step 3: Implement helper + loop call**

In `tool_images.rs`:

```rust
pub fn llm_tool_message(
    tool_call_id: impl Into<String>,
    name: impl Into<String>,
    result_json: String,
) -> vox_actor_runtime::llm::LlmChatMessage {
    let mut promoted = promote_tool_image(&result_json);
    attach_image_from_cached_path(&mut promoted);
    let content = json_text(&promoted);
    let content_parts = promoted.image.map(|img| {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&img.bytes);
        vec![vox_llm_egress::LlmContentPart::ImageUrl {
            image_url: vox_llm_egress::LlmImageUrl {
                url: format!("data:{};base64,{b64}", img.mime),
            },
        }]
    });
    vox_actor_runtime::llm::LlmChatMessage {
        role: "tool".into(),
        content,
        tool_call_id: Some(tool_call_id.into()),
        name: Some(name.into()),
        content_parts,
        ..Default::default()
    }
}
```

Confirm `vox-orchestrator-mcp` already depends on `vox-actor-runtime` and `vox-llm-egress`.

In `agent_loop.rs` replace:

```rust
                    messages.push(LlmChatMessage {
                        role: "tool".into(),
                        content,
                        tool_call_id: Some(call.id.clone()),
                        name: Some(call.name.clone()),
                        ..Default::default()
                    });
```

with:

```rust
                    messages.push(crate::tool_images::llm_tool_message(
                        call.id.clone(),
                        call.name.clone(),
                        content,
                    ));
```

Error strings (`Error: …`) still go through `llm_tool_message`; promote is a no-op and `content_parts` stays `None`.

- [ ] **Step 4: Run tests**

```
cargo test -p vox-orchestrator-mcp --lib tool_result_message_gets_image_part_from_cached_png tool_call_dispatches_and_feeds_result_back_with_matching_call_id
```

The existing dispatch test still expects a `role: tool` string `content` for `vox_git_status` (no image). Assert it still has `tool_call_id == call_1`. If that test now sees `content` as an object because of a mistaken always-on `content_parts`, fix `llm_tool_message` so `content_parts` is `None` when there is no image — wire then stays a string.

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p vox-orchestrator-mcp
git add crates/vox-orchestrator-mcp/src/tool_images.rs crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs
git commit -m "$(cat <<'EOF'
feat: forward browser frames through the Axis tool loop

The second model request can include an image_url part so Axis sees the
page instead of a JSON blob of pixels.
EOF
)"
```

---

### Task 5: `browser-research` skill + SSOT

**Files:**
- Create: `assets/skills/browser-research/SKILL.md`
- Modify: `crates/vox-plugin-catalog/catalog.toml`
- Modify: `docs/src/architecture/vox-gui-browser-support-2026.md`
- Modify: `docs/src/architecture/where-things-live.md` (row ~230)
- Modify: `docs/src/architecture/agent-browser-driver-research-2026.md` §7 leftover
- Modify: `docs/src/architecture/data-storage-ssot-2026.md` §4.4 (one bullet: browser frames)

**Interfaces:**
- Consumes: shipped tools and the image-part transport
- Produces: catalog-discoverable skill; docs that match code

- [ ] **Step 1: Write the failing catalog test (already exists)**

`crates/vox-plugin-catalog/tests/skill_bundle_parity.rs` fails if `assets/skills/browser-research/SKILL.md` exists without a `[[skill-bundle]]`, and fails if the bundle exists without the file. Create the skill first, run parity, then add the bundle (TDD: add SKILL.md, run fail, add catalog row).

- [ ] **Step 2: Add SKILL.md only, run parity**

Create `assets/skills/browser-research/SKILL.md`:

```markdown
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
```

Run: `cargo test -p vox-plugin-catalog --test skill_bundle_parity`

Expected: FAIL `assets/skills/browser-research missing [[skill-bundle]]`.

- [ ] **Step 3: Catalog row + docs**

Append to `catalog.toml` (after the `vox-graph` block):

```toml
[[skill-bundle]]
id = "browser-research"
description = "Use when the user wants to research, summarize, or cite what a web page shows in the agent browser. Snapshot first, then screenshot so you can see the page, then extract and quote — never invent CSS or dump cookies."
status = "stable"
license = "Apache-2.0"
source = "https://github.com/vox-foundation/vox"
pin = "2026-09-08-chat-research-loop"
bundle-path = "assets/skills/browser-research"
```

`description` must stay ≤ 1024 chars and match the SKILL.md description closely (tier-1 uses the parsed skill description; catalog description is for the plugin catalog).

`vox-gui-browser-support-2026.md` — after **Semantic loop**, add:

```markdown
**Chat research loop:** MCP `call_tool` attaches an image part for viewport/screencast frames. JSON is `{page_id, path, width, height, mime}` under `$VOX_CACHE_DIR/browser-frames` (Tier D). Axis maps the part through `LlmChatMessage.content_parts`. Skill: `browser-research`. Contract: [2026-09-08 chat harness research loop](../../superpowers/specs/2026-09-08-chat-harness-research-loop-design.md).
```

Related-docs bullet: add the 2026-09-08 spec.

`where-things-live.md` replace/extend the snapshot row:

```markdown
| Agent browser snapshot + refs / named profiles / Chrome attach / chat research loop | Snapshot/refs/profiles/attach: [`2026-09-07-agent-browser-driver-design.md`](../../superpowers/specs/2026-09-07-agent-browser-driver-design.md). Chat pixels: [`2026-09-08-chat-harness-research-loop-design.md`](../../superpowers/specs/2026-09-08-chat-harness-research-loop-design.md) — `tool_images.rs`, MCP `Content::image`, `LlmChatMessage.content_parts`, skill `assets/skills/browser-research/`. Frames via `vox_config::paths::browser_frames_cache_dir()` (Tier D). |
```

`agent-browser-driver-research-2026.md` §7 leftover — add:

```markdown
**Chat/harness loop (2026-09-08):** MCP image parts + Axis `content_parts` + `browser-research` skill. Spec [`docs/superpowers/specs/2026-09-08-chat-harness-research-loop-design.md`](../../superpowers/specs/2026-09-08-chat-harness-research-loop-design.md). Still later: computer-use, chrome-devtools-mcp as optional skills, iframe merge, SPA observer, NeedsYou rows, Vox `Browser.*` ref builtins.
```

`data-storage-ssot-2026.md` §4.4 list — add: `browser-frames` under `$VOX_CACHE_DIR` (viewport/screencast PNGs; deleteable).

Do not add a `SOURCES.toml` row.

- [ ] **Step 4: Run tests / gates**

```
cargo test -p vox-plugin-catalog --test skill_bundle_parity
cargo run -q -p vox-cli -- ci agentskills-compliance
cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/vox-gui-browser-support-2026.md docs/src/architecture/where-things-live.md docs/src/architecture/agent-browser-driver-research-2026.md docs/src/architecture/data-storage-ssot-2026.md
```

Expected: PASS / lint clean. If `agentskills-compliance` is only reachable as `vox ci …`, use that.

- [ ] **Step 5: Commit**

```bash
git add assets/skills/browser-research/SKILL.md crates/vox-plugin-catalog/catalog.toml docs/src/architecture/vox-gui-browser-support-2026.md docs/src/architecture/where-things-live.md docs/src/architecture/agent-browser-driver-research-2026.md docs/src/architecture/data-storage-ssot-2026.md
git commit -m "$(cat <<'EOF'
docs: browser-research skill and chat-loop SSOT

Nudge models toward snapshot then screenshot then extract without adding
a mega-tool or a forced dispatch pipeline.
EOF
)"
```

---

## Self-review (author)

**Spec coverage**

| Spec section | Task |
| --- | --- |
| §5 promote / strip / cap / persist / jail | 0, 1 |
| §5.2 viewport/screencast JSON | 1 |
| §5.3 MCP contents | 2 |
| §5.4 `cache_dir` | 1 |
| §6 LLM parts + wire | 3 |
| §6.3 agent loop + no pixels in transcript | 4 |
| §7 GUI path | 1 |
| §8 skill + catalog | 5 |
| §9 errors | 0, 1 |
| §10 merge-bar tests | 0–5 |
| Non-goals (no mega-tool, no Playwright, no new crate) | all |

**Placeholder scan:** no TBD / “similar to Task N” / “add tests later”.

**Type consistency:** `ImageBlob`, `PromotedToolImage`, `promote_tool_image`, `json_text`, `persist_browser_frame_png`, `attach_image_from_cached_path`, `mcp_contents_for_tool_json`, `llm_tool_message`, `LlmContentPart`, `LlmImageUrl`, `content_parts`, `frame_bytes_from_mcp_data`, `BROWSER_FRAME_IMAGE_PART_MAX_BYTES = 400_000`, `BROWSER_FRAMES_CACHE_LEAF = "browser-frames"`.
