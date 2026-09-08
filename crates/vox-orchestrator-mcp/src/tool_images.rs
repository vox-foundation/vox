//! Strip `image_base64` from MCP tool JSON and optionally keep the decoded PNG.

use base64::Engine;
use rmcp::model::Content;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const BROWSER_FRAME_IMAGE_PART_MAX_BYTES: usize = 400_000;

/// Only these MCP tool names may produce an image part (stdio `call_tool` + Axis).
pub const FRAME_IMAGE_TOOL_NAMES: &[&str] = &[
    "vox_browser_screenshot_viewport",
    "vox_browser_screencast_frame",
];

pub enum FramePersistMode {
    LiveReplace,
    Snapshot,
}

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

pub fn persist_browser_frame_png(
    cache_root: &Path,
    page_id: &str,
    bytes: &[u8],
    mode: FramePersistMode,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(cache_root).map_err(|e| e.to_string())?;
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
    let safe = if safe.is_empty() { "page".into() } else { safe };
    let ext = match sniff_image_mime(bytes) {
        Some("image/jpeg") => "jpg",
        Some("image/png") => "png",
        _ => return Err("frame bytes are not PNG or JPEG".into()),
    };
    let dest = match mode {
        FramePersistMode::LiveReplace => cache_root.join(format!("{safe}-live.{ext}")),
        FramePersistMode::Snapshot => {
            let ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            cache_root.join(format!("{safe}-{ms}.{ext}"))
        }
    };
    std::fs::write(&dest, bytes).map_err(|e| e.to_string())?;
    if !vox_config::paths::path_is_under(cache_root, &dest) {
        let _ = std::fs::remove_file(&dest);
        return Err("frame path escaped cache jail".into());
    }
    if matches!(mode, FramePersistMode::Snapshot) {
        let _ = prune_browser_frames(cache_root, &dest, std::time::Duration::from_secs(3600));
    }
    Ok(dest)
}

/// Delete `*.png` / `*.jpg` under `cache_root` whose mtime is older than `older_than`,
/// except `keep`. `Duration::ZERO` deletes every sibling (used by the unit test).
pub fn prune_browser_frames(
    cache_root: &Path,
    keep: &Path,
    older_than: std::time::Duration,
) -> usize {
    let Ok(keep_cmp) = std::fs::canonicalize(keep) else {
        return 0;
    };
    let Ok(rd) = std::fs::read_dir(cache_root) else {
        return 0;
    };
    let now = SystemTime::now();
    let mut n = 0usize;
    for ent in rd.flatten() {
        let p = ent.path();
        let ext = p.extension().and_then(|e| e.to_str());
        if ext != Some("png") && ext != Some("jpg") && ext != Some("jpeg") {
            continue;
        }
        let Ok(cmp) = std::fs::canonicalize(&p) else {
            continue;
        };
        if cmp == keep_cmp {
            continue;
        }
        let stale = ent
            .metadata()
            .and_then(|m| m.modified())
            .map(|mtime| now.duration_since(mtime).unwrap_or_default() >= older_than)
            .unwrap_or(false);
        if stale && std::fs::remove_file(&p).is_ok() {
            n += 1;
        }
    }
    n
}

pub fn attach_image_from_cached_path(cache_root: &Path, promoted: &mut PromotedToolImage) {
    if promoted.image.is_some() {
        return;
    }
    let Some(path) = promoted.json.pointer("/data/path").and_then(|v| v.as_str()) else {
        return;
    };
    let path = Path::new(path);
    if !path.is_absolute() || !vox_config::paths::path_is_under(cache_root, path) {
        return;
    }
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    if bytes.len() > BROWSER_FRAME_IMAGE_PART_MAX_BYTES {
        return;
    }
    let Some(mime) = sniff_image_mime(&bytes) else {
        return;
    };
    promoted.image = Some(ImageBlob {
        mime: mime.into(),
        bytes,
    });
}

pub fn mcp_contents_for_tool_json(
    tool_name: &str,
    result_json: &str,
    cache_root: &Path,
) -> Vec<Content> {
    if !FRAME_IMAGE_TOOL_NAMES.contains(&tool_name) {
        return vec![Content::text(result_json.to_string())];
    }
    let mut promoted = promote_tool_image(result_json);
    if promoted.json.get("success") != Some(&serde_json::Value::Bool(true)) {
        return vec![Content::text(json_text(&promoted))];
    }
    attach_image_from_cached_path(cache_root, &mut promoted);
    let mut out = vec![Content::text(json_text(&promoted))];
    if let Some(img) = promoted.image {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&img.bytes);
        out.push(Content::image(b64, img.mime));
    }
    out
}

pub fn tool_json_from_screencast_value(cache_root: &Path, page_id: &str, value: Value) -> String {
    let width = value
        .get("viewport_width")
        .cloned()
        .unwrap_or(serde_json::json!(0));
    let height = value
        .get("viewport_height")
        .cloned()
        .unwrap_or(serde_json::json!(0));
    let mut promoted = promote_tool_image(&value.to_string());
    if let Some(img) = promoted.image.take() {
        match persist_browser_frame_png(
            cache_root,
            page_id,
            &img.bytes,
            FramePersistMode::LiveReplace,
        ) {
            Ok(path) => {
                return serde_json::json!({
                    "success": true,
                    "data": {
                        "page_id": page_id,
                        "path": path.to_string_lossy(),
                        "width": width,
                        "height": height,
                        "mime": img.mime
                    }
                })
                .to_string();
            }
            Err(e) => {
                return serde_json::json!({ "success": false, "error": e }).to_string();
            }
        }
    }
    serde_json::json!({ "success": true, "data": promoted.json }).to_string()
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
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .ok()?;
    if bytes.len() > BROWSER_FRAME_IMAGE_PART_MAX_BYTES {
        return None;
    }
    let mime = sniff_image_mime(&bytes)?;
    Some(ImageBlob {
        mime: mime.into(),
        bytes,
    })
}

pub fn sniff_image_mime(bytes: &[u8]) -> Option<&'static str> {
    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if bytes.starts_with(PNG) {
        return Some("image/png");
    }
    if bytes.len() >= 3 && bytes[0] == 0xff && bytes[1] == 0xd8 && bytes[2] == 0xff {
        return Some("image/jpeg");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG_1X1_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

    /// Minimal JPEG sniff fixture: magic `FF D8 FF` (3 bytes).
    const JPEG_MAGIC_B64: &str = "/9j/";

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
        let img = got.image.clone().expect("image");
        assert_eq!(img.mime, "image/png");
        assert_eq!(img.bytes.len(), 70);
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

    #[test]
    fn persist_then_attach_reads_jailed_png() {
        let tmp = std::env::temp_dir().join(format!("vox-frames-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(PNG_1X1_B64)
            .unwrap();
        let path = persist_browser_frame_png(&tmp, "page-1", &bytes, FramePersistMode::Snapshot)
            .expect("write");
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
        attach_image_from_cached_path(&tmp, &mut got);
        assert_eq!(got.image.as_ref().map(|i| i.bytes.len()), Some(70));
        let contents = mcp_contents_for_tool_json("vox_browser_screenshot_viewport", &raw, &tmp);
        assert_eq!(contents.len(), 2);
        let cookie = mcp_contents_for_tool_json(
            "vox_browser_cookies_export",
            &serde_json::json!({"success":true,"data":{"count":1,"path": path.to_string_lossy()}})
                .to_string(),
            &tmp,
        );
        assert_eq!(
            cookie.len(),
            1,
            "cookie path must never become an image part"
        );
    }

    #[test]
    fn live_replace_overwrites_same_page_file() {
        let tmp = std::env::temp_dir().join(format!("vox-live-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(PNG_1X1_B64)
            .unwrap();
        let a =
            persist_browser_frame_png(&tmp, "p", &bytes, FramePersistMode::LiveReplace).unwrap();
        let b =
            persist_browser_frame_png(&tmp, "p", &bytes, FramePersistMode::LiveReplace).unwrap();
        assert_eq!(a, b);
        assert!(
            a.file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with("-live.png")
        );
    }

    #[test]
    fn prune_zero_age_deletes_siblings_keeps_dest() {
        let tmp = std::env::temp_dir().join(format!("vox-prune-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let keep = tmp.join("keep.png");
        let drop = tmp.join("drop.png");
        std::fs::write(&keep, b"keep").unwrap();
        std::fs::write(&drop, b"drop").unwrap();
        let n = prune_browser_frames(&tmp, &keep, std::time::Duration::ZERO);
        assert!(n >= 1);
        assert!(keep.exists());
        assert!(!drop.exists());
    }

    #[test]
    fn attach_rejects_path_outside_cache() {
        let tmp = std::env::temp_dir().join(format!("vox-jail-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let raw = serde_json::json!({
            "success": true,
            "data": { "path": "/etc/hosts" }
        })
        .to_string();
        let mut got = promote_tool_image(&raw);
        attach_image_from_cached_path(&tmp, &mut got);
        assert!(got.image.is_none());
    }

    #[test]
    fn tool_json_from_screencast_jpeg_live_replace() {
        let tmp = std::env::temp_dir().join(format!("vox-sc-jpeg-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let value = serde_json::json!({
            "viewport_width": 640,
            "viewport_height": 480,
            "image_base64": JPEG_MAGIC_B64,
        });
        let raw = tool_json_from_screencast_value(&tmp, "page-jpeg", value);
        let parsed: Value = serde_json::from_str(&raw).expect("json");
        assert_eq!(parsed["success"], true);
        let data = &parsed["data"];
        assert_eq!(data["page_id"], "page-jpeg");
        assert_eq!(data["width"], 640);
        assert_eq!(data["height"], 480);
        assert_eq!(data["mime"], "image/jpeg");
        let path = data["path"].as_str().expect("path");
        assert!(
            path.ends_with("-live.jpg"),
            "expected *-live.jpg, got {path}"
        );
        assert!(Path::new(path).exists());
        let on_disk = std::fs::read(path).expect("read live frame");
        assert_eq!(sniff_image_mime(&on_disk), Some("image/jpeg"));
        assert!(!raw.contains("image_base64"));
    }

    #[test]
    fn tool_json_from_screencast_persist_error() {
        let base = std::env::temp_dir().join(format!("vox-sc-err-{}", std::process::id()));
        let _ = std::fs::remove_file(&base);
        std::fs::write(&base, b"not-a-dir").unwrap();
        let cache_root = base.join("frames");
        let value = serde_json::json!({
            "viewport_width": 1,
            "viewport_height": 1,
            "image_base64": PNG_1X1_B64,
        });
        let raw = tool_json_from_screencast_value(&cache_root, "p", value);
        let parsed: Value = serde_json::from_str(&raw).expect("json");
        assert_eq!(parsed["success"], false);
        assert!(parsed["error"].is_string());
    }

    #[test]
    fn tool_json_from_screencast_no_image_passthrough() {
        let tmp = std::env::temp_dir().join(format!("vox-sc-none-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let value = serde_json::json!({
            "viewport_width": 1024,
            "viewport_height": 768,
            "success": true,
            "data": { "status": "waiting" },
        });
        let raw = tool_json_from_screencast_value(&tmp, "p", value);
        let parsed: Value = serde_json::from_str(&raw).expect("json");
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["data"]["data"]["status"], "waiting");
        assert!(parsed["data"].get("path").is_none());
    }

    #[test]
    fn attach_skips_non_image_path_under_jail() {
        let tmp = std::env::temp_dir().join(format!("vox-cookie-jail-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let cookie_path = tmp.join("cookies.json");
        std::fs::write(&cookie_path, b"[]").unwrap();
        let raw = serde_json::json!({
            "success": true,
            "data": { "path": cookie_path.to_string_lossy() }
        })
        .to_string();
        let mut got = promote_tool_image(&raw);
        attach_image_from_cached_path(&tmp, &mut got);
        assert!(got.image.is_none());
    }

    #[test]
    fn mcp_contents_serializes_image_type() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(PNG_1X1_B64)
            .unwrap();
        let tmp = std::env::temp_dir().join(format!("vox-mcp-img-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let path =
            persist_browser_frame_png(&tmp, "p", &bytes, FramePersistMode::Snapshot).unwrap();
        let raw = serde_json::json!({
            "success": true,
            "data": { "path": path.to_string_lossy() }
        })
        .to_string();
        let contents = mcp_contents_for_tool_json("vox_browser_screenshot_viewport", &raw, &tmp);
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
        let tmp = std::env::temp_dir();
        let contents = mcp_contents_for_tool_json("vox_browser_screenshot_viewport", raw, &tmp);
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn server_rs_call_site_passes_tool_name() {
        let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/server.rs"));
        assert!(
            src.contains("mcp_contents_for_tool_json(&name_str, &result_json,"),
            "call_tool must pass the tool name and cache root; helper-only tests are not enough"
        );
    }
}
